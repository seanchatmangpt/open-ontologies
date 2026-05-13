//! R6 WA-2 — §15 A11 TemporalValidity tautology closure: deny-path proof.
//!
//! Before this round, `OntoStarAdmissionGate::evaluate` constructed
//! `granted_at_chain = vec![chrono::Utc::now().to_rfc3339()]` — a
//! single-element Vec. A11's gate at `src/cell_ready.rs:367`:
//!
//! ```ignore
//! for w in inp.granted_at_chain.windows(2) {
//!     if w[0] > w[1] {
//!         return Err(DefectClass::TemporalSkew { observed_skew_ms });
//!     }
//! }
//! ```
//!
//! `vec![x].windows(2)` produces ZERO windows — the loop body was dead
//! code and A11 could never fail regardless of timestamps.
//!
//! R6 WA-2 introduces `re_read_granted_at_chain` which queries prior
//! `granted_at` values from `receipts ORDER BY sequence ASC` then
//! appends `Utc::now()`, giving a multi-element chain that `windows(2)`
//! can actually iterate.
//!
//! This file proves the deny path at the `cell_ready` unit level — no
//! admission flow needed. It directly constructs `CellReadyInputs` with
//! an out-of-order `granted_at_chain` and asserts `TemporalSkew`.
//!
//! Companion: `tests/saboteur_a11_temporal_validity_load_bearing.rs`

use open_ontologies::cell_ready::{cell_ready, CellReadyInputs, PowlOpRef};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::WorkflowScope;
use tempfile::tempdir;

const HEX32: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn fresh_db() -> StateDb {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("a11-deny-path.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn setup_scope(db: &StateDb, session: &str) -> String {
    let scope = WorkflowScope::new(db, session);
    let token = scope
        .open(None, Some("PO=(nodes={a, b}, order={a-->b})"), None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    let conn = db.conn();
    conn.execute(
        "INSERT INTO conformance_runs (
             run_id, scope_token, fitness, precision,
             generalization, simplicity, verdict, defects_json,
             trace_canonical_hash, ran_at
         ) VALUES (?1, ?2, 0.99, 0.99, NULL, NULL, 'conform', '[]', ?3, ?4)",
        rusqlite::params![
            format!("run-{}", token),
            &token,
            HEX32,
            chrono::Utc::now().to_rfc3339(),
        ],
    )
    .expect("insert conformance row");
    token
}

#[allow(clippy::too_many_arguments)]
fn build_inputs<'a>(
    scope_token: &'a str,
    session_id: &'a str,
    powl_string: &'a str,
    powl_hash: [u8; 32],
    artifact_hash_hex: &'a str,
    provenance_evidence: &'a [String],
    granted_at_chain: &'a [String],
    admitted_receipts: &'a [String],
) -> CellReadyInputs<'a> {
    let powl_ref = Box::leak(Box::new(PowlOpRef {
        powl_string,
        powl_hash,
    }));
    let attestation: &'static str = Box::leak(artifact_hash_hex.to_string().into_boxed_str());
    let required_stages: &'static [String] =
        Box::leak(vec!["a".to_string(), "b".to_string()].into_boxed_slice());
    let observed_stages: &'static [String] =
        Box::leak(vec!["a".to_string(), "b".to_string()].into_boxed_slice());
    let run_id: &'static str = Box::leak(format!("run-{}", scope_token).into_boxed_str());
    CellReadyInputs {
        scope_token,
        declared_powl: powl_ref,
        ocel_trace_hash: HEX32,
        artifact_hash: artifact_hash_hex,
        gate_config_hash: HEX32,
        session_revoked: false,
        fitness_observed: 0.99,
        precision_observed: 0.99,
        fitness_required: 0.95,
        precision_required: 0.85,
        required_stages,
        observed_stages,
        conformance_run_id: run_id,
        production_law_version: "ontostar-1.0.0",
        prior_receipt: None,
        session_id,
        provenance_evidence,
        external_attestation: attestation,
        granted_at_chain,
        admitted_receipts,
        replay_canonical_hash: HEX32,
        signature: None,
        signing_key_fpr: None,
        trusted_keys: None,
        allow_legacy_unsigned: true,
        trusted_keys_db: None,
        post_bootstrap: false,
        prior_tenant_receipt_count: 0,
    }
}

/// A11 denies when granted_at_chain has an out-of-order pair.
#[test]
fn a11_deny_on_out_of_order_granted_at_chain() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "a11-deny-out-of-order-session";
    let token = setup_scope(&db, session);
    let powl_string = "PO=(nodes={a, b}, order={a-->b})";
    let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
    let artifact_hash = HEX32.to_string();
    let evidence: Vec<String> = vec![artifact_hash.clone()];
    let admitted: Vec<String> = Vec::new();

    // Inverted: index 0 is later than index 1.
    let granted: Vec<String> = vec![
        "2026-05-09T12:00:00Z".to_string(),
        "2026-05-09T11:00:00Z".to_string(),
    ];

    let inputs = build_inputs(
        &token, session, powl_string, powl_hash,
        &artifact_hash, &evidence, &granted, &admitted,
    );
    match cell_ready(inputs, &store) {
        Err(DefectClass::TemporalSkew { observed_skew_ms }) => {
            assert!(
                observed_skew_ms < 0 || observed_skew_ms == -1,
                "inverted chain should yield negative skew or sentinel; got {observed_skew_ms}"
            );
        }
        other => panic!(
            "expected TemporalSkew on out-of-order granted_at_chain; got {:?}",
            other
        ),
    }
}

/// A11 denies when the chain is empty.
#[test]
fn a11_deny_on_empty_granted_at_chain() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "a11-deny-empty-chain-session";
    let token = setup_scope(&db, session);
    let powl_string = "PO=(nodes={a, b}, order={a-->b})";
    let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
    let artifact_hash = HEX32.to_string();
    let evidence: Vec<String> = vec![artifact_hash.clone()];
    let admitted: Vec<String> = Vec::new();
    let granted: Vec<String> = Vec::new();

    let inputs = build_inputs(
        &token, session, powl_string, powl_hash,
        &artifact_hash, &evidence, &granted, &admitted,
    );
    match cell_ready(inputs, &store) {
        Err(DefectClass::TemporalSkew { .. }) => {}
        other => panic!(
            "expected TemporalSkew on empty granted_at_chain; got {:?}", other
        ),
    }
}

/// A11 passes on a strictly monotonic chain.
#[test]
fn a11_pass_on_monotonic_granted_at_chain() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "a11-pass-monotonic-session";
    let token = setup_scope(&db, session);
    let powl_string = "PO=(nodes={a, b}, order={a-->b})";
    let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
    let artifact_hash = HEX32.to_string();
    let evidence: Vec<String> = vec![artifact_hash.clone()];
    let admitted: Vec<String> = Vec::new();

    let granted: Vec<String> = vec![
        "2026-05-09T10:00:00Z".to_string(),
        "2026-05-09T11:00:00Z".to_string(),
        "2026-05-09T12:00:00Z".to_string(),
    ];

    let inputs = build_inputs(
        &token, session, powl_string, powl_hash,
        &artifact_hash, &evidence, &granted, &admitted,
    );
    let result = cell_ready(inputs, &store);
    if let Err(DefectClass::TemporalSkew { .. }) = result {
        panic!(
            "A11 must NOT fail with TemporalSkew on a monotonic chain; got {:?}", result
        );
    }
}
