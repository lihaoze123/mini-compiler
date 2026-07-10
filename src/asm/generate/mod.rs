mod binary;
mod control_flow;
mod value;

use koopa::ir::{BasicBlock, FunctionData, Program, Value, ValueKind};

use super::{AsmBuilder, context::AsmContext, error::GenerateAsmError, frame::StackFrame};

pub(super) struct FunctionGenerator<'ctx, 'func> {
    context: &'ctx mut AsmContext,
    func_data: &'func FunctionData,
    frame: StackFrame,
}

impl AsmBuilder {
    pub fn gen_program(&mut self, program: &Program) -> Result<String, GenerateAsmError> {
        self.context.reset_generation();

        for &func in program.func_layout() {
            FunctionGenerator::new(&mut self.context, program.func(func)).generate()?;
        }

        Ok(self.context.take_output())
    }
}

impl<'ctx, 'func> FunctionGenerator<'ctx, 'func> {
    fn new(context: &'ctx mut AsmContext, func_data: &'func FunctionData) -> Self {
        Self {
            context,
            frame: StackFrame::build(func_data),
            func_data,
        }
    }

    fn generate(mut self) -> Result<(), GenerateAsmError> {
        let name = Self::strip_prefix(self.func_data.name())?;

        emit_instruction!(self, ".text");
        emit_instruction!(self, ".globl {name}");
        emit_line!(self, "{name}:");

        let frame_size = self.frame.size();
        emit_instruction!(self, "addi sp, sp, -{frame_size}");

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

    fn strip_prefix(name: &str) -> Result<String, GenerateAsmError> {
        let name = match &name[0..1] {
            prefix @ ("@" | "%") => name.strip_prefix(prefix).ok_or(GenerateAsmError::Parse)?,
            _ => return Err(GenerateAsmError::Parse),
        };
        Ok(name.to_owned())
    }

    fn basic_block_name(&self, bb: BasicBlock) -> Result<String, GenerateAsmError> {
        Self::strip_prefix(
            self.func_data
                .dfg()
                .bb(bb)
                .name()
                .as_ref()
                .ok_or(GenerateAsmError::BBNoName)?,
        )
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
}
