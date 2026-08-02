#[derive(Debug)]
pub struct CompileError {
    pub line: u16,
    pub message: String,
}

impl CompileError {
    pub fn new(line: u16, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}
