#!/usr/bin/env python3
"""Fetch official test/corpus directories for the 8 compared tree-sitter grammars."""

from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
CORPUS_DIR = ROOT / "corpus"

REPOS = [
    ("c", "https://github.com/tree-sitter/tree-sitter-c.git", "b780e47", ["test/corpus"]),
    ("go", "https://github.com/tree-sitter/tree-sitter-go.git", "2346a3a", ["test/corpus"]),
    ("java", "https://github.com/tree-sitter/tree-sitter-java.git", "e10607b", ["test/corpus"]),
    ("python", "https://github.com/tree-sitter/tree-sitter-python.git", "26855ea", ["test/corpus"]),
    ("javascript", "https://github.com/tree-sitter/tree-sitter-javascript.git", "58404d8", ["test/corpus"]),
    ("typescript", "https://github.com/tree-sitter/tree-sitter-typescript.git", "75b3874", ["test/corpus"]),
    ("rust", "https://github.com/tree-sitter/tree-sitter-rust.git", "77a3747", ["test/corpus"]),
    ("bash", "https://github.com/tree-sitter/tree-sitter-bash.git", "a06c2e4", ["test/corpus"]),
]


def main() -> None:
    CORPUS_DIR.mkdir(parents=True, exist_ok=True)
    for name, url, rev, paths in REPOS:
        dest = CORPUS_DIR / name
        if (dest / ".fetched").exists():
            print(f"[skip] {name}")
            continue
        if dest.exists():
            subprocess.run(["rm", "-rf", str(dest)], check=True)
        subprocess.run(
            ["git", "clone", "--filter=blob:none", "--no-checkout", url, str(dest)],
            check=True,
        )
        subprocess.run(["git", "-C", str(dest), "sparse-checkout", "init", "--cone"], check=True)
        subprocess.run(["git", "-C", str(dest), "sparse-checkout", "set", *paths], check=True)
        subprocess.run(["git", "-C", str(dest), "checkout", rev], check=True)
        (dest / ".fetched").write_text(f"{url} @ {rev}\n")
        print(f"[ok] {name} @ {rev}")


if __name__ == "__main__":
    main()
