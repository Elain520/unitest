//! 错误处理模块
//!
//! 定义项目中使用的错误类型

use thiserror::Error;

/// x86汇编测试框架错误类型
#[derive(Error, Debug)]
pub enum AsmTestError {
    /// IO错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON解析错误
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// ELF解析错误
    #[error("ELF parse error: {0}")]
    ElfParse(#[from] goblin::error::Error),

    /// 内存映射错误
    #[error("Memory mapping error: {0}")]
    MemoryMap(String),

    /// 执行错误
    #[error("Execution error: {0}")]
    Execution(String),

    /// 配置解析错误
    #[error("Configuration parse error: {0}")]
    ConfigParse(String),

    /// 汇编文件格式错误
    #[error("Assembly file format error: {0}")]
    AsmFormat(String),

    /// 系统调用错误
    #[error("System call error: {0}")]
    SystemCall(String),

    /// 其他错误
    #[error("Other error: {0}")]
    Other(String),
}

/// Result类型别名
pub type Result<T> = std::result::Result<T, AsmTestError>;