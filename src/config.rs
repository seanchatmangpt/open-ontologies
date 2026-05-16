//! Runtime configuration for open-ontologies.
//!
//! Loaded from a TOML file (`config.toml` by default, falling back to
//! built-in defaults). Every public `resolve_*` function reads one logical
//! setting in priority order: **env var → TOML field → compiled-in default**.
//!
//! See `config.example.toml` in the repository root for all 14 sections
//! with defaults and env-var overrides documented inline.
//!
//! # Config search path (highest priority first)
//! 1. Path supplied to [`Config::load`]
//! 2. `~/.config/open-ontologies/config.toml`
//! 3. `/etc/open-ontologies/config.toml`
//! 4. Built-in defaults (every `resolve_*` function returns a sane value
//!    with no config file present — suitable for local development)

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub general: GeneralConfig,
    pub embeddings: EmbeddingsConfig,
    pub cache: CacheConfig,
    pub tools: ToolsConfig,
    pub webhook: WebhookConfig,
    pub http: HttpConfig,
    pub monitor: MonitorConfig,
    pub reasoner: ReasonerConfig,
    pub feedback: FeedbackConfig,
    pub imports: ImportsConfig,
    pub repo: RepoConfig,
    pub socket: SocketConfig,
    pub logging: LoggingConfig,
    pub codegen: CodegenConfig,
    pub llm: LlmConfig,
    /// `[retention]` — Round 4 WD §29 Cell8 retirement closure. Per-table
    /// retention windows (in days) for the [`crate::retention::RetentionWorker`]
    /// background job. Defaults are chosen so that single-tenant deployments
    /// with no explicit `[retention]` section still cap unbounded growth.
    pub retention: RetentionConfig,
    /// `[verifier]` — R7 WA2 A2 V1 Receipt-Chain Verifier worker.
    /// Continuous tokio loop; ZERO LLM by invariant — crypto verdicts
    /// must be reproducible bit-for-bit from `(receipt_row,
    /// trusted_keys_history_row)`. See [`crate::verifier_worker`].
    pub verifier: VerifierConfig,
    /// `[authority]` — R5 WC-1 §28 HumanOverride closure. Cached at
    /// startup; subsequent env-var changes are ignored (TOCTOU-immune).
    /// Closed-by-default: an empty `admin_principals` list means NO
    /// admin operations are permitted (admin-only handlers reject all
    /// callers). Operators must opt in explicitly.
    pub authority: AuthorityConfig,
    /// `[telemetry]` — R8-3 OTEL export wiring. When `otlp_endpoint` is set,
    /// `src/telemetry.rs` exports `tracing` spans to the configured collector.
    pub telemetry: TelemetryConfig,
    /// R9-1 — optional external A13 attestation endpoint.
    /// When set (or via `OPEN_ONTOLOGIES_ATTESTATION_ENDPOINT`), `cell_ready`
    /// POSTs the replay+OCEL hash pair to this URL for external witnessing.
    pub attestation_endpoint: Option<String>,
}


impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        Ok(config)
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub data_dir: String,
    /// Directories that act as on-disk ontology repositories. The
    /// `onto_repo_list` and `onto_repo_load` MCP tools enumerate and load
    /// RDF files (.ttl, .nt, .rdf, .owl, .nq, .trig, .jsonld) from these
    /// directories. Useful for containerized deployments where a host
    /// directory of TTL files is mounted into the server.
    ///
    /// Accepts either a TOML array under the canonical name `ontology_dirs`
    /// or, for compatibility with the original design proposal, the alias
    /// `data_dirs`. Each entry has `~` expanded to the user's home.
    #[serde(alias = "data_dirs")]
    pub ontology_dirs: Vec<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            data_dir: "~/.open-ontologies".into(),
            ontology_dirs: Vec::new(),
        }
    }
}

