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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_values() {
        let cli = Cli::parse_from(["test"]);
        assert_eq!(cli.test_file, None);
        assert_eq!(cli.include_path, None);
        assert_eq!(cli.output_file, None);
        assert_eq!(cli.verbose, 0);
        assert_eq!(cli.quiet, false);
    }

    #[test]
    fn test_cli_with_test_file() {
        let cli = Cli::parse_from(["test", "--test", "test.asm"]);
        assert_eq!(cli.test_file, Some("test.asm".to_string()));
        assert_eq!(cli.include_path, None);
        assert_eq!(cli.output_file, None);
        assert_eq!(cli.verbose, 0);
        assert_eq!(cli.quiet, false);
    }

    #[test]
    fn test_cli_with_include_path() {
        let cli = Cli::parse_from(["test", "--include", "/path/to/include"]);
        assert_eq!(cli.test_file, None);
        assert_eq!(cli.include_path, Some("/path/to/include".to_string()));
        assert_eq!(cli.output_file, None);
        assert_eq!(cli.verbose, 0);
        assert_eq!(cli.quiet, false);
    }

    #[test]
    fn test_cli_with_output_file() {
        let cli = Cli::parse_from(["test", "--output", "result.asm"]);
        assert_eq!(cli.test_file, None);
        assert_eq!(cli.include_path, None);
        assert_eq!(cli.output_file, Some("result.asm".to_string()));
        assert_eq!(cli.verbose, 0);
        assert_eq!(cli.quiet, false);
    }

    #[test]
    fn test_cli_with_verbose_flag() {
        let cli = Cli::parse_from(["test", "--verbose"]);
        assert_eq!(cli.test_file, None);
        assert_eq!(cli.include_path, None);
        assert_eq!(cli.output_file, None);
        assert_eq!(cli.verbose, 1);
        assert_eq!(cli.quiet, false);
    }

    #[test]
    fn test_cli_with_multiple_verbose_flags() {
        let cli = Cli::parse_from(["test", "-vv"]);
        assert_eq!(cli.test_file, None);
        assert_eq!(cli.include_path, None);
        assert_eq!(cli.output_file, None);
        assert_eq!(cli.verbose, 2);
        assert_eq!(cli.quiet, false);
    }

    #[test]
    fn test_cli_with_quiet_flag() {
        let cli = Cli::parse_from(["test", "--quiet"]);
        assert_eq!(cli.test_file, None);
        assert_eq!(cli.include_path, None);
        assert_eq!(cli.output_file, None);
        assert_eq!(cli.verbose, 0);
        assert_eq!(cli.quiet, true);
    }

    #[test]
    fn test_cli_with_combined_flags() {
        let cli = Cli::parse_from(["test", "--test", "test.asm", "--include", "/include", "--output", "result.asm", "-v", "-q"]);
        assert_eq!(cli.test_file, Some("test.asm".to_string()));
        assert_eq!(cli.include_path, Some("/include".to_string()));
        assert_eq!(cli.output_file, Some("result.asm".to_string()));
        assert_eq!(cli.verbose, 1);
        assert_eq!(cli.quiet, true);
    }

    #[test]
    fn test_cli_with_short_flags() {
        let cli = Cli::parse_from(["test", "-t", "test.asm", "-i", "/include", "-o", "result.asm"]);
        assert_eq!(cli.test_file, Some("test.asm".to_string()));
        assert_eq!(cli.include_path, Some("/include".to_string()));
        assert_eq!(cli.output_file, Some("result.asm".to_string()));
        assert_eq!(cli.verbose, 0);
        assert_eq!(cli.quiet, false);
    }
}