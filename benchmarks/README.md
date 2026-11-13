# Benchmarks: `yolowsyntax` vs C `tree-sitter`

Compares `yolowsyntax` (pure Rust, `#![forbid(unsafe_code)]`) against the official C `tree-sitter` runtime (`tree-sitter` 0.26.13 + upstream grammar crates) on official grammar test corpora, ~64 KiB corpus-concatenated inputs, standalone real-world open-source files, outline extraction overhead, cold table load latency, and single-process peak RSS.

## Environment

- CPU: Intel Core i7-10700 (pinned to CPU 4 via `taskset -c 4`)
- OS: Linux 7.0.0-43-generic x86_64
- Rust: `rustc 1.99.0` (`--release`, `lto = "fat"`, `codegen-units = 1`)
- Reference crates: `tree-sitter = 0.26.13`, `tree-sitter-c = 0.24.2`, `tree-sitter-go = 0.25.0`, `tree-sitter-java = 0.23.5`, `tree-sitter-python = 0.25.0`, `tree-sitter-javascript = 0.25.0`, `tree-sitter-typescript = 0.23.2`, `tree-sitter-rust = 0.24.2`, `tree-sitter-bash = 0.25.1`

## Reproduction

```bash
# 1. Fetch official test/corpus files (commits in ../THIRD-PARTY.md) and 8 pinned OSS source files
python3 benchmarks/fetch_corpus.py

# 2. Build comparison runner (independent Cargo project under benchmarks/compare)
cd benchmarks/compare && cargo build --release

# 3. Official corpus pass rates (8-lang 1v1 and all 41 languages)
taskset -c 4 ./target/release/yolowsyntax-compare corpus ../corpus
taskset -c 4 ./target/release/yolowsyntax-compare all-corpus ../corpus

# 4. ~64 KiB corpus-concatenated throughput (with parity and outline checks)
taskset -c 4 ./target/release/yolowsyntax-compare bench ../corpus 64 25
taskset -c 4 ./target/release/yolowsyntax-compare all-bench ../corpus 64 15

# 5. Standalone real-world OSS single-file benchmark (31 KiB - 176 KiB)
taskset -c 4 ./target/release/yolowsyntax-compare real-bench ../corpus 25

# 6. Single-process peak RSS (VmHWM in KiB)
python3 ../run_rss.py
python3 ../run_rss.py --all
```

## Results

### 1. Official Corpus Pass Rate (higher is better)

Across the 41 pinned upstream `test/corpus` suites (`5,897` parsed cases):
- **Active cases**: **5,875 / 5,875 (100.00%)** passed (`0` failed), consisting of `5,844` S-expression matches and `31` expected `:error` cases (verified via `has_error()`).
- **Skipped cases**: **22** cases skipped solely due to `:language(...)` mismatches in shared multi-dialect test directories (`typescript`: 1, `tsx`: 1, `php`: 2, `ocaml`: 18).
- **Runner skip policy**: The runner also skips `:skip`, `:cst`, and platform-excluded `:platform(...)` directives (`0` occurrences in these 41 pinned suites).

| Language | Active Cases | `yolowsyntax` | C `tree-sitter` | Skipped (`:language`) | Notes |
|---|---:|---:|---:|---:|---|
| C | 85 | **85 / 85 (100.00%)** | 85 / 85 (100.00%) | 0 | Exact S-expression match |
| Go | 67 | **67 / 67 (100.00%)** | 67 / 67 (100.00%) | 0 | Exact S-expression match |
| Java | 108 | **108 / 108 (100.00%)** | 108 / 108 (100.00%) | 0 | Exact S-expression match |
| Python | 117 | **117 / 117 (100.00%)** | 25 / 117 (21.37%)* | 0 | \*`tree-sitter-python` 0.25.0 + `tree-sitter` 0.26.13 `to_sexp()` wraps statements in `(expression_statement ...)`, whereas `yolowsyntax` matches the `26855ea` corpus verbatim |
| JavaScript | 116 | **116 / 116 (100.00%)** | 116 / 116 (100.00%) | 0 | Exact S-expression match |
| TypeScript | 111 | **111 / 111 (100.00%)** | 111 / 111 (100.00%) | 1 | 1 `:language(tsx)` case skipped |
| Rust | 151 | **151 / 151 (100.00%)** | 151 / 151 (100.00%) | 0 | Exact S-expression match |
| Bash | 100 | **100 / 100 (100.00%)** | 100 / 100 (100.00%) | 0 | Exact S-expression match |
| **8-Lang Total** | **855** | **855 / 855 (100.00%)** | — | **1** | — |

### 2. Parse Speed & Outline Check (~64 KiB Corpus-Concatenated Inputs, median of 25 rounds)

