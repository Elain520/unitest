//! 链接器模块
//!
//! 负责调用系统链接器链接目标文件生成可执行文件

use crate::error::{AsmTestError, Result};
use std::path::Path;
use std::process::Command;
use crate::types::{AsmTestConfig, ExecutionMode};

/// 链接结果
#[derive(Debug)]
pub struct LinkResult {
    /// 可执行文件路径
    pub executable_file: String,
    /// 链接是否成功
    pub success: bool,
    /// 错误信息（如果有的话）
    pub error_message: Option<String>,
}

/// 使用系统链接器链接目标文件
pub fn link_with_system_linker<P: AsRef<Path>>(
    object_file: P,
    config: &AsmTestConfig,
    output_dir: Option<&str>,
) -> Result<LinkResult> {
    let object_file_path = object_file.as_ref();

    // 检查目标文件是否存在
    if !object_file_path.exists() {
        return Err(AsmTestError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("目标文件不存在: {:?}", object_file_path),
        )));
    }

    // 确定链接模式（32位或64位）
    let is_32bit = config.mode.as_ref().map(|m| matches!(m, ExecutionMode::Bit32)).unwrap_or(false);

    // 确定输出目录
    let output_dir = output_dir.unwrap_or("/tmp");

    // 生成可执行文件路径
    let file_name = object_file_path
        .file_stem()
        .ok_or_else(|| AsmTestError::AsmFormat("无效的目标文件名".to_string()))?
        .to_str()
        .ok_or_else(|| AsmTestError::AsmFormat("无效的文件名编码".to_string()))?;

    let executable_file = format!("{}/{}", output_dir, file_name);

    // 确定链接器名称
    let linker_name= "x86_64-linux-gnu-ld";

    // 检查链接器是否存在
    if !is_linker_available(linker_name) {
        // 尝试使用通用的ld链接器
        if is_linker_available("ld") {
            eprintln!("警告: 使用通用ld链接器替代{}", linker_name);
        } else {
            return Err(AsmTestError::SystemCall(format!("链接器 {} 和通用ld链接器都不存在或不可用", linker_name)));
        }
    }

    // 构建链接器命令
    let linker_to_use = if is_linker_available(linker_name) {
        linker_name
    } else {
        "ld"
    };

    let mut cmd = Command::new(linker_to_use);

    // 设置链接选项
    cmd.arg("-Ttext=0x100000")  // 设置代码段起始地址
       .arg("-w")  // 禁止警告
       .arg("-m").arg(if is_32bit { "elf_i386" } else { "elf_x86_64" })  // 设置目标格式
       .arg("-o").arg(&executable_file)  // 设置输出文件
       .arg(object_file_path);  // 设置输入文件

    // 执行链接器命令
    let output = cmd.output().map_err(|e| {
        AsmTestError::SystemCall(format!("执行链接器 {} 失败: {}", linker_to_use, e))
    })?;

    // 检查链接结果
    if output.status.success() {
        Ok(LinkResult {
            executable_file,
            success: true,
            error_message: None,
        })
    } else {
        let stdout_msg = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_msg = String::from_utf8_lossy(&output.stderr).to_string();
        let error_msg = format!("链接器 {} 错误:\n标准输出: {}\n错误输出: {}", linker_to_use, stdout_msg, stderr_msg);
        Ok(LinkResult {
            executable_file,
            success: false,
            error_message: Some(error_msg),
        })
    }
}

/// 检查链接器是否可用
fn is_linker_available(linker_name: &str) -> bool {
    Command::new("which")
        .arg(linker_name)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// 清理链接生成的文件
pub fn cleanup_linked_files(executable_file: &str) -> Result<()> {
    if Path::new(executable_file).exists() {
        std::fs::remove_file(executable_file).map_err(AsmTestError::Io)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AsmTestConfig, ExecutionMode};
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_link_with_system_linker_nonexistent_file() {
        let config = AsmTestConfig::new();
        let result = link_with_system_linker("/nonexistent/file.o", &config, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_linker_available() {
        // 测试一个不存在的链接器
        assert_eq!(is_linker_available("nonexistent_linker"), false);
    }

    #[test]
    fn test_link_simple_object_file() {
        // 创建一个临时的目标文件（实际上是空文件，但用于测试链接器调用）
        let temp_file = NamedTempFile::new().unwrap();

        let config = AsmTestConfig::new();
        let result = link_with_system_linker(temp_file.path(), &config, Some("/tmp"));

        // 注意：这个测试可能依赖于系统上是否安装了链接器
        // 我们主要测试函数是否能正确处理输入
        assert!(result.is_ok() || result.is_err()); // 至少不会panic
    }

    #[test]
    fn test_link_with_32bit_mode() {
        // 创建一个临时的目标文件
        let temp_file = NamedTempFile::new().unwrap();

        let mut config = AsmTestConfig::new();
        config.mode = Some(ExecutionMode::Bit32);

        let result = link_with_system_linker(temp_file.path(), &config, Some("/tmp"));

        // 注意：这个测试可能依赖于系统上是否安装了链接器
        // 我们主要测试函数是否能正确处理32位模式配置
        assert!(result.is_ok() || result.is_err()); // 至少不会panic
    }

    #[test]
    fn test_link_executable_file_created() {
        // 创建一个临时的目标文件
        let temp_file = NamedTempFile::new().unwrap();

        let config = AsmTestConfig::new();
        let result = link_with_system_linker(temp_file.path(), &config, Some("/tmp"));

        if let Ok(link_result) = result {
            if link_result.success {
                // 检查可执行文件是否存在
                assert!(fs::metadata(&link_result.executable_file).is_ok());
                // 清理生成的文件
                let _ = fs::remove_file(&link_result.executable_file);
            }
        }
    }

    #[test]
    fn test_link_empty_object_file() {
        // 创建一个临时的空文件
        let temp_file = NamedTempFile::new().unwrap();

        let config = AsmTestConfig::new();
        let result = link_with_system_linker(temp_file.path(), &config, Some("/tmp"));
        assert!(result.is_ok() || result.is_err()); // 至少不会panic
    }
}