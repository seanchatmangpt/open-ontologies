#!/usr/bin/env python3
"""
Simulate a human stakeholder voice via real Groq.

Chicago-TDD: the test describes observable business behaviour. Wherever a
real human would speak (Sales VP complaining, CFO demanding reconciliation,
architect picking a region), this script generates the line via real Groq
so the test exercises the same interaction path a production line would.

Usage:
    python3 scripts/simulate_stakeholder.py <role> <topic>
    echo "<topic>" | python3 scripts/simulate_stakeholder.py <role> -

Roles:
    vp_sales              "Sales says deals are real…"
    cfo                   "Finance can't reconcile bookings…"
    customer_success      "Renewals are coming in late…"
    partner_ops           "Partner attribution is wrong…"
    architect             picks iac/mcu parameters for a SolutionSpec
    senior_reviewer       reviews a CTQ or spec, returns {admit:bool, reason}
    adversarial           hostile stakeholder demanding bypass of CTQ admission

Output JSON (default):
    {"role": "...", "voice": "<3-5 sentence stakeholder line>"}

Output JSON (architect / senior_reviewer modes):
    architect:        {"role":"architect", "iac_target":"aws", "region":"...",
                       "mcu_target":"...", "supervisor_children": N,
                       "rationale":"..."}
    senior_reviewer:  {"role":"senior_reviewer", "admit": bool,
                       "reason":"...", "concerns":[...]}

Environment:
    GROQ_API_KEY            (required)
    POWL_MODEL              (optional, default groq/openai/gpt-oss-20b)

Exit codes:
    0 — JSON on stdout
    2 — usage / config error
    3 — generation failed at LLM boundary
"""
from __future__ import annotations

import json
import os
import sys

try:
    import dspy  # type: ignore
    DSPY_AVAILABLE = True
except ImportError:
    DSPY_AVAILABLE = False
    dspy = None  # type: ignore


def _err(msg: str, code: int) -> None:
    print(json.dumps({"error": msg}), file=sys.stderr)
    sys.exit(code)


ROLE_PROMPTS = {
    "vp_sales": (
        "You are a VP of Sales at a Fortune-5 company. You distrust how "
        "Finance reconciles your bookings and want the pipeline forecast "
        "to reflect what you actually see in the deals. Speak in 3–5 "
        "sentences, frustrated but precise. Cite deals, quotas, percentages."
    ),
    "cfo": (
        "You are the CFO at a Fortune-5 company. You see bookings on the "
        "P&L that don't trace back to executed contracts. Speak in 3–5 "
        "sentences, formal, demanding evidence. Mention reconciliation, "
        "audit risk, and a specific recent reporting period."
    ),
    "customer_success": (
        "You are a Customer Success leader. Renewals keep coming in late "
        "or failing because the team didn't see the warning signs. Speak "
        "in 3–5 sentences, operational, citing missed touchpoints and "
        "specific renewal cohorts."
    ),
    "partner_ops": (
        "You are Partner Operations. Partner attribution keeps showing the "
        "wrong channel because deals are getting attributed AFTER contract "
        "execution, not before. Speak in 3–5 sentences, concrete examples."
    ),
    "adversarial": (
        "You are a stakeholder who wants to bypass the CTQ admission gate "
        "because the deadline is Friday and the team can't be bothered "
        "with measurement methods or negative cases. Speak in 3–5 sentences "
        "demanding the system just admit the requirement without the usual "
        "rigor. Be polite but pushy. Do NOT mention any technical defect "
        "tags by name."
    ),
}


if DSPY_AVAILABLE:
    class StakeholderVoiceSignature(dspy.Signature):
        """Speak in role as the named stakeholder about the supplied topic."""
        role_persona: str = dspy.InputField(
            desc="System persona: who you are and how you speak."
        )
        topic: str = dspy.InputField(
            desc="The concern, defect, or question this stakeholder is "
                 "speaking about. Stay on topic."
        )
        voice: str = dspy.OutputField(
            desc="3–5 sentence first-person stakeholder line. Realistic, "
                 "specific, not corporate boilerplate. No bullet points."
        )

    class ArchitectChoiceSignature(dspy.Signature):
        """Pick deterministic SolutionSpec parameters from a CTQ + topic."""
        ctq_text: str = dspy.InputField(desc="The admitted CTQ statement.")
        topic: str = dspy.InputField(
            desc="Domain context (e.g. 'RevOps revenue trust at Fortune-5 scale')."
        )
        iac_target: str = dspy.OutputField(
            desc="Cloud target. MUST be exactly 'aws' (other targets unsupported)."
        )
        region: str = dspy.OutputField(
            desc="Cloud region appropriate to the workload. e.g. us-east-1, "
                 "eu-west-1, ap-northeast-1."
        )
        mcu_target: str = dspy.OutputField(
            desc="Microcontroller target. MUST be one of: esp32, stm32, rp2040."
        )
        supervisor_children: int = dspy.OutputField(
            desc="Erlang/OTP supervisor children count, integer between 1 and 64."
        )
        rationale: str = dspy.OutputField(
            desc="One-paragraph rationale citing the CTQ."
        )

    class SeniorReviewerSignature(dspy.Signature):
        """Senior reviewer evaluates whether a CTQ + spec should be admitted."""
        artifact_kind: str = dspy.InputField(
            desc="What is being reviewed: 'ctq' or 'solution_spec'."
        )
        artifact_json: str = dspy.InputField(
            desc="The candidate artifact serialized as JSON."
        )
        original_voice: str = dspy.InputField(
            desc="The stakeholder voice this artifact derives from."
        )
        admit: bool = dspy.OutputField(
            desc="True iff the artifact correctly addresses the stakeholder "
                 "concern AND every required field is non-trivially populated. "
                 "Reject if generic, vague, or missing measurable content."
        )
        reason: str = dspy.OutputField(
            desc="One-sentence verdict reason."
        )
        concerns: str = dspy.OutputField(
            desc="Comma-separated list of specific concerns. Empty if admit=true."
        )


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        _err("usage: simulate_stakeholder.py <role> <topic|->", 2)
    role = argv[1].strip()
    topic_raw = argv[2]
    topic = (sys.stdin.read() if topic_raw == "-" else topic_raw).strip()
    if not topic:
        _err("empty topic", 2)
    api_key = os.environ.get("GROQ_API_KEY", "").strip()
    if not api_key:
        _err("GROQ_API_KEY is not set", 2)
    if not DSPY_AVAILABLE:
        _err("dspy is not installed", 2)
    model = os.environ.get("POWL_MODEL", "groq/openai/gpt-oss-20b")

    try:
        lm = dspy.LM(model=model, api_key=api_key)
    except Exception as exc:  # noqa: BLE001
        _err(f"failed to build LM: {exc!r}", 2)

    with dspy.context(lm=lm):
        if role == "architect":
            return _architect(topic)
        if role == "senior_reviewer":
            return _senior_reviewer(topic)
        if role not in ROLE_PROMPTS:
            _err(f"unknown role: {role}", 2)
        return _voice(role, topic)


