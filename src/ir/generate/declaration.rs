use crate::ast::{BType, ConstDecl, ConstInitVal, Decl, Ident, InitVal, VarDecl, VarDef};

use super::super::{
    IRBuilder,
    context::{Immediate, Value},
    error::IRBuilderErr,
    symbol::Symbol,
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
            match def {
                VarDef::ID(id) => self.gen_var_def(&var_decl.b_type, id, None)?,
                VarDef::InitVal(id, init_val) => {
                    self.gen_var_def(&var_decl.b_type, id, Some(init_val))?
                }
            }
        }
        Ok(())
    }

    fn gen_var_def(
        &mut self,
        b_type: &BType,
        id: &Ident,
        init_val: Option<&InitVal>,
    ) -> Result<(), IRBuilderErr> {
        let value = match init_val {
            Some(init_val) => self.gen_value_exp(&init_val.exp)?,
            None => Value::from(0),
        };

        let variable = self.context.new_variable(id);
        self.context
            .define_symbol(id, Symbol::Var(variable.clone()))?;

        emit_instruction!(self, "{variable} = alloc {b_type}");
        emit_instruction!(self, "store {value}, {variable}");
        Ok(())
    }

    fn gen_const_decl(&mut self, const_decl: &ConstDecl) -> Result<(), IRBuilderErr> {
        let _ = const_decl.b_type;
        for def in &const_decl.const_defs {
            self.gen_const_def(&def.id, &def.const_init_val)?;
        }
        Ok(())
    }

    fn gen_const_def(
        &mut self,
        id: &Ident,
        const_init_val: &ConstInitVal,
    ) -> Result<(), IRBuilderErr> {
        let value = self.eval_exp(&const_init_val.const_exp.exp)?;
        self.context
            .define_symbol(id, Symbol::Const(Immediate::from(value)))?;
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
            let (id, initializer) = match def {
                VarDef::ID(id) => (id, None),
                VarDef::InitVal(id, init_val) => (id, Some(self.eval_exp(&init_val.exp)?)),
            };

            let variable = self.context.new_global_variable(id);
            self.context
                .define_global_symbol(id, Symbol::Var(variable.clone()))?;

            match initializer {
                Some(value) => {
                    emit_line!(
                        self,
                        "global {variable} = alloc {}, {value}",
                        var_decl.b_type
                    )
                }
                None => emit_line!(
                    self,
                    "global {variable} = alloc {}, zeroinit",
                    var_decl.b_type
                ),
            }
        }
        Ok(())
    }

    fn gen_global_const_decl(&mut self, const_decl: &ConstDecl) -> Result<(), IRBuilderErr> {
        for def in &const_decl.const_defs {
            let value = self.eval_exp(&def.const_init_val.const_exp.exp)?;
            self.context
                .define_global_symbol(&def.id, Symbol::Const(Immediate::from(value)))?;
        }
        Ok(())
    }
}
