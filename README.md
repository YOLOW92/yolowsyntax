<div align="center">

# yolowsyntax

**100% Pure-Rust Tree-Sitter Runtime & 41-Language Code Outline Extractor (`#![forbid(unsafe_code)]`)**

[**English**](README.md) · [**简体中文**](docs/README.zh-CN.md) · [**Benchmarks**](benchmarks/README.md) · [**Changelog**](CHANGELOG.md)

[![Crates.io](https://img.shields.io/crates/v/yolowsyntax.svg)](https://crates.io/crates/yolowsyntax)
[![Docs.rs](https://docs.rs/yolowsyntax/badge.svg)](https://docs.rs/yolowsyntax)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust: 1.89+](https://img.shields.io/badge/rust-1.89%2B-orange.svg)](Cargo.toml)

</div>

A pure-Rust parser runtime compatible with tree-sitter grammar tables and external scanners, extracted from [PageSift](https://github.com/YOLOW92/pagesift) and released under the **MIT License**. Zero C compiler required at build time, `#![forbid(unsafe_code)]`, a single pure-Rust runtime dependency ([`yolowzstd`](https://crates.io/crates/yolowzstd), `#![forbid(unsafe_code)]`), and embedded Zstd-compressed parse tables for **41 languages** with lazy per-language decompression and structural code outline extraction.

---

## At a Glance

- **Zero C Toolchain, 100% Safe Rust**: No `cc`, no `bindgen`, no FFI, and `#![forbid(unsafe_code)]` end-to-end (including `yolowzstd` at runtime). Cross-compiles cleanly to any Rust target.
- **41 Embedded Languages, Lazy Zstd Decompression**: Ships Zstd-compressed `.pslg.z` parse tables (`6.2 MiB` across 41 grammars; `6.3 MiB` `.crate` package) and pure-Rust ports of upstream external scanners. Only the languages you actually call are decompressed (`0.04–10.23 ms` cold load) and cached via `OnceLock`.
- **5,875 / 5,875 (100.00%) Across All 41 Official Corpora**: Passes every active test case across all **41 languages** (`5,875 / 5,875`, with `22` upstream-marked skips) and achieves **1.94×–4.10×** the parse speed of the C `tree-sitter` runtime across all 8 benchmarked languages (`C`, `Go`, `Java`, `Python`, `JavaScript`, `TypeScript`, `Rust`, `Bash`).
- **Built-In Code Outline Extraction (`outline`)**: Turns source files into top-level `sections` (losslessly concatenating back to the original source), `symbols` (functions, methods, structs, classes, enums, traits, interfaces, types), and `imports` for RAG and code chunking.
- **Hard Resource Budgets (`Limits`)**: Every parse enforces bounded GLR stack depth, version count, node count, step budget, and wall-clock timeout—returning a bounded `truncated` tree instead of panicking or OOMing on adversarial inputs.

---

## Story

When [PageSift](https://github.com/YOLOW92/pagesift) added source-code extraction for LLM and RAG pipelines, we needed to split arbitrary code files along real structural boundaries—top-level functions, classes, structs, traits, and imports—rather than chopping code at arbitrary line counts.

Using the standard C `tree-sitter` ecosystem meant pulling in a C compiler (`cc`), linking dozens of native `src/parser.c` and `src/scanner.c` compilation units across 41 languages, and passing untrusted files from the open web through C FFI where pathological inputs or deep nesting could threaten worker stability or cross-compilation.

At the same time, we did not need incremental editing for a live IDE buffer—we needed a **one-shot, sand-boxed, pure-Rust parser** that could run safely inside multi-threaded pipelines across Linux, macOS, and Windows without touching a C toolchain.

So we built `yolowsyntax`:
1. **A compact `.pslg.z` binary table converter (`ts-convert`)** that reads upstream `parser.c` files offline and packs their parse tables into Zstd-compressed blobs (`yolowzstd` decompresses a language lazily on first use).
2. **A safe GLR parser runtime (`#![forbid(unsafe_code)]`)** implementing dense O(1) lookup tables, ASCII lexer fast paths, tree-sitter error recovery, version merging/pruning, and pure-Rust ports of the upstream external scanners, guarded at every step by explicit `Limits`.
3. **A lossless code outline layer (`outline`)** that extracts top-level symbols, imports, and contiguous byte-range `sections` (`sections` joined == original file) for Markdown code chunking.

Extracted from PageSift (`pagesift-syntax`) and relicensed under **MIT**, `yolowsyntax` now stands on its own so any Rust project can embed 41 tree-sitter grammars without a single line of C.

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

let out = outline("rust", src).expect("supported language");
assert_eq!(out.imports, ["use std::collections::HashMap;"]);
assert_eq!(out.symbols.len(), 2);

// Every section maps cleanly back to the original source bytes
let reconstructed: String = out.sections.iter().map(|s| &src[s.start..s.end]).collect();
assert_eq!(reconstructed, src);
```

### 2. Direct Syntax Tree Parsing with Resource Budgets (`api::parse`)

```rust
use yolowsyntax::{Limits, api::parse, langs::Lang};

let lang = Lang::from_tag("python").unwrap().table().unwrap();
let limits = Limits {
    max_stack_depth: 2048,
    max_millis: 500,
    ..Limits::default()
};

let tree = parse(lang, b"def greet(name):\n    return f'hi {name}'\n", limits);
assert!(!tree.has_error());
assert!(!tree.truncated);
println!("{}", tree.to_sexp(lang));
```

---

## Supported Languages (41)

- **Systems & Compiled**: `c`, `cpp`, `rust`, `go`, `objc`, `d`, `pascal`, `verilog`
- **JVM & Managed**: `java`, `csharp`, `kotlin`, `groovy`, `dart`
- **Web & Scripting**: `javascript` (`jsx`), `typescript`, `tsx`, `python`, `php`, `ruby`, `lua`, `bash` (`sh`, `shell`, `zsh`), `powershell`
- **Functional & ML**: `ocaml`, `elixir`, `erlang`, `elm`, `gleam`, `scheme`, `commonlisp`, `julia`, `r`
- **Web Templates, Config & Build**: `html`, `css`, `vue`, `svelte`, `nix`, `dockerfile`, `cmake`, `make`, `hcl` (`terraform`), `solidity`

*Note*: Markup/config grammars without symbol definitions (`html`, `css`, `vue`, `svelte`, `nix`, `dockerfile`, `make`, `scheme`, `hcl`) parse into full syntax trees via `Lang::table()` + `api::parse()`, while `outline()` returns `None` when a file has no structural outline definitions.

---

## Benchmarks

Measured on Intel Core i7-10700 (pinned with `taskset -c 4`), Linux x86-64, `rustc 1.99.0` (`--release`, `lto = "fat"`, `codegen-units = 1`) against `tree-sitter 0.26.13` + official C grammar crates on ~64 KiB inputs (median of 25 alternating rounds) and all **41 official `test/corpus` suites** (`5,875 / 5,875` passing). Full reproduction scripts, RSS data, and 41-language results are in [`benchmarks/README.md`](benchmarks/README.md).

| Language | Corpus (`yolowsyntax`) | `yolowsyntax` parse (ms ↓) | C `tree-sitter` parse (ms ↓) | `yolowsyntax` `outline` (ms ↓) | Speed (`ts / yolow` ↑) |
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

## License

MIT License — see [LICENSE](LICENSE), [THIRD-PARTY.md](THIRD-PARTY.md), and [UPSTREAM.md](UPSTREAM.md).
