use thiserror::Error;

use crate::lexer::types::Span;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("[{span}] undefined variable: {name}")]
    UndefinedVar { name: String, span: Span },

    #[error("[{span}] immutable mutation: cannot mutate immutable variable `{name}`")]
    ImmutableMutation { name: String, span: Span },

    #[error("[{span}] invalid cast: cannot cast `{from}` to `{to}`")]
    InvalidCast {
        from: String,
        to: String,
        span: Span,
    },

    #[error(
        "[{span}] type mismatch: expected `{expected}`, found `{found}` (use `as` for explicit conversion)"
    )]
    Mismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("[{span}] invalid comparison: cannot compare type `{left}` to type `{right}`")]
    InvalidComparison {
        left: String,
        right: String,
        span: Span,
    },

    #[error("[{span}] invalid binary operation: {op} on {left} and {right}")]
    InvalidBinaryOp {
        op: String,
        left: String,
        right: String,
        span: Span,
    },

    #[error("[{span}] invalid unary operation: {op} on {operand}")]
    InvalidUnaryOp {
        op: String,
        operand: String,
        span: Span,
    },

    #[error("[{span}] return outside function")]
    ReturnOutsideFunction { span: Span },

    #[error("[{span}] return type mismatch: expected {expected}, found {found}")]
    ReturnMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("[{span}] argument count mismatch: function expects {expected}, found {found}")]
    ArgCountMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("[{span}] unknown type: `{name}`")]
    UnknownType { name: String, span: Span },

    #[error("[{span}] type `{ty}` has no properties")]
    NoProperties { ty: String, span: Span },

    #[error("[{span}] type `{ty}` has no property `{prop}`")]
    UndefinedProperty {
        ty: String,
        prop: String,
        span: Span,
    },

    #[error("[{span}] type `{name}` already defined")]
    AlreadyDefinedType { name: String, span: Span },

    #[error("[{span}] cannot infer array type because it has no elements")]
    AmbiguousArrayType { span: Span },

    #[error("[{span}] cannot index type `{ty}`")]
    NotIndexable { ty: String, span: Span },

    #[error("[{span}] cannot define a struct inside a scope")]
    StructDeclInsideScope { span: Span },

    #[error("[{span}] file {path} doesn't exists")]
    InvalidImportPath { path: String, span: Span },

    #[error("[{span}] type `{ns_ty}` is not a namespace")]
    InvalidNamespace { ns_ty: String, span: Span },

    #[error("[{span}] public declaration inside scope")]
    PubDeclInsideScope { span: Span },

    #[error("{msg}")]
    LexError { msg: String },

    #[error("{msg}")]
    ParseError { msg: String },

    #[error("[{span}] break outside loop")]
    BreakOutsideLoop { span: Span },

    #[error("[{span}] continue outside loop")]
    ContinueOutsideLoop { span: Span },
}
