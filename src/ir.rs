use std::fmt::Write;
use thiserror::Error;

use crate::ast::{
    AddExp, Block, CompUnit, Exp, FuncDef, MulExp, PrimaryExp, Stmt, UnaryExp, UnaryOp,
};

#[derive(Error, Debug)]
pub enum IRBuilderErr {
    #[error("写字符串错误")]
    Write(#[from] std::fmt::Error),
}

#[derive(Default)]
pub struct IRBuilder {
    output: String,
    temp_id: usize,
}

impl IRBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    fn new_temp(&mut self) -> String {
        let res = format!("%{}", self.temp_id);
        self.temp_id += 1;
        res
    }

    pub fn gen_comp_unit(&mut self, comp_unit: &CompUnit) -> Result<String, IRBuilderErr> {
        self.output.clear();
        self.gen_func_def(&comp_unit.func_def)?;
        Ok(std::mem::take(&mut self.output))
    }

    fn gen_func_def(&mut self, func_def: &FuncDef) -> Result<(), IRBuilderErr> {
        writeln!(
            self.output,
            "fun @{}(): {} {{",
            func_def.id, func_def.func_type
        )?;
        self.gen_block(&func_def.block)?;
        writeln!(self.output, "}}")?;

        Ok(())
    }

    fn gen_block(&mut self, block: &Block) -> Result<(), IRBuilderErr> {
        writeln!(self.output, "%entry:")?;
        self.gen_stmt(&block.stmt)?;

        Ok(())
    }

    fn gen_stmt(&mut self, stmt: &Stmt) -> Result<(), IRBuilderErr> {
        let value = self.gen_exp(&stmt.exp)?;
        writeln!(self.output, "ret {}", value)?;

        Ok(())
    }

    fn gen_exp(&mut self, exp: &Exp) -> Result<String, IRBuilderErr> {
        self.gen_add_exp(&exp.add_exp)
    }

    fn gen_add_exp(&mut self, add_exp: &AddExp) -> Result<String, IRBuilderErr> {
        match add_exp {
            AddExp::MulExp(mul_exp) => self.gen_mul_exp(mul_exp),
            AddExp::AddOp(add_exp, add_op, mul_exp) => {
                let lhs = self.gen_add_exp(add_exp)?;
                let rhs = self.gen_mul_exp(mul_exp)?;
                let temp_id = self.new_temp();
                writeln!(self.output, "{temp_id} = {add_op} {lhs}, {rhs}")?;
                Ok(temp_id)
            },
        }
    }

    fn gen_mul_exp(&mut self, mul_exp: &MulExp) -> Result<String, IRBuilderErr> {
        match mul_exp {
            MulExp::UnaryExp(unary_exp) => self.gen_unary_exp(unary_exp),
            MulExp::MulOp(mul_exp, mul_op, unary_exp) => {
                let lhs = self.gen_mul_exp(mul_exp)?;
                let rhs = self.gen_unary_exp(unary_exp)?;
                let temp_id = self.new_temp();
                writeln!(self.output, "{temp_id} = {mul_op} {lhs}, {rhs}")?;
                Ok(temp_id)
            }
        }
    }

    fn gen_unary_exp(&mut self, unary_exp: &UnaryExp) -> Result<String, IRBuilderErr> {
        match unary_exp {
            UnaryExp::PrimaryExp(primary_exp) => self.gen_primary_exp(primary_exp),
            UnaryExp::UnaryOp(unary_op, unary_exp) => {
                let value = self.gen_unary_exp(unary_exp)?;
                let temp_id = self.new_temp();
                match unary_op {
                    UnaryOp::Minus => {
                        writeln!(self.output, "{temp_id} = sub 0, {value}")?;
                    }
                    UnaryOp::Plus => {
                        return Ok(value);
                    }
                    UnaryOp::Not => {
                        writeln!(self.output, "{temp_id} = eq 0, {value}")?;
                    }
                }
                Ok(temp_id)
            }
        }
    }

    fn gen_primary_exp(&mut self, primary_exp: &PrimaryExp) -> Result<String, IRBuilderErr> {
        match primary_exp {
            PrimaryExp::Exp(exp) => self.gen_exp(exp),
            PrimaryExp::Number(num) => Ok(num.value.to_string()),
        }
    }
}
