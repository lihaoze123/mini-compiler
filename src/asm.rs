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
    temps: HashMap<Value, usize>,
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

    fn align_to_16(size: usize) -> usize {
        (size + 15) / 16 * 16
    }

    fn get_temp(&mut self, value: Value) -> String {
        let res = self.temps.entry(value).or_insert_with(|| {
            let res = self.temp_id;
            self.temp_id += 4;
            res
        });
        format!("{res}(sp)")
    }

    fn load_to(
        &mut self,
        value: Value,
        reg: &str,
        func_data: &FunctionData,
    ) -> Result<(), GenerateAsmError> {
        match func_data.dfg().value(value).kind() {
            ValueKind::Integer(int) => {
                asm!(self, "li {reg}, {}", int.value());
            }
            _ => {
                let slot = self.get_temp(value);
                asm!(self, "lw {reg}, {slot}");
            }
        }
        Ok(())
    }

    fn store_from(&mut self, value: Value, reg: &str) -> Result<(), GenerateAsmError> {
        let slot = self.get_temp(value);
        asm!(self, "sw {reg}, {slot}");
        Ok(())
    }

    fn prepare_stack_slots(&mut self, func_data: &FunctionData) -> usize {
        self.temp_id = 0;
        self.temps.clear();

        for (_bb, node) in func_data.layout().bbs() {
            for &inst in node.insts().keys() {
                match func_data.dfg().value(inst).kind() {
                    ValueKind::Alloc(_) | ValueKind::Binary(_) | ValueKind::Load(_) => {
                        self.get_temp(inst);
                    }
                    _ => {}
                }
            }
        }

        Self::align_to_16(self.temp_id)
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

        let s = self.prepare_stack_slots(func_data);
        asm!(self, "addi sp, sp, -{s}");

        for (_bb, node) in func_data.layout().bbs() {
            for &inst in node.insts().keys() {
                self.gen_value(inst, func_data, s)?;
            }
        }

        Ok(())
    }

    fn gen_value(
        &mut self,
        value: Value,
        func_data: &FunctionData,
        s: usize,
    ) -> Result<(), GenerateAsmError> {
        let value_data = func_data.dfg().value(value);
        match value_data.kind() {
            ValueKind::Return(ret) => {
                let value = ret.value().ok_or(GenerateAsmError::Unknown)?;
                self.load_to(value, "a0", func_data)?;
                asm!(self, "addi sp, sp, {s}");
                asm!(self, "ret");
            }
            ValueKind::Binary(bin) => {
                let op = bin.op();
                self.load_to(bin.lhs(), "t0", func_data)?;
                self.load_to(bin.rhs(), "t1", func_data)?;

                match op {
                    BinaryOp::Eq => {
                        asm!(self, "xor t2, t0, t1");
                        asm!(self, "seqz t2, t2");
                    }
                    BinaryOp::NotEq => {
                        asm!(self, "xor t2, t0, t1");
                        asm!(self, "snez t2, t2");
                    }
                    BinaryOp::Add => {
                        asm!(self, "add t2, t0, t1");
                    }
                    BinaryOp::Sub => {
                        asm!(self, "sub t2, t0, t1");
                    }
                    BinaryOp::Mul => {
                        asm!(self, "mul t2, t0, t1");
                    }
                    BinaryOp::Div => {
                        asm!(self, "div t2, t0, t1");
                    }
                    BinaryOp::Mod => {
                        asm!(self, "rem t2, t0, t1");
                    }
                    BinaryOp::Lt => {
                        asm!(self, "slt t2, t0, t1");
                    }
                    BinaryOp::Gt => {
                        asm!(self, "slt t2, t1, t0");
                    }
                    BinaryOp::Le => {
                        asm!(self, "slt t2, t1, t0");
                        asm!(self, "seqz t2, t2");
                    }
                    BinaryOp::Ge => {
                        asm!(self, "slt t2, t0, t1");
                        asm!(self, "seqz t2, t2");
                    }
                    BinaryOp::And => {
                        asm!(self, "and t2, t0, t1");
                    }
                    BinaryOp::Or => {
                        asm!(self, "or t2, t0, t1");
                    }
                    BinaryOp::Xor => {
                        asm!(self, "xor t2, t0, t1");
                    }
                    BinaryOp::Shl => {
                        asm!(self, "sll t2, t0, t1");
                    }
                    BinaryOp::Shr => {
                        asm!(self, "srl t2, t0, t1");
                    }
                    BinaryOp::Sar => {
                        asm!(self, "sra t2, t0, t1");
                    }
                }
                self.store_from(value, "t2")?;
            }
            ValueKind::Alloc(_) => {
                self.get_temp(value);
            }
            ValueKind::Store(store) => {
                self.load_to(store.value(), "t0", func_data)?;
                let dest = self.get_temp(store.dest());
                asm!(self, "sw t0, {dest}");
            }
            ValueKind::Load(load) => {
                let src = self.get_temp(load.src());
                asm!(self, "lw t0, {src}");
                self.store_from(value, "t0")?;
            }
            kind => {
                unimplemented!("{:?}", kind);
            }
        }

        Ok(())
    }
}
