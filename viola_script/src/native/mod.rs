pub mod lookup;
pub mod message;
pub mod send;
pub mod specs;
pub mod utils;

use std::{fmt, sync::Arc};

use crate::{error::NativeError, native::specs::SingleSelectSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum NativeId {
    SendText,
    SendSingleSelect,
    SendReaction,
    ArgsGet,
    ArgsAll,
    ArgsCount,
    MessageText,
    MessageSender,
    MessageIsGroup,
}

pub struct NativeSig {
    pub id: NativeId,
    pub min_argc: u8,
    pub max_argc: u8,
}

impl NativeSig {
    pub fn fixed(id: NativeId, argc: u8) -> Self {
        Self {
            id,
            min_argc: argc,
            max_argc: argc,
        }
    }

    pub fn with_optional(id: NativeId, required: u8, optional: u8) -> Self {
        Self {
            id,
            min_argc: required,
            max_argc: required + optional,
        }
    }
}

pub struct ExecContext<H: Host> {
    pub args: Vec<Value>,
    pub host: H,
}

#[allow(async_fn_in_trait)]
pub trait Host {
    // basic message sender
    async fn send_text(&self, text: &str, quoted: bool) -> Result<(), NativeError>;
    async fn send_reaction(&self, emoji: &str) -> Result<(), NativeError>;

    // interactive message sender
    async fn send_single_select(&self, spec: SingleSelectSpec<'_>) -> Result<(), NativeError>;

    // information
    fn message_text(&self) -> Option<&str>;
    fn sender(&self) -> &str;
    fn is_group(&self) -> bool;
}

#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Arc<str>),
    Array(Arc<Vec<Value>>),
    Object(Arc<Vec<(Arc<str>, Value)>>),
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
            Self::Object(_) => "object",
        }
    }

    pub fn get_field(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Object(fields) => fields
                .iter()
                .find(|(k, _)| k.as_ref() == name)
                .map(|(_, v)| v),
            _ => None,
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
            (Value::Object(a), Value::Object(b)) => a.as_ref() == b.as_ref(),
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
            Value::Object(fields) => {
                write!(f, "{{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl<H: Host> ExecContext<H> {
    pub fn new(args: Vec<Value>, host: H) -> Self {
        Self { args, host }
    }
}

#[cfg(test)]
mod tests {
    use crate::native::lookup::lookup_native;

    use super::*;

    #[test]
    fn type_names() {
        assert_eq!(Value::Nil.type_name(), "nil");
        assert_eq!(Value::Bool(true).type_name(), "bool");
        assert_eq!(Value::Int(1).type_name(), "int");
        assert_eq!(Value::Float(1.5).type_name(), "float");
        assert_eq!(Value::Str("hello".into()).type_name(), "str");
        assert_eq!(Value::Array(Arc::new(vec![])).type_name(), "array");
        assert_eq!(Value::Object(Arc::new(vec![])).type_name(), "object");
    }

    #[test]
    fn truthiness() {
        assert!(!Value::Nil.is_truthy());
        assert!(!Value::Bool(false).is_truthy());

        assert!(Value::Bool(true).is_truthy());
        assert!(Value::Int(0).is_truthy());
        assert!(Value::Float(0.0).is_truthy());
        assert!(Value::Str("".into()).is_truthy());
        assert!(Value::Array(Arc::new(vec![])).is_truthy());
    }

    #[test]
    fn display_values() {
        assert_eq!(Value::Nil.to_string(), "nil");
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Int(42).to_string(), "42");
        assert_eq!(Value::Float(3.5).to_string(), "3.5");
        assert_eq!(Value::Str("hello".into()).to_string(), "hello");
    }

    #[test]
    fn primitive_equality() {
        assert_eq!(Value::Nil, Value::Nil);

        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));

        assert_eq!(Value::Int(10), Value::Int(10));
        assert_ne!(Value::Int(10), Value::Int(20));

        assert_eq!(Value::Float(1.5), Value::Float(1.5));
        assert_ne!(Value::Float(1.5), Value::Float(2.0));

        assert_eq!(Value::Str("abc".into()), Value::Str("abc".into()));
        assert_ne!(Value::Str("abc".into()), Value::Str("xyz".into()));
    }

    #[test]
    fn lookup_native_send_text() {
        let sig = lookup_native("send", Some("text")).unwrap();

        assert_eq!(sig.id, NativeId::SendText);
        assert_eq!(sig.min_argc, 1);
    }

    #[test]
    fn lookup_native_unknown() {
        assert!(lookup_native("send", None).is_none());
        assert!(lookup_native("foo", Some("bar")).is_none());
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

    #[test]
    fn object_equality_is_structural() {
        let a = Value::Object(Arc::new(vec![(Arc::from("x"), Value::Int(1))]));
        let b = Value::Object(Arc::new(vec![(Arc::from("x"), Value::Int(1))]));
        let c = Value::Object(Arc::new(vec![(Arc::from("x"), Value::Int(2))]));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn object_display() {
        let o = Value::Object(Arc::new(vec![
            (Arc::from("a"), Value::Int(1)),
            (Arc::from("b"), Value::Str("x".into())),
        ]));
        assert_eq!(o.to_string(), "{a: 1, b: x}");
    }

    #[test]
    fn get_field_found() {
        let o = Value::Object(Arc::new(vec![(
            Arc::from("name"),
            Value::Str("viola".into()),
        )]));
        assert_eq!(o.get_field("name"), Some(&Value::Str("viola".into())));
    }

    #[test]
    fn get_field_missing() {
        let o = Value::Object(Arc::new(vec![(
            Arc::from("name"),
            Value::Str("viola".into()),
        )]));
        assert_eq!(o.get_field("nope"), None);
    }

    #[test]
    fn get_field_on_non_object_is_none() {
        assert_eq!(Value::Int(1).get_field("x"), None);
    }
}
