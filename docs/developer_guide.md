# x86汇编测试框架开发者指南

## 目录
1. [项目架构](#项目架构)
2. [模块介绍](#模块介绍)
3. [开发环境搭建](#开发环境搭建)
4. [代码规范](#代码规范)
5. [测试](#测试)
6. [贡献指南](#贡献指南)

## 项目架构

### 整体架构
```
x86-asm-test/
├── src/                 # 源代码目录
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
├── Cargo.toml         # 项目配置文件
├── README.md           # 项目说明
└── LICENSE             # 许可证文件
```

### 核心执行流程
1. **解析阶段**：CLI → Parser → 配置提取
2. **编译阶段**：Compiler（NASM）→ Linker（系统链接器）→ ELF文件
3. **执行阶段**：Executor（父子进程+ptrace）→ 寄存器状态捕获
4. **结果阶段**：生成结果文件

## 模块介绍

### CLI模块（cli.rs）
处理命令行参数解析和用户输入。

**主要功能**：
- 使用clap库解析命令行参数
- 提供友好的用户界面
- 支持多种输出模式（详细/静默）

### Parser模块（parser.rs）
解析包含JSON配置的汇编文件。

**主要功能**：
- 提取CONFIG块中的JSON配置
- 解析汇编代码部分
- 验证配置有效性

### Compiler模块（compiler.rs）
集成NASM汇编器编译汇编代码。

**主要功能**：
- 调用NASM编译汇编文件
- 支持32位和64位编译模式
- 处理编译错误

### Linker模块（linker.rs）
集成系统链接器生成可执行文件。

**主要功能**：
- 调用系统链接器链接目标文件
- 生成可执行的ELF文件
- 处理链接错误

### ELF模块（elf.rs）
解析ELF文件，提取代码段和数据段信息。

**主要功能**：
- 使用goblin库解析ELF文件
- 提取代码段和数据段
- 解析符号表信息

### Executor模块（executor.rs）【核心】
在原生x86环境下执行代码，使用父子进程协作模型和ptrace系统调用。

**主要功能**：
- 创建父子进程执行环境
- 使用ptrace系统调用控制子进程执行
- 使用mmap分配固定地址内存
- 精确控制执行流程（使用int3断点）
- 捕获完整的寄存器状态
- 支持XMM/YMM寄存器状态捕获

### Error模块（error.rs）
统一的错误处理机制。

**主要功能**：
- 定义项目专用错误类型
- 使用thiserror库简化错误处理
- 提供友好的错误信息

### Types模块（types.rs）
定义项目中使用的数据结构。

**主要功能**：
- 定义配置数据结构
- 定义寄存器数据结构
- 定义执行结果数据结构

## 开发环境搭建

### 系统要求
- Linux操作系统（推荐Ubuntu 20.04+或CentOS 8+）
- Rust 1.56或更高版本
- NASM汇编器
- x86_64-linux-gnu-ld链接器
- Git版本控制工具

### 安装步骤

1. 安装Rust：
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

2. 安装依赖：
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install nasm binutils git

# CentOS/RHEL/Fedora
sudo yum install nasm binutils git
# 或者对于较新版本
sudo dnf install nasm binutils git
```

3. 克隆项目：
```bash
git clone <repository-url>
cd unit-test
```

4. 构建项目：
```bash
cargo build
```

### IDE推荐
- **VS Code**：安装Rust Analyzer插件
- **IntelliJ IDEA**：安装Rust插件
- **Vim/Neovim**：配置rust.vim插件

## 代码规范

### Rust编码规范
遵循Rust官方编码规范和社区最佳实践：

1. **命名约定**：
   - 模块和变量：snake_case
   - 类型和特征：PascalCase
   - 常量：SCREAMING_SNAKE_CASE

2. **文档注释**：
   - 所有公共函数和类型都需要文档注释
   - 使用`//!`为模块添加文档
   - 使用`///`为函数和类型添加文档

3. **错误处理**：
   - 优先使用`Result<T, E>`而非`panic!`
   - 自定义错误类型继承`std::error::Error`
   - 使用`thiserror`简化错误定义

4. **测试**：
   - 每个模块都应包含单元测试
   - 使用集成测试验证端到端功能
   - 保持测试覆盖率在80%以上

### 代码组织
```rust
//! 模块描述
//!
//! 更详细的模块说明

use std::collections::HashMap;

/// 结构体/枚举说明
#[derive(Debug)]
pub struct MyStruct {
    /// 字段说明
    field: String,
}

impl MyStruct {
    /// 方法说明
    /// 
    /// # 参数
    /// * `param` - 参数说明
    /// 
    /// # 返回值
    /// 返回值说明
    /// 
    /// # 示例
    /// ```
    /// let instance = MyStruct::new();
    /// ```
    pub fn new() -> Self {
        Self {
            field: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let instance = MyStruct::new();
        assert_eq!(instance.field, "");
    }
}
```

## 测试

### 测试策略
采用分层测试策略：

1. **单元测试**：针对每个模块的独立功能
2. **集成测试**：验证模块间的协同工作
3. **端到端测试**：完整的执行流程验证

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test cli

# 运行测试并显示输出
cargo test -- --nocapture

# 运行测试并生成覆盖率报告
cargo tarpaulin --out Html
```

### 编写测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = "test";
        
        // Act
        let result = my_function(input);
        
        // Assert
        assert_eq!(result, "expected");
    }

    #[test]
    fn test_function_with_error() {
        // Arrange
        let input = "invalid";
        
        // Act
        let result = my_function(input);
        
        // Assert
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "expected error message");
    }
}
```

## 技术细节

### 执行引擎详解

#### 父子进程模型
```
父进程 (控制进程)          子进程 (执行进程)
    │                         │
    ├─ ptrace(TRACE_ME) ─────► 允许父进程控制
    │                         │
    ◄── raise(SIGSTOP) ───────┤ 停止等待父进程附加
    │                         │
    ├─ ptrace(ATTACH) ────────► 附加到子进程
    │                         │
    ├─ ptrace(SETREGS) ──────► 设置初始寄存器状态
    │                         │
    ├─ ptrace(CONT) ─────────► 继续执行
    │                         │
    ◄───────────────────────── raise(SIGTRAP) 遇到断点
    │                         │
    ├─ ptrace(GETREGS) ──────► 获取寄存器状态
    │                         │
    └─ ptrace(DETACH) ───────► 分离进程
```

#### 内存管理
使用mmap系统调用分配固定地址内存：
- 代码段：0xC0000000
- 栈段：0xE0000000
- 用户定义内存区域：根据配置分配

#### 断点控制
使用int3指令（0xCC）作为断点：
1. 第一个断点：代码开始处，用于父进程附加
2. 第二个断点：代码结束处（nop指令替换为int3），用于捕获最终状态

### 安全考虑

#### 进程隔离
- 使用父子进程模型隔离执行环境
- 子进程只能通过ptrace接受父进程控制
- 严格的内存访问控制

#### 内存保护
- 使用mprotect设置内存保护属性
- 只在必要时设置可执行权限
- 执行完毕后立即撤销可执行权限

#### 代码验证
- 执行前验证代码安全性
- 限制可执行代码大小
- 监控异常系统调用

## 性能优化

### 关键优化点

#### 内存分配优化
- 使用mmap预分配固定地址内存
- 避免频繁的内存分配和释放
- 合理设置内存区域大小

#### 执行控制优化
- 减少ptrace调用次数
- 批量处理寄存器状态读取
- 优化断点设置和清除

#### 并行处理
- 支持多测试用例并行执行
- 使用线程池管理并发任务
- 实现结果缓存机制

## 未来发展方向

### 功能增强
1. **调试支持**：集成调试器功能
2. **可视化**：提供图形界面
3. **云执行**：支持在云端执行测试
4. **多架构**：支持ARM等其他架构

### 性能提升
1. **JIT优化**：使用JIT技术加速执行
2. **缓存机制**：实现智能缓存减少重复执行
3. **分布式执行**：支持分布式测试执行

### 生态建设
1. **包管理**：建立测试用例包管理系统
2. **社区支持**：建立开发者社区
3. **文档完善**：持续完善文档和教程

## 联系方式

如有问题或建议，请联系项目维护者：
- 邮箱：chihao@iscas.ac.cn