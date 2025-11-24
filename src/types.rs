use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 汇编测试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsmTestConfig {
    /// 寄存器数据（在输入文件中应忽略，在输出文件中包含执行后的寄存器状态）
    #[serde(rename = "RegData", skip_serializing_if = "Option::is_none")]
    pub reg_data: Option<RegisterData>,

    /// 寄存器初始值（在输入文件中指定执行前的寄存器初始状态）
    #[serde(rename = "RegInit", skip_serializing_if = "Option::is_none")]
    pub reg_init: Option<RegisterData>,

    /// 执行模式
    #[serde(rename = "Mode", skip_serializing_if = "Option::is_none")]
    pub mode: Option<ExecutionMode>,

    /// 内存区域配置
    #[serde(rename = "MemoryRegions", skip_serializing_if = "Option::is_none")]
    pub memory_regions: Option<HashMap<String, MemorySize>>,

    /// 内存数据配置
    #[serde(rename = "MemoryData", skip_serializing_if = "Option::is_none")]
    pub memory_data: Option<HashMap<String, String>>,
}

/// 寄存器数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterData {
    /// 通用寄存器
    #[serde(rename = "RAX", skip_serializing_if = "Option::is_none")]
    pub rax: Option<String>,
    #[serde(rename = "RCX", skip_serializing_if = "Option::is_none")]
    pub rcx: Option<String>,
    #[serde(rename = "RDX", skip_serializing_if = "Option::is_none")]
    pub rdx: Option<String>,
    #[serde(rename = "RBX", skip_serializing_if = "Option::is_none")]
    pub rbx: Option<String>,
    #[serde(rename = "RSP", skip_serializing_if = "Option::is_none")]
    pub rsp: Option<String>,
    #[serde(rename = "RBP", skip_serializing_if = "Option::is_none")]
    pub rbp: Option<String>,
    #[serde(rename = "RSI", skip_serializing_if = "Option::is_none")]
    pub rsi: Option<String>,
    #[serde(rename = "RDI", skip_serializing_if = "Option::is_none")]
    pub rdi: Option<String>,
    #[serde(rename = "RIP", skip_serializing_if = "Option::is_none")]
    pub rip: Option<String>,
    #[serde(rename = "R8", skip_serializing_if = "Option::is_none")]
    pub r8: Option<String>,
    #[serde(rename = "R9", skip_serializing_if = "Option::is_none")]
    pub r9: Option<String>,
    #[serde(rename = "R10", skip_serializing_if = "Option::is_none")]
    pub r10: Option<String>,
    #[serde(rename = "R11", skip_serializing_if = "Option::is_none")]
    pub r11: Option<String>,
    #[serde(rename = "R12", skip_serializing_if = "Option::is_none")]
    pub r12: Option<String>,
    #[serde(rename = "R13", skip_serializing_if = "Option::is_none")]
    pub r13: Option<String>,
    #[serde(rename = "R14", skip_serializing_if = "Option::is_none")]
    pub r14: Option<String>,
    #[serde(rename = "R15", skip_serializing_if = "Option::is_none")]
    pub r15: Option<String>,

    /// XMM寄存器
    #[serde(rename = "XMM0", skip_serializing_if = "Option::is_none")]
    pub xmm0: Option<Vec<String>>,
    #[serde(rename = "XMM1", skip_serializing_if = "Option::is_none")]
    pub xmm1: Option<Vec<String>>,
    #[serde(rename = "XMM2", skip_serializing_if = "Option::is_none")]
    pub xmm2: Option<Vec<String>>,
    #[serde(rename = "XMM3", skip_serializing_if = "Option::is_none")]
    pub xmm3: Option<Vec<String>>,
    #[serde(rename = "XMM4", skip_serializing_if = "Option::is_none")]
    pub xmm4: Option<Vec<String>>,
    #[serde(rename = "XMM5", skip_serializing_if = "Option::is_none")]
    pub xmm5: Option<Vec<String>>,
    #[serde(rename = "XMM6", skip_serializing_if = "Option::is_none")]
    pub xmm6: Option<Vec<String>>,
    #[serde(rename = "XMM7", skip_serializing_if = "Option::is_none")]
    pub xmm7: Option<Vec<String>>,
    #[serde(rename = "XMM8", skip_serializing_if = "Option::is_none")]
    pub xmm8: Option<Vec<String>>,
    #[serde(rename = "XMM9", skip_serializing_if = "Option::is_none")]
    pub xmm9: Option<Vec<String>>,
    #[serde(rename = "XMM10", skip_serializing_if = "Option::is_none")]
    pub xmm10: Option<Vec<String>>,
    #[serde(rename = "XMM11", skip_serializing_if = "Option::is_none")]
    pub xmm11: Option<Vec<String>>,
    #[serde(rename = "XMM12", skip_serializing_if = "Option::is_none")]
    pub xmm12: Option<Vec<String>>,
    #[serde(rename = "XMM13", skip_serializing_if = "Option::is_none")]
    pub xmm13: Option<Vec<String>>,
    #[serde(rename = "XMM14", skip_serializing_if = "Option::is_none")]
    pub xmm14: Option<Vec<String>>,
    #[serde(rename = "XMM15", skip_serializing_if = "Option::is_none")]
    pub xmm15: Option<Vec<String>>,

    /// 标志寄存器
    #[serde(rename = "Flags", skip_serializing_if = "Option::is_none")]
    pub flags: Option<String>,
}

