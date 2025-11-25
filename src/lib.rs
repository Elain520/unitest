//! x86汇编测试框架核心模块
//!
//! 该模块提供了x86汇编测试框架的核心功能，包括：
//! - 汇编文件解析
//! - JSON配置解析
//! - 内存管理
//! - 原生执行环境
//! - 寄存器状态捕获
pub mod cli;
pub mod error;
pub mod parser;
pub mod compiler;
pub mod linker;
pub mod elf;
pub mod executor;
pub mod types;


#[cfg(test)]
mod tests {
    use crate::types::{AsmTestConfig, RegisterData};

    #[test]
    fn test_asm_test_config_new() {
        let config = AsmTestConfig::new();
        assert!(config.reg_data.is_none());
        assert!(config.mode.is_none());
        assert!(config.memory_regions.is_none());
        assert!(config.memory_data.is_none());
    }

    #[test]
    fn test_register_data_new() {
        let reg_data = RegisterData::new();
        assert!(reg_data.rax.is_none());
        assert!(reg_data.rcx.is_none());
        // ... other assertions
    }
}