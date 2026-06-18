use std::fmt::Write;

pub trait ToKoopa {
    fn to_koopa(&self) -> String;
}

#[derive(Debug)]
pub struct CompUnit {
    pub func_def: FuncDef,
}

impl ToKoopa for CompUnit {
    fn to_koopa(&self) -> String {
        self.func_def.to_koopa()
    }
}

#[derive(Debug)]
pub struct FuncDef {
    pub func_type: FuncType,
    pub id: Ident,
    pub block: Block,
}

impl ToKoopa for FuncDef {
    fn to_koopa(&self) -> String {
        let mut res: String = String::new();
        let _ = writeln!(res, "fun @{}(): {} {{", self.id, self.func_type.to_koopa());
        let _ = writeln!(res, "{}", self.block.to_koopa());
        let _ = writeln!(res, "}}");
        res
    }
}

#[derive(Debug)]
pub enum FuncType {
    Int,
}

impl ToKoopa for FuncType {
    fn to_koopa(&self) -> String {
        match *self {
            FuncType::Int => String::from("i32"),
        }
    }
}

#[derive(Debug)]
pub struct Block {
    pub stmt: Stmt,
}

impl ToKoopa for Block {
    fn to_koopa(&self) -> String {
        format!("%entry:\n{}", self.stmt.to_koopa())
    }
}

#[derive(Debug)]
pub struct Stmt {
    pub num: i32,
}

impl ToKoopa for Stmt {
    fn to_koopa(&self) -> String {
        format!("ret {}", self.num)
    }
}

pub type Ident = String;
