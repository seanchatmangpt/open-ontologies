//! Server Commands — MCP server lifecycle verbs

use clap_noun_verb::Result as NounVerbResult;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::sync::Arc;

use open_ontologies::config::{expand_tilde, Config};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;

// ── output types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ServeOutput {
    pub status: String,
}

#[derive(Serialize)]
pub struct InitOutput {
    pub ok: bool,
    pub data_dir: String,
    pub db: String,
    pub config: String,
    pub config_created: bool,
}

// ── domain helpers ────────────────────────────────────────────────────────

const DEFAULT_CONFIG_PATH: &str = "~/.open-ontologies/config.toml";

const INIT_CONFIG_TEMPLATE: &str = r#"# Open Ontologies Configuration Template

[general]
# Data directory for SQLite database, cache, and snapshots
data_dir = "~/.open-ontologies"
# Directories to search for ontology files (for onto_repo_list/onto_repo_load)
# ontology_dirs = ["/path/to/ontologies"]

[http]
# Server binding
host = "127.0.0.1"
port = 8080
# Optional bearer token for authentication
token = ""
# CORS allowed origins (empty = permissive for dev)
# cors_origins = ["https://example.com", "https://app.example.com"]
# Rate limit in requests per second (None = no limit)
# rate_limit_rps = 1000

[logging]
# Log level: trace, debug, info, warn, error
level = "info"
# Optional log file path (leave empty for stderr only)
# file = "~/.open-ontologies/open-ontologies.log"

[cache]
# Ontology compile cache TTL in seconds (7200 = 2 hours)
idle_ttl_secs = 7200
# Auto-refresh cache on startup if TTL expired
auto_refresh = false

[monitor]
# Enable background monitor loop
enabled = false
# Monitor sweep interval in seconds
interval_secs = 30

[embeddings]
# Provider: "local" (ONNX) or "openai" (or compatible API)
provider = "local"
# For OpenAI-compatible APIs, set api_base and api_key
# api_base = "https://api.openai.com/v1"
# api_key = "" # or set OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY

[llm]
# LLM provider: "groq" (recommended) or "openai"
provider = "groq"
# API base URL (Groq default is public)
# api_base = "https://api.groq.com/openai/v1"

[telemetry]
# OTLP endpoint for tracing export (e.g., "http://localhost:4317")
# Unset or empty = local logging only
# otlp_endpoint = "http://localhost:4317"
# Service name for OTLP resource attributes
service_name = "open-ontologies"

[authority]
# Admin principals allowed to call admin-only tools
# admin_principals = ["default"]
# Known tenants (empty = any well-formed tenant accepted)
# known_tenants = ["default"]

[retention]
# Retention windows (days) for different artifact types
# [retention.artifacts] days = 90
# [retention.receipts] days = 180

[verifier]
# Receipt verification worker
enabled = false
interval_secs = 60
"#;

pub(crate) fn load_cfg(config_path: &str) -> anyhow::Result<Config> {
    let path = expand_tilde(config_path);
    match Config::load(std::path::Path::new(&path)) {
        Ok(c) => Ok(c),
        Err(e) => {
            if e.to_string().contains("failed to read") {
                eprintln!("warn: config not found at {}; using defaults. Run `open-ontologies server init` to create it.", path);
                Ok(Config::default())
            } else {
                Err(e)
            }
        }
    }
}

pub(crate) fn build_cache_cfg(cfg: &Config, idle_ttl_secs: Option<u64>, auto_refresh: bool) -> open_ontologies::config::CacheConfig {
    let mut cc = cfg.cache.clone();
    if let Some(ttl) = idle_ttl_secs { cc.idle_ttl_secs = ttl; }
    if auto_refresh { cc.auto_refresh = true; }
    cc
}

pub(crate) fn build_tool_filter_cfg(cfg: &Config, allow: Option<&str>, deny: Option<&str>) -> anyhow::Result<open_ontologies::toolfilter::ToolFilter> {
    use open_ontologies::toolfilter::{Mode, ToolFilter, parse_csv};
    if allow.is_some() && deny.is_some() { anyhow::bail!("--tools-allow and --tools-deny are mutually exclusive"); }
    if let Some(spec) = allow { let (list, groups) = parse_csv(spec); return Ok(ToolFilter { mode: Mode::Allow, list, groups }); }
    if let Some(spec) = deny { let (list, groups) = parse_csv(spec); return Ok(ToolFilter { mode: Mode::Deny, list, groups }); }
    let mode = if cfg.tools.mode.is_empty() { Mode::All } else { Mode::parse(&cfg.tools.mode).map_err(|e| anyhow::anyhow!(e))? };
    Ok(ToolFilter { mode, list: cfg.tools.list.clone(), groups: cfg.tools.groups.clone() })
}

