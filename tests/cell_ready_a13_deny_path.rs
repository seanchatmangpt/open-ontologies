//! R5 WB-1 — §15 A13 ReplayProof tautology closure: deny-path proof.
//!
//! Before this round, `OntoStarAdmissionGate::evaluate` aliased the SAME
//! `ocel_trace_hash_hex` (computed at admission.rs:519) into BOTH
//! `CellReadyInputs::ocel_trace_hash` AND
//! `CellReadyInputs::replay_canonical_hash`, so the A13 equality check at
//! `cell_ready.rs:378` was vacuously true by construction. The gate could
//! not fail. This was a structural twin to the §15 A10 disease that R2
//! closed in 2024.
//!
//! R5 WB-1 introduces `re_snapshot_ocel_for_replay_proof` — an independent
//! BLAKE3 re-hash of `canonical_ocel_projection`. If the OCEL store
//! mutates between the two snapshots, A13 now fails with
//! `DefectClass::ReplayDivergence { expected, observed }` and the
//! observed/expected hashes are DISTINCT.
//!
//! This test drives a real `OntoStarAdmissionGate::evaluate` end-to-end,
//! installs `admission::A13_BETWEEN_SNAPSHOT_HOOK` (a `#[cfg(test)]`
//! thread_local), and uses the hook to emit a synthetic OCEL event with
//! a NEW event_type between the two snapshots. Because
//! `observed_event_types_for_session` returns DISTINCT event_types in
//! stable order, the second projection legitimately produces a different
//! byte vector → different BLAKE3 → different hex → A13 ReplayDivergence
//! with `expected != observed`.

use open_ontologies::admission::{
    self, AdmissionOp, ArtifactRef, NoopPowlReplay, OntoStarAdmissionGate,
};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("a13-deny-path.db");
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

/// Install the `A13_BETWEEN_SNAPSHOT_HOOK` for the current thread, run
/// `f`, and clear it on the way out so we never leak a hook across tests.
fn with_a13_hook<F: FnOnce() -> R, R>(
    hook: Box<dyn Fn(&OcelStore, &str, &str) + Send + 'static>,
    f: F,
) -> R {
    admission::A13_BETWEEN_SNAPSHOT_HOOK.with(|cell| {
        *cell.borrow_mut() = Some(hook);
    });
    let out = f();
    admission::A13_BETWEEN_SNAPSHOT_HOOK.with(|cell| {
        *cell.borrow_mut() = None;
    });
    out
}

/// The load-bearing test for R5 WB-1.
///
/// Drives a `DataExtensionFastPath` admission (a SEQ-only workflow whose
/// happy path is exercised by `tests/admission.rs::happy_path_admission_persists_receipt`).
/// The hook fires between the two OCEL snapshots and emits an event with
/// a **new** event_type (`a13_test_concurrent_mutation`). The second
/// projection sees the new type; the first did not. The two BLAKE3 hashes
/// MUST diverge, and `cell_ready` MUST refuse with `ReplayDivergence`.
#[test]
fn a13_replay_divergence_under_concurrent_mutation() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "a13-deny-session";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");

    // Required stages for the SEQ workflow — emitted BEFORE evaluation
    // so the conformance machinery sees a perfect trace.
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, session, &token, stage);
    }
    let observed = store.observed_event_types_for_session(session).unwrap();

    let workflow = by_name("DataExtensionFastPath").expect("workflow lookup");
    let required: Vec<String> = workflow
        .required_stages
        .iter()
        .map(|s| s.to_string())
        .collect();
    let gate = OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0");
    let powl = workflow.powl_string;
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"a13-deny-bytes",
    };

    // The hook installs a *new* event_type into the OCEL store between
    // the two snapshots. Because `observed_event_types_for_session` returns
    // DISTINCT event_types in stable order, the new event_type is ONLY
    // visible to the second projection — the first projection's bytes
    // are already on the stack inside `evaluate`. → distinct BLAKE3
    // hashes → A13 ReplayDivergence.
    let hook: Box<dyn Fn(&OcelStore, &str, &str) + Send + 'static> =
        Box::new(|store: &OcelStore, session: &str, scope: &str| {
            let now = chrono::Utc::now().to_rfc3339();
            let event_id = format!(
                "{}:a13_concurrent_mutation:{}",
                session,
                chrono::Utc::now()
                    .timestamp_nanos_opt()
                    .unwrap_or(0)
            );
            // event_type is a NEW string — guarantees the projection
            // changes (DISTINCT semantics on event_type).
            store
                .emit_event(
                    &event_id,
                    "a13_test_concurrent_mutation",
                    &now,
                    session,
                    &[],
                    &[],
                    Some(scope),
                )
                .expect("hook emit_event");
        });

    let result = with_a13_hook(hook, || {
        gate.evaluate(
            &token,
            AdmissionOp::Apply,
            &artifact,
            &store,
            &NoopPowlReplay,
            session,
            powl,
            &observed,
            "default",
        )
    });

    match result {
        Err((DefectClass::ReplayDivergence { expected, observed }, _)) => {
            assert_eq!(
                expected.len(),
                64,
                "expected hash must be 64-char hex (BLAKE3): {expected}"
            );
            assert_eq!(
                observed.len(),
                64,
                "observed hash must be 64-char hex (BLAKE3): {observed}"
            );
            assert_ne!(
                expected, observed,
                "A13 must report DISTINCT hashes: expected={expected} observed={observed}"
            );
        }
        other => panic!(
            "expected ReplayDivergence with two distinct hashes; got {:?}",
            other
        ),
    }
}

/// Sanity check: with NO hook installed, the same admission flow grants
/// successfully. Proves the hook is the only thing forcing divergence —
/// the post-fix re-snapshot is byte-identical to the first snapshot
/// when the store is quiescent, so A13 still passes on quiet workloads.
#[test]
fn a13_re_snapshot_quiescent_store_still_grants() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "a13-quiet-session";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, session, &token, stage);
    }
    let observed = store.observed_event_types_for_session(session).unwrap();

    let workflow = by_name("DataExtensionFastPath").expect("workflow lookup");
    let required: Vec<String> = workflow
        .required_stages
        .iter()
        .map(|s| s.to_string())
        .collect();
    let gate = OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0");
    let powl = workflow.powl_string;
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"a13-quiet-bytes",
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
        "default",
    );

    // Non-tautological A13: under quiescent store the two snapshots
    // produce the SAME hex (deterministic projection of the same set
    // of distinct event_types) → equality holds → A13 passes → grant.
    // (The replay verdict from NoopPowlReplay is fitness=1.0, precision=1.0,
    // so the upstream A4/A5 gates pass too.)
    assert!(
        result.is_ok(),
        "quiescent store must still grant; got {:?}",
        result.err()
    );
}
