//! 集成测试
//!
//! 测试整个x86汇编测试框架的功能

use std::path::Path;

// 导入模块
use x86_asm_test::{
    parser,
    executor,
    types::{ExecutionMode}
};

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

    let parsed = parser::parse_asm_test_content(content).unwrap();
    assert!(parsed.config.mode.is_some());
    assert!(matches!(parsed.config.mode.unwrap(), ExecutionMode::Bit32));
    assert_eq!(parsed.assembly_code.trim(), "mov eax, 1\nret");
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

nop
"#;

    let parsed = parser::parse_asm_test_content(content).unwrap();
    assert!(parsed.config.reg_data.is_some());
    let reg_data = parsed.config.reg_data.as_ref().unwrap();
    assert_eq!(reg_data.rax, Some("0x123456789abcdef0".to_string()));
    assert_eq!(reg_data.rbx, Some("0x0fedcba987654321".to_string()));
    assert_eq!(parsed.assembly_code.trim(), "mov rax, rbx\nadd rax, 0x10000000\n\nnop");
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

nop
"#;

    let parsed = parser::parse_asm_test_content(content).unwrap();
    assert!(parsed.config.reg_init.is_some());
    let reg_init = parsed.config.reg_init.as_ref().unwrap();
    assert_eq!(reg_init.rax, Some("0x123456789abcdef0".to_string()));
    assert_eq!(reg_init.rbx, Some("0x0fedcba987654321".to_string()));
    assert_eq!(parsed.assembly_code.trim(), "mov rax, rbx\nadd rax, 0x10000000\n\nnop");
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

nop
"#;

    let parsed = parser::parse_asm_test_content(content).unwrap();
    assert!(parsed.config.memory_regions.is_some());
    let memory_regions = parsed.config.memory_regions.as_ref().unwrap();
    assert_eq!(memory_regions.len(), 2);
    assert!(memory_regions.contains_key("0x10000000"));
    assert!(memory_regions.contains_key("0x20000000"));
    assert_eq!(parsed.assembly_code.trim(), "mov rax, 0x10000000\nmov [rax], 0x12345678\n\nnop");
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

nop
"#;

    let parsed = parser::parse_asm_test_content(content).unwrap();
    assert!(parsed.config.memory_data.is_some());
    let memory_data = parsed.config.memory_data.as_ref().unwrap();
    assert_eq!(memory_data.len(), 1);
    assert!(memory_data.contains_key("0x10000000"));
    assert_eq!(parsed.assembly_code.trim(), "mov rax, 0x10000000\nmov ebx, [rax]\n\nnop");
}

#[test]
fn test_parse_real_asm_files() {
    // 测试真实存在的ASM文件
    let simple_test_path = "sample/simple_test.asm";
    if Path::new(simple_test_path).exists() {
        let parsed = parser::parse_asm_test_file(simple_test_path).unwrap();
        assert!(parsed.assembly_code.contains("mov rax, 5"));
        assert!(parsed.assembly_code.contains("mov rbx, 10"));
        assert!(parsed.assembly_code.contains("add rax, rbx"));
    }

    let reg_init_test_path = "sample/reg_init_test.asm";
    if Path::new(reg_init_test_path).exists() {
        let parsed = parser::parse_asm_test_file(reg_init_test_path).unwrap();
        assert!(parsed.config.reg_init.is_some());
        assert!(parsed.assembly_code.contains("add rax, rbx"));
    }

    let memory_test_path = "sample/memory_test.asm";
    if Path::new(memory_test_path).exists() {
        let parsed = parser::parse_asm_test_file(memory_test_path).unwrap();
        assert!(parsed.config.memory_regions.is_some());
        assert!(parsed.config.memory_data.is_some());
        assert!(parsed.assembly_code.contains("mov rax, [0x10000000]"));
    }

    let xmm_ymm_test_path = "tests/xmm_ymm_test.asm";
    if Path::new(xmm_ymm_test_path).exists() {
        let parsed = parser::parse_asm_test_file(xmm_ymm_test_path).unwrap();
        assert!(parsed.config.reg_init.is_some());
        if let Some(ref reg_init) = parsed.config.reg_init {
            assert!(reg_init.xmm0.is_some());
            let xmm0_values = reg_init.xmm0.as_ref().unwrap();
            assert_eq!(xmm0_values.len(), 4);
            assert_eq!(xmm0_values[0], "0x1111111111111111");
            assert_eq!(xmm0_values[1], "0x2222222222222222");
            assert_eq!(xmm0_values[2], "0x3333333333333333");
            assert_eq!(xmm0_values[3], "0x4444444444444444");
        }
    }
}

#[test]
fn test_parse_hex_address() {
    assert_eq!(executor::parse_hex_address("0x10000000").unwrap(), 0x10000000);
    assert_eq!(executor::parse_hex_address("0X20000000").unwrap(), 0x20000000);
    assert_eq!(executor::parse_hex_address("10000000").unwrap(), 10000000);
    assert!(executor::parse_hex_address("invalid").is_err());
}

#[test]
fn test_parse_hex_value() {
    assert_eq!(executor::parse_hex_value("0x12345678").unwrap(), 0x12345678);
    assert_eq!(executor::parse_hex_value("0XABCDEF").unwrap(), 0xABCDEF);
    assert_eq!(executor::parse_hex_value("123456").unwrap(), 0x123456); // 十六进制123456 = 1193046
    assert_eq!(executor::parse_hex_value("").unwrap(), 0);
    assert!(executor::parse_hex_value("invalid").is_err());
}