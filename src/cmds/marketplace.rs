//! Marketplace Commands — list and install standard ontologies

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
