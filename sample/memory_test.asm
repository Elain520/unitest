%ifdef CONFIG
{
    "MemoryRegions": {
        "0x10000000": 4096
    },
    "MemoryData": {
        "0x10000000": "0x12345678"
    },
    "RegData": {
        "RAX": "0x12345678"
    }
}
%endif

mov rax, [0x10000000]
nop