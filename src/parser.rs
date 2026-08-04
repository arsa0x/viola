use std::rc::Rc;

use crate::{
    ast::{BinOp, Expr, Literal, Script, ScriptMeta, Stmt, UnOp},
    error::CompileError,
    lexer::Token,
};

pub struct Parser<'a> {
    tokens: Vec<(Token<'a>, u16)>,
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<(Token<'a>, u16)>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token<'a> {
        &self.tokens[self.pos].0
    }

    fn line(&self) -> u16 {
        self.tokens[self.pos].1
    }

    fn advance(&mut self) -> Token<'a> {
        let t = self.tokens[self.pos].0.clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

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

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline) {
            self.advance();
        }
    }

    fn parse_meta(&mut self) -> Result<ScriptMeta, CompileError> {
        let mut meta = ScriptMeta::default();

        while self.check(&Token::At) {
            self.advance();

            let key = self.expect_ident("meta name")?;

            match &*key {
                "name" => meta.name = Some(self.expect_ident("script name")?),
                "triggers" => meta.triggers = self.parse_triggers()?,
                other => {
                    return Err(CompileError::new(
                        self.line(),
                        format!("unknown metadata: @{other}"),
                    ));
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
                Token::Ident(s) => triggers.push(Rc::from(s)),
                Token::Pipe => continue,
                Token::Newline | Token::EOF => break,
                other => {
                    return Err(CompileError::new(
                        self.line(),
                        format!("expected trigger name, found {other:?}",),
                    ));
                }
            }
        }

        Ok(triggers)
    }

    fn expect_ident(&mut self, ctx: &str) -> Result<Rc<str>, CompileError> {
        match self.advance() {
            Token::Ident(s) => Ok(Rc::from(s)),
            other => Err(CompileError::new(
                self.line(),
                format!("expected: {ctx}, found: {other:?}"),
            )),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        let line = self.line();
        match self.advance() {
            Token::Int(i) => Ok(Expr::Literal(Literal::Int(i), line)),
            Token::Float(x) => Ok(Expr::Literal(Literal::Float(x), line)),
            Token::Str(s) => Ok(Expr::Literal(Literal::Str(Rc::from(s)), line)),
            Token::True => Ok(Expr::Literal(Literal::Bool(true), line)),
            Token::False => Ok(Expr::Literal(Literal::Bool(false), line)),
            Token::Var(name) => Ok(Expr::Var(Rc::from(name), line)),
            Token::Colon => {
                let cmd_token = self.advance();

                let command: Rc<str> = match cmd_token {
                    Token::Ident(n) => Rc::from(n),
                    _ => {
                        return Err(CompileError::new(
                            line,
                            format!(
                                "expected identifier after ':' for native call, found {cmd_token:?}"
                            ),
                        ));
                    }
                };

                let mut method = None;

                if self.check(&Token::Dot) {
                    self.advance();

                    let mt = self.advance();
                    match mt {
                        Token::Ident(n) => method = Some(Rc::from(n)),
                        _ => {
                            return Err(CompileError::new(
                                self.line(),
                                format!("expected method name after '.', found {mt:?}"),
                            ));
                        }
                    }
                }

                let mut args = Vec::new();

                if self.check(&Token::LParen) {
                    self.consume(&Token::LParen)?;

                    while !self.check(&Token::RParen) {
                        args.push(self.parse_expr()?);
                        if self.check(&Token::Comma) {
                            self.advance();
                        }
                    }

                    self.consume(&Token::RParen)?;
                } else if self.can_start_expr() {
                    args.push(self.parse_expr()?);

                    while self.check(&Token::Comma) {
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
            Token::LParen => {
                let e = self.parse_expr()?;
                self.consume(&Token::RParen)?;
                Ok(e)
            }
            Token::LBrace => self.parse_object(line),
            Token::LBracket => self.parse_array(line),
            other => Err(CompileError::new(
                line,
                format!("invalid expression found {:?}", other),
            )),
        }
    }

    fn can_start_expr(&mut self) -> bool {
        !matches!(
            self.peek(),
            Token::Newline
                | Token::RBrace
                | Token::RParen
                | Token::RBracket
                | Token::Comma
                | Token::EOF
        )
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
            }
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
            _ => self.parse_primary(),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, CompileError> {
        self.parse_equality()
    }

    fn parse_stmt(&mut self) -> Result<Stmt, CompileError> {
        let line = self.line();

        if let Token::Ident(name) = self.peek().clone() {
            if self.tokens.get(self.pos + 1).map(|(t, _)| t) == Some(&Token::Assign) {
                self.advance();
                self.advance();

                let value = self.parse_expr()?;
                return Ok(Stmt::Assign {
                    name: Rc::from(name),
                    value,
                    line,
                });
            }

            if name == "if" {
                return self.parse_if();
            }

            return Err(CompileError::new(
                line,
                format!("unknown statement: `{}`", name),
            ));
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

        let else_blk = if let Token::Ident(kw) = self.peek().clone() {
            if &*kw == "else" {
                self.advance();
                Some(self.parse_block()?)
            } else {
                None
            }
        } else {
            None
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

        let mut stmts = Vec::new();

        while !self.check(&Token::RBrace) {
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }

        self.consume(&Token::RBrace)?;

        Ok(stmts)
    }

    fn parse_array(&mut self, line: u16) -> Result<Expr, CompileError> {
        let mut elements = Vec::new();

        self.skip_newlines();

        while !self.check(&Token::RBracket) {
            elements.push(self.parse_expr()?);
            self.skip_newlines();

            if self.check(&Token::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }

        self.skip_newlines();
        self.consume(&Token::RBracket)?;

        Ok(Expr::Array { elements, line })
    }

    fn parse_object(&mut self, line: u16) -> Result<Expr, CompileError> {
        let mut properties = Vec::new();

        self.skip_newlines();

        while !self.check(&Token::RBrace) {
            self.skip_newlines();

            if self.check(&Token::RBrace) {
                break;
            }

            let key_token = self.advance();
            let key = match key_token {
                Token::Ident(k) => Rc::from(k),
                Token::Str(k) => Rc::from(k),
                _ => {
                    return Err(CompileError::new(
                        self.line(),
                        format!(
                            "expected property name (identifier or string), found {key_token:?}"
                        ),
                    ));
                }
            };

            self.consume(&Token::Colon)?;
            let value = self.parse_expr()?;

            properties.push((key, value));

            if self.check(&Token::Comma) {
                self.advance();
            } else {
                self.skip_newlines();
                break;
            }
        }

        self.consume(&Token::RBrace)?;

        Ok(Expr::Objet { properties, line })
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
    fn parse_variable_expression() {
        let script = parse("$player");

        match &script.body[0] {
            Stmt::ExprStmt { expr, .. } => match expr {
                Expr::Var(name, _) => assert_eq!(name.as_ref(), "player"),
                _ => panic!("expected variable"),
            },
            _ => panic!("expected expression statement"),
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
        let tokens = Lexer::new("hello").tokenize().unwrap();
        let result = Parser::new(tokens).parse_script();

        assert!(result.is_err());
    }
}