/// Resolve the configured ontology repository directories.
///
/// Behavior:
///  - If the env var `OPEN_ONTOLOGIES_ONTOLOGY_DIRS` is set and non-empty,
///    its value (split on `:` on Unix, `;` on Windows, accepting either on
///    both for convenience) overrides the config entries.
///  - Each entry has `~` expanded.
///  - Empty strings are dropped.
///  - Duplicates (after canonicalization fallback to the expanded string)
///    are removed while preserving order.
pub fn resolve_ontology_dirs(cfg: &[String]) -> Vec<std::path::PathBuf> {
    let from_env = std::env::var("OPEN_ONTOLOGIES_ONTOLOGY_DIRS").ok();
    let raw: Vec<String> = match from_env {
        Some(v) if !v.trim().is_empty() => v
            .split([':', ';'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => cfg.iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
    };
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(raw.len());
    for entry in raw {
        let expanded = expand_tilde(&entry);
        let key = std::fs::canonicalize(&expanded)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| expanded.clone());
        if seen.insert(key) {
            out.push(std::path::PathBuf::from(expanded));
        }
    }
    out
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct EmbeddingsConfig {
    /// Embedding provider: "local" (default — ONNX model on disk) or "openai"
    /// (any OpenAI-compatible HTTP API, e.g. OpenAI, Azure OpenAI, Ollama,
    /// vLLM, LM Studio, LocalAI, Together, etc.). Override at runtime with
    /// `OPEN_ONTOLOGIES_EMBEDDINGS_PROVIDER`.
    pub provider: Option<String>,
    /// Path to the ONNX model file (provider = "local" only).
    /// Default: ~/.open-ontologies/models/bge-small-en-v1.5.onnx
    pub model_path: Option<String>,
    /// Path to the tokenizer.json file (provider = "local" only).
    /// Default: ~/.open-ontologies/models/tokenizer.json
    pub tokenizer_path: Option<String>,
    /// URL to download the ONNX model from. Default: BGE-small-en-v1.5 from Hugging Face
    pub model_url: Option<String>,
    /// URL to download the tokenizer from. Default: BGE-small-en-v1.5 tokenizer from Hugging Face
    pub tokenizer_url: Option<String>,
    /// Filename for the downloaded model. Default: bge-small-en-v1.5.onnx
    pub model_name: Option<String>,

    // ─── OpenAI-compatible provider (provider = "openai") ───────────────
    /// Base URL of the OpenAI-compatible API, without the trailing
    /// `/embeddings` path. Default: `https://api.openai.com/v1`. Override
    /// at runtime with `OPEN_ONTOLOGIES_EMBEDDINGS_API_BASE`.
    #[serde(alias = "base_url")]
    pub api_base: Option<String>,
    /// API key. If unset, falls back to the `OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY`
    /// or `OPENAI_API_KEY` env var. Sent as `Authorization: Bearer <key>`.
    /// Optional — gateways that don't require auth (Ollama, LocalAI,
    /// vLLM behind a private network, …) can leave this unset.
    pub api_key: Option<String>,
    /// Model name to request, e.g. `text-embedding-3-small`,
    /// `text-embedding-3-large`, `text-embedding-ada-002`, or any model
    /// served by an OpenAI-compatible gateway. Default:
    /// `text-embedding-3-small`. Override with
    /// `OPEN_ONTOLOGIES_EMBEDDINGS_MODEL`.
    pub model: Option<String>,
    /// Optional `dimensions` parameter sent in the request body. Lets you
    /// truncate output dimensionality on models that support it
    /// (text-embedding-3-*). When unset, the API's default dimension is
    /// used and detected from the first response.
    pub dimensions: Option<usize>,
    /// HTTP request timeout in seconds. Default: 30.
    pub request_timeout_secs: Option<u64>,
}

/// Configuration for the on-disk N-Triples compile cache and TTL eviction.
/// Resolve the configured embedding provider name.
///
/// Precedence: `OPEN_ONTOLOGIES_EMBEDDINGS_PROVIDER` env var > config field >
/// default ("local"). Returns a lowercased, trimmed string.
pub fn resolve_embeddings_provider(cfg: &EmbeddingsConfig) -> String {
    let raw = std::env::var("OPEN_ONTOLOGIES_EMBEDDINGS_PROVIDER")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| cfg.provider.clone())
        .unwrap_or_else(|| "local".to_string());
    raw.trim().to_lowercase()
}

/// Resolve the OpenAI-compatible API base URL.
///
/// Precedence: `OPEN_ONTOLOGIES_EMBEDDINGS_API_BASE` env var > config >
/// `https://api.openai.com/v1`. Trailing slashes are stripped.
pub fn resolve_embeddings_api_base(cfg: &EmbeddingsConfig) -> String {
    let raw = std::env::var("OPEN_ONTOLOGIES_EMBEDDINGS_API_BASE")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| cfg.api_base.clone())
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    raw.trim().trim_end_matches('/').to_string()
}

/// Resolve the OpenAI-compatible API key.
///
/// Precedence: `OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY` env var >
/// `OPENAI_API_KEY` env var > config. Returns `None` if no key is configured
/// (some local OpenAI-compatible gateways accept unauthenticated requests).
pub fn resolve_embeddings_api_key(cfg: &EmbeddingsConfig) -> Option<String> {
    std::env::var("OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("OPENAI_API_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .or_else(|| cfg.api_key.clone().filter(|v| !v.trim().is_empty()))
}

/// Resolve the OpenAI-compatible model name.
///
/// Precedence: `OPEN_ONTOLOGIES_EMBEDDINGS_MODEL` env var > config >
/// `text-embedding-3-small`.
pub fn resolve_embeddings_model(cfg: &EmbeddingsConfig) -> String {
    std::env::var("OPEN_ONTOLOGIES_EMBEDDINGS_MODEL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| cfg.model.clone().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| "text-embedding-3-small".to_string())
}

// ─── LLM Boundary Translator (Groq) ───────────────────────────────────────
//
// The translator is a *language-boundary proposer*, not an authority. It
// converts messy stakeholder voice into candidate CTQ structure that the
// deterministic CTQ admission gate then admits or denies. The Groq API is
// OpenAI-compatible at `/v1/chat/completions`.
//
// Secret hygiene (Invariant 7): the resolved key is held only on the
// translator struct and bound to outbound requests via `bearer_auth`. It
// must never appear in logs, OCEL attributes, receipts, error messages,
// or projections. See `src/llm_translator.rs`.

/// Configuration for the Groq-backed LLM boundary translator.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct LlmConfig {
    /// LLM provider name. Currently only `"groq"` is wired; the field is
    /// reserved for future provider substitution. Override with
    /// `OPEN_ONTOLOGIES_LLM_PROVIDER`.
    pub provider: Option<String>,
    /// OpenAI-compatible API base URL, **without** the trailing
    /// `/chat/completions` path. Default: `https://api.groq.com/openai/v1`.
    /// Override with `OPEN_ONTOLOGIES_LLM_API_BASE`.
    #[serde(alias = "base_url")]
    pub api_base: Option<String>,
    /// API key. Resolution order:
    ///   `OPEN_ONTOLOGIES_LLM_API_KEY` env > `GROQ_API_KEY` env > config.
    /// Sent as `Authorization: Bearer <key>` and **never** logged.
    pub api_key: Option<String>,
    /// Model name. Default: `llama-3.3-70b-versatile`. Override with
    /// `OPEN_ONTOLOGIES_LLM_MODEL`.
    pub model: Option<String>,
    /// HTTP request timeout in seconds. Default: 30.
    pub request_timeout_secs: Option<u64>,
    /// Default engine selecting how `onto_translate_candidate`,
    /// `onto_executive_projection`, and `onto_groq_status` run when the
    /// caller does not supply an explicit `engine` parameter or
    /// `X-Ontostar-LLM-Engine` HTTP header. Recognised values:
    ///   - `"inproc"` — in-process `GroqTranslator` HTTP path.
    ///   - `"groq_pm4py"` — shell out to `scripts/*.py` (real-Groq via dspy).
    ///
    /// Override with `OPEN_ONTOLOGIES_LLM_ENGINE`. Default: auto-detected
    /// (`groq_pm4py` when an API key is available, else `inproc`).
    pub engine: Option<String>,
    /// Path to the python interpreter used by the `groq_pm4py` engine.
    /// Override with `OPEN_ONTOLOGIES_LLM_PYTHON` or per-call via the
    /// `python` tool argument. Default: `"python3"`.
    pub python_interpreter: Option<String>,
    /// Hard timeout (seconds) for every subprocess invoked by the LLM
    /// path (`groq_pm4py` engine, `ggen sync`, `ontostar_planner.py`,
    /// `wvda_agent.py`, `mu_star_agent.py`, etc.). Wired into
    /// [`crate::subprocess::run_with_timeout`] at the 8 historical
    /// shell-out sites in `src/server.rs`. Default: 60.
    ///
    /// Pre-R7-WB-1 this field was *dead* — declared on the config but
    /// never read by any call site. The active wedge risk that closure
    /// addresses: the auto-default `groq_pm4py` engine spawns
    /// `scripts/*.py`, which itself opens a Groq HTTP request; any
    /// network or API hang wedged the Tokio worker indefinitely.
    pub subprocess_timeout_secs: Option<u64>,
    /// R7 WB-3 — daily token budget per tenant. When the running
    /// 24h-window total exceeds this value (within the
    /// `grace_period_pct` warn band) every subsequent LLM call is
    /// denied with `DefectClass::LlmBudgetExceeded` until midnight UTC
    /// or an admin invokes `onto_llm_budget_reset`. `None` (default)
    /// disables the gate even when `budget_enforce = true`. Default
    /// magnitude (when set): 100_000 tokens/day.
    #[serde(default)]
    pub daily_token_budget_per_tenant: Option<u64>,
    /// R7 WB-3 — fraction of the daily budget that triggers a warning
    /// (logged + OCEL `llm_budget_warning`) before hard denial. e.g.
    /// `0.10` means 0–110% of the budget is allowed but the band 100–
    /// 110% is logged. Default: `0.10`.
    #[serde(default = "default_grace_period_pct")]
    pub grace_period_pct: f64,
    /// R7 WB-3 — when `false` (default) the budget meter records and
    /// warns but never denies. Operators flip this to `true` after
    /// observing one quiet week of capture. Default: `false`.
    #[serde(default)]
    pub budget_enforce: bool,
    /// R7 WB-3 — toggle whether `llm_invoked` OCEL events get the
    /// additive token attribute set. Off in tests by default to avoid
    /// schema churn. Default: `true`.
    #[serde(default = "default_true_bool")]
    pub emit_token_attrs: bool,
    /// R7 WD-4 — when `true`, the `llm_invoked_full` OCEL event also
    /// stores the redacted, 32 KiB-truncated prompt and completion
    /// text. The BLAKE3 prompt/completion hashes are ALWAYS stored
    /// regardless of this flag. Default: `false` (production-safe;
    /// enable in test/staging to capture LLM IO for debugging).
    /// Override with `OPEN_ONTOLOGIES_LLM_PERSIST_FULL_IO=1`.
    pub persist_full_io: Option<bool>,
}

fn default_grace_period_pct() -> f64 { 0.10 }
fn default_true_bool() -> bool { true }

/// R7 WD-4 — resolve `[llm] persist_full_io`. Precedence:
/// `OPEN_ONTOLOGIES_LLM_PERSIST_FULL_IO` env > config > `false`.
pub fn resolve_llm_persist_full_io(cfg: &LlmConfig) -> bool {
    if let Ok(v) = std::env::var("OPEN_ONTOLOGIES_LLM_PERSIST_FULL_IO") {
        let trimmed = v.trim();
        return matches!(trimmed, "1" | "true" | "TRUE" | "True" | "yes" | "on");
    }
    cfg.persist_full_io.unwrap_or(false)
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: None,
            api_base: None,
            api_key: None,
            model: None,
            request_timeout_secs: None,
            engine: None,
            python_interpreter: None,
            subprocess_timeout_secs: None,
            daily_token_budget_per_tenant: None,
            grace_period_pct: default_grace_period_pct(),
            budget_enforce: false,
            emit_token_attrs: default_true_bool(),
            persist_full_io: None,
        }
    }
}

