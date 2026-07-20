use std::{cell::RefCell, fmt, rc::Rc};

use crate::{
    compiler::{Upvalue, chunk::Chunk},
    vm::error::VmError,
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
    Closure(Closure),
    NativeFunc(Rc<dyn Fn(&[Value]) -> Result<Value, VmError>>),
    Module(Vec<(String, Value)>),
    Upvalue(Rc<RefCell<Value>>),
}

#[derive(Clone)]
pub struct Closure {
    pub chunk: Chunk,
    pub upvalues: Vec<Rc<RefCell<Value>>>,
    pub upvalues_specs: Vec<Upvalue>,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),

            Value::Array(elems) => {
                let borrowed = elems.borrow();
                write!(f, "[")?;
                for (i, e) in borrowed.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "]")
            }

            Value::Struct { type_name, fields } => {
                let borrowed = fields.borrow();
                write!(f, "{} {{ ", type_name)?;
                for (i, (n, v)) in borrowed.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", n, v)?;
                }
                write!(f, " }}")
            }

            Value::Closure(_) => write!(f, "<function>"),
            Value::NativeFunc(_) => write!(f, "<native function>"),
            Value::Module(_) => write!(f, "<module>"),
            Value::Upvalue(_) => write!(f, "<upvalue>"),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "Int({})", n),
            Value::Float(n) => write!(f, "Float({})", n),
            Value::String(s) => write!(f, "String({:?})", s),
            Value::Bool(b) => write!(f, "Bool({})", b),
            Value::Nil => write!(f, "Nil"),
            Value::Array(elems) => f.debug_list().entries(elems.borrow().iter()).finish(),
            Value::Struct { type_name, fields } => f
                .debug_struct(type_name)
                .field("fields", &*fields.borrow())
                .finish(),
            Value::Closure(_) => write!(f, "Closure(...)"),
            Value::NativeFunc(_) => write!(f, "NativeFunc(...)"),
            Value::Module(m) => f.debug_list().entries(m.iter()).finish(),
            Value::Upvalue(_) => write!(f, "Upvalue(...)"),
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
            (Value::Array(a), Value::Array(b)) => *a.borrow() == *b.borrow(),
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
            _ => false,
        }
    }
}
