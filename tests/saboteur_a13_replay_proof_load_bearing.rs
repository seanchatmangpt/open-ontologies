//! R5 WB-1 — Saboteur matrix for §15 A13 ReplayProof load-bearingness.
//!
//! This is a documentation-test marked `#[ignore]`. It is NOT part of the
//! standard `cargo make test` run — it exists to be invoked manually with
//! `cargo test --test saboteur_a13_replay_proof_load_bearing -- --ignored`
//! when the maintainer wants a face-to-face sanity check that the §15 A13
//! ReplayProof gate is still load-bearing after future refactors.
//!
//! Why this test exists
//! ====================
//!
//! Before R5 WB-1, `OntoStarAdmissionGate::evaluate` aliased the SAME
//! `ocel_trace_hash_hex` (computed from one BLAKE3 of one OCEL projection
//! at admission.rs:519) into BOTH `CellReadyInputs::ocel_trace_hash` AND
//! `CellReadyInputs::replay_canonical_hash`. The A13 equality check at
//! `cell_ready.rs:378`:
//!
//! ```ignore
//! if inp.replay_canonical_hash != inp.ocel_trace_hash {
//!     return Err(DefectClass::ReplayDivergence { ... });
//! }
//! ```
//!
//! was vacuously true by construction — the gate was a structural
//! tautology. This was the same disease as the §15 A10 tautology that
//! Round 2 closed in 2024. Plan B's deeper §15 audit (R5) surfaced A13,
//! A9, A11, and A12 as the remaining tautological conjuncts.
//!
//! R5 WB-1 closes A13 by introducing
//! `re_snapshot_ocel_for_replay_proof` — an INDEPENDENT BLAKE3 re-hash
//! of the OCEL projection. If the store mutates between the two
//! snapshots, A13 now fails with two DISTINCT hashes.
//! `tests/cell_ready_a13_deny_path.rs` proves the deny path with a
//! deterministic `#[cfg(test)] A13_BETWEEN_SNAPSHOT_HOOK`.
//!
//! Saboteur matrix
//! ===============
//!
//! With-fix (current main):
//!   1. Emit `load → extend → query` events on a `DataExtensionFastPath` scope.
//!   2. Open `OntoStarAdmissionGate::evaluate`.
//!   3. Hook fires between line-519 first-snapshot and the new
//!      `re_snapshot_ocel_for_replay_proof` second-snapshot.
//!   4. Hook emits an OCEL event with NEW event_type
//!      (`a13_test_concurrent_mutation`).
//!   5. Second projection sees the new event_type;
//!      `observed_event_types_for_session` returns `DISTINCT event_type`s
//!      so the new type is ONLY visible to the second projection.
//!   6. Two distinct projection byte vectors → two distinct BLAKE3
//!      hashes → two distinct hex strings.
//!   7. cell_ready returns `Err(DefectClass::ReplayDivergence { expected,
//!      observed })`.
//!   8. ✅ A13 caught the race.
//!
//! Without-fix (pre-R5-WB-1, hypothetical sabotage):
//!   1–4. Same as above.
//!   5'. The struct literal aliases the same line-519 hex into BOTH
//!       fields. The hook's mutation cannot influence either input
//!       (both come from the same single hash already on the stack).
//!   6'. `inp.replay_canonical_hash == inp.ocel_trace_hash` is TRUE by
//!       construction.
//!   7'. cell_ready returns `Ok(receipt)` — A13 silently passed.
//!   8'. ❌ A13 was a tautology; the race was undetectable.
//!
//! How to verify the fix is load-bearing manually
//! ===============================================
//!
//! 1. Run this binary with `--ignored`:
//!    ```bash
//!    cargo test --test saboteur_a13_replay_proof_load_bearing -- --ignored
//!    ```
//!    The active assertion below MUST pass — proving the deny path fires
//!    after the fix.
//!
//! 2. To prove the OLD code was a tautology:
//!    a. In `src/admission.rs`, comment out the new local
//!       `let replay_canonical_hash_hex = re_snapshot_ocel_for_replay_proof(...);`
//!    b. Restore `replay_canonical_hash: &ocel_trace_hash_hex` (the
//!       pre-R5-WB-1 alias).
//!    c. Re-run this test → it MUST FAIL because the gate now grants
//!       even with the hook installed (tautology restored).
//!    d. Revert the sabotage. The test passes again.
//!
//! 3. To prove the hook is the only thing producing divergence:
//!    a. In `tests/cell_ready_a13_deny_path.rs`, remove the
//!       `with_a13_hook(...)` wrapper.
//!    b. Re-run that test → admission GRANTS (no race). This is
//!       expected: a quiescent OCEL store has identical first and
//!       second snapshots. A13 only fails under real concurrent
//!       mutation, exactly as the gate is designed.
//!
//! Companion files
//! ===============
//! - `tests/cell_ready_a13_deny_path.rs` — the active proof.
//! - `src/admission.rs::re_snapshot_ocel_for_replay_proof` — the fix.
//! - `src/cell_ready.rs:378` — the A13 equality check.
//! - `src/defects.rs::DefectClass::ReplayDivergence` — the typed defect.

use open_ontologies::admission::{
    self, AdmissionOp, ArtifactRef, NoopPowlReplay, OntoStarAdmissionGate,
};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

#[test]
#[ignore = "Documentation/saboteur test — run manually with --ignored to \
            confirm A13 ReplayProof remains load-bearing"]
fn a13_replay_proof_is_load_bearing_under_concurrent_mutation() {
    // ---- setup: identical to tests/cell_ready_a13_deny_path.rs ----
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("a13-saboteur.db");
    std::mem::forget(dir);
    let db = StateDb::open(&path).expect("open StateDb");
    let store = OcelStore::new(db.clone());
    let session = "a13-saboteur-session";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    for stage in &["load", "extend", "query"] {
        let now = chrono::Utc::now().to_rfc3339();
        let event_id = format!(
            "{}:{}:{}",
            session,
            stage,
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        store
            .emit_event(&event_id, stage, &now, session, &[], &[], Some(&token))
            .unwrap();
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
        bytes: b"a13-saboteur-bytes",
    };

    // ---- saboteur hook: emit a NEW event_type between snapshots ----
    let hook: Box<dyn Fn(&OcelStore, &str, &str) + Send + 'static> =
        Box::new(|store: &OcelStore, session: &str, scope: &str| {
            let now = chrono::Utc::now().to_rfc3339();
            let event_id = format!(
                "{}:a13_saboteur:{}",
                session,
                chrono::Utc::now()
                    .timestamp_nanos_opt()
                    .unwrap_or(0)
            );
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
                .expect("saboteur emit");
        });

    admission::A13_BETWEEN_SNAPSHOT_HOOK.with(|cell| {
        *cell.borrow_mut() = Some(hook);
    });
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
    admission::A13_BETWEEN_SNAPSHOT_HOOK.with(|cell| {
        *cell.borrow_mut() = None;
    });

    // ---- assertion: A13 caught the race ----
    match result {
        Err((DefectClass::ReplayDivergence { expected, observed }, _)) => {
            assert_ne!(
                expected, observed,
                "A13 must report two DISTINCT hashes; got expected={expected} observed={observed}"
            );
        }
        other => panic!(
            "A13 ReplayProof is no longer load-bearing! Expected \
             ReplayDivergence under concurrent OCEL mutation; got {:?}. \
             If you see this after refactoring `re_snapshot_ocel_for_replay_proof`, \
             A13 has regressed to a tautology. See file header for the \
             saboteur matrix and remediation steps.",
            other
        ),
    }
}
