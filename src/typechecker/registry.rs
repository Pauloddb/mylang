use std::collections::HashMap;

use crate::{
    lexer::types::Span,
    typechecker::{error::TypeError, types::Type},
};

#[derive(Debug)]
pub struct TypeRegistry {
    structs: HashMap<String, Vec<(String, Type)>>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            structs: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        name: String,
        fields: Vec<(String, Type)>,
        span: Span,
    ) -> Result<(), TypeError> {
        if self.structs.contains_key(&name) {
            return Err(TypeError::AlreadyDefinedType { name, span });
        }
        self.structs.insert(name, fields);
        Ok(())
    }

    pub fn resolve(&self, name: &str) -> Option<Type> {
        self.structs
            .get(name)
            .map(|fields| Type::Struct(fields.clone()))
    }

    pub fn get_fields(&self, name: &str) -> Option<&Vec<(String, Type)>> {
        self.structs.get(name)
    }
}
