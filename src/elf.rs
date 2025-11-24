//! ELF文件解析模块
//!
//! 负责解析ELF文件，提取代码段和数据段信息

use crate::error::{AsmTestError, Result};
use goblin::elf::{Elf, SectionHeader, Sym};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// ELF文件信息
#[derive(Debug)]
pub struct ElfInfo {
    /// 代码段信息
    pub code_section: Option<SectionInfo>,
    /// 数据段信息
    pub data_section: Option<SectionInfo>,
    /// 符号表信息
    pub symbols: HashMap<String, SymbolInfo>,
    /// 入口点地址
    pub entry_point: u64,
    /// 架构类型（32位或64位）
    pub is_32bit: bool,
}

/// 符号信息
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// 符号名称
    pub name: String,
    /// 符号地址
    pub address: u64,
    /// 符号大小
    pub size: u64,
    /// 符号类型
    pub sym_type: SymbolType,
}

/// 符号类型
#[derive(Debug, Clone)]
pub enum SymbolType {
    /// 函数符号
    Function,
    /// 对象符号
    Object,
    /// 未定义符号
    Undefined,
    /// 其他符号类型
    Other,
}

/// 段信息
#[derive(Debug)]
pub struct SectionInfo {
    /// 段名称
    pub name: String,
    /// 段在文件中的偏移
    pub offset: usize,
    /// 段的大小
    pub size: usize,
    /// 段在内存中的地址
    pub address: u64,
    /// 段的原始数据
    pub data: Vec<u8>,
}

/// 解析ELF文件
pub fn parse_elf_file<P: AsRef<Path>>(elf_file: P) -> Result<ElfInfo> {
    let elf_file_path = elf_file.as_ref();

    // 检查ELF文件是否存在
    if !elf_file_path.exists() {
        return Err(AsmTestError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("ELF文件不存在: {:?}", elf_file_path),
        )));
    }

    // 读取ELF文件内容
    let buffer = fs::read(elf_file_path).map_err(AsmTestError::Io)?;

    // 解析ELF文件
    let elf = Elf::parse(&buffer).map_err(AsmTestError::ElfParse)?;

    // 确定架构类型
    let is_32bit = elf.is_64 == false;

    // 查找代码段和数据段
    let mut code_section = None;
    let mut data_section = None;

    // 遍历所有段
    for section_header in &elf.section_headers {
        if let Some(section_name) = get_section_name(&elf, section_header) {
            if section_name == ".text" {
                // 提取代码段信息
                code_section = Some(extract_section_info(&elf, section_header, &section_name, &buffer)?);
            } else if section_name == ".data" {
                // 提取数据段信息
                data_section = Some(extract_section_info(&elf, section_header, &section_name, &buffer)?);
            }
        }
    }

    // 解析符号表
    let symbols = parse_symbols(&elf, &buffer)?;

    Ok(ElfInfo {
        code_section,
        data_section,
        symbols,
        entry_point: elf.entry,
        is_32bit,
    })
}

/// 获取段名称
fn get_section_name(elf: &Elf, section_header: &SectionHeader) -> Option<String> {
    elf.shdr_strtab.get_at(section_header.sh_name).map(|s| s.to_string())
}

/// 提取段信息
fn extract_section_info(_elf: &Elf, section_header: &SectionHeader, section_name: &str, buffer: &[u8]) -> Result<SectionInfo> {
    let offset = section_header.sh_offset as usize;
    let size = section_header.sh_size as usize;

    // 确保偏移和大小在缓冲区范围内
    if offset + size > buffer.len() {
        return Err(AsmTestError::ElfParse(goblin::error::Error::Malformed(format!(
            "段 {} 的偏移或大小超出文件范围", section_name
        ))));
    }

    let data = buffer[offset..offset + size].to_vec();

    Ok(SectionInfo {
        name: section_name.to_string(),
        offset,
        size,
        address: section_header.sh_addr,
        data,
    })
}

/// 清理ELF解析生成的文件
pub fn cleanup_elf_files(elf_file: &str) -> Result<()> {
    if Path::new(elf_file).exists() {
        fs::remove_file(elf_file).map_err(AsmTestError::Io)?;
    }
    Ok(())
}

/// 解析ELF符号表
fn parse_symbols(elf: &Elf, _buffer: &[u8]) -> Result<HashMap<String, SymbolInfo>> {
    let mut symbols = HashMap::new();

    // 遍历符号表
    for sym in &elf.syms {
        if let Some(name) = elf.strtab.get_at(sym.st_name) {
            if !name.is_empty() {
                let symbol_info = SymbolInfo {
                    name: name.to_string(),
                    address: sym.st_value,
                    size: sym.st_size,
                    sym_type: get_symbol_type(&sym),
                };
                symbols.insert(name.to_string(), symbol_info);
            }
        }
    }

    Ok(symbols)
}

