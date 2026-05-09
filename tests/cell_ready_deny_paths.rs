//! Deny-path coverage for `cell_ready::cell_ready`.
//!
//! Each test starts from a passing baseline (`ok_inputs()`), mutates one
//! field, and asserts the typed `DefectClass` returned. The baseline must
//! itself pass — verified in `cell_ready_ok_baseline_passes`.

use open_ontologies::cell_ready::{cell_ready, CellReadyInputs, PowlOpRef};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::WorkflowScope;
use tempfile::tempdir;

const HEX32: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("cell-ready-test.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

/// Set up a scope that satisfies the WorkflowDeclared / ScopeClosed /
/// POWLReplayPass conjuncts and return its token.
fn setup_scope(db: &StateDb, session: &str) -> String {
    let scope = WorkflowScope::new(db, session);
    let token = scope
        .open(None, Some("PO=(nodes={a, b}, order={a-->b})"), None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    // Insert a conforming run for the scope so `replay_pass` returns true.
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

struct Bag {
    scope_token: String,
    session_id: String,
    powl_string: String,
    powl_hash: [u8; 32],
    artifact_hash: String,
    ocel_trace_hash: String,
    gate_config_hash: String,
    fitness_observed: f64,
    precision_observed: f64,
    fitness_required: f64,
    precision_required: f64,
    required_stages: Vec<String>,
    observed_stages: Vec<String>,
    conformance_run_id: String,
    production_law_version: String,
    session_revoked: bool,
}

fn ok_bag(scope_token: String, session: &str) -> Bag {
    let powl_string = "PO=(nodes={a, b}, order={a-->b})".to_string();
    let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
    Bag {
        scope_token,
        session_id: session.to_string(),
        powl_string,
        powl_hash,
        artifact_hash: HEX32.to_string(),
        ocel_trace_hash: HEX32.to_string(),
        gate_config_hash: HEX32.to_string(),
        fitness_observed: 0.99,
        precision_observed: 0.99,
        fitness_required: 0.95,
        precision_required: 0.85,
        required_stages: vec!["a".into(), "b".into()],
        observed_stages: vec!["a".into(), "b".into()],
        conformance_run_id: "run-test".into(),
        production_law_version: "ontostar-1.0.0".into(),
        session_revoked: false,
    }
}

fn happy_provenance(bag: &Bag) -> Vec<String> {
    vec![bag.artifact_hash.clone()]
}

fn happy_granted_at() -> Vec<String> {
    vec!["2026-05-08T00:00:00Z".to_string()]
}

fn inputs_from(bag: &Bag) -> CellReadyInputs<'_> {
    // Construct PowlOpRef with a leaked reference (test scope, fine).
    let powl_ref = Box::leak(Box::new(PowlOpRef {
        powl_string: &bag.powl_string,
        powl_hash: bag.powl_hash,
    }));
    // Phase-10 13-conjunct evidence — leak so we can hand out borrowed slices.
    let provenance: &'static [String] = Box::leak(happy_provenance(bag).into_boxed_slice());
    let granted: &'static [String] = Box::leak(happy_granted_at().into_boxed_slice());
    let admitted: &'static [String] = Box::leak(Vec::<String>::new().into_boxed_slice());
    let attestation: &'static str = Box::leak(bag.artifact_hash.clone().into_boxed_str());
    let replay_hash: &'static str = Box::leak(bag.ocel_trace_hash.clone().into_boxed_str());
    CellReadyInputs {
        scope_token: &bag.scope_token,
        declared_powl: powl_ref,
        ocel_trace_hash: &bag.ocel_trace_hash,
        artifact_hash: &bag.artifact_hash,
        gate_config_hash: &bag.gate_config_hash,
        session_revoked: bag.session_revoked,
        fitness_observed: bag.fitness_observed,
        precision_observed: bag.precision_observed,
        fitness_required: bag.fitness_required,
        precision_required: bag.precision_required,
        required_stages: &bag.required_stages,
        observed_stages: &bag.observed_stages,
        conformance_run_id: &bag.conformance_run_id,
        production_law_version: &bag.production_law_version,
        prior_receipt: None,
        session_id: &bag.session_id,
        provenance_evidence: provenance,
        external_attestation: attestation,
        granted_at_chain: granted,
        admitted_receipts: admitted,
        replay_canonical_hash: replay_hash,
        signature: None,
        signing_key_fpr: None,
        trusted_keys: None,
        allow_legacy_unsigned: true,
        trusted_keys_db: None,
    }
}

#[test]
fn cell_ready_ok_baseline_passes() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "cr-baseline";
    let token = setup_scope(&db, session);
    let bag = ok_bag(token, session);
    cell_ready(inputs_from(&bag), &store).expect("baseline must pass");
}

#[test]
fn cell_ready_threshold_failed_on_low_fitness() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "cr-fitness";
    let token = setup_scope(&db, session);
    let mut bag = ok_bag(token, session);
    bag.fitness_observed = 0.50;
    bag.fitness_required = 0.95;

    match cell_ready(inputs_from(&bag), &store) {
        Err(DefectClass::ThresholdFailed { metric, observed, required }) => {
            assert_eq!(metric, "fitness");
            assert!((observed - 0.50).abs() < 1e-9);
            assert!((required - 0.95).abs() < 1e-9);
        }
        other => panic!("expected ThresholdFailed{{fitness}}, got {other:?}"),
    }
}

