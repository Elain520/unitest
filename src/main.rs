//! x86汇编测试框架主程序

use anyhow::Result;

mod cli;
mod error;
mod parser;

use cli::Cli;
use parser::parse_asm_test_file;

fn main() -> Result<()> {
    // 解析命令行参数
    let cli = Cli::parse_args();

    // 如果没有指定测试文件，显示帮助信息
    if cli.test_file.is_none() {
        println!("请指定要测试的汇编文件");
        println!("使用 --help 查看更多选项");
        return Ok(());
    }

    // 执行测试
    execute_test(&cli)?;

    Ok(())
}

/// 执行测试
fn execute_test(cli: &Cli) -> Result<()> {
    if !cli.quiet {
        println!("正在执行测试文件: {:?}", cli.test_file);
    }

    // 解析汇编测试文件
    if let Some(ref test_file_path) = cli.test_file {
        match parse_asm_test_file(test_file_path) {
            Ok(asm_test_file) => {
                if !cli.quiet {
                    println!("成功解析汇编文件");
                    println!("配置: {:?}", asm_test_file.config);
                    println!("汇编代码行数: {}", asm_test_file.assembly_code.lines().count());
                }
            }
            Err(e) => {
                eprintln!("解析汇编文件时出错: {}", e);
                return Err(e.into());
            }
        }
    }

    // TODO: 实现后续的测试执行逻辑

    Ok(())
}