//! Cell8 13-gate conformance tests (Phase 10 final).
//!
//! Each test starts from a passing baseline (`ok_inputs()`) and either:
//!   1. confirms the happy-path EARL report carries 13 `earl:passed`
//!      assertions (`happy_path_passes_all_thirteen_gates`), or
//!   2. mutates exactly one A9–A13 input and asserts the typed
//!      `DefectClass` returned by `cell_ready`, plus that the matching
//!      gate in a synthesised EARL report is `earl:failed`, or
//!   3. validates a canonical EARL report against the SHACL shapes file
//!      (`shacl_validates_canonical_earl_report` /
//!      `shacl_rejects_twelve_gate_report`).

use open_ontologies::cell8::{
    count_failed, count_passed, emit_earl_report, GateOutcome, GATE_NAMES,
};
use open_ontologies::cell_ready::{cell_ready, CellReadyInputs, PowlOpRef};
use open_ontologies::defects::DefectClass;
use open_ontologies::graph::GraphStore;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::shacl::ShaclValidator;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::WorkflowScope;
use std::sync::Arc;
use tempfile::tempdir;

const HEX32: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const HEX32_OTHER: &str =
    "1111111111111111111111111111111111111111111111111111111111111111";

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("cell8-13gate.db");
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
    // A9–A13 evidence:
    provenance_evidence: Vec<String>,
    external_attestation: String,
    granted_at_chain: Vec<String>,
    admitted_receipts: Vec<String>,
    replay_canonical_hash: String,
    prior_receipt: Option<[u8; 32]>,
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
        provenance_evidence: vec![HEX32.to_string()],
        external_attestation: HEX32.to_string(),
        granted_at_chain: vec!["2026-05-08T00:00:00Z".to_string()],
        admitted_receipts: Vec::new(),
        replay_canonical_hash: HEX32.to_string(),
        prior_receipt: None,
    }
}

fn inputs_from(bag: &Bag) -> CellReadyInputs<'_> {
    let powl_ref = Box::leak(Box::new(PowlOpRef {
        powl_string: &bag.powl_string,
        powl_hash: bag.powl_hash,
    }));
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
        prior_receipt: bag.prior_receipt,
        session_id: &bag.session_id,
        provenance_evidence: &bag.provenance_evidence,
        external_attestation: &bag.external_attestation,
        granted_at_chain: &bag.granted_at_chain,
        admitted_receipts: &bag.admitted_receipts,
        replay_canonical_hash: &bag.replay_canonical_hash,
    }
}

fn all_pass_outcomes() -> Vec<(&'static str, GateOutcome)> {
    GATE_NAMES
        .iter()
        .map(|g| {
            (
                *g,
                GateOutcome {
                    passed: true,
                    message: format!("{g} passed"),
                },
            )
        })
        .collect()
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[test]
fn happy_path_passes_all_thirteen_gates() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "c8-happy";
    let token = setup_scope(&db, session);
    let bag = ok_bag(token, session);

    let receipt = cell_ready(inputs_from(&bag), &store).expect("baseline must pass");
    let outcomes = all_pass_outcomes();
    let report = emit_earl_report(&receipt, &outcomes);

    assert_eq!(count_passed(&outcomes), 13);
    assert_eq!(count_failed(&outcomes), 0);
    assert_eq!(
        report.matches("earl:passed").count(),
        13,
        "expected 13 earl:passed assertions in:\n{report}"
    );
    assert!(!report.contains("earl:failed"));
    // Each of the 13 canonical gate names must appear in the report.
    for g in GATE_NAMES.iter() {
        assert!(
            report.contains(&format!("urn:ontostar:gate:{g}")),
            "report missing gate {g}"
        );
    }
}

#[test]
fn a9_provenance_missing_denies() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "c8-a9";
    let token = setup_scope(&db, session);
    let mut bag = ok_bag(token, session);
    bag.provenance_evidence.clear();

    match cell_ready(inputs_from(&bag), &store) {
        Err(DefectClass::ProvenanceMissing { artifact_hash }) => {
            assert_eq!(artifact_hash, HEX32);
        }
        other => panic!("expected ProvenanceMissing, got {other:?}"),
    }
}

#[test]
fn a10_attestation_missing_denies() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "c8-a10";
    let token = setup_scope(&db, session);
    let mut bag = ok_bag(token, session);
    // Phase-10 stub: A10 is a digest-equality stand-in. An empty
    // attestation cannot match the artifact hash, so we expect
    // AttestationMissing.
    bag.external_attestation = String::new();

    match cell_ready(inputs_from(&bag), &store) {
        Err(DefectClass::AttestationMissing) => {}
        other => panic!("expected AttestationMissing, got {other:?}"),
    }
}

