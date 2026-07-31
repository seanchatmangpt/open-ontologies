#!/usr/bin/env python3
"""Inventory executable WIP markers without treating documentation as product code."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INCLUDED_PREFIXES = (
    "src/",
    "packages/",
    "apps/",
    "scripts/",
    "tools/",
    ".specify/queries/",
    ".specify/templates/",
    "ontology/",
    ".github/workflows/",
)
INCLUDED_ROOTS = {
    "Cargo.toml",
    "ggen.toml",
    "package.json",
    "tsconfig.json",
    "rust-toolchain.toml",
}
EXCLUDED_PARTS = {
    "node_modules",
    "target",
    ".git",
    ".cache",
    ".ggen-v2",
    "receipts",
    "evidence",
    "reports",
    "research",
    "fixtures",
}
HARD_PATTERNS = {
    "temporarily_disabled": re.compile(r"TEMPORARILY DISABLED", re.IGNORECASE),
    "not_yet_implemented": re.compile(r"not yet implemented", re.IGNORECASE),
    "unimplemented_macro": re.compile(r"\bunimplemented!\s*\("),
    "todo_macro": re.compile(r"\btodo!\s*\("),
    "todo_stub": re.compile(r"TODO\s+stub", re.IGNORECASE),
    "not_implemented_error": re.compile(r"(?:Err|error|raise)[^\n]{0,100}not implemented", re.IGNORECASE),
}
SOFT_PATTERN = re.compile(r"\b(?:TODO|FIXME|XXX)\b")


def tracked_files() -> list[Path]:
    output = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT)
    paths: list[Path] = []
    for raw in output.split(b"\0"):
        if not raw:
            continue
        rel = Path(raw.decode("utf-8", errors="strict"))
        rel_text = rel.as_posix()
        if any(part in EXCLUDED_PARTS for part in rel.parts):
            continue
        if rel_text in INCLUDED_ROOTS or rel_text.startswith(INCLUDED_PREFIXES):
            paths.append(rel)
    return paths


def main() -> int:
    hard: list[dict[str, object]] = []
    soft: list[dict[str, object]] = []
    scanned = 0
    for rel in tracked_files():
        path = ROOT / rel
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        scanned += 1
        for line_no, line in enumerate(text.splitlines(), start=1):
            for kind, pattern in HARD_PATTERNS.items():
                if pattern.search(line):
                    hard.append({"kind": kind, "path": rel.as_posix(), "line": line_no, "text": line.strip()[:240]})
            if SOFT_PATTERN.search(line):
                soft.append({"path": rel.as_posix(), "line": line_no, "text": line.strip()[:240]})

    report = {
        "schema": "open-ontologies.wip-inventory/v1",
        "scanned_files": scanned,
        "hard_count": len(hard),
        "soft_count": len(soft),
        "hard": hard,
        "soft": soft,
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    return 2 if hard else 0


if __name__ == "__main__":
    raise SystemExit(main())
