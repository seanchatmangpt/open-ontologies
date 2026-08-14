#!/usr/bin/env python3
"""Executable Definition-of-Done verifier for the DFLSS/DMEDI arXiv package."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ONTOLOGY = ROOT / "ontology" / "dflss-dmedi.ttl"
SHAPES = ROOT / "ontology" / "dflss-dmedi-shapes.ttl"
QUERY = ROOT / "sparql" / "dflss-dmedi-topic-coverage.rq"
DOCS = ROOT / "docs" / "dflss-dmedi-arxiv-coverage.md"
DOD = ROOT / "docs" / "dflss-dmedi-definition-of-done.md"

EXPECTED_PHASES = ["Define", "Measure", "Explore", "Develop", "Implement", "Capstone"]
EXPECTED_TOPICS = [
    "Charter", "MGPP", "RiskManagement", "CommunicationPlan", "VoiceOfCustomer",
    "QualityFunctionDeployment", "TargetCosting", "Scorecards", "StatisticalSoftware",
    "BasicStatistics", "VariationAndControlCharts", "MeasurementSystemsAnalysis",
    "ProcessCapability", "ConceptGeneration", "TRIZProductDesign", "TransactionalTRIZ",
    "ConceptSelectionPughAHP", "StatisticalToleranceDesign", "MonteCarloSimulation",
    "HypothesisTesting", "ConfidenceIntervals", "TestsOfMeansMediansVariances",
    "ProportionAndChiSquare", "Regression", "MultiVariAnalysis", "DesignFMEA",
    "DetailedDesign", "TwoWayANOVA", "IntroDOE", "FullFactorialDOE",
    "FractionalFactorialDOE", "CatapultDOESimulation", "LeanConcepts", "LeanDesign",
    "DFMA", "Reliability", "DOEWithCurvature", "ConjointAnalysis", "MixtureDesigns",
    "RobustDesign", "HelicopterRSMSimulation", "PrototypeAndPilot", "ProcessControl",
    "ImplementationPlanning", "DMEDICapstoneProject",
]
EXPECTED_ARXIV_IDS = [
    "2403.13002", "1901.04443", "1503.06885", "2604.15544", "2603.14479",
    "2112.10338", "2406.18114", "1803.06536", "1712.09074", "2510.24349",
]
EXPECTED_SPARSE_TOPICS = [
    "MGPP", "TargetCosting", "StatisticalSoftware", "CatapultDOESimulation",
    "HelicopterRSMSimulation",
]
ALLOWED_RELEVANCE = ["Direct", "StronglyRelated", "Adjacent", "Sparse", "NoDirectArxivCoverage"]


class DodFailure(RuntimeError):
    pass


def require(condition: bool, gate: str, message: str) -> None:
    if not condition:
        raise DodFailure(f"{gate}: {message}")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def count_subject_declarations(ttl: str, local_name: str) -> int:
    return len(re.findall(rf"(?m)^dflss:{re.escape(local_name)}\s+a\s+", ttl))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--receipt",
        default=str(ROOT / "target" / "verifier" / "dflss-dmedi-dod.json"),
        help="machine-readable receipt path",
    )
    args = parser.parse_args()

    gates: list[dict[str, str]] = []

    def passed(gate: str, evidence: str) -> None:
        gates.append({"gate": gate, "status": "PASSED", "evidence": evidence})

    try:
        artifacts = (ONTOLOGY, SHAPES, QUERY, DOCS, DOD)
        for path in artifacts:
            require(path.exists(), "DOD-01", f"missing {path.relative_to(ROOT)}")
        passed("DOD-01", "all canonical package artifacts exist")

        ttl = ONTOLOGY.read_text(encoding="utf-8")
        shapes = SHAPES.read_text(encoding="utf-8")
        query = QUERY.read_text(encoding="utf-8")
        docs = DOCS.read_text(encoding="utf-8")
        dod = DOD.read_text(encoding="utf-8")

        for phase in EXPECTED_PHASES:
            require(count_subject_declarations(ttl, phase) == 1, "DOD-02", f"phase {phase} must be declared exactly once")
            require(re.search(rf"dflss:{phase}[\s\S]*?dflss:phaseOrder\s+\d+", ttl) is not None, "DOD-02", f"phase {phase} lacks phaseOrder")
        passed("DOD-02", f"exact DMEDI phase spine present ({len(EXPECTED_PHASES)} phases)")

        for topic in EXPECTED_TOPICS:
            require(count_subject_declarations(ttl, topic) == 1, "DOD-03", f"topic {topic} must be declared exactly once")
            require(re.search(rf"dflss:{re.escape(topic)}\s+a\s+dflss:(Topic|Tool)\s*;", ttl) is not None, "DOD-03", f"{topic} is not a Topic/Tool")
            require(re.search(rf"dflss:{re.escape(topic)}[\s\S]*?dflss:belongsToPhase\s+dflss:[A-Za-z]+", ttl) is not None, "DOD-04", f"{topic} lacks phase ownership")
            require(re.search(rf"dflss:{re.escape(topic)}[\s\S]*?dflss:topicOrder\s+\d+", ttl) is not None, "DOD-04", f"{topic} lacks topicOrder")
            require(re.search(rf"dflss:{re.escape(topic)}[\s\S]*?dflss:mapsToSubject\s+dflss:[A-Za-z0-9]+", ttl) is not None, "DOD-04", f"{topic} lacks research-subject mapping")
        passed("DOD-03", f"exact curriculum inventory present ({len(EXPECTED_TOPICS)} topics)")
        passed("DOD-04", "every topic has phase ownership, deterministic order, and research-subject mapping")

        for arxiv_id in EXPECTED_ARXIV_IDS:
            require(ttl.count(arxiv_id) >= 2, "DOD-05", f"arXiv seed {arxiv_id} identity/URL incomplete")
            require(f"https://arxiv.org/abs/{arxiv_id}" in ttl, "DOD-05", f"missing canonical arXiv URL for {arxiv_id}")
        require(len(re.findall(r"dflss:arxivId\s+\"", ttl)) == len(EXPECTED_ARXIV_IDS), "DOD-05", "unexpected arXiv seed cardinality")
        passed("DOD-05", f"arXiv bibliographic identity closed for {len(EXPECTED_ARXIV_IDS)} admitted seeds")

        for sparse_topic in EXPECTED_SPARSE_TOPICS:
            require(re.search(rf"dflss:{re.escape(sparse_topic)}[\s\S]*?dflss:sparseCoverageReason", ttl) is not None, "DOD-06", f"{sparse_topic} lacks sparse reason")
        require("dflss:NoDirectArxivCoverage" in ttl and "dflss:Sparse" in ttl, "DOD-06", "sparse/no-direct semantics missing")
        passed("DOD-06", f"explicit sparse/no-direct semantics present for {len(EXPECTED_SPARSE_TOPICS)} bounded gaps")

        require("dflss:claim-" in ttl, "DOD-07", "no coverage claims declared")
        for relevance in ALLOWED_RELEVANCE:
            require(f"dflss:{relevance}" in ttl, "DOD-07", f"missing relevance state {relevance}")
        require("dflss:claimTopic" in ttl and "dflss:claimRelevance" in ttl and "dflss:claimPaper" in ttl, "DOD-07", "coverage-claim contract incomplete")
        passed("DOD-07", "coverage claims are typed through bounded relevance states")

        required_shapes = ["PhaseShape", "TopicShape", "ArxivPaperShape", "CoverageClaimShape"]
        for shape in required_shapes:
            require(f"dflss:{shape}" in shapes, "DOD-08", f"missing SHACL {shape}")
        for required_path in ("dflss:belongsToPhase", "dflss:mapsToSubject", "dflss:arxivId", "dflss:claimRelevance"):
            require(f"sh:path {required_path}" in shapes, "DOD-08", f"SHACL contract missing {required_path}")
        passed("DOD-08", "SHACL contract covers phases, topics, arXiv papers, and coverage claims")

        for field in ("?phaseOrder", "?topicOrder", "?relevanceLabel", "?arxivId", "?paperUrl", "?sparseReason"):
            require(field in query, "DOD-09", f"coverage query omits {field}")
        require("ORDER BY ?phaseOrder ?topicOrder" in query, "DOD-09", "coverage query is not deterministically phase/topic ordered")
        passed("DOD-09", "SPARQL projection exposes deterministic coverage and sparse-gap evidence")

        for token in ("Definition of Done", "DOD-01", "DOD-10", "ALIVE"):
            require(token in dod, "DOD-10", f"DoD specification omits {token}")
        require("dflss-dmedi-definition-of-done.md" in docs and "dflss-dod.sh" in docs, "DOD-10", "operating docs do not point to executable DoD")
        passed("DOD-10", "human-readable DoD and operating docs bind to the executable verifier")

        receipt_path = Path(args.receipt)
        if not receipt_path.is_absolute():
            receipt_path = ROOT / receipt_path
        receipt_path.parent.mkdir(parents=True, exist_ok=True)
        receipt = {
            "schema": "dflss-dmedi-dod/v1",
            "status": "ALIVE",
            "generated_at": datetime.now(timezone.utc).isoformat(),
            "subject": {
                "phases": len(EXPECTED_PHASES),
                "topics": len(EXPECTED_TOPICS),
                "arxiv_seeds": len(EXPECTED_ARXIV_IDS),
                "sparse_topics": len(EXPECTED_SPARSE_TOPICS),
            },
            "artifacts": {
                str(path.relative_to(ROOT)): sha256(path)
                for path in artifacts
            },
            "gates": gates,
        }
        receipt_path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print("DFLSS DMEDI Definition of Done: ALIVE")
        for gate in gates:
            print(f"{gate['gate']}={gate['status']}")
        print(f"receipt={receipt_path}")
        return 0
    except DodFailure as exc:
        print(f"DFLSS DMEDI Definition of Done: BLOCKED: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
