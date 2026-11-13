#!/usr/bin/env python3
"""Fetch official test/corpus directories for all 41 tree-sitter grammars at THIRD-PARTY.md commits."""

from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent
CORPUS_DIR = ROOT / "corpus"

# (tag, repo_url, commit, preferred_sparse_paths)
REPOS = [
    ("c", "https://github.com/tree-sitter/tree-sitter-c.git", "b780e47", ["test/corpus"]),
    ("cpp", "https://github.com/tree-sitter/tree-sitter-cpp.git", "c009222", ["test/corpus"]),
    ("csharp", "https://github.com/tree-sitter/tree-sitter-c-sharp.git", "8c0abe0", ["test/corpus"]),
    ("go", "https://github.com/tree-sitter/tree-sitter-go.git", "2346a3a", ["test/corpus"]),
    ("java", "https://github.com/tree-sitter/tree-sitter-java.git", "e10607b", ["test/corpus"]),
    ("javascript", "https://github.com/tree-sitter/tree-sitter-javascript.git", "58404d8", ["test/corpus"]),
    ("typescript", "https://github.com/tree-sitter/tree-sitter-typescript.git", "75b3874", ["test/corpus"]),
    ("tsx", "https://github.com/tree-sitter/tree-sitter-typescript.git", "75b3874", ["tsx/test/corpus", "common/test/corpus"]),
    ("php", "https://github.com/tree-sitter/tree-sitter-php.git", "58d5643", ["php/test/corpus", "common/test/corpus"]),
    ("python", "https://github.com/tree-sitter/tree-sitter-python.git", "26855ea", ["test/corpus"]),
    ("rust", "https://github.com/tree-sitter/tree-sitter-rust.git", "77a3747", ["test/corpus"]),
    ("lua", "https://github.com/MunifTanjim/tree-sitter-lua.git", "4fbec84", ["test/corpus"]),
    ("bash", "https://github.com/tree-sitter/tree-sitter-bash.git", "a06c2e4", ["test/corpus"]),
    ("ruby", "https://github.com/tree-sitter/tree-sitter-ruby.git", "ad907a6", ["test/corpus"]),
    ("kotlin", "https://github.com/fwcd/tree-sitter-kotlin.git", "5fd3624", ["test/corpus"]),
    ("css", "https://github.com/tree-sitter/tree-sitter-css.git", "dda5cfc", ["test/corpus"]),
    ("html", "https://github.com/tree-sitter/tree-sitter-html.git", "73a3947", ["test/corpus"]),
    ("objc", "https://github.com/tree-sitter-grammars/tree-sitter-objc.git", "181a81b", ["test/corpus"]),
    ("dart", "https://github.com/UserNobody14/tree-sitter-dart.git", "0038280", ["test/corpus"]),
    ("julia", "https://github.com/tree-sitter/tree-sitter-julia.git", "e0f9dcd", ["test/corpus"]),
    ("nix", "https://github.com/nix-community/tree-sitter-nix.git", "e49c2f3", ["test/corpus"]),
    ("dockerfile", "https://github.com/camdencheek/tree-sitter-dockerfile.git", "971acdd", ["test/corpus"]),
    ("ocaml", "https://github.com/tree-sitter/tree-sitter-ocaml.git", "3b2e14e", ["grammars/ocaml/test/corpus", "ocaml/test/corpus", "test/corpus"]),
    ("elixir", "https://github.com/elixir-lang/tree-sitter-elixir.git", "cb32ccb", ["test/corpus"]),
    ("r", "https://github.com/r-lib/tree-sitter-r.git", "58a2279", ["test/corpus"]),
    ("svelte", "https://github.com/tree-sitter-grammars/tree-sitter-svelte.git", "ae5199d", ["test/corpus"]),
    ("solidity", "https://github.com/JoranHonig/tree-sitter-solidity.git", "048fe68", ["test/corpus"]),
    ("groovy", "https://github.com/murtaza64/tree-sitter-groovy.git", "2a6ddd5", ["test/corpus"]),
    ("make", "https://github.com/tree-sitter-grammars/tree-sitter-make.git", "70613f3", ["test/corpus"]),
    ("scheme", "https://github.com/6cdh/tree-sitter-scheme.git", "45c7246", ["test/corpus"]),
    ("commonlisp", "https://github.com/tree-sitter-grammars/tree-sitter-commonlisp.git", "3232350", ["test/corpus"]),
    ("pascal", "https://github.com/Isopod/tree-sitter-pascal.git", "042119e", ["test/corpus"]),
    ("gleam", "https://github.com/gleam-lang/tree-sitter-gleam.git", "fa6d0d9", ["test/corpus"]),
    ("powershell", "https://github.com/airbus-cert/tree-sitter-powershell.git", "65ea691", ["test/corpus"]),
    ("cmake", "https://github.com/uyha/tree-sitter-cmake.git", "a3ec2b2", ["test/corpus"]),
    ("erlang", "https://github.com/WhatsApp/tree-sitter-erlang.git", "62c82e6", ["test/corpus"]),
    ("hcl", "https://github.com/tree-sitter-grammars/tree-sitter-hcl.git", "64ad627", ["test/corpus"]),
    ("vue", "https://github.com/tree-sitter-grammars/tree-sitter-vue.git", "ce8011a", ["test/corpus"]),
    ("d", "https://github.com/gdamore/tree-sitter-d.git", "64f2793", ["test/corpus"]),
    ("elm", "https://github.com/elm-tooling/tree-sitter-elm.git", "e1e8fea", ["test/corpus"]),
    ("verilog", "https://github.com/tree-sitter/tree-sitter-verilog.git", "227d277", ["test/corpus"]),
]


