#!/usr/bin/env python3
"""
Real-Groq SolutionSpec proposer.

Reads a CTQ admission record (JSON on argv[1] or stdin if "-") with keys:
    ctq_text, measure_text, verification_text, negative_case_text,
    control_plan_text, work_order_receipt_hash

Calls real Groq via DSPy to propose a SolutionSpec, post-validates it
against the constraints in src/manufacturing/mod.rs::validate_spec, and
prints a JSON object on stdout:

    {
      "spec": {
        "name": "...",
        "description": "...",
        "iac_target": "aws",
        "region": "us-east-1",
        "supervisor_children": 4,
        "mcu_target": "esp32",
        "work_order_receipt_hash": "<verbatim from input>"
      },
      "verdict": true,
      "refinements": <int>
    }

The work_order_receipt_hash is FIXED to the input value — the LLM never
gets to invent it. The script injects it after refinement.

Environment:
    GROQ_API_KEY            (required)
    SPEC_MODEL              (optional) default: groq/openai/gpt-oss-20b
    SPEC_MAX_REFINEMENTS    (optional) default: 2

Exit codes:
    0 — success
    2 — usage / config error (missing key, malformed input)
    3 — generation failed at the LLM boundary (or could not reach verdict)
"""

from __future__ import annotations

import json
import os
import re
import sys


NAME_RE = re.compile(r"^[a-z][a-z0-9_]*$")
HEX64_RE = re.compile(r"^[0-9a-fA-F]{64}$")
ALLOWED_IAC = {"aws"}
ALLOWED_MCU = {"esp32", "stm32", "rp2040"}


def _err(msg: str, code: int) -> None:
    print(json.dumps({"error": msg}), file=sys.stderr)
    sys.exit(code)


