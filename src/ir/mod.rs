mod context;
mod error;
mod symbol;

macro_rules! emit_instruction {
    ($builder:expr, $($arg:tt)*) => {
        $builder.context.emit_instruction(format_args!($($arg)*))?
    };
}

macro_rules! emit_line {
    ($builder:expr, $($arg:tt)*) => {
        $builder.context.emit_line(format_args!($($arg)*))?
    };
}

mod const_eval;
mod generate;

use crate::ast::CompUnit;
use context::IRContext;

pub use error::IRBuilderErr;

#[derive(Default)]
pub struct IRBuilder {
    context: IRContext,
}

impl IRBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gen_comp_unit(&mut self, comp_unit: &CompUnit) -> Result<String, IRBuilderErr> {
        self.context.reset_generation();
        self.gen_func_def(&comp_unit.func_def)?;
        Ok(self.context.take_output())
    }
}
