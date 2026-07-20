use thiserror::Error;

use crate::lexer::types::Span;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("[{span}] undefined variable: `{name}`")]
    UndefinedVariable { name: String, span: Span },

    #[error("[{span}] immutable mutation: cannot mutate immutable variable `{name}`")]
    ImmutableMutation { name: String, span: Span },

    #[error("[{span}] index out of bounds: {msg}")]
    IndexError { msg: String, span: Span },

    #[error("[{span}] cannot pop from empty array")]
    PopEmpty { span: Span },

    #[error("[{span}] `{value}` is not callable")]
    NotCallable { value: String, span: Span },

    #[error("[{span}] value `{value}` has no property `{prop}`")]
    UnknownProperty {
        value: String,
        prop: String,
        span: Span,
    },

    #[error("[{span}] value `{value}` has no properties")]
    NoProperties { value: String, span: Span },

    #[error("[{span}] value `{ns_val}` is not a namespace")]
    InvalidNamespace { ns_val: String, span: Span },

    #[error("[{span}] division by zero")]
    DivisionByZero { span: Span },

    #[error("import error: {msg}")]
    ImportError { msg: String },

    #[error("{msg}")]
    LexError { msg: String },

    #[error("{msg}")]
    ParseError { msg: String },

    #[error("{msg}")]
    TypeError { msg: String },

    #[error("{msg}")]
    CompileError { msg: String },
}
