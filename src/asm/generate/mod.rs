mod binary;
mod control_flow;
mod value;

use koopa::ir::{BasicBlock, FunctionData, Program, Value, ValueKind};

use super::{AsmBuilder, context::AsmContext, error::GenerateAsmError, frame::StackFrame};

pub(super) struct FunctionGenerator<'ctx, 'func> {
    context: &'ctx mut AsmContext,
    program: &'func Program,
    func_data: &'func FunctionData,
    frame: StackFrame,
}

impl AsmBuilder {
    pub fn gen_program(&mut self, program: &Program) -> Result<String, GenerateAsmError> {
        self.context.reset_generation();

        for &value in program.inst_layout() {
            let value_data = program.borrow_value(value);
            let alloc = match value_data.kind() {
                ValueKind::GlobalAlloc(alloc) => alloc,
                _ => return Err(GenerateAsmError::UnsupportedGlobalInitializer),
            };
            let name = strip_prefix(
                value_data
                    .name()
                    .as_deref()
                    .ok_or(GenerateAsmError::GlobalValueNoName)?,
            )?;
            let initializer = program.borrow_value(alloc.init());

            emit_instruction!(self, ".data");
            emit_instruction!(self, ".globl {name}");
            emit_line!(self, "{name}:");
            match initializer.kind() {
                ValueKind::Integer(integer) => {
                    emit_instruction!(self, ".word {}", integer.value())
                }
                ValueKind::ZeroInit(_) => emit_instruction!(self, ".zero 4"),
                _ => return Err(GenerateAsmError::UnsupportedGlobalInitializer),
            }
        }

        for &func in program.func_layout() {
            let func_data = program.func(func);
            if func_data.layout().entry_bb().is_some() {
                FunctionGenerator::new(&mut self.context, program, func_data).generate()?;
            }
        }

        Ok(self.context.take_output())
    }
}

impl<'ctx, 'func> FunctionGenerator<'ctx, 'func> {
    fn new(
        context: &'ctx mut AsmContext,
        program: &'func Program,
        func_data: &'func FunctionData,
    ) -> Self {
        Self {
            context,
            program,
            frame: StackFrame::build(func_data),
            func_data,
        }
    }

    fn generate(mut self) -> Result<(), GenerateAsmError> {
        let name = strip_prefix(self.func_data.name())?;

        emit_instruction!(self, ".text");
        emit_instruction!(self, ".globl {name}");
        emit_line!(self, "{name}:");

        let frame_size = self.frame.size();
        emit_instruction!(self, "addi sp, sp, -{frame_size}");

        if let Some(ra_slot) = self.frame.ra_slot() {
            emit_instruction!(self, "sw ra, {ra_slot}");
        }

        let params = self.func_data.params().to_vec();
        for (index, param) in params.into_iter().enumerate() {
            let local_slot = self.frame.slot(param)?;
            if index < 8 {
                emit_instruction!(self, "sw a{index}, {local_slot}");
            } else {
                let incoming_slot = self.frame.incoming_arg_slot(index - 8);
                emit_instruction!(self, "lw t0, {incoming_slot}");
                emit_instruction!(self, "sw t0, {local_slot}");
            }
        }

        let func_data = self.func_data;
        for (bb, node) in func_data.layout().bbs() {
            let name = self.basic_block_name(*bb)?;
            emit_line!(self, "{name}:");

            for &inst in node.insts().keys() {
                self.gen_value(inst)?;
            }
        }

        Ok(())
    }

    fn basic_block_name(&self, bb: BasicBlock) -> Result<String, GenerateAsmError> {
        let function_name = strip_prefix(self.func_data.name())?;
        let block_name = strip_prefix(
            self.func_data
                .dfg()
                .bb(bb)
                .name()
                .as_ref()
                .ok_or(GenerateAsmError::BBNoName)?,
        )?;
        Ok(format!(".L_{function_name}_{block_name}"))
    }

    fn load_to(&mut self, value: Value, register: &str) -> Result<(), GenerateAsmError> {
        match self.func_data.dfg().value(value).kind() {
            ValueKind::Integer(integer) => {
                emit_instruction!(self, "li {register}, {}", integer.value());
            }
            _ => {
                let slot = self.frame.slot(value)?;
                emit_instruction!(self, "lw {register}, {slot}");
            }
        }
        Ok(())
    }

    fn store_from(&mut self, value: Value, register: &str) -> Result<(), GenerateAsmError> {
        let slot = self.frame.slot(value)?;
        emit_instruction!(self, "sw {register}, {slot}");
        Ok(())
    }

    fn global_name(&self, value: Value) -> Result<Option<String>, GenerateAsmError> {
        let values = self.program.borrow_values();
        let Some(value_data) = values.get(&value) else {
            return Ok(None);
        };
        let name = value_data
            .name()
            .as_deref()
            .ok_or(GenerateAsmError::GlobalValueNoName)?;
        strip_prefix(name).map(Some)
    }
}

pub(super) fn strip_prefix(name: &str) -> Result<String, GenerateAsmError> {
    let Some(prefix) = name.chars().next() else {
        return Err(GenerateAsmError::Parse);
    };
    if !matches!(prefix, '@' | '%') {
        return Err(GenerateAsmError::Parse);
    }
    Ok(name[1..].to_owned())
}
