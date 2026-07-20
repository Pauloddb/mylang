use std::rc::Rc;

use crate::typechecker::{env::TypeEnv, types::Type};

pub fn register_builtins(env: &Rc<TypeEnv>) {
    let builtins = builtins();

    for (name, ty) in builtins.iter() {
        env.define(name.clone(), ty.clone(), false, false);
    }
}

fn builtins() -> Vec<(String, Type)> {
    basic_builtins()
}

fn basic_builtins() -> Vec<(String, Type)> {
    vec![]
}

pub fn std_module() -> Type {
    Type::Module(vec![
        ("io".to_string(), io_module()),
        ("math".to_string(), math_module()),
    ])
}

fn io_module() -> Type {
    Type::Module(vec![
        (
            "readln".to_string(),
            Type::Func {
                params: vec![Type::String],
                ret: Box::new(Type::String),
            },
        ),
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
    ])
}

fn math_module() -> Type {
    Type::Module(vec![
        ("PI".to_string(), Type::Float),
        (
            "abs".to_string(),
            Type::Func {
                params: vec![Type::Int],
                ret: Box::new(Type::Int),
            },
        ),
    ])
}
