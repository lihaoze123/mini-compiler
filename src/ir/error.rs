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

    #[error("数组长度必须为正数，实际为 {0}")]
    InvalidArrayLength(i32),

    #[error("数组大小溢出")]
    ArraySizeOverflow,

    #[error("数组 {0} 的下标过多")]
    TooManyIndices(String),

    #[error("数组 {array} 的常量下标 {index} 越界，维度长度为 {length}")]
    ArrayIndexOutOfBounds {
        array: String,
        index: i32,
        length: usize,
    },

    #[error("类型不匹配，期望 {expected}，实际为 {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("函数 {function} 的第 {index} 个参数类型不匹配，期望 {expected}，实际为 {actual}")]
    ArgumentTypeMismatch {
        function: String,
        index: usize,
        expected: String,
        actual: String,
    },

    #[error("{0} 不是可赋值的标量左值")]
    ExpectedScalarLVal(String),

    #[error("初始化器与目标类型 {0} 不匹配")]
    InvalidInitializer(String),

    #[error("类型 {0} 的初始化元素过多")]
    TooManyInitializers(String),

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
