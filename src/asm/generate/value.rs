use koopa::ir::{Value, ValueKind};

use super::{FunctionGenerator, GenerateAsmError};

impl FunctionGenerator<'_, '_> {
    pub(super) fn gen_value(&mut self, value: Value) -> Result<(), GenerateAsmError> {
        let value_data = self.func_data.dfg().value(value);
        match value_data.kind() {
            ValueKind::Return(ret) => {
                let value = ret.value().ok_or(GenerateAsmError::Unknown)?;
                self.load_to(value, "a0")?;
                let frame_size = self.frame.size();
                emit_instruction!(self, "addi sp, sp, {frame_size}");
                emit_instruction!(self, "ret");
            }
            ValueKind::Binary(binary) => self.gen_binary(value, binary)?,
            ValueKind::Alloc(_) => {}
            ValueKind::Store(store) => {
                self.load_to(store.value(), "t0")?;
                let destination = self.frame.slot(store.dest())?;
                emit_instruction!(self, "sw t0, {destination}");
            }
            ValueKind::Load(load) => {
                let source = self.frame.slot(load.src())?;
                emit_instruction!(self, "lw t0, {source}");
                self.store_from(value, "t0")?;
            }
            ValueKind::Branch(branch) => self.gen_branch(branch)?,
            ValueKind::Jump(jump) => self.gen_jump(jump)?,
            kind => unimplemented!("{:?}", kind),
        }

        Ok(())
    }
}
