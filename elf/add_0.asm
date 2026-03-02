%ifdef CONFIG
{
    "RegInit": {
        "RAX": "0xc740",
        "RDX": "0x39de"
    },
    "MemoryRegions": {
        "0x10000000": 4096
    }
}
%endif

; Case 0: ADD
mov rax, 0xc740
mov rdx, 0x39de
add rax, rdx
nop
