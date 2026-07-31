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
WASM4PM_PRECISE = "f1d4d7ac8b2f9a0265be82991487766eb35b4675"
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
REQUIRED_WORKFLOW_IDS = {
    "ggen-standards",
    "verification-matrix",
    "regression-gates",
    "cascade",
    "docker",
}
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


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ContractRefusal(
            "GGEN-STD-PARSE-002",
            f"cannot parse required JSON {path.as_posix()}: {exc}",
        ) from exc
    if not isinstance(value, dict):
        raise ContractRefusal("GGEN-STD-PARSE-002", f"{path.as_posix()} must be a JSON object")
    return value


def field(block: str, name: str) -> str | None:
    match = re.search(rf'^\s*{re.escape(name)}\s*=\s*"([^"]*)"', block, re.MULTILINE)
    return match.group(1) if match else None


def package_blocks(lock_path: Path) -> list[dict[str, Any]]:
    try:
        text = lock_path.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        raise ContractRefusal("GGEN-STD-LOCK-001", f"cannot read Cargo.lock: {exc}") from exc
    return [
        {
            "block": number,
            "name": field(block, "name"),
            "version": field(block, "version"),
            "source": field(block, "source"),
            "text": block,
        }
        for number, block in enumerate(text.split("[[package]]")[1:], 1)
        if field(block, "name")
    ]


def duplicate_lock_identities(lock_path: Path) -> list[dict[str, Any]]:
    duplicates: list[dict[str, Any]] = []
    seen: dict[tuple[str | None, str | None, str | None], int] = {}
    for package in package_blocks(lock_path):
        key = (package["name"], package["version"], package["source"])
        if key in seen:
            duplicates.append(
                {
                    "name": key[0],
                    "version": key[1],
                    "source": key[2],
                    "first_block": seen[key],
                    "duplicate_block": package["block"],
                }
            )
        else:
            seen[key] = package["block"]
    return duplicates


def find_package(lock_path: Path, name: str) -> list[dict[str, Any]]:
    return [package for package in package_blocks(lock_path) if package["name"] == name]


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


def stale_dead_param_references(root: Path) -> list[dict[str, Any]]:
    findings: list[dict[str, Any]] = []
    paths = [root / ".github" / "workflows", root / ".github" / "scripts", root / "Makefile"]
    for entry in paths:
        files = [entry] if entry.is_file() else sorted(entry.rglob("*")) if entry.exists() else []
        for path in files:
            if not path.is_file() or path.suffix not in {".yml", ".yaml", ".sh", ""}:
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            for lineno, line in enumerate(text.splitlines(), 1):
                if "tools/dead-param-gate.sh" in line:
                    findings.append(
                        {
                            "path": path.relative_to(root).as_posix(),
                            "line": lineno,
                            "text": line.strip(),
                        }
                    )
    return findings


def manifest_version(cargo: str) -> str | None:
    package = cargo.split("[dependencies]", 1)[0]
    return field(package, "version")


def verify_contract(root: Path) -> list[Check]:
    checks: list[Check] = []
    agents_path = root / "AGENTS.md"
    profile_path = root / "standards" / "ggen-v26.7.31.toml"
    cargo_path = root / "Cargo.toml"
    namespace_path = root / ".chatmangpt" / "namespace.toml"
    release_path = root / "RELEASE_STANDING.json"

    for required in (agents_path, profile_path, cargo_path, namespace_path, release_path):
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

    release = load_json(release_path)
    checks.append(
        require(
            release.get("standing") == "UNKNOWN"
            and release.get("external_actuation") is False
            and release.get("external_consequence_replay") is False,
            "GGEN-STD-RELEASE-001",
            "unobserved external release surfaces remain UNKNOWN",
            release=release,
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
    checks.append(
        require(
            "mcpp-core" not in cargo and re.search(r"^mcpp\s*=\s*\[\s*\]\s*$", cargo, re.MULTILINE),
            "GGEN-STD-UNSUPPORTED-001",
            "unpublished mcpp-core is quarantined as an unsupported empty feature",
        )
    )
    return checks


def verify_repository(root: Path) -> list[Check]:
    checks = verify_contract(root)
    lock_path = root / "Cargo.lock"
    checks.append(require(lock_path.is_file(), "GGEN-STD-LOCK-001", "Cargo.lock is committed"))

    duplicates = duplicate_lock_identities(lock_path)
    checks.append(
        require(
            not duplicates,
            "GGEN-STD-LOCK-002",
            "Cargo.lock has unique package identities",
            duplicates=duplicates,
        )
    )

    cargo = (root / "Cargo.toml").read_text(encoding="utf-8", errors="replace")
    root_packages = find_package(lock_path, "open-ontologies")
    checks.append(
        require(
            len(root_packages) == 1 and root_packages[0]["version"] == manifest_version(cargo),
            "GGEN-STD-LOCK-003",
            "manifest and root lock package versions agree",
            manifest_version=manifest_version(cargo),
            lock_versions=[package["version"] for package in root_packages],
        )
    )

    wasm4pm = find_package(lock_path, "wasm4pm")
    cognition = find_package(lock_path, "wasm4pm-cognition")
    expected_source = f"git+https://github.com/seanchatmangpt/wasm4pm#{WASM4PM_PRECISE}"
    checks.append(
        require(
            len(wasm4pm) == 1
            and len(cognition) == 1
            and wasm4pm[0]["source"] == expected_source
            and cognition[0]["source"] == expected_source,
            "GGEN-STD-LOCK-004",
            "wasm4pm process and cognition authorities resolve to one exact commit",
            expected_source=expected_source,
            wasm4pm_sources=[package["source"] for package in wasm4pm],
            cognition_sources=[package["source"] for package in cognition],
        )
    )
    compat = find_package(lock_path, "wasm4pm-compat")
    checks.append(
        require(
            len(compat) == 1 and compat[0]["version"] == "26.6.29",
            "GGEN-STD-LOCK-005",
            "portable wasm4pm type boundary is exactly wasm4pm-compat 26.6.29",
            observed=[package["version"] for package in compat],
        )
    )

    stale_refs = stale_dead_param_references(root)
    checks.append(
        require(
            not stale_refs,
            "GGEN-STD-CI-001",
            "CI invokes the admitted Rust dead-parameter gate rather than a deleted shell path",
            findings=stale_refs,
        )
    )

    inventory_path = root / "standards" / "workflows.toml"
    inventory = load_toml(inventory_path)
    workflows = inventory.get("workflow", [])
    observed_ids = {entry.get("id") for entry in workflows if isinstance(entry, dict)}
    missing_ids = sorted(REQUIRED_WORKFLOW_IDS - observed_ids)
    missing_paths = sorted(
        entry.get("path")
        for entry in workflows
        if isinstance(entry, dict)
        and isinstance(entry.get("path"), str)
        and not (root / entry["path"]).is_file()
    )
    checks.append(
        require(
            not missing_ids and not missing_paths,
            "GGEN-STD-CI-002",
            "critical workflows have named semantic and output owners",
            missing_ids=missing_ids,
            missing_paths=missing_paths,
        )
    )

    workflow = root / ".github" / "workflows" / "ggen-standards.yml"
    checks.append(
        require(
            workflow.is_file(),
            "GGEN-STD-CI-003",
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
            "GGEN-STD-CI-004",
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
            {"id": refusal.refusal_id, "detail": refusal.detail, "evidence": refusal.evidence}
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
