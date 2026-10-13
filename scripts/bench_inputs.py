#!/usr/bin/env python3
"""Collect fixture diagrams and generate large graphs for benches.

Writes one .mmd file per unique mermaid input under the given directory.
Usage: bench_inputs.py <out-dir>
"""
from __future__ import annotations

import hashlib
import os
import pathlib
import re
import subprocess
import sys

KEYWORDS = (
    "graph ",
    "flowchart ",
    "sequenceDiagram",
    "erDiagram",
    "stateDiagram",
    "classDiagram",
)

RAW = re.compile(r'r(?P<h>#*)"(?P<body>.*?)"(?P=h)', re.DOTALL)
REGULAR = re.compile(r'"(?P<body>(?:\\.|[^"\\])*)"', re.DOTALL)


def is_mermaid(text: str) -> bool:
    trimmed = text.lstrip()
    return any(trimmed.startswith(key) for key in KEYWORDS)


def rust_unescape(body: str) -> str:
    out = []
    i = 0
    while i < len(body):
        if body[i] != "\\":
            out.append(body[i])
            i += 1
            continue
        if i + 1 < len(body) and body[i + 1] == "\n":
            i += 2
            continue
        if i + 1 < len(body) and body[i + 1] == "\r" and i + 2 < len(body) and body[i + 2] == "\n":
            i += 3
            continue
        escapes = {
            "n": "\n",
            "r": "\r",
            "t": "\t",
            "\\": "\\",
            '"': '"',
            "'": "'",
            "0": "\0",
        }
        nxt = body[i + 1] if i + 1 < len(body) else ""
        out.append(escapes.get(nxt, nxt))
        i += 2
    return "".join(out)


def extract_from_rust(path: pathlib.Path) -> list[str]:
    text = path.read_text()
    found = []
    for match in RAW.finditer(text):
        body = match.group("body")
        if is_mermaid(body):
            found.append(body)
    for match in REGULAR.finditer(text):
        body = rust_unescape(match.group("body"))
        if is_mermaid(body):
            found.append(body)
    return found


def gen_chain(n: int) -> str:
    lines = ["graph TD"]
    for i in range(n):
        lines.append(f"    n{i}[N{i}]")
    for i in range(n - 1):
        lines.append(f"    n{i} --> n{i + 1}")
        if i % 7 == 0 and i + 3 < n:
            lines.append(f"    n{i} --> n{i + 3}")
    return "\n".join(lines) + "\n"


def gen_chain_subgraphs(n: int, group: int = 10) -> str:
    lines = ["graph TD"]
    ngroups = (n + group - 1) // group
    for g in range(ngroups):
        start = g * group
        end = min(start + group, n)
        lines.append(f"    subgraph g{g}[Group {g}]")
        for i in range(start, end):
            lines.append(f"        n{i}[N{i}]")
        for i in range(start, end - 1):
            lines.append(f"        n{i} --> n{i + 1}")
        lines.append("    end")
    for g in range(ngroups - 1):
        lines.append(f"    n{(g + 1) * group - 1} --> n{(g + 1) * group}")
    return "\n".join(lines) + "\n"


def fingerprint(text: str) -> str:
    return hashlib.sha1(text.encode()).hexdigest()[:12]


def find_ma(repo: pathlib.Path) -> pathlib.Path | None:
    env = os.environ.get("MA_BIN")
    if env:
        return pathlib.Path(env)
    candidate = repo / "target" / "release" / "ma"
    if candidate.is_file():
        return candidate
    return None


def renders(ma_bin: pathlib.Path | None, text: str) -> bool:
    if ma_bin is None:
        return True
    result = subprocess.run(
        [str(ma_bin)],
        input=text.encode(),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return result.returncode == 0


def write_unique(
    out_dir: pathlib.Path,
    prefix: str,
    texts: list[str],
    ma_bin: pathlib.Path | None,
) -> tuple[int, int]:
    seen: set[str] = set()
    kept = 0
    skipped = 0
    for text in texts:
        key = fingerprint(text)
        if key in seen:
            continue
        seen.add(key)
        if not renders(ma_bin, text):
            skipped += 1
            continue
        (out_dir / f"{prefix}_{key}.mmd").write_text(text)
        kept += 1
    return kept, skipped


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: bench_inputs.py <out-dir>", file=sys.stderr)
        return 2
    out = pathlib.Path(sys.argv[1])
    fixtures = out / "fixtures"
    generated = out / "generated"
    fixtures.mkdir(parents=True, exist_ok=True)
    generated.mkdir(parents=True, exist_ok=True)
    for child in list(fixtures.glob("*.mmd")) + list(generated.glob("*.mmd")):
        child.unlink()

    repo = pathlib.Path(__file__).resolve().parent.parent
    ma_bin = find_ma(repo)
    extracted: list[str] = []
    for path in sorted((repo / "tests").glob("*.rs")):
        extracted.extend(extract_from_rust(path))
    for path in sorted((repo / "tests" / "fixtures").glob("*.mmd")):
        extracted.append(path.read_text())

    n_fix, n_skip = write_unique(fixtures, "fx", extracted, ma_bin)
    generated_specs = {
        "chain_50": gen_chain(50),
        "chain_200": gen_chain(200),
        "chain_500": gen_chain(500),
        "subgraphs_50": gen_chain_subgraphs(50),
        "subgraphs_200": gen_chain_subgraphs(200),
        "subgraphs_500": gen_chain_subgraphs(500),
    }
    for name, body in generated_specs.items():
        (generated / f"{name}.mmd").write_text(body)
    print(
        f"wrote {n_fix} unique fixtures ({n_skip} unrenderable skipped) "
        f"and {len(generated_specs)} generated graphs to {out}"
        + (f" using {ma_bin}" if ma_bin else " (no ma binary, skipped render filter)")
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
