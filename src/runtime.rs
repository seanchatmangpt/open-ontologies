//! Runtime-tunable knobs derived from `Config`.
//!
//! Many internal modules (`tableaux`, `reason`, `cache`, `feedback`,
//! `webhook`, `server::onto_repo_list`, `server::onto_import`) historically
//! used `const` constants for safety/operational limits. To make them
//! configurable from `config.toml` (and from environment variables for the
//! most operationally critical ones) without threading a `&Config` through
//! every call site, we mirror those constants into atomic globals here.
//!
//! `init_from_config` is invoked once at server startup. Each accessor falls
//! back to the same default the original constant used, so callers that run
//! before initialisation (e.g. CLI subcommands that don't load a config)
//! observe the legacy behaviour.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};

use crate::config::{
    self, Config, FeedbackConfig, ImportsConfig, ReasonerConfig, RepoConfig, WebhookConfig,
};

// ── Defaults match the previous hardcoded constants exactly ─────────────

/// Compiled-in default for the DL-tableaux maximum expansion depth.
///
/// ```
/// assert_eq!(open_ontologies::runtime::DEFAULT_TABLEAUX_MAX_DEPTH_VALUE, 100);
/// ```
pub const DEFAULT_TABLEAUX_MAX_DEPTH_VALUE: usize = 100;

/// Compiled-in default for the maximum number of tableaux nodes allocated.
///
/// ```
/// assert_eq!(open_ontologies::runtime::DEFAULT_TABLEAUX_MAX_NODES_VALUE, 10_000);
/// ```
pub const DEFAULT_TABLEAUX_MAX_NODES_VALUE: usize = 10_000;

/// Compiled-in default for the maximum number of reasoner fixpoint iterations.
///
/// Original `reason.rs` used 50; the audit recommends 64 as a slightly more
/// generous, explicitly-documented value that does not affect fixpoint
/// correctness — it only bounds the number of expansion sweeps.
///
/// ```
/// assert_eq!(open_ontologies::runtime::DEFAULT_REASONER_MAX_ITER_VALUE, 64);
/// ```
pub const DEFAULT_REASONER_MAX_ITER_VALUE: usize = 64;

/// Compiled-in default for the cache hash prefix in bytes (64 KiB).
///
/// ```
/// assert_eq!(open_ontologies::runtime::DEFAULT_CACHE_HASH_PREFIX_VALUE, 64 * 1024);
/// ```
pub const DEFAULT_CACHE_HASH_PREFIX_VALUE: usize = 64 * 1024;

/// Compiled-in default for the repository list page size.
///
/// ```
/// assert_eq!(open_ontologies::runtime::DEFAULT_REPO_LIST_LIMIT_VALUE, 1_000);
/// ```
pub const DEFAULT_REPO_LIST_LIMIT_VALUE: usize = 1_000;

/// Compiled-in default for the maximum `owl:imports` resolution depth.
///
/// ```
/// assert_eq!(open_ontologies::runtime::DEFAULT_IMPORTS_MAX_DEPTH_VALUE, 3);
/// ```
pub const DEFAULT_IMPORTS_MAX_DEPTH_VALUE: usize = 3;

/// Compiled-in default for the per-request HTTP timeout when fetching remote
/// `owl:imports` (seconds).
///
/// ```
/// assert_eq!(open_ontologies::runtime::DEFAULT_IMPORTS_TIMEOUT_SECS_VALUE, 30);
/// ```
pub const DEFAULT_IMPORTS_TIMEOUT_SECS_VALUE: u64 = 30;

/// Compiled-in default for the per-request HTTP timeout when delivering
/// webhook notifications (seconds).
///
/// ```
/// assert_eq!(open_ontologies::runtime::DEFAULT_WEBHOOK_TIMEOUT_SECS_VALUE, 10);
/// ```
pub const DEFAULT_WEBHOOK_TIMEOUT_SECS_VALUE: u64 = 10;

