//! Cell8 13-gate EARL conformance attestation emitter.
//!
//! Hand-written Turtle (no oxigraph serializer round-trip) for byte-stable,
//! diff-friendly reports that downstream auditors can re-parse and SHACL-
//! validate against `ontology/cell8-conformance-shapes.ttl`.
//!
//! The report emits, for a single receipt:
//!   - one `earl:TestSubject` declaration (the receipt URN)
//!   - thirteen `earl:Assertion` blank nodes, each pointing at the receipt
//!     via `earl:subject` and at one of the thirteen Cell8 gate IRIs via
//!     `earl:test`, with an `earl:result` of pass or fail.
//!   - thirteen `cell8:hasGate` triples on the receipt (one per assertion)
//!     so the SHACL `sh:minCount 13 / sh:maxCount 13` shape can validate
//!     coverage on a forward path (the inline validator in `crate::shacl`
//!     does not implement `sh:inversePath`).
//!
//! ## Phase 10 stub status
//!
//! The A10 ExternalAttestation gate is currently a digest-equality stand-
//! in (see `crate::cell_ready`). The EARL report emitted here records the
//! gate result honestly; once Ed25519 verification lands the result will
//! reflect the cryptographic outcome instead.

use crate::receipts::Receipt;

/// Outcome of a single Cell8 gate as observed by `cell_ready`.
#[derive(Debug, Clone)]
pub struct GateOutcome {
    pub passed: bool,
    pub message: String,
}

/// Count assertions that recorded `earl:passed`.
pub fn count_passed(results: &[(&str, GateOutcome)]) -> u8 {
    results.iter().filter(|(_, g)| g.passed).count() as u8
}

/// Count assertions that recorded `earl:failed`.
pub fn count_failed(results: &[(&str, GateOutcome)]) -> u8 {
    results.iter().filter(|(_, g)| !g.passed).count() as u8
}

/// The thirteen Cell8 gate names in canonical declaration order. Used by
/// callers (admission gate, MCP `onto_cell8_attest`) that need to enumerate
/// gates without re-implementing the order.
pub const GATE_NAMES: [&str; 13] = [
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
];

/// Render an EARL Turtle report for the given receipt and gate outcomes.
///
/// The `gate_results` slice MUST contain exactly the thirteen entries from
/// [`GATE_NAMES`] in declaration order. Callers that have fewer outcomes
/// (e.g. when admission denied early) should pad with `passed: false`
/// entries to keep coverage exact — auditors rely on the SHACL shape
/// rejecting any report with ≠ 13 assertions.
pub fn emit_earl_report(receipt: &Receipt, gate_results: &[(&str, GateOutcome)]) -> String {
    let receipt_iri = format!("urn:ontostar:receipt:{}", receipt.hex());

    let mut s = String::with_capacity(2048);
    s.push_str("@prefix earl:    <http://www.w3.org/ns/earl#> .\n");
    s.push_str("@prefix dcterms: <http://purl.org/dc/terms/> .\n");
    s.push_str("@prefix cell8:   <urn:ontostar:cell8:> .\n");
    s.push_str("@prefix xsd:     <http://www.w3.org/2001/XMLSchema#> .\n");
    s.push('\n');

    // Receipt subject declaration + per-gate forward `cell8:hasGate` edges.
    s.push_str(&format!("<{receipt_iri}> a earl:TestSubject ;\n"));
    s.push_str(&format!(
        "    dcterms:identifier \"{}\" ;\n",
        receipt.hex()
    ));
    // Emit one cell8:hasGate per assertion so the SHACL shape can count
    // coverage on a forward path.
    let last = gate_results.len().saturating_sub(1);
    for (idx, (gate, _)) in gate_results.iter().enumerate() {
        let sep = if idx == last { " ." } else { " ;" };
        s.push_str(&format!(
            "    cell8:hasGate <urn:ontostar:gate:{gate}>{sep}\n"
        ));
    }
    s.push('\n');

    // Per-gate assertion blank nodes.
    for (gate, outcome) in gate_results.iter() {
        let outcome_iri = if outcome.passed {
            "earl:passed"
        } else {
            "earl:failed"
        };
        s.push_str("[] a earl:Assertion ;\n");
        s.push_str(&format!("    earl:subject <{receipt_iri}> ;\n"));
        s.push_str(&format!(
            "    earl:test <urn:ontostar:gate:{gate}> ;\n"
        ));
        s.push_str("    earl:result [\n");
        s.push_str("        a earl:TestResult ;\n");
        s.push_str(&format!("        earl:outcome {outcome_iri} ;\n"));
        s.push_str(&format!(
            "        earl:info \"{}\"\n",
            escape_turtle_string(&outcome.message)
        ));
        s.push_str("    ] ;\n");
        s.push_str(&format!(
            "    dcterms:description \"{}\" .\n\n",
            escape_turtle_string(gate)
        ));
    }
    s
}

/// Minimal Turtle string escape: backslash, double-quote, newline.
fn escape_turtle_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::production_record::ProductionRecord;

    fn fake_receipt() -> Receipt {
        let rec = ProductionRecord {
            artifact_hash: [1u8; 32],
            scope_token: "scope-x".into(),
            declared_powl_hash: [2u8; 32],
            ocel_canonical_hash: [3u8; 32],
            conformance_run_id: "run-x".into(),
            gate_config_hash: [4u8; 32],
            production_law_version: "ontostar-1.0.0".into(),
            defects_taxonomy_version: crate::defects::DEFECTS_TAXONOMY_VERSION.into(),
            gates_passed: GATE_NAMES.iter().map(|s| s.to_string()).collect(),
            gates_refused: Vec::new(),
            prior_receipt: None,
            signature: None,
            signing_key_fpr: None,
        };
        crate::receipts::build(rec)
    }

    fn all_pass() -> Vec<(&'static str, GateOutcome)> {
        GATE_NAMES
            .iter()
            .map(|g| {
                (
                    *g,
                    GateOutcome {
                        passed: true,
                        message: format!("{g} passed"),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn count_passed_and_failed_sum_to_total() {
        let mut r = all_pass();
        r[5].1.passed = false;
        assert_eq!(count_passed(&r), 12);
        assert_eq!(count_failed(&r), 1);
    }

    #[test]
    fn report_contains_thirteen_assertions() {
        let receipt = fake_receipt();
        let report = emit_earl_report(&receipt, &all_pass());
        let n = report.matches("a earl:Assertion").count();
        assert_eq!(n, 13, "expected 13 assertion blank nodes in report");
        assert!(report.contains("earl:passed"));
        assert!(!report.contains("earl:failed"));
    }

    #[test]
    fn report_records_failed_outcome_when_gate_fails() {
        let receipt = fake_receipt();
        let mut r = all_pass();
        r[8].1 = GateOutcome {
            passed: false,
            message: "ProvenanceMissing".into(),
        };
        let report = emit_earl_report(&receipt, &r);
        assert!(report.contains("earl:failed"));
        assert!(report.contains("ProvenanceMissing"));
    }
}
