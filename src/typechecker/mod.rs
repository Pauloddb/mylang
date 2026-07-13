mod env;
pub mod error;
pub mod registry;
pub mod types;

use std::{cell::RefCell, rc::Rc};

use crate::{
    lexer::types::Op,
    parser::types::{AssignTarget, Ast, Expr, Stmt},
    properties::property_info,
    typechecker::{
        env::{Binding, TypeEnv},
        error::TypeError,
        registry::TypeRegistry,
        types::{Type, TypedAst, TypedExpr, TypedParam, TypedStmt},
    },
};

pub struct TypeChecker {
    env: RefCell<Rc<TypeEnv>>,
    registry: Rc<RefCell<TypeRegistry>>,
    current_ret: RefCell<Option<Type>>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: RefCell::new(Rc::new(TypeEnv::new())),
            registry: Rc::new(RefCell::new(TypeRegistry::new())),
            current_ret: RefCell::new(None),
        }
    }

    fn push_scope(&self) {
        let current = self.env.borrow().clone();
        let child = TypeEnv::child(&current);
        self.env.replace(child);
    }

    fn pop_scope(&self) {
        let parent = self.env.borrow().parent.clone().unwrap();
        self.env.replace(parent);
    }

    ///Allows shadowing
    fn define_var(&self, name: String, ty: &Type, is_mutable: bool) {
        self.env.borrow().define(name, ty.clone(), is_mutable)
    }

    fn lookup_var(&self, name: &str) -> Option<Binding> {
        self.env.borrow().lookup(name)
    }

    fn infer_expr(&self, expr: &Expr, expected: Option<&Type>) -> Result<Type, TypeError> {
        match expr {
            Expr::Int(_, _) => Ok(Type::Int),
            Expr::Float(_, _) => Ok(Type::Float),
            Expr::Bool(_, _) => Ok(Type::Bool),
            Expr::Nil(_) => Ok(Type::Void),
            Expr::String(_, _) => Ok(Type::String),
            Expr::Ident(name, span) => Ok(self
                .lookup_var(name)
                .ok_or(TypeError::UndefinedVar {
                    name: name.clone(),
                    span: *span,
                })?
                .ty),
            Expr::ArrayLiteral(elements, span) => {
                if elements.is_empty() {
                    if let Some(ty) = expected {
                        return Ok(ty.clone());
                    } else {
                        return Err(TypeError::AmbiguousArrayType { span: *span });
                    }
                }

                let first_ty = self.infer_expr(&elements[0], None)?;

                for elem in elements.iter().skip(1) {
                    let ty = self.infer_expr(elem, None)?;

                    if ty != first_ty {
                        return Err(TypeError::Mismatch {
                            expected: first_ty.to_string(),
                            found: ty.to_string(),
                            span: *span,
                        });
                    }
                }

                Ok(Type::Array(Box::new(first_ty)))
            }
            Expr::Func {
                params,
                ret_ty,
                span,
                ..
            } => {
                let mut param_types = vec![];
                for p in params.iter() {
                    param_types.push(Type::from_str(&p.ty, *span, &self.registry.borrow())?);
                }

                let ret = Type::from_str(ret_ty, *span, &self.registry.borrow())?;
                self.current_ret.borrow_mut().replace(ret.clone());

                Ok(Type::Func {
                    params: param_types,
                    ret: Box::new(ret),
                })
            }
            Expr::Struct { name, fields, span } => {
                let typed_fields = fields
                    .iter()
                    .map(|(f_name, f_ty)| {
                        let ty = self.infer_expr(f_ty, None)?;
                        Ok((f_name.clone(), ty))
                    })
                    .collect::<Result<Vec<_>, TypeError>>()?;

                let registry = self.registry.borrow();
                let struct_fields = registry.get_fields(name).ok_or(TypeError::UnknownType {
                    name: name.clone(),
                    span: *span,
                })?;

                let struct_type = registry.resolve(name).ok_or(TypeError::UnknownType {
                    name: name.clone(),
                    span: *span,
                })?;

                if typed_fields != *struct_fields {
                    return Err(TypeError::Mismatch {
                        expected: struct_type.to_string(),
                        found: Type::Struct(typed_fields.clone()).to_string(),
                        span: *span,
                    });
                }

                Ok(struct_type)
            }
            Expr::Assign {
                target,
                value,
                span,
            } => {
                let value_ty = self.infer_expr(value, None)?;

                match target {
                    AssignTarget::Ident(name, _) => {
                        let binding = self
                            .lookup_var(name)
                            .ok_or(TypeError::UndefinedVar {
                                name: name.clone(),
                                span: *span,
                            })?
                            .clone();

                        if !binding.is_mutable {
                            return Err(TypeError::ImmutableAssign {
                                name: name.clone(),
                                span: *span,
                            });
                        }

                        if binding.ty != value_ty {
                            return Err(TypeError::Mismatch {
                                expected: binding.ty.to_string(),
                                found: value_ty.to_string(),
                                span: *span,
                            });
                        }

                        Ok(value_ty)
                    }
                    AssignTarget::Property { object, prop, span } => {
                        let obj_ty = self.infer_expr(object, None)?;

                        let prop_info = property_info(&obj_ty, prop, *span)?;

                        if !prop_info.is_mutable || !prop_info.needs_mutable_owner {
                            return Err(TypeError::ImmutableAssign {
                                name: format!("{}.{}", obj_ty, prop),
                                span: *span,
                            });
                        }

                        if prop_info.ty != value_ty {
                            return Err(TypeError::Mismatch {
                                expected: prop_info.ty.to_string(),
                                found: value_ty.to_string(),
                                span: *span,
                            });
                        }

                        Ok(value_ty)
                    }
                    AssignTarget::Index {
                        object,
                        index,
                        span,
                    } => {
                        let obj_ty = self.infer_expr(object, None)?;
                        let index_ty = self.infer_expr(index, None)?;

                        if index_ty != Type::Int {
                            return Err(TypeError::Mismatch {
                                expected: Type::Int.to_string(),
                                found: index_ty.to_string(),
                                span: *span,
                            });
                        }

                        let elem_ty = match &obj_ty {
                            Type::String => Type::String,
                            Type::Array(elem) => *elem.clone(),
                            _ => {
                                return Err(TypeError::NotIndexable {
                                    ty: obj_ty.to_string(),
                                    span: *span,
                                });
                            }
                        };

                        if elem_ty != value_ty {
                            return Err(TypeError::Mismatch {
                                expected: elem_ty.to_string(),
                                found: value_ty.to_string(),
                                span: *span,
                            });
                        }

                        Ok(obj_ty)
                    }
                }
            }
            Expr::Unary { op, right, span } => {
                let right_ty = self.infer_expr(right, None)?;

                match (op, &right_ty) {
                    (Op::Sub | Op::MinusMinus | Op::PlusPlus, Type::Int) => Ok(Type::Int),
                    (Op::Sub | Op::MinusMinus | Op::PlusPlus, Type::Float) => Ok(Type::Float),
                    (Op::Not, Type::Bool) => Ok(Type::Bool),
                    _ => Err(TypeError::InvalidUnaryOp {
                        op: format!("{:?}", op),
                        operand: right_ty.to_string(),
                        span: *span,
                    }),
                }
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let left_ty = self.infer_expr(left, None)?;
                let right_ty = self.infer_expr(right, None)?;

                match (op, &left_ty, &right_ty) {
                    (Op::Add, Type::Int, Type::Int) => Ok(Type::Int),
                    (Op::Add, Type::Float, Type::Int) => Ok(Type::Float),
                    (Op::Add, Type::Int, Type::Float) => Ok(Type::Float),
                    (Op::Add, Type::Float, Type::Float) => Ok(Type::Float),

                    (Op::Sub, Type::Int, Type::Int) => Ok(Type::Int),
                    (Op::Sub, Type::Float, Type::Int) => Ok(Type::Float),
                    (Op::Sub, Type::Int, Type::Float) => Ok(Type::Float),
                    (Op::Sub, Type::Float, Type::Float) => Ok(Type::Float),

                    (Op::Mul, Type::Int, Type::Int) => Ok(Type::Int),
                    (Op::Mul, Type::Float, Type::Int) => Ok(Type::Float),
                    (Op::Mul, Type::Int, Type::Float) => Ok(Type::Float),
                    (Op::Mul, Type::Float, Type::Float) => Ok(Type::Float),

                    (Op::Div, Type::Int, Type::Int) => Ok(Type::Float),
                    (Op::Div, Type::Float, Type::Int) => Ok(Type::Float),
                    (Op::Div, Type::Int, Type::Float) => Ok(Type::Float),
                    (Op::Div, Type::Float, Type::Float) => Ok(Type::Float),

                    (Op::Mod, Type::Int, Type::Int) => Ok(Type::Int),
                    (Op::Mod, Type::Float, Type::Int) => Ok(Type::Float),
                    (Op::Mod, Type::Int, Type::Float) => Ok(Type::Float),
                    (Op::Mod, Type::Float, Type::Float) => Ok(Type::Float),

                    (Op::Pow, Type::Int, Type::Int) => Ok(Type::Int),
                    (Op::Pow, Type::Float, Type::Int) => Ok(Type::Float),
                    (Op::Pow, Type::Int, Type::Float) => Ok(Type::Float),
                    (Op::Pow, Type::Float, Type::Float) => Ok(Type::Float),

                    (Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge, lty, rty) => {
                        if lty == rty
                            || ((lty == &Type::Int && rty == &Type::Float)
                                || (lty == &Type::Float && rty == &Type::Int))
                        {
                            Ok(Type::Bool)
                        } else {
                            Err(TypeError::InvalidComparison {
                                left: left_ty.to_string(),
                                right: right_ty.to_string(),
                                span: *span,
                            })
                        }
                    }

                    (Op::BitAnd | Op::BitOr | Op::BitXor, Type::Bool, Type::Bool) => Ok(Type::Bool),
                    (
                        Op::BitAnd | Op::BitOr | Op::BitXor | Op::Shl | Op::Shr,
                        Type::Int,
                        Type::Int,
                    ) => Ok(Type::Int),

                    (Op::And | Op::Or, Type::Bool, Type::Bool) => Ok(Type::Bool),

                    _ => Err(TypeError::InvalidBinaryOp {
                        op: format!("{:?}", op),
                        left: left_ty.to_string(),
                        right: right_ty.to_string(),
                        span: *span,
                    }),
                }
            }
            Expr::Call { callee, args, span } => {
                let callee_ty = self.infer_expr(callee, None)?;

                if let Type::Func { params, ret } = callee_ty {
                    if args.len() != params.len() {
                        return Err(TypeError::ArgCountMismatch {
                            expected: params.len(),
                            found: args.len(),
                            span: *span,
                        });
                    }

                    let typed_args = args
                        .iter()
                        .map(|arg| self.infer_expr(arg, None))
                        .collect::<Result<Vec<_>, TypeError>>()?;

                    for (p, a) in params.iter().zip(typed_args.iter()) {
                        if p != a {
                            return Err(TypeError::Mismatch {
                                expected: p.to_string(),
                                found: a.to_string(),
                                span: *span,
                            });
                        }
                    }

                    Ok(*ret)
                } else {
                    Err(TypeError::Mismatch {
                        expected: "func".into(),
                        found: callee_ty.to_string(),
                        span: *span,
                    })
                }
            }
            Expr::Property { object, prop, span } => {
                let obj_ty = self.infer_expr(object, None)?;
                let prop_info = property_info(&obj_ty, prop, *span)?;
                Ok(prop_info.ty)
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let cond_ty = self.infer_expr(cond, None)?;

                if cond_ty != Type::Bool {
                    return Err(TypeError::Mismatch {
                        expected: Type::Bool.to_string(),
                        found: cond_ty.to_string(),
                        span: *span,
                    });
                }

                let then_branch_ty = self.infer_expr(then_branch, None)?;

                if let Some(branch) = else_branch {
                    let else_branch_ty = self.infer_expr(branch, None)?;

                    if then_branch_ty != else_branch_ty {
                        return Err(TypeError::Mismatch {
                            expected: then_branch_ty.to_string(),
                            found: else_branch_ty.to_string(),
                            span: *span,
                        });
                    }
                }

                Ok(then_branch_ty)
            }
            Expr::Index {
                object,
                index,
                span,
            } => {
                let obj_ty = self.infer_expr(object, None)?;
                let index_ty = self.infer_expr(index, None)?;

                if index_ty != Type::Int {
                    return Err(TypeError::Mismatch {
                        expected: Type::Int.to_string(),
                        found: index_ty.to_string(),
                        span: *span,
                    });
                }

                match obj_ty {
                    Type::String => Ok(Type::String),
                    Type::Array(elem) => Ok(*elem),
                    _ => Err(TypeError::NotIndexable {
                        ty: obj_ty.to_string(),
                        span: *span,
                    }),
                }
            }
            Expr::Block(stmts, _) => {
                self.push_scope();

                let mut last_ty = Type::Void;

                for stmt in stmts {
                    last_ty = match stmt {
                        Stmt::Expr(expr) => self.infer_expr(expr, None)?,
                        Stmt::VarDecl {
                            name,
                            type_annotation,
                            value,
                            is_mutable,
                            span,
                        } => {
                            let ann_ty = type_annotation
                                .as_ref()
                                .map(|ann| Type::from_str(ann, *span, &self.registry.borrow()))
                                .transpose()?;

                            let value_ty = self.infer_expr(value, ann_ty.as_ref())?;

                            let ty = if let Some(ann) = type_annotation {
                                let ann_ty =
                                    Type::from_str(ann, stmt.span(), &self.registry.borrow())?;

                                if ann_ty != value_ty {
                                    return Err(TypeError::Mismatch {
                                        expected: ann_ty.to_string(),
                                        found: value_ty.to_string(),
                                        span: *span,
                                    });
                                }
                                ann_ty
                            } else {
                                value_ty
                            };

                            self.define_var(name.clone(), &ty, *is_mutable);
                            Type::Void
                        }
                        Stmt::StructDecl { name, fields, span } => {
                            if self.env.borrow().is_inside_scope() {
                                return Err(TypeError::StructDeclInsideScope { span: *span });
                            }

                            let typed_fields: Vec<_> = fields
                                .iter()
                                .map(|(f_name, f_type)| {
                                    let ty =
                                        Type::from_str(f_type, *span, &self.registry.borrow())?;
                                    Ok((f_name.clone(), ty))
                                })
                                .collect::<Result<Vec<_>, TypeError>>()?;

                            self.registry.borrow_mut().register(
                                name.clone(),
                                typed_fields,
                                *span,
                            )?;
                            Type::Void
                        }
                        Stmt::Return(expr, span) => {
                            let has_ret = self.current_ret.borrow().is_some();
                            if has_ret {
                                if let Some(e) = expr {
                                    let ty = self.infer_expr(e, None)?;

                                    let expected = self.current_ret.borrow().clone().unwrap();
                                    if ty != expected {
                                        return Err(TypeError::ReturnMismatch {
                                            expected: expected.to_string(),
                                            found: ty.to_string(),
                                            span: *span,
                                        });
                                    }

                                    return Ok(ty);
                                } else {
                                    return Ok(Type::Void);
                                }
                            } else {
                                return Err(TypeError::ReturnOutsideFunction { span: *span });
                            }
                        }
                        Stmt::While { cond, body, span } => {
                            let cond_ty = self.infer_expr(cond, None)?;

                            if cond_ty != Type::Bool {
                                return Err(TypeError::Mismatch {
                                    expected: Type::Bool.to_string(),
                                    found: cond_ty.to_string(),
                                    span: *span,
                                });
                            }

                            self.infer_expr(body, None)?;
                            Type::Void
                        }
                    };
                }

                self.pop_scope();
                Ok(last_ty)
            }
        }
    }

    fn check_expr(&self, expr: &Expr) -> Result<TypedExpr, TypeError> {
        match expr {
            Expr::Int(n, span) => Ok(TypedExpr::Int(*n, *span)),
            Expr::Float(n, span) => Ok(TypedExpr::Float(*n, *span)),
            Expr::Bool(b, span) => Ok(TypedExpr::Bool(*b, *span)),
            Expr::String(s, span) => Ok(TypedExpr::String(s.clone(), *span)),
            Expr::Nil(span) => Ok(TypedExpr::Nil(*span)),
            Expr::Ident(name, span) => {
                let ty = self.infer_expr(expr, None)?;
                Ok(TypedExpr::Ident(name.clone(), ty, *span))
            }
            Expr::ArrayLiteral(elements, span) => {
                if elements.is_empty() {
                    return Ok(TypedExpr::ArrayLiteral(
                        vec![],
                        Type::Array(Box::new(Type::Void)),
                        *span,
                    ));
                }

                let ty = self.infer_expr(expr, None)?;

                let mut typed_elements = vec![];

                for elem in elements {
                    typed_elements.push(self.check_expr(elem)?);
                }

                Ok(TypedExpr::ArrayLiteral(typed_elements, ty, *span))
            }
            Expr::Func {
                params,
                body,
                name,
                span,
                ..
            } => {
                let func_ty = self.infer_expr(expr, None)?;

                let ret = match &func_ty {
                    Type::Func { ret, .. } => *ret.clone(),
                    _ => unreachable!(),
                };

                self.push_scope();

                let mut typed_params = vec![];
                for p in params.iter() {
                    let param_ty = Type::from_str(&p.ty, *span, &self.registry.borrow())?;

                    self.define_var(p.name.clone(), &param_ty, false);

                    typed_params.push(TypedParam {
                        name: p.name.clone(),
                        ty: param_ty,
                        span: *span,
                    });
                }

                let old_ret = self.current_ret.borrow().clone();

                let typed_body = self.check_expr(body)?;

                *self.current_ret.borrow_mut() = old_ret;
                self.pop_scope();

                Ok(TypedExpr::Func {
                    params: typed_params,
                    body: Box::new(typed_body),
                    name: name.clone(),
                    ret,
                    span: *span,
                })
            }
            Expr::Struct { name, fields, span } => {
                self.infer_expr(expr, None)?; // infer_expr cuida de procurar no registry

                let mut typed_fields = vec![];

                for (f_name, f_expr) in fields {
                    typed_fields.push((f_name.clone(), self.check_expr(f_expr)?));
                }

                Ok(TypedExpr::Struct {
                    name: name.clone(),
                    fields: typed_fields,
                    span: *span,
                })
            }
            Expr::Unary { op, right, span } => {
                let typed_right = self.check_expr(right)?;
                let ty = self.infer_expr(expr, None)?;

                Ok(TypedExpr::Unary {
                    op: *op,
                    right: Box::new(typed_right),
                    ty,
                    span: *span,
                })
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let typed_left = self.check_expr(left)?;
                let typed_right = self.check_expr(right)?;
                let ty = self.infer_expr(expr, None)?;

                Ok(TypedExpr::Binary {
                    op: *op,
                    left: Box::new(typed_left),
                    right: Box::new(typed_right),
                    ty,
                    span: *span,
                })
            }
            Expr::Call { callee, args, span } => {
                let ty = self.infer_expr(expr, None)?;
                let typed_callee = self.check_expr(callee)?;

                let mut typed_args = vec![];

                for arg in args {
                    typed_args.push(self.check_expr(arg)?);
                }

                Ok(TypedExpr::Call {
                    callee: Box::new(typed_callee),
                    args: typed_args,
                    ty,
                    span: *span,
                })
            }
            Expr::Property { object, prop, span } => {
                let ty = self.infer_expr(expr, None)?;
                let typed_obj = self.check_expr(object)?;

                Ok(TypedExpr::Property {
                    object: Box::new(typed_obj),
                    prop: prop.clone(),
                    ty,
                    span: *span,
                })
            }
            Expr::Assign {
                target,
                value,
                span,
            } => {
                let ty = self.infer_expr(expr, None)?;
                let typed_value = self.check_expr(value)?;

                Ok(TypedExpr::Assign {
                    target: target.clone(),
                    value: Box::new(typed_value),
                    ty,
                    span: *span,
                })
            }
            Expr::Block(stmts, span) => {
                let ty = self.infer_expr(expr, None)?;

                let mut typed_stmts = vec![];

                for stmt in stmts {
                    typed_stmts.push(self.check_stmt(stmt)?);
                }

                Ok(TypedExpr::Block(typed_stmts, ty, *span))
            }
            Expr::Index {
                object,
                index,
                span,
            } => {
                let ty = self.infer_expr(expr, None)?;
                let typed_obj = self.check_expr(object)?;
                let typed_index = self.check_expr(index)?;

                Ok(TypedExpr::Index {
                    object: Box::new(typed_obj),
                    index: Box::new(typed_index),
                    ty,
                    span: *span,
                })
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let ty = self.infer_expr(expr, None)?;
                let typed_cond = Box::new(self.check_expr(cond)?);
                let typed_then_branch = Box::new(self.check_expr(then_branch)?);

                let typed_else_branch = if let Some(branch) = else_branch {
                    Some(Box::new(self.check_expr(branch)?))
                } else {
                    None
                };

                Ok(TypedExpr::If {
                    cond: typed_cond,
                    then_branch: typed_then_branch,
                    else_branch: typed_else_branch,
                    ty,
                    span: *span,
                })
            }
        }
    }

    fn check_stmt(&self, stmt: &Stmt) -> Result<TypedStmt, TypeError> {
        match stmt {
            Stmt::Expr(expr) => Ok(TypedStmt::Expr(self.check_expr(expr)?)),
            Stmt::VarDecl {
                name,
                type_annotation,
                value,
                is_mutable,
                span,
            } => {
                let ann_ty = type_annotation
                    .as_ref()
                    .map(|ann| Type::from_str(ann, *span, &self.registry.borrow()))
                    .transpose()?;

                let value_ty = self.infer_expr(value, ann_ty.as_ref())?;

                let ty = if let Some(ann) = &ann_ty {
                    if *ann != value_ty {
                        return Err(TypeError::Mismatch {
                            expected: ann.to_string(),
                            found: value_ty.to_string(),
                            span: *span,
                        });
                    }
                    value_ty
                } else {
                    value_ty
                };

                self.define_var(name.clone(), &ty, *is_mutable);

                let typed_value = self.check_expr(value)?;

                Ok(TypedStmt::VarDecl {
                    name: name.clone(),
                    ty,
                    value: Box::new(typed_value),
                    is_mutable: *is_mutable,
                    span: *span,
                })
            }
            Stmt::StructDecl { name, fields, span } => {
                // Struct declarations must be done in first scope
                if self.env.borrow().is_inside_scope() {
                    return Err(TypeError::StructDeclInsideScope { span: *span });
                }

                let typed_fields = fields
                    .iter()
                    .map(|(f_name, f_ty)| {
                        let ty = Type::from_str(f_ty, *span, &self.registry.borrow())?;
                        Ok((f_name.clone(), ty))
                    })
                    .collect::<Result<Vec<_>, TypeError>>()?;

                self.registry
                    .borrow_mut()
                    .register(name.clone(), typed_fields.clone(), *span)?;

                Ok(TypedStmt::StructDecl {
                    name: name.clone(),
                    fields: typed_fields,
                    span: *span,
                })
            }
            Stmt::While { cond, body, span } => {
                let cond_ty = self.infer_expr(cond, None)?;

                if cond_ty != Type::Bool {
                    return Err(TypeError::Mismatch {
                        expected: Type::Bool.to_string(),
                        found: cond_ty.to_string(),
                        span: *span,
                    });
                }

                let typed_cond = self.check_expr(cond)?;
                let typed_body = self.check_expr(body)?;

                Ok(TypedStmt::While {
                    cond: Box::new(typed_cond),
                    body: Box::new(typed_body),
                    span: *span,
                })
            }
            Stmt::Return(ret_expr, span) => {
                let has_ret = self.current_ret.borrow().is_some();
                if has_ret {
                    if let Some(e) = ret_expr {
                        let ty = self.infer_expr(e, None)?;

                        let expected = self.current_ret.borrow().clone().unwrap();
                        if ty != expected {
                            return Err(TypeError::ReturnMismatch {
                                expected: expected.to_string(),
                                found: ty.to_string(),
                                span: *span,
                            });
                        }

                        let typed_ret_expr = self.check_expr(e)?;

                        *self.current_ret.borrow_mut() = None;
                        Ok(TypedStmt::Return(Some(typed_ret_expr), *span))
                    } else {
                        Ok(TypedStmt::Return(None, *span))
                    }
                } else {
                    Err(TypeError::ReturnOutsideFunction { span: *span })
                }
            }
        }
    }

    pub fn check(&mut self, ast: &Ast) -> Result<TypedAst, TypeError> {
        let mut typed_stmts = vec![];

        for stmt in ast.stmts.iter() {
            typed_stmts.push(self.check_stmt(stmt)?);
        }

        Ok(TypedAst { stmts: typed_stmts })
    }
}
