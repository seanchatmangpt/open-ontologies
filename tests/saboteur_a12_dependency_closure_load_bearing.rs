//! R6 WA-3 — Saboteur matrix for §15 A12 DependencyClosure load-bearingness.
//!
//! This is a documentation-test marked `#[ignore]`. It is NOT part of the
//! standard `cargo make test` run — invoke manually with:
//!
//! ```bash
//! cargo test --test saboteur_a12_dependency_closure_load_bearing -- --ignored
//! ```
//!
//! Why this test exists
//! ====================
//!
//! Before R6 WA-3, `OntoStarAdmissionGate::evaluate` constructed:
//!
//! ```ignore
//! let admitted_receipts: Vec<String> = match prior_receipt.as_ref() {
//!     Some(h) => vec![hex32_pub(h)],
//!     None => Vec::new(),
//! };
//! ```
//!
//! Both `inp.prior_receipt` and `inp.admitted_receipts[0]` came from the
//! same `Option<[u8;32]>` — A12's check was `[X].contains(X)`, a
//! structural tautology. Deleting the prior receipt from the DB had zero
//! effect on the gate outcome.
//!
//! R6 WA-3 closes the tautology by introducing `re_read_admitted_receipts`,
//! which does a `SELECT receipt_hash FROM receipts WHERE receipt_hash = ?1
//! AND tenant_id = ?2`. If the row is absent, the helper returns empty and
//! A12 denies with `DependencyClosureBroken`.
//!
//! Saboteur matrix
//! ===============
//!
//! With-fix (current):
//!   1. Run a full admission to land a receipt in the `receipts` table.
//!   2. Open a second scope, run admission again so `prior_receipt` is Some.
//!   3. Hook fires BEFORE `re_read_admitted_receipts`'s SELECT.
//!   4. Hook DELETEs the prior receipt row from `receipts`.
//!   5. Helper returns empty Vec.
//!   6. A12: `iter().any(...)` returns false → `DependencyClosureBroken`.
//!   7. ✅ A12 caught the missing prior receipt.
//!
//! Without-fix (pre-R6-WA-3, hypothetical sabotage):
//!   1–4. Same as above.
//!   5'. `admitted_receipts = vec![hex(prior_receipt)]` — DB not consulted.
//!   6'. `[X].contains(X)` → gate passes regardless of DB state.
//!   7'. ❌ A12 was a tautology; the deleted row was undetectable.
//!
//! Companion files
//! ===============
//! - `tests/cell_ready_a12_deny_path.rs` — unit-level deny-path proof.
//! - `src/admission.rs::re_read_admitted_receipts` — the fix.
//! - `src/cell_ready.rs:376` — the A12 gate.

use open_ontologies::admission::{
    self, AdmissionOp, ArtifactRef, NoopPowlReplay, OntoStarAdmissionGate,
};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

/// Performs a complete admission run and returns the session's tenant +
/// receipt hash that was written, so the caller can reference it.
fn run_first_admission(
    db: &StateDb,
    store: &OcelStore,
    session: &str,
) -> String {
    let scope = WorkflowScope::new(db, session);
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
    let required: Vec<String> = workflow.required_stages.iter().map(|s| s.to_string()).collect();
    let gate = OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0");
    let artifact = ArtifactRef { kind: "test", bytes: b"a12-first-admission" };
    let _receipt = gate
        .evaluate(
            &token,
            AdmissionOp::Apply,
            &artifact,
            store,
            &NoopPowlReplay,
            session,
            workflow.powl_string,
            &observed,
            "default",
        )
        .expect("first admission must succeed");
    // Return the receipt hash hex so the saboteur can target it.
    let conn = db.conn();
    conn.query_row(
        "SELECT receipt_hash FROM receipts WHERE scope_token = ?1",
        rusqlite::params![token],
        |r| r.get::<_, String>(0),
    )
    .unwrap_or_default()
}

#[test]
#[ignore = "Documentation/saboteur test — run manually with --ignored to \
            confirm A12 DependencyClosure remains load-bearing"]
fn a12_dependency_closure_is_load_bearing_under_deleted_prior_receipt() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("a12-saboteur.db");
    std::mem::forget(dir);
    let db = StateDb::open(&path).expect("open StateDb");
    let store = OcelStore::new(db.clone());

    // Land first admission to populate the receipts table.
    let prior_hash = run_first_admission(&db, &store, "a12-saboteur-session-1");
    assert!(!prior_hash.is_empty(), "first admission must produce a receipt");

    // Now open a second scope on the SAME session so `prior_receipt` is Some.
    let session = "a12-saboteur-session-1"; // same session → same chain
    let scope = WorkflowScope::new(&db, session);
    let token2 = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open second scope");
    scope.close(&token2).expect("close second scope");
    for stage in &["load", "extend", "query"] {
        let now = chrono::Utc::now().to_rfc3339();
        let event_id = format!(
            "{}:{}:{}",
            session,
            stage,
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        store
            .emit_event(&event_id, stage, &now, session, &[], &[], Some(&token2))
            .unwrap();
    }
    let observed = store.observed_event_types_for_session(session).unwrap();
    let workflow = by_name("DataExtensionFastPath").expect("workflow lookup");
    let required: Vec<String> = workflow.required_stages.iter().map(|s| s.to_string()).collect();
    let gate = OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0");
    let artifact = ArtifactRef { kind: "test", bytes: b"a12-second-admission" };

    // ---- saboteur hook: DELETE the prior receipt row ----
    let hook: Box<dyn Fn(&OcelStore, &str, &str) + Send + 'static> =
        Box::new(|store: &OcelStore, prior_hex: &str, tenant_id: &str| {
            let conn = store.db().conn();
            let _ = conn.execute(
                "DELETE FROM receipts WHERE receipt_hash = ?1 AND tenant_id = ?2",
                rusqlite::params![prior_hex, tenant_id],
            );
        });

    admission::A12_ADMITTED_RECEIPTS_REREAD_HOOK.with(|cell| {
        *cell.borrow_mut() = Some(hook);
    });
    let result = gate.evaluate(
        &token2,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &NoopPowlReplay,
        session,
        workflow.powl_string,
        &observed,
        "default",
    );
    admission::A12_ADMITTED_RECEIPTS_REREAD_HOOK.with(|cell| {
        *cell.borrow_mut() = None;
    });

    match result {
        Err((DefectClass::DependencyClosureBroken { missing_hash }, _)) => {
            assert_eq!(
                missing_hash, prior_hash,
                "A12 must report the correct missing receipt hash"
            );
            // ✅ A12 caught the deleted prior receipt.
        }
        other => panic!(
            "A12 DependencyClosure is no longer load-bearing! Expected \
             DependencyClosureBroken after deleting prior receipt; got {:?}. \
             If you see this after refactoring `re_read_admitted_receipts`, \
             A12 has regressed to a tautology. See file header.",
            other
        ),
    }
}