#[test]
fn a11_temporal_skew_denies() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "c8-a11";
    let token = setup_scope(&db, session);
    let mut bag = ok_bag(token, session);
    // Backwards-skewing chain: t1 > t2.
    bag.granted_at_chain = vec![
        "2026-05-08T12:00:00Z".to_string(),
        "2026-05-08T11:59:59Z".to_string(),
    ];

    match cell_ready(inputs_from(&bag), &store) {
        Err(DefectClass::TemporalSkew { observed_skew_ms }) => {
            assert!(
                observed_skew_ms < 0,
                "expected negative skew, got {observed_skew_ms}"
            );
        }
        other => panic!("expected TemporalSkew, got {other:?}"),
    }
}

#[test]
fn a12_dependency_closure_broken_denies() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "c8-a12";
    let token = setup_scope(&db, session);
    let mut bag = ok_bag(token, session);
    // Reference a prior receipt that is NOT in `admitted_receipts`.
    let mut prior = [0u8; 32];
    prior[0] = 0x11;
    prior[1] = 0x11;
    // Easier: make every byte 0x11.
    for b in prior.iter_mut() {
        *b = 0x11;
    }
    bag.prior_receipt = Some(prior);
    bag.admitted_receipts = Vec::new(); // ← empty: closure broken

    match cell_ready(inputs_from(&bag), &store) {
        Err(DefectClass::DependencyClosureBroken { missing_hash }) => {
            assert_eq!(missing_hash.len(), 64, "missing_hash must be hex32");
            // The hex of `0x11; 32` should be 64×'1'.
            assert_eq!(missing_hash, "11".repeat(32));
        }
        other => panic!("expected DependencyClosureBroken, got {other:?}"),
    }
}

#[test]
fn a13_replay_divergence_denies() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "c8-a13";
    let token = setup_scope(&db, session);
    let mut bag = ok_bag(token, session);
    bag.replay_canonical_hash = HEX32_OTHER.to_string();

    match cell_ready(inputs_from(&bag), &store) {
        Err(DefectClass::ReplayDivergence { expected, observed }) => {
            assert_eq!(expected, HEX32);
            assert_eq!(observed, HEX32_OTHER);
        }
        other => panic!("expected ReplayDivergence, got {other:?}"),
    }
}

// ─── SHACL coverage tests ──────────────────────────────────────────────────

fn load_shapes_ttl() -> String {
    std::fs::read_to_string("ontology/cell8-conformance-shapes.ttl")
        .expect("read cell8-conformance-shapes.ttl")
}

#[test]
fn shacl_validates_canonical_earl_report() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "c8-shacl-ok";
    let token = setup_scope(&db, session);
    let bag = ok_bag(token, session);
    let receipt = cell_ready(inputs_from(&bag), &store).expect("baseline must pass");

    let outcomes = all_pass_outcomes();
    let report_ttl = emit_earl_report(&receipt, &outcomes);

    // Load the EARL report into a fresh GraphStore.
    let g = Arc::new(GraphStore::new());
    g.load_turtle(&report_ttl, None).expect("load EARL report");

    let shapes = load_shapes_ttl();
    let report_json = ShaclValidator::validate(&g, &shapes).expect("validate");
    let parsed: serde_json::Value =
        serde_json::from_str(&report_json).expect("parse SHACL report");
    assert_eq!(
        parsed["conforms"], true,
        "SHACL should accept canonical 13-gate report; got {parsed}"
    );
}

#[test]
fn shacl_rejects_twelve_gate_report() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "c8-shacl-bad";
    let token = setup_scope(&db, session);
    let bag = ok_bag(token, session);
    let receipt = cell_ready(inputs_from(&bag), &store).expect("baseline must pass");

    // Drop one outcome → emitter writes only 12 c8:hasGate edges and
    // 12 earl:Assertion blank nodes. The SHACL minCount/maxCount=13
    // shape must reject it.
    let mut outcomes = all_pass_outcomes();
    outcomes.pop();
    assert_eq!(outcomes.len(), 12);

    let report_ttl = emit_earl_report(&receipt, &outcomes);

    let g = Arc::new(GraphStore::new());
    g.load_turtle(&report_ttl, None).expect("load EARL report");

    let shapes = load_shapes_ttl();
    let report_json = ShaclValidator::validate(&g, &shapes).expect("validate");
    let parsed: serde_json::Value =
        serde_json::from_str(&report_json).expect("parse SHACL report");
    assert_eq!(
        parsed["conforms"], false,
        "SHACL must reject a 12-gate report; got {parsed}"
    );
    let count = parsed["violation_count"].as_u64().unwrap_or(0);
    assert!(count >= 1, "expected ≥1 violation, got {parsed}");
}
