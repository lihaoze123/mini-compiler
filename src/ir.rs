use core::fmt;
use std::{collections::HashMap, fmt::Write};
use thiserror::Error;

use crate::ast::{
    AddExp, AddOp, BType, Block, BlockItem, CompUnit, ConstDecl, ConstInitVal, Decl, EqExp, EqOp,
    Exp, FuncDef, Ident, InitVal, LAndExp, LOrExp, LVal, MulExp, MulOp, PrimaryExp, RelExp, RelOp,
    Stmt, UnaryExp, UnaryOp, VarDecl, VarDef,
};

#[derive(Error, Debug)]
pub enum IRBuilderErr {
    #[error("写字符串错误")]
    Write(#[from] std::fmt::Error),

    #[error("未定义符号 {0}")]
    UndefinedSymbol(String),

    #[error("非常量符号 {0}")]
    NonConstSymbol(String),

    #[error("赋值到常量符号 {0}")]
    AssignToConst(String),

    #[error("重复定义符号 {0}")]
    DuplicateSymbol(String),

    #[error("没有作用域")]
    NoScope,
}

#[derive(Clone)]
enum Symbol {
    Const(i32),
    Var(String),
}

#[derive(Default)]
pub struct IRBuilder {
    output: String,
    temp_id: usize,
    label_id: usize,
    var_id: usize,
    symbols: Vec<HashMap<Ident, Symbol>>,
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}", self.id)
    }
}

macro_rules! ir {
    ($builder:expr, $($arg:tt)*) => {
        $builder.emit::<true>(format_args!($($arg)*))?
    };
}

