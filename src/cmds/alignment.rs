//! Alignment Commands — ontology alignment and feedback

use clap_noun_verb::Result as NounVerbResult;
use clap_noun_verb_macros::verb;

use super::helpers::{DEFAULT_DATA_DIR, setup, to_verb_err};

// ── verbs ─────────────────────────────────────────────────────────────────

/// Detect alignment candidates between two ontologies
#[verb]
fn align(source: String, target: Option<String>, min_confidence: Option<f64>, dry_run: Option<bool>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let source_ttl = std::fs::read_to_string(&source).map_err(to_verb_err)?;
    let target_ttl = target.as_ref().map(|t| std::fs::read_to_string(t)).transpose().map_err(to_verb_err)?;
    let engine = open_ontologies::align::AlignmentEngine::new(db, graph);
    let result = engine.align(&source_ttl, target_ttl.as_deref(), min_confidence.unwrap_or(0.85), dry_run.unwrap_or(false))
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// Accept or reject an alignment candidate
#[verb]
fn align_feedback(source: String, target: String, accept: Option<bool>, reject: Option<bool>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let engine = open_ontologies::align::AlignmentEngine::new(db, graph);
    let accepted = accept.unwrap_or(true) && !reject.unwrap_or(false);
    let result = engine.record_feedback(&source, &target, "user_feedback", accepted, None)
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// Accept or dismiss a lint issue
#[verb]
fn lint_feedback(rule_id: String, entity: String, accept: Option<bool>, dismiss: Option<bool>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (db, _graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let accepted = accept.unwrap_or(false) || !dismiss.unwrap_or(false);
    let result = open_ontologies::feedback::record_tool_feedback(&db, "lint", &rule_id, &entity, accepted)
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// Accept or dismiss an enforce violation
#[verb]
fn enforce_feedback(rule_id: String, entity: String, accept: Option<bool>, dismiss: Option<bool>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (db, _graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let accepted = accept.unwrap_or(false) || !dismiss.unwrap_or(false);
    let result = open_ontologies::feedback::record_tool_feedback(&db, "enforce", &rule_id, &entity, accepted)
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}
