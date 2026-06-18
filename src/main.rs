use lalrpop_util::lalrpop_mod;
use std::env::args;
use std::fs::{read_to_string, write};
use std::io::Result;

use crate::ast::ToKoopa;
mod ast;

lalrpop_mod!(sysy);

fn main() -> Result<()> {
    let mut args = args();
    args.next();
    let mode = args.next().unwrap();
    let input = args.next().unwrap();
    args.next();
    let output = args.next();

    let input = read_to_string(input)?;

    let ast = sysy::CompUnitParser::new().parse(&input).unwrap();

    let res = if mode == "-koopa" {
        ast.to_koopa()
    } else {
        format!("{:#?}", ast)
    };

    if let Some(output_file) = output {
        write(output_file, res)?;
    } else {
        println!("{}", res);
    }

    Ok(())
}