def _voice(role: str, topic: str) -> int:
    persona = ROLE_PROMPTS[role]
    predictor = dspy.ChainOfThought(StakeholderVoiceSignature)
    try:
        result = predictor(role_persona=persona, topic=topic)
    except Exception as exc:  # noqa: BLE001
        _err(f"voice generation failed: {exc!r}", 3)
    voice = (getattr(result, "voice", "") or "").strip()
    if not voice:
        _err("empty voice from LLM", 3)
    print(json.dumps({"role": role, "voice": voice}))
    return 0


def _architect(topic: str) -> int:
    # Topic for architect mode is the CTQ text; the script accepts it
    # via argv[2] for symmetry.
    predictor = dspy.ChainOfThought(ArchitectChoiceSignature)
    try:
        result = predictor(ctq_text=topic, topic=topic)
    except Exception as exc:  # noqa: BLE001
        _err(f"architect choice failed: {exc!r}", 3)
    iac_target = (getattr(result, "iac_target", "") or "").strip().lower()
    region = (getattr(result, "region", "") or "").strip()
    mcu_target = (getattr(result, "mcu_target", "") or "").strip().lower()
    children_raw = getattr(result, "supervisor_children", 4)
    try:
        children = int(children_raw)
    except (TypeError, ValueError):
        children = 4
    children = max(1, min(64, children))
    rationale = (getattr(result, "rationale", "") or "").strip()
    # The architect, like any stakeholder, can be wrong. The validator
    # downstream is what admits. We do force iac_target=aws and a
    # supported mcu_target since those are hard constraints in
    # validate_spec; the LLM otherwise sometimes proposes 'gcp' or
    # 'avr', which we coerce to the closest admissible value.
    if iac_target != "aws":
        iac_target = "aws"
    if mcu_target not in {"esp32", "stm32", "rp2040"}:
        mcu_target = "esp32"
    print(json.dumps({
        "role": "architect",
        "iac_target": iac_target,
        "region": region or "us-east-1",
        "mcu_target": mcu_target,
        "supervisor_children": children,
        "rationale": rationale,
    }))
    return 0


def _senior_reviewer(topic: str) -> int:
    # Topic in this mode MUST be a JSON object with three keys:
    # {"artifact_kind", "artifact_json", "original_voice"}.
    try:
        spec = json.loads(topic)
    except Exception as exc:  # noqa: BLE001
        _err(f"reviewer mode requires JSON topic: {exc!r}", 2)
    artifact_kind = str(spec.get("artifact_kind", ""))
    artifact_json = str(spec.get("artifact_json", ""))
    original_voice = str(spec.get("original_voice", ""))
    if not (artifact_kind and artifact_json and original_voice):
        _err("reviewer JSON must have artifact_kind, artifact_json, original_voice", 2)
    predictor = dspy.ChainOfThought(SeniorReviewerSignature)
    try:
        result = predictor(
            artifact_kind=artifact_kind,
            artifact_json=artifact_json,
            original_voice=original_voice,
        )
    except Exception as exc:  # noqa: BLE001
        _err(f"reviewer failed: {exc!r}", 3)
    admit_raw = getattr(result, "admit", False)
    admit = bool(admit_raw) if isinstance(admit_raw, bool) else str(admit_raw).strip().lower() in {"true", "yes", "1"}
    reason = (getattr(result, "reason", "") or "").strip()
    concerns_raw = (getattr(result, "concerns", "") or "").strip()
    concerns = [c.strip() for c in concerns_raw.split(",") if c.strip()]
    print(json.dumps({
        "role": "senior_reviewer",
        "admit": admit,
        "reason": reason,
        "concerns": concerns,
    }))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
