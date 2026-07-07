use std::fmt::Write;
use thiserror::Error;

use crate::ast::{
    AddExp, Block, CompUnit, EqExp, Exp, FuncDef, LAndExp, LOrExp, MulExp, PrimaryExp, RelExp,
    Stmt, UnaryExp, UnaryOp,
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

    fn emit_binary(
        &mut self,
        op: impl std::fmt::Display,
        lhs: impl std::fmt::Display,
        rhs: impl std::fmt::Display,
    ) -> Result<String, IRBuilderErr> {
        let temp = self.new_temp();
        writeln!(self.output, "{temp} = {op} {lhs}, {rhs}")?;
        Ok(temp)
    }

    fn boolify(&mut self, value: String) -> Result<String, IRBuilderErr> {
        self.emit_binary("ne", "0", value)
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
        // self.gen_stmt(&block.stmt)?;

        Ok(())
    }

    fn gen_stmt(&mut self, stmt: &Stmt) -> Result<(), IRBuilderErr> {
        let value = self.gen_exp(&stmt.exp)?;
        writeln!(self.output, "ret {}", value)?;

        Ok(())
    }

    fn gen_exp(&mut self, exp: &Exp) -> Result<String, IRBuilderErr> {
        self.gen_l_or_exp(&exp.l_or_exp)
    }

    fn gen_l_or_exp(&mut self, l_or_exp: &LOrExp) -> Result<String, IRBuilderErr> {
        match l_or_exp {
            LOrExp::LAndExp(l_and_exp) => self.gen_l_and_exp(l_and_exp),
            LOrExp::LOrOp(l_or_exp, l_and_exp) => {
                let lhs = self.gen_l_or_exp(l_or_exp)?;
                let rhs = self.gen_l_and_exp(l_and_exp)?;
                let lhs = self.boolify(lhs)?;
                let rhs = self.boolify(rhs)?;
                self.emit_binary("or", lhs, rhs)
            }
        }
    }

    fn gen_l_and_exp(&mut self, l_and_exp: &LAndExp) -> Result<String, IRBuilderErr> {
        match l_and_exp {
            LAndExp::EqExp(eq_exp) => self.gen_eq_exp(eq_exp),
            LAndExp::LAndOp(l_and_exp, eq_exp) => {
                let lhs = self.gen_l_and_exp(l_and_exp)?;
                let rhs = self.gen_eq_exp(eq_exp)?;
                let lhs = self.boolify(lhs)?;
                let rhs = self.boolify(rhs)?;
                self.emit_binary("and", lhs, rhs)
            }
        }
    }

    fn gen_eq_exp(&mut self, eq_exp: &EqExp) -> Result<String, IRBuilderErr> {
        match eq_exp {
            EqExp::RelExp(rel_exp) => self.gen_rel_exp(rel_exp),
            EqExp::EqOp(eq_exp, eq_op, rel_exp) => {
                let lhs = self.gen_eq_exp(eq_exp)?;
                let rhs = self.gen_rel_exp(rel_exp)?;
                self.emit_binary(eq_op, lhs, rhs)
            }
        }
    }

    fn gen_rel_exp(&mut self, rel_exp: &RelExp) -> Result<String, IRBuilderErr> {
        match rel_exp {
            RelExp::AddExp(add_exp) => self.gen_add_exp(add_exp),
            RelExp::RelOp(rel_exp, rel_op, add_exp) => {
                let lhs = self.gen_rel_exp(rel_exp)?;
                let rhs = self.gen_add_exp(add_exp)?;
                self.emit_binary(rel_op, lhs, rhs)
            }
        }
    }

    fn gen_add_exp(&mut self, add_exp: &AddExp) -> Result<String, IRBuilderErr> {
        match add_exp {
            AddExp::MulExp(mul_exp) => self.gen_mul_exp(mul_exp),
            AddExp::AddOp(add_exp, add_op, mul_exp) => {
                let lhs = self.gen_add_exp(add_exp)?;
                let rhs = self.gen_mul_exp(mul_exp)?;
                self.emit_binary(add_op, lhs, rhs)
            }
        }
    }

    fn gen_mul_exp(&mut self, mul_exp: &MulExp) -> Result<String, IRBuilderErr> {
        match mul_exp {
            MulExp::UnaryExp(unary_exp) => self.gen_unary_exp(unary_exp),
            MulExp::MulOp(mul_exp, mul_op, unary_exp) => {
                let lhs = self.gen_mul_exp(mul_exp)?;
                let rhs = self.gen_unary_exp(unary_exp)?;
                self.emit_binary(mul_op, lhs, rhs)
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
