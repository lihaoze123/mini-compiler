use crate::ast::{
    AddExp, EqExp, Exp, Ident, LAndExp, LOrExp, MulExp, PrimaryExp, RelExp, UnaryExp, UnaryOp,
};

use super::super::{
    IRBuilder,
    context::{Label, Value},
    error::IRBuilderErr,
    symbol::Symbol,
};

impl IRBuilder {
    pub(super) fn gen_exp(&mut self, exp: &Exp) -> Result<Value, IRBuilderErr> {
        self.gen_l_or_exp(&exp.l_or_exp)
    }

    fn gen_l_or_exp(&mut self, l_or_exp: &LOrExp) -> Result<Value, IRBuilderErr> {
        match l_or_exp {
            LOrExp::LAndExp(l_and_exp) => self.gen_l_and_exp(l_and_exp),
            LOrExp::LOrOp(l_or_exp, l_and_exp) => {
                let lhs = self.gen_l_or_exp(l_or_exp)?;

                let rhs_label = self.context.new_label("or_rhs");
                let true_label = self.context.new_label("or_true");
                let false_label = self.context.new_label("or_false");
                let end_label = self.context.new_label("or_end");

                emit_instruction!(self, "br {lhs}, {true_label}, {rhs_label}");

                emit_line!(self, "{rhs_label}:");
                let rhs = self.gen_l_and_exp(l_and_exp)?;
                emit_instruction!(self, "br {rhs}, {true_label}, {false_label}");

                self.gen_bool_merge(&true_label, &false_label, &end_label)
            }
        }
    }

    fn gen_l_and_exp(&mut self, l_and_exp: &LAndExp) -> Result<Value, IRBuilderErr> {
        match l_and_exp {
            LAndExp::EqExp(eq_exp) => self.gen_eq_exp(eq_exp),
            LAndExp::LAndOp(l_and_exp, eq_exp) => {
                let lhs = self.gen_l_and_exp(l_and_exp)?;

                let rhs_label = self.context.new_label("and_rhs");
                let true_label = self.context.new_label("and_true");
                let false_label = self.context.new_label("and_false");
                let end_label = self.context.new_label("and_end");

                emit_instruction!(self, "br {lhs}, {rhs_label}, {false_label}");

                emit_line!(self, "{rhs_label}:");
                let rhs = self.gen_eq_exp(eq_exp)?;
                emit_instruction!(self, "br {rhs}, {true_label}, {false_label}");

                self.gen_bool_merge(&true_label, &false_label, &end_label)
            }
        }
    }

    fn gen_bool_merge(
        &mut self,
        true_label: &Label,
        false_label: &Label,
        end_label: &Label,
    ) -> Result<Value, IRBuilderErr> {
        emit_line!(self, "{true_label}:");
        emit_instruction!(self, "jump {end_label}(1)");

        emit_line!(self, "{false_label}:");
        emit_instruction!(self, "jump {end_label}(0)");

        let result = self.context.new_temp();
        emit_line!(self, "{end_label}({result}: i32):");
        Ok(result.into())
    }

    fn gen_eq_exp(&mut self, eq_exp: &EqExp) -> Result<Value, IRBuilderErr> {
        match eq_exp {
            EqExp::RelExp(rel_exp) => self.gen_rel_exp(rel_exp),
            EqExp::EqOp(eq_exp, eq_op, rel_exp) => {
                let lhs = self.gen_eq_exp(eq_exp)?;
                let rhs = self.gen_rel_exp(rel_exp)?;
                self.context.emit_binary(eq_op, lhs, rhs)
            }
        }
    }

    fn gen_rel_exp(&mut self, rel_exp: &RelExp) -> Result<Value, IRBuilderErr> {
        match rel_exp {
            RelExp::AddExp(add_exp) => self.gen_add_exp(add_exp),
            RelExp::RelOp(rel_exp, rel_op, add_exp) => {
                let lhs = self.gen_rel_exp(rel_exp)?;
                let rhs = self.gen_add_exp(add_exp)?;
                self.context.emit_binary(rel_op, lhs, rhs)
            }
        }
    }

    fn gen_add_exp(&mut self, add_exp: &AddExp) -> Result<Value, IRBuilderErr> {
        match add_exp {
            AddExp::MulExp(mul_exp) => self.gen_mul_exp(mul_exp),
            AddExp::AddOp(add_exp, add_op, mul_exp) => {
                let lhs = self.gen_add_exp(add_exp)?;
                let rhs = self.gen_mul_exp(mul_exp)?;
                self.context.emit_binary(add_op, lhs, rhs)
            }
        }
    }

    fn gen_mul_exp(&mut self, mul_exp: &MulExp) -> Result<Value, IRBuilderErr> {
        match mul_exp {
            MulExp::UnaryExp(unary_exp) => self.gen_unary_exp(unary_exp),
            MulExp::MulOp(mul_exp, mul_op, unary_exp) => {
                let lhs = self.gen_mul_exp(mul_exp)?;
                let rhs = self.gen_unary_exp(unary_exp)?;
                self.context.emit_binary(mul_op, lhs, rhs)
            }
        }
    }

    fn gen_unary_exp(&mut self, unary_exp: &UnaryExp) -> Result<Value, IRBuilderErr> {
        match unary_exp {
            UnaryExp::PrimaryExp(primary_exp) => self.gen_primary_exp(primary_exp),
            UnaryExp::UnaryOp(unary_op, unary_exp) => {
                let value = self.gen_unary_exp(unary_exp)?;
                let temp = self.context.new_temp();
                match unary_op {
                    UnaryOp::Minus => {
                        emit_instruction!(self, "{temp} = sub 0, {value}");
                    }
                    UnaryOp::Plus => {
                        return Ok(value);
                    }
                    UnaryOp::Not => {
                        emit_instruction!(self, "{temp} = eq 0, {value}");
                    }
                }
                Ok(temp.into())
            }
        }
    }

    fn gen_primary_exp(&mut self, primary_exp: &PrimaryExp) -> Result<Value, IRBuilderErr> {
        match primary_exp {
            PrimaryExp::Exp(exp) => self.gen_exp(exp),
            PrimaryExp::Number(num) => Ok(num.value.into()),
            PrimaryExp::LVal(l_val) => self.gen_l_val(&l_val.id),
        }
    }

    fn gen_l_val(&mut self, id: &Ident) -> Result<Value, IRBuilderErr> {
        match self.context.get_symbol(id)? {
            Symbol::Const(value) => Ok(value.into()),
            Symbol::Var(variable) => {
                let temp = self.context.new_temp();
                emit_instruction!(self, "{temp} = load {variable}");
                Ok(temp.into())
            }
        }
    }
}
