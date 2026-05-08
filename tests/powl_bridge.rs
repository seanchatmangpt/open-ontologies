//! Stream 2 integration test — exercises the wasm4pm-backed bridge.
//!
//! All fitness/replay numbers in this test originate in
//! `wasm4pm::powl::conformance::token_replay`. The bridge is plumbing only.

use open_ontologies::powl_bridge::{classify_replay, PowlBridge};
use open_ontologies::DefectClass;

/// `SEQ(a, b, c)` in the plan maps to a POWL strict-partial-order with
/// edges `a-->b, b-->c` in wasm4pm's grammar (no `SEQ` keyword exists in the
/// parser; sequencing is expressed via `PO=(nodes={...}, order={...})`).
const SEQ_ABC: &str =
    "PO=(nodes={a, b, c}, order={a-->b, b-->c})";

#[test]
fn seq_perfect_trace_has_fitness_one() {
    let mut bridge = PowlBridge::new();
    let root = bridge.parse(SEQ_ABC).expect("parse SEQ(a,b,c)");
    let trace = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let r = bridge.replay_trace(root, &trace).expect("replay perfect");
    assert!(
        (r.fitness - 1.0).abs() < 1e-6,
        "expected fitness ≈ 1.0, got {}",
        r.fitness
    );
    assert_eq!(r.missing_tokens, 0);
    assert_eq!(r.remaining_tokens, 0);

    let cls = classify_replay(&bridge, root, &trace, &r);
    assert!(cls.is_conform(), "verdict={}", cls.verdict);
    assert!(cls.defects.is_empty(), "defects={:?}", cls.defects);
}

#[test]
fn seq_skipped_stage_yields_skipped_task_defect() {
    let mut bridge = PowlBridge::new();
    let root = bridge.parse(SEQ_ABC).expect("parse");
    let trace = vec!["a".to_string(), "c".to_string()];
    let r = bridge.replay_trace(root, &trace).expect("replay skip");

    // wasm4pm reports fitness < 1 for skipped stages.
    assert!(
        r.fitness < 1.0,
        "expected fitness < 1 when stage skipped, got {}",
        r.fitness
    );

    let cls = classify_replay(&bridge, root, &trace, &r);
    assert!(cls.fitness < 1.0);
    let has_skipped_b = cls.defects.iter().any(|(d, _)| {
        matches!(d, DefectClass::SkippedTask { stage } if stage == "b")
    });
    assert!(
        has_skipped_b,
        "expected SkippedTask{{stage='b'}} in defects, got {:?}",
        cls.defects
    );
}
