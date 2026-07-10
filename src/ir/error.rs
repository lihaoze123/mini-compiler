use thiserror::Error;

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
