mod declaration;
mod expression;
mod statement;

use crate::ast::FuncDef;

use super::{IRBuilder, error::IRBuilderErr};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ControlFlow {
    Continues,
    Terminated,
}

impl ControlFlow {
    pub(super) fn is_terminated(self) -> bool {
        self == Self::Terminated
    }
}

impl IRBuilder {
    pub(super) fn gen_func_def(&mut self, func_def: &FuncDef) -> Result<(), IRBuilderErr> {
        emit_line!(self, "fun {}(): {} {{", func_def.id, func_def.func_type);
        emit_line!(self, "%entry:");

        self.gen_block(&func_def.block)?;
        emit_line!(self, "}}");

        Ok(())
    }
}
