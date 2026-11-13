<div align="center">

# yolowsyntax

**100% Pure-Rust Tree-Sitter Runtime & 41-Language Code Outline Extractor (`#![forbid(unsafe_code)]`)**

[**English**](README.md) · [**简体中文**](docs/README.zh-CN.md) · [**Benchmarks**](benchmarks/README.md) · [**Changelog**](CHANGELOG.md)

[![Crates.io](https://img.shields.io/crates/v/yolowsyntax.svg)](https://crates.io/crates/yolowsyntax)
[![Docs.rs](https://docs.rs/yolowsyntax/badge.svg)](https://docs.rs/yolowsyntax)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust: 1.89+](https://img.shields.io/badge/rust-1.89%2B-orange.svg)](Cargo.toml)

</div>

A pure-Rust parser runtime compatible with converted tree-sitter parse tables and ported external scanners, extracted from [PageSift](https://github.com/YOLOW92/pagesift) and released under MIT (see [THIRD-PARTY.md](THIRD-PARTY.md) for MIT / Apache-2.0 upstream grammar and scanner notices). Zero C compiler required at build time, `#![forbid(unsafe_code)]`, a single pure-Rust runtime dependency ([`yolowzstd`](https://crates.io/crates/yolowzstd) `0.1.2`, `#![forbid(unsafe_code)]`), embedded Zstd-compressed parse tables for **41 languages** with lazy per-language decompression, and structural code outline extraction.

---

## At a Glance

- **Zero C Toolchain, 100% Safe Rust**: No `cc`, no `bindgen`, no FFI, and `#![forbid(unsafe_code)]` end-to-end (including `yolowzstd` at runtime). Compiles on standard Rust `std` targets without native C build dependencies (MSRV `1.89+`, tested on Linux x86-64).
- **41 Embedded Languages, Lazy Zstd Decompression**: Ships Zstd-compressed `.pslg.z` parse tables (`6.22 MiB` across 41 grammars; `6.28 MiB` `.crate` package) and pure-Rust ports of upstream external scanners for one-shot (`&[u8]`) parsing. Only the languages you actually call are decompressed (`0.068–11.781 ms` cold load) and cached via `OnceLock`.
- **Scope & Compatibility**: Embeds 41 compiled grammars and ported scanners, rather than aiming for drop-in C `tree-sitter` API compatibility—there is no dynamic C grammar loading, no Tree-sitter Query DSL, and no incremental re-parsing (`tree.edit`).
- **5,875 / 5,875 (100.00%) Active Official Corpus Cases**: Passes all `5,875` active test cases across 41 pinned upstream `test/corpus` suites (`0` failed; `5,844` S-exp matches + `31` expected `:error` cases; `22` skipped due to `:language(...)` dialect mismatch across shared suites; `:skip`/`:cst`/excluded `:platform` directives are also skipped by the runner, with `0` present here).
- **Code Outline Extraction (`outline`)**: For 32 symbol-bearing languages, extracts `symbols`, `imports`, and contiguous `sections` (losslessly concatenating back to the original source). The remaining 9 markup/config grammars still parse into syntax trees via `api::parse()`, while `outline()` returns `None` when a grammar or file yields no definitions (or exceeds `2 MiB` / `20%` error bytes).
- **Cooperative Resource Budgets (`Limits`)**: Bounds GLR stack depth, version count (`max_versions = 6`), AST node count (`max_nodes = 4,000,000`), parser step budget, and an optional cooperative wall-clock deadline (`max_millis = 0` by default = disabled), returning a tiny `truncated` degraded tree on overflow. Note: this is in-process cooperative bounding, **not** OS/cgroup memory isolation, and does not guarantee protection against OOM for arbitrary host allocations outside parser step/budget checks.

---

## Story

When [PageSift](https://github.com/YOLOW92/pagesift) added source-code extraction for LLM and RAG pipelines, we needed to split source files along real structural boundaries—top-level functions, classes, structs, traits, and imports—rather than chopping code at arbitrary line counts.

Using the standard C `tree-sitter` ecosystem meant pulling in a C compiler (`cc`), linking dozens of native `src/parser.c` and `src/scanner.c` compilation units across 41 languages, and passing untrusted files from the open web through C FFI. At the same time, we did not need incremental editing for a live IDE buffer—we needed a **one-shot, pure-Rust parser** with explicit runtime budgets.

So we built `yolowsyntax`:
1. **A compact `.pslg.z` binary table converter (`ts-convert`)** that reads upstream `parser.c` files offline and packs their parse tables into Zstd-compressed blobs (`yolowzstd` decompresses a language lazily on first use).
2. **A safe GLR parser runtime (`#![forbid(unsafe_code)]`)** implementing dense O(1) lookup tables, ASCII lexer fast paths, tree-sitter error recovery, version merging/pruning, pure-Rust ports of the upstream external scanners, and cooperative `Limits`.
3. **A lossless code outline layer (`outline`)** that extracts top-level/container symbols, imports, and contiguous byte-range `sections` (`sections` joined == original file when `Some`).

---

## Install

```toml
[dependencies]
yolowsyntax = "0.1"
```

To enable the offline grammar converter and corpus/parse CLI tools (`ts-convert`, `ts-corpus`, `ts-parse`):

```toml
[dependencies]
yolowsyntax = { version = "0.1", features = ["tools"] }
```

---

## Usage

### 1. Extract Code Outline (`outline`)

```rust
use yolowsyntax::outline;

let src = r#"
use std::collections::HashMap;

pub struct Config {
    pub limit: usize,
}

pub fn build_index(items: &[&str]) -> HashMap<String, usize> {
    items.iter().enumerate().map(|(i, s)| ((*s).to_string(), i)).collect()
}
"#;

let out = outline("rust", src).expect("supported language with symbols");
assert_eq!(out.imports, ["use std::collections::HashMap;"]);
assert_eq!(out.symbols.len(), 2);

// Every section maps cleanly back to the original source bytes
let reconstructed: String = out.sections.iter().map(|s| &src[s.start..s.end]).collect();
assert_eq!(reconstructed, src);
```

### 2. Direct Syntax Tree Parsing with Cooperative Budgets (`api::parse`)

```rust
use yolowsyntax::{Limits, api::parse, langs::Lang};

let lang = Lang::from_tag("python").unwrap().table().unwrap();
let limits = Limits {
    max_stack_depth: 2048,
    max_millis: 500, // default is 0 (disabled)
    ..Limits::default()
};

let tree = parse(lang, b"def greet(name):\n    return f'hi {name}'\n", limits);
assert!(!tree.has_error());
assert!(!tree.truncated);
println!("{}", tree.to_sexp(lang));
```

### `outline()` Semantics & When It Returns `None`

- **`symbols`**: Top-level declarations plus methods/constructors/fields nested directly inside types (`struct`, `class`, `trait`, `impl`, `interface`, `enum`) or single-file module containers; capped at `2,000` symbols.
- **`imports`**: Extracted `import`/`use`/`include` statements; capped at `500` entries.
- **`sections`**: Contiguous non-overlapping byte ranges covering `[0..source.len())` anchored on top-level symbol boundaries (leading preamble has `title: None`).
- **Returns `None`** when: the tag is unknown, input is empty or `> 2 MiB`, parsing hits `Limits` (including the internal `3,000 ms` cooperative budget), syntax error spans exceed `20%` of input bytes, the language has no outline rules (`html`, `css`, `vue`, `svelte`, `nix`, `dockerfile`, `make`, `scheme`, `hcl`), or `symbols` is empty (e.g., expression-only files or code wrapped inside a single IIFE/block).

---

## Supported Languages (41)

- **Systems & Compiled**: `c`, `cpp`, `rust`, `go`, `objc`, `d`, `pascal`, `verilog`
- **JVM & Managed**: `java`, `csharp`, `kotlin`, `groovy`, `dart`
- **Web & Scripting**: `javascript` (`jsx`), `typescript`, `tsx`, `python`, `php`, `ruby`, `lua`, `bash` (`sh`, `shell`, `zsh`), `powershell`
- **Functional & ML**: `ocaml`, `elixir`, `erlang`, `elm`, `gleam`, `scheme`, `commonlisp`, `julia`, `r`
- **Web Templates, Config & Build**: `html`, `css`, `vue`, `svelte`, `nix`, `dockerfile`, `cmake`, `make`, `hcl` (`terraform`), `solidity`

---

## Benchmarks

Measured on **Intel Core i7-10700** (`taskset -c 4`), **Linux 7.0.0-43-generic x86-64**, **`rustc 1.99.0`** (`--release`, `lto = "fat"`, `codegen-units = 1`) against **`tree-sitter 0.26.13`** + upstream C grammar crates (median of 25 alternating rounds). Both corpus parity and error parity are verified by the harness (`y_err == ts_err`); full scripts, RSS (`3,700–24,948 KiB`), real OSS file runs, and 41-language results are in [`benchmarks/README.md`](benchmarks/README.md).

### ~64 KiB Corpus-Concatenated Suite (Historical Baseline, Hardware/Dataset-Specific)

These **1.95×–4.02×** speedups apply specifically to ~64 KiB concatenated upstream `test/corpus` snippets under this harness configuration, not as a universal multiplier for all inputs:

| Language | Active Corpus (`yolowsyntax`) | `yolowsyntax` parse (ms ↓) | C `tree-sitter` parse (ms ↓) | `yolowsyntax` `outline` (ms ↓) | Speed (`ts / yolow` ↑) | Parity / `outline` |
|---|---:|---:|---:|---:|---:|---|
| C | **85 / 85 (100%)** | **5.689** | 22.847 | 7.444 | **4.02×** | `exact` / `some(361)` |
| Go | **67 / 67 (100%)** | **5.749** | 12.660 | 7.829 | **2.20×** | `exact(both_err)` / `some(724)` |
| Java | **108 / 108 (100%)** | **4.866** | 11.486 | 7.027 | **2.36×** | `exact` / `some(615)` |
| Python | **117 / 117 (100%)** | **7.532** | 16.933 | 9.819 | **2.25×** | `both_err_diff` / `some(255)` |
| JavaScript | **116 / 116 (100%)** | **5.998** | 14.808 | 8.499 | **2.47×** | `exact(both_err)` / `some(253)` |
| TypeScript | **111 / 111 (100%)** | **5.491** | 12.566 | 7.771 | **2.29×** | `exact(both_err)` / `some(902)` |
| Rust | **151 / 151 (100%)** | **5.700** | 14.827 | 8.303 | **2.60×** | `exact(both_err)` / `some(722)` |
| Bash | **100 / 100 (100%)** | **14.212** | 27.645 | 16.267 | **1.95×** | `both_err_diff` / `none` |

### Standalone Real-World OSS Files (31 KiB – 176 KiB, `real-bench`)

| Language (File) | Bytes | `yolowsyntax` parse (ms ↓) | C `tree-sitter` parse (ms ↓) | `yolowsyntax` `outline` (ms ↓) | Speed (`ts / yolow` ↑) | Parity / `outline` |
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

## License

MIT License — see [LICENSE](LICENSE), [THIRD-PARTY.md](THIRD-PARTY.md) (for MIT and Apache-2.0 upstream grammar/scanner attributions), and [UPSTREAM.md](UPSTREAM.md).
