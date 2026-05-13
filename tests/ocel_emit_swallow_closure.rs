//! R5 WB-2 — §15 OCEL anchor closure: counterfactual proof for the 3 emit
//! sites that previously did `let _ = store.emit_event(...)`.
//!
//! Before this round, three sites in `src/admission.rs` swallowed OCEL
//! emit failures:
//!
//! 1. `admission_denied` (every denial path) — caller saw `Err(...)` but
//!    OCEL had no witness, so a downstream auditor could not see the deny.
//! 2. `workflow_declared` (replay-portability anchor) — caller proceeded
//!    to write a receipt whose declared model could not be reconstructed
//!    from OCEL alone.
//! 3. `conformance_runs` INSERT — no backing OCEL event, so a verifier
//!    joining `receipts` ↔ `ocel_events` ↔ `conformance_runs` could not
//!    prove the conformance row was used at admission.
//!
//! These tests use the new
//! `open_ontologies::ocel_store::EMIT_FAILURE_INJECTION_HOOK` thread_local
//! to force `emit_event` to fail for specific event_types. They prove:
//!
//! - Sites 1 and 2: the fallback `*_emit_failed` event lands when the
//!   primary fails. No phantom denial, no broken replay anchor.
//! - Site 3: when the OCEL emit inside the conformance transaction fails,
//!   the INSERT INTO conformance_runs is rolled back atomically — the
//!   row is NOT visible to a subsequent `SELECT`. When the OCEL emit
//!   succeeds, the `conformance_recorded` event IS observable.
//!
//! These are deny-path / counterfactual gates per §17. Without the hook
//! the bug would silently survive every round of refactoring.

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, NoopPowlReplay, OntoStarAdmissionGate,
};
use open_ontologies::ocel_store::{OcelStore, EMIT_FAILURE_INJECTION_HOOK};
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("emit-swallow-closure.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn emit_stage(store: &OcelStore, session: &str, scope: &str, stage: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let event_id = format!(
        "{}:{}:{}",
        session,
        stage,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
    );
    store
        .emit_event(&event_id, stage, &now, session, &[], &[], Some(scope))
        .expect("emit_stage");
}

/// Install a closure that returns `Some(error)` whenever `emit_event_rows`
/// is called with an `event_type` matching `block`. Run `f`, then clear
/// the hook so we never leak it across tests.
fn with_emit_failure<F: FnOnce() -> R, R>(block: &'static str, f: F) -> R {
    EMIT_FAILURE_INJECTION_HOOK.with(|cell| {
        *cell.borrow_mut() = Some(Box::new(move |et: &str| {
            if et == block {
                Some(anyhow::anyhow!(
                    "synthetic emit failure injected for event_type={et}"
                ))
            } else {
                None
            }
        }));
    });
    let out = f();
    EMIT_FAILURE_INJECTION_HOOK.with(|cell| {
        *cell.borrow_mut() = None;
    });
    out
}

fn count_events(db: &StateDb, event_type: &str) -> i64 {
    db.conn()
        .query_row(
            "SELECT COUNT(*) FROM ocel_events WHERE event_type = ?1",
            rusqlite::params![event_type],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
}

fn count_conformance_rows(db: &StateDb, scope_token: &str) -> i64 {
    db.conn()
        .query_row(
            "SELECT COUNT(*) FROM conformance_runs WHERE scope_token = ?1",
            rusqlite::params![scope_token],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
}

/// Site 1 counterfactual: when the primary `admission_denied` emit is
/// forced to fail, the fallback `admission_denied_ocel_failed` event
/// MUST be present. Previously this test would have shown ZERO events
/// of either type — the swallow erased the witness entirely.
#[test]
fn denial_witness_survives_primary_emit_failure() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "wb2-denial-session";
    // No scope_token → admission denies with `ScopeUnclosed`. Choose this
    // because it is the cheapest deny path — it never touches the
    // conformance row or the workflow_declared anchor.
    let workflow = by_name("DataExtensionFastPath").expect("workflow");
    let required: Vec<String> = workflow
        .required_stages
        .iter()
        .map(|s| s.to_string())
        .collect();
    let gate = OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0");
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"wb2-denial-bytes",
    };

    let result = with_emit_failure("admission_denied", || {
        gate.evaluate(
            "", // empty scope_token forces ScopeUnclosed deny BEFORE any
                // workflow_declared emit could happen.
            AdmissionOp::Apply,
            &artifact,
            &store,
            &NoopPowlReplay,
            session,
            workflow.powl_string,
            &[],
            "default",
        )
    });

    assert!(result.is_err(), "ScopeUnclosed must deny");

    // Primary emit was sabotaged → ZERO admission_denied events.
    assert_eq!(
        count_events(&db, "admission_denied"),
        0,
        "primary emit was injected to fail; no admission_denied event must land",
    );
    // Fallback MUST be present — the deny is no longer phantom.
    assert_eq!(
        count_events(&db, "admission_denied_ocel_failed"),
        1,
        "fallback admission_denied_ocel_failed event MUST land when primary fails",
    );
}

