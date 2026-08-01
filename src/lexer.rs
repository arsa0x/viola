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
            c if c.is_alphabetic() || c == '_' => Ok(self.read_ident(c)),
            '"' => self.read_string(start),
            other => Err(LexError {
                line: start,
                message: format!("unknown character: {other:?}"),
            }),
        }
    }

    fn read_ident(&mut self, c: char) -> Token {
        let mut s = String::new();
        s.push(c);

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

    #[test]
    fn assignment_test() {
        let source = "name = \"viola\"";
        let lex = Lexer::new(source).tokenize();
        assert!(lex.is_ok());

        let tokens = lex.unwrap();

        assert_eq!(tokens[0], (Token::Ident(Rc::from("name")), 1));
        assert_eq!(tokens[1], (Token::Assign, 1));
        assert_eq!(tokens[2], (Token::Str(Rc::from("viola")), 1));
    }
}
