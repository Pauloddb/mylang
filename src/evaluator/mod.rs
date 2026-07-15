use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use mylang::EXTENSION;

use crate::{
    evaluator::{
        env::{Binding, EvalEnv},
        error::EvalError,
        properties::property_info,
        types::Value,
    },
    lexer::{
        Lexer,
        types::{Op, Span},
    },
    parser::Parser,
    typechecker::{
        TypeChecker,
        types::{Type, TypedAssignTarget, TypedAst, TypedExpr, TypedStmt},
    },
};

mod env;
mod error;
mod properties;
mod types;

pub struct Evaluator {
    env: RefCell<Rc<EvalEnv>>,
    current_file: RefCell<PathBuf>,
    module_cache: RefCell<HashMap<String, Value>>,
    module_exports: RefCell<Vec<(String, Value)>>,
    return_flag: RefCell<Option<Value>>,
    break_flag: Cell<bool>,
    continue_flag: Cell<bool>,
}

impl Evaluator {
    pub fn new(path: PathBuf) -> Self {
        Self {
            env: RefCell::new(Rc::new(EvalEnv::new())),
            current_file: RefCell::new(path),
            module_cache: RefCell::new(HashMap::new()),
            module_exports: RefCell::new(vec![]),
            return_flag: RefCell::new(None),
            break_flag: Cell::new(false),
            continue_flag: Cell::new(false),
        }
    }

    fn push_scope(&self) {
        let new_rc = EvalEnv::child(&self.env.borrow());
        self.env.replace(new_rc);
    }

    fn pop_scope(&self) {
        let parent = self.env.borrow().parent.clone().unwrap();
        self.env.replace(parent);
    }

    fn define_var(&self, name: String, value: Value, is_mutable: bool) {
        self.env.borrow().define(name, value, is_mutable)
    }

    fn lookup_var(&self, name: &str, span: &Span) -> Result<Binding, EvalError> {
        self.env.borrow().lookup(name, span)
    }

    fn resolve_import(&self, path: String, span: Span) -> Result<Value, EvalError> {
        let cached_key = path.clone();

        if let Some(cached) = self.module_cache.borrow().get(&cached_key) {
            return Ok(cached.clone());
        }

        let dir = self.current_file.borrow().parent().unwrap().to_path_buf();
        let filepath = dir.join(format!("{}.{}", path, EXTENSION));

        let old_file = self.current_file.replace(filepath);
        let old_exports = self.module_exports.replace(vec![]);

        let result = (|| -> Result<Value, EvalError> {
            let content = fs::read_to_string::<&Path>(self.current_file.borrow().as_ref())
                .map_err(|_| EvalError::ImportError {
                    msg: format!(
                        "invalid import path: {}",
                        self.current_file.borrow().display()
                    ),
                    span,
                })?;

            let tokens = Lexer::new(&content, self.current_file.borrow().display().to_string())
                .lex()
                .map_err(|e| EvalError::LexError { msg: e.to_string() })?;

            let ast = Parser::new(tokens)
                .parse()
                .map_err(|e| EvalError::ParseError { msg: e.to_string() })?;

            let typed_ast = TypeChecker::new(self.current_file.borrow().clone())
                .check(&ast)
                .map_err(|e| EvalError::TypeCheckerError { msg: e.to_string() })?;

            self.eval(&typed_ast)?;

            Ok(Value::Module(vec![])) // placeholder
        })();

        let module_exports = self.module_exports.replace(old_exports);
        *self.current_file.borrow_mut() = old_file;

        result?;

        let module = Value::Module(module_exports);
        self.module_cache
            .borrow_mut()
            .insert(cached_key, module.clone());

        Ok(module)
    }