Inputs are synthesized by concatenating valid upstream `test/corpus` snippets up to ~64 KiB. `yolowsyntax-compare` asserts `!truncated`, checks `has_error()` parity against C `tree-sitter`, compares field-stripped S-expressions (`parity`), and reports whether `outline()` returned `some(symbols)` (with lossless `sections` reconstruction verified) or `none`.

> Note: These `1.95×–4.02×` numbers measure ~64 KiB concatenated corpus snippets on this hardware (`i7-10700`, `tree-sitter 0.26.13`); see Section 3 below for real-world standalone source files.

| Language | Bytes | Cold Load (ms ↓) | `yolowsyntax` parse (ms ↓) | C `tree-sitter` parse (ms ↓) | `yolowsyntax` `outline` (ms ↓) | `yolowsyntax` (MB/s ↑) | C `tree-sitter` (MB/s ↑) | Speed (`ts / yolow` ↑) | Parity | `outline` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| C | 65,614 | 2.142 | **5.689** | 22.847 | 7.444 | **11.00** | 2.74 | **4.02×** | `exact` | `some(361)` |
| Go | 65,623 | 0.414 | **5.749** | 12.660 | 7.829 | **10.89** | 4.94 | **2.20×** | `exact(both_err)` | `some(724)` |
| Java | 65,597 | 0.733 | **4.866** | 11.486 | 7.027 | **12.86** | 5.45 | **2.36×** | `exact` | `some(615)` |
| Python | 65,699 | 0.707 | **7.532** | 16.933 | 9.819 | **8.32** | 3.70 | **2.25×** | `both_err_diff` | `some(255)` |
| JavaScript | 65,647 | 0.563 | **5.998** | 14.808 | 8.499 | **10.44** | 4.23 | **2.47×** | `exact(both_err)` | `some(253)` |
| TypeScript | 65,615 | 1.388 | **5.491** | 12.566 | 7.771 | **11.40** | 4.98 | **2.29×** | `exact(both_err)` | `some(902)` |
| Rust | 65,557 | 1.417 | **5.700** | 14.827 | 8.303 | **10.97** | 4.22 | **2.60×** | `exact(both_err)` | `some(722)` |
| Bash | 65,979 | 1.893 | **14.212** | 27.645 | 16.267 | **4.43** | 2.28 | **1.95×** | `both_err_diff` | `none` |

### 3. Real-World Open-Source Files (`real-bench`, median of 25 rounds)

Standalone source files (31 KiB – 176 KiB) fetched from pinned upstream tags in `benchmarks/fetch_corpus.py`:
- `c`: `tree-sitter` `v0.25.3` (`lib/src/parser.c`, both runtimes encounter identical unexpanded macro syntax errors -> `exact(both_err)`)
- `go`: `golang/go` `go1.23.0` (`src/encoding/json/decode.go`)
- `java`: `google/guava` `v33.3.1` (`ImmutableList.java`)
- `python`: `python/cpython` `v3.13.0` (`Lib/argparse.py`; `clean(py_expr_stmt)` reflects the `expression_statement` wrapper difference in upstream `tree-sitter-python`, zero errors on either parser)
- `javascript`: `moment/moment` `2.30.1` (`moment.js`; wraps the library inside a top-level UMD IIFE, so `outline()` finds 0 top-level definitions and returns `none`)
- `typescript`: `colinhacks/zod` `v3.23.8` (`src/types.ts`)
- `rust`: `BurntSushi/ripgrep` `14.1.1` (`crates/searcher/src/searcher/mod.rs`)
- `bash`: `nvm-sh/nvm` `v0.40.1` (`nvm.sh`; wraps the entire script inside a top-level `{ ... }` compound block, so `outline()` returns `none`)

| Language | Bytes | Cold Load (ms ↓) | `yolowsyntax` parse (ms ↓) | C `tree-sitter` parse (ms ↓) | `yolowsyntax` `outline` (ms ↓) | `yolowsyntax` (MB/s ↑) | C `tree-sitter` (MB/s ↑) | Speed (`ts / yolow` ↑) | Parity | `outline` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| C (`parser.c`) | 78,797 | 1.553 | **2.906** | 15.156 | 4.085 | **25.86** | 4.96 | **5.22×** | `exact(both_err)` | `some(59)` |
| Go (`decode.go`) | 36,112 | 0.694 | **2.396** | 4.921 | 3.141 | **14.37** | 7.00 | **2.05×** | `exact` | `some(38)` |
| Java (`ImmutableList.java`) | 31,280 | 0.618 | **0.996** | 2.260 | 1.344 | **29.96** | 13.20 | **2.27×** | `exact` | `some(85)` |
| Python (`argparse.py`) | 102,236 | 0.706 | **4.527** | 10.351 | 5.910 | **21.54** | 9.42 | **2.29×** | `clean(py_expr_stmt)` | `some(161)` |
| JavaScript (`moment.js`) | 176,435 | 0.566 | **7.956** | 20.295 | 11.176 | **21.15** | 8.29 | **2.55×** | `exact` | `none` |
| TypeScript (`types.ts`) | 153,859 | 1.433 | **9.250** | 21.406 | 12.542 | **15.86** | 6.85 | **2.31×** | `exact` | `some(372)` |
| Rust (`searcher/mod.rs`) | 39,771 | 1.612 | **1.243** | 2.880 | 1.675 | **30.51** | 13.17 | **2.32×** | `exact` | `some(72)` |
| Bash (`nvm.sh`) | 148,844 | 1.742 | **7.433** | 36.250 | 9.862 | **19.10** | 3.92 | **4.88×** | `exact` | `none` |

