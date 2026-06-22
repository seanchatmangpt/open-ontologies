//! Graph-projection loss audit (#35) and Token-overlap check for executive-projection.
//!
//! Exposes:
//! 1. `check_projection_loss` to audit whether a projected TBox slice has dropped predicates
//!    vs the loaded graph.
//! 2. `invented_tokens` to detect tokens introduced in the LLM executive summary that are absent from evidence.

use crate::graph::GraphStore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Per-seed loss report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedLossReport {
    pub seed_iri: String,
    /// Predicates appearing in the source but absent from the projection.
    pub dropped_predicates: Vec<String>,
    /// Object IRIs appearing in the source but absent from the projection.
    pub dropped_objects: Vec<String>,
    /// Total source-side triples touching this seed.
    pub source_triples: u64,
    /// Total projection-side triples touching this seed.
    pub projected_triples: u64,
    /// `projected_triples / source_triples` (0.0 if source is empty).
    pub coverage_ratio: f64,
}

/// Top-level audit report aggregating per-seed losses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectionLossReport {
    /// Whether projection parsed as Turtle.
    pub projection_parses: bool,
    /// Aggregate coverage ratio across all seeds (mean of per-seed ratios).
    pub aggregate_coverage_ratio: f64,
    /// Per-seed details.
    pub per_seed: Vec<SeedLossReport>,
    /// True iff projection_parses AND all per-seed coverage_ratio == 1.0.
    pub ok: bool,
    /// Total dropped predicates across all seeds (deduped).
    pub total_dropped_predicates: usize,
    /// Total dropped objects across all seeds (deduped).
    pub total_dropped_objects: usize,
}

/// Audit a projected Turtle slice against the loaded ontology's full
/// neighbourhood of the seed IRIs.
pub fn check_projection_loss(
    graph: &Arc<GraphStore>,
    seed_iris: &[String],
    projected_ttl: &str,
) -> anyhow::Result<ProjectionLossReport> {
    // Load the projection into a temp store. Parse failure ⇒ projection_parses=false.
    let projected = GraphStore::new();
    let parse_ok = projected.load_turtle(projected_ttl, None).is_ok();
    if !parse_ok {
        return Ok(ProjectionLossReport {
            projection_parses: false,
            aggregate_coverage_ratio: 0.0,
            per_seed: Vec::new(),
            ok: false,
            total_dropped_predicates: 0,
            total_dropped_objects: 0,
        });
    }

    let projected_ref: Arc<GraphStore> = Arc::new(projected);

    let mut per_seed = Vec::with_capacity(seed_iris.len());
    let mut all_dropped_preds: BTreeSet<String> = BTreeSet::new();
    let mut all_dropped_objs: BTreeSet<String> = BTreeSet::new();
    let mut sum_ratio = 0.0;

    for iri in seed_iris {
        // Collect (predicate, object) pairs for this seed from both stores.
        let source_pairs = neighbourhood_pairs(graph, iri)?;
        let projected_pairs = neighbourhood_pairs(&projected_ref, iri)?;

        let source_preds: BTreeSet<&str> = source_pairs.iter().map(|(p, _)| p.as_str()).collect();
        let projected_preds: BTreeSet<&str> =
            projected_pairs.iter().map(|(p, _)| p.as_str()).collect();
        let dropped_preds: Vec<String> = source_preds
            .difference(&projected_preds)
            .map(|s| s.to_string())
            .collect();

        let source_objs: BTreeSet<&str> = source_pairs.iter().map(|(_, o)| o.as_str()).collect();
        let projected_objs: BTreeSet<&str> = projected_pairs.iter().map(|(_, o)| o.as_str()).collect();
        let dropped_objs: Vec<String> = source_objs
            .difference(&projected_objs)
            .map(|s| s.to_string())
            .collect();

        let source_n = source_pairs.len() as u64;
        let projected_n = projected_pairs.len() as u64;
        let coverage = if source_n == 0 {
            1.0
        } else {
            (projected_n.min(source_n) as f64) / (source_n as f64)
        };

        for d in &dropped_preds {
            all_dropped_preds.insert(d.clone());
        }
        for d in &dropped_objs {
            all_dropped_objs.insert(d.clone());
        }
        sum_ratio += coverage;

        per_seed.push(SeedLossReport {
            seed_iri: iri.clone(),
            dropped_predicates: dropped_preds,
            dropped_objects: dropped_objs,
            source_triples: source_n,
            projected_triples: projected_n,
            coverage_ratio: coverage,
        });
    }

    let n = seed_iris.len().max(1) as f64;
    let aggregate = sum_ratio / n;
    let ok = per_seed.iter().all(|r| (r.coverage_ratio - 1.0).abs() < 1e-9);

    Ok(ProjectionLossReport {
        projection_parses: true,
        aggregate_coverage_ratio: aggregate,
        per_seed,
        ok,
        total_dropped_predicates: all_dropped_preds.len(),
        total_dropped_objects: all_dropped_objs.len(),
    })
}

