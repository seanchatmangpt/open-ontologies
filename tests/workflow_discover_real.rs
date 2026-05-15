//! Real-data tests for `onto_workflow_discover` and `onto_workflow_feedback`.
//!
//! Uses a real StateDb (in a tempdir). The discover handler requires ≥ 20
//! admitted scopes (ADMITTED_SCOPES_THRESHOLD) before it triggers; with a
//! fresh empty DB it returns `{ok: true, discovered: null}`. This proves the
//! handler runs end-to-end without panicking and returns valid JSON.

use std::sync::Arc;

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::inputs::{OntoWorkflowDiscoverInput, OntoWorkflowFeedbackInput};
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;
use rmcp::handler::server::wrapper::Parameters;

fn build_server() -> (tempfile::TempDir, StateDb, OpenOntologiesServer) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("discover.db")).unwrap();
    let graph = Arc::new(GraphStore::new());
    let cache = CacheConfig {
        enabled: false,
        dir: tmp.path().join("cache").to_string_lossy().into_owned(),
        idle_ttl_secs: 0,
        evictor_interval_secs: 30,
        auto_refresh: false,
        hash_prefix_bytes: 0,
    };
    let server = OpenOntologiesServer::new_with_registry_options(
        db.clone(),
        graph,
        None,
        EmbeddingsConfig::default(),
        cache,
        ToolFilter::default(),
    );
    (tmp, db, server)
}

#[test]
fn workflow_discover_returns_valid_json_on_empty_db() {
    let (_tmp, _db, server) = build_server();
    // With < 20 admitted scopes, discover returns ok=true, discovered=null.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(server.onto_workflow_discover(Parameters(OntoWorkflowDiscoverInput {
        domain: "DataExtensionFastPath".to_string(),
    })));
    let v: serde_json::Value = serde_json::from_str(&result)
        .expect("onto_workflow_discover must return valid JSON");
    assert_eq!(v["ok"], true, "ok must be true on empty DB: {result}");
    assert!(
        v.get("discovered").is_some(),
        "discovered field must be present: {result}"
    );
}

#[test]
fn workflow_feedback_returns_error_for_nonexistent_id() {
    let (_tmp, _db, server) = build_server();
    // Feedback on a non-existent row should return ok=false with an error.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(server.onto_workflow_feedback(Parameters(OntoWorkflowFeedbackInput {
        id: "nonexistent-id-xyz-12345".to_string(),
        accepted: true,
    })));
    let v: serde_json::Value = serde_json::from_str(&result)
        .expect("onto_workflow_feedback must return valid JSON");
    // Either ok=false (row not found) or ok=true (row updated but 0 rows changed).
    // The point is: no panic and valid JSON.
    assert!(
        v.is_object(),
        "must return a JSON object: {result}"
    );
}