/// Resolve the LLM provider name. Precedence:
/// `OPEN_ONTOLOGIES_LLM_PROVIDER` env > config > `"groq"`.
pub fn resolve_llm_provider(cfg: &LlmConfig) -> String {
    std::env::var("OPEN_ONTOLOGIES_LLM_PROVIDER")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| cfg.provider.clone().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| "groq".to_string())
        .trim()
        .to_lowercase()
}

/// Resolve the LLM API base URL. Precedence:
/// `OPEN_ONTOLOGIES_LLM_API_BASE` env > config >
/// `https://api.groq.com/openai/v1`. Trailing slashes are stripped.
pub fn resolve_llm_api_base(cfg: &LlmConfig) -> String {
    std::env::var("OPEN_ONTOLOGIES_LLM_API_BASE")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| cfg.api_base.clone().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| "https://api.groq.com/openai/v1".to_string())
        .trim()
        .trim_end_matches('/')
        .to_string()
}

/// Resolve the LLM API key. Precedence:
/// `OPEN_ONTOLOGIES_LLM_API_KEY` env > `GROQ_API_KEY` env > config.
/// Returns `None` if no key is configured — the translator then refuses
/// to call the remote and the CTQ admission gate denies any proposal that
/// requires translation, with `LlmAuthorityClaimed`.
pub fn resolve_llm_api_key(cfg: &LlmConfig) -> Option<String> {
    std::env::var("OPEN_ONTOLOGIES_LLM_API_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("GROQ_API_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .or_else(|| cfg.api_key.clone().filter(|v| !v.trim().is_empty()))
}

/// Resolve the LLM model name. Precedence:
/// `OPEN_ONTOLOGIES_LLM_MODEL` env > config > `llama-3.3-70b-versatile`.
pub fn resolve_llm_model(cfg: &LlmConfig) -> String {
    std::env::var("OPEN_ONTOLOGIES_LLM_MODEL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| cfg.model.clone().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| "llama-3.3-70b-versatile".to_string())
}

/// Recognised `engine` values for the LLM boundary.
///
/// `"inproc"`        → in-process `GroqTranslator` (HTTP via reqwest).
/// `"groq_pm4py"`    → shell out to `scripts/*.py` (dspy / pm4py path).
/// `"gemini"`        → headless Gemini CLI via OAuth (`gemini -p … --approval-mode yolo`);
///                     no API key required. Binary resolved via `GEMINI_BIN` env var or
///                     `"gemini"` default. Mirrors the speckit-ralph `gemini-invoke.sh` pattern.
pub const VALID_LLM_ENGINES: &[&str] = &["inproc", "groq_pm4py", "gemini"];

/// Default Gemini model used by the headless CLI engine (`gemini` engine).
/// Override via the `--model` flag or a future config key.
pub const GEMINI_DEFAULT_MODEL: &str = "gemini-3.1-flash-lite-preview";

/// Resolve the default LLM engine for unparametrised tool calls.
///
/// Precedence:
/// 1. `OPEN_ONTOLOGIES_LLM_ENGINE` env var (validated against
///    [`VALID_LLM_ENGINES`]; unknown values are dropped).
/// 2. `[llm] engine = "..."` in config (same validation).
/// 3. Auto-detect: when an API key is resolvable via
///    [`resolve_llm_api_key`], default to `"groq_pm4py"` — every
///    real-Groq integration test in `tests/real_groq_*` proves the
///    subprocess path. Otherwise fall back to `"inproc"` so the
///    in-process translator still serves audit-only callers without
///    requiring a python venv.
pub fn resolve_llm_engine(cfg: &LlmConfig) -> String {
    fn validated(v: String) -> Option<String> {
        let trimmed = v.trim().to_string();
        if VALID_LLM_ENGINES.contains(&trimmed.as_str()) {
            Some(trimmed)
        } else {
            None
        }
    }

    if let Some(v) = std::env::var("OPEN_ONTOLOGIES_LLM_ENGINE")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .and_then(validated)
    {
        return v;
    }
    if let Some(v) = cfg
        .engine
        .clone()
        .filter(|v| !v.trim().is_empty())
        .and_then(validated)
    {
        return v;
    }
    if resolve_llm_api_key(cfg).is_some() {
        "groq_pm4py".to_string()
    } else {
        "inproc".to_string()
    }
}

/// Resolve the Gemini CLI binary path for the `gemini` engine.
/// Precedence: `GEMINI_BIN` env var > `"gemini"` default.
pub fn resolve_gemini_bin() -> String {
    std::env::var("GEMINI_BIN")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "gemini".to_string())
}

/// Resolve the python interpreter for the `groq_pm4py` engine.
/// Precedence: `OPEN_ONTOLOGIES_LLM_PYTHON` env > config > `"python3"`.
pub fn resolve_llm_python(cfg: &LlmConfig) -> String {
    std::env::var("OPEN_ONTOLOGIES_LLM_PYTHON")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| cfg.python_interpreter.clone().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| "python3".to_string())
}

