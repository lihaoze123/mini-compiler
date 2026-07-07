use std::{collections::HashMap, fmt::Write};
use thiserror::Error;

use crate::ast::{
    AddExp, AddOp, Block, BlockItem, CompUnit, ConstDecl, Decl, EqExp, EqOp, Exp, FuncDef, LAndExp,
    LOrExp, MulExp, MulOp, PrimaryExp, RelExp, RelOp, Stmt, UnaryExp, UnaryOp,
};

#[derive(Error, Debug)]
pub enum IRBuilderErr {
    #[error("写字符串错误")]
    Write(#[from] std::fmt::Error),
    #[error("未定义符号 {0}")]
    UndefinedSymbol(String),
}

#[derive(Default)]
pub struct IRBuilder {
    output: String,
    temp_id: usize,
    consts: HashMap<String, i32>,
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
        for item in &block.items {
            self.gen_block_item(item)?;
        }

        Ok(())
    }

    fn gen_block_item(&mut self, block: &BlockItem) -> Result<(), IRBuilderErr> {
        match block {
            BlockItem::Decl(decl) => self.gen_decl(decl),
            BlockItem::Stmt(stmt) => self.gen_stmt(stmt),
        }
    }

    fn gen_decl(&mut self, decl: &Decl) -> Result<(), IRBuilderErr> {
        match decl {
            Decl::ConstDecl(const_decl) => self.gen_const_decl(const_decl),
        }
    }

    fn gen_const_decl(&mut self, const_decl: &ConstDecl) -> Result<(), IRBuilderErr> {
        for def in &const_decl.const_defs {
            let value = self.eval_exp(&def.const_init_val.const_exp.exp)?;
            self.consts.insert(def.id.clone(), value);
        }
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
            PrimaryExp::LVal(l_val) => {
                let value = self
                    .consts
                    .get(&l_val.id)
                    .ok_or_else(|| IRBuilderErr::UndefinedSymbol(l_val.id.clone()))?;
                Ok(value.to_string())
            }
        }
    }

    fn eval_exp(&self, exp: &Exp) -> Result<i32, IRBuilderErr> {
        self.eval_l_or_exp(&exp.l_or_exp)
    }

    fn eval_l_or_exp(&self, l_or_exp: &LOrExp) -> Result<i32, IRBuilderErr> {
        match l_or_exp {
            LOrExp::LAndExp(l_and_exp) => self.eval_l_and_exp(l_and_exp),
            LOrExp::LOrOp(l_or_exp, l_and_exp) => {
                let lhs = self.eval_l_or_exp(l_or_exp)?;
                let rhs = self.eval_l_and_exp(l_and_exp)?;
                Ok((lhs != 0 || rhs != 0).into())
            }
        }
    }

    fn eval_l_and_exp(&self, l_and_exp: &LAndExp) -> Result<i32, IRBuilderErr> {
        match l_and_exp {
            LAndExp::EqExp(eq_exp) => self.eval_eq_exp(eq_exp),
            LAndExp::LAndOp(l_and_op, eq_exp) => {
                let lhs = self.eval_l_and_exp(l_and_op)?;
                let rhs = self.eval_eq_exp(eq_exp)?;
                Ok((lhs != 0 && rhs != 0).into())
            }
        }
    }

    fn eval_eq_exp(&self, eq_exp: &EqExp) -> Result<i32, IRBuilderErr> {
        match eq_exp {
            EqExp::RelExp(rel_exp) => self.eval_rel_exp(rel_exp),
            EqExp::EqOp(eq_exp, eq_op, rel_exp) => {
                let lhs = self.eval_eq_exp(eq_exp)?;
                let rhs = self.eval_rel_exp(rel_exp)?;
                match eq_op {
                    EqOp::Eq => Ok((lhs == rhs).into()),
                    EqOp::Ne => Ok((lhs != rhs).into()),
                }
            }
        }
    }

    fn eval_rel_exp(&self, rel_exp: &RelExp) -> Result<i32, IRBuilderErr> {
        match rel_exp {
            RelExp::AddExp(add_exp) => self.eval_add_exp(add_exp),
            RelExp::RelOp(rel_exp, rel_op, add_exp) => {
                let lhs = self.eval_rel_exp(rel_exp)?;
                let rhs = self.eval_add_exp(add_exp)?;
                match rel_op {
                    RelOp::Lt => Ok((lhs < rhs).into()),
                    RelOp::Gt => Ok((lhs > rhs).into()),
                    RelOp::Le => Ok((lhs <= rhs).into()),
                    RelOp::Ge => Ok((lhs >= rhs).into()),
                }
            }
        }
    }

    fn eval_add_exp(&self, add_exp: &AddExp) -> Result<i32, IRBuilderErr> {
        match add_exp {
            AddExp::MulExp(mul_exp) => self.eval_mul_exp(mul_exp),
            AddExp::AddOp(add_exp, add_op, mul_exp) => {
                let lhs = self.eval_add_exp(add_exp)?;
                let rhs = self.eval_mul_exp(mul_exp)?;
                match add_op {
                    AddOp::Add => Ok(lhs + rhs),
                    AddOp::Sub => Ok(lhs - rhs),
                }
            }
        }
    }

    fn eval_mul_exp(&self, mul_exp: &MulExp) -> Result<i32, IRBuilderErr> {
        match mul_exp {
            MulExp::UnaryExp(unary_exp) => self.eval_unary_exp(unary_exp),
            MulExp::MulOp(mul_exp, mul_op, unary_exp) => {
                let lhs = self.eval_mul_exp(mul_exp)?;
                let rhs = self.eval_unary_exp(unary_exp)?;
                match mul_op {
                    MulOp::Mul => Ok(lhs * rhs),
                    MulOp::Div => Ok(lhs / rhs),
                    MulOp::Mod => Ok(lhs % rhs),
                }
            }
        }
    }

    fn eval_unary_exp(&self, unary_exp: &UnaryExp) -> Result<i32, IRBuilderErr> {
        match unary_exp {
            UnaryExp::PrimaryExp(primary_exp) => self.eval_primary_exp(primary_exp),
            UnaryExp::UnaryOp(unary_op, unary_exp) => {
                let value = self.eval_unary_exp(unary_exp)?;
                match unary_op {
                    UnaryOp::Plus => Ok(value),
                    UnaryOp::Minus => Ok(-value),
                    UnaryOp::Not => Ok((value == 0).into()),
                }
            }
        }
    }

    fn eval_primary_exp(&self, primary_exp: &PrimaryExp) -> Result<i32, IRBuilderErr> {
        match primary_exp {
            PrimaryExp::Exp(exp) => self.eval_exp(exp),
            PrimaryExp::Number(num) => Ok(num.value),
            PrimaryExp::LVal(l_val) => self
                .consts
                .get(&l_val.id)
                .copied()
                .ok_or_else(|| IRBuilderErr::UndefinedSymbol(l_val.id.clone())),
        }
    }
}
