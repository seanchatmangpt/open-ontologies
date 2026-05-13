//! R6 WA-1 — §15 A9 ProvenanceChain tautology closure: deny-path proof.
//!
//! Before this round, `OntoStarAdmissionGate::evaluate` constructed
//! `provenance_evidence: Vec<String> = vec![artifact_hash_hex.clone()]`
//! at `src/admission.rs:663` and passed THAT same value as
//! `CellReadyInputs::provenance_evidence`. The A9 gate predicate at
//! `src/cell_ready.rs:200-206`:
//!
//! ```ignore
//! if inp.provenance_evidence.is_empty()
//!     || !inp.provenance_evidence.iter().any(|p| p == inp.artifact_hash)
//! {
//!     return Err(DefectClass::ProvenanceMissing { ... });
//! }
//! ```
//!
//! was vacuously true by construction — `[X].contains(X)` always holds,
//! so the conjunct could never fail. This was a structural twin to the
//! §15 A13 ReplayProof tautology that R5 WB-1 closed (commit 791d9e0).
//!
//! R6 WA-1 introduces `re_read_provenance_evidence` —
//! an INDEPENDENT SELECT against `ocel_events` filtered by
//! `event_type='artifact_generated'` AND session_id AND artifact_hash.
//! The new `artifact_generated` OCEL anchor in `admission.rs` provides
//! the witness rows. If the witness row is missing (emit failure,
//! sabotage), the helper returns an empty Vec and A9 denies with
//! `DefectClass::ProvenanceMissing`.
//!
//! This file proves the deny path at the `cell_ready` unit level — no
//! admission flow needed. It directly constructs `CellReadyInputs` with
//! `provenance_evidence: &[]` and asserts `ProvenanceMissing`.
//!
//! Companion file: `tests/saboteur_a9_provenance_chain_load_bearing.rs`
//! drives the FULL admission gate end-to-end with the
//! `A9_PROVENANCE_REREAD_HOOK` to delete the witness row mid-flight.

use open_ontologies::cell_ready::{cell_ready, CellReadyInputs, PowlOpRef};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::WorkflowScope;
use tempfile::tempdir;

const HEX32: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn fresh_db() -> StateDb {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("a9-deny-path.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

/// Set up a scope that satisfies all the upstream conjuncts (workflow
/// declared, scope closed, conformance run row) so A9 is the FIRST
/// failing predicate when its evidence vector is empty.
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
    let production_law: &'static str = "ontostar-1.0.0";
    let run_id: &'static str = Box::leak(format!("run-{}", scope_token).into_boxed_str());
    let gate_config_hash: &'static str = HEX32;
    CellReadyInputs {
        scope_token,
        declared_powl: powl_ref,
        ocel_trace_hash: HEX32,
        artifact_hash: artifact_hash_hex,
        gate_config_hash,
        session_revoked: false,
        fitness_observed: 0.99,
        precision_observed: 0.99,
        fitness_required: 0.95,
        precision_required: 0.85,
        required_stages,
        observed_stages,
        conformance_run_id: run_id,
        production_law_version: production_law,
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

/// The load-bearing deny-path test for R6 WA-1.
///
/// Construct `CellReadyInputs` with an EMPTY `provenance_evidence` slice
/// (the exact shape `re_read_provenance_evidence` returns when the
/// witness row is missing — emit failure or sabotage DELETE). The A9
/// predicate at `cell_ready.rs:200-206` MUST refuse with
/// `DefectClass::ProvenanceMissing { artifact_hash }` carrying the
/// caller's artifact_hash_hex.
#[test]
fn a9_deny_when_provenance_evidence_empty() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "a9-deny-empty-session";
    let token = setup_scope(&db, session);
    let powl_string = "PO=(nodes={a, b}, order={a-->b})";
    let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
    let artifact_hash = HEX32.to_string();
    let granted: Vec<String> = vec!["2026-05-09T00:00:00Z".to_string()];
    let admitted: Vec<String> = Vec::new();

    // Empty witness vector — exactly what the helper returns when the
    // OCEL `artifact_generated` row is missing.
    let evidence: Vec<String> = Vec::new();

    let inputs = build_inputs(
        &token,
        session,
        powl_string,
        powl_hash,
        &artifact_hash,
        &evidence,
        &granted,
        &admitted,
    );
    match cell_ready(inputs, &store) {
        Err(DefectClass::ProvenanceMissing { artifact_hash: got }) => {
            assert_eq!(
                got, artifact_hash,
                "A9 deny must carry the caller's artifact_hash_hex; got {got}"
            );
        }
        other => panic!(
            "expected ProvenanceMissing on empty provenance_evidence; got {:?}",
            other
        ),
    }
}

/// Sanity check: when `provenance_evidence` includes the artifact hash
/// (the shape `re_read_provenance_evidence` returns when the
/// `artifact_generated` row is durable), A9 passes — admission can
/// progress to A10/A11/etc. This test does NOT assert overall success
/// (later conjuncts have their own concerns); it asserts that
/// `ProvenanceMissing` is NOT the failure mode.
#[test]
fn a9_pass_when_provenance_evidence_includes_artifact_hash() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "a9-pass-session";
    let token = setup_scope(&db, session);
    let powl_string = "PO=(nodes={a, b}, order={a-->b})";
    let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
    let artifact_hash = HEX32.to_string();
    let granted: Vec<String> = vec!["2026-05-09T00:00:00Z".to_string()];
    let admitted: Vec<String> = Vec::new();

    // Witness vector contains the artifact hash — the gate's predicate
    // `iter().any(|p| p == artifact_hash)` returns true.
    let evidence: Vec<String> = vec![artifact_hash.clone()];

    let inputs = build_inputs(
        &token,
        session,
        powl_string,
        powl_hash,
        &artifact_hash,
        &evidence,
        &granted,
        &admitted,
    );
    let result = cell_ready(inputs, &store);
    if let Err(DefectClass::ProvenanceMissing { .. }) = result {
        panic!(
            "A9 must NOT fail with ProvenanceMissing when evidence contains \
             artifact_hash; got {:?}",
            result
        );
    }
    // Either Ok(receipt) or some non-A9 failure mode is acceptable — the
    // sanity assertion is purely that A9's predicate is NOT the offender.
}