/// R7 WB-1 — resolve the subprocess wall-clock timeout in seconds.
///
/// Precedence order: `OPEN_ONTOLOGIES_SUBPROCESS_TIMEOUT_SECS` env,
/// then config, then 60s default. The env override lets operators
/// (and integration tests) tighten the deadline without rewriting
/// `config.toml`. Returns a `Duration` so call sites skip the cast.
pub fn resolve_subprocess_timeout(cfg: &LlmConfig) -> std::time::Duration {
    let secs = std::env::var("OPEN_ONTOLOGIES_SUBPROCESS_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .or(cfg.subprocess_timeout_secs)
        .unwrap_or(60);
    std::time::Duration::from_secs(secs)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct CacheConfig {
    /// Master switch for the compile cache. When false, every load re-parses
    /// from source and no metadata is recorded.
    pub enabled: bool,
    /// Directory where N-Triples cache files are stored.
    pub dir: String,
    /// If > 0, the active ontology will be unloaded from memory after this
    /// many seconds without access. The cache file is preserved and reloaded
    /// automatically on the next query.
    ///
    /// Accepts either canonical name `idle_ttl_secs` or the more descriptive
    /// alias `unload_timeout_secs` — both populate the same field.
    #[serde(alias = "unload_timeout_secs")]
    pub idle_ttl_secs: u64,
    /// How often the background evictor checks idle entries (seconds).
    pub evictor_interval_secs: u64,
    /// When true, every read tool checks the source file's mtime/sha and
    /// recompiles if it changed. Off by default for predictability.
    pub auto_refresh: bool,
    /// Number of bytes from the head of a source ontology file that are
    /// hashed (sha256) to form the cache fingerprint tie-breaker. Larger
    /// values reduce collision probability for very large dumps where many
    /// files share an identical leading region (e.g. `@prefix` headers in
    /// big N-Quads exports). Default 64 KiB.
    pub hash_prefix_bytes: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: "~/.open-ontologies/cache".into(),
            idle_ttl_secs: 0,
            evictor_interval_secs: 30,
            auto_refresh: false,
            hash_prefix_bytes: 64 * 1024,
        }
    }
}

/// Configuration for limiting which MCP tools are exposed.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ToolsConfig {
    /// "all" (default), "allow", or "deny".
    pub mode: String,
    /// Explicit tool names included by the filter.
    pub list: Vec<String>,
    /// Group names (e.g. "read_only") expanded into tool names.
    pub groups: Vec<String>,
}

/// Expand a leading `~` in a path to the user's home directory.
pub fn expand_tilde(path: &str) -> String {
    if (path.starts_with("~/") || path == "~")
        && let Some(home) = std::env::var_os("HOME") {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    path.to_string()
}

// ─── New section configs ────────────────────────────────────────────────

/// `[webhook]` — outbound HTTP for governance / monitor alerts.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct WebhookConfig {
    /// HTTP timeout (seconds) for governance / monitor webhook deliveries.
    /// Override at runtime with `OPEN_ONTOLOGIES_WEBHOOK_REQUEST_TIMEOUT_SECS`.
    pub request_timeout_secs: u64,
}
impl Default for WebhookConfig {
    fn default() -> Self { Self { request_timeout_secs: 10 } }
}

