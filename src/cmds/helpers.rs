//! Shared helpers for open-ontologies verb functions.
//!
//! # Examples
//!
//! ```no_run
//! // Open the default data directory (creates it if absent).
//! // CLI verbs call setup() with the --data-dir argument or DEFAULT_DATA_DIR.
//! //
//! //   open-ontologies ontology load my-ontology.ttl
//! //   open-ontologies ontology load my-ontology.ttl --data-dir /tmp/onto-test
//! ```

use open_ontologies::config::expand_tilde;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use std::sync::Arc;

/// Default data directory used when no `--data-dir` flag is provided.
///
/// # Examples
///
/// ```no_run
/// // All CLI verbs that accept --data-dir fall back to this path.
/// //   open-ontologies ontology load my-ontology.ttl
/// //   open-ontologies ontology load my-ontology.ttl --data-dir /tmp/onto-test
/// //
/// // DEFAULT_DATA_DIR == "~/.open-ontologies"
/// ```
pub const DEFAULT_DATA_DIR: &str = "~/.open-ontologies";

/// Expand the default data dir and set up (StateDb + GraphStore).
///
/// # Examples
///
/// ```no_run
/// // setup() is called at the top of every verb that needs the triple store.
/// // It creates the directory (and any parents) if absent, opens the SQLite
/// // state database, and returns a file-backed Oxigraph store (falling back
/// // to in-memory if the on-disk lock is held by another process).
/// //
/// //   let (db, graph) = setup("~/.open-ontologies").expect("setup failed");
/// ```
pub fn setup(data_dir: &str) -> anyhow::Result<(StateDb, Arc<GraphStore>)> {
    let data_dir = expand_tilde(data_dir);
    let data_path = std::path::Path::new(&data_dir);
    std::fs::create_dir_all(data_path)?;
    let db_path = data_path.join("open-ontologies.db");
    let db = StateDb::open(&db_path)?;
    // File-backed Oxigraph rooted at <data_dir>/oxigraph so that separate
    // CLI invocations (load → query → version → clear → rollback) sharing
    // the same --data_dir actually see the same triple set. Fall back to an
    // in-memory store if the on-disk path is unavailable (e.g. concurrent
    // lock held by another `open-ontologies` process) — better to accept
    // reduced isolation than to crash the verb.
    let oxi_path = data_path.join("oxigraph");
    let graph = match GraphStore::open(&oxi_path) {
        Ok(g) => Arc::new(g),
        Err(_) => Arc::new(GraphStore::new()),
    };
    Ok((db, graph))
}

/// Convert an anyhow error into a NounVerbError.
///
/// Used in every verb handler to convert `anyhow::Error` (from `setup()` and
/// library calls) into the `NounVerbError` type that the `#[verb]` macro
/// expects.
///
/// # Examples
///
/// ```no_run
/// // Typical usage inside a verb:
/// //
/// //   let (db, graph) = setup(data_dir).map_err(to_verb_err)?;
/// //
/// // Any type that implements Display can be wrapped:
/// //
/// //   some_call().map_err(to_verb_err)?;
/// ```
pub fn to_verb_err(e: impl std::fmt::Display) -> clap_noun_verb::NounVerbError {
    clap_noun_verb::NounVerbError::execution_error(e.to_string())
}
