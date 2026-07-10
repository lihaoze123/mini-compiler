use koopa::ir::{BasicBlock, BinaryOp, FunctionData, Program, Value, ValueKind};
use std::{
    collections::HashMap,
    fmt::{self, Write},
};
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

    #[error("缺少栈槽")]
    MissingStackSlot,

    #[error("基本块无名称")]
    BBNoName,

    #[error("未知错误")]
    Unknown,
}

#[derive(Default)]
pub struct AsmBuilder {
    output: String,
    edge_label_id: usize,
}

struct FuncCtx<'a> {
    func_data: &'a FunctionData,
    frame: StackFrame,
}

#[derive(Clone, Copy, derive_more::Display)]
#[display("{offset}(sp)")]
struct StackSlot {
    offset: usize,
}

#[derive(Default)]
struct StackFrame {
    size: usize,
    slots: HashMap<Value, StackSlot>,
    arg_scratch_slots: Vec<StackSlot>,
}

impl StackFrame {
    fn build(func_data: &FunctionData) -> Self {
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

    fn slot(&self, value: Value) -> Result<StackSlot, GenerateAsmError> {
        self.slots
            .get(&value)
            .copied()
            .ok_or(GenerateAsmError::MissingStackSlot)
    }

    fn arg_scratch_slot(&self, index: usize) -> Result<StackSlot, GenerateAsmError> {
        self.arg_scratch_slots
            .get(index)
            .copied()
            .ok_or(GenerateAsmError::MissingStackSlot)
    }

    fn align_to_16(size: usize) -> usize {
        size.div_ceil(16) * 16
    }
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

    fn emit(&mut self, args: fmt::Arguments<'_>) -> Result<(), GenerateAsmError> {
        writeln!(self.output, "\t{}", args)?;
        Ok(())
    }

    fn new_edge_label(&mut self) -> String {
        let label = format!(".L_edge_{}", self.edge_label_id);
        self.edge_label_id += 1;
        label
    }

    fn strip_prefix(s: &str) -> Result<String, GenerateAsmError> {
        let res = match &s[0..1] {
            x @ ("@" | "%") => s.strip_prefix(x).ok_or(GenerateAsmError::Parse)?,
            _ => {
                return Err(GenerateAsmError::Parse);
            }
        };
        Ok(res.to_string())
    }

    fn get_bb_name(&self, bb: BasicBlock, ctx: &FuncCtx<'_>) -> Result<String, GenerateAsmError> {
        Self::strip_prefix(
            ctx.func_data
                .dfg()
                .bb(bb)
                .name()
                .as_ref()
                .ok_or(GenerateAsmError::BBNoName)?,
        )
    }

    fn load_to(
        &mut self,
        value: Value,
        reg: &str,
        ctx: &FuncCtx<'_>,
    ) -> Result<(), GenerateAsmError> {
        match ctx.func_data.dfg().value(value).kind() {
            ValueKind::Integer(int) => {
                asm!(self, "li {reg}, {}", int.value());
            }
            _ => {
                let slot = ctx.frame.slot(value)?;
                asm!(self, "lw {reg}, {slot}");
            }
        }
        Ok(())
    }

    fn store_from(
        &mut self,
        value: Value,
        reg: &str,
        ctx: &FuncCtx<'_>,
    ) -> Result<(), GenerateAsmError> {
        let slot = ctx.frame.slot(value)?;
        asm!(self, "sw {reg}, {slot}");
        Ok(())
    }

    fn pass_bb_args(
        &mut self,
        target: BasicBlock,
        args: &[Value],
        ctx: &FuncCtx<'_>,
    ) -> Result<(), GenerateAsmError> {
        let params = ctx.func_data.dfg().bb(target).params();
        if params.len() != args.len() {
            return Err(GenerateAsmError::Unknown);
        }

        for (index, &arg) in args.iter().enumerate() {
            let scratch = ctx.frame.arg_scratch_slot(index)?;
            self.load_to(arg, "t1", ctx)?;
            asm!(self, "sw t1, {scratch}");
        }

        for (index, &param) in params.iter().enumerate() {
            let scratch = ctx.frame.arg_scratch_slot(index)?;
            asm!(self, "lw t1, {scratch}");
            self.store_from(param, "t1", ctx)?;
        }

        Ok(())
    }

    pub fn gen_program(&mut self, program: &Program) -> Result<String, GenerateAsmError> {
        self.output.clear();
        self.edge_label_id = 0;

        for &func in program.func_layout() {
            self.gen_func(program.func(func))?;
        }

        Ok(std::mem::take(&mut self.output))
    }

    fn gen_func(&mut self, func_data: &FunctionData) -> Result<(), GenerateAsmError> {
        let name = Self::strip_prefix(func_data.name())?;

        asm!(self, ".text");
        asm!(self, ".globl {}", name);
        writeln!(self.output, "{}:", name)?;

        let ctx = FuncCtx {
            func_data,
            frame: StackFrame::build(func_data),
        };
        let s = ctx.frame.size;
        asm!(self, "addi sp, sp, -{s}");

        // for (_bb, node) in func_data.layout().bbs() {
        //     for &inst in node.insts().keys() {
        //         let inst_data = func_data.dfg().value(inst);
        //         eprintln!("{inst:?}: {inst_data:?}");
        //     }
        // }

        for (_bb, node) in func_data.layout().bbs() {
            let name = self.get_bb_name(*_bb, &ctx)?;
            writeln!(self.output, "{}:", name)?;

            for &inst in node.insts().keys() {
                self.gen_value(inst, &ctx)?;
            }
        }

        Ok(())
    }

    fn gen_value(&mut self, value: Value, ctx: &FuncCtx<'_>) -> Result<(), GenerateAsmError> {
        let value_data = ctx.func_data.dfg().value(value);
        match value_data.kind() {
            ValueKind::Return(ret) => {
                let value = ret.value().ok_or(GenerateAsmError::Unknown)?;
                self.load_to(value, "a0", ctx)?;
                let s = ctx.frame.size;
                asm!(self, "addi sp, sp, {s}");
                asm!(self, "ret");
            }
            ValueKind::Binary(bin) => {
                let op = bin.op();
                self.load_to(bin.lhs(), "t0", ctx)?;
                self.load_to(bin.rhs(), "t1", ctx)?;

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
                self.store_from(value, "t2", ctx)?;
            }
            ValueKind::Alloc(_) => {}
            ValueKind::Store(store) => {
                self.load_to(store.value(), "t0", ctx)?;
                let dest = ctx.frame.slot(store.dest())?;
                asm!(self, "sw t0, {dest}");
            }
            ValueKind::Load(load) => {
                let src = ctx.frame.slot(load.src())?;
                asm!(self, "lw t0, {src}");
                self.store_from(value, "t0", ctx)?;
            }
            ValueKind::Branch(branch) => {
                self.load_to(branch.cond(), "t0", ctx)?;

                let true_bb = self.get_bb_name(branch.true_bb(), ctx)?;
                let false_bb = self.get_bb_name(branch.false_bb(), ctx)?;

                if branch.true_args().is_empty() && branch.false_args().is_empty() {
                    asm!(self, "bnez t0, {true_bb}");
                    asm!(self, "j {false_bb}");
                } else {
                    let false_edge = self.new_edge_label();
                    asm!(self, "beqz t0, {false_edge}");

                    self.pass_bb_args(branch.true_bb(), branch.true_args(), ctx)?;
                    asm!(self, "j {true_bb}");

                    writeln!(self.output, "{false_edge}:")?;
                    self.pass_bb_args(branch.false_bb(), branch.false_args(), ctx)?;
                    asm!(self, "j {false_bb}");
                }
            }
            ValueKind::Jump(jump) => {
                let target_bb = self.get_bb_name(jump.target(), ctx)?;
                self.pass_bb_args(jump.target(), jump.args(), ctx)?;
                asm!(self, "j {target_bb}");
            }
            kind => {
                unimplemented!("{:?}", kind);
            }
        }

        Ok(())
    }
}
