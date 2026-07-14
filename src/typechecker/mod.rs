mod env;
pub mod error;
pub mod registry;
pub mod types;

use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use mylang::EXTENSION;

use crate::{
    lexer::{
        Lexer,
        types::{Op, Span},
    },
    parser::{
        Parser,
        types::{AssignTarget, Ast, Expr, Stmt},
    },
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
    current_file: RefCell<PathBuf>,
    module_exports: RefCell<Vec<(String, Type)>>,
    module_cache: RefCell<HashMap<String, Type>>,
}

impl TypeChecker {
    pub fn new(path: PathBuf) -> Self {
        Self {
            env: RefCell::new(Rc::new(TypeEnv::new())),
            registry: Rc::new(RefCell::new(TypeRegistry::new())),
            current_ret: RefCell::new(None),
            current_file: RefCell::new(path),
            module_cache: RefCell::new(HashMap::new()),
            module_exports: RefCell::new(vec![]),
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

    fn define_public(&self, name: String, ty: &Type) {
        let mut i = 0;
        let exports = self.module_exports.borrow().clone();

        while i < exports.len() {
            let (exp_name, _) = &exports[i];
            if exp_name == &name {
                self.module_exports.borrow_mut().remove(i);
            }
            i += 1;
        }

        self.module_exports.borrow_mut().push((name, ty.clone()));
    }

    ///Allows shadowing
    fn define_var(&self, name: String, ty: &Type, is_mutable: bool, is_public: bool) {
        if is_public {
            self.define_public(name.clone(), ty);
        }

        self.env
            .borrow()
            .define(name, ty.clone(), is_mutable, is_public)
    }

    fn lookup_var(&self, name: &str) -> Option<Binding> {
        self.env.borrow().lookup(name)
    }

    fn resolve_import(&self, path: String, span: Span) -> Result<Type, TypeError> {
        let cached_key = path.clone();

        if let Some(cached) = self.module_cache.borrow().get(&cached_key) {
            return Ok(cached.clone());
        }

        let dir = self.current_file.borrow().parent().unwrap().to_path_buf();
        let file_path = dir.join(format!("{}.{}", path, EXTENSION));

        let old_file = self.current_file.replace(file_path);
        let old_exports = self.module_exports.replace(vec![]);

        // Processa e garante restore mesmo em erro
        let result = (|| -> Result<Type, TypeError> {
            let content = fs::read_to_string::<&Path>(self.current_file.borrow().as_ref())
                .map_err(|_| TypeError::InvalidImportPath {
                    path: cached_key.clone(),
                    span: span.clone(),
                })?;

            let tokens = Lexer::new(&content, self.current_file.borrow().display().to_string())
                .lex()
                .map_err(|e| TypeError::LexError { msg: e.to_string() })?;

            let ast = Parser::new(tokens)
                .parse()
                .map_err(|e| TypeError::ParseError { msg: e.to_string() })?;

            self.check(&ast)?;

            Ok(Type::Module(vec![])) // placeholder
        })();

        // Restore GARANTIDO
        let module_exports = self.module_exports.replace(old_exports);
        *self.current_file.borrow_mut() = old_file;

        result?;

        let module = Type::Module(module_exports);
        self.module_cache
            .borrow_mut()
            .insert(cached_key, module.clone());

        Ok(module)
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
                    span: span.clone(),
                })?
                .ty),
            Expr::ArrayLiteral(elements, span) => {
                if elements.is_empty() {
                    if let Some(ty) = expected {
                        return Ok(ty.clone());
                    } else {
                        return Err(TypeError::AmbiguousArrayType { span: span.clone() });
                    }
                }

                let first_ty = self.infer_expr(&elements[0], None)?;

                for elem in elements.iter().skip(1) {
                    let ty = self.infer_expr(elem, None)?;

                    if ty != first_ty {
                        return Err(TypeError::Mismatch {
                            expected: first_ty.to_string(),
                            found: ty.to_string(),
                            span: span.clone(),
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
                    param_types.push(Type::from_str(
                        &p.ty,
                        span.clone(),
                        &self.registry.borrow(),
                    )?);
                }

                let ret = Type::from_str(ret_ty, span.clone(), &self.registry.borrow())?;
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
                    span: span.clone(),
                })?;

                let struct_type = registry.resolve(name).ok_or(TypeError::UnknownType {
                    name: name.clone(),
                    span: span.clone(),
                })?;

                if typed_fields != *struct_fields {
                    return Err(TypeError::Mismatch {
                        expected: struct_type.to_string(),
                        found: Type::Struct(typed_fields.clone()).to_string(),
                        span: span.clone(),
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
                                span: span.clone(),
                            })?
                            .clone();

                        if !binding.is_mutable {
                            return Err(TypeError::ImmutableAssign {
                                name: name.clone(),
                                span: span.clone(),
                            });
                        }

                        if binding.ty != value_ty {
                            return Err(TypeError::Mismatch {
                                expected: binding.ty.to_string(),
                                found: value_ty.to_string(),
                                span: span.clone(),
                            });
                        }

                        Ok(value_ty)
                    }
                    AssignTarget::Property { object, prop, span } => {
                        let obj_ty = self.infer_expr(object, None)?;

                        let prop_info = property_info(&obj_ty, prop, span.clone())?;

                        if !prop_info.is_mutable {
                            return Err(TypeError::ImmutableAssign {
                                name: format!("{}.{}", obj_ty, prop),
                                span: span.clone(),
                            });
                        }

                        if prop_info.needs_mutable_owner {
                            if let Expr::Ident(name, _) = object.as_ref() {
                                if let Some(binding) = self.lookup_var(name) {
                                    if !binding.is_mutable {
                                        return Err(TypeError::ImmutableAssign {
                                            name: format!("{}.{}", obj_ty, prop),
                                            span: span.clone(),
                                        });
                                    }
                                }
                            }
                        }

                        if prop_info.ty != value_ty {
                            return Err(TypeError::Mismatch {
                                expected: prop_info.ty.to_string(),
                                found: value_ty.to_string(),
                                span: span.clone(),
                            });
                        }

                        Ok(value_ty)
                    }
                    AssignTarget::Index {
                        object,
                        index,
                        span,
                    } => {
                        if let Expr::Ident(name, _) = object.as_ref() {
                            if let Some(binding) = self.lookup_var(name) {
                                if !binding.is_mutable {
                                    return Err(TypeError::ImmutableAssign {
                                        name: name.clone(),
                                        span: span.clone(),
                                    });
                                }
                            }
                        }

                        let obj_ty = self.infer_expr(object, None)?;
                        let index_ty = self.infer_expr(index, None)?;

                        if index_ty != Type::Int {
                            return Err(TypeError::Mismatch {
                                expected: Type::Int.to_string(),
                                found: index_ty.to_string(),
                                span: span.clone(),
                            });
                        }

                        let elem_ty = match &obj_ty {
                            Type::String => Type::String,
                            Type::Array(elem) => *elem.clone(),
                            _ => {
                                return Err(TypeError::NotIndexable {
                                    ty: obj_ty.to_string(),
                                    span: span.clone(),
                                });
                            }
                        };

                        if elem_ty != value_ty {
                            return Err(TypeError::Mismatch {
                                expected: elem_ty.to_string(),
                                found: value_ty.to_string(),
                                span: span.clone(),
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
                        span: span.clone(),
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
                                span: span.clone(),
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
                        span: span.clone(),
                    }),
                }
            }
            Expr::Call { callee, args, span } => {
                // Import builtin
                if let Expr::Ident(name, _) = callee.as_ref() {
                    if name == "import" {
                        if let Expr::String(path, _) = &args[0] {
                            return self.resolve_import(path.to_string(), span.clone());
                        }
                    }
                }

                let callee_ty = self.infer_expr(callee, None)?;

                if let Type::Func { params, ret } = callee_ty {
                    if args.len() != params.len() {
                        return Err(TypeError::ArgCountMismatch {
                            expected: params.len(),
                            found: args.len(),
                            span: span.clone(),
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
                                span: span.clone(),
                            });
                        }
                    }

                    Ok(*ret)
                } else {
                    Err(TypeError::Mismatch {
                        expected: "func".into(),
                        found: callee_ty.to_string(),
                        span: span.clone(),
                    })
                }
            }
            Expr::Property { object, prop, span } => {
                let obj_ty = self.infer_expr(object, None)?;
                let prop_info = property_info(&obj_ty, prop, span.clone())?;
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
                        span: span.clone(),
                    });
                }

                let then_branch_ty = self.infer_expr(then_branch, None)?;

                if let Some(branch) = else_branch {
                    let else_branch_ty = self.infer_expr(branch, None)?;

                    if then_branch_ty != else_branch_ty {
                        return Err(TypeError::Mismatch {
                            expected: then_branch_ty.to_string(),
                            found: else_branch_ty.to_string(),
                            span: span.clone(),
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
                        span: span.clone(),
                    });
                }

                match obj_ty {
                    Type::String => Ok(Type::String),
                    Type::Array(elem) => Ok(*elem),
                    _ => Err(TypeError::NotIndexable {
                        ty: obj_ty.to_string(),
                        span: span.clone(),
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
                            is_public,
                            span,
                        } => {
                            let ann_ty = type_annotation
                                .as_ref()
                                .map(|ann| {
                                    Type::from_str(ann, span.clone(), &self.registry.borrow())
                                })
                                .transpose()?;

                            let value_ty = self.infer_expr(value, ann_ty.as_ref())?;

                            let ty = if let Some(ann) = type_annotation {
                                let ann_ty =
                                    Type::from_str(ann, stmt.span(), &self.registry.borrow())?;

                                if ann_ty != value_ty {
                                    return Err(TypeError::Mismatch {
                                        expected: ann_ty.to_string(),
                                        found: value_ty.to_string(),
                                        span: span.clone(),
                                    });
                                }
                                ann_ty
                            } else {
                                value_ty
                            };

                            self.define_var(name.clone(), &ty, *is_mutable, *is_public);
                            Type::Void
                        }
                        Stmt::StructDecl {
                            name,
                            fields,
                            is_public,
                            span,
                        } => {
                            if self.env.borrow().is_inside_scope() {
                                return Err(TypeError::StructDeclInsideScope {
                                    span: span.clone(),
                                });
                            }

                            let typed_fields: Vec<_> = fields
                                .iter()
                                .map(|(f_name, f_type)| {
                                    let ty = Type::from_str(
                                        f_type,
                                        span.clone(),
                                        &self.registry.borrow(),
                                    )?;
                                    Ok((f_name.clone(), ty))
                                })
                                .collect::<Result<Vec<_>, TypeError>>()?;

                            if *is_public {
                                self.define_public(
                                    name.clone(),
                                    &Type::Struct(typed_fields.clone()),
                                );
                            }

                            self.registry.borrow_mut().register(
                                name.clone(),
                                typed_fields,
                                span.clone(),
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
                                            span: span.clone(),
                                        });
                                    }

                                    return Ok(ty);
                                } else {
                                    return Ok(Type::Void);
                                }
                            } else {
                                return Err(TypeError::ReturnOutsideFunction {
                                    span: span.clone(),
                                });
                            }
                        }
                        Stmt::While { cond, body, span } => {
                            let cond_ty = self.infer_expr(cond, None)?;

                            if cond_ty != Type::Bool {
                                return Err(TypeError::Mismatch {
                                    expected: Type::Bool.to_string(),
                                    found: cond_ty.to_string(),
                                    span: span.clone(),
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
            Expr::Cast {
                object,
                target_type,
                span,
            } => {
                let target_ty = Type::from_str(target_type, span.clone(), &self.registry.borrow())?;
                let obj_ty = self.infer_expr(object, None)?;

                match (&obj_ty, &target_ty) {
                    (Type::Int, Type::Float) => Ok(Type::Float),
                    (Type::Float, Type::Int) => Ok(Type::Int), // trunca o float em int
                    (t1, t2) if t1 == t2 => Ok(target_ty.clone()),
                    _ => Err(TypeError::InvalidCast {
                        from: obj_ty.to_string(),
                        to: target_ty.to_string(),
                        span: span.clone(),
                    }),
                }
            }
            Expr::Path {
                namespace,
                member,
                span,
            } => {
                let ns_ty = self.infer_expr(namespace, None)?;

                match &ns_ty {
                    Type::Module(exports) => {
                        let (_, ty) = exports.iter().find(|(name, _)| name == member).ok_or(
                            TypeError::UndefinedProperty {
                                ty: ns_ty.to_string(),
                                prop: member.clone(),
                                span: span.clone(),
                            },
                        )?;
                        Ok(ty.clone())
                    }
                    _ => Err(TypeError::InvalidNamespace {
                        ns_ty: ns_ty.to_string(),
                        span: span.clone(),
                    }),
                }
            }
        }
    }

    fn check_expr(&self, expr: &Expr) -> Result<TypedExpr, TypeError> {
        match expr {
            Expr::Int(n, span) => Ok(TypedExpr::Int(*n, span.clone())),
            Expr::Float(n, span) => Ok(TypedExpr::Float(*n, span.clone())),
            Expr::Bool(b, span) => Ok(TypedExpr::Bool(*b, span.clone())),
            Expr::String(s, span) => Ok(TypedExpr::String(s.clone(), span.clone())),
            Expr::Nil(span) => Ok(TypedExpr::Nil(span.clone())),
            Expr::Ident(name, span) => {
                let ty = self.infer_expr(expr, None)?;
                Ok(TypedExpr::Ident(name.clone(), ty, span.clone()))
            }
            Expr::ArrayLiteral(elements, span) => {
                if elements.is_empty() {
                    return Ok(TypedExpr::ArrayLiteral(
                        vec![],
                        Type::Array(Box::new(Type::Void)),
                        span.clone(),
                    ));
                }

                let ty = self.infer_expr(expr, None)?;

                let mut typed_elements = vec![];

                for elem in elements {
                    typed_elements.push(self.check_expr(elem)?);
                }

                Ok(TypedExpr::ArrayLiteral(typed_elements, ty, span.clone()))
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
                    let param_ty = Type::from_str(&p.ty, span.clone(), &self.registry.borrow())?;

                    self.define_var(p.name.clone(), &param_ty, false, false);

                    typed_params.push(TypedParam {
                        name: p.name.clone(),
                        ty: param_ty,
                        span: span.clone(),
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
                    span: span.clone(),
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
                    span: span.clone(),
                })
            }
            Expr::Unary { op, right, span } => {
                let typed_right = self.check_expr(right)?;
                let ty = self.infer_expr(expr, None)?;

                Ok(TypedExpr::Unary {
                    op: *op,
                    right: Box::new(typed_right),
                    ty,
                    span: span.clone(),
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
                    span: span.clone(),
                })
            }
            Expr::Call { callee, args, span } => {
                let ty = self.infer_expr(expr, None)?;

                let typed_callee = if let Expr::Ident(name, _) = callee.as_ref() {
                    if name == "import" {
                        TypedExpr::Ident(
                            name.clone(),
                            Type::Func {
                                params: vec![Type::String],
                                ret: Box::new(ty.clone()),
                            },
                            span.clone(),
                        )
                    } else {
                        self.check_expr(callee)?
                    }
                } else {
                    self.check_expr(callee)?
                };

                let mut typed_args = vec![];

                for arg in args {
                    typed_args.push(self.check_expr(arg)?);
                }

                Ok(TypedExpr::Call {
                    callee: Box::new(typed_callee),
                    args: typed_args,
                    ty,
                    span: span.clone(),
                })
            }
            Expr::Property { object, prop, span } => {
                let ty = self.infer_expr(expr, None)?;
                let typed_obj = self.check_expr(object)?;

                Ok(TypedExpr::Property {
                    object: Box::new(typed_obj),
                    prop: prop.clone(),
                    ty,
                    span: span.clone(),
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
                    span: span.clone(),
                })
            }
            Expr::Block(stmts, span) => {
                let ty = self.infer_expr(expr, None)?;

                let mut typed_stmts = vec![];

                for stmt in stmts {
                    typed_stmts.push(self.check_stmt(stmt)?);
                }

                Ok(TypedExpr::Block(typed_stmts, ty, span.clone()))
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
                    span: span.clone(),
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
                    span: span.clone(),
                })
            }
            Expr::Cast { object, span, .. } => {
                let ty = self.infer_expr(expr, None)?;
                let typed_obj = self.check_expr(object)?;

                Ok(TypedExpr::Cast {
                    object: Box::new(typed_obj),
                    target_type: ty,
                    span: span.clone(),
                })
            }
            Expr::Path {
                namespace,
                member,
                span,
            } => {
                let ty = self.infer_expr(expr, None)?;
                let typed_ns = self.check_expr(namespace)?;

                Ok(TypedExpr::Path {
                    namespace: Box::new(typed_ns),
                    member: member.clone(),
                    ty,
                    span: span.clone(),
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
                is_public,
                span,
            } => {
                let ann_ty = type_annotation
                    .as_ref()
                    .map(|ann| Type::from_str(ann, span.clone(), &self.registry.borrow()))
                    .transpose()?;

                let value_ty = self.infer_expr(value, ann_ty.as_ref())?;

                let ty = if let Some(ann) = &ann_ty {
                    if *ann != value_ty {
                        return Err(TypeError::Mismatch {
                            expected: ann.to_string(),
                            found: value_ty.to_string(),
                            span: span.clone(),
                        });
                    }
                    value_ty
                } else {
                    value_ty
                };

                self.define_var(name.clone(), &ty, *is_mutable, *is_public);

                let typed_value = self.check_expr(value)?;

                Ok(TypedStmt::VarDecl {
                    name: name.clone(),
                    ty,
                    value: Box::new(typed_value),
                    is_mutable: *is_mutable,
                    is_public: *is_public,
                    span: span.clone(),
                })
            }
            Stmt::StructDecl {
                name,
                fields,
                is_public,
                span,
            } => {
                // Struct declarations must be done in first scope
                if self.env.borrow().is_inside_scope() {
                    return Err(TypeError::StructDeclInsideScope { span: span.clone() });
                }

                let typed_fields = fields
                    .iter()
                    .map(|(f_name, f_ty)| {
                        let ty = Type::from_str(f_ty, span.clone(), &self.registry.borrow())?;
                        Ok((f_name.clone(), ty))
                    })
                    .collect::<Result<Vec<_>, TypeError>>()?;

                self.registry.borrow_mut().register(
                    name.clone(),
                    typed_fields.clone(),
                    span.clone(),
                )?;

                if *is_public {
                    self.define_public(name.clone(), &Type::Struct(typed_fields.clone()));
                }

                Ok(TypedStmt::StructDecl {
                    name: name.clone(),
                    fields: typed_fields,
                    is_public: *is_public,
                    span: span.clone(),
                })
            }
            Stmt::While { cond, body, span } => {
                let cond_ty = self.infer_expr(cond, None)?;

                if cond_ty != Type::Bool {
                    return Err(TypeError::Mismatch {
                        expected: Type::Bool.to_string(),
                        found: cond_ty.to_string(),
                        span: span.clone(),
                    });
                }

                let typed_cond = self.check_expr(cond)?;
                let typed_body = self.check_expr(body)?;

                Ok(TypedStmt::While {
                    cond: Box::new(typed_cond),
                    body: Box::new(typed_body),
                    span: span.clone(),
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
                                span: span.clone(),
                            });
                        }

                        let typed_ret_expr = self.check_expr(e)?;

                        *self.current_ret.borrow_mut() = None;
                        Ok(TypedStmt::Return(Some(typed_ret_expr), span.clone()))
                    } else {
                        Ok(TypedStmt::Return(None, span.clone()))
                    }
                } else {
                    Err(TypeError::ReturnOutsideFunction { span: span.clone() })
                }
            }
        }
    }

    pub fn check(&self, ast: &Ast) -> Result<TypedAst, TypeError> {
        let mut typed_stmts = vec![];

        for stmt in ast.stmts.iter() {
            typed_stmts.push(self.check_stmt(stmt)?);
        }

        Ok(TypedAst { stmts: typed_stmts })
    }
}
