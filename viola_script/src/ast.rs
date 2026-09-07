use std::rc::Rc;

#[derive(Debug, Default, Clone)]
pub struct ScriptMeta {
    pub name: Option<Rc<str>>,
    pub triggers: Vec<Rc<str>>,
    pub category: Option<Rc<str>>,
}

#[derive(Debug)]
pub struct Script {
    pub meta: ScriptMeta,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub enum Stmt {
    Assign {
        name: Rc<str>,
        value: Expr,
        line: u32,
    },
    ExprStmt {
        expr: Expr,
        line: u32,
    },
    If {
        cond: Expr,
        then_blk: Vec<Stmt>,
        else_blk: Option<Vec<Stmt>>,
        line: u32,
    },
}

impl Stmt {
    pub fn line(&self) -> u32 {
        match self {
            Stmt::Assign { line, .. } => *line,
            Stmt::ExprStmt { line, .. } => *line,
            Stmt::If { line, .. } => *line,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal, u32),
    Var(Rc<str>, u32),
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        line: u32,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
        line: u32,
    },
    Array {
        elements: Vec<Expr>,
        line: u32,
    },
    Object {
        properties: Vec<(Rc<str>, Expr)>,
        line: u32,
    },
    PropertyAccess {
        object: Box<Expr>,
        property: Rc<str>,
        line: u32,
    },
    MethodCall {
        object: Box<Expr>,
        method: Rc<str>,
        args: Vec<Expr>,
        line: u32,
    },
    NativeCall {
        command: Rc<str>,
        method: Option<Rc<str>>,
        args: Vec<Expr>,
        line: u32,
    },
    Logical {
        op: LogicalOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        line: u32,
    },
}

impl Expr {
    pub fn line(&self) -> u32 {
        match self {
            Expr::Literal(_, line)
            | Expr::Var(_, line)
            | Expr::Binary { line, .. }
            | Expr::Unary { line, .. }
            | Expr::NativeCall { line, .. }
            | Expr::Array { line, .. }
            | Expr::MethodCall { line, .. }
            | Expr::PropertyAccess { line, .. }
            | Expr::Logical { line, .. }
            | Expr::Object { line, .. } => *line,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Bool(bool),
    Nil,
}
