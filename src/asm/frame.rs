use std::collections::HashMap;

use koopa::ir::{FunctionData, TypeKind, Value, ValueKind};

use super::error::GenerateAsmError;

#[derive(Clone, Copy, derive_more::Display)]
#[display("{offset}(sp)")]
pub(super) struct StackSlot {
    offset: usize,
}

impl StackSlot {
    pub(super) fn offset(self) -> usize {
        self.offset
    }
}

#[derive(Default)]
pub(super) struct StackFrame {
    size: usize,
    slots: HashMap<Value, StackSlot>,
    ra_slot: Option<StackSlot>,
    outgoing_args_slots: Vec<StackSlot>,
    arg_scratch_slots: Vec<StackSlot>,
}

impl StackFrame {
    pub(super) fn build(func_data: &FunctionData) -> Self {
        let mut frame = Self::default();

        let mut has_call = false;
        let mut max_call_args = 0;

        for (_, node) in func_data.layout().bbs() {
            for &inst in node.insts().keys() {
                let inst_data = func_data.dfg().value(inst);
                if let ValueKind::Call(call) = inst_data.kind() {
                    has_call = true;
                    max_call_args = max_call_args.max(call.args().len());
                }
            }
        }

        for _ in 8..max_call_args {
            frame.alloc_outgoing_args_slot();
        }

        for &param in func_data.params() {
            frame.alloc_slot(param, func_data.dfg().value(param).ty().size());
        }

        let mut max_bb_params = 0;
        for (bb, node) in func_data.layout().bbs() {
            let params = func_data.dfg().bb(*bb).params();
            max_bb_params = max_bb_params.max(params.len());
            for &param in params {
                frame.alloc_slot(param, func_data.dfg().value(param).ty().size());
            }

            for &inst in node.insts().keys() {
                let inst_data = func_data.dfg().value(inst);
                match inst_data.kind() {
                    ValueKind::Alloc(_) => {
                        let TypeKind::Pointer(base) = inst_data.ty().kind() else {
                            unreachable!("alloc must produce a pointer")
                        };
                        frame.alloc_slot(inst, base.size());
                    }
                    ValueKind::Binary(_)
                    | ValueKind::Load(_)
                    | ValueKind::GetPtr(_)
                    | ValueKind::GetElemPtr(_) => {
                        frame.alloc_slot(inst, inst_data.ty().size());
                    }
                    ValueKind::Call(_) if !inst_data.ty().is_unit() => {
                        frame.alloc_slot(inst, inst_data.ty().size());
                    }
                    _ => {}
                }
            }
        }

        for _ in 0..max_bb_params {
            frame.alloc_arg_scratch_slot();
        }

        if has_call {
            frame.size += 4;
        }
        frame.size = Self::align_to_16(frame.size);
        if has_call {
            frame.alloc_ra_slot();
        }

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

    pub(super) fn ra_slot(&self) -> Option<StackSlot> {
        self.ra_slot
    }

    pub(super) fn outgoing_args_slot(&self, index: usize) -> Result<StackSlot, GenerateAsmError> {
        self.outgoing_args_slots
            .get(index)
            .copied()
            .ok_or(GenerateAsmError::MissingStackSlot)
    }

    pub(super) fn incoming_arg_slot(&self, index: usize) -> StackSlot {
        StackSlot {
            offset: self.size + index * 4,
        }
    }

    pub(super) fn arg_scratch_slot(&self, index: usize) -> Result<StackSlot, GenerateAsmError> {
        self.arg_scratch_slots
            .get(index)
            .copied()
            .ok_or(GenerateAsmError::MissingStackSlot)
    }

    fn alloc_slot(&mut self, value: Value, size: usize) {
        if self.slots.contains_key(&value) {
            return;
        }

        self.size = self.size.div_ceil(4) * 4;
        let slot = StackSlot { offset: self.size };
        self.size += size;
        self.slots.insert(value, slot);
    }

    fn alloc_ra_slot(&mut self) {
        let slot = StackSlot {
            offset: self.size - 4,
        };
        self.ra_slot = Some(slot);
    }

    fn alloc_outgoing_args_slot(&mut self) {
        let slot = StackSlot { offset: self.size };
        self.size += 4;
        self.outgoing_args_slots.push(slot);
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
