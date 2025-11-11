#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>
#include <errno.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/ptrace.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/uio.h>
#include <sys/user.h>
#include <signal.h>

#ifndef NT_X86_XSTATE
#define NT_X86_XSTATE 0x202
#endif

static void hexdump_rev32(const uint8_t* p, size_t n) {
    for (ssize_t i = (ssize_t)n - 1; i >= 0; --i) {
        printf("%02x", p[i]);
    }
}

int main() {
    // 你的“待验证指令”字节序列，末尾加 int3 (0xCC)
    // 示例：设置 CF 并清零 YMM0
    unsigned char dynamic[] = {
        0xF9, // stc
        0xC5,0xFC,0x57,0xC0, // vxorps ymm0, ymm0, ymm0
        0xCC // int3
    };

    pid_t pid = fork();
    if (pid < 0) {
        perror("fork");
        return 1;
    }

    if (pid == 0) {
        // 子进程：声明自己可被 ptrace，先停一下让父进程接管
        if (ptrace(PTRACE_TRACEME, 0, NULL, NULL) != 0) {
            perror("ptrace(TRACEME)");
            _exit(1);
        }
        raise(SIGSTOP);

        // 映射并写入动态指令
        size_t pagesize = (size_t)sysconf(_SC_PAGESIZE);
        void* codebuf = mmap(NULL, pagesize, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        if (codebuf == MAP_FAILED) {
            perror("mmap");
            _exit(1);
        }
        memcpy(codebuf, dynamic, sizeof(dynamic));

        if (mprotect(codebuf, pagesize, PROT_READ | PROT_EXEC) != 0) {
            perror("mprotect");
            _exit(1);
        }

        // 执行：到 int3 会陷入 SIGTRAP 停止，函数不会返回
        void (*fn)(void) = (void(*)(void))codebuf;
        fn();

        // 若继续运行到这里（通常不会），退出
        _exit(0);
    }

    // 父进程：等待子进程停住（SIGSTOP）
    int status = 0;
    if (waitpid(pid, &status, 0) < 0) {
        perror("waitpid");
        return 1;
    }
    if (!WIFSTOPPED(status)) {
        fprintf(stderr, "child not stopped initially\n");
        return 1;
    }

    // 让子进程继续运行到 int3
    if (ptrace(PTRACE_CONT, pid, NULL, NULL) != 0) {
        perror("ptrace(CONT)");
        return 1;
    }

    // 等待 INT3 导致的 SIGTRAP
    if (waitpid(pid, &status, 0) < 0) {
        perror("waitpid");
        return 1;
    }
    if (!(WIFSTOPPED(status) && WSTOPSIG(status) == SIGTRAP)) {
        fprintf(stderr, "expected SIGTRAP, got status=0x%x\n", status);
        // 也可用 PTRACE_DETACH + SIGKILL 清理
    }

    // 读取通用寄存器 + rflags
    struct user_regs_struct regs;
    if (ptrace(PTRACE_GETREGS, pid, NULL, &regs) != 0) {
        perror("ptrace(GETREGS)");
        // 清理并退出
    }

    printf("GPRs at breakpoint:\n");
    printf("rax=%016llx rbx=%016llx rcx=%016llx rdx=%016llx\n",
    (unsigned long long)regs.rax, (unsigned long long)regs.rbx,
    (unsigned long long)regs.rcx, (unsigned long long)regs.rdx);
    printf("rsi=%016llx rdi=%016llx rbp=%016llx rsp=%016llx\n",
    (unsigned long long)regs.rsi, (unsigned long long)regs.rdi,
    (unsigned long long)regs.rbp, (unsigned long long)regs.rsp);
    printf("r8 =%016llx r9 =%016llx r10=%016llx r11=%016llx\n",
    (unsigned long long)regs.r8, (unsigned long long)regs.r9,
    (unsigned long long)regs.r10, (unsigned long long)regs.r11);
    printf("r12=%016llx r13=%016llx r14=%016llx r15=%016llx\n",
    (unsigned long long)regs.r12, (unsigned long long)regs.r13,
    (unsigned long long)regs.r14, (unsigned long long)regs.r15);
    printf("rip=%016llx eflags/rflags=%016llx\n",
    (unsigned long long)regs.rip, (unsigned long long)regs.eflags);

    unsigned long long f = regs.eflags;
    printf("CF=%d ZF=%d SF=%d OF=%d PF=%d AF=%d IF=%d DF=%d\n",
    !!(f & (1ULL<<0)), !!(f & (1ULL<<6)), !!(f & (1ULL<<7)),
    !!(f & (1ULL<<11)), !!(f & (1ULL<<2)), !!(f & (1ULL<<4)),
    !!(f & (1ULL<<9)), !!(f & (1ULL<<10)));

    // 读取 XSAVE（XMM/YMM 等）
    // 给一个足够大的缓冲区；内核会把实际大小写回 iov_len
    size_t bufsize = 4096; // 通常足够；也可用 CPUID(0xD,0) 查询 ebx 的最大尺寸
    uint8_t* xstate = NULL;
    if (posix_memalign((void**)&xstate, 64, bufsize) != 0) {
        perror("posix_memalign");
        // 清理后退出
    }
    memset(xstate, 0, bufsize);
    struct iovec iov;
    iov.iov_base = xstate;
    iov.iov_len = bufsize;

    if (ptrace(PTRACE_GETREGSET, pid, (void*)NT_X86_XSTATE, &iov) != 0) {
        perror("ptrace(GETREGSET NT_X86_XSTATE)");
        // 清理后退出
    }
    size_t xlen = iov.iov_len;
    printf("XSAVE size reported by kernel: %zu bytes\n", xlen);

    // 解析非压缩 XSAVE：XMM 在 0xA0；YMM 上半在 0x240
    // 检测压缩格式（xcomp_bv bit63），若为压缩则给出提示
    if (xlen >= 576 + 16*16) {
        // xsave header 在偏移 512
        uint64_t* hdr = (uint64_t*)(xstate + 512);
        uint64_t xstate_bv = hdr[0];
        uint64_t xcomp_bv = hdr[1];
        int compact = (xcomp_bv >> 63) & 1;

        if (compact) {
            printf("Kernel returned compact XSAVE format (xcomp_bv bit63=1).\n");
            printf("This demo assumes non-compacted layout; parsing compact requires per-component sizes.\n");
            printf("On most distros, regset returns non-compacted format; consider disabling XSAVES or adapt parser.\n");
        }

        uint8_t* xmm_base = xstate + 0xA0; // XMM0..15
        uint8_t* ymmhi_base = xstate + 0x240; // YMM_Hi128 0..15

        // 打印 XMM
        printf("XMM registers (128-bit):\n");
        for (int i = 0; i < 16; ++i) {
            printf("xmm%-2d =", i);
            hexdump_rev32(xmm_base + i * 16, 16);
            printf("\n");
        }

        // 打印 YMM（256-bit）：拼接 XMM(low128) + YMM_Hi128(upper128)
        if (xstate_bv & (1u << 2)) {
        printf("YMM registers (256-bit):\n");
            for (int i = 0; i < 16; ++i) {
                uint8_t ymm[32];
                memcpy(ymm, xmm_base + i * 16, 16);
                memcpy(ymm + 16, ymmhi_base + i * 16, 16);
                printf("ymm%-2d =", i);
                hexdump_rev32(ymm, 32);
                printf("\n");
            }
        } else {
            printf("AVX state (bit2) not present in xstate_bv; YMM not available.\n");
        }
    } else {
        printf("XSAVE buffer too small for XMM/YMM parsing (size=%zu)\n", xlen);
    }

    // 清理：杀掉子进程
    if (ptrace(PTRACE_DETACH, pid, NULL, (void*)(long)SIGKILL) != 0) {
        perror("ptrace(DETACH SIGKILL)");
    }
    free(xstate);
    return 0;
}