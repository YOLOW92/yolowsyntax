<div align="center">

# yolowsyntax

**100% Pure-Rust Tree-Sitter Runtime & 41-Language Code Outline Extractor (`#![forbid(unsafe_code)]`)**

[**English**](README.md) · [**简体中文**](docs/README.zh-CN.md) · [**Benchmarks**](benchmarks/README.md) · [**Changelog**](CHANGELOG.md)

[![Crates.io](https://img.shields.io/crates/v/yolowsyntax.svg)](https://crates.io/crates/yolowsyntax)
[![Docs.rs](https://docs.rs/yolowsyntax/badge.svg)](https://docs.rs/yolowsyntax)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust: 1.89+](https://img.shields.io/badge/rust-1.89%2B-orange.svg)](Cargo.toml)

</div>

A pure-Rust parser runtime compatible with tree-sitter grammar tables and external scanners, extracted from [PageSift](https://github.com/YOLOW92/pagesift) and released under the **MIT License**. Zero C compiler required at build time, `#![forbid(unsafe_code)]`, a single runtime dependency (`miniz_oxide`), and embedded compressed parse tables for **41 languages** with lazy per-language decompression and structural code outline extraction.

---

## At a Glance

- **Zero C Toolchain, 100% Safe Rust**: No `cc`, no `bindgen`, no FFI, and `#![forbid(unsafe_code)]`. Cross-compiles cleanly to any Rust target with only `miniz_oxide` at runtime.
- **41 Embedded Languages, Lazy Decompression**: Ships zlib-compressed `.pslg.z` parse tables (`5.8 MiB` `.crate` package) and pure-Rust ports of upstream external scanners. Only the languages you actually call are decompressed and cached via `OnceLock`.
- **855 / 855 (100.00%) Official Corpus Pass Rate**: Passes every test case across all 8 benchmarked official tree-sitter corpora (`C`, `Go`, `Java`, `Python`, `JavaScript`, `TypeScript`, `Rust`, `Bash`) at **0.83×–1.75×** the speed of the C `tree-sitter` runtime.
- **Built-In Code Outline Extraction (`outline`)**: Turns source files into top-level `sections` (losslessly concatenating back to the original source), `symbols` (functions, methods, structs, classes, enums, traits, interfaces, types), and `imports` for RAG and code chunking.
- **Hard Resource Budgets (`Limits`)**: Every parse enforces bounded GLR stack depth, version count, node count, step budget, and wall-clock timeout—returning a bounded `truncated` tree instead of panicking or OOMing on adversarial inputs.

---

## Story

When [PageSift](https://github.com/YOLOW92/pagesift) added source-code extraction for LLM and RAG pipelines, we needed to split arbitrary code files along real structural boundaries—top-level functions, classes, structs, traits, and imports—rather than chopping code at arbitrary line counts.

Using the standard C `tree-sitter` ecosystem meant pulling in a C compiler (`cc`), linking dozens of native `src/parser.c` and `src/scanner.c` compilation units across 41 languages, and passing untrusted files from the open web through C FFI where pathological inputs or deep nesting could threaten worker stability or cross-compilation.

At the same time, we did not need incremental editing for a live IDE buffer—we needed a **one-shot, sand-boxed, pure-Rust parser** that could run safely inside multi-threaded pipelines across Linux, macOS, and Windows without touching a C toolchain.

So we built `yolowsyntax`:
1. **A compact `.pslg.z` binary table converter (`ts-convert`)** that reads upstream `parser.c` files offline and packs their parse tables into zlib-compressed blobs (`miniz_oxide` inflates a language lazily on first use).
2. **A safe GLR parser runtime (`#![forbid(unsafe_code)]`)** implementing tree-sitter lexing, error recovery, version merging/pruning, and pure-Rust ports of the upstream external scanners, guarded at every step by explicit `Limits`.
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

Measured on Intel Core i7-10700 (pinned with `taskset -c 4`), Linux x86-64, `rustc 1.99.0` (`--release`, `lto = "fat"`, `codegen-units = 1`) against `tree-sitter 0.26.13` + official C grammar crates on ~64 KiB inputs (median of 25 alternating rounds) and 8 official `test/corpus` suites. Full reproduction scripts and RSS numbers are in [`benchmarks/README.md`](benchmarks/README.md).

| Language | Corpus (`yolowsyntax`) | `yolowsyntax` parse (ms ↓) | C `tree-sitter` parse (ms ↓) | `yolowsyntax` `outline` (ms ↓) | Speed (`ts / yolow`) |
|---|---:|---:|---:|---:|---:|
| C | **85 / 85 (100%)** | **12.905** | 22.637 | 14.735 | **1.75×** |
| Go | **67 / 67 (100%)** | 13.507 | **11.781** | 15.754 | 0.87× |
| Java | **108 / 108 (100%)** | 11.546 | **9.595** | 13.109 | 0.83× |
| Python | **117 / 117 (100%)** | 16.254 | **16.252** | 18.493 | 1.00× |
| JavaScript | **116 / 116 (100%)** | **13.872** | 14.051 | 15.785 | **1.01×** |
| TypeScript | **111 / 111 (100%)** | 13.435 | **12.657** | 15.492 | 0.94× |
| Rust | **151 / 151 (100%)** | 14.745 | **14.511** | 16.948 | 0.98× |
| Bash | **100 / 100 (100%)** | **22.400** | 27.404 | 24.336 | **1.22×** |

---

## License

MIT License — see [LICENSE](LICENSE), [THIRD-PARTY.md](THIRD-PARTY.md), and [UPSTREAM.md](UPSTREAM.md).
