use koopa::ir::{FunctionData, Program, ValueKind, entities::ValueData};
use std::fmt::Write;
use thiserror::Error;

pub fn str_to_program(s: &str) -> Result<Program, GenerateAsmError> {
    koopa::front::Driver::from(s)
        .generate_program()
        .map_err(GenerateAsmError::KoopaParse)
}

pub fn generate_asm(program: &Program) -> Result<String, GenerateAsmError> {
    let mut builder = AsmBuilder::new();
    builder.gen_program(program)
}

#[derive(Error, Debug)]
pub enum GenerateAsmError {
    #[error("生成汇编时写字符串错误")]
    Write(#[from] std::fmt::Error),

    #[error("解析字符串时错误")]
    Parse,

    #[error("解析 Koopa IR 时错误: {0:?}")]
    KoopaParse(koopa::front::span::Error),

    #[error("未知错误")]
    Unknown,
}

#[derive(Default)]
pub struct AsmBuilder {
    output: String,
}

impl AsmBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gen_program(&mut self, program: &Program) -> Result<String, GenerateAsmError> {
        self.output.clear();

        for &func in program.func_layout() {
            self.gen_func(program.func(func))?;
        }

        Ok(std::mem::take(&mut self.output))
    }

    fn gen_func(&mut self, func_data: &FunctionData) -> Result<(), GenerateAsmError> {
        let name = func_data
            .name()
            .strip_prefix("@")
            .ok_or(GenerateAsmError::Parse)?;

        writeln!(self.output, "\t.text")?;
        writeln!(self.output, "\t.globl {}", name)?;
        writeln!(self.output, "{}:", name)?;

        for (_bb, node) in func_data.layout().bbs() {
            for &inst in node.insts().keys() {
                self.gen_value(func_data.dfg().value(inst), func_data)?;
            }
        }

        Ok(())
    }

    fn gen_value(
        &mut self,
        value_data: &ValueData,
        func_data: &FunctionData,
    ) -> Result<(), GenerateAsmError> {
        match value_data.kind() {
            ValueKind::Return(ret) => {
                let value = ret.value().ok_or(GenerateAsmError::Unknown)?;
                let value_data = func_data.dfg().value(value);
                match value_data.kind() {
                    ValueKind::Integer(int) => {
                        writeln!(self.output, "\tli a0, {}", int.value())?;
                    }
                    _ => unimplemented!(),
                }
                writeln!(self.output, "\tret")?;
            }
            kind => {
                unimplemented!("{:?}", kind);
            }
        }

        Ok(())
    }
}
