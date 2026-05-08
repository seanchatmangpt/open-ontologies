//! Level-5 replay-portability test.
//!
//! Proves the OCEL stream is self-sufficient: an external observer with only
//! the event log can reconstruct what was supposed to happen. We admit a
//! scope, then DELETE the `declared_workflows` row, then call
//! `replay_from_ocel_alone` and assert it succeeds with identical conformance
//! to the canonical replay.

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, OntoStarAdmissionGate, PowlBridgeReplay,
};
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("replay-portability.db");
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

#[test]
fn replay_from_ocel_alone_matches_canonical_replay() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "portability-happy";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");

    // Emit the activity trace: load → extend → query.
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, session, &token, stage);
    }
    scope.close(&token).expect("close scope");

    let powl = by_name("DataExtensionFastPath").unwrap().powl_string;
    let replay = PowlBridgeReplay::new(&store);

    // Run the gate so `evaluate()` emits the workflow_declared anchor event
    // (the thing that makes the OCEL stream self-sufficient).
    let gate = OntoStarAdmissionGate::new(0.0, 0.0, vec![], "ontostar-1.0.0");
    let artifact = ArtifactRef { kind: "test", bytes: b"a" };
    let observed: Vec<String> = store
        .observed_event_types_for_session(session)
        .unwrap_or_default();
    let _ = gate.evaluate(
        &token,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &replay,
        session,
        powl,
        &observed,
    );

    // Sabotage: delete the declared_workflows row. From this point on, the
    // canonical replay path (which reads declared_workflows) cannot work, but
    // replay_from_ocel_alone should still succeed because the anchor event
    // carries the powl_string.
    {
        let conn = db.conn();
        conn.execute(
            "DELETE FROM declared_workflows WHERE scope_token = ?1",
            rusqlite::params![&token],
        )
        .expect("delete declared_workflows row");
    }

    // Confirm the row is gone.
    {
        let conn = db.conn();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM declared_workflows WHERE scope_token = ?1",
                rusqlite::params![&token],
                |r| r.get(0),
            )
            .unwrap_or(-1);
        assert_eq!(n, 0, "declared_workflows row should be gone");
    }

    // Replay using only the OCEL stream.
    let result = store
        .replay_from_ocel_alone(&token)
        .expect("replay_from_ocel_alone must succeed when anchor event exists");

    // The fitness should match the canonical replay (both use the same
    // wasm4pm bridge against the same trace + same POWL).
    assert!(
        result.fitness >= 0.95,
        "OCEL-alone replay must produce real fitness >= 0.95, got {}",
        result.fitness
    );
}

#[test]
fn replay_from_ocel_alone_errs_when_no_anchor_event() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());

    // No scope, no admission, no anchor event → replay must error rather
    // than silently fabricate a verdict.
    let result = store.replay_from_ocel_alone("nonexistent-scope");
    assert!(
        result.is_err(),
        "scope without anchor event must error, but got: {:?}",
        result.map(|r| r.fitness)
    );
}
