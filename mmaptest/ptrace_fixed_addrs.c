#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>
#include <errno.h>
#include <unistd.h>
#include <signal.h>
#include <sys/mman.h>
#include <sys/ptrace.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/uio.h>
#include <sys/user.h>

#ifndef NT_X86_XSTATE
#define NT_X86_XSTATE 0x202
#endif
#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif

#define CODE_BASE ((void*)0xC0000000ull)
#define CODE_SIZE (4096UL)
#define STACK_BASE ((void*)0xE0000000ull)
#define STACK_SIZE (16UL*4096UL)

static void dump_gprs(const struct user_regs_struct* r) {
    printf("rax=%016llx rbx=%016llx rcx=%016llx rdx=%016llx\n",
    (unsigned long long)r->rax, (unsigned long long)r->rbx,
    (unsigned long long)r->rcx, (unsigned long long)r->rdx);
    printf("rsi=%016llx rdi=%016llx rbp=%016llx rsp=%016llx\n",
    (unsigned long long)r->rsi, (unsigned long long)r->rdi,
    (unsigned long long)r->rbp, (unsigned long long)r->rsp);
    printf("r8 =%016llx r9 =%016llx r10=%016llx r11=%016llx\n",
    (unsigned long long)r->r8, (unsigned long long)r->r9,
    (unsigned long long)r->r10, (unsigned long long)r->r11);
    printf("r12=%016llx r13=%016llx r14=%016llx r15=%016llx\n",
    (unsigned long long)r->r12, (unsigned long long)r->r13,
    (unsigned long long)r->r14, (unsigned long long)r->r15);
    printf("rip=%016llx rflags=%016llx\n",
    (unsigned long long)r->rip, (unsigned long long)r->eflags);
}

static void hexdump_rev(const uint8_t* p, size_t n) {
    static const char* hex = "0123456789abcdef";
    for (ssize_t i = (ssize_t)n - 1; i >= 0; --i) {
        putchar(hex[p[i] >> 4]);
        putchar(hex[p[i] & 0xF]);
    }
}

static void* map_fixed_addr(void* addr, size_t size, int prot) {
    void* p = mmap(addr, size, prot, MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE, -1, 0);
    if (p == MAP_FAILED && (errno == EINVAL || errno == ENOTSUP)) {
        // 老内核不支持 MAP_FIXED_NOREPLACE，回退到 MAP_FIXED（注意：会覆盖冲突映射）
        p = mmap(addr, size, prot, MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED, -1, 0);
    }
    return p;
}

