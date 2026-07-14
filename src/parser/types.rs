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
    Cast {
        object: Box<Expr>,
        target_type: String,
        span: Span,
    },
    Path {
        namespace: Box<Expr>,
        member: String,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, span) => span.clone(),
            Expr::Float(_, span) => span.clone(),
            Expr::Bool(_, span) => span.clone(),
            Expr::String(_, span) => span.clone(),
            Expr::Nil(span) => span.clone(),
            Expr::ArrayLiteral(_, span) => span.clone(),
            Expr::Ident(_, span) => span.clone(),
            Expr::Block(_, span) => span.clone(),
            Expr::Unary { span, .. } => span.clone(),
            Expr::Binary { span, .. } => span.clone(),
            Expr::Call { span, .. } => span.clone(),
            Expr::Func { span, .. } => span.clone(),
            Expr::Struct { span, .. } => span.clone(),
            Expr::Property { span, .. } => span.clone(),
            Expr::Assign { span, .. } => span.clone(),
            Expr::If { span, .. } => span.clone(),
            Expr::Index { span, .. } => span.clone(),
            Expr::Cast { span, .. } => span.clone(),
            Expr::Path { span, .. } => span.clone(),
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
        is_public: bool,
        span: Span,
    },
    StructDecl {
        name: String,
        fields: Vec<(String, String)>,
        is_public: bool,
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
            Stmt::VarDecl { span, .. } => span.clone(),
            Stmt::While { span, .. } => span.clone(),
            Stmt::Return(_, span) => span.clone(),
            Stmt::StructDecl { span, .. } => span.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ast {
    pub stmts: Vec<Stmt>,
}