/// 执行模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    #[serde(rename = "32BIT")]
    Bit32,
}

/// 内存大小
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MemorySize {
    Number(u64),
    HexString(String),
}

/// 汇编测试文件
#[derive(Debug, Clone)]
pub struct AsmTestFile {
    /// 配置部分
    pub config: AsmTestConfig,
    /// 汇编代码部分
    pub assembly_code: String,
}

/// 执行结果
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// 执行后的寄存器状态
    pub register_data: RegisterData,
    /// 执行是否成功
    pub success: bool,
    /// 错误信息（如果有的话）
    pub error_message: Option<String>,
}

impl AsmTestFile {
    /// 生成包含执行结果的汇编文件内容
    pub fn generate_result_file(&self, register_data: &RegisterData) -> String {
        let mut result = String::new();

        // 添加CONFIG块
        result.push_str("%ifdef CONFIG\n");

        // 创建包含结果的配置
        let mut result_config = self.config.clone();
        result_config.reg_data = Some(register_data.clone());

        // 序列化配置为JSON
        if let Ok(config_json) = serde_json::to_string_pretty(&result_config) {
            result.push_str(&config_json);
        }

        result.push_str("\n%endif\n\n");

        // 添加原始汇编代码
        result.push_str(&self.assembly_code);

        // 确保代码以hlt指令结束
        if !self.assembly_code.trim().ends_with("hlt") && !self.assembly_code.trim().ends_with("hlt\n") {
            result.push_str("\nhlt\n");
        }

        result
    }
}

impl AsmTestConfig {
    /// 创建新的配置实例
    pub fn new() -> Self {
        AsmTestConfig {
            reg_data: None,
            reg_init: None,
            mode: None,
            memory_regions: None,
            memory_data: None,
        }
    }
}

impl Default for AsmTestConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterData {
    /// 创建新的寄存器数据实例
    pub fn new() -> Self {
        RegisterData {
            rax: None,
            rcx: None,
            rdx: None,
            rbx: None,
            rsp: None,
            rbp: None,
            rsi: None,
            rdi: None,
            rip: None,
            r8: None,
            r9: None,
            r10: None,
            r11: None,
            r12: None,
            r13: None,
            r14: None,
            r15: None,
            xmm0: None,
            xmm1: None,
            xmm2: None,
            xmm3: None,
            xmm4: None,
            xmm5: None,
            xmm6: None,
            xmm7: None,
            xmm8: None,
            xmm9: None,
            xmm10: None,
            xmm11: None,
            xmm12: None,
            xmm13: None,
            xmm14: None,
            xmm15: None,
            flags: None,
        }
    }
}

impl Default for RegisterData {
    fn default() -> Self {
        Self::new()
    }
}

/// XMM寄存器数据
#[derive(Debug)]
pub struct XmmRegisters {
    pub xmm0: Option<Vec<String>>,
    pub xmm1: Option<Vec<String>>,
    pub xmm2: Option<Vec<String>>,
    pub xmm3: Option<Vec<String>>,
    pub xmm4: Option<Vec<String>>,
    pub xmm5: Option<Vec<String>>,
    pub xmm6: Option<Vec<String>>,
    pub xmm7: Option<Vec<String>>,
    pub xmm8: Option<Vec<String>>,
    pub xmm9: Option<Vec<String>>,
    pub xmm10: Option<Vec<String>>,
    pub xmm11: Option<Vec<String>>,
    pub xmm12: Option<Vec<String>>,
    pub xmm13: Option<Vec<String>>,
    pub xmm14: Option<Vec<String>>,
    pub xmm15: Option<Vec<String>>,
}