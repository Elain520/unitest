%ifdef CONFIG
{
    "RegData": {
        "RDX": "0xc"
    }
}
%endif

mov rdx, 0xc0
shr rdx, 4

int 0xCC