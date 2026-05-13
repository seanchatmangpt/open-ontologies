//! R6 WA-1 — Saboteur matrix for §15 A9 ProvenanceChain load-bearingness.
//!
//! This is a documentation-test marked `#[ignore]`. It is NOT part of the
//! standard `cargo make test` run — it exists to be invoked manually with
//! `cargo test --test saboteur_a9_provenance_chain_load_bearing -- --ignored`
//! when the maintainer wants a face-to-face sanity check that the §15 A9
//! ProvenanceChain gate is still load-bearing after future refactors.
//!
//! Why this test exists
//! ====================
//!
//! Before R6 WA-1, `OntoStarAdmissionGate::evaluate` constructed
//! `provenance_evidence: Vec<String> = vec![artifact_hash_hex.clone()]`
//! at `src/admission.rs:663` and passed THAT same value as the SOLE
//! input to the A9 gate predicate at `src/cell_ready.rs:200-206`:
//!
//! ```ignore
//! if inp.provenance_evidence.is_empty()
//!     || !inp.provenance_evidence.iter().any(|p| p == inp.artifact_hash)
//! {
//!     return Err(DefectClass::ProvenanceMissing { ... });
//! }
//! ```
//!
//! `[X].contains(X)` is vacuously true by construction — A9 was a
//! tautology. R5 WB-1 closed the structural twin (A13 ReplayProof);
//! R5 WB-1's TODO comment at admission.rs:660-662 carried A9 forward
//! as `caller-trust-burden`. R6 WA-1 closes A9.
//!
//! R6 WA-1 introduces:
//!   1. A new OCEL emit at `admission.rs:617+`
//!      (`event_type='artifact_generated'`, `attrs.artifact_hash =
//!      artifact_hash_hex`) — non-atomic with the receipt by design;
//!      this is a gauge anchor, not a proof object.
//!   2. `re_read_provenance_evidence(store, session, artifact_hash) ->
//!      Vec<String>` — an INDEPENDENT SELECT over the OCEL store. Empty
//!      result → A9 fails closed with `ProvenanceMissing`.
//!   3. `A9_PROVENANCE_REREAD_HOOK` `thread_local`, gated
//!      `#[cfg(debug_assertions)]`, fires BEFORE the helper's SELECT so
//!      tests can inject synthetic mutations (DELETE the witness row)
//!      without flaky timing.
//!
//! Saboteur matrix
//! ===============
//!
//! With-fix (current main):
//!   1. Open & close a `DataExtensionFastPath` scope.
//!   2. Emit `load → extend → query` events.
//!   3. Open `OntoStarAdmissionGate::evaluate`.
//!   4. Admission emits `artifact_generated` row with attrs.artifact_hash.
//!   5. Hook fires BEFORE the helper's SELECT and DELETEs the row.
//!   6. Helper's SELECT returns 0 rows → empty Vec.
//!   7. cell_ready's A9 predicate sees `provenance_evidence.is_empty() ==
//!      true` → returns `Err(DefectClass::ProvenanceMissing { artifact_hash
//!      })`.
//!   8. ✅ A9 caught the missing witness.
//!
//! Without-fix (pre-R6-WA-1, hypothetical sabotage):
//!   1–4. Same as above, but no `artifact_generated` emit happens.
//!   5'. The struct literal hands the gate
//!       `provenance_evidence: vec![artifact_hash_hex.clone()]`. The
//!       hook's row-DELETE has no effect (the gate doesn't read OCEL).
//!   6'. `iter().any(|p| p == artifact_hash)` is true by construction.
//!   7'. cell_ready returns `Ok(receipt)` — A9 silently passed.
//!   8'. ❌ A9 was a tautology; the missing witness was undetectable.
//!
//! How to verify the fix is load-bearing manually
//! ===============================================
//!
//! 1. Run this binary with `--ignored`:
//!    ```bash
//!    cargo test --test saboteur_a9_provenance_chain_load_bearing -- --ignored
//!    ```
//!    The active assertion below MUST pass — proving the deny path fires
//!    after the fix.
//!
//! 2. To prove the OLD code was a tautology:
//!    a. In `src/admission.rs`, restore the line
//!       `let provenance_evidence: Vec<String> =
//!       vec![artifact_hash_hex.clone()];`
//!    b. Comment out the helper call.
//!    c. Re-run this test → it MUST FAIL because admission now grants
//!       even with the witness row deleted (tautology restored).
//!    d. Revert the sabotage. The test passes again.
//!
//! 3. To prove the helper is the only thing producing denial:
//!    a. Remove the `with_a9_hook(...)` wrapper in this test.
//!    b. Re-run → admission GRANTS (no concurrent mutation; the
//!       `artifact_generated` row is durable). This is expected: A9
//!       only fails when the witness row is missing, exactly as the
//!       gate is designed.
//!
//! Companion files
//! ===============
//! - `tests/cell_ready_a9_deny_path.rs` — the deterministic deny-path
//!   proof at the cell_ready unit level (no admission flow needed).
//! - `src/admission.rs::re_read_provenance_evidence` — the helper.
//! - `src/admission.rs::A9_PROVENANCE_REREAD_HOOK` — the test hook.
//! - `src/cell_ready.rs:200-206` — the A9 equality check.
//! - `src/defects.rs::DefectClass::ProvenanceMissing` — the typed defect.

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
            confirm A9 ProvenanceChain remains load-bearing"]
