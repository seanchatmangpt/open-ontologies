#!/usr/bin/env python3
"""Activate the admitted Zoela ggen rules and verify their query authorities.

This script is intentionally narrow. It edits exactly eleven commented rule
entries in ggen.toml, removes only their adjacent temporary-disable comments,
and refuses missing or duplicate rules. Re-running --apply is byte-idempotent.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "ggen.toml"

RULES: dict[str, str] = {
    "zoela-service-routes": ".specify/queries/zoela/extract-service-routes.rq",
    "zoela-receipts": ".specify/queries/zoela/extract-receipt-models.rq",
    "zoela-roles": ".specify/queries/zoela/extract-role-requirements.rq",
    "zoela-assignment-eligibility": ".specify/queries/zoela/extract-assignment-eligibility.rq",
    "zoela-resource-match": ".specify/queries/zoela/extract-resource-matching-rules.rq",
    "zoela-followup-rules": ".specify/queries/zoela/extract-followup-rules.rq",
    "zoela-cg-stages": ".specify/queries/zoela/extract-connect-group-stages.rq",
    "zoela-cg-work-orders": ".specify/queries/zoela/extract-connect-group-work-orders.rq",
    "zoela-cg-admin": ".specify/queries/zoela/extract-connect-group-admin.rq",
    "zoela-cg-interest-form": ".specify/queries/zoela/extract-connect-group-stages.rq",
    "zoela-edge-fn": ".specify/queries/zoela/extract-service-routes.rq",
}

EXECUTABLE_VALUES = re.compile(r"(?im)^\s*VALUES\s*\(")


def activate_manifest(text: str) -> str:
    lines = text.splitlines(keepends=True)
    output: list[str] = []
    activated: set[str] = set()

    for line in lines:
        disabled_match = re.match(r"^(\s*)#\s*(\{\s*name\s*=\s*\"([^\"]+)\".*)$", line)
        if disabled_match and disabled_match.group(3) in RULES:
            name = disabled_match.group(3)
            if name in activated:
                raise ValueError(f"duplicate disabled rule: {name}")
            output.append(disabled_match.group(1) + disabled_match.group(2))
            if line.endswith("\n") and not output[-1].endswith("\n"):
                output[-1] += "\n"
            activated.add(name)
            continue

        if "TEMPORARILY DISABLED" in line:
            named = next((name for name in RULES if name in line), None)
            generic = (
                "uses extract-connect-group-stages.rq" in line
                or "uses extract-service-routes.rq" in line
            )
            if named is not None or generic:
                continue

        output.append(line)

    rendered = "".join(output)
    for name, query in RULES.items():
        active = re.findall(
            rf"(?m)^\s*\{{\s*name\s*=\s*\"{re.escape(name)}\"[^\n]*query\s*=\s*\{{\s*file\s*=\s*\"{re.escape(query)}\"",
            rendered,
        )
        if len(active) != 1:
            raise ValueError(f"expected one active {name} rule for {query}, found {len(active)}")
        disabled = re.findall(rf"(?m)^\s*#\s*\{{\s*name\s*=\s*\"{re.escape(name)}\"", rendered)
        if disabled:
            raise ValueError(f"disabled copy remains for {name}")

    remaining = [line for line in rendered.splitlines() if "TEMPORARILY DISABLED" in line]
    if remaining:
        raise ValueError(f"temporary-disable comments remain: {remaining}")
    return rendered


def verify_queries() -> None:
    for query in sorted(set(RULES.values())):
        path = ROOT / query
        if not path.is_file():
            raise ValueError(f"missing query authority: {query}")
        text = path.read_text(encoding="utf-8")
        if EXECUTABLE_VALUES.search(text):
            raise ValueError(f"executable VALUES table remains: {query}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--apply", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()

    original = MANIFEST.read_text(encoding="utf-8")
    rendered = activate_manifest(original)
    verify_queries()

    if args.apply:
        MANIFEST.write_text(rendered, encoding="utf-8")
        replay = activate_manifest(MANIFEST.read_text(encoding="utf-8"))
        if replay != rendered:
            raise ValueError("second activation replay is not byte-idempotent")
        print(f"activated {len(RULES)} Zoela rules; replay byte-identical")
        return 0

    if rendered != original:
        raise ValueError("Zoela rules are not fully activated; run --apply")
    print(f"verified {len(RULES)} active Zoela rules and {len(set(RULES.values()))} query authorities")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
