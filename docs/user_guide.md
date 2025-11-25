# x86汇编测试框架用户指南

## 目录
1. [简介](#简介)
2. [安装](#安装)
3. [快速开始](#快速开始)
4. [命令行参数](#命令行参数)
5. [汇编文件格式](#汇编文件格式)
6. [配置选项](#配置选项)
7. [示例](#示例)
8. [故障排除](#故障排除)

## 简介

x86汇编测试框架是一个用于测试和验证x86汇编代码的工具。它可以直接在原生x86环境下执行汇编代码，并捕获执行后的寄存器状态，为rv环境提供基准测试数据。

### 主要特性
- 直接在原生x86环境下执行汇编代码
- 精确控制执行环境（使用ptrace和mmap）
- 捕获完整的寄存器状态（包括通用寄存器和XMM寄存器）
- 支持32位和64位执行模式
- 支持自定义内存区域和数据初始化
- 使用int3断点精确控制执行流程

## 安装

### 系统要求
- Linux操作系统（支持x86/x64架构）
- Rust 1.56或更高版本
- NASM汇编器
- x86_64-linux-gnu-ld链接器

### 安装步骤

1. 克隆项目仓库：
```bash
git clone <repository-url>
cd unit-test
```

2. 安装依赖：
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install nasm binutils

# CentOS/RHEL/Fedora
sudo yum install nasm binutils
# 或者对于较新版本
sudo dnf install nasm binutils
```

3. 构建项目：
```bash
cargo build --release
```

4. （可选）将可执行文件复制到系统路径：
```bash
sudo cp target/release/x86-asm-test /usr/local/bin/
```

## 快速开始

### 创建一个简单的测试文件

创建一个名为`simple_test.asm`的文件：

```asm
%ifdef CONFIG
{
    "RegData": {
        "RAX": "0x000000000000000f"
    }
}
%endif

mov rax, 5
mov rbx, 10
add rax, rbx

```

### 执行测试

```bash
unit-test --test simple_test.asm
```

执行后，程序会输出寄存器状态，并生成包含结果的`result.asm`文件。

## 命令行参数

### 基本参数

| 参数 | 简写 | 描述 | 示例 |
|------|------|------|------|
| `--test` | `-t` | 指定要测试的汇编文件 | `--test test.asm` |
| `--include` | `-i` | 指定汇编文件的包含路径 | `--include /path/to/includes` |
| `--output` | `-o` | 指定输出文件路径 | `--output result.asm` |
| `--verbose` | `-v` | 显示详细执行信息 | `--verbose` |
| `--quiet` | `-q` | 静默模式，不显示任何输出 | `--quiet` |
| `--help` | `-h` | 显示帮助信息 | `--help` |
| `--version` | `-V` | 显示版本信息 | `--version` |

### 使用示例

```bash
# 基本使用
x86-asm-test --test my_test.asm

# 指定输出文件
x86-asm-test --test my_test.asm --output my_result.asm

# 显示详细信息
x86-asm-test --test my_test.asm --verbose

# 静默执行
x86-asm-test --test my_test.asm --quiet
```

## 汇编文件格式

### 基本结构

汇编测试文件由两部分组成：
1. **配置块**：使用`%ifdef CONFIG`和`%endif`包围的JSON配置
2. **汇编代码**：标准的x86汇编代码

### 配置块

配置块是一个JSON对象，包含以下字段：

```asm
%ifdef CONFIG
{
    "RegInit": {
        "RAX": "0x123456789abcdef0",
        "RBX": "0x0fedcba987654321"
    },
    "RegData": {
        "RAX": "0x000000000000000f",
        "RBX": "0x0000000000000000"
    },
    "Mode": "32BIT",
    "MemoryRegions": {
        "0x10000000": 4096,
        "0x20000000": "0x2000"
    },
    "MemoryData": {
        "0x10000000": "0x12345678 0x9abcdef0"
    }
}
%endif

; 汇编代码
mov rax, rbx
add rax, 0x10000000

```

## 配置选项

### RegData
指定执行后预期的寄存器状态。在没有Mode字段时，默认为64位寄存器；如果MODE = 32BIT的时候需要考虑32位寄存器，Flags字段为EFLAGS的最终结果。

### RegInit
指定执行前的寄存器初始值。如果没有该字段，则默认为0。

### Mode
指定执行模式：
- `"32BIT"`：32位执行模式
- 省略：64位执行模式（默认）

### MemoryRegions
指定需要预先申请的内存区域：
- 键：内存起始地址（十六进制字符串）
- 值：内存区域大小（可以是数字或十六进制字符串）

### MemoryData
指定内存数据，为申请的内存中填入对应的数据：
- 键：内存地址（十六进制字符串）
- 值：要写入的数据（十六进制字符串，可以用空格分隔多个值）

## 示例

### 示例1：基本寄存器测试

```asm
%ifdef CONFIG
{
    "RegData": {
        "RAX": "0x000000000000000f"
    }
}
%endif

mov rax, 5
mov rbx, 10
add rax, rbx

nop
```

### 示例2：带初始寄存器值的测试

```asm
%ifdef CONFIG
{
    "RegInit": {
        "RAX": "0x0000000000000005",
        "RBX": "0x000000000000000a"
    },
    "RegData": {
        "RAX": "0x000000000000000f"
    }
}
%endif

add rax, rbx

nop
```

### 示例3：32位模式测试

```asm
%ifdef CONFIG
{
    "Mode": "32BIT",
    "RegInit": {
        "EAX": "0x12345678",
        "EBX": "0x87654321"
    },
    "RegData": {
        "EAX": "0x99999999"
    }
}
%endif

add eax, ebx

nop
```

### 示例4：内存操作测试

```asm
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
```

### 示例5：XMM寄存器测试

```asm
%ifdef CONFIG
{
    "RegData": {
        "XMM0": ["0x0000000000000000", "0x0000000000000000"]
    }
}
%endif

pxor xmm0, xmm0

nop
```

## 故障排除

### 常见问题

#### 1. "无法找到NASM"错误
**问题**：系统提示找不到NASM汇编器。
**解决方案**：确保已安装NASM并添加到PATH环境变量中。

#### 2. "权限被拒绝"错误
**问题**：执行时出现权限错误。
**解决方案**：确保运行用户有足够的权限，或者使用sudo运行。

#### 3. "无法分配内存"错误
**问题**：无法分配指定的内存区域。
**解决方案**：检查系统内存是否充足，或者调整MemoryRegions配置。

#### 4. "段错误"错误
**问题**：汇编代码执行时出现段错误。
**解决方案**：检查汇编代码是否正确，避免访问非法内存地址。

### 调试技巧

1. 使用`--verbose`参数查看更多执行信息
2. 检查生成的`result.asm`文件了解实际执行结果
3. 使用gdb等调试工具分析问题

### 性能优化

1. 对于大量测试用例，考虑使用批处理脚本
2. 合理设置MemoryRegions大小，避免浪费内存
3. 使用相对较小的代码段以提高执行效率

## 联系方式

如有问题或建议，请联系项目维护者：
- 邮箱：chihao@iscas.ac.cn