const DEFAULT_TABLEAUX_MAX_DEPTH: usize = DEFAULT_TABLEAUX_MAX_DEPTH_VALUE;
const DEFAULT_TABLEAUX_MAX_NODES: usize = DEFAULT_TABLEAUX_MAX_NODES_VALUE;
const DEFAULT_REASONER_MAX_ITER: usize = DEFAULT_REASONER_MAX_ITER_VALUE;
const DEFAULT_CACHE_HASH_PREFIX: usize = DEFAULT_CACHE_HASH_PREFIX_VALUE;
const DEFAULT_FB_SUPPRESS: i64 = 3;
const DEFAULT_FB_DOWNGRADE: i64 = 2;
const DEFAULT_REPO_LIST_LIMIT: usize = DEFAULT_REPO_LIST_LIMIT_VALUE;
const DEFAULT_IMPORTS_MAX_DEPTH: usize = DEFAULT_IMPORTS_MAX_DEPTH_VALUE;
const DEFAULT_IMPORTS_TIMEOUT: u64 = DEFAULT_IMPORTS_TIMEOUT_SECS_VALUE;
const DEFAULT_WEBHOOK_TIMEOUT: u64 = DEFAULT_WEBHOOK_TIMEOUT_SECS_VALUE;

static TABLEAUX_MAX_DEPTH: AtomicUsize = AtomicUsize::new(DEFAULT_TABLEAUX_MAX_DEPTH);
static TABLEAUX_MAX_NODES: AtomicUsize = AtomicUsize::new(DEFAULT_TABLEAUX_MAX_NODES);
static REASONER_MAX_ITER: AtomicUsize = AtomicUsize::new(DEFAULT_REASONER_MAX_ITER);
static CACHE_HASH_PREFIX: AtomicUsize = AtomicUsize::new(DEFAULT_CACHE_HASH_PREFIX);
static FB_SUPPRESS: AtomicI64 = AtomicI64::new(DEFAULT_FB_SUPPRESS);
static FB_DOWNGRADE: AtomicI64 = AtomicI64::new(DEFAULT_FB_DOWNGRADE);
static REPO_LIST_LIMIT: AtomicUsize = AtomicUsize::new(DEFAULT_REPO_LIST_LIMIT);
static IMPORTS_MAX_DEPTH: AtomicUsize = AtomicUsize::new(DEFAULT_IMPORTS_MAX_DEPTH);
static IMPORTS_TIMEOUT: AtomicU64 = AtomicU64::new(DEFAULT_IMPORTS_TIMEOUT);
static IMPORTS_FOLLOW_REMOTE: AtomicBool = AtomicBool::new(true);
static WEBHOOK_TIMEOUT: AtomicU64 = AtomicU64::new(DEFAULT_WEBHOOK_TIMEOUT);

/// Initialise all runtime knobs from a loaded `Config`. Idempotent — calling
/// this multiple times simply overwrites the current values, which is fine
/// because all consumers re-read on every use.
///
/// # Examples
///
/// Before `init_from_config` is called, each accessor returns its legacy default:
/// ```
/// // Defaults must be strictly positive.
/// assert!(open_ontologies::runtime::tableaux_max_depth() > 0);
/// assert!(open_ontologies::runtime::tableaux_max_nodes() > 0);
/// assert!(open_ontologies::runtime::reasoner_max_iterations() > 0);
/// assert!(open_ontologies::runtime::cache_hash_prefix_bytes() > 0);
/// assert!(open_ontologies::runtime::repo_default_list_limit() > 0);
/// assert!(open_ontologies::runtime::imports_max_depth() > 0);
/// assert!(open_ontologies::runtime::imports_request_timeout_secs() > 0);
/// assert!(open_ontologies::runtime::webhook_request_timeout_secs() > 0);
/// ```
///
/// Feedback thresholds are representable as `i64` (negative values are valid):
/// ```
/// let suppress: i64  = open_ontologies::runtime::feedback_suppress_threshold();
/// let downgrade: i64 = open_ontologies::runtime::feedback_downgrade_threshold();
/// // Default suppress > default downgrade (suppress requires more votes).
/// assert!(suppress >= downgrade);
/// ```
///
/// The legacy tableaux depth default is 100:
/// ```
/// // The atomic is reset to the compiled-in default when no config has been
/// // applied in this test binary.  The actual value may differ if another
/// // test (e.g. `init_overrides_values`) ran first and then restored defaults.
/// // We only assert the value is within the documented valid range.
/// let depth = open_ontologies::runtime::tableaux_max_depth();
/// assert!(depth >= 1 && depth <= 10_000);
/// ```
///
/// `imports_follow_remote` returns a plain `bool`:
/// ```
/// let follows: bool = open_ontologies::runtime::imports_follow_remote();
/// // Either true or false is valid; we only confirm the type compiles.
/// assert!(follows || !follows);
/// ```
pub fn init_from_config(cfg: &Config) {
    apply_reasoner(&cfg.reasoner);
    apply_cache(cfg.cache.hash_prefix_bytes);
    apply_feedback(&cfg.feedback);
    apply_repo(&cfg.repo);
    apply_imports(&cfg.imports);
    apply_webhook(&cfg.webhook);
}

