# Benchmarks: `yolowsyntax` vs C `tree-sitter`

Compares `yolowsyntax` (pure Rust, `#![forbid(unsafe_code)]`) against the official C `tree-sitter` runtime (`tree-sitter` 0.26.13 + upstream grammar crates) on official grammar test corpora, 64 KiB parse throughput, outline extraction overhead, cold table load latency, and single-process peak RSS.

## Environment

- CPU: Intel Core i7-10700 (pinned to CPU 4 via `taskset -c 4`)
- OS: Linux 7.0.0-43-generic x86_64
- Rust: `rustc 1.99.0` (`--release`, `lto = "fat"`, `codegen-units = 1`)
- Reference crates: `tree-sitter = 0.26.13`, `tree-sitter-c = 0.24.2`, `tree-sitter-go = 0.25.0`, `tree-sitter-java = 0.23.5`, `tree-sitter-python = 0.25.0`, `tree-sitter-javascript = 0.25.0`, `tree-sitter-typescript = 0.23.2`, `tree-sitter-rust = 0.24.2`, `tree-sitter-bash = 0.25.1`

## Reproduction

```bash
# 1. Fetch official test/corpus files at the exact commits recorded in ../THIRD-PARTY.md
python3 benchmarks/fetch_corpus.py

# 2. Build comparison runner (independent Cargo project under benchmarks/compare)
cd benchmarks/compare && cargo build --release

# 3. Official corpus pass rates
taskset -c 4 ./target/release/yolowsyntax-compare corpus ../corpus

# 4. 64 KiB throughput & latency (25 alternating rounds, median)
taskset -c 4 ./target/release/yolowsyntax-compare bench ../corpus 64 25

# 5. Single-process peak RSS (VmHWM in KiB)
python3 ../run_rss.py
```

## Results

### 1. Official Corpus Pass Rate (higher is better)

| Language | Cases | `yolowsyntax` | C `tree-sitter` | Notes |
|---|---:|---:|---:|---|
| C | 85 | **85 / 85 (100.00%)** | 85 / 85 (100.00%) | Exact S-expression match |
| Go | 67 | **67 / 67 (100.00%)** | 67 / 67 (100.00%) | Exact S-expression match |
| Java | 108 | **108 / 108 (100.00%)** | 108 / 108 (100.00%) | Exact S-expression match |
| Python | 117 | **117 / 117 (100.00%)** | 25 / 117 (21.37%)* | \*`tree-sitter-python` 0.25.0 + `tree-sitter` 0.26.13 `to_sexp()` wraps statements in `(expression_statement ...)`, whereas `yolowsyntax` matches the `26855ea` corpus verbatim |
| JavaScript | 116 | **116 / 116 (100.00%)** | 116 / 116 (100.00%) | Exact S-expression match |
| TypeScript | 111 | **111 / 111 (100.00%)** | 111 / 111 (100.00%) | 1 `:language(tsx)` case skipped |
| Rust | 151 | **151 / 151 (100.00%)** | 151 / 151 (100.00%) | Exact S-expression match |
| Bash | 100 | **100 / 100 (100.00%)** | 100 / 100 (100.00%) | Exact S-expression match |
| **Total** | **855** | **855 / 855 (100.00%)** | — | — |

### 2. Parse Speed & Outline Overhead (~64 KiB inputs, median of 25 rounds)

| Language | Bytes | Cold Table Load (ms ↓) | `yolowsyntax` parse (ms ↓) | C `tree-sitter` parse (ms ↓) | `yolowsyntax` `outline` (ms ↓) | `yolowsyntax` (MB/s ↑) | C `tree-sitter` (MB/s ↑) | Speed Ratio (`ts / yolow`) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| C | 65,614 | 1.863 | **12.905** | 22.637 | 14.735 | **4.85** | 2.76 | **1.75×** |
| Go | 65,623 | 0.547 | 13.507 | **11.781** | 15.754 | 4.63 | **5.31** | 0.87× |
| Java | 65,597 | 0.925 | 11.546 | **9.595** | 13.109 | 5.42 | **6.52** | 0.83× |
| Python | 65,699 | 0.971 | 16.254 | **16.252** | 18.493 | 3.85 | **3.86** | 1.00× |
| JavaScript | 65,647 | 0.675 | **13.872** | 14.051 | 15.785 | **4.51** | 4.46 | **1.01×** |
| TypeScript | 65,615 | 2.157 | 13.435 | **12.657** | 15.492 | 4.66 | **4.94** | 0.94× |
| Rust | 65,557 | 2.669 | 14.745 | **14.511** | 16.948 | 4.24 | **4.31** | 0.98× |
| Bash | 65,979 | 2.721 | **22.400** | 27.404 | 24.336 | **2.81** | 2.30 | **1.22×** |

### 3. Single-Process Peak RSS (`VmHWM`, KiB ↓)

| Language | `yolowsyntax` (KiB ↓) | C `tree-sitter` (KiB ↓) |
|---|---:|---:|
| C | 12,700 | **5,968** |
| Go | 13,484 | **6,336** |
| Java | 12,172 | **6,080** |
| Python | 15,092 | **7,644** |
| JavaScript | 14,072 | **6,672** |
| TypeScript | 13,756 | **7,480** |
| Rust | 15,788 | **7,312** |
| Bash | 16,452 | **8,860** |

*Note*: Both runs execute inside the `yolowsyntax-compare` binary (which links all 41 compressed `.pslg.z` tables plus the 8 C grammar crates). The ~6–8 MiB difference reflects `yolowsyntax` lazily inflating its `.pslg.z` table into an owned heap `Language` on first use (`OnceLock`), whereas the C grammar crates place their parse tables directly in static `.rodata`.