/// `[http]` — Streamable HTTP transport (`serve-http`).
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct HttpConfig {
    /// Bind host. Default `127.0.0.1`. CLI `--host` and the
    /// `OPEN_ONTOLOGIES_HTTP_HOST` env var take precedence.
    pub host: String,
    /// Bind port. Default `8080`. CLI `--port` and the
    /// `OPEN_ONTOLOGIES_HTTP_PORT` env var take precedence.
    pub port: u16,
    /// Optional bearer token for authentication. Empty string ⇒ disabled.
    /// CLI `--token` and `OPEN_ONTOLOGIES_TOKEN` env var take precedence.
    pub token: String,
    /// Whether the rmcp `StreamableHttpServer` keeps per-session state.
    /// Default `true` (matches existing behaviour).
    pub stateful_mode: bool,
    /// Per-request HTTP timeout in seconds. `0` ⇒ no explicit cap (rmcp
    /// default). Override with `OPEN_ONTOLOGIES_HTTP_REQUEST_TIMEOUT_SECS`.
    pub request_timeout_secs: u64,
    /// HTTP keep-alive timeout in seconds. `0` ⇒ no explicit cap.
    pub keep_alive_secs: u64,
}
impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
            token: String::new(),
            stateful_mode: true,
            request_timeout_secs: 0,
            keep_alive_secs: 0,
        }
    }
}

/// `[monitor]` — continuous watcher loop.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct MonitorConfig {
    /// When true, start the background monitor loop on `serve` / `serve-http`.
    /// Equivalent to passing `--watch`. Default `false`.
    pub enabled: bool,
    /// Interval in seconds between monitor sweeps. Default `30`.
    /// Override with `OPEN_ONTOLOGIES_MONITOR_INTERVAL_SECS`.
    #[serde(alias = "watch_interval_secs")]
    pub interval_secs: u64,
}
impl Default for MonitorConfig {
    fn default() -> Self { Self { enabled: false, interval_secs: 30 } }
}

/// `[reasoner]` — RDFS / OWL-RL fixpoint and DL tableaux limits.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ReasonerConfig {
    /// Tableaux DL reasoner: max recursion depth before giving up. Default 100.
    pub tableaux_max_depth: usize,
    /// Tableaux DL reasoner: max nodes in a model before giving up.
    /// Default 10 000. Increase for very large ontologies; raising this past
    /// ~100 000 typically indicates the ontology is unsatisfiable in
    /// pathological ways and the reasoner should bail out anyway.
    pub tableaux_max_nodes: usize,
    /// RDFS / OWL-RL fixpoint guard. Maximum number of expansion iterations
    /// before the reasoner returns the partial closure. Default 64.
    pub max_iterations: usize,
}
impl Default for ReasonerConfig {
    fn default() -> Self {
        Self { tableaux_max_depth: 100, tableaux_max_nodes: 10_000, max_iterations: 64 }
    }
}

/// `[feedback]` — lint / enforce self-calibration thresholds.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct FeedbackConfig {
    /// Number of consecutive dismissals after which a lint/enforce issue
    /// is fully suppressed for that (tool, rule_id, entity) tuple. Default 3.
    pub suppress_threshold: i64,
    /// Number of consecutive dismissals after which a lint/enforce issue
    /// is downgraded one severity level. Must be `< suppress_threshold` to
    /// take effect. Default 2.
    pub downgrade_threshold: i64,
}
impl Default for FeedbackConfig {
    fn default() -> Self { Self { suppress_threshold: 3, downgrade_threshold: 2 } }
}

/// `[retention]` — Round 4 WD §29 Cell8 retirement closure. Per-table
/// retention windows in days. The [`crate::retention::RetentionWorker`]
/// runs on a `poll_interval_secs` cadence and prunes rows older than
/// each respective window.
///
/// `archive_path` (when set) names the on-disk directory where the
/// receipt cold-storage Parquet shards + sidecar index are written.
/// Receipts older than `hot_receipt_days` are archived there before
/// being removed from the hot `receipts` table.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RetentionConfig {
    pub poll_interval_secs: u64,
    pub ocel_days: u64,
    pub lineage_days: u64,
    pub conformance_days: u64,
    pub revocation_grace_days: u64,
    pub receipt_files_days: u64,
    pub exemplar_days: u64,
    pub feedback_days: u64,
    pub archive_path: Option<std::path::PathBuf>,
    pub hot_receipt_days: u64,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 86_400, // once per day
            ocel_days: 90,
            lineage_days: 180,
            conformance_days: 30,
            revocation_grace_days: 30,
            receipt_files_days: 365,
            exemplar_days: 365,
            feedback_days: 365,
            archive_path: None,
            hot_receipt_days: 365,
        }
    }
}

/// `[verifier]` — R7 WA2 A2 V1 Receipt-Chain Verifier.
///
/// The verifier worker (`crate::verifier_worker::VerifierWorker`) ticks
/// every `tick_secs`, scans a batch of new receipts past its cursor, and
/// runs `crate::verify::crypto_verify` on each row. The function is pure
/// — verdicts depend only on the receipt row and its matching
/// `trusted_keys_history` row, NOT on any LLM, network call, or wall
/// clock comparison performed lazily.
///
/// # ZERO-LLM invariant
///
/// This is the deterministic autonomic loop. An LLM in this code path
/// would be a §22 regression. The config has no `llm_*` fields by
/// design.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct VerifierConfig {
    /// When false, the worker spawns but skips every tick. Default: true.
    pub enabled: bool,
    /// Tick interval in seconds. Clamped to >= 1. Default: 300 (5 min).
    pub tick_secs: u64,
    /// Maximum number of receipts to verify per tick. Default: 5000.
    pub batch_limit: i64,
    /// On a corruption verdict, advance `retention_paused_until` via
    /// `fetch_max(now + pause_minutes_on_failure * 60)`. Default: true.
    pub pause_retention_on_failure: bool,
    /// How long to pause retention on a corruption verdict, in minutes.
    /// Default: 60.
    pub pause_minutes_on_failure: u64,
    /// On a corruption verdict, emit `tracing::error!(target:"andon",
    /// ...)` so log scrapers can stop the line. Default: true.
    pub andon_on_failure: bool,
    /// Optional clamp: if set, only verify receipts whose `granted_at`
    /// falls within the past N days. Useful for very large back-fills.
    /// Default: None (unbounded — verify the full chain).
    pub max_lookback_days: Option<u64>,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tick_secs: 300,
            batch_limit: 5000,
            pause_retention_on_failure: true,
            pause_minutes_on_failure: 60,
            andon_on_failure: true,
            max_lookback_days: None,
        }
    }
}

