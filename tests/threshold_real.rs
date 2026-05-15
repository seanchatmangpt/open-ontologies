//! Real-data tests for `onto_threshold_status` and `onto_threshold_sweep`.
//!
//! Uses a real StateDb (in a tempdir) and calls the MCP handler methods
//! in-process. Proves:
//!   1. `onto_threshold_status` returns valid JSON on an empty DB.
//!   2. After seeding a workflow_thresholds row, the status response reflects it.
//!   3. `onto_threshold_sweep` runs end-to-end without panicking.

use std::sync::Arc;

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;

fn build_server() -> (tempfile::TempDir, StateDb, OpenOntologiesServer) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("threshold.db")).unwrap();
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
fn threshold_status_returns_valid_json_on_empty_db() {
    let (_tmp, _db, server) = build_server();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(server.onto_threshold_status());
    let v: serde_json::Value = serde_json::from_str(&result)
        .expect("onto_threshold_status must return valid JSON");
    assert_eq!(v["ok"], true, "ok must be true on empty DB: {result}");
    assert_eq!(v["count"], 0, "count must be 0 on empty DB: {result}");
    let thresholds = v["thresholds"].as_array().expect("thresholds must be an array");
    assert!(thresholds.is_empty(), "thresholds must be empty: {result}");
}

#[test]
fn threshold_status_reflects_seeded_row() {
    let (_tmp, db, server) = build_server();

    // Seed a row directly into workflow_thresholds.
    {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO workflow_thresholds
             (workflow_class, precision_threshold, fitness_threshold, sample_count, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["DataExtensionFastPath", 0.85, 0.90, 42, "2026-01-01T00:00:00Z"],
        )
        .expect("insert workflow_thresholds");
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(server.onto_threshold_status());
    let v: serde_json::Value = serde_json::from_str(&result)
        .expect("onto_threshold_status must return valid JSON");
    assert_eq!(v["ok"], true, "ok must be true: {result}");
    assert_eq!(v["count"], 1, "count must be 1 after seeding: {result}");
    let thresholds = v["thresholds"].as_array().expect("thresholds must be an array");
    assert_eq!(thresholds.len(), 1);
    assert_eq!(thresholds[0]["workflow_class"], "DataExtensionFastPath");
    assert!(
        thresholds[0]["fitness_threshold"].as_f64().unwrap() > 0.0,
        "fitness_threshold must be present: {result}"
    );
}

#[test]
fn threshold_sweep_runs_without_error() {
    let (_tmp, _db, server) = build_server();
    // With an empty DB, sweep should complete cleanly (no bypass events to sweep).
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(server.onto_threshold_sweep());
    let v: serde_json::Value = serde_json::from_str(&result)
        .expect("onto_threshold_sweep must return valid JSON");
    assert_eq!(v["ok"], true, "sweep must succeed on empty DB: {result}");
}
