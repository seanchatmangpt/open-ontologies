//! Phase 3 — Fortune-5 RevOps CTQ admission tests.
//!
//! One test per CTQ in the test plan §5:
//!   CTQ-1 Forecast Trust       — supported by contract/order/milestone evidence
//!   CTQ-2 Booking Reconciliation — every booking traces invoice→order→contract
//!   CTQ-3 Renewal Risk         — required touchpoints completed by threshold
//!   CTQ-4 Partner Attribution  — partner_registered before contract_executed
//!   CTQ-5 No Raw Data Export   — restricted fields refused via RawDataLeak
//!
//! Each CTQ runs against a HappyPath OCEL trace and asserts the
//! deterministic CTQ admission gate produces a Receipt. The shapes of
//! the 5 CTQ field strings are derived from test plan §5 verbatim.

mod revops_common;

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, NoopPowlReplay, OntoStarAdmissionGate,
};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use revops_common::{
    booking_chain_is_reconciled, build_scenario, observed_events,
    partner_attribution_is_in_order, REQUIREMENTS_WORKFLOW, Scenario,
};
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ctq-admission.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn build_gate() -> OntoStarAdmissionGate {
    let required: Vec<String> = by_name(REQUIREMENTS_WORKFLOW)
        .map(|w| w.required_stages.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0")
}

/// Drive a CTQ admission. Emits the RequirementsManufacturing trace
/// (so observed_stages is satisfied), then evaluates the CTQ gate
/// against the supplied 5 CTQ field strings.
fn admit_ctq(
    store: &OcelStore,
    session: &str,
    scope: &str,
    source: &str,
    ctq: &str,
    measure: &str,
    verify: &str,
    neg: &str,
    control: &str,
) -> Result<open_ontologies::receipts::Receipt, (DefectClass, Vec<open_ontologies::defects::Deviation>)> {
    // Required-stages preflight emission.
    revops_common::emit(store, session, scope, "requirement_proposed",
        &[("source_voice", source)], &[]);
    revops_common::emit(store, session, scope, "llm_candidate_translated",
        &[("provisional", "true")], &[]);
    revops_common::emit(store, session, scope, "ctq_admitted", &[("ctq", ctq)], &[]);
    revops_common::emit(store, session, scope, "verification_bound", &[], &[]);
    revops_common::emit(store, session, scope, "negative_case_bound", &[], &[]);
    revops_common::emit(store, session, scope, "control_plan_bound", &[], &[]);
    revops_common::emit(store, session, scope, "work_order_admitted", &[], &[]);
    let observed: Vec<String> = store.observed_event_types_for_session(session).unwrap();
    let canonical = format!(
        "src\u{1f}{source}\u{1e}ctq\u{1f}{ctq}\u{1e}m\u{1f}{measure}\u{1e}v\u{1f}{verify}\u{1e}n\u{1f}{neg}\u{1e}c\u{1f}{control}",
    );
    let artifact = ArtifactRef { kind: "ctq", bytes: canonical.as_bytes() };
    let powl = by_name(REQUIREMENTS_WORKFLOW).unwrap().powl_string;
    let gate = build_gate();
    gate.evaluate(scope, AdmissionOp::CtqAdmitted, &artifact, store, &NoopPowlReplay,
        session, powl, &observed)
}

fn fresh_scope(session: &str) -> (StateDb, OcelStore, String) {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let scope_mgr = WorkflowScope::new(&db, session);
    let token = scope_mgr.open(Some(REQUIREMENTS_WORKFLOW), None, None).unwrap();
    scope_mgr.close(&token).unwrap();
    (db, store, token)
}

// ── CTQ-1 Forecast Trust ────────────────────────────────────────────────────

#[test]
fn ctq_1_forecast_trust_admits_when_chain_complete() {
    let (_db, store, scope) = fresh_scope("ctq-1");
    build_scenario(&store, "ctq-1", &scope, Scenario::HappyPath);

    let receipt = admit_ctq(
        &store, "ctq-1", &scope,
        "Executives do not trust the pipeline forecast",
        "Forecast risk must be explainable from process evidence",
        "Percentage of forecasted revenue with complete supporting evidence",
        "Forecast risk report identifies unsupported revenue and links each risk to evidence",
        "If supporting evidence is missing, refuse to mark forecast as trusted",
        "Block forecast trust claim when contract_executed missing for any committed opportunity",
    ).expect("CTQ-1 must admit on HappyPath");
    assert_eq!(receipt.record.production_law_version, "ontostar-1.0.0");
    // Smoke: chain reconciliation truth held by the trace.
    let evts = observed_events(&store, "ctq-1");
    assert!(booking_chain_is_reconciled(&evts), "happy path chain should reconcile");
}

// ── CTQ-2 Booking Reconciliation ───────────────────────────────────────────

#[test]
fn ctq_2_booking_reconciliation_admits_when_chain_complete() {
    let (_db, store, scope) = fresh_scope("ctq-2");
    build_scenario(&store, "ctq-2", &scope, Scenario::HappyPath);

    let receipt = admit_ctq(
        &store, "ctq-2", &scope,
        "Finance says bookings do not reconcile",
        "Every booked amount must reconcile to contract, order, invoice, and revenue milestone evidence",
        "Reconciliation completeness rate",
        "Each booking is classified as reconciled, partially reconciled, or refused",
        "A booking with invoice evidence but no contract/order chain must be denied as complete",
        "Block booking_complete classification unless every prior chain event present",
    ).expect("CTQ-2 must admit on HappyPath");
    assert_eq!(receipt.record.defects_taxonomy_version, open_ontologies::defects::DEFECTS_TAXONOMY_VERSION);
    let evts = observed_events(&store, "ctq-2");
    assert!(booking_chain_is_reconciled(&evts));
}

// ── CTQ-3 Renewal Risk ─────────────────────────────────────────────────────

#[test]
fn ctq_3_renewal_risk_admits_when_touchpoints_present() {
    let (_db, store, scope) = fresh_scope("ctq-3");
    build_scenario(&store, "ctq-3", &scope, Scenario::HappyPath);

    let receipt = admit_ctq(
        &store, "ctq-3", &scope,
        "Customer Success says renewals are late",
        "Renewal risk must be detected before deadline based on process-motion evidence",
        "Percentage of renewals with required pre-renewal touchpoints completed by threshold date",
        "System identifies renewals missing required touchpoints and emits risk findings",
        "A renewal cannot be marked healthy if required touchpoints are absent",
        "Block renewal_healthy classification when touchpoint events absent at threshold",
    ).expect("CTQ-3 must admit on HappyPath");
    assert!(receipt.record.scope_token == scope);
}

// ── CTQ-4 Partner Attribution ──────────────────────────────────────────────

#[test]
fn ctq_4_partner_attribution_admits_when_in_order() {
    let (_db, store, scope) = fresh_scope("ctq-4");
    build_scenario(&store, "ctq-4", &scope, Scenario::HappyPath);

    let receipt = admit_ctq(
        &store, "ctq-4", &scope,
        "Partner Ops says attribution is wrong",
        "Partner attribution must be supported by ordered evidence across lead, registration, opportunity, quote, and contract stages",
        "Attribution evidence completeness and ordering correctness",
        "Incorrect or late partner attribution is flagged",
        "A partner cannot receive attribution when partner_registered occurs after contract_executed",
        "Block partner_attributed when partner_registered timestamp >= contract_executed timestamp",
    ).expect("CTQ-4 must admit on HappyPath");
    assert!(!receipt.hex().is_empty());
    let evts = observed_events(&store, "ctq-4");
    assert!(partner_attribution_is_in_order(&evts), "happy path partner ordering should be correct");
}

// ── CTQ-5 No Raw Data Export ───────────────────────────────────────────────

#[test]
fn ctq_5_no_raw_data_export_admits_with_clean_evidence() {
    let (_db, store, scope) = fresh_scope("ctq-5");
    build_scenario(&store, "ctq-5", &scope, Scenario::HappyPath);

    let receipt = admit_ctq(
        &store, "ctq-5", &scope,
        "Executives want weekly revenue-risk view without exporting restricted raw data",
        "RevOps intelligence must operate on fake or reduced object-centric events, not raw customer payloads",
        "Zero restricted raw-data fields in exported evidence/projections",
        "Outputs contain only allowed fake/reduced fields",
        "Any raw-data-like payload triggers refusal with RawDataLeak{field}",
        "Token-overlap check on every projection rejects fields not on allowlist",
    ).expect("CTQ-5 must admit on HappyPath");
    // HappyPath has no raw-email-shaped attribute — no leak.
    let evts = observed_events(&store, "ctq-5");
    let serialized = serde_json::to_string(&evts).unwrap();
    assert!(
        !serialized.contains("@fortune5.example.com"),
        "happy path must not contain raw-email-shaped attribute"
    );
    assert_eq!(receipt.record.scope_token, scope);
}
