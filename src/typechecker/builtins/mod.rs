use std::rc::Rc;

use crate::typechecker::{env::TypeEnv, types::Type};

pub fn register_builtins(env: &Rc<TypeEnv>) {
    let builtins = builtins();

    for (name, ty) in builtins.iter() {
        env.define(name.clone(), ty.clone(), false, false);
    }
}

fn builtins() -> Vec<(String, Type)> {
    vec![("std".to_string(), std_module())]
}

fn std_module() -> Type {
    Type::Module(vec![("io".to_string(), io_module())])
}

fn io_module() -> Type {
    Type::Module(vec![
        (
            "print".to_string(),
            Type::Func {
                params: vec![Type::String],
                ret: Box::new(Type::Void),
            },
        ),
        (
            "println".to_string(),
            Type::Func {
                params: vec![Type::String],
                ret: Box::new(Type::Void),
            },
        ),
        (
            "readln".to_string(),
            Type::Func {
                params: vec![Type::String],
                ret: Box::new(Type::String),
            },
        ),
    ])
}
