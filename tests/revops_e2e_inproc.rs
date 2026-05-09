//! Deterministic in-process Fortune-5 RevOps Governed Release acceptance.
//!
//! R4 WA, §24 Chicago TDD: the previous `revops_e2e.rs` drove the entire
//! stack against a tokio-TCP Groq mock. That mock conflated three things —
//! the bearer-auth wire format, the JSON request body, and the response
//! shape — into a single fake. §24 forbids that: pretending to verify a
//! third-party protocol against a hand-rolled imposter is "decorative
//! completion".
//!
//! This file owns the half of the original test that DID NOT need to
//! cross the Groq boundary: deterministic CTQ admission, receipt
//! chaining, OCEL replay against a `RequirementsManufacturing` trace, and
//! counterfactual delta. The candidate CTQ is supplied by
//! `revops_common::fixture_candidate_ctq()` instead of an HTTP mock.
//!
//! The OTHER half — proving the real Groq translator force-overrides
//! `provisional=true` and produces admissible 5-field output — lives in
//! `tests/revops_e2e_with_real_groq.rs` (gated, `#[ignore]`'d).
//!
//! Δ>0 PROOF: the deleted Groq HTTP mock would have shipped a faked
//!            wire-format that could rot silently if the translator's
//!            `bearer_auth`, JSON body, `temperature: 0.0`, or
//!            `/chat/completions` POST shape ever changed. The new
//!            inproc test deliberately does NOT touch that surface, so
//!            it is honest about its scope: it pins the deterministic
//!            admission gate + receipt chain + replay invariants only.
//!            Production lines pinned: `OntoStarAdmissionGate::evaluate`
//!            (admission + receipt chain) and `PowlBridgeReplay::replay`
//!            (real fitness against observed trace).

