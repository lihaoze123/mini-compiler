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

use std::collections::HashSet;

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
            if let CompUnitItem::Decl(decl) = item {
                self.gen_global_decl(decl)?;
            }
        }
        let globals = self.context.take_output();

        for item in &comp_unit.items {
            match item {
                CompUnitItem::FuncDecl(func_decl) => self.register_func_decl(func_decl)?,
                CompUnitItem::FuncDef(func_def) => self.register_func(func_def)?,
                CompUnitItem::Decl(_) => {}
            }
        }

        let mut output = String::from(LIB_FUNCS);
        output.push('\n');

        let mut emitted_declarations = HashSet::new();
        for item in &comp_unit.items {
            if let CompUnitItem::FuncDecl(func_decl) = item
                && emitted_declarations.insert(func_decl.id.clone())
                && !self.context.is_function_defined(&func_decl.id)?
            {
                self.gen_func_decl(func_decl)?;
            }
        }

        let declarations = self.context.take_output();
        output.push_str(&declarations);
        if !declarations.is_empty() {
            output.push('\n');
        }

        output.push_str(&globals);
        if !globals.is_empty() {
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
