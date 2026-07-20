use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    evaluator::{error::EvalError, types::Value},
    lexer::types::Span,
};

#[derive(Clone, Debug)]
pub struct Binding {
    pub value: Value,
    pub is_mutable: bool,
}

#[derive(Debug, Clone)]
pub struct EvalEnv {
    bindings: RefCell<HashMap<String, Binding>>,
    pub parent: Option<Rc<EvalEnv>>,
}

impl EvalEnv {
    pub fn new() -> Self {
        Self {
            bindings: RefCell::new(HashMap::new()),
            parent: None,
        }
    }

    ///Allows shadowing
    pub fn define(&self, name: String, value: Value, is_mutable: bool) {
        let binding = Binding { value, is_mutable };
        self.bindings.borrow_mut().insert(name, binding);
    }

    pub fn set(&self, name: &str, value: Value) -> bool {
        if self.bindings.borrow().contains_key(name) {
            self.bindings.borrow_mut().get_mut(name).unwrap().value = value;
            true
        } else if let Some(parent) = &self.parent {
            parent.set(name, value)
        } else {
            false
        }
    }

    pub fn lookup(&self, name: &str, span: &Span) -> Result<Binding, EvalError> {
        let borrowed = self.bindings.borrow();
        let result = borrowed.get(name);

        if result.is_some() {
            Ok(result.cloned().unwrap())
        } else if let Some(p) = &self.parent {
            p.lookup(name, span)
        } else {
            Err(EvalError::UndefinedVar {
                name: name.to_string(),
                span: span.clone(),
            })
        }
    }

    pub fn child(env: &Rc<Self>) -> Rc<Self> {
        Rc::new(Self {
            bindings: RefCell::new(HashMap::new()),
            parent: Some(env.clone()),
        })
    }
}
