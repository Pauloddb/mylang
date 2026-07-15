use crate::lexer::types::Span;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EvalError {
    #[error("[{span}] immutable mutation: cannot mutate immutable variable `{name}`")]
    ImmutableMutation { name: String, span: Span },

    #[error("[{span}] undefined variable: `{name}`")]
    UndefinedVar { name: String, span: Span },

    #[error("[{span}] type error: {msg}")]
    TypeError { msg: String, span: Span },

    #[error("[{span}] index out of bounds: {msg}")]
    IndexError { msg: String, span: Span },

    #[error("[{span}] `{value}` is not callable")]
    NotCallable { value: String, span: Span },

    #[error("[{span}] import error: {msg}")]
    ImportError { msg: String, span: Span },

    #[error("[{span}] value `{ns_val}` is not a namespace")]
    InvalidNamespace { ns_val: String, span: Span },

    #[error("[{span}] cannot pop from empty array")]
    PopEmpty { span: Span },

    #[error("[{span}] value {value} has no properties")]
    NoProperties { value: String, span: Span },

    #[error("[{span}] value {value} has no property `prop`")]
    UnknownProperty {
        value: String,
        prop: String,
        span: Span,
    },

    #[error("{msg}")]
    LexError { msg: String },

    #[error("{msg}")]
    ParseError { msg: String },

    #[error("{msg}")]
    TypeCheckerError { msg: String },

    // Controle (não são erros reais)
    #[error("[control] return")]
    Return,
}
