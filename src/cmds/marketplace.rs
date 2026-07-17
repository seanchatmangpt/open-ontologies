//! Marketplace Commands — list and install standard ontologies
//!
//! Provides CLI verbs for browsing and installing standard W3C/ISO/industry
//! ontologies from the built-in curated catalogue (29 entries).
//!
//! # CLI usage
//!
//! ```no_run
//! // List all available ontologies in the marketplace:
//! //   open-ontologies marketplace list
//! //
//! // List ontologies filtered by domain (e.g. "upper", "clinical", "geo"):
//! //   open-ontologies marketplace list --domain upper
//! //
//! // Install the FOAF ontology into the local store:
//! //   open-ontologies marketplace install foaf
//! //
//! // Install SKOS into a custom data directory:
//! //   open-ontologies marketplace install skos --data-dir /tmp/onto-test
//! ```

use clap_noun_verb::Result as NounVerbResult;
use clap_noun_verb_macros::verb;
use serde::Serialize;

use super::helpers::{DEFAULT_DATA_DIR, setup, to_verb_err};
use open_ontologies::graph::GraphStore;
use open_ontologies::marketplace;

// ── output types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct MarketplaceEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub format: String,
}

#[derive(Serialize)]
pub struct ListOutput {
    pub count: usize,
    pub ontologies: Vec<MarketplaceEntry>,
}

/// Result returned after installing a marketplace ontology.
///
/// # Examples
///
/// ```no_run
/// // JSON output for `marketplace install foaf`:
/// //
/// // {
/// //   "ok": true,
/// //   "installed": "foaf",
/// //   "name": "Friend of a Friend (FOAF)",
/// //   "triples_loaded": 631
/// // }
/// ```
#[derive(Serialize)]
pub struct InstallOutput {
    pub ok: bool,
    pub installed: String,
    pub name: String,
    pub triples_loaded: usize,
}

// ── verbs ─────────────────────────────────────────────────────────────────

/// Browse available standard ontologies in the marketplace
#[verb]
fn list(domain: Option<String>) -> NounVerbResult<ListOutput> {
    let entries = marketplace::list(domain.as_deref());
    let ontologies = entries.iter().map(|e| MarketplaceEntry {
        id: e.id.to_string(),
        name: e.name.to_string(),
        description: e.description.to_string(),
        domain: e.domain.to_string(),
        format: marketplace::format_name(e.format).to_string(),
    }).collect::<Vec<_>>();
    Ok(ListOutput { count: ontologies.len(), ontologies })
}

/// Install a standard ontology from the marketplace
#[verb]
fn install(id: String, data_dir: Option<String>) -> NounVerbResult<InstallOutput> {
    let entry = marketplace::find(&id).ok_or_else(|| {
        clap_noun_verb::NounVerbError::execution_error(format!("Unknown ontology ID: '{}'. Run 'marketplace list'.", id))
    })?;
    let (_db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let content = tokio::runtime::Handle::current()
        .block_on(GraphStore::fetch_url(entry.url))
        .map_err(to_verb_err)?;
    let count = graph.load_content_with_base(&content, entry.format, Some(entry.url)).map_err(to_verb_err)?;
    Ok(InstallOutput { ok: true, installed: entry.id.to_string(), name: entry.name.to_string(), triples_loaded: count })
}
