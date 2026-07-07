use std::fmt::{self, Display};

use crate::ast::FuncType::Int;

#[derive(Debug)]
pub struct CompUnit {
    pub func_def: FuncDef,
}

#[derive(Debug)]
pub struct FuncDef {
    pub func_type: FuncType,
    pub id: Ident,
    pub block: Block,
}

#[derive(Debug)]
pub enum FuncType {
    Int,
}

impl Display for FuncType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Int => "i32",
            }
        )
    }
}

#[derive(Debug)]
pub struct Block {
    pub stmt: Stmt,
}

#[derive(Debug)]
pub struct Stmt {
    pub exp: Exp,
}

#[derive(Debug)]
pub struct Exp {
    pub add_exp: AddExp,
}

#[derive(Debug)]
pub enum UnaryExp {
    PrimaryExp(PrimaryExp),
    UnaryOp(UnaryOp, Box<UnaryExp>),
}

#[derive(Debug)]
pub enum PrimaryExp {
    Exp(Box<Exp>),
    Number(Number),
}

#[derive(Debug)]
pub enum MulExp {
    UnaryExp(UnaryExp),
    MulOp(Box<MulExp>, MulOp, UnaryExp),
}

#[derive(Debug)]
pub enum AddExp {
    MulExp(MulExp),
    AddOp(Box<AddExp>, AddOp, MulExp),
}

#[derive(Debug)]
pub struct Number {
    pub value: IntConst,
}

#[derive(Debug)]
pub enum MulOp {
    Mul,
    Div,
    Mod,
}

impl fmt::Display for MulOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            MulOp::Mul => "mul",
            MulOp::Div => "div",
            MulOp::Mod => "mod",
        })
    }
}

#[derive(Debug)]
pub enum AddOp {
    Add,
    Sub,
}

impl fmt::Display for AddOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            AddOp::Add => "add",
            AddOp::Sub => "sub",
        })
    }
}

#[derive(Debug)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
}

pub type IntConst = i32;

pub type Ident = String;
