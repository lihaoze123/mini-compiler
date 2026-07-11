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

            ir_builder.gen_comp_unit(&ast)?
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

#[cfg(test)]
mod tests {
    use crate::ast::{
        BlockItem, CompUnit, CompUnitItem, ConstInitVal, Decl, FuncDeclParamType, FuncParamType,
        InitVal, Stmt,
    };

    fn parse(source: &str) -> CompUnit {
        crate::sysy::CompUnitParser::new()
            .parse(source)
            .unwrap_or_else(|error| panic!("failed to parse test input: {error:?}"))
    }

    fn generate_ir(source: &str) -> Result<String, crate::ir::IRBuilderErr> {
        let unit = parse(source);
        crate::ir::IRBuilder::new().gen_comp_unit(&unit)
    }

    fn generate_asm(source: &str) -> String {
        let ir = generate_ir(source).expect("failed to generate Koopa IR");
        let program = crate::asm::str_to_program(&ir).expect("failed to parse generated Koopa IR");
        crate::asm::generate_asm(&program).expect("failed to generate RISC-V assembly")
    }

    #[test]
    fn parses_multidimensional_array_initializer() {
        let unit = parse("int a[2][3] = {{1, 2}, {3}};");
        let CompUnitItem::Decl(Decl::VarDecl(decl)) = &unit.items[0] else {
            panic!("expected a variable declaration");
        };
        let def = &decl.var_defs[0];

        assert_eq!(def.id.id, "a");
        assert_eq!(def.dimensions.len(), 2);

        let Some(InitVal::List(rows)) = &def.init else {
            panic!("expected a list initializer");
        };
        assert_eq!(rows.len(), 2);
        assert!(matches!(&rows[0], InitVal::List(values) if values.len() == 2));
        assert!(matches!(&rows[1], InitVal::List(values) if values.len() == 1));
    }

    #[test]
    fn parses_const_array_initializer() {
        let unit = parse("const int a[2][2] = {{1}, {2, 3}};");
        let CompUnitItem::Decl(Decl::ConstDecl(decl)) = &unit.items[0] else {
            panic!("expected a constant declaration");
        };
        let def = &decl.const_defs[0];

        assert_eq!(def.dimensions.len(), 2);
        let ConstInitVal::List(rows) = &def.init else {
            panic!("expected a list initializer");
        };
        assert_eq!(rows.len(), 2);
        assert!(matches!(&rows[0], ConstInitVal::List(values) if values.len() == 1));
        assert!(matches!(&rows[1], ConstInitVal::List(values) if values.len() == 2));
    }

    #[test]
    fn parses_array_parameter_and_indexed_assignment() {
        let unit = parse("int f(int a[][3], int i) { a[i][1] = 2; return 0; }");
        let CompUnitItem::FuncDef(function) = &unit.items[0] else {
            panic!("expected a function definition");
        };
        let params = function.params.as_deref().expect("expected parameters");

        assert_eq!(params.len(), 2);
        assert!(matches!(
            &params[0].ty,
            FuncParamType::Array {
                trailing_dimensions,
                ..
            } if trailing_dimensions.len() == 1
        ));
        assert!(matches!(&params[1].ty, FuncParamType::Scalar(_)));

        let BlockItem::Stmt(Stmt::Assign(l_val, _)) = &function.block.items[0] else {
            panic!("expected an indexed assignment");
        };
        assert_eq!(l_val.id.id, "a");
        assert_eq!(l_val.indices.len(), 2);
    }

    #[test]
    fn parses_array_type_in_function_declaration() {
        let unit = parse("extern int f(int[][3], int);");
        let CompUnitItem::FuncDecl(function) = &unit.items[0] else {
            panic!("expected a function declaration");
        };

        assert_eq!(function.params.len(), 2);
        assert!(matches!(
            &function.params[0],
            FuncDeclParamType::Array {
                trailing_dimensions,
                ..
            } if trailing_dimensions.len() == 1
        ));
        assert!(matches!(&function.params[1], FuncDeclParamType::Scalar(_)));
    }

    #[test]
    fn rejects_empty_dimension_in_variable_declaration() {
        let result = crate::sysy::CompUnitParser::new().parse("int a[];");
        assert!(result.is_err());
    }

