use koopa::ir::{FunctionData, Program, ValueKind, entities::ValueData};
use std::fmt::Write;
use thiserror::Error;

pub fn str_to_program(s: &str) -> Result<Program, GenerateAsmError> {
    koopa::front::Driver::from(s)
        .generate_program()
        .map_err(GenerateAsmError::KoopaParse)
}

pub fn generate_asm(program: &Program) -> Result<String, GenerateAsmError> {
    program.generate()
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

trait GenerateAsm {
    fn generate(&self) -> Result<String, GenerateAsmError>;
}

trait GenerateAsmInFunc {
    fn generate(&self, func_data: &FunctionData) -> Result<String, GenerateAsmError>;
}

impl GenerateAsm for Program {
    fn generate(&self) -> Result<String, GenerateAsmError> {
        let mut res = String::new();

        for &func in self.func_layout() {
            let func_data = self.func(func);
            write!(res, "{}", func_data.generate()?)?;
        }

        Ok(res)
    }
}

impl GenerateAsm for FunctionData {
    fn generate(&self) -> Result<String, GenerateAsmError> {
        let mut res = String::new();

        let name = self
            .name()
            .strip_prefix("@")
            .ok_or(GenerateAsmError::Parse)?;

        writeln!(res, "\t.text")?;
        writeln!(res, "\t.globl {}", name)?;
        writeln!(res, "{}:", name)?;

        for (_bb, node) in self.layout().bbs() {
            for &inst in node.insts().keys() {
                let value_data = self.dfg().value(inst);
                write!(res, "{}", value_data.generate(self)?)?;
            }
        }

        Ok(res)
    }
}

impl GenerateAsmInFunc for ValueData {
    fn generate(&self, func_data: &FunctionData) -> Result<String, GenerateAsmError> {
        let mut res = String::new();

        match self.kind() {
            ValueKind::Return(ret) => {
                let value = ret.value().ok_or(GenerateAsmError::Unknown)?;
                let value_data = func_data.dfg().value(value);
                match value_data.kind() {
                    ValueKind::Integer(int) => {
                        writeln!(res, "\tli a0, {}", int.value())?;
                    }
                    _ => unimplemented!(),
                }
                writeln!(res, "\tret")?;
            }
            _ => unimplemented!(),
        }

        Ok(res)
    }
}
