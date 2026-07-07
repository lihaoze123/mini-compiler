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

macro_rules! asm {
    ($builder:expr, $($arg:tt)*) => {
        $builder.emit(format_args!($($arg)*))?
    };
}

impl AsmBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    fn emit(&mut self, args: std::fmt::Arguments<'_>) -> Result<(), GenerateAsmError> {
        writeln!(self.output, "\t{}", args)?;
        Ok(())
    }

    fn get_temp(&mut self, value: Value) -> String {
        self.temps
            .entry(value)
            .or_insert_with(|| {
                let res = if self.temp_id <= 4 {
                    format!("t{}", self.temp_id)
                } else {
                    // TODO 临时做法
                    format!("a{}", self.temp_id - 4)
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
                    asm!(self, "li {}, {}", reg, int.value());
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

        asm!(self, ".text");
        asm!(self, ".globl {}", name);
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
                    asm!(self, "mv a0, {}", reg);
                }
                asm!(self, "ret");
            }
            ValueKind::Binary(bin) => {
                let op = bin.op();
                let dst = self.get_temp(value);
                let lhs = self.load_value(bin.lhs(), "t5", func_data)?;
                let rhs = self.load_value(bin.rhs(), "t6", func_data)?;

                match op {
                    BinaryOp::Eq => {
                        asm!(self, "xor {dst}, {lhs}, {rhs}");
                        asm!(self, "seqz {dst}, {dst}");
                    }
                    BinaryOp::NotEq => {
                        asm!(self, "xor {dst}, {lhs}, {rhs}");
                        asm!(self, "snez {dst}, {dst}");
                    }
                    BinaryOp::Add => {
                        asm!(self, "add {dst}, {lhs}, {rhs}");
                    }
                    BinaryOp::Sub => {
                        asm!(self, "sub {dst}, {lhs}, {rhs}");
                    }
                    BinaryOp::Mul => {
                        asm!(self, "mul {dst}, {lhs}, {rhs}");
                    }
                    BinaryOp::Div => {
                        asm!(self, "div {dst}, {lhs}, {rhs}");
                    }
                    BinaryOp::Mod => {
                        asm!(self, "rem {dst}, {lhs}, {rhs}");
                    }
                    BinaryOp::Lt => {
                        asm!(self, "slt {dst}, {lhs}, {rhs}");
                    }
                    BinaryOp::Gt => {
                        asm!(self, "slt {dst}, {rhs}, {lhs}");
                    }
                    BinaryOp::Le => {
                        asm!(self, "slt {dst}, {rhs}, {lhs}");
                        asm!(self, "seqz {dst}, {dst}");
                    }
                    BinaryOp::Ge => {
                        asm!(self, "slt {dst}, {lhs}, {rhs}");
                        asm!(self, "seqz {dst}, {dst}");
                    }
                    BinaryOp::And => {
                        asm!(self, "and {dst}, {lhs}, {rhs}");
                    }
                    BinaryOp::Or => {
                        asm!(self, "or {dst}, {lhs}, {rhs}");
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
