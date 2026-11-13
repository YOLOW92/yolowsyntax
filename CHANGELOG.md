# Changelog

## 0.1.1 - 2026-11-13

- Replace `miniz_oxide` with pure-Rust [`yolowzstd`](https://crates.io/crates/yolowzstd) (`0.1.2`, `#![forbid(unsafe_code)]`) and recompress all 41 `.pslg.z` tables with Zstd level 19 (`6.52 MB` / `6.2 MiB` on disk, down 29.4% from `9.23 MB` zlib).
- Speed up cold load, lexing, and GLR parsing: add O(1) `LexAscii` fast path, O(1) `DenseTable` state lookup (stripping redundant `entries` prefixes via `from_bytes_runtime`), 8-byte packed `Seg`, inline `SNode` links + pooled `extra_links`, zero-alloc `Parser::reduce` fast path, and batched little-endian decoding.
- Fix `recover::do_all_potential_reductions` when `lookahead_symbol == 0` under `DenseTable` via `Language::for_each_token_actions`, bringing official corpus pass rate to **5,875 / 5,875 (100.00%)** across all 41 languages.
- Benchmark speedups on 64 KiB inputs vs C `tree-sitter 0.26.13` reach **1.94×–4.10×** across all 8 benchmarked languages (up from `0.83×–1.75×` in `0.1.0`), with `8–11 MiB` (`9,256–11,412 KiB`) peak RSS.
- Expand benchmark harness (`fetch_corpus.py`, `yolowsyntax-compare`, `run_rss.py`) to cover all 41 languages (`all-corpus`, `all-bench`, `run_rss.py --all`).

## 0.1.0 - 2026-11-13

- Initial standalone release extracted from PageSift (`pagesift-syntax`).
- Pure-Rust (`#![forbid(unsafe_code)]`) tree-sitter-compatible runtime with bounded GLR stack depth, version count, node count, step budget, and wall-clock timeout (`Limits`).
- Embedded zlib-compressed parse tables (`tables/*.pslg.z`) and external scanners for 41 languages, lazily decompressed per language via `OnceLock`.
- Structural code outline extraction (`outline`) producing top-level `sections`, `symbols` (functions, methods, structs, classes, enums, traits, interfaces, types), and `imports`.
- Optional `tools` feature providing `ts-convert`, `ts-corpus`, and `ts-parse` CLI binaries.
