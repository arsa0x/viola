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

    UnknownMethod {
        line: u32,
        receiver_type: &'static str,
        method: String,
    },

    ArityMismatch {
        line: u32,
        method: String,
        expected: usize,
        got: usize,
    },

    MethodArgType {
        line: u32,
        method: String,
        arg_index: usize,
        expected: &'static str,
        got: &'static str,
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
            VmError::UnknownMethod {
                line,
                receiver_type,
                method,
            } => {
                write!(
                    f,
                    "line {line}: `{receiver_type}` has no method `.{method}()`"
                )
            }
            VmError::ArityMismatch {
                line,
                method,
                expected,
                got,
            } => {
                write!(
                    f,
                    "line {line}: `.{method}()` expects {expected} argument(s), got {got}"
                )
            }
            VmError::MethodArgType {
                line,
                method,
                arg_index,
                expected,
                got,
            } => {
                write!(
                    f,
                    "line {line}: `.{method}()` argument {arg_index} should be {expected}, got {got}"
                )
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
