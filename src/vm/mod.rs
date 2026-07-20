use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

use crate::{
    compiler::{Compiler, chunk::Chunk, opcode::OpCode},
    lexer::Lexer,
    parser::Parser,
    typechecker::TypeChecker,
    vm::{
        builtins::std_module,
        error::VmError,
        properties::property_info,
        value::{Closure, Value},
    },
};

mod builtins;
mod error;
mod properties;
pub mod value;

pub struct CallFrame {
    pub chunk: Chunk,
    pub ip: usize,
    pub base: usize,
    pub closure: Closure,
}

pub struct Vm {
    stack: Vec<Value>,
    ip: usize,
    chunk: Chunk,
    frames: Vec<CallFrame>,
}

impl Vm {
    pub fn new(chunk: Chunk) -> Self {
        let mut vm = Vm {
            stack: vec![],
            ip: 0,
            chunk,
            frames: vec![],
        };
        vm.register_builtins();
        vm
    }

    fn register_builtins(&mut self) {
        // import builtin
        self.stack.push(Value::NativeFunc(Rc::new(move |args| {
            let path = match &args[0] {
                Value::String(s) => s.clone(),
                _ => unreachable!("import bug"),
            };

            if &path == "std" {
                Ok(std_module())
            } else {
                let content = fs::read_to_string(&path).map_err(|_| VmError::ImportError {
                    msg: format!("couldn't read file `{}`", path),
                })?;

                let tokens = Lexer::new(&content, path.clone())
                    .lex()
                    .map_err(|e| VmError::LexError { msg: e.to_string() })?;
                let ast = Parser::new(tokens)
                    .parse()
                    .map_err(|e| VmError::ParseError { msg: e.to_string() })?;
                let typed_ast = TypeChecker::new(PathBuf::from(&path))
                    .check(&ast)
                    .map_err(|e| VmError::TypeError { msg: e.to_string() })?;
                let (chunk, pub_locals) = Compiler::compile(&path, &typed_ast)
                    .map_err(|e| VmError::CompileError { msg: e.to_string() })?;

                let mut sub_vm = Vm::new(chunk);
                sub_vm.run(false)?;

                let mut exports = vec![];
                for (name, slot) in pub_locals.iter() {
                    exports.push((name.clone(), sub_vm.stack[*slot as usize].clone()));
                }
                Ok(Value::Module(exports))
            }
        })));
    }

    fn current_frame_base(&self) -> usize {
        self.frames.last().map(|f| f.base).unwrap_or(0)
    }

