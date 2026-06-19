use std::fmt;

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub start: Pos,
    pub end: Pos,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: impl Into<String>, start: Pos, end: Pos) -> Self {
        Self {
            lexeme: lexeme.into(),
            kind,
            start,
            end,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
    pub byte_offset: usize,
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub pos: Pos,
    pub message: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lexer error at [{}]: {}", self.pos, self.message)
    }
}

impl std::error::Error for LexError {}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Literal(Literal),
    Keyword(Keyword),
    Op(Op),
    Delim(Delim),
    Ident,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int,
    Float,
    String,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Op {
    Add,
    PlusPlus,
    Sub,
    MinusMinus,
    Mul,
    Div,
    Mod,
    Pow,

    Assign,

    Eq,
    Ne,
    Le,
    Lt,
    Ge,
    Gt,

    BitAnd,
    BitOr,
    BitXor,

    Not,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Delim {
    LParen,
    RParen,

    LBracket,
    RBracket,

    LCurly,
    RCurly,

    Colon,
    Semicolon,
    Comma,
    Dot,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Mut,
    Def,

    If,
    Else,
    Then,

    While,
    Do,

    Nil,
    True,
    False,

    Func,
    Return,
    Rec,
}

impl Keyword {
    pub fn from_str(input: &str) -> Option<Self> {
        match input {
            "mut" => Some(Self::Mut),
            "def" => Some(Self::Def),

            "if" => Some(Self::If),
            "else" => Some(Self::Else),
            "then" => Some(Self::Then),

            "while" => Some(Self::While),
            "do" => Some(Self::Do),

            "nil" => Some(Self::Nil),
            "true" => Some(Self::True),
            "false" => Some(Self::False),

            "func" => Some(Self::Func),
            "return" => Some(Self::Return),
            "rec" => Some(Self::Rec),

            _ => None,
        }
    }
}
