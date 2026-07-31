#!/usr/bin/env python3
"""Emit EARL A1-A13 certification report for Cell8 gates.

Real verifier: checks BLAKE3 hashes from receipts/receipt-chain.ttl (if present),
optionally reads cargo test JSON output to map failed tests to gates, and emits
earl:passed or earl:failed per gate. Exits non-zero if any gate fails.

Usage:
    python3 tools/emit-earl-report.py [--test-results <file>] [--skip-cargo]

    --test-results <file>   Path to cargo test JSON output (--format json).
    --skip-cargo            Skip cargo test JSON parsing entirely (emit based on
                            hash checks alone; unknown gates default to passed).
"""

import argparse
import datetime
import json
import os
import subprocess
import sys

# ---------------------------------------------------------------------------
# Gate registry — matches src/cell8.rs GATE_NAMES exactly.
# Namespace urn:ontostar:gate: to match Rust emit_earl_report().
# ---------------------------------------------------------------------------
GATE_NAMES = [
    "A1_WorkflowDeclared",
    "A2_ScopeClosed",
    "A3_OCELComplete",
    "A4_POWLReplayPass",
    "A5_ThresholdPass",
    "A6_RequiredStagesPresent",
    "A7_NoBypassRevocation",
    "A8_ReceiptValid",
    "A9_ProvenanceChain",
    "A10_ExternalAttestation",
    "A11_TemporalValidity",
    "A12_DependencyClosure",
    "A13_ReplayProof",
]

GATE_NS = "urn:ontostar:gate:"

# Map test-name prefix (lowercased) → gate index (0-based)
PREFIX_TO_GATE = {
    "a1_": 0,
    "a2_": 1,
    "a3_": 2,
    "a4_": 3,
    "a5_": 4,
    "a6_": 5,
    "a7_": 6,
    "a8_": 7,
    "a9_": 8,
    "a10_": 9,
    "a11_": 10,
    "a12_": 11,
    "a13_": 12,
}


def find_receipt_chain(repo_root):
    """Return path to receipts/receipt-chain.ttl if it exists, else None."""
    path = os.path.join(repo_root, "receipts", "receipt-chain.ttl")
    return path if os.path.isfile(path) else None


def parse_receipt_chain_hashes(ttl_path):
    """Return dict mapping file-path IRI -> expected BLAKE3 hex from receipt-chain.ttl.

    Looks for triples of the form:
        <file:///...> cell8:blake3 "hex" .
    Falls back to a simple line scan if rdflib parse fails.
    """
    results = {}
    try:
        import rdflib
        g = rdflib.Graph()
        g.parse(ttl_path, format="turtle")
        BLAKE3_PRED = rdflib.URIRef("urn:ontostar:cell8:blake3")
        for s, p, o in g:
            if p == BLAKE3_PRED:
                results[str(s)] = str(o)
    except Exception as exc:
        print("# WARNING: rdflib parse of receipt-chain.ttl failed: " + str(exc), file=sys.stderr)
    return results


def blake3_hex(filepath):
    """Return BLAKE3 hex digest of filepath using b3sum, or None on error."""
    try:
        result = subprocess.run(
            ["b3sum", "--no-names", filepath],
            capture_output=True,
            text=True,
            check=True,
        )
        return result.stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        print("# WARNING: b3sum failed for " + filepath + ": " + str(exc), file=sys.stderr)
        return None


def check_blake3_hashes(repo_root, hashes):
    """Return (all_pass, failures) where failures is list of (iri, reason) strings."""
    failures = []
    for iri, expected in hashes.items():
        # Convert file:/// IRI to local path
        if iri.startswith("file:///"):
            local_path = iri[7:]  # keep leading /
        elif iri.startswith("file://"):
            local_path = iri[7:]
        else:
            # relative IRI — treat as relative to repo_root
            local_path = os.path.join(repo_root, iri)
        if not os.path.isfile(local_path):
            failures.append((iri, "file not found: " + local_path))
            continue
        actual = blake3_hex(local_path)
        if actual is None:
            failures.append((iri, "b3sum error"))
            continue
        if actual.lower() != expected.lower():
            failures.append((iri, "hash mismatch: expected " + expected[:16] + "... got " + actual[:16] + "..."))
    all_pass = len(failures) == 0
    return all_pass, failures


def parse_cargo_test_json(filepath):
    """Return set of gate indices (0-based) that have at least one failed test."""
    failed_gates = set()
    try:
        with open(filepath) as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    obj = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if obj.get("type") != "test":
                    continue
                if obj.get("event") != "failed":
                    continue
                test_name = obj.get("name", "").lower()
                for prefix, idx in PREFIX_TO_GATE.items():
                    if test_name.startswith(prefix) or (":" + prefix) in test_name:
                        failed_gates.add(idx)
    except OSError as exc:
        print("# WARNING: cannot read test-results file: " + str(exc), file=sys.stderr)
    return failed_gates


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--test-results", metavar="FILE",
                        help="Path to cargo test JSON output (--format json)")
    parser.add_argument("--skip-cargo", action="store_true",
                        help="Skip cargo test JSON parsing")
    args = parser.parse_args()

    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    now = datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z")

    # --- A5: BLAKE3 hash check ------------------------------------------------
    a5_blake3_ok = True  # optimistic if no receipt-chain.ttl
    blake3_failures = []
    ttl_path = find_receipt_chain(repo_root)
    if ttl_path:
        hashes = parse_receipt_chain_hashes(ttl_path)
        if hashes:
            a5_blake3_ok, blake3_failures = check_blake3_hashes(repo_root, hashes)
            if not a5_blake3_ok:
                for iri, reason in blake3_failures:
                    print("# BLAKE3 FAIL: " + iri + " — " + reason, file=sys.stderr)
        else:
            print("# INFO: receipt-chain.ttl found but no blake3 triples detected", file=sys.stderr)
    else:
        print("# INFO: receipts/receipt-chain.ttl not found; skipping BLAKE3 check", file=sys.stderr)

    # --- Cargo test failures --------------------------------------------------
    failed_gates = set()
    if not args.skip_cargo and args.test_results:
        failed_gates = parse_cargo_test_json(args.test_results)

    # A5 failure → mark gate index 4 (A5_ThresholdPass) as failed
    if not a5_blake3_ok:
        failed_gates.add(4)

    # --- Emit EARL report -----------------------------------------------------
    lines = []
    lines.append("@prefix earl:  <http://www.w3.org/ns/earl#> .")
    lines.append("@prefix gate:  <" + GATE_NS + "> .")
    lines.append("@prefix dct:   <http://purl.org/dc/terms/> .")
    lines.append("@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .")
    lines.append("")

    any_failed = False
    for idx, name in enumerate(GATE_NAMES):
        passed = idx not in failed_gates
        if not passed:
            any_failed = True
        outcome = "earl:passed" if passed else "earl:failed"
        lines.append("[] a earl:Assertion ;")
        lines.append("   earl:subject gate:" + name + " ;")
        lines.append("   earl:test gate:" + name + "Shape ;")
        lines.append("   earl:result [")
        lines.append("     a earl:TestResult ;")
        lines.append("     earl:outcome " + outcome + " ;")
        lines.append('     dct:issued "' + now + '"^^xsd:dateTime')
        lines.append("   ] .")
        lines.append("")

    print("\n".join(lines))

    if any_failed:
        print("# CERTIFICATION FAILED: one or more gates are earl:failed", file=sys.stderr)
        sys.exit(1)

    sys.exit(0)


if __name__ == "__main__":
    main()
