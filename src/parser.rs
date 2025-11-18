//! 汇编文件解析模块
//!
//! 负责解析包含JSON配置的汇编文件，提取配置部分和汇编代码部分

use crate::error::{AsmTestError, Result};
use serde_json;
use std::fs;
use std::path::Path;
use crate::types::{AsmTestConfig, AsmTestFile};

/// 解析汇编测试文件
pub fn parse_asm_test_file<P: AsRef<Path>>(file_path: P) -> Result<AsmTestFile> {
    // 读取文件内容
    let content = fs::read_to_string(file_path)?;

    // 解析文件内容
    parse_asm_test_content(&content)
}

/// 解析汇编测试文件内容
pub fn parse_asm_test_content(content: &str) -> Result<AsmTestFile> {
    // 查找CONFIG块
    let (config_str, asm_code) = extract_config_block(content)?;

    // 解析JSON配置
    let config = if !config_str.is_empty() {
        parse_config_json(&config_str)?
    } else {
        AsmTestConfig::new()
    };

    Ok(AsmTestFile {
        config,
        assembly_code: asm_code,
    })
}

/// 提取CONFIG块
fn extract_config_block(content: &str) -> Result<(String, String)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_config_block = false;
    let mut config_lines = Vec::new();
    let mut asm_lines = Vec::new();

    for line in lines {
        if line.trim() == "%ifdef CONFIG" {
            in_config_block = true;
            continue;
        } else if line.trim() == "%endif" {
            in_config_block = false;
            continue;
        }

        if in_config_block {
            config_lines.push(line);
        } else {
            asm_lines.push(line);
        }
    }

    let config_str = config_lines.join("\n");
    let asm_code = asm_lines.join("\n");

    Ok((config_str, asm_code))
}

/// 解析JSON配置
fn parse_config_json(json_str: &str) -> Result<AsmTestConfig> {
    if json_str.trim().is_empty() {
        return Ok(AsmTestConfig::new());
    }

    // 解析JSON
    let config: AsmTestConfig = serde_json::from_str(json_str)
        .map_err(|e| AsmTestError::ConfigParse(format!("JSON解析失败: {}", e)))?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_asm_file() {
        let content = r#"%ifdef CONFIG
{
    "Mode": "32BIT"
}
%endif

mov eax, 1
ret
"#;

        let result = parse_asm_test_content(content).unwrap();
        assert!(result.config.mode.is_some());
        assert!(result.config.reg_data.is_none());
        assert_eq!(result.assembly_code.trim(), "mov eax, 1\nret");
    }

    #[test]
    fn test_parse_asm_file_without_config() {
        let content = r#"mov eax, 1
ret
"#;

        let result = parse_asm_test_content(content).unwrap();
        assert!(result.config.mode.is_none());
        assert!(result.config.reg_data.is_none());
        assert_eq!(result.assembly_code.trim(), "mov eax, 1\nret");
    }
}