//! Clinical Commands — crosswalk, enrich, validate-clinical
//!
//! Provides CLI verbs for looking up clinical terminology mappings
//! (ICD-10 ↔ SNOMED ↔ MeSH), enriching ontology classes with
//! `skos:exactMatch` triples, and validating class labels against
//! the clinical crosswalk.
//!
//! # CLI usage
//!
//! ```no_run
//! // Look up all mappings for ICD-10 code J18.9 (pneumonia):
//! //   open-ontologies clinical crosswalk --code J18.9 --system ICD-10
//! //
//! // Add a skos:exactMatch triple linking an OWL class to a clinical code:
//! //   open-ontologies clinical enrich \
//! //     --class-iri http://example.org/onto/Pneumonia \
//! //     --code J18.9 \
//! //     --system ICD-10
//! //
//! // Validate all class labels against the clinical crosswalk:
//! //   open-ontologies clinical validate-clinical
//! ```

use clap_noun_verb::Result as NounVerbResult;
use clap_noun_verb_macros::verb;
use serde::Serialize;

use super::helpers::{DEFAULT_DATA_DIR, setup, to_verb_err};

const CROSSWALKS_PATH: &str = "data/crosswalks.parquet";

// ── output types ─────────────────────────────────────────────────────────

/// A single source → target terminology mapping returned by `crosswalk`.
///
/// # Examples
///
/// ```no_run
/// // CrosswalkMapping is returned inside CrosswalkOutput.mappings.
/// // Example JSON output for `clinical crosswalk --code J18.9 --system ICD-10`:
/// //
/// // {
/// //   "code": "J18.9",
/// //   "system": "ICD-10",
/// //   "mappings": [
/// //     {
/// //       "target_code": "233604007",
/// //       "target_system": "SNOMED",
/// //       "relation": "exactMatch",
/// //       "source_label": "Pneumonia, unspecified organism",
/// //       "target_label": "Pneumonia (disorder)"
/// //     }
/// //   ]
/// // }
/// ```
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
