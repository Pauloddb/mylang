use crate::lexer::types::Op;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
    Ident(String),
    Func {
        params: Vec<Param>,
        ret_ty: String,
        body: Box<Expr>,
        name: Option<String>,
    },
    Binary {
        op: Op,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: Op,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Property {
        object: Box<Expr>,
        prop: String,
    },
    Assign {
        target: AssignTarget,
        value: Box<Expr>,
    },
    Block(Vec<Stmt>),
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Ident(String),
    Property { object: Box<Expr>, prop: String },
    Index { object: Box<Expr>, index: Box<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    VarDecl {
        name: String,
        type_annotation: Option<String>,
        value: Box<Expr>,
        is_mutable: bool,
    },
    While {
        cond: Box<Expr>,
        body: Box<Stmt>,
    },
    Return(Option<Expr>),
}

#[derive(Debug, Clone)]
pub struct Ast {
    pub stmts: Vec<Stmt>,
}
