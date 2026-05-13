//! Phase 8 / Plan 4 — LLM engine resolution tests.
//!
//! Proves the precedence chain (per-call `engine` arg > HTTP header
//! override > server default) and the auto-detect fallback used to
//! populate the server default at construction.
//!
//! These tests are pure — they do NOT call the real Groq subprocess.
//! They construct an `OpenOntologiesServer` and assert resolution
//! behaviour at the helper level, plus they unit-test
//! `config::resolve_llm_engine` directly.
//!
//! Mutating `GROQ_API_KEY` / `OPEN_ONTOLOGIES_LLM_ENGINE` requires
//! `--test-threads=1`; we serialise via `cargo test --test
//! llm_engine_resolution` which runs each `#[test]` sequentially when
//! the test binary is single-threaded. The functions also wrap the
//! mutation+assertion in a guard so a panic in one test does not leak
//! environment into another.

use std::sync::{Arc, Mutex, OnceLock};

use open_ontologies::config::{
    resolve_llm_engine, CacheConfig, EmbeddingsConfig, LlmConfig,
};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;

/// Serialises all env-mutating tests in this binary.
/// `unsafe { std::env::set_var }` is not thread-safe; this mutex ensures
/// only one test touches env vars at a time, matching the `--test-threads=1`
/// contract described in the module-level comment.
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
}

/// RAII guard that restores environment-variable state when dropped, so
/// concurrent test files do not see leaked overrides if a test panics.
struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn capture(keys: &[&'static str]) -> Self {
        let saved = keys
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect();
        Self { saved }
    }

    fn unset_all(&self) {
        for (k, _) in &self.saved {
            // SAFETY: tests run with --test-threads=1.
            unsafe { std::env::remove_var(k); }
        }
    }

    fn set(&self, key: &str, value: &str) {
        // SAFETY: tests run with --test-threads=1.
        unsafe { std::env::set_var(key, value); }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.saved {
            unsafe {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
    }
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

const RELEVANT_KEYS: &[&str] = &[
    "OPEN_ONTOLOGIES_LLM_ENGINE",
    "OPEN_ONTOLOGIES_LLM_API_KEY",
    "GROQ_API_KEY",
];

#[test]
fn default_engine_is_groq_pm4py_when_key_present() {
    let _lock = env_lock();
    let guard = EnvGuard::capture(RELEVANT_KEYS);
    guard.unset_all();
    guard.set("GROQ_API_KEY", "test-key-not-real-not-sent-anywhere");

    // Constructor reads env → resolve_llm_engine sees the key → picks groq_pm4py.
    let (_tmp, server) = build_server();
    assert_eq!(
        server.default_llm_engine(),
        "groq_pm4py",
        "auto-detect should select groq_pm4py when GROQ_API_KEY is present"
    );

    // Confirm the resolver agrees stand-alone.
    let resolved = resolve_llm_engine(&LlmConfig::default());
    assert_eq!(resolved, "groq_pm4py");
}

#[test]
fn header_override_to_inproc_takes_effect() {
    let _lock = env_lock();
    let guard = EnvGuard::capture(RELEVANT_KEYS);
    guard.unset_all();
    guard.set("GROQ_API_KEY", "test-key-not-real-not-sent-anywhere");

    let (_tmp, server) = build_server();
    assert_eq!(server.default_llm_engine(), "groq_pm4py");

    // Per-call override beats header beats default.
    let from_per_call = server.resolve_engine(Some("inproc"), Some("groq_pm4py"));
    assert_eq!(from_per_call, "inproc", "per-call must beat header");

    // Header override beats default when no per-call argument supplied.
    let from_header = server.resolve_engine(None, Some("inproc"));
    assert_eq!(
        from_header, "inproc",
        "header override must flip groq_pm4py default to inproc"
    );

    // No overrides → server default.
    let from_default = server.resolve_engine(None, None);
    assert_eq!(from_default, "groq_pm4py");
}

#[test]
fn key_unset_falls_back_to_inproc() {
    let _lock = env_lock();
    let guard = EnvGuard::capture(RELEVANT_KEYS);
    guard.unset_all();

    // No env, no config, no key → inproc.
    let resolved = resolve_llm_engine(&LlmConfig::default());
    assert_eq!(
        resolved, "inproc",
        "with no key and no override, the resolver must default to inproc"
    );

    let (_tmp, server) = build_server();
    assert_eq!(server.default_llm_engine(), "inproc");

    // Explicit env override still wins even without a key.
    guard.set("OPEN_ONTOLOGIES_LLM_ENGINE", "groq_pm4py");
    let resolved2 = resolve_llm_engine(&LlmConfig::default());
    assert_eq!(
        resolved2, "groq_pm4py",
        "explicit env var must beat auto-detect"
    );

    // Invalid env value is dropped (resolver validates against
    // VALID_LLM_ENGINES) and we fall through to auto-detect.
    guard.set("OPEN_ONTOLOGIES_LLM_ENGINE", "definitely_not_a_real_engine");
    let resolved3 = resolve_llm_engine(&LlmConfig::default());
    assert_eq!(
        resolved3, "inproc",
        "invalid env value must be ignored and auto-detect must fall back to inproc"
    );
}
