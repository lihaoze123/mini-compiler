use std::collections::HashMap;

use koopa::ir::{FunctionData, Value, ValueKind};

use super::error::GenerateAsmError;

#[derive(Clone, Copy, derive_more::Display)]
#[display("{offset}(sp)")]
pub(super) struct StackSlot {
    offset: usize,
}

#[derive(Default)]
pub(super) struct StackFrame {
    size: usize,
    slots: HashMap<Value, StackSlot>,
    arg_scratch_slots: Vec<StackSlot>,
}

impl StackFrame {
    pub(super) fn build(func_data: &FunctionData) -> Self {
        let mut frame = Self::default();

        let mut max_bb_params = 0;
        for (bb, node) in func_data.layout().bbs() {
            let params = func_data.dfg().bb(*bb).params();
            max_bb_params = max_bb_params.max(params.len());
            for &param in params {
                frame.alloc_slot(param);
            }

            for &inst in node.insts().keys() {
                match func_data.dfg().value(inst).kind() {
                    ValueKind::Alloc(_) | ValueKind::Binary(_) | ValueKind::Load(_) => {
                        frame.alloc_slot(inst);
                    }
                    _ => {}
                }
            }
        }

        for _ in 0..max_bb_params {
            frame.alloc_arg_scratch_slot();
        }

        frame.size = Self::align_to_16(frame.size);
        frame
    }

    pub(super) fn size(&self) -> usize {
        self.size
    }

    pub(super) fn slot(&self, value: Value) -> Result<StackSlot, GenerateAsmError> {
        self.slots
            .get(&value)
            .copied()
            .ok_or(GenerateAsmError::MissingStackSlot)
    }

    pub(super) fn arg_scratch_slot(&self, index: usize) -> Result<StackSlot, GenerateAsmError> {
        self.arg_scratch_slots
            .get(index)
            .copied()
            .ok_or(GenerateAsmError::MissingStackSlot)
    }

    fn alloc_slot(&mut self, value: Value) {
        if self.slots.contains_key(&value) {
            return;
        }

        let slot = StackSlot { offset: self.size };
        self.size += 4;
        self.slots.insert(value, slot);
    }

    fn alloc_arg_scratch_slot(&mut self) {
        let slot = StackSlot { offset: self.size };
        self.size += 4;
        self.arg_scratch_slots.push(slot);
    }

    fn align_to_16(size: usize) -> usize {
        size.div_ceil(16) * 16
    }
}
