#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

struct Snapshot {
    uint64_t rax, rbx, rcx, rdx;
    uint64_t rsi, rdi, rbp, rsp;
    uint64_t r8, r9, r10, r11, r12, r13, r14, r15;
    uint64_t rflags;
};

int main() {
    unsigned char code[] = {
    // mov rax, 0x1122334455667788; stc
    0x48,0xB8,0x88,0x77,0x66,0x55,0x44,0x33,0x22,0x11,
    0xF9,
    // save regs to [rdi + offset]
    0x48,0x89,0x07, // [rdi+0x00] = rax
    0x48,0x89,0x5F,0x08, // [rdi+0x08] = rbx
    0x48,0x89,0x4F,0x10, // [rdi+0x10] = rcx
    0x48,0x89,0x57,0x18, // [rdi+0x18] = rdx
    0x48,0x89,0x77,0x20, // [rdi+0x20] = rsi
    0x48,0x89,0x7F,0x28, // [rdi+0x28] = rdi
    0x48,0x89,0x6F,0x30, // [rdi+0x30] = rbp
    0x48,0x89,0x67,0x38, // [rdi+0x38] = rsp
    0x4C,0x89,0x47,0x40, // [rdi+0x40] = r8
    0x4C,0x89,0x4F,0x48, // [rdi+0x48] = r9
    0x4C,0x89,0x57,0x50, // [rdi+0x50] = r10
    0x4C,0x89,0x5F,0x58, // [rdi+0x58] = r11
    0x4C,0x89,0x67,0x60, // [rdi+0x60] = r12
    0x4C,0x89,0x6F,0x68, // [rdi+0x68] = r13
    0x4C,0x89,0x77,0x70, // [rdi+0x70] = r14
    0x4C,0x89,0x7F,0x78, // [rdi+0x78] = r15
    0x9C, // pushfq
    0x58, // pop rax
    0x48,0x89,0x87,0x80,0x00,0x00,0x00, // [rdi+0x80] = rflags (in rax)
    0xC3 // ret
    };

    size_t pagesize = (size_t)sysconf(_SC_PAGESIZE);
    void* buf = mmap(NULL, pagesize, PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        if (buf == MAP_FAILED) {
        perror("mmap");
        return 1;
    }

    memcpy(buf, code, sizeof(code));
    if (mprotect(buf, pagesize, PROT_READ | PROT_EXEC) != 0) {
        perror("mprotect");
        return 1;
    }

    struct Snapshot snap = {0};

    // 作为普通函数指针调用：System V x86-64 ABI 会把第一个参数放到 rdi
    void (*fn)(struct Snapshot*) = (void(*)(struct Snapshot*))buf;
    fn(&snap);

    printf("rax=%016llx\n", (unsigned long long)snap.rax);
    printf("rbx=%016llx\n", (unsigned long long)snap.rbx);
    printf("rcx=%016llx\n", (unsigned long long)snap.rcx);
    printf("rdx=%016llx\n", (unsigned long long)snap.rdx);
    printf("rsi=%016llx\n", (unsigned long long)snap.rsi);
    printf("rdi=%016llx\n", (unsigned long long)snap.rdi);
    printf("rbp=%016llx\n", (unsigned long long)snap.rbp);
    printf("rsp=%016llx\n", (unsigned long long)snap.rsp);
    printf("r8 =%016llx\n", (unsigned long long)snap.r8);
    printf("r9 =%016llx\n", (unsigned long long)snap.r9);
    printf("r10=%016llx\n", (unsigned long long)snap.r10);
    printf("r11=%016llx\n", (unsigned long long)snap.r11);
    printf("r12=%016llx\n", (unsigned long long)snap.r12);
    printf("r13=%016llx\n", (unsigned long long)snap.r13);
    printf("r14=%016llx\n", (unsigned long long)snap.r14);
    printf("r15=%016llx\n", (unsigned long long)snap.r15);
    printf("rflags=%016llx\n", (unsigned long long)snap.rflags);

    unsigned long long f = snap.rflags;
    printf("CF=%d ZF=%d SF=%d OF=%d PF=%d AF=%d IF=%d DF=%d\n",
    !!(f & (1ULL<<0)), !!(f & (1ULL<<6)), !!(f & (1ULL<<7)),
    !!(f & (1ULL<<11)), !!(f & (1ULL<<2)), !!(f & (1ULL<<4)),
    !!(f & (1ULL<<9)), !!(f & (1ULL<<10)));

    munmap(buf, pagesize);
    return 0;
}