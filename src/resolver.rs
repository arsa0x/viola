use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    ast::{BinOp, Expr, Literal, Script, Stmt, UnOp},
    error::CompileError,
};

#[derive(Debug)]
pub struct Resolver {
    scopes: Vec<Scope>,
    next_slot: u16,
    high_water: u16,
}

#[derive(Debug)]
pub struct Scope {
    vars: HashMap<Rc<str>, u16>,
    assigned: HashSet<Rc<str>>,
}

#[derive(Debug)]
pub struct ResolvedScript {
    pub body: Vec<RStmt>,
    pub local_count: u16,
}

#[derive(Debug)]
pub enum RStmt {
    Assign {
        slot: u16,
        value: RExpr,
        line: u16,
    },

    ExprStmt {
        expr: RExpr,
        line: u16,
    },

    If {
        cond: RExpr,
        then_blk: Vec<RStmt>,
        else_blk: Option<Vec<RStmt>>,
        line: u16,
    },
}

#[derive(Debug)]
pub enum RExpr {
    Literal(Literal, u16),

    GetLocal(u16, u16),

    Binary {
        op: BinOp,
        lhs: Box<RExpr>,
        rhs: Box<RExpr>,
        line: u16,
    },

    Unary {
        op: UnOp,
        expr: Box<RExpr>,
        line: u16,
    },

    NativeCall {
        id: u16,
        args: Vec<RExpr>,
        line: u16,
    },
}

impl RStmt {
    pub fn line(&self) -> u16 {
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

        Ok(ResolvedScript {
            body,
            local_count: r.high_water,
        })
    }

    fn resolve_block(&mut self, stmts: &[Stmt]) -> Result<Vec<RStmt>, CompileError> {
        stmts.iter().map(|s| self.resolve_stmt(s)).collect()
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) -> Result<RStmt, CompileError> {
        match stmt {
            Stmt::Assign { name, value, line } => {
                let value = self.resolve_expr(value)?;
                let slot = self.declare_or_get_slot(name);

                self.mark_assigned(name.clone());

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
                let then_assigned: HashSet<Rc<str>> = self.scopes.last().unwrap().assigned.clone();

                self.pop_scope(mark);

                let else_r = if let Some(else_blk) = else_blk {
                    let mark = self.push_scope();
                    let r = self.resolve_block(else_blk)?;
                    let else_assigned = self.scopes.last().unwrap().assigned.clone();
                    self.pop_scope(mark);

                    for n in then_assigned.intersection(&else_assigned) {
                        self.mark_assigned(n.clone());
                    }

                    Some(r)
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

    fn declare_or_get_slot(&mut self, name: &Rc<str>) -> u16 {
        if let Some(&slot) = self.scopes.last().unwrap().vars.get(name) {
            return slot;
        }

        let slot = self.next_slot;

        self.next_slot += 1;
        self.high_water = self.high_water.max(self.next_slot);
        self.scopes
            .last_mut()
            .unwrap()
            .vars
            .insert(name.clone(), slot);

        slot
    }

    fn mark_assigned(&mut self, name: Rc<str>) {
        self.scopes.last_mut().unwrap().assigned.insert(name);
    }

    fn is_assigned(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|s| s.assigned.contains(name))
    }

    fn resolve_expr(&mut self, expr: &Expr) -> Result<RExpr, CompileError> {
        match expr {
            Expr::Binary { op, lhs, rhs, line } => Ok(RExpr::Binary {
                op: *op,
                lhs: Box::new(self.resolve_expr(lhs)?),
                rhs: Box::new(self.resolve_expr(rhs)?),
                line: *line,
            }),
            Expr::Literal(l, line) => Ok(RExpr::Literal(l.clone(), *line)),
            Expr::NativeCall {
                command,
                method,
                args,
                line,
            } => Err(CompileError::new(*line, "to do")),
            Expr::Unary { op, expr, line } => Ok(RExpr::Unary {
                op: *op,
                expr: Box::new(self.resolve_expr(expr)?),
                line: *line,
            }),
            Expr::Var(v, line) => Err(CompileError::new(*line, "to do")),
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
        let script = resolve("x = 10");

        assert_eq!(script.local_count, 1);
        assert_eq!(script.body.len(), 1);

        match &script.body[0] {
            RStmt::Assign { slot, value, .. } => {
                assert_eq!(*slot, 0);

                assert!(matches!(value, RExpr::Literal(Literal::Int(10), _)));
            }
            _ => panic!("expected assignment"),
        }
    }

    #[test]
    fn resolve_multiple_assignments() {
        let script = resolve(
            r#"x = 1
y = 2
"#,
        );

        assert_eq!(script.local_count, 2);

        match &script.body[0] {
            RStmt::Assign { slot, .. } => assert_eq!(*slot, 0),
            _ => panic!(),
        }

        match &script.body[1] {
            RStmt::Assign { slot, .. } => assert_eq!(*slot, 1),
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_reassignment_uses_same_slot() {
        let script = resolve(
            r#"x = 1
x = 2
"#,
        );

        assert_eq!(script.local_count, 1);

        match &script.body[0] {
            RStmt::Assign { slot, .. } => assert_eq!(*slot, 0),
            _ => panic!(),
        }

        match &script.body[1] {
            RStmt::Assign { slot, .. } => assert_eq!(*slot, 0),
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_binary_expression() {
        let script = resolve("x = 1 + 2");

        match &script.body[0] {
            RStmt::Assign { value, .. } => match value {
                RExpr::Binary { op: BinOp::Add, .. } => {}
                _ => panic!("expected binary expression"),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_unary_expression() {
        let script = resolve("x = -10");

        match &script.body[0] {
            RStmt::Assign { value, .. } => {
                assert!(matches!(value, RExpr::Unary { op: UnOp::Neg, .. }));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_if_statement() {
        let script = resolve(
            r#"
if true {
    x = 1
}
"#,
        );

        assert_eq!(script.body.len(), 1);

        match &script.body[0] {
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
        let script = resolve(
            r#"x = 1
if true {
    y = 2
}
z = 3
"#,
        );

        assert_eq!(script.local_count, 2);
    }
}
