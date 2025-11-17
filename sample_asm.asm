%ifdef CONFIG
{
    "RegData": {
        "RDX": "0xc"
    },
    "MemoryRegions": {
        "0x10000000": 4096,
        "0x20000000": "0x2000"
      },
      "MemoryData": {
        "0x10000000" : "9abcdef0 12345678",
        "0x10000020" : "fa aa 55 33",
        "0x10000038" : "0x123456789"
      }
}
%endif
;1
;mov rdx, 0xc0
;shr rdx, 4

;2
mov rax,0xdeadbeefcafebabe
push rax
pop rdx

;3
;mov rax,0x4040000040400000
;movq xmm0,rax
;mov rax,0x408000003f800000
;movq xmm1,rax
;punpcklqdq xmm0,xmm1
;shufps xmm0,xmm0,0x1b

;4
;mov rax, 0x7FFFFFFFFFFFFFFF
;add rax, 0x1