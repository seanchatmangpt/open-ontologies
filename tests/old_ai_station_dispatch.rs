//! Phase 1.6 — old-AI station dispatcher smoke test.
//!
//! Drives `wasm4pm_cognition::breeds::dispatch_breed_test` against all 9
//! breeds with a minimal but-non-empty `BreedInput` and asserts:
//!
//! 1. Each breed returns Ok with a non-empty `inference_trace` (the
//!    fraud-detection invariant: a real algorithm must produce trace
//!    steps).
//! 2. Each breed's output is JSON-serializable (so the MCP handler can
//!    return it).
//! 3. The dispatcher rejects unknown breed names.
//!
//! This is a wiring proof — the breeds themselves are extensively tested
//! in the wasm4pm-cognition crate. We just verify the integration.

use wasm4pm_cognition::breeds::{
    dispatch_breed_test, BreedInput, Candidate, Case, Fact, Goal, Rule, StateAtom,
};

fn fixture_input() -> BreedInput {
    BreedInput {
        intent: "RevOps revenue leakage detection on a Fortune-5 booking pipeline"
            .to_string(),
        candidates: vec![
            Candidate {
                id: "centralized-cloud".into(),
                score: 0.5,
                eliminated: false,
                elimination_reason: None,
            },
            Candidate {
                id: "edge-distributed".into(),
                score: 0.5,
                eliminated: false,
                elimination_reason: None,
            },
        ],
        facts: vec![
            Fact { key: "scale".into(), value: "billion".into() },
            Fact { key: "latency".into(), value: "low".into() },
            Fact { key: "compliance".into(), value: "strict".into() },
        ],
        cases: vec![Case {
            id: "case-001".into(),
            intent: "Booking reconciliation gap".into(),
            architecture: "centralized-cloud".into(),
            outcome_score: 0.9,
            facts: vec![Fact { key: "scale".into(), value: "billion".into() }],
        }],
        rules: vec![
            // For MYCIN / Prolog: premise atoms in MYCIN-rule form.
            Rule {
                id: "r1".into(),
                premise: vec!["scale=billion".into()],
                conclusion: "favor=centralized-cloud".into(),
                certainty: 0.85,
            },
            // For STRIPS: premise must be satisfied by initial state
            // atoms (predicate=value form), conclusion adds the goal
            // atom. State has `current=no-architecture`, goal is
            // `performance=high`, so this single-step plan satisfies.
            Rule {
                id: "promote-cloud".into(),
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

#[test]
fn every_breed_returns_non_empty_trace() {
    let input = fixture_input();
    let breeds = [
        "eliza", "cbr", "dendral", "strips", "prolog", "mycin", "gps", "soar", "hearsay",
    ];
    for breed in &breeds {
        let result = dispatch_breed_test(breed, &input);
        let output = match result {
            Ok(o) => o,
            Err(e) => panic!("breed `{breed}` failed: {e}"),
        };
        // Fraud-detection invariant: a real algorithm produces trace steps.
        assert!(
            !output.inference_trace.is_empty(),
            "breed `{breed}` produced an empty inference_trace — that is a fraud signal"
        );
        // Output must be JSON-serializable (so MCP handler can return it).
        let _json: serde_json::Value =
            serde_json::to_value(&output).expect("output should serialize as JSON");
    }
}

#[test]
fn unknown_breed_is_rejected() {
    let input = fixture_input();
    let err = dispatch_breed_test("not-a-real-breed", &input).unwrap_err();
    assert!(
        err.contains("unknown breed"),
        "expected `unknown breed` in error, got: {err}"
    );
}

#[test]
fn output_is_json_round_trippable() {
    // The MCP handler returns the BreedOutput as JSON. Round-trip a
    // representative breed to lock the format.
    let input = fixture_input();
    let output = dispatch_breed_test("mycin", &input).expect("mycin runs");
    let json = serde_json::to_string(&output).expect("serialize");
    // Field presence smoke check.
    assert!(json.contains("\"breed\":"));
    assert!(json.contains("\"inference_trace\":"));
    assert!(json.contains("\"explanation\":"));
}
