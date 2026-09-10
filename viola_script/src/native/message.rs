use std::sync::Arc;

use crate::{
    error::NativeError,
    native::{ExecContext, Host, Value},
};

pub fn args_get<H: Host>(args: &[Value], ctx: &ExecContext<H>) -> Result<Value, NativeError> {
    let index = match args.first() {
        Some(Value::Int(i)) if *i >= 0 => *i as usize,
        Some(v) => {
            return Err(NativeError::invalid_arg(format!(
                "`:args.get` needs a non-negative int index, got {}",
                v.type_name()
            )));
        }
        None => return Err(NativeError::invalid_arg("`:args.get` requires 1 argument")),
    };

    Ok(ctx.args.get(index).cloned().unwrap_or(Value::Nil))
}

pub fn args_all<H: Host>(ctx: &ExecContext<H>) -> Value {
    Value::Array(Arc::new(ctx.args.clone()))
}

pub fn args_count<H: Host>(ctx: &ExecContext<H>) -> Value {
    Value::Int(ctx.args.len() as i64)
}

pub fn message_text<H: Host>(ctx: &ExecContext<H>) -> Value {
    match ctx.host.message_text() {
        Some(t) => Value::Str(Arc::from(t)),
        None => Value::Nil,
    }
}

pub fn message_sender<H: Host>(ctx: &ExecContext<H>) -> Value {
    Value::Str(Arc::from(ctx.host.sender()))
}

pub fn message_is_group<H: Host>(ctx: &ExecContext<H>) -> Value {
    Value::Bool(ctx.host.is_group())
}
