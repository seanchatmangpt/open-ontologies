//! Shared fixtures for the §19 counterfactual evidence drift test layer.
//!
//! These helpers are extracted from `tests/cell_ready_deny_paths.rs` (the
//! `Bag` / `ok_bag_for` / `inputs_from_bag` / `setup_scope` shape that
//! drives the 13-conjunct cell_ready predicate) and from
//! `tests/real_swarm_e2e.rs::revops_scenario` (the BreedInput shape that
//! drives the 9-breed swarm consensus).
//!
//! Sub-modules in `tests/` are per-binary, so each `.rs` file under
//! `tests/` that wants these helpers declares its own `mod
//! cell_ready_fixtures;` (mirroring the existing `mod revops_common;`
//! pattern in `tests/revops_counterfactual.rs`). Cargo treats the
//! sibling `cell_ready_fixtures/` directory as the module body.
//!
//! Some helpers are unused by certain test binaries; that is expected and
//! the `dead_code` allow on this module is intentional.

#![allow(dead_code)]

use open_ontologies::cell_ready::{CellReadyInputs, PowlOpRef};
use open_ontologies::state::StateDb;
use open_ontologies::workflows::WorkflowScope;
use tempfile::tempdir;
use wasm4pm_cognition::breeds::{
    BreedInput, Candidate, Case, Fact, Goal, Rule, StateAtom,
};

/// 64-char lowercase hex string suitable for the `parse_hex32` parser
/// in `cell_ready.rs`. Reused across baseline and deny-path tests.
pub const HEX32: &str =
    "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

/// A second distinct hex32 used to provoke A13 ReplayDivergence. Same
/// length, different bytes — the deterministic equality check fails.
pub const HEX32_OTHER: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Open a fresh on-disk SQLite-backed StateDb in a temporary directory.
/// The tempdir is intentionally leaked: the OS reclaims it at process
/// exit and the file handle stays valid for the test's lifetime.
pub fn fresh_db() -> StateDb {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("cell-ready-fixture.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

/// Open + close a workflow scope and insert a conformance row so the
/// A4 POWLReplayPass conjunct passes for the returned scope token.
pub fn setup_scope(db: &StateDb, session: &str) -> String {
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

/// Open + close a scope WITHOUT inserting a conformance row. Used by
/// the A4 POWLReplayPass deny-path test.
pub fn setup_scope_without_replay(db: &StateDb, session: &str) -> String {
    let scope = WorkflowScope::new(db, session);
    let token = scope
        .open(None, Some("PO=(nodes={a, b}, order={a-->b})"), None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    token
}

/// Owning bag of every CellReadyInputs field, so deny-path tests mutate
/// one field at a time and the rest stay on the happy baseline.
pub struct Bag {
    pub scope_token: String,
    pub session_id: String,
    pub powl_string: String,
    pub powl_hash: [u8; 32],
    pub artifact_hash: String,
    pub ocel_trace_hash: String,
    pub gate_config_hash: String,
    pub fitness_observed: f64,
    pub precision_observed: f64,
    pub fitness_required: f64,
    pub precision_required: f64,
    pub required_stages: Vec<String>,
    pub observed_stages: Vec<String>,
    pub conformance_run_id: String,
    pub production_law_version: String,
    pub session_revoked: bool,
    pub provenance_evidence: Vec<String>,
    pub external_attestation: String,
    pub granted_at_chain: Vec<String>,
    pub admitted_receipts: Vec<String>,
    pub replay_canonical_hash: String,
}

/// Build a passing baseline `Bag` for the given scope token + session.
pub fn ok_bag_for(scope_token: String, session: &str) -> Bag {
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
        provenance_evidence: vec![HEX32.to_string()],
        external_attestation: HEX32.to_string(),
        granted_at_chain: vec!["2026-05-08T00:00:00Z".to_string()],
        admitted_receipts: Vec::new(),
        replay_canonical_hash: HEX32.to_string(),
    }
}

/// Build `CellReadyInputs` borrowing from a `Bag`. Slice fields are
/// `Box::leak`'d so the returned struct's lifetime is `'static` for the
/// borrowed slices — fine for a test scope.
pub fn inputs_from_bag(bag: &Bag) -> CellReadyInputs<'_> {
    let powl_ref = Box::leak(Box::new(PowlOpRef {
        powl_string: &bag.powl_string,
        powl_hash: bag.powl_hash,
    }));
    let provenance: &'static [String] =
        Box::leak(bag.provenance_evidence.clone().into_boxed_slice());
    let granted: &'static [String] =
        Box::leak(bag.granted_at_chain.clone().into_boxed_slice());
    let admitted: &'static [String] =
        Box::leak(bag.admitted_receipts.clone().into_boxed_slice());
    let attestation: &'static str =
        Box::leak(bag.external_attestation.clone().into_boxed_str());
    let replay_hash: &'static str =
        Box::leak(bag.replay_canonical_hash.clone().into_boxed_str());
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

/// Compact RevOps scenario suitable for swarm consensus saboteur
/// tests. Smaller than `tests/real_swarm_e2e.rs::revops_scenario` but
/// keeps the structural shape (multiple candidates, facts, rules,
/// goals, state) so Hearsay-II has real material to fuse over.
pub fn small_revops_scenario() -> BreedInput {
    BreedInput {
        intent:
            "RevOps revenue leakage detection across the booking pipeline at \
             Fortune-5 scale"
                .to_string(),
        candidates: vec![
            Candidate {
                id: "centralized-revenue-engine".into(),
                score: 0.5,
                eliminated: false,
                elimination_reason: None,
            },
            Candidate {
                id: "edge-distributed-reconciliation".into(),
                score: 0.5,
                eliminated: false,
                elimination_reason: None,
            },
        ],
        facts: vec![
            Fact { key: "scale".into(), value: "billion".into() },
            Fact { key: "leakage".into(), value: "detected".into() },
            Fact { key: "current".into(), value: "no-architecture".into() },
        ],
        cases: vec![Case {
            id: "case-rev-001".into(),
            intent: "Booking reconciliation gap".into(),
            architecture: "centralized-revenue-engine".into(),
            outcome_score: 0.92,
            facts: vec![Fact { key: "scale".into(), value: "billion".into() }],
        }],
        rules: vec![
            Rule {
                id: "r1".into(),
                premise: vec!["scale=billion".into()],
                conclusion: "favor=centralized-revenue-engine".into(),
                certainty: 0.9,
            },
            Rule {
                id: "establish-arch".into(),
                premise: vec!["current=no-architecture".into()],
                conclusion: "performance=high".into(),
                certainty: 1.0,
            },
        ],
        goals: vec![Goal {
            id: "g1".into(),
            predicate: "performance".into(),
            value: "high".into(),
        }],
        state: vec![StateAtom {
            predicate: "current".into(),
            value: "no-architecture".into(),
        }],
    }
}
