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

# 3. Official corpus pass rates (8-lang 1v1 and all 41 languages)
taskset -c 4 ./target/release/yolowsyntax-compare corpus ../corpus
taskset -c 4 ./target/release/yolowsyntax-compare all-corpus ../corpus

# 4. 64 KiB throughput & latency
taskset -c 4 ./target/release/yolowsyntax-compare bench ../corpus 64 25
taskset -c 4 ./target/release/yolowsyntax-compare all-bench ../corpus 64 15

# 5. Single-process peak RSS (VmHWM in KiB)
python3 ../run_rss.py
python3 ../run_rss.py --all
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

| Language | Bytes | Cold Table Load (ms ↓) | `yolowsyntax` parse (ms ↓) | C `tree-sitter` parse (ms ↓) | `yolowsyntax` `outline` (ms ↓) | `yolowsyntax` (MB/s ↑) | C `tree-sitter` (MB/s ↑) | Speed Ratio (`ts / yolow` ↑) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| C | 65,614 | 1.269 | **5.243** | 21.504 | 6.571 | **11.93** | 2.91 | **4.10×** |
| Go | 65,623 | 0.346 | **6.129** | 11.899 | 7.324 | **10.21** | 5.26 | **1.94×** |
| Java | 65,597 | 0.525 | **3.966** | 9.616 | 5.278 | **15.77** | 6.51 | **2.42×** |
| Python | 65,699 | 0.570 | **7.392** | 16.342 | 9.261 | **8.48** | 3.83 | **2.21×** |
| JavaScript | 65,647 | 0.438 | **7.022** | 14.349 | 8.373 | **8.92** | 4.36 | **2.04×** |
| TypeScript | 65,615 | 1.626 | **5.314** | 12.188 | 6.872 | **11.77** | 5.13 | **2.29×** |
| Rust | 65,557 | 1.148 | **5.483** | 14.602 | 7.418 | **11.40** | 4.28 | **2.66×** |
| Bash | 65,979 | 1.435 | **13.515** | 27.234 | 15.256 | **4.66** | 2.31 | **2.02×** |

### 3. Single-Process Peak RSS (`VmHWM`, KiB ↓)

| Language | `yolowsyntax` (KiB ↓) | C `tree-sitter` (KiB ↓) |
|---|---:|---:|
| C | 9,400 | **6,028** |
| Go | 9,476 | **6,452** |
| Java | 9,256 | **6,088** |
| Python | 10,132 | **7,828** |
| JavaScript | 9,676 | **6,824** |
| TypeScript | 10,756 | **7,456** |
| Rust | 11,060 | **7,424** |
| Bash | 11,412 | **9,108** |

*Note*: Both runs execute inside the `yolowsyntax-compare` binary (which links all 41 Zstd-compressed `.pslg.z` tables plus the 8 C grammar crates). The ~2.3–3.6 MiB difference reflects `yolowsyntax` lazily decompressing its `.pslg.z` table and building an owned heap `Language` (plus up to 1 MiB dense state LUT) on first use (`OnceLock`), whereas the C grammar crates place their parse tables directly in static `.rodata`.

### 4. All 41 Languages (`all-corpus`, `all-bench 64 15`, `run_rss.py --all`)

All **5,875 / 5,875 (100.00%)** active corpus test cases pass across all 41 grammars (`22` upstream-marked skips: `typescript` 1, `tsx` 1, `php` 2, `ocaml` 18).