pub(crate) fn init_tracing_cfg(cfg: &open_ontologies::config::LoggingConfig) {
    use tracing_subscriber::{fmt, EnvFilter};
    let level = open_ontologies::config::resolve_logging_level(cfg);
    let env_filter = EnvFilter::try_new(&level).unwrap_or_else(|_| EnvFilter::new("info"));
    let writer_file = cfg.file.as_deref().and_then(|p| {
        let path = expand_tilde(p);
        if let Some(parent) = std::path::Path::new(&path).parent() { let _ = std::fs::create_dir_all(parent); }
        std::fs::OpenOptions::new().create(true).append(true).open(&path).ok()
    });
    let format = cfg.format.trim().to_lowercase();
    let _ = match (format.as_str(), writer_file) {
        ("json", Some(f)) => fmt().with_env_filter(env_filter).json().with_writer(std::sync::Mutex::new(f)).try_init(),
        ("json", None) => fmt().with_env_filter(env_filter).json().with_writer(std::io::stderr).try_init(),
        ("pretty", Some(f)) => fmt().with_env_filter(env_filter).pretty().with_writer(std::sync::Mutex::new(f)).try_init(),
        ("pretty", None) => fmt().with_env_filter(env_filter).pretty().with_writer(std::io::stderr).try_init(),
        (_, Some(f)) => fmt().with_env_filter(env_filter).compact().with_writer(std::sync::Mutex::new(f)).try_init(),
        (_, None) => fmt().with_env_filter(env_filter).compact().with_writer(std::io::stderr).try_init(),
    };
}

fn open_db_and_graph(data_dir: &str) -> NounVerbResult<(String, StateDb, Arc<GraphStore>)> {
    let data_dir_expanded = expand_tilde(data_dir);
    let db_path = std::path::Path::new(&data_dir_expanded).join("open-ontologies.db");
    std::fs::create_dir_all(&data_dir_expanded)
        .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
    let db = StateDb::open(&db_path)
        .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
    let graph = Arc::new(GraphStore::new());
    Ok((data_dir_expanded, db, graph))
}

fn maybe_start_monitor(watch: bool, cfg: &Config, db_path_str: &str, watch_interval: Option<u64>, graph: Arc<GraphStore>) -> NounVerbResult<()> {
    let monitor_enabled = watch || cfg.monitor.enabled;
    let interval = watch_interval.unwrap_or_else(|| open_ontologies::config::resolve_monitor_interval_secs(&cfg.monitor));
    if monitor_enabled {
        let db_path = std::path::Path::new(db_path_str).to_path_buf();
        let watch_db = StateDb::open(&db_path)
            .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
        open_ontologies::monitor::start_background_loop(watch_db, graph, std::time::Duration::from_secs(interval));
    }
    Ok(())
}

fn run_stdio_server(cfg: Config, db: StateDb, graph: Arc<GraphStore>, governance_webhook: Option<String>, cache_config: open_ontologies::config::CacheConfig, tool_filter: open_ontologies::toolfilter::ToolFilter) -> NounVerbResult<()> {
    use rmcp::ServiceExt;
    let ontology_dirs = open_ontologies::config::resolve_ontology_dirs(&cfg.general.ontology_dirs);
    for d in &ontology_dirs {
        if !d.exists() { eprintln!("warning: ontology_dirs entry does not exist: {}", d.display()); }
    }
    let llm_engine = open_ontologies::config::resolve_llm_engine(&cfg.llm);
    eprintln!("info: default LLM engine = {}", llm_engine);
    // R5 WC-1 — resolve the admin principal allowlist ONCE at startup.
    // Subsequent env mutations are ignored (TOCTOU-immune).
    let admin_principals =
        open_ontologies::config::resolve_admin_principals(&cfg.authority);
    eprintln!(
        "info: admin principals configured = {} entries",
        admin_principals.len()
    );
    // R5 WC-2 — share the retention pause handle between the worker
    // and the MCP server so `onto_retention_pause` /
    // `onto_retention_resume` mutate the same atomic the worker reads.
    let pause_handle =
        std::sync::Arc::new(std::sync::atomic::AtomicI64::new(0));
    let server = OpenOntologiesServer::new_with_repo_options(db.clone(), graph, governance_webhook, cfg.embeddings, cache_config, tool_filter, ontology_dirs)
        .with_default_llm_engine(llm_engine)
        .with_admin_principals(admin_principals)
        .with_retention_pause(pause_handle.clone());
    let _evictor = open_ontologies::registry::spawn_evictor(server.registry());
    // Round 4 WD — §29 Cell8 retirement closure. Spawn the retention
    // worker alongside the cache evictor so every persistent table has
    // a defined retirement path. The worker logs (does not panic) on
    // failure; dropping the handle does not abort.
    //
    // R5 WC-2 — spawn with the externally-owned pause handle so the
    // server's `onto_retention_pause` admin tool can drive the
    // worker's tick.
    let (_retention, _pause_handle_kept) =
        open_ontologies::retention::RetentionWorker::spawn_with_pause(
            db.clone(),
            cfg.retention.clone(),
            pause_handle.clone(),
        );

    // R7 WA2 — A2 V1 Receipt-Chain Verifier. ZERO LLM by invariant —
    // crypto verdicts are reproducible bit-for-bit from
    // `(receipt_row, trusted_keys_history_row)`. Shares the same
    // `pause_handle` atomic with the retention worker; on a corruption
    // verdict the verifier calls `fetch_max(now + pause_secs)` so
    // retention skips its next tick. Monotone — never shortens a pause.
    let verifier_ocel = std::sync::Arc::new(
        open_ontologies::ocel_store::OcelStore::new(db.clone()),
    );
    let (_verifier, _verifier_cursor) =
        open_ontologies::verifier_worker::VerifierWorker::spawn_with_cursor(
            db,
            verifier_ocel,
            cfg.verifier.clone(),
            pause_handle,
        );
    tokio::runtime::Handle::current().block_on(async {
        let service = server.serve(rmcp::transport::stdio()).await
            .map_err(|e| anyhow::anyhow!(e))?;
        service.waiting().await.map_err(|e| anyhow::anyhow!(e))?;
        Ok::<(), anyhow::Error>(())
    }).map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))
}

