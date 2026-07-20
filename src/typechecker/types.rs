use std::fmt;

use crate::{
    lexer::types::{Op, Span},
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
                let mut paren_depth = 0u32;

                let mut i = 0;
                let remainder = &s[5..];
                while i < remainder.len() {
                    let ch = remainder.chars().nth(i).unwrap();
                    match ch {
                        '(' => {
                            paren_depth += 1;
                            buf.push(ch);
                        }
                        ')' => {
                            if paren_depth == 0 {
                                if !buf.is_empty() {
                                    let ty = Type::from_str(&buf, span.clone(), registry)?;
                                    params.push(ty);
                                }
                                buf.clear();
                                break;
                            } else {
                                paren_depth -= 1;
                                buf.push(ch);
                            }
                        }
                        ',' if paren_depth == 0 => {
                            if !buf.is_empty() {
                                let ty = Type::from_str(&buf, span.clone(), registry)?;
                                params.push(ty);
                            }
                            buf.clear();
                            i += 1;
                        }
                        ch if ch.is_ascii_whitespace() && paren_depth == 0 => i += 1,
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
                let mut brackets = 0;
                let mut inner = s;
                while inner.ends_with("[]") {
                    inner = &inner[..inner.len() - 2];
                    brackets += 1;
                }
                let ty = Type::from_str(inner, span, registry)?;
                let mut result = ty;
                for _ in 0..brackets {
                    result = Self::Array(Box::new(result));
                }
                Ok(result)
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
                write!(f, "{}[]", ty)
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

#[derive(Debug, Clone, PartialEq)]
pub enum TypedAssignTarget {
    Ident(String, Span),
    Property {
        object: Box<TypedExpr>,
        prop: String,
        span: Span,
    },
    Index {
        object: Box<TypedExpr>,
        index: Box<TypedExpr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
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
        target: TypedAssignTarget,
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

impl TypedExpr {
    pub fn span(&self) -> Span {
        match self {
            TypedExpr::Int(_, span) => span.clone(),
            TypedExpr::Float(_, span) => span.clone(),
            TypedExpr::Bool(_, span) => span.clone(),
            TypedExpr::String(_, span) => span.clone(),
            TypedExpr::Nil(span) => span.clone(),
            TypedExpr::Ident(_, _, span) => span.clone(),
            TypedExpr::Unary { span, .. } => span.clone(),
            TypedExpr::Binary { span, .. } => span.clone(),
            TypedExpr::Assign { span, .. } => span.clone(),
            TypedExpr::Call { span, .. } => span.clone(),
            TypedExpr::Index { span, .. } => span.clone(),
            TypedExpr::Block(_, _, span) => span.clone(),
            TypedExpr::Func { span, .. } => span.clone(),
            TypedExpr::Cast { span, .. } => span.clone(),
            TypedExpr::Property { span, .. } => span.clone(),
            TypedExpr::If { span, .. } => span.clone(),
            TypedExpr::ArrayLiteral(_, _, span) => span.clone(),
            TypedExpr::Path { span, .. } => span.clone(),
            TypedExpr::Struct { span, .. } => span.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedParam {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
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
    Break(Span),
    Continue(Span),
}

#[derive(Debug, Clone)]
pub struct TypedAst {
    pub stmts: Vec<TypedStmt>,
}
