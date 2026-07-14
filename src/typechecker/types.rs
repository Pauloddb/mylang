use std::fmt;

use crate::{
    lexer::types::{Op, Span},
    parser::types::AssignTarget,
    typechecker::{error::TypeError, registry::TypeRegistry},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Array(Box<Type>),
    Void,
    Func { params: Vec<Type>, ret: Box<Type> },
    Struct(Vec<(String, Type)>),
    Module(Vec<(String, Type)>),
}

impl Type {
    pub fn from_str(s: &str, span: Span, registry: &TypeRegistry) -> Result<Self, TypeError> {
        match s {
            "int" => Ok(Self::Int),
            "float" => Ok(Self::Float),
            "bool" => Ok(Self::Bool),
            "string" => Ok(Self::String),
            "void" => Ok(Self::Void),
            s if s.starts_with("func(") => {
                let mut buf = String::new();
                let mut params = vec![];

                let mut i = 0;
                let remainder = &s[5..];
                while i < remainder.len() {
                    let ch = remainder.chars().nth(i).unwrap();
                    match ch {
                        ')' => {
                            let ty = Type::from_str(&buf, span.clone(), registry)?;
                            params.push(ty);

                            buf.clear();
                            break;
                        }
                        ',' => {
                            let ty = Type::from_str(&buf, span.clone(), registry)?;
                            params.push(ty);

                            buf.clear();
                            i += 1;
                        }
                        ch if ch.is_ascii_whitespace() => i += 1,
                        _ => buf.push(ch),
                    }
                    i += 1;
                }

                let mut depth = 0u32;
                let mut arrow_index = None;
                let chars = s.chars().collect::<Vec<_>>();
                let mut j = 0;

                while j < chars.len() - 1 {
                    match chars[j] {
                        '(' => depth += 1,
                        ')' => depth -= 1,
                        '-' if depth == 0 && chars[j + 1] == '>' => {
                            arrow_index = Some(j);
                            break;
                        }
                        _ => {}
                    }
                    j += 1;
                }

                let arrow_index = arrow_index.ok_or(TypeError::UnknownType {
                    name: s.to_string(),
                    span: span.clone(),
                })?;

                let ret = Type::from_str(s[(arrow_index + 2)..].trim(), span, registry)?;

                Ok(Self::Func {
                    params,
                    ret: Box::new(ret),
                })
            }
            s if s.ends_with("[]") => {
                let ty = Type::from_str(&s[..(s.len() - 2)], span, registry)?;
                Ok(Self::Array(Box::new(ty)))
            }
            s => registry.resolve(s).ok_or(TypeError::UnknownType {
                name: s.to_string(),
                span,
            }),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Array(ty) => {
                write!(f, "{}[]", ty.to_string())
            }
            Type::Func { params, ret } => {
                write!(f, "func(")?;

                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }

                write!(f, ") -> {}", ret)
            }
            Type::Struct(fields) => {
                write!(f, "struct {{ ")?;

                for (i, (f_name, f_type)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", f_name, f_type)?;
                }

                write!(f, " }}")
            }
            Type::Module(_) => write!(f, "module"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypedExpr {
    Int(i64, Span),
    Float(f64, Span),
    Bool(bool, Span),
    String(String, Span),
    Nil(Span),
    Ident(String, Type, Span),
    ArrayLiteral(Vec<TypedExpr>, Type, Span),
    Unary {
        op: Op,
        right: Box<TypedExpr>,
        ty: Type,
        span: Span,
    },
    Binary {
        op: Op,
        left: Box<TypedExpr>,
        right: Box<TypedExpr>,
        ty: Type,
        span: Span,
    },
    Assign {
        target: AssignTarget,
        value: Box<TypedExpr>,
        ty: Type,
        span: Span,
    },
    Call {
        callee: Box<TypedExpr>,
        args: Vec<TypedExpr>,
        ty: Type,
        span: Span,
    },
    Property {
        object: Box<TypedExpr>,
        prop: String,
        ty: Type,
        span: Span,
    },
    Index {
        object: Box<TypedExpr>,
        index: Box<TypedExpr>,
        ty: Type,
        span: Span,
    },
    If {
        cond: Box<TypedExpr>,
        then_branch: Box<TypedExpr>,
        else_branch: Option<Box<TypedExpr>>,
        ty: Type,
        span: Span,
    },
    Block(Vec<TypedStmt>, Type, Span), // tipo do bloco = tipo da última expr
    Func {
        params: Vec<TypedParam>,
        name: Option<String>,
        ret: Type,
        body: Box<TypedExpr>,
        span: Span,
    },
    Struct {
        name: String,
        fields: Vec<(String, TypedExpr)>,
        span: Span,
    },
    Cast {
        object: Box<TypedExpr>,
        target_type: Type,
        span: Span,
    },
    Path {
        namespace: Box<TypedExpr>,
        member: String,
        ty: Type,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct TypedParam {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypedStmt {
    Expr(TypedExpr),
    VarDecl {
        name: String,
        ty: Type,
        value: Box<TypedExpr>,
        is_mutable: bool,
        is_public: bool,
        span: Span,
    },
    StructDecl {
        name: String,
        fields: Vec<(String, Type)>,
        is_public: bool,
        span: Span,
    },
    While {
        cond: Box<TypedExpr>,
        body: Box<TypedExpr>,
        span: Span,
    },
    Return(Option<TypedExpr>, Span),
}

#[derive(Debug, Clone)]
pub struct TypedAst {
    pub stmts: Vec<TypedStmt>,
}
