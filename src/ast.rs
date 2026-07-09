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

#[derive(Debug, derive_more::Display)]
pub enum FuncType {
    #[display("i32")]
    Int,
}

#[derive(Debug)]
pub struct Block {
    pub items: Vec<BlockItem>,
}

#[derive(Debug)]
pub enum BlockItem {
    Decl(Decl),
    Stmt(Stmt),
}

#[derive(Debug)]
pub enum Stmt {
    Assign(LVal, Exp),
    Exp(Option<Exp>),
    Block(Block),
    Return(Exp),
    If(Exp, Box<Stmt>, Option<Box<Stmt>>),
}

#[derive(Debug)]
pub struct Exp {
    pub l_or_exp: LOrExp,
}

#[derive(Debug)]
pub enum UnaryExp {
    PrimaryExp(PrimaryExp),
    UnaryOp(UnaryOp, Box<UnaryExp>),
}

#[derive(Debug)]
pub enum Decl {
    ConstDecl(ConstDecl),
    VarDecl(VarDecl),
}

#[derive(Debug)]
pub struct ConstDecl {
    pub b_type: BType,
    pub const_defs: Vec<ConstDef>,
}

#[derive(Debug, derive_more::Display)]
pub enum BType {
    #[display("i32")]
    Int,
}

#[derive(Debug)]
pub struct ConstDef {
    pub id: Ident,
    pub const_init_val: ConstInitVal,
}

#[derive(Debug)]
pub struct ConstInitVal {
    pub const_exp: ConstExp,
}

#[derive(Debug)]
pub struct VarDecl {
    pub b_type: BType,
    pub var_defs: Vec<VarDef>,
}

#[derive(Debug)]
pub enum VarDef {
    ID(Ident),
    InitVal(Ident, InitVal),
}

#[derive(Debug)]
pub struct InitVal {
    pub exp: Exp,
}

#[derive(Debug)]
pub struct ConstExp {
    pub exp: Exp,
}

#[derive(Debug, Clone)]
pub struct LVal {
    pub id: Ident,
}

#[derive(Debug)]
pub enum PrimaryExp {
    Exp(Box<Exp>),
    LVal(LVal),
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

#[derive(Debug, Clone, Copy)]
pub struct Number {
    pub value: IntConst,
}

#[derive(Debug, derive_more::Display)]
pub enum MulOp {
    #[display("mul")]
    Mul,
    #[display("div")]
    Div,
    #[display("mod")]
    Mod,
}

#[derive(Debug, derive_more::Display)]
pub enum AddOp {
    #[display("add")]
    Add,
    #[display("sub")]
    Sub,
}

#[derive(Debug)]
pub enum RelExp {
    AddExp(AddExp),
    RelOp(Box<RelExp>, RelOp, AddExp),
}

#[derive(Debug, derive_more::Display)]
pub enum RelOp {
    #[display("lt")]
    Lt,
    #[display("gt")]
    Gt,
    #[display("le")]
    Le,
    #[display("ge")]
    Ge,
}

#[derive(Debug)]
pub enum EqExp {
    RelExp(RelExp),
    EqOp(Box<EqExp>, EqOp, RelExp),
}

#[derive(Debug, derive_more::Display)]
pub enum EqOp {
    #[display("eq")]
    Eq,
    #[display("ne")]
    Ne,
}

#[derive(Debug)]
pub enum LAndExp {
    EqExp(EqExp),
    LAndOp(Box<LAndExp>, EqExp),
}

#[derive(Debug)]
pub enum LOrExp {
    LAndExp(LAndExp),
    LOrOp(Box<LOrExp>, LAndExp),
}

#[derive(Debug)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Ident {
    pub id: String,
}

impl From<Ident> for String {
    fn from(value: Ident) -> Self {
        value.id
    }
}

pub type IntConst = i32;
