use koopa::ir::{BinaryOp, FunctionData, Program, Value, ValueKind};
use std::{collections::HashMap, fmt::Write};
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
    temp_id: usize,
    temps: HashMap<Value, String>,
}

impl AsmBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_temp(&mut self, value: Value) -> String {
        self.temps
            .entry(value)
            .or_insert_with(|| {
                let res = if self.temp_id <= 6 {
                    format!("t{}", self.temp_id)
                } else {
                    // TODO 临时做法
                    format!("a{}", self.temp_id - 6)
                };
                self.temp_id += 1;
                res
            })
            .clone()
    }

    fn load_value(
        &mut self,
        value: Value,
        reg: &str,
        func_data: &FunctionData,
    ) -> Result<String, GenerateAsmError> {
        match func_data.dfg().value(value).kind() {
            ValueKind::Integer(int) => {
                if int.value() == 0 {
                    Ok("x0".to_owned())
                } else {
                    writeln!(self.output, "\tli {}, {}", reg, int.value())?;
                    Ok(reg.to_string())
                }
            }
            _ => Ok(self.get_temp(value)),
        }
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
                self.gen_value(inst, func_data)?;
            }
        }

        Ok(())
    }

    fn gen_value(
        &mut self,
        value: Value,
        func_data: &FunctionData,
    ) -> Result<(), GenerateAsmError> {
        match func_data.dfg().value(value).kind() {
            ValueKind::Return(ret) => {
                let value = ret.value().ok_or(GenerateAsmError::Unknown)?;
                let reg = self.load_value(value, "a0", func_data)?;
                if reg != "a0" {
                    writeln!(self.output, "\tmv a0, {}", reg)?;
                }
                writeln!(self.output, "\tret")?;
            }
            ValueKind::Binary(bin) => {
                let op = bin.op();
                let dst = self.get_temp(value);
                let lhs = self.load_value(bin.lhs(), "t5", func_data)?;
                let rhs = self.load_value(bin.rhs(), "t6", func_data)?;

                match op {
                    BinaryOp::Eq => {
                        writeln!(self.output, "\txor {dst}, {lhs}, {rhs}")?;
                        writeln!(self.output, "\tseqz {dst}, {dst}")?;
                    }
                    BinaryOp::NotEq => {
                        writeln!(self.output, "\txor {dst}, {lhs}, {rhs}")?;
                        writeln!(self.output, "\tsnez {dst}, {dst}")?;
                    }
                    BinaryOp::Add => {
                        writeln!(self.output, "\tadd {dst}, {lhs}, {rhs}")?;
                    }
                    BinaryOp::Sub => {
                        writeln!(self.output, "\tsub {dst}, {lhs}, {rhs}")?;
                    }
                    BinaryOp::Mul => {
                        writeln!(self.output, "\tmul {dst}, {lhs}, {rhs}")?;
                    }
                    BinaryOp::Div => {
                        writeln!(self.output, "\tdiv {dst}, {lhs}, {rhs}")?;
                    }
                    BinaryOp::Mod => {
                        writeln!(self.output, "\trem {dst}, {lhs}, {rhs}")?;
                    }
                    BinaryOp::Lt => {
                        writeln!(self.output, "\tslt {dst}, {lhs}, {rhs}")?;
                    }
                    BinaryOp::Gt => {
                        writeln!(self.output, "\tslt {dst}, {rhs}, {lhs}")?;
                    }
                    BinaryOp::Le => {
                        writeln!(self.output, "\tslt {dst}, {rhs}, {lhs}")?;
                        writeln!(self.output, "\tseqz {dst}, {dst}")?;
                    }
                    BinaryOp::Ge => {
                        writeln!(self.output, "\tslt {dst}, {lhs}, {rhs}")?;
                        writeln!(self.output, "\tseqz {dst}, {dst}")?;
                    }
                    BinaryOp::And => {
                        writeln!(self.output, "\tand {dst}, {lhs}, {rhs}")?;
                    }
                    BinaryOp::Or => {
                        writeln!(self.output, "\tor {dst}, {lhs}, {rhs}")?;
                    }
                    _ => unimplemented!("{:?}", op),
                }
            }
            kind => {
                unimplemented!("{:?}", kind);
            }
        }

        Ok(())
    }
}
