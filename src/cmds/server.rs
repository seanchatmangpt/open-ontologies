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
    let server = OpenOntologiesServer::new_with_repo_options(db, graph, governance_webhook, cfg.embeddings, cache_config, tool_filter, ontology_dirs);
    let _evictor = open_ontologies::registry::spawn_evictor(server.registry());
    tokio::runtime::Handle::current().block_on(async {
        let service = server.serve(rmcp::transport::stdio()).await
            .map_err(|e| anyhow::anyhow!(e))?;
        service.waiting().await.map_err(|e| anyhow::anyhow!(e))?;
        Ok::<(), anyhow::Error>(())
    }).map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))
}

fn auto_restore_last_ontology(db: &StateDb, graph: Arc<GraphStore>) -> NounVerbResult<()> {
    if let Ok(Some(path)) = db.get_last_active_path() {
        if std::path::Path::new(&path).exists() {
            match graph.load_file(&path) {
                Ok(n) => eprintln!("info: restored last active ontology from {path} ({n} triples)"),
                Err(e) => eprintln!("warn: could not restore last active ontology: {e}"),
            }
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

    let service: StreamableHttpService<_, LocalSessionManager> = StreamableHttpService::new(
        move || {
            let db = StateDb::open(&db_path).map_err(std::io::Error::other)?;
            Ok(OpenOntologiesServer::new_with_repo_options(db, sg.clone(), gw.clone(), embed.clone(), cc.clone(), tf.clone(), dirs.clone()))
        },
        Default::default(),
        http_config,
    );

    let api = build_api_router(shared_graph, shared_db);
    let mut router = axum::Router::new().nest("/api", api).nest_service("/mcp", service);

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
    router = router.layer(tower_http::cors::CorsLayer::permissive());
    (router, host, port, ct)
}

fn build_api_router(shared_graph: Arc<GraphStore>, shared_db: StateDb) -> axum::Router {
    let sg_stats = shared_graph.clone();
    let sg_query = shared_graph.clone();
    let sg_update = shared_graph.clone();
    let sg_load = shared_graph.clone();
    let sg_save = shared_graph.clone();
    let sg_turtle = shared_graph.clone();

    axum::Router::new()
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

// ── verbs ─────────────────────────────────────────────────────────────────

/// Start the MCP server (stdio transport)
#[verb]
fn serve(config: Option<String>, governance_webhook: Option<String>, watch: Option<bool>, watch_interval: Option<u64>, tools_allow: Option<String>, tools_deny: Option<String>, idle_ttl_secs: Option<u64>, auto_refresh: Option<bool>) -> NounVerbResult<ServeOutput> {
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
#[verb]
fn serve_http(config: Option<String>, host: Option<String>, port: Option<u16>, token: Option<String>, governance_webhook: Option<String>, watch: Option<bool>, watch_interval: Option<u64>, tools_allow: Option<String>, tools_deny: Option<String>, idle_ttl_secs: Option<u64>, auto_refresh: Option<bool>) -> NounVerbResult<ServeOutput> {
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
    let _ = (model_url, tokenizer_url, model_name);
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
        std::fs::write(&config_path, "[general]\ndata_dir = \"~/.open-ontologies\"\n")
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
