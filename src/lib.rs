pub mod ast;
pub mod error;
pub mod lexer;
pub mod native;
pub mod parser;
pub mod resolver;

use crate::{ast::Script, error::CompileError, lexer::Lexer, parser::Parser};

pub fn compile(src: &str) -> Result<Script, CompileError> {
    let tokens = Lexer::new(src)
        .tokenize()
        .map_err(|e| CompileError::new(e.line, e.message))?;

    Parser::new(tokens).parse_script()
}
