use std::{io::Write, rc::Rc};

use crate::evaluator::{env::EvalEnv, types::Value};

pub fn register_builtins(env: &Rc<EvalEnv>) {
    let builtins = builtins();

    for (name, value) in builtins.iter() {
        env.define(name.clone(), value.clone(), false);
    }
}

fn builtins() -> Vec<(String, Value)> {
    vec![("std".to_string(), std_module())]
}

fn std_module() -> Value {
    Value::Module(vec![("io".to_string(), io_module())])
}

fn io_module() -> Value {
    Value::Module(vec![
        (
            "print".to_string(),
            Value::NativeFunc(Rc::new(move |args| {
                let val = args[0].clone();
                print!("{}", val);
                Ok(Value::Nil)
            })),
        ),
        (
            "println".to_string(),
            Value::NativeFunc(Rc::new(move |args| {
                let val = args[0].clone();
                println!("{}", val);
                Ok(Value::Nil)
            })),
        ),
        (
            "readln".to_string(),
            Value::NativeFunc(Rc::new(move |args| {
                let val = args[0].clone();
                print!("{}", val);

                std::io::stdout().flush().unwrap();

                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf).unwrap();

                let result = buf
                    .trim_end_matches("\n")
                    .trim_end_matches("\r")
                    .to_string();

                Ok(Value::String(result))
            })),
        ),
    ])
}
