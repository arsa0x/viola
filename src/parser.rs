use std::rc::Rc;

use crate::{
    ast::{BinOp, Expr, Literal, Script, ScriptMeta, Stmt, UnOp},
    error::CompileError,
    token::Token,
};

/// Recursive-descent parser for the language.
///
/// The parser consumes the token stream produced by the lexer and builds
/// an [`Script`] AST.
///
/// The expression parser is organized by precedence:
///
/// ```text
/// expression
///   └── equality
///        └── comparison
///             └── additive
///                  └── multiplicative
///                       └── unary
///                            └── postfix
///                                 └── primary
/// ```
///
/// Postfix expressions are parsed iteratively, allowing constructs such as:
///
/// ```text
/// user.name
/// user.name.first
/// user.profile.name.first
/// ```
///
/// without recursive calls for every property access.
pub struct Parser<'a> {
    tokens: Vec<(Token<'a>, u32)>,
    pos: usize,
}

impl<'a> Parser<'a> {
    #[inline]
    pub fn new(tokens: Vec<(Token<'a>, u32)>) -> Self {
        Self { tokens, pos: 0 }
    }

    #[inline(always)]
    fn line(&self) -> u32 {
        self.tokens[self.pos].1
    }

    #[inline(always)]
    fn peek(&self) -> &Token<'a> {
        &self.tokens[self.pos].0
    }

