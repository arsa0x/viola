use std::{fmt, rc::Rc};

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
    pub chat_id: Rc<str>,
    pub host: H,
}

#[allow(async_fn_in_trait)]
pub trait Host {
    async fn send_text(&self, chat_id: &str, text: &str) -> Result<(), NativeError>;
}

#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<str>),
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
        }
    }
}

impl<H: Host> ExecContext<H> {
    pub fn new(args: Vec<Value>, chat_id: impl Into<Rc<str>>, host: H) -> Self {
        Self {
            args,
            chat_id: chat_id.into(),
            host,
        }
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
                "`:send .text` butuh str, dapat {}",
                v.type_name()
            )));
        }
        None => {
            return Err(NativeError::invalid_arg(
                "`:send .text` requires 1 argument",
            ));
        }
    };

    ctx.host.send_text(&ctx.chat_id, &text).await?;

    Ok(Value::Nil)
}