def detect_paths(dest: Path, rev: str, preferred: list[str]) -> list[str]:
    out = subprocess.check_output(
        ["git", "-C", str(dest), "ls-tree", "-d", "-r", "--name-only", rev],
        text=True,
    )
    dirs = set(out.splitlines())
    found = [p for p in preferred if p in dirs]
    if found:
        return found
    candidates = sorted(d for d in dirs if d == "corpus" or d.endswith("/corpus"))
    return candidates or preferred


def fetch_one(name: str, url: str, rev: str, preferred: list[str]) -> str:
    dest = CORPUS_DIR / name
    if (dest / ".fetched").exists():
        return f"[skip] {name}"
    if dest.exists():
        shutil.rmtree(dest)
    subprocess.run(
        ["git", "clone", "--filter=blob:none", "--no-checkout", url, str(dest)],
        check=True,
        capture_output=True,
    )
    paths = detect_paths(dest, rev, preferred)
    subprocess.run(
        ["git", "-C", str(dest), "sparse-checkout", "init", "--cone"],
        check=True,
        capture_output=True,
    )
    subprocess.run(
        ["git", "-C", str(dest), "sparse-checkout", "set", *paths],
        check=True,
        capture_output=True,
    )
    subprocess.run(
        ["git", "-C", str(dest), "checkout", rev],
        check=True,
        capture_output=True,
    )
    (dest / ".fetched").write_text(f"{url} @ {rev} ({','.join(paths)})\n")
    return f"[ok] {name} @ {rev} -> {paths}"


REAL_SOURCES = [
    (
        "c.c",
        "https://raw.githubusercontent.com/tree-sitter/tree-sitter/v0.25.3/lib/src/parser.c",
        "27737a8b41c0a75430d8226faabf3f72d8bf422835ca2300cbf95789e185a5ce",
    ),
    (
        "go.go",
        "https://raw.githubusercontent.com/golang/go/go1.23.0/src/encoding/json/decode.go",
        "2c4d202bbce137c4d75bbbb25d111f9190dba68c84da2137f733e0d3fe893e7a",
    ),
    (
        "java.java",
        "https://raw.githubusercontent.com/google/guava/v33.3.1/guava/src/com/google/common/collect/ImmutableList.java",
        "a5012324a2c0f9deae963d6945be5522ff719fcf31577205aa4df448d4fe0f8f",
    ),
    (
        "python.py",
        "https://raw.githubusercontent.com/python/cpython/v3.13.0/Lib/argparse.py",
        "67267741eccb30cfa2a03e76b0c2e3f70641c944b0240ee600cca22cd6ef0a34",
    ),
    (
        "javascript.js",
        "https://raw.githubusercontent.com/moment/moment/2.30.1/moment.js",
        "7dc0a51c32dae143f2eade235145dfd6a7756388c0f0bf409fa373dd6c233629",
    ),
    (
        "typescript.ts",
        "https://raw.githubusercontent.com/colinhacks/zod/v3.23.8/src/types.ts",
        "e98b4b517c2552aea380980d5669ea9a88dc40cb2a163a3bb188575aa4ddecc8",
    ),
    (
        "rust.rs",
        "https://raw.githubusercontent.com/BurntSushi/ripgrep/14.1.1/crates/searcher/src/searcher/mod.rs",
        "b1ba96daf77e0298c70bc2b381d2b50b358b632c6dfa954e5a0925f0ace32e27",
    ),
    (
        "bash.sh",
        "https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/nvm.sh",
        "dc84afb1ded75cccdb1a62b35d99ecbbc0c8653eb70224cd3c1c01c171340ddf",
    ),
]


def fetch_real_one(filename: str, url: str, expected_sha256: str) -> str:
    import hashlib
    import urllib.request

    real_dir = CORPUS_DIR / "_real"
    real_dir.mkdir(parents=True, exist_ok=True)
    dest = real_dir / filename
    if dest.exists():
        got = hashlib.sha256(dest.read_bytes()).hexdigest()
        if got == expected_sha256:
            return f"[cached] _real/{filename} ({dest.stat().st_size} B)"
    data = urllib.request.urlopen(url, timeout=30).read()
    got = hashlib.sha256(data).hexdigest()
    if got != expected_sha256:
        raise RuntimeError(f"sha256 mismatch for {filename}: {got} != {expected_sha256}")
    dest.write_bytes(data)
    return f"[ok] _real/{filename} ({len(data)} B)"


def main() -> None:
    CORPUS_DIR.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=8) as pool:
        futs = [pool.submit(fetch_one, *entry) for entry in REPOS]
        futs += [pool.submit(fetch_real_one, *entry) for entry in REAL_SOURCES]
        for fut in as_completed(futs):
            print(fut.result())


if __name__ == "__main__":
    main()
