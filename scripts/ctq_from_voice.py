#!/usr/bin/env python3
"""
Thin CLI wrapper around a real Groq-backed DSPy CTQ-Forge generator.

Reads a stakeholder source-voice from argv[1] (or stdin if argv[1] is "-"),
calls the real Groq endpoint via dspy.ChainOfThought, validates the
generated CTQ structure, and prints the result as JSON on stdout.

Output JSON matches the OntoAdmitCtqInput contract (src/inputs.rs):
    source_voice_echo, ctq_text, measure_text, verification_text,
    negative_case_text, control_plan_text, defect_class_hint,
    verdict (bool), refinements (int)

Usage:
    python3 scripts/ctq_from_voice.py "Sales says deals are real, Finance can't reconcile bookings"
    echo "..." | python3 scripts/ctq_from_voice.py -

Environment:
    GROQ_API_KEY            (required) — real Groq API key
    CTQ_MODEL               (optional) — DSPy/Groq model id
                            default: groq/openai/gpt-oss-20b
    CTQ_MAX_REFINEMENTS     (optional) — int, default 2
    CTQ_VOICE_KIND          (optional) — voice category
                            default: "operator"
    PM4PY_FORK_PATH         (optional) — accepted for parity with the
                            POWL wrapper; not required by this script.

Exit codes:
    0 — success, JSON printed on stdout
    2 — usage / config error (missing key, empty input, missing dspy)
    3 — generation failed at the LLM boundary
"""

from __future__ import annotations

import json
import os
import sys