    #[inline(always)]
    fn peek_at(&self, offset: usize) -> Option<&Token<'a>> {
        self.tokens.get(self.pos + offset).map(|(token, _)| token)
    }

    #[inline]
    fn advance(&mut self) -> Token<'a> {
        let token = std::mem::replace(&mut self.tokens[self.pos].0, Token::EOF);

        if !matches!(token, Token::EOF) && self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }

        token
    }

    #[inline]
    fn check(&self, t: &Token<'a>) -> bool {
        self.peek() == t
    }

    fn consume(&mut self, t: &Token<'a>) -> Result<(), CompileError> {
        if self.check(t) {
            self.advance();
            Ok(())
        } else {
            Err(CompileError::new(
                self.line(),
                format!("expected: {t:?}, found: {:?}", self.peek()),
            ))
        }
    }

    #[inline]
    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline) {
            self.pos += 1;
        }
    }

    #[inline]
    fn error(&self, message: impl Into<String>) -> CompileError {
        CompileError::new(self.line(), message.into())
    }

    fn parse_meta(&mut self) -> Result<ScriptMeta, CompileError> {
        let mut meta = ScriptMeta::default();

        while matches!(self.peek(), Token::At) {
            self.advance();

            let key = self.expect_ident("metadata name")?;

            match key.as_ref() {
                "name" => {
                    meta.name = Some(self.expect_ident("script name")?);
                }

                "triggers" => {
                    meta.triggers = self.parse_triggers()?;
                }

                other => {
                    return Err(self.error(format!("unknown metadata: @{other}")));
                }
            }

            self.skip_newlines();
        }

        Ok(meta)
    }

    fn parse_triggers(&mut self) -> Result<Vec<Rc<str>>, CompileError> {
        let mut triggers = Vec::new();

        loop {
            match self.advance() {
                Token::Ident(name) => {
                    triggers.push(Rc::from(name));
                }

                Token::Pipe => {}

                Token::Newline | Token::EOF => {
                    break;
                }

                other => {
                    return Err(self.error(format!("expected trigger name, found {other:?}")));
                }
            }
        }

        Ok(triggers)
    }

    #[inline]
    fn expect_ident(&mut self, context: &str) -> Result<Rc<str>, CompileError> {
        match self.advance() {
            Token::Ident(name) => Ok(Rc::from(name)),

            other => Err(self.error(format!("expected {context}, found {other:?}"))),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        let line = self.line();

        match self.advance() {
            Token::Ident(name) => Ok(Expr::Var(Rc::from(name), line)),

            Token::Int(value) => Ok(Expr::Literal(Literal::Int(value), line)),

            Token::Float(value) => Ok(Expr::Literal(Literal::Float(value), line)),

            Token::Str(value) => Ok(Expr::Literal(Literal::Str(Rc::from(value)), line)),

            Token::True => Ok(Expr::Literal(Literal::Bool(true), line)),

            Token::False => Ok(Expr::Literal(Literal::Bool(false), line)),

            Token::LParen => {
                let expr = self.parse_expr()?;

                self.consume(&Token::RParen)?;

                Ok(expr)
            }

            Token::LBrace => self.parse_object(line),

            Token::LBracket => self.parse_array(line),

            Token::Colon => self.parse_native_call(line),

            other => Err(self.error(format!("invalid expression found {other:?}"))),
        }
    }

    fn parse_native_call(&mut self, line: u32) -> Result<Expr, CompileError> {
        let command = match self.advance() {
            Token::Ident(name) => Rc::from(name),

            other => {
                return Err(self.error(format!("expected identifier after ':', found {other:?}")));
            }
        };

        let mut method = None;

        if matches!(self.peek(), Token::Dot) {
            self.advance();

            method = Some(match self.advance() {
                Token::Ident(name) => Rc::from(name),

                other => {
                    return Err(
                        self.error(format!("expected method name after '.', found {other:?}"))
                    );
                }
            });
        }

        let mut args = Vec::new();

        if matches!(self.peek(), Token::LParen) {
            self.advance();

            if !matches!(self.peek(), Token::RParen) {
                loop {
                    args.push(self.parse_expr()?);

                    if !matches!(self.peek(), Token::Comma) {
                        break;
                    }

                    self.advance();

                    if matches!(self.peek(), Token::RParen) {
                        break;
                    }
                }
            }

            self.consume(&Token::RParen)?;
        } else if self.can_start_expr() {
            args.push(self.parse_expr()?);

            while matches!(self.peek(), Token::Comma) {
                self.advance();
                args.push(self.parse_expr()?);
            }
        }

        Ok(Expr::NativeCall {
            command,
            method,
            args,
            line,
        })
    }

    #[inline]
    fn can_start_expr(&self) -> bool {
        matches!(
            self.peek(),
            Token::Ident(_)
                | Token::Int(_)
                | Token::Float(_)
                | Token::Str(_)
                | Token::True
                | Token::False
                | Token::LParen
                | Token::LBracket
                | Token::Colon
                | Token::Bang
                | Token::Minus
        )
    }

    fn parse_postfix(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Token::Dot => {
                    let line = self.line();

                    self.advance();

                    let name = self.expect_ident("property or method name")?;

                    if matches!(self.peek(), Token::LParen) {
                        self.advance();

                        let mut args = Vec::new();

                        if !matches!(self.peek(), Token::RParen) {
                            loop {
                                args.push(self.parse_expr()?);

                                if !matches!(self.peek(), Token::Comma) {
                                    break;
                                }

                                self.advance();

                                if matches!(self.peek(), Token::RParen) {
                                    break;
                                }
                            }
                        }

                        self.consume(&Token::RParen)?;

                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method: name,
                            args,
                            line,
                        };
                    } else {
                        expr = Expr::PropertyAccess {
                            object: Box::new(expr),
                            property: name,
                            line,
                        };
                    }
                }

                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, CompileError> {
        let line = self.line();

        match self.peek() {
            Token::Bang => {
                self.advance();

                let expr = self.parse_unary()?;

                Ok(Expr::Unary {
                    op: UnOp::Not,
                    expr: Box::new(expr),
                    line,
                })
            }

            Token::Minus => {
                self.advance();

                let expr = self.parse_unary()?;

                Ok(Expr::Unary {
                    op: UnOp::Neg,
                    expr: Box::new(expr),
                    line,
                })
            }

            _ => self.parse_postfix(),
        }
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, CompileError> {
        let mut lhs = self.parse_unary()?;

        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                _ => break,
            };

            let line = self.line();

            self.advance();

            let rhs = self.parse_unary()?;

            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                line,
            };
        }

        Ok(lhs)
    }

    fn parse_additive(&mut self) -> Result<Expr, CompileError> {
        let mut lhs = self.parse_multiplicative()?;

        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };

            let line = self.line();

            self.advance();

            let rhs = self.parse_multiplicative()?;

            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                line,
            };
        }

        Ok(lhs)
    }

    fn parse_comparison(&mut self) -> Result<Expr, CompileError> {
        let mut lhs = self.parse_additive()?;

        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Le => BinOp::Le,
                Token::Gt => BinOp::Gt,
                Token::Ge => BinOp::Ge,
                _ => break,
            };

            let line = self.line();

            self.advance();

            let rhs = self.parse_additive()?;

            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                line,
            };
        }

        Ok(lhs)
    }

    fn parse_equality(&mut self) -> Result<Expr, CompileError> {
        let mut lhs = self.parse_comparison()?;

        loop {
            let op = match self.peek() {
                Token::Eq => BinOp::Eq,
                Token::NotEq => BinOp::NotEq,
                _ => break,
            };

            let line = self.line();

            self.advance();

            let rhs = self.parse_comparison()?;

            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                line,
            };
        }

        Ok(lhs)
    }

    #[inline]
    fn parse_expr(&mut self) -> Result<Expr, CompileError> {
        self.parse_equality()
    }

    fn parse_stmt(&mut self) -> Result<Stmt, CompileError> {
        let line = self.line();

        if let Token::Ident(name) = self.peek() {
            if matches!(self.peek_at(1), Some(Token::Assign)) {
                let name = Rc::from(*name);

                self.advance();
                self.advance();

                let value = self.parse_expr()?;

                return Ok(Stmt::Assign { name, value, line });
            }

            if *name == "if" {
                return self.parse_if();
            }
        }

        let expr = self.parse_expr()?;

        Ok(Stmt::ExprStmt { expr, line })
    }

    fn parse_if(&mut self) -> Result<Stmt, CompileError> {
        let line = self.line();

        self.advance();

        let cond = self.parse_expr()?;

        let then_blk = self.parse_block()?;

        self.skip_newlines();

        let else_blk = match self.peek() {
            Token::Ident(name) if *name == "else" => {
                self.advance();

                Some(self.parse_block()?)
            }

            _ => None,
        };

        Ok(Stmt::If {
            cond,
            then_blk,
            else_blk,
            line,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, CompileError> {
        self.consume(&Token::LBrace)?;

        self.skip_newlines();

        let mut statements = Vec::new();

        while !matches!(self.peek(), Token::RBrace | Token::EOF) {
            statements.push(self.parse_stmt()?);

            self.skip_newlines();
        }

        self.consume(&Token::RBrace)?;

        Ok(statements)
    }

    fn parse_array(&mut self, line: u32) -> Result<Expr, CompileError> {
        let mut elements = Vec::new();

        self.skip_newlines();

        if matches!(self.peek(), Token::RBracket) {
            self.advance();

            return Ok(Expr::Array { elements, line });
        }

        loop {
            elements.push(self.parse_expr()?);

            self.skip_newlines();

            if !matches!(self.peek(), Token::Comma) {
                break;
            }

            self.advance();

            self.skip_newlines();

            if matches!(self.peek(), Token::RBracket) {
                break;
            }
        }

        self.skip_newlines();

        self.consume(&Token::RBracket)?;

        Ok(Expr::Array { elements, line })
    }

    fn parse_object(&mut self, line: u32) -> Result<Expr, CompileError> {
        let mut properties = Vec::new();

        self.skip_newlines();

        if matches!(self.peek(), Token::RBrace) {
            self.advance();

            return Ok(Expr::Object { properties, line });
        }

        loop {
            self.skip_newlines();

            let key = match self.advance() {
                Token::Ident(key) => Rc::from(key),

                Token::Str(key) => Rc::from(key),

                other => {
                    return Err(self.error(format!("expected property name, found {other:?}")));
                }
            };

            self.consume(&Token::Colon)?;

            let value = self.parse_expr()?;

            properties.push((key, value));

            self.skip_newlines();

            if !matches!(self.peek(), Token::Comma) {
                break;
            }

            self.advance();

            self.skip_newlines();

            if matches!(self.peek(), Token::RBrace) {
                break;
            }
        }

        self.skip_newlines();

        self.consume(&Token::RBrace)?;

        Ok(Expr::Object { properties, line })
    }

    pub fn parse_script(&mut self) -> Result<Script, CompileError> {
        self.skip_newlines();

        let meta = self.parse_meta()?;

        self.skip_newlines();

        let mut body = Vec::new();

        while !matches!(self.peek(), Token::EOF) {
            body.push(self.parse_stmt()?);

            self.skip_newlines();
        }

        Ok(Script { meta, body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BinOp, Expr, Literal, Stmt};
    use crate::lexer::Lexer;
    use std::rc::Rc;

    fn parse(src: &str) -> Script {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_script().unwrap()
    }

    #[test]
    fn parse_integer_assignment() {
        let script = parse("x = 10");

        assert_eq!(script.body.len(), 1);

        match &script.body[0] {
            Stmt::Assign { name, value, .. } => {
                assert_eq!(name.as_ref(), "x");

                match value {
                    Expr::Literal(Literal::Int(10), _) => {}
                    other => panic!("expected integer literal, got {:?}", other),
                }
            }
            other => panic!("expected assignment, got {:?}", other),
        }
    }

    #[test]
    fn parse_string_assignment() {
        let script = parse(r#"name = "viola""#);

        match &script.body[0] {
            Stmt::Assign { value, .. } => match value {
                Expr::Literal(Literal::Str(s), _) => {
                    assert_eq!(s.as_ref(), "viola");
                }
                _ => panic!("expected string literal"),
            },
            _ => panic!("expected assignment"),
        }
    }

    #[test]
    fn parse_boolean_assignment() {
        let script = parse("flag = true");

        match &script.body[0] {
            Stmt::Assign { value, .. } => {
                assert!(matches!(value, Expr::Literal(Literal::Bool(true), _)));
            }
            _ => panic!("expected assignment"),
        }
    }

    #[test]
    fn parse_operator_precedence() {
        let script = parse("x = 1 + 2 * 3");

        let expr = match &script.body[0] {
            Stmt::Assign { value, .. } => value,
            _ => panic!(),
        };

        match expr {
            Expr::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
                ..
            } => {
                assert!(matches!(**lhs, Expr::Literal(Literal::Int(1), _)));

                match &**rhs {
                    Expr::Binary {
                        op: BinOp::Mul,
                        lhs,
                        rhs,
                        ..
                    } => {
                        assert!(matches!(**lhs, Expr::Literal(Literal::Int(2), _)));
                        assert!(matches!(**rhs, Expr::Literal(Literal::Int(3), _)));
                    }
                    _ => panic!("expected multiplication"),
                }
            }
            _ => panic!("expected addition"),
        }
    }

    #[test]
    fn parse_parentheses() {
        let script = parse("x = (1 + 2) * 3");

        let expr = match &script.body[0] {
            Stmt::Assign { value, .. } => value,
            _ => panic!(),
        };

        match expr {
            Expr::Binary {
                op: BinOp::Mul,
                lhs,
                rhs,
                ..
            } => {
                match &**lhs {
                    Expr::Binary { op: BinOp::Add, .. } => {}
                    _ => panic!("expected addition inside parentheses"),
                }

                assert!(matches!(**rhs, Expr::Literal(Literal::Int(3), _)));
            }
            _ => panic!("expected multiplication"),
        }
    }

    #[test]
    fn parse_unary_minus() {
        let script = parse("x = -10");

        let expr = match &script.body[0] {
            Stmt::Assign { value, .. } => value,
            _ => panic!(),
        };

        match expr {
            Expr::Unary { op: UnOp::Neg, .. } => {}
            _ => panic!("expected unary negation"),
        }
    }

    #[test]
    fn parse_unary_not() {
        let script = parse("x = !false");

        let expr = match &script.body[0] {
            Stmt::Assign { value, .. } => value,
            _ => panic!(),
        };

        match expr {
            Expr::Unary { op: UnOp::Not, .. } => {}
            _ => panic!("expected unary not"),
        }
    }

    #[test]
    fn parse_metadata() {
        let script = parse(
            r#"@name TestScript
@triggers hello|helo|hi

x = 1
"#,
        );

        assert_eq!(script.meta.name, Some(Rc::from("TestScript")));
        assert_eq!(script.meta.triggers.len(), 3);
        assert_eq!(script.meta.triggers[0].as_ref(), "hello");
        assert_eq!(script.meta.triggers[1].as_ref(), "helo");
        assert_eq!(script.meta.triggers[2].as_ref(), "hi");
    }

    #[test]
    fn parse_if_statement() {
        let script = parse(
            r#"
    if true {
        x = 1
    }
    "#,
        );

        assert_eq!(script.body.len(), 1);

        match &script.body[0] {
            Stmt::If {
                cond,
                then_blk,
                else_blk,
                ..
            } => {
                assert!(matches!(cond, Expr::Literal(Literal::Bool(true), _)));

                assert!(else_blk.is_none());
                assert_eq!(then_blk.len(), 1);

                match &then_blk[0] {
                    Stmt::Assign { name, value, .. } => {
                        assert_eq!(name.as_ref(), "x");
                        assert!(matches!(value, Expr::Literal(Literal::Int(1), _)));
                    }
                    _ => panic!("expected assignment"),
                }
            }
            other => panic!("expected if statement, got {:?}", other),
        }
    }

    #[test]
    fn parse_native_without_method() {
        let script = parse(":send");

        match &script.body[0] {
            Stmt::ExprStmt { expr, .. } => match expr {
                Expr::NativeCall {
                    command,
                    method,
                    args,
                    ..
                } => {
                    assert_eq!(command.as_ref(), "send");
                    assert!(method.is_none());
                    assert!(args.is_empty());
                }
                _ => panic!("expected native call"),
            },
            _ => panic!("expected expression statement"),
        }
    }

    #[test]
    fn parse_native_with_method() {
        let script = parse(":args .at 0");

        match &script.body[0] {
            Stmt::ExprStmt { expr, .. } => match expr {
                Expr::NativeCall {
                    command,
                    method,
                    args,
                    ..
                } => {
                    assert_eq!(command.as_ref(), "args");
                    assert!(method.is_some());

                    assert_eq!(args.len(), 1);

                    assert!(matches!(
                        &args[0],
                        Expr::Literal(Literal::Int(n), _) if *n == 0
                    ));
                }
                _ => panic!("expected native call"),
            },
            _ => panic!("expected expression statement"),
        }
    }

    #[test]
    fn parse_empty_script() {
        let script = parse("");

        assert!(script.body.is_empty());
    }

    #[test]
    fn invalid_expression_returns_error() {
        let tokens = Lexer::new("x = )").tokenize().unwrap();
        let result = Parser::new(tokens).parse_script();

        assert!(result.is_err());
    }

    #[test]
    fn unknown_statement_returns_error() {
        let tokens = Lexer::new("@unknown").tokenize().unwrap();
        let result = Parser::new(tokens).parse_script();

        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_object() {
        let script = parse("user = {}");

        match &script.body[0] {
            Stmt::Assign { value, .. } => match value {
                Expr::Object { properties, .. } => {
                    assert!(properties.is_empty(), "Object should have no properties");
                }
                other => panic!("expected object expression, got {:?}", other),
            },
            _ => panic!("expected assignment"),
        }
    }

    #[test]
    fn parse_simple_object() {
        let script = parse(r#"user = { name: "viola", age: 18 }"#);

        match &script.body[0] {
            Stmt::Assign { value, .. } => match value {
                Expr::Object { properties, .. } => {
                    assert_eq!(properties.len(), 2);

                    assert_eq!(properties[0].0.as_ref(), "name");
                    assert!(matches!(
                        &properties[0].1,
                        Expr::Literal(Literal::Str(s), _) if s.as_ref() == "viola"
                    ));

                    assert_eq!(properties[1].0.as_ref(), "age");
                    assert!(matches!(
                        properties[1].1,
                        Expr::Literal(Literal::Int(18), _)
                    ));
                }
                other => panic!("expected object expression, got {:?}", other),
            },
            _ => panic!("expected assignment"),
        }
    }

    #[test]
    fn parse_nested_object() {
        let script = parse(r#"data = { user: { id: 1 } }"#);

        match &script.body[0] {
            Stmt::Assign { value, .. } => match value {
                Expr::Object { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_eq!(properties[0].0.as_ref(), "user");

                    match &properties[0].1 {
                        Expr::Object {
                            properties: inner_props,
                            ..
                        } => {
                            assert_eq!(inner_props.len(), 1);
                            assert_eq!(inner_props[0].0.as_ref(), "id");
                            assert!(matches!(
                                inner_props[0].1,
                                Expr::Literal(Literal::Int(1), _)
                            ));
                        }
                        other => panic!("expected inner object, got {:?}", other),
                    }
                }
                other => panic!("expected object expression, got {:?}", other),
            },
            _ => panic!("expected assignment"),
        }
    }

    #[test]
    fn parse_property_access() {
        let script = parse(
            r#"user = { name: "viola", age: 18 }
name = user.name"#,
        );

        match &script.body[1] {
            Stmt::Assign { value, .. } => match value {
                Expr::PropertyAccess {
                    object, property, ..
                } => {
                    assert_eq!(property.as_ref(), "name");

                    match &**object {
                        Expr::Var(var_name, _) => assert_eq!(var_name.as_ref(), "user"),
                        other => panic!("expected variable object, got {:?}", other),
                    }
                }
                other => panic!("expected property access expression, got {:?}", other),
            },
            _ => panic!("expected assignment"),
        }
    }

    #[test]
    fn parse_chained_property_access() {
        let script = parse("first_name = user.name.first");

        match &script.body[0] {
            Stmt::Assign { value, .. } => match value {
                Expr::PropertyAccess {
                    object, property, ..
                } => {
                    assert_eq!(property.as_ref(), "first");

                    match &**object {
                        Expr::PropertyAccess {
                            object: inner_obj,
                            property: inner_prop,
                            ..
                        } => {
                            assert_eq!(inner_prop.as_ref(), "name");

                            match &**inner_obj {
                                Expr::Var(var_name, _) => assert_eq!(var_name.as_ref(), "user"),
                                other => {
                                    panic!("expected variable for inner object, got {:?}", other)
                                }
                            }
                        }
                        other => panic!("expected inner property access, got {:?}", other),
                    }
                }
                other => panic!("expected property access expression, got {:?}", other),
            },
            _ => panic!("expected assignment"),
        }
    }
}
