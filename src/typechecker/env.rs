use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::typechecker::types::Type;

#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub bindings: RefCell<HashMap<String, Binding>>,
    pub parent: Option<Rc<TypeEnv>>,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub ty: Type,
    pub is_mutable: bool,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: RefCell::new(HashMap::new()),
            parent: None,
        }
    }

    pub fn child(env: &Rc<Self>) -> Rc<Self> {
        let child = Rc::new(Self {
            bindings: RefCell::new(HashMap::new()),
            parent: Some(env.clone()),
        });
        child
    }

    pub fn is_inside_scope(&self) -> bool {
        self.parent.is_some()
    }

    ///Allows shadowing
    pub fn define(&self, name: String, ty: Type, is_mutable: bool) {
        self.bindings
            .borrow_mut()
            .insert(name, Binding { ty, is_mutable });
    }

    pub fn lookup(&self, name: &str) -> Option<Binding> {
        let binding = self.bindings.borrow().get(name).cloned();

        if binding.is_some() {
            binding
        } else {
            match &self.parent {
                Some(p) => p.lookup(name),
                None => None,
            }
        }
    }
}
