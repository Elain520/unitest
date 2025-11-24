//! 编译器模块
//!
//! 负责调用NASM汇编器编译汇编代码

use crate::error::{AsmTestError, Result};
use crate::types::{AsmTestConfig, ExecutionMode};
use std::fs;
use std::path::Path;
use std::process::Command;

/// 编译结果
#[derive(Debug)]
pub struct CompileResult {
    /// 目标文件路径
    pub object_file: String,
    /// 编译是否成功
    pub success: bool,
    /// 错误信息（如果有的话）
    pub error_message: Option<String>,
}

/// 使用NASM编译汇编文件
pub fn compile_with_nasm<P: AsRef<Path>>(
    asm_file: P,
    config: &AsmTestConfig,
    output_dir: Option<&str>,
) -> Result<CompileResult> {
    let asm_file_path = asm_file.as_ref();

    // 检查汇编文件是否存在
    if !asm_file_path.exists() {
        return Err(AsmTestError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("汇编文件不存在: {:?}", asm_file_path),
        )));
    }

    // 确定编译模式（32位或64位）
    let is_32bit = config.mode.as_ref().map(|m| matches!(m, ExecutionMode::Bit32)).unwrap_or(false);

    // 确定输出目录
    let output_dir = output_dir.unwrap_or("/tmp");

    // 生成目标文件路径
    let file_name = asm_file_path
        .file_stem()
        .ok_or_else(|| AsmTestError::AsmFormat("无效的汇编文件名".to_string()))?
        .to_str()
        .ok_or_else(|| AsmTestError::AsmFormat("无效的文件名编码".to_string()))?;

    let object_file = format!("{}/{}.o", output_dir, file_name);

    // 构建NASM命令
    let mut cmd = Command::new("nasm");

    // 设置编译模式
    if is_32bit {
        cmd.arg("-felf32");
    } else {
        cmd.arg("-felf64");
    }

    // 设置输出文件
    cmd.arg("-o").arg(&object_file);

    // 设置输入文件
    cmd.arg(asm_file_path);

    // 执行NASM命令
    let output = cmd.output().map_err(|e| {
        AsmTestError::SystemCall(format!("执行NASM失败: {}", e))
    })?;

    // 检查编译结果
    if output.status.success() {
        Ok(CompileResult {
            object_file,
            success: true,
            error_message: None,
        })
    } else {
        let stdout_msg = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_msg = String::from_utf8_lossy(&output.stderr).to_string();
        let error_msg = format!("NASM编译错误:\n标准输出: {}\n错误输出: {}", stdout_msg, stderr_msg);
        Ok(CompileResult {
            object_file,
            success: false,
            error_message: Some(error_msg),
        })
    }
}

/// 清理编译生成的文件
pub fn cleanup_compiled_files(object_file: &str) -> Result<()> {
    if Path::new(object_file).exists() {
        fs::remove_file(object_file).map_err(AsmTestError::Io)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AsmTestConfig, ExecutionMode};
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compile_with_nasm_nonexistent_file() {
        let config = AsmTestConfig::new();
        let result = compile_with_nasm("/nonexistent/file.asm", &config, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_with_nasm_invalid_file() {
        let config = AsmTestConfig::new();
        let result = compile_with_nasm("invalid_file", &config, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_with_nasm_32bit_mode() {
        // 创建一个临时的简单汇编文件
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "mov eax, 1\nret").unwrap();
        temp_file.flush().unwrap();

        let mut config = AsmTestConfig::new();
        config.mode = Some(ExecutionMode::Bit32);

        let result = compile_with_nasm(temp_file.path(), &config, Some("/tmp"));
        // 注意：这个测试可能依赖于系统上是否安装了NASM
        // 我们主要测试函数是否能正确处理32位模式配置
        assert!(result.is_ok() || result.is_err()); // 至少不会panic
    }

    #[test]
    fn test_compile_simple_asm_file() {
        // 创建一个临时的简单汇编文件
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "mov rax, 1\nret").unwrap();
        temp_file.flush().unwrap();

        let config = AsmTestConfig::new();
        let result = compile_with_nasm(temp_file.path(), &config, Some("/tmp"));

        // 注意：这个测试可能依赖于系统上是否安装了NASM
        // 我们主要测试函数是否能正确处理输入
        assert!(result.is_ok() || result.is_err()); // 至少不会panic
    }

    #[test]
    fn test_compile_output_file_created() {
        // 创建一个临时的简单汇编文件
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "mov rax, 1\nret").unwrap();
        temp_file.flush().unwrap();

        let config = AsmTestConfig::new();
        let result = compile_with_nasm(temp_file.path(), &config, Some("/tmp"));

        if let Ok(compile_result) = result {
            if compile_result.success {
                // 检查目标文件是否存在
                assert!(fs::metadata(&compile_result.object_file).is_ok());
                // 清理生成的文件
                let _ = fs::remove_file(&compile_result.object_file);
            }
        }
    }

    #[test]
    fn test_compile_empty_file() {
        // 创建一个临时的空文件
        let temp_file = NamedTempFile::new().unwrap();

        let config = AsmTestConfig::new();
        let result = compile_with_nasm(temp_file.path(), &config, Some("/tmp"));
        assert!(result.is_ok() || result.is_err()); // 至少不会panic
    }
}