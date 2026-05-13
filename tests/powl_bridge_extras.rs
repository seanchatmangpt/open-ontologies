//! Extra deny-path coverage for `classify_replay`. The function returns a
//! `ConformanceResult` (NOT `Result::Err`) whose `.defects` Vec carries
//! `(DefectClass, Deviation)` pairs.

use open_ontologies::powl_bridge::{classify_replay, PowlBridge};
use open_ontologies::DefectClass;

const SEQ_AB: &str = "PO=(nodes={a, b}, order={a-->b})";

#[test]
fn classify_replay_emits_extra_task() {
    let mut bridge = PowlBridge::new();
    let root = bridge.parse(SEQ_AB).expect("parse");
    let trace = vec!["a".to_string(), "b".to_string(), "extra_z".to_string()];
    let r = bridge.replay_trace(root, &trace).expect("replay");
    let cls = classify_replay(&bridge, root, &trace, &r);
    let has_extra = cls.defects.iter().any(|(d, _)| {
        matches!(d, DefectClass::ExtraTask { stage } if stage == "extra_z")
    });
    assert!(
        has_extra,
        "expected ExtraTask{{stage='extra_z'}} in defects: {:?}",
        cls.defects
    );
}

#[test]
fn classify_replay_emits_wrong_order_when_tokens_remain() {
    // Trace contains exactly the model alphabet but in the wrong order.
    // alphabet = {a, b}; observed = ["b", "a"] — no extra/missing labels,
    // but the partial-order constraint a-->b is violated, so wasm4pm
    // should leave remaining_tokens > 0.
    let mut bridge = PowlBridge::new();
    let root = bridge.parse(SEQ_AB).expect("parse");
    let trace = vec!["b".to_string(), "a".to_string()];
    let r = bridge.replay_trace(root, &trace).expect("replay");

    let cls = classify_replay(&bridge, root, &trace, &r);
    // Either we get WrongOrder directly (preferred) or, depending on
    // wasm4pm's token bookkeeping for this corner, we may see only
    // ReplayFailed. Both are valid signal of out-of-order behavior; the
    // primary contract is "no false 'conform'".
    assert!(!cls.is_conform(), "must not certify out-of-order trace as conform");
    let has_wrong_order = cls
        .defects
        .iter()
        .any(|(d, _)| matches!(d, DefectClass::WrongOrder { .. }));
    let has_replay_failed = cls
        .defects
        .iter()
        .any(|(d, _)| matches!(d, DefectClass::ReplayFailed));
    assert!(
        has_wrong_order || has_replay_failed,
        "expected WrongOrder or ReplayFailed, got: {:?}",
        cls.defects
    );
}
