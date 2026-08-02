use std::{fmt, rc::Rc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum NativeId {
    ArgsAt,
    SendText,
}

pub struct NativeSig {
    pub id: NativeId,
    pub expected_argc: u8,
}

pub struct ExecContext {
    pub args: Vec<Value>,
    pub sent: std::cell::RefCell<Vec<String>>,
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

impl ExecContext {
    pub fn new(args: Vec<Value>) -> Self {
        Self {
            args,
            sent: std::cell::RefCell::new(Vec::new()),
        }
    }
}

pub fn lookup_native(command: &str, method: Option<&str>) -> Option<NativeSig> {
    match (command, method) {
        ("args", Some("at")) => Some(NativeSig {
            id: NativeId::ArgsAt,
            expected_argc: 1,
        }),
        ("send", Some("text")) => Some(NativeSig {
            id: NativeId::SendText,
            expected_argc: 1,
        }),
        _ => None,
    }
}