fn auto_restore_last_ontology(db: &StateDb, graph: Arc<GraphStore>) -> NounVerbResult<()> {
    if let Ok(Some(path)) = db.get_last_active_path()
        && std::path::Path::new(&path).exists() {
            match graph.load_file(&path) {
                Ok(n) => eprintln!("info: restored last active ontology from {path} ({n} triples)"),
                Err(e) => eprintln!("warn: could not restore last active ontology: {e}"),
            }
        }
    Ok(())
}

fn run_unix_server(socket_path: String, files: Vec<String>) -> NounVerbResult<()> {
    let graph = Arc::new(GraphStore::new());
    for f in &files {
        let path = expand_tilde(f);
        match graph.load_file(&path) {
            Ok(n) => eprintln!("Loaded {path}: {n} triples"),
            Err(e) => { eprintln!("Failed to load {path}: {e}"); std::process::exit(1); }
        }
    }
    eprintln!("Graph has {} triples total", graph.triple_count());
    tokio::runtime::Handle::current()
        .block_on(open_ontologies::socket::serve(&socket_path, graph))
        .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))
}

fn build_http_axum_router(cfg: &Config, shared_graph: Arc<GraphStore>, shared_db: StateDb, governance_webhook: Option<String>, token: Option<String>, cache_config: open_ontologies::config::CacheConfig, tool_filter: open_ontologies::toolfilter::ToolFilter) -> (axum::Router, String, u16, tokio_util::sync::CancellationToken) {
    use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager};
    use tokio_util::sync::CancellationToken;

    let host = open_ontologies::config::resolve_http_host(&cfg.http);
    let port = open_ontologies::config::resolve_http_port(&cfg.http);
    let resolved_token = token.or_else(|| open_ontologies::config::resolve_http_token(&cfg.http));
    let cors_origins = open_ontologies::config::resolve_cors_origins(&cfg.http);
    let rate_limit_rps = open_ontologies::config::resolve_http_rate_limit_rps(&cfg.http);

    let ct = CancellationToken::new();
    let mut http_config = StreamableHttpServerConfig::default();
    http_config.stateful_mode = cfg.http.stateful_mode;
    http_config.cancellation_token = ct.clone();

    let db_path = std::path::Path::new(&expand_tilde(&cfg.general.data_dir)).join("open-ontologies.db");
    let gw = governance_webhook.clone();
    let embed = cfg.embeddings.clone();
    let cc = cache_config.clone();
    let tf = tool_filter.clone();
    let dirs = open_ontologies::config::resolve_ontology_dirs(&cfg.general.ontology_dirs);
    let sg = shared_graph.clone();
    let llm_engine = open_ontologies::config::resolve_llm_engine(&cfg.llm);
    let llm_engine_for_factory = llm_engine.clone();
    eprintln!("info: default LLM engine = {}", llm_engine);
    // R5 WC-1 — resolve admin principal allowlist ONCE at HTTP startup.
    // Cloned into each per-request server (the cache itself is an Arc, so
    // the actual Vec is shared, not duplicated). Env-var mutations
    // post-startup are ignored across all subsequent factory invocations.
    let admin_principals_for_factory =
        open_ontologies::config::resolve_admin_principals(&cfg.authority);
    eprintln!(
        "info: admin principals configured = {} entries",
        admin_principals_for_factory.len()
    );

    // R5 WC-2 — resolve the X-Ontostar-Tenant allowlist ONCE at HTTP
    // startup. Empty list preserves backwards-compat (any well-formed
    // tenant accepted); non-empty list enforces strict allowlist with
    // 403 on unknown.
    let known_tenants_for_layer = std::sync::Arc::new(
        open_ontologies::config::resolve_known_tenants(&cfg.authority),
    );
    eprintln!(
        "info: known tenants allowlist = {} entries (empty = open)",
        known_tenants_for_layer.len()
    );
    // Admin allowlist Arc for the principal_extract_layer — shared with
    // the factory closure (same Vec resolved once).
    let admin_principals_for_layer =
        std::sync::Arc::new(admin_principals_for_factory.clone());

    let service: StreamableHttpService<_, LocalSessionManager> = StreamableHttpService::new(
        move || {
            let db = StateDb::open(&db_path).map_err(std::io::Error::other)?;
            // Phase 11: per-request tenant rebind. The factory closure runs
            // once per HTTP request; reading the tenant from the
            // `TENANT_OVERRIDE` task-local (set by `tenant_extract_layer`)
            // means this server instance is bound to the tenant declared
            // in the `X-Ontostar-Tenant` header for the lifetime of the
            // call. Concurrent requests cannot leak across each other
            // because each gets its own task-local scope and its own
            // freshly-cloned `OpenOntologiesServer`.
            let tenant = open_ontologies::server::current_tenant_override()
                .unwrap_or_else(|| "default".to_string());
            Ok(OpenOntologiesServer::new_with_repo_options(db, sg.clone(), gw.clone(), embed.clone(), cc.clone(), tf.clone(), dirs.clone())
                .with_default_llm_engine(llm_engine_for_factory.clone())
                .with_admin_principals(admin_principals_for_factory.clone())
                .with_tenant(&tenant))
        },
        Default::default(),
        http_config,
    );

    let llm_cfg_for_health = cfg.llm.clone();
    let api = build_api_router(shared_graph, shared_db, llm_cfg_for_health);

    // Health endpoint bypasses auth middleware by being on separate router
    let health = axum::Router::new()
        .route("/health", axum::routing::get(|| async {
            axum::Json(serde_json::json!({
                "status": "ok",
                "version": env!("CARGO_PKG_VERSION"),
            }))
        }));

    let mut router = axum::Router::new().nest("/api", api).nest_service("/mcp", service);

    // R5 WC-2 — X-Ontostar-Principal extraction layer. Wired AFTER the
    // bearer-token layer (added below) so the caller's principal is
    // known. When the header is set, only callers in the admin
    // allowlist may carry it; non-admin → 403 with FalsePass shape.
    // Empty header is silently allowed (default principal resolution
    // unchanged). Layers in axum apply outside-in (last `.layer(...)`
    // runs first), so this is added AFTER tenant_extract_layer.
    {
        let admins = admin_principals_for_layer.clone();
        router = router.layer(axum::middleware::from_fn(
            move |req: axum::extract::Request, next: axum::middleware::Next| {
                let admins = admins.clone();
                async move { principal_extract_layer(admins, req, next).await }
            },
        ));
    }

    // R5 WC-2 — X-Ontostar-Tenant extraction layer with allowlist
    // enforcement. Empty `known_tenants` preserves Phase 11 behaviour
    // (any well-formed tenant accepted); non-empty enforces strict
    // 403 on unknown. Validates the value against
    // `^[a-z][a-z0-9_-]{0,63}$` and parks it in the `TENANT_OVERRIDE`
    // task-local for the per-request server factory. Must precede
    // `llm_engine_extract_layer` so layers compose correctly.
    {
        let known = known_tenants_for_layer.clone();
        router = router.layer(axum::middleware::from_fn(
            move |req: axum::extract::Request, next: axum::middleware::Next| {
                let known = known.clone();
                async move { tenant_extract_layer_with_allowlist(known, req, next).await }
            },
        ));
    }

    // X-Ontostar-LLM-Engine extraction layer. Validates the value
    // against `config::VALID_LLM_ENGINES` and parks it in the
    // `LLM_ENGINE_OVERRIDE` task-local for downstream tool handlers.
    router = router.layer(axum::middleware::from_fn(llm_engine_extract_layer));

    if let Some(ref t) = resolved_token {
        let expected = format!("Bearer {}", t);
        router = router.layer(axum::middleware::from_fn(move |req: axum::extract::Request, next: axum::middleware::Next| {
            let expected = expected.clone();
            async move {
                let auth = req.headers().get("authorization").and_then(|v| v.to_str().ok());
                if auth == Some(&expected) { next.run(req).await }
                else { axum::http::Response::builder().status(401).body(axum::body::Body::from("Unauthorized")).unwrap() }
            }
        }));
    }

    // CORS: empty origins list = permissive (dev); non-empty = strict allowlist
    if cors_origins.is_empty() {
        router = router.layer(tower_http::cors::CorsLayer::permissive());
    } else {
        use tower_http::cors::AllowOrigin;
        let parsed_origins: Vec<_> = cors_origins.into_iter().filter_map(|o| o.parse().ok()).collect();
        if !parsed_origins.is_empty() {
            router = router.layer(tower_http::cors::CorsLayer::new().allow_origin(AllowOrigin::list(parsed_origins)));
        }
    }

    // Rate limiting: configuration available via rate_limit_rps (implementation pending)
    let _rate_limit_rps = rate_limit_rps;

    let router = health.merge(router);
    (router, host, port, ct)
}

