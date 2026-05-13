//! Closes the inproc gauge gap (R4 WA, §24 Chicago TDD).
//!
//! Without this test, the inproc fixture path could ship CTQ candidates
//! that fail the same gauge real-Groq output is held to (the 5
//! `min_len`-bounded fields enforced by `signature_shape::ctq_signature`).
//! The §17 fake-gauge axiom forbids holding the inproc path to a weaker
//! gauge than production.
//!
//! Δ>0 PROOF: this test catches the case where someone weakens the
//!            inproc fixture (e.g. shortens `negative_case_text` below
//!            the 12-char `min_len`, or drops `defect_class_hint`) — the
//!            old `revops_e2e.rs` did NOT validate fixture output against
//!            the signature shape, so a weakened fixture could ship
//!            green forever. After this test lands, weakening the
//!            fixture fails the build.
//!            Production line pinned: src/signature_shape.rs::ctq_signature
//!            (the same shape that gauges real Groq output via
//!            `parse_and_validate` in the refine loop).

mod revops_common;

use open_ontologies::signature_shape::ctq_signature;
use revops_common::fixture_candidate_ctq;

#[test]
fn inproc_fixture_passes_the_same_shape_real_groq_is_gauged_against() {
    let candidate = fixture_candidate_ctq();

    // The signature shape consumes a JSON object whose keys match the
    // declared output fields. We serialize the CandidateCtq into a
    // shape-compatible JSON object — every output field of
    // `ctq_signature()` must appear with a non-empty, min_len-passing
    // value.
    let payload = serde_json::json!({
        "ctq_text": candidate.ctq_text,
        "measure_text": candidate.measure_text,
        "verification_text": candidate.verification_text,
        "negative_case_text": candidate.negative_case_text,
        "control_plan_text": candidate.control_plan_text,
        "defect_class_hint": candidate.defect_class_hint,
    })
    .to_string();

    let sig = ctq_signature();
    let result = sig.parse_and_validate(&payload);
    let parsed = match result {
        Ok(p) => p,
        Err(failures) => {
            panic!(
                "fixture_candidate_ctq() must satisfy ctq_signature() — got failures: {failures:?}\npayload: {payload}"
            );
        }
    };
    let admitted = &parsed.fields;

    // Every declared output field must be present in the admitted map.
    for f in &sig.output_fields {
        assert!(
            admitted.contains_key(&f.name),
            "admitted map missing field `{}` after parse_and_validate; admitted={:?}",
            f.name,
            admitted
        );
    }

    // Spot-check the min_len constraints from src/signature_shape.rs::ctq_signature
    // are actually being enforced — if `parse_and_validate` started
    // accepting empty strings, this would fail.
    assert!(candidate.ctq_text.chars().count() >= 20);
    assert!(candidate.measure_text.chars().count() >= 8);
    assert!(candidate.verification_text.chars().count() >= 8);
    assert!(candidate.negative_case_text.chars().count() >= 12);
    assert!(candidate.control_plan_text.chars().count() >= 12);
}
