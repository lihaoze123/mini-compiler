use crate::ast::{Block, BlockItem, Exp, LVal, Stmt};

use super::{
    super::{IRBuilder, context::Label, error::IRBuilderErr, symbol::Symbol},
    ControlFlow,
};

impl IRBuilder {
    pub(super) fn gen_block(&mut self, block: &Block) -> Result<ControlFlow, IRBuilderErr> {
        self.context.enter_scope();

        let mut flow = ControlFlow::Continues;
        for item in &block.items {
            flow = self.gen_block_item(item)?;
            if flow.is_terminated() {
                break;
            }
        }

        self.context.exit_scope();
        Ok(flow)
    }

    fn gen_block_item(&mut self, block: &BlockItem) -> Result<ControlFlow, IRBuilderErr> {
        match block {
            BlockItem::Decl(decl) => {
                self.gen_decl(decl)?;
                Ok(ControlFlow::Continues)
            }
            BlockItem::Stmt(stmt) => self.gen_stmt(stmt),
        }
    }

    fn gen_stmt(&mut self, stmt: &Stmt) -> Result<ControlFlow, IRBuilderErr> {
        match stmt {
            Stmt::Return(ret) => {
                self.gen_return(ret)?;
                Ok(ControlFlow::Terminated)
            }
            Stmt::Assign(l_val, exp) => {
                self.gen_assign(l_val, exp)?;
                Ok(ControlFlow::Continues)
            }
            Stmt::Exp(Some(exp)) => {
                self.gen_exp(exp)?;
                Ok(ControlFlow::Continues)
            }
            Stmt::Exp(None) => Ok(ControlFlow::Continues),
            Stmt::Block(block) => self.gen_block(block),
            Stmt::If(cond, then_stmt, else_stmt) => {
                self.gen_if(cond, then_stmt.as_ref(), else_stmt.as_deref())
            }
        }
    }

    fn gen_if(
        &mut self,
        cond: &Exp,
        then_stmt: &Stmt,
        else_stmt: Option<&Stmt>,
    ) -> Result<ControlFlow, IRBuilderErr> {
        let entry_label = self.context.new_label("if_entry");
        emit_instruction!(self, "jump {entry_label}");
        emit_line!(self, "{entry_label}:");

        let value = self.gen_exp(cond)?;

        let true_label = self.context.new_label("then");
        let end_label = self.context.new_label("end");
        let false_label = match else_stmt {
            Some(_) => self.context.new_label("else"),
            None => end_label.clone(),
        };

        emit_instruction!(self, "br {value}, {true_label}, {false_label}");

        let then_flow = self.gen_if_arm(&true_label, Some(then_stmt), &end_label)?;
        let else_flow = self.gen_if_arm(&false_label, else_stmt, &end_label)?;

        let flow = if then_flow.is_terminated() && else_flow.is_terminated() {
            ControlFlow::Terminated
        } else {
            ControlFlow::Continues
        };
        if !flow.is_terminated() {
            emit_line!(self, "{end_label}:");
        }
        Ok(flow)
    }

    fn gen_if_arm(
        &mut self,
        label: &Label,
        stmt: Option<&Stmt>,
        end_label: &Label,
    ) -> Result<ControlFlow, IRBuilderErr> {
        match stmt {
            Some(stmt) => {
                emit_line!(self, "{label}:");

                let flow = self.gen_stmt(stmt)?;
                if !flow.is_terminated() {
                    emit_instruction!(self, "jump {end_label}");
                }

                Ok(flow)
            }
            None => Ok(ControlFlow::Continues),
        }
    }

    fn gen_return(&mut self, ret: &Exp) -> Result<(), IRBuilderErr> {
        let value = self.gen_exp(ret)?;
        emit_instruction!(self, "ret {value}");
        Ok(())
    }

    fn gen_assign(&mut self, l_val: &LVal, exp: &Exp) -> Result<(), IRBuilderErr> {
        match self.context.get_symbol(&l_val.id)? {
            Symbol::Const(_) => Err(IRBuilderErr::AssignToConst(l_val.id.to_string())),
            Symbol::Var(variable) => {
                let value = self.gen_exp(exp)?;
                emit_instruction!(self, "store {value}, {variable}");
                Ok(())
            }
        }
    }
}
