use anyhow::{Result, anyhow};
use clap::Parser;
use lalrpop_util::lalrpop_mod;
use std::fs::{read_to_string, write};
use std::path::PathBuf;

use crate::ast::ToKoopa;
mod asm;
mod ast;

lalrpop_mod!(sysy);

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(
        value_name = "MODE",
        value_parser = parse_mode,
        allow_hyphen_values = true,
        help = "Output mode: -koopa or -riscv"
    )]
    mode: Mode,

    #[arg(value_name = "INPUT")]
    input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    Koopa,
    Riscv,
}

fn parse_mode(mode: &str) -> Result<Mode, String> {
    match mode {
        "-koopa" => Ok(Mode::Koopa),
        "-riscv" => Ok(Mode::Riscv),
        _ => Err("expected -koopa or -riscv".to_owned()),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let input = read_to_string(&cli.input)?;

    let ast = sysy::CompUnitParser::new()
        .parse(&input)
        .map_err(|err| anyhow!("failed to parse input: {err:?}"))?;

    let res = match cli.mode {
        Mode::Koopa => ast.to_koopa(),
        Mode::Riscv => {
            let koopa_ir = ast.to_koopa();
            let program = asm::str_to_program(&koopa_ir)?;
            asm::generate_asm(&program)?
        }
    };

    if let Some(output_file) = cli.output {
        write(&output_file, res)?;
    } else {
        println!("{}", res);
    }

    Ok(())
}
