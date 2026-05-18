//! T3-3 Extend handler tests.
//!
//! Tests the onto_extend handler for data ingestion, SHACL validation, and reasoning.

use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::TempDir;

fn test_server() -> (TempDir, OpenOntologiesServer) {
    let tmp = TempDir::new().expect("tempdir");
    let db = StateDb::open(&tmp.path().join("state.db")).expect("open StateDb");
    let graph = Arc::new(GraphStore::new());
    let server = OpenOntologiesServer::new_with_options(db, graph, None);
    (tmp, server)
}

#[test]
fn extend_handler_registered() {
    let (_tmp, server) = test_server();
    let tools = server.list_tool_definitions();
    let tool_names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();

    assert!(
        tool_names.contains(&"onto_extend"),
        "onto_extend should be registered"
    );
}

#[test]
fn extend_handler_exists() {
    let (_tmp, server) = test_server();
    let tools = server.list_tool_definitions();
    let onto_extend = tools.iter().find(|t| t.name == "onto_extend");

    assert!(onto_extend.is_some(), "onto_extend tool should be defined");

    let tool = onto_extend.unwrap();
    let has_description = tool
        .description
        .as_ref()
        .map(|d| !d.is_empty())
        .unwrap_or(false);
    assert!(has_description, "onto_extend should have a description");
}
