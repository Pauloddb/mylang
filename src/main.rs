mod lexer;
mod parser;
mod properties;
mod typechecker;

use std::{path::PathBuf, str::FromStr};

use anyhow::{Context, Result};

use crate::{lexer::Lexer, parser::Parser, typechecker::TypeChecker};

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<String>>();

    let filepath = if let Some(s) = args.get(1) {
        s.clone()
    } else {
        anyhow::bail!("Filename was not provided.");
    };

    let source_code = std::fs::read_to_string(filepath.clone())
        .context(format!("File `{}` doesn't exists", filepath))?;

    let tokens = Lexer::new(&source_code, filepath.clone()).lex()?;
    let ast = Parser::new(tokens.clone()).parse()?;

    let typed_ast = TypeChecker::new(PathBuf::from_str(&filepath)?).check(&ast)?;

    dbg!(&typed_ast);
    Ok(())
}
