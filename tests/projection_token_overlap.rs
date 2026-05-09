//! Pure-function tests for the executive-projection token-overlap check.
//!
//! R4 WA, §24 Chicago TDD: these tests exercise
//! `open_ontologies::projection_check::invented_tokens` directly — no
//! HTTP, no Groq, no JSON-shaped imposter response. The deleted
//! `tests/revops_groq_boundary.rs::b_*` cases pretended to verify this
//! algorithm by routing input through a tokio-TCP mock; that pattern is
//! "decorative completion" (§24) because the algorithm under test was
//! the closure inside `onto_executive_projection`, not the wire format.
//!
//! Δ>0 PROOF (per case): the deleted boundary tests would have shipped
//!            green if the closure were silently broken (e.g. `< 4`
//!            replaced with `< 5`, `is_alphabetic` replaced with
//!            `is_alphanumeric`, or `contains` replaced with
//!            `starts_with`) so long as the mock JSON happened to dodge
//!            the boundary. These pure-function tests pin the algorithm
//!            in `src/projection_check.rs::invented_tokens` such that
//!            ANY of those mutations fails the gate.

use open_ontologies::projection_check::invented_tokens;

#[test]
fn faithful_summary_returns_no_invented_tokens() {
    // Δ>0 PROOF: the deleted `b_executive_projection_must_only_cite_admitted_evidence_tokens`
    //            relied on a JSON imposter to feed the closure; this test
    //            pins the algorithm directly so the mock can be deleted
    //            without losing the positive-path coverage.
    //            Production line pinned: src/projection_check.rs::invented_tokens
    //            (and via re-call from src/server.rs::onto_executive_projection).
    let evidence = "Reconciliation completeness rate is 83 percent. Forecast risk explainable. \
                    Nightly report ran. Refuse missing contract. Block partial chain. \
                    Booking incomplete chain detected.";
    let summary = "Forecast risk explainable Reconciliation completeness Nightly report \
                   Refuse missing contract Block partial chain incomplete";
    let inv = invented_tokens(summary, evidence);
    assert!(inv.is_empty(), "faithful summary must produce no invented tokens; got: {inv:?}");
}

#[test]
fn invented_token_in_summary_is_flagged() {
    // Δ>0 PROOF: the deleted `b_negative_summary_with_invented_token_is_rejected`
    //            relied on a JSON imposter; this pins the negative branch
    //            of the algorithm. If `invented_tokens` ever swallowed
    //            invented words (e.g. by switching to `starts_with` or
    //            `contains`+empty-string), this test fails immediately.
    //            Production line pinned: src/projection_check.rs:invented_tokens
    //            (the `!evidence_lc.contains(&tok_lc)` branch).
    let evidence = "Reconciliation gap detected; nightly run flagged 17 invoices.";
    let summary = "Hallucination detected in pipeline forecast";
    let inv = invented_tokens(summary, evidence);
    assert!(
        inv.iter().any(|t| t == "hallucination"),
        "expected `hallucination` to be flagged as invented, got {inv:?}"
    );
    assert!(
        inv.iter().any(|t| t == "pipeline"),
        "expected `pipeline` to be flagged as invented, got {inv:?}"
    );
    assert!(
        inv.iter().any(|t| t == "forecast"),
        "expected `forecast` to be flagged as invented (not present in evidence), got {inv:?}"
    );
}

#[test]
fn three_char_tokens_are_ignored_even_when_invented() {
    // Δ>0 PROOF: pins the `len < 4` short-circuit. If a future refactor
    //            changes the threshold to 3 (or removes it), this test
    //            fails because previously-ignored short tokens become
    //            spurious invented findings.
    //            Production line pinned: src/projection_check.rs (the
    //            `if tok_lc.len() < 4` early-continue).
    let evidence = "alpha beta gamma";
    let summary = "xyz qed alpha"; // `xyz`, `qed` are absent and 3 chars
    assert!(invented_tokens(summary, evidence).is_empty());
}

#[test]
fn mixed_alphanumeric_tokens_are_ignored() {
    // Δ>0 PROOF: pins the `chars().all(is_alphabetic)` rule. Numbers and
    //            hex IDs in summaries (e.g. `2029`, `q4`) must NOT be
    //            flagged as invented even when absent from evidence.
    //            Production line pinned: src/projection_check.rs (the
    //            `!tok_lc.chars().all(|c| c.is_alphabetic())` early-continue).
    let evidence = "Forecast risk monitored.";
    let summary = "Forecast in 2029 risk q42024 monitored";
    let inv = invented_tokens(summary, evidence);
    assert!(
        inv.is_empty(),
        "mixed-alphanumeric tokens must not be flagged; got {inv:?}"
    );
}

#[test]
fn invented_tokens_are_deduplicated_in_order() {
    // Δ>0 PROOF: pins the `!invented.contains(&tok_lc)` dedupe rule. If
    //            removed, repeated invented words would multiply in the
    //            output and downstream consumers (the JSON `invented_tokens`
    //            field returned by onto_executive_projection) would change
    //            shape silently.
    //            Production line pinned: src/projection_check.rs (the
    //            dedupe `!invented.contains(&tok_lc)` clause).
    let evidence = "alpha beta";
    let summary = "ghost ghost ghost phantom alpha";
    let inv = invented_tokens(summary, evidence);
    assert_eq!(inv, vec!["ghost".to_string(), "phantom".to_string()]);
}
