# Changelog

## 0.1.0 - 2026-11-13

- Initial standalone release extracted from PageSift (`pagesift-syntax`).
- Pure-Rust (`#![forbid(unsafe_code)]`) tree-sitter-compatible runtime with bounded GLR stack depth, version count, node count, step budget, and wall-clock timeout (`Limits`).
- Embedded zlib-compressed parse tables (`tables/*.pslg.z`) and external scanners for 41 languages, lazily decompressed per language via `OnceLock`.
- Structural code outline extraction (`outline`) producing top-level `sections`, `symbols` (functions, methods, structs, classes, enums, traits, interfaces, types), and `imports`.
- Optional `tools` feature providing `ts-convert`, `ts-corpus`, and `ts-parse` CLI binaries.
