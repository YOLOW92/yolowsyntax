# Changelog

## 0.1.2 - 2026-11-13

- Wire `Limits.max_versions` into `Parser::reduce` and GLR recovery (default `6`, retaining `MAX_VERSION_COUNT_OVERFLOW = 4`), check oversized inputs and zero budgets before preallocation via `Parser::try_new`, cap initial `Arena` reserve capacity (`32,768` nodes), enforce `kids` arena budget, and drop oversized parser state when returning a tiny `truncated` degraded tree.
- Add batched cooperative wall-clock deadline checks (`Limits.max_millis`, default `0` = disabled) across `Lexer::run`, external scanners (`ScanLexer`), error recovery, and pre-compaction checks, plus incremental `Lexer::column` caching to avoid O(N²) scans on long single lines.
- Harden grammar table deserialization (`Language::from_bytes` / `validate`) against truncated payloads, oversized string count preallocations, out-of-range states/symbols/aliases/fields/lex modes/scanner maps, and non-monotonic lexer states; fix `DenseTable` + `Language::to_bytes()` roundtrip fidelity so runtime tables serialize cleanly without losing stripped prefix entries.
- Fix `compact::maybe_compact` to run `compact_stack()` before `compact_arena()` when dead GLR stack heads would otherwise pin unreachable AST nodes.
- Add `tests/safety.rs` regression tests and upgrade `benchmarks/compare` + `benchmarks/fetch_corpus.py` to assert parse parity, verify lossless `outline()` reconstruction vs `None`, and measure standalone real OSS files (`real-bench`).
- Clarify scope, `Limits` cooperative semantics, `outline()` rules (32 symbol languages vs 9 tree-only languages), corpus skip counts (`22` `:language(...)` skips across `5,897` cases), and MIT / Apache-2.0 attributions across README and `THIRD-PARTY.md`.

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
