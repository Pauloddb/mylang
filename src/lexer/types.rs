use std::fmt;

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: Span,
}

impl Token {
    pub fn new(
        kind: TokenKind,
        lexeme: impl Into<String>,
        file: String,
        start: Pos,
        end: Pos,
    ) -> Self {
        Self {
            lexeme: lexeme.into(),
            kind,
            span: Span { file, start, end },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub file: String,
    pub start: Pos,
    pub end: Pos,
}

impl Span {
    /// Mescla dois spans (do início do primeiro até o fim do segundo)
    pub fn merge(a: &Span, b: &Span) -> Self {
        Self {
            file: a.file.clone(),
            start: a.start,
            end: b.end,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{} - {}", self.file, self.start, self.end)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
    pub byte_offset: usize,
}

impl Pos {
    /// Construtor simples
    pub fn new(line: usize, col: usize, byte_offset: usize) -> Self {
        Self {
            line,
            col,
            byte_offset,
        }
    }
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub file: String,
    pub pos: Pos,
    pub message: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: lexer error: {}",
            self.file, self.pos, self.message
        )
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
    Shl,
    Shr,

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
    DoubleColon,
    Semicolon,
    Comma,
    Dot,
    Arrow,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Mut,
    Def,

    If,
    Else,
    Then,

    While,
    Break,
    Continue,
    Do,

    Nil,
    True,
    False,

    Func,
    Return,
    Rec,

    Struct,

    As,

    Pub,
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
            "break" => Some(Self::Break),
            "continue" => Some(Self::Continue),
            "do" => Some(Self::Do),

            "nil" => Some(Self::Nil),
            "true" => Some(Self::True),
            "false" => Some(Self::False),

            "func" => Some(Self::Func),
            "return" => Some(Self::Return),
            "rec" => Some(Self::Rec),

            "struct" => Some(Self::Struct),

            "as" => Some(Self::As),

            "pub" => Some(Self::Pub),

            _ => None,
        }
    }
}
