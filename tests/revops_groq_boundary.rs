//! Phase 3.4 — Fortune-5 RevOps Groq LLM boundary tests.
//!
//! Three tests from the test plan §6:
//!   A. Messy voice → candidate CTQ (output marked provisional)
//!   B. Evidence → executive projection with token-overlap check
//!      (rejects any 4+ char alpha word in summary not in evidence)
//!   C. Contradictory stakeholder voice → impasse map (LLM stabilizes
//!      contradiction; deterministic gate decides admission)
//!
//! All three drive the GroqTranslator against the in-process tokio
//! HTTP mock from tests/revops_common/groq_mock.rs. No real Groq
//! calls in CI.

mod revops_common;

use open_ontologies::llm_translator::GroqTranslator;
use revops_common::{groq_mock, CANARY_GROQ_KEY};
use std::time::Duration;

const REAL_KEY_NEVER_USED_IN_CI: &str = "should-not-be-emitted-key-3vh3";

// ── A. Messy voice → candidate CTQ ─────────────────────────────────────────

#[tokio::test]
async fn a_messy_voice_translates_to_provisional_candidate() {
    let candidate_json = serde_json::json!({
        "source_voice_echo": "Sales says committed; Finance says not booked",
        "defect_class_hint": "ctq_incomplete",
        "ctq_text": "Booking must reconcile",
        "measure_text": "Reconciliation rate",
        "verification_text": "Run nightly reconciliation",
        "negative_case_text": "Refuse without contract chain",
        "control_plan_text": "Block bookings missing contract",
        // Adversarial: the mock claims authoritative.
        "provisional": false,
    })
    .to_string();
    let (base, captured_auth) = groq_mock::spawn_with_response(candidate_json).await;
    let translator = GroqTranslator::new(
        &base,
        Some(CANARY_GROQ_KEY.to_string()),
        "llama-3.3-70b-versatile",
        Duration::from_secs(5),
    )
    .unwrap();
    let candidate = translator
        .translate_candidate_ctq("Sales keeps saying the quarter is fine, but Finance keeps finding bookings that do not tie out.")
        .await
        .expect("translation should succeed against mock");

    // Provisional MUST be forced true, regardless of mock value.
    assert!(
        candidate.provisional,
        "translator must force provisional=true even when LLM returns false"
    );
    // 5 mandatory CTQ fields are present and non-empty.
    for (label, value) in [
        ("ctq_text", &candidate.ctq_text),
        ("measure_text", &candidate.measure_text),
        ("verification_text", &candidate.verification_text),
        ("negative_case_text", &candidate.negative_case_text),
        ("control_plan_text", &candidate.control_plan_text),
    ] {
        assert!(!value.trim().is_empty(), "candidate.{label} must be non-empty");
    }
    // Auth was sent with the canary, but the candidate doesn't echo it.
    let auth = captured_auth.lock().await.clone();
    assert!(auth.contains(CANARY_GROQ_KEY), "mock did not receive bearer auth");
    let serialized = serde_json::to_string(&candidate).unwrap();
    assert!(
        !serialized.contains(CANARY_GROQ_KEY),
        "candidate must not echo the bearer key — got {serialized}"
    );
}

// ── B. Evidence → executive projection with token-overlap check ────────────

#[tokio::test]
async fn b_executive_projection_must_only_cite_admitted_evidence_tokens() {
    // The translator returns a CandidateCtq whose fields are then used
    // to produce a "summary" by concatenating them. We then run the
    // token-overlap check that the onto_executive_projection handler
    // implements: every 4+ char alpha word in the summary must appear
    // in the evidence (lowercase, substring).
    // For this positive test, the candidate text is constrained to
    // tokens already in the evidence — proves the token-overlap
    // check accepts a faithful summary. The defect_class_hint uses
    // a single-word value (incomplete) that is also in the evidence.
    let candidate_json = serde_json::json!({
        "source_voice_echo": "Reconciliation gap reported",
        "defect_class_hint": "incomplete",
        "ctq_text": "Forecast risk explainable",
        "measure_text": "Reconciliation completeness",
        "verification_text": "Nightly report",
        "negative_case_text": "Refuse missing contract",
        "control_plan_text": "Block partial chain",
        "provisional": false,
    })
    .to_string();
    let (base, _captured) = groq_mock::spawn_with_response(candidate_json).await;
    let translator = GroqTranslator::new(
        &base,
        Some(REAL_KEY_NEVER_USED_IN_CI.to_string()),
        "llama-3.3-70b-versatile",
        Duration::from_secs(5),
    )
    .unwrap();
    let evidence = "Reconciliation completeness rate is 83%. Forecast risk explainable. \
                    Nightly report ran. Refuse missing contract. Block partial chain. \
                    Booking incomplete chain detected.";
    let candidate = translator
        .translate_candidate_ctq(evidence)
        .await
        .expect("translation succeeds");

    // Reconstruct the summary the handler would produce.
    let summary = format!(
        "{} {} {} {} {} {}",
        candidate.ctq_text,
        candidate.measure_text,
        candidate.verification_text,
        candidate.negative_case_text,
        candidate.control_plan_text,
        candidate.defect_class_hint,
    );

    // Run the token-overlap check (mirror of src/server.rs::onto_
    // executive_projection). Every 4+ char alpha word in the summary
    // must appear in the evidence.
    let evidence_lc = evidence.to_lowercase();
    let mut invented: Vec<String> = Vec::new();
    for tok in summary.split(|c: char| !c.is_alphanumeric()) {
        let tok_lc = tok.to_lowercase();
        if tok_lc.len() < 4 || !tok_lc.chars().all(|c| c.is_alphabetic()) {
            continue;
        }
        if !evidence_lc.contains(&tok_lc) && !invented.contains(&tok_lc) {
            invented.push(tok_lc);
        }
    }
    assert!(
        invented.is_empty(),
        "executive projection invented tokens not present in evidence: {invented:?}\nsummary: {summary}\nevidence: {evidence}"
    );
}