/// `[imports]` — `owl:imports` resolution policy.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ImportsConfig {
    /// Maximum transitive depth to follow when resolving owl:imports. The
    /// `onto_import` tool's `max_depth` parameter (when supplied) overrides
    /// this. Default 3.
    pub max_depth: usize,
    /// HTTP timeout in seconds for fetching each remote import. Set to `0`
    /// to disable the explicit per-call timeout and use reqwest's default
    /// (which itself has no timeout). Override with
    /// `OPEN_ONTOLOGIES_IMPORTS_REQUEST_TIMEOUT_SECS`.
    pub request_timeout_secs: u64,
    /// When false, remote (`http(s)://`) imports are refused — useful in
    /// air-gapped or sandboxed deployments. Default `true`.
    pub follow_remote: bool,
}
impl Default for ImportsConfig {
    fn default() -> Self {
        Self { max_depth: 3, request_timeout_secs: 30, follow_remote: true }
    }
}

/// `[repo]` — on-disk ontology repository tools.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RepoConfig {
    /// Default `limit` for `onto_repo_list` when the caller doesn't supply
    /// one. Default 1000.
    pub default_list_limit: usize,
}
impl Default for RepoConfig {
    fn default() -> Self { Self { default_list_limit: 1000 } }
}

/// `[socket]` — Unix domain socket adapter for Tardygrada fact grounding.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SocketConfig {
    /// When true, `serve` / `serve-http` will additionally start the unix
    /// socket adapter. Currently the dedicated `serve-unix` subcommand reads
    /// these defaults. Default `false`.
    pub enabled: bool,
    /// Default socket path for `serve-unix`. CLI `--socket` overrides.
    pub path: Option<String>,
    /// Default ontology files to preload on `serve-unix` startup.
    /// CLI `--file` (repeatable) overrides.
    pub preload_files: Vec<String>,
}

/// `[logging]` — tracing subscriber configuration.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct LoggingConfig {
    /// Default log directives (e.g. `info`, `debug`,
    /// `open_ontologies=debug,reqwest=warn`). The `RUST_LOG` env var, when
    /// set, takes precedence over this value.
    pub level: String,
    /// Output format: `compact` (default), `pretty`, or `json`.
    pub format: String,
    /// Optional path to write logs to. When unset, logs go to stderr.
    pub file: Option<String>,
}
impl Default for LoggingConfig {
    fn default() -> Self {
        Self { level: "info".into(), format: "compact".into(), file: None }
    }
}

/// `[telemetry]` — R8-3 OTEL export wiring.
///
/// When `otlp_endpoint` is set, `src/telemetry.rs::init_telemetry` will wire
/// a `tracing-opentelemetry` layer exporting spans to that endpoint. When
/// unset, only the `tracing-subscriber` logging layer is active.
///
/// All `tracing::debug!(target: "ontostar.*", ...)` spans produced by the
/// admission gate and verifier worker are exported via this path when wired.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct TelemetryConfig {
    /// OTLP gRPC endpoint URL, e.g. `http://localhost:4317`. When `None`,
    /// OTEL export is disabled and spans are consumed only by the local
    /// `tracing-subscriber` log sink.
    pub otlp_endpoint: Option<String>,
    /// Service name reported in OTLP resource attributes.
    pub service_name: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: None,
            service_name: "open-ontologies".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct CodegenConfig {
    /// Path to the ggen binary. Default: "ggen" (searched in PATH).
    /// Set to "~/.local/bin/ggen" if ggen is installed there but not in standard PATH.
    /// Override at runtime with `OPEN_ONTOLOGIES_CODEGEN_GGEN_PATH` env var.
    pub ggen_path: String,
}

impl Default for CodegenConfig {
    fn default() -> Self {
        Self { ggen_path: "ggen".to_string() }
    }
}

/// `[authority]` — R5 WC-1 §28 HumanOverride closure.
///
/// `admin_principals` lists the principal IDs (currently the
/// caller's `tenant_id` until R3 Task B's principal helper lands)
/// that may invoke admin-only MCP tools. The list is read from the
/// config file AND, if set, the `OPEN_ONTOLOGIES_ADMIN_PRINCIPALS`
/// env var (env wins over config); the resolution is performed
/// **once at startup** by [`resolve_admin_principals`] and cached
/// on `OpenOntologiesServer` as `Arc<Vec<String>>`. Subsequent env
/// changes do not affect already-running servers — closes the
/// TOCTOU race that the previous per-call `std::env::var(...)` read
/// admitted.
///
/// Closed by default: an empty list means NO callers are admin.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct AuthorityConfig {
    /// Principal IDs (typically tenant IDs in the current implementation)
    /// authorised to invoke admin-only MCP tools. Empty list → no admins.
    pub admin_principals: Vec<String>,
    /// R5 WC-2 — tenant allowlist for the HTTP `X-Ontostar-Tenant` header.
    /// When non-empty, requests carrying a tenant header NOT in this list
    /// are rejected with HTTP 403 + `FalsePass { reason:
    /// "tenant_not_in_allowlist" }`. Empty list preserves the
    /// pre-R5-WC-2 behaviour (any well-formed tenant accepted), so
    /// existing single-tenant deployments are unaffected. Resolved at
    /// startup by [`resolve_known_tenants`] and cached on the HTTP
    /// router; subsequent env-var mutations are ignored. Closed-by-
    /// default once any value is configured.
    pub known_tenants: Vec<String>,
}

/// Resolve the admin principal allowlist ONCE at startup. Precedence:
/// `OPEN_ONTOLOGIES_ADMIN_PRINCIPALS` env var (comma-separated) > config
/// file (`[authority] admin_principals = [...]`) > empty (closed).
///
/// Trims whitespace and drops empty entries. The returned `Vec<String>`
/// is intended to be wrapped in an `Arc` and stored on the server so
/// subsequent calls to `is_admin_principal` are TOCTOU-immune.
///
/// This is the **only** place the env var is read in production code;
/// any other reader is a §28 HumanOverride leak.
pub fn resolve_admin_principals(cfg: &AuthorityConfig) -> Vec<String> {
    let from_env = std::env::var("OPEN_ONTOLOGIES_ADMIN_PRINCIPALS").ok();
    let raw_iter: Vec<String> = match from_env {
        Some(v) if !v.trim().is_empty() => v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => cfg
            .admin_principals
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    };
    // Deduplicate while preserving order.
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(raw_iter.len());
    for entry in raw_iter {
        if seen.insert(entry.clone()) {
            out.push(entry);
        }
    }
    out
}

