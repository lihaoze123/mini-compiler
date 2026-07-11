use koopa::ir::{
    BasicBlock, Value,
    values::{Branch, Call, Jump},
};

use super::{FunctionGenerator, GenerateAsmError, strip_prefix};

impl FunctionGenerator<'_, '_> {
    pub(super) fn gen_branch(&mut self, branch: &Branch) -> Result<(), GenerateAsmError> {
        self.load_to(branch.cond(), "t0")?;

        let true_bb = self.basic_block_name(branch.true_bb())?;
        let false_bb = self.basic_block_name(branch.false_bb())?;

        if branch.true_args().is_empty() && branch.false_args().is_empty() {
            emit_instruction!(self, "bnez t0, {true_bb}");
            emit_instruction!(self, "j {false_bb}");
        } else {
            let false_edge = self.context.new_edge_label();
            emit_instruction!(self, "beqz t0, {false_edge}");

            self.pass_bb_args(branch.true_bb(), branch.true_args())?;
            emit_instruction!(self, "j {true_bb}");

            emit_line!(self, "{false_edge}:");
            self.pass_bb_args(branch.false_bb(), branch.false_args())?;
            emit_instruction!(self, "j {false_bb}");
        }

        Ok(())
    }

    pub(super) fn gen_jump(&mut self, jump: &Jump) -> Result<(), GenerateAsmError> {
        let target_bb = self.basic_block_name(jump.target())?;
        self.pass_bb_args(jump.target(), jump.args())?;
        emit_instruction!(self, "j {target_bb}");
        Ok(())
    }

    fn pass_bb_args(&mut self, target: BasicBlock, args: &[Value]) -> Result<(), GenerateAsmError> {
        let params = self.func_data.dfg().bb(target).params();
        if params.len() != args.len() {
            return Err(GenerateAsmError::Unknown);
        }

        for (index, &arg) in args.iter().enumerate() {
            let scratch = self.frame.arg_scratch_slot(index)?;
            self.load_to(arg, "t1")?;
            self.emit_stack_store("t1", scratch)?;
        }

        for (index, &param) in params.iter().enumerate() {
            let scratch = self.frame.arg_scratch_slot(index)?;
            self.emit_stack_load("t1", scratch)?;
            self.store_from(param, "t1")?;
        }

        Ok(())
    }

    pub(super) fn gen_call(&mut self, value: Value, call: &Call) -> Result<(), GenerateAsmError> {
        for (index, &arg) in call.args().iter().enumerate() {
            if index < 8 {
                self.load_to(arg, &format!("a{index}"))?;
            } else {
                let outgoing_arg = self.frame.outgoing_args_slot(index - 8)?;
                self.load_to(arg, "t1")?;
                self.emit_stack_store("t1", outgoing_arg)?;
            }
        }

        let callee_name = strip_prefix(self.program.func(call.callee()).name())?;
        emit_instruction!(self, "call {callee_name}");

        if !self.func_data.dfg().value(value).ty().is_unit() {
            self.store_from(value, "a0")?;
        }

        Ok(())
    }
}
