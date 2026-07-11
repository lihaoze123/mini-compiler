use thiserror::Error;

#[derive(Error, Debug)]
pub enum GenerateAsmError {
    #[error("生成汇编时写字符串错误")]
    Write(#[from] std::fmt::Error),

    #[error("解析字符串时错误")]
    Parse,

    #[error("解析 Koopa IR 时错误: {0:?}")]
    KoopaParse(koopa::front::span::Error),

    #[error("缺少栈槽")]
    MissingStackSlot,

    #[error("基本块无名称")]
    BBNoName,

    #[error("全局变量无名称")]
    GlobalValueNoName,

    #[error("不支持的全局变量初始化器")]
    UnsupportedGlobalInitializer,

    #[error("期望指针类型")]
    ExpectedPointer,

    #[error("不支持的 Koopa 值: {0}")]
    UnsupportedValue(String),

    #[error("未知错误")]
    Unknown,
}
