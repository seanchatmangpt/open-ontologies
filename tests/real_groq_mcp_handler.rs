//! Phase 8 — REAL Groq MCP-handler integration test.
//!
//! Constructs an `OpenOntologiesServer` directly (no transport) and calls
//! the `onto_translate_candidate`, `onto_executive_projection`, and
//! `onto_groq_status` handlers in-process. The `groq_pm4py` engine path
//! shells out to `scripts/ctq_from_voice.py` / `scripts/executive_projection.py`
//! / `scripts/groq_status.py` and makes REAL Groq API calls via dspy.
//!
//! No mocks. No tokio HTTP listener. No canned JSON. Real LLM call,
//! real network, real provider.
//!
//! SKIP-on-missing-deps mirrors `tests/real_groq_powl.rs`: missing venv /
//! missing key emits an `eprintln` SKIP and returns Ok. CI without local
//! setup does not redden.
//!
//! Run serially:
//!     cargo test --test real_groq_mcp_handler -- --test-threads=1 --nocapture

use std::sync::Arc;

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::inputs::{
    OntoExecutiveProjectionInput, OntoGroqStatusInput, OntoTranslateCandidateInput,
};
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;
use rmcp::handler::server::wrapper::Parameters;

const VENV_PYTHON: &str = "/Users/sac/chatmangpt/ostar/.venv/bin/python";

fn read_groq_key() -> Option<String> {
    let env_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    if let Ok(content) = std::fs::read_to_string(&env_path) {
        for line in content.lines() {
            if let Some(rest) = line.trim().strip_prefix("GROQ_API_KEY=") {
                let v = rest.trim_matches('"').trim_matches('\'').trim();
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    if let Ok(v) = std::env::var("GROQ_API_KEY") {
        if !v.trim().is_empty() {
            return Some(v);
        }
    }
    None
}

fn skip_unless_available() -> Option<String> {
    if !std::path::Path::new(VENV_PYTHON).exists() {
        eprintln!("SKIP: venv python not at {VENV_PYTHON}");
        return None;
    }
    let key = read_groq_key()?;
    if key.is_empty() {
        eprintln!("SKIP: GROQ_API_KEY not set in env or .env");
        return None;
    }
    Some(key)
}

fn build_server() -> (tempfile::TempDir, OpenOntologiesServer) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("s.db")).unwrap();
    let graph = Arc::new(GraphStore::new());
    let cache = CacheConfig {
        enabled: true,
        dir: tmp.path().join("cache").to_string_lossy().into_owned(),
        idle_ttl_secs: 0,
        evictor_interval_secs: 30,
        auto_refresh: false,
        hash_prefix_bytes: 64 * 1024,
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

/// Make sure the subprocess sees the key from .env even if the test process
/// itself was launched without it. The real-Groq scripts read `GROQ_API_KEY`
/// from their own environment, which on macOS is inherited from the parent.
fn install_key_into_env(key: &str) {
    // SAFETY: tests run with --test-threads=1 (per the file-level docstring).
    unsafe {
        std::env::set_var("GROQ_API_KEY", key);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn translate_candidate_groq_pm4py_engine_returns_real_groq_response() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };
    install_key_into_env(&key);

    let (_tmp, server) = build_server();
    let voice = "Sales says deals are real, Finance can't reconcile bookings";

    let input = OntoTranslateCandidateInput {
        scope_token: "scope-test-real-groq".to_string(),
        source_voice: voice.to_string(),
        engine: Some("groq_pm4py".to_string()),
        python: Some(VENV_PYTHON.to_string()),
    };
    let raw = server.onto_translate_candidate(Parameters(input)).await;
    eprintln!("REAL GROQ MCP HANDLER raw: {raw}");

    let resp: serde_json::Value =
        serde_json::from_str(&raw).expect("handler returned non-JSON");

    assert_eq!(
        resp.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "handler ok=false: {resp}"
    );
    assert_eq!(
        resp.get("engine").and_then(|v| v.as_str()),
        Some("groq_pm4py"),
        "engine field missing or wrong"
    );

    let ctq_text = resp
        .get("ctq_text")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        !ctq_text.trim().is_empty(),
        "ctq_text must be non-empty real-Groq response: {resp}"
    );
    assert!(
        ctq_text.len() >= 20,
        "ctq_text below 20-char floor: {ctq_text:?}"
    );

    assert_eq!(
        resp.get("verdict").and_then(|v| v.as_bool()),
        Some(true),
        "real Groq must validate the canonical demo voice as verdict=true: {resp}"
    );

    // CRITICAL — the API key must NEVER appear anywhere in the response,
    // even base64-encoded. The handler scrubs subprocess stderr but we
    // double-check at the boundary.
    assert!(
        !raw.contains(&key),
        "API KEY LEAKED INTO RESPONSE BODY"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn groq_status_returns_truthful_state_when_key_present() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };
    install_key_into_env(&key);

    let (_tmp, server) = build_server();
    let raw = server
        .onto_groq_status(Parameters(OntoGroqStatusInput {
            python: Some(VENV_PYTHON.to_string()),
        }))
        .await;
    eprintln!("REAL GROQ STATUS raw: {raw}");

    let resp: serde_json::Value =
        serde_json::from_str(&raw).expect("groq_status handler returned non-JSON");

    assert_eq!(
        resp.get("key_present").and_then(|v| v.as_bool()),
        Some(true),
        "key_present should be true when GROQ_API_KEY is set: {resp}"
    );
    assert_eq!(
        resp.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "ok should be true (dspy importable + key set): {resp}"
    );

    let model = resp.get("model").and_then(|v| v.as_str()).unwrap_or("");
    assert!(!model.is_empty(), "model must be a non-empty string: {resp}");

    // Defence-in-depth: status response must never carry the key.
    assert!(
        !raw.contains(&key),
        "API KEY LEAKED INTO STATUS RESPONSE BODY"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn executive_projection_groq_pm4py_engine_grounds_in_evidence() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };
    install_key_into_env(&key);

    let (_tmp, server) = build_server();
    let evidence = "Reconciliation completeness rate is 83%. Forecast risk \
                    explainable. Nightly report ran. Refuse missing contract. \
                    Block partial chain.";
    let input = OntoExecutiveProjectionInput {
        scope_token: "scope-test-exec-real".to_string(),
        admitted_evidence: evidence.to_string(),
        engine: Some("groq_pm4py".to_string()),
        python: Some(VENV_PYTHON.to_string()),
    };
    let raw = server.onto_executive_projection(Parameters(input)).await;
    eprintln!("REAL GROQ EXEC raw: {raw}");

    let resp: serde_json::Value =
        serde_json::from_str(&raw).expect("exec handler returned non-JSON");

    assert_eq!(
        resp.get("engine").and_then(|v| v.as_str()),
        Some("groq_pm4py"),
        "engine field missing or wrong: {resp}"
    );
    let summary = resp.get("summary").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        !summary.is_empty(),
        "executive projection summary must be non-empty: {resp}"
    );

    assert!(
        !raw.contains(&key),
        "API KEY LEAKED INTO EXEC PROJECTION RESPONSE BODY"
    );
}
