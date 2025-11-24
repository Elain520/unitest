//! 汇编文件解析模块
//!
//! 负责解析包含JSON配置的汇编文件，提取配置部分和汇编代码部分

use crate::error::{AsmTestError, Result};
use crate::types::{AsmTestConfig, AsmTestFile, ExecutionMode};
use serde_json;
use std::fs;
use std::path::Path;

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
    use crate::types::ExecutionMode;
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

    #[test]
    fn test_parse_asm_file_with_reg_data() {
        let content = r#"%ifdef CONFIG
{
    "RegData": {
        "RAX": "0x123456789abcdef0",
        "RBX": "0x0fedcba987654321"
    }
}
%endif

mov rax, rbx
add rax, 0x10000000

hlt
"#;

        let result = parse_asm_test_content(content).unwrap();
        assert!(result.config.reg_data.is_some());
        let reg_data = result.config.reg_data.as_ref().unwrap();
        assert_eq!(reg_data.rax, Some("0x123456789abcdef0".to_string()));
        assert_eq!(reg_data.rbx, Some("0x0fedcba987654321".to_string()));
        assert_eq!(result.assembly_code.trim(), "mov rax, rbx\nadd rax, 0x10000000\n\nhlt");
    }

    #[test]
    fn test_parse_asm_file_with_reg_init() {
        let content = r#"%ifdef CONFIG
{
    "RegInit": {
        "RAX": "0x123456789abcdef0",
        "RBX": "0x0fedcba987654321"
    }
}
%endif

mov rax, rbx
add rax, 0x10000000

hlt
"#;

        let result = parse_asm_test_content(content).unwrap();
        assert!(result.config.reg_init.is_some());
        let reg_init = result.config.reg_init.as_ref().unwrap();
        assert_eq!(reg_init.rax, Some("0x123456789abcdef0".to_string()));
        assert_eq!(reg_init.rbx, Some("0x0fedcba987654321".to_string()));
        assert_eq!(result.assembly_code.trim(), "mov rax, rbx\nadd rax, 0x10000000\n\nhlt");
    }

    #[test]
    fn test_parse_asm_file_with_memory_regions() {
        let content = r#"%ifdef CONFIG
{
    "MemoryRegions": {
        "0x10000000": 4096,
        "0x20000000": "0x2000"
    }
}
%endif

mov rax, 0x10000000
mov [rax], 0x12345678

hlt
"#;

        let result = parse_asm_test_content(content).unwrap();
        assert!(result.config.memory_regions.is_some());
        let memory_regions = result.config.memory_regions.as_ref().unwrap();
        assert_eq!(memory_regions.len(), 2);
        assert!(memory_regions.contains_key("0x10000000"));
        assert!(memory_regions.contains_key("0x20000000"));
        assert_eq!(result.assembly_code.trim(), "mov rax, 0x10000000\nmov [rax], 0x12345678\n\nhlt");
    }

    #[test]
    fn test_parse_asm_file_with_memory_data() {
        let content = r#"%ifdef CONFIG
{
    "MemoryData": {
        "0x10000000": "0x12345678 0x9abcdef0"
    }
}
%endif

mov rax, 0x10000000
mov ebx, [rax]

hlt
"#;

        let result = parse_asm_test_content(content).unwrap();
        assert!(result.config.memory_data.is_some());
        let memory_data = result.config.memory_data.as_ref().unwrap();
        assert_eq!(memory_data.len(), 1);
        assert!(memory_data.contains_key("0x10000000"));
        assert_eq!(result.assembly_code.trim(), "mov rax, 0x10000000\nmov ebx, [rax]\n\nhlt");
    }

    #[test]
    fn test_parse_asm_file_with_complex_config() {
        let content = r#"%ifdef CONFIG
{
    "RegInit": {
        "RAX": "0x123456789abcdef0",
        "RBX": "0x0fedcba987654321"
    },
    "Mode": "32BIT",
    "MemoryRegions": {
        "0x10000000": 4096
    },
    "MemoryData": {
        "0x10000000": "0x12345678"
    }
}
%endif

mov eax, ebx
add eax, [0x10000000]

hlt
"#;

        let result = parse_asm_test_content(content).unwrap();
        assert!(result.config.reg_init.is_some());
        assert!(result.config.mode.is_some());
        assert!(result.config.memory_regions.is_some());
        assert!(result.config.memory_data.is_some());

        let reg_init = result.config.reg_init.as_ref().unwrap();
        assert_eq!(reg_init.rax, Some("0x123456789abcdef0".to_string()));
        assert_eq!(reg_init.rbx, Some("0x0fedcba987654321".to_string()));

        assert!(matches!(result.config.mode.as_ref().unwrap(), ExecutionMode::Bit32));

        let memory_regions = result.config.memory_regions.as_ref().unwrap();
        assert_eq!(memory_regions.len(), 1);
        assert!(memory_regions.contains_key("0x10000000"));

        let memory_data = result.config.memory_data.as_ref().unwrap();
        assert_eq!(memory_data.len(), 1);
        assert!(memory_data.contains_key("0x10000000"));

        assert_eq!(result.assembly_code.trim(), "mov eax, ebx\nadd eax, [0x10000000]\n\nhlt");
    }

    #[test]
    fn test_parse_asm_file_with_empty_config() {
        let content = r#"%ifdef CONFIG
{}
%endif

mov eax, 1
ret
"#;

        let result = parse_asm_test_content(content).unwrap();
        assert!(result.config.mode.is_none());
        assert!(result.config.reg_data.is_none());
        assert!(result.config.reg_init.is_none());
        assert!(result.config.memory_regions.is_none());
        assert!(result.config.memory_data.is_none());
        assert_eq!(result.assembly_code.trim(), "mov eax, 1\nret");
    }

    #[test]
    fn test_parse_asm_file_with_invalid_json() {
        let content = r#"%ifdef CONFIG
{
    "Mode": "32BIT",
    "InvalidJson":
}
%endif

mov eax, 1
ret
"#;

        let result = parse_asm_test_content(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_asm_file_with_malformed_config() {
        let content = r#"%ifdef CONFIG
{
    "Mode": "INVALID_MODE"
}
%endif

mov eax, 1
ret
"#;

        let result = parse_asm_test_content(content);
        assert!(result.is_err());
    }
}