use crate::{
    lexer::types::{Delim, Keyword, Literal, Op, Span, Token, TokenKind},
    parser::types::{AssignTarget, Ast, Expr, Param, Stmt},
};
use anyhow::Result;

pub mod types;

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
                "[{}] {} (found Token {{ kind: {:?} lexeme: {} }})",
                tok.span,
                msg,
                tok.kind,
                tok.lexeme,
            )
        }
    }

    // ============ EXPRESSIONS (Pratt Parsing) ============
    fn infix_bp(kind: &TokenKind) -> Option<BindingPower> {
        match kind {
            TokenKind::Op(Op::Assign) => Some((10, 9)), // right associative
            TokenKind::Op(Op::Or) => Some((30, 31)),
            TokenKind::Op(Op::And) => Some((40, 41)),
            TokenKind::Op(Op::Eq | Op::Ne) => Some((50, 51)),
            TokenKind::Op(Op::Lt | Op::Le | Op::Gt | Op::Ge) => Some((60, 61)),
            TokenKind::Op(Op::BitOr) => Some((70, 71)),
            TokenKind::Op(Op::BitXor) => Some((80, 81)),
            TokenKind::Op(Op::BitAnd) => Some((90, 91)),
            TokenKind::Keyword(Keyword::As) => Some((100, 101)),
            TokenKind::Op(Op::Shl | Op::Shr) => Some((110, 111)),
            TokenKind::Op(Op::Add | Op::Sub) => Some((120, 121)),
            TokenKind::Op(Op::Mul | Op::Div | Op::Mod) => Some((130, 131)),
            TokenKind::Op(Op::Pow) => Some((140, 139)), // right associative
            TokenKind::Delim(
                Delim::Dot | Delim::DoubleColon | Delim::LParen | Delim::LBracket | Delim::LCurly,
            ) => Some((150, 151)),
            _ => None,
        }
    }

    fn prefix_bp(kind: &TokenKind) -> Option<u8> {
        match kind {
            TokenKind::Op(Op::Not | Op::Sub | Op::PlusPlus | Op::MinusMinus) => Some(130),
            _ => None,
        }
    }

    fn parse_type(&mut self) -> Result<String> {
        // NOVO: func(params) -> ret
        if self.peek().kind == TokenKind::Keyword(Keyword::Func) {
            self.next(); // consome 'func'

            self.consume(
                &TokenKind::Delim(Delim::LParen),
                "Expected '(' after 'func'",
            )?;

            let mut params = vec![];

            if self.peek().kind != TokenKind::Delim(Delim::RParen) {
                loop {
                    params.push(self.parse_type()?);
                    if self.peek().kind == TokenKind::Delim(Delim::Comma) {
                        self.next();
                    } else {
                        break;
                    }
                }
            }

            self.consume(
                &TokenKind::Delim(Delim::RParen),
                "Expected ')' after parameters",
            )?;
            self.consume(
                &TokenKind::Delim(Delim::Arrow),
                "Expected '->' before function return type",
            )?;

            let ret = self.parse_type()?;

            return Ok(format!("func({}) -> {}", params.join(", "), ret));
        }

        // EXISTENTE: ident + []
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

    fn parse_lambda(&mut self, name: Option<String>, start_span: Span) -> Result<Expr> {
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

                // Span do param = do ident até o tipo
                let param_span = Span::merge(&start_span, &self.prev().span);

                params.push(Param {
                    name: param_name,
                    ty,
                    span: param_span,
                });

                if self.peek().kind == TokenKind::Delim(Delim::Comma) {
                    self.next();
                } else {
                    break;
                }
            }
        }

        self.consume(
            &TokenKind::Delim(Delim::RParen),
            "Expected ')' after parameters",
        )?;

        self.consume(
            &TokenKind::Delim(Delim::Arrow),
            "Expected '->' before funcion return type",
        )?;

        let ret_ty = self.parse_type()?;

        let body = self.block_expr()?;

        // Span da func = do 'func'/'rec' até o fim do body
        let span = Span::merge(&start_span, &body.span());

        Ok(Expr::Func {
            params,
            ret_ty,
            body: Box::new(body),
            name,
            span,
        })
    }

    fn parse_struct_fields(&mut self) -> Result<Vec<(String, Expr)>> {
        let mut fields = vec![];
        if self.peek().kind != TokenKind::Delim(Delim::RCurly) {
            loop {
                let field_name = self
                    .consume(&TokenKind::Ident, "Expected field name")?
                    .lexeme
                    .clone();
                self.consume(
                    &TokenKind::Delim(Delim::Colon),
                    "Expected ':' after field name",
                )?;
                let value = self.parse_expression()?;
                fields.push((field_name, value));
                if self.peek().kind == TokenKind::Delim(Delim::Comma) {
                    self.next();
                } else {
                    break;
                }
            }
        }
        Ok(fields)
    }

    fn parse_struct_literal(&mut self, name: String, start_span: Span) -> Result<Expr> {
        self.consume(
            &TokenKind::Delim(Delim::LCurly),
            "Expected '{' in struct literal",
        )?;
        let fields = self.parse_struct_fields()?;
        let end_tok = self.consume(
            &TokenKind::Delim(Delim::RCurly),
            "Expected '}' after struct literal",
        )?;
        let span = Span::merge(&start_span, &end_tok.span);
        Ok(Expr::Struct { name, fields, span })
    }

    fn nud(&mut self, token: &Token) -> Result<Expr> {
        let start_span = token.span.clone();

        match token.kind {
            TokenKind::Literal(Literal::Int) => Ok(Expr::Int(token.lexeme.parse()?, start_span)),
            TokenKind::Literal(Literal::Float) => {
                Ok(Expr::Float(token.lexeme.parse()?, start_span))
            }
            TokenKind::Literal(Literal::String) => Ok(Expr::String(
                token.lexeme[1..token.lexeme.len().saturating_sub(1)].to_string(),
                start_span,
            )),
            TokenKind::Keyword(Keyword::True) => Ok(Expr::Bool(true, start_span)),
            TokenKind::Keyword(Keyword::False) => Ok(Expr::Bool(false, start_span)),
            TokenKind::Keyword(Keyword::Nil) => Ok(Expr::Nil(start_span)),
            TokenKind::Keyword(Keyword::If) => self.if_expr(start_span),
            TokenKind::Op(Op::Sub) => {
                let bp = Self::prefix_bp(&token.kind).unwrap();
                let right = self.expression(bp)?;
                let span = Span::merge(&start_span, &right.span());

                Ok(Expr::Unary {
                    op: Op::Sub,
                    right: Box::new(right),
                    span,
                })
            }
            TokenKind::Op(Op::Not) => {
                let bp = Self::prefix_bp(&token.kind).unwrap();
                let right = self.expression(bp)?;
                let span = Span::merge(&start_span, &right.span());
                Ok(Expr::Unary {
                    op: Op::Not,
                    right: Box::new(right),
                    span,
                })
            }
            TokenKind::Op(Op::PlusPlus) => {
                let bp = Self::prefix_bp(&token.kind).unwrap();
                let right = self.expression(bp)?;
                let span = Span::merge(&start_span, &right.span());
                Ok(Expr::Unary {
                    op: Op::PlusPlus,
                    right: Box::new(right),
                    span,
                })
            }
            TokenKind::Op(Op::MinusMinus) => {
                let bp = Self::prefix_bp(&token.kind).unwrap();
                let right = self.expression(bp)?;
                let span = Span::merge(&start_span, &right.span());
                Ok(Expr::Unary {
                    op: Op::MinusMinus,
                    right: Box::new(right),
                    span,
                })
            }
            TokenKind::Delim(Delim::LParen) => {
                let expr = self.parse_expression()?;
                let end_tok = self.consume(&TokenKind::Delim(Delim::RParen), "Expected ')'")?;
                // Span do grouping = do '(' ao ')'
                let span = Span::merge(&start_span, &end_tok.span);
                Ok(Expr::Block(vec![Stmt::Expr(expr)], span)) // ou crie Expr::Grouping
            }
            TokenKind::Delim(Delim::LCurly) => self.block_expr_from_token(start_span),
            TokenKind::Delim(Delim::LBracket) => {
                let mut elements = vec![];

                if self.peek().kind != TokenKind::Delim(Delim::RBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if self.peek().kind == TokenKind::Delim(Delim::Comma) {
                            self.next();
                        } else {
                            break;
                        }
                    }
                }

                let end_tok = self.consume(
                    &TokenKind::Delim(Delim::RBracket),
                    "Expected ']' after array elements",
                )?;
                let span = Span::merge(&token.span, &end_tok.span);

                Ok(Expr::ArrayLiteral(elements, span))
            }
            TokenKind::Keyword(Keyword::Rec) => {
                self.consume(
                    &TokenKind::Keyword(Keyword::Func),
                    "Expected 'func' after 'rec'",
                )?;
                self.parse_lambda(Some(String::new()), start_span)
            }
            TokenKind::Keyword(Keyword::Func) => self.parse_lambda(None, start_span),
            TokenKind::Ident => {
                if self.peek().kind == TokenKind::Delim(Delim::LCurly) {
                    self.parse_struct_literal(token.lexeme.clone(), start_span)
                } else {
                    Ok(Expr::Ident(token.lexeme.clone(), start_span))
                }
            }

            _ => anyhow::bail!(
                "[{}] Unexpected token at start: {:?}",
                token.span,
                token.kind
            ),
        }
    }

    fn binary(&mut self, left: Expr, op: Op, rbp: u8) -> Result<Expr> {
        let right = self.expression(rbp)?;
        let span = Span::merge(&left.span(), &right.span());

        Ok(Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
            span,
        })
    }

    fn expr_to_target(expr: Expr) -> Result<(AssignTarget, Span)> {
        let span = expr.span();
        match expr {
            Expr::Ident(name, _) => Ok((AssignTarget::Ident(name, span.clone()), span)),
            Expr::Property { object, prop, .. } => Ok((
                AssignTarget::Property {
                    object,
                    prop,
                    span: span.clone(),
                },
                span,
            )),
            Expr::Index { object, index, .. } => Ok((
                AssignTarget::Index {
                    object,
                    index,
                    span: span.clone(),
                },
                span,
            )),
            _ => anyhow::bail!("Cannot assign to {:?}", expr),
        }
    }

    fn led(&mut self, token: &Token, left: Expr) -> Result<Expr> {
        let start_span = left.span();

        match token.kind {
            TokenKind::Op(Op::Add) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Add, rbp)
            }
            TokenKind::Op(Op::Sub) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Sub, rbp)
            }
            TokenKind::Op(Op::Mul) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Mul, rbp)
            }
            TokenKind::Op(Op::Div) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Div, rbp)
            }
            TokenKind::Op(Op::Mod) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Mod, rbp)
            }
            TokenKind::Op(Op::Pow) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Pow, rbp)
            }
            TokenKind::Op(Op::Eq) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Eq, rbp)
            }
            TokenKind::Op(Op::Ne) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Ne, rbp)
            }
            TokenKind::Op(Op::Lt) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Lt, rbp)
            }
            TokenKind::Op(Op::Le) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Le, rbp)
            }
            TokenKind::Op(Op::Gt) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Gt, rbp)
            }
            TokenKind::Op(Op::Ge) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Ge, rbp)
            }
            TokenKind::Op(Op::BitAnd) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::BitAnd, rbp)
            }
            TokenKind::Op(Op::BitOr) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::BitOr, rbp)
            }
            TokenKind::Op(Op::BitXor) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::BitXor, rbp)
            }
            TokenKind::Op(Op::And) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::And, rbp)
            }
            TokenKind::Op(Op::Or) => {
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                self.binary(left, Op::Or, rbp)
            }

            TokenKind::Op(Op::Assign) => {
                let (target, _) = Self::expr_to_target(left)?;
                let (_, rbp) = Self::infix_bp(&token.kind).unwrap();
                let value = self.expression(rbp)?;
                let span = Span::merge(&start_span, &value.span());

                Ok(Expr::Assign {
                    target,
                    value: Box::new(value),
                    span,
                })
            }

            TokenKind::Delim(Delim::Dot) => {
                let prop = self.consume(&TokenKind::Ident, "Expected property name after '.'")?;
                let span = Span::merge(&start_span, &prop.span);

                Ok(Expr::Property {
                    object: Box::new(left),
                    prop: prop.lexeme.clone(),
                    span,
                })
            }

            TokenKind::Delim(Delim::DoubleColon) => {
                let member = self.consume(&TokenKind::Ident, "Expected identifier after '::'")?;
                let span = Span::merge(&start_span, &member.span);

                Ok(Expr::Path {
                    namespace: Box::new(left),
                    member: member.lexeme.clone(),
                    span,
                })
            }

            TokenKind::Delim(Delim::LCurly) => {
                if let Expr::Path { member, .. } = &left {
                    let fields = self.parse_struct_fields()?;
                    let end_tok = self.consume(
                        &TokenKind::Delim(Delim::RCurly),
                        "Expected '}}' after struct literal",
                    )?;
                    let span = Span::merge(&start_span, &end_tok.span);
                    Ok(Expr::Struct {
                        name: member.clone(),
                        fields,
                        span,
                    })
                } else {
                    anyhow::bail!("[{}] Unexpected '{{' after expression", token.span)
                }
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

                let end_tok = self.consume(
                    &TokenKind::Delim(Delim::RParen),
                    "Expected ')' after arguments",
                )?;

                let span = Span::merge(&start_span, &end_tok.span);

                Ok(Expr::Call {
                    callee: Box::new(left),
                    args,
                    span,
                })
            }

            TokenKind::Delim(Delim::LBracket) => {
                let index = self.expression(0)?;

                let end_tok = self.consume(
                    &TokenKind::Delim(Delim::RBracket),
                    "Expected ']' after index",
                )?;

                let span = Span::merge(&start_span, &end_tok.span);

                Ok(Expr::Index {
                    object: Box::new(left),
                    index: Box::new(index),
                    span,
                })
            }
            TokenKind::Keyword(Keyword::As) => {
                let target_type = self.parse_type()?;
                let end_span = self.prev().span.clone();

                let span = Span::merge(&start_span, &end_span);

                Ok(Expr::Cast {
                    object: Box::new(left),
                    target_type,
                    span,
                })
            }

            _ => anyhow::bail!("[{}] Unexpected token: {:?}", token.span, token.kind),
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

    fn var_decl(&mut self, is_public: bool) -> Result<Stmt> {
        let start_tok = self.peek();
        let start_span = start_tok.span.clone();
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
            self.next();

            Some(self.parse_type()?)
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

        let end_tok = self.consume(
            &TokenKind::Delim(Delim::Semicolon),
            "Expected ';' after variable declaration",
        )?;

        let span = Span::merge(&start_span, &end_tok.span);

        Ok(Stmt::VarDecl {
            name,
            type_annotation,
            value: Box::new(value),
            is_mutable,
            is_public,
            span,
        })
    }

    fn block_expr(&mut self) -> Result<Expr> {
        let start_tok = self
            .consume(
                &TokenKind::Delim(Delim::LCurly),
                "Expected '{' before block",
            )?
            .clone();

        self.block_expr_from_token(start_tok.span)
    }

    fn block_expr_from_token(&mut self, start_span: Span) -> Result<Expr> {
        let mut stmts = vec![];

        while !self.is_at_end() && self.peek().kind != TokenKind::Delim(Delim::RCurly) {
            stmts.push(self.parse_stmt()?);
        }

        let end_tok = self.consume(&TokenKind::Delim(Delim::RCurly), "Expected '}' after block")?;

        let span = Span::merge(&start_span, &end_tok.span);

        Ok(Expr::Block(stmts, span))
    }

    fn if_expr(&mut self, start_span: Span) -> Result<Expr> {
        let cond = self.parse_expression()?;

        self.consume(
            &TokenKind::Delim(Delim::Semicolon),
            "Expected ';' after condition",
        )?;
        self.consume(
            &TokenKind::Keyword(Keyword::Then),
            "Expected 'then' after condition",
        )?;

        let then_branch = self.block_expr()?;

        let else_branch = if self.peek().kind == TokenKind::Keyword(Keyword::Else) {
            self.next();

            if self.peek().kind == TokenKind::Keyword(Keyword::If) {
                let else_if_span = self.next().span.clone();
                Some(Box::new(self.if_expr(else_if_span)?))
            } else {
                Some(Box::new(self.block_expr()?))
            }
        } else {
            None
        };

        // Span do if = do 'if' até o fim do else/then
        let end_span = else_branch
            .as_ref()
            .map(|e| e.span())
            .unwrap_or_else(|| then_branch.span());

        let span = Span::merge(&start_span, &end_span);

        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch,
            span,
        })
    }

    fn while_stmt(&mut self) -> Result<Stmt> {
        let start_span = self.next().span.clone();

        let cond = self.parse_expression()?;

        self.consume(
            &TokenKind::Delim(Delim::Semicolon),
            "Expected ';' after condition",
        )?;

        self.consume(
            &TokenKind::Keyword(Keyword::Do),
            "Expected 'do' after condition in while statements",
        )?;

        let body = self.block_expr()?;

        let span = Span::merge(&start_span, &body.span());

        Ok(Stmt::While {
            cond: Box::new(cond),
            body: Box::new(body),
            span,
        })
    }

    fn return_stmt(&mut self) -> Result<Stmt> {
        let start_span = self.next().span.clone();

        let ret_val = if self.peek().kind != TokenKind::Delim(Delim::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let end_tok = self.consume(
            &TokenKind::Delim(Delim::Semicolon),
            "Expected ';' after return",
        )?;

        let span = Span::merge(&start_span, &end_tok.span);

        Ok(Stmt::Return(ret_val, span))
    }

    fn struct_decl(&mut self, is_public: bool) -> Result<Stmt> {
        let start_span = self.next().span.clone();

        let name = self
            .consume(
                &TokenKind::Ident,
                "Expected identifier after 'struct' in struct declarations",
            )?
            .lexeme
            .clone();
        self.consume(&TokenKind::Delim(Delim::LCurly), "Expected '{'")?;

        let mut fields = vec![];

        if self.peek().kind != TokenKind::Delim(Delim::RCurly) {
            loop {
                if self.peek().kind == TokenKind::Delim(Delim::RCurly) {
                    break;
                }

                let field_name = self
                    .consume(&TokenKind::Ident, "Expected field name")?
                    .lexeme
                    .clone();

                self.consume(&TokenKind::Delim(Delim::Colon), "Expected ':'")?;

                let field_type = self.parse_type()?;

                fields.push((field_name, field_type));

                if self.peek().kind == TokenKind::Delim(Delim::Comma) {
                    self.next();
                } else {
                    break;
                }
            }
        }

        let end_span = self
            .consume(&TokenKind::Delim(Delim::RCurly), "Expected '}'")?
            .span
            .clone();

        Ok(Stmt::StructDecl {
            name,
            fields,
            is_public,
            span: Span::merge(&start_span, &end_span),
        })
    }

    fn break_stmt(&mut self) -> Result<Stmt> {
        let start_span = self.next().span.clone();
        let end_tok = self.consume(
            &TokenKind::Delim(Delim::Semicolon),
            "Expected ';' after break",
        )?;
        let span = Span::merge(&start_span, &end_tok.span);
        Ok(Stmt::Break(span))
    }

    fn continue_stmt(&mut self) -> Result<Stmt> {
        let start_span = self.next().span.clone();
        let end_tok = self.consume(
            &TokenKind::Delim(Delim::Semicolon),
            "Expected ';' after break",
        )?;
        let span = Span::merge(&start_span, &end_tok.span);
        Ok(Stmt::Continue(span))
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        match self.peek().kind {
            TokenKind::Keyword(Keyword::Pub) => {
                self.next();
                match self.peek().kind {
                    TokenKind::Keyword(Keyword::Def | Keyword::Mut) => self.var_decl(true),
                    TokenKind::Keyword(Keyword::Struct) => self.struct_decl(true),
                    _ => anyhow::bail!(
                        "[{}] keyword `pub` can be only used before `def` or `mut`",
                        self.prev().span
                    ),
                }
            }
            TokenKind::Keyword(Keyword::Def | Keyword::Mut) => self.var_decl(false),
            TokenKind::Keyword(Keyword::While) => self.while_stmt(),
            TokenKind::Keyword(Keyword::Return) => self.return_stmt(),
            TokenKind::Keyword(Keyword::Struct) => self.struct_decl(false),
            TokenKind::Keyword(Keyword::Break) => self.break_stmt(),
            TokenKind::Keyword(Keyword::Continue) => self.continue_stmt(),
            _ => {
                let expr = self.parse_expression()?;

                match &expr {
                    Expr::If { .. } => {}
                    _ => {
                        self.consume(
                            &TokenKind::Delim(Delim::Semicolon),
                            "Expected ';' after expression",
                        )?;
                    }
                }
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
