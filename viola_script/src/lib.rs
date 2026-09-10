pub mod ast;
pub mod chunk;
pub mod emitter;
pub mod error;
pub mod lexer;
pub mod methods;
pub mod native;
pub mod parser;
pub mod resolver;
pub mod token;
pub mod vm;

pub use chunk::Chunk;
pub use vm::Vm;

pub fn compile(src: &str) -> Result<chunk::Chunk, error::CompileError> {
    let tokens = lexer::Lexer::new(src)
        .tokenize()
        .map_err(|e| error::CompileError::new(e.line, e.message))?;

    let script = parser::Parser::new(tokens).parse_script()?;
    let resolved = resolver::Resolver::resolve(&script)?;

    emitter::Emitter::emit_script(&resolved)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        error::NativeError,
        native::{self, ExecContext, Host, Value, specs::SingleSelectSpec},
    };

    struct TestHost {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl Host for TestHost {
        async fn send_text(&self, text: &str, _: bool) -> Result<(), NativeError> {
            self.calls.lock().unwrap().push(text.to_owned());
            Ok(())
        }
        async fn send_reaction(&self, _: &str) -> Result<(), NativeError> {
            Ok(())
        }
        async fn send_single_select(&self, _: SingleSelectSpec<'_>) -> Result<(), NativeError> {
            Ok(())
        }
        fn is_group(&self) -> bool {
            false
        }
        fn message_text(&self) -> Option<&str> {
            None
        }
        fn sender(&self) -> &str {
            ""
        }
    }

    #[tokio::test]
    async fn send_text_calls_host() {
        let calls = Arc::new(Mutex::new(Vec::new()));

        let host = TestHost {
            calls: calls.clone(),
        };

        let ctx = ExecContext::new(Vec::new(), host);

        let result = native::send::send_text(&[Value::Str("hello".into())], &ctx)
            .await
            .unwrap();

        assert_eq!(result, Value::Nil);
        assert_eq!(calls.lock().unwrap().as_slice(), ["hello"]);
    }

    #[tokio::test]
    async fn send_text_requires_string() {
        let host = TestHost {
            calls: Arc::new(Mutex::new(Vec::new())),
        };

        let ctx = ExecContext::new(Vec::new(), host);

        let err = native::send::send_text(&[Value::Nil], &ctx)
            .await
            .unwrap_err();

        assert!(err.to_string().contains("need str"));
    }
}
