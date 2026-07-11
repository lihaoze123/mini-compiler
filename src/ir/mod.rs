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

use crate::ast::{CompUnit, CompUnitItem};
use context::IRContext;
use lib_func::LIB_FUNCS;

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
        self.context.reset_program();
        self.register_lib_funcs()?;

        for item in &comp_unit.items {
            if let CompUnitItem::FuncDef(func_def) = item {
                self.register_func(func_def)?;
            }
        }

        let mut output = String::from(LIB_FUNCS);
        output.push('\n');

        for item in &comp_unit.items {
            if let CompUnitItem::Decl(decl) = item {
                self.gen_global_decl(decl)?;
            }
        }
        output.push_str(&self.context.take_output());
        if comp_unit
            .items
            .iter()
            .any(|item| matches!(item, CompUnitItem::Decl(_)))
        {
            output.push('\n');
        }

        for item in &comp_unit.items {
            if let CompUnitItem::FuncDef(func_def) = item {
                self.gen_function_into(func_def, &mut output)?;
            }
        }

        Ok(output)
    }

    fn gen_function_into(
        &mut self,
        func_def: &crate::ast::FuncDef,
        output: &mut String,
    ) -> Result<(), IRBuilderErr> {
        self.context.reset_function();
        self.gen_func_def(func_def)?;
        output.push_str(&self.context.take_output());
        Ok(())
    }
}
