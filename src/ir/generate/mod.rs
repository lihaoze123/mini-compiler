mod declaration;
mod expression;
mod statement;

use crate::{
    ast::FuncDef,
    ir::{
        context::{Func, Type},
        symbol::Symbol,
    },
};

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
        let params = func_def.params.as_deref().unwrap_or_default();
        let return_type = Type::from(func_def.func_type);

        self.context.define_global_symbol(
            &func_def.id,
            Symbol::Func(Func {
                identifier: func_def.id.id.clone(),
                params: params
                    .iter()
                    .map(|param| Type::from(param.b_type))
                    .collect(),
                ret: return_type,
            }),
        )?;

        self.context.enter_scope();
        self.context.set_current_return_type(Some(return_type));

        let mut param_variables = Vec::new();

        for param in params {
            let variable = self.context.new_variable(&param.id);
            self.context
                .define_symbol(&param.id, Symbol::Var(variable.clone()))?;
            param_variables.push((param, variable));
        }

        let result = (|| {
            let params = params
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let return_type = match return_type {
                Type::I32 => ": i32",
                Type::Void => "",
            };

            emit_line!(self, "fun {}({params}){return_type} {{", func_def.id);
            emit_line!(self, "%entry:");

            for (param, variable) in param_variables {
                emit_instruction!(self, "{variable} = alloc {}", param.b_type);
                emit_instruction!(self, "store {}, {variable}", param.id);
            }

            let flow = self.gen_block(&func_def.block)?;
            if !flow.is_terminated() {
                match Type::from(func_def.func_type) {
                    Type::I32 => {
                        return Err(IRBuilderErr::MissingFunctionReturn(func_def.id.to_string()));
                    }
                    Type::Void => emit_instruction!(self, "ret"),
                }
            }
            emit_line!(self, "}}");
            Ok(())
        })();

        self.context.exit_scope();
        self.context.set_current_return_type(None);

        result
    }
}
