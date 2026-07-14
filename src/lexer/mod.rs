use std::{iter::Peekable, str::Chars};

use crate::lexer::types::{Delim, Keyword, LexError, Literal, Op, Pos, Token, TokenKind};
use anyhow::Result;

pub mod types;

pub struct Lexer<'a> {
    src: Peekable<Chars<'a>>,
    input: &'a str,
    pos: Pos,
    file: String,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str, file: String) -> Self {
        Self {
            src: input.chars().peekable(),
            input,
            pos: Pos {
                line: 1,
                col: 1,
                byte_offset: 0,
            },
            file,
        }
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.src.next()?;

        if c == '\n' {
            self.pos.line += 1;
            self.pos.col = 1;
        } else {
            self.pos.col += 1;
        }
        self.pos.byte_offset += c.len_utf8();

        Some(c)
    }

    fn current(&mut self) -> Option<char> {
        self.src.peek().copied()
    }

    fn skip_trivia(&mut self) {
        loop {
            // skip whitespace
            while let Some(c) = self.current() {
                if c.is_ascii_whitespace() {
                    self.bump();
                } else {
                    break;
                }
            }

            // skip comment
            if self.current() == Some('#') {
                while let Some(c) = self.current() {
                    self.bump();
                    if c == '\n' {
                        break;
                    }
                }
                continue; // volta pro topo — pode ter mais ws/comments
            }

            break;
        }
    }

    fn number(&mut self) -> TokenKind {
        while let Some(ch) = self.current() {
            if ch.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }

        let mut is_float = false;

        if self.current() == Some('.') {
            let has_digits_after = self
                .src
                .clone()
                .nth(1)
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false);

            if has_digits_after {
                is_float = true;

                self.bump();

                while let Some(ch) = self.current() {
                    if ch.is_ascii_digit() {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
        }

        if is_float {
            TokenKind::Literal(Literal::Float)
        } else {
            TokenKind::Literal(Literal::Int)
        }
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_trivia();

        let start = self.pos;

        let kind = match self.current() {
            // ==== Delim ====
            Some('(') => {
                self.bump();
                TokenKind::Delim(Delim::LParen)
            }
            Some(')') => {
                self.bump();
                TokenKind::Delim(Delim::RParen)
            }
            Some('[') => {
                self.bump();
                TokenKind::Delim(Delim::LBracket)
            }
            Some(']') => {
                self.bump();
                TokenKind::Delim(Delim::RBracket)
            }
            Some('{') => {
                self.bump();
                TokenKind::Delim(Delim::LCurly)
            }
            Some('}') => {
                self.bump();
                TokenKind::Delim(Delim::RCurly)
            }
            Some('.') => {
                self.bump();
                TokenKind::Delim(Delim::Dot)
            }
            Some(':') => {
                self.bump();
                if self.current() == Some(':') {
                    self.bump();
                    TokenKind::Delim(Delim::DoubleColon)
                } else {
                    TokenKind::Delim(Delim::Colon)
                }
            }
            Some(';') => {
                self.bump();
                TokenKind::Delim(Delim::Semicolon)
            }
            Some(',') => {
                self.bump();
                TokenKind::Delim(Delim::Comma)
            }
            // ==== Op ====
            Some('+') => {
                self.bump();

                if self.current() == Some('+') {
                    self.bump();
                    TokenKind::Op(Op::PlusPlus)
                } else {
                    TokenKind::Op(Op::Add)
                }
            }
            Some('-') => {
                self.bump();

                if self.current() == Some('-') {
                    self.bump();
                    TokenKind::Op(Op::MinusMinus)
                } else if self.current() == Some('>') {
                    self.bump();
                    TokenKind::Delim(Delim::Arrow)
                } else {
                    TokenKind::Op(Op::Sub)
                }
            }
            Some('*') => {
                self.bump();
                if self.current() == Some('*') {
                    self.bump();
                    TokenKind::Op(Op::Pow)
                } else {
                    TokenKind::Op(Op::Mul)
                }
            }
            Some('/') => {
                self.bump();
                TokenKind::Op(Op::Div)
            }
            Some('%') => {
                self.bump();
                TokenKind::Op(Op::Mod)
            }
            Some('=') => {
                self.bump();

                if let Some('=') = self.current() {
                    self.bump();
                    TokenKind::Op(Op::Eq)
                } else {
                    TokenKind::Op(Op::Assign)
                }
            }
            Some('!') => {
                self.bump();

                if let Some('=') = self.current() {
                    self.bump();
                    TokenKind::Op(Op::Ne)
                } else {
                    TokenKind::Op(Op::Not)
                }
            }
            Some('&') => {
                self.bump();

                if let Some('&') = self.current() {
                    self.bump();
                    TokenKind::Op(Op::And)
                } else {
                    TokenKind::Op(Op::BitAnd)
                }
            }
            Some('|') => {
                self.bump();

                if let Some('|') = self.current() {
                    self.bump();
                    TokenKind::Op(Op::Or)
                } else {
                    TokenKind::Op(Op::BitOr)
                }
            }
            Some('^') => {
                self.bump();
                TokenKind::Op(Op::BitXor)
            }
            Some('>') => {
                self.bump();

                if let Some('=') = self.current() {
                    self.bump();
                    TokenKind::Op(Op::Ge)
                } else if let Some('>') = self.current() {
                    self.bump();
                    TokenKind::Op(Op::Shr)
                } else {
                    TokenKind::Op(Op::Gt)
                }
            }
            Some('<') => {
                self.bump();

                if let Some('=') = self.current() {
                    self.bump();
                    TokenKind::Op(Op::Le)
                } else if let Some('<') = self.current() {
                    self.bump();
                    TokenKind::Op(Op::Shl)
                } else {
                    TokenKind::Op(Op::Lt)
                }
            }
            // ==== String ====
            Some('"') => {
                self.bump();

                while let Some(ch) = self.current() {
                    if ch == '"' {
                        self.bump();
                        break;
                    }
                    self.bump();
                }

                TokenKind::Literal(Literal::String)
            }
            // ==== Int / Float ====
            Some(c) if c.is_ascii_digit() => self.number(),
            // ==== Ident / Keyword ====
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                self.bump();

                while let Some(ch) = self.current() {
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        self.bump();
                    } else {
                        break;
                    }
                }

                match Keyword::from_str(&self.input[start.byte_offset..self.pos.byte_offset]) {
                    Some(kw) => TokenKind::Keyword(kw),
                    None => TokenKind::Ident,
                }
            }
            Some(c) => {
                return Err(LexError {
                    file: self.file.clone(),
                    pos: start,
                    message: format!("Unexpected char: {}", c),
                });
            }
            None => TokenKind::Eof,
        };

        let end = self.pos;

        let lexeme = &self.input[start.byte_offset..end.byte_offset];

        Ok(Token::new(kind, lexeme, self.file.clone(), start, end))
    }

    pub fn lex(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = vec![];

        loop {
            let tok = self.next_token()?;

            if tok.kind == TokenKind::Eof {
                tokens.push(tok);
                break;
            }

            tokens.push(tok);
        }

        // Reset
        self.src = self.input.chars().peekable();
        self.pos = Pos {
            line: 1,
            col: 1,
            byte_offset: 0,
        };

        Ok(tokens)
    }
}
