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
///
/// All-caps tokens are normalised to lowercase before comparison — case
/// sensitivity does not affect outcome:
///
/// ```
/// use open_ontologies::projection_check::invented_tokens;
///
/// // "RECONCILIATION" in the summary matches "reconciliation" in the evidence
/// // because both are lowercased before the substring check.
/// let evidence = "reconciliation rate is high";
/// let summary = "RECONCILIATION rate";
/// assert!(invented_tokens(summary, evidence).is_empty(),
///     "all-caps token must match its lowercase equivalent in evidence");
/// ```
///
/// An all-caps token that is genuinely absent from the evidence is still
/// flagged (case-insensitively):
///
/// ```
/// use open_ontologies::projection_check::invented_tokens;
///
/// let evidence = "revenue growth expected";
/// let summary = "HALLUCINATED revenue";
/// let inv = invented_tokens(summary, evidence);
/// assert!(inv.contains(&"hallucinated".to_string()),
///     "all-caps absent token must be reported in lowercase");
/// ```
///
/// Unicode alphabetic characters pass the `is_alphabetic` filter and are
/// handled without panicking:
///
/// ```
/// use open_ontologies::projection_check::invented_tokens;
///
/// // "café" — all characters are alphabetic (including the accented 'é'),
/// // length is 4, and it is absent from evidence → should be flagged.
/// let evidence = "coffee revenue data";
/// let inv = invented_tokens("café revenue", evidence);
/// assert!(inv.contains(&"café".to_string()),
///     "unicode alphabetic token absent from evidence must be flagged");
/// ```
///
/// ```
/// use open_ontologies::projection_check::invented_tokens;
///
/// // Unicode token present in evidence → not flagged.
/// let evidence = "café revenue data";
/// let inv = invented_tokens("café revenue", evidence);
/// assert!(inv.is_empty(),
///     "unicode token that appears in evidence must not be flagged");
/// ```
///
/// Token at the exact 4-character boundary is included in the check:
///
/// ```
/// use open_ontologies::projection_check::invented_tokens;
///
/// // "four" is exactly 4 chars — it must be evaluated, not skipped.
/// // When absent from evidence it is reported.
/// let evidence = "alpha beta gamma";
/// let inv = invented_tokens("four alpha", evidence);
/// assert!(inv.contains(&"four".to_string()),
///     "exactly-4-char token absent from evidence must be flagged");
/// ```
///
/// ```
/// use open_ontologies::projection_check::invented_tokens;
///
/// // "four" is exactly 4 chars and present in evidence → not flagged.
/// let evidence = "four score and seven";
/// let inv = invented_tokens("four score", evidence);
/// assert!(inv.is_empty(),
///     "exactly-4-char token present in evidence must not be flagged");
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
