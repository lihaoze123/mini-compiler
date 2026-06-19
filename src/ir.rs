use std::fmt::Write;
use thiserror::Error;

use crate::ast::{Block, CompUnit, Exp, FuncDef, PrimaryExp, Stmt, UnaryExp, UnaryOp};

#[derive(Error, Debug)]
pub enum IRBuilderErr {
    #[error("写字符串错误")]
    Write(#[from] std::fmt::Error),
}

pub struct IRBuilder {
    output: String,
    temp_id: usize,
}

impl Default for IRBuilder {
    fn default() -> Self {
        Self {
            output: String::new(),
            temp_id: 0,
        }
    }
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
        self.gen_unary_exp(&exp.unary_exp)
    }

    fn gen_unary_exp(&mut self, unary_exp: &UnaryExp) -> Result<String, IRBuilderErr> {
        match unary_exp {
            UnaryExp::PrimaryExp(primary_exp) => self.gen_primary_exp(primary_exp),
            UnaryExp::UnaryOp(unary_op, unary_exp) => {
                let value = self.gen_unary_exp(unary_exp)?;
                let temp_id = self.new_temp();
                match unary_op {
                    UnaryOp::Minus => {
                        writeln!(self.output, "{} = sub 0, {}", temp_id, value)?;
                    }
                    UnaryOp::Plus => {
                        return Ok(value);
                    }
                    UnaryOp::Not => {
                        writeln!(self.output, "{} = eq 0, {}", temp_id, value)?;
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
