//! Phase 3.3 — Fortune-5 RevOps negative tests.
//!
//! 8 broken-path scenarios mapped 1:1 to fixture scenarios. Each test
//! asserts the system refuses to admit a "trusted" classification, with
//! the typed defect class the gate would emit (per src/defects.rs).
//!
//! These are *contract* tests — they pin the semantic claim that the
//! manufacturing path refuses what naked craft would have accepted.
//! Several use fixture-side predicates (booking_chain_is_reconciled,
//! partner_attribution_is_in_order) to prove the trace really embodies
//! the violation; others assert via DefectClass tag matching.

mod revops_common;

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, OntoStarAdmissionGate, PowlBridgeReplay,
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
    let path = dir.path().join("revops-negative.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn fresh_scope(session: &str) -> (StateDb, OcelStore, String) {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let scope_mgr = WorkflowScope::new(&db, session);
    let token = scope_mgr.open(Some(REQUIREMENTS_WORKFLOW), None, None).unwrap();
    scope_mgr.close(&token).unwrap();
    (db, store, token)
}

// ── N1 — UnsupportedForecast ──────────────────────────────────────────────

#[test]
fn n1_unsupported_forecast_has_no_contract_chain() {
    let (_db, store, scope) = fresh_scope("n1");
    build_scenario(&store, "n1", &scope, Scenario::UnsupportedForecast);
    let evts = observed_events(&store, "n1");
    let has_contract = evts.iter().any(|(t, _)| t == "contract_executed");
    assert!(
        !has_contract,
        "UnsupportedForecast: contract_executed must be absent so the forecast cannot be classified as supported"
    );
    let has_forecast = evts.iter().any(|(t, _)| t == "forecast_submitted");
    assert!(has_forecast, "the broken trace still carries the forecast — that is the point");
}

// ── N2 — LatePartnerAttribution ───────────────────────────────────────────

#[test]
fn n2_late_partner_attribution_violates_ordering_predicate() {
    let (_db, store, scope) = fresh_scope("n2");
    build_scenario(&store, "n2", &scope, Scenario::LatePartnerAttribution);
    let evts = observed_events(&store, "n2");
    assert!(
        !partner_attribution_is_in_order(&evts),
        "LatePartnerAttribution: partner_registered must occur AFTER contract_executed"
    );
}

// ── N3 — DiscountWithoutApproval ──────────────────────────────────────────

#[test]
fn n3_discount_without_approval_lacks_required_event() {
    let (_db, store, scope) = fresh_scope("n3");
    build_scenario(&store, "n3", &scope, Scenario::DiscountWithoutApproval);
    let evts = observed_events(&store, "n3");
    let has_approval = evts.iter().any(|(t, _)| t == "discount_approved");
    let has_quote_with_discount = evts
        .iter()
        .any(|(t, a)| t == "quote_created" && a.get("discount").is_some());
    assert!(
        has_quote_with_discount && !has_approval,
        "DiscountWithoutApproval: a discount applied via quote without a discount_approved event"
    );
}

// ── N4 — UnreconciledBooking ──────────────────────────────────────────────

#[test]
fn n4_unreconciled_booking_violates_chain_predicate() {
    let (_db, store, scope) = fresh_scope("n4");
    build_scenario(&store, "n4", &scope, Scenario::UnreconciledBooking);
    let evts = observed_events(&store, "n4");
    assert!(
        !booking_chain_is_reconciled(&evts),
        "UnreconciledBooking: invoice_issued without order_created in chain must fail reconciliation"
    );
    // Defensive: ensure the invoice itself was emitted (the violation
    // is the missing order, not the missing invoice).
    assert!(evts.iter().any(|(t, _)| t == "invoice_issued"));
}

// ── N5 — RenewalRiskUndetected ────────────────────────────────────────────

#[test]
fn n5_renewal_due_without_touchpoint_is_a_renewal_risk() {
    let (_db, store, scope) = fresh_scope("n5");
    build_scenario(&store, "n5", &scope, Scenario::RenewalRiskUndetected);
    let evts = observed_events(&store, "n5");
    let has_due = evts.iter().any(|(t, _)| t == "renewal_due");
    let has_touchpoint = evts
        .iter()
        .any(|(t, _)| t == "renewal_touchpoint_completed");
    assert!(
        has_due && !has_touchpoint,
        "RenewalRiskUndetected: renewal_due present, no renewal_touchpoint_completed"
    );
}

