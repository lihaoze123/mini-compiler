use koopa::ir::{Value, ValueKind};

use super::{FunctionGenerator, GenerateAsmError};

impl FunctionGenerator<'_, '_> {
    pub(super) fn gen_value(&mut self, value: Value) -> Result<(), GenerateAsmError> {
        let value_data = self.func_data.dfg().value(value);
        match value_data.kind() {
            ValueKind::Return(ret) => {
                if let Some(value) = ret.value() {
                    self.load_to(value, "a0")?;
                }
                if let Some(ra_slot) = self.frame.ra_slot() {
                    emit_instruction!(self, "lw ra, {ra_slot}");
                }
                let frame_size = self.frame.size();
                emit_instruction!(self, "addi sp, sp, {frame_size}");
                emit_instruction!(self, "ret");
            }
            ValueKind::Binary(binary) => self.gen_binary(value, binary)?,
            ValueKind::Alloc(_) => {}
            ValueKind::Store(store) => {
                self.load_to(store.value(), "t0")?;
                if let Some(name) = self.global_name(store.dest())? {
                    emit_instruction!(self, "la t1, {name}");
                    emit_instruction!(self, "sw t0, 0(t1)");
                } else {
                    let destination = self.frame.slot(store.dest())?;
                    emit_instruction!(self, "sw t0, {destination}");
                }
            }
            ValueKind::Load(load) => {
                if let Some(name) = self.global_name(load.src())? {
                    emit_instruction!(self, "la t0, {name}");
                    emit_instruction!(self, "lw t0, 0(t0)");
                } else {
                    let source = self.frame.slot(load.src())?;
                    emit_instruction!(self, "lw t0, {source}");
                }
                self.store_from(value, "t0")?;
            }
            ValueKind::Branch(branch) => self.gen_branch(branch)?,
            ValueKind::Jump(jump) => self.gen_jump(jump)?,
            ValueKind::Call(call) => self.gen_call(value, call)?,
            kind => unimplemented!("{:?}", kind),
        }

        Ok(())
    }
}