def _err(msg: str, code: int) -> None:
    print(json.dumps({"error": msg}), file=sys.stderr)
    sys.exit(code)


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        _err("usage: ctq_from_voice.py <source_voice | ->", 2)

    if argv[1] == "-":
        source_voice = sys.stdin.read().strip()
    else:
        source_voice = argv[1].strip()

    if not source_voice:
        _err("empty source_voice", 2)

    api_key = os.environ.get("GROQ_API_KEY", "").strip()
    if not api_key:
        _err("GROQ_API_KEY is not set in environment", 2)

    try:
        import dspy  # noqa: WPS433
    except Exception as exc:  # noqa: BLE001
        _err(f"failed to import dspy: {exc!r}", 2)

    model = os.environ.get("CTQ_MODEL", "groq/openai/gpt-oss-20b")
    voice_kind = os.environ.get("CTQ_VOICE_KIND", "operator")
    try:
        max_refinements = int(os.environ.get("CTQ_MAX_REFINEMENTS", "2"))
    except ValueError:
        max_refinements = 2

    # ── DSPy 2.x signatures ──────────────────────────────────────────
    class CtqGenerationSignature(dspy.Signature):
        """Translate stakeholder voice into a candidate Critical-To-Quality (CTQ).

        The five descriptive fields are MANDATORY: the deterministic CTQ
        admission gate denies any candidate missing a measure, verification,
        negative case, or control plan. Do NOT invent facts beyond the input.
        """

        source_voice: str = dspy.InputField(
            desc="Verbatim stakeholder complaint or signal"
        )
        voice_kind: str = dspy.InputField(
            desc="Voice category: customer, operator, process, defect, "
            "control_plan, counterfactual, business, policy, customer_success"
        )
        ctq_text: str = dspy.OutputField(
            desc="One-sentence CTQ statement (>= 20 chars)"
        )
        measure_text: str = dspy.OutputField(
            desc="Measurable indicator (>= 8 chars)"
        )
        verification_text: str = dspy.OutputField(
            desc="How to verify the CTQ holds (>= 8 chars)"
        )
        negative_case_text: str = dspy.OutputField(
            desc="What must be refused (>= 12 chars)"
        )
        control_plan_text: str = dspy.OutputField(
            desc="How regression is prevented (>= 12 chars)"
        )
        defect_class_hint: str = dspy.OutputField(
            desc="Best-guess defect class tag (free text, e.g. ctq_incomplete)"
        )

    class CtqValidatorSignature(dspy.Signature):
        """Validate that a candidate CTQ structure is sound and admissible."""

        source_voice: str = dspy.InputField(desc="Original stakeholder voice")
        ctq_text: str = dspy.InputField(desc="Proposed CTQ statement")
        measure_text: str = dspy.InputField(desc="Proposed measure")
        verification_text: str = dspy.InputField(desc="Proposed verification method")
        negative_case_text: str = dspy.InputField(desc="Proposed negative case")
        control_plan_text: str = dspy.InputField(desc="Proposed control plan")
        is_valid: bool = dspy.OutputField(
            desc="Whether the structure is syntactically complete and non-empty"
        )
        verdict: bool = dspy.OutputField(
            desc="Overall verdict: would the deterministic admission gate accept this CTQ?"
        )
        reasoning: str = dspy.OutputField(desc="Explanation of validation outcome")
        issues: str = dspy.OutputField(desc="Concrete issues found, if any")

    class CtqRefinementSignature(dspy.Signature):
        """Refine a candidate CTQ that failed validation."""

        source_voice: str = dspy.InputField(desc="Original stakeholder voice")
        original_ctq_text: str = dspy.InputField()
        original_measure_text: str = dspy.InputField()
        original_verification_text: str = dspy.InputField()
        original_negative_case_text: str = dspy.InputField()
        original_control_plan_text: str = dspy.InputField()
        issues: str = dspy.InputField(desc="Issues raised by the validator")
        ctq_text: str = dspy.OutputField()
        measure_text: str = dspy.OutputField()
        verification_text: str = dspy.OutputField()
        negative_case_text: str = dspy.OutputField()
        control_plan_text: str = dspy.OutputField()
        defect_class_hint: str = dspy.OutputField()

    # ── Configure LM ─────────────────────────────────────────────────
    try:
        lm = dspy.LM(model=model, api_key=api_key)
    except Exception as exc:  # noqa: BLE001
        _err(f"failed to create LM for model {model}: {exc!r}", 2)

    # ── Few-shot demos (mirror src/signature_shape.rs::ctq_signature) ─
    demos = [
        dspy.Example(
            source_voice="Sales says deals are real, Finance can't reconcile bookings",
            voice_kind="operator",
            ctq_text="Booking must reconcile to contract chain before classification",
            measure_text="Reconciliation completeness rate (admitted bookings with chain / total bookings)",
            verification_text="Run nightly reconciliation comparing booked amounts to invoice/order/contract chain",
            negative_case_text="Refuse classification when invoice has no order_created or contract_executed",
            control_plan_text="Block booking_complete event unless every prior chain stage is observed",
            defect_class_hint="ctq_incomplete",
        ).with_inputs("source_voice", "voice_kind"),
        dspy.Example(
            source_voice="Renewals are coming in late and CS doesn't see them in time",
            voice_kind="customer_success",
            ctq_text="Renewal risk must be detected before deadline based on touchpoint evidence",
            measure_text="Percentage of renewals with required pre-renewal touchpoints completed by threshold",
            verification_text="Daily check that every renewal_due event has matching touchpoint events",
            negative_case_text="A renewal cannot be marked healthy if required touchpoints are absent",
            control_plan_text="Block renewal_healthy classification when touchpoint events absent at threshold",
            defect_class_hint="renewal_risk_undetected",
        ).with_inputs("source_voice", "voice_kind"),
    ]

    with dspy.context(lm=lm):
        generator = dspy.ChainOfThought(CtqGenerationSignature)
        generator.demos = demos
        validator = dspy.ChainOfThought(CtqValidatorSignature)
        refiner = dspy.ChainOfThought(CtqRefinementSignature)

        # ── Initial generation ───────────────────────────────────────
        try:
            gen = generator(source_voice=source_voice, voice_kind=voice_kind)
        except Exception as exc:  # noqa: BLE001
            _err(f"generation failed: {exc!r}", 3)

        ctq_text = (getattr(gen, "ctq_text", "") or "").strip()
        measure_text = (getattr(gen, "measure_text", "") or "").strip()
        verification_text = (getattr(gen, "verification_text", "") or "").strip()
        negative_case_text = (getattr(gen, "negative_case_text", "") or "").strip()
        control_plan_text = (getattr(gen, "control_plan_text", "") or "").strip()
        defect_class_hint = (getattr(gen, "defect_class_hint", "") or "").strip()

        # ── Validate / refine loop ───────────────────────────────────
        final_verdict = False
        refinements_used = 0
        for refinement in range(max_refinements + 1):
            try:
                val = validator(
                    source_voice=source_voice,
                    ctq_text=ctq_text,
                    measure_text=measure_text,
                    verification_text=verification_text,
                    negative_case_text=negative_case_text,
                    control_plan_text=control_plan_text,
                )
            except Exception as exc:  # noqa: BLE001
                if refinement == 0:
                    _err(f"validation failed: {exc!r}", 3)
                break

            is_valid = bool(getattr(val, "is_valid", False))
            verdict = bool(getattr(val, "verdict", False))
            refinements_used = refinement

            if is_valid and verdict:
                final_verdict = True
                break

            if refinement < max_refinements:
                issues = getattr(val, "issues", "") or "fields too short or missing"
                try:
                    ref = refiner(
                        source_voice=source_voice,
                        original_ctq_text=ctq_text,
                        original_measure_text=measure_text,
                        original_verification_text=verification_text,
                        original_negative_case_text=negative_case_text,
                        original_control_plan_text=control_plan_text,
                        issues=issues,
                    )
                except Exception:  # noqa: BLE001
                    break
                ctq_text = (getattr(ref, "ctq_text", ctq_text) or ctq_text).strip()
                measure_text = (getattr(ref, "measure_text", measure_text) or measure_text).strip()
                verification_text = (getattr(ref, "verification_text", verification_text) or verification_text).strip()
                negative_case_text = (getattr(ref, "negative_case_text", negative_case_text) or negative_case_text).strip()
                control_plan_text = (getattr(ref, "control_plan_text", control_plan_text) or control_plan_text).strip()
                new_hint = (getattr(ref, "defect_class_hint", "") or "").strip()
                if new_hint:
                    defect_class_hint = new_hint

    result = {
        "source_voice_echo": source_voice,
        "ctq_text": ctq_text,
        "measure_text": measure_text,
        "verification_text": verification_text,
        "negative_case_text": negative_case_text,
        "control_plan_text": control_plan_text,
        "defect_class_hint": defect_class_hint,
        "verdict": final_verdict,
        "refinements": refinements_used,
    }
    print(json.dumps(result, default=str))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
