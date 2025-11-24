/// Parser模块测试计划
/// 
/// 1. 基本功能测试
///    - test_parse_simple_asm_file: 测试包含简单配置的汇编文件解析
///    - test_parse_asm_file_without_config: 测试不包含配置的汇编文件解析
/// 
/// 2. RegData测试
///    - test_parse_asm_file_with_reg_data: 测试包含RegData字段的配置解析
///    - test_parse_asm_file_with_reg_init: 测试包含RegInit字段的配置解析
/// 
/// 3. Memory测试
///    - test_parse_asm_file_with_memory_regions: 测试包含MemoryRegions字段的配置解析
///    - test_parse_asm_file_with_memory_data: 测试包含MemoryData字段的配置解析
/// 
/// 4. Mode测试
///    - test_parse_asm_file_with_32bit_mode: 测试32位模式配置解析
///    - test_parse_asm_file_with_64bit_mode: 测试64位模式配置解析（默认）
/// 
/// 5. 复杂配置测试
///    - test_parse_asm_file_with_complex_config: 测试包含所有字段的复杂配置解析
/// 
/// 6. 错误处理测试
///    - test_parse_asm_file_with_invalid_json: 测试无效JSON配置的错误处理
///    - test_parse_asm_file_with_malformed_config: 测试格式错误的配置块处理
/// 
/// 7. 边界情况测试
///    - test_parse_asm_file_with_empty_config: 测试空配置块的处理
///    - test_parse_asm_file_with_whitespace: 测试包含大量空白字符的配置处理