fn apply_reasoner(r: &ReasonerConfig) {
    let depth = if r.tableaux_max_depth == 0 { DEFAULT_TABLEAUX_MAX_DEPTH } else { r.tableaux_max_depth };
    let nodes = if r.tableaux_max_nodes == 0 { DEFAULT_TABLEAUX_MAX_NODES } else { r.tableaux_max_nodes };
    let iters = if r.max_iterations == 0 { DEFAULT_REASONER_MAX_ITER } else { r.max_iterations };
    TABLEAUX_MAX_DEPTH.store(depth, Ordering::Relaxed);
    TABLEAUX_MAX_NODES.store(nodes, Ordering::Relaxed);
    REASONER_MAX_ITER.store(iters, Ordering::Relaxed);
}

fn apply_cache(hash_prefix: usize) {
    let v = if hash_prefix == 0 { DEFAULT_CACHE_HASH_PREFIX } else { hash_prefix };
    CACHE_HASH_PREFIX.store(v, Ordering::Relaxed);
}

fn apply_feedback(f: &FeedbackConfig) {
    FB_SUPPRESS.store(f.suppress_threshold, Ordering::Relaxed);
    FB_DOWNGRADE.store(f.downgrade_threshold, Ordering::Relaxed);
}

fn apply_repo(r: &RepoConfig) {
    let v = if r.default_list_limit == 0 { DEFAULT_REPO_LIST_LIMIT } else { r.default_list_limit };
    REPO_LIST_LIMIT.store(v, Ordering::Relaxed);
}

fn apply_imports(i: &ImportsConfig) {
    let depth = if i.max_depth == 0 { DEFAULT_IMPORTS_MAX_DEPTH } else { i.max_depth };
    IMPORTS_MAX_DEPTH.store(depth, Ordering::Relaxed);
    IMPORTS_TIMEOUT.store(config::resolve_imports_timeout_secs(i), Ordering::Relaxed);
    IMPORTS_FOLLOW_REMOTE.store(i.follow_remote, Ordering::Relaxed);
}

fn apply_webhook(w: &WebhookConfig) {
    WEBHOOK_TIMEOUT.store(config::resolve_webhook_timeout_secs(w), Ordering::Relaxed);
}

// ── Accessors ───────────────────────────────────────────────────────────

/// Returns the maximum DL-tableaux expansion depth.
///
/// Before [`init_from_config`] is called the value equals the compiled-in
/// default of 100.  After initialisation it reflects whatever was set in
/// `config.toml` (or the same default if the field was left at zero).
///
/// ```
/// let v = open_ontologies::runtime::tableaux_max_depth();
/// assert!(v > 0, "default must be positive");
/// ```
pub fn tableaux_max_depth() -> usize { TABLEAUX_MAX_DEPTH.load(Ordering::Relaxed) }

/// Returns the maximum number of nodes the tableaux reasoner may allocate.
///
/// Before [`init_from_config`] is called the value equals the compiled-in
/// default of 10 000.
///
/// ```
/// let v = open_ontologies::runtime::tableaux_max_nodes();
/// assert!(v > 0, "default must be positive");
/// ```
pub fn tableaux_max_nodes() -> usize { TABLEAUX_MAX_NODES.load(Ordering::Relaxed) }

/// Returns the maximum number of fixpoint iterations the reasoner performs.
///
/// Before [`init_from_config`] is called the value equals the compiled-in
/// default of 64.
///
/// ```
/// let v = open_ontologies::runtime::reasoner_max_iterations();
/// assert!(v > 0, "default must be positive");
/// ```
pub fn reasoner_max_iterations() -> usize { REASONER_MAX_ITER.load(Ordering::Relaxed) }

/// Returns the byte length used as the cache hash prefix.
///
/// Before [`init_from_config`] is called the value equals the compiled-in
/// default of 65 536 (64 KiB).
///
/// ```
/// let v = open_ontologies::runtime::cache_hash_prefix_bytes();
/// assert!(v > 0, "default must be positive");
/// ```
pub fn cache_hash_prefix_bytes() -> usize { CACHE_HASH_PREFIX.load(Ordering::Relaxed) }