/// R5 WC-2 — resolve the tenant allowlist for the HTTP
/// `X-Ontostar-Tenant` header ONCE at startup. Precedence:
/// `OPEN_ONTOLOGIES_KNOWN_TENANTS` env var (comma-separated) > config
/// file (`[authority] known_tenants = [...]`) > empty (open: any
/// well-formed tenant accepted).
///
/// The env var is the **only** source read in production code; any
/// other reader is a §28 HumanOverride leak. Mirrors
/// [`resolve_admin_principals`] semantics: trims whitespace, drops
/// empty entries, deduplicates while preserving order.
///
/// An empty result means "no allowlist configured" — backward-compatible
/// with single-tenant deployments. A non-empty result enforces the
/// allowlist; unknown tenants are rejected at the HTTP middleware layer
/// before the per-request server is constructed.
pub fn resolve_known_tenants(cfg: &AuthorityConfig) -> Vec<String> {
    let from_env = std::env::var("OPEN_ONTOLOGIES_KNOWN_TENANTS").ok();
    let raw_iter: Vec<String> = match from_env {
        Some(v) if !v.trim().is_empty() => v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => cfg
            .known_tenants
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    };
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(raw_iter.len());
    for entry in raw_iter {
        if seen.insert(entry.clone()) {
            out.push(entry);
        }
    }
    out
}

// ─── Env-override resolvers for the most operationally critical fields ──

/// Resolve the webhook request timeout. Precedence:
/// `OPEN_ONTOLOGIES_WEBHOOK_REQUEST_TIMEOUT_SECS` > config > default.
pub fn resolve_webhook_timeout_secs(cfg: &WebhookConfig) -> u64 {
    parse_env_u64("OPEN_ONTOLOGIES_WEBHOOK_REQUEST_TIMEOUT_SECS")
        .unwrap_or(cfg.request_timeout_secs)
}

/// Resolve the imports request timeout. Precedence:
/// `OPEN_ONTOLOGIES_IMPORTS_REQUEST_TIMEOUT_SECS` > config > default.
pub fn resolve_imports_timeout_secs(cfg: &ImportsConfig) -> u64 {
    parse_env_u64("OPEN_ONTOLOGIES_IMPORTS_REQUEST_TIMEOUT_SECS")
        .unwrap_or(cfg.request_timeout_secs)
}

/// Resolve the monitor sweep interval. Precedence:
/// `OPEN_ONTOLOGIES_MONITOR_INTERVAL_SECS` > config > default.
pub fn resolve_monitor_interval_secs(cfg: &MonitorConfig) -> u64 {
    parse_env_u64("OPEN_ONTOLOGIES_MONITOR_INTERVAL_SECS")
        .unwrap_or(cfg.interval_secs)
}

/// Resolve the HTTP bind host. Precedence:
/// `OPEN_ONTOLOGIES_HTTP_HOST` > config > default.
pub fn resolve_http_host(cfg: &HttpConfig) -> String {
    std::env::var("OPEN_ONTOLOGIES_HTTP_HOST")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| cfg.host.clone())
}

/// Resolve the HTTP bind port. Precedence:
/// `OPEN_ONTOLOGIES_HTTP_PORT` > config > default.
pub fn resolve_http_port(cfg: &HttpConfig) -> u16 {
    std::env::var("OPEN_ONTOLOGIES_HTTP_PORT")
        .ok()
        .and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(cfg.port)
}

/// Resolve the HTTP bearer token. Precedence:
/// `OPEN_ONTOLOGIES_TOKEN` > config (if non-empty) > `None`.
pub fn resolve_http_token(cfg: &HttpConfig) -> Option<String> {
    std::env::var("OPEN_ONTOLOGIES_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| Some(cfg.token.clone()).filter(|v| !v.trim().is_empty()))
}

/// Resolve the logging directives. Precedence: `RUST_LOG` > config > default.
pub fn resolve_logging_level(cfg: &LoggingConfig) -> String {
    std::env::var("RUST_LOG")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| cfg.level.clone())
}

