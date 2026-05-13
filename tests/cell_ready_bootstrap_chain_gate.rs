//! R8-1 — Bootstrap chain-length gate: deny-path proof.
//!
//! After `bootstrap_lock` is set (production mode), `granted_at_chain.len() < 2`
//! must be denied with [`DefectClass::BootstrapChainTooShort`]. During the
//! bootstrap window a single-entry chain is acceptable.
//!
//! Three load-bearing tests:
//!
//! 1. `bootstrap_mode_allows_chain_length_1` — bootstrap window open, chain = [now],
//!    gate must pass (not BootstrapChainTooShort).
//! 2. `post_bootstrap_denies_chain_length_1` — `post_bootstrap = true`, chain = [now],
//!    gate MUST deny with `BootstrapChainTooShort`.
//! 3. `post_bootstrap_admits_chain_length_2` — `post_bootstrap = true`, chain = [t0, now],
//!    gate must pass (chain is long enough).

use open_ontologies::cell_ready::{cell_ready, CellReadyInputs, PowlOpRef};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::WorkflowScope;
use tempfile::tempdir;

const HEX32: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const POWL_STR: &str = "PO=(nodes={a, b}, order={a-->b})";

fn fresh_store() -> (StateDb, OcelStore) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("bootstrap-chain-gate.db");
    std::mem::forget(dir);
    let db = StateDb::open(&path).expect("open StateDb");
    let store = OcelStore::new(db.clone());
    (db, store)
}

fn setup_scope(db: &StateDb, session: &str) -> String {
    let scope = WorkflowScope::new(db, session);
    let token = scope
        .open(None, Some(POWL_STR), None)
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

fn build_inputs<'a>(
    scope_token: &'a str,
    session_id: &'a str,
    granted_at_chain: &'a [String],
    post_bootstrap: bool,
    prior_tenant_receipt_count: usize,
) -> CellReadyInputs<'a> {
    let mut powl_hash = [0u8; 32];
    powl_hash[0] = 0xab;
    let powl_ref = Box::leak(Box::new(PowlOpRef {
        powl_string: POWL_STR,
        powl_hash,
    }));
    let required_stages: &'static [String] =
        Box::leak(vec!["a".to_string(), "b".to_string()].into_boxed_slice());
    let observed_stages: &'static [String] =
        Box::leak(vec!["a".to_string(), "b".to_string()].into_boxed_slice());
    let run_id: &'static str = Box::leak(format!("run-{}", scope_token).into_boxed_str());
    let provenance: &'static [String] =
        Box::leak(vec![HEX32.to_string()].into_boxed_slice());
    let admitted: &'static [String] = Box::leak(Vec::<String>::new().into_boxed_slice());
    CellReadyInputs {
        scope_token,
        declared_powl: powl_ref,
        ocel_trace_hash: HEX32,
        artifact_hash: HEX32,
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
        provenance_evidence: provenance,
        external_attestation: "",
        granted_at_chain,
        admitted_receipts: admitted,
        replay_canonical_hash: HEX32,
        signature: None,
        signing_key_fpr: None,
        trusted_keys: None,
        allow_legacy_unsigned: true,
        trusted_keys_db: None,
        post_bootstrap,
        prior_tenant_receipt_count,
    }
}

/// 1. Bootstrap window open → single-entry chain allowed (A11 has nothing to
///    compare; gate passes the chain-length check because `post_bootstrap = false`).
#[test]
fn bootstrap_mode_allows_chain_length_1() {
    let (_db, store) = fresh_store();
    let db = store.db().clone();
    let session = "sess-bootstrap-chain-1";
    let scope_token = setup_scope(&db, session);

    let chain = vec![chrono::Utc::now().to_rfc3339()];
    // prior_tenant_receipt_count=0 → bootstrap window applies to both old and new tenants.
    let inputs = build_inputs(&scope_token, session, &chain, false, 0);
    let result = cell_ready(inputs, &store);
    assert!(
        !matches!(result, Err(DefectClass::BootstrapChainTooShort)),
        "bootstrap window must NOT raise BootstrapChainTooShort; got {result:?}"
    );
}

/// 2. Production mode (post_bootstrap = true) with chain length 1 and
///    prior_tenant_receipt_count > 0 → DENIED.
///    This is the load-bearing negative path: proves the gate is not
///    bypassed when `bootstrap_lock` is active and the tenant has prior history.
///    (A new tenant with prior_tenant_receipt_count=0 is allowed its first receipt
///    even in post-bootstrap; the denial case requires a known existing tenant.)
#[test]
fn post_bootstrap_denies_chain_length_1() {
    let (_db, store) = fresh_store();
    let db = store.db().clone();
    let session = "sess-post-bootstrap-chain-1";
    let scope_token = setup_scope(&db, session);

    let chain = vec![chrono::Utc::now().to_rfc3339()];
    // prior_tenant_receipt_count=1: simulates an existing tenant whose chain
    // suspiciously returned only 1 entry (e.g., history tampered with).
    let inputs = build_inputs(&scope_token, session, &chain, true, 1);
    let result = cell_ready(inputs, &store);
    assert!(
        matches!(result, Err(DefectClass::BootstrapChainTooShort)),
        "post-bootstrap single-entry chain for existing tenant MUST raise BootstrapChainTooShort; got {result:?}"
    );
}

/// 2b. Production mode (post_bootstrap = true) with chain length 1 but
///     prior_tenant_receipt_count=0 → ALLOWED.
///     A genuinely new tenant entering a post-bootstrap DB gets its first
///     receipt — the bootstrap lock is DB-wide, not per-tenant.
#[test]
fn post_bootstrap_new_tenant_chain_length_1_allowed() {
    let (_db, store) = fresh_store();
    let db = store.db().clone();
    let session = "sess-post-bootstrap-new-tenant";
    let scope_token = setup_scope(&db, session);

    let chain = vec![chrono::Utc::now().to_rfc3339()];
    // prior_tenant_receipt_count=0: this tenant has never admitted before.
    let inputs = build_inputs(&scope_token, session, &chain, true, 0);
    let result = cell_ready(inputs, &store);
    assert!(
        !matches!(result, Err(DefectClass::BootstrapChainTooShort)),
        "post-bootstrap new tenant (prior_count=0) must NOT raise BootstrapChainTooShort; got {result:?}"
    );
}

/// 3. Production mode (post_bootstrap = true) with chain length 2 → ADMITTED.
///    Proves the boundary: exactly 2 entries (seed + in-flight) is sufficient.
#[test]
fn post_bootstrap_admits_chain_length_2() {
    let (_db, store) = fresh_store();
    let db = store.db().clone();
    let session = "sess-post-bootstrap-chain-2";
    let scope_token = setup_scope(&db, session);

    let t0 = "2026-05-12T00:00:00Z".to_string();
    let t1 = chrono::Utc::now().to_rfc3339();
    let chain = vec![t0, t1];
    let inputs = build_inputs(&scope_token, session, &chain, true, 1);
    let result = cell_ready(inputs, &store);
    assert!(
        !matches!(result, Err(DefectClass::BootstrapChainTooShort)),
        "post-bootstrap chain length 2 must NOT raise BootstrapChainTooShort; got {result:?}"
    );
}
