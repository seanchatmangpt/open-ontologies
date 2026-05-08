#!/usr/bin/env python3
"""
Real-Groq POWL refiner.

Given a discovered POWL string + the original natural-language process
description + a free-text description of what's wrong, calls real Groq
through DSPy via pm4py.algo.dspy.powl.natural_language.POWLRefinementSignature
to produce a tightened POWL that better matches the description.

After each refinement attempt, the produced POWL is run through the
in-fork PowlValidatorSignature (defined inline inside
_generate_powl_from_text_inner). If the validator's verdict is still
False, accumulated issues are fed back into a second refine pass, up
to POWL_MAX_REFINEMENTS attempts (default 2).

Usage:
    python3 scripts/powl_refine.py \
        --original-powl "X (Submit, Manager review)" \
        --description "Submit expense report, then manager reviews it" \
        --issues "should be sequential not exclusive choice"

  Or pipe a single JSON object on stdin with the same keys
  (kebab-case or snake_case) and pass `-` as the only argv:

    echo '{"original_powl":"...","description":"...","issues":"..."}' \
      | python3 scripts/powl_refine.py -

Environment:
    GROQ_API_KEY            (required)
    PM4PY_FORK_PATH         (optional) default /Users/sac/chatmangpt/pm4py
    POWL_MODEL              (optional) default groq/openai/gpt-oss-20b
    POWL_MAX_REFINEMENTS    (optional) default 2
    POWL_DOMAIN             (optional) default "general"

Stdout JSON (last line):
    {
      "original_powl": "...",
      "refined_powl":  "...",
      "changed":       <bool>,
      "verdict":       <bool>,
      "reasoning":     "...",
      "issues":        "...",
      "refinements":   <int>
    }

Exit codes:
    0 — success
    2 — usage / config error (missing key/fork/empty original_powl)
    3 — refinement failed at LLM boundary
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path


def _err(msg: str, code: int) -> None:
    print(json.dumps({"error": msg}), file=sys.stderr)
    sys.exit(code)


def _parse_args(argv: list[str]) -> tuple[str, str, str]:
    # Stdin JSON form: a single argv "-"
    if len(argv) == 2 and argv[1] == "-":
        try:
            payload = json.loads(sys.stdin.read())
        except Exception as exc:  # noqa: BLE001
            _err(f"invalid stdin JSON: {exc!r}", 2)
        original = payload.get("original_powl") or payload.get("original-powl") or ""
        description = payload.get("description") or ""
        issues = payload.get("issues") or ""
        return str(original), str(description), str(issues)

    parser = argparse.ArgumentParser(prog="powl_refine.py", add_help=True)
    parser.add_argument("--original-powl", dest="original_powl", required=True)
    parser.add_argument("--description", dest="description", required=True)
    parser.add_argument("--issues", dest="issues", default="")
    ns = parser.parse_args(argv[1:])
    return ns.original_powl, ns.description, ns.issues


def main(argv: list[str]) -> int:
    original_powl, description, issues_in = _parse_args(argv)

    if not original_powl.strip():
        _err("empty original_powl", 2)
    if not description.strip():
        _err("empty description", 2)

    api_key = os.environ.get("GROQ_API_KEY", "").strip()
    if not api_key:
        _err("GROQ_API_KEY is not set in environment", 2)

    fork_path = os.environ.get("PM4PY_FORK_PATH", "/Users/sac/chatmangpt/pm4py")
    if not Path(fork_path).is_dir():
        _err(f"pm4py fork not found at {fork_path}", 2)

    sys.path.insert(0, fork_path)

    try:
        import dspy  # noqa: WPS433
        from pm4py.algo.dspy.powl.natural_language import (  # noqa: WPS433
            DSPY_AVAILABLE,
            DEFAULT_MODEL,
            POWLRefinementSignature,
        )
    except Exception as exc:  # noqa: BLE001
        _err(f"failed to import pm4py POWL module: {exc!r}", 2)

    if not DSPY_AVAILABLE:
        _err("DSPy is not available in this Python environment", 2)

    model = os.environ.get("POWL_MODEL", DEFAULT_MODEL)
    try:
        max_refinements = int(os.environ.get("POWL_MAX_REFINEMENTS", "2"))
    except ValueError:
        max_refinements = 2
    max_refinements = max(1, max_refinements)

    # Configure DSPy with real Groq.
    try:
        lm = dspy.LM(model=model, api_key=api_key)
    except Exception as exc:  # noqa: BLE001
        _err(f"dspy.LM init failed: {exc!r}", 3)

    # Mirror the validator signature defined inline inside
    # _generate_powl_from_text_inner so we can re-validate after refine.
    class PowlValidatorSignature(dspy.Signature):
        """Validate POWL model syntax and semantics."""
        powl: str = dspy.InputField(desc="POWL model string to validate")
        description: str = dspy.InputField(desc="Original process description")
        is_valid: bool = dspy.OutputField(desc="Whether POWL is syntactically valid")
        verdict: bool = dspy.OutputField(desc="Overall verdict: structurally sound or not")
        reasoning: str = dspy.OutputField(desc="Explanation of the validation result")
        issues: str = dspy.OutputField(desc="List of issues found, if any")

    current_powl = original_powl
    accumulated_issues = issues_in.strip()
    last_reasoning = ""
    last_verdict = False
    last_issues = accumulated_issues
    attempts = 0

    with dspy.context(lm=lm):
        refiner = dspy.ChainOfThought(POWLRefinementSignature)
        validator = dspy.ChainOfThought(PowlValidatorSignature)

        for attempt in range(max_refinements):
            attempts = attempt + 1
            try:
                refine_result = refiner(
                    original_powl=current_powl,
                    issues=accumulated_issues
                    or "Tighten the model so it best matches: " + description,
                )
            except Exception as exc:  # noqa: BLE001
                _err(f"refine call failed: {exc!r}", 3)

            refined = getattr(refine_result, "refined_powl", "") or ""
            refined = refined.strip()
            if refined:
                current_powl = refined

            # Re-validate.
            try:
                v = validator(powl=current_powl, description=description)
            except Exception as exc:  # noqa: BLE001
                # Treat validator failure as non-fatal: keep going with the
                # last refined POWL but record the error in reasoning.
                last_reasoning = f"validator error: {exc!r}"
                last_verdict = False
                last_issues = accumulated_issues
                break

            last_reasoning = getattr(v, "reasoning", "") or ""
            last_verdict = bool(getattr(v, "verdict", False)) and bool(
                getattr(v, "is_valid", False)
            )
            last_issues = getattr(v, "issues", "") or ""

            if last_verdict:
                break

            # Otherwise accumulate the validator's issues into the next
            # refine pass, prefixed by the user's original complaint so
            # both signals reach the LLM.
            next_issues = []
            if issues_in.strip():
                next_issues.append(issues_in.strip())
            if last_issues.strip() and last_issues.strip() not in next_issues:
                next_issues.append(last_issues.strip())
            accumulated_issues = "\n".join(next_issues)

    out = {
        "original_powl": original_powl,
        "refined_powl": current_powl,
        "changed": current_powl.strip() != original_powl.strip(),
        "verdict": last_verdict,
        "reasoning": last_reasoning,
        "issues": last_issues,
        "refinements": attempts,
    }
    print(json.dumps(out, default=str))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
