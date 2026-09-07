use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    ast::{BinOp, Expr, Literal, LogicalOp, Script, Stmt, UnOp},
    error::CompileError,
    native::{self, NativeId},
};

#[derive(Debug)]
pub struct Resolver {
    scopes: Vec<Scope>,
    next_slot: u16,
    high_water: u16,
}

#[derive(Debug)]
pub struct ResolvedScript {
    pub body: Vec<RStmt>,
    pub local_count: u16,
    pub locals_by_name: HashMap<Rc<str>, u16>,

    pub name: Option<Rc<str>>,
    pub triggers: Vec<Rc<str>>,
    pub category: Option<Rc<str>>,
}

#[derive(Debug)]
pub struct Scope {
    vars: HashMap<Rc<str>, u16>,
    assigned: HashSet<u16>,
}

#[derive(Debug)]
pub enum RStmt {
    Assign {
        slot: u16,
        value: RExpr,
        line: u32,
    },

    ExprStmt {
        expr: RExpr,
        line: u32,
    },

    If {
        cond: RExpr,
        then_blk: Vec<RStmt>,
        else_blk: Option<Vec<RStmt>>,
        line: u32,
    },
}

#[derive(Debug)]
pub enum RExpr {
    Literal(Literal, u32),

    GetLocal(u16, u32),

    Binary {
        op: BinOp,
        lhs: Box<RExpr>,
        rhs: Box<RExpr>,
        line: u32,
    },

    Unary {
        op: UnOp,
        expr: Box<RExpr>,
        line: u32,
    },

    NativeCall {
        id: NativeId,
        args: Vec<RExpr>,
        line: u32,
    },

    Array(Vec<RExpr>, u32),

    Object {
        properties: Vec<(Rc<str>, RExpr)>,
        line: u32,
    },

    PropertyAccess {
        object: Box<RExpr>,
        property: Rc<str>,
        line: u32,
    },

    MethodCall {
        object: Box<RExpr>,
        method: Rc<str>,
        args: Vec<RExpr>,
        line: u32,
    },

    Logical {
        op: LogicalOp,
        lhs: Box<RExpr>,
        rhs: Box<RExpr>,
        line: u32,
    },
}

