//! 执行器模块
//!
//! 负责在原生x86环境下执行编译后的代码，使用父子进程协作模型和ptrace系统调用

use crate::elf::ElfInfo;
use crate::error::{AsmTestError, Result};
use libc::{c_void, fork, iovec, kill, mmap, munmap, pid_t, ptrace, raise, user_regs_struct, waitpid, MAP_ANONYMOUS, MAP_FIXED, MAP_FIXED_NOREPLACE, MAP_PRIVATE, PROT_EXEC, PROT_READ, PROT_WRITE, PTRACE_CONT, PTRACE_GETREGS, PTRACE_GETREGSET, PTRACE_SETREGS, PTRACE_SETREGSET, PTRACE_TRACEME, SIGSTOP, SIGTRAP, WIFSTOPPED, WSTOPSIG};
use crate::types::{AsmTestConfig, ExecutionMode, MemorySize, RegisterData, XmmRegisters};

/// 执行结果
#[derive(Debug)]
pub struct ExecuteResult {
    /// 执行是否成功
    pub success: bool,
    /// 执行后的寄存器状态
    pub register_data: Option<RegisterData>,
    /// 错误信息（如果有的话）
    pub error_message: Option<String>,
}
/// x86 XSTATE寄存器集类型
const NT_X86_XSTATE: i32 = 0x202;

/// 在原生x86环境下执行ELF文件
pub fn execute_elf_file(elf_info: &ElfInfo, config: &AsmTestConfig) -> Result<ExecuteResult> {
    // 创建父子进程
    // 父进程使用ptrace控制子进程执行
    // 子进程负责执行代码

    unsafe {
        let pid = fork();
        if pid == 0 {
            // 子进程
            if let Err(e) = execute_in_child_process(elf_info, config) {
                eprintln!("[child] 子进程执行错误: {}", e);
                std::process::exit(1);
            }
            // 子进程不应该到达这里
            std::process::exit(0);
        } else if pid > 0 {
            // 父进程
            return execute_in_parent_process(pid, elf_info, config);
        } else {
            return Err(AsmTestError::Execution("[parent] 创建子进程失败".to_string()));
        }
    }
}

