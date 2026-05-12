//! R6 WA-3 — §15 A12 DependencyClosure tautology closure: deny-path proof.
//!
//! Before this round, `admitted_receipts = vec![hex(prior_receipt)]` was
//! derived from the same `Option<[u8;32]>` as `prior_receipt` — A12's
//! check was `[X].contains(X)` by construction. Deleting the prior receipt
//! from the DB had zero effect on the gate outcome.
//!
//! R6 WA-3 introduces `re_read_admitted_receipts` which does a
//! `SELECT receipt_hash FROM receipts WHERE receipt_hash = prior_hex AND
//! tenant_id = scope_tenant`. If the row is absent the helper returns
//! empty and A12 denies with `DependencyClosureBroken`.
//!
//! This file proves the deny path at the `cell_ready` unit level by
//! passing `admitted_receipts: &[]` (empty) with a non-None `prior_receipt`.
//!
//! Companion: `tests/saboteur_a12_dependency_closure_load_bearing.rs`

use open_ontologies::cell_ready::{cell_ready, CellReadyInputs, PowlOpRef};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::WorkflowScope;
use tempfile::tempdir;

const HEX32: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn fresh_db() -> StateDb {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("a12-deny-path.db");
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

fn build_inputs_with_prior<'a>(
    scope_token: &'a str,
    session_id: &'a str,
    powl_string: &'a str,
    powl_hash: [u8; 32],
    artifact_hash_hex: &'a str,
    provenance_evidence: &'a [String],
    granted_at_chain: &'a [String],
    admitted_receipts: &'a [String],
    prior_receipt: Option<[u8; 32]>,
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
        prior_receipt,
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
    }
}

/// A12 denies when prior_receipt is Some but admitted_receipts is empty.
#[test]
fn a12_deny_when_prior_receipt_not_in_admitted_set() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "a12-deny-missing-prior-session";
    let token = setup_scope(&db, session);
    let powl_string = "PO=(nodes={a, b}, order={a-->b})";
    let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
    let artifact_hash = HEX32.to_string();
    let evidence: Vec<String> = vec![artifact_hash.clone()];
    let granted: Vec<String> = vec!["2026-05-09T10:00:00Z".to_string()];
    let admitted: Vec<String> = Vec::new(); // empty — prior row absent
    let prior: [u8; 32] = [0xaa; 32];

    let inputs = build_inputs_with_prior(
        &token, session, powl_string, powl_hash,
        &artifact_hash, &evidence, &granted, &admitted, Some(prior),
    );
    match cell_ready(inputs, &store) {
        Err(DefectClass::DependencyClosureBroken { missing_hash }) => {
            let expected_hex: String = prior.iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(
                missing_hash, expected_hex,
                "A12 must report the correct missing receipt hash"
            );
        }
        other => panic!(
            "expected DependencyClosureBroken when prior_receipt absent from admitted_receipts; \
             got {:?}",
            other
        ),
    }
}

/// A12 passes when admitted_receipts contains the prior hex.
#[test]
fn a12_pass_when_prior_receipt_in_admitted_set() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "a12-pass-admitted-session";
    let token = setup_scope(&db, session);
    let powl_string = "PO=(nodes={a, b}, order={a-->b})";
    let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
    let artifact_hash = HEX32.to_string();
    let evidence: Vec<String> = vec![artifact_hash.clone()];
    let granted: Vec<String> = vec!["2026-05-09T10:00:00Z".to_string()];
    let prior: [u8; 32] = [0xbb; 32];
    let prior_hex: String = prior.iter().map(|b| format!("{b:02x}")).collect();
    let admitted: Vec<String> = vec![prior_hex];

    let inputs = build_inputs_with_prior(
        &token, session, powl_string, powl_hash,
        &artifact_hash, &evidence, &granted, &admitted, Some(prior),
    );
    let result = cell_ready(inputs, &store);
    if let Err(DefectClass::DependencyClosureBroken { .. }) = result {
        panic!(
            "A12 must NOT fail when prior_receipt is present in admitted_receipts; got {:?}",
            result
        );
    }
}

/// A12 passes on None prior_receipt (bootstrap path).
#[test]
fn a12_pass_on_none_prior_receipt() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "a12-pass-none-prior-session";
    let token = setup_scope(&db, session);
    let powl_string = "PO=(nodes={a, b}, order={a-->b})";
    let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
    let artifact_hash = HEX32.to_string();
    let evidence: Vec<String> = vec![artifact_hash.clone()];
    let granted: Vec<String> = vec!["2026-05-09T10:00:00Z".to_string()];
    let admitted: Vec<String> = Vec::new();

    let inputs = build_inputs_with_prior(
        &token, session, powl_string, powl_hash,
        &artifact_hash, &evidence, &granted, &admitted, None,
    );
    let result = cell_ready(inputs, &store);
    if let Err(DefectClass::DependencyClosureBroken { .. }) = result {
        panic!(
            "A12 must NOT fail with DependencyClosureBroken when prior_receipt is None; \
             got {:?}",
            result
        );
    }
}
