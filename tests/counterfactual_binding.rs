//! Level-5 counterfactual-binding test.
//!
//! At admission time, the gate persists `gates_fired_json` (Ok) /
//! `gates_denied_json` (Err) and `manufacturing_delta_json` on the
//! `declared_workflows` row. This test admits a scope and then a sabotaged
//! scope, and asserts the persisted columns reflect what actually fired —
//! not the previously-hardcoded `vec![]` placeholders.

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, OntoStarAdmissionGate, PowlBridgeReplay,
};
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("counterfactual.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn emit_stage(store: &OcelStore, session: &str, scope: &str, stage: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let event_id = format!(
        "{}:{}:{}",
        session,
        stage,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    store
        .emit_event(&event_id, stage, &now, session, &[], &[], Some(scope))
        .unwrap();
}

fn read_outcome(db: &StateDb, scope_token: &str) -> (Option<i64>, Option<String>, Option<String>, Option<String>) {
    let conn = db.conn();
    conn
        .query_row(
            "SELECT admitted, gates_fired_json, gates_denied_json, manufacturing_delta_json
             FROM declared_workflows WHERE scope_token = ?1",
            rusqlite::params![scope_token],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap()
}

#[test]
fn admission_ok_persists_gates_fired_and_manufacturing_delta() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "cf-happy";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, session, &token, stage);
    }
    scope.close(&token).expect("close scope");

    let powl = by_name("DataExtensionFastPath").unwrap().powl_string;
    let replay = PowlBridgeReplay::new(&store);
    let gate = OntoStarAdmissionGate::new(0.0, 0.0, vec![], "ontostar-1.0.0");
    let artifact = ArtifactRef { kind: "test", bytes: b"a" };
    let observed: Vec<String> = store
        .observed_event_types_for_session(session)
        .unwrap_or_default();
    let result = gate.evaluate(
        &token, AdmissionOp::Apply, &artifact, &store, &replay,
        session, powl, &observed, "default",
    );
    assert!(result.is_ok(), "happy-path admission must succeed: {:?}", result.err());

    let (admitted, gates_fired, gates_denied, delta) = read_outcome(&db, &token);
    assert_eq!(admitted, Some(1), "admitted column should be 1");
    let fired: serde_json::Value = serde_json::from_str(
        gates_fired.as_deref().expect("gates_fired_json must be present"),
    )
    .expect("gates_fired_json must be valid JSON");
    let fired_arr = fired.as_array().expect("gates_fired must be array");
    assert!(
        fired_arr.len() >= 6,
        "gates_fired should contain ≥6 conjuncts on success, got {} ({:?})",
        fired_arr.len(),
        fired
    );
    // Spot-check that the canonical conjunct names are present.
    let names: Vec<String> = fired_arr
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    // Phase 10: gate names are now `A<n>_<Name>` per the 13-gate canonical
    // ordering in `src/cell_ready.rs`. Tests must spot-check against the
    // canonical names (was `WorkflowDeclared`, now `A1_WorkflowDeclared`).
    for required in &[
        "A1_WorkflowDeclared",
        "A2_ScopeClosed",
        "A3_OCELComplete",
        "A4_POWLReplayPass",
        "A6_RequiredStagesPresent",
    ] {
        assert!(
            names.iter().any(|n| n == required),
            "expected gate {} in fired list, got {:?}",
            required,
            names
        );
    }
    assert!(
        gates_denied.as_deref() == Some("[]"),
        "gates_denied should be empty array on success, got {:?}",
        gates_denied
    );
    let delta_str = delta.as_deref().expect("manufacturing_delta_json must be present");
    assert!(
        delta_str.contains("granted_by_force"),
        "manufacturing_delta must reference naked-craft verdict, got {}",
        delta_str
    );
    assert!(
        delta_str.contains("fired_only_under_ontostar"),
        "manufacturing_delta must name the OntoStar-exclusive gates field, got {}",
        delta_str
    );
}

#[test]
fn admission_denial_persists_gates_denied_with_defect_tag() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "cf-denied";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");

    // Trace conforms to the declared POWL (load → extend → query), so
    // POWLReplayPass and OCELComplete will both pass. We then require a
    // stage that's NOT in the trace, so `RequiredStagesPresent` (conjunct
    // #6) is the conjunct that fires the denial — yielding `CapabilityZero`.
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, session, &token, stage);
    }

    let powl = by_name("DataExtensionFastPath").unwrap().powl_string;
    let replay = PowlBridgeReplay::new(&store);
    let gate = OntoStarAdmissionGate::new(
        0.0,
        0.0,
        vec!["mandatory_stage_not_in_trace".to_string()],
        "ontostar-1.0.0",
    );
    let artifact = ArtifactRef { kind: "test", bytes: b"x" };
    let observed: Vec<String> = store
        .observed_event_types_for_session(session)
        .unwrap_or_default();
    let result = gate.evaluate(
        &token, AdmissionOp::Apply, &artifact, &store, &replay,
        session, powl, &observed, "default",
    );
    assert!(result.is_err(), "skipped-stage admission must deny");

    let (admitted, _gates_fired, gates_denied, _delta) = read_outcome(&db, &token);
    assert_eq!(admitted, Some(0), "admitted column should be 0 on denial");
    let denied: serde_json::Value = serde_json::from_str(
        gates_denied.as_deref().expect("gates_denied_json must be present"),
    )
    .expect("gates_denied_json must be valid JSON");
    let denied_arr = denied.as_array().expect("gates_denied must be array");
    assert_eq!(denied_arr.len(), 1, "gates_denied should have exactly 1 entry, got {:?}", denied);
    let tag = denied_arr[0].as_str().expect("denied entry must be string");
    assert_eq!(
        tag, "capability_zero",
        "expected capability_zero defect tag, got {}",
        tag
    );
}
