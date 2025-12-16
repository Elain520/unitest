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

    /// 生成包含初始化指令的汇编文件内容
    pub fn generate_with_init_instructions(&self) -> String {
        let mut result = String::new();

        // 添加CONFIG块（但不包含RegInit）
        result.push_str("%ifdef CONFIG\n");

        // 创建不包含RegInit的配置
        let mut result_config = self.config.clone();
        result_config.reg_init = None;

        // 序列化配置为JSON
        if let Ok(config_json) = serde_json::to_string_pretty(&result_config) {
            result.push_str(&config_json);
        }

        result.push_str("\n%endif\n\n");

        // 如果有RegInit配置，生成初始化指令
        if let Some(ref reg_init) = self.config.reg_init {
            eprintln!("生成初始化指令: {:?}", reg_init);
            // 生成通用寄存器初始化指令
            if let Some(ref rax) = reg_init.rax {
                result.push_str(&format!("mov rax, {}\n", rax));
            }
            if let Some(ref rcx) = reg_init.rcx {
                result.push_str(&format!("mov rcx, {}\n", rcx));
            }
            if let Some(ref rdx) = reg_init.rdx {
                result.push_str(&format!("mov rdx, {}\n", rdx));
            }
            if let Some(ref rbx) = reg_init.rbx {
                result.push_str(&format!("mov rbx, {}\n", rbx));
            }
            if let Some(ref rsp) = reg_init.rsp {
                result.push_str(&format!("mov rsp, {}\n", rsp));
            }
            if let Some(ref rbp) = reg_init.rbp {
                result.push_str(&format!("mov rbp, {}\n", rbp));
            }
            if let Some(ref rsi) = reg_init.rsi {
                result.push_str(&format!("mov rsi, {}\n", rsi));
            }
            if let Some(ref rdi) = reg_init.rdi {
                result.push_str(&format!("mov rdi, {}\n", rdi));
            }
            if let Some(ref r8) = reg_init.r8 {
                result.push_str(&format!("mov r8, {}\n", r8));
            }
            if let Some(ref r9) = reg_init.r9 {
                result.push_str(&format!("mov r9, {}\n", r9));
            }
            if let Some(ref r10) = reg_init.r10 {
                result.push_str(&format!("mov r10, {}\n", r10));
            }
            if let Some(ref r11) = reg_init.r11 {
                result.push_str(&format!("mov r11, {}\n", r11));
            }
            if let Some(ref r12) = reg_init.r12 {
                result.push_str(&format!("mov r12, {}\n", r12));
            }
            if let Some(ref r13) = reg_init.r13 {
                result.push_str(&format!("mov r13, {}\n", r13));
            }
            if let Some(ref r14) = reg_init.r14 {
                result.push_str(&format!("mov r14, {}\n", r14));
            }
            if let Some(ref r15) = reg_init.r15 {
                result.push_str(&format!("mov r15, {}\n", r15));
            }

            // 生成XMM/YMM寄存器初始化指令
            // 首先初始化XMM寄存器，使用rax作为临时寄存器
            // 注意：按照XMM寄存器编号顺序初始化，避免相互干扰

            // 为避免干扰原始数据，我们先保存rax的值（如果需要的话）
            let has_general_reg_init = reg_init.rax.is_some() || reg_init.rcx.is_some() ||
                reg_init.rdx.is_some() || reg_init.rbx.is_some() || reg_init.rsp.is_some() ||
                reg_init.rbp.is_some() || reg_init.rsi.is_some() || reg_init.rdi.is_some() ||
                reg_init.r8.is_some() || reg_init.r9.is_some() || reg_init.r10.is_some() ||
                reg_init.r11.is_some() || reg_init.r12.is_some() || reg_init.r13.is_some() ||
                reg_init.r14.is_some() || reg_init.r15.is_some();

            if has_general_reg_init {
                // 如果后面需要设置通用寄存器，先保存rax
                result.push_str("push rax\n");
            }

            // 初始化XMM寄存器
            Self::generate_xmm_init_instructions(&mut result, "xmm0", &reg_init.xmm0);
            Self::generate_xmm_init_instructions(&mut result, "xmm1", &reg_init.xmm1);
            Self::generate_xmm_init_instructions(&mut result, "xmm2", &reg_init.xmm2);
            Self::generate_xmm_init_instructions(&mut result, "xmm3", &reg_init.xmm3);
            Self::generate_xmm_init_instructions(&mut result, "xmm4", &reg_init.xmm4);
            Self::generate_xmm_init_instructions(&mut result, "xmm5", &reg_init.xmm5);
            Self::generate_xmm_init_instructions(&mut result, "xmm6", &reg_init.xmm6);
            Self::generate_xmm_init_instructions(&mut result, "xmm7", &reg_init.xmm7);
            Self::generate_xmm_init_instructions(&mut result, "xmm8", &reg_init.xmm8);
            Self::generate_xmm_init_instructions(&mut result, "xmm9", &reg_init.xmm9);
            Self::generate_xmm_init_instructions(&mut result, "xmm10", &reg_init.xmm10);
            Self::generate_xmm_init_instructions(&mut result, "xmm11", &reg_init.xmm11);
            Self::generate_xmm_init_instructions(&mut result, "xmm12", &reg_init.xmm12);
            Self::generate_xmm_init_instructions(&mut result, "xmm13", &reg_init.xmm13);
            Self::generate_xmm_init_instructions(&mut result, "xmm14", &reg_init.xmm14);
            Self::generate_xmm_init_instructions(&mut result, "xmm15", &reg_init.xmm15);

            if has_general_reg_init {
                // 恢复rax的值
                result.push_str("pop rax\n");
            }
        } else {
            eprintln!("没有RegInit配置");
        }

        // 添加原始汇编代码
        result.push_str(&self.assembly_code);

        // 确保代码以hlt指令结束
        if !self.assembly_code.trim().ends_with("hlt") && !self.assembly_code.trim().ends_with("hlt\n") {
            result.push_str("\nhlt\n");
        }

        result
    }

    /// 生成XMM寄存器初始化指令
    fn generate_xmm_init_instructions(result: &mut String, reg_name: &str, values: &Option<Vec<String>>) {
        if let Some(ref xmm_values) = values {
            if xmm_values.is_empty() {
                return;
            }

            if xmm_values.len() >= 4 {
                // 4个值，使用YMM寄存器
                let ymm_name = reg_name.replacen("xmm", "ymm", 1);
                result.push_str(&format!("; 初始化{}寄存器 (YMM version)\n", reg_name.to_uppercase()));

                // 加载前两个64位值到低128位
                result.push_str(&format!("mov rax, {}\n", xmm_values[0]));
                result.push_str(&format!("movq {}, rax\n", reg_name));
                result.push_str(&format!("mov rax, {}\n", xmm_values[1]));
                result.push_str("movq xmm15, rax\n");  // 使用xmm15作为临时寄存器
                result.push_str(&format!("movhpd {}, xmm15\n", reg_name));

                // 加载后两个64位值到高128位
                result.push_str(&format!("mov rax, {}\n", xmm_values[2]));
                result.push_str("movq xmm15, rax\n");  // 使用xmm15作为临时寄存器
                result.push_str("movq xmm14, rax\n");  // 使用xmm14作为临时寄存器
                result.push_str(&format!("mov rax, {}\n", xmm_values[3]));
                result.push_str("movq xmm13, rax\n");  // 使用xmm13作为临时寄存器
                result.push_str("movhpd xmm14, xmm13\n");
                result.push_str(&format!("vinserti128 {}, {}, xmm14, 1\n", ymm_name, ymm_name));
            } else if xmm_values.len() >= 2 {
                // 2个值，使用XMM寄存器
                result.push_str(&format!("; 初始化{}寄存器\n", reg_name.to_uppercase()));
                result.push_str(&format!("mov rax, {}\n", xmm_values[0]));
                result.push_str(&format!("movq {}, rax\n", reg_name));
                result.push_str(&format!("mov rax, {}\n", xmm_values[1]));
                result.push_str("movq xmm15, rax\n");  // 使用xmm15作为临时寄存器
                result.push_str(&format!("movhpd {}, xmm15\n", reg_name));
            } else {
                // 1个值，使用XMM寄存器
                result.push_str(&format!("; 初始化{}寄存器\n", reg_name.to_uppercase()));
                result.push_str(&format!("mov rax, {}\n", xmm_values[0]));
                result.push_str(&format!("movq {}, rax\n", reg_name));
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_with_init_instructions() {
        let mut config = AsmTestConfig::new();
        let mut reg_init = RegisterData::new();
        reg_init.rax = Some("0x123456789ABCDEF0".to_string());
        reg_init.rbx = Some("0xFEDCBA9876543210".to_string());
        config.reg_init = Some(reg_init);

        let asm_test_file = AsmTestFile {
            config,
            assembly_code: "add rax, rbx\n".to_string(),
        };

        let result = asm_test_file.generate_with_init_instructions();
        println!("Generated result:\n{}", result);

        // 验证结果中包含初始化指令
        assert!(result.contains("mov rax, 0x123456789ABCDEF0"));
        assert!(result.contains("mov rbx, 0xFEDCBA9876543210"));
        assert!(result.contains("add rax, rbx"));
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