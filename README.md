# x86汇编测试框架

## 项目概述

本项目是一个使用Rust编写的x86汇编测试框架，用于为rv环境提供基准测试数据。项目直接在原生x86环境下执行汇编代码来验证其正确性，并生成包含执行结果的输出文件。

## 功能特性

- 解析包含嵌入式JSON配置的汇编文件
- 使用NASM编译和链接汇编代码
- 在原生x86环境下执行代码（使用ptrace和mmap精确控制执行环境）
- 捕获完整的寄存器状态并生成结果文件
- 支持32位和64位执行模式
- 支持自定义内存区域和数据初始化
- 精确控制执行流程，使用int3断点确保执行在预期位置停止
- 支持XMM/YMM寄存器状态捕获

## 文档

- [用户使用指南](docs/user_guide.md) - 详细的用户使用说明
- [开发者指南](docs/developer_guide.md) - 开发者和技术文档

## 安装依赖

在使用本项目之前，请确保系统已安装以下依赖：

- Rust (推荐使用最新稳定版本)
- NASM (Netwide Assembler)
- x86_64-linux-gnu-ld (或相应的链接器)

在Ubuntu/Debian系统上，可以使用以下命令安装依赖：

```bash
# 安装Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装NASM和链接器
sudo apt update
sudo apt install nasm binutils
```

在CentOS/RHEL/Fedora系统上，可以使用以下命令安装依赖：

```bash
# 安装Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装NASM和链接器 (CentOS/RHEL)
sudo yum install nasm binutils

# 或者对于较新版本 (Fedora)
sudo dnf install nasm binutils
```

## 构建项目

```bash
# 克隆项目
git clone <repository-url>
cd x86-asm-test

# 构建项目
cargo build

# 或者构建发布版本
cargo build --release
```

## 使用方法

### 基本用法

```bash
# 基本用法
./target/release/x86-asm-test --test <assembly_file.asm>

# 带包含路径
./target/release/x86-asm-test --test <assembly_file.asm> --include <include_path>

# 指定输出文件
./target/release/x86-asm-test --test <input.asm> --output <output.asm>

# 显示详细信息
./target/release/x86-asm-test --test <input.asm> --verbose

# 静默模式
./target/release/x86-asm-test --test <input.asm> --quiet
```

### 生成包含执行结果的输出文件

```bash
# 生成包含执行结果的输出文件
./target/release/x86-asm-test --test <input.asm> > <output.asm>
```

## 汇编文件格式

汇编文件遵循以下结构：

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
        "0x10000000": "0x12345678 0x9abcdef0",
        "0x10000020": "0xfa 0xaa 0x55 0x33"
    }
}
%endif

; 您的汇编代码
mov rax, 5
mov rbx, 10
add rax, rbx

```

## 配置说明

### RegData
在输入文件中应忽略此字段。在输出文件中，此字段将包含执行后的寄存器状态。
在没有Mode字段时，默认为64位寄存器，如果MODE = 32BIT的时候需要考虑32位寄存器，Flags字段为EFLAGS的最终结果

### RegInit
在输入文件中指定执行前的寄存器初始状态，如果没有该字段，则默认为0

### Mode
执行模式：
- "32BIT": 32位执行模式
- 省略: 64位执行模式（默认）

### MemoryRegions
需要在执行前分配的内存区域：
- 键: 内存起始地址
- 值: 内存区域大小

### MemoryData
初始化内存区域的数据：
- 键: 内存地址
- 值: 要写入的数据（十六进制字符串，可以用空格分隔）

## 命令行参数

```bash
USAGE:
    x86-asm-test [OPTIONS] --test <FILE>

OPTIONS:
    -h, --help                     Print help information
    -i, --include <PATH>           Include path for assembly files
    -o, --output <FILE>            Output file path
    -q, --quiet                    Quiet mode, suppress all output
    -t, --test <FILE>              Test mode: specify the assembly file to test
    -v, --verbose                  Verbose mode: show more execution information
        --reg-init-code            RegInit mode: if set, RegInit will be initialized through code; otherwise RegInit will be converted to initialization instructions
    -V, --version                  Print version information
```

## 示例

### 简单示例

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

nop
```

执行测试：

```bash
./target/release/x86-asm-test --test simple_test.asm
```

### 带初始寄存器的示例

创建一个名为`init_reg_test.asm`的文件：

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

执行测试：

```bash
./target/release/x86-asm-test --test init_reg_test.asm
```

### 内存操作示例

创建一个名为`memory_test.asm`的文件：

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

执行测试：

```bash
./target/release/x86-asm-test --test memory_test.asm
```

## 开发指南

### 项目结构

```
.
├── src/                 # Rust源代码
│   ├── cli.rs          # 命令行参数解析
│   ├── compiler.rs     # 编译器集成（NASM）
│   ├── linker.rs       # 链接器集成（系统链接器）
│   ├── elf.rs          # ELF文件解析
│   ├── executor.rs     # 执行引擎（核心）
│   ├── parser.rs       # 汇编文件解析
│   ├── error.rs        # 错误处理
│   ├── types.rs        # 数据类型定义
│   ├── lib.rs          # 库模块入口
│   └── main.rs         # 主程序入口
├── tests/              # 测试文件
├── docs/               # 文档目录
├── sample/             # 示例文件
├── Cargo.toml          # 项目配置文件
├── README.md           # 项目说明
└── DEVELOPMENT_PLAN.md # 开发计划文档
```

### 构建和测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test parser

# 检查代码格式
cargo fmt --check

# 运行代码检查
cargo clippy

# 运行基准测试
cargo bench
```

### 项目模块说明

1. **CLI模块** (`cli.rs`)：处理命令行参数解析
2. **Parser模块** (`parser.rs`)：解析包含JSON配置的汇编文件
3. **Compiler模块** (`compiler.rs`)：集成NASM编译汇编代码
4. **Linker模块** (`linker.rs`)：集成系统链接器生成可执行文件
5. **ELF模块** (`elf.rs`)：解析ELF文件提取代码段和数据段
6. **Executor模块** (`executor.rs`)：在原生x86环境下执行代码（核心模块）
7. **Error模块** (`error.rs`)：统一错误处理
8. **Types模块** (`types.rs`)：定义项目数据结构

## 贡献

欢迎贡献代码、报告问题或提出改进建议。请遵循以下步骤：

1. Fork项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启Pull Request

### 贡献指南

请参阅[开发者指南](docs/developer_guide.md)了解更多关于项目架构、编码规范和贡献流程的信息。

## 联系方式

项目维护者: chihao@iscas.ac.cn