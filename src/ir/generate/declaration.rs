use crate::ast::{ConstDecl, Decl, Exp, VarDecl};

use super::{
    super::{
        IRBuilder,
        context::{Immediate, Type, Value, VariableAddress},
        error::IRBuilderErr,
        symbol::{Object, Symbol},
    },
    initializer::{InitNode, format_aggregate, normalize_initializer, scalar_count},
};

impl IRBuilder {
    pub(super) fn gen_decl(&mut self, decl: &Decl) -> Result<(), IRBuilderErr> {
        match decl {
            Decl::ConstDecl(const_decl) => self.gen_const_decl(const_decl),
            Decl::VarDecl(var_decl) => self.gen_var_decl(var_decl),
        }
    }

    fn gen_var_decl(&mut self, var_decl: &VarDecl) -> Result<(), IRBuilderErr> {
        for def in &var_decl.var_defs {
            let ty = self.build_decl_type(var_decl.b_type, &def.dimensions)?;
            let variable = self.context.new_variable(&def.id);

            emit_instruction!(self, "{variable} = alloc {ty}");
            self.context.define_symbol(
                &def.id,
                Symbol::Object(Object {
                    address: variable.clone(),
                    ty: ty.clone(),
                    mutable: true,
                    const_values: None,
                }),
            )?;

            match &def.init {
                Some(init) => {
                    let values = normalize_initializer(&ty, InitNode::from(init))?;
                    self.emit_local_initializer(variable.into(), &ty, &values)?;
                }
                None if ty == Type::I32 => {
                    emit_instruction!(self, "store 0, {variable}");
                }
                None => {}
            }
        }
        Ok(())
    }

    fn gen_const_decl(&mut self, const_decl: &ConstDecl) -> Result<(), IRBuilderErr> {
        for def in &const_decl.const_defs {
            let ty = self.build_decl_type(const_decl.b_type, &def.dimensions)?;
            let normalized = normalize_initializer(&ty, InitNode::from(&def.init))?;
            let values = self.eval_constant_initializer(&normalized)?;

            if ty == Type::I32 {
                self.context
                    .define_symbol(&def.id, Symbol::Const(Immediate::from(values[0])))?;
                continue;
            }

            let variable = self.context.new_variable(&def.id);
            emit_instruction!(self, "{variable} = alloc {ty}");
            self.emit_constant_local_initializer(variable.clone().into(), &ty, &values)?;
            self.context.define_symbol(
                &def.id,
                Symbol::Object(Object {
                    address: variable,
                    ty,
                    mutable: false,
                    const_values: Some(values),
                }),
            )?;
        }
        Ok(())
    }

    pub(crate) fn gen_global_decl(&mut self, decl: &Decl) -> Result<(), IRBuilderErr> {
        match decl {
            Decl::ConstDecl(const_decl) => self.gen_global_const_decl(const_decl),
            Decl::VarDecl(var_decl) => self.gen_global_var_decl(var_decl),
        }
    }

    fn gen_global_var_decl(&mut self, var_decl: &VarDecl) -> Result<(), IRBuilderErr> {
        for def in &var_decl.var_defs {
            let ty = self.build_decl_type(var_decl.b_type, &def.dimensions)?;
            let values = match &def.init {
                Some(init) => {
                    let normalized = normalize_initializer(&ty, InitNode::from(init))?;
                    self.eval_constant_initializer(&normalized)?
                }
                None => vec![0; scalar_count(&ty)],
            };
            let variable = self.context.new_global_variable(&def.id);

            self.emit_global_alloc(&variable, &ty, &values)?;
            self.context.define_global_symbol(
                &def.id,
                Symbol::Object(Object {
                    address: variable,
                    ty,
                    mutable: true,
                    const_values: None,
                }),
            )?;
        }
        Ok(())
    }

    fn gen_global_const_decl(&mut self, const_decl: &ConstDecl) -> Result<(), IRBuilderErr> {
        for def in &const_decl.const_defs {
            let ty = self.build_decl_type(const_decl.b_type, &def.dimensions)?;
            let normalized = normalize_initializer(&ty, InitNode::from(&def.init))?;
            let values = self.eval_constant_initializer(&normalized)?;

            if ty == Type::I32 {
                self.context
                    .define_global_symbol(&def.id, Symbol::Const(Immediate::from(values[0])))?;
                continue;
            }

            let variable = self.context.new_global_variable(&def.id);
            self.emit_global_alloc(&variable, &ty, &values)?;
            self.context.define_global_symbol(
                &def.id,
                Symbol::Object(Object {
                    address: variable,
                    ty,
                    mutable: false,
                    const_values: Some(values),
                }),
            )?;
        }
        Ok(())
    }

    fn emit_local_initializer(
        &mut self,
        base: Value,
        ty: &Type,
        values: &[Option<&Exp>],
    ) -> Result<(), IRBuilderErr> {
        for (index, init) in values.iter().enumerate() {
            let value = match init {
                Some(exp) => Self::expect_i32(self.gen_value_exp(exp)?)?,
                None => 0.into(),
            };
            let address = self.element_address(base.clone(), ty, index)?;
            emit_instruction!(self, "store {value}, {address}");
        }
        Ok(())
    }

    fn emit_constant_local_initializer(
        &mut self,
        base: Value,
        ty: &Type,
        values: &[i32],
    ) -> Result<(), IRBuilderErr> {
        for (index, value) in values.iter().enumerate() {
            let address = self.element_address(base.clone(), ty, index)?;
            emit_instruction!(self, "store {value}, {address}");
        }
        Ok(())
    }

    fn element_address(
        &mut self,
        mut address: Value,
        ty: &Type,
        mut flat_index: usize,
    ) -> Result<Value, IRBuilderErr> {
        let mut current = ty;
        while let Type::Array(element, _) = current {
            let element_count = scalar_count(element);
            let index = flat_index / element_count;
            flat_index %= element_count;
            let index = i32::try_from(index).map_err(|_| IRBuilderErr::ArraySizeOverflow)?;
            address = self.emit_getelemptr(address, index.into())?;
            current = element;
        }
        Ok(address)
    }

    fn eval_constant_initializer(&self, values: &[Option<&Exp>]) -> Result<Vec<i32>, IRBuilderErr> {
        values
            .iter()
            .map(|value| value.map_or(Ok(0), |exp| self.eval_exp(exp)))
            .collect()
    }

    fn emit_global_alloc(
        &mut self,
        variable: &VariableAddress,
        ty: &Type,
        values: &[i32],
    ) -> Result<(), IRBuilderErr> {
        let initializer = if values.iter().all(|value| *value == 0) {
            "zeroinit".to_owned()
        } else if *ty == Type::I32 {
            values[0].to_string()
        } else {
            format_aggregate(ty, values)
        };
        emit_line!(self, "global {variable} = alloc {ty}, {initializer}");
        Ok(())
    }
}