#[test]
fn cell_ready_receipt_missing_on_bad_artifact_hash() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "cr-bad-hash";
    let token = setup_scope(&db, session);
    let mut bag = ok_bag(token, session);
    bag.artifact_hash = "not-a-hex32".into();

    match cell_ready(inputs_from(&bag), &store) {
        Err(DefectClass::ReceiptMissing) => {}
        other => panic!("expected ReceiptMissing, got {other:?}"),
    }
}

#[test]
fn cell_ready_scope_unclosed_when_close_skipped() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "cr-unclosed";
    // Open but DO NOT close.
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(None, Some("PO=(nodes={a}, order={})"), None)
        .expect("open scope");
    let bag = ok_bag(token, session);

    match cell_ready(inputs_from(&bag), &store) {
        Err(DefectClass::ScopeUnclosed) => {}
        other => panic!("expected ScopeUnclosed, got {other:?}"),
    }
}

// ─── Phase R4 WB additions: A3, A4, A5-precision deny paths ──────────────
//
// These three tests close the §19 counterfactual evidence drift gap on
// the cell_ready conjuncts whose deny paths were previously only
// indirectly exercised. Each has a Δ>0 PROOF comment explaining what
// naked-craft would have shipped.

#[test]
fn cell_ready_ocel_incomplete_when_no_observed_stages() {
    // Δ>0 PROOF: naked craft would build a "manufacturing run" with an
    //            empty event log and ship it as success because the
    //            outer admission gate only spot-checks artifact_hash.
    //            The manufacturing line refuses with OcelIncomplete
    //            because `cell_ready::ocel_complete()` requires at
    //            least one observed event.
    //            Pinned production line: src/cell_ready.rs:135-138
    //            (the A3 OCELComplete branch returning OcelIncomplete).
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "cr-ocel-incomplete";
    let token = setup_scope(&db, session);
    let mut bag = ok_bag(token, session);
    // Empty observed_stages triggers A3 before A6 RequiredStagesPresent
    // (which is checked later in the conjunct order). Required must
    // also be empty so A6 cannot tower over the A3 branch in error
    // message attribution.
    bag.observed_stages.clear();
    bag.required_stages.clear();

    match cell_ready(inputs_from(&bag), &store) {
        Err(DefectClass::OcelIncomplete) => {}
        other => panic!("expected OcelIncomplete, got {other:?}"),
    }
}

#[test]
fn cell_ready_replay_failed_when_no_conforming_run() {
    // Δ>0 PROOF: naked craft would persist a Receipt for a scope that
    //            never had a conforming POWL replay, since the receipt
    //            hash only covers the artifact and OCEL trace, not the
    //            replay verdict. The manufacturing line refuses because
    //            the A4 conjunct calls `OcelStore::has_conforming_replay`,
    //            which checks `conformance_runs.verdict='conform'`. A
    //            scope with NO conformance row falls through.
    //            Pinned production line: src/cell_ready.rs:140-143
    //            (the A4 POWLReplayPass branch returning ReplayFailed).
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "cr-no-replay";
    // Open + close the scope but skip the conformance_runs INSERT so
    // `replay_pass` returns false.
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(None, Some("PO=(nodes={a, b}, order={a-->b})"), None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    let bag = ok_bag(token, session);

    match cell_ready(inputs_from(&bag), &store) {
        Err(DefectClass::ReplayFailed) => {}
        other => panic!("expected ReplayFailed, got {other:?}"),
    }
}

#[test]
fn cell_ready_threshold_failed_on_low_precision() {
    // Δ>0 PROOF: naked craft, having gauged fitness alone, would ship
    //            a model that hit fitness 0.99 but precision 0.40 — a
    //            classic process-mining over-fit (the model accepts
    //            far too many traces). The manufacturing line refuses
    //            because A5 ThresholdPass tests BOTH metrics and
    //            short-circuits on the precision branch when fitness
    //            already passed.
    //            Pinned production line: src/cell_ready.rs:153-159
    //            (the precision-branch ThresholdFailed return).
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "cr-precision";
    let token = setup_scope(&db, session);
    let mut bag = ok_bag(token, session);
    // Fitness passes; precision falls below required.
    bag.fitness_observed = 0.99;
    bag.fitness_required = 0.95;
    bag.precision_observed = 0.40;
    bag.precision_required = 0.85;

    match cell_ready(inputs_from(&bag), &store) {
        Err(DefectClass::ThresholdFailed { metric, observed, required }) => {
            assert_eq!(
                metric, "precision",
                "must short-circuit on precision branch, got metric={metric}"
            );
            assert!((observed - 0.40).abs() < 1e-9);
            assert!((required - 0.85).abs() < 1e-9);
        }
        other => panic!("expected ThresholdFailed{{precision}}, got {other:?}"),
    }
}
