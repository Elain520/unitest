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

static void hexdump_rev(const uint8_t* p, size_t n) {
    for (ssize_t i = (ssize_t)n - 1; i >= 0; --i) putchar("0123456789abcdef"[p[i] >> 4]), putchar("0123456789abcdef"[p[i] & 0xf]);
}

int main() {
    // 你的动态指令序列；末尾无需自己加 int3，本程序会加
        unsigned char dynamic[] = {
        0xF9, // stc
        0xC5,0xFC,0x57,0xC0, // vxorps ymm0, ymm0, ymm0
        // ... 这里可以放任意被测指令（不要包含 sys_exit 之类直接结束进程的）
    };

    pid_t pid = fork();
    if (pid < 0) { perror("fork"); return 1; }

    if (pid == 0) {
        // 子进程
        if (ptrace(PTRACE_TRACEME, 0, NULL, NULL) != 0) {
            perror("PTRACE_TRACEME");
            _exit(1);
        }
        raise(SIGSTOP); // 让父进程先附加

        size_t pagesz = (size_t)sysconf(_SC_PAGESIZE);
        uint8_t* code = mmap(NULL, pagesz, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        if (code == MAP_FAILED) {
            perror("mmap");
            _exit(1);
        }

        // 生成 [int3] + dynamic + [int3]
        size_t off = 0;
        code[off++] = 0xCC; // 起始断点
        memcpy(code + off, dynamic, sizeof(dynamic));
        off += sizeof(dynamic);
        code[off++] = 0xCC; // 结束断点

        if (mprotect(code, pagesz, PROT_READ | PROT_EXEC) != 0) {
            perror("mprotect");
            _exit(1);
        }

        // 跳入执行（使用函数指针调用也可以；这里直接调用，第一条就是 int3）
        void (*fn)(void) = (void(*)(void))code;
        fn();

        _exit(0);
    }

    // 父进程
    int status = 0;
    if (waitpid(pid, &status, 0) < 0) {
        perror("waitpid#init");
        return 1;
    }
    if (!WIFSTOPPED(status)) {
        fprintf(stderr, "child not stopped initially\n");
        goto kill_child;
    }

    // 继续运行，直到命中“起始 int3”
    if (ptrace(PTRACE_CONT, pid, NULL, NULL) != 0) {
        perror("PTRACE_CONT#to_start_bp");
        goto kill_child;
    }
    if (waitpid(pid, &status, 0) < 0) {
        perror("waitpid#start_bp");
        goto kill_child;
    }
    if (!(WIFSTOPPED(status) && WSTOPSIG(status) == SIGTRAP)) {
        fprintf(stderr, "expected first SIGTRAP, got 0x%x\n", status);
        goto kill_child;
    }

    // 第一处断点：设置“基线寄存器”和“基线 XMM/YMM”
    struct user_regs_struct regs;
    if (ptrace(PTRACE_GETREGS, pid, NULL, &regs) != 0) {
        perror("PTRACE_GETREGS#start");
        goto kill_child;
    }

    // 设置所有 GPR 为已知值（示例用不同花纹；你可按需求设置）
    regs.rax = 0x0;
    regs.rbx = 0x0;
    regs.rcx = 0x0;
    regs.rdx = 0x0;
    regs.rsi = 0x0;
    regs.rdi = 0x0;
    regs.rbp = 0x0;
    // rsp 保持原样或设置到你控制的可用栈；这里沿用当前栈，避免破坏返回
    // regs.rsp = regs.rsp;
    regs.r8 = 0x0;
    regs.r9 = 0x0;
    regs.r10 = 0x0;
    regs.r11 = 0x0;
    regs.r12 = 0x0;
    regs.r13 = 0x0;
    regs.r14 = 0x0;
    regs.r15 = 0x0;

    // 处理 rflags：保留必须位，清/置算术标志为基线
    unsigned long long f = regs.eflags;
    const unsigned long long keep = (1ULL<<1) | (1ULL<<9); // bit1恒1，IF通常保持1
    const unsigned long long clear_mask = (1ULL<<0)|(1ULL<<2)|(1ULL<<4)|(1ULL<<6)|(1ULL<<7)|(1ULL<<10)|(1ULL<<11);
    f = (f & ~clear_mask) | keep; // 全清算术方向溢出等
    regs.eflags = f;

    if (ptrace(PTRACE_SETREGS, pid, NULL, &regs) != 0) {
        perror("PTRACE_SETREGS#baseline");
        goto kill_child;
    }

    // 设置 XMM/YMM 基线：先取一份 xstate，修改后写回
    // 先用一个较大的缓冲；内核会回填实际大小
    size_t xbuf_cap = 4096;
    uint8_t* xstate = NULL;
    if (posix_memalign((void**)&xstate, 64, xbuf_cap) != 0) {
        perror("posix_memalign");
        goto kill_child;
    }
    memset(xstate, 0, xbuf_cap);
    struct iovec iov = { .iov_base = xstate, .iov_len = xbuf_cap };
    if (ptrace(PTRACE_GETREGSET, pid, (void*)NT_X86_XSTATE, &iov) != 0) {
        perror("PTRACE_GETREGSET#baseline");
        free(xstate);
        goto kill_child;
    }
    size_t xlen = iov.iov_len;

    if (xlen >= 0x240 + 16*16) {
        // 非压缩 XSAVE 假设：XMM 在 0xA0，YMM_Hi 在 0x240
        uint8_t* xmm = xstate + 0xA0;
        uint8_t* ymmh = xstate + 0x240;

        // 给出一个基线花纹：全零
        memset(xmm, 0, 16*16);
        memset(ymmh, 0, 16*16);

        // MXCSR 默认 0x1F80 在 FXSAVE 区偏移 24（但 PTRACE 可能忽略这处写入）
        // xsave header 在 512：xstate_bv/ xcomp_bv
        uint64_t* hdr = (uint64_t*)(xstate + 512);
        // 告诉内核哪些分量有效：bit1=SSE，bit2=AVX
        hdr[0] |= (1u<<1) | (1u<<2); // xstate_bv
        // 非压缩格式：xcomp_bv bit63 应为 0
        hdr[1] &= ~(1ULL<<63);

        struct iovec iovw = { .iov_base = xstate, .iov_len = xlen };
        if (ptrace(PTRACE_SETREGSET, pid, (void*)NT_X86_XSTATE, &iovw) != 0) {
            perror("PTRACE_SETREGSET#baseline");
            // 继续执行也可以，只是矢量基线无法保证
        }
    } else {
        // 缓冲不够或内核不返回 AVX 区；跳过
    }
    free(xstate);

    // 继续运行，命中“末尾 int3”
    if (ptrace(PTRACE_CONT, pid, NULL, NULL) != 0) {
        perror("PTRACE_CONT#run");
        goto kill_child;
    }
    if (waitpid(pid, &status, 0) < 0) {
        perror("waitpid#end_bp");
        goto kill_child;
    }
    if (!(WIFSTOPPED(status) && WSTOPSIG(status) == SIGTRAP)) {
        fprintf(stderr, "expected second SIGTRAP, got 0x%x\n", status);
        goto kill_child;
    }

    // 采集最终 GPR/rflags
    if (ptrace(PTRACE_GETREGS, pid, NULL, &regs) != 0) {
        perror("PTRACE_GETREGS#final");
        goto kill_child;
    }

    printf("GPRs at final breakpoint:\n");
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
    printf("rip=%016llx rflags=%016llx\n",
    (unsigned long long)regs.rip, (unsigned long long)regs.eflags);

    // 采集最终 XMM/YMM
    size_t cap2 = 8192;
    uint8_t* x2 = NULL;
    if (posix_memalign((void**)&x2, 64, cap2) != 0) {
        perror("posix_memalign#final");
        goto kill_child;
    }
    memset(x2, 0, cap2);
    struct iovec i2 = { .iov_base = x2, .iov_len = cap2 };
    if (ptrace(PTRACE_GETREGSET, pid, (void*)NT_X86_XSTATE, &i2) != 0) {
        perror("PTRACE_GETREGSET#final");
        free(x2);
        goto kill_child;
    }
    size_t xl = i2.iov_len;
    if (xl >= 0x240 + 16*16) {
        uint8_t* xmm = x2 + 0xA0;
        uint8_t* ymmh = x2 + 0x240;
        uint64_t* hdr = (uint64_t*)(x2 + 512);
        uint64_t xstate_bv = hdr[0];
        uint64_t xcomp_bv = hdr[1];
        int compact = (xcomp_bv >> 63) & 1;
        if (compact) {
            printf("Kernel returned compact XSAVE; this demo assumes non-compacted layout.\n");
        }

        printf("XMM registers:\n");
        for (int i = 0; i < 16; ++i) {
            printf("xmm%-2d =", i); hexdump_rev(xmm + i*16, 16); printf("\n");
        }
        if (xstate_bv & (1u<<2)) {
            printf("YMM registers:\n");
            for (int i = 0; i < 16; ++i) {
                uint8_t ymm[32];
                memcpy(ymm, xmm + i*16, 16);
                memcpy(ymm + 16, ymmh + i*16, 16);
                printf("ymm%-2d =", i); hexdump_rev(ymm, 32); printf("\n");
            }
        } else {
            printf("AVX state not present; YMM unavailable.\n");
        }
    } else {
        printf("XSAVE buffer too small or AVX not exposed by kernel.\n");
    }
    free(x2);

    // 清理
    if (ptrace(PTRACE_DETACH, pid, NULL, (void*)(long)SIGKILL) != 0) perror("PTRACE_DETACH");
    return 0;
    kill_child:
    // 强制结束子进程
    kill(pid, SIGKILL);
    waitpid(pid, NULL, 0);
    return 1;
}