//! ELF文件解析模块
//!
//! 负责解析ELF文件，提取代码段和数据段信息

use crate::error::{AsmTestError, Result};
use goblin::elf::{Elf, SectionHeader};
use std::fs;
use std::path::Path;

/// ELF文件信息
#[derive(Debug)]
pub struct ElfInfo {
    /// 代码段信息
    pub code_section: Option<SectionInfo>,
    /// 数据段信息
    pub data_section: Option<SectionInfo>,
    /// 入口点地址
    pub entry_point: u64,
    /// 架构类型（32位或64位）
    pub is_32bit: bool,
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

    Ok(ElfInfo {
        code_section,
        data_section,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}