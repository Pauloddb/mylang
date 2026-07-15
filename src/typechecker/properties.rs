use crate::{
    lexer::types::Span,
    typechecker::{error::TypeError, types::Type},
};

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyInfo {
    pub ty: Type,
    pub is_mutable: bool,
    pub needs_mutable_owner: bool,
}

pub fn property_info(ty: &Type, prop: &str, span: Span) -> Result<PropertyInfo, TypeError> {
    match ty {
        Type::String => match prop {
            "len" => Ok(PropertyInfo {
                ty: Type::Func {
                    params: vec![],
                    ret: Box::new(Type::Int),
                },
                is_mutable: false,
                needs_mutable_owner: false,
            }),
            "upcase" => Ok(PropertyInfo {
                ty: Type::Func {
                    params: vec![],
                    ret: Box::new(Type::String),
                },
                is_mutable: false,
                needs_mutable_owner: false,
            }),
            "lowcase" => Ok(PropertyInfo {
                ty: Type::Func {
                    params: vec![],
                    ret: Box::new(Type::String),
                },
                is_mutable: false,
                needs_mutable_owner: false,
            }),
            "chars" => Ok(PropertyInfo {
                ty: Type::Func {
                    params: vec![],
                    ret: Box::new(Type::Array(Box::new(Type::String))),
                },
                is_mutable: false,
                needs_mutable_owner: false,
            }),
            _ => Err(TypeError::UndefinedProperty {
                ty: ty.to_string(),
                prop: prop.into(),
                span,
            }),
        },
        Type::Array(elem) => match prop {
            "len" => Ok(PropertyInfo {
                ty: Type::Func {
                    params: vec![],
                    ret: Box::new(Type::Int),
                },
                is_mutable: false,
                needs_mutable_owner: false,
            }),
            "push" => Ok(PropertyInfo {
                ty: Type::Func {
                    params: vec![*elem.clone()],
                    ret: Box::new(Type::Void),
                },
                is_mutable: false,
                needs_mutable_owner: true,
            }),
            "pop" => Ok(PropertyInfo {
                ty: Type::Func {
                    params: vec![],
                    ret: elem.clone(),
                },
                is_mutable: false,
                needs_mutable_owner: true,
            }),
            "clear" => Ok(PropertyInfo {
                ty: Type::Func {
                    params: vec![],
                    ret: Box::new(Type::Void),
                },
                is_mutable: false,
                needs_mutable_owner: true,
            }),
            _ => Err(TypeError::UndefinedProperty {
                ty: ty.to_string(),
                prop: prop.into(),
                span,
            }),
        },
        Type::Struct(fields) => {
            let field = fields.iter().find(|(name, _)| name == prop);
            match field {
                Some((_, ty)) => Ok(PropertyInfo {
                    ty: ty.clone(),
                    is_mutable: true,
                    needs_mutable_owner: true,
                }),
                _ => Err(TypeError::UndefinedProperty {
                    ty: ty.to_string(),
                    prop: prop.into(),
                    span,
                }),
            }
        }
        Type::Module(exports) => {
            let export = exports.iter().find(|(name, _)| name == prop);
            match export {
                Some((_, ty)) => Ok(PropertyInfo {
                    ty: ty.clone(),
                    is_mutable: false,
                    needs_mutable_owner: false,
                }),
                _ => Err(TypeError::UndefinedProperty {
                    ty: ty.to_string(),
                    prop: prop.to_string(),
                    span,
                }),
            }
        }
        _ => Err(TypeError::NoProperties {
            ty: ty.to_string(),
            span,
        }),
    }
}
