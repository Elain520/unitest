//! 执行器模块
//!
//! 负责在原生x86环境下执行编译后的代码，使用父子进程协作模型和ptrace系统调用

use crate::error::{AsmTestError, Result};
use crate::elf::ElfInfo;
use x86_asm_test::{AsmTestConfig, RegisterData, XmmRegisters, MemorySize, MemoryValue};
use libc::{c_void, pid_t, waitpid, WIFSTOPPED, WSTOPSIG, SIGTRAP, SIGSTOP, fork, ptrace, PTRACE_TRACEME, PTRACE_ATTACH, PTRACE_DETACH, PTRACE_GETREGS, PTRACE_SETREGS, PTRACE_CONT, PTRACE_GETREGSET, mmap, munmap, MAP_PRIVATE, MAP_ANONYMOUS, MAP_FIXED, PROT_READ, PROT_WRITE, PROT_EXEC, user_regs_struct, kill, raise, sleep, MAP_FIXED_NOREPLACE, iovec, PTRACE_SETREGSET};

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
        for (address_str, data) in memory_data {
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

                        // 写入数据
                        let mut current_offset = 0;
                        for value in data {
                            if offset + current_offset >= region_size {
                                eprintln!("[child] 内存数据超出区域边界: 地址={}, 值={:?}", address_str, value);
                                break;
                            }

                            match value {
                                MemoryValue::Number(n) => {
                                    let bytes = n.to_le_bytes();
                                    for (i, byte) in bytes.iter().enumerate() {
                                        if offset + current_offset + i < region_size {
                                            *region_ptr.add(current_offset + i) = *byte;
                                        }
                                    }
                                    current_offset += bytes.len();
                                }
                                MemoryValue::HexString(hex_str) => {
                                    match parse_hex_value(hex_str) {
                                        Ok(val) => {
                                            let bytes = val.to_le_bytes();
                                            for (i, byte) in bytes.iter().enumerate() {
                                                if offset + current_offset + i < region_size {
                                                    *region_ptr.add(current_offset + i) = *byte;
                                                }
                                            }
                                            current_offset += bytes.len();
                                        }
                                        Err(e) => {
                                            eprintln!("[child] 无法解析十六进制值 {}: {}", hex_str, e);
                                        }
                                    }
                                }
                            }
                        }

                        if cfg!(debug_assertions) {
                            // 显示写入数据后的内存内容
                            eprintln!("[child] 写入数据后的内存内容:");
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
    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        u64::from_str_radix(&trimmed[2..], 16)
            .map_err(|e| AsmTestError::Execution(format!("无法解析十六进制值 {}: {}", hex_str, e)))
    } else {
        trimmed.parse::<u64>()
            .map_err(|e| AsmTestError::Execution(format!("无法解析值 {}: {}", hex_str, e)))
    }
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
    if let Err(e) = set_initial_registers(pid, &mut regs) {
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

/// 使用ptrace分离进程
fn detach_from_process(_pid: pid_t) -> Result<()> {
    // 不再需要分离进程，因为子进程已经设置了PTRACE_TRACEME
    Ok(())
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
    // register_data.flags = Some(format!("0x{:08x}", regs.eflags));
    let flags_value = regs.eflags;
    register_data.flags = Some(format_flags(flags_value));

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

fn format_flags(flags: u64) -> String {
    let cf = (flags >> 0) & 1;  // 进位标志
    let pf = (flags >> 2) & 1;  // 奇偶标志
    let af = (flags >> 4) & 1;  // 辅助进位标志
    let zf = (flags >> 6) & 1;  // 零标志
    let sf = (flags >> 7) & 1;  // 符号标志
    let tf = (flags >> 8) & 1;  // 陷阱标志
    let if_flag = (flags >> 9) & 1;  // 中断允许标志
    let df = (flags >> 10) & 1; // 方向标志
    let of = (flags >> 11) & 1; // 溢出标志

    let flags_desc = format!(
        "CF:{}(进位) PF:{}(奇偶) AF:{}(辅助进位) ZF:{}(零) SF:{}(符号) TF:{}(陷阱) IF:{}(中断) DF:{}(方向) OF:{}(溢出)",
        cf, pf, af, zf, sf, tf, if_flag, df, of
    );

    format!("0x{:08x} [{}]", flags, flags_desc)
}

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

        // 解析每个XMM寄存器 (128位 = 16字节)
        for i in 0..16 {
            let xmm_ptr = unsafe { xmm_base.add(i * 16) };
            let mut xmm_data = Vec::new();

            // 以64位为单位读取XMM寄存器数据
            for j in 0..2 {
                let data_ptr = unsafe { (xmm_ptr as *const u64).add(j) };
                let data = unsafe { *data_ptr };
                xmm_data.push(format!("0x{:016x}", data));
            }

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

    unsafe { libc::free(xstate_buffer); }
    Ok(xmm_registers)
}

/// 设置初始寄存器状态
fn set_initial_registers(pid: pid_t, regs: &mut user_regs_struct) -> Result<()> {
    // 清空寄存器
    regs.rax = 0;
    regs.rbx = 0;
    regs.rcx = 0;
    regs.rdx = 0;
    regs.rsi = 0;
    regs.rdi = 0;
    regs.rbp = 0;
    // 设置栈指针到栈顶，预留红区（128字节）并保持16字节对齐
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

    // 设置标志寄存器
    regs.eflags = 0x202; // 设置默认标志位

    // 设置寄存器
    let result = unsafe { ptrace(PTRACE_SETREGS, pid, std::ptr::null_mut::<c_void>(), regs as *mut user_regs_struct as *mut c_void) };
    if result == -1 {
        return Err(AsmTestError::Execution("无法设置子进程寄存器状态".to_string()));
    }

    // 重置XMM/YMM寄存器
    if let Err(e) = reset_xmm_ymm_registers(pid) {
        eprintln!("[parent] 警告: 无法重置XMM/YMM寄存器: {}", e);
        // 继续执行，即使XMM/YMM寄存器重置失败
    }

    Ok(())
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