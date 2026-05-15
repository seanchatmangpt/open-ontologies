//! Real-data tests for `onto_conformance_check`.
//!
//! Seeds `declared_workflows` with a simple POWL workflow string, then calls
//! the MCP handler to replay OCEL events against it. With an empty OCEL trace
//! the replay runs against an empty trace — fitness and precision are still
//! populated (typically 1.0 and 0.0 respectively for an empty trace).

use std::sync::Arc;

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::inputs::OntoConformanceCheckInput;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;
use rmcp::handler::server::wrapper::Parameters;

const SCOPE_TOKEN: &str = "test-conformance-check-scope";
const POWL_STRING: &str =
    "PO=(nodes={load, extend, query}, order={load-->extend, extend-->query})";

fn build_server() -> (tempfile::TempDir, StateDb, OpenOntologiesServer) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("conformance.db")).unwrap();
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

/// Seed a declared_workflows row with a minimal POWL string.
fn seed_declared_workflow(db: &StateDb) {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO declared_workflows \
         (scope_token, session_id, name, powl_string, powl_hash, \
          alphabet_json, declared_at, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            SCOPE_TOKEN,
            "test-session-conformance",
            "TestWorkflow",
            POWL_STRING,
            blake3::hash(POWL_STRING.as_bytes()).to_hex().to_string(),
            r#"["load","extend","query"]"#,
            "2026-01-01T00:00:00Z",
            "open",
        ],
    )
    .expect("insert declared_workflows");
}

#[test]
fn conformance_check_returns_valid_json_on_empty_trace() {
    let (_tmp, db, server) = build_server();
    seed_declared_workflow(&db);

    let result = server.onto_conformance_check(Parameters(OntoConformanceCheckInput {
        scope_token: SCOPE_TOKEN.to_string(),
    }));

    let v: serde_json::Value =
        serde_json::from_str(&result).expect("onto_conformance_check must return valid JSON");

    // With an empty OCEL trace, replay returns ok=true and populates the fields.
    assert_eq!(v["ok"], true, "ok must be true: {result}");
    assert!(v.get("fitness").is_some(), "fitness must be present: {result}");
    assert!(v.get("run_id").is_some(), "run_id must be present: {result}");
    assert!(
        v["run_id"].as_str().map_or(false, |s| !s.is_empty()),
        "run_id must be non-empty: {result}"
    );
}

#[test]
fn conformance_check_returns_error_for_missing_scope() {
    let (_tmp, _db, server) = build_server();

    let result = server.onto_conformance_check(Parameters(OntoConformanceCheckInput {
        scope_token: "nonexistent-scope-token".to_string(),
    }));

    let v: serde_json::Value =
        serde_json::from_str(&result).expect("must return valid JSON on missing scope");
    assert_eq!(v["ok"], false, "ok must be false for missing scope: {result}");
}
