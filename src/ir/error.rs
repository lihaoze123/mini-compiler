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

    #[error("函数 {0} 的声明与已有签名不一致")]
    ConflictingFunctionDeclaration(String),

    #[error("没有作用域")]
    NoScope,

    #[error("循环栈为空，在循环外使用了 {0}")]
    EmptyLoopStack(String),

    #[error("不合法的左值 {0}")]
    InvalidLVal(String),

    #[error("表达式无值，应该是有值")]
    ExpectedValue,

    #[error("符号 {0} 不是函数")]
    NotAFunction(String),

    #[error("函数 {function} 需要 {expected} 个参数，实际传入 {actual} 个")]
    ArgumentCountMismatch {
        function: String,
        expected: usize,
        actual: usize,
    },

    #[error("无返回值函数不能返回一个值")]
    UnexpectedReturnValue,

    #[error("有返回值函数必须返回一个值")]
    MissingReturnValue,

    #[error("函数 {0} 的控制流结束时没有返回值")]
    MissingFunctionReturn(String),

    #[error("当前不在函数中")]
    NoCurrentFunction,
}
