use std::{cell::RefCell, rc::Rc};

use crate::{
    compiler::{chunk::Chunk, error::CompileError, opcode::OpCode},
    lexer::types::{Op, Span},
    typechecker::types::{Type, TypedAssignTarget, TypedAst, TypedExpr, TypedStmt},
    vm::value::{Closure, Value},
};

pub mod chunk;
mod error;
pub mod opcode;

// === Local ===
#[derive(Clone)]
struct Local {
    pub name: String,
    pub slot: u8,
    pub depth: u32,
}

#[derive(Clone)]
pub struct Upvalue {
    pub slot: u8,
    pub is_local: bool,
}

pub enum Resolution {
    Local(u8),
    Upvalue(u8),
    Global,
}

// === Compiler ===
#[derive(Clone)]
pub struct Compiler {
    chunk: Chunk,
    scope_depth: u32,
    locals: Vec<Local>,
    pub_locals: Vec<(String, u8)>,
    func_depth: u32,
    breaks: Vec<usize>,
    continues: Vec<usize>,
    parent: Option<Rc<RefCell<Compiler>>>,
    upvalues: Vec<Upvalue>,
}

impl Compiler {
    pub fn new() -> Self {
        let mut compiler = Self {
            chunk: Chunk::new(),
            scope_depth: 0,
            locals: vec![],
            pub_locals: vec![],
            func_depth: 0,
            breaks: vec![],
            continues: vec![],
            parent: None,
            upvalues: vec![],
        };
        compiler.register_builtins();
        compiler
    }

    fn new_inner() -> Self {
        Self {
            chunk: Chunk::new(),
            scope_depth: 0,
            locals: vec![],
            pub_locals: vec![],
            func_depth: 0,
            breaks: vec![],
            continues: vec![],
            parent: None,
            upvalues: vec![],
        }
    }

    fn register_builtins(&mut self) {
        self.add_local("import".to_string());
    }

    fn emit(&mut self, opcode: OpCode, span: Span) {
        self.chunk.emit(opcode, span);
    }

    fn emit_jump(&mut self, opcode: OpCode, span: Span) -> usize {
        self.chunk.emit_jump(opcode, span)
    }

    fn internal(&self, span: Span, msg: impl Into<String>) -> CompileError {
        CompileError::Internal {
            span,
            msg: msg.into(),
        }
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self, span: &Span, preserve_top: bool) {
        self.scope_depth -= 1;

        let mut locals_to_pop = 0u8;
        while let Some(local) = self.locals.last() {
            if local.depth > self.scope_depth {
                self.locals.pop();
                locals_to_pop += 1;
            } else {
                break;
            }
        }

        if locals_to_pop == 0 {
            return;
        }

        if preserve_top {
            self.emit(OpCode::Rotate(locals_to_pop), span.clone());
        }

        // Pop os locais (agora no topo)
        for _ in 0..locals_to_pop {
            self.emit(OpCode::Pop, span.clone());
        }
    }

    fn resolve_var(&mut self, name: &str) -> Resolution {
        if let Some(local) = self.find_local(name) {
            return Resolution::Local(local.slot);
        }

        if let Some(parent) = &self.parent {
            let resolved_in_parent = parent.borrow_mut().resolve_var(name);

            match resolved_in_parent {
                Resolution::Local(slot) => {
                    let idx = self.add_upvalue(slot, true);
                    Resolution::Upvalue(idx)
                }
                Resolution::Upvalue(slot) => {
                    let idx = self.add_upvalue(slot, false);
                    Resolution::Upvalue(idx)
                }
                Resolution::Global => Resolution::Global,
            }
        } else {
            Resolution::Global
        }
    }

    fn add_upvalue(&mut self, slot: u8, is_local: bool) -> u8 {
        for (i, upv) in self.upvalues.iter().enumerate() {
            if upv.slot == slot && upv.is_local == is_local {
                return i as u8;
            }
        }
        let idx = self.upvalues.len() as u8;
        self.upvalues.push(Upvalue { slot, is_local });
        idx
    }

    fn find_local(&self, name: &str) -> Option<&Local> {
        self.locals.iter().rev().find(|l| l.name == name)
    }

    fn add_local(&mut self, name: String) {
        let slot = self.locals.len() as u8;
        self.locals.push(Local {
            name,
            slot,
            depth: self.scope_depth,
        });
    }

    // === Expressions ===

