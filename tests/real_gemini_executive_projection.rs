#![allow(clippy::all, unused)]

//! Mocked gemini CLI/API integration test for `onto_executive_projection`.
use std::sync::Arc;

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::inputs::OntoExecutiveProjectionInput;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;
use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn build_server() -> (tempfile::TempDir, OpenOntologiesServer) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("gemini_exec.db")).unwrap();
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

#[tokio::test(flavor = "multi_thread")]
async fn executive_projection_gemini_engine_returns_summary() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let mock_server = MockServer::start().await;
    let response_body = serde_json::json!({
        "choices": [
            {
                "message": {
                    "content": "{\"summary\":\"The system response time exceeds 4 hours.\",\"key_findings\":[\"response time exceeds 4 hours\"],\"risk_level\":\"high\",\"provisional\":true}"
                }
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    unsafe {
        std::env::set_var("OPEN_ONTOLOGIES_LLM_API_BASE", mock_server.uri());
        std::env::set_var("OPEN_ONTOLOGIES_LLM_API_KEY", "mock-key");
    }

    let (_tmp, server) = build_server();
    let resp = server
        .onto_executive_projection(Parameters(OntoExecutiveProjectionInput {
            scope_token: "test-gemini-exec-proj".to_string(),
            admitted_evidence: "The system response time exceeds 4 hours for critical tickets. \
                               Escalation rate is 23 percent. Customer satisfaction score dropped \
                               from 87 to 71 in Q3. Root cause is insufficient triage staffing."
                .to_string(),
            engine: Some("gemini".to_string()),
            python: None,
        }))
        .await;

    unsafe {
        std::env::remove_var("OPEN_ONTOLOGIES_LLM_API_BASE");
        std::env::remove_var("OPEN_ONTOLOGIES_LLM_API_KEY");
    }

    let v: serde_json::Value =
        serde_json::from_str(&resp).expect("onto_executive_projection must return valid JSON");

    assert_eq!(v["ok"], true, "ok must be true: {resp}");
    assert_eq!(v["engine"], "gemini", "engine must be gemini: {resp}");
    assert_eq!(v["provisional"], true, "must be provisional: {resp}");

    let summary = v["summary"].as_str().unwrap_or("");
    assert!(!summary.is_empty(), "summary must be non-empty: {resp}");

    assert!(
        v["latency_ms"].as_u64().is_some(),
        "latency_ms must be present: {resp}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn executive_projection_gemini_engine_spawn_failure_returns_error_json() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("OPEN_ONTOLOGIES_LLM_API_BASE", "http://127.0.0.1:1");
        std::env::set_var("OPEN_ONTOLOGIES_LLM_API_KEY", "mock-key");
    }

    let (_tmp, server) = build_server();
    let resp = server
        .onto_executive_projection(Parameters(OntoExecutiveProjectionInput {
            scope_token: "test-gemini-exec-spawn-fail".to_string(),
            admitted_evidence: "test evidence body for spawn failure case".to_string(),
            engine: Some("gemini".to_string()),
            python: None,
        }))
        .await;

    unsafe {
        std::env::remove_var("OPEN_ONTOLOGIES_LLM_API_BASE");
        std::env::remove_var("OPEN_ONTOLOGIES_LLM_API_KEY");
    }

    let v: serde_json::Value =
        serde_json::from_str(&resp).expect("must return valid JSON even on failure");
    assert_eq!(v["ok"], false, "ok must be false on spawn failure: {resp}");
    assert!(
        v["error"].as_str().is_some(),
        "error field must be present: {resp}"
    );
}
