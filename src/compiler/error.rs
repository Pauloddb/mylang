use crate::lexer::types::Span;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("[{span}] undefined variable: `{name}`")]
    UndefinedVariable { name: String, span: Span },

    #[error("[{span}] `pub` is only allowed on top-level declarations")]
    PubInScope { span: Span },

    #[error("[{span}] return outside function")]
    ReturnOutsideFunction { span: Span },

    #[error("[{span}] invalid unary operator: {op}")]
    InvalidUnaryOp { op: String, span: Span },

    #[error("[{span}] invalid binary operator: {op}")]
    InvalidBinaryOp { op: String, span: Span },

    #[error("[{span}] internal compiler invariant violated: {msg}")]
    Internal { span: Span, msg: String },
}