fn parse_env_u64(var: &str) -> Option<u64> {
    std::env::var(var)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openai_provider_block() {
        let toml_src = r#"
            [embeddings]
            provider = "openai"
            api_base = "https://api.example.com/v1/"
            api_key = "sk-test"
            model = "text-embedding-3-large"
            dimensions = 256
            request_timeout_secs = 60
        "#;
        let cfg: Config = toml::from_str(toml_src).expect("parse");
        assert_eq!(cfg.embeddings.provider.as_deref(), Some("openai"));
        assert_eq!(cfg.embeddings.api_key.as_deref(), Some("sk-test"));
        assert_eq!(cfg.embeddings.dimensions, Some(256));
        assert_eq!(cfg.embeddings.request_timeout_secs, Some(60));

        // Trailing slash is stripped by the resolver.
        assert_eq!(
            resolve_embeddings_api_base(&cfg.embeddings),
            "https://api.example.com/v1"
        );
        assert_eq!(resolve_embeddings_provider(&cfg.embeddings), "openai");
        assert_eq!(
            resolve_embeddings_model(&cfg.embeddings),
            "text-embedding-3-large"
        );
    }

    #[test]
    fn provider_defaults_to_local_when_unset() {
        // Verify the default-resolution logic without touching process-wide
        // env vars (which would race with other tests). When the env override
        // is absent the function should fall back to the config field, then
        // to "local".
        let cfg = EmbeddingsConfig::default();
        let resolved = cfg
            .provider
            .clone()
            .unwrap_or_else(|| "local".to_string())
            .trim()
            .to_lowercase();
        assert_eq!(resolved, "local");
    }

    #[test]
    fn base_url_alias_accepted() {
        // The legacy/alternative `base_url` key should also populate
        // `api_base` via serde alias.
        let toml_src = r#"
            [embeddings]
            base_url = "http://localhost:11434/v1"
        "#;
        let cfg: Config = toml::from_str(toml_src).expect("parse");
        assert_eq!(
            cfg.embeddings.api_base.as_deref(),
            Some("http://localhost:11434/v1")
        );
    }

    #[test]
    fn unload_timeout_alias_for_idle_ttl() {
        let toml_src = r#"
            [cache]
            unload_timeout_secs = 120
            hash_prefix_bytes = 131072
        "#;
        let cfg: Config = toml::from_str(toml_src).expect("parse");
        assert_eq!(cfg.cache.idle_ttl_secs, 120);
        assert_eq!(cfg.cache.hash_prefix_bytes, 131072);
    }

    #[test]
    fn new_sections_parse_with_defaults() {
        let toml_src = r#"
            [webhook]
            request_timeout_secs = 5

            [http]
            host = "0.0.0.0"
            port = 9000
            stateful_mode = false

            [monitor]
            enabled = true
            interval_secs = 15

            [reasoner]
            tableaux_max_depth = 200
            tableaux_max_nodes = 50000
            max_iterations = 128

            [feedback]
            suppress_threshold = 5
            downgrade_threshold = 3

            [imports]
            max_depth = 5
            request_timeout_secs = 60
            follow_remote = false

            [repo]
            default_list_limit = 250

            [socket]
            enabled = true
            path = "/tmp/foo.sock"
            preload_files = ["a.ttl", "b.ttl"]

            [logging]
            level = "debug"
            format = "json"
        "#;
        let cfg: Config = toml::from_str(toml_src).expect("parse");
        assert_eq!(cfg.webhook.request_timeout_secs, 5);
        assert_eq!(cfg.http.host, "0.0.0.0");
        assert_eq!(cfg.http.port, 9000);
        assert!(!cfg.http.stateful_mode);
        assert!(cfg.monitor.enabled);
        assert_eq!(cfg.monitor.interval_secs, 15);
        assert_eq!(cfg.reasoner.tableaux_max_depth, 200);
        assert_eq!(cfg.reasoner.tableaux_max_nodes, 50_000);
        assert_eq!(cfg.reasoner.max_iterations, 128);
        assert_eq!(cfg.feedback.suppress_threshold, 5);
        assert_eq!(cfg.feedback.downgrade_threshold, 3);
        assert_eq!(cfg.imports.max_depth, 5);
        assert_eq!(cfg.imports.request_timeout_secs, 60);
        assert!(!cfg.imports.follow_remote);
        assert_eq!(cfg.repo.default_list_limit, 250);
        assert!(cfg.socket.enabled);
        assert_eq!(cfg.socket.path.as_deref(), Some("/tmp/foo.sock"));
        assert_eq!(cfg.socket.preload_files, vec!["a.ttl", "b.ttl"]);
        assert_eq!(cfg.logging.level, "debug");
        assert_eq!(cfg.logging.format, "json");
    }

    #[test]
    fn new_sections_default_when_absent() {
        let cfg: Config = toml::from_str("").expect("parse empty");
        assert_eq!(cfg.webhook.request_timeout_secs, 10);
        assert_eq!(cfg.http.host, "127.0.0.1");
        assert_eq!(cfg.http.port, 8080);
        assert!(cfg.http.stateful_mode);
        assert!(!cfg.monitor.enabled);
        assert_eq!(cfg.monitor.interval_secs, 30);
        assert_eq!(cfg.reasoner.tableaux_max_depth, 100);
        assert_eq!(cfg.reasoner.tableaux_max_nodes, 10_000);
        assert_eq!(cfg.reasoner.max_iterations, 64);
        assert_eq!(cfg.feedback.suppress_threshold, 3);
        assert_eq!(cfg.feedback.downgrade_threshold, 2);
        assert_eq!(cfg.imports.max_depth, 3);
        assert_eq!(cfg.imports.request_timeout_secs, 30);
        assert!(cfg.imports.follow_remote);
        assert_eq!(cfg.repo.default_list_limit, 1000);
        assert!(!cfg.socket.enabled);
        assert!(cfg.socket.preload_files.is_empty());
        assert_eq!(cfg.logging.level, "info");
        assert_eq!(cfg.logging.format, "compact");
        assert_eq!(cfg.cache.hash_prefix_bytes, 64 * 1024);
    }

    #[test]
    fn resolvers_pass_through_config_when_env_unset() {
        // Regression: `resolve_*` functions must return the config-supplied
        // value when no env override is set. This exercises the precedence
        // path documented in PR #4 (env > config > default) without mutating
        // process-wide env vars (which would race with parallel tests).
        // We rely on these env vars being unset in CI; if a future test
        // sets them, this assertion will surface that interference.
        for var in [
            "OPEN_ONTOLOGIES_WEBHOOK_REQUEST_TIMEOUT_SECS",
            "OPEN_ONTOLOGIES_IMPORTS_REQUEST_TIMEOUT_SECS",
            "OPEN_ONTOLOGIES_MONITOR_INTERVAL_SECS",
        ] {
            if std::env::var(var).is_ok() {
                eprintln!("skipping resolver passthrough test: {var} is set");
                return;
            }
        }

        let webhook = WebhookConfig { request_timeout_secs: 7 };
        assert_eq!(resolve_webhook_timeout_secs(&webhook), 7);

        let imports = ImportsConfig {
            max_depth: 5,
            request_timeout_secs: 42,
            follow_remote: true,
        };
        assert_eq!(resolve_imports_timeout_secs(&imports), 42);

        let monitor = MonitorConfig { enabled: true, interval_secs: 11 };
        assert_eq!(resolve_monitor_interval_secs(&monitor), 11);
    }

    #[test]
    fn resolve_imports_timeout_preserves_zero_sentinel() {
        // The `0` sentinel for `[imports] request_timeout_secs` is documented
        // (commit cdd5384) as "disable the explicit per-call timeout and use
        // reqwest's default". The resolver must propagate the 0 verbatim so
        // downstream callers can decide whether to skip `.timeout()`.
        if std::env::var("OPEN_ONTOLOGIES_IMPORTS_REQUEST_TIMEOUT_SECS").is_ok() {
            eprintln!("skipping zero-sentinel test: env override is set");
            return;
        }
        let imports = ImportsConfig {
            max_depth: 3,
            request_timeout_secs: 0,
            follow_remote: true,
        };
        assert_eq!(resolve_imports_timeout_secs(&imports), 0);
    }
}