### 4. Single-Process Peak RSS (`VmHWM`, KiB ↓)

| Language | `yolowsyntax` (KiB ↓) | C `tree-sitter` (KiB ↓) |
|---|---:|---:|
| C | 8,524 | **6,072** |
| Go | 7,848 | **6,348** |
| Java | 8,196 | **6,000** |
| Python | 9,244 | **7,724** |
| JavaScript | 8,108 | **6,768** |
| TypeScript | 10,456 | **7,428** |
| Rust | 11,000 | **7,480** |
| Bash | 12,856 | **8,996** |

*Note*: Both runs execute inside the `yolowsyntax-compare` binary (which links all 41 Zstd-compressed `.pslg.z` tables plus the 8 C grammar crates). The ~1.3–3.9 MiB difference reflects `yolowsyntax` lazily decompressing its `.pslg.z` table and building an owned heap `Language` (plus up to 1 MiB dense state LUT) on first use (`OnceLock`), whereas the C grammar crates place their parse tables directly in static `.rodata`.

### 5. All 41 Languages (`all-corpus`, `all-bench 64 15`, `run_rss.py --all`)

All **5,875 / 5,875 (100.00%)** active corpus test cases pass across all 41 grammars (`22` `:language(...)` skips: `typescript` 1, `tsx` 1, `php` 2, `ocaml` 18). Note that in the `all-bench` timing loop, the 9 markup/config grammars without symbol rules (`css`, `html`, `svelte`, `nix`, `dockerfile`, `make`, `scheme`, `hcl`, `vue`) as well as concatenated snippet inputs with >20% error bytes or 0 symbols measure the parse + empty check before returning `none` from `outline()`.