    fn eval_expr(&self, expr: &TypedExpr) -> Result<Value, EvalError> {
        match expr {
            TypedExpr::Int(n, _) => Ok(Value::Int(*n)),
            TypedExpr::Float(n, _) => Ok(Value::Float(*n)),
            TypedExpr::Bool(b, _) => Ok(Value::Bool(*b)),
            TypedExpr::String(s, _) => Ok(Value::String(s.clone())),
            TypedExpr::Nil(_) => Ok(Value::Nil),
            TypedExpr::Ident(name, _, span) => Ok(self.lookup_var(name, span)?.value),
            TypedExpr::ArrayLiteral(elements, _, _) => {
                let mut values = vec![];

                for elem in elements.iter() {
                    values.push(self.eval_expr(elem)?);
                }

                Ok(Value::Array(Rc::new(RefCell::new(values))))
            }
            TypedExpr::Struct { name, fields, .. } => {
                let mut fields_values = vec![];

                for (f_name, f_expr) in fields.iter() {
                    fields_values.push((f_name.clone(), self.eval_expr(f_expr)?));
                }

                Ok(Value::Struct {
                    type_name: name.clone(),
                    fields: Rc::new(RefCell::new(fields_values)),
                })
            }
            TypedExpr::Func {
                params, name, body, ..
            } => {
                let value = Value::Func {
                    params: params.clone(),
                    body: body.clone(),
                    env: RefCell::new(EvalEnv::child(&self.env.borrow())),
                };

                if let Some(n) = name {
                    self.define_var(n.clone(), value.clone(), false);
                }

                Ok(value)
            }
            TypedExpr::Unary {
                op, right, span, ..
            } => match op {
                Op::PlusPlus | Op::MinusMinus => match right.as_ref() {
                    TypedExpr::Ident(name, _, var_span) => {
                        let binding = self.lookup_var(name, var_span)?;

                        if !binding.is_mutable {
                            return Err(EvalError::ImmutableMutation {
                                name: name.clone(),
                                span: var_span.clone(),
                            });
                        }

                        let new_val = match (&binding.value, op) {
                            (Value::Int(n), Op::PlusPlus) => Value::Int(n + 1),
                            (Value::Int(n), Op::MinusMinus) => Value::Int(n - 1),
                            (Value::Float(n), Op::PlusPlus) => Value::Float(n + 1.0),
                            (Value::Float(n), Op::MinusMinus) => Value::Float(n - 1.0),
                            _ => {
                                return Err(EvalError::TypeError {
                                    msg: format!("cannot {:?} non-numeric value", op),
                                    span: span.clone(),
                                });
                            }
                        };

                        self.define_var(name.clone(), new_val.clone(), true);
                        Ok(new_val)
                    }
                    TypedExpr::Property { object, prop, .. } => {
                        let obj_val = self.eval_expr(object)?;
                        match &obj_val {
                            Value::Struct { fields, .. } => {
                                let current = fields
                                    .borrow()
                                    .iter()
                                    .find(|(n, _)| n == prop)
                                    .map(|(_, v)| v.clone());
                                let current = current.ok_or(EvalError::TypeError {
                                    msg: format!("struct has no field `{}`", prop),
                                    span: span.clone(),
                                })?;
                                let new_val = match (&current, op) {
                                    (Value::Int(n), Op::PlusPlus) => Value::Int(n + 1),
                                    (Value::Int(n), Op::MinusMinus) => Value::Int(n - 1),
                                    (Value::Float(n), Op::PlusPlus) => Value::Float(n + 1.0),
                                    (Value::Float(n), Op::MinusMinus) => Value::Float(n - 1.0),
                                    _ => {
                                        return Err(EvalError::TypeError {
                                            msg: format!("cannot {:?} non-numeric value", op),
                                            span: span.clone(),
                                        });
                                    }
                                };
                                let mut fields = fields.borrow_mut();
                                if let Some((_, field)) = fields.iter_mut().find(|(n, _)| n == prop)
                                {
                                    *field = new_val.clone();
                                }
                                Ok(new_val)
                            }
                            _ => Err(EvalError::TypeError {
                                msg: format!("cannot {:?} on {:?}", op, obj_val),
                                span: span.clone(),
                            }),
                        }
                    }
                    TypedExpr::Index { object, index, .. } => {
                        let obj_val = self.eval_expr(object)?;
                        let index_val = self.eval_expr(index)?;
                        match (&obj_val, &index_val) {
                            (Value::Array(elements), Value::Int(i)) => {
                                let idx = if *i >= 0 {
                                    *i as usize
                                } else {
                                    elements.borrow().len() - (-i) as usize
                                };
                                let current = elements.borrow()[idx].clone();
                                let new_val = match (&current, op) {
                                    (Value::Int(n), Op::PlusPlus) => Value::Int(n + 1),
                                    (Value::Int(n), Op::MinusMinus) => Value::Int(n - 1),
                                    (Value::Float(n), Op::PlusPlus) => Value::Float(n + 1.0),
                                    (Value::Float(n), Op::MinusMinus) => Value::Float(n - 1.0),
                                    _ => {
                                        return Err(EvalError::TypeError {
                                            msg: format!("cannot {:?} non-numeric value", op),
                                            span: span.clone(),
                                        });
                                    }
                                };
                                elements.borrow_mut()[idx] = new_val.clone();
                                Ok(new_val)
                            }
                            _ => Err(EvalError::TypeError {
                                msg: format!("cannot {:?} on {:?}", op, obj_val),
                                span: span.clone(),
                            }),
                        }
                    }
                    _ => unreachable!("typechecker should have validated this"),
                },
                _ => {
                    let value = self.eval_expr(right)?;

                    match (op, &value) {
                        (Op::Sub, Value::Int(n)) => Ok(Value::Int(-n)),
                        (Op::Sub, Value::Float(n)) => Ok(Value::Float(-n)),
                        (Op::Not, Value::Bool(b)) => Ok(Value::Bool(!*b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("invalid unary operation: {:?} on {:?}", op, value),
                            span: span.clone(),
                        }),
                    }
                }
            },
            TypedExpr::Binary {
                op,
                left,
                right,
                span,
                ..
            } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                match op {
                    // Aritmética
                    Op::Add => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                        (Value::String(a), Value::String(b)) => {
                            Ok(Value::String(format!("{}{}", a, b)))
                        }
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot add {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::Sub => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot subtract {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::Mul => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot multiply {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::Div => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Float(*a as f64 / *b as f64)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a / *b as f64)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot divide {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::Mod => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a % b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a % *b as f64)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 % b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot mod {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::Pow => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.pow(*b as u32))),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.powf(*b as f64))),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).powf(*b))),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(*b))),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot pow {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    // Comparação → retorna Bool
                    Op::Eq => Ok(Value::Bool(l == r)),
                    Op::Ne => Ok(Value::Bool(l != r)),
                    Op::Lt => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a < *b as f64)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) < *b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot compare {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::Le => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a <= *b as f64)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Bool(*a as f64 <= *b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot compare {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::Gt => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a > *b as f64)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Bool(*a as f64 > *b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot compare {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::Ge => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a >= *b as f64)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Bool(*a as f64 >= *b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot compare {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    // Bitwise
                    Op::BitAnd => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a & b)),
                        (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a & b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot bitwise AND {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::BitOr => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a | b)),
                        (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a | b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot bitwise OR {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::BitXor => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
                        (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a ^ b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot bitwise XOR {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::Shl => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a << b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot shift left {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    Op::Shr => match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a >> b)),
                        _ => Err(EvalError::TypeError {
                            msg: format!("cannot shift right {:?} and {:?}", l, r),
                            span: span.clone(),
                        }),
                    },
                    // Lógico
                    Op::And => Ok(Value::Bool(matches!(
                        (&l, &r),
                        (Value::Bool(true), Value::Bool(true))
                    ))),
                    Op::Or => Ok(Value::Bool(matches!(
                        (&l, &r),
                        (Value::Bool(true), _) | (_, Value::Bool(true))
                    ))),
                    _ => Err(EvalError::TypeError {
                        msg: format!("invalid binary op: {:?}", op),
                        span: span.clone(),
                    }),
                }
            }
            TypedExpr::Assign {
                target,
                value,
                span,
                ..
            } => {
                let val = self.eval_expr(value)?;

                match target {
                    TypedAssignTarget::Ident(name, var_span) => {
                        let binding = self.lookup_var(name, var_span)?;

                        if !binding.is_mutable {
                            return Err(EvalError::ImmutableMutation {
                                name: name.clone(),
                                span: span.clone(),
                            });
                        }

                        self.define_var(name.clone(), val.clone(), true);
                        Ok(val)
                    }
                    TypedAssignTarget::Index {
                        object,
                        index,
                        span,
                    } => {
                        if let TypedExpr::Ident(name, _, var_span) = object.as_ref() {
                            let binding = self.lookup_var(name, var_span)?;

                            if !binding.is_mutable {
                                return Err(EvalError::ImmutableMutation {
                                    name: name.clone(),
                                    span: span.clone(),
                                });
                            }
                        }

                        let obj_val = self.eval_expr(object)?;
                        let index_val = self.eval_expr(index)?;

                        match &index_val {
                            Value::Int(i) => match &obj_val {
                                Value::String(s) => {
                                    if *i < s.len() as i64 {
                                        let idx = if *i >= 0 {
                                            *i as usize
                                        } else {
                                            s.len() - (-i) as usize
                                        };

                                        Ok(Value::String(s.chars().nth(idx).unwrap().to_string()))
                                    } else {
                                        Err(EvalError::IndexError {
                                            msg: format!("index {} on size {}", i, s.len()),
                                            span: span.clone(),
                                        })
                                    }
                                }
                                Value::Array(elements) => {
                                    if *i < elements.borrow().len() as i64 {
                                        let idx = if *i >= 0 {
                                            *i as usize
                                        } else {
                                            elements.borrow().len() - (-i) as usize
                                        };

                                        let val = elements.borrow()[idx].clone();
                                        Ok(val)
                                    } else {
                                        Err(EvalError::IndexError {
                                            msg: format!(
                                                "index {} on size {}",
                                                i,
                                                elements.borrow().len()
                                            ),
                                            span: span.clone(),
                                        })
                                    }
                                }
                                _ => Err(EvalError::TypeError {
                                    msg: format!("value {:?} is not indexable", obj_val),
                                    span: span.clone(),
                                }),
                            },
                            _ => unreachable!("typechecker bug"),
                        }
                    }
                    TypedAssignTarget::Property { object, prop, span } => {
                        if let TypedExpr::Ident(name, _, var_span) = object.as_ref() {
                            let binding = self.lookup_var(name, var_span)?;

                            if !binding.is_mutable {
                                return Err(EvalError::ImmutableMutation {
                                    name: name.clone(),
                                    span: span.clone(),
                                });
                            }
                        }

                        let obj_val = self.eval_expr(object)?;
                        let prop_info = property_info(&obj_val, prop, span.clone())?;

                        if !prop_info.is_mutable {
                            return Err(EvalError::ImmutableMutation {
                                name: format!("{:?}.{}", obj_val, prop),
                                span: span.clone(),
                            });
                        }

                        match obj_val {
                            Value::Struct { fields, .. } => {
                                let mut fields = fields.borrow_mut();
                                if let Some((_, field)) = fields.iter_mut().find(|(n, _)| n == prop)
                                {
                                    *field = val.clone();
                                    Ok(val)
                                } else {
                                    Err(EvalError::TypeError {
                                        msg: format!("struct has no field `{}`", prop),
                                        span: span.clone(),
                                    })
                                }
                            }
                            _ => Err(EvalError::TypeError {
                                msg: format!("cannot set property on {:?}", obj_val),
                                span: span.clone(),
                            }),
                        }
                    }
                }
            }
            TypedExpr::Call {
                callee, args, span, ..
            } => {
                if let TypedExpr::Ident(name, _, _) = callee.as_ref()
                    && name == "import"
                {
                    if let TypedExpr::String(path, _) = &args[0] {
                        return self.resolve_import(path.clone(), span.clone());
                    }
                }

                let callee_val = self.eval_expr(callee)?;

                let mut evaluated_args = vec![];

                for arg_expr in args.iter() {
                    evaluated_args.push(self.eval_expr(arg_expr)?);
                }

                match &callee_val {
                    Value::NativeFunc(func) => func(&evaluated_args),
                    Value::Func { params, body, env } => {
                        let func_env = env.borrow().clone();
                        let call_env = EvalEnv::child(&func_env);
                        let old_env = self.env.replace(call_env);

                        for (param, arg) in params.iter().zip(evaluated_args.iter()) {
                            self.define_var(param.name.clone(), arg.clone(), false);
                        }

                        let result = self.eval_expr(body);
                        self.env.replace(old_env);

                        if let Some(val) = self.return_flag.borrow_mut().take() {
                            return Ok(val);
                        }

                        result
                    }
                    _ => Err(EvalError::NotCallable {
                        value: format!("{:?}", callee_val),
                        span: span.clone(),
                    }),
                }
            }
            TypedExpr::Property {
                object, prop, span, ..
            } => {
                let obj_val = self.eval_expr(object)?;

                let prop_info = property_info(&obj_val, prop, span.clone())?;

                Ok(prop_info.value)
            }
            TypedExpr::Index {
                object,
                index,
                span,
                ..
            } => {
                let obj_val = self.eval_expr(object)?;
                let index_val = self.eval_expr(index)?;

                match &index_val {
                    Value::Int(i) => match &obj_val {
                        Value::String(s) => {
                            if *i < s.len() as i64 {
                                let idx = if *i >= 0 {
                                    *i as usize
                                } else {
                                    s.len() - (-i) as usize
                                };

                                Ok(Value::String(s.chars().nth(idx).unwrap().to_string()))
                            } else {
                                Err(EvalError::IndexError {
                                    msg: format!("index {} on size {}", i, s.len()),
                                    span: span.clone(),
                                })
                            }
                        }
                        Value::Array(elements) => {
                            if *i < elements.borrow().len() as i64 {
                                let idx = if *i >= 0 {
                                    *i as usize
                                } else {
                                    elements.borrow().len() - (-i) as usize
                                };

                                let val = elements.borrow()[idx].clone();
                                Ok(val)
                            } else {
                                Err(EvalError::IndexError {
                                    msg: format!("index {} on size {}", i, elements.borrow().len()),
                                    span: span.clone(),
                                })
                            }
                        }
                        _ => Err(EvalError::TypeError {
                            msg: format!("value {:?} is not indexable", obj_val),
                            span: span.clone(),
                        }),
                    },
                    _ => unreachable!("typechecker bug"),
                }
            }
            TypedExpr::Cast {
                object,
                target_type,
                ..
            } => {
                let obj_val = self.eval_expr(object)?;

                match (&obj_val, target_type) {
                    (Value::Int(n), Type::Float) => Ok(Value::Float(*n as f64)),
                    (Value::Float(n), Type::Int) => Ok(Value::Int(*n as i64)),
                    _ => Ok(obj_val),
                }
            }
            TypedExpr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let cond_val = self.eval_expr(cond)?;

                match cond_val {
                    Value::Bool(true) => {
                        self.push_scope();
                        let then_val = self.eval_expr(then_branch);
                        self.pop_scope();

                        then_val
                    }
                    Value::Bool(false) => {
                        if let Some(branch) = else_branch {
                            self.push_scope();
                            let else_val = self.eval_expr(branch);
                            self.pop_scope();

                            else_val
                        } else {
                            Ok(Value::Nil)
                        }
                    }
                    _ => unreachable!("typechecker bug"),
                }
            }
            TypedExpr::Path {
                namespace,
                member,
                span,
                ..
            } => {
                let ns_val = self.eval_expr(namespace)?;

                match &ns_val {
                    Value::Module(exports) => {
                        let (_, val) = exports.iter().find(|(name, _)| name == member).ok_or(
                            EvalError::UnknownProperty {
                                value: format!("{:?}", ns_val),
                                prop: member.clone(),
                                span: span.clone(),
                            },
                        )?;
                        Ok(val.clone())
                    }
                    _ => Err(EvalError::InvalidNamespace {
                        ns_val: format!("{:?}", ns_val),
                        span: span.clone(),
                    }),
                }
            }
            TypedExpr::Block(stmts, _, _) => {
                let mut last = Value::Nil;

                self.push_scope();
                for stmt in stmts.iter() {
                    last = self.eval_stmt(stmt)?;
                }
                self.pop_scope();

                Ok(last)
            }
        }
    }

    fn eval_stmt(&self, stmt: &TypedStmt) -> Result<Value, EvalError> {
        match stmt {
            TypedStmt::Expr(expr) => self.eval_expr(expr),
            TypedStmt::StructDecl {
                name,
                fields,
                is_public,
                ..
            } => {
                if *is_public {
                    let struct_val = Value::Struct {
                        type_name: name.clone(),
                        fields: Rc::new(RefCell::new(
                            fields
                                .iter()
                                .map(|(n, _)| (n.clone(), Value::Nil))
                                .collect(),
                        )),
                    };
                    self.module_exports
                        .borrow_mut()
                        .push((name.clone(), struct_val));
                }
                Ok(Value::Nil)
            }
            TypedStmt::VarDecl {
                name,
                value,
                is_mutable,
                is_public,
                ..
            } => {
                let val = self.eval_expr(value)?;

                if *is_public {
                    self.module_exports
                        .borrow_mut()
                        .push((name.clone(), val.clone()));
                }

                self.define_var(name.clone(), val, *is_mutable);
                Ok(Value::Nil)
            }
            TypedStmt::Return(ret_expr, _) => {
                let ret_val = if let Some(expr) = ret_expr {
                    self.eval_expr(expr)?
                } else {
                    Value::Nil
                };

                *self.return_flag.borrow_mut() = Some(ret_val);
                Err(EvalError::Return)
            }
            TypedStmt::While { cond, body, .. } => {
                loop {
                    let cond_val = self.eval_expr(cond)?;
                    if let Value::Bool(true) = cond_val {
                        let body_result = self.eval_expr(body);

                        if self.break_flag.get() {
                            self.break_flag.set(false);
                            break;
                        }

                        if self.continue_flag.get() {
                            self.continue_flag.set(false);
                            continue;
                        }

                        body_result?;
                    } else {
                        break;
                    }
                }
                Ok(Value::Nil)
            }
            TypedStmt::Break(_) => {
                self.break_flag.set(true);
                Ok(Value::Nil)
            }
            TypedStmt::Continue(_) => {
                self.continue_flag.set(true);
                Ok(Value::Nil)
            }
        }
    }

    pub fn eval(&self, typed_ast: &TypedAst) -> Result<Value, EvalError> {
        let mut last = Value::Nil;

        for stmt in typed_ast.stmts.iter() {
            last = self.eval_stmt(stmt)?;
        }

        Ok(last)
    }
}
