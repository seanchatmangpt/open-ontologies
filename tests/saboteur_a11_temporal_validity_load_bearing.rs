//! R6 WA-2 — Saboteur matrix for §15 A11 TemporalValidity load-bearingness.
//!
//! This is a documentation-test marked `#[ignore]`. It is NOT part of the
//! standard `cargo make test` run — invoke manually with:
//!
//! ```bash
//! cargo test --test saboteur_a11_temporal_validity_load_bearing -- --ignored
//! ```
//!
//! Why this test exists
//! ====================
//!
//! Before R6 WA-2, `OntoStarAdmissionGate::evaluate` set:
//!
//! ```ignore
//! let granted_at_chain: Vec<String> = vec![chrono::Utc::now().to_rfc3339()];
//! ```
//!
//! A single-element Vec produces ZERO `windows(2)` — the A11 monotonicity
//! loop at `cell_ready.rs:367` was dead code.
//!
//! R6 WA-2 closes the tautology by introducing `re_read_granted_at_chain`,
//! which SELECTs `granted_at ORDER BY sequence ASC` from `receipts` for the
//! session's tenant and appends `Utc::now()`.
//!
//! Saboteur matrix
//! ===============
//!
//! Setup: land a FIRST admission to populate `receipts` with
//! `sequence=0, granted_at="2026-..."`. The hook fires BEFORE
//! `re_read_granted_at_chain`'s SELECT during the SECOND admission.
//!
//! With-fix (current):
//!   1. First admission writes `receipts` row: seq=0, granted_at="2026-...".
//!   2. Second scope opened on same session. Hook installed.
//!   3. Hook fires before SELECT; inserts row: seq=1, granted_at="2020-...".
//!   4. SELECT returns ORDER BY sequence ASC:
//!      ["2026-...", "2020-..."] — inverted at window (0,1).
//!   5. Caller appends Utc::now(); full chain still has inversion at (0,1).
//!   6. A11's `windows(2)` detects `w[0] > w[1]` → `TemporalSkew`.
//!   7. ✅ A11 caught the backdated receipt.
//!
//! Without-fix (pre-R6-WA-2, hypothetical sabotage):
//!   1–3. Same as above.
//!   4'. `granted_at_chain = vec![Utc::now()]` — DB not consulted at all.
//!   5'. Single-element chain, zero `windows(2)` — loop body never runs.
//!   6'. `Ok(receipt)` returned.
//!   7'. ❌ A11 was a tautology.
//!
//! Companion files
//! ===============
//! - `tests/cell_ready_a11_deny_path.rs` — unit-level deny-path proof.
//! - `src/admission.rs::re_read_granted_at_chain` — the fix.
//! - `src/cell_ready.rs:363` — the A11 gate.

use open_ontologies::admission::{
    self, AdmissionOp, ArtifactRef, NoopPowlReplay, OntoStarAdmissionGate,
};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

fn run_first_admission(db: &StateDb, store: &OcelStore, session: &str) {
    let scope = WorkflowScope::new(db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open first scope");
    scope.close(&token).expect("close first scope");
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
    let artifact = ArtifactRef { kind: "test", bytes: b"a11-first-bytes" };
    gate.evaluate(
        &token, AdmissionOp::Apply, &artifact, store,
        &NoopPowlReplay, session, workflow.powl_string, &observed,
        "default",
    )
    .expect("first admission must succeed to populate receipts table");
}

#[test]
#[ignore = "Documentation/saboteur test — run manually with --ignored to \
            confirm A11 TemporalValidity remains load-bearing"]
fn a11_temporal_validity_is_load_bearing_under_backdated_receipt() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("a11-saboteur.db");
    std::mem::forget(dir);
    let db = StateDb::open(&path).expect("open StateDb");
    let store = OcelStore::new(db.clone());
    let session = "a11-saboteur-session";

    // Land first admission so receipts table has one row:
    // sequence = 0, granted_at = "2026-..." (some real timestamp).
    run_first_admission(&db, &store, session);

    // Verify that the first admission wrote a receipt.
    let first_count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM receipts WHERE session_id = ?1",
            rusqlite::params![session],
            |r| r.get(0),
        )
        .unwrap_or(0);
    assert_eq!(first_count, 1, "first admission must write exactly one receipt row");

    // Open a second scope on the same session (same tenant chain).
    let scope = WorkflowScope::new(&db, session);
    let token2 = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open second scope");
    scope.close(&token2).expect("close second scope");
    for stage in &["load", "extend", "query"] {
        let now = chrono::Utc::now().to_rfc3339();
        let event_id = format!(
            "{}:{}:{}:2nd",
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
    let artifact = ArtifactRef { kind: "test", bytes: b"a11-second-bytes" };

    // ---- saboteur hook: insert a backdated receipt with higher sequence ----
    //
    // At hook-fire time, the receipts table has one row with sequence=0 and
    // granted_at="2026-...". We insert sequence=1, granted_at="2020-01-01".
    // The SELECT (ORDER BY sequence ASC) then returns:
    //   ["2026-...", "2020-01-01T00:00:00Z"]
    // The caller appends Utc::now(); the window (0,1) is inverted → TemporalSkew.
    let hook: Box<dyn Fn(&OcelStore, &str, &str) + Send + 'static> =
        Box::new(|store: &OcelStore, session_id: &str, tenant_id: &str| {
            let conn = store.db().conn();
            let max_seq: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(sequence), -1) FROM receipts \
                     WHERE session_id = ?1 AND tenant_id = ?2",
                    rusqlite::params![session_id, tenant_id],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap_or(-1);
            // Only inject when we have an existing row to invert against.
            if max_seq >= 0 {
                let fake_hash = format!("a11saboteur{:054x}", max_seq);
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO receipts (
                         receipt_hash, scope_token, artifact_hash,
                         declared_powl_hash, ocel_canonical_hash,
                         gate_config_hash, production_law_version,
                         granted_at, session_id, sequence, tenant_id
                     ) VALUES (?1, 'fake-scope', 'fake-artifact',
                               'fake-powl', 'fake-ocel', 'fake-gate',
                               'ontostar-1.0.0',
                               '2020-01-01T00:00:00Z',
                               ?2, ?3, ?4)",
                    rusqlite::params![
                        fake_hash,
                        session_id,
                        max_seq + 1,
                        tenant_id
                    ],
                );
            }
        });

    admission::A11_GRANTED_AT_REREAD_HOOK.with(|cell| {
        *cell.borrow_mut() = Some(hook);
    });
    let result = gate.evaluate(
        &token2, AdmissionOp::Apply, &artifact, &store,
        &NoopPowlReplay, session, workflow.powl_string, &observed,
        "default",
    );
    admission::A11_GRANTED_AT_REREAD_HOOK.with(|cell| {
        *cell.borrow_mut() = None;
    });

    match result {
        Err((DefectClass::TemporalSkew { .. }, _)) => {
            // ✅ A11 caught the backdated receipt.
        }
        other => panic!(
            "A11 TemporalValidity is no longer load-bearing! Expected TemporalSkew \
             under backdated receipt injection; got {:?}. \
             If you see this after refactoring `re_read_granted_at_chain`, A11 has \
             regressed to a tautology. See file header for the saboteur matrix.",
            other
        ),
    }
}
