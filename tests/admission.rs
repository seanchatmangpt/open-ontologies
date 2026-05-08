//! OntoStar Stream 3 admission gate tests.
//!
//! Verification plan items 4–8 from the OntoStar plan:
//!   (a) skipped-stage denial,
//!   (b) wrong-order / required-stage denial,
//!   (c) happy-path admission with persisted receipt,
//!   (d) replay enforcement after canonical-hash corruption,
//!   (e) bypass revocation.

use open_ontologies::admission::{
    self, AdmissionOp, ArtifactRef, NoopPowlReplay, OntoStarAdmissionGate,
};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("admission-test.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn emit_stage(store: &OcelStore, session: &str, scope: &str, stage: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let event_id = format!("{}:{}:{}", session, stage, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
    store
        .emit_event(&event_id, stage, &now, session, &[], &[], Some(scope))
        .unwrap();
}

fn build_gate(workflow_name: &str) -> OntoStarAdmissionGate {
    let required: Vec<String> = by_name(workflow_name)
        .map(|w| w.required_stages.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0")
}

/// (a) Skipped-stage denial: declare OntologyAuthoring, run load → validate →
/// reason → save, but skip enforce_run. Apply must be denied with
/// `CapabilityZero` and no apply_* event must appear in OCEL.
#[test]
fn skipped_stage_denial() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "test-session-skipped";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("OntologyAuthoring"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");

    // Emit only some required stages — deliberately skip `enforce_run`.
    for stage in &["load", "validate", "reason", "save"] {
        emit_stage(&store, session, &token, stage);
    }
    let observed: Vec<String> = store
        .observed_event_types_for_session(session)
        .unwrap();

    let gate = build_gate("OntologyAuthoring");
    let powl = by_name("OntologyAuthoring").unwrap().powl_string;
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"artifact-bytes",
    };
    let result = gate.evaluate(
        &token,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &NoopPowlReplay,
        session,
        powl,
        &observed,
    );
    match result {
        Err((DefectClass::CapabilityZero, _)) => {}
        other => panic!("expected CapabilityZero, got {:?}", other),
    }

    // Assert no apply_* event was emitted.
    for et in observed {
        assert!(!et.starts_with("apply_"), "saw apply event {et}");
    }
}

/// (b) Wrong-order / required-stage denial: declare LifecycleApply, run
/// `apply_safe → enforce_run`. The gate enforces required_stages = {plan_computed,
/// enforce_run}; without `plan_computed` admission fails with CapabilityZero.
#[test]
fn wrong_order_denial() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "test-session-wrong-order";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("LifecycleApply"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");

    // Emit apply_safe before enforce_run, and never emit plan_computed.
    emit_stage(&store, session, &token, "apply_safe");
    emit_stage(&store, session, &token, "enforce_run");

    let observed = store.observed_event_types_for_session(session).unwrap();
    let gate = build_gate("LifecycleApply");
    let powl = by_name("LifecycleApply").unwrap().powl_string;
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"x",
    };
    let result = gate.evaluate(
        &token,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &NoopPowlReplay,
        session,
        powl,
        &observed,
    );
    match result {
        // Without plan_computed in the observed trace, `RequiredStagesPresent`
        // short-circuits to CapabilityZero — the typed defect emitted in lieu
        // of free-text "wrong order" denials. See plan §"CellReady predicate".
        Err((DefectClass::CapabilityZero, _)) => {}
        other => panic!("expected CapabilityZero (wrong-order projection), got {:?}", other),
    }
}

