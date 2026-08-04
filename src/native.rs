use std::{fmt, sync::Arc};

use crate::error::NativeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum NativeId {
    SendText,
}

pub struct NativeSig {
    pub id: NativeId,
    pub expected_argc: u8,
}

pub struct ExecContext<H: Host> {
    pub args: Vec<Value>,
    pub host: H,
}

#[allow(async_fn_in_trait)]
pub trait Host {
    async fn send_text(&self, text: &str) -> Result<(), NativeError>;
}

#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Arc<str>),
    Array(Arc<Vec<Value>>),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::Array(_) => "array",
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a.as_ref() == b.as_ref(),
            (Value::Array(a), Value::Array(b)) => a.as_ref() == b.as_ref(),
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Array(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
        }
    }
}

impl<H: Host> ExecContext<H> {
    pub fn new(args: Vec<Value>, host: H) -> Self {
        Self { args, host }
    }
}

pub fn lookup_native(command: &str, method: Option<&str>) -> Option<NativeSig> {
    match (command, method) {
        ("send", Some("text")) => Some(NativeSig {
            id: NativeId::SendText,
            expected_argc: 1,
        }),
        _ => None,
    }
}

pub async fn send_text<H: Host>(
    args: &[Value],
    ctx: &ExecContext<H>,
) -> Result<Value, NativeError> {
    let text = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        Some(v) => {
            return Err(NativeError::invalid_arg(format!(
                "`:send .text` need str, get {}",
                v.type_name()
            )));
        }
        None => {
            return Err(NativeError::invalid_arg(
                "`:send .text` requires 1 argument",
            ));
        }
    };

    ctx.host.send_text(&text).await?;

    Ok(Value::Nil)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_type_name() {
        assert_eq!(Value::Array(Arc::new(vec![])).type_name(), "array");
    }

    #[test]
    fn array_equality_is_structural() {
        let a = Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)]));
        let b = Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)]));
        let c = Value::Array(Arc::new(vec![Value::Int(1), Value::Int(3)]));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn array_clone_is_cheap_pointer_bump() {
        let a = Value::Array(Arc::new(vec![Value::Int(1)]));
        let b = a.clone();
        if let (Value::Array(x), Value::Array(y)) = (&a, &b) {
            assert!(Arc::ptr_eq(x, y));
        } else {
            panic!();
        }
    }

    #[test]
    fn array_display() {
        let a = Value::Array(Arc::new(vec![Value::Int(1), Value::Str("x".into())]));
        assert_eq!(a.to_string(), "[1, x]");
    }

    #[test]
    fn array_is_always_truthy() {
        assert!(Value::Array(Arc::new(vec![])).is_truthy());
    }
}
