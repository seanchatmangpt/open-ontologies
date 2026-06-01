//! Mocked gemini CLI/API integration test for `onto_translate_candidate`.
use std::sync::Arc;

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::inputs::OntoTranslateCandidateInput;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;
use rmcp::handler::server::wrapper::Parameters;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

#[tokio::test(flavor = "multi_thread")]
async fn translate_candidate_gemini_engine_returns_candidate_ctq() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let mock_server = MockServer::start().await;
    let response_body = serde_json::json!({
        "choices": [
            {
                "message": {
                    "content": "{\"defect_class_hint\":\"SupportResponseDelay\",\"ctq_text\":\"Respond to tickets within 4 hours\",\"measure_text\":\"Time between submission and response\",\"verification_text\":\"Timestamp delta\",\"negative_case_text\":\"Response after 4 hours\",\"control_plan_text\":\"Escalate to manager\",\"verdict\":true,\"refinements\":0,\"provisional\":true}"
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
        .onto_translate_candidate(Parameters(OntoTranslateCandidateInput {
            source_voice: "When a customer submits a support ticket, we need to respond within 4 hours or the ticket escalates automatically.".to_string(),
            scope_token: "test-gemini-translate".to_string(),
            engine: Some("gemini".to_string()),
            python: None,
        }))
        .await;

    unsafe {
        std::env::remove_var("OPEN_ONTOLOGIES_LLM_API_BASE");
        std::env::remove_var("OPEN_ONTOLOGIES_LLM_API_KEY");
    }

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
        v["latency_ms"].as_u64().is_some(),
        "latency_ms must be present: {resp}"
    );
    assert!(
        v["candidate_ctq_id"].as_str().is_some(),
        "candidate_ctq_id must be present: {resp}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn translate_candidate_gemini_engine_spawn_failure_returns_error_json() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("OPEN_ONTOLOGIES_LLM_API_BASE", "http://127.0.0.1:1");
        std::env::set_var("OPEN_ONTOLOGIES_LLM_API_KEY", "mock-key");
    }

    let (_tmp, server) = build_server();
    let resp = server
        .onto_translate_candidate(Parameters(OntoTranslateCandidateInput {
            source_voice: "test voice".to_string(),
            scope_token: "test-gemini-spawn-fail".to_string(),
            engine: Some("gemini".to_string()),
            python: None,
        }))
        .await;

    unsafe {
        std::env::remove_var("OPEN_ONTOLOGIES_LLM_API_BASE");
        std::env::remove_var("OPEN_ONTOLOGIES_LLM_API_KEY");
    }

    let v: serde_json::Value =
        serde_json::from_str(&resp).expect(&format!("must return valid JSON even on failure. got: {}", resp));
    assert_eq!(v["ok"], false, "ok must be false on spawn failure: {resp}");
    assert!(
        v["error"].as_str().is_some(),
        "error field must be present: {resp}"
    );
}
