#!/usr/bin/env python3
"""Measure single-process peak RSS (VmHWM in KiB) for yolowsyntax vs C tree-sitter."""

from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
BIN = ROOT / "compare" / "target" / "release" / "yolowsyntax-compare"
CORPUS = ROOT / "corpus"
LANGS_8 = ["c", "go", "java", "python", "javascript", "typescript", "rust", "bash"]
ALL_41 = [
    "c", "cpp", "csharp", "go", "java", "javascript", "typescript", "tsx", "php", "python",
    "rust", "lua", "bash", "ruby", "kotlin", "css", "html", "objc", "dart", "julia", "nix",
    "dockerfile", "ocaml", "elixir", "r", "svelte", "solidity", "groovy", "make", "scheme",
    "commonlisp", "pascal", "gleam", "powershell", "cmake", "erlang", "hcl", "vue", "d", "elm",
    "verilog",
]


def main() -> None:
    if "--all" in sys.argv:
        print("lang\tyolow_rss_kib")
        for lang in ALL_41:
            y_out = subprocess.check_output(
                [str(BIN), "rss", str(CORPUS), "yolow", lang, "64"], text=True
            ).strip()
            print(f"{lang}\t{y_out.split()[-1]}")
        return
    print("lang\tyolow_rss_kib\tts_rss_kib")
    for lang in LANGS_8:
        y_out = subprocess.check_output(
            [str(BIN), "rss", str(CORPUS), "yolow", lang, "64"], text=True
        ).strip()
        t_out = subprocess.check_output(
            [str(BIN), "rss", str(CORPUS), "ts", lang, "64"], text=True
        ).strip()
        y_kib = y_out.split("\t")[-1]
        t_kib = t_out.split("\t")[-1]
        print(f"{lang}\t{y_kib}\t{t_kib}")


if __name__ == "__main__":
    main()
