use std::borrow::Cow;

use crate::token::Token;

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: u32,
    interp_stack: Vec<u32>,
}

#[derive(Debug)]
pub struct LexError {
    pub line: u32,
    pub message: String,
}

impl<'a> Lexer<'a> {
    #[inline]
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            line: 1,
            interp_stack: Vec::new(),
        }
    }

    #[inline(always)]
    fn peek_byte(&mut self) -> Option<u8> {
        self.src.as_bytes().get(self.pos).copied()
    }

    #[inline(always)]
    fn bump_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.pos += 1;
        if byte == b'\n' {
            self.line += 1;
        }

        Some(byte)
    }

    #[inline]
    fn bump_char(&mut self) -> Option<char> {
        let c = self.src[self.pos..].chars().next()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
        }

        Some(c)
    }

    #[inline(always)]
    fn slice(&self, start: usize) -> &'a str {
        &self.src[start..self.pos]
    }

    #[inline]
    fn error(&self, line: u32, message: impl Into<String>) -> LexError {
        LexError {
            line,
            message: message.into(),
        }
    }

    #[inline]
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek_byte() {
                Some(b' ' | b'\t' | b'\r') => {
                    self.pos += 1;
                }
                Some(b'#') => {
                    while let Some(byte) = self.peek_byte() {
                        if byte == b'\n' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn next_token(&mut self) -> Result<(Token<'a>, u32), LexError> {
        self.skip_whitespace_and_comments();

        let start = self.pos;
        let line = self.line;

        let byte = match self.peek_byte() {
            Some(b) => b,
            None => return Ok((Token::EOF, line)),
        };

        match byte {
            b'\n' => {
                self.bump_byte();
                Ok((Token::Newline, self.line))
            }
            b'{' => {
                self.bump_byte();
                if let Some(depth) = self.interp_stack.last_mut() {
                    *depth += 1;
                }
                Ok((Token::LBrace, line))
            }
            b'}' => {
                self.bump_byte();
                if let Some(depth) = self.interp_stack.last_mut() {
                    *depth -= 1;
                    if *depth == 0 {
                        self.interp_stack.pop();
                        return self.read_template_string_continue();
                    }
                }
                Ok((Token::RBrace, line))
            }
            b'[' => {
                self.bump_byte();
                Ok((Token::LBracket, line))
            }
            b']' => {
                self.bump_byte();
                Ok((Token::RBracket, line))
            }
            b'(' => {
                self.bump_byte();
                Ok((Token::LParen, line))
            }
            b')' => {
                self.bump_byte();
                Ok((Token::RParen, line))
            }
            b',' => {
                self.bump_byte();
                Ok((Token::Comma, line))
            }
            b'@' => {
                self.bump_byte();
                Ok((Token::At, line))
            }
            b'+' => {
                self.bump_byte();
                Ok((Token::Plus, line))
            }
            b'-' => {
                self.bump_byte();
                Ok((Token::Minus, line))
            }
            b'*' => {
                self.bump_byte();
                Ok((Token::Star, line))
            }
            b'/' => {
                self.bump_byte();
                Ok((Token::Slash, line))
            }
            b'|' => {
                self.bump_byte();
                Ok((Token::Pipe, line))
            }
            b'=' => {
                self.bump_byte();
                if self.peek_byte() == Some(b'=') {
                    self.bump_byte();
                    Ok((Token::Eq, line))
                } else {
                    Ok((Token::Assign, line))
                }
            }
            b'!' => {
                self.bump_byte();
                if self.peek_byte() == Some(b'=') {
                    self.bump_byte();
                    Ok((Token::NotEq, line))
                } else {
                    Ok((Token::Bang, line))
                }
            }
            b'<' => {
                self.bump_byte();
                if self.peek_byte() == Some(b'=') {
                    self.bump_byte();
                    Ok((Token::Le, line))
                } else {
                    Ok((Token::Lt, line))
                }
            }
            b'>' => {
                self.bump_byte();
                if self.peek_byte() == Some(b'=') {
                    self.bump_byte();
                    Ok((Token::Ge, line))
                } else {
                    Ok((Token::Gt, line))
                }
            }
            b':' => {
                self.bump_byte();
                Ok((Token::Colon, line))
            }
            b'.' => {
                self.bump_byte();
                Ok((Token::Dot, line))
            }
            b'"' => {
                self.bump_byte();
                self.read_string(line)
            }
            b'0'..=b'9' => {
                let token = self.read_number(start, line)?;
                Ok((token, line))
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => Ok(self.read_ident(start, line)),
            byte if byte >= 0x80 => {
                let c = self.bump_char().expect("valid UTF-8 source");
                if c.is_alphabetic() {
                    Ok(self.read_unicode_ident(start, line))
                } else {
                    Err(self.error(line, format!("unknown character: {c:?}")))
                }
            }
            other => Err(self.error(line, format!("unknown character: {:?}", other as char))),
        }
    }

    fn read_unicode_ident(&mut self, start: usize, line: u32) -> (Token<'a>, u32) {
        while let Some(c) = self.src[self.pos..].chars().next() {
            if c.is_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }

        let ident = self.slice(start);

        let token = match ident {
            "true" => Token::True,
            "false" => Token::False,
            "and" => Token::And,
            "or" => Token::Or,
            _ => Token::Ident(ident),
        };

        (token, line)
    }

    fn read_template_string_continue(&mut self) -> Result<(Token<'a>, u32), LexError> {
        let line = self.line;
        let start = self.pos;

        let mut owned: Option<String> = None;
        let mut segment_start = start;

        loop {
            match self.bump_char() {
                Some('"') => {
                    let value = match owned {
                        Some(mut value) => {
                            value.push_str(&self.src[segment_start..self.pos - 1]);
                            Cow::Owned(value)
                        }

                        None => Cow::Borrowed(&self.src[start..self.pos - 1]),
                    };

                    return Ok((Token::StrTemplateEnd(value), line));
                }

                Some('$') if self.peek_byte() == Some(b'{') => {
                    let segment_end = self.pos - 1;

                    self.bump_byte();

                    let value = match owned {
                        Some(mut value) => {
                            value.push_str(&self.src[segment_start..segment_end]);
                            Cow::Owned(value)
                        }

                        None => Cow::Borrowed(&self.src[start..segment_end]),
                    };

                    self.interp_stack.push(1);

                    return Ok((Token::StrTemplateMiddle(value), line));
                }

                Some('\\') => {
                    let value = owned.get_or_insert_with(String::new);

                    value.push_str(&self.src[segment_start..self.pos - 1]);

                    match self.bump_char() {
                        Some('n') => value.push('\n'),
                        Some('t') => value.push('\t'),
                        Some('"') => value.push('"'),
                        Some('\\') => value.push('\\'),
                        Some('$') => value.push('$'),

                        Some(c) => value.push(c),

                        None => {
                            return Err(self.error(line, "unclosed string"));
                        }
                    }

                    segment_start = self.pos;
                }

                Some(_) => {}

                None => {
                    return Err(self.error(line, "unclosed string"));
                }
            }
        }
    }

    fn read_number(&mut self, start: usize, line: u32) -> Result<Token<'a>, LexError> {
        let bytes = self.src.as_bytes();

        while matches!(bytes.get(self.pos), Some(b'0'..=b'9')) {
            self.pos += 1;
        }

        let mut is_float = false;

        if bytes.get(self.pos) == Some(&b'.')
            && matches!(bytes.get(self.pos + 1), Some(b'0'..=b'9'))
        {
            is_float = true;
            self.pos += 1;

            while matches!(bytes.get(self.pos), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }

        let value = self.slice(start);

        if is_float {
            value
                .parse::<f64>()
                .map(Token::Float)
                .map_err(|e| self.error(line, format!("invalid float number: {e}")))
        } else {
            value
                .parse::<i64>()
                .map(Token::Int)
                .map_err(|e| self.error(line, format!("invalid integer: {e}")))
        }
    }

    #[inline(always)]
    fn is_ident_continue_ascii(&self, byte: u8) -> bool {
        matches!(
            byte,
            b'a'..=b'z'
                | b'A'..=b'Z'
                | b'0'..=b'9'
                | b'_'
        )
    }

    #[inline]
    fn read_ident(&mut self, start: usize, line: u32) -> (Token<'a>, u32) {
        let bytes = self.src.as_bytes();

        while let Some(&byte) = bytes.get(self.pos) {
            if self.is_ident_continue_ascii(byte) {
                self.pos += 1;
            } else {
                break;
            }
        }

        let ident = self.slice(start);

        let token = match ident {
            "true" => Token::True,
            "false" => Token::False,
            "and" => Token::And,
            "or" => Token::Or,
            _ => Token::Ident(ident),
        };

        (token, line)
    }

    fn read_string(&mut self, line: u32) -> Result<(Token<'a>, u32), LexError> {
        let start = self.pos;
        let mut owned: Option<String> = None;
        let mut segment_start = start;

        loop {
            match self.bump_char() {
                Some('"') => {
                    let value = match owned {
                        Some(mut value) => {
                            value.push_str(&self.src[segment_start..self.pos - 1]);
                            Cow::Owned(value)
                        }

                        None => Cow::Borrowed(&self.src[start..self.pos - 1]),
                    };

                    return Ok((Token::Str(value), line));
                }

                Some('$') if self.peek_byte() == Some(b'{') => {
                    let segment_end = self.pos - 1;

                    self.bump_byte();

                    let value = match owned {
                        Some(mut value) => {
                            value.push_str(&self.src[segment_start..segment_end]);
                            Cow::Owned(value)
                        }

                        None => Cow::Borrowed(&self.src[start..segment_end]),
                    };

                    self.interp_stack.push(1);

                    return Ok((Token::StrTemplateStart(value), line));
                }

                Some('\\') => {
                    let value = owned.get_or_insert_with(String::new);

                    value.push_str(&self.src[segment_start..self.pos - 1]);

                    match self.bump_char() {
                        Some('n') => value.push('\n'),
                        Some('t') => value.push('\t'),
                        Some('"') => value.push('"'),
                        Some('\\') => value.push('\\'),
                        Some('$') => value.push('$'),

                        Some(c) => {
                            value.push(c);
                        }

                        None => {
                            return Err(self.error(line, "unclosed string"));
                        }
                    }

                    segment_start = self.pos;
                }

                Some('\n') => {
                    continue;
                }

                Some(_) => {}

                None => {
                    return Err(self.error(line, "unclosed string"));
                }
            }
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<(Token<'a>, u32)>, LexError> {
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

    fn lex<'a>(src: &'a str) -> Vec<(Token<'a>, u32)> {
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
    fn error_unclosed_string() {
        let err = Lexer::new("\"hello").tokenize().unwrap_err();

        assert_eq!(err.line, 1);
        assert!(err.message.contains("unclosed string"));
    }

    #[test]
    fn object_literal() {
        let tokens = lex(r#"{ name: "viola", age: 20 }"#);

        assert_eq!(
            tokens,
            vec![
                (Token::LBrace, 1),
                (Token::Ident("name"), 1),
                (Token::Colon, 1),
                (Token::Str("viola".into()), 1),
                (Token::Comma, 1),
                (Token::Ident("age"), 1),
                (Token::Colon, 1),
                (Token::Int(20), 1),
                (Token::RBrace, 1),
                (Token::EOF, 1),
            ]
        );
    }

    #[test]
    fn multiline_object_literal() {
        let tokens = lex("{\n  name: \"viola\",\n  age: 20\n}");

        assert_eq!(
            tokens,
            vec![
                (Token::LBrace, 1),
                (Token::Newline, 2),
                (Token::Ident("name"), 2),
                (Token::Colon, 2),
                (Token::Str("viola".into()), 2),
                (Token::Comma, 2),
                (Token::Newline, 3),
                (Token::Ident("age"), 3),
                (Token::Colon, 3),
                (Token::Int(20), 3),
                (Token::Newline, 4),
                (Token::RBrace, 4),
                (Token::EOF, 4),
            ]
        );
    }

    #[test]
    fn string_interpolation_with_sigil() {
        let tokens = lex(r#""author: ${json.result.author}\ncaption: ${json.result.caption}""#);

        assert_eq!(
            tokens,
            vec![
                (Token::StrTemplateStart("author: ".into()), 1),
                (Token::Ident("json"), 1),
                (Token::Dot, 1),
                (Token::Ident("result"), 1),
                (Token::Dot, 1),
                (Token::Ident("author"), 1),
                (Token::StrTemplateMiddle("\ncaption: ".into()), 1),
                (Token::Ident("json"), 1),
                (Token::Dot, 1),
                (Token::Ident("result"), 1),
                (Token::Dot, 1),
                (Token::Ident("caption"), 1),
                (Token::StrTemplateEnd("".into()), 1),
                (Token::EOF, 1),
            ]
        );
    }

    #[test]
    fn string_interpolation_ignores_plain_braces() {
        let tokens = lex(r#""{ this is literal json } ${var}""#);

        assert_eq!(
            tokens,
            vec![
                (
                    Token::StrTemplateStart("{ this is literal json } ".into()),
                    1
                ),
                (Token::Ident("var"), 1),
                (Token::StrTemplateEnd("".into()), 1),
                (Token::EOF, 1),
            ]
        );
    }

    #[test]
    fn string_interpolation_nested_object() {
        let tokens = lex(r#""value is ${ {a: 42}.a }!""#);

        assert_eq!(
            tokens,
            vec![
                (Token::StrTemplateStart("value is ".into()), 1),
                (Token::LBrace, 1),
                (Token::Ident("a"), 1),
                (Token::Colon, 1),
                (Token::Int(42), 1),
                (Token::RBrace, 1),
                (Token::Dot, 1),
                (Token::Ident("a"), 1),
                (Token::StrTemplateEnd("!".into()), 1),
                (Token::EOF, 1),
            ]
        );
    }
}
