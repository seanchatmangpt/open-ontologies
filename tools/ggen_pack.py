#!/usr/bin/env python3
"""Manufacture the in-repo open-ontologies ggen pack from canonical sources."""
from __future__ import annotations

import argparse
import hashlib
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACK = ROOT / "packs" / "open-ontologies-pack"
PROJECTIONS = {
    ROOT / "ontology" / "cli-open-ontologies.ttl": PACK / "ontology.ttl",
    ROOT / ".specify" / "queries" / "cli" / "commands_aggregated.rq": PACK / "queries" / "commands_aggregated.rq",
    ROOT / ".specify" / "templates" / "cli" / "cmds.rs.tera": PACK / "templates" / "cmds.rs.tera",
}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write() -> int:
    for source, target in PROJECTIONS.items():
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        print(f"WRITE {target.relative_to(ROOT)} sha256={digest(target)}")
    return 0


def check() -> int:
    failures = []
    for source, target in PROJECTIONS.items():
        if not source.is_file():
            failures.append(f"missing source: {source.relative_to(ROOT)}")
            continue
        if not target.is_file():
            failures.append(f"missing projection: {target.relative_to(ROOT)}")
            continue
        if source.read_bytes() != target.read_bytes():
            failures.append(
                f"drift: {target.relative_to(ROOT)} != {source.relative_to(ROOT)}"
            )
            continue
        print(
            f"MATCH {target.relative_to(ROOT)} <- {source.relative_to(ROOT)} "
            f"sha256={digest(target)}"
        )
    if failures:
        for failure in failures:
            print(f"REFUSED:GGEN_PACK_DRIFT {failure}", file=sys.stderr)
        return 1
    print("ALIVE:GGEN_PACK_PROJECTIONS_MATCH")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    return write() if args.write else check()


if __name__ == "__main__":
    raise SystemExit(main())
