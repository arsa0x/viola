pub mod ast;
pub mod chunk;
pub mod emitter;
pub mod error;
pub mod lexer;
pub mod native;
pub mod parser;
pub mod resolver;
pub mod vm;

pub fn compile(src: &str) -> Result<chunk::Chunk, error::CompileError> {
    let tokens = lexer::Lexer::new(src)
        .tokenize()
        .map_err(|e| error::CompileError::new(e.line, e.message))?;

    let script = parser::Parser::new(tokens).parse_script()?;
    let resolved = resolver::Resolver::resolve(&script)?;

    emitter::Emitter::emit_script(&resolved)
}