macro_rules! label {
    ($builder:expr, $($arg:tt)*) => {
        $builder.emit::<false>(format_args!($($arg)*))?
    };
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

    fn new_label(&mut self, label: &str) -> String {
        let res = format!("%L_{label}_{}", self.label_id);
        self.label_id += 1;
        res
    }

    fn emit<const IDENT: bool>(&mut self, args: fmt::Arguments<'_>) -> Result<(), IRBuilderErr> {
        if IDENT {
            writeln!(self.output, "\t{}", args)?;
        } else {
            writeln!(self.output, "{}", args)?;
        }
        Ok(())
    }

    fn emit_binary(
        &mut self,
        op: impl std::fmt::Display,
        lhs: impl std::fmt::Display,
        rhs: impl std::fmt::Display,
    ) -> Result<String, IRBuilderErr> {
        let temp = self.new_temp();
        ir!(self, "{temp} = {op} {lhs}, {rhs}");
        Ok(temp)
    }

    fn define_symbol(&mut self, id: &Ident, symbol: Symbol) -> Result<(), IRBuilderErr> {
        let scope = self.symbols.last_mut().ok_or(IRBuilderErr::NoScope)?;
        if scope.contains_key(id) {
            return Err(IRBuilderErr::DuplicateSymbol(id.to_string()));
        }

        scope.insert(id.clone(), symbol);
        Ok(())
    }

    fn get_symbol(&self, id: &Ident) -> Result<Symbol, IRBuilderErr> {
        self.symbols
            .iter()
            .rev()
            .find_map(|scope| scope.get(id).cloned())
            .ok_or_else(|| IRBuilderErr::UndefinedSymbol(id.to_string()))
    }

    pub fn gen_comp_unit(&mut self, comp_unit: &CompUnit) -> Result<String, IRBuilderErr> {
        self.output.clear();
        self.temp_id = 0;
        self.symbols.clear();
        self.gen_func_def(&comp_unit.func_def)?;
        Ok(std::mem::take(&mut self.output))
    }

    fn gen_func_def(&mut self, func_def: &FuncDef) -> Result<(), IRBuilderErr> {
        label!(self, "fun {}(): {} {{", func_def.id, func_def.func_type);
        label!(self, "%entry:");

        self.gen_block(&func_def.block)?;
        label!(self, "}}");

        Ok(())
    }

    fn gen_block(&mut self, block: &Block) -> Result<bool, IRBuilderErr> {
        self.symbols.push(HashMap::new());

        let mut terminated = false;
        for item in &block.items {
            if self.gen_block_item(item)? {
                terminated = true;
                break;
            }
        }

        self.symbols.pop();
        Ok(terminated)
    }

    fn gen_block_item(&mut self, block: &BlockItem) -> Result<bool, IRBuilderErr> {
        match block {
            BlockItem::Decl(decl) => {
                self.gen_decl(decl)?;
                Ok(false)
            }
            BlockItem::Stmt(stmt) => self.gen_stmt(stmt),
        }
    }

    fn gen_decl(&mut self, decl: &Decl) -> Result<(), IRBuilderErr> {
        match decl {
            Decl::ConstDecl(const_decl) => self.gen_const_decl(const_decl),
            Decl::VarDecl(var_decl) => self.gen_var_decl(var_decl),
        }
    }

    fn gen_var_decl(&mut self, var_decl: &VarDecl) -> Result<(), IRBuilderErr> {
        for def in &var_decl.var_defs {
            match def {
                VarDef::ID(id) => self.gen_var_def(&var_decl.b_type, id, None)?,
                VarDef::InitVal(id, init_val) => {
                    self.gen_var_def(&var_decl.b_type, id, Some(init_val))?
                }
            }
        }
        Ok(())
    }

    fn gen_var_def(
        &mut self,
        b_type: &BType,
        id: &Ident,
        init_val: Option<&InitVal>,
    ) -> Result<(), IRBuilderErr> {
        let value = match init_val {
            Some(init_val) => self.gen_exp(&init_val.exp)?,
            None => "0".to_owned(),
        };

        let name = format!("@{}_{}", id.id, self.var_id);
        self.var_id += 1;

        self.define_symbol(id, Symbol::Var(name.clone()))?;

        ir!(self, "{name} = alloc {b_type}");
        ir!(self, "store {value}, {name}");
        Ok(())
    }

    fn gen_const_decl(&mut self, const_decl: &ConstDecl) -> Result<(), IRBuilderErr> {
        for def in &const_decl.const_defs {
            self.gen_const_def(&def.id, &def.const_init_val)?;
        }
        Ok(())
    }

    fn gen_const_def(
        &mut self,
        id: &Ident,
        const_init_val: &ConstInitVal,
    ) -> Result<(), IRBuilderErr> {
        let value = self.eval_exp(&const_init_val.const_exp.exp)?;
        self.define_symbol(id, Symbol::Const(value))?;
        Ok(())
    }

    fn gen_stmt(&mut self, stmt: &Stmt) -> Result<bool, IRBuilderErr> {
        match stmt {
            Stmt::Return(ret) => {
                self.gen_return(ret)?;
                Ok(true)
            }
            Stmt::Assign(l_val, exp) => {
                self.gen_assign(l_val, exp)?;
                Ok(false)
            }
            Stmt::Exp(Some(exp)) => {
                self.gen_exp(exp)?;
                Ok(false)
            }
            Stmt::Exp(None) => Ok(false),
            Stmt::Block(block) => self.gen_block(block),
            Stmt::If(cond, then_stmt, else_stmt) => {
                self.gen_if(cond, then_stmt.as_ref(), else_stmt.as_deref())
            }
        }
    }

    fn gen_if(
        &mut self,
        cond: &Exp,
        then_stmt: &Stmt,
        else_stmt: Option<&Stmt>,
    ) -> Result<bool, IRBuilderErr> {
        let entry_label = self.new_label("if_entry");
        ir!(self, "jump {entry_label}");
        label!(self, "{entry_label}:");

        let value = self.gen_exp(cond)?;

        let true_label = self.new_label("then");
        let end_label = self.new_label("end");
        let false_label = match else_stmt {
            Some(_) => self.new_label("else"),
            None => end_label.clone(),
        };

        ir!(self, "br {value}, {true_label}, {false_label}");

        let then_terminated = self.gen_if_arm(&true_label, Some(then_stmt), &end_label)?;
        let else_terminated = self.gen_if_arm(&false_label, else_stmt, &end_label)?;

        let terminated = then_terminated && else_terminated;
        if !terminated {
            label!(self, "{end_label}:");
        }
        Ok(terminated)
    }

    fn gen_if_arm(
        &mut self,
        label: &str,
        stmt: Option<&Stmt>,
        end_label: &str,
    ) -> Result<bool, IRBuilderErr> {
        match stmt {
            Some(stmt) => {
                label!(self, "{label}:");

                let terminated = self.gen_stmt(stmt)?;
                if !terminated {
                    ir!(self, "jump {end_label}");
                }

                Ok(terminated)
            }
            None => Ok(false),
        }
    }

    fn gen_return(&mut self, ret: &Exp) -> Result<(), IRBuilderErr> {
        let value = self.gen_exp(ret)?;
        ir!(self, "ret {value}");
        Ok(())
    }

    fn gen_assign(&mut self, l_val: &LVal, exp: &Exp) -> Result<(), IRBuilderErr> {
        match self.get_symbol(&l_val.id)? {
            Symbol::Const(_) => Err(IRBuilderErr::AssignToConst(l_val.id.to_string())),
            Symbol::Var(name) => {
                let value = self.gen_exp(exp)?;
                ir!(self, "store {value}, {name}");
                Ok(())
            }
        }
    }

    fn gen_exp(&mut self, exp: &Exp) -> Result<String, IRBuilderErr> {
        self.gen_l_or_exp(&exp.l_or_exp)
    }

    fn gen_l_or_exp(&mut self, l_or_exp: &LOrExp) -> Result<String, IRBuilderErr> {
        match l_or_exp {
            LOrExp::LAndExp(l_and_exp) => self.gen_l_and_exp(l_and_exp),
            LOrExp::LOrOp(l_or_exp, l_and_exp) => {
                let lhs = self.gen_l_or_exp(l_or_exp)?;

                let rhs_label = self.new_label("or_rhs");
                let true_label = self.new_label("or_true");
                let false_label = self.new_label("or_false");
                let end_label = self.new_label("or_end");

                ir!(self, "br {lhs}, {true_label}, {rhs_label}");

                label!(self, "{rhs_label}:");
                let rhs = self.gen_l_and_exp(l_and_exp)?;
                ir!(self, "br {rhs}, {true_label}, {false_label}");

                self.gen_bool_merge(&true_label, &false_label, &end_label)
            }
        }
    }

    fn gen_l_and_exp(&mut self, l_and_exp: &LAndExp) -> Result<String, IRBuilderErr> {
        match l_and_exp {
            LAndExp::EqExp(eq_exp) => self.gen_eq_exp(eq_exp),
            LAndExp::LAndOp(l_and_exp, eq_exp) => {
                let lhs = self.gen_l_and_exp(l_and_exp)?;

                let rhs_label = self.new_label("and_rhs");
                let true_label = self.new_label("and_true");
                let false_label = self.new_label("and_false");
                let end_label = self.new_label("and_end");

                ir!(self, "br {lhs}, {rhs_label}, {false_label}");

                label!(self, "{rhs_label}:");
                let rhs = self.gen_eq_exp(eq_exp)?;
                ir!(self, "br {rhs}, {true_label}, {false_label}");

                self.gen_bool_merge(&true_label, &false_label, &end_label)
            }
        }
    }

    fn gen_bool_merge(
        &mut self,
        true_label: &str,
        false_label: &str,
        end_label: &str,
    ) -> Result<String, IRBuilderErr> {
        label!(self, "{true_label}:");
        ir!(self, "jump {end_label}(1)");

        label!(self, "{false_label}:");
        ir!(self, "jump {end_label}(0)");

        let result = self.new_temp();
        label!(self, "{end_label}({result}: i32):");
        Ok(result)
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
                        ir!(self, "{temp_id} = sub 0, {value}");
                    }
                    UnaryOp::Plus => {
                        return Ok(value);
                    }
                    UnaryOp::Not => {
                        ir!(self, "{temp_id} = eq 0, {value}");
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
            PrimaryExp::LVal(l_val) => self.gen_l_val(&l_val.id),
        }
    }

    fn gen_l_val(&mut self, id: &Ident) -> Result<String, IRBuilderErr> {
        match self.get_symbol(id)? {
            Symbol::Const(value) => Ok(value.to_string()),
            Symbol::Var(name) => {
                let temp = self.new_temp();
                ir!(self, "{temp} = load {name}");
                Ok(temp)
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
        match self.get_symbol(id)? {
            Symbol::Const(value) => Ok(value),
            Symbol::Var(_) => Err(IRBuilderErr::NonConstSymbol(id.to_string())),
        }
    }
}
