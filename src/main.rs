mod compiler;
mod lexer;
mod parser;
mod typechecker;
mod vm;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser as ClapParser;

use crate::{compiler::Compiler, lexer::Lexer, parser::Parser, typechecker::TypeChecker, vm::Vm};

#[derive(clap::Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
)]
struct Cli {
    filepath: PathBuf,
    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let source_code = std::fs::read_to_string(args.filepath.clone()).context(format!(
        "File `{}` doesn't exists",
        args.filepath.to_string_lossy().into_owned()
    ))?;

    let tokens = Lexer::new(&source_code, args.filepath.to_string_lossy().into_owned()).lex()?;

    let ast = Parser::new(tokens.clone()).parse()?;

    let typed_ast = TypeChecker::new(args.filepath.clone()).check(&ast)?;

    let (chunk, pub_locals) =
        Compiler::compile(args.filepath.clone().to_str().unwrap(), &typed_ast)?;

    pub_locals
        .iter()
        .for_each(|(name, slot)| println!("pub local `{}` in slot {}", name, slot));

    // chunk.disassemble();

    let _final_stack = Vm::new(chunk).run(args.debug)?;
    // dbg!(&final_stack);

    Ok(())
}