    pub fn run(&mut self, debug_mode: bool) -> Result<Vec<Value>, VmError> {
        while self.ip < self.chunk.code.len() {
            let opcode = self.chunk.code[self.ip].clone();
            let span = self.chunk.spans[self.ip].clone();
            self.ip += 1;

            if debug_mode {
                println!("[DEBUG] Executing opcode {:?}", opcode);
            }

            match opcode {
                OpCode::Const(idx) => self.stack.push(self.chunk.consts[idx].clone()),
                OpCode::Nil => self.stack.push(Value::Nil),
                OpCode::True => self.stack.push(Value::Bool(true)),
                OpCode::False => self.stack.push(Value::Bool(false)),
                OpCode::Pop => {
                    self.stack.pop().unwrap();
                }
                OpCode::Rotate(n) => {
                    let len = self.stack.len();
                    // Rotação: move o último elemento para a posição `len - 1 - n`
                    // Equivalente a: remove o último, insere na posição `len - 1 - n`
                    // Com rotate_right: rotaciona a fatia [len-1-n .. len] para a direita em 1
                    self.stack[len - 1 - n as usize..].rotate_right(1);
                }
                OpCode::GetLocal(slot) => {
                    let base = self.current_frame_base();
                    let cell = &self.stack[base + slot as usize];

                    let val = match cell {
                        Value::Upvalue(rc) => rc.borrow().clone(),
                        other => other.clone(),
                    };

                    self.stack.push(val);
                }
                OpCode::SetLocal(slot) => {
                    let val = self.stack.pop().unwrap().clone();
                    let base = self.current_frame_base();

                    match &self.stack[base + slot as usize] {
                        Value::Upvalue(rc) => *rc.borrow_mut() = val.clone(),
                        _ => self.stack[base + slot as usize] = val.clone(),
                    }

                    self.stack.push(val);
                }
                // === Aritmética ===
                OpCode::Add => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(*x + *y),
                        (Value::Float(x), Value::Int(y)) => Value::Float(*x + *y as f64),
                        (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 + *y),
                        (Value::Float(x), Value::Float(y)) => Value::Float(*x + *y),
                        (Value::String(x), Value::String(y)) => Value::String(x.clone() + y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Sub => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(*x - *y),
                        (Value::Float(x), Value::Int(y)) => Value::Float(*x - *y as f64),
                        (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 - *y),
                        (Value::Float(x), Value::Float(y)) => Value::Float(*x - *y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Mul => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(*x * *y),
                        (Value::Float(x), Value::Int(y)) => Value::Float(*x * *y as f64),
                        (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 * *y),
                        (Value::Float(x), Value::Float(y)) => Value::Float(*x * *y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Mod => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => {
                            if *y == 0 {
                                return Err(VmError::DivisionByZero { span });
                            }
                            Value::Int(*x % *y)
                        }
                        (Value::Float(x), Value::Int(y)) => {
                            if *y == 0 {
                                return Err(VmError::DivisionByZero { span });
                            }
                            Value::Float(*x % *y as f64)
                        }
                        (Value::Int(x), Value::Float(y)) => {
                            if *y == 0.0 {
                                return Err(VmError::DivisionByZero { span });
                            }
                            Value::Float(*x as f64 % *y)
                        }
                        (Value::Float(x), Value::Float(y)) => {
                            if *y == 0.0 {
                                return Err(VmError::DivisionByZero { span });
                            }
                            Value::Float(*x % *y)
                        }
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Div => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => {
                            if *y == 0 {
                                return Err(VmError::DivisionByZero { span });
                            }
                            Value::Float(*x as f64 / *y as f64)
                        }
                        (Value::Float(x), Value::Int(y)) => {
                            if *y == 0 {
                                return Err(VmError::DivisionByZero { span });
                            }
                            Value::Float(*x / *y as f64)
                        }
                        (Value::Int(x), Value::Float(y)) => {
                            if *y == 0.0 {
                                return Err(VmError::DivisionByZero { span });
                            }
                            Value::Float(*x as f64 / *y)
                        }
                        (Value::Float(x), Value::Float(y)) => {
                            if *y == 0.0 {
                                return Err(VmError::DivisionByZero { span });
                            }
                            Value::Float(*x / *y)
                        }
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Pow => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => {
                            if *y >= 0 {
                                Value::Int(x.pow(*y as u32))
                            } else {
                                Value::Float((*x as f64).powf(*y as f64))
                            }
                        }
                        (Value::Float(x), Value::Int(y)) => Value::Float(x.powi(*y as i32)),
                        (Value::Int(x), Value::Float(y)) => Value::Float((*x as f64).powf(*y)),
                        (Value::Float(x), Value::Float(y)) => Value::Float(x.powf(*y)),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Neg => {
                    let a = self.stack.pop().unwrap().clone();

                    let result = match &a {
                        Value::Int(x) => Value::Int(-*x),
                        Value::Float(x) => Value::Float(-*x),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Eq => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    self.stack.push(Value::Bool(a == b));
                }
                OpCode::Neq => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    self.stack.push(Value::Bool(a != b));
                }
                OpCode::Lt => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Bool(x < y),
                        (Value::Float(x), Value::Int(y)) => Value::Bool(*x < *y as f64),
                        (Value::Int(x), Value::Float(y)) => Value::Bool((*x as f64) < *y),
                        (Value::Float(x), Value::Float(y)) => Value::Bool(x < y),
                        (Value::String(x), Value::String(y)) => Value::Bool(x < y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Le => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Bool(x <= y),
                        (Value::Float(x), Value::Int(y)) => Value::Bool(*x <= *y as f64),
                        (Value::Int(x), Value::Float(y)) => Value::Bool((*x as f64) <= *y),
                        (Value::Float(x), Value::Float(y)) => Value::Bool(x <= y),
                        (Value::String(x), Value::String(y)) => Value::Bool(x <= y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Gt => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Bool(x > y),
                        (Value::Float(x), Value::Int(y)) => Value::Bool(*x > *y as f64),
                        (Value::Int(x), Value::Float(y)) => Value::Bool((*x as f64) > *y),
                        (Value::Float(x), Value::Float(y)) => Value::Bool(x > y),
                        (Value::String(x), Value::String(y)) => Value::Bool(x > y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Ge => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Bool(x >= y),
                        (Value::Float(x), Value::Int(y)) => Value::Bool(*x >= *y as f64),
                        (Value::Int(x), Value::Float(y)) => Value::Bool((*x as f64) >= *y),
                        (Value::Float(x), Value::Float(y)) => Value::Bool(x >= y),
                        (Value::String(x), Value::String(y)) => Value::Bool(x >= y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::And => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Bool(x), Value::Bool(y)) => Value::Bool(*x && *y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Or => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Bool(x), Value::Bool(y)) => Value::Bool(*x || *y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Not => {
                    let a = self.stack.pop().unwrap().clone();

                    let result = match &a {
                        Value::Bool(x) => Value::Bool(!*x),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::BitAnd => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(x & y),
                        (Value::Bool(x), Value::Bool(y)) => Value::Bool(*x & *y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::BitOr => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(x | y),
                        (Value::Bool(x), Value::Bool(y)) => Value::Bool(*x | *y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::BitXor => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(x ^ y),
                        (Value::Bool(x), Value::Bool(y)) => Value::Bool(*x ^ *y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Shl => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(x << y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Shr => {
                    let b = self.stack.pop().unwrap().clone();
                    let a = self.stack.pop().unwrap().clone();

                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(x >> y),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Increment(slot) => {
                    let base = self.current_frame_base();
                    let slot_val = &self.stack[base + slot as usize].clone();

                    let result = match slot_val {
                        Value::Upvalue(rc) => {
                            let mut v = rc.borrow_mut();
                            if let Value::Int(n) = &*v {
                                let r = Value::Int(*n + 1);
                                *v = r.clone();
                                r
                            } else {
                                unreachable!("typechecker bug")
                            }
                        }
                        Value::Int(n) => {
                            let v = Value::Int(*n + 1);
                            self.stack[base + slot as usize] = v.clone();
                            v
                        }
                        _ => unreachable!("typechecker bug"),
                    };
                    self.stack.push(result);
                }
                OpCode::Decrement(slot) => {
                    let base = self.current_frame_base();
                    let slot_val = &self.stack[base + slot as usize].clone();

                    let result = match slot_val {
                        Value::Upvalue(rc) => {
                            let mut v = rc.borrow_mut();
                            if let Value::Int(n) = &*v {
                                let r = Value::Int(*n - 1);
                                *v = r.clone();
                                r
                            } else {
                                unreachable!("typechecker bug")
                            }
                        }
                        Value::Int(n) => {
                            let v = Value::Int(*n - 1);
                            self.stack[base + slot as usize] = v.clone();
                            v
                        }
                        _ => unreachable!("typechecker bug"),
                    };
                    self.stack.push(result);
                }
                OpCode::AsInt => {
                    let val = self.stack.pop().unwrap().clone();

                    let result = match &val {
                        Value::Int(_) => val.clone(),
                        Value::Float(n) => Value::Int(*n as i64),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::AsFloat => {
                    let val = self.stack.pop().unwrap();

                    let result = match &val {
                        Value::Float(_) => val.clone(),
                        Value::Int(n) => Value::Float(*n as f64),
                        _ => unreachable!("typechecker bug"),
                    };

                    self.stack.push(result);
                }
                OpCode::Jump(addr) => self.ip = addr,
                OpCode::JumpIfFalse(addr) => {
                    let cond = self.stack.pop().unwrap();
                    if let Value::Bool(false) = cond {
                        self.ip = addr;
                    }
                }
                OpCode::Struct(type_name, n_fields) => {
                    let mut fields = vec![];

                    for _ in 0..n_fields {
                        let name = match self.stack.pop().unwrap() {
                            Value::String(n) => n.clone(),
                            _ => unreachable!("compiler bug"),
                        };
                        let value = self.stack.pop().unwrap();
                        fields.push((name, value));
                    }

                    self.stack.push(Value::Struct {
                        type_name,
                        fields: Rc::new(RefCell::new(fields)),
                    });
                }
                OpCode::Array(n_elements) => {
                    let mut elements = vec![];

                    for _ in 0..n_elements {
                        let val = self.stack.pop().unwrap();
                        elements.push(val);
                    }

                    eprintln!("[DEBUG Array] popped (LIFO order): {:?}", elements);

                    elements.reverse();
                    self.stack
                        .push(Value::Array(Rc::new(RefCell::new(elements))));
                }
                OpCode::Closure(idx, n_upv) => {
                    let closure = match &self.chunk.consts[idx] {
                        Value::Closure(c) => c.clone(),
                        _ => unreachable!("compiler bug"),
                    };

                    let mut upvalues = Vec::with_capacity(n_upv as usize);
                    for spec in closure.upvalues_specs.iter() {
                        if spec.is_local {
                            let frame_base = self.current_frame_base();
                            let slot = &mut self.stack[frame_base + spec.slot as usize];

                            let rc = match slot {
                                Value::Upvalue(existing) => existing.clone(),
                                _ => {
                                    let rc = Rc::new(RefCell::new(slot.clone()));
                                    *slot = Value::Upvalue(rc.clone());
                                    rc
                                }
                            };
                            upvalues.push(rc);
                        } else {
                            let caller_closure = self.frames.last().unwrap().closure.clone();
                            upvalues.push(caller_closure.upvalues[spec.slot as usize].clone());
                        }
                    }

                    let new_closure = Closure {
                        chunk: closure.chunk,
                        upvalues: upvalues.clone(),
                        upvalues_specs: closure.upvalues_specs.clone(),
                    };
                    self.stack.push(Value::Closure(new_closure));
                }
                OpCode::GetUpvalue(idx) => {
                    let frame = self.frames.last().unwrap();
                    let val = frame.closure.upvalues[idx as usize].borrow().clone();
                    self.stack.push(val);
                }
                OpCode::SetUpvalue(idx) => {
                    let val = self.stack.pop().unwrap();
                    let frame = self.frames.last().unwrap();
                    *frame.closure.upvalues[idx as usize].borrow_mut() = val.clone();
                    self.stack.push(val);
                }
                OpCode::Return => {
                    let result = self.stack.pop().unwrap();

                    if let Some(frame) = self.frames.pop() {
                        self.chunk = frame.chunk;
                        self.ip = frame.ip;

                        self.stack.truncate(frame.base);
                        self.stack.push(result);
                    }
                }
                OpCode::Call(argc) => {
                    let callee = self.stack.pop().unwrap();

                    match &callee {
                        Value::NativeFunc(func) => {
                            let n = argc as usize;
                            let args = self.stack.drain(self.stack.len() - n..).collect::<Vec<_>>();
                            self.stack.push(func(&args)?);
                        }
                        Value::Closure(closure) => {
                            let base = self.stack.len() - argc as usize;

                            let frame = CallFrame {
                                chunk: std::mem::replace(&mut self.chunk, closure.chunk.clone()),
                                ip: self.ip,
                                base,
                                closure: closure.clone(),
                            };
                            self.frames.push(frame);
                            self.ip = 0;
                        }
                        _ => {
                            return Err(VmError::NotCallable {
                                value: callee.to_string(),
                                span: span.clone(),
                            });
                        }
                    }
                }
                OpCode::IndexGet => {
                    let index = match self.stack.pop().unwrap() {
                        Value::Int(n) => n,
                        _ => unreachable!("typechecker bug"),
                    };
                    let obj = self.stack.pop().unwrap();

                    match &obj {
                        Value::Array(elements) => {
                            if index < 0 {
                                if (-index as usize) > elements.borrow().len() {
                                    return Err(VmError::IndexError {
                                        msg: "index out of bounds".to_string(),
                                        span: span.clone(),
                                    });
                                }

                                let val = elements.borrow()[-index as usize - 1].clone();
                                self.stack.push(val);
                            } else {
                                if elements.borrow().len() <= index as usize {
                                    return Err(VmError::IndexError {
                                        msg: "index out of bounds".to_string(),
                                        span: span.clone(),
                                    });
                                }

                                let val = elements.borrow()[index as usize].clone();
                                eprintln!(
                                    "[DEBUG IndexGet] arr[{}] = {:?}  (array len={})",
                                    index,
                                    val,
                                    elements.borrow().len()
                                );
                                self.stack.push(val);
                            }
                        }
                        _ => unreachable!("not indexable"),
                    }
                }
                OpCode::IndexSet => {
                    let index = match self.stack.pop().unwrap() {
                        Value::Int(n) => n,
                        _ => unreachable!("typechecker bug"),
                    };

                    let obj = self.stack.pop().unwrap();

                    let val = self.stack.pop().unwrap();

                    match &obj {
                        Value::Array(elements) => {
                            if index < 0 {
                                if (-index as usize) > elements.borrow().len() {
                                    return Err(VmError::IndexError {
                                        msg: "index out of bounds".to_string(),
                                        span: span.clone(),
                                    });
                                }

                                elements.borrow_mut()[-index as usize - 1] = val.clone();
                                self.stack.push(val);
                            } else {
                                if elements.borrow().len() <= index as usize {
                                    return Err(VmError::IndexError {
                                        msg: "index out of bounds".to_string(),
                                        span: span.clone(),
                                    });
                                }

                                elements.borrow_mut()[index as usize] = val.clone();
                                self.stack.push(val);
                            }
                        }
                        _ => unreachable!("not indexable"),
                    }
                }
                OpCode::GetProperty(prop) => {
                    let obj = self.stack.pop().unwrap();

                    let prop_info = property_info(&obj, &prop, span.clone())?;
                    self.stack.push(prop_info.value.clone());
                }
                OpCode::SetProperty(prop) => {
                    let obj = self.stack.pop().unwrap();
                    let val = self.stack.pop().unwrap();

                    let prop_info = property_info(&obj, &prop, span.clone())?;

                    if !prop_info.is_mutable {
                        return Err(VmError::ImmutableMutation {
                            name: format!("{}.{}", obj, prop),
                            span: span.clone(),
                        });
                    }

                    match &obj {
                        Value::Struct { fields, .. } => {
                            let mut fields = fields.borrow_mut();
                            if let Some((_, field)) = fields.iter_mut().find(|(n, _)| n == &prop) {
                                *field = val.clone();
                                self.stack.push(val);
                            }
                        }
                        _ => unreachable!("typechecker bug"),
                    }
                }
            }

            if debug_mode {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        Ok(self.stack.clone())
    }
}
