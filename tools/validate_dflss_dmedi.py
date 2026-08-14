#!/usr/bin/env python3
"""Static verifier for the DFLSS DMEDI arXiv coverage ontology package."""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ONTOLOGY = ROOT / "ontology" / "dflss-dmedi.ttl"
SHAPES = ROOT / "ontology" / "dflss-dmedi-shapes.ttl"
QUERY = ROOT / "sparql" / "dflss-dmedi-topic-coverage.rq"
DOCS = ROOT / "docs" / "dflss-dmedi-arxiv-coverage.md"

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


def fail(message: str) -> None:
    print(f"DFLSS DMEDI validation failed: {message}", file=sys.stderr)
    sys.exit(1)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def main() -> None:
    for path in (ONTOLOGY, SHAPES, QUERY, DOCS):
        require(path.exists(), f"missing {path.relative_to(ROOT)}")
    ttl = ONTOLOGY.read_text(encoding="utf-8")
    shapes = SHAPES.read_text(encoding="utf-8")
    query = QUERY.read_text(encoding="utf-8")
    docs = DOCS.read_text(encoding="utf-8")
    for phase in EXPECTED_PHASES:
        require(f"dflss:{phase}" in ttl, f"missing phase {phase}")
    for topic in EXPECTED_TOPICS:
        require(f"dflss:{topic}" in ttl, f"missing topic {topic}")
        require(re.search(rf"dflss:{re.escape(topic)}\s+a\s+dflss:(Topic|Tool)\s*;", ttl), f"{topic} is not declared as a Topic or Tool")
    for arxiv_id in EXPECTED_ARXIV_IDS:
        require(arxiv_id in ttl, f"missing arXiv seed {arxiv_id}")
        require(f"https://arxiv.org/abs/{arxiv_id}" in ttl, f"missing arXiv URL for {arxiv_id}")
    for sparse_topic in EXPECTED_SPARSE_TOPICS:
        require(re.search(rf"dflss:{re.escape(sparse_topic)}[\s\S]*?dflss:sparseCoverageReason", ttl), f"{sparse_topic} lacks sparse coverage reason")
    require("dflss:NoDirectArxivCoverage" in ttl, "missing no-direct coverage class")
    require("dflss:claim-" in ttl, "missing coverage claims")
    require("dflss:ArxivPaperShape" in shapes, "missing arXiv SHACL shape")
    require("dflss:CoverageClaimShape" in shapes, "missing coverage claim SHACL shape")
    require("ORDER BY ?phaseOrder ?topicOrder" in query, "coverage query is not phase ordered")
    require("Sparse" in docs and "NoDirectArxivCoverage" in docs, "docs do not explain sparse coverage semantics")
    print("DFLSS DMEDI ontology validation passed")
    print(f"phases={len(EXPECTED_PHASES)}")
    print(f"topics={len(EXPECTED_TOPICS)}")
    print(f"arxiv_seeds={len(EXPECTED_ARXIV_IDS)}")
    print(f"sparse_topics={len(EXPECTED_SPARSE_TOPICS)}")


if __name__ == "__main__":
    main()
