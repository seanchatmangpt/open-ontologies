//! T3-2 SQL ingest handler tests.
//!
//! Tests the onto_sql_ingest handler for DuckDB/PostgreSQL ingestion.
//! Requires the duckdb feature to be enabled.

#![cfg(feature = "duckdb")]

use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use std::sync::Arc;

#[test]
fn sql_ingest_handler_registered() {
    let db = StateDb::memory().expect("create memory db");
    let graph = GraphStore::new();
    let server = OpenOntologiesServer::new(db, graph, None);

    let tools = server.list_tool_definitions();
    let tool_names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();

    assert!(
        tool_names.contains(&"onto_sql_ingest"),
        "onto_sql_ingest should be registered"
    );
}

#[test]
fn sql_ingest_handler_exists() {
    let db = StateDb::memory().expect("create memory db");
    let graph = GraphStore::new();
    let server = OpenOntologiesServer::new(db, graph, None);

    let tools = server.list_tool_definitions();
    let onto_sql_ingest = tools.iter().find(|t| t.name == "onto_sql_ingest");

    assert!(
        onto_sql_ingest.is_some(),
        "onto_sql_ingest tool should be defined"
    );
}
