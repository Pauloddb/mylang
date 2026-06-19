use crate::{
    lexer::types::{Delim, Keyword, Literal, Op, Token, TokenKind},
    parser::types::{AssignTarget, Ast, Expr, Param, Stmt},
};
use anyhow::Result;

mod types;

type BindingPower = (u8, u8);

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current.min(self.tokens.len() - 1)]
    }

    fn prev(&self) -> &Token {
        &self.tokens[self.current.saturating_sub(1)]
    }

    fn next(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.prev()
    }

    fn check(&self, tok_kind: &TokenKind) -> bool {
        !self.is_at_end() && self.peek().kind == *tok_kind
    }

    fn consume(&mut self, tok_kind: &TokenKind, msg: &str) -> Result<&Token> {
        if self.check(tok_kind) {
            Ok(self.next())
        } else {
            let tok = self.peek();
            anyhow::bail!(
                "[{}] {} (found Token {}\n\tkind: {:?}\n\tlexeme: {}\n{})",
                tok.start,
                msg,
                "{",
                tok.kind,
                tok.lexeme,
                "}",
            )
        }
    }

    // ============ EXPRESSIONS (Pratt Parsing) ============
    fn infix_bp(kind: &TokenKind) -> Option<BindingPower> {
        match kind {
            TokenKind::Op(Op::Assign) => Some((10, 9)), // right-associativo
            TokenKind::Op(Op::Or) => Some((30, 31)),
            TokenKind::Op(Op::And) => Some((40, 41)),
            TokenKind::Op(Op::Eq | Op::Ne) => Some((50, 51)),
            TokenKind::Op(Op::Lt | Op::Le | Op::Gt | Op::Ge) => Some((60, 61)),
            TokenKind::Op(Op::Add | Op::Sub) => Some((70, 71)),
            TokenKind::Op(Op::Mul | Op::Div | Op::Mod) => Some((80, 81)),
            TokenKind::Op(Op::Pow) => Some((90, 89)), // right-associativo
            TokenKind::Delim(Delim::Dot | Delim::LParen | Delim::LBracket) => Some((100, 101)),
            _ => None,
        }
    }

    fn prefix_bp(kind: &TokenKind) -> Option<u8> {
        match kind {
            TokenKind::Op(Op::Not | Op::Sub | Op::PlusPlus | Op::MinusMinus) => Some(110),
            _ => None,
        }
    }

    fn parse_type(&mut self) -> Result<String> {
        let mut ty = self
            .consume(&TokenKind::Ident, "Expected type name")?
            .lexeme
            .clone();

        if self.peek().kind == TokenKind::Delim(Delim::LBracket) {
            self.next();
            self.consume(&TokenKind::Delim(Delim::RBracket), "Expected ']'")?;
            ty.push_str("[]");
        }

        Ok(ty)
    }

    fn parse_lambda(&mut self, name: Option<String>) -> Result<Expr> {
        self.consume(
            &TokenKind::Delim(Delim::LParen),
            "Expected '(' after 'func'",
        )?;

        let mut params = vec![];

        if self.peek().kind != TokenKind::Delim(Delim::RParen) {
            loop {
                let param_name = self
                    .consume(&TokenKind::Ident, "Expected param name")?
                    .lexeme
                    .clone();

                self.consume(
                    &TokenKind::Delim(Delim::Colon),
                    "Expected ':' after parameter name",
                )?;

                let ty = self.parse_type()?;
                params.push(Param {
                    name: param_name,
                    ty,
                });

                if self.peek().kind == TokenKind::Delim(Delim::Comma) {
                    self.next(); // consome ','
                } else {
                    break;
                }
            }
        }

        self.consume(
            &TokenKind::Delim(Delim::RParen),
            "Expected ')' after parameters",
        )?;

        let ret_ty = self
            .consume(
                &TokenKind::Ident,
                "Expected return type after ')' in function declaration",
            )?
            .lexeme
            .clone();
        let body = self.block_expr()?;

        Ok(Expr::Func {
            params,
            ret_ty,
            body: Box::new(body),
            name,
        })
    }

    fn nud(&mut self, token: &Token) -> Result<Expr> {
        match token.kind {
            TokenKind::Literal(Literal::Int) => Ok(Expr::Int(token.lexeme.parse()?)),
            TokenKind::Literal(Literal::Float) => Ok(Expr::Float(token.lexeme.parse()?)),
            TokenKind::Literal(Literal::String) => Ok(Expr::String(
                token.lexeme[1..token.lexeme.len().saturating_sub(1)].to_string(),
            )),
            TokenKind::Keyword(Keyword::True) => Ok(Expr::Bool(true)),
            TokenKind::Keyword(Keyword::False) => Ok(Expr::Bool(false)),
            TokenKind::Keyword(Keyword::Nil) => Ok(Expr::Nil),
            TokenKind::Ident => Ok(Expr::Ident(token.lexeme.clone())),
            // If as expr
            TokenKind::Keyword(Keyword::If) => self.if_expr(),
            // Prefix
            TokenKind::Op(Op::Sub) => {
                let bp = Self::prefix_bp(&token.kind).unwrap();
                Ok(Expr::Unary {
                    op: Op::Sub,
                    right: Box::new(self.expression(bp)?),
                })
            }
            TokenKind::Op(Op::Not) => {
                let bp = Self::prefix_bp(&token.kind).unwrap();
                Ok(Expr::Unary {
                    op: Op::Not,
                    right: Box::new(self.expression(bp)?),
                })
            }
            TokenKind::Op(Op::PlusPlus) => {
                let bp = Self::prefix_bp(&token.kind).unwrap();
                Ok(Expr::Unary {
                    op: Op::PlusPlus,
                    right: Box::new(self.expression(bp)?),
                })
            }
            TokenKind::Op(Op::MinusMinus) => {
                let bp = Self::prefix_bp(&token.kind).unwrap();
                Ok(Expr::Unary {
                    op: Op::MinusMinus,
                    right: Box::new(self.expression(bp)?),
                })
            }
            TokenKind::Delim(Delim::LParen) => {
                let expr = self.parse_expression()?;
                self.consume(&TokenKind::Delim(Delim::RParen), "Expected ')'")?;
                Ok(expr)
            }
            TokenKind::Delim(Delim::LCurly) => self.block_expr(),
            TokenKind::Keyword(Keyword::Rec) => {
                self.consume(
                    &TokenKind::Keyword(Keyword::Func),
                    "Expected 'func' after 'rec'",
                )?;
                self.parse_lambda(Some(String::new())) // Placeholder
            }
            TokenKind::Keyword(Keyword::Func) => self.parse_lambda(None),
            _ => anyhow::bail!(
                "[{}] Unexpected token at start: {:?}",
                token.start,
                token.kind
            ),
        }
    }

    fn binary(&mut self, left: Expr, op: Op, rbp: u8) -> Result<Expr> {
        Ok(Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(self.expression(rbp)?),
        })
    }

    fn expr_to_target(expr: Expr) -> Result<AssignTarget> {
        match expr {
            Expr::Ident(name) => Ok(AssignTarget::Ident(name)),
            Expr::Property { object, prop } => Ok(AssignTarget::Property { object, prop }),
            Expr::Index { object, index } => Ok(AssignTarget::Index { object, index }),
            _ => anyhow::bail!("Cannot assign to {:?}", expr),
        }
    }

    fn led(&mut self, token: &Token, left: Expr) -> Result<Expr> {
        match token.kind {
            TokenKind::Op(Op::Add) => {
                self.binary(left, Op::Add, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Sub) => {
                self.binary(left, Op::Sub, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Mul) => {
                self.binary(left, Op::Mul, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Div) => {
                self.binary(left, Op::Div, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Mod) => {
                self.binary(left, Op::Mod, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Pow) => {
                self.binary(left, Op::Pow, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Eq) => {
                self.binary(left, Op::Eq, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Ne) => {
                self.binary(left, Op::Ne, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Lt) => {
                self.binary(left, Op::Lt, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Le) => {
                self.binary(left, Op::Le, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Gt) => {
                self.binary(left, Op::Gt, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Ge) => {
                self.binary(left, Op::Ge, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::BitAnd) => {
                self.binary(left, Op::BitAnd, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::BitOr) => {
                self.binary(left, Op::BitOr, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::BitXor) => {
                self.binary(left, Op::BitXor, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::And) => {
                self.binary(left, Op::And, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Or) => {
                self.binary(left, Op::Or, Self::infix_bp(&token.kind).unwrap().0)
            }
            TokenKind::Op(Op::Assign) => {
                let target = Self::expr_to_target(left)?;
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                let value = self.expression(rbp)?;

                Ok(Expr::Assign {
                    target,
                    value: Box::new(value),
                })
            }
            TokenKind::Delim(Delim::Dot) => {
                let prop = self.consume(&TokenKind::Ident, "Expected property name after '.'")?;

                Ok(Expr::Property {
                    object: Box::new(left),
                    prop: prop.lexeme.clone(),
                })
            }
            TokenKind::Delim(Delim::LParen) => {
                let mut args = vec![];

                if self.peek().kind != TokenKind::Delim(Delim::RParen) {
                    loop {
                        args.push(self.expression(0)?);
                        if self.peek().kind == TokenKind::Delim(Delim::Comma) {
                            self.next();
                        } else {
                            break;
                        }
                    }
                }

                self.consume(
                    &TokenKind::Delim(Delim::RParen),
                    "Expected ')' after arguments",
                )?;

                Ok(Expr::Call {
                    callee: Box::new(left),
                    args,
                })
            }
            TokenKind::Delim(Delim::LBracket) => {
                let index = self.expression(0)?;

                self.consume(
                    &TokenKind::Delim(Delim::RBracket),
                    "Expected ']' after index",
                )?;

                match left {
                    Expr::Ident(_) | Expr::Index { .. } | Expr::Property { .. } => {
                        Ok(Expr::Index {
                            object: Box::new(left),
                            index: Box::new(index),
                        })
                    }
                    _ => anyhow::bail!("Cannot index {:?}", left),
                }
            }
            _ => anyhow::bail!("[{}] Unexpected token: {:?}", token.start, token.kind),
        }
    }

    fn expression(&mut self, min_bp: u8) -> Result<Expr> {
        let token = self.next().clone();
        let mut lhs = self.nud(&token)?;

        loop {
            let next = self.peek();

            if let Some((lbp, _)) = Self::infix_bp(&next.kind) {
                if lbp < min_bp {
                    break;
                }
                let op = self.next().clone();
                lhs = self.led(&op, lhs)?;
                continue;
            }

            break;
        }

        Ok(lhs)
    }

    fn parse_expression(&mut self) -> Result<Expr> {
        self.expression(0)
    }

    fn var_decl(&mut self) -> Result<Stmt> {
        let is_mutable = self.next().kind == TokenKind::Keyword(Keyword::Mut);

        let name = self
            .consume(
                &TokenKind::Ident,
                &format!(
                    "Expected variable name after '{}'",
                    if is_mutable { "mut" } else { "def" }
                ),
            )?
            .lexeme
            .clone();

        let type_annotation = if self.peek().kind == TokenKind::Delim(Delim::Colon) {
            let mut ty = self
                .consume(
                    &TokenKind::Ident,
                    "Expected type after ':' in variable declaration",
                )?
                .lexeme
                .clone();

            if self.peek().kind == TokenKind::Delim(Delim::LBracket) {
                self.next();
                self.consume(&TokenKind::Delim(Delim::RBracket), "Expected ']'")?;
                ty.push_str("[]");
            }

            Some(ty)
        } else {
            None
        };

        self.consume(
            &TokenKind::Op(Op::Assign),
            "Expected expression after '=' in variable declaration",
        )?;

        let mut value = self.parse_expression()?;

        if let Expr::Func {
            name: ref mut func_name,
            ..
        } = value
            && func_name.as_ref().is_some_and(|n| n.is_empty())
        {
            *func_name = Some(name.clone())
        }

        self.consume(
            &TokenKind::Delim(Delim::Semicolon),
            "Expected ';' after variable declaration",
        )?;

        Ok(Stmt::VarDecl {
            name,
            type_annotation,
            value: Box::new(value),
            is_mutable,
        })
    }

    fn block_expr(&mut self) -> Result<Expr> {
        self.consume(
            &TokenKind::Delim(Delim::LCurly),
            "Expected '{' before block",
        )?;

        let mut stmts = vec![];

        loop {
            stmts.push(self.parse_stmt()?);
            if self.peek().kind == TokenKind::Delim(Delim::RCurly) {
                break;
            }
        }

        self.consume(&TokenKind::Delim(Delim::RCurly), "Expected '}' after block")?;

        Ok(Expr::Block(stmts))
    }

    fn if_expr(&mut self) -> Result<Expr> {
        // 'if' já foi consumido pelo next() que chamou nud
        // Agora parseia o resto: cond then { ... } else { ... }

        let cond = self.parse_expression()?;

        self.consume(
            &TokenKind::Keyword(Keyword::Then),
            "Expected 'then' after condition in if statements",
        )?;

        let then_branch = self.block_expr()?;

        let else_branch = if self.peek().kind == TokenKind::Keyword(Keyword::Else) {
            self.next();

            if self.peek().kind == TokenKind::Keyword(Keyword::If) {
                // Consome 'if'
                self.next();
                Some(Box::new(self.if_expr()?))
            } else {
                Some(Box::new(self.block_expr()?))
            }
        } else {
            None
        };

        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch,
        })
    }

    fn while_stmt(&mut self) -> Result<Stmt> {
        self.next();

        let cond = self.parse_expression()?;

        self.consume(
            &TokenKind::Keyword(Keyword::Do),
            "Expected 'do' after condition in while statements",
        )?;

        let body = self.block_expr()?;

        Ok(Stmt::While {
            cond: Box::new(cond),
            body: Box::new(Stmt::Expr(body)),
        })
    }

    fn return_stmt(&mut self) -> Result<Stmt> {
        self.next();

        let ret_val = if self.peek().kind != TokenKind::Delim(Delim::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(
            &TokenKind::Delim(Delim::Semicolon),
            "Expected ';' after return",
        )?;

        Ok(Stmt::Return(ret_val))
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        match self.peek().kind {
            TokenKind::Keyword(Keyword::Def | Keyword::Mut) => self.var_decl(),
            TokenKind::Keyword(Keyword::While) => self.while_stmt(),
            TokenKind::Keyword(Keyword::Return) => self.return_stmt(),
            _ => {
                let expr = self.parse_expression()?;
                self.consume(
                    &TokenKind::Delim(Delim::Semicolon),
                    "Expected ';' after expression",
                )?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    pub fn parse(&mut self) -> Result<Ast> {
        let mut stmts = vec![];

        while !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
        }

        Ok(Ast { stmts })
    }
}
