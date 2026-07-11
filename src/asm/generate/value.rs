use koopa::ir::{TypeKind, Value, ValueKind};

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
                    self.emit_stack_load("ra", ra_slot)?;
                }
                let frame_size = self.frame.size();
                self.emit_sp_adjust(frame_size as i64)?;
                emit_instruction!(self, "ret");
            }
            ValueKind::Binary(binary) => self.gen_binary(value, binary)?,
            ValueKind::Alloc(_) => {}
            ValueKind::Store(store) => {
                self.load_to(store.value(), "t0")?;
                self.load_address_to(store.dest(), "t1")?;
                emit_instruction!(self, "sw t0, 0(t1)");
            }
            ValueKind::Load(load) => {
                self.load_address_to(load.src(), "t0")?;
                emit_instruction!(self, "lw t0, 0(t0)");
                self.store_from(value, "t0")?;
            }
            ValueKind::GetPtr(get_ptr) => {
                self.gen_pointer_offset(value, get_ptr.src(), get_ptr.index())?;
            }
            ValueKind::GetElemPtr(get_elem_ptr) => {
                self.gen_pointer_offset(value, get_elem_ptr.src(), get_elem_ptr.index())?;
            }
            ValueKind::Branch(branch) => self.gen_branch(branch)?,
            ValueKind::Jump(jump) => self.gen_jump(jump)?,
            ValueKind::Call(call) => self.gen_call(value, call)?,
            kind => return Err(GenerateAsmError::UnsupportedValue(format!("{kind:?}"))),
        }

        Ok(())
    }

    fn gen_pointer_offset(
        &mut self,
        result: Value,
        source: Value,
        index: Value,
    ) -> Result<(), GenerateAsmError> {
        self.load_address_to(source, "t0")?;
        self.load_to(index, "t1")?;

        let result_type = self.func_data.dfg().value(result).ty();
        let TypeKind::Pointer(base) = result_type.kind() else {
            return Err(GenerateAsmError::ExpectedPointer);
        };
        let stride = base.size();
        if stride != 1 {
            emit_instruction!(self, "li t2, {stride}");
            emit_instruction!(self, "mul t1, t1, t2");
        }
        emit_instruction!(self, "add t0, t0, t1");
        self.store_from(result, "t0")
    }
}
