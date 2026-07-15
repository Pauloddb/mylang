use std::{cell::RefCell, fmt, rc::Rc};

use crate::{
    evaluator::{env::EvalEnv, error::EvalError},
    typechecker::types::{TypedExpr, TypedParam},
};

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
    Array(Rc<RefCell<Vec<Value>>>),
    Struct {
        type_name: String,
        fields: Rc<RefCell<Vec<(String, Value)>>>,
    },
    Func {
        params: Vec<TypedParam>,
        body: Box<TypedExpr>,
        env: RefCell<Rc<EvalEnv>>,
    },
    NativeFunc(Rc<dyn Fn(&[Value]) -> Result<Value, EvalError>>),
    Module(Vec<(String, Value)>),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "Int({})", n),
            Value::Float(n) => write!(f, "Float({})", n),
            Value::String(s) => write!(f, "String({:?})", s),
            Value::Bool(b) => write!(f, "Bool({})", b),
            Value::Nil => write!(f, "Nil"),
            Value::Array(arr) => write!(f, "Array({:?})", arr.borrow()),
            Value::Struct { type_name, fields } => {
                write!(f, "{} {{ {:?} }}", type_name, fields.borrow())
            }
            Value::Func { .. } => write!(f, "Func(...)"),
            Value::NativeFunc { .. } => write!(f, "NativeFunc(...)"),
            Value::Module(exports) => write!(f, "Module({:?})", exports),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Array(a), Value::Array(b)) => {
                // compara por conteúdo
                *a.borrow() == *b.borrow()
            }
            (
                Value::Struct {
                    type_name: t1,
                    fields: f1,
                },
                Value::Struct {
                    type_name: t2,
                    fields: f2,
                },
            ) => t1 == t2 && *f1.borrow() == *f2.borrow(),
            // Func/Module: compara por ponteiro (mesma instância)
            (Value::Func { .. }, Value::Func { .. }) => {
                std::ptr::eq(self as *const Value, other as *const Value)
            }
            (Value::Module(_), Value::Module(_)) => {
                std::ptr::eq(self as *const Value, other as *const Value)
            }
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Nil => write!(f, "nil"),
            Value::Array(elements) => {
                write!(f, "[")?;

                let borrowed = elements.borrow();
                for (i, elem) in borrowed.iter().enumerate() {
                    write!(f, "{}", elem)?;

                    if i < borrowed.len() - 1 {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "]")
            }
            Value::Struct { type_name, fields } => {
                write!(f, "{}", type_name)?;
                write!(f, "{{ ")?;

                let borrowed = fields.borrow();
                for (i, (f_name, f_val)) in borrowed.iter().enumerate() {
                    write!(f, "{}: {}", f_name, f_val)?;

                    if i < borrowed.len() - 1 {
                        write!(f, ", ")?;
                    }
                }

                write!(f, " }}")
            }
            _ => write!(f, "undisplayable value"),
        }
    }
}