impl RStmt {
    pub fn line(&self) -> u32 {
        match self {
            RStmt::Assign { line, .. } => *line,
            RStmt::ExprStmt { line, .. } => *line,
            RStmt::If { line, .. } => *line,
        }
    }
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope {
                vars: HashMap::new(),
                assigned: Default::default(),
            }],
            next_slot: 0,
            high_water: 0,
        }
    }

    pub fn resolve(script: &Script) -> Result<ResolvedScript, CompileError> {
        let mut r = Resolver::new();
        let body = r.resolve_block(&script.body)?;
        let locals_by_name = r.scopes[0].vars.clone();

        Ok(ResolvedScript {
            body,
            locals_by_name,
            local_count: r.high_water,
            name: script.meta.name.clone(),
            category: script.meta.category.clone(),
            triggers: script.meta.triggers.clone(),
        })
    }

    fn resolve_block(&mut self, stmts: &[Stmt]) -> Result<Vec<RStmt>, CompileError> {
        stmts.iter().map(|s| self.resolve_stmt(s)).collect()
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) -> Result<RStmt, CompileError> {
        match stmt {
            Stmt::Assign { name, value, line } => {
                let value = self.resolve_expr(value)?;
                let slot = self.resolve_or_declare(name);

                self.mark_assigned(slot);

                Ok(RStmt::Assign {
                    slot,
                    value,
                    line: *line,
                })
            }

            Stmt::ExprStmt { expr, line } => Ok(RStmt::ExprStmt {
                expr: self.resolve_expr(expr)?,
                line: *line,
            }),

            Stmt::If {
                cond,
                then_blk,
                else_blk,
                line,
            } => {
                let cond = self.resolve_expr(cond)?;
                let mark = self.push_scope();

                let then_r = self.resolve_block(then_blk)?;
                let then_assigned = self.current_assigned();

                self.pop_scope(mark);

                let else_r = if let Some(else_blk) = else_blk {
                    let mark = self.push_scope();

                    let else_r = self.resolve_block(else_blk)?;
                    let else_assigned = self.current_assigned();

                    self.pop_scope(mark);

                    for slot in then_assigned.intersection(&else_assigned) {
                        if *slot < mark {
                            self.mark_assigned(*slot);
                        }
                    }

                    Some(else_r)
                } else {
                    None
                };

                Ok(RStmt::If {
                    cond,
                    then_blk: then_r,
                    else_blk: else_r,
                    line: *line,
                })
            }
        }
    }

    fn resolve_or_declare(&mut self, name: &Rc<str>) -> u16 {
        if let Some(slot) = self.find_slot(name) {
            return slot;
        }

        let slot = self.next_slot;

        self.next_slot = self
            .next_slot
            .checked_add(1)
            .expect("too many local variables");

        self.high_water = self.high_water.max(self.next_slot);

        self.scopes
            .last_mut()
            .expect("resolver always has root scope")
            .vars
            .insert(name.clone(), slot);

        slot
    }

    fn push_scope(&mut self) -> u16 {
        let mark = self.next_slot;
        self.scopes.push(Scope {
            vars: HashMap::new(),
            assigned: Default::default(),
        });

        mark
    }

    fn pop_scope(&mut self, mark: u16) {
        self.scopes.pop();
        self.next_slot = mark;
    }

    fn mark_assigned(&mut self, slot: u16) {
        self.scopes
            .last_mut()
            .expect("resolver always has root scope")
            .assigned
            .insert(slot);
    }

    fn is_assigned(&self, slot: u16) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.assigned.contains(&slot))
    }

    fn current_assigned(&self) -> HashSet<u16> {
        self.scopes
            .last()
            .expect("resolver always has root scope")
            .assigned
            .clone()
    }

    fn find_slot(&self, name: &str) -> Option<u16> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.vars.get(name).copied())
    }

    fn resolve_expr(&mut self, expr: &Expr) -> Result<RExpr, CompileError> {
        match expr {
            Expr::Binary { op, lhs, rhs, line } => Ok(RExpr::Binary {
                op: *op,
                lhs: Box::new(self.resolve_expr(lhs)?),
                rhs: Box::new(self.resolve_expr(rhs)?),
                line: *line,
            }),

            Expr::Literal(literal, line) => Ok(RExpr::Literal(literal.clone(), *line)),

            Expr::NativeCall {
                command,
                method,
                args,
                line,
            } => {
                let sig = native::lookup_native(command, method.as_deref()).ok_or_else(|| {
                    let full = match method {
                        Some(method) => {
                            format!(":{command} .{method}")
                        }
                        None => {
                            format!(":{command}")
                        }
                    };

                    CompileError::new(*line, format!("unknown command: `{full}`"))
                })?;

                if args.len() != sig.expected_argc as usize {
                    return Err(CompileError::new(
                        *line,
                        format!(
                            "command `:{command}` takes {} arguments, found {}",
                            sig.expected_argc,
                            args.len()
                        ),
                    ));
                }

                let resolved_args = args
                    .iter()
                    .map(|arg| self.resolve_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(RExpr::NativeCall {
                    id: sig.id,
                    args: resolved_args,
                    line: *line,
                })
            }

            Expr::Unary { op, expr, line } => Ok(RExpr::Unary {
                op: *op,
                expr: Box::new(self.resolve_expr(expr)?),
                line: *line,
            }),

            Expr::Var(name, line) => {
                let slot = match self.find_slot(name) {
                    Some(slot) => slot,
                    None => {
                        return Err(CompileError::new(
                            *line,
                            format!(
                                "variable `${name}` is read before it is filled in all branches"
                            ),
                        ));
                    }
                };

                if !self.is_assigned(slot) {
                    return Err(CompileError::new(
                        *line,
                        format!("variable `${name}` is read before it is filled in all branches"),
                    ));
                }

                Ok(RExpr::GetLocal(slot, *line))
            }

            Expr::Array { elements, line } => {
                let elements = elements
                    .iter()
                    .map(|element| self.resolve_expr(element))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(RExpr::Array(elements, *line))
            }

            Expr::Object { properties, line } => {
                let mut resolved = Vec::with_capacity(properties.len());

                for (key, value) in properties {
                    resolved.push((key.clone(), self.resolve_expr(value)?));
                }

                Ok(RExpr::Object {
                    properties: resolved,
                    line: *line,
                })
            }

            Expr::PropertyAccess {
                object,
                property,
                line,
            } => {
                let object = self.resolve_expr(object)?;

                Ok(RExpr::PropertyAccess {
                    object: Box::new(object),
                    property: property.clone(),
                    line: *line,
                })
            }

            Expr::MethodCall {
                object,
                method,
                args,
                line,
            } => {
                let object = self.resolve_expr(object)?;

                let args = args
                    .iter()
                    .map(|arg| self.resolve_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(RExpr::MethodCall {
                    object: Box::new(object),
                    method: method.clone(),
                    args,
                    line: *line,
                })
            }

            Expr::Logical { op, lhs, rhs, line } => Ok(RExpr::Logical {
                op: *op,
                lhs: Box::new(self.resolve_expr(lhs)?),
                rhs: Box::new(self.resolve_expr(rhs)?),
                line: *line,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser};

    fn resolve(src: &str) -> ResolvedScript {
        let tokens = Lexer::new(src).tokenize().unwrap();
        let script = Parser::new(tokens).parse_script().unwrap();

        Resolver::resolve(&script).unwrap()
    }

    #[test]
    fn resolve_single_assignment() {
        let resolved = resolve("x = 10");

        assert_eq!(resolved.local_count, 1);

        match &resolved.body[0] {
            RStmt::Assign { slot, .. } => {
                assert_eq!(*slot, 0);
            }
            _ => panic!("expected assignment"),
        }
    }

    #[test]
    fn resolve_multiple_assignments() {
        let resolved = resolve(
            r#"x = 1
y = 2
"#,
        );

        assert_eq!(resolved.local_count, 2);

        match &resolved.body[0] {
            RStmt::Assign { slot, .. } => assert_eq!(*slot, 0),
            _ => panic!(),
        }

        match &resolved.body[1] {
            RStmt::Assign { slot, .. } => assert_eq!(*slot, 1),
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_reassignment_uses_same_slot() {
        let resolved = resolve(
            r#"x = 1
x = 2
"#,
        );

        assert_eq!(resolved.local_count, 1);

        match &resolved.body[0] {
            RStmt::Assign { slot, .. } => assert_eq!(*slot, 0),
            _ => panic!(),
        }

        match &resolved.body[1] {
            RStmt::Assign { slot, .. } => assert_eq!(*slot, 0),
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_binary_expression() {
        let resolved = resolve("x = 1 + 2");

        match &resolved.body[0] {
            RStmt::Assign { value, .. } => match value {
                RExpr::Binary { op: BinOp::Add, .. } => {}
                _ => panic!("expected binary expression"),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_unary_expression() {
        let resolved = resolve("x = -10");

        match &resolved.body[0] {
            RStmt::Assign { value, .. } => {
                assert!(matches!(value, RExpr::Unary { op: UnOp::Neg, .. }));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_if_statement() {
        let resolved = resolve(
            r#"
if true {
    x = 1
}
"#,
        );

        assert_eq!(resolved.body.len(), 1);

        match &resolved.body[0] {
            RStmt::If {
                cond,
                then_blk,
                else_blk,
                ..
            } => {
                assert!(matches!(cond, RExpr::Literal(Literal::Bool(true), _)));

                assert!(else_blk.is_none());
                assert_eq!(then_blk.len(), 1);

                match &then_blk[0] {
                    RStmt::Assign { slot, .. } => {
                        assert_eq!(*slot, 0);
                    }
                    _ => panic!("expected assignment"),
                }
            }
            _ => panic!("expected if"),
        }
    }

    #[test]
    fn local_count_tracks_peak_slots() {
        let resolved = resolve(
            r#"x = 1
if true {
    y = 2
}
z = 3
"#,
        );

        assert_eq!(resolved.local_count, 2);
    }

    #[test]
    fn reassign_reuses_slot() {
        let resolved = resolve(
            r#"
x = 1
x = 2
    "#,
        );

        match &resolved.body[..] {
            [
                RStmt::Assign { slot: s1, .. },
                RStmt::Assign { slot: s2, .. },
            ] => {
                assert_eq!(*s1, 0);
                assert_eq!(*s2, 0);
            }
            _ => panic!(),
        }

        assert_eq!(resolved.local_count, 1);
    }

    #[test]
    fn error_read_before_assignment() {
        let tokens = Lexer::new(":send .text x").tokenize().unwrap();
        let script = Parser::new(tokens).parse_script().unwrap();

        let err = Resolver::resolve(&script).unwrap_err();

        assert!(err.message.contains("read before"), "{}", err.message);
    }

    #[test]
    fn unknown_native_returns_error() {
        let tokens = Lexer::new(":does_not_exist").tokenize().unwrap();
        let script = Parser::new(tokens).parse_script().unwrap();

        let err = Resolver::resolve(&script).unwrap_err();

        assert!(err.message.contains("unknown command"));
    }

    #[test]
    fn slots_are_reused_after_scope() {
        let resolved = resolve(
            r#"
    if true {
        a = 1
    }

    b = 2
    "#,
        );

        match &resolved.body[1] {
            RStmt::Assign { slot, .. } => {
                assert_eq!(*slot, 0);
            }
            _ => panic!(),
        }

        assert_eq!(resolved.local_count, 1);
    }

    #[test]
    fn resolve_array_literal() {
        let resolved = resolve("x = [1, 2, 3]");

        match &resolved.body[0] {
            RStmt::Assign { value, .. } => match value {
                RExpr::Array(elements, _) => assert_eq!(elements.len(), 3),
                _ => panic!("expected array"),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_array_with_variable_reference() {
        let err = Resolver::resolve(
            &Parser::new(Lexer::new("y = [x]").tokenize().unwrap())
                .parse_script()
                .unwrap(),
        )
        .unwrap_err();

        assert!(err.message.contains("read before"));
    }

    #[test]
    fn resolve_nested_array() {
        let resolved = resolve("x = [[1, 2], [3]]");

        match &resolved.body[0] {
            RStmt::Assign { value, .. } => match value {
                RExpr::Array(elements, _) => {
                    assert_eq!(elements.len(), 2);
                    assert!(matches!(elements[0], RExpr::Array(_, _)));
                }
                _ => panic!("expected array"),
            },
            _ => panic!(),
        }
    }
}
