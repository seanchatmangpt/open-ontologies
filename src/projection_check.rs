//! Pure-function token-overlap check for the executive-projection gate.
//!
//! Extracted from `src/server.rs::onto_executive_projection` (R4 WA, §24
//! Chicago TDD) so the algorithm is testable at function-level without
//! crossing the Groq HTTP boundary. The translator may return a
//! `CandidateCtq` whose flattened text introduces tokens absent from the
//! admitted evidence — this module identifies those "invented" tokens.
//!
//! Doctrine: an executive projection is a **summary** of admitted
//! evidence, not a creative restatement. Any 4+ char alphabetic word
//! that appears in the summary but not in the evidence (case-insensitive,
//! substring match) is treated as an LLM hallucination and rejected by
//! the calling handler.
//!
//! § Counterfactual proof: the previous test layer relied on an HTTP
//! mock that conflated "translator wire format works" with "token-overlap
//! algorithm works". This module pins the algorithm independently so the
//! mock can be deleted without losing coverage.

/// Return the list of 4+ char alphabetic tokens present in `summary`
/// (lowercased) that do NOT appear as a substring of `evidence`
/// (lowercased). Order-preserving and de-duplicated.
///
/// Token rules:
///   1. Split on any non-alphanumeric character.
///   2. Lowercase.
///   3. Reject if length < 4.
///   4. Reject if any character is non-alphabetic (drops `q4`, `83pct`).
///   5. Compare against `evidence` via substring `contains`.
///
/// The substring check is intentionally permissive — `forecast` matches
/// `forecasted`, keeping the gate from rejecting morphological variants.
///
/// # Examples
///
/// Empty summary always returns empty:
/// ```
/// # use open_ontologies::projection_check::invented_tokens;
/// assert!(invented_tokens("", "anything").is_empty());
/// ```
///
/// A faithful summary (all words present in evidence) returns empty:
/// ```
/// # use open_ontologies::projection_check::invented_tokens;
/// let evidence = "Reconciliation completeness rate is 83 percent. Forecast risk explainable.";
/// let summary  = "Reconciliation forecast risk completeness";
/// assert!(invented_tokens(summary, evidence).is_empty());
/// ```
///
/// A word absent from evidence is flagged:
/// ```
/// # use open_ontologies::projection_check::invented_tokens;
/// let inv = invented_tokens(
///     "Reconciliation hallucination detected",
///     "Reconciliation gap detected.",
/// );
/// assert_eq!(inv, vec!["hallucination".to_string()]);
/// ```
///
/// Short tokens (< 4 chars) are ignored even when absent from evidence:
/// ```
/// # use open_ontologies::projection_check::invented_tokens;
/// assert!(invented_tokens("xyz alpha", "alpha beta gamma").is_empty());
/// ```
pub fn invented_tokens(summary: &str, evidence: &str) -> Vec<String> {
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
    invented
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_summary_returns_empty() {
        assert!(invented_tokens("", "anything").is_empty());
    }

    #[test]
    fn faithful_summary_returns_empty() {
        let evidence = "Reconciliation completeness rate is 83 percent. Forecast risk explainable.";
        let summary = "Reconciliation forecast risk completeness";
        assert!(invented_tokens(summary, evidence).is_empty());
    }

    #[test]
    fn invented_word_is_flagged() {
        let evidence = "Reconciliation gap detected.";
        let summary = "Reconciliation hallucination detected";
        let inv = invented_tokens(summary, evidence);
        assert_eq!(inv, vec!["hallucination".to_string()]);
    }

    #[test]
    fn short_tokens_are_ignored() {
        // 3-char tokens drop out even when absent from evidence.
        let evidence = "alpha beta gamma";
        let summary = "xyz alpha";
        assert!(invented_tokens(summary, evidence).is_empty());
    }
}