/// All `(predicate, object)` pairs where `iri` appears as the subject.
fn neighbourhood_pairs(
    graph: &Arc<GraphStore>,
    iri: &str,
) -> anyhow::Result<Vec<(String, String)>> {
    let q = format!(
        r#"SELECT DISTINCT ?p ?o WHERE {{ <{iri}> ?p ?o }} LIMIT 1000"#
    );
    let mut out = Vec::new();
    let json_str = graph.sparql_select(&q)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
    if let Some(rows) = parsed["results"].as_array() {
        for row in rows {
            let p = row["p"].as_str().unwrap_or("").trim_matches(|c| c == '<' || c == '>').to_string();
            let o = row["o"].as_str().unwrap_or("").trim_matches(|c| c == '<' || c == '>').to_string();
            if !p.is_empty() {
                out.push((p, o));
            }
        }
    }
    Ok(out)
}

/// Return the list of 4+ char alphabetic tokens present in `summary`
/// (lowercased) that do NOT appear as a substring of `evidence`
/// (lowercased). Order-preserving and de-duplicated.
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

    fn loaded(ttl: &str) -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(ttl, None).expect("load");
        g
    }

    #[test]
    fn full_projection_reports_ok_and_full_coverage() {
        let source = r#"
            @prefix ex: <http://ex.org/> .
            ex:Cat ex:hasColour ex:Black ; ex:age 5 .
        "#;
        let projection = source; // identical
        let g = loaded(source);
        let report = check_projection_loss(&g, &["http://ex.org/Cat".to_string()], projection).unwrap();
        assert!(report.ok);
        assert_eq!(report.aggregate_coverage_ratio, 1.0);
        assert_eq!(report.total_dropped_predicates, 0);
    }

    #[test]
    fn dropped_predicate_is_reported() {
        let source = r#"
            @prefix ex: <http://ex.org/> .
            ex:Cat ex:hasColour ex:Black ; ex:age 5 ; ex:species "Felis catus" .
        "#;
        // Projection keeps only hasColour; drops age and species.
        let projection = r#"
            @prefix ex: <http://ex.org/> .
            ex:Cat ex:hasColour ex:Black .
        "#;
        let g = loaded(source);
        let report = check_projection_loss(&g, &["http://ex.org/Cat".to_string()], projection).unwrap();
        assert!(!report.ok);
        assert!(report.total_dropped_predicates >= 2);
        let cat_report = &report.per_seed[0];
        assert!(cat_report.dropped_predicates.iter().any(|p| p.contains("age")));
        assert!(cat_report.dropped_predicates.iter().any(|p| p.contains("species")));
    }

    #[test]
    fn invalid_projection_turtle_reports_parses_false() {
        let g = loaded(r#"@prefix ex: <http://ex.org/> . ex:X ex:p ex:Y ."#);
        let report = check_projection_loss(&g, &["http://ex.org/X".to_string()], "this is not turtle: << >>").unwrap();
        assert!(!report.projection_parses);
        assert!(!report.ok);
    }

    #[test]
    fn missing_seed_in_source_has_full_coverage_by_definition() {
        let g = loaded(r#"@prefix ex: <http://ex.org/> . ex:Other ex:p ex:Z ."#);
        let report = check_projection_loss(
            &g,
            &["http://ex.org/UnknownSeed".to_string()],
            r#"@prefix ex: <http://ex.org/> . ex:Other ex:p ex:Z ."#,
        )
        .unwrap();
        assert!(report.ok);
        assert_eq!(report.per_seed[0].source_triples, 0);
    }

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
        let evidence = "alpha beta gamma";
        let summary = "xyz alpha";
        assert!(invented_tokens(summary, evidence).is_empty());
    }
}
