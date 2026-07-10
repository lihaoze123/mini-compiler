mod context;
mod error;
mod frame;

macro_rules! emit_instruction {
    ($generator:expr, $($arg:tt)*) => {
        $generator.context.emit_instruction(format_args!($($arg)*))?
    };
}

macro_rules! emit_line {
    ($generator:expr, $($arg:tt)*) => {
        $generator.context.emit_line(format_args!($($arg)*))?
    };
}

mod generate;

use koopa::ir::Program;

use context::AsmContext;

pub use error::GenerateAsmError;

pub fn str_to_program(source: &str) -> Result<Program, GenerateAsmError> {
    koopa::front::Driver::from(source)
        .generate_program()
        .map_err(GenerateAsmError::KoopaParse)
}

pub fn generate_asm(program: &Program) -> Result<String, GenerateAsmError> {
    let mut builder = AsmBuilder::new();
    builder.gen_program(program)
}

#[derive(Default)]
pub struct AsmBuilder {
    context: AsmContext,
}

impl AsmBuilder {
    pub fn new() -> Self {
        Self::default()
    }
}
