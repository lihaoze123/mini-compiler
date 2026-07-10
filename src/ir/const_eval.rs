use crate::ast::{
    AddExp, AddOp, EqExp, EqOp, Exp, Ident, LAndExp, LOrExp, MulExp, MulOp, PrimaryExp, RelExp,
    RelOp, UnaryExp, UnaryOp,
};

use super::{IRBuilder, error::IRBuilderErr, symbol::Symbol};

impl IRBuilder {
    pub(super) fn eval_exp(&self, exp: &Exp) -> Result<i32, IRBuilderErr> {
        self.eval_l_or_exp(&exp.l_or_exp)
    }

    fn eval_l_or_exp(&self, l_or_exp: &LOrExp) -> Result<i32, IRBuilderErr> {
        match l_or_exp {
            LOrExp::LAndExp(l_and_exp) => self.eval_l_and_exp(l_and_exp),
            LOrExp::LOrOp(l_or_exp, l_and_exp) => {
                let lhs = self.eval_l_or_exp(l_or_exp)?;
                if lhs != 0 {
                    Ok(1)
                } else {
                    let rhs = self.eval_l_and_exp(l_and_exp)?;
                    Ok((rhs != 0).into())
                }
            }
        }
    }

    fn eval_l_and_exp(&self, l_and_exp: &LAndExp) -> Result<i32, IRBuilderErr> {
        match l_and_exp {
            LAndExp::EqExp(eq_exp) => self.eval_eq_exp(eq_exp),
            LAndExp::LAndOp(l_and_op, eq_exp) => {
                let lhs = self.eval_l_and_exp(l_and_op)?;
                if lhs == 0 {
                    Ok(0)
                } else {
                    let rhs = self.eval_eq_exp(eq_exp)?;
                    Ok((rhs != 0).into())
                }
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
            UnaryExp::FuncCall(id, _) => Err(IRBuilderErr::NonConstSymbol(id.to_string())),
        }
    }

    fn eval_primary_exp(&self, primary_exp: &PrimaryExp) -> Result<i32, IRBuilderErr> {
        match primary_exp {
            PrimaryExp::Exp(exp) => self.eval_exp(exp),
            PrimaryExp::Number(num) => Ok(num.value),
            PrimaryExp::LVal(l_val) => self.eval_l_val(&l_val.id),
        }
    }

    fn eval_l_val(&self, id: &Ident) -> Result<i32, IRBuilderErr> {
        match self.context.get_symbol(id)? {
            Symbol::Const(value) => Ok(value.value()),
            Symbol::Var(_) => Err(IRBuilderErr::NonConstSymbol(id.to_string())),
            Symbol::Func(_) => Err(IRBuilderErr::InvalidLVal(id.to_string())),
        }
    }
}
