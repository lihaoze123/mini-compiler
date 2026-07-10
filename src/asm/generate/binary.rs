use koopa::ir::{BinaryOp, Value, values::Binary};

use super::{FunctionGenerator, GenerateAsmError};

impl FunctionGenerator<'_, '_> {
    pub(super) fn gen_binary(
        &mut self,
        value: Value,
        binary: &Binary,
    ) -> Result<(), GenerateAsmError> {
        self.load_to(binary.lhs(), "t0")?;
        self.load_to(binary.rhs(), "t1")?;

        match binary.op() {
            BinaryOp::Eq => {
                emit_instruction!(self, "xor t2, t0, t1");
                emit_instruction!(self, "seqz t2, t2");
            }
            BinaryOp::NotEq => {
                emit_instruction!(self, "xor t2, t0, t1");
                emit_instruction!(self, "snez t2, t2");
            }
            BinaryOp::Add => emit_instruction!(self, "add t2, t0, t1"),
            BinaryOp::Sub => emit_instruction!(self, "sub t2, t0, t1"),
            BinaryOp::Mul => emit_instruction!(self, "mul t2, t0, t1"),
            BinaryOp::Div => emit_instruction!(self, "div t2, t0, t1"),
            BinaryOp::Mod => emit_instruction!(self, "rem t2, t0, t1"),
            BinaryOp::Lt => emit_instruction!(self, "slt t2, t0, t1"),
            BinaryOp::Gt => emit_instruction!(self, "slt t2, t1, t0"),
            BinaryOp::Le => {
                emit_instruction!(self, "slt t2, t1, t0");
                emit_instruction!(self, "seqz t2, t2");
            }
            BinaryOp::Ge => {
                emit_instruction!(self, "slt t2, t0, t1");
                emit_instruction!(self, "seqz t2, t2");
            }
            BinaryOp::And => emit_instruction!(self, "and t2, t0, t1"),
            BinaryOp::Or => emit_instruction!(self, "or t2, t0, t1"),
            BinaryOp::Xor => emit_instruction!(self, "xor t2, t0, t1"),
            BinaryOp::Shl => emit_instruction!(self, "sll t2, t0, t1"),
            BinaryOp::Shr => emit_instruction!(self, "srl t2, t0, t1"),
            BinaryOp::Sar => emit_instruction!(self, "sra t2, t0, t1"),
        }

        self.store_from(value, "t2")
    }
}