def _shape_violations(spec: dict) -> list[str]:
    """Mirror of src/manufacturing/mod.rs::validate_spec — returns a list
    of human-readable violations (empty list = spec is shape-valid)."""
    issues: list[str] = []
    name = str(spec.get("name", "")).strip()
    if not NAME_RE.match(name):
        issues.append(f"name must match [a-z][a-z0-9_]*; got {name!r}")
    if not str(spec.get("description", "")).strip():
        issues.append("description must be non-empty")
    iac = str(spec.get("iac_target", "")).strip()
    if iac not in ALLOWED_IAC:
        issues.append(f"iac_target must be one of {sorted(ALLOWED_IAC)}; got {iac!r}")
    region = str(spec.get("region", "")).strip()
    if not region:
        issues.append("region must be non-empty")
    mcu = str(spec.get("mcu_target", "")).strip()
    if mcu not in ALLOWED_MCU:
        issues.append(f"mcu_target must be one of {sorted(ALLOWED_MCU)}; got {mcu!r}")
    try:
        sc = int(spec.get("supervisor_children", 0))
    except (TypeError, ValueError):
        sc = 0
    if sc < 1 or sc > 64:
        issues.append(f"supervisor_children must be in [1, 64]; got {sc!r}")
    return issues


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        _err("usage: spec_from_ctq.py <ctq_json | ->", 2)

    raw = sys.stdin.read() if argv[1] == "-" else argv[1]
    raw = raw.strip()
    if not raw:
        _err("empty CTQ input", 2)

    try:
        ctq = json.loads(raw)
    except Exception as exc:  # noqa: BLE001
        _err(f"CTQ input is not valid JSON: {exc!r}", 2)

    required = [
        "ctq_text",
        "measure_text",
        "verification_text",
        "negative_case_text",
        "control_plan_text",
        "work_order_receipt_hash",
    ]
    missing = [k for k in required if not str(ctq.get(k, "")).strip()]
    if missing:
        _err(f"CTQ input missing required fields: {missing}", 2)

    wor_hash = str(ctq["work_order_receipt_hash"]).strip()
    if not HEX64_RE.match(wor_hash):
        _err(
            f"work_order_receipt_hash must be 64 hex chars; got {wor_hash!r}",
            2,
        )

    api_key = os.environ.get("GROQ_API_KEY", "").strip()
    if not api_key:
        _err("GROQ_API_KEY is not set in environment", 2)

    model = os.environ.get("SPEC_MODEL", "groq/openai/gpt-oss-20b")
    try:
        max_refinements = int(os.environ.get("SPEC_MAX_REFINEMENTS", "2"))
    except ValueError:
        max_refinements = 2

    try:
        import dspy  # noqa: WPS433
    except Exception as exc:  # noqa: BLE001
        _err(f"failed to import dspy: {exc!r}", 2)

    try:
        lm = dspy.LM(model=model, api_key=api_key)
        dspy.configure(lm=lm)
    except Exception as exc:  # noqa: BLE001
        _err(f"failed to configure DSPy LM: {exc!r}", 2)

    class SpecGenerationSignature(dspy.Signature):
        """Propose a SolutionSpec for the OntoStar manufacturing layer
        from an admitted CTQ work order. The spec must satisfy:

          - name  : lowercase identifier matching ^[a-z][a-z0-9_]*$
          - description : one short sentence
          - iac_target  : MUST be exactly "aws"
          - region      : an AWS region (e.g. "us-east-1")
          - supervisor_children : integer in [1, 64]
          - mcu_target  : MUST be one of "esp32", "stm32", "rp2040"

        Do NOT emit a work_order_receipt_hash — the caller injects it.
        """

        ctq_text: str = dspy.InputField(desc="CTQ statement")
        measure_text: str = dspy.InputField(desc="how the CTQ is measured")
        verification_text: str = dspy.InputField(desc="how verification proves it")
        negative_case_text: str = dspy.InputField(desc="counterfactual / negative case")
        control_plan_text: str = dspy.InputField(desc="ongoing control plan")
        issues: str = dspy.InputField(
            desc="prior violations to fix (empty on first pass)"
        )

        name: str = dspy.OutputField(desc="lowercase identifier ^[a-z][a-z0-9_]*$")
        description: str = dspy.OutputField(desc="one short sentence")
        iac_target: str = dspy.OutputField(desc='exactly "aws"')
        region: str = dspy.OutputField(desc='AWS region, e.g. "us-east-1"')
        supervisor_children: int = dspy.OutputField(desc="integer in [1, 64]")
        mcu_target: str = dspy.OutputField(desc='one of "esp32", "stm32", "rp2040"')

    class SpecValidatorSignature(dspy.Signature):
        """Verify that a proposed SolutionSpec satisfies the manufacturing
        layer's shape constraints."""

        name: str = dspy.InputField()
        description: str = dspy.InputField()
        iac_target: str = dspy.InputField()
        region: str = dspy.InputField()
        supervisor_children: int = dspy.InputField()
        mcu_target: str = dspy.InputField()

        verdict: bool = dspy.OutputField(
            desc="true iff name matches ^[a-z][a-z0-9_]*$, "
            'iac_target=="aws", mcu_target in {"esp32","stm32","rp2040"}, '
            "and supervisor_children in [1,64]"
        )
        reasoning: str = dspy.OutputField()

    proposer = dspy.ChainOfThought(SpecGenerationSignature)
    validator = dspy.ChainOfThought(SpecValidatorSignature)

    issues_str = ""
    spec: dict = {}
    refinements = 0
    last_violations: list[str] = []
    for attempt in range(max_refinements + 1):
        try:
            pred = proposer(
                ctq_text=ctq["ctq_text"],
                measure_text=ctq["measure_text"],
                verification_text=ctq["verification_text"],
                negative_case_text=ctq["negative_case_text"],
                control_plan_text=ctq["control_plan_text"],
                issues=issues_str,
            )
        except Exception as exc:  # noqa: BLE001
            _err(f"spec proposal failed: {exc!r}", 3)

        try:
            sc_int = int(pred.supervisor_children)
        except (TypeError, ValueError):
            sc_int = 0
        spec = {
            "name": str(pred.name).strip(),
            "description": str(pred.description).strip(),
            "iac_target": str(pred.iac_target).strip(),
            "region": str(pred.region).strip(),
            "supervisor_children": sc_int,
            "mcu_target": str(pred.mcu_target).strip(),
        }
        last_violations = _shape_violations(spec)
        if not last_violations:
            break
        refinements = attempt + 1
        issues_str = "; ".join(last_violations)

    # Post-validate via the DSPy validator signature for the verdict
    # field — but the AUTHORITATIVE verdict is the inline shape check
    # (we mirror Rust validate_spec). The validator call is a sanity
    # cross-check; if our shape check passes we report verdict=true.
    verdict = not last_violations
    if verdict:
        try:
            v = validator(
                name=spec["name"],
                description=spec["description"],
                iac_target=spec["iac_target"],
                region=spec["region"],
                supervisor_children=spec["supervisor_children"],
                mcu_target=spec["mcu_target"],
            )
            # If the LLM validator disagrees with our deterministic check,
            # trust the deterministic check (it mirrors Rust).
            _ = v.verdict
        except Exception:  # noqa: BLE001
            # Validator failure does not flip the deterministic verdict.
            pass

    # Inject the FIXED receipt hash from the input — the LLM never
    # gets to choose this.
    spec["work_order_receipt_hash"] = wor_hash

    out = {
        "spec": spec,
        "verdict": verdict,
        "refinements": refinements,
        "violations": last_violations,
    }
    print(json.dumps(out))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
