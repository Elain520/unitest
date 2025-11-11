//! x86汇编测试框架主程序

use anyhow::Result;

mod cli;
mod error;
mod parser;
mod compiler;
mod linker;
mod elf;

use cli::Cli;
use parser::parse_asm_test_file;
use compiler::compile_with_nasm;
use linker::link_with_system_linker;
use elf::parse_elf_file;

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

                // 编译汇编文件
                match compile_with_nasm(test_file_path, &asm_test_file.config, Some("/tmp")) {
                    Ok(compile_result) => {
                        if compile_result.success {
                            if !cli.quiet {
                                println!("成功编译汇编文件: {}", compile_result.object_file);
                            }

                            // 链接目标文件
                            match link_with_system_linker(&compile_result.object_file, &asm_test_file.config, Some("/tmp")) {
                                Ok(link_result) => {
                                    if link_result.success {
                                        if !cli.quiet {
                                            println!("成功链接目标文件: {}", link_result.executable_file);
                                        }

                                        // 解析ELF文件
                                        match parse_elf_file(&link_result.executable_file) {
                                            Ok(elf_info) => {
                                                if !cli.quiet {
                                                    println!("成功解析ELF文件");
                                                    println!("入口点地址: 0x{:x}", elf_info.entry_point);
                                                    println!("架构类型: {}", if elf_info.is_32bit { "32位" } else { "64位" });

                                                    if let Some(ref code_section) = elf_info.code_section {
                                                        println!("代码段大小: {} 字节", code_section.size);
                                                    }

                                                    if let Some(ref data_section) = elf_info.data_section {
                                                        println!("数据段大小: {} 字节", data_section.size);
                                                    }
                                                }

                                                // TODO: 实现后续的执行逻辑

                                                // 清理ELF文件
                                                if let Err(e) = elf::cleanup_elf_files(&link_result.executable_file) {
                                                    eprintln!("清理ELF文件时出错: {}", e);
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("解析ELF文件时出错: {}", e);
                                                return Err(e.into());
                                            }
                                        }

                                        // 清理链接生成的文件
                                        if let Err(e) = linker::cleanup_linked_files(&link_result.executable_file) {
                                            eprintln!("清理链接文件时出错: {}", e);
                                        }

                                        // 清理编译生成的文件
                                        if let Err(e) = compiler::cleanup_compiled_files(&compile_result.object_file) {
                                            eprintln!("清理编译文件时出错: {}", e);
                                        }
                                    } else {
                                        eprintln!("链接失败: {:?}", link_result.error_message);
                                        return Err(anyhow::anyhow!("链接失败"));
                                    }
                                }
                                Err(e) => {
                                    eprintln!("链接目标文件时出错: {}", e);
                                    return Err(e.into());
                                }
                            }
                        } else {
                            eprintln!("编译失败: {:?}", compile_result.error_message);
                            return Err(anyhow::anyhow!("编译失败"));
                        }
                    }
                    Err(e) => {
                        eprintln!("编译汇编文件时出错: {}", e);
                        return Err(e.into());
                    }
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