<div align="center">

# yolowsyntax

**100% 纯 Rust tree-sitter 解析运行时与 41 种语言代码大纲提取器（`#![forbid(unsafe_code)]`）**

[**English**](../README.md) · [**简体中文**](README.zh-CN.md) · [**性能评测**](../benchmarks/README.md) · [**更新日志**](../CHANGELOG.md)

[![Crates.io](https://img.shields.io/crates/v/yolowsyntax.svg)](https://crates.io/crates/yolowsyntax)
[![Docs.rs](https://docs.rs/yolowsyntax/badge.svg)](https://docs.rs/yolowsyntax)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)
[![Rust: 1.89+](https://img.shields.io/badge/rust-1.89%2B-orange.svg)](../Cargo.toml)

</div>

兼容 tree-sitter 语法表与外部扫描器的纯 Rust 解析运行时，从 [PageSift](https://github.com/YOLOW92/pagesift) 独立拆出并以 **MIT 协议**发布。编译期无需 C 编译器、全库 `#![forbid(unsafe_code)]`、运行时仅依赖纯 Rust 编写的 [`yolowzstd`](https://crates.io/crates/yolowzstd)（同样 `#![forbid(unsafe_code)]`），内嵌 **41 种语言**的 Zstd 压缩语法表（按需懒解压）与结构化代码大纲提取。

---

## 一览

- **零 C 工具链，100% 安全 Rust**：无需 `cc`、无需 `bindgen`、无 FFI，全库到运行时压缩依赖端到端 `#![forbid(unsafe_code)]`（运行时唯一依赖为 `yolowzstd`）。
- **41 种内嵌语言，按需 Zstd 懒解压**：内置 Zstd 压缩的 `.pslg.z` 语法表（41 门语言合计 `6.2 MiB`，`.crate` 包 `6.3 MiB`）及对应外部扫描器的纯 Rust 移植；只有实际调用到的语言才会通过 `OnceLock` 解压并缓存（冷加载 `0.04–10.23 ms`）。
- **41 门语言官方 Corpus 5,875 / 5,875（100.00%）全通过**：覆盖全部 41 门语言的官方 `test/corpus` 达到 100.00% 通过（`22` 条上游标记跳过）；在 8 门 1v1 对比语言上，`64 KiB` 解析速度全面达到原生 C `tree-sitter` 运行时的 **1.94×–4.10×**。
- **开箱即用的代码大纲提取（`outline`）**：把源码无损切分为顶层 `sections`（所有片段拼接严格等于原文），并提取 `symbols`（函数、方法、结构体、类、枚举、Trait、接口、类型）与 `imports`，适合 RAG 与文档代码分块。
- **有界资源保护（`Limits`）**：每次解析都受最大栈深、GLR 版本数、节点数、步数预算与墙钟超时约束，面对畸形或恶意深嵌套输入会安全返回 `truncated` 树而不会崩溃或内存失控。

---

## 缘起

在 [PageSift](https://github.com/YOLOW92/pagesift) 为 LLM 和 RAG 管线接入源代码抽取时，我们希望按真实语法边界（顶层函数、类、结构体、Trait、导入语句）来切分代码文件，而不是生硬地按行数截断。

如果直接走传统 C 版 `tree-sitter` 生态，意味着构建机必须装好 C 编译器（`cc`），跨平台交叉编译几十个原生 `parser.c` 与 `scanner.c` 编译单元，同时把来自公网的不可信文件送进 C FFI——遇到深嵌套或异常输入时，线程安全与稳定性的边界很难控住。

但我们并不需要 IDE 编辑器那样的增量编辑，我们需要的是一个**一次解析、沙箱可控、零 C 依赖的纯 Rust 运行时**，能在 Linux、macOS 和 Windows 的高并发管线里稳定跑起来。

于是有了 `yolowsyntax`：
1. **离线语法表转换器（`ts-convert`）**：从上游各语法的 `parser.c` 离线生成紧凑的 Zstd `.pslg.z` 压缩表，运行时仅在首次用到某种语言时由 `yolowzstd` 懒解压。
2. **安全 GLR 运行时（`#![forbid(unsafe_code)]`）**：实现词法扫描 ASCII 快路径、O(1) 稠密状态直查表、错误恢复、GLR 版本合并与裁剪，以及各语言外部扫描器的纯 Rust 移植，全程由 `Limits` 硬性兜底。
3. **无损代码大纲层（`outline`）**：提取顶层符号、导入声明，以及与原文字节一一对应的 `sections`。

现在这套能力已从 PageSift（`pagesift-syntax`）独立并改按 **MIT** 开源，任何 Rust 项目都可以直接引入 41 种语言的语法解析能力，而不必碰一行 C 代码。

---

## 安装

```toml
[dependencies]
yolowsyntax = "0.1"
```

如需离线语法表转换及 corpus/parse 命令行工具（`ts-convert`、`ts-corpus`、`ts-parse`）：

```toml
[dependencies]
yolowsyntax = { version = "0.1", features = ["tools"] }
```

---

## 评测摘要

在 Intel Core i7-10700（`taskset -c 4` 绑核）、Linux x86-64、`rustc 1.99.0`（`--release`, `lto = "fat"`, `codegen-units = 1`）环境下，对比 `tree-sitter 0.26.13` 与对应官方 C 语法 crate（`~64 KiB` 输入，25 轮交替运行取中位数；41 门语言全量 Corpus `5,875 / 5,875` 全部通过；完整脚本、内存数据与 41 语言全量成绩见 [`benchmarks/README.md`](../benchmarks/README.md)）：

| 语言 | 官方 Corpus 通过率 | `yolowsyntax` 解析 (ms ↓) | C `tree-sitter` 解析 (ms ↓) | `yolowsyntax` `outline` (ms ↓) | 速度比 (`ts / yolow` ↑) |
|---|---:|---:|---:|---:|---:|
| C | **85 / 85 (100%)** | **5.243** | 21.504 | 6.571 | **4.10×** |
| Go | **67 / 67 (100%)** | **6.129** | 11.899 | 7.324 | **1.94×** |
| Java | **108 / 108 (100%)** | **3.966** | 9.616 | 5.278 | **2.42×** |
| Python | **117 / 117 (100%)** | **7.392** | 16.342 | 9.261 | **2.21×** |
| JavaScript | **116 / 116 (100%)** | **7.022** | 14.349 | 8.373 | **2.04×** |
| TypeScript | **111 / 111 (100%)** | **5.314** | 12.188 | 6.872 | **2.29×** |
| Rust | **151 / 151 (100%)** | **5.483** | 14.602 | 7.418 | **2.66×** |
| Bash | **100 / 100 (100%)** | **13.515** | 27.234 | 15.256 | **2.02×** |

---

## 许可证

MIT License — 详见 [LICENSE](../LICENSE)、[THIRD-PARTY.md](../THIRD-PARTY.md) 与 [UPSTREAM.md](../UPSTREAM.md)。