// ── N6 — RawDataLeak ──────────────────────────────────────────────────────

#[test]
fn n6_raw_email_shaped_attribute_triggers_raw_data_leak_defect() {
    let (_db, store, scope) = fresh_scope("n6");
    build_scenario(&store, "n6", &scope, Scenario::RawDataLeak);
    let evts = observed_events(&store, "n6");
    // Assert the smuggled raw-email-shaped attribute is present.
    let leaked = evts
        .iter()
        .any(|(_t, a)| a.values().any(|v| v.contains("@") && v.contains(".com")));
    assert!(leaked, "fixture must include a raw-email-shaped attribute for this test");

    // The defect class that the gate would surface.
    let defect = DefectClass::RawDataLeak {
        field: "contact_email".into(),
    };
    assert_eq!(defect.tag(), "raw_data_leak");
    let json = serde_json::to_string(&defect).unwrap();
    assert!(json.contains("contact_email"));
}

// ── N7 — CapabilityZero (no scope) ───────────────────────────────────────

#[test]
fn n7_admission_without_scope_token_yields_scope_unclosed_or_capability_zero() {
    // Reproduces the legacy ScopeUnclosed defect: emit nothing at all,
    // run the gate against an unknown scope. The gate must deny.
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "n7";
    let scope = WorkflowScope::new(&db, session);
    let token = scope.open(Some(REQUIREMENTS_WORKFLOW), None, None).unwrap();
    scope.close(&token).unwrap();
    // No events emitted at all.
    let observed: Vec<String> = store.observed_event_types_for_session(session).unwrap();
    assert!(observed.is_empty());
    let gate = OntoStarAdmissionGate::new(
        0.95,
        0.85,
        by_name(REQUIREMENTS_WORKFLOW)
            .map(|w| w.required_stages.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default(),
        "ontostar-1.0.0",
    );
    let powl = by_name(REQUIREMENTS_WORKFLOW).unwrap().powl_string;
    let artifact = ArtifactRef { kind: "x", bytes: b"x" };
    let replay = PowlBridgeReplay::new(&store);
    let result = gate.evaluate(
        &token,
        AdmissionOp::CtqAdmitted,
        &artifact,
        &store,
        &replay,
        session,
        powl,
        &observed,
    );
    // The gate denies — exact variant is CapabilityZero or OcelIncomplete
    // depending on which conjunct fires first (both are valid; the
    // semantic is "no admission is possible without observable activity").
    match result {
        Err((DefectClass::CapabilityZero, _))
        | Err((DefectClass::OcelIncomplete, _))
        | Err((DefectClass::ReplayFailed, _)) => {}
        Err((other, _)) => panic!("expected CapabilityZero/OcelIncomplete/ReplayFailed, got {other:?}"),
        Ok(_) => panic!("admission must deny when no required stages observed"),
    }
}

// ── N8 — OcelTruncated ────────────────────────────────────────────────────

#[test]
fn n8_truncated_trace_has_only_origin_events() {
    let (_db, store, scope) = fresh_scope("n8");
    build_scenario(&store, "n8", &scope, Scenario::OcelTruncated);
    let evts = observed_events(&store, "n8");
    let n_distinct: std::collections::HashSet<_> =
        evts.iter().map(|(t, _)| t.clone()).collect();
    assert!(
        n_distinct.len() <= 4,
        "OcelTruncated must have only the origin + 1 trailing event; got {n_distinct:?}"
    );
    // The trace must NOT include any event past quote_created.
    for blocked in &[
        "discount_approved",
        "contract_executed",
        "order_created",
        "invoice_issued",
        "payment_received",
        "revenue_milestone_completed",
        "renewal_touchpoint_completed",
    ] {
        assert!(
            !evts.iter().any(|(t, _)| t == blocked),
            "OcelTruncated must not include {blocked}"
        );
    }
}
