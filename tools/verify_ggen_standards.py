#!/usr/bin/env python3
"""Verify the ggen v26.7.31 constitutional contract.

This verifier is intentionally stdlib-only. It observes repository law and emits a
machine-readable report; it never promotes external or production standing.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

SCHEMA = "chatmangpt.ggen-standards-report/v1"
EXPECTED_STANDINGS = [
    "ALIVE",
    "PARTIAL_ALIVE",
    "BLOCKED",
    "BUILD_BROKEN",
    "UNKNOWN",
    "UNSUPPORTED",
]
EXPECTED_EVIDENCE = [
    "positive_witness",
    "negative_falsifier",
    "independent_verifier",
    "receipt_verification",
    "deterministic_replay",
]
EXPECTED_CHAIN = ["O", "O*", "I", "G", "A", "R", "O'"]
REQUIRED_AGENT_FRAGMENTS = [
    "A = μ(O*)",
    "candidate != verified != authorized != actuated",
    "planning != actuation",
    "command success != consequence success",
    "BRCE",
    "positive witness",
    "negative falsifier",
    "independent verifier",
    "receipt verification",
    "deterministic replay",
    "cargo metadata --locked",
    "make adversarial",
    "make cell8-certify",
]


@dataclass(frozen=True)
class Check:
    id: str
    passed: bool
    detail: str
    evidence: dict[str, Any]


class ContractRefusal(RuntimeError):
    def __init__(self, refusal_id: str, detail: str, evidence: dict[str, Any] | None = None):
        super().__init__(detail)
        self.refusal_id = refusal_id
        self.detail = detail
        self.evidence = evidence or {}


def require(condition: bool, refusal_id: str, detail: str, **evidence: Any) -> Check:
    if not condition:
        raise ContractRefusal(refusal_id, detail, evidence)
    return Check(refusal_id, True, detail, evidence)


def load_toml(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as stream:
            return tomllib.load(stream)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise ContractRefusal(
            "GGEN-STD-PARSE-001",
            f"cannot parse required TOML {path.as_posix()}: {exc}",
        ) from exc


def field(block: str, name: str) -> str | None:
    match = re.search(rf'^\s*{re.escape(name)}\s*=\s*"([^"]*)"', block, re.MULTILINE)
    return match.group(1) if match else None


def duplicate_lock_identities(lock_path: Path) -> list[dict[str, Any]]:
    try:
        text = lock_path.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        raise ContractRefusal("GGEN-STD-LOCK-001", f"cannot read Cargo.lock: {exc}") from exc

    duplicates: list[dict[str, Any]] = []
    seen: dict[tuple[str | None, str | None, str | None], int] = {}
    for number, block in enumerate(text.split("[[package]]")[1:], 1):
        key = (field(block, "name"), field(block, "version"), field(block, "source"))
        if not key[0]:
            continue
        if key in seen:
            duplicates.append(
                {
                    "name": key[0],
                    "version": key[1],
                    "source": key[2],
                    "first_block": seen[key],
                    "duplicate_block": number,
                }
            )
        else:
            seen[key] = number
    return duplicates


def absolute_path_references(root: Path) -> list[dict[str, Any]]:
    candidates = [root / "Cargo.toml"]
    findings: list[dict[str, Any]] = []
    pattern = re.compile(r'\bpath\s*=\s*"(?:/|[A-Za-z]:\\)')
    for path in candidates:
        text = path.read_text(encoding="utf-8", errors="replace")
        for lineno, line in enumerate(text.splitlines(), 1):
            if pattern.search(line):
                findings.append(
                    {"path": path.relative_to(root).as_posix(), "line": lineno, "text": line.strip()}
                )
    return findings


def dead_param_gate_findings(root: Path) -> list[dict[str, Any]]:
    wrapper = root / "tools" / "dead-param-gate.sh"
    findings: list[dict[str, Any]] = []
    if not wrapper.is_file():
        return [{"path": "tools/dead-param-gate.sh", "detail": "compatibility wrapper missing"}]

    text = wrapper.read_text(encoding="utf-8", errors="replace")
    authority = "cargo test --locked --test dead_param_gate_test"
    if authority not in text:
        findings.append(
            {
                "path": "tools/dead-param-gate.sh",
                "detail": "wrapper does not delegate to the admitted Rust AST gate",
            }
        )
    forbidden = ("rg -n", "grep -", "GATE_LET", "GENERIC_DISCARD")
    for token in forbidden:
        if token in text:
            findings.append(
                {
                    "path": "tools/dead-param-gate.sh",
                    "detail": f"wrapper contains superseded scanner logic: {token}",
                }
            )
    return findings

def verify_contract(root: Path) -> list[Check]:
    checks: list[Check] = []
    agents_path = root / "AGENTS.md"
    profile_path = root / "standards" / "ggen-v26.7.31.toml"
    cargo_path = root / "Cargo.toml"
    namespace_path = root / ".chatmangpt" / "namespace.toml"

    for required in (agents_path, profile_path, cargo_path, namespace_path):
        checks.append(
            require(
                required.is_file(),
                "GGEN-STD-SURFACE-001",
                "required constitutional surface exists",
                path=required.relative_to(root).as_posix(),
            )
        )

    agents = agents_path.read_text(encoding="utf-8")
    missing_fragments = [fragment for fragment in REQUIRED_AGENT_FRAGMENTS if fragment not in agents]
    checks.append(
        require(
            not missing_fragments,
            "GGEN-STD-CONSTITUTION-001",
            "AGENTS.md carries the required authority and evidence laws",
            missing=missing_fragments,
        )
    )

    profile = load_toml(profile_path)
    checks.append(
        require(
            profile.get("schema") == "chatmangpt.ggen-standards/v1"
            and profile.get("standard_id") == "ggen-v26.7.31",
            "GGEN-STD-IDENTITY-001",
            "standards profile identity is exact",
            observed_schema=profile.get("schema"),
            observed_standard_id=profile.get("standard_id"),
        )
    )
    checks.append(
        require(
            profile.get("required_broker") == "BRCE"
            and profile.get("authority", {}).get("external_actuation") == "brce_only"
            and profile.get("authority", {}).get("emergency_bypass") is False
            and profile.get("authority", {}).get("legacy_bypass") is False,
            "GGEN-STD-AUTHORITY-001",
            "BRCE is the sole external actuation boundary and no bypass is admitted",
            required_broker=profile.get("required_broker"),
            authority=profile.get("authority", {}),
        )
    )
    checks.append(
        require(
            profile.get("standing", {}).get("allowed") == EXPECTED_STANDINGS,
            "GGEN-STD-STANDING-001",
            "standing vocabulary is exact and ordered",
            expected=EXPECTED_STANDINGS,
            observed=profile.get("standing", {}).get("allowed"),
        )
    )
    checks.append(
        require(
            profile.get("release_standing") == "UNKNOWN"
            and profile.get("metadata_implies_standing") is False
            and profile.get("source_generation_may_promote") is False
            and profile.get("standing", {}).get("weighted_average_may_promote") is False,
            "GGEN-STD-PROMOTION-001",
            "metadata, source generation, and aggregate scores cannot self-promote standing",
            release_standing=profile.get("release_standing"),
        )
    )
    checks.append(
        require(
            profile.get("evidence", {}).get("required_surfaces") == EXPECTED_EVIDENCE,
            "GGEN-STD-EVIDENCE-001",
            "ALIVE requires all five evidence surfaces",
            expected=EXPECTED_EVIDENCE,
            observed=profile.get("evidence", {}).get("required_surfaces"),
        )
    )
    checks.append(
        require(
            profile.get("process", {}).get("chain") == EXPECTED_CHAIN
            and profile.get("process", {}).get("command_success_is_consequence_success") is False,
            "GGEN-STD-PROCESS-001",
            "operating process includes consequence re-observation",
            expected=EXPECTED_CHAIN,
            observed=profile.get("process", {}).get("chain"),
        )
    )
    checks.append(
        require(
            profile.get("generation", {}).get("hand_edit_generated_outputs") is False
            and profile.get("generation", {}).get("one_owner_per_output") is True
            and profile.get("generation", {}).get("second_manufacture_required") is True,
            "GGEN-STD-GENERATION-001",
            "generated consequences have one owner and require deterministic replay",
            generation=profile.get("generation", {}),
        )
    )
    checks.append(
        require(
            profile.get("process_evidence", {}).get("powl_is_precedence_list") is False
            and profile.get("process_evidence", {}).get("ocel_from_real_execution") is True
            and profile.get("process_evidence", {}).get("models_have_execution_authority") is False,
            "GGEN-STD-PROCESS-EVIDENCE-001",
            "POWL, OCEL, MuStar, and model authority boundaries are preserved",
            process_evidence=profile.get("process_evidence", {}),
        )
    )

    namespace = load_toml(namespace_path)
    checks.append(
        require(
            namespace.get("repository", {}).get("full_name") == "seanchatmangpt/open-ontologies"
            and namespace.get("repository", {}).get("standing") == "UNKNOWN"
            and namespace.get("public_object", {}).get("standing") == "UNKNOWN"
            and namespace.get("law", {}).get("external_actuation_broker") == "BRCE",
            "GGEN-STD-NAMESPACE-001",
            "namespace separates identity from standing and preserves BRCE authority",
            repository=namespace.get("repository", {}),
            public_object=namespace.get("public_object", {}),
        )
    )

    cargo = cargo_path.read_text(encoding="utf-8", errors="replace")
    absolute_paths = absolute_path_references(root)
    checks.append(
        require(
            not absolute_paths,
            "GGEN-STD-PORTABILITY-001",
            "Cargo dependency graph contains no absolute workstation paths",
            findings=absolute_paths,
        )
    )
    checks.append(
        require(
            'repository = "https://github.com/seanchatmangpt/open-ontologies"' in cargo,
            "GGEN-STD-REPOSITORY-001",
            "Cargo package metadata names the governing repository",
        )
    )
    return checks


def verify_repository(root: Path) -> list[Check]:
    checks = verify_contract(root)
    lock_path = root / "Cargo.lock"
    checks.append(
        require(
            lock_path.is_file(),
            "GGEN-STD-LOCK-001",
            "Cargo.lock is committed",
        )
    )
    duplicates = duplicate_lock_identities(lock_path)
    checks.append(
        require(
            not duplicates,
            "GGEN-STD-LOCK-002",
            "Cargo.lock has unique package identities",
            duplicates=duplicates,
        )
    )

    gate_findings = dead_param_gate_findings(root)
    checks.append(
        require(
            not gate_findings,
            "GGEN-STD-CI-001",
            "dead-parameter compatibility wrapper delegates only to the admitted Rust AST gate",
            findings=gate_findings,
        )
    )

    workflow = root / ".github" / "workflows" / "ggen-standards.yml"
    checks.append(
        require(
            workflow.is_file(),
            "GGEN-STD-CI-002",
            "permanent exact-head ggen standards workflow exists",
            path=workflow.relative_to(root).as_posix(),
        )
    )
    workflow_text = workflow.read_text(encoding="utf-8", errors="replace")
    checks.append(
        require(
            "permissions:\n  contents: read" in workflow_text
            and "python3 tools/verify_ggen_standards.py" in workflow_text
            and "cargo metadata --locked" in workflow_text,
            "GGEN-STD-CI-003",
            "standards workflow is read-only and verifies the locked exact tree",
        )
    )
    return checks


def build_report(root: Path, contract_only: bool) -> tuple[dict[str, Any], int]:
    checks: list[Check] = []
    refusal: ContractRefusal | None = None
    try:
        checks = verify_contract(root) if contract_only else verify_repository(root)
    except ContractRefusal as exc:
        refusal = exc

    report = {
        "schema": SCHEMA,
        "standard_id": "ggen-v26.7.31",
        "repository": "seanchatmangpt/open-ontologies",
        "mode": "contract_only" if contract_only else "repository",
        "checks": [asdict(check) for check in checks],
        "refusal": (
            {
                "id": refusal.refusal_id,
                "detail": refusal.detail,
                "evidence": refusal.evidence,
            }
            if refusal
            else None
        ),
        "bounded_standing": "PARTIAL_ALIVE" if refusal is None else "BUILD_BROKEN",
        "external_release_standing": "UNKNOWN",
        "actuation_performed": False,
    }
    return report, 0 if refusal is None else 2


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--contract-only", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = args.root.resolve()
    report, status = build_report(root, args.contract_only)
    payload = json.dumps(report, indent=2, sort_keys=True) + "\n"
    output = args.output or root / "target" / "standards" / "ggen-standards.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(payload, encoding="utf-8")
    sys.stdout.write(payload)
    return status


if __name__ == "__main__":
    raise SystemExit(main())