| Language | Corpus Pass | Skipped | Cold Load (ms ↓) | `parse` (ms ↓) | `outline` (ms ↓) | Throughput (MB/s ↑) | `has_error` | `outline` | Peak RSS (KiB ↓) |
|---|---:|---:|---:|---:|---:|---:|---|---|---:|
| `c` | 85 / 85 (100.00%) | 0 | 1.466 | 5.796 | 7.457 | 10.80 | `false` | `some(361)` | 8,552 |
| `cpp` | 198 / 198 (100.00%) | 0 | 8.142 | 6.226 | 8.171 | 10.09 | `false` | `some(466)` | 18,400 |
| `csharp` | 179 / 179 (100.00%) | 0 | 4.597 | 8.529 | 10.926 | 7.34 | `true` | `some(473)` | 16,600 |
| `go` | 67 / 67 (100.00%) | 0 | 0.400 | 5.059 | 7.329 | 12.37 | `true` | `some(724)` | 7,792 |
| `java` | 108 / 108 (100.00%) | 0 | 0.637 | 4.037 | 5.766 | 15.50 | `false` | `some(615)` | 8,428 |
| `javascript` | 116 / 116 (100.00%) | 0 | 0.501 | 5.879 | 8.160 | 10.65 | `true` | `some(253)` | 8,220 |
| `typescript` | 111 / 111 (100.00%) | 1 | 1.450 | 5.418 | 7.564 | 11.55 | `true` | `some(902)` | 10,448 |
| `tsx` | 111 / 111 (100.00%) | 1 | 1.427 | 5.636 | 7.803 | 11.10 | `true` | `some(902)` | 10,688 |
| `php` | 146 / 146 (100.00%) | 2 | 1.029 | 9.792 | 12.077 | 6.39 | `true` | `some(314)` | 10,288 |
| `python` | 117 / 117 (100.00%) | 0 | 0.687 | 6.909 | 9.121 | 9.07 | `true` | `some(255)` | 9,280 |
| `rust` | 151 / 151 (100.00%) | 0 | 1.479 | 5.458 | 7.607 | 11.45 | `true` | `some(722)` | 10,944 |
| `lua` | 37 / 37 (100.00%) | 0 | 0.110 | 0.417 | 0.441 | 150.20 | `true` | `none` | 3,700 |
| `bash` | 100 / 100 (100.00%) | 0 | 1.705 | 14.013 | 16.173 | 4.49 | `true` | `none` | 12,832 |
| `ruby` | 291 / 291 (100.00%) | 0 | 2.020 | 0.679 | 0.873 | 92.03 | `false` | `some(76)` | 8,688 |
| `kotlin` | 1413 / 1413 (100.00%) | 0 | 11.781 | 11.098 | 13.642 | 5.69 | `true` | `some(793)` | 24,948 |
| `css` | 40 / 40 (100.00%) | 0 | 0.198 | 4.498 | 7.536 | 13.90 | `false` | `none` | 6,764 |
| `html` | 20 / 20 (100.00%) | 0 | 0.070 | 16.827 | 18.076 | 3.72 | `false` | `none` | 5,988 |
| `objc` | 75 / 75 (100.00%) | 0 | 3.507 | 4.995 | 6.663 | 12.52 | `false` | `some(257)` | 13,596 |
| `dart` | 205 / 205 (100.00%) | 0 | 1.604 | 21.067 | 24.428 | 2.97 | `true` | `none` | 11,908 |
| `julia` | 63 / 63 (100.00%) | 0 | 4.818 | 10.524 | 14.593 | 5.95 | `false` | `some(480)` | 17,284 |
| `nix` | 54 / 54 (100.00%) | 0 | 0.216 | 6.108 | 7.169 | 10.23 | `true` | `none` | 6,272 |
| `dockerfile` | 113 / 113 (100.00%) | 0 | 0.200 | 4.057 | 5.116 | 15.42 | `true` | `none` | 6,672 |
| `ocaml` | 86 / 86 (100.00%) | 18 | 4.442 | 3.407 | 3.760 | 18.36 | `true` | `none` | 15,348 |
| `elixir` | 293 / 293 (100.00%) | 0 | 3.622 | 9.655 | 12.552 | 6.48 | `true` | `some(138)` | 17,164 |
| `r` | 89 / 89 (100.00%) | 0 | 0.780 | 35.451 | 37.771 | 1.77 | `true` | `none` | 13,760 |
| `svelte` | 45 / 45 (100.00%) | 0 | 0.095 | 44.388 | 47.302 | 1.41 | `false` | `none` | 14,536 |
| `solidity` | 126 / 126 (100.00%) | 0 | 0.817 | 2.857 | 5.418 | 21.90 | `false` | `some(1216)` | 6,924 |
| `groovy` | 87 / 87 (100.00%) | 0 | 0.850 | 7.302 | 9.078 | 8.57 | `true` | `some(234)` | 9,964 |
| `make` | 117 / 117 (100.00%) | 0 | 0.277 | 3.329 | 4.869 | 18.81 | `false` | `none` | 6,756 |
| `scheme` | 41 / 41 (100.00%) | 0 | 0.212 | 4.488 | 6.670 | 13.93 | `true` | `none` | 10,148 |
| `commonlisp` | 42 / 42 (100.00%) | 0 | 1.240 | 4.354 | 5.657 | 14.36 | `false` | `some(70)` | 10,308 |
| `pascal` | 88 / 88 (100.00%) | 0 | 0.735 | 4.588 | 6.510 | 13.71 | `true` | `none` | 9,204 |
| `gleam` | 79 / 79 (100.00%) | 0 | 0.551 | 7.368 | 9.249 | 8.49 | `false` | `some(492)` | 8,296 |
| `powershell` | 165 / 165 (100.00%) | 0 | 1.062 | 5.464 | 8.521 | 11.45 | `false` | `some(234)` | 10,312 |
| `cmake` | 68 / 68 (100.00%) | 0 | 0.121 | 7.420 | 10.766 | 8.42 | `false` | `some(130)` | 9,264 |
| `erlang` | 230 / 230 (100.00%) | 0 | 0.467 | 18.600 | 21.465 | 3.36 | `true` | `none` | 14,772 |
| `hcl` | 102 / 102 (100.00%) | 0 | 0.407 | 5.844 | 8.466 | 10.98 | `true` | `none` | 8,640 |
| `vue` | 10 / 10 (100.00%) | 0 | 0.068 | 4.101 | 5.082 | 15.24 | `true` | `none` | 5,528 |
| `d` | 218 / 218 (100.00%) | 0 | 3.309 | 0.441 | 0.585 | 141.99 | `false` | `some(18)` | 12,296 |
| `elm` | 119 / 119 (100.00%) | 0 | 0.521 | 1.396 | 1.647 | 44.91 | `true` | `none` | 4,848 |
| `verilog` | 70 / 70 (100.00%) | 0 | 7.429 | 8.457 | 11.445 | 7.39 | `false` | `some(860)` | 18,632 |
| **Total (41 langs)** | **5,875 / 5,875 (100.00%)** | **22** | **0.068–11.781** | **0.417–44.388** | **0.441–47.302** | **1.41–150.20** | — | — | **3,700–24,948** |