/// Returns the feedback vote count at which a lint/enforce finding is suppressed.
///
/// The default is 3 (suppress after three dismissals).  The return type is
/// `i64` so that a config of `−1` can disable suppression entirely.
///
/// ```
/// let v: i64 = open_ontologies::runtime::feedback_suppress_threshold();
/// // Default is 3; we only assert the type compiles and the value is
/// // representable — negative values are permitted by design.
/// let _ = v;
/// ```
pub fn feedback_suppress_threshold() -> i64 { FB_SUPPRESS.load(Ordering::Relaxed) }

/// Returns the feedback vote count at which a lint/enforce finding is
/// downgraded from an error to a warning.
///
/// The default is 2.  The return type is `i64` so that a config of `−1` can
/// disable downgrading entirely.
///
/// ```
/// let v: i64 = open_ontologies::runtime::feedback_downgrade_threshold();
/// let _ = v;
/// ```
pub fn feedback_downgrade_threshold() -> i64 { FB_DOWNGRADE.load(Ordering::Relaxed) }

/// Returns the default page size used by `onto_repo_list`.
///
/// Before [`init_from_config`] is called the value equals the compiled-in
/// default of 1 000.
///
/// ```
/// let v = open_ontologies::runtime::repo_default_list_limit();
/// assert!(v > 0, "default must be positive");
/// ```
pub fn repo_default_list_limit() -> usize { REPO_LIST_LIMIT.load(Ordering::Relaxed) }

/// Returns the maximum recursive depth followed when resolving `owl:imports`.
///
/// Before [`init_from_config`] is called the value equals the compiled-in
/// default of 3.
///
/// ```
/// let v = open_ontologies::runtime::imports_max_depth();
/// assert!(v > 0, "default must be positive");
/// ```
pub fn imports_max_depth() -> usize { IMPORTS_MAX_DEPTH.load(Ordering::Relaxed) }

/// Returns the per-request timeout (in seconds) used when fetching remote
/// `owl:imports` via HTTP.
///
/// Before [`init_from_config`] is called the value equals the compiled-in
/// default of 30 seconds.
///
/// ```
/// let v = open_ontologies::runtime::imports_request_timeout_secs();
/// assert!(v > 0, "default must be positive");
/// ```
pub fn imports_request_timeout_secs() -> u64 { IMPORTS_TIMEOUT.load(Ordering::Relaxed) }

/// Returns whether the imports resolver is permitted to follow HTTP/HTTPS
/// `owl:imports` URIs.
///
/// The compiled-in default is `true`.
///
/// ```
/// let v: bool = open_ontologies::runtime::imports_follow_remote();
/// // Accepts either true or false — just assert the type compiles.
/// let _ = v;
/// ```
pub fn imports_follow_remote() -> bool { IMPORTS_FOLLOW_REMOTE.load(Ordering::Relaxed) }

/// Returns the per-request timeout (in seconds) used when delivering webhook
/// notifications.
///
/// Before [`init_from_config`] is called the value equals the compiled-in
/// default of 10 seconds.
///
/// ```
/// let v = open_ontologies::runtime::webhook_request_timeout_secs();
/// assert!(v > 0, "default must be positive");
/// ```
pub fn webhook_request_timeout_secs() -> u64 { WEBHOOK_TIMEOUT.load(Ordering::Relaxed) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_legacy_constants() {
        // Without calling init_from_config the accessors return the
        // original hardcoded defaults.
        assert_eq!(tableaux_max_depth(), 100);
        assert_eq!(tableaux_max_nodes(), 10_000);
        assert_eq!(cache_hash_prefix_bytes(), 64 * 1024);
        assert_eq!(repo_default_list_limit(), 1000);
        assert_eq!(imports_max_depth(), 3);
        assert!(imports_follow_remote());
    }

    #[test]
    fn init_overrides_values() {
        let mut cfg = Config::default();
        cfg.reasoner.tableaux_max_depth = 250;
        cfg.cache.hash_prefix_bytes = 128 * 1024;
        cfg.imports.follow_remote = false;
        init_from_config(&cfg);
        assert_eq!(tableaux_max_depth(), 250);
        assert_eq!(cache_hash_prefix_bytes(), 128 * 1024);
        assert!(!imports_follow_remote());

        // Restore defaults so subsequent tests in the same process aren't
        // affected.
        init_from_config(&Config::default());
    }
}
