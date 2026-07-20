use std::{cell::RefCell, rc::Rc};

use crate::{
    lexer::types::Span,
    vm::{error::VmError, value::Value},
};

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyInfo {
    pub value: Value,
    pub is_mutable: bool,
    pub needs_mutable_owner: bool,
}

pub fn property_info(object: &Value, prop: &str, span: Span) -> Result<PropertyInfo, VmError> {
    match object {
        Value::String(s) => {
            let s = s.clone();

            match prop {
                "len" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |_args| Ok(Value::Int(s.len() as i64)))),
                    is_mutable: false,
                    needs_mutable_owner: false,
                }),
                "upcase" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |_args| {
                        Ok(Value::String(s.to_uppercase()))
                    })),
                    is_mutable: false,
                    needs_mutable_owner: false,
                }),
                "lowcase" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |_args| {
                        Ok(Value::String(s.to_lowercase()))
                    })),
                    is_mutable: false,
                    needs_mutable_owner: false,
                }),
                "chars" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |_args| {
                        let chars = s
                            .chars()
                            .map(|ch| Value::String(ch.to_string()))
                            .collect::<Vec<_>>();

                        Ok(Value::Array(Rc::new(RefCell::new(chars))))
                    })),
                    is_mutable: false,
                    needs_mutable_owner: false,
                }),
                "trim" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |_args| {
                        Ok(Value::String(s.trim().to_string()))
                    })),
                    is_mutable: false,
                    needs_mutable_owner: false,
                }),
                _ => Err(VmError::UnknownProperty {
                    value: format!("{:?}", object),
                    prop: prop.to_string(),
                    span,
                }),
            }
        }
        Value::Int(n) => {
            let n = n.clone();
            match prop {
                "to_str" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |_args| {
                        Ok(Value::String(n.to_string()))
                    })),
                    is_mutable: false,
                    needs_mutable_owner: false,
                }),
                _ => Err(VmError::UnknownProperty {
                    value: format!("{:?}", object),
                    prop: prop.to_string(),
                    span,
                }),
            }
        }
        Value::Float(n) => {
            let n = n.clone();
            match prop {
                "to_str" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |_args| {
                        Ok(Value::String(n.to_string()))
                    })),
                    is_mutable: false,
                    needs_mutable_owner: false,
                }),
                _ => Err(VmError::UnknownProperty {
                    value: format!("{:?}", object),
                    prop: prop.to_string(),
                    span,
                }),
            }
        }
        Value::Bool(b) => {
            let b = b.clone();
            match prop {
                "to_str" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |_args| {
                        Ok(Value::String(b.to_string()))
                    })),
                    is_mutable: false,
                    needs_mutable_owner: false,
                }),
                _ => Err(VmError::UnknownProperty {
                    value: format!("{:?}", object),
                    prop: prop.to_string(),
                    span,
                }),
            }
        }
        Value::Array(elements) => {
            let elements = elements.clone();
            let span = span.clone();

            match prop {
                "len" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |_args| {
                        Ok(Value::Int(elements.borrow().len() as i64))
                    })),
                    is_mutable: false,
                    needs_mutable_owner: false,
                }),
                "push" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |args| {
                        let val = args[0].clone();
                        elements.borrow_mut().push(val);
                        Ok(Value::Nil)
                    })),
                    is_mutable: false,
                    needs_mutable_owner: true,
                }),
                "pop" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |_args| {
                        let val = elements
                            .borrow_mut()
                            .pop()
                            .ok_or(VmError::PopEmpty { span: span.clone() })?;
                        Ok(val)
                    })),
                    is_mutable: false,
                    needs_mutable_owner: true,
                }),
                "clear" => Ok(PropertyInfo {
                    value: Value::NativeFunc(Rc::new(move |_args| {
                        elements.borrow_mut().clear();
                        Ok(Value::Nil)
                    })),
                    is_mutable: false,
                    needs_mutable_owner: true,
                }),
                _ => Err(VmError::UnknownProperty {
                    value: format!("{:?}", object),
                    prop: prop.to_string(),
                    span,
                }),
            }
        }
        Value::Struct { fields, .. } => {
            let fields = fields.clone();
            let field = fields
                .borrow()
                .iter()
                .find(|(name, _)| name == prop)
                .cloned();

            match field {
                Some((_, val)) => Ok(PropertyInfo {
                    value: val.clone(),
                    is_mutable: true,
                    needs_mutable_owner: true,
                }),
                _ => Err(VmError::UnknownProperty {
                    value: format!("{:?}", object),
                    prop: prop.to_string(),
                    span,
                }),
            }
        }
        Value::Module(exports) => {
            let export = exports.iter().find(|(name, _)| name == prop).cloned();

            match export {
                Some((_, val)) => Ok(PropertyInfo {
                    value: val.clone(),
                    is_mutable: false,
                    needs_mutable_owner: false,
                }),
                _ => Err(VmError::UnknownProperty {
                    value: format!("{:?}", object),
                    prop: prop.to_string(),
                    span,
                }),
            }
        }
        _ => Err(VmError::NoProperties {
            value: format!("{:?}", object),
            span,
        }),
    }
}
