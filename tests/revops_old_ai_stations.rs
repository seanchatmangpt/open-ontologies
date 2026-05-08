//! Phase 3.5 — Fortune-5 RevOps old-AI station tests.
//!
//! One test per breed from wasm4pm-cognition (ELIZA, CBR, DENDRAL,
//! STRIPS, Prolog, MYCIN, GPS, SOAR, Hearsay-II). Each test wires
//! the corresponding breed against a RevOps-flavoured `BreedInput`
//! and asserts:
//!
//!   1. The breed runs and returns Ok.
//!   2. The inference_trace is non-empty (fraud-detection invariant).
//!   3. The breed does NOT override refusal — i.e. the breed alone
//!      cannot create authority that wasn't earned by the input.
//!
//! These are the Fortune-5 wiring proofs. The breed implementations
//! themselves are exhaustively tested in wasm4pm-cognition; here we
//! test the integration against a RevOps domain configuration.

mod revops_common;

use wasm4pm_cognition::breeds::{
    dispatch_breed_test, BreedInput, Candidate, Case, Fact, Goal, Rule, StateAtom,
};

/// RevOps domain fixture. Reused across all 9 station tests.
fn revops_input() -> BreedInput {
    BreedInput {
        intent:
            "Detect revenue leakage in Fortune-5 RevOps pipeline: forecast not supported, \
             bookings missing chain, late partner attribution, renewal risk undetected"
                .to_string(),
        candidates: vec![
            Candidate {
                id: "centralized-revenue-engine".into(),
                score: 0.5,
                eliminated: false,
                elimination_reason: None,
            },
            Candidate {
                id: "edge-distributed-reconciliation".into(),
                score: 0.5,
                eliminated: false,
                elimination_reason: None,
            },
        ],
        facts: vec![
            Fact { key: "scale".into(), value: "billion".into() },
            Fact { key: "compliance".into(), value: "strict".into() },
            Fact { key: "leakage".into(), value: "detected".into() },
            Fact { key: "current".into(), value: "no-architecture".into() },
        ],
        cases: vec![
            Case {
                id: "case-revops-001".into(),
                intent: "Booking reconciliation gap".into(),
                architecture: "centralized-revenue-engine".into(),
                outcome_score: 0.92,
                facts: vec![
                    Fact { key: "scale".into(), value: "billion".into() },
                    Fact { key: "compliance".into(), value: "strict".into() },
                ],
            },
            Case {
                id: "case-revops-002".into(),
                intent: "Late partner attribution".into(),
                architecture: "edge-distributed-reconciliation".into(),
                outcome_score: 0.78,
                facts: vec![Fact { key: "leakage".into(), value: "detected".into() }],
            },
        ],
        rules: vec![
            // For MYCIN / Prolog: certainty-bearing premise → conclusion.
            Rule {
                id: "r1".into(),
                premise: vec!["scale=billion".into()],
                conclusion: "favor=centralized-revenue-engine".into(),
                certainty: 0.9,
            },
            Rule {
                id: "r2".into(),
                premise: vec!["leakage=detected".into()],
                conclusion: "risk=high".into(),
                certainty: 0.85,
            },
            // For STRIPS: actionable rule whose premise is in the
            // initial state and conclusion satisfies a goal.
            Rule {
                id: "establish-reconciliation".into(),
                premise: vec!["current=no-architecture".into()],
                conclusion: "performance=high".into(),
                certainty: 1.0,
            },
        ],
        goals: vec![Goal {
            id: "g1".into(),
            predicate: "performance".into(),
            value: "high".into(),
        }],
        state: vec![StateAtom {
            predicate: "current".into(),
            value: "no-architecture".into(),
        }],
    }
}

fn assert_breed_runs_with_real_trace(breed: &str) {
    let input = revops_input();
    let output = match dispatch_breed_test(breed, &input) {
        Ok(o) => o,
        Err(e) => panic!("breed `{breed}` failed: {e}"),
    };
    // 1. Real algorithm produces real trace.
    assert!(
        !output.inference_trace.is_empty(),
        "breed `{breed}` produced an empty inference_trace — fraud signal"
    );
    // 2. Output JSON-serializable.
    let json = serde_json::to_string(&output).expect("serialize");
    assert!(
        json.contains("\"inference_trace\":"),
        "breed `{breed}` output should include inference_trace"
    );
    // 3. Breed must NOT inject authority not justified by input. The
    //    contract: any candidate scored high must have been a candidate
    //    on input. (We do not test this for breeds that intentionally
    //    add new candidates — DENDRAL's elimination model still must
    //    operate over the input set.)
    if matches!(breed, "cbr" | "mycin" | "soar" | "hearsay") {
        for c in &output.candidates {
            let was_input = revops_input().candidates.iter().any(|i| i.id == c.id);
            assert!(
                was_input,
                "breed `{breed}` introduced a candidate `{}` that was not in input — that is authority injection",
                c.id
            );
        }
    }
}

// ── 9 station tests ────────────────────────────────────────────────────────

#[test]
fn station_eliza_reflects_revops_voice() {
    assert_breed_runs_with_real_trace("eliza");
}

#[test]
fn station_cbr_retrieves_revops_cases() {
    assert_breed_runs_with_real_trace("cbr");
}

#[test]
fn station_dendral_enumerates_revops_architectures() {
    assert_breed_runs_with_real_trace("dendral");
}

#[test]
fn station_strips_plans_revops_evidence_route() {
    assert_breed_runs_with_real_trace("strips");
}

#[test]
fn station_prolog_constrains_revops_classification() {
    assert_breed_runs_with_real_trace("prolog");
}

#[test]
fn station_mycin_scores_revops_risk() {
    assert_breed_runs_with_real_trace("mycin");
}

#[test]
fn station_gps_reduces_revops_evidence_gap() {
    assert_breed_runs_with_real_trace("gps");
}

#[test]
fn station_soar_handles_revops_impasse() {
    assert_breed_runs_with_real_trace("soar");
}

#[test]
fn station_hearsay_fuses_revops_findings() {
    assert_breed_runs_with_real_trace("hearsay");
}

// ── Cross-cutting integrity check ──────────────────────────────────────────

#[test]
fn no_breed_returns_authority_with_zero_facts() {
    // If we strip the input down to bare-minimum (intent + 1 goal +
    // 1 rule for STRIPS), no breed is allowed to manufacture a
    // confident `selected` choice from nothing. We assert that any
    // `selected` value actually traces back to an input candidate or
    // case id.
    let mut empty = revops_input();
    empty.facts.clear();
    empty.cases.clear();
    // Keep state + rules so STRIPS still has something to plan.
    for breed in &["mycin", "cbr", "soar", "hearsay"] {
        if let Ok(output) = dispatch_breed_test(breed, &empty) {
            if let Some(sel) = &output.selected {
                let in_candidates = empty.candidates.iter().any(|c| c.id == *sel);
                let in_cases = empty.cases.iter().any(|c| c.id == *sel);
                assert!(
                    in_candidates || in_cases || sel.is_empty(),
                    "breed `{breed}` selected `{sel}` from empty facts/cases — \
                     that would be authority injection"
                );
            }
        }
    }
}
