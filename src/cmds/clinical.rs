//! Clinical Commands — crosswalk, enrich, validate-clinical

use clap_noun_verb::Result as NounVerbResult;
use clap_noun_verb_macros::verb;
use serde::Serialize;

use super::helpers::{DEFAULT_DATA_DIR, setup, to_verb_err};

const CROSSWALKS_PATH: &str = "data/crosswalks.parquet";

// ── output types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CrosswalkMapping {
    pub target_code: String,
    pub target_system: String,
    pub relation: String,
    pub source_label: String,
    pub target_label: String,
}

#[derive(Serialize)]
pub struct CrosswalkOutput {
    pub code: String,
    pub system: String,
    pub mappings: Vec<CrosswalkMapping>,
}

// ── domain helpers ────────────────────────────────────────────────────────

fn load_crosswalks() -> NounVerbResult<open_ontologies::clinical::ClinicalCrosswalks> {
    open_ontologies::clinical::ClinicalCrosswalks::load(CROSSWALKS_PATH)
        .map_err(|e| clap_noun_verb::NounVerbError::execution_error(format!("Crosswalks not loaded: {}", e)))
}

// ── verbs ─────────────────────────────────────────────────────────────────

/// Look up clinical terminology crosswalk
#[verb]
fn crosswalk(code: String, system: String) -> NounVerbResult<CrosswalkOutput> {
    let cw = load_crosswalks()?;
    let results = cw.lookup(&code, &system);
    Ok(CrosswalkOutput {
        mappings: results.iter().map(|r| CrosswalkMapping {
            target_code: r.target_code.clone(), target_system: r.target_system.clone(),
            relation: r.relation.clone(), source_label: r.source_label.clone(),
            target_label: r.target_label.clone(),
        }).collect(),
        code,
        system,
    })
}

/// Add skos:exactMatch triple for clinical code
#[verb]
fn enrich(class_iri: String, code: String, system: String, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (_db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let cw = load_crosswalks()?;
    let result = cw.enrich(&graph, &class_iri, &code, &system);
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// Validate class labels against clinical terminology
#[verb]
fn validate_clinical(data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (_db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let cw = load_crosswalks()?;
    let result = cw.validate_clinical(&graph);
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}
