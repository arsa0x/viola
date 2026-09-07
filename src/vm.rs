use std::sync::Arc;

use crate::{
    chunk::{Chunk, OpCode},
    error::{NativeError, VmError},
    methods,
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
                OpCode::Const(idx) => {
                    self.stack.push(self.chunk.constants[idx as usize].clone());
                }
                OpCode::GetL(slot) => {
                    self.stack.push(self.locals[slot as usize].clone());
                }
                OpCode::SetL(slot) => {
                    let v = self.pop()?;
                    self.locals[slot as usize] = v;
                }

                OpCode::Add => self.binary_num_or_concat()?,
                OpCode::Sub => self.binary_checked(i64::checked_sub, |a, b| a - b, "sub")?,

                OpCode::Mul => self.binary_checked(i64::checked_mul, |a, b| a * b, "mul")?,
                OpCode::Div => self.binary_checked(i64::checked_div, |a, b| a / b, "div")?,

                OpCode::Eq => {
                    let b = self.pop()?;
                    let a = self.pop()?;

                    self.stack.push(Value::Bool(a == b));
                }
                OpCode::Ne => {
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
                        Value::Int(i) => match i.checked_neg() {
                            Some(v) => Value::Int(v),
                            None => return Err(VmError::ArithmeticOverflow { line, op: "neg" }),
                        },
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

                OpCode::Jmp(offset) => {
                    self.ip = self.jump_target(self.ip, offset);
                }
                OpCode::JmpF(offset) => {
                    let cond = self.pop()?;
                    if !cond.is_truthy() {
                        self.ip = self.jump_target(self.ip, offset);
                    }
                }
                OpCode::JmpFKeep(offset) => {
                    let line = self.line();
                    let cond = self.stack.last().ok_or(VmError::StackUnderflow { line })?;

                    if !cond.is_truthy() {
                        self.ip = self.jump_target(self.ip, offset);
                    }
                }
                OpCode::JmpTKeep(offset) => {
                    let line = self.line();
                    let cond = self.stack.last().ok_or(VmError::StackUnderflow { line })?;

                    if cond.is_truthy() {
                        self.ip = self.jump_target(self.ip, offset);
                    }
                }
                OpCode::CallN { id, argc } => {
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
                OpCode::MkArr(n) => {
                    let start = self.stack.len() - n as usize;
                    let elements = self.stack.split_off(start);

                    self.stack.push(Value::Array(Arc::new(elements)));
                }
                OpCode::MkObj(layout_idx) => {
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
                OpCode::GetP(n) => {
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
                OpCode::SetP(_n) => {
                    return Err(VmError::Unsupported {
                        line: self.line(),
                        what: "property assignment",
                    });
                }
                OpCode::CallM { name_idx, argc } => {
                    let line = self.line();
                    let argc_usize = argc as usize;

                    if self.stack.len() < argc_usize + 1 {
                        return Err(VmError::StackUnderflow { line });
                    }

                    let method_name = match &self.chunk.constants[name_idx as usize] {
                        Value::Str(name) => name.clone(),
                        _ => unreachable!("method name must be string"),
                    };

                    let args_start = self.stack.len() - argc_usize;
                    let args = self.stack.split_off(args_start);
                    let receiver = self.pop()?;

                    let result = methods::call_method(&receiver, &method_name, &args, line)?;

                    self.stack.push(result);
                }
                OpCode::Pop => {
                    self.pop()?;
                }
                OpCode::Ret => return Ok(()),
            }
        }
    }

    fn binary_checked(
        &mut self,
        fi: fn(i64, i64) -> Option<i64>,
        ff: fn(f64, f64) -> f64,
        op_name: &'static str,
    ) -> Result<(), VmError> {
        let line = self.line();

        let b = self.pop()?;
        let a = self.pop()?;

        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => match fi(*x, *y) {
                Some(v) => Value::Int(v),
                None => {
                    if op_name == "div" && *y == 0 {
                        return Err(VmError::DivisionByZero { line });
                    }
                    return Err(VmError::ArithmeticOverflow { line, op: op_name });
                }
            },
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

    fn binary_num_or_concat(&mut self) -> Result<(), VmError> {
        let line = self.line();

        let b = self.pop()?;
        let a = self.pop()?;

        let result = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => match x.checked_add(*y) {
                Some(v) => Value::Int(v),
                None => {
                    return Err(VmError::ArithmeticOverflow { line, op: "add" });
                }
            },
            (Value::Float(x), Value::Float(y)) => Value::Float(x + y),

            (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 + y),
            (Value::Float(x), Value::Int(y)) => Value::Float(x + *y as f64),

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

    pub fn get_local(&self, slot: u16) -> Option<&Value> {
        self.locals.get(slot as usize)
    }

    pub fn get_var(&self, name: &str) -> Option<&Value> {
        let slot = *self.chunk.locals_by_name.get(name)?;
        self.get_local(slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile;

    struct NullHost;

    impl Host for NullHost {
        async fn send_text(&self, _text: &str) -> Result<(), NativeError> {
            Ok(())
        }
    }

    fn block_on<F: std::future::Future>(mut fut: F) -> F::Output {
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

        fn noop(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);

        let raw = RawWaker::new(std::ptr::null(), &VTABLE);
        let waker = unsafe { Waker::from_raw(raw) };
        let mut cx = Context::from_waker(&waker);

        let mut fut = unsafe { std::pin::Pin::new_unchecked(&mut fut) };

        loop {
            if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    async fn run(source: &str) -> Result<(), VmError> {
        let chunk = compile(source).expect("script should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);

        block_on(vm.run(&ctx))
    }

    async fn run_ok(source: &str) {
        run(source)
            .await
            .unwrap_or_else(|err| panic!("script failed unexpectedly: {err:?}"));
    }

    #[tokio::test]
    async fn make_array_builds_correct_order() {
        let result = run("x = [1, 2, 3]\n:send .text \"done\"").await;

        assert!(result.is_ok());
    }

    #[test]
    fn array_literal_preserves_order() {
        let arr = Value::Array(Arc::new(vec![
            Value::Int(10),
            Value::Int(20),
            Value::Int(30),
        ]));
        assert_eq!(arr.to_string(), "[10, 20, 30]");
    }

    #[test]
    fn empty_array_has_zero_elements() {
        let arr = Value::Array(Arc::new(vec![]));
        assert_eq!(arr.to_string(), "[]");
    }

    #[tokio::test]
    async fn empty_array_runs_without_underflow() {
        let result = run("x = []").await;

        assert!(result.is_ok());
    }

    #[test]
    fn nested_array_preserves_structure() {
        let arr = Value::Array(Arc::new(vec![
            Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)])),
            Value::Array(Arc::new(vec![Value::Int(3), Value::Int(4)])),
        ]));
        assert_eq!(arr.to_string(), "[[1, 2], [3, 4]]");
    }

    #[test]
    fn array_values_have_expected_types() {
        let arr = Value::Array(Arc::new(vec![
            Value::Int(1),
            Value::Str("hello".into()),
            Value::Bool(true),
        ]));

        let Value::Array(values) = arr else {
            panic!("expected array");
        };

        assert_eq!(values.len(), 3);
        assert!(matches!(values[0], Value::Int(1)));
        assert!(matches!(&values[1], Value::Str(s) if s.as_ref() == *Arc::new("hello")));
        assert!(matches!(values[2], Value::Bool(true)));
    }

    #[test]
    fn nested_array_executes_successfully() {
        block_on(run_ok("x = [[1, 2], [3, 4]]"));
    }

    #[test]
    fn array_can_contain_native_call_result() {
        block_on(run_ok(r#"x = [:send .text "hi"]"#));
    }

    #[test]
    fn empty_object_executes_without_stack_underflow() {
        block_on(run_ok("x = {}"));
    }

    #[test]
    fn object_literal_executes_successfully() {
        block_on(run_ok(r#"x = { a: 1, b: "hello", c: 3.14 }"#));
    }

    #[test]
    fn object_with_nested_object_executes_successfully() {
        block_on(run_ok(r#" data = { user: { id: 99, active: true } } "#));
    }

    #[test]
    fn property_access_executes_successfully() {
        block_on(run_ok(r#" obj = { name: "test", val: 42 } x = obj.val "#));
    }

    #[test]
    fn nested_property_access_executes_successfully() {
        block_on(run_ok(
            r#" data = { user: { id: 99, active: true } } res = data.user.id "#,
        ));
    }

    #[test]
    fn property_access_from_array_of_objects_executes_successfully() {
        block_on(run_ok(
            r#" arr = [ { id: 1 }, { id: 2 } ] x = arr[0].id y = arr[1].id "#,
        ));
    }

    #[tokio::test]
    async fn missing_property_returns_vm_error() {
        let chunk = compile(
            r#"
            obj = { a: 1 }
            x = obj.missing_field
            "#,
        )
        .expect("script should compile");

        let mut vm = Vm::new(&chunk);

        let ctx = ExecContext::new(vec![], NullHost);

        block_on(vm.run(&ctx)).expect("missing property should not fail the run");

        assert!(
            matches!(vm.get_var("x"), Some(Value::Nil)),
            "expected `x` to read as Nil, got {:?}",
            vm.get_var("x")
        );
    }

    #[tokio::test]
    async fn division_by_zero_is_a_vmerror_not_a_panic() {
        let result = run("x = 10 / 0").await;

        assert!(
            matches!(result, Err(VmError::DivisionByZero { .. })),
            "{result:?}"
        );
    }

    #[tokio::test]
    async fn i64_min_div_neg_one_is_overflow_error_not_panic() {
        let result = run("x = (-9223372036854775807 - 1) / -1").await;

        assert!(
            matches!(result, Err(VmError::ArithmeticOverflow { op: "div", .. })),
            "{result:?}"
        );
    }

    #[tokio::test]
    async fn integer_overflow_add_is_a_vmerror_not_a_panic() {
        let result = run("x = 9223372036854775807 + 1").await;

        assert!(
            matches!(result, Err(VmError::ArithmeticOverflow { op: "add", .. })),
            "{result:?}"
        );
    }

    #[tokio::test]
    async fn integer_overflow_mul_is_a_vmerror_not_a_panic() {
        let result = run("x = 9223372036854775807 * 2").await;

        assert!(
            matches!(result, Err(VmError::ArithmeticOverflow { op: "mul", .. })),
            "{result:?}"
        );
    }

    #[tokio::test]
    async fn normal_arithmetic_still_works() {
        let result = run("x = (1 + 2) * 3 - 4 / 2").await;

        assert!(result.is_ok(), "{result:?}");
    }

    #[tokio::test]
    async fn method_call_is_rejected_cleanly_at_runtime() {
        let chunk = compile(
            r#"
            x = { a: 1 }
            y = x.a(1)
            "#,
        )
        .expect("method call should compile");

        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);

        let result = block_on(vm.run(&ctx));

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn plain_property_access_is_unaffected() {
        let result = run(r#"x = {a: 1}
    y = x.a"#)
        .await;

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn string_interpolation_is_rejected_at_compile_time() {
        let result = compile(r#"x = "halo ${name}""#);

        let err = result.expect_err("string interpolation should not compile");

        assert!(
            !err.message.is_empty(),
            "compile error should contain a useful message"
        );
    }

    #[tokio::test]
    async fn string_concat_still_works() {
        let result = run(r#"x = "hello " + "world""#).await;

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn and_evaluates_normally() {
        let chunk = compile("x = true and 5").expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        block_on(vm.run(&ctx)).expect("should run without error");
        assert!(
            matches!(vm.get_var("x"), Some(Value::Int(5))),
            "{:?}",
            vm.get_var("x")
        );
    }

    #[test]
    fn or_evaluates_normally() {
        let chunk = compile("x = false or 5").expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        block_on(vm.run(&ctx)).expect("should run without error");
        assert!(
            matches!(vm.get_var("x"), Some(Value::Int(5))),
            "{:?}",
            vm.get_var("x")
        );
    }

    #[test]
    fn and_short_circuit_returns_the_falsy_lhs() {
        let chunk = compile("x = false and (10 / 0)").expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        block_on(vm.run(&ctx)).expect("and did not short-circuit");
        assert!(
            matches!(vm.get_var("x"), Some(Value::Bool(false))),
            "{:?}",
            vm.get_var("x")
        );
    }

    #[test]
    fn or_short_circuit_returns_the_truthy_lhs() {
        let chunk = compile("x = true or (10 / 0)").expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        block_on(vm.run(&ctx)).expect("or did not short-circuit");
        assert!(
            matches!(vm.get_var("x"), Some(Value::Bool(true))),
            "{:?}",
            vm.get_var("x")
        );
    }

    #[tokio::test]
    async fn and_short_circuits_and_does_not_evaluate_rhs() {
        let result = run("x = false and (10 / 0)").await;
        assert!(result.is_ok(), "and did not short-circuit: {result:?}");
    }

    #[tokio::test]
    async fn or_short_circuits_and_does_not_evaluate_rhs() {
        let result = run("x = true or (10 / 0)").await;
        assert!(result.is_ok(), "or did not short-circuit: {result:?}");
    }

    #[tokio::test]
    async fn and_does_not_short_circuit_when_it_should_not() {
        let result = run("x = true and (10 / 0)").await;
        assert!(
            matches!(result, Err(VmError::DivisionByZero { .. })),
            "expected the rhs to actually run and fail, got {result:?}"
        );
    }

    #[tokio::test]
    async fn or_does_not_short_circuit_when_it_should_not() {
        let result = run("x = false or (10 / 0)").await;
        assert!(
            matches!(result, Err(VmError::DivisionByZero { .. })),
            "expected the rhs to actually run and fail, got {result:?}"
        );
    }

    #[test]
    fn and_or_precedence_matches_mainstream_languages() {
        let chunk = compile("x = false and false or true").expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);

        block_on(vm.run(&ctx)).expect("should run without error");

        assert!(
            matches!(vm.get_var("x"), Some(Value::Bool(true))),
            "expected (false and false) or true == true, got {:?}",
            vm.get_var("x")
        );
    }

    #[tokio::test]
    async fn and_works_inside_if_condition() {
        let result = run(r#"
            is_admin = true
            has_permission = true
            if is_admin and has_permission {
                x = 1
            } else {
                x = 2
            }
        "#)
        .await;

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn unknown_method_fails_cleanly_at_runtime_not_a_panic() {
        let chunk =
            compile("x = {a: 1}\ny = x.a(1)").expect("should parse as a real MethodCall now");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        let result = block_on(vm.run(&ctx));
        assert!(
            matches!(result, Err(VmError::UnknownMethod { .. })),
            "{result:?}"
        );
    }

    #[test]
    fn string_len_method_works() {
        let chunk = compile(r#"x = "hello".len()"#).expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        block_on(vm.run(&ctx)).expect("should run without error");
        assert!(
            matches!(vm.get_var("x"), Some(Value::Int(5))),
            "{:?}",
            vm.get_var("x")
        );
    }

    #[test]
    fn string_upper_lower_trim_work() {
        let chunk = compile(
            r#"a = "  Halo  ".trim()
    b = "halo".upper()
    c = "HALO".lower()"#,
        )
        .expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        block_on(vm.run(&ctx)).expect("should run without error");
        assert!(matches!(vm.get_var("a"), Some(Value::Str(s)) if s.as_ref() == "Halo"));
        assert!(matches!(vm.get_var("b"), Some(Value::Str(s)) if s.as_ref() == "HALO"));
        assert!(matches!(vm.get_var("c"), Some(Value::Str(s)) if s.as_ref() == "halo"));
    }

    #[test]
    fn string_contains_works_for_wa_bot_style_keyword_matching() {
        let chunk = compile(
            r#"text = "Halo, apa kabar?"
    x = text.lower().contains("halo")"#,
        )
        .expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        block_on(vm.run(&ctx)).expect("should run without error");
        assert!(
            matches!(vm.get_var("x"), Some(Value::Bool(true))),
            "{:?}",
            vm.get_var("x")
        );
    }

    #[test]
    fn array_len_and_contains_work() {
        let chunk = compile(
            r#"arr = [1, 2, 3]
    n = arr.len()
    has_two = arr.contains(2)
    has_five = arr.contains(5)"#,
        )
        .expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        block_on(vm.run(&ctx)).expect("should run without error");
        assert!(matches!(vm.get_var("n"), Some(Value::Int(3))));
        assert!(matches!(vm.get_var("has_two"), Some(Value::Bool(true))));
        assert!(matches!(vm.get_var("has_five"), Some(Value::Bool(false))));
    }

    #[test]
    fn object_has_and_keys_work() {
        let chunk = compile(
            r#"obj = { a: 1, b: 2 }
    x = obj.has("a")
    y = obj.has("z")"#,
        )
        .expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        block_on(vm.run(&ctx)).expect("should run without error");
        assert!(matches!(vm.get_var("x"), Some(Value::Bool(true))));
        assert!(matches!(vm.get_var("y"), Some(Value::Bool(false))));
    }

    #[test]
    fn method_call_wrong_argc_is_a_clean_error() {
        let chunk = compile(r#"x = "hello".len(1)"#).expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        let result = block_on(vm.run(&ctx));
        assert!(
            matches!(result, Err(VmError::ArityMismatch { .. })),
            "{result:?}"
        );
    }

    #[test]
    fn method_call_wrong_arg_type_is_a_clean_error() {
        let chunk = compile(r#"x = "hello".contains(5)"#).expect("should compile");
        let mut vm = Vm::new(&chunk);
        let ctx = ExecContext::new(vec![], NullHost);
        let result = block_on(vm.run(&ctx));
        assert!(
            matches!(result, Err(VmError::MethodArgType { .. })),
            "{result:?}"
        );
    }

    #[test]
    fn string_interpolation_fails_with_a_clear_message_not_a_confusing_one() {
        let err = compile(r#"x = "halo ${name}""#).unwrap_err();
        assert!(
            err.message.contains("not supported yet"),
            "expected a clear 'not supported yet' message, got: {}",
            err.message
        );
    }
}
