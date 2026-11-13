<div align="center">

# yolowsyntax

**100% 纯 Rust tree-sitter 解析运行时与 41 种语言代码大纲提取器（`#![forbid(unsafe_code)]`）**

[**English**](../README.md) · [**简体中文**](README.zh-CN.md) · [**性能评测**](../benchmarks/README.md) · [**更新日志**](../CHANGELOG.md)

[![Crates.io](https://img.shields.io/crates/v/yolowsyntax.svg)](https://crates.io/crates/yolowsyntax)
[![Docs.rs](https://docs.rs/yolowsyntax/badge.svg)](https://docs.rs/yolowsyntax)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)
[![Rust: 1.89+](https://img.shields.io/badge/rust-1.89%2B-orange.svg)](../Cargo.toml)

</div>

兼容离线转换语法表与纯 Rust 移植外部扫描器的解析运行时，从 [PageSift](https://github.com/YOLOW92/pagesift) 独立拆出并以 **MIT 协议**开源（内嵌上游语法与扫描器的 MIT / Apache-2.0 版权归属见 [THIRD-PARTY.md](../THIRD-PARTY.md)）。编译期无需 C 编译器、全库 `#![forbid(unsafe_code)]`、运行时仅依赖纯 Rust 编写的 [`yolowzstd`](https://crates.io/crates/yolowzstd) `0.1.2`（同样 `#![forbid(unsafe_code)]`），内嵌 **41 种语言**的 Zstd 压缩语法表（按需懒解压）与结构化代码大纲提取。

---

## 一览

- **零 C 工具链，100% 安全 Rust**：无需 `cc`、`bindgen` 或 FFI，从运行库到 Zstd 解压依赖端到端 `#![forbid(unsafe_code)]`；可在支持标准 `std` 的 Rust 目标上无 C 依赖编译（MSRV `1.89+`，已在 Linux x86-64 验证）。
- **41 种内嵌语言，按需 Zstd 懒解压**：内置 Zstd 压缩的 `.pslg.z` 语法表（41 门语言合计 `6.22 MiB`，`.crate` 包 `6.28 MiB`）及对应外部扫描器的纯 Rust 移植，面向一次性（`&[u8]`）解析；只有实际调用到的语言才会通过 `OnceLock` 解压并缓存（冷加载 `0.068–11.781 ms`）。
- **定位与边界**：支持的是这 **41 门内嵌语法** 的离线转换表与移植扫描器，不等同于完整兼容 C 版 `tree-sitter` 全部 API，不支持动态加载任意第三方 C `parser.c` / 外部扫描器、Query 查询语言或增量编辑重解析（`tree.edit`）。
- **官方 Corpus 5,875 / 5,875（100.00%）活跃用例全通过**：覆盖 41 门固定版本上游 `test/corpus` 的 `5,875` 条活跃用例全部通过（`0` 失败；含 `5,844` 条 S 表达式精确匹配与 `31` 条 `:error` 错误树校验）；`22` 条因多方言共享测试集里的 `:language(...)` 不匹配而跳过（`typescript` 1、`tsx` 1、`php` 2、`ocaml` 18），另跑测器也会跳过 `:skip`、`:cst` 与非当前平台 `:platform(...)` 用例（当前 41 套中均为 `0` 条）。
- **代码大纲提取（`outline`）**：对 32 门带符号定义规则的语言提取 `symbols`、`imports` 与无损拼接的 `sections`；其余 9 门标记/配置语法仍可通过 `api::parse()` 解析语法树，但 `outline()` 会直接返回 `None`。
- **协作式资源预算（`Limits`）**：在解析热路径与词法/扫描器中协作式检查最大栈深、GLR 版本数（默认 `max_versions = 6`）、节点数（默认 `4,000,000`）、步数预算与可选墙钟超时（默认 `max_millis = 0` 表示关闭），超限时丢弃大 Arena 并返回极小的 `truncated` 退化树。注：这属于单进程内的协作式预算控制，**不是**操作系统或 cgroup 级别的内存隔离，也不能承诺所有任意输入在调用前或外部分配阶段绝对不会触发 OOM。

---

## 缘起

在 [PageSift](https://github.com/YOLOW92/pagesift) 为 LLM 和 RAG 管线接入源代码抽取时，我们希望按真实语法边界（顶层函数、类、结构体、Trait、导入语句）切分代码文件，而不是生硬地按行数截断。

传统 C 版 `tree-sitter` 生态需要构建机配备 C 编译器、链接几十个 `parser.c` 与 `scanner.c` 编译单元；而我们不需要 IDE 式的增量编辑，只需要一个**一次性解析、可设预算边界、零 C 依赖的纯 Rust 运行时**。

于是有了 `yolowsyntax`：
1. **离线语法表转换器（`ts-convert`）**：从上游各语法的 `parser.c` 离线生成紧凑的 Zstd `.pslg.z` 压缩表，运行时仅在首次用到某种语言时由 `yolowzstd` 懒解压。
2. **安全 GLR 运行时（`#![forbid(unsafe_code)]`）**：实现词法 ASCII 快路径、O(1) 稠密状态直查表、错误恢复、GLR 版本合并与裁剪、外部扫描器纯 Rust 移植以及协作式 `Limits`。
3. **无损代码大纲层（`outline`）**：提取顶层与类型内符号、导入声明，以及严格拼回原文的 `sections`。

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

## `outline()` 的语言适用范围与返回 `None` 条件

- **`symbols`**：提取顶层声明以及直接位于类型容器（`struct`、`class`、`trait`、`impl`、`interface`、`enum` 等）或单文件模块内的方法/构造器/字段；最多保留 `2,000` 个符号。
- **`imports`**：提取导入/引用声明；最多保留 `500` 条。
- **`sections`**：按顶层符号锚点切成首尾相接的字节区间（前置非符号内容 `title: None`），返回 `Some` 时按顺序拼接严格等于原源码。
- **返回 `None` 的条件**：未知语言标记、空输入或超过 `2 MiB`、解析超限或超过内置 `3,000 ms` 协作超时（`truncated`）、错误节点字节占比超过 `20%`、9 门无符号提取规则的语法（`html`、`css`、`vue`、`svelte`、`nix`、`dockerfile`、`make`、`scheme`、`hcl`），或最终未提取到任何 `symbols`（例如纯表达式文件、或全部定义包在顶层 IIFE / `{ ... }` 语句块内）。

---

## 评测摘要

在 **Intel Core i7-10700**（`taskset -c 4` 绑核）、**Linux 7.0.0-43-generic x86-64**、**`rustc 1.99.0`**（`--release`, `lto = "fat"`, `codegen-units = 1`）环境下，对比 **`tree-sitter 0.26.13`** 与官方 C 语法 crate（25 轮交替运行取中位数，并检验 `has_error()` 与 S 表达式等价性；RSS `3,700–24,948 KiB` 与 41 语言完整表格见 [`benchmarks/README.md`](../benchmarks/README.md)）。

### ~64 KiB Corpus 拼接基准（历史可比基线，特定硬件与合成负载）

下列 **1.95×–4.02×** 加速比测自上述硬件与配置下的 ~64 KiB 官方 `test/corpus` 片段拼接输入，不代表所有真实代码场景的普遍比例：

| 语言 | 活跃 Corpus 通过率 | `yolowsyntax` 解析 (ms ↓) | C `tree-sitter` 解析 (ms ↓) | `yolowsyntax` `outline` (ms ↓) | 速度比 (`ts / yolow` ↑) | 等价性 / `outline` |
|---|---:|---:|---:|---:|---:|---|
| C | **85 / 85 (100%)** | **5.689** | 22.847 | 7.444 | **4.02×** | `exact` / `some(361)` |
| Go | **67 / 67 (100%)** | **5.749** | 12.660 | 7.829 | **2.20×** | `exact(both_err)` / `some(724)` |
| Java | **108 / 108 (100%)** | **4.866** | 11.486 | 7.027 | **2.36×** | `exact` / `some(615)` |
| Python | **117 / 117 (100%)** | **7.532** | 16.933 | 9.819 | **2.25×** | `both_err_diff` / `some(255)` |
| JavaScript | **116 / 116 (100%)** | **5.998** | 14.808 | 8.499 | **2.47×** | `exact(both_err)` / `some(253)` |
| TypeScript | **111 / 111 (100%)** | **5.491** | 12.566 | 7.771 | **2.29×** | `exact(both_err)` / `some(902)` |
| Rust | **151 / 151 (100%)** | **5.700** | 14.827 | 8.303 | **2.60×** | `exact(both_err)` / `some(722)` |
| Bash | **100 / 100 (100%)** | **14.212** | 27.645 | 16.267 | **1.95×** | `both_err_diff` / `none` |

### 真实开源单文件对比（31 KiB – 176 KiB，`real-bench`）

| 语言（文件） | 字节 | `yolowsyntax` 解析 (ms ↓) | C `tree-sitter` 解析 (ms ↓) | `yolowsyntax` `outline` (ms ↓) | 速度比 (`ts / yolow` ↑) | 等价性 / `outline` |
|---|---:|---:|---:|---:|---:|---|
| C (`parser.c`) | 78,797 | **2.906** | 15.156 | 4.085 | **5.22×** | `exact(both_err)` / `some(59)` |
| Go (`decode.go`) | 36,112 | **2.396** | 4.921 | 3.141 | **2.05×** | `exact` / `some(38)` |
| Java (`ImmutableList.java`) | 31,280 | **0.996** | 2.260 | 1.344 | **2.27×** | `exact` / `some(85)` |
| Python (`argparse.py`) | 102,236 | **4.527** | 10.351 | 5.910 | **2.29×** | `clean(py_expr_stmt)` / `some(161)` |
| JavaScript (`moment.js`) | 176,435 | **7.956** | 20.295 | 11.176 | **2.55×** | `exact` / `none` |
| TypeScript (`types.ts`) | 153,859 | **9.250** | 21.406 | 12.542 | **2.31×** | `exact` / `some(372)` |
| Rust (`searcher/mod.rs`) | 39,771 | **1.243** | 2.880 | 1.675 | **2.32×** | `exact` / `some(72)` |
| Bash (`nvm.sh`) | 148,844 | **7.433** | 36.250 | 9.862 | **4.88×** | `exact` / `none` |

---

## 许可证

MIT License — 详见 [LICENSE](../LICENSE)、[THIRD-PARTY.md](../THIRD-PARTY.md)（含上游 MIT 与 Apache-2.0 语法表及扫描器归属声明）与 [UPSTREAM.md](../UPSTREAM.md)。
