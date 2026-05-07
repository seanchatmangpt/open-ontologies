//! Shared helpers for open-ontologies verb functions.

use open_ontologies::config::expand_tilde;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use std::sync::Arc;

/// Default data directory.
pub const DEFAULT_DATA_DIR: &str = "~/.open-ontologies";

/// Expand the default data dir and set up (StateDb + GraphStore).
pub fn setup(data_dir: &str) -> anyhow::Result<(StateDb, Arc<GraphStore>)> {
    let data_dir = expand_tilde(data_dir);
    let data_path = std::path::Path::new(&data_dir);
    std::fs::create_dir_all(data_path)?;
    let db_path = data_path.join("open-ontologies.db");
    let db = StateDb::open(&db_path)?;
    let graph = Arc::new(GraphStore::new());
    Ok((db, graph))
}

/// Convert an anyhow error into a NounVerbError.
pub fn to_verb_err(e: impl std::fmt::Display) -> clap_noun_verb::NounVerbError {
    clap_noun_verb::NounVerbError::execution_error(e.to_string())
}