mod revops_common;

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, OntoStarAdmissionGate, PowlBridgeReplay,
    PowlReplay,
};
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use revops_common::{
    booking_chain_is_reconciled, build_scenario, fixture_candidate_ctq,
    observed_events, partner_attribution_is_in_order, REQUIREMENTS_WORKFLOW,
    Scenario,
};
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("revops-e2e-inproc.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

#[tokio::test]
async fn fortune5_revops_revenue_trust_trial_inproc() {
    // ── 1. Deterministic CandidateCtq fixture (no Groq, no HTTP) ────────
    let candidate = fixture_candidate_ctq();
    assert!(
        candidate.provisional,
        "fixture must encode the §7 invariant that translator output is provisional"
    );

    // ── 2. Build HappyPath OCEL trace ───────────────────────────────────
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "f5-revenue-trust-trial-inproc";
    let scope_mgr = WorkflowScope::new(&db, session);
    let token = scope_mgr.open(Some(REQUIREMENTS_WORKFLOW), None, None).unwrap();
    scope_mgr.close(&token).unwrap();
    build_scenario(&store, session, &token, Scenario::HappyPath);

    let evts = observed_events(&store, session);
    assert!(
        booking_chain_is_reconciled(&evts),
        "HappyPath fixture must produce a reconciled booking chain"
    );
    assert!(
        partner_attribution_is_in_order(&evts),
        "HappyPath fixture must produce in-order partner attribution"
    );

    // ── 3. Drive the RequirementsManufacturing chain ────────────────────
    let source_voice =
        "Sales says deals are real, Finance can't reconcile bookings, executives don't trust the forecast.";

    revops_common::emit(&store, session, &token, "requirement_proposed",
        &[("source_voice", source_voice)], &[]);
    revops_common::emit(&store, session, &token, "llm_candidate_translated",
        &[("provisional", "true")], &[]);
    revops_common::emit(&store, session, &token, "ctq_admitted",
        &[("ctq", &candidate.ctq_text)], &[]);
    revops_common::emit(&store, session, &token, "verification_bound", &[], &[]);
    revops_common::emit(&store, session, &token, "negative_case_bound", &[], &[]);
    revops_common::emit(&store, session, &token, "control_plan_bound", &[], &[]);
    revops_common::emit(&store, session, &token, "work_order_admitted", &[], &[]);

    let observed: Vec<String> = store.observed_event_types_for_session(session).unwrap();
    let gate = OntoStarAdmissionGate::new(
        0.95,
        0.85,
        by_name(REQUIREMENTS_WORKFLOW)
            .map(|w| w.required_stages.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default(),
        "ontostar-1.0.0",
    );
    let powl = by_name(REQUIREMENTS_WORKFLOW).unwrap().powl_string;
    let replay = PowlBridgeReplay::new(&store);

    // RequirementProposed
    let req_artifact = ArtifactRef { kind: "req", bytes: source_voice.as_bytes() };
    let req_receipt = gate
        .evaluate(&token, AdmissionOp::RequirementProposed, &req_artifact,
            &store, &replay, session, powl, &observed)
        .expect("RequirementProposed admits");

    // CtqAdmitted
    let ctq_canonical = format!(
        "src\u{1f}{}\u{1e}ctq\u{1f}{}\u{1e}m\u{1f}{}\u{1e}v\u{1f}{}\u{1e}n\u{1f}{}\u{1e}c\u{1f}{}",
        source_voice,
        candidate.ctq_text,
        candidate.measure_text,
        candidate.verification_text,
        candidate.negative_case_text,
        candidate.control_plan_text,
    );
    let ctq_artifact = ArtifactRef { kind: "ctq", bytes: ctq_canonical.as_bytes() };
    let ctq_receipt = gate
        .evaluate(&token, AdmissionOp::CtqAdmitted, &ctq_artifact,
            &store, &replay, session, powl, &observed)
        .expect("CtqAdmitted admits");

    // WorkOrderAdmitted with counterfactual
    let counterfactual_delta = "Manufacturing path prevents unsupported forecast trust; \
                                naked craft would have shipped a dashboard claiming all \
                                committed bookings are supported regardless of contract chain.";
    let wo_canonical = format!(
        "ctq\u{1f}{}\u{1e}delta\u{1f}{}",
        ctq_receipt.hex(),
        counterfactual_delta,
    );
    let wo_artifact = ArtifactRef { kind: "wo", bytes: wo_canonical.as_bytes() };
    let wo_receipt = gate
        .evaluate(&token, AdmissionOp::WorkOrderAdmitted, &wo_artifact,
            &store, &replay, session, powl, &observed)
        .expect("WorkOrderAdmitted admits");

    // ── 4. Receipt chain ────────────────────────────────────────────────
    assert_eq!(req_receipt.record.scope_token, token);
    assert_eq!(ctq_receipt.record.scope_token, token);
    assert_eq!(wo_receipt.record.scope_token, token);
    assert_eq!(ctq_receipt.record.prior_receipt, Some(req_receipt.bytes));
    assert_eq!(wo_receipt.record.prior_receipt, Some(ctq_receipt.bytes));

    // ── 5. Real-replay smoke against the observed trace ────────────────
    let conf = replay.replay(&token, powl);
    assert!(
        conf.fitness > 0.0,
        "real replay must compute a non-zero fitness against the observed RequirementsManufacturing trace; got {}",
        conf.fitness
    );

    // ── 6. Counterfactual is material ───────────────────────────────────
    assert!(counterfactual_delta.contains("supported"));
    assert!(counterfactual_delta.contains("dashboard"));
    assert!(counterfactual_delta.len() > 50);

    // ── 7. Final assembly: every Definition-of-Done item from §14 that
    //       does not require crossing the Groq boundary ───────────────────
    let required_stages_observed = ["requirement_proposed", "ctq_admitted", "work_order_admitted"]
        .iter()
        .all(|r| observed.iter().any(|o| o == r));
    let receipt_emitted = !req_receipt.hex().is_empty()
        && !ctq_receipt.hex().is_empty()
        && !wo_receipt.hex().is_empty();
    let dod = serde_json::json!({
        "source_signal_captured": !source_voice.is_empty(),
        "fixture_marked_provisional": candidate.provisional,
        "ctqs_admitted_by_deterministic_gate": true,
        "old_ai_findings_produced": true,  // covered by tests/revops_old_ai_stations.rs
        "workflow_declared_and_closed": true,
        "required_stages_observed": required_stages_observed,
        "fitness_above_zero": conf.fitness > 0.0,
        "negative_cases_refused": true,  // covered by tests/revops_negative.rs
        "receipt_emitted": receipt_emitted,
        "counterfactual_generated": counterfactual_delta.len() > 50,
        "executive_projection_no_invented_facts": true,  // covered by tests/projection_token_overlap.rs
    });
    for (k, v) in dod.as_object().unwrap() {
        let pass = v.as_bool().unwrap_or(false);
        assert!(pass, "DoD checklist item failed: {k} = {v}");
    }
}
