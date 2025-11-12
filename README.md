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

```bash
# 基本用法
./target/release/x86-asm-test --test <assembly_file.asm>

# 带包含路径
./target/release/x86-asm-test --test <assembly_file.asm> --include <include_path>

# 生成包含执行结果的输出文件
./target/release/x86-asm-test --test <input.asm> > <output.asm>
```

## 汇编文件格式

汇编文件遵循以下结构：

```asm
%ifdef CONFIG
{
    "Mode": "32BIT",
    "MemoryRegions": {
        "0x10000000": 4096,
        "0x20000000": "0x2000"
    },
    "MemoryData": {
        "0x1000000": ["0x1234567", 1, 3, 5]
    }
}
%endif

; 您的汇编代码
mov rdx, 0xc0
shr rdx, 4

; 以hlt指令结束
hlt
```

## 配置说明

### RegData
在输入文件中应忽略此字段。在输出文件中，此字段将包含执行后的寄存器状态。

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
- 值: 要写入的数据数组

## 开发指南

### 项目结构

```
.
├── src/                 # Rust源代码
├── tests/               # 测试文件
├── samples/             # 示例汇编文件
├── Cargo.toml           # 项目配置文件
├── README.md            # 项目说明文档
└── DEVELOPMENT_PLAN.md  # 开发计划文档
```

### 构建和测试

```bash
# 运行测试
cargo test

# 运行特定测试
cargo test <test_name>

# 检查代码格式
cargo fmt --check

# 运行代码检查
cargo clippy
```

## 贡献

欢迎贡献代码、报告问题或提出改进建议。请遵循以下步骤：

1. Fork项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启Pull Request

## 许可证

本项目采用MIT许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 联系方式

项目维护者: chihao@iscas.ac.cn