use crate::lexer::types::{Op, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64, Span),
    Float(f64, Span),
    String(String, Span),
    Bool(bool, Span),
    Nil(Span),
    Ident(String, Span),
    ArrayLiteral(Vec<Expr>, Span),
    Func {
        params: Vec<Param>,
        ret_ty: String,
        body: Box<Expr>,
        name: Option<String>,
        span: Span,
    },
    Struct {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    Binary {
        op: Op,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Unary {
        op: Op,
        right: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Property {
        object: Box<Expr>,
        prop: String,
        span: Span,
    },
    Assign {
        target: AssignTarget,
        value: Box<Expr>,
        span: Span,
    },
    Block(Vec<Stmt>, Span),
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, span) => *span,
            Expr::Float(_, span) => *span,
            Expr::Bool(_, span) => *span,
            Expr::String(_, span) => *span,
            Expr::Nil(span) => *span,
            Expr::ArrayLiteral(_, span) => *span,
            Expr::Ident(_, span) => *span,
            Expr::Block(_, span) => *span,
            Expr::Unary { span, .. } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::Func { span, .. } => *span,
            Expr::Struct { span, .. } => *span,
            Expr::Property { span, .. } => *span,
            Expr::Assign { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::Index { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Ident(String, Span),
    Property {
        object: Box<Expr>,
        prop: String,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    VarDecl {
        name: String,
        type_annotation: Option<String>,
        value: Box<Expr>,
        is_mutable: bool,
        span: Span,
    },
    StructDecl {
        name: String,
        fields: Vec<(String, String)>,
        span: Span,
    },
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    Return(Option<Expr>, Span),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Expr(expr) => expr.span(),
            Stmt::VarDecl { span, .. } => *span,
            Stmt::While { span, .. } => *span,
            Stmt::Return(_, span) => *span,
            Stmt::StructDecl { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ast {
    pub stmts: Vec<Stmt>,
}
