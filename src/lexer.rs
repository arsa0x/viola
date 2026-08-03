use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    Ident(&'a str),
    Var(&'a str),
    Native(&'a str),
    Method(&'a str),

    Str(Cow<'a, str>),
    Int(i64),
    Float(f64),
    True,
    False,

    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Bang,
    Assign,
    Pipe,

    LBrace,
    RBrace,
    LParen,
    RParen,
    Comma,
    At,

    Newline,
    EOF,
}

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: u16,
}

#[derive(Debug)]
pub struct LexError {
    pub line: u16,
    pub message: String,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            line: 1,
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek_char()?;

        self.pos += c.len_utf8();

        if c == '\n' {
            self.line += 1;
        }

        Some(c)
    }

    fn current_slice(&self, start: usize) -> &'a str {
        &self.src[start..self.pos]
    }

    fn next_token(&mut self) -> Result<(Token<'a>, u16), LexError> {
        loop {
            match self.peek_char() {
                None => return Ok((Token::EOF, self.line)),
                Some('#') => {
                    while let Some(c) = self.peek_char() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some('\n') => {
                    self.bump();
                    return Ok((Token::Newline, self.line));
                }
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                _ => break,
            }
        }

        let line = self.line;
        let start = self.pos;

        let c = self.bump().unwrap();

        match c {
            '{' => Ok((Token::LBrace, line)),
            '}' => Ok((Token::RBrace, line)),
            '(' => Ok((Token::LParen, line)),
            ')' => Ok((Token::RParen, line)),
            ',' => Ok((Token::Comma, line)),
            '@' => Ok((Token::At, line)),
            '+' => Ok((Token::Plus, line)),
            '-' => Ok((Token::Minus, line)),
            '*' => Ok((Token::Star, line)),
            '/' => Ok((Token::Slash, line)),
            '|' => Ok((Token::Pipe, line)),
            '=' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    Ok((Token::Eq, line))
                } else {
                    Ok((Token::Assign, line))
                }
            }
            '!' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    Ok((Token::NotEq, line))
                } else {
                    Ok((Token::Bang, line))
                }
            }
            '<' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    Ok((Token::Le, line))
                } else {
                    Ok((Token::Lt, line))
                }
            }
            '>' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    Ok((Token::Ge, line))
                } else {
                    Ok((Token::Gt, line))
                }
            }

            '"' => self.read_string(line),

            ':' => self.read_sig_name(start + 1, line, Token::Native),
            '$' => self.read_sig_name(start + 1, line, Token::Var),
            '.' => self.read_sig_name(start + 1, line, Token::Method),

            c if c.is_alphabetic() || c == '_' => Ok(self.read_ident(start)),
            c if c.is_ascii_digit() => Ok((self.read_number(start, line)?, self.line)),

            other => Err(LexError {
                line: line,
                message: format!("unknown character: {other:?}"),
            }),
        }
    }

    fn read_number(&mut self, start: usize, line: u16) -> Result<Token<'a>, LexError> {
        let mut is_float = false;

        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                self.bump();
            } else if c == '.' && !is_float {
                is_float = true;
                self.bump();
            } else {
                break;
            }
        }

        let s = self.current_slice(start);

        if is_float {
            s.parse::<f64>().map(Token::Float).map_err(|e| LexError {
                line,
                message: format!("invalid float number: {e}"),
            })
        } else {
            s.parse::<i64>().map(Token::Int).map_err(|e| LexError {
                line,
                message: format!("invalid int number: {e}"),
            })
        }
    }

    fn read_sig_name(
        &mut self,
        start: usize,
        line: u16,
        wrap: fn(&'a str) -> Token<'a>,
    ) -> Result<(Token<'a>, u16), LexError> {
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                self.bump();
            } else {
                break;
            }
        }

        let name = self.current_slice(start);

        if name.is_empty() {
            return Err(LexError {
                line,
                message: "empty name after sigil".into(),
            });
        }

        Ok((wrap(name), self.line))
    }

    fn read_ident(&mut self, start: usize) -> (Token<'a>, u16) {
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                self.bump();
            } else {
                break;
            }
        }

        let ident = self.current_slice(start);

        match ident {
            "true" => (Token::True, self.line),
            "false" => (Token::False, self.line),
            _ => (Token::Ident(ident), self.line),
        }
    }

    fn read_string(&mut self, line: u16) -> Result<(Token<'a>, u16), LexError> {
        let start = self.pos;

        let mut owned: Option<String> = None;
        let mut segment_start = start;

        loop {
            match self.bump() {
                Some('"') => {
                    return if let Some(mut s) = owned {
                        s.push_str(&self.src[segment_start..self.pos - 1]);
                        Ok((Token::Str(Cow::Owned(s)), self.line))
                    } else {
                        Ok((
                            Token::Str(Cow::Borrowed(&self.src[start..self.pos - 1])),
                            line,
                        ))
                    };
                }

                Some('\\') => {
                    let s = owned.get_or_insert_with(String::new);
                    s.push_str(&self.src[segment_start..self.pos - 1]);

                    match self.bump() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some(c) => s.push(c),
                        None => {
                            return Err(LexError {
                                line,
                                message: "unclosed string".into(),
                            });
                        }
                    }

                    segment_start = self.pos;
                }

                Some(_) => {}

                None => {
                    return Err(LexError {
                        line,
                        message: "unclosed string".into(),
                    });
                }
            }
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<(Token<'a>, u16)>, LexError> {
        let mut tokens = Vec::new();

        loop {
            let (token, line) = self.next_token()?;
            let eof = token == Token::EOF;

            tokens.push((token, line));

            if eof {
                break;
            }
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex<'a>(src: &'a str) -> Vec<(Token<'a>, u16)> {
        Lexer::new(src).tokenize().unwrap()
    }

    #[test]
    fn assignment_string() {
        let tokens = lex(r#"name = "viola""#);

        assert_eq!(
            tokens,
            vec![
                (Token::Ident("name"), 1),
                (Token::Assign, 1),
                (Token::Str("viola".into()), 1),
                (Token::EOF, 1),
            ]
        );
    }

    #[test]
    fn integer_and_float() {
        let tokens = lex("age = 20\npi = 3.14");

        assert_eq!(
            tokens,
            vec![
                (Token::Ident("age"), 1),
                (Token::Assign, 1),
                (Token::Int(20), 1),
                (Token::Newline, 2),
                (Token::Ident("pi"), 2),
                (Token::Assign, 2),
                (Token::Float(3.14), 2),
                (Token::EOF, 2),
            ]
        );
    }

    #[test]
    fn bool_literals() {
        let tokens = lex("true false truth");

        assert_eq!(
            tokens,
            vec![
                (Token::True, 1),
                (Token::False, 1),
                (Token::Ident("truth"), 1),
                (Token::EOF, 1),
            ]
        );
    }

    #[test]
    fn operators() {
        let tokens = lex("== != <= >= < > + - * / ! =");

        assert_eq!(
            tokens,
            vec![
                (Token::Eq, 1),
                (Token::NotEq, 1),
                (Token::Le, 1),
                (Token::Ge, 1),
                (Token::Lt, 1),
                (Token::Gt, 1),
                (Token::Plus, 1),
                (Token::Minus, 1),
                (Token::Star, 1),
                (Token::Slash, 1),
                (Token::Bang, 1),
                (Token::Assign, 1),
                (Token::EOF, 1),
            ]
        );
    }

    #[test]
    fn punctuation() {
        let tokens = lex("{ } ( ) , @");

        assert_eq!(
            tokens,
            vec![
                (Token::LBrace, 1),
                (Token::RBrace, 1),
                (Token::LParen, 1),
                (Token::RParen, 1),
                (Token::Comma, 1),
                (Token::At, 1),
                (Token::EOF, 1),
            ]
        );
    }

    #[test]
    fn meta() {
        let tokens = lex("@triggers meta|test|mt");

        assert_eq!(
            tokens,
            vec![
                (Token::At, 1),
                (Token::Ident("triggers".into()), 1),
                (Token::Ident("meta".into()), 1),
                (Token::Pipe, 1),
                (Token::Ident("test".into()), 1),
                (Token::Pipe, 1),
                (Token::Ident("mt".into()), 1),
                (Token::EOF, 1),
            ]
        )
    }

    #[test]
    fn sigils() {
        let tokens = lex(":print $name .len");

        assert_eq!(
            tokens,
            vec![
                (Token::Native("print"), 1),
                (Token::Var("name"), 1),
                (Token::Method("len"), 1),
                (Token::EOF, 1),
            ]
        );
    }

    #[test]
    fn string_escape_sequences() {
        let tokens = lex(r#""hello\n\t\"world\"\\!""#);

        assert_eq!(
            tokens,
            vec![
                (Token::Str("hello\n\t\"world\"\\!".into()), 1),
                (Token::EOF, 1),
            ]
        );
    }

    #[test]
    fn comments_are_ignored() {
        let tokens = lex(r#"
# this is comment
x = 1 # another comment
"#);

        assert_eq!(
            tokens,
            vec![
                (Token::Newline, 2),
                (Token::Newline, 3),
                (Token::Ident("x"), 3),
                (Token::Assign, 3),
                (Token::Int(1), 3),
                (Token::Newline, 4),
                (Token::EOF, 4),
            ]
        );
    }

    #[test]
    fn line_numbers() {
        let tokens = lex("a\nb\nc");

        assert_eq!(tokens[0], (Token::Ident("a"), 1));
        assert_eq!(tokens[1], (Token::Newline, 2));

        assert_eq!(tokens[2], (Token::Ident("b"), 2));
        assert_eq!(tokens[3], (Token::Newline, 3));

        assert_eq!(tokens[4], (Token::Ident("c"), 3));
        assert_eq!(tokens[5], (Token::EOF, 3));
    }

    #[test]
    fn error_unknown_character() {
        let err = Lexer::new("^").tokenize().unwrap_err();

        assert_eq!(err.line, 1);
        assert!(err.message.contains("unknown character"));
    }

    #[test]
    fn error_empty_sigil_name() {
        let err = Lexer::new("$ ").tokenize().unwrap_err();

        assert_eq!(err.line, 1);
        assert!(err.message.contains("empty name"));
    }

    #[test]
    fn error_unclosed_string() {
        let err = Lexer::new("\"hello").tokenize().unwrap_err();

        assert_eq!(err.line, 1);
        assert!(err.message.contains("unclosed string"));
    }
}