#[tokio::test]
async fn b_negative_summary_with_invented_token_is_rejected() {
    // Negative control: the mock returns a summary containing a word
    // ("hallucination") that is NOT in the evidence. The token-overlap
    // check must flag it.
    let candidate_json = serde_json::json!({
        "source_voice_echo": "Reconciliation gap",
        "defect_class_hint": "ctq_incomplete",
        "ctq_text": "Hallucination detected in pipeline forecast",
        "measure_text": "Reconciliation rate",
        "verification_text": "Nightly run",
        "negative_case_text": "Refuse missing chain",
        "control_plan_text": "Block partial",
        "provisional": false,
    })
    .to_string();
    let (base, _captured) = groq_mock::spawn_with_response(candidate_json).await;
    let translator = GroqTranslator::new(
        &base,
        Some(REAL_KEY_NEVER_USED_IN_CI.to_string()),
        "llama-3.3-70b-versatile",
        Duration::from_secs(5),
    )
    .unwrap();
    let evidence = "Reconciliation gap detected; nightly run flagged 17 invoices.";
    let candidate = translator
        .translate_candidate_ctq(evidence)
        .await
        .expect("translation succeeds");
    let summary = format!(
        "{} {} {} {} {} {}",
        candidate.ctq_text,
        candidate.measure_text,
        candidate.verification_text,
        candidate.negative_case_text,
        candidate.control_plan_text,
        candidate.defect_class_hint,
    );
    let evidence_lc = evidence.to_lowercase();
    let mut invented: Vec<String> = Vec::new();
    for tok in summary.split(|c: char| !c.is_alphanumeric()) {
        let tok_lc = tok.to_lowercase();
        if tok_lc.len() < 4 || !tok_lc.chars().all(|c| c.is_alphabetic()) {
            continue;
        }
        if !evidence_lc.contains(&tok_lc) && !invented.contains(&tok_lc) {
            invented.push(tok_lc);
        }
    }
    assert!(
        invented.iter().any(|t| t == "hallucination"),
        "expected `hallucination` to be flagged as invented, got {invented:?}"
    );
}

// ── C. Contradictory voice → impasse map ──────────────────────────────────

#[tokio::test]
async fn c_contradictory_voice_yields_candidate_with_contradiction_hint() {
    // The LLM is asked to translate a voice that contains internal
    // contradictions. The deterministic gate is what decides admission;
    // the LLM's job is to surface the contradiction shape, not resolve
    // it.
    let candidate_json = serde_json::json!({
        "source_voice_echo": "Sales: committed. Finance: not booked. Legal: contract executed. RevRec: milestone evidence missing.",
        "defect_class_hint": "ctq_incomplete",
        "ctq_text": "Resolve contradiction by requiring revenue_milestone evidence before classifying booking complete",
        "measure_text": "Number of bookings with contradiction across systems",
        "verification_text": "Cross-reference Sales / Finance / Legal / RevRec event streams nightly",
        "negative_case_text": "Refuse classification when revenue_milestone evidence absent despite Legal contract execution",
        "control_plan_text": "Block booking_complete unless all four systems agree",
        "provisional": false,
    })
    .to_string();
    let (base, _captured) = groq_mock::spawn_with_response(candidate_json).await;
    let translator = GroqTranslator::new(
        &base,
        Some(REAL_KEY_NEVER_USED_IN_CI.to_string()),
        "llama-3.3-70b-versatile",
        Duration::from_secs(5),
    )
    .unwrap();
    let voice = "Sales says the opportunity is committed. Finance says it is not booked. \
                 Legal says contract is executed. RevRec says milestone evidence is missing.";
    let candidate = translator
        .translate_candidate_ctq(voice)
        .await
        .expect("translation succeeds");
    assert!(candidate.provisional);
    // The candidate must carry hints about the contradiction (e.g.
    // mentions of multiple systems / "milestone").
    let combined = format!(
        "{} {} {} {} {}",
        candidate.ctq_text,
        candidate.measure_text,
        candidate.verification_text,
        candidate.negative_case_text,
        candidate.control_plan_text,
    )
    .to_lowercase();
    assert!(
        combined.contains("milestone")
            || combined.contains("contradiction")
            || combined.contains("revenue"),
        "candidate must surface the contradiction shape, got: {combined}"
    );
    // The LLM does NOT get to admit. Deterministic admission would
    // happen at onto_admit_ctq with the 5 fields above. We assert here
    // that the LLM provided the structure; whether it gets admitted is
    // the gate's decision (covered by tests/revops_ctq_admission.rs).
    assert!(!candidate.ctq_text.is_empty());
    assert!(!candidate.negative_case_text.is_empty());
}
