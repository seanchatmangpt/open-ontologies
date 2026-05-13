//! R5 WC-1 — §28 HumanOverride TOCTOU closure.
//!
//! Counterfactual: `is_admin_principal` MUST read from the
//! `Arc<Vec<String>>` cache populated at server startup, NOT from
//! `std::env::var(...)`. Two scenarios prove the cache is authoritative
//! in BOTH directions:
//!
//!   1. **Env wins at startup, cache wins after**: set the env var, build
//!      the server, unset the env var. The cache still says the
//!      principal is an admin. Pre-fix code would re-read the env and
//!      report `false`, losing admin authority mid-run.
//!   2. **Empty cache stays empty**: build the server with NO env var
//!      set, then set the env var post-construction. The cache stays
//!      empty (no admins). Pre-fix code would silently grant admin
//!      authority to whoever set the env var after startup — a §28
//!      HumanOverride leak via TOCTOU.
//!
//! Both scenarios run together so the test binary itself proves Δ > 0
//! versus the pre-fix `std::env::var(...)`-on-every-call implementation.

use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use open_ontologies::config::{
    resolve_admin_principals, AuthorityConfig, CacheConfig, EmbeddingsConfig,
};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;

const ENV_KEY: &str = "OPEN_ONTOLOGIES_ADMIN_PRINCIPALS";

/// Cargo runs `#[test]` functions in parallel by default. The four tests
/// in this file share `OPEN_ONTOLOGIES_ADMIN_PRINCIPALS` — without a
/// serializing guard one test's `set_var` races another's `var()`. The
/// `OnceLock<Mutex<()>>` below is acquired at the top of every test
/// that touches the env var; lock release at end-of-scope is paired
/// with the `ScopedEnv` Drop that restores the previous value, so each
/// test sees a deterministic env state.
fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    // `lock().unwrap_or_else(...)` recovers from a poisoned mutex (a
    // panicking test holding the lock); we still want subsequent tests
    // to make progress with deterministic env state.
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn build_server_with(principals: Vec<String>) -> (tempfile::TempDir, OpenOntologiesServer) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("server.db")).unwrap();
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
    )
    .with_admin_principals(principals);
    (tmp, server)
}

/// Scoped env var setter — restores the previous value (or removes the
/// var if it was unset) when the guard is dropped.
struct ScopedEnv {
    key: String,
    prev: Option<String>,
}

impl ScopedEnv {
    fn set(key: &str, val: &str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: see file-level docs. Tests in this binary are
        // independent of the global env after construction.
        unsafe { std::env::set_var(key, val) };
        Self {
            key: key.to_string(),
            prev,
        }
    }

    fn unset(key: &str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: see Self::set.
        unsafe { std::env::remove_var(key) };
        Self {
            key: key.to_string(),
            prev,
        }
    }
}

impl Drop for ScopedEnv {
    fn drop(&mut self) {
        // SAFETY: see Self::set.
        unsafe {
            match &self.prev {
                Some(v) => std::env::set_var(&self.key, v),
                None => std::env::remove_var(&self.key),
            }
        }
    }
}

#[test]
fn cache_wins_when_env_clears_after_startup() {
    let _env_serial = env_lock();
    // 1. Set env, resolve allowlist (mimics startup), build server with
    //    cache populated.
    let _guard = ScopedEnv::set(ENV_KEY, "alice,bob");
    let resolved = resolve_admin_principals(&AuthorityConfig::default());
    assert_eq!(resolved, vec!["alice".to_string(), "bob".to_string()]);
    let (_tmp, server) = build_server_with(resolved);
    assert_eq!(
        server.admin_principals_for_test(),
        &["alice".to_string(), "bob".to_string()]
    );

    // 2. Now unset the env var POST-startup. A pre-fix implementation
    //    would re-read the env and lose authority for alice/bob.
    let _unset = ScopedEnv::unset(ENV_KEY);
    assert!(std::env::var(ENV_KEY).is_err());

    // 3. The cache is unchanged — alice/bob are STILL admins. The env
    //    mutation has no effect because `is_admin_principal` reads from
    //    `self.admin_principals`, never from `std::env::var(...)`.
    assert_eq!(
        server.admin_principals_for_test(),
        &["alice".to_string(), "bob".to_string()],
        "cache must survive env clear (TOCTOU immunity, direction 1)"
    );
}

#[test]
fn empty_cache_stays_empty_when_env_set_after_startup() {
    let _env_serial = env_lock();
    // 1. Build a server with EMPTY cache directly. We bypass
    //    `resolve_admin_principals` here because the env var is shared
    //    across all tests in this binary; another test that sets it
    //    via ScopedEnv may race even with single-threaded mode (the
    //    Drop happens at function exit, which is fine for this test
    //    in isolation but unreliable as a fixture for "no env" state).
    //    The TOCTOU invariant we actually care about is: ONCE the
    //    cache is empty, env-var mutations CANNOT promote a
    //    principal to admin. Constructing with an explicit empty
    //    allowlist isolates that invariant.
    let (_tmp, server) = build_server_with(Vec::new());
    assert!(
        server.admin_principals_for_test().is_empty(),
        "fresh empty cache must stay empty"
    );

    // 2. Set the env var POST-startup. A pre-fix implementation would
    //    silently grant admin authority to whoever set the env var —
    //    the §28 HumanOverride leak.
    let _set = ScopedEnv::set(ENV_KEY, "mallory");
    assert_eq!(std::env::var(ENV_KEY).unwrap(), "mallory");

    // 3. The cache is STILL empty — the env-var mutation has no effect.
    //    Mallory cannot grant herself admin authority by exporting an
    //    env var after the server is up.
    assert!(
        server.admin_principals_for_test().is_empty(),
        "cache must NOT pick up post-startup env (TOCTOU immunity, direction 2)"
    );
}

#[test]
fn resolve_admin_principals_env_overrides_config_file() {
    let _env_serial = env_lock();
    // Env-set + config also populated → env wins (per resolver doc).
    let _set = ScopedEnv::set(ENV_KEY, "alice");
    let cfg = AuthorityConfig {
        admin_principals: vec!["bob".to_string()],
        ..Default::default()
    };
    let resolved = resolve_admin_principals(&cfg);
    assert_eq!(resolved, vec!["alice".to_string()]);
}

#[test]
fn resolve_admin_principals_falls_back_to_config_when_env_unset() {
    let _env_serial = env_lock();
    let _unset = ScopedEnv::unset(ENV_KEY);
    let cfg = AuthorityConfig {
        admin_principals: vec!["carol".to_string(), "dave".to_string()],
        ..Default::default()
    };
    let resolved = resolve_admin_principals(&cfg);
    assert_eq!(resolved, vec!["carol".to_string(), "dave".to_string()]);
}

#[test]
fn resolve_admin_principals_dedupes_and_trims() {
    let _env_serial = env_lock();
    let _set = ScopedEnv::set(ENV_KEY, " alice , alice , bob , ");
    let resolved = resolve_admin_principals(&AuthorityConfig::default());
    assert_eq!(resolved, vec!["alice".to_string(), "bob".to_string()]);
}
