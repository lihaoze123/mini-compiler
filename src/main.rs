use lalrpop_util::lalrpop_mod;
use std::env::args;
use std::fs::{read_to_string, write};

use crate::ast::ToKoopa;
mod asm;
mod ast;

lalrpop_mod!(sysy);

fn main() -> Result<(), anyhow::Error> {
    let mut args = args();
    args.next();
    let mode = args.next().unwrap();
    let input = args.next().unwrap();
    args.next();
    let output = args.next();

    let input = read_to_string(input)?;

    let ast = sysy::CompUnitParser::new().parse(&input).unwrap();

    let res = match mode.as_str() {
        "-koopa" => ast.to_koopa(),
        "-riscv" => {
            let koopa_ir = ast.to_koopa();
            let program = asm::str_to_program(&koopa_ir)?;
            asm::generate_asm(&program)?
        }
        _ => unimplemented!(),
    };

    if let Some(output_file) = output {
        write(output_file, res)?;
    } else {
        println!("{}", res);
    }

    Ok(())
}
