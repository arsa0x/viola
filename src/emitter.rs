use std::sync::Arc;

use crate::ast::{BinOp, Literal, UnOp};
use crate::chunk::{Chunk, OpCode};
use crate::error::CompileError;
use crate::native::Value;
use crate::resolver::{RExpr, RStmt, ResolvedScript};

pub struct Emitter {
    chunk: Chunk,
}

impl Emitter {
    pub fn emit_script(script: &ResolvedScript) -> Result<Chunk, CompileError> {
        let mut e = Emitter {
            chunk: Chunk::default(),
        };

        e.chunk.local_count = script.local_count;

        e.chunk.name = script.name.as_deref().map(str::to_string);
        e.chunk.category = script.category.as_deref().map(str::to_string);
        e.chunk.triggers = script.triggers.iter().map(|s| s.to_string()).collect();
        e.chunk.locals_by_name = script
            .locals_by_name
            .iter()
            .map(|(n, s)| (n.to_string(), *s))
            .collect();

        for stmt in &script.body {
            e.emit_stmt(stmt);
        }

        e.chunk.emit(
            OpCode::Ret,
            script.body.last().map(|s| s.line()).unwrap_or(0),
        );

        verify_stack_balance(&e.chunk)?;
        Ok(e.chunk)
    }

    fn emit_stmt(&mut self, stmt: &RStmt) {
        match stmt {
            RStmt::Assign { slot, value, line } => {
                self.emit_expr(value);
                self.chunk.emit(OpCode::SetL(*slot), *line);
            }
            RStmt::ExprStmt { expr, line } => {
                self.emit_expr(expr);
                self.chunk.emit(OpCode::Pop, *line);
            }
            RStmt::If {
                cond,
                then_blk,
                else_blk,
                line,
            } => {
                self.emit_expr(cond);
                let jf = self.chunk.emit(OpCode::JmpF(0), *line);
                for s in then_blk {
                    self.emit_stmt(s);
                }
                if let Some(else_blk) = else_blk {
                    let j = self.chunk.emit(OpCode::Jmp(0), *line);
                    let else_start = self.chunk.code.len();
                    self.chunk.patch_jump(jf, else_start);
                    for s in else_blk {
                        self.emit_stmt(s);
                    }
                    let end = self.chunk.code.len();
                    self.chunk.patch_jump(j, end);
                } else {
                    let end = self.chunk.code.len();
                    self.chunk.patch_jump(jf, end);
                }
            }
        }
    }

    fn emit_expr(&mut self, expr: &RExpr) {
        match expr {
            RExpr::Literal(lit, line) => {
                let v = literal_to_value(lit);
                let idx = self.chunk.add_constant(v);
                self.chunk.emit(OpCode::Const(idx), *line);
            }
            RExpr::GetLocal(slot, line) => {
                self.chunk.emit(OpCode::GetL(*slot), *line);
            }
            RExpr::Binary { op, lhs, rhs, line } => {
                self.emit_expr(lhs);
                self.emit_expr(rhs);
                let op = match op {
                    BinOp::Add => OpCode::Add,
                    BinOp::Sub => OpCode::Sub,
                    BinOp::Mul => OpCode::Mul,
                    BinOp::Div => OpCode::Div,
                    BinOp::Eq => OpCode::Eq,
                    BinOp::NotEq => OpCode::Ne,
                    BinOp::Lt => OpCode::Lt,
                    BinOp::Le => OpCode::Le,
                    BinOp::Gt => OpCode::Gt,
                    BinOp::Ge => OpCode::Ge,
                };
                self.chunk.emit(op, *line);
            }
            RExpr::Unary { op, expr, line } => {
                self.emit_expr(expr);
                let op = match op {
                    UnOp::Neg => OpCode::Neg,
                    UnOp::Not => OpCode::Not,
                };
                self.chunk.emit(op, *line);
            }
            RExpr::NativeCall { id, args, line } => {
                for a in args {
                    self.emit_expr(a);
                }
                self.chunk.emit(
                    OpCode::CallN {
                        id: *id,
                        argc: args.len() as u8,
                    },
                    *line,
                );
            }
            RExpr::Array(elements, line) => {
                for e in elements {
                    self.emit_expr(e);
                }
                self.chunk.emit(OpCode::MkArr(elements.len() as u16), *line);
            }
            RExpr::Object { properties, line } => {
                let mut layout = Vec::with_capacity(properties.len());

                for (key, value) in properties {
                    let key_idx = self.chunk.add_constant(Value::Str(Arc::from(key.as_ref())));

                    layout.push(key_idx);

                    self.emit_expr(value);
                }

                let layout_idx = self.chunk.add_object_layout(layout.into_boxed_slice());

                self.chunk.emit(OpCode::MkObj(layout_idx), *line);
            }
            RExpr::PropertyAccess {
                object,
                property,
                line,
            } => {
                self.emit_expr(object);
                let prop_idx = self
                    .chunk
                    .add_constant(Value::Str(Arc::from(property.as_ref())));
                self.chunk.emit(OpCode::GetP(prop_idx), *line);
            }
            RExpr::MethodCall {
                object,
                method,
                args,
                line,
            } => {
                self.emit_expr(object);
                for arg in args {
                    self.emit_expr(arg);
                }
                let name_idx = self
                    .chunk
                    .add_constant(Value::Str(Arc::from(method.as_ref())));
                self.chunk.emit(
                    OpCode::CallM {
                        name_idx,
                        argc: args.len() as u8,
                    },
                    *line,
                );
            }
        }
    }
}