    #[test]
    fn scalar_program_still_parses() {
        let unit = parse("int main() { int x = 1; x++; return x; }");
        assert_eq!(unit.items.len(), 1);
    }

    #[test]
    fn lowers_multidimensional_arrays_and_parameters() {
        let source = r#"
            int matrix[2][3] = {{1, 2}, {3}};
            int get(int values[][3], int row, int col) {
                return values[row][col];
            }
            int main() {
                int local[2][3] = {1, 2, 3, 4};
                local[1][2] = matrix[1][0];
                return get(local, 1, 2);
            }
        "#;
        let ir = generate_ir(source).expect("failed to generate array IR");

        assert!(ir.contains("global @matrix = alloc [[i32, 3], 2], {{1, 2, 0}, {3, 0, 0}}"));
        assert!(ir.contains("fun @get(%arg_0: *[i32, 3]"));
        assert!(ir.contains(" = getptr "));
        assert!(ir.contains(" = getelemptr "));
        assert!(ir.contains("call @get("));
        crate::asm::str_to_program(&ir).expect("generated array IR should be valid Koopa");
    }

    #[test]
    fn lowers_const_array_values_in_dimensions() {
        let source = r#"
            const int dimensions[2] = {2, 3};
            int matrix[dimensions[0]][dimensions[1]];
            int main() { return matrix[1][2]; }
        "#;
        let ir = generate_ir(source).expect("failed to evaluate const array dimensions");

        assert!(ir.contains("global @matrix = alloc [[i32, 3], 2], zeroinit"));
    }

    #[test]
    fn rejects_excess_array_initializers() {
        let error = generate_ir("int main() { int values[2] = {1, 2, 3}; return 0; }")
            .expect_err("excess initializers should be rejected");
        assert!(matches!(
            error,
            crate::ir::IRBuilderErr::TooManyInitializers(_)
        ));
    }

    #[test]
    fn rejects_mismatched_array_parameter_shape() {
        let source = r#"
            int first(int values[][3]) { return values[0][0]; }
            int main() {
                int values[2][4];
                return first(values);
            }
        "#;
        let error = generate_ir(source).expect_err("array shapes should be type checked");
        assert!(matches!(
            error,
            crate::ir::IRBuilderErr::ArgumentTypeMismatch { .. }
        ));
    }

    #[test]
    fn aligns_nested_initializers_to_the_current_subarray() {
        let source = r#"
            int values[2][3][4] = {1, 2, 3, 4, {5}, {}};
            int main() { return values[0][1][0]; }
        "#;
        let ir = generate_ir(source).expect("nested initializer alignment should be supported");

        assert!(ir.contains("{{{1, 2, 3, 4}, {5, 0, 0, 0}, {0, 0, 0, 0}}"));
        crate::asm::str_to_program(&ir).expect("aligned initializer should produce valid Koopa");
    }

    #[test]
    fn function_parameters_do_not_collide_with_globals() {
        let source = r#"
            int n;
            int first(int values[], int n) { return values[n]; }
            int main() {
                int values[1] = {7};
                return first(values, n);
            }
        "#;
        let ir = generate_ir(source).expect("global and parameter names may be identical");

        assert!(ir.contains("fun @first(%arg_0: *i32, %arg_1: i32)"));
        crate::asm::str_to_program(&ir).expect("parameter names should resolve locally");
    }

    #[test]
    fn emits_array_assembly_and_handles_large_stack_offsets() {
        let source = r#"
            int main() {
                int values[2000];
                values[1999] = 7;
                return values[1999];
            }
        "#;
        let assembly = generate_asm(source);

        assert!(assembly.contains("li t0, -"));
        assert!(assembly.contains("add sp, sp, t0"));
        assert!(assembly.contains("li t2, 4"));
        for line in assembly.lines().map(str::trim) {
            if let Some(immediate) = line.strip_prefix("addi sp, sp, ") {
                let immediate = immediate.parse::<i64>().expect("invalid addi immediate");
                assert!((-2048..=2047).contains(&immediate));
            }
        }
    }
}