/// Validate a tenant_id against `^[a-z][a-z0-9_-]{0,63}$`. Implemented
/// without `regex` to avoid pulling a new top-level dep.
pub(crate) fn is_valid_tenant_id(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    if !bytes[0].is_ascii_lowercase() {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_' || *b == b'-')
}

/// Read the `X-Ontostar-Tenant` header (if any), validate it against
/// `^[a-z][a-z0-9_-]{0,63}$`, and run the downstream handler with
/// [`open_ontologies::server::TENANT_OVERRIDE`] set. An invalid /
/// missing header falls back to `"default"` (the server's compile-time
/// default tenant) — single-tenant deployments work unchanged.
///
/// **R5 WC-2**: this function preserves the original (pre-allowlist)
/// behaviour for tests and code paths that don't have the allowlist
/// available. The HTTP router uses
/// [`tenant_extract_layer_with_allowlist`] for the allowlist-enforced
/// path.
#[allow(dead_code)] // retained for test compat; HTTP router uses allowlist variant
pub(crate) async fn tenant_extract_layer(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let header_val = req
        .headers()
        .get("x-ontostar-tenant")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .filter(|s| is_valid_tenant_id(s));
    let tenant = header_val.unwrap_or_else(|| "default".to_string());
    open_ontologies::server::TENANT_OVERRIDE
        .scope(Some(tenant), next.run(req))
        .await
}

/// R5 WC-2 — strict-allowlist variant of [`tenant_extract_layer`].
///
/// When `allowlist` is empty: behaves exactly like `tenant_extract_layer`
/// (backward-compat — any well-formed tenant accepted, invalid /
/// missing falls back to `"default"`).
///
/// When `allowlist` is non-empty:
///   * If the header is set AND its value is in the allowlist: parks it
///     in `TENANT_OVERRIDE` and passes through.
///   * If the header is set AND its value is NOT in the allowlist:
///     returns HTTP 403 with `FalsePass { reason: "tenant_not_in_allowlist" }`.
///     The downstream factory is never invoked.
///   * If the header is unset / blank / invalid syntax: falls back to
///     `"default"` (single-tenant operators set the env / config to
///     match their `tenant_id`).
///
/// This intentionally enforces 403 instead of silent fallback for
/// known-invalid tenants — closes the §28 path where a malicious
/// caller spoofs a tenant header and gets quietly downgraded to
/// `default`.
pub(crate) async fn tenant_extract_layer_with_allowlist(
    allowlist: std::sync::Arc<Vec<String>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let header_raw = req
        .headers()
        .get("x-ontostar-tenant")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if !allowlist.is_empty()
        && let Some(ref hv) = header_raw {
            // Caller explicitly set the header — strictly validate.
            if !is_valid_tenant_id(hv) || !allowlist.iter().any(|t| t == hv) {
                let body = serde_json::json!({
                    "ok": false,
                    "defect": {
                        "kind": "FalsePass",
                        "reason": "tenant_not_in_allowlist",
                    },
                    "error": format!("tenant '{}' is not in OPEN_ONTOLOGIES_KNOWN_TENANTS allowlist",
                        hv.replace('"', "'")),
                })
                .to_string();
                return axum::http::Response::builder()
                    .status(403)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(body))
                    .unwrap();
            }
        }
    let tenant = header_raw
        .filter(|s| is_valid_tenant_id(s))
        .unwrap_or_else(|| "default".to_string());
    open_ontologies::server::TENANT_OVERRIDE
        .scope(Some(tenant), next.run(req))
        .await
}