fn literal_to_value(lit: &Literal) -> Value {
    match lit {
        Literal::Int(i) => Value::Int(*i),
        Literal::Float(x) => Value::Float(*x),
        Literal::Str(s) => Value::Str(Arc::from(s.as_ref())),
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Nil => Value::Nil,
    }
}

fn stack_effect(chunk: &Chunk, op: &OpCode) -> i32 {
    match op {
        OpCode::Const(_) | OpCode::GetL(_) => 1,
        OpCode::SetL(_) | OpCode::Pop | OpCode::JmpF(_) => -1,
        OpCode::Add
        | OpCode::Sub
        | OpCode::Mul
        | OpCode::Div
        | OpCode::Eq
        | OpCode::Ne
        | OpCode::Lt
        | OpCode::Le
        | OpCode::Gt
        | OpCode::Ge => -1,
        OpCode::Not | OpCode::Neg => 0,
        OpCode::Jmp(_) | OpCode::Ret => 0,
        OpCode::CallN { argc, .. } => 1 - (*argc as i32),
        OpCode::MkArr(n) => 1 - (*n as i32),
        OpCode::CallM { argc, .. } => -(*argc as i32),
        OpCode::GetP(_) => 0,
        OpCode::SetP(_) => -1,
        OpCode::MkObj(n) => 1 - (chunk.object_layouts[*n as usize].len() as i32),
    }
}

fn verify_stack_balance(chunk: &Chunk) -> Result<(), CompileError> {
    let mut depth: i32 = 0;
    for (i, op) in chunk.code.iter().enumerate() {
        depth += stack_effect(chunk, op);
        if depth < 0 {
            return Err(CompileError::new(
                chunk.lines[i],
                "internal emitter error: stack underflow detected during compile",
            ));
        }
    }

    if depth != 0 {
        return Err(CompileError::new(
            chunk.lines.last().copied().unwrap_or(0),
            format!("internal emitter error: stack leak detected ({depth} unhandled values)"),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolver::Resolver;

    fn compile(src: &str) -> Chunk {
        let tokens = Lexer::new(src).tokenize().unwrap();
        let script = Parser::new(tokens).parse_script().unwrap();
        let resolved = Resolver::resolve(&script).unwrap();
        Emitter::emit_script(&resolved).unwrap()
    }

    #[test]
    fn emit_array_literal() {
        let chunk = compile("x = [1, 2, 3]");

        let make_array_count = chunk
            .code
            .iter()
            .filter(|op| matches!(op, OpCode::MkArr(3)))
            .count();
        assert_eq!(make_array_count, 1);
    }

    #[test]
    fn emit_empty_array_balances_stack() {
        let chunk = compile("x = []");
        assert!(chunk.code.iter().any(|op| matches!(op, OpCode::MkArr(0))));
    }

    #[test]
    fn emit_nested_array_balances_stack() {
        let _ = compile("x = [[1, 2], [3, 4], []]");
    }
}
