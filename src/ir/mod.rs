mod context;
mod error;
mod lib_func;
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
use lib_func::LIB_FUNCS;

pub use error::IRBuilderErr;

#[derive(Default)]
pub struct IRBuilder {
    context: IRContext,
    lib_funcs_registered: bool,
}

impl IRBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gen_comp_unit(&mut self, comp_unit: &CompUnit) -> Result<String, IRBuilderErr> {
        self.register_lib_funcs()?;

        let mut output = String::from(LIB_FUNCS);
        output.push('\n');
        self.gen_comp_unit_body(comp_unit, &mut output)?;
        Ok(output)
    }

    fn gen_comp_unit_body(
        &mut self,
        comp_unit: &CompUnit,
        output: &mut String,
    ) -> Result<(), IRBuilderErr> {
        match comp_unit {
            CompUnit::FuncDef(func_def) => {
                self.gen_function_into(func_def, output)?;
            }
            CompUnit::CompUnit(comp_unit, func_def) => {
                self.gen_comp_unit_body(comp_unit, output)?;
                self.gen_function_into(func_def, output)?;
            }
        }
        Ok(())
    }

    fn gen_function_into(
        &mut self,
        func_def: &crate::ast::FuncDef,
        output: &mut String,
    ) -> Result<(), IRBuilderErr> {
        self.context.reset_generation();
        self.gen_func_def(func_def)?;
        output.push_str(&self.context.take_output());
        Ok(())
    }
}