fn a9_provenance_chain_is_load_bearing_under_witness_deletion() {
    // ---- setup ----
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("a9-saboteur.db");
    std::mem::forget(dir);
    let db = StateDb::open(&path).expect("open StateDb");
    let store = OcelStore::new(db.clone());
    let session = "a9-saboteur-session";
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
    // Use stable artifact bytes — the BLAKE3 hash is deterministic so
    // we can construct the artifact_event_id ourselves to DELETE.
    let artifact_bytes = b"a9-saboteur-bytes" as &[u8];
    let artifact = ArtifactRef {
        kind: "test",
        bytes: artifact_bytes,
    };

    // ---- saboteur hook: DELETE the artifact_generated row(s) before
    //                     the helper's SELECT ----
    //
    // The hook fires inside `re_read_provenance_evidence` BEFORE the
    // helper's SQL prepare. We DELETE the OCEL event rows joined to
    // attrs where event_type='artifact_generated' AND session_id=?1
    // AND attr_name='artifact_hash' AND attr_value=?2. Both the
    // event_attrs row(s) and the parent event row are removed so the
    // helper's INNER JOIN returns zero rows.
    let saboteur_db = db.clone();
    let hook: Box<dyn Fn(&OcelStore, &str, &str) + Send + 'static> =
        Box::new(move |_store: &OcelStore, session: &str, artifact_hash_hex: &str| {
            let conn = saboteur_db.conn();
            // Delete the attrs first (foreign-key safety).
            conn.execute(
                "DELETE FROM ocel_event_attrs WHERE event_id IN (
                    SELECT e.event_id FROM ocel_events e
                    INNER JOIN ocel_event_attrs a ON a.event_id = e.event_id
                    WHERE e.event_type = 'artifact_generated'
                      AND e.session_id = ?1
                      AND a.name = 'artifact_hash'
                      AND a.value = ?2
                 )",
                rusqlite::params![session, artifact_hash_hex],
            )
            .expect("saboteur DELETE attrs");
            // Then delete the parent event row(s).
            conn.execute(
                "DELETE FROM ocel_events
                 WHERE event_type = 'artifact_generated'
                   AND session_id = ?1
                   AND event_id LIKE 'artifact_generated:%'",
                rusqlite::params![session],
            )
            .expect("saboteur DELETE events");
        });

    admission::A9_PROVENANCE_REREAD_HOOK.with(|cell| {
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
        "default",
    );
    admission::A9_PROVENANCE_REREAD_HOOK.with(|cell| {
        *cell.borrow_mut() = None;
    });

    // ---- assertion: A9 caught the missing witness ----
    let expected_artifact_hash = {
        let bytes = *blake3::hash(artifact_bytes).as_bytes();
        let mut s = String::with_capacity(64);
        for b in bytes.iter() {
            use std::fmt::Write as _;
            let _ = write!(&mut s, "{:02x}", b);
        }
        s
    };
    match result {
        Err((DefectClass::ProvenanceMissing { artifact_hash }, _)) => {
            assert_eq!(
                artifact_hash, expected_artifact_hash,
                "A9 deny must carry the caller's artifact_hash_hex; got {artifact_hash}"
            );
        }
        other => panic!(
            "A9 ProvenanceChain is no longer load-bearing! Expected \
             ProvenanceMissing under witness-deletion sabotage; got {:?}. \
             If you see this after refactoring `re_read_provenance_evidence`, \
             A9 has regressed to a tautology. See file header for the \
             saboteur matrix and remediation steps.",
            other
        ),
    }
}