| Language | Corpus Pass | Skipped | Cold Load (ms ↓) | `parse` (ms ↓) | `outline` (ms ↓) | Throughput (MB/s ↑) | Peak RSS (KiB ↓) |
|---|---:|---:|---:|---:|---:|---:|---:|
| `c` | 85 / 85 (100.00%) | 0 | 1.489 | 4.987 | 6.184 | 12.55 | 9,468 |
| `cpp` | 198 / 198 (100.00%) | 0 | 6.965 | 6.151 | 7.661 | 10.21 | 18,428 |
| `csharp` | 179 / 179 (100.00%) | 0 | 3.834 | 8.698 | 10.899 | 7.19 | 18,240 |
| `go` | 67 / 67 (100.00%) | 0 | 0.366 | 4.903 | 6.748 | 12.77 | 9,708 |
| `java` | 108 / 108 (100.00%) | 0 | 0.490 | 3.797 | 5.373 | 16.48 | 9,268 |
| `javascript` | 116 / 116 (100.00%) | 0 | 0.413 | 5.789 | 7.631 | 10.81 | 9,632 |
| `typescript` | 111 / 111 (100.00%) | 1 | 1.118 | 5.243 | 7.007 | 11.93 | 10,760 |
| `tsx` | 111 / 111 (100.00%) | 1 | 1.192 | 5.256 | 7.004 | 11.91 | 10,496 |
| `php` | 146 / 146 (100.00%) | 2 | 0.797 | 8.830 | 10.626 | 7.08 | 11,496 |
| `python` | 117 / 117 (100.00%) | 0 | 0.573 | 6.895 | 8.765 | 9.09 | 10,076 |
| `rust` | 151 / 151 (100.00%) | 0 | 1.392 | 5.228 | 6.982 | 11.96 | 10,940 |
| `lua` | 37 / 37 (100.00%) | 0 | 0.088 | 0.381 | 0.397 | 164.32 | 3,696 |
| `bash` | 100 / 100 (100.00%) | 0 | 1.401 | 13.323 | 14.812 | 4.72 | 11,268 |
| `ruby` | 291 / 291 (100.00%) | 0 | 1.713 | 0.663 | 0.833 | 94.21 | 8,904 |
| `kotlin` | 1413 / 1413 (100.00%) | 0 | 10.232 | 10.766 | 13.168 | 5.87 | 24,948 |
| `css` | 40 / 40 (100.00%) | 0 | 0.184 | 3.797 | 5.106 | 16.46 | 8,308 |
| `html` | 20 / 20 (100.00%) | 0 | 0.044 | 16.299 | 17.358 | 3.84 | 6,208 |
| `objc` | 75 / 75 (100.00%) | 0 | 2.963 | 4.847 | 6.244 | 12.90 | 13,716 |
| `dart` | 205 / 205 (100.00%) | 0 | 0.967 | 16.269 | 18.398 | 3.84 | 13,092 |
| `julia` | 63 / 63 (100.00%) | 0 | 3.018 | 7.750 | 10.400 | 8.08 | 15,116 |
| `nix` | 54 / 54 (100.00%) | 0 | 0.118 | 5.572 | 6.522 | 11.22 | 5,792 |
| `dockerfile` | 113 / 113 (100.00%) | 0 | 0.129 | 3.738 | 4.706 | 16.74 | 8,104 |
| `ocaml` | 86 / 86 (100.00%) | 18 | 4.215 | 3.162 | 3.425 | 19.77 | 15,476 |
| `elixir` | 293 / 293 (100.00%) | 0 | 3.394 | 9.291 | 11.214 | 6.73 | 15,068 |
| `r` | 89 / 89 (100.00%) | 0 | 0.595 | 29.333 | 31.323 | 2.14 | 11,632 |
| `svelte` | 45 / 45 (100.00%) | 0 | 0.076 | 33.465 | 35.135 | 1.87 | 13,472 |
| `solidity` | 126 / 126 (100.00%) | 0 | 0.586 | 2.857 | 4.434 | 21.91 | 6,756 |
| `groovy` | 87 / 87 (100.00%) | 0 | 0.652 | 6.747 | 8.737 | 9.27 | 10,652 |
| `make` | 117 / 117 (100.00%) | 0 | 0.317 | 3.151 | 4.435 | 19.88 | 8,328 |
| `scheme` | 41 / 41 (100.00%) | 0 | 0.188 | 4.299 | 6.000 | 14.54 | 9,892 |
| `commonlisp` | 42 / 42 (100.00%) | 0 | 1.215 | 5.908 | 5.670 | 10.59 | 10,004 |
| `pascal` | 88 / 88 (100.00%) | 0 | 0.616 | 4.451 | 6.151 | 14.14 | 10,096 |
| `gleam` | 79 / 79 (100.00%) | 0 | 0.469 | 7.086 | 9.234 | 8.83 | 10,340 |
| `powershell` | 165 / 165 (100.00%) | 0 | 0.936 | 5.443 | 8.413 | 11.50 | 11,420 |
| `cmake` | 68 / 68 (100.00%) | 0 | 0.115 | 6.817 | 9.143 | 9.17 | 10,592 |
| `erlang` | 230 / 230 (100.00%) | 0 | 0.399 | 16.822 | 19.103 | 3.72 | 12,344 |
| `hcl` | 102 / 102 (100.00%) | 0 | 0.337 | 5.568 | 7.493 | 11.53 | 10,208 |
| `vue` | 10 / 10 (100.00%) | 0 | 0.057 | 3.738 | 4.594 | 16.73 | 5,248 |
| `d` | 218 / 218 (100.00%) | 0 | 2.560 | 0.435 | 0.536 | 143.71 | 12,248 |
| `elm` | 119 / 119 (100.00%) | 0 | 0.444 | 1.259 | 1.437 | 49.78 | 4,960 |
| `verilog` | 70 / 70 (100.00%) | 0 | 6.069 | 7.485 | 9.919 | 8.35 | 18,756 |
| **Total (41 langs)** | **5,875 / 5,875 (100.00%)** | **22** | **0.044–10.232** | **0.381–33.465** | **0.397–35.135** | **1.87–164.32** | **3,696–24,948** |
