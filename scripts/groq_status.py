#!/usr/bin/env python3
"""
Liveness / readiness probe for the real-Groq subprocess engine.

Checks (in order):
  1. `dspy` import succeeds
  2. `GROQ_API_KEY` is present and non-empty in the environment
  3. (best-effort) construction of `dspy.LM(model=…, api_key=…)` succeeds.
     We do NOT make an actual Groq HTTP request — that would (a) burn quota
     on every status probe and (b) leak the key into HTTP-error stack traces
     when network is degraded. Local SDK construction is the strictest check
     we can perform without spending tokens.

Output JSON shape (always emitted, even on error):
    {
      "ok": bool,                  # True iff dspy importable AND key present
      "model_reachable": bool,     # True iff dspy.LM() constructed without error
      "key_present": bool,         # True iff GROQ_API_KEY is non-empty
      "model": str,                # Model id under inspection
      "error": str | null
    }

The API key MUST NOT appear in the output under any circumstance.

Exit codes:
    0 — JSON emitted (regardless of `ok` value)
    Any nonzero exit means stdout may be malformed; treat as ok=false.
"""

from __future__ import annotations

import json
import os
import sys


def main() -> int:
    out: dict = {
        "ok": False,
        "model_reachable": False,
        "key_present": False,
        "model": os.environ.get("CTQ_MODEL", "groq/openai/gpt-oss-20b"),
        "error": None,
    }

    api_key = os.environ.get("GROQ_API_KEY", "").strip()
    out["key_present"] = bool(api_key)

    try:
        import dspy  # noqa: WPS433
    except Exception as exc:  # noqa: BLE001
        out["error"] = f"dspy import failed: {exc!r}"
        # Sanitize any accidental key leakage in repr (defence in depth).
        if api_key and api_key in (out["error"] or ""):
            out["error"] = "dspy import failed (details suppressed: key in error)"
        print(json.dumps(out))
        return 0

    if not out["key_present"]:
        out["error"] = "GROQ_API_KEY is not set in environment"
        print(json.dumps(out))
        return 0

    try:
        _lm = dspy.LM(model=out["model"], api_key=api_key)
        out["model_reachable"] = True
    except Exception as exc:  # noqa: BLE001
        msg = f"dspy.LM construction failed: {exc!r}"
        # Defence in depth: never leak the key value through error chains.
        if api_key and api_key in msg:
            msg = "dspy.LM construction failed (details suppressed: key in error)"
        out["error"] = msg
        print(json.dumps(out))
        return 0

    out["ok"] = True
    print(json.dumps(out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