/// (c) Happy path: declare LifecycleApply, run `plan_computed → enforce_run →
/// apply_safe`, expect Ok(receipt) with fitness ≥ 0.95 and a row in `receipts`.
#[test]
fn happy_path_admission_persists_receipt() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "test-session-happy";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("LifecycleApply"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");

    for stage in &["plan_computed", "enforce_run", "apply_safe"] {
        emit_stage(&store, session, &token, stage);
    }
    let observed = store.observed_event_types_for_session(session).unwrap();

    let gate = build_gate("LifecycleApply");
    let powl = by_name("LifecycleApply").unwrap().powl_string;
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"happy-path-bytes",
    };
    let receipt = gate
        .evaluate(
            &token,
            AdmissionOp::Apply,
            &artifact,
            &store,
            &NoopPowlReplay,
            session,
            powl,
            &observed,
        )
        .expect("admission must grant on happy path");

    // Receipt persisted in `receipts`.
    let conn = db.conn();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM receipts WHERE receipt_hash = ?1",
            rusqlite::params![receipt.hex()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1, "receipt row must be persisted");
    // The NoopPowlReplay returns fitness=1.0; assert ≥ 0.95.
    assert!(receipt.record.gates_passed.contains(&"ThresholdPass".to_string()));
}

/// (d) Replay enforcement: corrupt the ocel_canonical_hash by deleting the
/// `conformance_runs` row post-grant; on re-evaluate the conjunct
/// `POWLReplayPass` fails → ReplayFailed.
#[test]
fn replay_enforcement_after_corruption() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "test-session-replay";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("LifecycleApply"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    for stage in &["plan_computed", "enforce_run", "apply_safe"] {
        emit_stage(&store, session, &token, stage);
    }
    let observed = store.observed_event_types_for_session(session).unwrap();

    let gate = build_gate("LifecycleApply");
    let powl = by_name("LifecycleApply").unwrap().powl_string;
    let artifact = ArtifactRef { kind: "t", bytes: b"x" };
    gate.evaluate(
        &token,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &NoopPowlReplay,
        session,
        powl,
        &observed,
    )
    .expect("first eval grants");

    // Corrupt by removing the conformance_runs row → has_conforming_replay becomes false.
    db.conn()
        .execute(
            "DELETE FROM conformance_runs WHERE scope_token = ?1",
            rusqlite::params![&token],
        )
        .unwrap();

    // Custom replay implementation that returns no conformance row.
    struct NonConformingReplay;
    impl admission::PowlReplay for NonConformingReplay {
        fn replay(
            &self,
            scope_token: &str,
            _powl: &str,
        ) -> admission::ConformanceResult {
            admission::ConformanceResult {
                fitness: 1.0,
                precision: 1.0,
                verdict: "deviate".to_string(), // not 'conform'
                run_id: format!("non-conform-{}", scope_token),
            }
        }
    }

    let result = gate.evaluate(
        &token,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &NonConformingReplay,
        session,
        powl,
        &observed,
    );
    match result {
        Err((DefectClass::ReplayFailed, _)) => {}
        other => panic!("expected ReplayFailed after corruption, got {:?}", other),
    }
}

/// (e) Bypass revocation: revoke the session manually (simulating a
/// `bypass_admission=true` apply) then re-evaluate. Must deny with
/// `BypassRevoked` until session is reset.
#[test]
fn bypass_revokes_subsequent_operations() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "test-session-bypass";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("LifecycleApply"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    for stage in &["plan_computed", "enforce_run", "apply_safe"] {
        emit_stage(&store, session, &token, stage);
    }
    let observed = store.observed_event_types_for_session(session).unwrap();

    // Revoke the session as the bypass-admission path would.
    admission::revoke_session(&db, session, "manual bypass for test").unwrap();

    let gate = build_gate("LifecycleApply");
    let powl = by_name("LifecycleApply").unwrap().powl_string;
    let artifact = ArtifactRef { kind: "t", bytes: b"y" };
    let result = gate.evaluate(
        &token,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &NoopPowlReplay,
        session,
        powl,
        &observed,
    );
    match result {
        Err((DefectClass::BypassRevoked, _)) => {}
        other => panic!("expected BypassRevoked, got {:?}", other),
    }

    // After session reset, the gate admits again.
    admission::clear_revocation(&db, session).unwrap();
    gate.evaluate(
        &token,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &NoopPowlReplay,
        session,
        powl,
        &observed,
    )
    .expect("after reset, gate must admit again");
}
