//! Real gemini CLI integration test for `onto_translate_candidate`.
//!
//! Uses the headless `gemini` CLI (OAuth, no API key required) as the LLM
//! backend — the same pattern as speckit-ralph's `copilot-shim.sh` /
//! `gemini-invoke.sh`. Invokes:
//!
//!   npx -y @google/gemini-cli -p "<prompt>" --model gemini-3.1-flash-lite --approval-mode yolo
//!
//! Skip conditions (not a test failure):
//!   - `gemini` binary not found in PATH or GEMINI_BIN
//!   - gemini exits non-zero (OAuth not configured)
//!
//! Run with:
//!   cargo test --test real_gemini_translate -- --nocapture

use std::sync::Arc;

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::inputs::OntoTranslateCandidateInput;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;
use rmcp::handler::server::wrapper::Parameters;

fn build_server() -> (tempfile::TempDir, OpenOntologiesServer) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("gemini.db")).unwrap();
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
        db,
        graph,
        None,
        EmbeddingsConfig::default(),
        cache,
        ToolFilter::default(),
    );
    (tmp, server)
}

/// Returns None if the gemini CLI is not available or not authenticated.
fn gemini_available() -> bool {
    let bin = std::env::var("GEMINI_BIN").unwrap_or_else(|_| "gemini".to_string());
    // Try a trivial prompt with a very short timeout to check auth.
    match std::process::Command::new(&bin)
        .args(["-p", "ping", "--model", "gemini-3.1-flash-lite", "--approval-mode", "yolo"])
        .output()
    {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn translate_candidate_gemini_engine_returns_candidate_ctq() {
    if !gemini_available() {
        eprintln!("SKIP: gemini CLI not available or not authenticated");
        return;
    }

    let (_tmp, server) = build_server();
    let resp = server
        .onto_translate_candidate(Parameters(OntoTranslateCandidateInput {
            source_voice: "When a customer submits a support ticket, we need to respond within 4 hours or the ticket escalates automatically.".to_string(),
            scope_token: "test-gemini-translate".to_string(),
            engine: Some("gemini".to_string()),
            python: None,
        }))
        .await;

    let v: serde_json::Value =
        serde_json::from_str(&resp).expect("onto_translate_candidate must return valid JSON");

    assert_eq!(v["ok"], true, "ok must be true: {resp}");
    assert_eq!(v["engine"], "gemini", "engine must be gemini: {resp}");
    assert_eq!(v["provisional"], true, "must be provisional: {resp}");
    assert_eq!(v["_projection_only"], true, "must be projection_only: {resp}");

    let candidate = &v["candidate"];
    assert!(
        !candidate["ctq_text"].as_str().unwrap_or("").is_empty(),
        "ctq_text must be non-empty: {resp}"
    );
    assert!(
        !candidate["measure_text"].as_str().unwrap_or("").is_empty(),
        "measure_text must be non-empty: {resp}"
    );
    assert!(
        v["latency_ms"].as_u64().unwrap_or(0) > 0,
        "latency_ms must be positive: {resp}"
    );
    assert!(
        v["candidate_ctq_id"].as_str().is_some(),
        "candidate_ctq_id must be present: {resp}"
    );

    eprintln!("gemini CTQ translation: ctq_text={:?}", candidate["ctq_text"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn translate_candidate_gemini_engine_spawn_failure_returns_error_json() {
    // Set GEMINI_BIN to a nonexistent path — server must return error JSON, not panic.
    // SAFETY: single-threaded test; no other thread reads GEMINI_BIN concurrently.
    unsafe { std::env::set_var("GEMINI_BIN", "/tmp/nonexistent_gemini_binary_xyz") };

    let (_tmp, server) = build_server();
    let resp = server
        .onto_translate_candidate(Parameters(OntoTranslateCandidateInput {
            source_voice: "test voice".to_string(),
            scope_token: "test-gemini-spawn-fail".to_string(),
            engine: Some("gemini".to_string()),
            python: None,
        }))
        .await;

    // SAFETY: paired with set_var above; restore before assertions.
    unsafe { std::env::remove_var("GEMINI_BIN") };

    let v: serde_json::Value =
        serde_json::from_str(&resp).expect(&format!("must return valid JSON even on failure. got: {}", resp));
    assert_eq!(v["ok"], false, "ok must be false on spawn failure: {resp}");
    assert!(
        v["error"].as_str().is_some(),
        "error field must be present: {resp}"
    );
}
