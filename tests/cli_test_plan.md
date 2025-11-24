/// CLI模块测试计划
///
/// 1. 基本功能测试
///    - test_cli_parse_args: 测试命令行参数解析
///    - test_cli_parse_test_file: 测试测试文件参数解析
///    - test_cli_parse_include_path: 测试包含路径参数解析
///
/// 2. 选项组合测试
///    - test_cli_parse_multiple_options: 测试多个选项的组合
///    - test_cli_parse_conflicting_options: 测试冲突选项的处理
///    - test_cli_parse_verbose_quiet: 测试详细模式和静默模式的组合
///
/// 3. 错误处理测试
///    - test_cli_parse_invalid_args: 测试无效参数的错误处理
///    - test_cli_parse_missing_required_args: 测试缺少必要参数的错误处理
///    - test_cli_parse_unknown_options: 测试未知选项的错误处理
///
/// 4. 默认值测试
///    - test_cli_default_values: 测试默认参数值
///    - test_cli_overridden_defaults: 测试覆盖默认值
///
/// 5. 边界情况测试
///    - test_cli_parse_empty_args: 测试空参数的处理
///    - test_cli_parse_long_paths: 测试长路径参数的处理
///    - test_cli_parse_special_characters: 测试特殊字符参数的处理