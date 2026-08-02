use std::{iter::Peekable, rc::Rc, str::CharIndices};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(Rc<str>),
    Var(Rc<str>),
    Native(Rc<str>),
    Method(Rc<str>),

    Str(Rc<str>),
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
    chars: Peekable<CharIndices<'a>>,
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
            chars: src.char_indices().peekable(),
            line: 1,
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_, c)| c)
    }

    fn bump(&mut self) -> Option<char> {
        self.chars.next().map(|(_, c)| {
            if c == '\n' {
                self.line += 1;
            }
            c
        })
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        loop {
            match self.peek_char() {
                None => return Ok(Token::EOF),
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
                    return Ok(Token::Newline);
                }
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                _ => break,
            }
        }

        let start = self.line;
        let c = match self.bump() {
            Some(c) => c,
            None => return Ok(Token::EOF),
        };

        match c {
            '{' => Ok(Token::LBrace),
            '}' => Ok(Token::RBrace),
            '(' => Ok(Token::LParen),
            ')' => Ok(Token::RParen),
            ',' => Ok(Token::Comma),
            '@' => Ok(Token::At),
            '+' => Ok(Token::Plus),
            '-' => Ok(Token::Minus),
            '*' => Ok(Token::Star),
            '/' => Ok(Token::Slash),
            '|' => Ok(Token::Pipe),
            '=' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    Ok(Token::Eq)
                } else {
                    Ok(Token::Assign)
                }
            }
            '!' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    Ok(Token::NotEq)
                } else {
                    Ok(Token::Bang)
                }
            }
            '<' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    Ok(Token::Le)
                } else {
                    Ok(Token::Lt)
                }
            }
            '>' => {
                if self.peek_char() == Some('=') {
                    self.bump();
                    Ok(Token::Ge)
                } else {
                    Ok(Token::Gt)
                }
            }

            '"' => self.read_string(start),

            ':' => self.read_sig_name(start, Token::Native as fn(Rc<str>) -> Token),
            '$' => self.read_sig_name(start, Token::Var as fn(Rc<str>) -> Token),
            '.' => self.read_sig_name(start, Token::Method as fn(Rc<str>) -> Token),

            c if c.is_alphabetic() || c == '_' => Ok(self.read_ident(c)),
            c if c.is_ascii_digit() => self.read_number(c, start),

            other => Err(LexError {
                line: start,
                message: format!("unknown character: {other:?}"),
            }),
        }
    }

    fn read_number(&mut self, start: char, line: u16) -> Result<Token, LexError> {
        let mut s = String::new();
        s.push(start);
        let mut is_float = false;

        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                s.push(c);
                self.bump();
            } else if c == '.' && !is_float {
                is_float = true;
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }

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

    fn read_sig_name(&mut self, line: u16, wrap: fn(Rc<str>) -> Token) -> Result<Token, LexError> {
        let mut s = String::new();
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }

        if s.is_empty() {
            return Err(LexError {
                line,
                message: "empty name after sigil".into(),
            });
        }

        Ok(wrap(Rc::from(s)))
    }

    fn read_ident(&mut self, start: char) -> Token {
        let mut s = String::new();
        s.push(start);

        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }

        match s.as_str() {
            "true" => Token::True,
            "false" => Token::False,
            _ => Token::Ident(Rc::from(s)),
        }
    }

    fn read_string(&mut self, line: u16) -> Result<Token, LexError> {
        let mut s = String::new();
        loop {
            match self.bump() {
                Some('"') => return Ok(Token::Str(Rc::from(s))),
                Some('\\') => match self.bump() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some(other) => s.push(other),
                    None => {
                        return Err(LexError {
                            line,
                            message: "unclosed string".into(),
                        });
                    }
                },
                Some(c) => s.push(c),
                None => {
                    return Err(LexError {
                        line,
                        message: "unclosed string".into(),
                    });
                }
            }
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<(Token, u16)>, LexError> {
        let mut out = Vec::new();

        loop {
            let token = self.next_token()?;
            let line = self.line;

            let is_eof = token == Token::EOF;

            out.push((token, line));
            if is_eof {
                break;
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn lex(src: &str) -> Vec<(Token, u16)> {
        Lexer::new(src).tokenize().unwrap()
    }

    #[test]
    fn assignment_string() {
        let tokens = lex(r#"name = "viola""#);

        assert_eq!(
            tokens,
            vec![
                (Token::Ident(Rc::from("name")), 1),
                (Token::Assign, 1),
                (Token::Str(Rc::from("viola")), 1),
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
                (Token::Ident(Rc::from("age")), 1),
                (Token::Assign, 1),
                (Token::Int(20), 1),
                (Token::Newline, 2),
                (Token::Ident(Rc::from("pi")), 2),
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
                (Token::Ident(Rc::from("truth")), 1),
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
                (Token::Native(Rc::from("print")), 1),
                (Token::Var(Rc::from("name")), 1),
                (Token::Method(Rc::from("len")), 1),
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
                (Token::Str(Rc::from("hello\n\t\"world\"\\!")), 1),
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
                (Token::Ident(Rc::from("x")), 3),
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

        assert_eq!(tokens[0], (Token::Ident(Rc::from("a")), 1));
        assert_eq!(tokens[1], (Token::Newline, 2));

        assert_eq!(tokens[2], (Token::Ident(Rc::from("b")), 2));
        assert_eq!(tokens[3], (Token::Newline, 3));

        assert_eq!(tokens[4], (Token::Ident(Rc::from("c")), 3));
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
