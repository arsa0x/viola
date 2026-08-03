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
                self.chunk.emit(OpCode::SetLocal(*slot), *line);
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
                let jf = self.chunk.emit(OpCode::JumpIfFalse(0), *line);
                for s in then_blk {
                    self.emit_stmt(s);
                }
                if let Some(else_blk) = else_blk {
                    let j = self.chunk.emit(OpCode::Jump(0), *line);
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
                self.chunk.emit(OpCode::Constant(idx), *line);
            }
            RExpr::GetLocal(slot, line) => {
                self.chunk.emit(OpCode::GetLocal(*slot), *line);
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
                    BinOp::NotEq => OpCode::NotEq,
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
                    OpCode::CallNative {
                        id: *id,
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
        Literal::Str(s) => Value::Str(std::sync::Arc::from(s.as_ref())),
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Nil => Value::Nil,
    }
}

fn stack_effect(op: &OpCode) -> i32 {
    match op {
        OpCode::Constant(_) | OpCode::GetLocal(_) => 1,
        OpCode::SetLocal(_) | OpCode::Pop | OpCode::JumpIfFalse(_) => -1,
        OpCode::Add
        | OpCode::Sub
        | OpCode::Mul
        | OpCode::Div
        | OpCode::Eq
        | OpCode::NotEq
        | OpCode::Lt
        | OpCode::Le
        | OpCode::Gt
        | OpCode::Ge => -1,
        OpCode::Not | OpCode::Neg => 0,
        OpCode::Jump(_) | OpCode::Ret => 0,
        OpCode::CallNative { argc, .. } => 1 - (*argc as i32),
    }
}

fn verify_stack_balance(chunk: &Chunk) -> Result<(), CompileError> {
    let mut depth: i32 = 0;
    for (i, op) in chunk.code.iter().enumerate() {
        depth += stack_effect(op);
        if depth < 0 {
            return Err(CompileError::new(
                chunk.lines[i],
                "internal emitter error: stack underflow detected during compile".to_string(),
            ));
        }
    }

    let _ = depth;
    Ok(())
}