/// R5 WC-2 — principal override extraction layer.
///
/// Reads the `X-Ontostar-Principal` header (if any). When set, the
/// caller MUST appear in the admin allowlist; a non-admin caller
/// presenting the header receives HTTP 403 with
/// `FalsePass { reason: "principal_override_requires_admin" }`. When
/// the header is unset, default principal resolution is unchanged
/// (the per-request server factory uses the tenant_id as principal_id
/// fallback).
///
/// The admin gate uses the same allowlist resolved at startup for
/// `OpenOntologiesServer::admin_principals` — TOCTOU-immune by virtue
/// of being captured into this closure once at router construction.
///
/// NOTE: this layer must be wired AFTER the bearer-token layer (in axum
/// `.layer(...)` order, which is outside-in: later `.layer` runs
/// first) so the bearer token has already been validated by the time
/// we check the principal header. The bearer token authenticates the
/// caller; this layer authorises an admin override of the per-request
/// principal identity.
async fn principal_extract_layer(
    admin_allowlist: std::sync::Arc<Vec<String>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let header_val = req
        .headers()
        .get("x-ontostar-principal")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(ref pid) = header_val {
        // The header is set — admin gate.
        // Until R3 Task B's principal helper lands, we authorise on
        // the same identity space as `is_admin_principal`: an admin
        // is a tenant_id in the cached allowlist. The tenant header
        // carries that identity. If the tenant header is unset, the
        // override is rejected (no default-tenant admin).
        let caller_tenant = req
            .headers()
            .get("x-ontostar-tenant")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_default();
        let is_admin = !admin_allowlist.is_empty()
            && admin_allowlist.iter().any(|p| p == &caller_tenant);
        if !is_admin {
            let body = serde_json::json!({
                "ok": false,
                "defect": {
                    "kind": "FalsePass",
                    "reason": "principal_override_requires_admin",
                },
                "error": format!(
                    "X-Ontostar-Principal='{}' presented by non-admin caller (tenant='{}')",
                    pid.replace('"', "'"),
                    caller_tenant.replace('"', "'")
                ),
            })
            .to_string();
            return axum::http::Response::builder()
                .status(403)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(body))
                .unwrap();
        }
        // Admin caller — accept the override. Currently we do not have
        // a per-request principal task-local (R3 Task B's territory);
        // when it lands, install it here. The header presence is
        // already audit-logged via the request log.
    }
    next.run(req).await
}