    fn compile_expr(&mut self, expr: &TypedExpr) -> Result<(), CompileError> {
        match expr {
            TypedExpr::Int(n, span) => {
                let idx = self.chunk.add_const(Value::Int(*n));
                self.emit(OpCode::Const(idx), span.clone());
            }
            TypedExpr::Float(n, span) => {
                let idx = self.chunk.add_const(Value::Float(*n));
                self.emit(OpCode::Const(idx), span.clone());
            }
            TypedExpr::String(s, span) => {
                let idx = self.chunk.add_const(Value::String(s.clone()));
                self.emit(OpCode::Const(idx), span.clone());
            }
            TypedExpr::Bool(b, span) => {
                let op = if *b { OpCode::True } else { OpCode::False };
                self.emit(op, span.clone());
            }
            TypedExpr::Nil(span) => {
                self.emit(OpCode::Nil, span.clone());
            }
            TypedExpr::Ident(name, _, span) => match self.resolve_var(name) {
                Resolution::Local(slot) => self.emit(OpCode::GetLocal(slot), span.clone()),
                Resolution::Upvalue(idx) => self.emit(OpCode::GetUpvalue(idx), span.clone()),
                Resolution::Global => {
                    return Err(CompileError::UndefinedVariable {
                        name: name.clone(),
                        span: span.clone(),
                    });
                }
            },
            TypedExpr::ArrayLiteral(elements, _, span) => {
                for element in elements.iter() {
                    self.compile_expr(element)?;
                }
                self.emit(OpCode::Array(elements.len()), span.clone());
            }
            TypedExpr::Unary {
                op, right, span, ..
            } => {
                // ++x / --x: target deve ser Ident (local ou upvalue map)
                if matches!(op, Op::PlusPlus | Op::MinusMinus) {
                    match right.as_ref() {
                        TypedExpr::Ident(name, _, _) => {
                            let slot = match self.resolve_var(name) {
                                Resolution::Local(s) => s,
                                Resolution::Upvalue(idx) => {
                                    let opcode = match op {
                                        Op::PlusPlus => OpCode::IncrementUpvalue(idx),
                                        Op::MinusMinus => OpCode::DecrementUpvalue(idx),
                                        _ => unreachable!("typechecker bug"),
                                    };
                                    self.emit(opcode, span.clone());
                                    return Ok(());
                                }
                                Resolution::Global => {
                                    return Err(CompileError::UndefinedVariable {
                                        name: name.clone(),
                                        span: span.clone(),
                                    });
                                }
                            };
                            let opcode = match op {
                                Op::PlusPlus => OpCode::Increment(slot),
                                Op::MinusMinus => OpCode::Decrement(slot),
                                _ => unreachable!("filtered above"),
                            };
                            self.emit(opcode, span.clone());
                            return Ok(());
                        }
                        _ => {
                            return Err(CompileError::InvalidUnaryOp {
                                op: format!("operador {:?} requer um identificador simples", op),
                                span: span.clone(),
                            });
                        }
                    }
                }
                // Neg / Not: operadores unários puros
                self.compile_expr(right)?;
                let opcode = match op {
                    Op::Sub => OpCode::Neg,
                    Op::Not => OpCode::Not,
                    other => {
                        return Err(CompileError::InvalidUnaryOp {
                            op: format!("{:?}", other),
                            span: span.clone(),
                        });
                    }
                };
                self.emit(opcode, span.clone());
            }
            TypedExpr::Binary {
                op,
                left,
                right,
                span,
                ..
            } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                let opcode = match op {
                    Op::Add => OpCode::Add,
                    Op::Sub => OpCode::Sub,
                    Op::Mul => OpCode::Mul,
                    Op::Div => OpCode::Div,
                    Op::Mod => OpCode::Mod,
                    Op::Pow => OpCode::Pow,
                    Op::Eq => OpCode::Eq,
                    Op::Ne => OpCode::Neq,
                    Op::Lt => OpCode::Lt,
                    Op::Le => OpCode::Le,
                    Op::Gt => OpCode::Gt,
                    Op::Ge => OpCode::Ge,
                    Op::And => OpCode::And,
                    Op::Or => OpCode::Or,
                    Op::BitAnd => OpCode::BitAnd,
                    Op::BitOr => OpCode::BitOr,
                    Op::BitXor => OpCode::BitXor,
                    Op::Shl => OpCode::Shl,
                    Op::Shr => OpCode::Shr,
                    Op::Assign | Op::PlusPlus | Op::MinusMinus | Op::Not => {
                        return Err(CompileError::InvalidBinaryOp {
                            op: format!("{:?}", op),
                            span: span.clone(),
                        });
                    }
                };
                self.emit(opcode, span.clone());
            }
            TypedExpr::Assign {
                target,
                value,
                span,
                ..
            } => {
                self.compile_expr(value)?;

                match target {
                    TypedAssignTarget::Ident(name, _) => match self.resolve_var(name) {
                        Resolution::Local(slot) => self.emit(OpCode::SetLocal(slot), span.clone()),
                        Resolution::Upvalue(idx) => {
                            self.emit(OpCode::SetUpvalue(idx), span.clone())
                        }
                        Resolution::Global => {
                            return Err(CompileError::UndefinedVariable {
                                name: name.clone(),
                                span: span.clone(),
                            });
                        }
                    },
                    TypedAssignTarget::Property {
                        object,
                        prop,
                        span: pspan,
                    } => {
                        self.compile_expr(object)?;
                        self.emit(OpCode::SetProperty(prop.clone()), pspan.clone());
                    }
                    TypedAssignTarget::Index {
                        object,
                        index,
                        span: ispan,
                    } => {
                        self.compile_expr(object)?;
                        self.compile_expr(index)?;
                        self.emit(OpCode::IndexSet, ispan.clone());
                    }
                }
            }
            TypedExpr::Property {
                object, prop, span, ..
            } => {
                self.compile_expr(object)?;
                self.emit(OpCode::GetProperty(prop.clone()), span.clone());
            }
            TypedExpr::Index {
                object,
                index,
                span,
                ..
            } => {
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.emit(OpCode::IndexGet, span.clone());
            }
            TypedExpr::Block(stmts, _, span) => {
                self.begin_scope();
                self.compile_stmts(stmts, true)?;

                let produced = matches!(stmts.last(), Some(TypedStmt::Expr(_)));
                self.end_scope(span, produced);

                if !produced {
                    self.emit(OpCode::Nil, span.clone());
                }
            }
            TypedExpr::If {
                cond,
                then_branch,
                else_branch,
                span,
                ..
            } => {
                self.compile_expr(cond)?;
                let jump_else = self.emit_jump(OpCode::JumpIfFalse(0), span.clone());

                self.begin_scope();
                self.compile_expr(then_branch)?;
                self.end_scope(span, true);

                let jump_end = self.emit_jump(OpCode::Jump(0), span.clone());
                self.chunk.patch_jump(jump_else);

                if let Some(branch) = else_branch {
                    self.begin_scope();
                    self.compile_expr(branch)?;
                    self.end_scope(span, true);
                } else {
                    self.emit(OpCode::Nil, span.clone());
                }

                self.chunk.patch_jump(jump_end);
            }
            TypedExpr::Cast {
                object,
                target_type,
                span,
            } => {
                self.compile_expr(object)?;
                let opcode = match target_type {
                    Type::Int => OpCode::AsInt,
                    Type::Float => OpCode::AsFloat,
                    other => {
                        return Err(self
                            .internal(span.clone(), format!("invalid cast target: {:?}", other)));
                    }
                };
                self.emit(opcode, span.clone());
            }
            TypedExpr::Func {
                params, body, span, ..
            } => {
                self.func_depth += 1;

                let mut inner = Compiler::new_inner();
                inner.func_depth = self.func_depth;

                for param in params {
                    inner.add_local(param.name.clone());
                }

                inner.parent = Some(Rc::new(RefCell::new(self.clone())));
                inner.compile_expr(body)?;
                inner.emit(OpCode::Return, span.clone());

                let captured_count = inner.upvalues.len() as u8;
                let chunk = inner.chunk;

                self.func_depth -= 1;

                let idx = self.chunk.add_const(Value::Closure(Closure {
                    chunk,
                    upvalues: vec![],
                    upvalues_specs: inner.upvalues.clone(),
                })); // upvalues placeholder

                self.emit(OpCode::Closure(idx, captured_count), span.clone());
            }
            TypedExpr::Call {
                callee, args, span, ..
            } => {
                for arg in args.iter() {
                    self.compile_expr(arg)?;
                }

                // callee depois
                self.compile_expr(callee)?;

                self.emit(OpCode::Call(args.len() as u8), span.clone());
            }
            TypedExpr::Struct { name, fields, span } => {
                for (f_name, f_expr) in fields.iter().rev() {
                    // expr primeiro pois vm faz pop primeiro em name (LIFO)
                    self.compile_expr(f_expr)?;

                    let idx = self.chunk.add_const(Value::String(f_name.clone()));
                    self.emit(OpCode::Const(idx), span.clone());
                }
                self.emit(
                    OpCode::Struct(name.clone(), fields.len() as u8),
                    span.clone(),
                );
            }
            TypedExpr::Path {
                namespace,
                member,
                span,
                ..
            } => {
                self.compile_expr(namespace)?;
                self.emit(OpCode::GetProperty(member.clone()), span.clone());
            }
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &TypedStmt, discard: bool) -> Result<(), CompileError> {
        match stmt {
            TypedStmt::Expr(expr) => {
                let span = expr.span();
                self.compile_expr(expr)?;
                if discard {
                    self.emit(OpCode::Pop, span);
                }
            }
            TypedStmt::Return(ret_expr, span) => {
                if self.func_depth == 0 {
                    return Err(CompileError::ReturnOutsideFunction { span: span.clone() });
                }
                if let Some(expr) = ret_expr {
                    self.compile_expr(expr)?;
                } else {
                    self.emit(OpCode::Nil, span.clone());
                }
                self.emit(OpCode::Return, span.clone());
            }
            TypedStmt::VarDecl {
                name,
                value,
                is_public,
                span,
                ..
            } => {
                if *is_public {
                    if self.scope_depth > 0 {
                        return Err(CompileError::PubInScope { span: span.clone() });
                    }
                    self.pub_locals
                        .push((name.clone(), self.locals.len() as u8));
                }

                if matches!(value.as_ref(), TypedExpr::Func { .. }) {
                    self.add_local(name.clone());
                    self.emit(OpCode::Nil, span.clone());

                    self.compile_expr(value)?;

                    if let Resolution::Local(slot) = self.resolve_var(name) {
                        self.emit(OpCode::SetLocal(slot), span.clone());
                        self.emit(OpCode::Pop, span.clone()); // ← remove closure extra da stack
                    }
                } else {
                    self.compile_expr(value)?;
                    self.add_local(name.clone());
                }
            }
            TypedStmt::While { cond, body, span } => {
                let loop_start = self.chunk.code.len();

                self.compile_expr(cond)?;
                let exit_jump = self.emit_jump(OpCode::JumpIfFalse(0), span.clone());

                let old_breaks = std::mem::take(&mut self.breaks);
                let old_continues = std::mem::take(&mut self.continues);

                self.begin_scope();
                self.compile_expr(body)?;
                self.end_scope(span, false);

                self.emit(OpCode::Pop, span.clone()); // limpa stack

                self.emit(OpCode::Jump(loop_start), span.clone());
                self.chunk.patch_jump(exit_jump);

                for brk in &self.breaks {
                    self.chunk.patch_jump(*brk);
                }
                for cont in &self.continues {
                    self.chunk.code[*cont] = OpCode::Jump(loop_start);
                }

                self.breaks = old_breaks;
                self.continues = old_continues;
            }
            TypedStmt::StructDecl { .. } => {}
            TypedStmt::Break(span) => {
                let offset = self.emit_jump(OpCode::Jump(0), span.clone());
                self.breaks.push(offset);
            }
            TypedStmt::Continue(span) => {
                let offset = self.emit_jump(OpCode::Jump(0), span.clone());
                self.continues.push(offset);
            }
        }
        Ok(())
    }

    fn compile_stmts(&mut self, stmts: &[TypedStmt], keep_last: bool) -> Result<(), CompileError> {
        let last_idx = stmts.len().saturating_sub(1);
        for (i, stmt) in stmts.iter().enumerate() {
            let discard = if keep_last { i < last_idx } else { true };
            self.compile_stmt(stmt, discard)?;
        }
        Ok(())
    }

    pub fn compile(
        file: &str,
        typed_ast: &TypedAst,
    ) -> Result<(Chunk, Vec<(String, u8)>), CompileError> {
        let _ = file;
        let mut compiler = Compiler::new();
        compiler.compile_stmts(&typed_ast.stmts, false)?;
        Ok((compiler.chunk, compiler.pub_locals))
    }
}
