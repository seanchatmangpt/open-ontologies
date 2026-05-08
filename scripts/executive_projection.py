#!/usr/bin/env python3
"""
Real-Groq executive projection wrapper.

Reads `admitted_evidence` from argv[1] (or stdin if argv[1] is "-"), calls a
real Groq-backed DSPy ChainOfThought signature constrained to use ONLY tokens
that already appear in the admitted evidence, then computes a token-overlap
audit in Python and emits JSON on stdout.

Usage:
    python3 scripts/executive_projection.py "Reconciliation rate is 83%."
    echo "..." | python3 scripts/executive_projection.py -

Environment:
    GROQ_API_KEY            (required) — real Groq API key
    POWL_MODEL              (optional) — DSPy/Groq model id
                            default: groq/openai/gpt-oss-20b
    POWL_MAX_REFINEMENTS    (optional) — int, default 2

Exit codes:
    0 — success, JSON printed on stdout with keys:
            summary, tokens_used, tokens_invented, verdict, refinements
    2 — usage / config error (missing key, missing dspy, etc)
    3 — generation failed at the LLM boundary

Output JSON shape:
    {
      "summary": "...",
      "tokens_used": ["..."],
      "tokens_invented": ["..."],
      "verdict": true,
      "refinements": 0
    }

The Rust test in tests/real_groq_executive_projection.rs spawns this script
with a real key and asserts the output is parseable JSON with a populated
`summary` field plus an honest token-overlap audit.
"""

from __future__ import annotations

import json
import os
import re
import sys


def _err(msg: str, code: int) -> None:
    print(json.dumps({"error": msg}), file=sys.stderr)
    sys.exit(code)


_TOKEN_RE = re.compile(r"[^A-Za-z0-9]+")


def _audit_tokens(summary: str, evidence: str) -> tuple[list[str], list[str]]:
    """Return (tokens_used, tokens_invented) using substring containment.

    Tokens: split summary on non-alphanumeric, lowercase, keep length>=4 and
    purely alphabetic. A token is "used" iff its lowercase form appears as a
    substring of evidence.lower(); else "invented". De-duplicated, order
    preserved.
    """
    ev_lc = evidence.lower()
    used: list[str] = []
    invented: list[str] = []
    seen: set[str] = set()
    for raw in _TOKEN_RE.split(summary):
        t = raw.lower()
        if len(t) < 4 or not t.isalpha() or t in seen:
            continue
        seen.add(t)
        if t in ev_lc:
            used.append(t)
        else:
            invented.append(t)
    return used, invented


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        _err("usage: executive_projection.py <admitted_evidence | ->", 2)

    if argv[1] == "-":
        evidence = sys.stdin.read().strip()
    else:
        evidence = argv[1].strip()

    if not evidence:
        _err("empty admitted_evidence", 2)

    api_key = os.environ.get("GROQ_API_KEY", "").strip()
    if not api_key:
        _err("GROQ_API_KEY is not set in environment", 2)

    try:
        import dspy  # noqa: WPS433
    except Exception as exc:  # noqa: BLE001
        _err(f"failed to import dspy: {exc!r}", 2)

    model = os.environ.get("POWL_MODEL", "groq/openai/gpt-oss-20b")
    try:
        max_refinements = int(os.environ.get("POWL_MAX_REFINEMENTS", "2"))
    except ValueError:
        max_refinements = 2
    if max_refinements < 0:
        max_refinements = 0

    try:
        lm = dspy.LM(model=model, api_key=api_key)
        dspy.configure(lm=lm)
    except Exception as exc:  # noqa: BLE001
        _err(f"failed to configure dspy LM: {exc!r}", 2)

    class ExecutiveProjectionSignature(dspy.Signature):
        """Produce a faithful executive summary of the admitted evidence.

        STRICT CONSTRAINT: The summary may ONLY use words that already appear
        in admitted_evidence. Do not invent facts, account names, numbers, or
        dates. Do not add information not present in admitted_evidence. If the
        evidence is sparse, the summary must also be sparse.
        """

        admitted_evidence: str = dspy.InputField(
            desc="Ground-truth evidence. The summary must stay within these words."
        )
        summary: str = dspy.OutputField(
            desc="Executive summary using ONLY words present in admitted_evidence."
        )

    class ExecutiveProjectionRefineSignature(dspy.Signature):
        """Rewrite the summary to remove invented tokens.

        Given the prior summary, the admitted_evidence, and a list of issues
        (invented tokens), produce a new summary that only uses words present
        in admitted_evidence.
        """

        admitted_evidence: str = dspy.InputField()
        prior_summary: str = dspy.InputField()
        issues: str = dspy.InputField(
            desc="Description of invented tokens that must be removed."
        )
        summary: str = dspy.OutputField(
            desc="Revised summary using ONLY words present in admitted_evidence."
        )

    generate = dspy.ChainOfThought(ExecutiveProjectionSignature)
    refine = dspy.ChainOfThought(ExecutiveProjectionRefineSignature)

    try:
        pred = generate(admitted_evidence=evidence)
        summary = (pred.summary or "").strip()
    except Exception as exc:  # noqa: BLE001
        _err(f"generation failed: {exc!r}", 3)

    used, invented = _audit_tokens(summary, evidence)
    verdict = len(invented) == 0
    refinements = 0

    while not verdict and refinements < max_refinements:
        refinements += 1
        issues = "invented tokens: " + ", ".join(invented)
        try:
            pred = refine(
                admitted_evidence=evidence,
                prior_summary=summary,
                issues=issues,
            )
            summary = (pred.summary or "").strip()
        except Exception as exc:  # noqa: BLE001
            _err(f"refinement failed: {exc!r}", 3)
        used, invented = _audit_tokens(summary, evidence)
        verdict = len(invented) == 0

    print(
        json.dumps(
            {
                "summary": summary,
                "tokens_used": used,
                "tokens_invented": invented,
                "verdict": verdict,
                "refinements": refinements,
            }
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
