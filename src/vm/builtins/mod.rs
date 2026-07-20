use std::{f64::consts::PI, io::Write, rc::Rc};

use crate::vm::value::Value;

pub fn std_module() -> Value {
    Value::Module(vec![
        ("io".to_string(), io_module()),
        ("math".to_string(), math_module()),
    ])
}

fn io_module() -> Value {
    Value::Module(vec![
        (
            "print".to_string(),
            Value::NativeFunc(Rc::new(move |args| {
                let s = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => unreachable!("typechecker bug"),
                };
                print!("{}", s);
                Ok(Value::Nil)
            })),
        ),
        (
            "println".to_string(),
            Value::NativeFunc(Rc::new(move |args| {
                let s = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => unreachable!("typechecker bug"),
                };
                println!("{}", s);
                Ok(Value::Nil)
            })),
        ),
        (
            "readln".to_string(),
            Value::NativeFunc(Rc::new(move |args| {
                let prompt = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => unreachable!("typechecker bug"),
                };
                print!("{}", prompt);
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

fn math_module() -> Value {
    Value::Module(vec![
        ("PI".to_string(), Value::Float(PI)),
        (
            "abs".to_string(),
            Value::NativeFunc(Rc::new(move |args| {
                let n = match &args[0] {
                    Value::Int(n) => *n,
                    _ => unreachable!("typechecker bug"),
                };
                Ok(Value::Int(if n > 0 { n } else { -n }))
            })),
        ),
    ])
}
