//! T3-3 Extend handler tests.
//!
//! Tests the onto_extend handler for data ingestion, SHACL validation, and reasoning.

use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use std::sync::Arc;

#[test]
fn extend_handler_registered() {
    let db = StateDb::memory().expect("create memory db");
    let graph = GraphStore::new();
    let server = OpenOntologiesServer::new(db, graph, None);

    let tools = server.list_tool_definitions();
    let tool_names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();

    assert!(
        tool_names.contains(&"onto_extend"),
        "onto_extend should be registered"
    );
}

#[test]
fn extend_handler_exists() {
    let db = StateDb::memory().expect("create memory db");
    let graph = GraphStore::new();
    let server = OpenOntologiesServer::new(db, graph, None);

    let tools = server.list_tool_definitions();
    let onto_extend = tools.iter().find(|t| t.name == "onto_extend");

    assert!(
        onto_extend.is_some(),
        "onto_extend tool should be defined"
    );

    let tool = onto_extend.unwrap();
    assert!(
        !tool.description.is_empty(),
        "onto_extend should have a description"
    );
}