/// Sanity baseline for site 1: with NO injection, the primary emit
/// records and the fallback does NOT.
#[test]
fn denial_witness_normal_path_no_fallback() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "wb2-denial-quiet";
    let workflow = by_name("DataExtensionFastPath").expect("workflow");
    let required: Vec<String> = workflow
        .required_stages
        .iter()
        .map(|s| s.to_string())
        .collect();
    let gate = OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0");
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"wb2-quiet-bytes",
    };
    let _ = gate.evaluate(
        "",
        AdmissionOp::Apply,
        &artifact,
        &store,
        &NoopPowlReplay,
        session,
        workflow.powl_string,
        &[],
        "default",
    );
    assert_eq!(count_events(&db, "admission_denied"), 1);
    assert_eq!(count_events(&db, "admission_denied_ocel_failed"), 0);
}

/// Site 2 counterfactual: the `workflow_declared` anchor is load-bearing
/// for replay-from-OCEL-alone (Plan B). When the primary emit is forced
/// to fail, the fallback `workflow_declared_emit_failed` event MUST be
/// present so an external auditor can still rebuild the declared model
/// from a degraded trail.
#[test]
fn workflow_declared_witness_survives_primary_emit_failure() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "wb2-workflow-declared";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, session, &token, stage);
    }
    let observed = store.observed_event_types_for_session(session).unwrap();
    let workflow = by_name("DataExtensionFastPath").expect("workflow");
    let required: Vec<String> = workflow
        .required_stages
        .iter()
        .map(|s| s.to_string())
        .collect();
    let gate = OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0");
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"wb2-wd-bytes",
    };

    // Sabotage workflow_declared specifically — sites 1 and 3 are
    // unaffected. Admission still proceeds (the helper records the
    // failure but does not propagate). Conformance + replay both pass
    // for this workflow → grant.
    let result = with_emit_failure("workflow_declared", || {
        gate.evaluate(
            &token,
            AdmissionOp::Apply,
            &artifact,
            &store,
            &NoopPowlReplay,
            session,
            workflow.powl_string,
            &observed,
            "default",
        )
    });
    // Independent of grant/deny — the witness contract is what we are
    // proving here. Grant or deny, the OCEL anchor MUST exist (degraded
    // or primary).
    let _ = result;

    assert_eq!(
        count_events(&db, "workflow_declared"),
        0,
        "primary emit was injected to fail; no workflow_declared event must land",
    );
    assert_eq!(
        count_events(&db, "workflow_declared_emit_failed"),
        1,
        "fallback workflow_declared_emit_failed MUST land — Plan B's \
         replay-portability anchor cannot be silently dropped",
    );
}

/// Site 3a (positive path): when conformance INSERT succeeds, the
/// `conformance_recorded` OCEL event MUST be present in the SAME
/// transaction window. Joiner-readiness anchor for downstream verifiers.
#[test]
fn conformance_runs_writes_atomic_with_ocel_witness() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "wb2-conformance-grant";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, session, &token, stage);
    }
    let observed = store.observed_event_types_for_session(session).unwrap();
    let workflow = by_name("DataExtensionFastPath").expect("workflow");
    let required: Vec<String> = workflow
        .required_stages
        .iter()
        .map(|s| s.to_string())
        .collect();
    let gate = OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0");
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"wb2-conf-grant-bytes",
    };

    let result = gate.evaluate(
        &token,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &NoopPowlReplay,
        session,
        workflow.powl_string,
        &observed,
        "default",
    );
    // The NoopPowlReplay returns a 1.0/1.0 verdict so the conformance
    // INSERT happens regardless of the final cell_ready outcome.
    let _ = result;

    // The conformance row must be durable.
    assert_eq!(
        count_conformance_rows(&db, &token),
        1,
        "conformance_runs row must be present after admission",
    );
    // The OCEL witness must be present — atomic-with-INSERT.
    assert!(
        count_events(&db, "conformance_recorded") >= 1,
        "conformance_recorded OCEL event MUST land in the same tx as the \
         conformance_runs INSERT — joiner-readiness anchor",
    );
}

/// Site 3b (counterfactual rollback): when the OCEL `conformance_recorded`
/// emit is forced to fail, the conformance_runs INSERT MUST be rolled
/// back. The row MUST NOT exist. This is the load-bearing proof that
/// the transaction is a real atomic boundary, not decorative.
#[test]
fn conformance_runs_rollback_on_ocel_witness_failure() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "wb2-conformance-rollback";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, session, &token, stage);
    }
    let observed = store.observed_event_types_for_session(session).unwrap();
    let workflow = by_name("DataExtensionFastPath").expect("workflow");
    let required: Vec<String> = workflow
        .required_stages
        .iter()
        .map(|s| s.to_string())
        .collect();
    let gate = OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0");
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"wb2-conf-rollback-bytes",
    };

    // Sabotage conformance_recorded specifically. The INSERT into
    // conformance_runs runs FIRST inside the tx; the sabotaged
    // OCEL emit runs SECOND. The tx commit never happens — INSERT is
    // rolled back.
    let _ = with_emit_failure("conformance_recorded", || {
        gate.evaluate(
            &token,
            AdmissionOp::Apply,
            &artifact,
            &store,
            &NoopPowlReplay,
            session,
            workflow.powl_string,
            &observed,
            "default",
        )
    });

    // Atomicity proof: NO conformance_runs row.
    assert_eq!(
        count_conformance_rows(&db, &token),
        0,
        "conformance_runs INSERT MUST be rolled back when its OCEL witness \
         emit fails — atomic boundary is load-bearing, not decorative",
    );
    // And NO conformance_recorded event.
    assert_eq!(
        count_events(&db, "conformance_recorded"),
        0,
        "no conformance_recorded event must land when its emit was sabotaged",
    );
}
