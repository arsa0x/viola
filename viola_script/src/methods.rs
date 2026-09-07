use std::sync::Arc;

use crate::{error::VmError, native::Value};

/// Dispatches `receiver.method(args)` to a built-in implementation based on
/// the receiver's *runtime* type.
///
/// Every method here is read-only / pure: it takes a `&Value` and returns a
/// brand-new `Value`, never mutating the receiver in place. That's a
/// deliberate scope limit, not an oversight — `Value::Array`/`Value::Object`
/// are `Arc<Vec<...>>`, which is cheap to clone but not meant to be mutated
/// through a shared reference. A mutating method (`.push()`, `.set()`, ...)
/// needs an explicit decision about whether objects/arrays have reference
/// semantics (mutating one alias affects all of them) or value semantics
/// (mutating writes back only through the variable it was reached through);
/// that's a language-design call, not a VM implementation detail, so it's
/// left out until that's decided.
pub fn call_method(
    receiver: &Value,
    method: &str,
    args: &[Value],
    line: u32,
) -> Result<Value, VmError> {
    match (receiver, method) {
        (Value::Str(s), "len") => {
            expect_argc(method, args, 0, line)?;
            Ok(Value::Int(s.chars().count() as i64))
        }

        (Value::Str(s), "upper") => {
            expect_argc(method, args, 0, line)?;
            Ok(Value::Str(Arc::from(s.to_uppercase())))
        }

        (Value::Str(s), "lower") => {
            expect_argc(method, args, 0, line)?;
            Ok(Value::Str(Arc::from(s.to_lowercase())))
        }

        (Value::Str(s), "trim") => {
            expect_argc(method, args, 0, line)?;
            Ok(Value::Str(Arc::from(s.trim())))
        }

        (Value::Str(s), "contains") => {
            expect_argc(method, args, 1, line)?;
            let needle = expect_str_arg(args, 0, method, line)?;
            Ok(Value::Bool(s.contains(needle.as_ref())))
        }

        (Value::Str(s), "starts_with") => {
            expect_argc(method, args, 1, line)?;
            let needle = expect_str_arg(args, 0, method, line)?;
            Ok(Value::Bool(s.starts_with(needle.as_ref())))
        }

        (Value::Str(s), "ends_with") => {
            expect_argc(method, args, 1, line)?;
            let needle = expect_str_arg(args, 0, method, line)?;
            Ok(Value::Bool(s.ends_with(needle.as_ref())))
        }

        (Value::Str(s), "split") => {
            expect_argc(method, args, 1, line)?;
            let sep = expect_str_arg(args, 0, method, line)?;

            let parts: Vec<Value> = if sep.is_empty() {
                s.split("")
                    .filter(|p| !p.is_empty())
                    .map(|p| Value::Str(Arc::from(p)))
                    .collect()
            } else {
                s.split(sep.as_ref())
                    .map(|p| Value::Str(Arc::from(p)))
                    .collect()
            };

            Ok(Value::Array(Arc::new(parts)))
        }

        (Value::Str(s), "replace") => {
            expect_argc(method, args, 2, line)?;
            let from = expect_str_arg(args, 0, method, line)?;
            let to = expect_str_arg(args, 1, method, line)?;
            Ok(Value::Str(Arc::from(s.replace(from.as_ref(), to.as_ref()))))
        }

        (Value::Array(items), "len") => {
            expect_argc(method, args, 0, line)?;
            Ok(Value::Int(items.len() as i64))
        }

        (Value::Array(items), "contains") => {
            expect_argc(method, args, 1, line)?;
            Ok(Value::Bool(items.iter().any(|v| v == &args[0])))
        }

        (Value::Array(items), "join") => {
            expect_argc(method, args, 1, line)?;
            let sep = expect_str_arg(args, 0, method, line)?;

            let joined = items
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(sep.as_ref());

            Ok(Value::Str(Arc::from(joined)))
        }

        (Value::Array(items), "first") => {
            expect_argc(method, args, 0, line)?;

            Ok(items.first().cloned().unwrap_or(Value::Nil))
        }

        (Value::Array(items), "last") => {
            expect_argc(method, args, 0, line)?;
            Ok(items.last().cloned().unwrap_or(Value::Nil))
        }

        (Value::Object(fields), "has") => {
            expect_argc(method, args, 1, line)?;
            let key = expect_str_arg(args, 0, method, line)?;
            Ok(Value::Bool(
                fields.iter().any(|(k, _)| k.as_ref() == key.as_ref()),
            ))
        }

        (Value::Object(fields), "keys") => {
            expect_argc(method, args, 0, line)?;
            let keys = fields
                .iter()
                .map(|(k, _)| Value::Str(k.clone()))
                .collect::<Vec<_>>();
            Ok(Value::Array(Arc::new(keys)))
        }

        (receiver, method) => Err(VmError::UnknownMethod {
            line,
            receiver_type: receiver.type_name(),
            method: method.to_string(),
        }),
    }
}

fn expect_argc(method: &str, args: &[Value], expected: usize, line: u32) -> Result<(), VmError> {
    if args.len() != expected {
        return Err(VmError::ArityMismatch {
            line,
            method: method.to_string(),
            expected,
            got: args.len(),
        });
    }

    Ok(())
}

fn expect_str_arg<'a>(
    args: &'a [Value],
    idx: usize,
    method: &str,
    line: u32,
) -> Result<&'a Arc<str>, VmError> {
    match &args[idx] {
        Value::Str(s) => Ok(s),
        other => Err(VmError::MethodArgType {
            line,
            method: method.to_string(),
            arg_index: idx,
            expected: "str",
            got: other.type_name(),
        }),
    }
}
