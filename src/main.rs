use anyhow::{Result, anyhow};
use clap::Parser;
use lalrpop_util::lalrpop_mod;
use std::fs::{read_to_string, write};
use std::path::PathBuf;

use crate::ir::IRBuilder;

mod asm;
mod ast;
mod ir;

lalrpop_mod!(sysy);

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(
        value_name = "MODE",
        value_parser = parse_mode,
        allow_hyphen_values = true,
        help = "Output mode: -koopa, -riscv or -ast"
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
    Ast,
}

fn parse_mode(mode: &str) -> Result<Mode, String> {
    match mode {
        "-koopa" => Ok(Mode::Koopa),
        "-riscv" => Ok(Mode::Riscv),
        "-ast" => Ok(Mode::Ast),
        _ => Err("expected -koopa, -riscv or -ast".to_owned()),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let input = read_to_string(&cli.input)?;

    let ast = sysy::CompUnitParser::new()
        .parse(&input)
        .map_err(|err| anyhow!("failed to parse input: {err:?}"))?;

    let res = match cli.mode {
        Mode::Koopa => {
            let mut ir_builder = IRBuilder::new();
            let koopa_ir = ir_builder.gen_comp_unit(&ast)?;
            koopa_ir
        }
        Mode::Riscv => {
            let mut ir_builder = IRBuilder::new();
            let koopa_ir = ir_builder.gen_comp_unit(&ast)?;
            let program = asm::str_to_program(&koopa_ir)?;
            asm::generate_asm(&program)?
        }
        Mode::Ast => {
            format!("{:#?}", ast)
        }
    };

    if let Some(output_file) = cli.output {
        write(&output_file, res)?;
    } else {
        println!("{}", res);
    }

    Ok(())
}
