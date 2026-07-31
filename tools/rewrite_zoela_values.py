#!/usr/bin/env python3
"""Normalize Zoela SPARQL VALUES tables and re-enable their ggen rules.

The pinned ggen/Oxigraph rail does not admit VALUES in these manufacturing
queries. This tool performs a deterministic, semantics-preserving rewrite:

    VALUES (?x ?y) { ("a" "b") ("c" "d") }

becomes a nested UNION of BIND-only rows. It also re-enables exactly the
Zoela rules whose only recorded blocker was VALUES support.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
QUERY_PATHS = (
    ".specify/queries/zoela/extract-service-routes.rq",
    ".specify/queries/zoela/extract-receipt-models.rq",
    ".specify/queries/zoela/extract-role-requirements.rq",
    ".specify/queries/zoela/extract-assignment-eligibility.rq",
    ".specify/queries/zoela/extract-resource-matching-rules.rq",
    ".specify/queries/zoela/extract-followup-rules.rq",
    ".specify/queries/zoela/extract-connect-group-stages.rq",
    ".specify/queries/zoela/extract-connect-group-work-orders.rq",
    ".specify/queries/zoela/extract-connect-group-admin.rq",
)
RULE_NAMES = (
    "zoela-service-routes",
    "zoela-receipts",
    "zoela-roles",
    "zoela-assignment-eligibility",
    "zoela-resource-match",
    "zoela-followup-rules",
    "zoela-cg-stages",
    "zoela-cg-work-orders",
    "zoela-cg-admin",
    "zoela-cg-interest-form",
    "zoela-edge-fn",
)
VALUES_RE = re.compile(r"\bVALUES\b", re.IGNORECASE)
VARIABLE_RE = re.compile(r"\?[A-Za-z_][A-Za-z0-9_-]*")


class RewriteError(RuntimeError):
    pass


def _skip_ws(text: str, index: int) -> int:
    while index < len(text) and text[index].isspace():
        index += 1
    return index


def _matching(text: str, start: int, opening: str, closing: str) -> int:
    if start >= len(text) or text[start] != opening:
        raise RewriteError(f"expected {opening!r} at offset {start}")
    depth = 0
    quote: str | None = None
    escaped = False
    iri = False
    for index in range(start, len(text)):
        char = text[index]
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            continue
        if iri:
            if char == ">":
                iri = False
            continue
        if char in {'"', "'"}:
            quote = char
            continue
        if char == "<":
            iri = True
            continue
        if char == opening:
            depth += 1
        elif char == closing:
            depth -= 1
            if depth == 0:
                return index
    raise RewriteError(f"unclosed {opening!r} beginning at offset {start}")


def _tokenize_row(row: str) -> list[str]:
    tokens: list[str] = []
    index = 0
    while index < len(row):
        index = _skip_ws(row, index)
        if index >= len(row):
            break
        start = index
        if row[index] in {'"', "'"}:
            quote = row[index]
            index += 1
            escaped = False
            while index < len(row):
                char = row[index]
                index += 1
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == quote:
                    break
            else:
                raise RewriteError(f"unterminated string literal in row: {row!r}")
            if row.startswith("^^", index):
                index += 2
                if index < len(row) and row[index] == "<":
                    end = row.find(">", index + 1)
                    if end < 0:
                        raise RewriteError(f"unterminated datatype IRI in row: {row!r}")
                    index = end + 1
                else:
                    while index < len(row) and not row[index].isspace():
                        index += 1
            elif index < len(row) and row[index] == "@":
                index += 1
                while index < len(row) and (row[index].isalnum() or row[index] == "-"):
                    index += 1
            tokens.append(row[start:index])
            continue
        if row[index] == "<":
            end = row.find(">", index + 1)
            if end < 0:
                raise RewriteError(f"unterminated IRI in row: {row!r}")
            tokens.append(row[index : end + 1])
            index = end + 1
            continue
        while index < len(row) and not row[index].isspace():
            index += 1
        tokens.append(row[start:index])
    return tokens


def _parse_rows(body: str, variables: list[str], source: Path) -> list[list[str]]:
    rows: list[list[str]] = []
    index = 0
    while True:
        index = _skip_ws(body, index)
        if index >= len(body):
            break
        if body[index] == "#":
            newline = body.find("\n", index)
            index = len(body) if newline < 0 else newline + 1
            continue
        if body[index] != "(":
            excerpt = body[index : index + 80].replace("\n", " ")
            raise RewriteError(f"{source}: expected VALUES row, found {excerpt!r}")
        end = _matching(body, index, "(", ")")
        tokens = _tokenize_row(body[index + 1 : end])
        if len(tokens) != len(variables):
            raise RewriteError(
                f"{source}: row has {len(tokens)} terms for {len(variables)} variables: {tokens}"
            )
        rows.append(tokens)
        index = end + 1
    if not rows:
        raise RewriteError(f"{source}: VALUES block contains no rows")
    return rows


def _render_union(variables: list[str], rows: list[list[str]], indent: str) -> str:
    row_blocks: list[str] = []
    for tokens in rows:
        binds = []
        for variable, token in zip(variables, tokens, strict=True):
            if token.upper() == "UNDEF":
                continue
            binds.append(f"{indent}    BIND({token} AS {variable})")
        body = "\n".join(binds) if binds else f"{indent}    # all values intentionally unbound"
        row_blocks.append(f"{indent}  {{\n{body}\n{indent}  }}")
    return f"{indent}{{\n" + f"\n{indent}  UNION\n".join(row_blocks) + f"\n{indent}}}"


def rewrite_values(text: str, source: Path) -> tuple[str, int]:
    rewrites = 0
    search_from = 0
    while True:
        match = VALUES_RE.search(text, search_from)
        if match is None:
            break
        start = match.start()
        cursor = _skip_ws(text, match.end())
        if cursor >= len(text) or text[cursor] != "(":
            raise RewriteError(f"{source}: only parenthesized VALUES variables are admitted")
        variables_end = _matching(text, cursor, "(", ")")
        variables = VARIABLE_RE.findall(text[cursor + 1 : variables_end])
        if not variables:
            raise RewriteError(f"{source}: VALUES block has no variables")
        body_start = _skip_ws(text, variables_end + 1)
        if body_start >= len(text) or text[body_start] != "{":
            raise RewriteError(f"{source}: VALUES variable list is not followed by a body")
        body_end = _matching(text, body_start, "{", "}")
        rows = _parse_rows(text[body_start + 1 : body_end], variables, source)
        line_start = text.rfind("\n", 0, start) + 1
        indent = text[line_start:start]
        replacement = _render_union(variables, rows, indent)
        text = text[:start] + replacement + text[body_end + 1 :]
        search_from = start + len(replacement)
        rewrites += 1
    return text, rewrites


def rewrite_manifest(text: str) -> str:
    found = {name: 0 for name in RULE_NAMES}
    out: list[str] = []
    for line in text.splitlines():
        if "TEMPORARILY DISABLED" in line:
            continue
        matched = next((name for name in RULE_NAMES if f'name = "{name}"' in line), None)
        if matched is not None:
            found[matched] += 1
            stripped = line.lstrip()
            indent = line[: len(line) - len(stripped)]
            if stripped.startswith("# "):
                line = indent + stripped[2:]
            elif stripped.startswith("#"):
                line = indent + stripped[1:].lstrip()
        out.append(line)
    missing = [name for name, count in found.items() if count != 1]
    if missing:
        raise RewriteError(f"ggen.toml must contain each target rule exactly once: {missing}")
    result = "\n".join(out) + "\n"
    for name in RULE_NAMES:
        active = re.search(rf'^\s*\{{\s*name\s*=\s*"{re.escape(name)}"', result, re.MULTILINE)
        if active is None:
            raise RewriteError(f"rule was not re-enabled: {name}")
    return result


def apply() -> None:
    total = 0
    for relative in QUERY_PATHS:
        path = ROOT / relative
        if not path.is_file():
            raise RewriteError(f"missing query authority: {relative}")
        original = path.read_text(encoding="utf-8")
        rewritten, count = rewrite_values(original, path)
        if VALUES_RE.search(rewritten):
            raise RewriteError(f"VALUES remains after rewrite: {relative}")
        path.write_text(rewritten, encoding="utf-8")
        total += count
    manifest = ROOT / "ggen.toml"
    manifest.write_text(rewrite_manifest(manifest.read_text(encoding="utf-8")), encoding="utf-8")
    print(f"rewrote {total} VALUES blocks and re-enabled {len(RULE_NAMES)} Zoela rules")


def check() -> None:
    failures: list[str] = []
    for relative in QUERY_PATHS:
        path = ROOT / relative
        if not path.is_file():
            failures.append(f"missing query authority: {relative}")
        elif VALUES_RE.search(path.read_text(encoding="utf-8")):
            failures.append(f"VALUES remains: {relative}")
    manifest_text = (ROOT / "ggen.toml").read_text(encoding="utf-8")
    if "TEMPORARILY DISABLED" in manifest_text:
        failures.append("ggen.toml retains TEMPORARILY DISABLED markers")
    for name in RULE_NAMES:
        if re.search(rf'^\s*\{{\s*name\s*=\s*"{re.escape(name)}"', manifest_text, re.MULTILINE) is None:
            failures.append(f"inactive rule: {name}")
    if failures:
        raise RewriteError("; ".join(failures))
    print("Zoela VALUES normalization and rule activation are complete")


def main() -> int:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--apply", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    try:
        apply() if args.apply else check()
    except RewriteError as error:
        print(f"ERROR: {error}")
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
