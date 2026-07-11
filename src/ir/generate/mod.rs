mod declaration;
mod expression;
mod initializer;
mod statement;

use crate::{
    ast::{BType, ConstExp, FuncDecl, FuncDeclParamType, FuncDef, FuncParamType},
    ir::{
        context::{Func, Type},
        symbol::{Object, Symbol},
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
    pub(super) fn register_func(&mut self, func_def: &FuncDef) -> Result<(), IRBuilderErr> {
        let params = func_def.params.as_deref().unwrap_or_default();
        let param_types = params
            .iter()
            .map(|param| self.lower_func_param_type(&param.ty))
            .collect::<Result<Vec<_>, _>>()?;
        self.context.register_global_func(
            &func_def.id,
            Func {
                identifier: func_def.id.id.clone(),
                params: param_types,
                ret: Type::from(func_def.func_type),
                defined: true,
            },
        )
    }

    pub(super) fn register_func_decl(&mut self, func_decl: &FuncDecl) -> Result<(), IRBuilderErr> {
        let param_types = func_decl
            .params
            .iter()
            .map(|param| self.lower_func_decl_param_type(param))
            .collect::<Result<Vec<_>, _>>()?;
        self.context.register_global_func(
            &func_decl.id,
            Func {
                identifier: func_decl.id.id.clone(),
                params: param_types,
                ret: Type::from(func_decl.func_type),
                defined: false,
            },
        )
    }

    pub(super) fn gen_func_decl(&mut self, func_decl: &FuncDecl) -> Result<(), IRBuilderErr> {
        let _ = func_decl.is_extern;
        let params = func_decl
            .params
            .iter()
            .map(|param| {
                self.lower_func_decl_param_type(param)
                    .map(|ty| ty.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        let return_type = match func_decl.func_type {
            crate::ast::FuncType::Int => ": i32",
            crate::ast::FuncType::Void => "",
        };
        emit_line!(self, "decl {}({params}){return_type}", func_decl.id);
        Ok(())
    }

    pub(super) fn gen_func_def(&mut self, func_def: &FuncDef) -> Result<(), IRBuilderErr> {
        let params = func_def.params.as_deref().unwrap_or_default();
        let return_type = Type::from(func_def.func_type);

        self.context.enter_scope();
        self.context
            .set_current_return_type(Some(return_type.clone()));

        let mut param_variables = Vec::new();

        for (index, param) in params.iter().enumerate() {
            let param_type = self.lower_func_param_type(&param.ty)?;
            let variable = self.context.new_variable(&param.id);
            let parameter_name = format!("%arg_{index}");
            self.context.define_symbol(
                &param.id,
                Symbol::Object(Object {
                    address: variable.clone(),
                    ty: param_type.clone(),
                    mutable: true,
                    const_values: None,
                }),
            )?;
            param_variables.push((param, variable, param_type, parameter_name));
        }

        let result = (|| {
            let params = params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    self.lower_func_param_type(&param.ty)
                        .map(|ty| format!("%arg_{index}: {ty}"))
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            let return_suffix = match return_type {
                Type::I32 => ": i32",
                Type::Void => "",
                Type::Array(_, _) | Type::Pointer(_) => unreachable!(),
            };

            emit_line!(self, "fun {}({params}){return_suffix} {{", func_def.id);
            emit_line!(self, "%entry:");

            for (_param, variable, param_type, parameter_name) in param_variables {
                emit_instruction!(self, "{variable} = alloc {param_type}");
                emit_instruction!(self, "store {parameter_name}, {variable}");
            }

            let flow = self.gen_block(&func_def.block)?;
            if !flow.is_terminated() {
                match Type::from(func_def.func_type) {
                    Type::I32 => {
                        return Err(IRBuilderErr::MissingFunctionReturn(func_def.id.to_string()));
                    }
                    Type::Void => emit_instruction!(self, "ret"),
                    Type::Array(_, _) | Type::Pointer(_) => unreachable!(),
                }
            }
            emit_line!(self, "}}");
            Ok(())
        })();

        self.context.exit_scope();
        self.context.set_current_return_type(None);

        result
    }

    pub(super) fn build_decl_type(
        &self,
        base: BType,
        dimensions: &[ConstExp],
    ) -> Result<Type, IRBuilderErr> {
        let lengths = self.eval_dimensions(dimensions)?;
        Ok(lengths
            .into_iter()
            .rev()
            .fold(Type::from(base), |ty, length| {
                Type::Array(Box::new(ty), length)
            }))
    }

    fn eval_dimensions(&self, dimensions: &[ConstExp]) -> Result<Vec<usize>, IRBuilderErr> {
        let mut total_elements = 1usize;
        let lengths = dimensions
            .iter()
            .map(|dimension| {
                let length = self.eval_exp(&dimension.exp)?;
                if length <= 0 {
                    return Err(IRBuilderErr::InvalidArrayLength(length));
                }
                let length = usize::try_from(length)
                    .map_err(|_| IRBuilderErr::InvalidArrayLength(length))?;
                total_elements = total_elements
                    .checked_mul(length)
                    .ok_or(IRBuilderErr::ArraySizeOverflow)?;
                Ok(length)
            })
            .collect::<Result<Vec<_>, _>>()?;
        total_elements
            .checked_mul(4)
            .ok_or(IRBuilderErr::ArraySizeOverflow)?;
        Ok(lengths)
    }

    fn lower_func_param_type(&self, param: &FuncParamType) -> Result<Type, IRBuilderErr> {
        match param {
            FuncParamType::Scalar(base) => Ok(Type::from(*base)),
            FuncParamType::Array {
                base,
                trailing_dimensions,
            } => Ok(Type::Pointer(Box::new(
                self.build_decl_type(*base, trailing_dimensions)?,
            ))),
        }
    }

    fn lower_func_decl_param_type(&self, param: &FuncDeclParamType) -> Result<Type, IRBuilderErr> {
        match param {
            FuncDeclParamType::Scalar(base) => Ok(Type::from(*base)),
            FuncDeclParamType::Array {
                base,
                trailing_dimensions,
            } => Ok(Type::Pointer(Box::new(
                self.build_decl_type(*base, trailing_dimensions)?,
            ))),
        }
    }
}
