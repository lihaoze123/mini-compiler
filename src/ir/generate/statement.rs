use crate::ast::{AddOp, Block, BlockItem, Exp, LVal, Stmt};

use super::{
    super::{
        IRBuilder,
        context::{Label, Type},
        error::IRBuilderErr,
        symbol::Symbol,
    },
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
                self.gen_return(ret.as_ref())?;
                Ok(ControlFlow::Terminated)
            }
            Stmt::Assign(l_val, exp) => {
                self.gen_assign(l_val, exp)?;
                Ok(ControlFlow::Continues)
            }
            Stmt::Update(l_val, op) => {
                self.gen_update(l_val, op)?;
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
            Stmt::Loop {
                init,
                cond,
                inc,
                body,
            } => self.gen_loop(
                init.as_deref(),
                cond.as_ref(),
                inc.as_deref(),
                body.as_ref(),
            ),
            Stmt::Break => {
                let target = self.context.break_target()?;
                emit_instruction!(self, "jump {target}");
                Ok(ControlFlow::Terminated)
            }
            Stmt::Continue => {
                let target = self.context.continue_target()?;
                emit_instruction!(self, "jump {target}");
                Ok(ControlFlow::Terminated)
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

        let value = Self::expect_i32(self.gen_value_exp(cond)?)?;

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

    fn gen_loop(
        &mut self,
        init: Option<&Stmt>,
        cond: Option<&Exp>,
        inc: Option<&Stmt>,
        body: &Stmt,
    ) -> Result<ControlFlow, IRBuilderErr> {
        let init_label = self.context.new_label("loop_init");
        let cond_label = self.context.new_label("loop_cond");
        let body_label = self.context.new_label("loop_body");
        let inc_label = self.context.new_label("loop_inc");
        let end_label = self.context.new_label("loop_end");

        // initialization
        emit_instruction!(self, "jump {init_label}");
        emit_line!(self, "{init_label}:");

        if let Some(init) = init {
            self.gen_stmt(init)?;
        }

        // condition
        emit_instruction!(self, "jump {cond_label}");
        emit_line!(self, "{cond_label}:");

        if let Some(cond) = cond {
            let value = Self::expect_i32(self.gen_value_exp(cond)?)?;
            emit_instruction!(self, "br {value}, {body_label}, {end_label}");
        } else {
            emit_instruction!(self, "jump {body_label}");
        }

        // body
        emit_line!(self, "{body_label}:");
        self.context.push_loop(inc_label.clone(), end_label.clone());

        let body_result = self.gen_stmt(body);
        let loop_frame = self.context.pop_loop()?;
        let body_flow = body_result?;

        if !body_flow.is_terminated() {
            emit_instruction!(self, "jump {inc_label}");
        }

        emit_line!(self, "{inc_label}:");
        if let Some(inc) = inc {
            self.gen_stmt(inc)?;
        }
        emit_instruction!(self, "jump {cond_label}");

        if cond.is_some() || loop_frame.has_break {
            emit_line!(self, "{end_label}:");
            Ok(ControlFlow::Continues)
        } else {
            Ok(ControlFlow::Terminated)
        }
    }

    fn gen_return(&mut self, ret: Option<&Exp>) -> Result<(), IRBuilderErr> {
        match (self.context.current_return_type()?, ret) {
            (Type::I32, Some(ret)) => {
                let value = Self::expect_i32(self.gen_value_exp(ret)?)?;
                emit_instruction!(self, "ret {value}");
                Ok(())
            }
            (Type::I32, None) => Err(IRBuilderErr::MissingReturnValue),
            (Type::Void, Some(_)) => Err(IRBuilderErr::UnexpectedReturnValue),
            (Type::Void, None) => {
                emit_instruction!(self, "ret");
                Ok(())
            }
            (Type::Array(_, _) | Type::Pointer(_), _) => unreachable!(),
        }
    }

    fn gen_assign(&mut self, l_val: &LVal, exp: &Exp) -> Result<(), IRBuilderErr> {
        if matches!(self.context.get_symbol(&l_val.id)?, Symbol::Const(_)) {
            return Err(IRBuilderErr::AssignToConst(l_val.id.to_string()));
        }
        let place = self.gen_place(l_val)?;
        if !place.mutable {
            return Err(IRBuilderErr::AssignToConst(l_val.id.to_string()));
        }
        if place.ty != Type::I32 {
            return Err(IRBuilderErr::ExpectedScalarLVal(l_val.id.to_string()));
        }
        let value = Self::expect_i32(self.gen_value_exp(exp)?)?;
        emit_instruction!(self, "store {value}, {}", place.address);
        Ok(())
    }

    fn gen_update(&mut self, l_val: &LVal, op: &AddOp) -> Result<(), IRBuilderErr> {
        if matches!(self.context.get_symbol(&l_val.id)?, Symbol::Const(_)) {
            return Err(IRBuilderErr::AssignToConst(l_val.id.to_string()));
        }
        let place = self.gen_place(l_val)?;
        if !place.mutable {
            return Err(IRBuilderErr::AssignToConst(l_val.id.to_string()));
        }
        if place.ty != Type::I32 {
            return Err(IRBuilderErr::ExpectedScalarLVal(l_val.id.to_string()));
        }

        let old_value = self.context.new_temp();
        emit_instruction!(self, "{old_value} = load {}", place.address);
        let new_value = self.context.emit_binary(op, old_value.into(), 1.into())?;
        emit_instruction!(self, "store {new_value}, {}", place.address);
        Ok(())
    }
}
