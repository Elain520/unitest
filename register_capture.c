#define _GNU_SOURCE
#include <stdio.h>
#include <signal.h>
#include <unistd.h>
#include <string.h>
#include <sys/ucontext.h>

// 寄存器捕获结构
typedef struct {
    unsigned long rax, rbx, rcx, rdx;
    unsigned long rsi, rdi, rbp, rsp;
    unsigned long rip, rflags;
} RegisterState;

// 全局存储寄存器状态
volatile RegisterState captured_registers;

// 异步信号安全的寄存器捕获
void capture_registers(ucontext_t *ctx) {
    captured_registers.rax = ctx->uc_mcontext.gregs[REG_RAX];
    captured_registers.rbx = ctx->uc_mcontext.gregs[REG_RBX];
    captured_registers.rcx = ctx->uc_mcontext.gregs[REG_RCX];
    captured_registers.rdx = ctx->uc_mcontext.gregs[REG_RDX];
    captured_registers.rsi = ctx->uc_mcontext.gregs[REG_RSI];
    captured_registers.rdi = ctx->uc_mcontext.gregs[REG_RDI];
    captured_registers.rbp = ctx->uc_mcontext.gregs[REG_RBP];
    captured_registers.rsp = ctx->uc_mcontext.gregs[REG_RSP];
    captured_registers.rip = ctx->uc_mcontext.gregs[REG_RIP];
    captured_registers.rflags = ctx->uc_mcontext.gregs[REG_EFL];
}

// 寄存器捕获信号处理函数
void register_capture_handler(int sig, siginfo_t *info, void *ucontext) {
    ucontext_t *ctx = (ucontext_t *)ucontext;

    // 1. 捕获当前寄存器状态
    capture_registers(ctx);

    // 2. 准备恢复执行 (跳过 int3 指令)
    ctx->uc_mcontext.gregs[REG_RIP] += 1;
}

// 初始化断点捕获
void init_breakpoint_capture() {
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_sigaction = register_capture_handler;
    sa.sa_flags = SA_SIGINFO | SA_NODEFER;

    if (sigaction(SIGTRAP, &sa, NULL) == -1) {
        perror("sigaction");
        _exit(1);
    }
}

// 触发断点并捕获寄存器
void trigger_breakpoint() {
    // 触发前的寄存器状态可能已变化，所以我们在断点前添加内存屏障
    asm volatile(
        "mfence\n\t"        // 确保内存操作完成
        "int $0xCC\n\t"     // 触发断点
        ::: "memory"
    );
}

// 在非信号上下文中安全打印寄存器
void print_registers() {
    printf("Capture Register State:\n");
    printf("RAX: 0x%016lx\n", captured_registers.rax);
    printf("RBX: 0x%016lx\n", captured_registers.rbx);
    printf("RCX: 0x%016lx\n", captured_registers.rcx);
    printf("RDX: 0x%016lx\n", captured_registers.rdx);
    printf("RSI: 0x%016lx\n", captured_registers.rsi);
    printf("RDI: 0x%016lx\n", captured_registers.rdi);
    printf("RBP: 0x%016lx\n", captured_registers.rbp);
    printf("RSP: 0x%016lx\n", captured_registers.rsp);
    printf("RIP: 0x%016lx\n", captured_registers.rip);
    printf("RFLAGS: 0x%016lx\n", captured_registers.rflags);
}

// 测试函数：操作寄存器
void manipulate_registers() {
    asm volatile(
        "mov $0x1122334455667788, %%rax\n\t"
        "mov $0x8877665544332211, %%rbx\n\t"
        "mov $0xAAAAAAAAAAAAAAAA, %%rcx\n\t"
        "mov $0xBBBBBBBBBBBBBBBB, %%rdx\n\t"
        ::: "rax", "rbx", "rcx", "rdx"
    );
}

int main() {
    init_breakpoint_capture();

    printf("=== Register Capture Test ===\n");
    printf("Before breakpoint:\n");

    // 操作寄存器以创建可见变化
    manipulate_registers();

    // 触发断点捕获寄存器状态
    trigger_breakpoint();

    // 断点后恢复执行
    printf("After breakpoint:\n");

    // 安全打印捕获的寄存器状态
    print_registers();

    return 0;
}
