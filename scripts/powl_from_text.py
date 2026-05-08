#!/usr/bin/env python3
"""
Thin CLI wrapper around pm4py.algo.dspy.powl.natural_language.generate_powl_from_text.

Reads a process description from argv[1] (or stdin if argv[1] is "-"), calls
the real Groq-backed POWL generator, and prints the result as JSON on stdout.

Usage:
    python3 scripts/powl_from_text.py "Approve purchase orders under $1000"
    echo "Expense report submission" | python3 scripts/powl_from_text.py -

Environment:
    GROQ_API_KEY            (required) — real Groq API key
    PM4PY_FORK_PATH         (optional) — path to chatmangpt/pm4py fork
                            default: /Users/sac/chatmangpt/pm4py
    POWL_MODEL              (optional) — DSPy/Groq model id
                            default: groq/openai/gpt-oss-20b
    POWL_MAX_REFINEMENTS    (optional) — int, default 2
    POWL_DOMAIN             (optional) — domain string for the signature
                            default: "general"

Exit codes:
    0 — success, JSON printed on stdout with keys:
            powl, verdict, reasoning, refinements, analysis, feedback
    2 — usage / config error (missing key, missing pm4py fork, etc)
    3 — generation failed at the LLM boundary

The Rust test in tests/real_groq_powl.rs spawns this script with a real
key and asserts the output is parseable JSON with a non-empty `powl` field.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path


def _err(msg: str, code: int) -> None:
    print(json.dumps({"error": msg}), file=sys.stderr)
    sys.exit(code)


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        _err("usage: powl_from_text.py <process_description | ->", 2)

    # Read description from argv or stdin
    if argv[1] == "-":
        description = sys.stdin.read().strip()
    else:
        description = argv[1].strip()

    if not description:
        _err("empty process description", 2)

    api_key = os.environ.get("GROQ_API_KEY", "").strip()
    if not api_key:
        _err("GROQ_API_KEY is not set in environment", 2)

    fork_path = os.environ.get(
        "PM4PY_FORK_PATH", "/Users/sac/chatmangpt/pm4py"
    )
    if not Path(fork_path).is_dir():
        _err(f"pm4py fork not found at {fork_path}", 2)

    # Inject the fork ahead of any installed pm4py package so the
    # `pm4py.algo.dspy.powl` namespace resolves to the chatmangpt fork
    # (which carries the DSPy/POWL extension; vanilla pm4py from PyPI
    # does not).
    sys.path.insert(0, fork_path)

    try:
        from pm4py.algo.dspy.powl.natural_language import (  # noqa: WPS433
            generate_powl_from_text,
            DSPY_AVAILABLE,
        )
    except Exception as exc:  # noqa: BLE001
        _err(f"failed to import pm4py POWL module: {exc!r}", 2)

    if not DSPY_AVAILABLE:
        _err("DSPy is not available in this Python environment", 2)

    model = os.environ.get("POWL_MODEL", "groq/openai/gpt-oss-20b")
    domain = os.environ.get("POWL_DOMAIN", "general")
    try:
        max_refinements = int(os.environ.get("POWL_MAX_REFINEMENTS", "2"))
    except ValueError:
        max_refinements = 2

    try:
        result = generate_powl_from_text(
            process_description=description,
            model=model,
            max_refinements=max_refinements,
            use_demos=True,
            domain=domain,
        )
    except RuntimeError as exc:
        _err(str(exc), 2)
    except Exception as exc:  # noqa: BLE001
        _err(f"generation failed: {exc!r}", 3)

    # generate_powl_from_text returns a dict; we just dump it.
    print(json.dumps(result, default=str))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
