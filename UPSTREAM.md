# Upstream

This repository extracts `pagesift-syntax` from PageSift (`https://github.com/YOLOW92/pagesift`), commit `2d356163` (`1.3.0`).

Copied and renamed:

| Upstream crate | Here |
|---|---|
| `pagesift-syntax` | `yolowsyntax` |

The code is the same pure-Rust implementation, with package metadata, repository references, and license (`MIT` here; PageSift remains `Apache-2.0`) updated by the copyright holder. All 41 embedded grammar tables (`tables/*.pslg.z`) and external scanner ports come from permissive MIT / Apache-2.0 upstream grammars listed in [`THIRD-PARTY.md`](THIRD-PARTY.md).

Public API entry points: `yolowsyntax::outline`, `yolowsyntax::Outline`, `yolowsyntax::Section`, `yolowsyntax::Symbol`, `yolowsyntax::SymbolKind`, `yolowsyntax::Limits`, `yolowsyntax::api::parse`, and `yolowsyntax::langs::Lang`.