/// Read the `X-Ontostar-LLM-Engine` header (if any), validate it
/// against [`open_ontologies::config::VALID_LLM_ENGINES`], and run the
/// downstream handler with [`open_ontologies::server::LLM_ENGINE_OVERRIDE`]
/// set. Unknown / blank values are silently dropped (the server default
/// then applies) — the goal is graceful degradation, not authentication.
async fn llm_engine_extract_layer(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let header_val = req
        .headers()
        .get("x-ontostar-llm-engine")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .filter(|s| open_ontologies::config::VALID_LLM_ENGINES.contains(&s.as_str()));
    open_ontologies::server::LLM_ENGINE_OVERRIDE
        .scope(header_val, next.run(req))
        .await
}

fn build_api_router(shared_graph: Arc<GraphStore>, shared_db: StateDb, llm_cfg: open_ontologies::config::LlmConfig) -> axum::Router {
    let sg_stats = shared_graph.clone();
    let sg_query = shared_graph.clone();
    let sg_update = shared_graph.clone();
    let sg_load = shared_graph.clone();
    let sg_save = shared_graph.clone();
    let sg_turtle = shared_graph.clone();
    let llm_cfg_health = llm_cfg.clone();

    axum::Router::new()
        // ── Plan 4: GET /health/llm ───────────────────────────────────────
        // Spawns scripts/groq_status.py once and returns the JSON line it
        // writes to stdout, alongside the resolved engine. NEVER logs or
        // returns the API key. Returns the same shape as `onto_groq_status`
        // plus an `engine` field; a `key_present=false` body means the
        // resolver auto-detected `inproc` (no remote probe possible).
        .route("/health/llm", axum::routing::get(move || {
            let cfg = llm_cfg_health.clone();
            async move { axum::Json(health_llm_probe(&cfg).await) }
        }))
        .route("/stats", axum::routing::get(move || { let g = sg_stats.clone(); async move { axum::Json(serde_json::from_str::<serde_json::Value>(&g.get_stats().unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))).unwrap_or_default()) } }))
        .route("/query", axum::routing::post(move |body: axum::Json<serde_json::Value>| { let g = sg_query.clone(); async move { let q = body.0["query"].as_str().unwrap_or("").to_string(); axum::Json(serde_json::from_str::<serde_json::Value>(&g.sparql_select(&q).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))).unwrap_or_default()) } }))
        .route("/update", axum::routing::post(move |body: axum::Json<serde_json::Value>| { let g = sg_update.clone(); async move { let q = body.0["query"].as_str().unwrap_or("").to_string(); axum::Json(serde_json::from_str::<serde_json::Value>(&match g.sparql_update(&q) { Ok(n) => format!(r#"{{"ok":true,"affected":{}}}"#, n), Err(e) => format!(r#"{{"error":"{}"}}"#, e) }).unwrap_or_default()) } }))
        .route("/load", axum::routing::post(move |body: axum::Json<serde_json::Value>| { let g = sg_load.clone(); async move { let p = expand_tilde(body.0["path"].as_str().unwrap_or("")); axum::Json(serde_json::from_str::<serde_json::Value>(&match g.load_file(&p) { Ok(n) => format!(r#"{{"ok":true,"triples_loaded":{}}}"#, n), Err(e) => format!(r#"{{"error":"{}"}}"#, e) }).unwrap_or_default()) } }))
        .route("/load-turtle", axum::routing::post(move |body: axum::Json<serde_json::Value>| { let g = sg_turtle.clone(); async move { let t = body.0["turtle"].as_str().unwrap_or("").to_string(); let b = body.0["base"].as_str().map(|s| s.to_string()); axum::Json(serde_json::from_str::<serde_json::Value>(&match g.load_turtle(&t, b.as_deref()) { Ok(n) => format!(r#"{{"ok":true,"triples_loaded":{}}}"#, n), Err(e) => format!(r#"{{"error":"{}"}}"#, e) }).unwrap_or_default()) } }))
        .route("/save", axum::routing::post(move |body: axum::Json<serde_json::Value>| { let g = sg_save.clone(); async move { let p = expand_tilde(body.0["path"].as_str().unwrap_or("~/.open-ontologies/studio-live.ttl")); let f = body.0["format"].as_str().unwrap_or("turtle").to_string(); axum::Json(serde_json::from_str::<serde_json::Value>(&match g.save_file(&p, &f) { Ok(_) => format!(r#"{{"ok":true,"path":"{}"}}"#, p), Err(e) => format!(r#"{{"error":"{}"}}"#, e) }).unwrap_or_default()) } }))
        .route("/lineage", axum::routing::get(move || {
            let db = shared_db.clone();
            async move {
                let conn = db.conn();
                let mut stmt = conn.prepare("SELECT session_id, seq, timestamp, event_type, operation, details FROM lineage_events ORDER BY CAST(timestamp AS INTEGER) ASC, seq ASC LIMIT 500").unwrap();
                let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
                    Ok(serde_json::json!({"session": row.get::<_,String>(0)?, "seq": row.get::<_,i64>(1)?, "ts": row.get::<_,String>(2)?, "type": row.get::<_,String>(3)?, "op": row.get::<_,String>(4)?, "details": row.get::<_,Option<String>>(5)?.unwrap_or_default()}))
                }).unwrap().filter_map(|r| r.ok()).collect();
                axum::Json(serde_json::json!({"events": rows}))
            }
        }))
}

/// `GET /health/llm` handler body. Returns:
/// `{ ok, engine, model_reachable, key_present, model, error? }`.
/// Spawns `scripts/groq_status.py` once when the resolved engine is
/// `groq_pm4py`; otherwise short-circuits with a static answer that
/// only reports whether a key is configured. Never logs the key.
async fn health_llm_probe(cfg: &open_ontologies::config::LlmConfig) -> serde_json::Value {
    let engine = open_ontologies::config::resolve_llm_engine(cfg);
    let key_present = open_ontologies::config::resolve_llm_api_key(cfg).is_some();
    if engine != "groq_pm4py" {
        return serde_json::json!({
            "ok": true,
            "engine": engine,
            "model_reachable": false,
            "key_present": key_present,
            "model": "",
        });
    }
    let python = open_ontologies::config::resolve_llm_python(cfg);
    let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/groq_status.py");
    let out = match tokio::task::spawn_blocking(move || {
        std::process::Command::new(&python).arg(&script).output()
    })
    .await
    {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            return serde_json::json!({
                "ok": false,
                "engine": engine,
                "model_reachable": false,
                "key_present": key_present,
                "model": "",
                "error": format!("failed to spawn groq_status.py: {e}"),
            });
        }
        Err(e) => {
            return serde_json::json!({
                "ok": false,
                "engine": engine,
                "model_reachable": false,
                "key_present": key_present,
                "model": "",
                "error": format!("join error: {e}"),
            });
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let json_line = stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .map(|s| s.trim().to_string());
    let mut resp = match json_line.and_then(|l| serde_json::from_str::<serde_json::Value>(&l).ok()) {
        Some(v) => v,
        None => serde_json::json!({
            "ok": false,
            "model_reachable": false,
            "key_present": key_present,
            "model": "",
            "error": format!("groq_status.py produced no JSON: stderr={}",
                stderr.replace('"', "'").replace('\n', " ")),
        }),
    };
    if let Some(obj) = resp.as_object_mut() {
        obj.insert("engine".to_string(), serde_json::Value::String(engine));
    }
    resp
}

// ── verbs ─────────────────────────────────────────────────────────────────

/// Apply --llm-engine / --llm-python overrides into the process
/// environment so [`open_ontologies::config::resolve_llm_engine`] picks
/// them up uniformly with config + auto-detect. Must be called before
/// `Config::load` so resolution is consistent.
fn apply_llm_cli_overrides(llm_engine: Option<&str>, llm_python: Option<&str>) -> NounVerbResult<()> {
    if let Some(e) = llm_engine.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if !open_ontologies::config::VALID_LLM_ENGINES.contains(&e) {
            return Err(clap_noun_verb::NounVerbError::execution_error(format!(
                "invalid --llm-engine={:?}; valid values: {:?}",
                e,
                open_ontologies::config::VALID_LLM_ENGINES
            )));
        }
        // SAFETY: process is single-threaded at CLI bootstrap time.
        unsafe { std::env::set_var("OPEN_ONTOLOGIES_LLM_ENGINE", e); }
    }
    if let Some(p) = llm_python.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        // SAFETY: process is single-threaded at CLI bootstrap time.
        unsafe { std::env::set_var("OPEN_ONTOLOGIES_LLM_PYTHON", p); }
    }
    Ok(())
}

/// Start the MCP server (stdio transport)
#[allow(clippy::too_many_arguments)] // Every parameter is a CLI flag exposed by clap_noun_verb; struct-wrapping would lose the auto-derived argument metadata.
#[verb]
fn serve(config: Option<String>, governance_webhook: Option<String>, watch: Option<bool>, watch_interval: Option<u64>, tools_allow: Option<String>, tools_deny: Option<String>, idle_ttl_secs: Option<u64>, auto_refresh: Option<bool>, llm_engine: Option<String>, llm_python: Option<String>) -> NounVerbResult<ServeOutput> {
    // Load .env into the process environment before resolving config so the
    // Groq translator can pick up GROQ_API_KEY without leaking it to a
    // shell. Best-effort: missing .env is not an error.
    dotenvy::dotenv().ok();
    apply_llm_cli_overrides(llm_engine.as_deref(), llm_python.as_deref())?;
    let cfg = load_cfg(config.as_deref().unwrap_or(DEFAULT_CONFIG_PATH))
        .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
    init_tracing_cfg(&cfg.logging);
    open_ontologies::runtime::init_from_config(&cfg);
    let db_path_str = format!("{}/open-ontologies.db", expand_tilde(&cfg.general.data_dir));
    let (_, db, graph) = open_db_and_graph(&cfg.general.data_dir)?;
    auto_restore_last_ontology(&db, graph.clone())?;
    maybe_start_monitor(watch.unwrap_or(false), &cfg, &db_path_str, watch_interval, graph.clone())?;
    let cache_config = build_cache_cfg(&cfg, idle_ttl_secs, auto_refresh.unwrap_or(false));
    let tool_filter = build_tool_filter_cfg(&cfg, tools_allow.as_deref(), tools_deny.as_deref())
        .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
    run_stdio_server(cfg, db, graph, governance_webhook, cache_config, tool_filter)?;
    Ok(ServeOutput { status: "done".to_string() })
}

/// Start the MCP server (Streamable HTTP transport)
#[allow(clippy::too_many_arguments)] // Every parameter is a CLI flag exposed by clap_noun_verb; struct-wrapping would lose the auto-derived argument metadata.
#[verb]
fn serve_http(config: Option<String>, host: Option<String>, port: Option<u16>, token: Option<String>, governance_webhook: Option<String>, watch: Option<bool>, watch_interval: Option<u64>, tools_allow: Option<String>, tools_deny: Option<String>, idle_ttl_secs: Option<u64>, auto_refresh: Option<bool>, llm_engine: Option<String>, llm_python: Option<String>) -> NounVerbResult<ServeOutput> {
    dotenvy::dotenv().ok();
    apply_llm_cli_overrides(llm_engine.as_deref(), llm_python.as_deref())?;
    let cfg = load_cfg(config.as_deref().unwrap_or(DEFAULT_CONFIG_PATH))
        .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
    init_tracing_cfg(&cfg.logging);
    open_ontologies::runtime::init_from_config(&cfg);
    let db_path_str = format!("{}/open-ontologies.db", expand_tilde(&cfg.general.data_dir));
    let (_, shared_db, shared_graph) = open_db_and_graph(&cfg.general.data_dir)?;
    auto_restore_last_ontology(&shared_db, shared_graph.clone())?;
    maybe_start_monitor(watch.unwrap_or(false), &cfg, &db_path_str, watch_interval, shared_graph.clone())?;
    let cache_config = build_cache_cfg(&cfg, idle_ttl_secs, auto_refresh.unwrap_or(false));
    let tool_filter = build_tool_filter_cfg(&cfg, tools_allow.as_deref(), tools_deny.as_deref())
        .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
    let resolved_host = host.unwrap_or_else(|| open_ontologies::config::resolve_http_host(&cfg.http));
    let resolved_port = port.unwrap_or_else(|| open_ontologies::config::resolve_http_port(&cfg.http));
    let (router, _, _, ct) = build_http_axum_router(&cfg, shared_graph, shared_db, governance_webhook, token, cache_config, tool_filter);
    tokio::runtime::Handle::current().block_on(async {
        let addr = format!("{resolved_host}:{resolved_port}");
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        eprintln!("Open Ontologies MCP server listening on http://{addr}/mcp");
        axum::serve(listener, router).with_graceful_shutdown(async move { ct.cancelled_owned().await }).await
    }).map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
    Ok(ServeOutput { status: "done".to_string() })
}

/// Start unix socket server for Tardygrada fact grounding
#[verb]
fn serve_unix(config: Option<String>, socket: Option<String>, files_csv: Option<String>) -> NounVerbResult<ServeOutput> {
    dotenvy::dotenv().ok();
    let cfg = load_cfg(config.as_deref().unwrap_or(DEFAULT_CONFIG_PATH))
        .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
    init_tracing_cfg(&cfg.logging);
    open_ontologies::runtime::init_from_config(&cfg);
    let socket_path = socket.or_else(|| cfg.socket.path.clone())
        .unwrap_or_else(|| "/tmp/tardygrada-ontology-complete.sock".to_string());
    let preload = if let Some(csv) = files_csv.filter(|s| !s.is_empty()) {
        csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    } else {
        cfg.socket.preload_files.clone()
    };
    run_unix_server(socket_path, preload)?;
    Ok(ServeOutput { status: "done".to_string() })
}

/// Initialize data directory, DB, and default config
#[verb]
fn init(data_dir: Option<String>, model_url: Option<String>, tokenizer_url: Option<String>, model_name: Option<String>) -> NounVerbResult<InitOutput> {
    // Option B: model_url/tokenizer_url/model_name are accepted as CLI flags but not yet wired.
    // Reject explicit values loudly so users aren't misled into thinking the download happened.
    if model_url.is_some() || tokenizer_url.is_some() || model_name.is_some() {
        return Err(clap_noun_verb::NounVerbError::execution_error(
            "model_url/tokenizer_url/model_name are not yet supported by `onto init`. Manually copy the model files into ~/.open-ontologies/models/ until this is wired.".to_string(),
        ));
    }
    let dir_str = data_dir.unwrap_or_else(|| "~/.open-ontologies".to_string());
    let dir_expanded = expand_tilde(&dir_str);
    let data_path = std::path::Path::new(&dir_expanded);
    std::fs::create_dir_all(data_path)
        .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
    let db_path = data_path.join("open-ontologies.db");
    let _db = StateDb::open(&db_path)
        .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
    let config_path = data_path.join("config.toml");
    let config_created = if !config_path.exists() {
        std::fs::write(&config_path, INIT_CONFIG_TEMPLATE)
            .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
        true
    } else { false };
    Ok(InitOutput {
        ok: true,
        data_dir: dir_expanded,
        db: db_path.display().to_string(),
        config: config_path.display().to_string(),
        config_created,
    })
}