/// 在子进程中执行代码
unsafe fn execute_in_child_process(elf_info: &ElfInfo, _config: &AsmTestConfig) -> Result<()> {
    // 子进程逻辑
    // 1. 允许父进程通过ptrace控制
    // 2. 分配固定地址内存
    // 3. 加载代码到内存
    // 4. 执行代码（第一个int3指令会触发SIGTRAP让父进程有机会附加）

    // 允许父进程通过ptrace控制
    let result = ptrace(PTRACE_TRACEME, 0, std::ptr::null_mut::<c_void>(), std::ptr::null_mut::<c_void>());
    if result == -1 {
        return Err(AsmTestError::Execution("[child] 无法设置PTRACE_TRACEME".to_string()));
    }

    // 发送SIGSTOP信号让自己停止，等待父进程附加
    raise(SIGSTOP);

    if let Err(e) = validate_memory_data_addresses(_config) {
        return Err(e);
    }

    // 分配代码内存
    let code_address = 0xC0000000u64;
    let code_size = 4096usize;
    let code_memory = allocate_fixed_memory_rwx(code_address, code_size)?;

    // 分配栈内存
    let stack_address = 0xE0000000u64;
    let stack_size = 16 * 4096usize;
    let _stack_memory = allocate_fixed_memory_rw(stack_address, stack_size)?;


    // 根据MemoryRegions配置分配内存区域
    let mut allocated_memory_regions = Vec::new();
    if let Some(ref memory_regions) = _config.memory_regions {
        for (address_str, size) in memory_regions {
            match parse_hex_address(address_str) {
                Ok(address) => {
                    let size_value = match size {
                        MemorySize::Number(n) => *n as usize,
                        MemorySize::HexString(hex_str) => {
                            match parse_hex_address(hex_str) {
                                Ok(val) => val as usize,
                                Err(_) => {
                                    eprintln!("[child] 无法解析内存区域大小: {}", hex_str);
                                    continue;
                                }
                            }
                        }
                    };

                    match allocate_fixed_memory_rw(address, size_value) {
                        Ok(memory) => {
                            allocated_memory_regions.push((address, size_value, memory));
                            if cfg!(debug_assertions) {
                                eprintln!("[child] 分配内存区域: 地址=0x{:x}, 大小={}", address, size_value);
                            }
                        }
                        Err(e) => {
                            eprintln!("[child] 无法分配内存区域 0x{:x}: {}", address, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[child] 无法解析内存区域地址 {}: {}", address_str, e);
                }
            }
        }
    }

    // 根据MemoryData配置初始化内存数据
    if let Some(ref memory_data) = _config.memory_data {
        for (address_str, hex_data) in memory_data {
            match parse_hex_address(address_str) {
                Ok(address) => {
                    // 查找对应的内存区域
                    let mut found_region = None;
                    for &(region_address, region_size, region_memory) in &allocated_memory_regions {
                        if address >= region_address && address < region_address + region_size as u64 {
                            found_region = Some((region_address, region_size, region_memory));
                            break;
                        }
                    }

                    if let Some((region_address, region_size, region_memory)) = found_region {
                        // 计算在区域内的偏移
                        let offset = (address - region_address) as usize;
                        let region_ptr = (region_memory as *mut u8).add(offset);

                        // 解析十六进制字符串数据
                        match parse_hex_string_detailed(hex_data) {
                            Ok(byte_data) => {
                                // 检查是否有足够的空间
                                if offset + byte_data.len() > region_size {
                                    eprintln!("[child] 内存数据超出区域边界: 地址={}, 数据长度={}", address_str, byte_data.len());
                                } else {
                                    // 写入字节数据
                                    for (i, byte) in byte_data.iter().enumerate() {
                                        *region_ptr.add(i) = *byte;
                                    }

                                    if cfg!(debug_assertions) {
                                        eprintln!("[child] 初始化内存数据: 地址={}, 数据长度={}", address_str, byte_data.len());
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("[child] 无法解析内存数据 {}: {}", hex_data, e);
                            }
                        }

                        if cfg!(debug_assertions) {
                            // 显示写入数据后的内存内容
                            dump_memory_region(region_memory, region_address, region_size, 64); // 显示前64字节
                        }
                    } else {
                        eprintln!("[child] 未找到对应的内存区域: {}", address_str);
                    }
                }
                Err(e) => {
                    eprintln!("[child] 无法解析内存数据地址 {}: {}", address_str, e);
                }
            }
        }
    }

    // 如果有代码段，将代码加载到内存中
    if let Some(ref code_section) = elf_info.code_section {
        if code_section.size + 2 <= code_size {
            // 在代码开始处插入int3断点，用于让父进程有机会附加
            let first_byte = code_memory as *mut u8;
            *first_byte = 0xCC; // int3指令

            // 将代码加载到内存中（从第二个字节开始）
            std::ptr::copy_nonoverlapping(code_section.data.as_ptr(), (code_memory as *mut u8).add(1), code_section.size);

            let end_byte = (code_memory as *mut u8).add(1 + code_section.size);
            *end_byte = 0xCC; // int3指令
            // 打印代码段内容用于调试
            if cfg!(debug_assertions) {
                eprintln!("[child] 代码段大小: {}", code_section.size);
                eprintln!("[child] 代码段内容 (hex):");
                for i in 0..code_section.size + 2 {
                    eprint!("{:02x} ", *((code_memory as *mut u8).add(i)));
                }
                eprintln!();
            }
        }
    }

    // 修改内存保护为可执行
    if cfg!(debug_assertions) {
        eprintln!("[child] 修改内存保护为可执行");
    }
    let result = libc::mprotect(code_memory, code_size, PROT_READ | PROT_EXEC);
    if result == -1 {
        if cfg!(debug_assertions) {
            eprintln!("[child] mprotect调用失败");
        }
        return Err(AsmTestError::Execution("[child] 无法设置代码内存为可执行".to_string()));
    }
    if cfg!(debug_assertions) {
        eprintln!("[child] mprotect调用成功");
    }

    // 直接跳转到代码开始处执行
    // 第一个int3指令会触发SIGTRAP，让父进程有机会附加
    let func: unsafe extern "C" fn() = std::mem::transmute(code_memory as *mut u8);
    func();


    // 不应该到达这里
    Ok(())
}

fn parse_hex_string_detailed(hex_str: &str) -> Result<Vec<u8>> {
    let mut byte_data = Vec::new();

    // 按空格分割处理多个数字
    for num_part in hex_str.split_whitespace() {
        let clean_part = num_part.trim();
        if clean_part.is_empty() {
            continue;
        }

        // 处理"0x"前缀
        let mut num_str = if clean_part.starts_with("0x") || clean_part.starts_with("0X") {
            clean_part[2..].to_string()
        } else {
            clean_part.to_string()
        };

        // 从右到左每两个字符一组解析为字节
        while !num_str.is_empty() {
            let byte_str = if num_str.len() >= 2 {
                // 取最后两个字符
                let len = num_str.len();
                let byte_str = num_str[len - 2..].to_string();
                num_str = num_str[..len - 2].to_string();
                byte_str
            } else {
                // 只剩一个字符，前面补0
                let byte_str = format!("0{}", num_str);
                num_str.clear();
                byte_str
            };

            match u8::from_str_radix(&byte_str, 16) {
                Ok(byte) => byte_data.push(byte),
                Err(e) => return Err(AsmTestError::Execution(format!("无法解析十六进制字节 {}: {}", byte_str, e))),
            }
        }
    }

    Ok(byte_data)
}

/// 验证MemoryData地址是否在合法的内存区域中
fn validate_memory_data_addresses(config: &AsmTestConfig) -> Result<()> {
    if let Some(ref memory_data) = config.memory_data {
        // 收集所有合法的内存区域地址范围
        let mut valid_regions = Vec::new();

        // 添加栈内存区域 (0xE0000000, 16 * 4096)
        let stack_start = 0xE0000000u64;
        let stack_size = 16 * 4096usize;
        valid_regions.push((stack_start, stack_start + stack_size as u64));

        // 添加代码内存区域 (0xC0000000, 4096)
        let code_start = 0xC0000000u64;
        let code_size = 4096usize;
        valid_regions.push((code_start, code_start + code_size as u64));

        // 添加配置中定义的内存区域
        if let Some(ref memory_regions) = config.memory_regions {
            for (address_str, size) in memory_regions {
                match parse_hex_address(address_str) {
                    Ok(start_address) => {
                        let size_value = match size {
                            MemorySize::Number(n) => *n,
                            MemorySize::HexString(hex_str) => {
                                match parse_hex_address(hex_str) {
                                    Ok(val) => val,
                                    Err(_) => {
                                        return Err(AsmTestError::Execution(format!("无法解析内存区域大小: {}", hex_str)));
                                    }
                                }
                            }
                        };
                        let end_address = start_address + size_value;
                        valid_regions.push((start_address, end_address));
                    }
                    Err(e) => {
                        return Err(AsmTestError::Execution(format!("无法解析内存区域地址 {}: {}", address_str, e)));
                    }
                }
            }
        }

        // 验证每个MemoryData地址是否在合法区域中
        for (address_str, _data) in memory_data {
            match parse_hex_address(address_str) {
                Ok(address) => {
                    let mut is_valid = false;
                    for &(start, end) in &valid_regions {
                        if address >= start && address < end {
                            is_valid = true;
                            break;
                        }
                    }

                    if !is_valid {
                        return Err(AsmTestError::Execution(format!(
                            "内存数据地址 {} 不在任何合法的内存区域中。合法区域: {:?}",
                            address_str, valid_regions
                        )));
                    }
                }
                Err(e) => {
                    return Err(AsmTestError::Execution(format!("无法解析内存数据地址 {}: {}", address_str, e)));
                }
            }
        }
    }

    Ok(())
}

/// 解析十六进制地址字符串
fn parse_hex_address(address_str: &str) -> Result<u64> {
    let trimmed = address_str.trim();
    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        u64::from_str_radix(&trimmed[2..], 16)
            .map_err(|e| AsmTestError::Execution(format!("无法解析十六进制地址 {}: {}", address_str, e)))
    } else {
        trimmed.parse::<u64>()
            .map_err(|e| AsmTestError::Execution(format!("无法解析地址 {}: {}", address_str, e)))
    }
}

/// 解析十六进制值字符串
fn parse_hex_value(hex_str: &str) -> Result<u64> {
    let trimmed = hex_str.trim();
    if trimmed.is_empty() {
        return Ok(0); // 空字符串返回0
    }

    // 处理"0x"前缀
    let clean_str = if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        &trimmed[2..]
    } else {
        trimmed
    };

    // 解析十六进制字符串
    u64::from_str_radix(clean_str, 16)
        .map_err(|e| AsmTestError::Execution(format!("无法解析十六进制值 {}: {}", hex_str, e)))
}

unsafe fn dump_memory_region(memory: *mut c_void, address: u64, size: usize, max_bytes: usize) {
    if !cfg!(debug_assertions) {
        return;
    }

    eprintln!("[child] 内存区域内容: 地址=0x{:x}, 大小={}", address, size);
    let ptr = memory as *const u8;
    let display_bytes = std::cmp::min(size, max_bytes);

    for i in 0..display_bytes {
        if i % 16 == 0 {
            eprint!("\n[0x{:08x}] ", address + i as u64);
        }
        eprint!("{:02x} ", *ptr.add(i));
    }
    eprintln!();

    if display_bytes < size {
        eprintln!("... (显示前{}字节，总共{}字节)", display_bytes, size);
    }
}

/// 在父进程中控制子进程执行
fn execute_in_parent_process(pid: pid_t, _elf_info: &ElfInfo, _config: &AsmTestConfig) -> Result<ExecuteResult> {
    // 父进程逻辑
    // 1. 等待子进程触发SIGSTOP信号
    // 2. 子进程已经设置了PTRACE_TRACEME，所以可以直接控制
    // 3. 继续执行直到遇到第一个SIGTRAP（代码开始处的int3）
    // 4. 设置初始寄存器状态
    // 5. 继续执行直到遇到第二个SIGTRAP（代码结束处的int3）
    // 6. 获取最终寄存器状态

    // 等待子进程触发SIGSTOP信号
    if cfg!(debug_assertions) {
        eprintln!("[parent] 等待子进程触发SIGSTOP信号");
    }
    let mut status: i32 = 0;
    let result = unsafe { waitpid(pid, &mut status, 0) };
    if result == -1 {
        // 杀死子进程
        unsafe { kill(pid, libc::SIGKILL) };
        return Err(AsmTestError::Execution("[parent] 等待子进程失败".to_string()));
    }

    if cfg!(debug_assertions) {
        eprintln!("[parent] 子进程状态: status={}", status);
        eprintln!("[parent] WIFSTOPPED: {}", WIFSTOPPED(status));
        if WIFSTOPPED(status) {
            eprintln!("[parent] WSTOPSIG: {}", WSTOPSIG(status));
        }
    }

    if !(WIFSTOPPED(status) && WSTOPSIG(status) == SIGSTOP) {
        // 杀死子进程
        unsafe { kill(pid, libc::SIGKILL) };
        return Err(AsmTestError::Execution("[parent] 子进程未在预期的SIGSTOP信号处停止".to_string()));
    }

    // 让子进程继续运行到第一个int3断点
    if cfg!(debug_assertions) {
        eprintln!("[parent] 让子进程继续运行到第一个int3断点");
    }
    let result = unsafe { ptrace(PTRACE_CONT, pid, std::ptr::null_mut::<c_void>(), std::ptr::null_mut::<c_void>()) };
    if result == -1 {
        // 杀死子进程
        unsafe { kill(pid, libc::SIGKILL) };
        return Err(AsmTestError::Execution("[parent] 无法继续执行子进程".to_string()));
    }

    // 等待子进程停止（遇到第一个SIGTRAP）
    if cfg!(debug_assertions) {
        eprintln!("[parent] 等待子进程停止（遇到第一个SIGTRAP）");
    }
    let result = unsafe { waitpid(pid, &mut status, 0) };
    if result == -1 {
        // 杀死子进程
        unsafe { kill(pid, libc::SIGKILL) };
        return Err(AsmTestError::Execution("[parent] 等待子进程失败".to_string()));
    }

    if !(WIFSTOPPED(status) && WSTOPSIG(status) == SIGTRAP) {
        // 杀死子进程
        unsafe { kill(pid, libc::SIGKILL) };
        return Err(AsmTestError::Execution("[parent] 子进程未在起始int3断点处停止".to_string()));
    }

    // 获取子进程寄存器状态
    if cfg!(debug_assertions) {
        eprintln!("[parent] 获取子进程寄存器状态");
    }
    let mut regs: user_regs_struct = unsafe { std::mem::zeroed() };
    let result = unsafe { ptrace(PTRACE_GETREGS, pid, std::ptr::null_mut::<c_void>(), &mut regs as *mut user_regs_struct as *mut c_void) };
    if result == -1 {
        // 杀死子进程
        unsafe { kill(pid, libc::SIGKILL) };
        return Err(AsmTestError::Execution("[parent] 无法获取子进程寄存器状态".to_string()));
    }

    // 设置初始寄存器状态
    if cfg!(debug_assertions) {
        eprintln!("[parent] 设置初始寄存器状态");
    }
    if let Err(e) = set_initial_registers(pid, &mut regs, _config) {
        // 杀死子进程
        unsafe { kill(pid, libc::SIGKILL) };
        return Err(e);
    }

    // 继续执行直到遇到第二个SIGTRAP（代码结束处的int3）
    if cfg!(debug_assertions) {
        eprintln!("[parent] 继续执行直到遇到第二个SIGTRAP（代码结束处的int3）");
    }
    let result = unsafe { ptrace(PTRACE_CONT, pid, std::ptr::null_mut::<c_void>(), std::ptr::null_mut::<c_void>()) };
    if result == -1 {
        // 杀死子进程
        unsafe { kill(pid, libc::SIGKILL) };
        return Err(AsmTestError::Execution("[parent] 无法继续执行子进程".to_string()));
    }

    // 等待子进程停止（遇到第二个SIGTRAP）
    if cfg!(debug_assertions) {
        eprintln!("[parent] 等待子进程停止（遇到第二个SIGTRAP）");
    }
    let result = unsafe { waitpid(pid, &mut status, 0) };
    if result == -1 {
        // 杀死子进程
        unsafe { kill(pid, libc::SIGKILL) };
        return Err(AsmTestError::Execution("[parent] 等待子进程失败".to_string()));
    }

    // 打印调试信息
    if cfg!(debug_assertions) {
        eprintln!("[parent] 子进程状态: status={}", status);
        eprintln!("[parent] WIFSTOPPED: {}", WIFSTOPPED(status));
        if WIFSTOPPED(status) {
            eprintln!("[parent] WSTOPSIG: {}", WSTOPSIG(status));
        }
    }

    if !(WIFSTOPPED(status) && WSTOPSIG(status) == SIGTRAP) {
        // 杀死子进程
        unsafe { kill(pid, libc::SIGKILL) };
        return Err(AsmTestError::Execution("[parent] 子进程未在结束int3断点处停止".to_string()));
    }

    // 获取最终寄存器状态
    let final_registers = match get_registers(pid) {
        Ok(registers) => registers,
        Err(e) => {
            // 杀死子进程
            unsafe { kill(pid, libc::SIGKILL) };
            return Err(e);
        }
    };

    // 杀死子进程
    unsafe { kill(pid, libc::SIGKILL) };

    // 等待子进程退出
    unsafe { waitpid(pid, &mut status, 0) };

    Ok(ExecuteResult {
        success: true,
        register_data: Some(final_registers),
        error_message: None,
    })
}


/// 使用ptrace获取寄存器状态
fn get_registers(pid: pid_t) -> Result<RegisterData> {
    let mut regs: user_regs_struct = unsafe { std::mem::zeroed() };
    let result = unsafe { ptrace(PTRACE_GETREGS, pid, std::ptr::null_mut::<c_void>(), &mut regs as *mut user_regs_struct as *mut c_void) };
    if result == -1 {
        return Err(AsmTestError::Execution("无法获取子进程寄存器状态".to_string()));
    }

    let mut register_data = RegisterData::new();
    register_data.rax = Some(format!("0x{:016x}", regs.rax));
    register_data.rbx = Some(format!("0x{:016x}", regs.rbx));
    register_data.rcx = Some(format!("0x{:016x}", regs.rcx));
    register_data.rdx = Some(format!("0x{:016x}", regs.rdx));
    register_data.rsi = Some(format!("0x{:016x}", regs.rsi));
    register_data.rdi = Some(format!("0x{:016x}", regs.rdi));
    register_data.rbp = Some(format!("0x{:016x}", regs.rbp));
    register_data.rsp = Some(format!("0x{:016x}", regs.rsp));
    register_data.rip = Some(format!("0x{:016x}", regs.rip));
    register_data.r8 = Some(format!("0x{:016x}", regs.r8));
    register_data.r9 = Some(format!("0x{:016x}", regs.r9));
    register_data.r10 = Some(format!("0x{:016x}", regs.r10));
    register_data.r11 = Some(format!("0x{:016x}", regs.r11));
    register_data.r12 = Some(format!("0x{:016x}", regs.r12));
    register_data.r13 = Some(format!("0x{:016x}", regs.r13));
    register_data.r14 = Some(format!("0x{:016x}", regs.r14));
    register_data.r15 = Some(format!("0x{:016x}", regs.r15));
    register_data.flags = Some(format!("0x{:08x}", regs.eflags));

    // 获取XMM寄存器状态
    if let Ok(xmm_registers) = get_xmm_registers(pid) {
        register_data.xmm0 = xmm_registers.xmm0;
        register_data.xmm1 = xmm_registers.xmm1;
        register_data.xmm2 = xmm_registers.xmm2;
        register_data.xmm3 = xmm_registers.xmm3;
        register_data.xmm4 = xmm_registers.xmm4;
        register_data.xmm5 = xmm_registers.xmm5;
        register_data.xmm6 = xmm_registers.xmm6;
        register_data.xmm7 = xmm_registers.xmm7;
        register_data.xmm8 = xmm_registers.xmm8;
        register_data.xmm9 = xmm_registers.xmm9;
        register_data.xmm10 = xmm_registers.xmm10;
        register_data.xmm11 = xmm_registers.xmm11;
        register_data.xmm12 = xmm_registers.xmm12;
        register_data.xmm13 = xmm_registers.xmm13;
        register_data.xmm14 = xmm_registers.xmm14;
        register_data.xmm15 = xmm_registers.xmm15;
    }

    Ok(register_data)
}

// fn format_flags(flags: u64) -> String {
//     let cf = (flags >> 0) & 1;  // 进位标志
//     let pf = (flags >> 2) & 1;  // 奇偶标志
//     let af = (flags >> 4) & 1;  // 辅助进位标志
//     let zf = (flags >> 6) & 1;  // 零标志
//     let sf = (flags >> 7) & 1;  // 符号标志
//     let tf = (flags >> 8) & 1;  // 陷阱标志
//     let if_flag = (flags >> 9) & 1;  // 中断允许标志
//     let df = (flags >> 10) & 1; // 方向标志
//     let of = (flags >> 11) & 1; // 溢出标志
//
//     let flags_desc = format!(
//         "CF:{}(进位) PF:{}(奇偶) AF:{}(辅助进位) ZF:{}(零) SF:{}(符号) TF:{}(陷阱) IF:{}(中断) DF:{}(方向) OF:{}(溢出)",
//         cf, pf, af, zf, sf, tf, if_flag, df, of
//     );
//
//     format!("0x{:08x} [{}]", flags, flags_desc)
// }

/// 使用ptrace获取XMM寄存器状态
fn get_xmm_registers(pid: pid_t) -> Result<XmmRegisters> {
    // 分配足够大的缓冲区来存储XSAVE状态
    let bufsize = 4096;
    let xstate_buffer = unsafe {
        libc::malloc(bufsize)
    };

    if xstate_buffer.is_null() {
        return Err(AsmTestError::Execution("无法分配XSAVE状态缓冲区".to_string()));
    }

    // 确保缓冲区被清零
    unsafe {
        libc::memset(xstate_buffer, 0, bufsize);
    }

    let mut iov = iovec {
        iov_base: xstate_buffer,
        iov_len: bufsize,
    };

    // 获取XSAVE状态
    let result = unsafe {
        ptrace(PTRACE_GETREGSET, pid, NT_X86_XSTATE as *mut c_void, &mut iov as *mut iovec as *mut c_void)
    };

    if result == -1 {
        unsafe { libc::free(xstate_buffer); }
        return Err(AsmTestError::Execution("无法获取XMM寄存器状态".to_string()));
    }

    let mut xmm_registers = XmmRegisters {
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
    };

    // 解析XMM寄存器数据
    // XSAVE格式: XMM寄存器通常在偏移0xA0处开始
    if iov.iov_len >= 0xA0 + 16 * 16 {
        let xmm_base = unsafe { (xstate_buffer as *mut u8).add(0xA0) };

        // 检查是否有AVX状态 (YMM寄存器)
        let has_avx = if iov.iov_len >= 512 + 8 {
            let xstate_hdr = unsafe { (xstate_buffer as *mut u8).add(512) as *const u64 };
            let xstate_bv = unsafe { *xstate_hdr };
            (xstate_bv & (1u64 << 2)) != 0 // AVX状态位 (bit 2)
        } else {
            false
        };

        if has_avx && iov.iov_len >= 0x240 + 16 * 16 {
            // 读取完整的YMM寄存器 (256位 = 32字节)
            let ymmh_base = unsafe { (xstate_buffer as *mut u8).add(0x240) };

            // 解析每个YMM寄存器 (256位 = 32字节)
            for i in 0..16 {
                let xmm_ptr = unsafe { xmm_base.add(i * 16) };
                let ymmh_ptr = unsafe { ymmh_base.add(i * 16) };
                let mut ymm_data = Vec::new();

                // 读取XMM低128位
                for j in 0..2 {
                    let data_ptr = unsafe { (xmm_ptr as *const u64).add(j) };
                    let data = unsafe { *data_ptr };
                    ymm_data.push(format!("0x{:016x}", data));
                }

                // 读取YMM高128位
                for j in 0..2 {
                    let data_ptr = unsafe { (ymmh_ptr as *const u64).add(j) };
                    let data = unsafe { *data_ptr };
                    ymm_data.push(format!("0x{:016x}", data));
                }

                match i {
                    0 => xmm_registers.xmm0 = Some(ymm_data),
                    1 => xmm_registers.xmm1 = Some(ymm_data),
                    2 => xmm_registers.xmm2 = Some(ymm_data),
                    3 => xmm_registers.xmm3 = Some(ymm_data),
                    4 => xmm_registers.xmm4 = Some(ymm_data),
                    5 => xmm_registers.xmm5 = Some(ymm_data),
                    6 => xmm_registers.xmm6 = Some(ymm_data),
                    7 => xmm_registers.xmm7 = Some(ymm_data),
                    8 => xmm_registers.xmm8 = Some(ymm_data),
                    9 => xmm_registers.xmm9 = Some(ymm_data),
                    10 => xmm_registers.xmm10 = Some(ymm_data),
                    11 => xmm_registers.xmm11 = Some(ymm_data),
                    12 => xmm_registers.xmm12 = Some(ymm_data),
                    13 => xmm_registers.xmm13 = Some(ymm_data),
                    14 => xmm_registers.xmm14 = Some(ymm_data),
                    15 => xmm_registers.xmm15 = Some(ymm_data),
                    _ => {}
                }
            }
        } else {
            // 只读取XMM寄存器 (128位 = 16字节)
            for i in 0..16 {
                let xmm_ptr = unsafe { xmm_base.add(i * 16) };
                let mut xmm_data = Vec::new();

                // 读取XMM寄存器 (128位 = 16字节)
                for j in 0..2 {
                    let data_ptr = unsafe { (xmm_ptr as *const u64).add(j) };
                    let data = unsafe { *data_ptr };
                    xmm_data.push(format!("0x{:016x}", data));
                }

                // 填充高128位为0（YMM高部分）
                xmm_data.push(format!("0x{:016x}", 0u64));
                xmm_data.push(format!("0x{:016x}", 0u64));

                match i {
                    0 => xmm_registers.xmm0 = Some(xmm_data),
                    1 => xmm_registers.xmm1 = Some(xmm_data),
                    2 => xmm_registers.xmm2 = Some(xmm_data),
                    3 => xmm_registers.xmm3 = Some(xmm_data),
                    4 => xmm_registers.xmm4 = Some(xmm_data),
                    5 => xmm_registers.xmm5 = Some(xmm_data),
                    6 => xmm_registers.xmm6 = Some(xmm_data),
                    7 => xmm_registers.xmm7 = Some(xmm_data),
                    8 => xmm_registers.xmm8 = Some(xmm_data),
                    9 => xmm_registers.xmm9 = Some(xmm_data),
                    10 => xmm_registers.xmm10 = Some(xmm_data),
                    11 => xmm_registers.xmm11 = Some(xmm_data),
                    12 => xmm_registers.xmm12 = Some(xmm_data),
                    13 => xmm_registers.xmm13 = Some(xmm_data),
                    14 => xmm_registers.xmm14 = Some(xmm_data),
                    15 => xmm_registers.xmm15 = Some(xmm_data),
                    _ => {}
                }
            }
        }
    }

    unsafe { libc::free(xstate_buffer); }
    Ok(xmm_registers)
}

fn reset_xmm_ymm_registers(pid: pid_t) -> Result<()> {
    // 分配足够大的缓冲区来存储XSAVE状态
    let bufsize = 4096;
    let xstate_buffer = unsafe {
        libc::malloc(bufsize)
    };

    if xstate_buffer.is_null() {
        return Err(AsmTestError::Execution("无法分配XSAVE状态缓冲区".to_string()));
    }

    // 确保缓冲区被清零
    unsafe {
        libc::memset(xstate_buffer, 0, bufsize);
    }

    let mut iov = iovec {
        iov_base: xstate_buffer,
        iov_len: bufsize,
    };

    // 获取当前的XSAVE状态
    let result = unsafe {
        ptrace(PTRACE_GETREGSET, pid, NT_X86_XSTATE as *mut c_void, &mut iov as *mut iovec as *mut c_void)
    };

    if result == -1 {
        unsafe { libc::free(xstate_buffer); }
        return Err(AsmTestError::Execution("无法获取当前XMM寄存器状态".to_string()));
    }

    // 初始化XMM寄存器为零
    // XSAVE格式: XMM寄存器通常在偏移0xA0处开始
    if iov.iov_len >= 0xA0 + 16 * 16 {
        let xmm_base = unsafe { (xstate_buffer as *mut u8).add(0xA0) };

        // 将所有XMM寄存器清零
        unsafe {
            libc::memset(xmm_base as *mut libc::c_void, 0, 16 * 16);
        }

        // 设置xstate_bv标志位，表示SSE状态有效
        if iov.iov_len >= 512 + 8 {
            let xstate_hdr = unsafe { (xstate_buffer as *mut u8).add(512) as *mut u64 };
            unsafe {
                // 设置SSE状态位 (bit 1)
                *xstate_hdr |= 1u64 << 1;
            }
        }

        // 写回修改后的XSAVE状态
        let result = unsafe {
            ptrace(PTRACE_SETREGSET, pid, NT_X86_XSTATE as *mut c_void, &mut iov as *mut iovec as *mut c_void)
        };

        if result == -1 {
            unsafe { libc::free(xstate_buffer); }
            return Err(AsmTestError::Execution("无法设置XMM寄存器初始状态".to_string()));
        }
    }

    unsafe { libc::free(xstate_buffer); }
    Ok(())
}

/// 使用mmap分配固定地址内存（可读写）
fn allocate_fixed_memory_rw(address: u64, size: usize) -> Result<*mut c_void> {
    // 首先尝试使用MAP_FIXED
    let result = unsafe {
        mmap(
            address as *mut c_void,
            size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
            -1,
            0,
        )
    };

    // 如果失败，回退到MAP_FIXED
    if result == libc::MAP_FAILED {
        let errno = unsafe { *libc::__errno_location() };
        if errno == libc::EINVAL || errno == libc::ENOTSUP {
            let result = unsafe {
                mmap(
                    address as *mut c_void,
                    size,
                    PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED,
                    -1,
                    0,
                )
            };

            if result == libc::MAP_FAILED {
                return Err(AsmTestError::MemoryMap("无法分配固定地址内存".to_string()));
            }

            return Ok(result);
        }

        return Err(AsmTestError::MemoryMap("无法分配固定地址内存".to_string()));
    }

    Ok(result)
}

/// 使用mmap分配固定地址内存（可读写可执行）
fn allocate_fixed_memory_rwx(address: u64, size: usize) -> Result<*mut c_void> {
    // 首先尝试使用MAP_FIXED
    let result = unsafe {
        mmap(
            address as *mut c_void,
            size,
            PROT_READ | PROT_WRITE | PROT_EXEC,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED,
            -1,
            0,
        )
    };

    // 如果失败，回退到MAP_FIXED
    if result == libc::MAP_FAILED {
        let errno = unsafe { *libc::__errno_location() };
        if errno == libc::EINVAL || errno == libc::ENOTSUP {
            let result = unsafe {
                mmap(
                    address as *mut c_void,
                    size,
                    PROT_READ | PROT_WRITE | PROT_EXEC,
                    MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED,
                    -1,
                    0,
                )
            };

            if result == libc::MAP_FAILED {
                return Err(AsmTestError::MemoryMap("无法分配固定地址内存".to_string()));
            }

            return Ok(result);
        }

        return Err(AsmTestError::MemoryMap("无法分配固定地址内存".to_string()));
    }

    Ok(result)
}

/// 释放使用mmap分配的内存
fn free_memory(address: *mut c_void, size: usize) -> Result<()> {
    let result = unsafe { munmap(address, size) };
    if result == -1 {
        return Err(AsmTestError::MemoryMap("无法释放内存".to_string()));
    }
    Ok(())
}

pub fn format_register_data(register_data: &RegisterData, is_32bit: bool) -> String {
    let mut output = String::new();

    // 格式化通用寄存器
    output.push_str("通用寄存器:\n");
    if is_32bit {
        // 32位模式下显示32位寄存器名称
        if let Some(rax) = &register_data.rax {
            let eax_value = format_32bit_value(rax);
            output.push_str(&format!("  EAX: {}\n", eax_value));
        }
        if let Some(rcx) = &register_data.rcx {
            let ecx_value = format_32bit_value(rcx);
            output.push_str(&format!("  ECX: {}\n", ecx_value));
        }
        if let Some(rdx) = &register_data.rdx {
            let edx_value = format_32bit_value(rdx);
            output.push_str(&format!("  EDX: {}\n", edx_value));
        }
        if let Some(rbx) = &register_data.rbx {
            let ebx_value = format_32bit_value(rbx);
            output.push_str(&format!("  EBX: {}\n", ebx_value));
        }
        if let Some(rsp) = &register_data.rsp {
            let esp_value = format_32bit_value(rsp);
            output.push_str(&format!("  ESP: {}\n", esp_value));
        }
        if let Some(rbp) = &register_data.rbp {
            let ebp_value = format_32bit_value(rbp);
            output.push_str(&format!("  EBP: {}\n", ebp_value));
        }
        if let Some(rsi) = &register_data.rsi {
            let esi_value = format_32bit_value(rsi);
            output.push_str(&format!("  ESI: {}\n", esi_value));
        }
        if let Some(rdi) = &register_data.rdi {
            let edi_value = format_32bit_value(rdi);
            output.push_str(&format!("  EDI: {}\n", edi_value));
        }
        if let Some(rip) = &register_data.rip {
            let eip_value = format_32bit_value(rip);
            output.push_str(&format!("  RIP: {}\n", eip_value));
        }
        // 32位模式下不显示R8-R15
    } else {
        // 64位模式下显示64位寄存器名称
        if let Some(rax) = &register_data.rax {
            output.push_str(&format!("  RAX: {}\n", rax));
        }
        if let Some(rcx) = &register_data.rcx {
            output.push_str(&format!("  RCX: {}\n", rcx));
        }
        if let Some(rdx) = &register_data.rdx {
            output.push_str(&format!("  RDX: {}\n", rdx));
        }
        if let Some(rbx) = &register_data.rbx {
            output.push_str(&format!("  RBX: {}\n", rbx));
        }
        if let Some(rsp) = &register_data.rsp {
            output.push_str(&format!("  RSP: {}\n", rsp));
        }
        if let Some(rbp) = &register_data.rbp {
            output.push_str(&format!("  RBP: {}\n", rbp));
        }
        if let Some(rsi) = &register_data.rsi {
            output.push_str(&format!("  RSI: {}\n", rsi));
        }
        if let Some(rdi) = &register_data.rdi {
            output.push_str(&format!("  RDI: {}\n", rdi));
        }
        if let Some(r8) = &register_data.r8 {
            output.push_str(&format!("  R8:  {}\n", r8));
        }
        if let Some(r9) = &register_data.r9 {
            output.push_str(&format!("  R9:  {}\n", r9));
        }
        if let Some(r10) = &register_data.r10 {
            output.push_str(&format!("  R10: {}\n", r10));
        }
        if let Some(r11) = &register_data.r11 {
            output.push_str(&format!("  R11: {}\n", r11));
        }
        if let Some(r12) = &register_data.r12 {
            output.push_str(&format!("  R12: {}\n", r12));
        }
        if let Some(r13) = &register_data.r13 {
            output.push_str(&format!("  R13: {}\n", r13));
        }
        if let Some(r14) = &register_data.r14 {
            output.push_str(&format!("  R14: {}\n", r14));
        }
        if let Some(r15) = &register_data.r15 {
            output.push_str(&format!("  R15: {}\n", r15));
        }
        if let Some(rip) = &register_data.rip {
            output.push_str(&format!("  RIP: {}\n", rip));
        }
    }

    // 格式化XMM/YMM寄存器
    output.push_str("\n向量寄存器:\n");
    format_xmm_register("XMM0", &register_data.xmm0, &mut output);
    format_xmm_register("XMM1", &register_data.xmm1, &mut output);
    format_xmm_register("XMM2", &register_data.xmm2, &mut output);
    format_xmm_register("XMM3", &register_data.xmm3, &mut output);
    format_xmm_register("XMM4", &register_data.xmm4, &mut output);
    format_xmm_register("XMM5", &register_data.xmm5, &mut output);
    format_xmm_register("XMM6", &register_data.xmm6, &mut output);
    format_xmm_register("XMM7", &register_data.xmm7, &mut output);
    if !is_32bit {
        format_xmm_register("XMM8", &register_data.xmm8, &mut output);
        format_xmm_register("XMM9", &register_data.xmm9, &mut output);
        format_xmm_register("XMM10", &register_data.xmm10, &mut output);
        format_xmm_register("XMM11", &register_data.xmm11, &mut output);
        format_xmm_register("XMM12", &register_data.xmm12, &mut output);
        format_xmm_register("XMM13", &register_data.xmm13, &mut output);
        format_xmm_register("XMM14", &register_data.xmm14, &mut output);
        format_xmm_register("XMM15", &register_data.xmm15, &mut output);
    }

    // 格式化标志寄存器
    if let Some(flags) = &register_data.flags {
        // 解析flags值并显示详细信息
        if let Ok(flags_value) = u64::from_str_radix(flags.strip_prefix("0x").unwrap_or(flags), 16) {
            output.push_str(&format!("\n标志寄存器:\n  {}\n", format_flags(flags_value)));
        } else {
            output.push_str(&format!("\n标志寄存器:\n  {}\n", flags));
        }
    }

    output
}

/// 格式化32位寄存器值
fn format_32bit_value(value: &str) -> String {
    if let Some(stripped) = value.strip_prefix("0x") {
        if stripped.len() > 8 {
            format!("0x{}", &stripped[stripped.len() - 8..])
        } else {
            value.to_string()
        }
    } else {
        value.to_string()
    }
}

/// 格式化单个XMM寄存器
fn format_xmm_register(name: &str, xmm_data: &Option<Vec<String>>, output: &mut String) {
    if let Some(data) = xmm_data {
        if data.len() >= 4 {
            // YMM寄存器格式 (256位)
            let low_qword1 = &data[0];
            let low_qword2 = &data[1];
            let high_qword1 = &data[2];
            let high_qword2 = &data[3];

            // 检查高128位是否全为0，如果是则显示为XMM格式
            if high_qword1 == "0x0000000000000000" && high_qword2 == "0x0000000000000000" {
                // XMM格式 (128位)
                output.push_str(&format!("  {}: {} {}\n", name, low_qword1, low_qword2));
            } else {
                // YMM格式 (256位)
                output.push_str(&format!("  {}: {} {} {} {}\n", name, low_qword1, low_qword2, high_qword1, high_qword2));
            }
        } else if data.len() >= 2 {
            // XMM格式 (128位)
            let qword1 = &data[0];
            let qword2 = &data[1];
            output.push_str(&format!("  {}: {} {}\n", name, qword1, qword2));
        }
    }
}

/// 格式化rflags寄存器，显示各个标志位的详细信息
fn format_flags(flags: u64) -> String {
    let cf = (flags >> 0) & 1;   // 进位标志
    let pf = (flags >> 2) & 1;   // 奇偶标志
    let af = (flags >> 4) & 1;   // 辅助进位标志
    let zf = (flags >> 6) & 1;   // 零标志
    let sf = (flags >> 7) & 1;   // 符号标志
    let tf = (flags >> 8) & 1;   // 陷阱标志
    let if_flag = (flags >> 9) & 1;   // 中断允许标志
    let df = (flags >> 10) & 1;  // 方向标志
    let of = (flags >> 11) & 1;  // 溢出标志
    let iopl = (flags >> 12) & 3;     // IO特权级别
    let nt = (flags >> 14) & 1;  // 嵌套任务
    let rf = (flags >> 16) & 1;  // 恢复标志
    let vm = (flags >> 17) & 1;  // 虚拟8086模式
    let ac = (flags >> 18) & 1;  // 对齐检查
    let vif = (flags >> 19) & 1; // 虚拟中断标志
    let vip = (flags >> 20) & 1; // 虚拟中断挂起
    let id = (flags >> 21) & 1;  // ID标志

    format!("0x{:08x} [CF:{} PF:{} AF:{} ZF:{} SF:{} TF:{} IF:{} DF:{} OF:{} IOPL:{} NT:{} RF:{} VM:{} AC:{} VIF:{} VIP:{} ID:{}]",
            flags, cf, pf, af, zf, sf, tf, if_flag, df, of, iopl, nt, rf, vm, ac, vif, vip, id)
}

/// 设置初始寄存器状态
fn set_initial_registers(pid: pid_t, regs: &mut user_regs_struct, config: &AsmTestConfig) -> Result<()> {
    // 检查是否为32位模式
    let is_32bit = config.mode.as_ref().map(|m| matches!(m, ExecutionMode::Bit32)).unwrap_or(false);

    // 根据RegInit设置初始寄存器值，如果没有则默认为0
    if let Some(ref reg_init) = config.reg_init {
        if is_32bit {
            // 32位模式下只设置32位寄存器
            regs.rax = (regs.rax & 0xFFFFFFFF00000000) | (parse_register_value(&reg_init.rax).unwrap_or(0) & 0xFFFFFFFF);
            regs.rbx = (regs.rbx & 0xFFFFFFFF00000000) | (parse_register_value(&reg_init.rbx).unwrap_or(0) & 0xFFFFFFFF);
            regs.rcx = (regs.rcx & 0xFFFFFFFF00000000) | (parse_register_value(&reg_init.rcx).unwrap_or(0) & 0xFFFFFFFF);
            regs.rdx = (regs.rdx & 0xFFFFFFFF00000000) | (parse_register_value(&reg_init.rdx).unwrap_or(0) & 0xFFFFFFFF);
            regs.rsi = (regs.rsi & 0xFFFFFFFF00000000) | (parse_register_value(&reg_init.rsi).unwrap_or(0) & 0xFFFFFFFF);
            regs.rdi = (regs.rdi & 0xFFFFFFFF00000000) | (parse_register_value(&reg_init.rdi).unwrap_or(0) & 0xFFFFFFFF);
            regs.rbp = (regs.rbp & 0xFFFFFFFF00000000) | (parse_register_value(&reg_init.rbp).unwrap_or(0) & 0xFFFFFFFF);
            // 栈指针特殊处理，即使有RegInit也使用固定值
            let stack_top = 0xE0000000u64 + 16u64 * 4096u64; // 栈顶地址
            let rsp_top = (stack_top - 128) & !0xFu64; // 预留红区并16字节对齐
            regs.rsp = (regs.rsp & 0xFFFFFFFF00000000) | (rsp_top & 0xFFFFFFFF);
            regs.r8 = 0;  // 32位模式下R8-R15应为0
            regs.r9 = 0;
            regs.r10 = 0;
            regs.r11 = 0;
            regs.r12 = 0;
            regs.r13 = 0;
            regs.r14 = 0;
            regs.r15 = 0;
        } else {
            // 64位模式下设置完整64位寄存器
            regs.rax = parse_register_value(&reg_init.rax).unwrap_or(0);
            regs.rbx = parse_register_value(&reg_init.rbx).unwrap_or(0);
            regs.rcx = parse_register_value(&reg_init.rcx).unwrap_or(0);
            regs.rdx = parse_register_value(&reg_init.rdx).unwrap_or(0);
            regs.rsi = parse_register_value(&reg_init.rsi).unwrap_or(0);
            regs.rdi = parse_register_value(&reg_init.rdi).unwrap_or(0);
            regs.rbp = parse_register_value(&reg_init.rbp).unwrap_or(0);
            // 栈指针特殊处理，即使有RegInit也使用固定值
            let stack_top = 0xE0000000u64 + 16u64 * 4096u64; // 栈顶地址
            let rsp_top = (stack_top - 128) & !0xFu64; // 预留红区并16字节对齐
            regs.rsp = rsp_top;
            regs.r8 = parse_register_value(&reg_init.r8).unwrap_or(0);
            regs.r9 = parse_register_value(&reg_init.r9).unwrap_or(0);
            regs.r10 = parse_register_value(&reg_init.r10).unwrap_or(0);
            regs.r11 = parse_register_value(&reg_init.r11).unwrap_or(0);
            regs.r12 = parse_register_value(&reg_init.r12).unwrap_or(0);
            regs.r13 = parse_register_value(&reg_init.r13).unwrap_or(0);
            regs.r14 = parse_register_value(&reg_init.r14).unwrap_or(0);
            regs.r15 = parse_register_value(&reg_init.r15).unwrap_or(0);
        }
    } else {
        if is_32bit {
            regs.rax &= 0xFFFFFFFF00000000;
            regs.rbx &= 0xFFFFFFFF00000000;
            regs.rcx &= 0xFFFFFFFF00000000;
            regs.rdx &= 0xFFFFFFFF00000000;
            regs.rsi &= 0xFFFFFFFF00000000;
            regs.rdi &= 0xFFFFFFFF00000000;
            regs.rbp &= 0xFFFFFFFF00000000;
            let stack_top = 0xE0000000u64 + 16u64 * 4096u64; // 栈顶地址
            let rsp_top = (stack_top - 128) & !0xFu64; // 预留红区并16字节对齐
            regs.rsp = (regs.rsp & 0xFFFFFFFF00000000) | (rsp_top & 0xFFFFFFFF);
            regs.r8 = 0;
            regs.r9 = 0;
            regs.r10 = 0;
            regs.r11 = 0;
            regs.r12 = 0;
            regs.r13 = 0;
            regs.r14 = 0;
            regs.r15 = 0;
        } else {
            // 默认清空寄存器
            regs.rax = 0;
            regs.rbx = 0;
            regs.rcx = 0;
            regs.rdx = 0;
            regs.rsi = 0;
            regs.rdi = 0;
            regs.rbp = 0;
            let stack_top = 0xE0000000u64 + 16u64 * 4096u64; // 栈顶地址
            let rsp_top = (stack_top - 128) & !0xFu64; // 预留红区并16字节对齐
            regs.rsp = rsp_top;
            regs.r8 = 0;
            regs.r9 = 0;
            regs.r10 = 0;
            regs.r11 = 0;
            regs.r12 = 0;
            regs.r13 = 0;
            regs.r14 = 0;
            regs.r15 = 0;
        }
    }

    // 设置标志寄存器
    regs.eflags = 0x202; // 设置默认标志位

    // 设置寄存器
    let result = unsafe { ptrace(PTRACE_SETREGS, pid, std::ptr::null_mut::<c_void>(), regs as *mut user_regs_struct as *mut c_void) };
    if result == -1 {
        return Err(AsmTestError::Execution("无法设置子进程寄存器状态".to_string()));
    }

    // 设置XMM寄存器初始状态
    if let Err(e) = set_xmm_registers(pid, config) {
        eprintln!("警告: 无法设置XMM寄存器初始状态: {}", e);
        // 不返回错误，因为XMM寄存器设置失败不应该导致整个执行失败
    }

    Ok(())
}

/// 解析寄存器值字符串
fn parse_register_value(value: &Option<String>) -> Option<u64> {
    if let Some(val_str) = value {
        let trimmed = val_str.trim();
        if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
            u64::from_str_radix(&trimmed[2..], 16).ok()
        } else {
            trimmed.parse::<u64>().ok()
        }
    } else {
        None
    }
}

/// 设置XMM寄存器初始状态
fn set_xmm_registers(pid: pid_t, config: &AsmTestConfig) -> Result<()> {
    // 分配足够大的缓冲区来存储XSAVE状态
    let bufsize = 4096;
    let xstate_buffer = unsafe {
        libc::malloc(bufsize)
    };

    if xstate_buffer.is_null() {
        return Err(AsmTestError::Execution("无法分配XSAVE状态缓冲区".to_string()));
    }

    // 确保缓冲区被清零
    unsafe {
        libc::memset(xstate_buffer, 0, bufsize);
    }

    let mut iov = iovec {
        iov_base: xstate_buffer,
        iov_len: bufsize,
    };

    // 获取当前的XSAVE状态
    let result = unsafe {
        ptrace(PTRACE_GETREGSET, pid, NT_X86_XSTATE as *mut c_void, &mut iov as *mut iovec as *mut c_void)
    };

    // 即使获取失败，我们也使用已清零的缓冲区
    if result == -1 {
        // 如果获取失败，继续使用已清零的缓冲区
        eprintln!("警告: 无法获取当前XMM寄存器状态，使用默认初始化");
    }

    // 根据RegInit设置XMM寄存器初始值，如果没有则默认为0
    if let Some(ref reg_init) = config.reg_init {
        set_xmm_register_value(xstate_buffer, iov.iov_len, 0, &reg_init.xmm0);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 1, &reg_init.xmm1);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 2, &reg_init.xmm2);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 3, &reg_init.xmm3);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 4, &reg_init.xmm4);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 5, &reg_init.xmm5);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 6, &reg_init.xmm6);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 7, &reg_init.xmm7);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 8, &reg_init.xmm8);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 9, &reg_init.xmm9);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 10, &reg_init.xmm10);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 11, &reg_init.xmm11);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 12, &reg_init.xmm12);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 13, &reg_init.xmm13);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 14, &reg_init.xmm14);
        set_xmm_register_value(xstate_buffer, iov.iov_len, 15, &reg_init.xmm15);
    }
    // 如果没有RegInit，XMM寄存器已经初始化为0了

    // 设置xstate_bv标志位，表示SSE状态有效
    if iov.iov_len >= 512 + 8 {
        let xstate_hdr = unsafe { (xstate_buffer as *mut u8).add(512) as *mut u64 };
        unsafe {
            // 设置SSE状态位 (bit 1)
            *xstate_hdr |= 1u64 << 1;
        }
    }

    // 写回修改后的XSAVE状态
    let result = unsafe {
        ptrace(PTRACE_SETREGSET, pid, NT_X86_XSTATE as *mut c_void, &mut iov as *mut iovec as *mut c_void)
    };

    unsafe { libc::free(xstate_buffer); }

    if result == -1 {
        return Err(AsmTestError::Execution("无法设置XMM寄存器初始状态".to_string()));
    }

    Ok(())
}

/// 设置单个XMM寄存器的值
fn set_xmm_register_value(buffer: *mut c_void, buffer_len: usize, index: usize, value: &Option<Vec<String>>) {
    // XSAVE格式: XMM寄存器通常在偏移0xA0处开始
    if buffer_len >= 0xA0 + (index + 1) * 16 {
        let xmm_base = unsafe { (buffer as *mut u8).add(0xA0) };
        let xmm_ptr = unsafe { xmm_base.add(index * 16) };

        // 首先将XMM寄存器清零
        unsafe {
            libc::memset(xmm_ptr as *mut libc::c_void, 0, 16);
        }

        if let Some(values) = value {
            // 写入XMM寄存器值 (128位 = 16字节)
            for (i, val_str) in values.iter().enumerate() {
                if i >= 2 {
                    break; // XMM寄存器只有128位，即2个64位值
                }

                // 只有非空字符串才尝试解析
                if !val_str.trim().is_empty() {
                    if let Ok(val) = parse_hex_value(val_str) {
                        let data_ptr = unsafe { (xmm_ptr as *mut u64).add(i) };
                        unsafe { *data_ptr = val; }
                    }
                }
            }
        }
        // 如果没有指定值或值为空，保持为0（已初始化）
    }
}