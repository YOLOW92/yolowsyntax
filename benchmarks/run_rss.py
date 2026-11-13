#!/usr/bin/env python3
"""Measure single-process peak RSS (VmHWM in KiB) for yolowsyntax vs C tree-sitter."""

from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
BIN = ROOT / "compare" / "target" / "release" / "yolowsyntax-compare"
CORPUS = ROOT / "corpus"
LANGS = ["c", "go", "java", "python", "javascript", "typescript", "rust", "bash"]


def main() -> None:
    print("lang\tyolow_rss_kib\tts_rss_kib")
    for lang in LANGS:
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
