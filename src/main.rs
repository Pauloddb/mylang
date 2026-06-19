mod lexer;
mod parser;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let source = get_source_code()?;

    let tokens = lexer::Lexer::new(&source).lex()?;
    let ast = parser::Parser::new(tokens.clone()).parse()?;

    dbg!(ast);
    Ok(())
}

fn get_source_code() -> Result<String> {
    let args = std::env::args().collect::<Vec<_>>();

    let filepath = if let Some(s) = args.get(1) {
        s.clone()
    } else {
        anyhow::bail!("Filename was not provided.");
    };

    let source_code = std::fs::read_to_string(filepath.clone())
        .context(format!("File `{}` doesn't exists", filepath))?;

    Ok(source_code)
}
