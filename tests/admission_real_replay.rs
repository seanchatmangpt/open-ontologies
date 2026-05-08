//! R3 integration test — exercise the admission gate with the real
//! `PowlBridgeReplay`, not the `NoopPowlReplay` stub.
//!
//! Two cases:
//!   1. happy-path: declared LifecycleApply, all required stages emitted,
//!      gate admits with fitness >= 0.95 sourced from wasm4pm.
//!   2. broken-POWL: declared powl is syntactically invalid. Gate must
//!      deny with `ReplayFailed` (not silently pass through with 1.0).

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, OntoStarAdmissionGate, PowlBridgeReplay, PowlReplay,
};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("real-replay.db");
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
fn powl_bridge_replay_returns_real_fitness_not_stub_one_point_zero() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "real-replay-happy";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");

    // Emit the three activities of DataExtensionFastPath in order: load → extend → query.
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, session, &token, stage);
    }
    scope.close(&token).expect("close scope");

    let powl = by_name("DataExtensionFastPath").unwrap().powl_string;
    let replay = PowlBridgeReplay::new(&store);
    let conf = replay.replay(&token, powl);

    // The bridge — not the stub — produced this verdict. The stub would
    // have returned the literal {fitness: 1.0, run_id: "stub-run-..."}.
    assert!(
        !conf.run_id.starts_with("stub-run-"),
        "verdict came from NoopPowlReplay stub, not PowlBridge: run_id={}",
        conf.run_id
    );
    assert!(
        conf.fitness >= 0.95,
        "fitness should be >= 0.95 for a perfect-trace replay, got {}",
        conf.fitness
    );
}

#[test]
fn syntactically_broken_powl_denies_with_replay_failed() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "real-replay-broken";
    let scope = WorkflowScope::new(&db, session);

    // Inject a manually-crafted scope row whose powl_string is broken.
    // We can't open a builtin and then mutate, so we reach in directly via
    // the StateDb. The schema lives in src/state.rs.
    let token = "broken-scope-token-01";
    {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO declared_workflows (
                scope_token, session_id, name, powl_string, powl_hash,
                alphabet_json, declared_at, status
             ) VALUES (?1, ?2, 'BrokenForTest', ?3, 'deadbeef', '[]',
                       datetime('now'), 'open')",
            rusqlite::params![
                token,
                session,
                "PO=(nodes={a, b, c}, order={a-->b, b-->", // unterminated
            ],
        )
        .unwrap();
    }

    // Trace can be anything; the parse failure should short-circuit before
    // replay even runs.
    emit_stage(&store, session, token, "a");

    let powl = "PO=(nodes={a, b, c}, order={a-->b, b-->";
    let replay = PowlBridgeReplay::new(&store);
    let conf = replay.replay(token, powl);

    assert!(
        conf.fitness == 0.0,
        "broken POWL must yield fitness=0.0, got {}",
        conf.fitness
    );
    assert!(
        conf.verdict.contains("non_conform"),
        "broken POWL must yield non_conform verdict, got {}",
        conf.verdict
    );

    // Now run the full gate path with this broken POWL — admission must deny.
    let gate = OntoStarAdmissionGate::new(0.95, 0.85, vec![], "ontostar-1.0.0");
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"x",
    };
    let observed: Vec<String> = store
        .observed_event_types_for_session(session)
        .unwrap_or_default();
    let result = gate.evaluate(
        token,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &replay,
        session,
        powl,
        &observed,
    );
    let _ = scope.close(token); // best-effort cleanup
    match result {
        Err((DefectClass::ReplayFailed, _)) => {}
        Err((other, _)) => panic!("expected ReplayFailed, got {:?}", other),
        Ok(_) => panic!("expected admission to deny on broken POWL, but it admitted"),
    }
}