int main() {
    // 你的被测指令（示例包含 push/pop 和标志位操作）；末尾必须有 int3 作为结束断点
    unsigned char dynamic[] = {
//        // #1
//        0xF9, // stc (CF=1)
//        0x50, // push rax (使用栈验证我们设置的新 rsp)
//        0x58, // pop rax
//        // #2
//        0x48, 0xC7, 0xC2, 0xC0, 0x00, 0x00, 0x00, // mov rdx,0xc0
//        0x48, 0xC1, 0xEA, 0x04, // shr rdx,0x4
        // #3
        0x48, 0xB8, 0xBE, 0xBA, 0xFE, 0xCA, 0xEF, 0xBE, 0xAD, 0xDE, //movabs rax,0xdeadbeefcafebabe
        0x50, // push rax
        0x5A, // pop rdx
//        // #4
//        0x48, 0xB8, 0x00, 0x00, 0x40, 0x40, 0x00, 0x00, 0x40, 0x40, //movabs rax,0x4040000040400000
//        0x66, 0x48, 0x0F, 0x6E, 0xC0, // movq   xmm0,rax
//        0x48, 0xB8, 0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x80, 0x40, // movabs rax,0x408000003f800000
//        0x66, 0x48, 0x0F, 0x6E, 0xC8, // movq   xmm1,rax
//        0x66, 0x0F, 0x6C, 0xC1, // punpcklqdq xmm0,xmm1
//        0x0F, 0xC6, 0xC0, 0x1B, // shufps xmm0,xmm0,0x1b
        0xCC // int3 (结束断点)
    };

    pid_t pid = fork();
    if (pid < 0) { perror("fork"); return 1; }

    if (pid == 0) {
        // 子进程：允许被跟踪并先停住
        if (ptrace(PTRACE_TRACEME, 0, NULL, NULL) != 0) { perror("PTRACE_TRACEME"); _exit(1); }
        raise(SIGSTOP);

        // 固定地址映射代码页
        void* code = map_fixed_addr(CODE_BASE, CODE_SIZE, PROT_READ|PROT_WRITE);
        if (code == MAP_FAILED || code != CODE_BASE) {
            perror("mmap code (fixed)");
            _exit(1);
        }
        // 写入 [int3] + dynamic
        uint8_t* c = (uint8_t*)code;
        size_t off = 0;
        c[off++] = 0xCC; // 起始断点
        if (off + sizeof(dynamic) > CODE_SIZE) { fprintf(stderr, "dynamic too large\n"); _exit(1); }
        memcpy(c + off, dynamic, sizeof(dynamic));
        off += sizeof(dynamic);
        if (mprotect(code, CODE_SIZE, PROT_READ|PROT_EXEC) != 0) { perror("mprotect code RX"); _exit(1); }

        // 固定地址映射数据栈
        void* stk = map_fixed_addr(STACK_BASE, STACK_SIZE, PROT_READ|PROT_WRITE);
        if (stk == MAP_FAILED || stk != STACK_BASE) {
            perror("mmap stack (fixed)");
            _exit(1);
        }

        // 进入代码（第一条就是 int3）
        void (*fn)(void) = (void(*)(void))code;
        fn();

        _exit(0);
    }

    // 父进程
    int status = 0;
    if (waitpid(pid, &status, 0) < 0 || !WIFSTOPPED(status)) {
        perror("waitpid init"); goto kill_child;
    }

    // 让子进程运行到“起始 int3”
    if (ptrace(PTRACE_CONT, pid, NULL, NULL) != 0) { perror("PTRACE_CONT#to_first_bp"); goto kill_child; }
    if (waitpid(pid, &status, 0) < 0) { perror("waitpid first trap"); goto kill_child; }
    if (!(WIFSTOPPED(status) && WSTOPSIG(status) == SIGTRAP)) {
        fprintf(stderr, "expected first SIGTRAP, got 0x%x\n", status);
        goto kill_child;
    }

    // 第一次断点：清空寄存器并把 rsp 指到固定栈顶
    struct user_regs_struct regs;
    if (ptrace(PTRACE_GETREGS, pid, NULL, &regs) != 0) { perror("GETREGS first"); goto kill_child; }

    // rip 当前位于 CODE_BASE+1（越过起始 int3），保持不动即可
    uintptr_t rsp_top = (uintptr_t)STACK_BASE + (size_t)STACK_SIZE - 128; // 预留红区
    rsp_top &= ~0xFULL; // 16 字节对齐
    regs.rsp = rsp_top;

    // 清空其它 GPR
    regs.rax = regs.rbx = regs.rcx = regs.rdx = 0;
    regs.rsi = regs.rdi = regs.rbp = 0;
    regs.r8 = regs.r9 = regs.r10 = regs.r11 = regs.r12 = regs.r13 = regs.r14 = regs.r15 = 0;

    // rflags：保留必要位（bit1=1，IF通常保持），清算术/方向/溢出等
    unsigned long long f = regs.eflags;
    const unsigned long long keep = (1ULL<<1) | (1ULL<<9);
    const unsigned long long clear = (1ULL<<0)|(1ULL<<2)|(1ULL<<4)|(1ULL<<6)|(1ULL<<7)|(1ULL<<10)|(1ULL<<11);
    regs.eflags = (f & ~clear) | keep;

    if (ptrace(PTRACE_SETREGS, pid, NULL, &regs) != 0) { perror("SETREGS baseline"); goto kill_child; }

    // 可选：清空 XMM/YMM（非压缩 XSAVE 假定）
    size_t cap = 8192;
    uint8_t* xstate = NULL;
    if (posix_memalign((void**)&xstate, 64, cap) == 0) {
        memset(xstate, 0, cap);
        struct iovec iov = { .iov_base = xstate, .iov_len = cap };
        if (ptrace(PTRACE_GETREGSET, pid, (void*)NT_X86_XSTATE, &iov) == 0) {
            size_t xlen = iov.iov_len;
            if (xlen >= 0x240 + 16*16) {
                // Header 在偏移 512：xstate_bv / xcomp_bv
                uint64_t* hdr = (uint64_t*)(xstate + 512);
                hdr[0] |= (1u<<1) | (1u<<2); // SSE + AVX
                hdr[1] &= ~(1ULL<<63); // 非压缩格式
                memset(xstate + 0xA0, 0, 16*16); // XMM0..15
                memset(xstate + 0x240, 0, 16*16); // YMM_Hi128 0..15
                struct iovec iow = { .iov_base = xstate, .iov_len = xlen };
                (void)ptrace(PTRACE_SETREGSET, pid, (void*)NT_X86_XSTATE, &iow); // 失败则忽略
            }
        }
        free(xstate);
    }

    // 继续执行到“结束 int3”
    if (ptrace(PTRACE_CONT, pid, NULL, NULL) != 0) { perror("PTRACE_CONT run"); goto kill_child; }
    if (waitpid(pid, &status, 0) < 0) { perror("waitpid second trap"); goto kill_child; }
    if (!(WIFSTOPPED(status) && WSTOPSIG(status) == SIGTRAP)) {
        fprintf(stderr, "expected second SIGTRAP, got 0x%x\n", status);
        goto kill_child;
    }

    // 读取最终寄存器
    struct user_regs_struct regs2;
    if (ptrace(PTRACE_GETREGS, pid, NULL, &regs2) != 0) { perror("GETREGS second"); goto kill_child; }
    printf("GPRs at second breakpoint:\n");
    dump_gprs(&regs2);
    unsigned long long rf = regs2.eflags;
    printf("CF=%d ZF=%d SF=%d OF=%d PF=%d AF=%d IF=%d DF=%d\n",
    !!(rf & (1ULL<<0)), !!(rf & (1ULL<<6)), !!(rf & (1ULL<<7)),
    !!(rf & (1ULL<<11)), !!(rf & (1ULL<<2)), !!(rf & (1ULL<<4)),
    !!(rf & (1ULL<<9)), !!(rf & (1ULL<<10)));

    // 读取 XSTATE 并打印 XMM/YMM（非压缩 XSAVE）
    size_t cap2 = 8192;
    uint8_t* xs2 = NULL;
    if (posix_memalign((void**)&xs2, 64, cap2) == 0) {
        memset(xs2, 0, cap2);
        struct iovec i2 = { .iov_base = xs2, .iov_len = cap2 };
        if (ptrace(PTRACE_GETREGSET, pid, (void*)NT_X86_XSTATE, &i2) == 0) {
            size_t xl = i2.iov_len;
            printf("XSAVE size: %zu bytes\n", xl);
            if (xl >= 0x240 + 16*16) {
                uint64_t* hdr = (uint64_t*)(xs2 + 512);
                uint64_t xstate_bv = hdr[0];
                uint64_t xcomp_bv = hdr[1];
                int compact = (xcomp_bv >> 63) & 1;
                if (compact) {
                    printf("Kernel returned compact XSAVE; parser here assumes non-compacted layout.\n");
                }
                uint8_t* xmm = xs2 + 0xA0;
                uint8_t* ymmh = xs2 + 0x240;

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
            }
        }
        free(xs2);
    }

    // 不依赖返回：直接终止子进程
    kill(pid, SIGKILL);
    waitpid(pid, NULL, 0);
    return 0;
    kill_child:
    kill(pid, SIGKILL);
    waitpid(pid, NULL, 0);
    return 1;
}