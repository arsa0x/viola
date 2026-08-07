use std::sync::Arc;

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

    fn line(&self) -> u32 {
        self.chunk.lines.get(self.ip).copied().unwrap_or(0)
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack
            .pop()
            .ok_or(VmError::StackUnderflow { line: self.line() })
    }

    pub async fn run<H: Host>(&mut self, ctx: &ExecContext<H>) -> Result<(), VmError> {
        loop {
            let op = self.chunk.code[self.ip];
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
                    let argc_usize = argc as usize;

                    if self.stack.len() < argc_usize {
                        return Err(VmError::StackUnderflow { line });
                    }

                    let args_start = self.stack.len() - argc_usize;
                    let result = self
                        .dispatch_native(id, &self.stack[args_start..], ctx)
                        .await;

                    self.stack.truncate(args_start);

                    match result {
                        Ok(v) => self.stack.push(v),
                        Err(err) => return Err(VmError::Native { line, err }),
                    }
                }
                OpCode::MakeArray(n) => {
                    let start = self.stack.len() - n as usize;
                    let elements = self.stack.split_off(start);

                    self.stack.push(Value::Array(Arc::new(elements)));
                }
                OpCode::MakeObject(layout_idx) => {
                    let layout = &self.chunk.object_layouts[layout_idx as usize];
                    let count = layout.len();
                    let line = self.line();

                    if self.stack.len() < count {
                        return Err(VmError::StackUnderflow { line });
                    }

                    let values = self.stack.split_off(self.stack.len() - count);

                    let mut props = Vec::with_capacity(count);

                    for (idx, value) in layout.iter().zip(values) {
                        let key = match &self.chunk.constants[*idx as usize] {
                            Value::Str(name) => name.clone(),
                            _ => unreachable!("object key must be string"),
                        };

                        props.push((key, value));
                    }

                    self.stack.push(Value::Object(Arc::new(props)));
                }
                OpCode::GetProperty(n) => {
                    let obj = self.pop()?;

                    let prop_name = match &self.chunk.constants[n as usize] {
                        Value::Str(name) => name,
                        _ => unreachable!("property name must be string"),
                    };

                    if let Some(val) = obj.get_field(prop_name.as_ref()) {
                        self.stack.push(val.clone());
                    } else {
                        self.stack.push(Value::Nil);
                    }
                }
                OpCode::SetProperty(_n) => {
                    unimplemented!("SetProperty not yet implemented")
                }
                OpCode::CallMethod { .. } => {
                    unimplemented!("CallMethod not yet implemented")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emitter::Emitter;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolver::Resolver;

    struct NullHost;
    impl Host for NullHost {
        async fn send_text(&self, _text: &str) -> Result<(), NativeError> {
            Ok(())
        }
    }

    fn compile(src: &str) -> Chunk {
        let tokens = Lexer::new(src).tokenize().unwrap();
        let script = Parser::new(tokens).parse_script().unwrap();
        let resolved = Resolver::resolve(&script).unwrap();
        Emitter::emit_script(&resolved).unwrap()
    }

    async fn run(chunk: &Chunk) -> Vec<Value> {
        let ctx = ExecContext::new(vec![], NullHost);
        let mut vm = Vm::new(chunk);
        vm.run(&ctx).await.unwrap();
        vm.stack
    }

    #[tokio::test]
    async fn make_array_builds_correct_order() {
        let chunk = compile("x = [1, 2, 3]\n:send .text \"done\"");
        run(&chunk).await;
    }

    #[tokio::test]
    async fn array_element_order_preserved() {
        let chunk = compile("[10, 20, 30]");
        let arr = Value::Array(std::sync::Arc::new(vec![
            Value::Int(10),
            Value::Int(20),
            Value::Int(30),
        ]));
        assert_eq!(arr.to_string(), "[10, 20, 30]");

        run(&chunk).await;
    }

    #[tokio::test]
    async fn empty_array_runs_without_underflow() {
        let chunk = compile("x = []");
        run(&chunk).await;
    }

    #[tokio::test]
    async fn nested_array_runs_correctly() {
        let chunk = compile("x = [[1, 2], [3, 4]]");
        run(&chunk).await;
    }

    #[tokio::test]
    async fn array_with_native_call_element() {
        let chunk = compile(r#"x = [:send .text "hi"]"#);
        run(&chunk).await;
    }

    #[tokio::test]
    async fn make_empty_object_runs_without_underflow() {
        let chunk = compile("x = {}");
        run(&chunk).await;
    }

    #[tokio::test]
    async fn make_object_runs_correctly() {
        let chunk = compile("x = { a: 1, b: \"hello\", c: 3.14 }");
        run(&chunk).await;
    }

    // #[tokio::test]
    // async fn get_property_runs_correctly() {
    //     let chunk = compile(
    //         r#"
    //             obj = { name: "test", val: 42 }
    //             x = obj.val
    //         "#,
    //     );
    //     run(&chunk).await;
    // }

    // #[tokio::test]
    // async fn get_missing_property_runs_without_panic() {
    //     let chunk = compile(
    //         r#"
    //             obj = { a: 1 }
    //             x = obj.missing_field
    //         "#,
    //     );
    //     run(&chunk).await;
    // }

    // #[tokio::test]
    // async fn nested_object_property_access() {
    //     let chunk = compile(
    //         r#"
    //             data = {
    //                 user: {
    //                     id: 99,
    //                     active: true
    //                 }
    //             }
    //             res = data.user.id
    //         "#,
    //     );
    //     run(&chunk).await;
    // }

    #[tokio::test]
    async fn get_property_from_array_of_objects() {
        let chunk = compile(
            r#"
                arr = [ {id: 1}, {id: 2} ]
            "#,
        );
        run(&chunk).await;
    }
}
