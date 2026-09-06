use std::fmt;

#[derive(Debug)]
pub struct CompileError {
    pub line: u32,
    pub message: String,
}

impl CompileError {
    pub fn new(line: u32, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "compile error at line {}: {}", self.line, self.message)
    }
}

#[derive(Debug)]
pub enum VmError {
    Native {
        line: u32,
        err: NativeError,
    },

    TypeMismatch {
        line: u32,
        op: &'static str,
        lhs: &'static str,
        rhs: &'static str,
    },

    StackUnderflow {
        line: u32,
    },

    DivisionByZero {
        line: u32,
    },

    ArithmeticOverflow {
        line: u32,
        op: &'static str,
    },

    Unsupported {
        line: u32,
        what: &'static str,
    },
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmError::Native { line, err } => write!(f, "line {line}: native call failed: {err}"),
            VmError::TypeMismatch { line, op, lhs, rhs } => {
                write!(f, "line {line}: type mismatch for `{op}`: {lhs} vs {rhs}")
            }
            VmError::StackUnderflow { line } => {
                write!(f, "line {line}: internal error: stack underflow")
            }
            VmError::DivisionByZero { line } => {
                write!(f, "line {line}: division by zero")
            }
            VmError::ArithmeticOverflow { line, op } => {
                write!(f, "line {line}: arithmetic overflow in `{op}`")
            }
            VmError::Unsupported { line, what } => {
                write!(f, "line {line}: {what} is not supported yet")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct NativeError {
    pub kind: NativeErrorKind,
    pub detail: String,
}

impl NativeError {
    pub fn invalid_arg(detail: impl Into<String>) -> Self {
        Self {
            kind: NativeErrorKind::InvalidArg,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.detail)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeErrorKind {
    InvalidArg,
    Io,
    HostRejected,
}

impl std::error::Error for VmError {}
impl std::error::Error for CompileError {}