/// 获取符号类型
fn get_symbol_type(sym: &Sym) -> SymbolType {
    match sym.st_type() {
        goblin::elf::sym::STT_FUNC => SymbolType::Function,
        goblin::elf::sym::STT_OBJECT => SymbolType::Object,
        goblin::elf::sym::STT_NOTYPE => {
            if sym.st_shndx == 0 {
                SymbolType::Undefined
            } else {
                SymbolType::Other
            }
        },
        _ => SymbolType::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_elf_file_nonexistent_file() {
        let result = parse_elf_file("/nonexistent/file.elf");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_section_name() {
        // 这个测试需要一个实际的ELF对象，所以我们只测试错误情况
        // 创建一个简单的ELF对象用于测试
        let buffer = vec![0u8; 64];
        if let Ok(elf) = Elf::parse(&buffer) {
            assert_eq!(get_section_name(&elf, &SectionHeader::default()), None);
        }
    }

    #[test]
    fn test_parse_empty_elf_file() {
        // 创建一个空的临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&[]).unwrap();
        temp_file.flush().unwrap();

        let result = parse_elf_file(temp_file.path());
        // 空文件应该无法解析为有效的ELF文件
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_elf_file() {
        // 创建一个包含无效数据的临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"invalid elf content").unwrap();
        temp_file.flush().unwrap();

        let result = parse_elf_file(temp_file.path());
        // 无效的ELF文件应该返回错误
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_elf_with_symbols() {
        use crate::compiler::*;
        use crate::linker::*;
        use crate::types::AsmTestConfig;
        use std::io::Write;
        use tempfile::NamedTempFile;

        // 创建一个包含函数和数据的汇编文件
        let mut asm_file = NamedTempFile::new().unwrap();
        writeln!(asm_file, "section .text\n global _start\n global my_function\n_start:\n  call my_function\n  mov rax, 1\n  ret\nmy_function:\n  mov rbx, 2\n  ret\nsection .data\nmy_data: dq 0x12345678").unwrap();
        asm_file.flush().unwrap();

        let config = AsmTestConfig::new();

        // 编译汇编文件
        let compile_result = compile_with_nasm(asm_file.path(), &config, Some("/tmp")).unwrap();
        assert!(compile_result.success);

        // 链接目标文件
        let link_result = link_with_system_linker(&compile_result.object_file, &config, Some("/tmp")).unwrap();
        assert!(link_result.success);

        // 解析生成的ELF文件
        let result = parse_elf_file(&link_result.executable_file);
        assert!(result.is_ok());

        if let Ok(elf_info) = result {
            // 检查符号表
            assert!(!elf_info.symbols.is_empty());

            // 检查是否存在_start符号
            assert!(elf_info.symbols.contains_key("_start"));

            // 检查是否存在my_function符号
            assert!(elf_info.symbols.contains_key("my_function"));
        }

        // 清理生成的文件
        let _ = std::fs::remove_file(&compile_result.object_file);
        let _ = std::fs::remove_file(&link_result.executable_file);
    }

    #[test]
    fn test_parse_32bit_elf_file() {
        use crate::compiler::*;
        use crate::linker::*;
        use crate::types::{AsmTestConfig, ExecutionMode};
        use std::io::Write;
        use tempfile::NamedTempFile;

        // 创建一个简单的32位汇编文件
        let mut asm_file = NamedTempFile::new().unwrap();
        writeln!(asm_file, "section .text\n bits 32\n global _start\n_start:\n  mov eax, 1\n  ret").unwrap();
        asm_file.flush().unwrap();

        let mut config = AsmTestConfig::new();
        config.mode = Some(ExecutionMode::Bit32);

        // 编译汇编文件（32位模式）
        let compile_result = compile_with_nasm(asm_file.path(), &config, Some("/tmp")).unwrap();
        assert!(compile_result.success);

        // 链接目标文件（32位模式）
        let link_result = link_with_system_linker(&compile_result.object_file, &config, Some("/tmp")).unwrap();
        assert!(link_result.success);

        // 解析生成的ELF文件
        let result = parse_elf_file(&link_result.executable_file);
        assert!(result.is_ok());

        if let Ok(elf_info) = result {
            // 检查架构类型
            assert_eq!(elf_info.is_32bit, true); // 应该是32位
        }

        // 清理生成的文件
        let _ = std::fs::remove_file(&compile_result.object_file);
        let _ = std::fs::remove_file(&link_result.executable_file);
    }

    #[test]
    fn test_parse_elf_with_text_section() {
        // 由于创建一个完整的包含.text段的ELF文件比较复杂，
        // 我们测试解析逻辑而不是完整的文件解析
        let buffer = vec![0u8; 128];
        if let Ok(elf) = Elf::parse(&buffer) {
            let section_header = SectionHeader {
                sh_name: 0,
                sh_type: goblin::elf::section_header::SHT_PROGBITS,
                sh_flags: (goblin::elf::section_header::SHF_ALLOC | goblin::elf::section_header::SHF_EXECINSTR) as u64,
                sh_addr: 0x1000,
                sh_offset: 0,
                sh_size: 32,
                sh_link: 0,
                sh_info: 0,
                sh_addralign: 16,
                sh_entsize: 0,
            };

            let result = extract_section_info(&elf, &section_header, ".text", &buffer);
            // 由于缓冲区太小，这应该会失败
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parse_elf_with_data_section() {
        // 测试数据段提取逻辑
        let buffer = vec![0u8; 128];
        if let Ok(elf) = Elf::parse(&buffer) {
            let section_header = SectionHeader {
                sh_name: 0,
                sh_type: goblin::elf::section_header::SHT_PROGBITS,
                sh_flags: (goblin::elf::section_header::SHF_ALLOC | goblin::elf::section_header::SHF_WRITE) as u64,
                sh_addr: 0x2000,
                sh_offset: 0,
                sh_size: 32,
                sh_link: 0,
                sh_info: 0,
                sh_addralign: 8,
                sh_entsize: 0,
            };

            let result = extract_section_info(&elf, &section_header, ".data", &buffer);
            // 由于缓冲区太小，这应该会失败
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_get_symbol_type() {
        // 测试函数符号类型
        let func_sym = Sym {
            st_name: 0,
            st_info: (goblin::elf::sym::STB_GLOBAL << 4) | goblin::elf::sym::STT_FUNC,
            st_other: 0,
            st_shndx: 1,
            st_value: 0x1000,
            st_size: 16,
        };
        assert!(matches!(get_symbol_type(&func_sym), SymbolType::Function));

        // 测试对象符号类型
        let obj_sym = Sym {
            st_name: 0,
            st_info: (goblin::elf::sym::STB_GLOBAL << 4) | goblin::elf::sym::STT_OBJECT,
            st_other: 0,
            st_shndx: 1,
            st_value: 0x2000,
            st_size: 8,
        };
        assert!(matches!(get_symbol_type(&obj_sym), SymbolType::Object));

        // 测试未定义符号类型
        let undef_sym = Sym {
            st_name: 0,
            st_info: (goblin::elf::sym::STB_GLOBAL << 4) | goblin::elf::sym::STT_NOTYPE,
            st_other: 0,
            st_shndx: 0, // SHN_UNDEF
            st_value: 0,
            st_size: 0,
        };
        assert!(matches!(get_symbol_type(&undef_sym), SymbolType::Undefined));

        // 测试其他符号类型
        let other_sym = Sym {
            st_name: 0,
            st_info: (goblin::elf::sym::STB_GLOBAL << 4) | goblin::elf::sym::STT_NOTYPE,
            st_other: 0,
            st_shndx: 1,
            st_value: 0x3000,
            st_size: 4,
        };
        assert!(matches!(get_symbol_type(&other_sym), SymbolType::Other));
    }

    #[test]
    fn test_parse_symbols() {
        // 创建一个简单的ELF对象用于测试符号解析
        let buffer = vec![0u8; 64];
        if let Ok(elf) = Elf::parse(&buffer) {
            let symbols = parse_symbols(&elf, &buffer).unwrap();
            // 没有符号的ELF文件应该返回空的符号映射
            assert_eq!(symbols.len(), 0);
        }
    }

    #[test]
    fn test_cleanup_elf_files_nonexistent() {
        // 测试清理不存在的文件
        let result = cleanup_elf_files("/nonexistent/file.elf");
        // 清理不存在的文件不应该返回错误
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_elf_files_existing() {
        // 创建一个临时文件并测试清理
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_str().unwrap().to_string();

        // 确保文件存在
        assert!(std::path::Path::new(&file_path).exists());

        // 清理文件
        let result = cleanup_elf_files(&file_path);
        assert!(result.is_ok());

        // 文件应该已被删除
        assert!(!std::path::Path::new(&file_path).exists());
    }
}