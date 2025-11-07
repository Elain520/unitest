//! 命令行参数解析模块
//!
//! 处理命令行参数和用户输入

use clap::Parser;

/// x86汇编测试框架
#[derive(Parser, Debug)]
#[clap(name = "x86-asm-test", version = "0.1.0", author = "Your Name <your.email@example.com>")]
pub struct Cli {
    /// 测试模式：指定要测试的汇编文件
    #[clap(long = "test", short = 't', value_name = "FILE")]
    pub test_file: Option<String>,

    /// 包含路径：指定汇编文件的包含路径
    #[clap(long = "include", short = 'i', value_name = "PATH")]
    pub include_path: Option<String>,

    /// 输出模式：指定输出文件路径
    #[clap(long = "output", short = 'o', value_name = "FILE")]
    pub output_file: Option<String>,

    /// 详细模式：显示更多执行信息
    #[clap(long = "verbose", short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// 静默模式：不显示任何输出
    #[clap(long = "quiet", short = 'q')]
    pub quiet: bool,
}

impl Cli {
    /// 解析命令行参数
    pub fn parse_args() -> Self {
        Self::parse()
    }
}