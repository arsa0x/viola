use crate::{
    chunk::{Chunk, OpCode},
    error::{NativeError, VmError},
    native::{self, ExecContext, Host, NativeId, Value},
};

pub struct Vm<'a> {
    chunk: &'a Chunk,
    ip: usize,
    stack: Vec<Value>,
    locals: Vec<Value>,
}

impl<'a> Vm<'a> {
    pub fn new(chunk: &'a Chunk) -> Self {
        Self {
            chunk,
            ip: 0,
            stack: Vec::with_capacity(8),
            locals: vec![Value::Nil; chunk.local_count as usize],
        }
    }

    fn line(&self) -> u16 {
        self.chunk.lines.get(self.ip).copied().unwrap_or(0)
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack
            .pop()
            .ok_or(VmError::StackUnderflow { line: self.line() })
    }

    fn pop_n(&mut self, n: usize) -> Result<Vec<Value>, VmError> {
        if self.stack.len() < n {
            return Err(VmError::StackUnderflow { line: self.line() });
        }

        Ok(self.stack.split_off(self.stack.len() - n))
    }

    pub async fn run<H: Host>(&mut self, ctx: &ExecContext<H>) -> Result<(), VmError> {
        loop {
            let op = self.chunk.code[self.ip].clone();
            self.ip += 1;

            match op {
                OpCode::Constant(idx) => {
                    self.stack.push(self.chunk.constants[idx as usize].clone());
                }
                OpCode::GetLocal(slot) => {
                    self.stack.push(self.locals[slot as usize].clone());
                }
                OpCode::SetLocal(slot) => {
                    let v = self.pop()?;
                    self.locals[slot as usize] = v;
                }

                OpCode::Add => self.binary_num_or_concat(|a, b| a + b, |a, b| a + b)?,
                OpCode::Sub => self.binary_num(|a, b| a - b, |a, b| a - b, "sub")?,

                OpCode::Mul => self.binary_num(|a, b| a * b, |a, b| a * b, "mul")?,
                OpCode::Div => self.binary_num(|a, b| a / b, |a, b| a / b, "div")?,

                OpCode::Eq => {
                    let b = self.pop()?;
                    let a = self.pop()?;

                    self.stack.push(Value::Bool(a == b));
                }
                OpCode::NotEq => {
                    let b = self.pop()?;
                    let a = self.pop()?;

                    self.stack.push(Value::Bool(a != b));
                }

                OpCode::Lt => self.compare(|a, b| a < b, |a, b| a < b)?,
                OpCode::Le => self.compare(|a, b| a <= b, |a, b| a <= b)?,
                OpCode::Gt => self.compare(|a, b| a > b, |a, b| a > b)?,
                OpCode::Ge => self.compare(|a, b| a >= b, |a, b| a >= b)?,
                OpCode::Not => {
                    let val = self.pop()?;
                    self.stack.push(Value::Bool(!val.is_truthy()));
                }

                OpCode::Neg => {
                    let v = self.pop()?;
                    let line = self.line();
                    let neg = match v {
                        Value::Int(i) => Value::Int(-i),
                        Value::Float(x) => Value::Float(-x),
                        other => {
                            return Err(VmError::TypeMismatch {
                                line,
                                op: "neg",
                                lhs: other.type_name(),
                                rhs: "-",
                            });
                        }
                    };
                    self.stack.push(neg);
                }

                OpCode::Jump(offset) => {
                    self.ip = self.jump_target(self.ip, offset);
                }
                OpCode::JumpIfFalse(offset) => {
                    let cond = self.pop()?;
                    if !cond.is_truthy() {
                        self.ip = self.jump_target(self.ip, offset);
                    }
                }
                OpCode::CallNative { id, argc } => {
                    let line = self.line();
                    let args = self.pop_n(argc as usize)?;

                    let result = self.dispatch_native(id, &args, ctx).await;

                    match result {
                        Ok(v) => self.stack.push(v),
                        Err(err) => {
                            eprintln!("[viola-script] native call failed at line {line}: {err}");
                            let _ = ctx
                                .host
                                .send_text("An error occurred while executing this command")
                                .await;
                            return Err(VmError::Native { line, err });
                        }
                    }
                }
                OpCode::Pop => {
                    self.pop()?;
                }
                OpCode::Ret => return Ok(()),
            }
        }
    }

    fn jump_target(&self, ip_after_fetch: usize, offset: i16) -> usize {
        (ip_after_fetch as isize + offset as isize) as usize
    }

    async fn dispatch_native<H: Host>(
        &self,
        id: NativeId,
        args: &[Value],
        ctx: &ExecContext<H>,
    ) -> Result<Value, NativeError> {
        match id {
            NativeId::SendText => native::send_text(args, ctx).await,
        }
    }

    fn compare(
        &mut self,
        fi: fn(i64, i64) -> bool,
        ff: fn(f64, f64) -> bool,
    ) -> Result<(), VmError> {
        let line = self.line();
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => fi(*x, *y),
            (Value::Float(x), Value::Float(y)) => ff(*x, *y),
            (Value::Int(x), Value::Float(y)) => ff(*x as f64, *y),
            (Value::Float(x), Value::Int(y)) => ff(*x, *y as f64),
            _ => {
                return Err(VmError::TypeMismatch {
                    line,
                    op: "compare",
                    lhs: a.type_name(),
                    rhs: b.type_name(),
                });
            }
        };
        self.stack.push(Value::Bool(result));
        Ok(())
    }

    fn binary_num(
        &mut self,
        fi: fn(i64, i64) -> i64,
        ff: fn(f64, f64) -> f64,
        op_name: &'static str,
    ) -> Result<(), VmError> {
        let line = self.line();

        let b = self.pop()?;
        let a = self.pop()?;

        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(fi(*x, *y)),
            (Value::Float(x), Value::Float(y)) => Value::Float(ff(*x, *y)),
            (Value::Int(x), Value::Float(y)) => Value::Float(ff(*x as f64, *y)),
            (Value::Float(x), Value::Int(y)) => Value::Float(ff(*x, *y as f64)),
            _ => {
                return Err(VmError::TypeMismatch {
                    line,
                    op: op_name,
                    lhs: a.type_name(),
                    rhs: b.type_name(),
                });
            }
        };

        self.stack.push(result);

        Ok(())
    }

    fn binary_num_or_concat(
        &mut self,
        fi: fn(i64, i64) -> i64,
        ff: fn(f64, f64) -> f64,
    ) -> Result<(), VmError> {
        let line = self.line();

        let b = self.pop()?;
        let a = self.pop()?;

        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(fi(*x, *y)),
            (Value::Float(x), Value::Float(y)) => Value::Float(ff(*x as f64, *y as f64)),

            (Value::Int(x), Value::Float(y)) => Value::Float(ff(*x as f64, *y)),
            (Value::Float(x), Value::Int(y)) => Value::Float(ff(*x, *y as f64)),

            (Value::Str(x), Value::Str(y)) => {
                let mut s = String::with_capacity(x.len() + y.len());

                s.push_str(x);
                s.push_str(y);

                Value::Str(s.into())
            }

            _ => {
                return Err(VmError::TypeMismatch {
                    line,
                    op: "add",
                    lhs: a.type_name(),
                    rhs: b.type_name(),
                });
            }
        };

        self.stack.push(result);
        Ok(())
    }
}
