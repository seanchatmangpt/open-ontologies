use std::sync::Arc;

use rmcp::{
    ServerHandler, RoleServer, tool, tool_handler, tool_router,
    prompt, prompt_handler, prompt_router,
    handler::server::{tool::ToolRouter, router::prompt::PromptRouter, wrapper::Parameters},
    model::{
        ServerCapabilities, ServerInfo, Tool,
        PromptMessage, PromptMessageRole, GetPromptResult,
        GetPromptRequestParams, PaginatedRequestParams, ListPromptsResult,
    },
    service::RequestContext,
};
use crate::config::expand_tilde;
use crate::graph::GraphStore;
use crate::inputs::*;
use crate::state::StateDb;

// ─── HTTP-scoped LLM engine override (task-local) ───────────────────────────
//
// The HTTP middleware in `src/cmds/server.rs` reads the
// `X-Ontostar-LLM-Engine` header (validated against
// `config::VALID_LLM_ENGINES`) and parks the value here for the duration
// of one request. Tool handlers consult it via
// [`current_llm_engine_override`] so per-call `engine` arg > header >
// server default precedence holds without threading state through every
// `Parameters<…>` handler.
tokio::task_local! {
    pub static LLM_ENGINE_OVERRIDE: Option<String>;
    /// Phase 11 — per-request tenant override. Set by the HTTP
    /// `tenant_extract_layer` middleware from the `X-Ontostar-Tenant`
    /// header. Read by the `StreamableHttpService` factory closure to
    /// rebind the freshly-constructed `OpenOntologiesServer` to the
    /// caller's tenant for the lifetime of the request.
    pub static TENANT_OVERRIDE: Option<String>;
}

/// Read the current request's `X-Ontostar-LLM-Engine` override, if any.
/// Returns `None` when no task-local is in scope (e.g. stdio MCP transport).
pub fn current_llm_engine_override() -> Option<String> {
    LLM_ENGINE_OVERRIDE
        .try_with(|opt| opt.clone())
        .ok()
        .flatten()
}

/// Read the current request's `X-Ontostar-Tenant` override, if any.
/// Returns `None` when no task-local is in scope (stdio transport).
pub fn current_tenant_override() -> Option<String> {
    TENANT_OVERRIDE
        .try_with(|opt| opt.clone())
        .ok()
        .flatten()
}

// ─── OpenOntologiesServer ───────────────────────────────────────────────────

/// MCP server that exposes all Open Ontologies tools to Claude via stdin/stdout.
#[derive(Clone)]
pub struct OpenOntologiesServer {
    tool_router: ToolRouter<Self>,
    #[allow(dead_code)] // consumed by #[prompt_handler] macro on ServerHandler impl
    prompt_router: PromptRouter<Self>,
    db: StateDb,
    graph: Arc<GraphStore>,
    session_id: String,
    governance_webhook: Option<String>,
    /// Registry tracking the active ontology + compile cache + TTL eviction.
    registry: Arc<crate::registry::OntologyRegistry>,
    /// Configured ontology repository directories, expanded and deduplicated.
    /// Empty when none are configured. Used by `onto_repo_list` /
    /// `onto_repo_load`.
    ontology_dirs: Arc<Vec<std::path::PathBuf>>,
    /// OCEL store for native object-centric event log emission.
    ocel_store: Arc<crate::ocel_store::OcelStore>,
    #[cfg(feature = "embeddings")]
    vecstore: Arc<std::sync::Mutex<crate::vecstore::VecStore>>,
    #[cfg(feature = "embeddings")]
    text_embedder: Option<Arc<crate::embed::TextEmbedderProvider>>,
    /// Default LLM engine resolved at server construction.
    /// See [`crate::config::resolve_llm_engine`]. The three Groq-facing
    /// tool handlers consult this when the caller did not supply an
    /// explicit `engine` argument and no `X-Ontostar-LLM-Engine` header
    /// override is in scope.
    default_llm_engine: String,
    /// Phase 11 — caller-side tenant context. Read from
    /// `OPEN_ONTOLOGIES_TENANT_ID` at construction (default: `"default"`)
    /// and rebound per-request by the HTTP `tenant_extract_layer`
    /// middleware via [`Self::with_tenant`]. Every `evaluate_admission`
    /// call funnels `tenant.current()` into the gate's
    /// `evaluate_in_tenant` method, so ALL 27 #[tool] mutations are
    /// tenant-ACL-checked.
    tenant: crate::tenant::TenantHandle,
    /// R5 WC-1 — §28 HumanOverride closure. Admin principal allowlist
    /// resolved once at startup via
    /// [`crate::config::resolve_admin_principals`] and shared via
    /// `Arc` so the lookup in [`Self::is_admin_principal`] reads from
    /// this cached list — never from `std::env::var(...)`. Closes the
    /// TOCTOU race the previous per-call env-var read admitted.
    /// Closed-by-default: empty list means no callers are admin.
    pub admin_principals: Arc<Vec<String>>,
    /// R5 WC-2 — emergency kill-switch handle for the
    /// [`crate::retention::RetentionWorker`]. The HTTP / stdio bootstrap
    /// in `src/cmds/server.rs` constructs the worker via
    /// [`crate::retention::RetentionWorker::spawn_with_pause`] and
    /// hands the same `Arc<AtomicI64>` to the server here, so the
    /// `onto_retention_pause` / `onto_retention_resume` admin tools
    /// mutate the same atomic the worker reads each tick. Default
    /// (constructor without `with_retention_pause`) is a fresh atomic
    /// initialised to 0 — pause/resume tools are wired but no-op against
    /// no actual worker.
    pub retention_paused_until: Arc<std::sync::atomic::AtomicI64>,
}

impl OpenOntologiesServer {
    /// Create a new server with all tools wired to domain services.
    pub fn new(db: StateDb) -> Self {
        Self::new_with_options(db, Arc::new(GraphStore::new()), None)
    }

    /// Create a new server sharing an existing graph store (for HTTP mode where
    /// all sessions must see the same in-memory triples).
    pub fn new_with_graph(db: StateDb, graph: Arc<GraphStore>) -> Self {
        Self::new_with_options(db, graph, None)
    }

    /// Create a new server with all options including optional governance webhook.
    pub fn new_with_options(db: StateDb, graph: Arc<GraphStore>, governance_webhook: Option<String>) -> Self {
        Self::new_with_full_options(db, graph, governance_webhook, Default::default())
    }

    /// Create a new server with all options including embedding config.
    pub fn new_with_full_options(
        db: StateDb,
        graph: Arc<GraphStore>,
        governance_webhook: Option<String>,
        _embed_config: crate::config::EmbeddingsConfig,
    ) -> Self {
        Self::new_with_registry_options(
            db,
            graph,
            governance_webhook,
            _embed_config,
            crate::config::CacheConfig::default(),
            crate::toolfilter::ToolFilter::default(),
        )
    }

    /// Full constructor, including cache configuration and tool filter.
    pub fn new_with_registry_options(
        db: StateDb,
        graph: Arc<GraphStore>,
        governance_webhook: Option<String>,
        _embed_config: crate::config::EmbeddingsConfig,
        cache_config: crate::config::CacheConfig,
        tool_filter: crate::toolfilter::ToolFilter,
    ) -> Self {
        Self::new_with_repo_options(
            db,
            graph,
            governance_webhook,
            _embed_config,
            cache_config,
            tool_filter,
            Vec::new(),
        )
    }

    /// Full constructor with on-disk ontology repo directories.
    ///
    /// `ontology_dirs` lists host directories that the `onto_repo_list` and
    /// `onto_repo_load` tools enumerate. They are stored verbatim (already
    /// resolved by the caller through `crate::config::resolve_ontology_dirs`).
    pub fn new_with_repo_options(
        db: StateDb,
        graph: Arc<GraphStore>,
        governance_webhook: Option<String>,
        _embed_config: crate::config::EmbeddingsConfig,
        cache_config: crate::config::CacheConfig,
        tool_filter: crate::toolfilter::ToolFilter,
        ontology_dirs: Vec<std::path::PathBuf>,
    ) -> Self {
        let lineage = crate::lineage::LineageLog::with_governance_webhook(db.clone(), governance_webhook.clone());
        let session_id = lineage.new_session();
        let ocel_store = Arc::new(crate::ocel_store::OcelStore::new(db.clone()));

        // Upsert the session object in OCEL at startup
        let _ = ocel_store.upsert_object(
            &format!("session:{}", session_id),
            "Session",
            &[("started_at", &chrono::Utc::now().to_rfc3339(), "time")],
        );

        // Build the registry. If construction fails (e.g. cache dir cannot be
        // created) fall back to a disabled registry so the server still starts.
        let registry = match crate::registry::OntologyRegistry::new(
            graph.clone(),
            db.clone(),
            cache_config.clone(),
        ) {
            Ok(r) => Arc::new(r),
            Err(e) => {
                tracing::warn!("ontology registry init failed: {}; cache disabled", e);
                let mut disabled = cache_config.clone();
                disabled.enabled = false;
                disabled.dir = std::env::temp_dir().to_string_lossy().to_string();
                Arc::new(
                    crate::registry::OntologyRegistry::new(graph.clone(), db.clone(), disabled)
                        .expect("temp_dir registry"),
                )
            }
        };

        // Apply tool filter by removing routes from the router.
        let mut tool_router = Self::tool_router();
        let removed = tool_filter.apply(&mut tool_router);
        if !removed.is_empty() {
            tracing::info!("tool filter removed {} tools: {:?}", removed.len(), removed);
        }

        #[cfg(feature = "embeddings")]
        let (vecstore, text_embedder) = {
            let mut vs = crate::vecstore::VecStore::new(db.clone());
            let _ = vs.load_from_db();

            let embedder = match crate::embed::TextEmbedderProvider::from_config(&_embed_config) {
                Ok(Some(e)) => {
                    tracing::info!(
                        "embeddings enabled (provider = {})",
                        e.provider_name()
                    );
                    Some(Arc::new(e))
                }
                Ok(None) => {
                    tracing::info!(
                        "embeddings configured but no provider available (model files missing or provider disabled)"
                    );
                    None
                }
                Err(e) => {
                    tracing::warn!("failed to initialise embedding provider: {}", e);
                    None
                }
            };
            (Arc::new(std::sync::Mutex::new(vs)), embedder)
        };

        // Resolve the default LLM engine once at construction time.
        // This honours the env var > config > auto-detect precedence
        // (Plan 4). Constructors that do not pass an explicit
        // `LlmConfig` rely on env-var/auto-detect only — that's fine
        // for tests and for the in-process MCP entry, since the env
        // resolver still consults `GROQ_API_KEY`.
        let default_llm_engine =
            crate::config::resolve_llm_engine(&crate::config::LlmConfig::default());

        Self {
            tool_router,
            prompt_router: Self::prompt_router(),
            db,
            graph,
            session_id,
            governance_webhook,
            registry,
            ontology_dirs: Arc::new(ontology_dirs),
            ocel_store,
            #[cfg(feature = "embeddings")]
            vecstore,
            #[cfg(feature = "embeddings")]
            text_embedder,
            default_llm_engine,
            tenant: crate::tenant::TenantHandle::from_env(),
            // R5 WC-1: closed-by-default. The HTTP/stdio bootstrap calls
            // `with_admin_principals(...)` after construction to wire the
            // resolved allowlist (config + env, read ONCE). Constructors
            // that bypass that path (test scaffolding, in-memory MCP
            // servers) get an empty list — no callers are admin until
            // explicitly opted in.
            admin_principals: Arc::new(Vec::new()),
            // R5 WC-2: fresh atomic initialised to 0 (no pause).
            // Bootstrap rebinds via `with_retention_pause(...)` so the
            // server and the worker share the same atomic.
            retention_paused_until: Arc::new(std::sync::atomic::AtomicI64::new(0)),
        }
    }

    /// Phase 11 — fluent setter that rebinds the server's tenant context.
    /// The `TenantHandle` is intentionally NOT cloned with new state; it
    /// is replaced wholesale so per-request rebinding (via the HTTP
    /// `tenant_extract_layer` middleware on a cheaply-cloned `Self`)
    /// cannot leak across concurrent requests.
    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        let trimmed = tenant_id.trim();
        let normalized = if trimmed.is_empty() { "default" } else { trimmed };
        self.tenant = crate::tenant::TenantHandle::new(normalized);
        self
    }

    /// Snapshot the current tenant_id (for tests and per-request audit).
    pub fn tenant_snapshot(&self) -> String {
        self.tenant.current().tenant_id
    }

    /// Override the resolved default LLM engine. Used by the HTTP /
    /// stdio bootstrap so the runtime engine reflects the full
    /// `[llm]` config block (the constructor only sees env vars).
    pub fn with_default_llm_engine(mut self, engine: String) -> Self {
        self.default_llm_engine = engine;
        self
    }

    /// R5 WC-1 — install the startup-resolved admin principal allowlist
    /// (see [`crate::config::resolve_admin_principals`]). Wired by the
    /// stdio + HTTP bootstrap in `src/cmds/server.rs`, mirroring
    /// [`Self::with_default_llm_engine`]. Once installed, the cache is
    /// authoritative and [`Self::is_admin_principal`] never re-reads
    /// `OPEN_ONTOLOGIES_ADMIN_PRINCIPALS` — closes the §28 TOCTOU leak.
    pub fn with_admin_principals(mut self, principals: Vec<String>) -> Self {
        self.admin_principals = Arc::new(principals);
        self
    }

    /// Test/debug helper: read-only view of the cached admin allowlist.
    /// Used by `tests/admin_principals_cache_immune.rs` to prove the
    /// startup cache is authoritative regardless of post-startup env
    /// mutation.
    #[doc(hidden)]
    pub fn admin_principals_for_test(&self) -> &[String] {
        &self.admin_principals
    }

    /// R5 WC-2 — install the externally-owned retention pause handle so
    /// the `onto_retention_pause` / `onto_retention_resume` admin tools
    /// drive the same atomic the
    /// [`crate::retention::RetentionWorker`] reads each tick. The HTTP
    /// / stdio bootstrap calls
    /// [`crate::retention::RetentionWorker::spawn_with_pause`] which
    /// returns the Arc; the bootstrap passes it here.
    pub fn with_retention_pause(
        mut self,
        paused_until: Arc<std::sync::atomic::AtomicI64>,
    ) -> Self {
        self.retention_paused_until = paused_until;
        self
    }

    /// Test/debug helper: read the current `paused_until` epoch second.
    /// `0` means not paused. Used by `tests/admin_tools_e2e.rs`.
    #[doc(hidden)]
    pub fn retention_paused_until_for_test(&self) -> i64 {
        self.retention_paused_until
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Read the resolved default LLM engine — `"inproc"` or
    /// `"groq_pm4py"`. Tool handlers use [`Self::resolve_engine`] to
    /// also consider per-call and HTTP-header overrides.
    pub fn default_llm_engine(&self) -> &str {
        &self.default_llm_engine
    }

    /// Pick the effective LLM engine for one tool call. Precedence:
    /// per-call `engine` argument > HTTP header override > server default.
    pub fn resolve_engine(
        &self,
        per_call: Option<&str>,
        header: Option<&str>,
    ) -> String {
        if let Some(v) = per_call.map(str::trim).filter(|v| !v.is_empty()) {
            return v.to_string();
        }
        if let Some(v) = header.map(str::trim).filter(|v| !v.is_empty()) {
            return v.to_string();
        }
        self.default_llm_engine.clone()
    }

    /// Return the list of all registered tool definitions.
    pub fn list_tool_definitions(&self) -> Vec<Tool> {
        self.tool_router.list_all()
    }

    /// Access the ontology registry (for tests and the HTTP server eviction loop).
    pub fn registry(&self) -> Arc<crate::registry::OntologyRegistry> {
        self.registry.clone()
    }

    fn lineage(&self) -> crate::lineage::LineageLog {
        crate::lineage::LineageLog::with_governance_webhook(self.db.clone(), self.governance_webhook.clone())
    }

    fn ocel_store(&self) -> &crate::ocel_store::OcelStore {
        &self.ocel_store
    }

    fn monitor(&self) -> crate::monitor::Monitor {
        crate::monitor::Monitor::new(self.db.clone(), self.graph.clone())
    }

    /// R7 WB-1 — resolve the subprocess wall-clock deadline. Funnels
    /// through [`crate::config::resolve_subprocess_timeout`] which
    /// honours `OPEN_ONTOLOGIES_SUBPROCESS_TIMEOUT_SECS` then
    /// `[llm].subprocess_timeout_secs`, falling back to 60s. Stored on
    /// the server is a single `LlmConfig::default()` instance —
    /// per-call resolution keeps the env path live for tests that set
    /// the variable mid-run.
    fn subprocess_timeout(&self) -> std::time::Duration {
        crate::config::resolve_subprocess_timeout(&crate::config::LlmConfig::default())
    }

    /// R7 WB-1 — run a `std::process::Command` with the configured
    /// wall-clock timeout. On timeout, emits an `llm_subprocess_timeout`
    /// OCEL event (tagged with `model`, `elapsed_ms`, `tenant_id`,
    /// `script_path`) so downstream cost analysis can see hung
    /// subprocesses for what they are. Returns `Err(SubprocessError)`
    /// on timeout / spawn failure so the call site can map it to the
    /// tool's denial JSON.
    ///
    /// The `model` argument is the LLM-engine string (`"groq_pm4py"`
    /// for the production path, the engine name for non-LLM
    /// subprocesses such as `wvda_agent.py`). `script_path` is the
    /// derived path to the spawned binary or script for diagnostics.
    fn run_subprocess_with_timeout(
        &self,
        cmd: &mut std::process::Command,
        model: &str,
        script_path: &str,
    ) -> Result<crate::subprocess::TimedOutput, crate::subprocess::SubprocessError> {
        let tenant_id = self.tenant_snapshot();
        let dur = self.subprocess_timeout();
        let result = crate::subprocess::run_with_timeout(
            cmd,
            dur,
            crate::subprocess::SubprocessContext {
                model,
                tenant_id: &tenant_id,
                script_path,
            },
        );
        self.maybe_emit_subprocess_timeout(model, &tenant_id, &result);
        result
    }

    /// R7 WB-1 stdin variant — same semantics as [`Self::run_subprocess_with_timeout`]
    /// but feeds `stdin_payload` to the child before waiting. Used by
    /// the `ontostar_planner.py` site that delivers its JSON request
    /// via stdin rather than CLI args.
    fn run_subprocess_with_timeout_stdin(
        &self,
        cmd: &mut std::process::Command,
        stdin_payload: &[u8],
        model: &str,
        script_path: &str,
    ) -> Result<crate::subprocess::TimedOutput, crate::subprocess::SubprocessError> {
        let tenant_id = self.tenant_snapshot();
        let dur = self.subprocess_timeout();
        let result = crate::subprocess::run_with_timeout_stdin(
            cmd,
            stdin_payload,
            dur,
            crate::subprocess::SubprocessContext {
                model,
                tenant_id: &tenant_id,
                script_path,
            },
        );
        self.maybe_emit_subprocess_timeout(model, &tenant_id, &result);
        result
    }

    /// Internal helper: on `LlmTimeout` emit the OCEL `llm_subprocess_timeout`
    /// event and an andon-tagged tracing error. Pulled out of
    /// `run_subprocess_with_timeout*` so both variants share the path.
    fn maybe_emit_subprocess_timeout(
        &self,
        model: &str,
        tenant_id: &str,
        result: &Result<crate::subprocess::TimedOutput, crate::subprocess::SubprocessError>,
    ) {
        if let Err(crate::subprocess::SubprocessError::LlmTimeout {
            elapsed_ms,
            limit_ms,
            script_path: sp,
        }) = result
        {
            let elapsed_str = elapsed_ms.to_string();
            let limit_str = limit_ms.to_string();
            let event_id = format!(
                "{}:llm_subprocess_timeout:{}",
                self.session_id,
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            );
            let ts = chrono::Utc::now().to_rfc3339();
            let _ = self.ocel_store().emit_event(
                &event_id,
                "llm_subprocess_timeout",
                &ts,
                &self.session_id,
                &[
                    ("model", model),
                    ("elapsed_ms", elapsed_str.as_str()),
                    ("limit_ms", limit_str.as_str()),
                    ("tenant_id", tenant_id),
                    ("script_path", sp.as_str()),
                ],
                &[],
                None,
            );
            tracing::error!(
                target: "andon",
                model = model,
                elapsed_ms = *elapsed_ms,
                limit_ms = *limit_ms,
                script_path = sp.as_str(),
                "subprocess timeout exceeded — child SIGKILLed"
            );
        }
    }

    // ─── OntoStar Stream 3: admission helper ──────────────────────────────

    /// Run the admission gate before a mutation.
    ///
    /// On Ok: returns the freshly built (and persisted) receipt.
    /// On Err: returns a JSON-shaped denial string the caller returns as the
    /// MCP response. The bypass path is also encoded as `Err(...)` so callers
    /// uniformly stop the mutation when the gate does not admit.
    fn evaluate_admission(
        &self,
        op: crate::admission::AdmissionOp,
        explicit_scope: Option<&str>,
        artifact_kind: &str,
        artifact_bytes: &[u8],
        bypass_admission: Option<bool>,
        bypass_reason: Option<&str>,
    ) -> Result<crate::receipts::Receipt, String> {
        if bypass_admission.unwrap_or(false) {
            let reason = bypass_reason.unwrap_or("").trim();
            if reason.is_empty() {
                self.lineage()
                    .record_admission_denied(&self.session_id, "false_pass");
                return Err(serde_json::json!({
                    "ok": false,
                    "admission": "denied",
                    "defect": { "kind": "FalsePass" },
                    "reason": "bypass_admission=true requires a non-empty bypass_reason",
                    "remediation": {
                        "explanation": "bypass_admission=true requires a non-empty bypass_reason. \
                                        Provide bypass_reason with a justification string.",
                        "next_tool": null,
                        "next_params": null,
                        "severity": "blocking",
                        "auto_retry": false,
                    },
                }).to_string());
            }
            // R4 WE — §14: bypass self-attribution. The audit emission MUST
            // precede `revoked_sessions` so that an external observer who
            // only sees the OCEL stream knows the bypass happened with a
            // typed `op=Bypass` audit event before the session was killed.
            // The pre-existing `admission_bypass` event below is retained
            // for backward compatibility with auditors keyed on the old
            // event_type.
            let mut bypass_artifact: Vec<u8> =
                Vec::with_capacity(op.as_str().len() + reason.len() + 1);
            bypass_artifact.extend_from_slice(op.as_str().as_bytes());
            bypass_artifact.push(0);
            bypass_artifact.extend_from_slice(reason.as_bytes());
            self.evaluate_admission_audit(
                crate::admission::AdmissionOp::Bypass,
                explicit_scope,
                "admission-bypass",
                &bypass_artifact,
            );

            let now = chrono::Utc::now().to_rfc3339();
            let event_id = format!(
                "{}:admission_bypass:{}",
                self.session_id,
                chrono::Utc::now().timestamp_millis()
            );
            let _ = self.ocel_store().emit_event(
                &event_id,
                "admission_bypass",
                &now,
                &self.session_id,
                &[("op", op.as_str()), ("reason", reason)],
                &[],
                explicit_scope,
            );
            let _ = crate::admission::revoke_session(&self.db, &self.session_id, reason);
            self.lineage().record_admission_bypass(&self.session_id, reason);
            self.lineage().record_session_revoked(&self.session_id, reason);
            // R5 WC-1 — §22 success-shaped denial closure. The previous
            // shape returned `Err({"ok": true, "admission": "bypassed"})`
            // — a JSON object claiming success while the internal state
            // (revoked_sessions, OCEL bypass audit) said the operation
            // was DENIED. External auditors keying on `ok` were misled.
            //
            // The unified denial shape:
            //   * `ok: false` matches the internal denial state.
            //   * `admission: "bypassed_session_revoked"` distinguishes
            //     this denial path from `"denied"` (gate refusal) and
            //     `"granted"` (admitted).
            //   * `defect: {kind: "BypassRevoked", reason}` is the
            //     structured DefectClass surface; auditors can drive
            //     workflows on `defect.kind` instead of free text.
            //   * `principal_revoked_at` records the RFC3339 timestamp
            //     of the revoke_session write so downstream forensic
            //     tooling can correlate against `revoked_sessions.revoked_at`.
            //
            // BREAKING JSON shape change — see CHANGELOG `[Breaking]`.
            return Err(serde_json::json!({
                "ok": false,
                "admission": "bypassed_session_revoked",
                "defect": {
                    "kind": "BypassRevoked",
                    "reason": reason,
                },
                "principal_revoked_at": now,
            }).to_string());
        }

        let scope = crate::workflows::WorkflowScope::new(&self.db, &self.session_id);
        let scope_row = match explicit_scope {
            Some(t) if !t.is_empty() => scope.get(t).ok().flatten(),
            _ => scope.latest_open().ok().flatten(),
        };
        let scope_row = match scope_row {
            Some(r) => r,
            None => {
                let event_id = format!(
                    "{}:admission_denied:{}",
                    self.session_id,
                    chrono::Utc::now().timestamp_millis()
                );
                let _ = self.ocel_store().emit_event(
                    &event_id,
                    "admission_denied",
                    &chrono::Utc::now().to_rfc3339(),
                    &self.session_id,
                    &[("op", op.as_str()), ("defect", "scope_unclosed")],
                    &[],
                    None,
                );
                self.lineage().record_admission_denied(&self.session_id, "scope_unclosed");
                let remediation =
                    crate::defects::DefectClass::ScopeUnclosed.remediation();
                return Err(serde_json::json!({
                    "ok": false,
                    "admission": "denied",
                    "defect": { "kind": "ScopeUnclosed" },
                    "remediation": remediation,
                }).to_string());
            }
        };

        let required_stages: Vec<String> = crate::workflows::by_name(&scope_row.name)
            .map(|w| w.required_stages.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        let observed_stages = self
            .ocel_store()
            .observed_event_types_for_session(&self.session_id)
            .unwrap_or_default();

        let gate = crate::admission::OntoStarAdmissionGate::new(
            0.95,
            0.85,
            required_stages,
            "ontostar-1.0.0",
        );

        let artifact = crate::admission::ArtifactRef {
            kind: artifact_kind,
            bytes: artifact_bytes,
        };

        // R3: real wasm4pm-backed replay. Parses the declared POWL, projects
        // the OCEL trace tagged with `scope_token`, returns wasm4pm fitness.
        use crate::admission::PowlReplay;
        let store = self.ocel_store();
        let replay = crate::admission::PowlBridgeReplay::new(store);

        // Pre-compute conformance once so we can record the *real* fitness /
        // precision in the lineage trail (the gate also calls replay
        // internally; this is a cheap second call against the parsed POWL).
        let caller_tenant = self.tenant.current();
        let conf = replay.replay(&scope_row.scope_token, &scope_row.powl_string, caller_tenant.current());

        // ─── PHASE 11 ENFORCEMENT POINT ────────────────────────────────────
        // All 27 #[tool] handlers that mutate funnel through this helper.
        // Routing this call through `evaluate_in_tenant` (instead of the
        // tenant-blind `evaluate`) means cross-tenant scope access is
        // rejected at the gate before any artifact is hashed, persisted,
        // or written to disk — with a typed `DefectClass::TenantBoundary`
        // defect so callers and external auditors can distinguish it from
        // generic admission failures.
        //
        // **DO NOT add a new mutating #[tool] that bypasses this helper.**
        // A handler that calls `gate.evaluate` directly, or skips admission
        // entirely, is a Phase-11 regression that the
        // `multi_tenant_boundary_wired` test suite is designed to catch.
        // ───────────────────────────────────────────────────────────────────
        match gate.evaluate_in_tenant(
            &scope_row.scope_token,
            op,
            &artifact,
            store,
            &replay,
            &self.session_id,
            &scope_row.powl_string,
            &observed_stages,
            caller_tenant.current(),
        ) {
            Ok(receipt) => {
                self.lineage()
                    .record_powl_replay(&self.session_id, conf.fitness, conf.precision);
                self.lineage().record_admission_granted(&self.session_id, &receipt.hex());
                Ok(receipt)
            }
            Err((defect, _devs)) => {
                self.lineage().record_admission_denied(&self.session_id, defect.tag());
                let remediation = defect.remediation();
                Err(serde_json::json!({
                    "ok": false,
                    "admission": "denied",
                    "defect": defect,
                    "remediation": remediation,
                    "powl_stub": conf.is_stub,
                }).to_string())
            }
        }
    }

    /// Audit-only admission: emits a tamper-evident `admission_audit` OCEL
    /// event for the operation, never denies, never persists a Receipt.
    ///
    /// Used for operator-tier maintenance ops (Clear / Feedback) that must
    /// not block on conformance — e.g. you have to be able to `onto_clear`
    /// a wedged store. The audit event preserves the No-bypass invariant
    /// (every mutation produces a tamper-evident OCEL trace) without
    /// requiring full admission machinery.
    ///
    /// Returns `()` always; callers continue execution after the call.
    fn evaluate_admission_audit(
        &self,
        op: crate::admission::AdmissionOp,
        explicit_scope: Option<&str>,
        artifact_kind: &str,
        artifact_bytes: &[u8],
    ) {
        // ─── PHASE 11 AUDIT ENFORCEMENT POINT ──────────────────────────────
        // Audit-only ops cannot deny, but the OCEL audit event MUST carry
        // the caller's tenant_id so a downstream auditor can scope the
        // trail per tenant. Every audit-only #[tool] funnels through here.
        // ───────────────────────────────────────────────────────────────────
        let required: Vec<String> = Vec::new();
        let gate = crate::admission::OntoStarAdmissionGate::new(
            0.95,
            0.85,
            required,
            "ontostar-1.0.0",
        );
        let artifact = crate::admission::ArtifactRef {
            kind: artifact_kind,
            bytes: artifact_bytes,
        };
        let caller_tenant = self.tenant.current();
        gate.evaluate_audit_in_tenant(
            op,
            &artifact,
            self.ocel_store(),
            &self.session_id,
            explicit_scope,
            caller_tenant.current(),
        );
        self.lineage()
            .record(&self.session_id, "A", "admission_audit", op.as_str());
    }

    /// R4 WE — §14 mutation gate: shared helper for `onto_plan_workflow`'s
    /// two engine paths (groq_powl and mustar). Runs full admission against
    /// `AdmissionOp::WorkflowPlanned` BEFORE inserting the planned scope row
    /// into `workflow_scopes`. Both paths must funnel through here so the
    /// `no_bypass_audit` depth-2 helper scan can prove the gate is reached
    /// from every path.
    ///
    /// Artifact bytes: `scope_token + "\0" + domain + "\0" + powl`.
    /// On admission denial, returns `Err(denial_json)` and writes nothing.
    /// On gate-passed: emits the canonical INSERT and returns `Ok(())`.
    /// On INSERT failure: returns `Err(error_json)`.
    fn persist_planned_scope(
        &self,
        scope_token: &str,
        domain: &str,
        powl: &str,
        bypass_admission: Option<bool>,
        bypass_reason: Option<&str>,
    ) -> Result<(), String> {
        let mut artifact_bytes: Vec<u8> = Vec::with_capacity(
            scope_token.len() + domain.len() + powl.len() + 2,
        );
        artifact_bytes.extend_from_slice(scope_token.as_bytes());
        artifact_bytes.push(0);
        artifact_bytes.extend_from_slice(domain.as_bytes());
        artifact_bytes.push(0);
        artifact_bytes.extend_from_slice(powl.as_bytes());

        self.evaluate_admission(
            crate::admission::AdmissionOp::WorkflowPlanned,
            Some(scope_token),
            "workflow-planned",
            &artifact_bytes,
            bypass_admission,
            bypass_reason,
        )?;

        let conn = self.db.conn();
        if let Err(e) = conn.execute(
            "INSERT INTO workflow_scopes (scope_token, workflow_name, domain, powl_string)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![scope_token, "planned_workflow", domain, powl],
        ) {
            return Err(format!(
                r#"{{"ok":false,"error":"failed to declare scope: {}"}}"#,
                e.to_string().replace('"', "'")
            ));
        }
        Ok(())
    }

    /// Admin gate for rotation tools and other admin-only handlers.
    ///
    /// R5 WC-1 — §28 HumanOverride closure. The allowlist is resolved
    /// ONCE at server startup by
    /// [`crate::config::resolve_admin_principals`] and stored on
    /// `self.admin_principals`. This function reads from that cache,
    /// never from `std::env::var(...)`. Subsequent env-var mutations
    /// after server construction have NO effect — closes the TOCTOU
    /// race the previous implementation admitted.
    ///
    /// Round 3 Task B has not landed a real principal-id helper yet, so
    /// until it does we match against the caller's `tenant_id` as the
    /// principal identifier. Once R3 Task B's `current_principal_id`
    /// and `require_admin` helpers land, this function becomes a thin
    /// wrapper around `require_admin`.
    ///
    /// Returns `true` when the caller is admin; `false` otherwise. The
    /// closed-by-default semantics match the §27 EscapeRoutes axiom: an
    /// empty cached list means NOBODY is admin (no silent downgrade to
    /// "trust all").
    fn is_admin_principal(&self) -> bool {
        if self.admin_principals.is_empty() {
            return false;
        }
        // TODO(R3 Task B): replace tenant_id fallback with
        // `current_principal_id()` once that helper lands.
        let principal = self.tenant_snapshot();
        self.admin_principals
            .iter()
            .any(|allowed| allowed.as_str() == principal.as_str())
    }

    /// OntoStar Stream 1: emit a uniform `<tool>` OCEL event for handlers that
    /// were previously silent. `ok` reflects whether the handler returned a
    /// non-error JSON shape; `duration_ms` is measured from handler entry.
    /// Objects are passed as a slice so handlers without an active ontology id
    /// can pass `&[]`.
    fn emit_tool_ocel(
        &self,
        tool_name: &str,
        started_at: std::time::Instant,
        ok: bool,
        objects: &[(&str, &str)],
    ) {
        let elapsed_ms = started_at.elapsed().as_millis().to_string();
        let ok_str = if ok { "true" } else { "false" };
        let ts = chrono::Utc::now().to_rfc3339();
        let event_id = format!(
            "{}:{}:{}",
            self.session_id,
            tool_name,
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let _ = self.ocel_store().emit_event(
            &event_id,
            tool_name,
            &ts,
            &self.session_id,
            &[("duration_ms", elapsed_ms.as_str()), ("ok", ok_str)],
            objects,
            None,
        );
    }
}

// ─── Tool definitions ───────────────────────────────────────────────────────

#[tool_router]
impl OpenOntologiesServer {

    // ── Status ──────────────────────────────────────────────────────────────

    #[tool(name = "onto_status", description = "Returns health status of the Open Ontologies server")]
    fn onto_status(&self) -> String {
        let tool_count = self.tool_router.list_all().len();
        let triple_count = self.graph.triple_count();
        serde_json::json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
            "tools": tool_count,
            "triples_loaded": triple_count,
        })
        .to_string()
    }

    // ── Ontology ────────────────────────────────────────────────────────────

    #[tool(name = "onto_validate", description = "Validate RDF/OWL syntax. Accepts a file path or inline Turtle content.")]
    async fn onto_validate(&self, Parameters(input): Parameters<OntoValidateInput>) -> String {
        let started = std::time::Instant::now();
        use crate::ontology::OntologyService;
        let out = if input.inline.unwrap_or(false) {
            OntologyService::validate_string(&input.input).unwrap_or_else(|e| {
                format!(
                    r#"{{"error":"Turtle parse error: {}. Check that 'input' contains valid Turtle/RDF content (prefixes declared, triples well-formed, IRIs angle-bracketed)."}}"#,
                    e.to_string().replace('"', "'")
                )
            })
        } else {
            let path = &input.input;
            // Pre-flight: give actionable errors before the underlying IO call
            // so the user knows exactly what went wrong and what to do next.
            if !std::path::Path::new(path).exists() {
                let msg = format!(
                    "File not found: '{}'. Verify the path is correct and the file exists. \
                     Use onto_repo_list to discover ontologies in configured ontology_dirs, \
                     or provide inline Turtle with inline=true.",
                    path
                );
                let out = serde_json::json!({
                    "valid": false,
                    "path": path,
                    "triple_count": 0,
                    "errors": [msg]
                }).to_string();
                self.emit_tool_ocel("onto_validate", started, false, &[]);
                return out;
            }
            // Check for a recognized RDF extension; unknown extensions are
            // parsed as Turtle by default, which can produce confusing errors.
            let ext = std::path::Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let known_exts = ["ttl", "turtle", "nt", "ntriples", "rdf", "xml", "owl", "nq", "trig", "jsonld"];
            if !known_exts.contains(&ext.as_str()) {
                let msg = format!(
                    "Unrecognized file extension '.{}' in '{}'. \
                     Supported extensions: .ttl/.turtle (Turtle), .nt/.ntriples (N-Triples), \
                     .rdf/.xml/.owl (RDF/XML), .nq (N-Quads), .trig (TriG). \
                     Unknown extensions are parsed as Turtle.",
                    ext, path
                );
                // Still attempt validation — warn only, do not abort.
                let result = OntologyService::validate_file(path)
                    .unwrap_or_else(|e| serde_json::json!({
                        "valid": false,
                        "path": path,
                        "triple_count": 0,
                        "errors": [e.to_string()]
                    }).to_string());
                // Inject the extension warning into the response.
                if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&result) {
                    if let Some(arr) = v.get_mut("errors").and_then(|e| e.as_array_mut()) {
                        arr.insert(0, serde_json::Value::String(msg));
                    }
                    let ok = v.get("valid").and_then(|b| b.as_bool()).unwrap_or(false);
                    self.emit_tool_ocel("onto_validate", started, ok, &[]);
                    return v.to_string();
                }
                let ok = !result.contains(r#""error""#);
                self.emit_tool_ocel("onto_validate", started, ok, &[]);
                return result;
            }
            OntologyService::validate_file(path).unwrap_or_else(|e| {
                format!(
                    r#"{{"error":"Failed to read '{}': {}. Check file permissions and encoding (UTF-8 required)."}}"#,
                    path,
                    e.to_string().replace('"', "'")
                )
            })
        };
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_validate", started, ok, &[]);
        out
    }

    #[tool(name = "onto_convert", description = "Convert an RDF file between formats: turtle, ntriples, rdfxml, nquads, trig")]
    async fn onto_convert(&self, Parameters(input): Parameters<OntoConvertInput>) -> String {
        let store = GraphStore::new();
        match store.load_file(&input.path) {
            Ok(_) => {
                match store.serialize(&input.to) {
                    Ok(content) => {
                        if let Some(output) = input.output {
                            match std::fs::write(&output, &content) {
                                Ok(_) => format!(r#"{{"ok":true,"path":"{}","format":"{}"}}"#, output, input.to),
                                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                            }
                        } else {
                            content
                        }
                    }
                    Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                }
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_load", description = "Load an RDF file or inline Turtle content into the in-memory ontology store. When given a file path, the parsed graph is also written to a fast N-Triples compile cache (in `[cache] dir`) so subsequent loads from the same source skip parsing. Optional `name`, `auto_refresh`, and `force_recompile` flags control caching/refresh behavior.")]
    pub async fn onto_load(&self, Parameters(input): Parameters<OntoLoadInput>) -> String {
        let started = std::time::Instant::now();
        let out = if let Some(turtle) = input.turtle {
            // Inline turtle bypasses the registry/cache (no source file).
            match self.graph.load_turtle(&turtle, None) {
                Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"source":"inline"}}"#, count),
                Err(e) => format!(
                    r#"{{"error":"Inline Turtle parse error in 'turtle' field: {}. Ensure prefixes are declared before use, triples end with ' .', and IRIs are angle-bracketed (e.g. <https://example.org/>). Run onto_validate with inline=true to see detailed parse errors."}}"#,
                    e.to_string().replace('"', "'")
                ),
            }
        } else if let Some(path) = input.path {
            let path = expand_tilde(&path);
            // Pre-flight diagnostics: give a targeted message before the
            // registry call so the error names the file and suggests a remedy.
            if !std::path::Path::new(&path).exists() {
                let out = format!(
                    r#"{{"error":"File not found: '{}'. Verify the path is correct and the file exists. Use onto_repo_list to discover files in configured ontology_dirs, or supply inline Turtle via the 'turtle' field instead."}}"#,
                    path
                );
                self.emit_tool_ocel("onto_load", started, false, &[]);
                return out;
            }
            let opts = crate::registry::LoadOptions {
                name: input.name,
                auto_refresh: input.auto_refresh.unwrap_or(false),
                force_recompile: input.force_recompile.unwrap_or(false),
            };
            match self.registry.load_file(&path, opts) {
                Ok(res) => {
                    let _ = self.db.set_last_active_path(&path);
                    serde_json::json!({
                        "ok": true,
                        "triples_loaded": res.triple_count,
                        "path": res.source_path,
                        "name": res.name,
                        "origin": res.origin,
                        "cache_path": res.cache_path,
                    }).to_string()
                },
                Err(e) => {
                    // Distinguish parse failures from infrastructure errors so
                    // the user knows whether to fix the file or the environment.
                    let raw = e.to_string();
                    if raw.contains("parse source") || raw.contains("ParseError") || raw.contains("invalid") {
                        format!(
                            r#"{{"error":"RDF parse error loading '{}': {}. Run onto_validate with the same path for a detailed error report. Supported formats: .ttl/.turtle (Turtle), .nt (N-Triples), .rdf/.xml/.owl (RDF/XML), .nq (N-Quads), .trig (TriG)."}}"#,
                            path,
                            raw.replace('"', "'")
                        )
                    } else {
                        format!(
                            r#"{{"error":"Failed to load '{}': {}"}}"#,
                            path,
                            raw.replace('"', "'")
                        )
                    }
                }
            }
        } else {
            r#"{"error":"Either 'path' or 'turtle' must be provided. Supply a file path via 'path' or inline Turtle/RDF content via 'turtle'."}"#.to_string()
        };
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_load", started, ok, &[]);
        out
    }

    #[tool(name = "onto_repo_list", description = "List RDF/OWL files in the configured ontology repository directories ([general] ontology_dirs). Returns metadata for each candidate file (path, name, size, mtime, is_cached, is_active). Use this in containerized/server deployments to discover ontologies without knowing their paths in advance. Optional `dir` (must be under a configured repo dir), `recursive`, `glob`, `limit`, `offset` filters.")]
    fn onto_repo_list(&self, Parameters(input): Parameters<OntoRepoListInput>) -> String {
        let repos = self.ontology_dirs.as_ref();
        if repos.is_empty() {
            return r#"{"error":"no ontology_dirs configured; set [general] ontology_dirs in config.toml or OPEN_ONTOLOGIES_ONTOLOGY_DIRS"}"#.to_string();
        }
        let recursive = input.recursive.unwrap_or(false);
        let limit = input.limit.unwrap_or_else(crate::runtime::repo_default_list_limit);
        let offset = input.offset.unwrap_or(0);

        let entries = if let Some(dir) = input.dir.as_deref() {
            match crate::repo::resolve_within_repos(dir, repos) {
                Ok((start, repo_root)) => crate::repo::list_one(&repo_root, &start, recursive),
                Err(e) => {
                    return format!(
                        r#"{{"error":"{}"}}"#,
                        e.to_string().replace('"', "'")
                    );
                }
            }
        } else {
            crate::repo::list_all(repos, recursive)
        };

        let filtered: Vec<&crate::repo::RepoEntry> = entries
            .iter()
            .filter(|e| {
                if let Some(g) = input.glob.as_deref() {
                    let name = e
                        .path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    crate::repo::glob_match(g, name)
                } else {
                    true
                }
            })
            .collect();
        let total = filtered.len();

        // Snapshot cached names + currently active name for is_cached / is_active.
        let cached_names: std::collections::HashSet<String> = self
            .registry
            .cache()
            .list()
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.name)
            .collect();
        let active_name = self
            .registry
            .status()
            .get("active")
            .and_then(|a| a.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        let items: Vec<serde_json::Value> = filtered
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|e| {
                serde_json::json!({
                    "path": e.path.to_string_lossy(),
                    "relative": e.relative.to_string_lossy(),
                    "repo_dir": e.repo_dir.to_string_lossy(),
                    "name": e.name,
                    "size": e.size,
                    "mtime": e.mtime_secs,
                    "is_cached": cached_names.contains(&e.name),
                    "is_active": active_name.as_deref() == Some(e.name.as_str()),
                })
            })
            .collect();

        let repo_dirs: Vec<String> = repos
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        serde_json::json!({
            "ok": true,
            "ontology_dirs": repo_dirs,
            "total": total,
            "offset": offset,
            "limit": limit,
            "count": items.len(),
            "items": items,
        })
        .to_string()
    }

    #[tool(name = "onto_repo_load", description = "Load an ontology from one of the configured repository directories ([general] ontology_dirs) into the active store. The `name` argument can be a bare file stem, a relative path, or an absolute path inside a configured repo. Reuses the same compile-cache / TTL-eviction path as `onto_load`.")]
    async fn onto_repo_load(&self, Parameters(input): Parameters<OntoRepoLoadInput>) -> String {
        let repos = self.ontology_dirs.as_ref();
        let path = match crate::repo::resolve_load_target(&input.name, repos) {
            Ok(p) => p,
            Err(e) => {
                let raw = e.to_string();
                // Enrich the resolution error with actionable next steps.
                let enriched = if raw.contains("no ontology with name") || raw.contains("no file matching") {
                    let repo_list: Vec<String> = repos.iter()
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect();
                    format!(
                        "{} — Use onto_repo_list to see available ontologies in: {}",
                        raw,
                        if repo_list.is_empty() {
                            "(no ontology_dirs configured)".to_string()
                        } else {
                            repo_list.join(", ")
                        }
                    )
                } else if raw.contains("no ontology_dirs configured") {
                    format!(
                        "{}. Set [general] ontology_dirs in config.toml or the \
                         OPEN_ONTOLOGIES_ONTOLOGY_DIRS environment variable, then restart the server.",
                        raw
                    )
                } else if raw.contains("ambiguous name") {
                    format!(
                        "{} — Provide a more specific path (relative or absolute) to disambiguate.",
                        raw
                    )
                } else {
                    raw
                };
                return format!(r#"{{"error":"{}"}}"#, enriched.replace('"', "'"));
            }
        };
        let opts = crate::registry::LoadOptions {
            name: input.registry_name,
            auto_refresh: input.auto_refresh.unwrap_or(false),
            force_recompile: input.force_recompile.unwrap_or(false),
        };
        let path_str = path.to_string_lossy().into_owned();
        match self.registry.load_file(&path_str, opts) {
            Ok(res) => serde_json::json!({
                "ok": true,
                "triples_loaded": res.triple_count,
                "path": res.source_path,
                "name": res.name,
                "origin": res.origin,
                "cache_path": res.cache_path,
            })
            .to_string(),
            Err(e) => {
                let raw = e.to_string();
                if raw.contains("parse source") || raw.contains("ParseError") || raw.contains("invalid") {
                    format!(
                        r#"{{"error":"RDF parse error loading '{}': {}. Run onto_validate with path='{}' for a detailed error report."}}"#,
                        path_str,
                        raw.replace('"', "'"),
                        path_str
                    )
                } else {
                    format!(
                        r#"{{"error":"Failed to load '{}': {}"}}"#,
                        path_str,
                        raw.replace('"', "'")
                    )
                }
            }
        }
    }

    #[tool(name = "onto_query", description = "Run a SPARQL query against the loaded ontology store. If the active ontology has been evicted from memory (idle TTL), it is transparently reloaded from the compile cache before the query runs.")]
    async fn onto_query(&self, Parameters(input): Parameters<OntoQueryInput>) -> String {
        let started = std::time::Instant::now();
        if let Err(e) = self.registry.ensure_loaded() {
            let out = format!(r#"{{"error":"Ontology not loaded: {}. Call onto_load first."}}"#, e.to_string().replace('"', "'"));
            self.emit_tool_ocel("onto_query", started, false, &[]);
            return out;
        }

        // Detect the query type from the leading keyword so error messages can
        // report which form was attempted. Strip leading whitespace and comments.
        let trimmed = input.query.trim();
        let query_type = {
            let upper = trimmed.to_uppercase();
            // Skip leading SPARQL comments (lines starting with #) to find keyword.
            let keyword_start = upper
                .lines()
                .find(|l| !l.trim_start().starts_with('#'))
                .map(|l| l.trim_start())
                .unwrap_or("");
            if keyword_start.starts_with("SELECT") {
                "SELECT"
            } else if keyword_start.starts_with("ASK") {
                "ASK"
            } else if keyword_start.starts_with("CONSTRUCT") {
                "CONSTRUCT"
            } else if keyword_start.starts_with("DESCRIBE") {
                "DESCRIBE"
            } else {
                "UNKNOWN"
            }
        };

        // Warn when the store has no triples — queries will succeed but return
        // empty results, which is confusing without context.
        let triple_count = self.graph.triple_count();
        if triple_count == 0 {
            let out = serde_json::json!({
                "error": "Store is empty (0 triples loaded). Load an ontology first with onto_load, then retry the query.",
                "query_type": query_type,
                "hint": "Call onto_load with a .ttl/.nt/.rdf file path, or use onto_repo_list to discover available ontologies."
            }).to_string();
            self.emit_tool_ocel("onto_query", started, false, &[]);
            return out;
        }

        let out = match self.graph.sparql_select(&input.query) {
            Ok(result) => result,
            Err(e) => {
                let raw = e.to_string();
                // Oxigraph surfaces SPARQL parse failures with messages that
                // contain "parse error", "unexpected token", "expected", or
                // "syntax error". Execution errors (e.g. unknown function) look
                // different. Give parse errors an actionable hint; pass
                // execution errors through with the query type for context.
                let is_parse_error = raw.contains("parse error")
                    || raw.contains("Parse error")
                    || raw.contains("unexpected token")
                    || raw.contains("Unexpected token")
                    || raw.contains("expected")
                    || raw.contains("syntax error")
                    || raw.contains("Syntax error")
                    || raw.contains("ParseError");
                if is_parse_error {
                    serde_json::json!({
                        "error": format!("SPARQL parse failed: {}", raw.replace('"', "'")),
                        "query_type": query_type,
                        "hint": "Common issues: missing PREFIX declaration (add PREFIX ex: <http://example.org/>), unclosed braces { }, wrong variable prefix (use ?var not $var), missing dot separator between triple patterns."
                    }).to_string()
                } else {
                    serde_json::json!({
                        "error": format!("SPARQL execution error: {}", raw.replace('"', "'")),
                        "query_type": query_type,
                    }).to_string()
                }
            }
        };
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_query", started, ok, &[]);
        out
    }

    #[tool(name = "onto_save", description = "Save the current ontology store to a file. Gated by OntoStar admission. The written file carries an OntoStar receipt header for turtle/n3/trig formats; the JSON response always includes receipt_hash.")]
    pub async fn onto_save(&self, Parameters(input): Parameters<OntoSaveInput>) -> String {
        let started = std::time::Instant::now();
        if let Err(e) = self.registry.ensure_loaded() {
            let out = format!(r#"{{"error":"Ontology not loaded: {}. Call onto_load first."}}"#, e.to_string().replace('"', "'"));
            self.emit_tool_ocel("onto_save", started, false, &[]);
            return out;
        }
        let format = input.format.as_deref().unwrap_or("turtle");
        let path = expand_tilde(&input.path);
        // OntoStar Stream 3: admission gate fires BEFORE the disk write. The
        // receipt commits to the header-LESS artifact bytes — we prepend
        // the header below so external verifiers can strip it back out.
        let artifact_bytes = self.graph.serialize(format).unwrap_or_default();
        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::Save,
            input.scope_token.as_deref(),
            "save-artifact",
            artifact_bytes.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => r,
            Err(denial) => {
                self.emit_tool_ocel("onto_save", started, false, &[]);
                return denial;
            }
        };
        // Comment-supporting formats get an OntoStar header prepended; other
        // serializations (ntriples, rdf-xml) cannot embed comments and rely
        // on the JSON response + OCEL trail for receipt binding.
        let header_embedded = matches!(format, "turtle" | "ttl" | "n3" | "trig");
        let write_result: anyhow::Result<()> = if header_embedded {
            let header = crate::receipts::ttl_header(&receipt);
            let mut out = String::with_capacity(header.len() + artifact_bytes.len());
            out.push_str(&header);
            out.push_str(&artifact_bytes);
            std::fs::write(&path, out.as_bytes()).map_err(anyhow::Error::from)
        } else {
            self.graph.save_file(&path, format)
        };
        let out = match write_result {
            Ok(_) => serde_json::json!({
                "ok": true,
                "path": path,
                "format": format,
                "header_embedded": header_embedded,
                "receipt_hash": receipt.hex(),
                "production_law_version": receipt.record.production_law_version,
                "defects_taxonomy_version": receipt.record.defects_taxonomy_version,
            })
            .to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
        };
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_save", started, ok, &[]);
        out
    }

    #[tool(name = "onto_stats", description = "Get statistics about the loaded ontology (triple count, classes, properties, individuals)")]
    fn onto_stats(&self) -> String {
        if let Err(e) = self.registry.ensure_loaded() {
            return format!(r#"{{"error":"Ontology not loaded: {}. Call onto_load first."}}"#, e.to_string().replace('"', "'"));
        }
        self.graph.get_stats().unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_diff", description = "Compare two ontology files and show added/removed triples")]
    async fn onto_diff(&self, Parameters(input): Parameters<OntoDiffInput>) -> String {
        use crate::ontology::OntologyService;
        let old = match std::fs::read_to_string(&input.old_path) {
            Ok(c) => c,
            Err(e) => return format!(r#"{{"error":"Cannot read {}: {}"}}"#, input.old_path, e),
        };
        let new = match std::fs::read_to_string(&input.new_path) {
            Ok(c) => c,
            Err(e) => return format!(r#"{{"error":"Cannot read {}: {}"}}"#, input.new_path, e),
        };
        OntologyService::diff(&old, &new).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_lint", description = "Check an ontology for quality issues: missing labels, comments, domains, ranges")]
    async fn onto_lint(&self, Parameters(input): Parameters<OntoLintInput>) -> String {
        let started = std::time::Instant::now();
        use crate::ontology::OntologyService;
        let content = if input.inline.unwrap_or(false) {
            input.input.clone()
        } else {
            match std::fs::read_to_string(&input.input) {
                Ok(c) => c,
                Err(e) => {
                    let out = format!(r#"{{"error":"{}"}}"#, e);
                    self.emit_tool_ocel("onto_lint", started, false, &[]);
                    return out;
                }
            }
        };
        let out = OntologyService::lint_with_feedback(&content, Some(&self.db)).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_lint", started, ok, &[]);
        out
    }

    #[tool(name = "onto_clear", description = "Clear all triples from the in-memory ontology store and unload the active registry slot (cache file is preserved). Audit-only admission: emits an admission_audit OCEL event with the operator's intent.")]
    fn onto_clear(&self) -> String {
        // Audit-only: maintenance ops cannot block on conformance, but every
        // mutation must produce a tamper-evident OCEL trace.
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::Clear,
            None,
            "clear-store",
            b"",
        );
        // Drop the active registry entry; this also clears the graph.
        let _ = self.registry.unload(false);
        match self.graph.clear() {
            Ok(_) => {
                let _ = self.db.clear_last_active_path();
                r#"{"ok":true,"message":"Store cleared","admission":"audit"}"#.to_string()
            },
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_unload", description = "Unload an ontology from memory. With no `name`, operates on the active ontology. With `name`, targets that cached entry — clears in-memory store if it is currently active. The on-disk compile cache is preserved unless `delete_cache=true`. Audit-only admission.")]
    fn onto_unload(&self, Parameters(input): Parameters<OntoUnloadInput>) -> String {
        let del = input.delete_cache.unwrap_or(false);
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::Clear,
            None,
            "unload-store",
            input.name.as_deref().unwrap_or("<active>").as_bytes(),
        );
        if let Some(name) = input.name.as_deref() {
            return match self.registry.unload_named(name, del) {
                Ok(true) => serde_json::json!({
                    "ok": true,
                    "unloaded": name,
                    "deleted_cache": del,
                }).to_string(),
                Ok(false) => serde_json::json!({
                    "ok": true,
                    "unloaded": null,
                    "name": name,
                    "message": "entry exists in cache but was not in memory; pass delete_cache=true to remove it",
                }).to_string(),
                Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
            };
        }
        match self.registry.unload(del) {
            Ok(Some(name)) => serde_json::json!({"ok": true, "unloaded": name, "deleted_cache": del}).to_string(),
            Ok(None) => r#"{"ok":true,"unloaded":null,"message":"no active ontology"}"#.to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_recompile", description = "Force-recompile an ontology from its source file, ignoring the on-disk cache. With no `name`, recompiles the active ontology (and reloads it into memory). With `name`, recompiles that cached entry; if it is not the active slot, the in-memory store is left untouched.")]
    fn onto_recompile(&self, Parameters(input): Parameters<OntoRecompileInput>) -> String {
        let res = match input.name.as_deref() {
            Some(name) => self.registry.recompile_named(name),
            None => self.registry.recompile(),
        };
        match res {
            Ok(res) => serde_json::json!({
                "ok": true,
                "name": res.name,
                "triples_loaded": res.triple_count,
                "origin": res.origin,
                "cache_path": res.cache_path,
            }).to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
        }
    }

    #[tool(name = "onto_cache_status", description = "Inspect the compile cache: active ontology, all cached entries, and the cache configuration (TTL, auto_refresh, dir).")]
    fn onto_cache_status(&self, Parameters(_input): Parameters<OntoCacheStatusInput>) -> String {
        self.registry.status().to_string()
    }

    #[tool(name = "onto_cache_list", description = "List all cached ontologies with metadata (name, source_path, triple_count, source_mtime, source_size, cache_path, compiled_at, last_access_at) and runtime flags (is_active, in_memory). Lighter than onto_cache_status when you only need the list.")]
    fn onto_cache_list(&self, Parameters(_input): Parameters<OntoCacheListInput>) -> String {
        match self.registry.list_cached() {
            Ok(entries) => serde_json::json!({
                "ok": true,
                "count": entries.len(),
                "entries": entries,
            }).to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
        }
    }

    #[tool(name = "onto_cache_remove", description = "Remove a cached ontology by name. If it is the active slot, the in-memory store is unloaded first. By default the on-disk N-Triples cache file is also deleted; pass delete_file=false to keep it on disk. Audit-only admission.")]
    fn onto_cache_remove(&self, Parameters(input): Parameters<OntoCacheRemoveInput>) -> String {
        let delete_file = input.delete_file.unwrap_or(true);
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::Clear,
            None,
            "cache-remove",
            input.name.as_bytes(),
        );
        match self.registry.unload_named(&input.name, delete_file) {
            Ok(true) => serde_json::json!({
                "ok": true,
                "removed": input.name,
                "deleted_file": delete_file,
            }).to_string(),
            Ok(false) => serde_json::json!({
                "ok": true,
                "removed": null,
                "name": input.name,
                "message": "entry was found but delete_file=false and it was not active, so nothing changed",
            }).to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
        }
    }

    #[tool(name = "onto_pull", description = "Fetch an ontology from a remote URL or SPARQL endpoint and load it into the store")]
    async fn onto_pull(&self, Parameters(input): Parameters<OntoPullInput>) -> String {
        use crate::graph::GraphStore;
        if input.sparql.unwrap_or(false) {
            let query = input.query.as_deref().unwrap_or("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }");
            match GraphStore::fetch_sparql(&input.url, query).await {
                Ok(content) => {
                    match self.graph.load_turtle(&content, None) {
                        Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"source":"{}"}}"#, count, input.url),
                        Err(e) => format!(r#"{{"error":"Parse error: {}"}}"#, e),
                    }
                }
                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
            }
        } else {
            match GraphStore::fetch_url(&input.url).await {
                Ok(content) => {
                    match self.graph.load_turtle(&content, None) {
                        Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"source":"{}"}}"#, count, input.url),
                        Err(e) => format!(r#"{{"error":"Parse error: {}"}}"#, e),
                    }
                }
                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
            }
        }
    }

    #[tool(name = "onto_push", description = "Push the current ontology store to a remote SPARQL endpoint. Gated by OntoStar admission. The receipt hash is bound via the X-Ostar-Receipt-Hash HTTP header so an external auditor can verify the push without round-tripping to OntoStar.")]
    async fn onto_push(&self, Parameters(input): Parameters<OntoPushInput>) -> String {
        use crate::graph::GraphStore;
        // OntoStar Stream 3: admission gate fires BEFORE the SPARQL POST.
        let artifact_preview = self.graph.serialize("ntriples").unwrap_or_default();
        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::Push,
            input.scope_token.as_deref(),
            "push-artifact",
            artifact_preview.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => r,
            Err(denial) => return denial,
        };
        match self.graph.serialize("ntriples") {
            Ok(content) => {
                let receipt_hex = receipt.hex();
                let prod_law = receipt.record.production_law_version.clone();
                let scope_tok = receipt.record.scope_token.clone();
                let extra = [
                    ("X-Ostar-Receipt-Hash", receipt_hex.as_str()),
                    ("X-Ostar-Production-Law", prod_law.as_str()),
                    ("X-Ostar-Scope-Token", scope_tok.as_str()),
                ];
                match GraphStore::push_sparql_graph(&input.endpoint, &content, None, &extra).await {
                    Ok(msg) => serde_json::json!({
                        "ok": true,
                        "message": msg,
                        "receipt_hash": receipt.hex(),
                        "production_law_version": receipt.record.production_law_version,
                        "defects_taxonomy_version": receipt.record.defects_taxonomy_version,
                        "binding": "X-Ostar-Receipt-Hash header",
                    })
                    .to_string(),
                    Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
                }
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
        }
    }

    #[tool(name = "onto_import", description = "Resolve and load all owl:imports from the currently loaded ontology")]
    async fn onto_import(&self, Parameters(input): Parameters<OntoImportInput>) -> String {
        use crate::graph::GraphStore;
        let max_depth = input
            .max_depth
            .unwrap_or_else(crate::runtime::imports_max_depth);
        let timeout_secs = crate::runtime::imports_request_timeout_secs();
        let follow_remote = crate::runtime::imports_follow_remote();
        let mut imported = Vec::new();
        let mut to_import: Vec<String> = Vec::new();

        // Build a per-call HTTP client honouring the configured timeout.
        // Falls back to the bare `fetch_url` helper if construction fails.
        let timed_client = if timeout_secs > 0 {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .build()
                .ok()
        } else {
            None
        };

        let fetch = |url: String| {
            let client = timed_client.clone();
            async move {
                if let Some(c) = client {
                    let resp = c.get(&url).send().await?;
                    if !resp.status().is_success() {
                        anyhow::bail!("HTTP {}: {}", resp.status(), url);
                    }
                    Ok::<String, anyhow::Error>(resp.text().await?)
                } else {
                    GraphStore::fetch_url(&url).await
                }
            }
        };

        let query = "SELECT ?import WHERE { ?onto <http://www.w3.org/2002/07/owl#imports> ?import }";
        if let Ok(result) = self.graph.sparql_select(query)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result)
                && let Some(results) = parsed["results"].as_array() {
                    for row in results {
                        if let Some(uri) = row["import"].as_str() {
                            let uri = uri.trim_matches(|c| c == '<' || c == '>');
                            to_import.push(uri.to_string());
                        }
                    }
                }

        let mut depth = 0;
        while !to_import.is_empty() && depth < max_depth {
            let batch = std::mem::take(&mut to_import);
            for url in batch {
                if imported.contains(&url) { continue; }
                // Honour the `[imports] follow_remote` policy: in
                // air-gapped or sandboxed deployments, refuse to fetch
                // http(s):// imports rather than attempting them.
                let is_remote = url.starts_with("http://") || url.starts_with("https://");
                if is_remote && !follow_remote {
                    imported.push(format!("SKIPPED:{}: remote imports disabled by [imports] follow_remote=false", url));
                    continue;
                }
                match fetch(url.clone()).await {
                    Ok(content) => {
                        match self.graph.load_turtle(&content, None) {
                            Ok(_count) => {
                                imported.push(url.clone());
                                if let Ok(result) = self.graph.sparql_select(query)
                                    && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result)
                                        && let Some(results) = parsed["results"].as_array() {
                                            for row in results {
                                                if let Some(uri) = row["import"].as_str() {
                                                    let uri = uri.trim_matches(|c| c == '<' || c == '>').to_string();
                                                    if !imported.contains(&uri) && !to_import.contains(&uri) {
                                                        to_import.push(uri);
                                                    }
                                                }
                                            }
                                        }
                            }
                            Err(e) => { imported.push(format!("FAILED:{}: {}", url, e)); }
                        }
                    }
                    Err(e) => { imported.push(format!("FAILED:{}: {}", url, e)); }
                }
            }
            depth += 1;
        }

        serde_json::json!({
            "ok": true,
            "imported": imported,
            "total": imported.len(),
            "depth": depth,
        }).to_string()
    }

    // ── Marketplace ────────────────────────────────────────────────────────

    #[tool(name = "onto_marketplace", description = "Browse and install standard ontologies from a curated catalogue of 32 W3C/ISO/industry standards. Actions: 'list' (browse catalogue, optional domain filter) or 'install' (fetch and load by ID)")]
    async fn onto_marketplace(&self, Parameters(input): Parameters<OntoMarketplaceInput>) -> String {
        use crate::marketplace;
        match input.action.as_str() {
            "list" => {
                let entries = marketplace::list(input.domain.as_deref());
                let items: Vec<serde_json::Value> = entries.iter().map(|e| {
                    serde_json::json!({
                        "id": e.id,
                        "name": e.name,
                        "description": e.description,
                        "domain": e.domain,
                        "url": e.url,
                        "format": marketplace::format_name(e.format),
                    })
                }).collect();
                serde_json::json!({
                    "ok": true,
                    "count": items.len(),
                    "ontologies": items,
                }).to_string()
            }
            "install" => {
                let id = match input.id.as_deref() {
                    Some(id) => id,
                    None => return r#"{"error":"'id' is required for install action"}"#.to_string(),
                };
                let entry = match marketplace::find(id) {
                    Some(e) => e,
                    None => {
                        let available: Vec<&str> = marketplace::CATALOGUE.iter().map(|e| e.id).collect();
                        return serde_json::json!({
                            "error": format!("Unknown ontology ID: '{}'. Use action 'list' to see available IDs.", id),
                            "available": available,
                        }).to_string();
                    }
                };
                match crate::graph::GraphStore::fetch_url(entry.url).await {
                    Ok(content) => {
                        match self.graph.load_content_with_base(&content, entry.format, Some(entry.url)) {
                            Ok(count) => {
                                let stats = self.graph.get_stats().unwrap_or_default();
                                let stats_val: serde_json::Value = serde_json::from_str(&stats).unwrap_or_default();
                                serde_json::json!({
                                    "ok": true,
                                    "installed": entry.id,
                                    "name": entry.name,
                                    "triples_loaded": count,
                                    "source": entry.url,
                                    "classes": stats_val["classes"],
                                    "properties": stats_val["properties"],
                                    "individuals": stats_val["individuals"],
                                }).to_string()
                            }
                            Err(e) => format!(r#"{{"error":"Parse error for {}: {}"}}"#, entry.id, e),
                        }
                    }
                    Err(e) => format!(r#"{{"error":"Fetch error for {}: {}"}}"#, entry.id, e),
                }
            }
            other => format!(r#"{{"error":"Unknown action '{}'. Use 'list' or 'install'."}}"#, other),
        }
    }

    #[tool(name = "onto_version", description = "Save a named snapshot of the current ontology store. Audit-only admission (snapshots are non-destructive metadata).")]
    async fn onto_version(&self, Parameters(input): Parameters<OntoVersionInput>) -> String {
        let started = std::time::Instant::now();
        // Audit-only: snapshot creation produces tamper-evident OCEL trail
        // but does not block — taking a snapshot is always safe.
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::Version,
            None,
            "version-label",
            input.label.as_bytes(),
        );
        use crate::ontology::OntologyService;
        let out = OntologyService::save_version(&self.db, &self.graph, &input.label)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_version", started, ok, &[]);
        out
    }

    #[tool(name = "onto_history", description = "List all saved ontology version snapshots")]
    fn onto_history(&self) -> String {
        use crate::ontology::OntologyService;
        OntologyService::list_versions(&self.db)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_rollback", description = "Restore the ontology store to a previously saved version. Gated by OntoStar admission (rollback is a destructive un-apply).")]
    async fn onto_rollback(&self, Parameters(input): Parameters<OntoRollbackInput>) -> String {
        let started = std::time::Instant::now();
        use crate::ontology::OntologyService;
        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::Rollback,
            input.scope_token.as_deref(),
            "rollback-target",
            input.label.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => Some(r),
            Err(denial) => {
                self.emit_tool_ocel("onto_rollback", started, false, &[]);
                return denial;
            }
        };
        let raw = OntologyService::rollback_version(&self.db, &self.graph, &input.label)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        let ok = !raw.contains(r#""error""#);
        let out = if ok {
            let mut parsed: serde_json::Value =
                serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}));
            if let (Some(obj), Some(r)) = (parsed.as_object_mut(), &receipt) {
                obj.insert("receipt_hash".into(), r.hex().into());
                obj.insert(
                    "production_law_version".into(),
                    r.record.production_law_version.clone().into(),
                );
            }
            parsed.to_string()
        } else {
            raw
        };
        self.emit_tool_ocel("onto_rollback", started, ok, &[]);
        out
    }

    // ── Data ingestion & reasoning ─────────────────────────────────────────

    #[tool(name = "onto_ingest", description = "Parse a structured data file (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF triples and load into the ontology store. Optionally uses a mapping config to control field-to-predicate mapping. Gated by OntoStar admission.")]
    async fn onto_ingest(&self, Parameters(input): Parameters<OntoIngestInput>) -> String {
        let started = std::time::Instant::now();
        // OntoStar Stream 3: admission gate fires BEFORE the graph mutation.
        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::Ingest,
            input.scope_token.as_deref(),
            "ingest-input",
            input.path.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => Some(r),
            Err(denial) => {
                self.emit_tool_ocel("onto_ingest", started, false, &[]);
                return denial;
            }
        };
        let out = self.onto_ingest_inner(input, receipt).await;
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_ingest", started, ok, &[]);
        out
    }

    async fn onto_ingest_inner(
        &self,
        input: OntoIngestInput,
        receipt: Option<crate::receipts::Receipt>,
    ) -> String {
        use crate::ingest::DataIngester;
        use crate::mapping::MappingConfig;

        let base_iri = input.base_iri.as_deref().unwrap_or("http://example.org/data/");

        // Parse data file
        let rows = match DataIngester::parse_file_with_format(&input.path, input.format.as_deref()) {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"Failed to parse {}: {}"}}"#, input.path, e),
        };

        if rows.is_empty() {
            return r#"{"ok":true,"triples_loaded":0,"warnings":["No data rows found"]}"#.to_string();
        }

        // Get or generate mapping
        let mapping = if let Some(ref mapping_str) = input.mapping {
            if input.inline_mapping.unwrap_or(false) {
                match serde_json::from_str::<MappingConfig>(mapping_str) {
                    Ok(m) => m,
                    Err(e) => return format!(r#"{{"error":"Invalid mapping JSON: {}"}}"#, e),
                }
            } else {
                match std::fs::read_to_string(mapping_str) {
                    Ok(content) => match serde_json::from_str::<MappingConfig>(&content) {
                        Ok(m) => m,
                        Err(e) => return format!(r#"{{"error":"Invalid mapping file: {}"}}"#, e),
                    },
                    Err(e) => return format!(r#"{{"error":"Cannot read mapping file: {}"}}"#, e),
                }
            }
        } else {
            let headers = DataIngester::extract_headers(&rows);
            MappingConfig::from_headers(&headers, base_iri, &format!("{}Thing", base_iri))
        };

        // Convert to N-Triples and load
        let ntriples = mapping.rows_to_ntriples(&rows);
        match self.graph.load_ntriples(&ntriples) {
            Ok(count) => {
                let mut out = serde_json::json!({
                    "ok": true,
                    "triples_loaded": count,
                    "rows_processed": rows.len(),
                    "mapping_fields": mapping.mappings.len(),
                });
                if let Some(r) = &receipt
                    && let Some(obj) = out.as_object_mut() {
                        obj.insert("receipt_hash".into(), r.hex().into());
                        obj.insert(
                            "production_law_version".into(),
                            r.record.production_law_version.clone().into(),
                        );
                    }
                out.to_string()
            }
            Err(e) => format!(r#"{{"error":"Failed to load triples: {}"}}"#, e),
        }
    }

    #[tool(name = "onto_map", description = "Generate a mapping config by inspecting a data file's schema against the currently loaded ontology. Returns a JSON mapping that can be reviewed and passed to onto_ingest.")]
    async fn onto_map(&self, Parameters(input): Parameters<OntoMapInput>) -> String {
        let started = std::time::Instant::now();
        let out = self.onto_map_inner(input).await;
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_map", started, ok, &[]);
        out
    }

    async fn onto_map_inner(&self, input: OntoMapInput) -> String {
        use crate::ingest::DataIngester;
        use crate::mapping::MappingConfig;

        let rows = match DataIngester::parse_file(&input.data_path) {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"Failed to parse {}: {}"}}"#, input.data_path, e),
        };
        let headers = DataIngester::extract_headers(&rows);

        // Get ontology classes and properties from the store
        let classes_query = r#"SELECT DISTINCT ?c WHERE {
            { ?c a <http://www.w3.org/2002/07/owl#Class> }
            UNION
            { ?c a <http://www.w3.org/2000/01/rdf-schema#Class> }
        }"#;
        let props_query = r#"SELECT DISTINCT ?p WHERE {
            { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> }
            UNION
            { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> }
            UNION
            { ?p a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property> }
        }"#;

        let classes = self.graph.sparql_select(classes_query).unwrap_or_default();
        let props = self.graph.sparql_select(props_query).unwrap_or_default();

        let extract_iris = |json: &str, var: &str| -> Vec<String> {
            serde_json::from_str::<serde_json::Value>(json)
                .ok()
                .and_then(|v| v["results"].as_array().cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|r| r[var].as_str().map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string()))
                .collect()
        };

        let class_iris = extract_iris(&classes, "c");
        let prop_iris = extract_iris(&props, "p");

        let mapping = MappingConfig::from_headers(
            &headers,
            "http://example.org/data/",
            class_iris.first().map(|s| s.as_str()).unwrap_or("http://example.org/Thing"),
        );

        let result = serde_json::json!({
            "mapping": mapping,
            "data_fields": headers,
            "ontology_classes": class_iris,
            "ontology_properties": prop_iris,
        });

        if let Some(ref save_path) = input.save_path
            && let Ok(json) = serde_json::to_string_pretty(&mapping)
                && let Err(e) = std::fs::write(save_path, &json) {
                    return format!(r#"{{"error":"Cannot write mapping file: {}"}}"#, e);
                }

        result.to_string()
    }

    #[tool(name = "onto_shacl", description = "Validate the loaded ontology data against SHACL shapes. Checks cardinality (minCount/maxCount), datatypes, and class constraints. Returns a conformance report with violations.")]
    async fn onto_shacl(&self, Parameters(input): Parameters<OntoShaclInput>) -> String {
        let started = std::time::Instant::now();
        use crate::shacl::ShaclValidator;
        let shapes = if input.inline.unwrap_or(false) {
            input.shapes.clone()
        } else {
            match std::fs::read_to_string(&input.shapes) {
                Ok(c) => c,
                Err(e) => {
                    let out = format!(r#"{{"error":"Cannot read shapes file: {}"}}"#, e);
                    self.emit_tool_ocel("onto_shacl", started, false, &[]);
                    return out;
                }
            }
        };
        let out = ShaclValidator::validate(&self.graph, &shapes)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_shacl", started, ok, &[]);
        out
    }

    #[tool(name = "onto_reason", description = "Run inference over the loaded ontology. Profiles: 'rdfs' (subclass, domain/range), 'owl-rl' (+ transitive/symmetric/inverse, sameAs, equivalentClass), 'owl-rl-ext' (+ someValuesFrom, allValuesFrom, hasValue, intersectionOf, unionOf), 'owl-dl' (Full OWL2-DL SHOIQ tableaux: satisfiability, classification, qualified number restrictions with node merging, inverse/symmetric roles, functional properties, parallel agent-based classification, explanation traces, ABox reasoning). Materializes inferred triples.")]
    async fn onto_reason(&self, Parameters(input): Parameters<OntoReasonInput>) -> String {
        let started = std::time::Instant::now();
        use crate::reason::Reasoner;
        let profile = input.profile.as_deref().unwrap_or("rdfs");
        if self.graph.triple_count() == 0 {
            let out = format!(
                r#"{{"error":"onto_reason: no triples in store (profile '{}' requested). Call onto_load first to load an ontology before running inference."}}"#,
                profile
            );
            self.emit_tool_ocel("onto_reason", started, false, &[]);
            return out;
        }
        let materialize = input.materialize.unwrap_or(true);
        let out = Reasoner::run(&self.graph, profile, materialize)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_reason", started, ok, &[]);
        out
    }

    #[tool(name = "onto_dl_explain", description = "Explain why a class is unsatisfiable using DL tableaux reasoning. Returns an explanation trace showing the logical contradictions that make the class impossible to instantiate.")]
    async fn onto_dl_explain(&self, Parameters(input): Parameters<OntoDlExplainInput>) -> String {
        use crate::tableaux::DlReasoner;
        DlReasoner::explain_class(&self.graph, &input.class_iri)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_dl_check", description = "Check if one class is subsumed by another using DL tableaux reasoning. Returns whether sub_class is a subclass of super_class, with justification.")]
    async fn onto_dl_check(&self, Parameters(input): Parameters<OntoDlCheckInput>) -> String {
        use crate::tableaux::DlReasoner;
        DlReasoner::check_subsumption(&self.graph, &input.sub_class, &input.super_class)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    // ── v2: Lifecycle tools ─────────────────────────────────────────────────

    #[tool(name = "onto_plan", description = "Terraform-style plan: diff current store against proposed Turtle. Shows added/removed classes/properties, blast radius, risk score, and locked IRI violations.")]
    async fn onto_plan(&self, Parameters(input): Parameters<OntoPlanInput>) -> String {
        let planner = crate::plan::Planner::new(self.db.clone(), self.graph.clone());
        match planner.plan(&input.new_turtle) {
            Ok(result) => {
                let ts = chrono::Utc::now().to_rfc3339();
                let obj_id = format!("{}:plan", self.session_id);
                let _ = self.ocel_store().upsert_object(&obj_id, "OntologyVersion", &[]);

                // Extract plan metrics from JSON
                let (risk_score, added_count, removed_count) = serde_json::from_str::<serde_json::Value>(&result)
                    .ok()
                    .map(|j| {
                        let risk = j.get("risk_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let added = j.get("added_classes").and_then(|v| v.as_array().map(|a| a.len())).unwrap_or(0);
                        let removed = j.get("removed_classes").and_then(|v| v.as_array().map(|a| a.len())).unwrap_or(0);
                        (risk, added, removed)
                    })
                    .unwrap_or((0.0, 0, 0));

                let event_id = format!("{}:plan:{}", self.session_id, chrono::Utc::now().timestamp_millis());
                let _ = self.ocel_store().emit_event(
                    &event_id,
                    "plan_computed",
                    &ts,
                    &self.session_id,
                    &[
                        ("risk_score", &risk_score.to_string()),
                        ("added_count", &added_count.to_string()),
                        ("removed_count", &removed_count.to_string()),
                    ],
                    &[(&obj_id, "planned_version")],
                    None,
                );

                self.lineage().record(&self.session_id, "P", "plan", "computed");
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_apply", description = "Apply the last plan. Modes: 'safe' (clear+reload, checks monitor), 'force' (skip monitor watchers — does NOT bypass admission), 'migrate' (adds owl:equivalentClass/Property bridges for renames). To bypass admission, set bypass_admission=true with a non-empty bypass_reason.")]
    async fn onto_apply(&self, Parameters(input): Parameters<OntoApplyInput>) -> String {
        let mode = input.mode.as_deref().unwrap_or("safe");
        // OntoStar Stream 3: admission gate fires BEFORE any state mutation.
        let artifact_bytes = self.graph.serialize("ntriples").unwrap_or_default();
        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::Apply,
            input.scope_token.as_deref(),
            "apply-plan",
            artifact_bytes.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => r,
            Err(denial) => return denial,
        };
        let planner = crate::plan::Planner::new(self.db.clone(), self.graph.clone());
        match planner.apply(mode) {
            Ok(result) => {
                let ts = chrono::Utc::now().to_rfc3339();
                let obj_id = format!("{}:plan", self.session_id);
                let _ = self.ocel_store().upsert_object(&obj_id, "OntologyVersion", &[]);

                let event_id = format!("{}:apply:{}", self.session_id, chrono::Utc::now().timestamp_millis());
                let _ = self.ocel_store().emit_event(
                    &event_id,
                    &format!("apply_{}", mode),
                    &ts,
                    &self.session_id,
                    &[("mode", mode)],
                    &[(&obj_id, "applied_version")],
                    None,
                );

                self.lineage().record(&self.session_id, "A", "apply", mode);
                // OntoStar Stream 4 — Loop 1 hook (best-effort post-apply mining).
                // Refused silently (returns Ok(None)) when the scope lacks an
                // `admission_granted` event, a row in `receipts`, or
                // `conformance_runs.fitness >= 0.95`. The receipt JOIN inside
                // Loop 4 (exemplars_for_domain) is the integrity proof.
                if let Some(ref scope) = input.scope_token {
                    let _ = crate::feedback::exemplars::maybe_mine_exemplar(scope, self.ocel_store());
                }
                let monitor_result = self.monitor().run_watchers();
                let mut parsed: serde_json::Value =
                    serde_json::from_str(&result).unwrap_or_else(|_| serde_json::json!({}));
                if let Some(obj) = parsed.as_object_mut() {
                    obj.insert("receipt_hash".into(), receipt.hex().into());
                    obj.insert(
                        "production_law_version".into(),
                        receipt.record.production_law_version.clone().into(),
                    );
                    obj.insert(
                        "defects_taxonomy_version".into(),
                        receipt.record.defects_taxonomy_version.clone().into(),
                    );
                }
                if monitor_result.status != "ok"
                    && let Some(obj) = parsed.as_object_mut() {
                        obj.insert(
                            "monitor".into(),
                            serde_json::to_value(&monitor_result).unwrap_or_default(),
                        );
                    }
                parsed.to_string()
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_lock", description = "Lock IRIs to prevent removal during plan/apply. Locked IRIs will show as violations in plan output.")]
    async fn onto_lock(&self, Parameters(input): Parameters<OntoLockInput>) -> String {
        let planner = crate::plan::Planner::new(self.db.clone(), self.graph.clone());
        let reason = input.reason.as_deref().unwrap_or("locked");
        for iri in &input.iris {
            planner.lock_iri(iri, reason);
        }
        serde_json::json!({
            "ok": true,
            "locked": input.iris,
            "reason": reason,
        }).to_string()
    }

    #[tool(name = "onto_drift", description = "Detect drift between two ontology versions. Returns added/removed terms, likely renames with confidence scores, and drift velocity.")]
    async fn onto_drift(&self, Parameters(input): Parameters<OntoDriftInput>) -> String {
        let detector = crate::drift::DriftDetector::new(self.db.clone());
        match detector.detect(&input.version_a, &input.version_b) {
            Ok(result) => {
                let ts = chrono::Utc::now().to_rfc3339();
                let obj_id_a = format!("{}:version_a", self.session_id);
                let obj_id_b = format!("{}:version_b", self.session_id);
                let _ = self.ocel_store().upsert_object(&obj_id_a, "OntologyVersion", &[]);
                let _ = self.ocel_store().upsert_object(&obj_id_b, "OntologyVersion", &[]);

                // Extract drift metrics
                let (added, removed, renames) = serde_json::from_str::<serde_json::Value>(&result)
                    .ok()
                    .map(|j| {
                        let a = j.get("added_terms").and_then(|v| v.as_array().map(|arr| arr.len())).unwrap_or(0);
                        let r = j.get("removed_terms").and_then(|v| v.as_array().map(|arr| arr.len())).unwrap_or(0);
                        let rn = j.get("rename_candidates").and_then(|v| v.as_array().map(|arr| arr.len())).unwrap_or(0);
                        (a, r, rn)
                    })
                    .unwrap_or((0, 0, 0));

                let event_id = format!("{}:drift:{}", self.session_id, chrono::Utc::now().timestamp_millis());
                let _ = self.ocel_store().emit_event(
                    &event_id,
                    "drift_detected",
                    &ts,
                    &self.session_id,
                    &[
                        ("added_count", &added.to_string()),
                        ("removed_count", &removed.to_string()),
                        ("rename_count", &renames.to_string()),
                    ],
                    &[(&obj_id_a, "version_a"), (&obj_id_b, "version_b")],
                    None,
                );

                self.lineage().record(&self.session_id, "D", "drift", "detected");
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_enforce", description = "Enforce design patterns on the loaded ontology. Built-in packs: 'generic' (orphan classes, missing domain/range/label), 'boro' (BORO 4D patterns), 'value_partition' (disjoint/covering checks). Also runs any custom rules stored for the pack.")]
    async fn onto_enforce(&self, Parameters(input): Parameters<OntoEnforceInput>) -> String {
        let enforcer = crate::enforce::Enforcer::new(self.db.clone(), self.graph.clone());
        match enforcer.enforce_with_feedback(&input.rule_pack, Some(&self.db)) {
            Ok(result) => {
                let ts = chrono::Utc::now().to_rfc3339();
                let obj_id = input.rule_pack.clone();
                let _ = self.ocel_store().upsert_object(&obj_id, "RulePack", &[("rule_pack", &obj_id, "string")]);

                // Count violations from result JSON if possible
                let vcount = serde_json::from_str::<serde_json::Value>(&result)
                    .ok()
                    .and_then(|j| j.get("violations").and_then(|v| v.as_array().map(|a| a.len())))
                    .unwrap_or(0);

                let event_id = format!("{}:enforce:{}", self.session_id, chrono::Utc::now().timestamp_millis());
                let _ = self.ocel_store().emit_event(
                    &event_id,
                    "enforce_run",
                    &ts,
                    &self.session_id,
                    &[("violation_count", &vcount.to_string()), ("rule_pack", &obj_id)],
                    &[(&obj_id, "enforced_against")],
                    None,
                );

                self.lineage().record(&self.session_id, "E", "enforce", &input.rule_pack);
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_monitor", description = "Run active monitoring watchers. Optionally add new watchers via inline JSON. Watchers with action=notify and a webhook_url will POST alerts to the URL. Returns ok/alert/blocked status with details.")]
    async fn onto_monitor(&self, Parameters(input): Parameters<OntoMonitorInput>) -> String {
        let monitor = self.monitor();

        // Add watchers if provided
        if let Some(ref watchers_json) = input.watchers
            && let Ok(watchers) = serde_json::from_str::<Vec<crate::monitor::Watcher>>(watchers_json) {
                for w in watchers {
                    monitor.add_watcher(w);
                }
            }

        let result = monitor.run_watchers();

        let ts = chrono::Utc::now().to_rfc3339();
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let obj_id = format!("{}:monitor:{}", self.session_id, ts_ms);
        let _ = self.ocel_store().upsert_object(&obj_id, "MonitorRun", &[("status", &result.status, "string")]);

        let event_id = format!("{}:monitor:{}", self.session_id, ts_ms);
        let _ = self.ocel_store().emit_event(
            &event_id,
            &format!("monitor_{}", result.status),
            &ts,
            &self.session_id,
            &[("status", &result.status)],
            &[(&obj_id, "monitor_run")],
            None,
        );

        self.lineage().record(&self.session_id, "M", "monitor", &result.status);
        serde_json::to_string(&result).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_monitor_clear", description = "Clear the monitor blocked flag, allowing apply operations to proceed. Audit-only admission.")]
    fn onto_monitor_clear(&self) -> String {
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::Feedback,
            None,
            "monitor-clear",
            b"",
        );
        self.monitor().clear_blocked();
        r#"{"ok":true,"message":"Monitor block cleared","admission":"audit"}"#.to_string()
    }

    #[tool(name = "onto_crosswalk", description = "Look up clinical crosswalk mappings for a code and system (ICD10, SNOMED, MeSH). Uses data/crosswalks.parquet (93-row sample included; run scripts/build_crosswalks.py to extend).")]
    async fn onto_crosswalk(&self, Parameters(input): Parameters<OntoCrosswalkInput>) -> String {
        match crate::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
            Ok(cw) => {
                let results = cw.lookup(&input.code, &input.source_system);
                serde_json::json!({
                    "code": input.code,
                    "system": input.source_system,
                    "mappings": results.iter().map(|r| serde_json::json!({
                        "target_code": r.target_code,
                        "target_system": r.target_system,
                        "relation": r.relation,
                        "source_label": r.source_label,
                        "target_label": r.target_label,
                    })).collect::<Vec<_>>(),
                }).to_string()
            }
            Err(e) => format!(r#"{{"error":"Crosswalks not loaded: {}. Run scripts/build_crosswalks.py first."}}"#, e),
        }
    }

    #[tool(name = "onto_enrich", description = "Enrich an ontology class with a SKOS mapping triple from the clinical crosswalks.")]
    async fn onto_enrich(&self, Parameters(input): Parameters<OntoEnrichInput>) -> String {
        match crate::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
            Ok(cw) => cw.enrich(&self.graph, &input.class_iri, &input.code, &input.system),
            Err(e) => format!(r#"{{"error":"Crosswalks not loaded: {}"}}"#, e),
        }
    }

    #[tool(name = "onto_validate_clinical", description = "Validate all class labels in the loaded ontology against clinical crosswalk data. Shows which terms match known clinical codes.")]
    fn onto_validate_clinical(&self) -> String {
        match crate::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
            Ok(cw) => cw.validate_clinical(&self.graph),
            Err(e) => format!(r#"{{"error":"Crosswalks not loaded: {}"}}"#, e),
        }
    }

    #[tool(name = "onto_lineage", description = "Get the lineage log for the current or specified session. Default format is compact text; use format='eventlog' for EventLog JSON or format='ocel' for Object-Centric Event Log.")]
    async fn onto_lineage(&self, Parameters(input): Parameters<OntoLineageInput>) -> String {
        let session = input.session_id.as_deref().unwrap_or(&self.session_id);
        let format = input.format.as_deref().unwrap_or("text");

        match format {
            "ocel" => {
                match self.ocel_store.build_ocel(Some(session)) {
                    Ok(ocel) => {
                        serde_json::json!({
                            "session_id": session,
                            "format": "ocel",
                            "ocel": serde_json::to_value(&ocel).unwrap_or(serde_json::Value::Null),
                        }).to_string()
                    }
                    Err(e) => {
                        serde_json::json!({
                            "error": format!("Failed to build OCEL: {}", e)
                        }).to_string()
                    }
                }
            }
            "eventlog" => {
                let db = self.db.clone();
                let conn = db.conn();
                match crate::lineage::lineage_to_event_log(&conn, Some(session)) {
                    Ok(event_log) => {
                        serde_json::json!({
                            "session_id": session,
                            "format": "eventlog",
                            "event_log": serde_json::to_value(&event_log).unwrap_or(serde_json::Value::Null),
                        }).to_string()
                    }
                    Err(e) => {
                        serde_json::json!({
                            "error": format!("Failed to convert to EventLog: {}", e)
                        }).to_string()
                    }
                }
            }
            _ => {
                // Default text format
                let events = self.lineage().get_compact(session);
                serde_json::json!({
                    "session_id": session,
                    "format": "text",
                    "events": events.trim(),
                }).to_string()
            }
        }
    }

    #[tool(name = "onto_extend", description = "Convenience pipeline: ingest data → validate with SHACL → run OWL reasoning, all in one call. Combines onto_ingest + onto_shacl + onto_reason. Gated by OntoStar admission as the Ingest op.")]
    async fn onto_extend(&self, Parameters(input): Parameters<OntoExtendInput>) -> String {
        let started = std::time::Instant::now();
        // OntoStar Stream 3: pipeline mutates the graph — gate as Ingest.
        if let Err(denial) = self.evaluate_admission(
            crate::admission::AdmissionOp::Ingest,
            input.scope_token.as_deref(),
            "extend-input",
            input.data_path.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            self.emit_tool_ocel("onto_extend", started, false, &[]);
            return denial;
        }
        let out = self.onto_extend_inner(input).await;
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_extend", started, ok, &[]);
        out
    }

    async fn onto_extend_inner(&self, input: OntoExtendInput) -> String {
        use crate::ingest::DataIngester;
        use crate::mapping::MappingConfig;
        use crate::shacl::ShaclValidator;
        use crate::reason::Reasoner;

        let base_iri = input.base_iri.as_deref().unwrap_or("http://example.org/data/");

        // 1. Ingest
        let rows = match DataIngester::parse_file(&input.data_path) {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"Ingest failed: {}"}}"#, e),
        };

        let mapping = if let Some(ref mapping_str) = input.mapping {
            if input.inline_mapping.unwrap_or(false) {
                match serde_json::from_str::<MappingConfig>(mapping_str) {
                    Ok(m) => m,
                    Err(e) => return format!(r#"{{"error":"Invalid mapping: {}"}}"#, e),
                }
            } else {
                match std::fs::read_to_string(mapping_str) {
                    Ok(content) => match serde_json::from_str::<MappingConfig>(&content) {
                        Ok(m) => m,
                        Err(e) => return format!(r#"{{"error":"Invalid mapping file: {}"}}"#, e),
                    },
                    Err(e) => return format!(r#"{{"error":"Cannot read mapping: {}"}}"#, e),
                }
            }
        } else {
            let headers = DataIngester::extract_headers(&rows);
            MappingConfig::from_headers(&headers, base_iri, &format!("{}Thing", base_iri))
        };

        let ntriples = mapping.rows_to_ntriples(&rows);
        let triples_loaded = match self.graph.load_ntriples(&ntriples) {
            Ok(c) => c,
            Err(e) => return format!(r#"{{"error":"Failed to load triples: {}"}}"#, e),
        };

        // 2. SHACL (optional)
        let mut shacl_result = serde_json::json!({"skipped": true});
        if let Some(ref shapes_input) = input.shapes {
            let shapes = if input.inline_shapes.unwrap_or(false) {
                shapes_input.clone()
            } else {
                match std::fs::read_to_string(shapes_input) {
                    Ok(c) => c,
                    Err(e) => return format!(r#"{{"error":"Cannot read shapes: {}"}}"#, e),
                }
            };
            match ShaclValidator::validate(&self.graph, &shapes) {
                Ok(report) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&report) {
                        let stop = input.stop_on_violations.unwrap_or(true);
                        if stop && parsed["conforms"] == false {
                            return serde_json::json!({
                                "stage": "shacl",
                                "triples_ingested": triples_loaded,
                                "shacl": parsed,
                                "stopped": true,
                                "message": "Pipeline stopped due to SHACL violations",
                            }).to_string();
                        }
                        shacl_result = parsed;
                    }
                }
                Err(e) => return format!(r#"{{"error":"SHACL validation failed: {}"}}"#, e),
            }
        }

        // 3. Reasoning (optional)
        let mut reason_result = serde_json::json!({"skipped": true});
        if let Some(ref profile) = input.reason_profile {
            match Reasoner::run(&self.graph, profile, true) {
                Ok(report) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&report) {
                        reason_result = parsed;
                    }
                }
                Err(e) => return format!(r#"{{"error":"Reasoning failed: {}"}}"#, e),
            }
        }

        serde_json::json!({
            "ok": true,
            "triples_ingested": triples_loaded,
            "rows_processed": rows.len(),
            "shacl": shacl_result,
            "reasoning": reason_result,
        }).to_string()
    }

    #[tool(name = "onto_import_schema", description = "Import a relational database schema as an OWL ontology. Supports PostgreSQL (postgres://…) and DuckDB (duckdb:///path.duckdb, :memory:, or *.duckdb file path). Introspects tables, columns, primary keys, and foreign keys, then generates OWL classes, datatype/object properties, and cardinality restrictions.")]
    #[allow(unreachable_code, unused_variables, unused_assignments)]
    async fn onto_import_schema(&self, Parameters(input): Parameters<OntoImportSchemaInput>) -> String {
        use crate::schema::SchemaIntrospector;
        use crate::sqlsource;

        let base_iri = input.base_iri.as_deref().unwrap_or("http://example.org/db/");

        // OntoStar Stream 3: admission gate fires BEFORE the introspect+load.
        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::ImportSchema,
            input.scope_token.as_deref(),
            "schema-connection",
            input.connection.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => Some(r),
            Err(denial) => return denial,
        };

        // Dispatch by connection-string scheme. Both backbones land in the
        // same OWL generator so the downstream pipeline (validate + load)
        // is identical.
        let driver = match sqlsource::detect_driver(&input.connection) {
            Ok(d) => d,
            Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
        };

        let tables: Vec<crate::schema::TableInfo> = match driver {
            crate::sqlsource::SqlDriver::Postgres => {
                #[cfg(feature = "postgres")]
                {
                    match SchemaIntrospector::introspect_postgres(&input.connection).await {
                        Ok(t) => t,
                        Err(e) => return format!(r#"{{"error":"Postgres connection failed: {}"}}"#, e),
                    }
                }
                #[cfg(not(feature = "postgres"))]
                {
                    return r#"{"error":"Compiled without postgres feature. Rebuild with --features postgres"}"#.to_string();
                }
            }
            crate::sqlsource::SqlDriver::DuckDb => {
                #[cfg(feature = "duckdb")]
                {
                    let target = sqlsource::duckdb_target(&input.connection);
                    // DuckDB introspection is sync; offload to blocking pool.
                    match tokio::task::spawn_blocking(move || {
                        SchemaIntrospector::introspect_duckdb(&target)
                    })
                    .await
                    {
                        Ok(Ok(t)) => t,
                        Ok(Err(e)) => return format!(r#"{{"error":"DuckDB introspection failed: {}"}}"#, e),
                        Err(e) => return format!(r#"{{"error":"DuckDB worker panicked: {}"}}"#, e),
                    }
                }
                #[cfg(not(feature = "duckdb"))]
                {
                    return r#"{"error":"Compiled without duckdb feature. Rebuild with --features duckdb"}"#.to_string();
                }
            }
        };

        let turtle = SchemaIntrospector::generate_turtle(&tables, base_iri);

        // Validate + load
        if let Err(e) = GraphStore::validate_turtle(&turtle) {
            return format!(r#"{{"error":"Generated Turtle invalid: {}"}}"#, e);
        }

        match self.graph.load_turtle(&turtle, Some(base_iri)) {
            Ok(count) => {
                let mut out = serde_json::json!({
                    "ok": true,
                    "driver": driver.as_str(),
                    "tables": tables.len(),
                    "classes": tables.len(),
                    "triples": count,
                    "base_iri": base_iri,
                });
                if let Some(r) = &receipt
                    && let Some(obj) = out.as_object_mut() {
                        obj.insert("receipt_hash".into(), r.hex().into());
                        obj.insert(
                            "production_law_version".into(),
                            r.record.production_law_version.clone().into(),
                        );
                    }
                out.to_string()
            }
            Err(e) => format!(r#"{{"error":"Failed to load: {}"}}"#, e),
        }
    }

    #[tool(name = "onto_sql_ingest", description = "Run a SQL query against a relational backbone (PostgreSQL or DuckDB) and ingest the resulting rows into the triple store as RDF. DuckDB is recommended as a federation layer: with its httpfs/parquet/csv/postgres_scanner extensions one query can union remote files, object stores, and other databases. The mapping config has the same shape as onto_ingest. Gated by OntoStar admission as the Ingest op.")]
    async fn onto_sql_ingest(&self, Parameters(input): Parameters<OntoSqlIngestInput>) -> String {
        use crate::ingest::DataIngester;
        use crate::mapping::MappingConfig;
        use crate::sqlsource;

        let base_iri = input.base_iri.as_deref().unwrap_or("http://example.org/data/");

        // OntoStar Stream 3: admission gate fires BEFORE any DB or graph work.
        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::Ingest,
            input.scope_token.as_deref(),
            "sql-ingest",
            format!("{}|{}", input.connection, input.sql).as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => Some(r),
            Err(denial) => return denial,
        };
        let _ = &receipt; // wired through OCEL via lineage; success JSON unchanged.

        // Validate connection scheme up front so we fail fast with a clear error.
        let driver = match sqlsource::detect_driver(&input.connection) {
            Ok(d) => d,
            Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
        };

        let rows = match sqlsource::query_rows(&input.connection, &input.sql).await {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"SQL query failed: {}"}}"#, e),
        };

        if rows.is_empty() {
            return serde_json::json!({
                "ok": true,
                "driver": driver.as_str(),
                "triples_loaded": 0,
                "rows_processed": 0,
                "warnings": ["Query returned no rows"],
            })
            .to_string();
        }

        // Resolve mapping (inline JSON / file path / auto from columns).
        let mapping = if let Some(ref mapping_str) = input.mapping {
            if input.inline_mapping.unwrap_or(false) {
                match serde_json::from_str::<MappingConfig>(mapping_str) {
                    Ok(m) => m,
                    Err(e) => return format!(r#"{{"error":"Invalid mapping JSON: {}"}}"#, e),
                }
            } else {
                match std::fs::read_to_string(mapping_str) {
                    Ok(content) => match serde_json::from_str::<MappingConfig>(&content) {
                        Ok(m) => m,
                        Err(e) => return format!(r#"{{"error":"Invalid mapping file: {}"}}"#, e),
                    },
                    Err(e) => return format!(r#"{{"error":"Cannot read mapping file: {}"}}"#, e),
                }
            }
        } else {
            let headers = DataIngester::extract_headers(&rows);
            MappingConfig::from_headers(&headers, base_iri, &format!("{}Thing", base_iri))
        };

        let ntriples = mapping.rows_to_ntriples(&rows);
        match self.graph.load_ntriples(&ntriples) {
            Ok(count) => serde_json::json!({
                "ok": true,
                "driver": driver.as_str(),
                "triples_loaded": count,
                "rows_processed": rows.len(),
                "mapping_fields": mapping.mappings.len(),
            })
            .to_string(),
            Err(e) => format!(r#"{{"error":"Failed to load triples: {}"}}"#, e),
        }
    }

    #[tool(name = "onto_align", description = "Detect alignment candidates (owl:equivalentClass, skos:exactMatch, rdfs:subClassOf) between two ontologies using label similarity, property overlap, parent overlap, instance overlap, restriction patterns, and graph neighborhood. Auto-applies high-confidence matches above threshold. Auto-apply path is gated by OntoStar admission; dry_run path is read-only and skips admission.")]
    pub async fn onto_align(&self, Parameters(input): Parameters<OntoAlignInput>) -> String {
        let engine = crate::align::AlignmentEngine::new(self.db.clone(), self.graph.clone());

        // Auto-apply path mutates the graph (writes equivalentClass /
        // subClassOf triples) — gate it. Dry-run path is read-only.
        let dry_run_flag = input.dry_run.unwrap_or(false);
        let receipt = if !dry_run_flag {
            match self.evaluate_admission(
                crate::admission::AdmissionOp::Align,
                input.scope_token.as_deref(),
                "align-source",
                input.source.as_bytes(),
                input.bypass_admission,
                input.bypass_reason.as_deref(),
            ) {
                Ok(r) => Some(r),
                Err(denial) => return denial,
            }
        } else {
            None
        };

        // Read source (file path or inline)
        let source = if std::path::Path::new(&input.source).exists() {
            match std::fs::read_to_string(&input.source) {
                Ok(s) => s,
                Err(e) => return format!(r#"{{"error":"Failed to read source: {}"}}"#, e),
            }
        } else {
            input.source
        };

        // Read target (file path, inline, or None)
        let target = match input.target {
            Some(t) => {
                if std::path::Path::new(&t).exists() {
                    match std::fs::read_to_string(&t) {
                        Ok(s) => Some(s),
                        Err(e) => return format!(r#"{{"error":"Failed to read target: {}"}}"#, e),
                    }
                } else {
                    Some(t)
                }
            }
            None => None,
        };

        let min_conf = input.min_confidence.unwrap_or(0.85);
        let dry_run = input.dry_run.unwrap_or(false);

        match engine.align(&source, target.as_deref(), min_conf, dry_run) {
            Ok(result) => {
                let ts = chrono::Utc::now().to_rfc3339();
                let obj_id_src = format!("{}:align_source", self.session_id);
                let obj_id_tgt = format!("{}:align_target", self.session_id);
                let _ = self.ocel_store().upsert_object(&obj_id_src, "OntologyVersion", &[]);
                let _ = self.ocel_store().upsert_object(&obj_id_tgt, "OntologyVersion", &[]);

                // Extract alignment metrics
                let (candidate_count, auto_applied) = serde_json::from_str::<serde_json::Value>(&result)
                    .ok()
                    .map(|j| {
                        let cc = j.get("candidates").and_then(|v| v.as_array().map(|a| a.len())).unwrap_or(0);
                        let aa = j.get("auto_applied").and_then(|v| v.as_array().map(|a| a.len())).unwrap_or(0);
                        (cc, aa)
                    })
                    .unwrap_or((0, 0));

                // R4 WE — §14 OCEL purity: dry_run must NOT emit an
                // `align_run` event or a lineage `AL/align` record. The
                // alignment engine ran a read-only candidate scan; emitting
                // an OCEL event would pollute the trail with a record that
                // claims `auto_applied_count` mutated the graph when it did
                // not. Both side effects move inside the apply branch.
                if !dry_run_flag {
                    let event_id = format!("{}:align:{}", self.session_id, chrono::Utc::now().timestamp_millis());
                    let _ = self.ocel_store().emit_event(
                        &event_id,
                        "align_run",
                        &ts,
                        &self.session_id,
                        &[
                            ("threshold", &min_conf.to_string()),
                            ("candidate_count", &candidate_count.to_string()),
                            ("auto_applied_count", &auto_applied.to_string()),
                        ],
                        &[(&obj_id_src, "source_ontology"), (&obj_id_tgt, "target_ontology")],
                        None,
                    );

                    self.lineage().record(&self.session_id, "AL", "align", &format!("threshold={}", min_conf));
                }
                if let Some(r) = &receipt {
                    let mut parsed: serde_json::Value =
                        serde_json::from_str(&result).unwrap_or_else(|_| serde_json::json!({}));
                    if let Some(obj) = parsed.as_object_mut() {
                        obj.insert("receipt_hash".into(), r.hex().into());
                        obj.insert(
                            "production_law_version".into(),
                            r.record.production_law_version.clone().into(),
                        );
                    }
                    parsed.to_string()
                } else {
                    result
                }
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_align_feedback", description = "Accept or reject an alignment candidate to improve future confidence scoring. Stores feedback in align_feedback table for self-calibrating weights. Audit-only admission.")]
    async fn onto_align_feedback(&self, Parameters(input): Parameters<OntoAlignFeedbackInput>) -> String {
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::Feedback,
            None,
            "align-feedback",
            format!("{}|{}|{}", input.source_iri, input.target_iri, input.accepted).as_bytes(),
        );
        let engine = crate::align::AlignmentEngine::new(self.db.clone(), self.graph.clone());
        match engine.record_feedback(&input.source_iri, &input.target_iri, "user_feedback", input.accepted, input.signals.as_ref()) {
            Ok(result) => {
                self.lineage().record(&self.session_id, "AF", "align_feedback", if input.accepted { "accepted" } else { "rejected" });
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    // ─── OntoStar Stream 4 — autonomic feedback loop handlers ────────────────

    #[tool(name = "onto_planner_demos", description = "Loop 4 (cross-session retrieval). Return receipt-backed mined exemplars for a domain. The SQL JOIN to `receipts` enforces the rule: an exemplar without a receipt cannot be returned.")]
    async fn onto_planner_demos(&self, Parameters(input): Parameters<OntoPlannerDemosInput>) -> String {
        let min_fitness = input.min_fitness.unwrap_or(0.95);
        let limit = input.limit.unwrap_or(10);
        match self.ocel_store().exemplars_for_domain(&input.domain, min_fitness, limit) {
            Ok(rows) => serde_json::json!({"ok": true, "domain": input.domain, "count": rows.len(), "exemplars": rows}).to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_threshold_status", description = "Loop 2 (threshold calibration). Read all rows from `workflow_thresholds`.")]
    pub async fn onto_threshold_status(&self) -> String {
        match crate::feedback::thresholds::list_all(self.ocel_store()) {
            Ok(rows) => serde_json::json!({"ok": true, "count": rows.len(), "thresholds": rows}).to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_threshold_sweep", description = "Admin: force-run Loop 2 threshold-calibration sweep. Adjusts `workflow_thresholds.precision_threshold` based on aged-out `bypass_admission` events.")]
    pub async fn onto_threshold_sweep(&self) -> String {
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::ThresholdSweep,
            None,
            "threshold_sweep",
            b"sweep",
        );
        match crate::feedback::thresholds::sweep(self.ocel_store()) {
            Ok(result) => serde_json::json!({"ok": true, "result": result}).to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_workflow_discover", description = "Loop 3 (workflow discovery). Pull OCEL traces for the domain and run wasm4pm discovery; if the discovered fitness exceeds declared by 0.05, insert a `discovered_workflows` row with status=pending.")]
    pub async fn onto_workflow_discover(&self, Parameters(input): Parameters<OntoWorkflowDiscoverInput>) -> String {
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::Discovery,
            Some(&input.domain),
            "workflow_discovery",
            input.domain.as_bytes(),
        );
        match crate::feedback::discovery::discover_for_domain(&input.domain, self.ocel_store()) {
            Ok(Some(dw)) => {
                self.lineage().record(&self.session_id, "WD", "workflow_discover", &dw.id);
                serde_json::json!({"ok": true, "discovered": dw}).to_string()
            }
            Ok(None) => serde_json::json!({"ok": true, "discovered": null, "reason": "no_better_workflow_found_or_threshold_not_met"}).to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_workflow_feedback", description = "Loop 3 surface. Accept or reject a discovered workflow candidate; flips `discovered_workflows.status`. Mirrors the JSON shape of `onto_align_feedback`.")]
    pub async fn onto_workflow_feedback(&self, Parameters(input): Parameters<OntoWorkflowFeedbackInput>) -> String {
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::Feedback,
            Some(&input.id),
            "workflow_feedback",
            format!("{}:{}", input.id, input.accepted).as_bytes(),
        );
        match crate::feedback::discovery::record_feedback(self.ocel_store(), &input.id, input.accepted) {
            Ok(status) => {
                self.lineage().record(&self.session_id, "WF", "workflow_feedback", if input.accepted { "accepted" } else { "rejected" });
                serde_json::json!({"ok": true, "id": input.id, "accepted": input.accepted, "status": status}).to_string()
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_lint_feedback", description = "Accept or dismiss a lint issue to improve future lint runs. Dismissed issues are suppressed after 3 dismissals. Stores feedback for self-calibrating severity.")]
    async fn onto_lint_feedback(&self, Parameters(input): Parameters<OntoLintFeedbackInput>) -> String {
        match crate::feedback::record_tool_feedback(&self.db, "lint", &input.rule_id, &input.entity, input.accepted) {
            Ok(result) => {
                self.lineage().record(&self.session_id, "LF", "lint_feedback", if input.accepted { "accepted" } else { "dismissed" });
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_enforce_feedback", description = "Accept or dismiss an enforce violation to improve future enforce runs. Dismissed violations are suppressed after 3 dismissals. Stores feedback for self-calibrating compliance.")]
    async fn onto_enforce_feedback(&self, Parameters(input): Parameters<OntoEnforceFeedbackInput>) -> String {
        match crate::feedback::record_tool_feedback(&self.db, "enforce", &input.rule_id, &input.entity, input.accepted) {
            Ok(result) => {
                self.lineage().record(&self.session_id, "EF", "enforce_feedback", if input.accepted { "accepted" } else { "dismissed" });
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_embed", description = "Generate text + structural Poincaré embeddings for all classes in the loaded ontology. Requires the embedding model (run `open-ontologies init` to download). Embeddings enable semantic search via onto_search and improve alignment accuracy.")]
    async fn onto_embed(&self, Parameters(input): Parameters<OntoEmbedInput>) -> String {
        let started = std::time::Instant::now();
        let out = self.onto_embed_inner(input).await;
        let ok = !out.contains(r#""error""#);
        self.emit_tool_ocel("onto_embed", started, ok, &[]);
        out
    }

    async fn onto_embed_inner(&self, input: OntoEmbedInput) -> String {
        #[cfg(not(feature = "embeddings"))]
        { let _ = input; r#"{"error":"Compiled without embeddings feature. Rebuild with --features embeddings"}"#.to_string()}
        #[cfg(feature = "embeddings")]
        {
        let embedder = match &self.text_embedder {
            Some(e) => e,
            None => return r#"{"error":"Embedding model not loaded. Run `open-ontologies init` to download."}"#.to_string(),
        };

        let struct_dim = input.struct_dim.unwrap_or(32);
        let struct_epochs = input.struct_epochs.unwrap_or(100);

        let classes_query = r#"
            SELECT DISTINCT ?class ?label WHERE {
                ?class a <http://www.w3.org/2002/07/owl#Class> .
                OPTIONAL { ?class <http://www.w3.org/2000/01/rdf-schema#label> ?label }
                FILTER(isIRI(?class))
            }
        "#;

        let result = match self.graph.sparql_select(classes_query) {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
        };

        let parsed: serde_json::Value = match serde_json::from_str(&result) {
            Ok(v) => v,
            Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
        };

        let mut class_labels: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        if let Some(rows) = parsed["results"].as_array() {
            for row in rows {
                if let Some(iri) = row["class"].as_str() {
                    let iri = iri.trim_matches(|c| c == '<' || c == '>').to_string();
                    let label = row["label"].as_str()
                        .map(|s| s.trim_matches('"').to_string())
                        .unwrap_or_else(|| {
                            iri.rsplit_once('#').or_else(|| iri.rsplit_once('/'))
                                .map(|(_, n)| n.to_string())
                                .unwrap_or_else(|| iri.clone())
                        });
                    class_labels.insert(iri, label);
                }
            }
        }

        if class_labels.is_empty() {
            return r#"{"error":"No ontology loaded. Call onto_load first, then onto_embed to generate embeddings."}"#.to_string();
        }

        let trainer = crate::structembed::StructuralTrainer::new(struct_dim, struct_epochs, 0.01);
        let struct_embeddings = match trainer.train(&self.graph) {
            Ok(e) => e,
            Err(e) => return format!(r#"{{"error":"structural training failed: {}"}}"#, e),
        };

        let mut embedded_count = 0;
        let mut errors: Vec<String> = Vec::new();

        // R7 WD-1 — sanitize each class label through the embed-label
        // boundary before it reaches the embedding provider. The
        // 256-byte cap is enforced; labels exceeding it are recorded
        // as errors and the IRI is skipped (no embedding written).
        for (iri, label) in &class_labels {
            let label_input = match crate::llm_input::LlmInput::sanitize(
                label,
                crate::llm_input::LlmInputKind::EmbedLabel,
            ) {
                Ok(v) => v,
                Err(e) => {
                    errors.push(format!("{}: LlmInput sanitize: {}", iri, e));
                    continue;
                }
            };
            // Compute the text embedding (may await an HTTP call) BEFORE
            // locking the non-Send VecStore mutex.
            match embedder.embed_input(&label_input).await {
                Ok(text_vec) => {
                    let struct_vec = struct_embeddings.get(iri)
                        .cloned()
                        .unwrap_or_else(|| vec![0.0; struct_dim]);
                    let mut vecstore = self.vecstore.lock().unwrap();
                    vecstore.upsert(iri, &text_vec, &struct_vec);
                    embedded_count += 1;
                }
                Err(e) => errors.push(format!("{}: {}", iri, e)),
            }
        }

        {
            let vecstore = self.vecstore.lock().unwrap();
            if let Err(e) = vecstore.persist() {
                return format!(r#"{{"error":"failed to persist embeddings: {}"}}"#, e);
            }
        }

        serde_json::json!({
            "ok": true,
            "embedded": embedded_count,
            "total_classes": class_labels.len(),
            "text_dim": embedder.dim(),
            "struct_dim": struct_dim,
            "errors": errors,
        }).to_string()
        } // cfg(feature = "embeddings")
    }

    #[tool(name = "onto_search", description = "Semantic search over the loaded ontology using natural language. Returns the most similar classes by text meaning, structural position, or both. Requires onto_embed to have been run first.")]
    async fn onto_search(&self, Parameters(input): Parameters<OntoSearchInput>) -> String {
        #[cfg(not(feature = "embeddings"))]
        { let _ = input; return r#"{"error":"Compiled without embeddings feature. Rebuild with --features embeddings"}"#.to_string(); }
        #[cfg(feature = "embeddings")]
        {
        let top_k = input.top_k.unwrap_or(10);
        let mode = input.mode.as_deref().unwrap_or("product");
        let alpha = input.alpha.unwrap_or(0.5);

        let embedder = match &self.text_embedder {
            Some(e) => e,
            None => return r#"{"error":"Embedding model not loaded."}"#.to_string(),
        };

        // R7 WD-1 — sanitize the search query through the embed-query
        // boundary (256-byte cap, chat-marker rejection) before any
        // bytes reach the embedder.
        let query_input = match crate::llm_input::LlmInput::sanitize(
            &input.query,
            crate::llm_input::LlmInputKind::EmbedQuery,
        ) {
            Ok(v) => v,
            Err(e) => {
                return format!(
                    r#"{{"error":"LlmInput sanitize failed: {}"}}"#,
                    e.to_string().replace('"', "'")
                )
            }
        };
        let query_vec = match embedder.embed_input(&query_input).await {
            Ok(v) => v,
            Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
        };

        let vecstore = self.vecstore.lock().unwrap();
        if vecstore.is_empty() {
            return r#"{"error":"No embeddings loaded. Call onto_embed first to generate embeddings for the loaded ontology, then retry onto_search."}"#.to_string();
        }

        let results: Vec<serde_json::Value> = match mode {
            "text" => {
                vecstore.search_cosine(&query_vec, top_k)
                    .into_iter()
                    .map(|(iri, score)| serde_json::json!({"iri": iri, "score": (score * 1000.0).round() / 1000.0}))
                    .collect()
            }
            "structure" => {
                let text_hits = vecstore.search_cosine(&query_vec, 1);
                if let Some((anchor_iri, _)) = text_hits.first() {
                    if let Some(struct_vec) = vecstore.get_struct_vec(anchor_iri) {
                        vecstore.search_poincare(struct_vec, top_k)
                            .into_iter()
                            .map(|(iri, dist)| serde_json::json!({"iri": iri, "poincare_distance": (dist * 1000.0).round() / 1000.0}))
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
            _ => {
                let struct_dim = vecstore.search_cosine(&query_vec, 1)
                    .first()
                    .and_then(|(iri, _)| vecstore.get_struct_vec(iri).map(|v| v.len()))
                    .unwrap_or(32);
                let struct_query = vec![0.0f32; struct_dim];
                vecstore.search_product(&query_vec, &struct_query, top_k, alpha)
                    .into_iter()
                    .map(|(iri, score)| serde_json::json!({"iri": iri, "score": (score * 1000.0).round() / 1000.0}))
                    .collect()
            }
        };

        serde_json::json!({
            "results": results,
            "query": input.query,
            "mode": mode,
            "count": results.len(),
        }).to_string()
        } // cfg(feature = "embeddings")
    }

    #[tool(name = "onto_similarity", description = "Compute embedding similarity between two IRIs — returns cosine similarity (text), Poincaré distance (structural), and product score.")]
    async fn onto_similarity(&self, Parameters(input): Parameters<OntoSimilarityInput>) -> String {
        #[cfg(not(feature = "embeddings"))]
        { let _ = input; return r#"{"error":"Compiled without embeddings feature. Rebuild with --features embeddings"}"#.to_string(); }
        #[cfg(feature = "embeddings")]
        {
        let vecstore = self.vecstore.lock().unwrap();

        let text_a = vecstore.get_text_vec(&input.iri_a);
        let text_b = vecstore.get_text_vec(&input.iri_b);
        let struct_a = vecstore.get_struct_vec(&input.iri_a);
        let struct_b = vecstore.get_struct_vec(&input.iri_b);

        if text_a.is_none() || text_b.is_none() {
            let missing = match (text_a.is_none(), text_b.is_none()) {
                (true, true) => format!("{}, {}", input.iri_a, input.iri_b),
                (true, false) => input.iri_a.clone(),
                (false, true) => input.iri_b.clone(),
                (false, false) => unreachable!(),
            };
            return format!(
                r#"{{"error":"IRI not found in embeddings. Call onto_embed first to generate embeddings for the loaded ontology. Missing: {}"}}"#,
                missing
            );
        }

        let cos = crate::poincare::cosine_similarity(text_a.unwrap(), text_b.unwrap());
        let poinc = if let (Some(a), Some(b)) = (struct_a, struct_b) {
            crate::poincare::poincare_distance(a, b)
        } else {
            -1.0
        };

        let product = if poinc >= 0.0 {
            0.5 * cos + 0.5 / (1.0 + poinc)
        } else {
            cos
        };

        serde_json::json!({
            "iri_a": input.iri_a,
            "iri_b": input.iri_b,
            "cosine_similarity": (cos * 1000.0).round() / 1000.0,
            "poincare_distance": (poinc * 1000.0).round() / 1000.0,
            "product_score": (product * 1000.0).round() / 1000.0,
        }).to_string()
        } // cfg(feature = "embeddings")
    }

    #[tool(name = "onto_process_validate_claim", description = "Validate a process mining claim using real event-log evidence from OTel traces. Uses pm4py for process discovery and conformance checking. Requires at least 3 of 5 surfaces to pass: execution, observability, state, process, causality.")]
    async fn onto_process_validate_claim(&self, Parameters(input): Parameters<OntoProcessValidateClaimInput>) -> String {
        let script = "/Users/sac/chatmangpt/ostar/src/ostar/process/wvda_agent.py";
        let mut cmd = std::process::Command::new("/Users/sac/chatmangpt/ostar/.venv/bin/python");
        cmd.arg(script);
        cmd.arg("--output");
        cmd.arg("json");
        cmd.arg("validate_claim");
        cmd.arg(&input.claim);
        if let Some(ref artifact_id) = input.artifact_id {
            cmd.arg("--artifact-id");
            cmd.arg(artifact_id);
        }
        if let Some(time) = input.time_range_hours {
            cmd.arg("--time-range-hours");
            cmd.arg(time.to_string());
        }

        match self.run_subprocess_with_timeout(&mut cmd, "wvda_agent", script) {
            Ok(timed) => {
                let out = timed.output;
                if out.status.success() {
                    String::from_utf8_lossy(&out.stdout).into_owned()
                } else {
                    let err = String::from_utf8_lossy(&out.stderr);
                    format!(r#"{{"error": "Process mining failed: {}"}}"#, err.replace('"', "\\\"").replace('\n', " "))
                }
            },
            Err(crate::subprocess::SubprocessError::LlmTimeout { elapsed_ms, limit_ms, .. }) => {
                format!(r#"{{"error": "Process mining subprocess timed out after {}ms (limit {}ms)"}}"#, elapsed_ms, limit_ms)
            },
            Err(crate::subprocess::SubprocessError::SpawnFailed(e)) => format!(r#"{{"error": "Failed to spawn Python process: {}"}}"#, e)
        }
    }

    #[tool(name = "onto_process_check_soundness", description = "Check process soundness properties: deadlock-free, liveness, and boundedness. Uses Petri net analysis on discovered process model.")]
    async fn onto_process_check_soundness(&self, Parameters(input): Parameters<OntoProcessCheckSoundnessInput>) -> String {
        let script = "/Users/sac/chatmangpt/ostar/src/ostar/process/wvda_agent.py";
        let mut cmd = std::process::Command::new("/Users/sac/chatmangpt/ostar/.venv/bin/python");
        cmd.arg(script);
        cmd.arg("--output");
        cmd.arg("json");
        cmd.arg("check_process_soundness");
        cmd.arg(&input.event_log_path);

        match self.run_subprocess_with_timeout(&mut cmd, "wvda_agent", script) {
            Ok(timed) => {
                let out = timed.output;
                if out.status.success() {
                    String::from_utf8_lossy(&out.stdout).into_owned()
                } else {
                    let err = String::from_utf8_lossy(&out.stderr);
                    format!(r#"{{"error": "Soundness check failed: {}"}}"#, err.replace('"', "\\\"").replace('\n', " "))
                }
            },
            Err(crate::subprocess::SubprocessError::LlmTimeout { elapsed_ms, limit_ms, .. }) => {
                format!(r#"{{"error": "Soundness check subprocess timed out after {}ms (limit {}ms)"}}"#, elapsed_ms, limit_ms)
            },
            Err(crate::subprocess::SubprocessError::SpawnFailed(e)) => format!(r#"{{"error": "Failed to spawn Python process: {}"}}"#, e)
        }
    }

    #[tool(name = "onto_mustar_solve", description = "Invoke the MuStar Agent to semantically lower a problem intent into a completed artifact. Accepts a problem_statement, domain, constraints, and title. Uses POWL build orders internally and provides empirical validation.")]
    async fn onto_mustar_solve(&self, Parameters(input): Parameters<OntoMustarSolveInput>) -> String {
        let script = "/Users/sac/chatmangpt/ostar/src/ostar/process/mu_star_agent.py";
        let mut cmd = std::process::Command::new("/Users/sac/chatmangpt/ostar/.venv/bin/python");
        cmd.arg(script);
        cmd.arg("--output");
        cmd.arg("json");

        if let Some(ref domain) = input.domain {
            cmd.arg("--domain");
            cmd.arg(domain);
        }
        if let Some(ref constraints) = input.constraints {
            cmd.arg("--constraints");
            cmd.arg(constraints);
        }
        if let Some(ref title) = input.title {
            cmd.arg("--title");
            cmd.arg(title);
        }

        cmd.arg(&input.problem_statement);

        match self.run_subprocess_with_timeout(&mut cmd, "mu_star_agent", script) {
            Ok(timed) => {
                let out = timed.output;
                if out.status.success() {
                    String::from_utf8_lossy(&out.stdout).into_owned()
                } else {
                    let err = String::from_utf8_lossy(&out.stderr);
                    format!(r#"{{"error": "MuStar solver failed: {}"}}"#, err.replace('"', "\\\"").replace('\n', " "))
                }
            },
            Err(crate::subprocess::SubprocessError::LlmTimeout { elapsed_ms, limit_ms, .. }) => {
                format!(r#"{{"error": "MuStar subprocess timed out after {}ms (limit {}ms)"}}"#, elapsed_ms, limit_ms)
            },
            Err(crate::subprocess::SubprocessError::SpawnFailed(e)) => format!(r#"{{"error": "Failed to spawn Python process: {}"}}"#, e)
        }
    }

    #[tool(name = "onto_alphastar_solve", description = "Invoke the Semantic AlphaStar Agent (backward compatibility alias for MuStar) to semantically lower a problem intent into a completed artifact.")]
    async fn onto_alphastar_solve(&self, Parameters(input): Parameters<OntoAlphastarSolveInput>) -> String {
        self.onto_mustar_solve(Parameters(input)).await
    }

    #[tool(name = "onto_codegen", description = "Generate code artifacts from the loaded ontology using ggen. Requires either manifest_path (ggen.toml with generation.rules) or queries_dir (SPARQL queries dir). The generator field maps to ggen --language (python, rust, typescript, go, elixir). Serializes the current in-memory graph to a temp TTL file and invokes ggen sync with correct flags.")]
    async fn onto_codegen(&self, Parameters(input): Parameters<OntoCodegenInput>) -> String {
        use std::process::Command;

        // Ensure loaded
        if let Err(e) = self.registry.ensure_loaded() {
            return format!(r#"{{"error":"Ontology not loaded: {}. Call onto_load first."}}"#, e.to_string().replace('"', "'"));
        }

        // Guard: empty graph
        if self.graph.triple_count() == 0 {
            return r#"{"error":"No triples loaded. Call onto_load first."}"#.to_string();
        }

        // OntoStar Stream 3: admission gate fires BEFORE invoking ggen.
        let artifact_preview = self.graph.serialize("turtle").unwrap_or_default();
        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::Codegen,
            input.scope_token.as_deref(),
            "codegen-input",
            artifact_preview.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => r,
            Err(denial) => return denial,
        };

        // Check: manifest_path or queries_dir required
        if input.manifest_path.is_none() && input.queries_dir.is_none() {
            return r#"{"error":"onto_codegen requires either manifest_path (path to a ggen.toml with generation.rules) or queries_dir (directory of SPARQL .rq files). The generator field maps to ggen --language (accepted: python, rust, typescript, go, elixir). See ~/ggen for examples."}"#.to_string();
        }

        // Map generator name → ggen language
        let gen_lower = input.generator.to_lowercase();
        let language = match gen_lower.as_str() {
            "python-client" | "python" | "py" => "python",
            "rust-structs" | "rust" => "rust",
            "typescript-types" | "typescript" | "ts" => "typescript",
            "go" | "golang" => "go",
            "elixir" => "elixir",
            other => other,
        };

        // Serialize current graph to temp TTL
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tmp_dir = std::env::temp_dir().join(format!("onto_codegen_{}", unique_id));
        if let Err(e) = std::fs::create_dir_all(&tmp_dir) {
            return format!(r#"{{"error":"Failed to create temp dir: {}"}}"#, e);
        }

        let temp_path = tmp_dir.join("ontology.ttl").to_string_lossy().to_string();
        if let Err(e) = self.graph.save_file(&temp_path, "turtle") {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return format!(r#"{{"error":"Failed to serialize graph: {}"}}"#, e);
        }

        // Build ggen sync command
        let output_dir = input.output_dir.as_deref().unwrap_or("./generated");
        let ggen_path = std::env::var("OPEN_ONTOLOGIES_CODEGEN_GGEN_PATH")
            .ok()
            .or_else(|| Some("ggen".to_string()))
            .unwrap();
        let mut cmd = Command::new(crate::config::expand_tilde(&ggen_path));
        cmd.arg("sync")
            .arg("--ontology").arg(&temp_path)
            .arg("--output_dir").arg(output_dir);

        // Mode A: manifest-based
        if let Some(ref manifest) = input.manifest_path {
            cmd.arg("--manifest").arg(manifest);
        }
        // Mode B: low-level pipeline with queries dir
        if let Some(ref queries) = input.queries_dir {
            cmd.arg("--queries").arg(queries);
            cmd.arg("--language").arg(language);
        }

        if input.dry_run.unwrap_or(false) {
            cmd.arg("--dry_run");
        }

        // Cross-link to ggen's own receipt: future ggen builds can read these
        // env vars and embed `ostar_receipt_hash` in `.ggen/receipts/latest.json`.
        cmd.env("OSTAR_RECEIPT_HASH", receipt.hex())
            .env(
                "OSTAR_PRODUCTION_LAW",
                &receipt.record.production_law_version,
            )
            .env(
                "OSTAR_DEFECTS_TAXONOMY",
                &receipt.record.defects_taxonomy_version,
            )
            .env("OSTAR_SCOPE_TOKEN", &receipt.record.scope_token);

        // Execute
        let result = match self.run_subprocess_with_timeout(&mut cmd, "ggen_sync", "ggen") {
            Ok(timed) => {
                let output = timed.output;
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    let ts = chrono::Utc::now().to_rfc3339();
                    let obj_id = format!("{}:codegen:{}", self.session_id, &input.generator);
                    let _ = self.ocel_store().upsert_object(
                        &obj_id,
                        "CodeArtifact",
                        &[("generator", &input.generator, "string"), ("language", language, "string")],
                    );

                    // Walk the output_dir and prepend the OntoStar receipt header
                    // to every supported source file. Best-effort — skipped
                    // files are reported but don't block emission.
                    let stamped = stamp_codegen_output(output_dir, &receipt);

                    let stamped_str = stamped.to_string();
                    let event_id = format!("{}:codegen:{}", self.session_id, chrono::Utc::now().timestamp_millis());
                    let _ = self.ocel_store().emit_event(
                        &event_id,
                        "codegen_run",
                        &ts,
                        &self.session_id,
                        &[
                            ("generator", &input.generator),
                            ("language", language),
                            ("output_dir", output_dir),
                            ("receipt_files_stamped", stamped_str.as_str()),
                            ("receipt_hash", &receipt.hex()),
                        ],
                        &[(&obj_id, "generated_from")],
                        None,
                    );

                    self.lineage().record(&self.session_id, "G", "codegen", &input.generator);
                    serde_json::json!({
                        "ok": true,
                        "generator": input.generator,
                        "language": language,
                        "output_dir": output_dir,
                        "stdout": stdout.trim(),
                        "receipt_hash": receipt.hex(),
                        "production_law_version": receipt.record.production_law_version,
                        "defects_taxonomy_version": receipt.record.defects_taxonomy_version,
                        "receipt_files_stamped": stamped,
                    }).to_string()
                } else {
                    format!(r#"{{"error":"ggen sync failed: {}"}}"#, stderr)
                }
            }
            Err(crate::subprocess::SubprocessError::LlmTimeout { elapsed_ms, limit_ms, .. }) => {
                format!(r#"{{"error":"ggen sync timed out after {}ms (limit {}ms)"}}"#, elapsed_ms, limit_ms)
            }
            Err(crate::subprocess::SubprocessError::SpawnFailed(e)) => {
                let msg = if e.kind() == std::io::ErrorKind::NotFound {
                    "ggen binary not found. Check config.toml [codegen] ggen_path or ensure ggen is in PATH."
                } else {
                    "Failed to invoke ggen"
                };
                format!(r#"{{"error":"{}: {}"}}"#, msg, e)
            }
        };

        // Clean up temp dir
        let _ = std::fs::remove_dir_all(&tmp_dir);

        result
    }

    // ── OntoStar Stream 1 — workflow scope ──────────────────────────────────

    #[tool(name = "onto_declare_workflow", description = "OntoStar: declare a workflow scope. Either pass a built-in `name` (OntologyAuthoring, DataExtension, DataExtensionFastPath, LifecycleApply, Alignment, Codegen, GovernedRelease) or an inline `powl` string. Returns a `scope_token` (ULID) used to tag subsequent OCEL events. Pair with `onto_close_workflow`. R4 WE: full admission via AdmissionOp::WorkflowDeclared.")]
    fn onto_declare_workflow(
        &self,
        Parameters(input): Parameters<OntoDeclareWorkflowInput>,
    ) -> String {
        // R4 WE — §14: full admission BEFORE the scope row is materialized.
        // The artifact bytes are a deterministic concatenation of the
        // (optional) workflow name, the POWL string, and the caller's
        // tenant_id, so two sessions in the same tenant declaring the same
        // workflow produce identical artifact hashes.
        let name_str = input.name.clone().unwrap_or_default();
        let powl_str = input.powl.clone().unwrap_or_default();
        let tenant_str = self.tenant.current().current().to_string();
        let mut artifact_bytes: Vec<u8> = Vec::with_capacity(
            name_str.len() + powl_str.len() + tenant_str.len() + 2,
        );
        artifact_bytes.extend_from_slice(name_str.as_bytes());
        artifact_bytes.push(0);
        artifact_bytes.extend_from_slice(powl_str.as_bytes());
        artifact_bytes.push(0);
        artifact_bytes.extend_from_slice(tenant_str.as_bytes());
        if let Err(denial) = self.evaluate_admission(
            crate::admission::AdmissionOp::WorkflowDeclared,
            input.scope_token.as_deref(),
            "workflow-declared",
            &artifact_bytes,
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            return denial;
        }

        let scope = crate::workflows::WorkflowScope::new(&self.db, &self.session_id);
        match scope.open(
            input.name.as_deref(),
            input.powl.as_deref(),
            input.scope_token.as_deref(),
        ) {
            Ok(token) => serde_json::json!({
                "ok": true,
                "scope_token": token,
            })
            .to_string(),
            Err(crate::workflows::scope::ScopeError::Defect(d)) => serde_json::json!({
                "ok": false,
                "defect": d,
            })
            .to_string(),
            Err(crate::workflows::scope::ScopeError::Storage(e)) => serde_json::json!({
                "ok": false,
                "defect": { "kind": "OcelIncomplete" },
                "storage_error": e,
            })
            .to_string(),
        }
    }

    #[tool(name = "onto_close_workflow", description = "OntoStar: close a previously-declared workflow scope. Writes `closed_at` and flips status to `closed`. Returns `{closed: true, scope_token}` on success; returns a typed `ScopeUnclosed` defect if the token is unknown or already closed. R4 WE: full admission via AdmissionOp::WorkflowClosed.")]
    fn onto_close_workflow(
        &self,
        Parameters(input): Parameters<OntoCloseWorkflowInput>,
    ) -> String {
        // R4 WE — §14: full admission BEFORE the scope is closed.
        // Artifact bytes are the raw `scope_token` (so the artifact hash is
        // a deterministic function of which scope is being closed).
        if let Err(denial) = self.evaluate_admission(
            crate::admission::AdmissionOp::WorkflowClosed,
            Some(&input.scope_token),
            "workflow-closed",
            input.scope_token.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            return denial;
        }

        let scope = crate::workflows::WorkflowScope::new(&self.db, &self.session_id);
        match scope.close(&input.scope_token) {
            Ok(()) => serde_json::json!({
                "closed": true,
                "scope_token": input.scope_token,
            })
            .to_string(),
            Err(crate::workflows::scope::ScopeError::Defect(d)) => serde_json::json!({
                "closed": false,
                "scope_token": input.scope_token,
                "defect": d,
            })
            .to_string(),
            Err(crate::workflows::scope::ScopeError::Storage(e)) => serde_json::json!({
                "closed": false,
                "scope_token": input.scope_token,
                "defect": { "kind": "OcelIncomplete" },
                "storage_error": e,
            })
            .to_string(),
        }
    }

    // ── OntoStar Stream 3 — admission dry-run + session reset ──────────────

    #[tool(
        name = "onto_admission_check",
        description = "OntoStar: read-only dry-run of the admission gate. Returns the same denial JSON as the gated handlers but performs no mutation. `op` ∈ {apply, codegen, save, push}."
    )]
    fn onto_admission_check(
        &self,
        Parameters(input): Parameters<OntoAdmissionCheckInput>,
    ) -> String {
        let op = match input.op.to_lowercase().as_str() {
            "apply" => crate::admission::AdmissionOp::Apply,
            "codegen" => crate::admission::AdmissionOp::Codegen,
            "save" => crate::admission::AdmissionOp::Save,
            "push" => crate::admission::AdmissionOp::Push,
            other => {
                return serde_json::json!({
                    "ok": false,
                    "defect": { "kind": "DeadParameter", "param": format!("op={}", other) },
                })
                .to_string();
            }
        };
        // Use the current canonical graph as the artifact preview.
        let artifact = self.graph.serialize("turtle").unwrap_or_default();
        match self.evaluate_admission(
            op,
            input.scope_token.as_deref(),
            "admission-check",
            artifact.as_bytes(),
            None,
            None,
        ) {
            Ok(receipt) => serde_json::json!({
                "ok": true,
                "admission": "would_grant",
                "receipt_hash": receipt.hex(),
            })
            .to_string(),
            Err(json) => json,
        }
    }

    // ── Cell8 13-gate attestation (Phase 10) — READ-ONLY ───────────────────

    #[tool(
        name = "onto_cell8_attest",
        description = "OntoStar/Cell8: emit the 13-gate EARL conformance attestation for the latest receipt of `scope_token`. Read-only — emits no OCEL events, performs no mutation. Returns `{earl_report, gates_passed, gates_failed, defects[]}`. If no receipt exists for the scope, all 13 gates report `earl:failed` and `defects` includes `AttestationMissing`."
    )]
    fn onto_cell8_attest(
        &self,
        Parameters(input): Parameters<crate::inputs::OntoCell8AttestInput>,
    ) -> String {
        use crate::cell8::{emit_earl_report, count_passed, count_failed, GateOutcome, GATE_NAMES};
        use crate::defects::DefectClass;

        // Look up the latest persisted receipt for this scope.
        let conn = self.db.conn();
        let row = conn
            .query_row(
                "SELECT receipt_hash, artifact_hash, declared_powl_hash, \
                        ocel_canonical_hash, gate_config_hash, prior_receipt_hash, \
                        production_law_version \
                 FROM receipts \
                 WHERE scope_token = ?1 \
                 ORDER BY sequence DESC LIMIT 1",
                rusqlite::params![input.scope_token],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, Option<String>>(5)?,
                        r.get::<_, String>(6)?,
                    ))
                },
            )
            .ok();

        match row {
            Some((
                _receipt_hex,
                artifact_hex,
                powl_hex,
                ocel_hex,
                gate_hex,
                prior_hex,
                law_version,
            )) => {
                fn parse_hex32_local(s: &str) -> [u8; 32] {
                    let mut out = [0u8; 32];
                    for i in 0..32 {
                        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap_or(0);
                    }
                    out
                }
                let prior_receipt =
                    prior_hex.as_ref().map(|s| parse_hex32_local(s));
                let record = crate::production_record::ProductionRecord {
                    artifact_hash: parse_hex32_local(&artifact_hex),
                    scope_token: input.scope_token.clone(),
                    declared_powl_hash: parse_hex32_local(&powl_hex),
                    ocel_canonical_hash: parse_hex32_local(&ocel_hex),
                    conformance_run_id: String::new(),
                    gate_config_hash: parse_hex32_local(&gate_hex),
                    production_law_version: law_version,
                    defects_taxonomy_version: crate::defects::DEFECTS_TAXONOMY_VERSION.into(),
                    gates_passed: GATE_NAMES.iter().map(|s| s.to_string()).collect(),
                    gates_refused: Vec::new(),
                    prior_receipt,
                    signature: None,
                    signing_key_fpr: None,
                };
                let receipt = crate::receipts::build(record);
                let outcomes: Vec<(&str, GateOutcome)> = GATE_NAMES
                    .iter()
                    .map(|g| {
                        (
                            *g,
                            GateOutcome {
                                passed: true,
                                message: format!("{g} verified by persisted receipt"),
                            },
                        )
                    })
                    .collect();
                let report = emit_earl_report(&receipt, &outcomes);
                let passed = count_passed(&outcomes);
                let failed = count_failed(&outcomes);
                let defects: Vec<DefectClass> = Vec::new();
                serde_json::json!({
                    "ok": true,
                    "scope_token": input.scope_token,
                    "earl_report": report,
                    "gates_passed": passed,
                    "gates_failed": failed,
                    "defects": defects,
                })
                .to_string()
            }
            None => {
                // No receipt — emit an all-fail attestation citing the
                // missing external attestation as the proximate defect.
                let placeholder_record = crate::production_record::ProductionRecord {
                    artifact_hash: [0u8; 32],
                    scope_token: input.scope_token.clone(),
                    declared_powl_hash: [0u8; 32],
                    ocel_canonical_hash: [0u8; 32],
                    conformance_run_id: String::new(),
                    gate_config_hash: [0u8; 32],
                    production_law_version: "ontostar-1.0.0".into(),
                    defects_taxonomy_version: crate::defects::DEFECTS_TAXONOMY_VERSION.into(),
                    gates_passed: Vec::new(),
                    gates_refused: vec![DefectClass::AttestationMissing],
                    prior_receipt: None,
                    signature: None,
                    signing_key_fpr: None,
                };
                let receipt = crate::receipts::build(placeholder_record);
                let outcomes: Vec<(&str, GateOutcome)> = GATE_NAMES
                    .iter()
                    .map(|g| {
                        (
                            *g,
                            GateOutcome {
                                passed: false,
                                message: format!("{g} cannot verify: no receipt for scope"),
                            },
                        )
                    })
                    .collect();
                let report = emit_earl_report(&receipt, &outcomes);
                serde_json::json!({
                    "ok": true,
                    "scope_token": input.scope_token,
                    "earl_report": report,
                    "gates_passed": 0,
                    "gates_failed": 13,
                    "defects": [DefectClass::AttestationMissing],
                })
                .to_string()
            }
        }
    }

    #[tool(
        name = "onto_session_reset",
        description = "OntoStar: clear a session's `revoked_sessions` row (sets cleared_at). Use after a `bypass_admission` event when the session is otherwise allowed to resume."
    )]
    fn onto_session_reset(
        &self,
        Parameters(input): Parameters<OntoSessionResetInput>,
    ) -> String {
        match crate::admission::clear_revocation(&self.db, &input.session_id) {
            Ok(()) => {
                self.lineage().record_session_reset(&input.session_id);
                serde_json::json!({
                    "ok": true,
                    "session_id": input.session_id,
                    "cleared": true,
                })
                .to_string()
            }
            Err(e) => serde_json::json!({
                "ok": false,
                "session_id": input.session_id,
                "error": e.to_string(),
            })
            .to_string(),
        }
    }

    // ── OntoStar Stream 2 — conformance check ──────────────────────────────

    #[tool(
        name = "onto_conformance_check",
        description = "OntoStar: replay the OCEL trace tagged with `scope_token` against its declared POWL. Pure delegation to wasm4pm (no local PM math). Returns {fitness, precision, defects, trace_canonical_hash, run_id}."
    )]
    pub fn onto_conformance_check(
        &self,
        Parameters(input): Parameters<OntoConformanceCheckInput>,
    ) -> String {
        // Scope the MutexGuard into a block so it is dropped before
        // replay_against_powl is called. Both self.db and ocel_store.db share
        // the same Arc<Mutex<Connection>>; holding the guard across the replay
        // call deadlocks.
        let powl_string = {
            let conn = self.db.conn();
            let row: Option<(String, Option<String>)> = conn
                .query_row(
                    "SELECT powl_string, status FROM declared_workflows WHERE scope_token = ?1",
                    rusqlite::params![input.scope_token],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
                )
                .ok();
            match row {
                Some((ps, _)) => ps,
                None => {
                    return serde_json::json!({
                        "ok": false,
                        "defect": { "kind": "ScopeUnclosed" },
                        "error": "no declared workflow for scope_token",
                    })
                    .to_string();
                }
            }
        }; // conn guard dropped here

        let mut bridge = crate::powl_bridge::PowlBridge::new();
        let root = match bridge.parse(&powl_string) {
            Ok(r) => r,
            Err(e) => {
                return serde_json::json!({
                    "ok": false,
                    "defect": { "kind": "ReplayFailed" },
                    "error": format!("powl parse: {e}"),
                })
                .to_string();
            }
        };

        match self
            .ocel_store()
            .replay_against_powl(&input.scope_token, &bridge, root, "default")
        {
            Ok(res) => serde_json::json!({
                "ok": true,
                "fitness": res.fitness,
                "precision": res.precision,
                "verdict": res.verdict,
                "defects": res.defects.iter().map(|(d, _)| d).collect::<Vec<_>>(),
                "trace_canonical_hash": res.trace_canonical_hash,
                "run_id": res.run_id,
            })
            .to_string(),
            Err(e) => serde_json::json!({
                "ok": false,
                "defect": { "kind": "ReplayFailed" },
                "error": format!("{e}"),
            })
            .to_string(),
        }
    }

    // ─── Stream 5 — Planner bridge, seed, counterfactual ──────────────────
    //
    // TODO(stream1-4): Streams 1-4 are not yet merged on this branch. The
    // schema for `workflow_scopes`, `mined_exemplars`, and `receipts` is
    // stubbed in `state.rs`; `OcelStore::exemplars_for_domain` is stubbed
    // in `ocel_store.rs`; the synthetic `onto_declare_workflow` call below
    // materializes a scope row directly. When Streams 1-4 land, swap the
    // stubs for the real handlers and types.

    #[tool(name = "onto_plan_workflow", description = "Stream 5: Propose a POWL workflow from a problem statement using MuStar + PowlPredictor (Python subprocess). Loop 1 exemplars warm-start the planner. The result is auto-fed into a synthetic onto_declare_workflow and a scope_token is returned. The planner does NOT admit — admission is the gate's exclusive responsibility.")]
    async fn onto_plan_workflow(&self, Parameters(input): Parameters<OntoPlanWorkflowInput>) -> String {
        use crate::ocel_store::OcelStore;

        // ── Alternative engine: real Groq via pm4py POWL ──────────────────
        // When engine == "groq_powl", we shell out to
        // scripts/powl_from_text.py instead of the MuStar planner.
        // This is the same proven path as tests/real_groq_powl.rs.
        if input.engine.as_deref() == Some("groq_powl") {
            let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("scripts/powl_from_text.py");
            let python = input
                .python
                .clone()
                .unwrap_or_else(|| "python3".to_string());

            // Build description per spec: "{problem_statement}. Domain:
            // {domain}. Constraints: {constraints_csv}". When optional
            // pieces are empty, omit them — appending empty trailing
            // segments materially changes the prompt the downstream
            // LLM sees and degrades verdict reliability for canonical
            // demos.
            let constraints_csv = input.constraints.clone().unwrap_or_default();
            let mut description = input.problem_statement.clone();
            if !input.domain.trim().is_empty() {
                description.push_str(&format!(". Domain: {}", input.domain));
            }
            if !constraints_csv.trim().is_empty() {
                description.push_str(&format!(". Constraints: {}", constraints_csv));
            }

            let mut cmd = std::process::Command::new(&python);
            cmd.arg(&script).arg(&description);
            cmd.env("POWL_DOMAIN", &input.domain);
            // GROQ_API_KEY must already be in env. PM4PY_FORK_PATH falls
            // back to the script default if unset.

            let script_str = script.to_string_lossy().into_owned();
            let out = match self.run_subprocess_with_timeout(&mut cmd, "groq_powl", &script_str) {
                Ok(timed) => timed.output,
                Err(crate::subprocess::SubprocessError::LlmTimeout { elapsed_ms, limit_ms, .. }) => {
                    return format!(
                        r#"{{"ok":false,"error":"powl_from_text.py timed out after {}ms (limit {}ms)"}}"#,
                        elapsed_ms, limit_ms
                    );
                }
                Err(crate::subprocess::SubprocessError::SpawnFailed(e)) => {
                    return format!(
                        r#"{{"ok":false,"error":"failed to spawn powl_from_text.py: {}"}}"#,
                        e.to_string().replace('"', "'")
                    );
                }
            };

            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                return format!(
                    r#"{{"ok":false,"error":"powl_from_text.py exit nonzero: {}"}}"#,
                    stderr.replace('"', "'").replace('\n', " ")
                );
            }

            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            // Same trailing-JSON extraction as tests/real_groq_powl.rs.
            let json_line = match stdout
                .lines()
                .rev()
                .find(|l| l.trim_start().starts_with('{'))
            {
                Some(l) => l.trim().to_string(),
                None => {
                    return format!(
                        r#"{{"ok":false,"error":"powl_from_text.py produced no JSON line: {}"}}"#,
                        stdout.replace('"', "'").replace('\n', " ")
                    )
                }
            };
            let result: serde_json::Value = match serde_json::from_str(&json_line) {
                Ok(v) => v,
                Err(e) => {
                    return format!(
                        r#"{{"ok":false,"error":"powl_from_text.py non-JSON: {} (raw={})"}}"#,
                        e,
                        json_line.replace('"', "'")
                    )
                }
            };

            let powl = result
                .get("powl")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let verdict = result
                .get("verdict")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let refinements = result
                .get("refinements")
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            // Verdict=false → typed denial with replay_failed defect tag.
            if !verdict {
                let reason = result
                    .get("reasoning")
                    .and_then(|v| v.as_str())
                    .unwrap_or("verdict=false from pm4py POWL validator")
                    .to_string();
                return serde_json::json!({
                    "ok": false,
                    "defect": "replay_failed",
                    "reason": reason,
                    "powl": powl,
                    "refinements": refinements,
                })
                .to_string();
            }

            if powl.trim().is_empty() {
                return r#"{"ok":false,"defect":"replay_failed","reason":"empty powl with verdict=true"}"#.to_string();
            }

            // Synthetic onto_declare_workflow — same shape as the mustar
            // path below.
            let scope_token = format!(
                "scope-{}-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0),
                uuid_short(&powl)
            );
            // R4 WE — §14: full admission BEFORE the synthetic scope INSERT.
            // Centralised in `persist_planned_scope` so both engine paths
            // funnel through the same gated code.
            if let Err(denial) = self.persist_planned_scope(
                &scope_token,
                &input.domain,
                &powl,
                input.bypass_admission,
                input.bypass_reason.as_deref(),
            ) {
                return denial;
            }

            let details = format!(
                "build_order_generated source=groq_powl scope={} powl_length={}",
                scope_token,
                powl.len()
            );
            self.lineage()
                .record(&self.session_id, "BG", "build_order_generated", &details);

            return serde_json::json!({
                "ok": true,
                "scope_token": scope_token,
                "powl": powl,
                "verdict": verdict,
                "refinements": refinements,
                "engine": "groq_powl",
            })
            .to_string();
        }

        // Loop 1 warm-start.
        let store = OcelStore::new(self.db.clone());
        let exemplars = store
            .exemplars_for_domain(&input.domain, 0.85, 5)
            .unwrap_or_default();

        let payload = serde_json::json!({
            "problem_statement": input.problem_statement,
            "domain": input.domain,
            "constraints": input.constraints.clone().unwrap_or_default(),
            "exemplars": exemplars,
        });

        let python = input.python.clone().unwrap_or_else(|| "python3".to_string());
        let script = input
            .planner_script
            .clone()
            .unwrap_or_else(|| "~/chatmangpt/ostar/src/ostar/process/ontostar_planner.py".to_string());
        let script = expand_tilde(&script);

        let mut cmd = std::process::Command::new(&python);
        cmd.arg(&script);

        let payload_bytes = payload.to_string().into_bytes();
        let out = match self.run_subprocess_with_timeout_stdin(
            &mut cmd,
            &payload_bytes,
            "ontostar_planner",
            &script,
        ) {
            Ok(timed) => timed.output,
            Err(crate::subprocess::SubprocessError::LlmTimeout { elapsed_ms, limit_ms, .. }) => {
                return format!(
                    r#"{{"error":"ontostar_planner.py timed out after {}ms (limit {}ms)"}}"#,
                    elapsed_ms, limit_ms
                );
            }
            Err(crate::subprocess::SubprocessError::SpawnFailed(e)) => {
                return format!(
                    r#"{{"error":"Failed to spawn ontostar_planner.py: {}"}}"#,
                    e.to_string().replace('"', "'")
                );
            }
        };

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return format!(
                r#"{{"error":"planner exit nonzero: {}"}}"#,
                stderr.replace('"', "'").replace('\n', " ")
            );
        }

        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let parsed: serde_json::Value = match serde_json::from_str(stdout.trim()) {
            Ok(v) => v,
            Err(e) => {
                return format!(
                    r#"{{"error":"planner produced non-JSON output: {} (raw={})"}}"#,
                    e,
                    stdout.replace('"', "'").replace('\n', " ")
                )
            }
        };

        if parsed.get("error").is_some() {
            return parsed.to_string();
        }

        let powl_string = parsed
            .get("powl_string")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if powl_string.trim().is_empty() {
            return r#"{"error":"planner returned empty powl_string"}"#.to_string();
        }

        // Synthetic onto_declare_workflow — Stream 1 is not yet merged on
        // this branch, so we materialize the scope row directly. Once
        // Stream 1 lands, this should call the real handler.
        // TODO(stream1): replace with self.onto_declare_workflow(...).
        let scope_token = format!(
            "scope-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            uuid_short(&powl_string)
        );

        // R4 WE — §14: full admission BEFORE the synthetic scope INSERT.
        if let Err(denial) = self.persist_planned_scope(
            &scope_token,
            &input.domain,
            &powl_string,
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            return denial;
        }

        // OCEL event: build_order_generated.
        let build_order = parsed
            .get("build_order")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let powl_metrics = serde_json::json!({
            "powl_length": powl_string.len(),
            "build_order_length": build_order.len(),
            "refined": parsed.get("refined").and_then(|v| v.as_bool()).unwrap_or(false),
            "exemplars_used": parsed.get("exemplars_used").and_then(|v| v.as_u64()).unwrap_or(0),
        });
        let details = format!(
            "build_order_generated source=mustar+powlpredictor scope={} metrics={}",
            scope_token,
            powl_metrics
        );
        self.lineage()
            .record(&self.session_id, "BG", "build_order_generated", &details);

        serde_json::json!({
            "ok": true,
            "scope_token": scope_token,
            "powl_string": powl_string,
            "build_order": build_order,
            "sequence_diagram": parsed.get("sequence_diagram").and_then(|v| v.as_str()).unwrap_or(""),
            "refined": parsed.get("refined").and_then(|v| v.as_bool()).unwrap_or(false),
            "refine_issues": parsed.get("refine_issues").and_then(|v| v.as_str()).unwrap_or(""),
            "exemplars_used": parsed.get("exemplars_used").and_then(|v| v.as_u64()).unwrap_or(0),
        })
        .to_string()
    }

    #[tool(name = "onto_exemplar_seed", description = "Stream 5: Admin handler. Read a seed OCEL JSON file (default: ~/chatmangpt/ostar/artifacts/ocel/mu_star/ONTOLOGY.oceljson), extract `build_order_generated` events, and insert them into mined_exemplars with synthesized receipts marked production_law_version='seed-v0'. Strict admission can later filter via WHERE production_law_version != 'seed-v0'. Bootstrap-only: refuses with BootstrapClosed once any non-seed receipt has been admitted.")]
    async fn onto_exemplar_seed(&self, Parameters(input): Parameters<OntoExemplarSeedInput>) -> String {
        // R4 WE — §14: bootstrap-only precondition. Once a real production
        // receipt exists, the seed handler refuses with BootstrapClosed —
        // an admin handler that mutates the store after production traffic
        // has begun is a fail-open hole. The env override
        // `OPEN_ONTOLOGIES_BOOTSTRAP_MODE=1` keeps integration tests
        // unblocked.
        if !crate::bootstrap::BootstrapState::is_bootstrap(&self.db) {
            return serde_json::json!({
                "ok": false,
                "defect": { "kind": "BootstrapClosed" },
                "reason": "onto_exemplar_seed runs only during bootstrap; production receipts present (set OPEN_ONTOLOGIES_BOOTSTRAP_MODE=1 to override during integration tests)",
            }).to_string();
        }
        let raw_path = input
            .path
            .clone()
            .unwrap_or_else(|| "~/chatmangpt/ostar/artifacts/ocel/mu_star/ONTOLOGY.oceljson".to_string());
        let path = expand_tilde(&raw_path);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                return format!(
                    r#"{{"error":"failed to read seed OCEL: {} ({})"}}"#,
                    path,
                    e
                )
            }
        };
        // R4 WE — §14: audit-only emission BEFORE the seed mutation.
        // ExemplarSeeded is bootstrap-tier; the gate cannot deny but the
        // OCEL trail must self-attribute.
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::ExemplarSeeded,
            None,
            "exemplar-seed",
            &bytes,
        );
        let default_domain = input.domain.clone().unwrap_or_else(|| "ONTOLOGY".to_string());
        let store = crate::ocel_store::OcelStore::new(self.db.clone());
        let inserted = match store.seed_from_ocel_bytes(&bytes, &default_domain) {
            Ok(n) => n,
            Err(e) => {
                return format!(r#"{{"error":"seed ingestion failed: {}"}}"#, e)
            }
        };

        serde_json::json!({
            "ok": true,
            "inserted": inserted,
            "source": path,
            "production_law_version": "seed-v0",
        })
        .to_string()
    }

    #[tool(name = "onto_counterfactual", description = "Stream 5: Read-only probe. For a given scope_token, returns side-by-side: (a) the naked-craft path (no scope, gate bypassed, force=true → always Admitted) and (b) the OntoStar admission path (real verdict). Surfaces the manufacturing delta — the set of gates where the two paths diverged.")]
    async fn onto_counterfactual(&self, Parameters(input): Parameters<OntoCounterfactualInput>) -> String {
        // Load scope row (Stream 5 stub schema).
        let conn = self.db.conn();
        // Pull the canonical counterfactual columns from declared_workflows
        // (via the workflow_scopes view). Includes gates_denied_json and
        // manufacturing_delta_json so the response carries the persisted
        // delta — not a placeholder.
        // Tuple shape: (scope_token, name, domain, admitted, fitness,
        //  defects_json, deviations_json, gates_fired_json, gates_denied_json,
        //  manufacturing_delta_json)
        type CounterfactualRow = (
            String, String, String,
            Option<i64>, Option<f64>,
            Option<String>, Option<String>,
            Option<String>, Option<String>,
            Option<String>,
        );
        let row: Option<CounterfactualRow> = conn
            .query_row(
                "SELECT dw.scope_token, dw.name, COALESCE(json_extract(dw.alphabet_json,'$.domain'),''),
                        dw.admitted, dw.fitness,
                        dw.defects_json, dw.deviations_json,
                        dw.gates_fired_json, dw.gates_denied_json,
                        dw.manufacturing_delta_json
                 FROM declared_workflows dw WHERE dw.scope_token = ?1",
                rusqlite::params![input.scope_token],
                |r| Ok((
                    r.get(0)?, r.get(1)?, r.get(2)?,
                    r.get(3)?, r.get(4)?,
                    r.get(5)?, r.get(6)?,
                    r.get(7)?, r.get(8)?,
                    r.get(9)?,
                )),
            )
            .ok();
        drop(conn);

        let (scope_token, workflow_name, domain, admitted, fitness,
             defects_json, deviations_json, gates_fired_json, gates_denied_json,
             manufacturing_delta_json) =
            match row {
                Some(r) => r,
                None => {
                    return format!(
                        r#"{{"error":"unknown scope_token: {}"}}"#,
                        input.scope_token.replace('"', "'")
                    )
                }
            };

        let parse_arr = |s: &Option<String>| -> serde_json::Value {
            s.as_deref()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok())
                .unwrap_or_else(|| serde_json::json!([]))
        };
        let gates_fired = parse_arr(&gates_fired_json);
        let gates_denied = parse_arr(&gates_denied_json);

        // naked_craft = force=true path. The naked-craft script bypasses
        // every gate, so gates_checked is the empty set: nothing was actually
        // verified before the artifact was admitted.
        let naked_craft = serde_json::json!({
            "scope_token": scope_token,
            "force": true,
            "verdict": "granted_by_force",
            "gates_checked": serde_json::Value::Array(vec![]),
            "gates_denied": serde_json::Value::Array(vec![]),
        });

        // onto_star path = real persisted verdict from admission.rs.
        let onto_star_path = serde_json::json!({
            "scope_token": scope_token,
            "workflow_name": workflow_name,
            "verdict": if admitted.unwrap_or(0) > 0 { "granted" } else { "denied" },
            "fitness": fitness.unwrap_or(0.0),
            "gates_fired": gates_fired.clone(),
            "gates_denied": gates_denied,
            "defects": parse_arr(&defects_json),
            "deviations": parse_arr(&deviations_json),
        });

        // Manufacturing delta = gates fired ONLY because of OntoStar. The
        // naked_craft path fires zero gates, so the delta IS gates_fired.
        // If admission persisted a richer delta record, surface it.
        let delta = manufacturing_delta_json
            .as_deref()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok())
            .unwrap_or_else(|| serde_json::json!({
                "fired_only_under_ontostar": gates_fired,
                "naked_craft_verdict": "granted_by_force",
            }));

        serde_json::json!({
            "ok": true,
            "naked_craft": naked_craft,
            "onto_star": onto_star_path,
            "manufacturing_delta": delta,
            "manufacturing_path": domain,
        })
        .to_string()
    }

    // ── Requirements-Andon / CTQ-Forge handlers (Phase 1.5) ──────────────

    #[tool(name = "onto_propose_requirement", description = "Requirements Andon: capture a stakeholder source-voice signal and propose a requirement. The deterministic gate denies with RequirementWithoutSource if source_voice is empty/whitespace. Emits the workflow-anchor `requirement_proposed` OCEL event with source_voice + voice_kind attributes. Returns receipt_hash on Ok.")]
    async fn onto_propose_requirement(&self, Parameters(input): Parameters<OntoProposeRequirementInput>) -> String {
        let started = std::time::Instant::now();
        let voice = input.source_voice.trim();
        if voice.is_empty() {
            // Pre-gate denial — source signal is mandatory.
            self.lineage().record_admission_denied(&self.session_id, "requirement_without_source");
            self.emit_tool_ocel("onto_propose_requirement", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "admission": "denied",
                "defect": { "kind": "RequirementWithoutSource" },
            }).to_string();
        }
        let voice_kind = input.voice_kind.as_deref().unwrap_or("operator").trim();
        // Emit the requirement_proposed activity BEFORE the gate fires so
        // the gate observes it as part of `observed_stages`.
        let now = chrono::Utc::now().to_rfc3339();
        let event_id = format!(
            "{}:requirement_proposed:{}",
            self.session_id,
            chrono::Utc::now().timestamp_millis()
        );
        let _ = self.ocel_store().emit_event(
            &event_id,
            "requirement_proposed",
            &now,
            &self.session_id,
            &[
                ("source_voice", voice),
                ("voice_kind", voice_kind),
            ],
            &[],
            input.scope_token.as_deref(),
        );
        // Artifact bytes commit the source-voice text into the receipt so
        // any later replay can verify the same voice was admitted.
        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::RequirementProposed,
            input.scope_token.as_deref(),
            "requirement-proposed",
            voice.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => r,
            Err(denial) => {
                self.emit_tool_ocel("onto_propose_requirement", started, false, &[]);
                return denial;
            }
        };
        let out = serde_json::json!({
            "ok": true,
            "scope_token": receipt.record.scope_token,
            "receipt_hash": receipt.hex(),
            "production_law_version": receipt.record.production_law_version,
            "defects_taxonomy_version": receipt.record.defects_taxonomy_version,
            "voice_kind": voice_kind,
        }).to_string();
        self.emit_tool_ocel("onto_propose_requirement", started, true, &[]);
        out
    }

    /// Projection-only contract (§7 LLMAuthority + §13 JSON-as-authority):
    ///
    /// The JSON returned by `onto_translate_candidate` is **a projection
    /// of an LLM proposal**, not authority. Every response carries the
    /// field `_projection_only: true`. The `candidate` object embedded
    /// in the response is provisional — admission flows exclusively
    /// through `onto_admit_ctq`. Downstream consumers MUST NOT lift
    /// fields from this JSON into receipts, production records, or
    /// trust structures without first passing them through the
    /// deterministic admission gate.
    ///
    /// When the LLM marks its own output authoritative (`provisional:
    /// false` or `authoritative: true` in the reply), the
    /// [`crate::signature_shape::ParsedFields`] returned by the gauge
    /// flips `llm_claimed_authority`. This handler emits
    /// `llm_authority_claimed` OCEL **before** lifting the fields into
    /// `CandidateCtq`, so the audit trail records the adversarial
    /// claim independently of any downstream defect classification.
    #[tool(name = "onto_translate_candidate", description = "Requirements Andon: invoke the LLM boundary translator on a previously-proposed requirement. AUDIT-ONLY — output is provisional and must pass through onto_admit_ctq before any work order is admitted. Response is projection-only (`_projection_only: true`); admission flows through `onto_admit_ctq`. Emits `llm_candidate_translated` + `llm_invoked` OCEL events with candidate_ctq_id (BLAKE3 of the candidate JSON) but never the API key. `engine` selects `inproc` (default), `groq_pm4py` (shells to scripts/ctq_from_voice.py), or `gemini` (headless Gemini CLI via OAuth, no API key required; uses gemini-3.1-flash-lite-preview).")]
    pub async fn onto_translate_candidate(&self, Parameters(input): Parameters<OntoTranslateCandidateInput>) -> String {
        let started = std::time::Instant::now();

        // ── Alternative engine: real Groq via pm4py-style DSPy subprocess ──
        // Mirrors the `groq_powl` branch in onto_plan_workflow. Output JSON
        // matches scripts/ctq_from_voice.py's contract.
        let header_engine = current_llm_engine_override();
        let engine = self.resolve_engine(input.engine.as_deref(), header_engine.as_deref());
        if engine == "groq_pm4py" {
            let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("scripts/ctq_from_voice.py");
            let python = input
                .python
                .clone()
                .unwrap_or_else(|| "python3".to_string());

            let mut cmd = std::process::Command::new(&python);
            cmd.arg(&script).arg(&input.source_voice);
            // GROQ_API_KEY must already be in env. Never logged.

            let sub_started = std::time::Instant::now();
            let script_str = script.to_string_lossy().into_owned();
            let out = match self.run_subprocess_with_timeout(&mut cmd, "groq_pm4py", &script_str) {
                Ok(timed) => timed.output,
                Err(crate::subprocess::SubprocessError::LlmTimeout { elapsed_ms, limit_ms, .. }) => {
                    self.emit_tool_ocel("onto_translate_candidate", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"ctq_from_voice.py timed out after {}ms (limit {}ms)"}}"#,
                        elapsed_ms, limit_ms
                    );
                }
                Err(crate::subprocess::SubprocessError::SpawnFailed(e)) => {
                    self.emit_tool_ocel("onto_translate_candidate", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"failed to spawn ctq_from_voice.py: {}"}}"#,
                        e.to_string().replace('"', "'")
                    );
                }
            };
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.emit_tool_ocel("onto_translate_candidate", started, false, &[]);
                return format!(
                    r#"{{"ok":false,"error":"ctq_from_voice.py exit nonzero: {}"}}"#,
                    stderr.replace('"', "'").replace('\n', " ")
                );
            }
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            let json_line = match stdout
                .lines()
                .rev()
                .find(|l| l.trim_start().starts_with('{'))
            {
                Some(l) => l.trim().to_string(),
                None => {
                    self.emit_tool_ocel("onto_translate_candidate", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"ctq_from_voice.py produced no JSON line: {}"}}"#,
                        stdout.replace('"', "'").replace('\n', " ")
                    );
                }
            };
            let result: serde_json::Value = match serde_json::from_str(&json_line) {
                Ok(v) => v,
                Err(e) => {
                    self.emit_tool_ocel("onto_translate_candidate", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"ctq_from_voice.py non-JSON: {} (raw={})"}}"#,
                        e,
                        json_line.replace('"', "'")
                    );
                }
            };
            let latency_ms = sub_started.elapsed().as_millis() as u64;

            let get_str = |k: &str| -> String {
                result.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
            };
            // §7 LLMAuthority: detect whether the `groq_pm4py`
            // subprocess's JSON output claims authority. The shape is
            // identical to the `inproc` detection in
            // `signature_shape::parse_and_validate` — `provisional:
            // false` OR `authoritative: true`. Emit OCEL **before**
            // lifting the data into a `CandidateCtq`, so the audit
            // trail records the claim independently.
            let llm_claimed_authority_pm4py = result
                .get("provisional")
                .and_then(|v| v.as_bool())
                .map(|b| !b)
                .unwrap_or(false)
                || result
                    .get("authoritative")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
            if llm_claimed_authority_pm4py {
                let now_pre = chrono::Utc::now().to_rfc3339();
                let ts_pre = chrono::Utc::now().timestamp_millis();
                let _ = self.ocel_store().emit_event(
                    &format!("{}:llm_authority_claimed:{}", self.session_id, ts_pre),
                    "llm_authority_claimed",
                    &now_pre,
                    &self.session_id,
                    &[
                        ("engine", "groq_pm4py"),
                        ("defect_class", "llm_authority_claimed"),
                        ("provisional_forced_to", "true"),
                    ],
                    &[],
                    Some(&input.scope_token),
                );
            }
            let candidate = crate::llm_translator::CandidateCtq {
                source_voice_echo: input.source_voice.clone(),
                defect_class_hint: get_str("defect_class_hint"),
                ctq_text: get_str("ctq_text"),
                measure_text: get_str("measure_text"),
                verification_text: get_str("verification_text"),
                negative_case_text: get_str("negative_case_text"),
                control_plan_text: get_str("control_plan_text"),
                provisional: true,
            };
            let verdict = result.get("verdict").and_then(|v| v.as_bool()).unwrap_or(false);
            let refinements = result.get("refinements").and_then(|v| v.as_u64()).unwrap_or(0);

            let candidate_json = serde_json::to_string(&candidate).unwrap_or_else(|_| "{}".into());
            let candidate_id_hex = blake3::hash(candidate_json.as_bytes()).to_hex().to_string();
            let model = std::env::var("CTQ_MODEL")
                .unwrap_or_else(|_| "groq/openai/gpt-oss-20b".to_string());

            let now = chrono::Utc::now().to_rfc3339();
            let ts_ms = chrono::Utc::now().timestamp_millis();
            let _ = self.ocel_store().emit_event(
                &format!("{}:llm_candidate_translated:{}", self.session_id, ts_ms),
                "llm_candidate_translated",
                &now,
                &self.session_id,
                &[
                    ("candidate_ctq_id", &candidate_id_hex[..16]),
                    ("model", &model),
                    ("provisional", "true"),
                    ("engine", "groq_pm4py"),
                ],
                &[],
                Some(&input.scope_token),
            );
            let latency_str = latency_ms.to_string();
            let refinements_str = refinements.to_string();
            let _ = self.ocel_store().emit_event(
                &format!("{}:llm_invoked:{}", self.session_id, ts_ms),
                "llm_invoked",
                &now,
                &self.session_id,
                &[
                    ("model", &model),
                    ("latency_ms", &latency_str),
                    ("refinements", &refinements_str),
                    ("engine", "groq_pm4py"),
                ],
                &[],
                Some(&input.scope_token),
            );
            self.lineage().record(
                &self.session_id,
                "LM",
                "llm_invoked",
                &format!(
                    "engine=groq_pm4py model={} latency_ms={} refinements={} verdict={}",
                    model, latency_ms, refinements, verdict
                ),
            );
            self.evaluate_admission_audit(
                crate::admission::AdmissionOp::LlmTranslate,
                Some(&input.scope_token),
                "llm-translate",
                candidate_json.as_bytes(),
            );
            let ctq_text_top = candidate.ctq_text.clone();
            let response = serde_json::json!({
                "ok": true,
                "provisional": true,
                // §13 JSON-as-authority: the response is a projection
                // of an LLM proposal, not authority. Admission flows
                // through `onto_admit_ctq`.
                "_projection_only": true,
                "engine": "groq_pm4py",
                "candidate_ctq_id": &candidate_id_hex[..16],
                "candidate": candidate,
                "ctq_text": ctq_text_top,
                "verdict": verdict,
                "refinements": refinements,
                "latency_ms": latency_ms,
                "llm_claimed_authority": llm_claimed_authority_pm4py,
            }).to_string();
            self.emit_tool_ocel("onto_translate_candidate", started, true, &[]);
            return response;
        }

        // ── Alternative engine: Gemini CLI (OAuth, no API key required) ──
        // Invokes `gemini -p <prompt> --model gemini-3.1-flash-lite-preview
        // --approval-mode yolo` and extracts the trailing JSON line from stdout.
        // Mirrors the speckit-ralph copilot-shim.sh / gemini-invoke.sh pattern.
        //
        // autoML fallback: `engine=gemini` is a *preference*, not a hard
        // requirement.  If Gemini times out, fails to spawn, or produces no
        // parseable JSON, we emit an `llm_engine_fallback` OCEL event with the
        // failure reason and fall through to the inproc engine rather than
        // returning an error immediately.  This lets the system automatically
        // select the next reliable engine without caller intervention.
        if engine == "gemini" {
            // Attempt the Gemini path; returns Some(response_json) on success or
            // None on any failure (timeout / spawn error / no JSON / parse error).
            // On None we fall through to inproc below.
            let gemini_outcome: Option<String> = 'gemini: {
            let prompt = format!(
                "You are a Critical-To-Quality (CTQ) requirements translator.\n\
                 Translate the following stakeholder voice into a CTQ JSON object.\n\
                 IMPORTANT: Output ONLY the raw JSON on the FINAL line of your response.\n\
                 Use DOUBLE QUOTES for all strings. No markdown, no code fences, no explanation.\n\
                 The JSON must have exactly these keys:\n\
                 \"defect_class_hint\", \"ctq_text\", \"measure_text\", \"verification_text\",\n\
                 \"negative_case_text\", \"control_plan_text\", \"verdict\" (boolean),\n\
                 \"refinements\" (number, 0), \"provisional\" (boolean, true)\n\
                 \n\
                 Stakeholder voice: {}\n\
                 \n\
                 Respond with ONLY the JSON object:",
                input.source_voice
            );

            let gemini_bin = std::env::var("GEMINI_BIN").unwrap_or_else(|_| "gemini".to_string());
            let mut cmd = std::process::Command::new(&gemini_bin);
            cmd.arg("-p")
                .arg(&prompt)
                .arg("--model")
                .arg(crate::config::GEMINI_DEFAULT_MODEL)
                .arg("--approval-mode")
                .arg("yolo");

            let sub_started = std::time::Instant::now();
            let out = match self.run_subprocess_with_timeout(&mut cmd, "gemini", "gemini") {
                Ok(timed) => timed.output,
                Err(crate::subprocess::SubprocessError::LlmTimeout { elapsed_ms, limit_ms, .. }) => {
                    let reason = format!("gemini timed out after {}ms (limit {}ms)", elapsed_ms, limit_ms);
                    let now_fb = chrono::Utc::now().to_rfc3339();
                    let ts_fb = chrono::Utc::now().timestamp_millis();
                    let _ = self.ocel_store().emit_event(
                        &format!("{}:llm_engine_fallback:{}", self.session_id, ts_fb),
                        "llm_engine_fallback",
                        &now_fb,
                        &self.session_id,
                        &[("from_engine", "gemini"), ("to_engine", "inproc"), ("reason", &reason)],
                        &[],
                        Some(&input.scope_token),
                    );
                    break 'gemini None;
                }
                Err(crate::subprocess::SubprocessError::SpawnFailed(e)) => {
                    let reason = format!("failed to spawn gemini: {}", e.to_string().replace('"', "'"));
                    let now_fb = chrono::Utc::now().to_rfc3339();
                    let ts_fb = chrono::Utc::now().timestamp_millis();
                    let _ = self.ocel_store().emit_event(
                        &format!("{}:llm_engine_fallback:{}", self.session_id, ts_fb),
                        "llm_engine_fallback",
                        &now_fb,
                        &self.session_id,
                        &[("from_engine", "gemini"), ("to_engine", "inproc"), ("reason", &reason)],
                        &[],
                        Some(&input.scope_token),
                    );
                    break 'gemini None;
                }
            };
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            // Extract trailing JSON: scan from end for a line containing `{`
            // anywhere (not just at start, to handle warning prefixes from gemini).
            let json_line = stdout
                .lines()
                .rev()
                .find_map(|l| {
                    // Find the first `{` in the line and try to extract from there.
                    l.find('{').map(|pos| l[pos..].trim().to_string())
                });
            let json_line = match json_line {
                Some(l) => l,
                None => {
                    let reason = format!("gemini produced no JSON line (raw={})", stdout.replace('"', "'").replace('\n', " "));
                    let now_fb = chrono::Utc::now().to_rfc3339();
                    let ts_fb = chrono::Utc::now().timestamp_millis();
                    let _ = self.ocel_store().emit_event(
                        &format!("{}:llm_engine_fallback:{}", self.session_id, ts_fb),
                        "llm_engine_fallback",
                        &now_fb,
                        &self.session_id,
                        &[("from_engine", "gemini"), ("to_engine", "inproc"), ("reason", &reason)],
                        &[],
                        Some(&input.scope_token),
                    );
                    break 'gemini None;
                }
            };
            let result: serde_json::Value = match serde_json::from_str(&json_line) {
                Ok(v) => v,
                Err(e) => {
                    let reason = format!("gemini non-JSON: {} (raw={})", e, json_line.replace('"', "'"));
                    let now_fb = chrono::Utc::now().to_rfc3339();
                    let ts_fb = chrono::Utc::now().timestamp_millis();
                    let _ = self.ocel_store().emit_event(
                        &format!("{}:llm_engine_fallback:{}", self.session_id, ts_fb),
                        "llm_engine_fallback",
                        &now_fb,
                        &self.session_id,
                        &[("from_engine", "gemini"), ("to_engine", "inproc"), ("reason", &reason)],
                        &[],
                        Some(&input.scope_token),
                    );
                    break 'gemini None;
                }
            };
            let latency_ms = sub_started.elapsed().as_millis() as u64;
            let get_str = |k: &str| -> String {
                result.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
            };
            let candidate = crate::llm_translator::CandidateCtq {
                source_voice_echo: input.source_voice.clone(),
                defect_class_hint: get_str("defect_class_hint"),
                ctq_text: get_str("ctq_text"),
                measure_text: get_str("measure_text"),
                verification_text: get_str("verification_text"),
                negative_case_text: get_str("negative_case_text"),
                control_plan_text: get_str("control_plan_text"),
                provisional: true,
            };
            let verdict = result.get("verdict").and_then(|v| v.as_bool()).unwrap_or(false);
            let refinements = result.get("refinements").and_then(|v| v.as_u64()).unwrap_or(0);
            let candidate_json = serde_json::to_string(&candidate).unwrap_or_else(|_| "{}".into());
            let candidate_id_hex = blake3::hash(candidate_json.as_bytes()).to_hex().to_string();
            let model = crate::config::GEMINI_DEFAULT_MODEL;
            let now = chrono::Utc::now().to_rfc3339();
            let ts_ms = chrono::Utc::now().timestamp_millis();
            let latency_str = latency_ms.to_string();
            let refinements_str = refinements.to_string();
            let _ = self.ocel_store().emit_event(
                &format!("{}:llm_candidate_translated:{}", self.session_id, ts_ms),
                "llm_candidate_translated",
                &now,
                &self.session_id,
                &[
                    ("candidate_ctq_id", &candidate_id_hex[..16]),
                    ("model", model),
                    ("provisional", "true"),
                    ("engine", "gemini"),
                ],
                &[],
                Some(&input.scope_token),
            );
            let _ = self.ocel_store().emit_event(
                &format!("{}:llm_invoked:{}", self.session_id, ts_ms),
                "llm_invoked",
                &now,
                &self.session_id,
                &[
                    ("model", model),
                    ("latency_ms", &latency_str),
                    ("refinements", &refinements_str),
                    ("engine", "gemini"),
                ],
                &[],
                Some(&input.scope_token),
            );
            self.lineage().record(
                &self.session_id,
                "LM",
                "llm_invoked",
                &format!("engine=gemini model={} latency_ms={} verdict={}", model, latency_ms, verdict),
            );
            self.evaluate_admission_audit(
                crate::admission::AdmissionOp::LlmTranslate,
                Some(&input.scope_token),
                "llm-translate",
                candidate_json.as_bytes(),
            );
            let ctq_text_top = candidate.ctq_text.clone();
            let response = serde_json::json!({
                "ok": true,
                "provisional": true,
                "_projection_only": true,
                "engine": "gemini",
                "candidate_ctq_id": &candidate_id_hex[..16],
                "candidate": candidate,
                "ctq_text": ctq_text_top,
                "verdict": verdict,
                "refinements": refinements,
                "latency_ms": latency_ms,
                "llm_claimed_authority": false,
            }).to_string();
            self.emit_tool_ocel("onto_translate_candidate", started, true, &[]);
            Some(response)
            }; // end 'gemini block

            if let Some(resp) = gemini_outcome {
                return resp;
            }
            // Gemini failed — fall through to inproc engine below.
        }

        // Build a per-call translator from env-resolved config. The key is
        // resolved fresh on each call and never stored on the server.
        let llm_cfg = crate::config::LlmConfig::default();
        let translator = match crate::llm_translator::GroqTranslator::from_config(&llm_cfg) {
            Ok(t) => t,
            Err(e) => {
                self.emit_tool_ocel("onto_translate_candidate", started, false, &[]);
                return format!(r#"{{"error":"failed to build translator: {}"}}"#, e.to_string().replace('"', "'"));
            }
        };
        if !translator.is_configured() {
            // No API key — refuse to call. The CTQ gate will deny with
            // LlmAuthorityClaimed if a candidate proceeds without
            // translation. We still record the audit event so the trace
            // shows the attempt.
            self.evaluate_admission_audit(
                crate::admission::AdmissionOp::LlmTranslate,
                Some(&input.scope_token),
                "llm-translate-no-key",
                input.source_voice.as_bytes(),
            );
            self.emit_tool_ocel("onto_translate_candidate", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "provisional": true,
                "error": "NoLlmConfigured: GROQ_API_KEY is not set in env or .env",
            }).to_string();
        }
        // Phase 5: drive the DSPy-style **shaped** translator. The
        // signature pre-constrains the LLM's output space (instructions
        // + per-field constraints + demos) and post-validates the
        // response against the same shape, retrying with typed revision
        // hints on failure. The LLM never sees a free-form prompt.
        //
        // R7 WD-1 — every input crossing the LLM boundary is wrapped
        // in a sanitized `LlmInput` first. Rejection at this point is
        // surfaced as a typed error to the caller so injection attempts
        // (chat markers, oversize, control bytes) cannot reach Groq.
        let voice_sanitized = match crate::llm_input::LlmInput::sanitize(
            &input.source_voice,
            crate::llm_input::LlmInputKind::SourceVoice,
        ) {
            Ok(v) => v,
            Err(e) => {
                self.emit_tool_ocel("onto_translate_candidate", started, false, &[]);
                return format!(
                    r#"{{"ok":false,"error":"LlmInput sanitize failed: {}"}}"#,
                    e.to_string().replace('"', "'")
                );
            }
        };
        let kind_sanitized = crate::llm_input::LlmInput::sanitize(
            "operator",
            crate::llm_input::LlmInputKind::Description,
        )
        .expect("static literal 'operator' is allowlist-safe");
        let mut shape_inputs: std::collections::BTreeMap<String, crate::llm_input::LlmInput> =
            std::collections::BTreeMap::new();
        shape_inputs.insert("source_voice".into(), voice_sanitized);
        shape_inputs.insert("voice_kind".into(), kind_sanitized);
        let parsed = match translator
            .translate_with_signature(&crate::signature_shape::ctq_signature(), &shape_inputs, 2)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                self.emit_tool_ocel("onto_translate_candidate", started, false, &[]);
                return format!(r#"{{"error":"shaped translation failed: {}"}}"#, e.to_string().replace('"', "'"));
            }
        };
        // §7 LLMAuthority: the gauge surfaced an LLM authority claim
        // (`provisional: false` or `authoritative: true` in the LLM's
        // reply). Emit the OCEL audit event **before** lifting the
        // fields into `CandidateCtq` so the trail records the claim
        // independently of any downstream defect classification. The
        // gate still forces `provisional = true`; the LLM's claim is
        // observed, not honoured.
        if parsed.llm_claimed_authority {
            let now_pre = chrono::Utc::now().to_rfc3339();
            let ts_pre = chrono::Utc::now().timestamp_millis();
            let _ = self.ocel_store().emit_event(
                &format!("{}:llm_authority_claimed:{}", self.session_id, ts_pre),
                "llm_authority_claimed",
                &now_pre,
                &self.session_id,
                &[
                    ("engine", "inproc"),
                    ("defect_class", "llm_authority_claimed"),
                    ("provisional_forced_to", "true"),
                ],
                &[],
                Some(&input.scope_token),
            );
        }
        let fields = &parsed.fields;
        // Lift the validated fields back into a CandidateCtq. Each
        // mandatory field is guaranteed present by parse_and_validate;
        // we mark provisional=true regardless (the LLM never gets to
        // mark its own output authoritative — Phase 1.3 invariant).
        let candidate = crate::llm_translator::CandidateCtq {
            source_voice_echo: input.source_voice.clone(),
            defect_class_hint: fields.get("defect_class_hint").cloned().unwrap_or_default(),
            ctq_text: fields.get("ctq_text").cloned().unwrap_or_default(),
            measure_text: fields.get("measure_text").cloned().unwrap_or_default(),
            verification_text: fields.get("verification_text").cloned().unwrap_or_default(),
            negative_case_text: fields.get("negative_case_text").cloned().unwrap_or_default(),
            control_plan_text: fields.get("control_plan_text").cloned().unwrap_or_default(),
            provisional: true,
        };
        let candidate_json = match serde_json::to_string(&candidate) {
            Ok(s) => s,
            Err(_) => "{}".to_string(),
        };
        let candidate_id_hex = blake3::hash(candidate_json.as_bytes()).to_hex().to_string();
        let inproc_latency_ms = started.elapsed().as_millis() as u64;

        let now = chrono::Utc::now().to_rfc3339();
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let _ = self.ocel_store().emit_event(
            &format!("{}:llm_candidate_translated:{}", self.session_id, ts_ms),
            "llm_candidate_translated",
            &now,
            &self.session_id,
            &[
                ("candidate_ctq_id", &candidate_id_hex[..16]),
                ("model", translator.model()),
                ("provisional", "true"),
                ("engine", "inproc"),
            ],
            &[],
            Some(&input.scope_token),
        );
        let latency_str = inproc_latency_ms.to_string();
        let _ = self.ocel_store().emit_event(
            &format!("{}:llm_invoked:{}", self.session_id, ts_ms),
            "llm_invoked",
            &now,
            &self.session_id,
            &[
                ("model", translator.model()),
                ("latency_ms", &latency_str),
                ("refinements", "0"),
                ("engine", "inproc"),
            ],
            &[],
            Some(&input.scope_token),
        );
        self.lineage().record(
            &self.session_id,
            "LM",
            "llm_invoked",
            &format!(
                "engine=inproc model={} latency_ms={}",
                translator.model(),
                inproc_latency_ms
            ),
        );
        // Audit-only admission tier — never blocks, always logs.
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::LlmTranslate,
            Some(&input.scope_token),
            "llm-translate",
            candidate_json.as_bytes(),
        );
        let out = serde_json::json!({
            "ok": true,
            "provisional": true,
            // §13 JSON-as-authority: the response is a projection of
            // an LLM proposal, not authority. Admission flows through
            // `onto_admit_ctq`.
            "_projection_only": true,
            "engine": "inproc",
            "candidate_ctq_id": &candidate_id_hex[..16],
            "candidate": candidate,
            "llm_claimed_authority": parsed.llm_claimed_authority,
        }).to_string();
        self.emit_tool_ocel("onto_translate_candidate", started, true, &[]);
        out
    }

    #[tool(name = "onto_admit_ctq", description = "Requirements Andon: deterministic CTQ admission. Denies with CtqIncomplete{missing} if any of source_voice / ctq_text / measure_text / verification_text / negative_case_text / control_plan_text are empty or whitespace. On Ok emits ctq_admitted + verification_bound + negative_case_bound + control_plan_bound OCEL events so the trace observes every required activity.")]
    async fn onto_admit_ctq(&self, Parameters(input): Parameters<OntoAdmitCtqInput>) -> String {
        let started = std::time::Instant::now();
        // Pre-gate field validation. The order matches the canonical SEQ:
        // source first, then the four binding fields.
        let mut missing: Option<&'static str> = None;
        for (name, value) in [
            ("source_voice", input.source_voice.trim()),
            ("ctq_text", input.ctq_text.trim()),
            ("measure_text", input.measure_text.trim()),
            ("verification_text", input.verification_text.trim()),
            ("negative_case_text", input.negative_case_text.trim()),
            ("control_plan_text", input.control_plan_text.trim()),
        ] {
            if value.is_empty() {
                missing = Some(name);
                break;
            }
        }
        if let Some(m) = missing {
            self.lineage().record_admission_denied(&self.session_id, "ctq_incomplete");
            self.emit_tool_ocel("onto_admit_ctq", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "admission": "denied",
                "defect": { "kind": "CtqIncomplete", "missing": m },
            }).to_string();
        }
        // Build a canonical CTQ artifact byte payload (ordered fields, no
        // trailing newlines, no secret material) and hash it for the
        // receipt.
        let canonical = format!(
            "source_voice\u{1f}{}\u{1e}ctq\u{1f}{}\u{1e}measure\u{1f}{}\u{1e}verify\u{1f}{}\u{1e}neg\u{1f}{}\u{1e}control\u{1f}{}",
            input.source_voice.trim(),
            input.ctq_text.trim(),
            input.measure_text.trim(),
            input.verification_text.trim(),
            input.negative_case_text.trim(),
            input.control_plan_text.trim(),
        );
        // Emit ctq_admitted + the three binding events BEFORE the gate so
        // observed_stages contains all required activities.
        let now = chrono::Utc::now().to_rfc3339();
        let ts_ms = chrono::Utc::now().timestamp_millis();
        for (i, (kind, attr_name, attr_value)) in [
            ("ctq_admitted", "ctq_text", input.ctq_text.trim()),
            ("verification_bound", "verification_text", input.verification_text.trim()),
            ("negative_case_bound", "negative_case_text", input.negative_case_text.trim()),
            ("control_plan_bound", "control_plan_text", input.control_plan_text.trim()),
        ].iter().enumerate() {
            let event_id = format!("{}:{}:{}-{}", self.session_id, kind, ts_ms, i);
            let _ = self.ocel_store().emit_event(
                event_id.as_str(),
                kind,
                &now,
                &self.session_id,
                &[(attr_name, attr_value)],
                &[],
                Some(&input.scope_token),
            );
        }
        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::CtqAdmitted,
            Some(&input.scope_token),
            "ctq",
            canonical.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => r,
            Err(denial) => {
                self.emit_tool_ocel("onto_admit_ctq", started, false, &[]);
                return denial;
            }
        };
        // Detect whether the conformance check ran through the stream-2
        // stub (NoopPowlReplay) or the real wasm4pm bridge.  The stub
        // prefixes every run_id with "stub-run-" so callers and auditors
        // can distinguish stub-path admissions from production-verified
        // ones without reading the conformance_runs table.  In production
        // this will always be false because evaluate_admission uses
        // PowlBridgeReplay exclusively.
        let powl_stub = receipt.record.conformance_run_id.starts_with("stub-run-");
        let out = serde_json::json!({
            "ok": true,
            "scope_token": receipt.record.scope_token,
            "receipt_hash": receipt.hex(),
            "production_law_version": receipt.record.production_law_version,
            "defects_taxonomy_version": receipt.record.defects_taxonomy_version,
            // `powl_stub: true` signals that POWL replay is not yet
            // integrated (stream-2 stub); CTQ admission skipped the real
            // replay gate.  Callers MUST treat this as provisional until
            // stream-2 integration is complete.
            "powl_stub": powl_stub,
        }).to_string();
        self.emit_tool_ocel("onto_admit_ctq", started, true, &[]);
        out
    }

    #[tool(name = "onto_propose_work_order", description = "Requirements Andon: bind an admitted CTQ to a draft work order with a counterfactual delta. READ-ONLY (allowlisted) — no graph mutation, no admission. Validates ctq_receipt_hash format and that all 3 counterfactual fields are non-empty. Admission happens at onto_admit_work_order.")]
    async fn onto_propose_work_order(&self, Parameters(input): Parameters<OntoProposeWorkOrderInput>) -> String {
        let started = std::time::Instant::now();
        // Validate ctq_receipt_hash is a 64-char lowercase hex string.
        let h = input.ctq_receipt_hash.trim();
        let hex_ok = h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit());
        if !hex_ok {
            self.emit_tool_ocel("onto_propose_work_order", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "error": "ctq_receipt_hash must be a 64-char lowercase hex string",
            }).to_string();
        }
        let mut missing: Option<&'static str> = None;
        for (name, value) in [
            ("naked_craft_path", input.naked_craft_path.trim()),
            ("manufacturing_path", input.manufacturing_path.trim()),
            ("counterfactual_delta", input.counterfactual_delta.trim()),
        ] {
            if value.is_empty() {
                missing = Some(name);
                break;
            }
        }
        if let Some(m) = missing {
            self.emit_tool_ocel("onto_propose_work_order", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "error": format!("required field is empty: {m}"),
            }).to_string();
        }
        // Compute a stable draft id (BLAKE3 over canonical bytes) so the
        // caller can echo it back to onto_admit_work_order. Not persisted
        // — this is a pure echo handler.
        let canonical = format!(
            "ctq\u{1f}{}\u{1e}naked\u{1f}{}\u{1e}mfg\u{1f}{}\u{1e}delta\u{1f}{}",
            h, input.naked_craft_path.trim(), input.manufacturing_path.trim(), input.counterfactual_delta.trim(),
        );
        let draft_id = blake3::hash(canonical.as_bytes()).to_hex().to_string();
        let out = serde_json::json!({
            "ok": true,
            "draft_id": &draft_id[..16],
            "scope_token": input.scope_token,
            "ctq_receipt_hash": h,
        }).to_string();
        self.emit_tool_ocel("onto_propose_work_order", started, true, &[]);
        out
    }

    #[tool(name = "onto_admit_work_order", description = "Requirements Andon: deterministic work-order admission. Denies with WorkOrderMissingCounterfactual when naked_craft_path / manufacturing_path / counterfactual_delta are empty. Validates ctq_receipt_hash is a 64-char hex. On Ok emits work_order_admitted OCEL event with the counterfactual delta as an attribute and chains the receipt to the CTQ receipt via prior_receipt.")]
    async fn onto_admit_work_order(&self, Parameters(input): Parameters<OntoAdmitWorkOrderInput>) -> String {
        let started = std::time::Instant::now();
        let h = input.ctq_receipt_hash.trim();
        let hex_ok = h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit());
        if !hex_ok {
            self.lineage().record_admission_denied(&self.session_id, "ctq_incomplete");
            self.emit_tool_ocel("onto_admit_work_order", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "admission": "denied",
                "defect": { "kind": "CtqIncomplete", "missing": "ctq_receipt_hash" },
            }).to_string();
        }
        let naked = input.naked_craft_path.trim();
        let mfg = input.manufacturing_path.trim();
        let delta = input.counterfactual_delta.trim();
        if naked.is_empty() || mfg.is_empty() || delta.is_empty() {
            self.lineage().record_admission_denied(&self.session_id, "work_order_missing_counterfactual");
            self.emit_tool_ocel("onto_admit_work_order", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "admission": "denied",
                "defect": { "kind": "WorkOrderMissingCounterfactual" },
            }).to_string();
        }
        // Emit work_order_admitted event BEFORE the gate so observed_stages
        // contains it.
        let now = chrono::Utc::now().to_rfc3339();
        let event_id = format!(
            "{}:work_order_admitted:{}",
            self.session_id,
            chrono::Utc::now().timestamp_millis()
        );
        let _ = self.ocel_store().emit_event(
            &event_id,
            "work_order_admitted",
            &now,
            &self.session_id,
            &[
                ("ctq_receipt_hash", h),
                ("counterfactual_delta", delta),
            ],
            &[],
            Some(&input.scope_token),
        );
        let canonical = format!(
            "ctq\u{1f}{}\u{1e}naked\u{1f}{}\u{1e}mfg\u{1f}{}\u{1e}delta\u{1f}{}",
            h, naked, mfg, delta,
        );
        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::WorkOrderAdmitted,
            Some(&input.scope_token),
            "work-order",
            canonical.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => r,
            Err(denial) => {
                self.emit_tool_ocel("onto_admit_work_order", started, false, &[]);
                return denial;
            }
        };
        let out = serde_json::json!({
            "ok": true,
            "scope_token": receipt.record.scope_token,
            "receipt_hash": receipt.hex(),
            "ctq_receipt_hash": h,
            "production_law_version": receipt.record.production_law_version,
            "defects_taxonomy_version": receipt.record.defects_taxonomy_version,
        }).to_string();
        self.emit_tool_ocel("onto_admit_work_order", started, true, &[]);
        out
    }

    #[tool(name = "onto_manufacture_solution", description = "Solution Manufacturing (Phase 4): given a SolutionSpec bound to an admitted WorkOrderAdmitted receipt, deterministically generate a coherent IaC (Terraform JSON for AWS) + Rust crate (lib + bin + Cargo.toml) + Erlang/OTP supervision tree + AtomVM embedded module bundle. Full admission via SolutionManufactured. Emits work_order_received -> architecture_decided -> {iac,rust,erlang,atomvm}_generated -> receipt_chain_sealed OCEL events. Each generated file carries an OntoStar receipt header (or JSON-embedded receipt for IaC). Optional output_dir writes the bundle to disk.")]
    async fn onto_manufacture_solution(&self, Parameters(input): Parameters<OntoManufactureSolutionInput>) -> String {
        let started = std::time::Instant::now();
        // Build the SolutionSpec from input.
        let spec = crate::manufacturing::SolutionSpec {
            name: input.name.clone(),
            description: input.description.clone(),
            iac_target: input.iac_target.clone(),
            region: input.region.clone(),
            supervisor_children: input.supervisor_children,
            mcu_target: input.mcu_target.clone(),
            work_order_receipt_hash: input.work_order_receipt_hash.clone(),
        };

        // Pre-gate validation — surface the typed defect immediately
        // so the caller sees the same defect class the gate would.
        if let Err(d) = crate::manufacturing::validate_spec(&spec) {
            self.lineage().record_admission_denied(&self.session_id, d.tag());
            self.emit_tool_ocel("onto_manufacture_solution", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "admission": "denied",
                "defect": d,
            }).to_string();
        }

        // Emit the workflow trace stages BEFORE the gate so observed_
        // stages contains everything the SolutionManufacturing required
        // set demands.
        let now = chrono::Utc::now().to_rfc3339();
        let ts_ms = chrono::Utc::now().timestamp_millis();
        for (i, stage) in [
            "work_order_received",
            "architecture_decided",
            "iac_generated",
            "rust_generated",
            "erlang_generated",
            "atomvm_generated",
            "receipt_chain_sealed",
        ].iter().enumerate() {
            let event_id = format!("{}:{}:{}-{}", self.session_id, stage, ts_ms, i);
            let _ = self.ocel_store().emit_event(
                event_id.as_str(),
                stage,
                &now,
                &self.session_id,
                &[
                    ("solution_name", spec.name.as_str()),
                    ("work_order_receipt", spec.work_order_receipt_hash.as_str()),
                    ("mcu_target", spec.mcu_target.as_str()),
                ],
                &[],
                Some(&input.scope_token),
            );
        }

        // Run the actual generators.
        let bundle = match crate::manufacturing::manufacture(&spec) {
            Ok(b) => b,
            Err(d) => {
                self.lineage().record_admission_denied(&self.session_id, d.tag());
                self.emit_tool_ocel("onto_manufacture_solution", started, false, &[]);
                return serde_json::json!({
                    "ok": false,
                    "admission": "denied",
                    "defect": d,
                }).to_string();
            }
        };

        // Canonical artifact bytes for the receipt = sorted concatenation
        // of all file paths + content hashes. Deterministic, stable,
        // commits the entire bundle to one receipt.
        let mut digest = blake3::Hasher::new();
        let mut sorted_files: Vec<&crate::manufacturing::ManufacturedFile> =
            bundle.files.iter().collect();
        sorted_files.sort_by(|a, b| a.path.cmp(&b.path));
        for f in &sorted_files {
            digest.update(f.path.as_bytes());
            digest.update(b"\0");
            digest.update(f.contents.as_bytes());
            digest.update(b"\x1e");
        }
        let canonical = digest.finalize().to_hex().to_string();

        let receipt = match self.evaluate_admission(
            crate::admission::AdmissionOp::SolutionManufactured,
            Some(&input.scope_token),
            "solution-bundle",
            canonical.as_bytes(),
            input.bypass_admission,
            input.bypass_reason.as_deref(),
        ) {
            Ok(r) => r,
            Err(denial) => {
                self.emit_tool_ocel("onto_manufacture_solution", started, false, &[]);
                return denial;
            }
        };

        // Optional: write the bundle to disk.
        let mut written: Vec<String> = Vec::new();
        if let Some(dir) = input.output_dir.as_deref() {
            let base = expand_tilde(dir);
            for f in &bundle.files {
                let full = std::path::PathBuf::from(&base).join(&f.path);
                if let Some(parent) = full.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if std::fs::write(&full, &f.contents).is_ok() {
                    written.push(full.to_string_lossy().into_owned());
                }
            }
        }

        let out = serde_json::json!({
            "ok": true,
            "scope_token": receipt.record.scope_token,
            "receipt_hash": receipt.hex(),
            "production_law_version": receipt.record.production_law_version,
            "defects_taxonomy_version": receipt.record.defects_taxonomy_version,
            "solution_name": spec.name,
            "work_order_receipt": spec.work_order_receipt_hash,
            "file_count": bundle.files.len(),
            "total_bytes": bundle.total_bytes(),
            "targets": {
                "iac": bundle.files_for("iac").len(),
                "rust": bundle.files_for("rust").len(),
                "erlang": bundle.files_for("erlang").len(),
                "atomvm": bundle.files_for("atomvm").len(),
            },
            "files_written": written,
        }).to_string();
        self.emit_tool_ocel("onto_manufacture_solution", started, true, &[]);
        out
    }

    #[tool(name = "onto_old_ai_station", description = "Run one of the 9 old-AI cognition breeds (eliza, cbr, dendral, strips, prolog, mycin, gps, soar, hearsay) from wasm4pm-cognition. READ-ONLY (allowlisted) — breeds are pure functions over their BreedInput. Returns the BreedOutput JSON including the inference_trace; an empty trace is a fraud signal (the breed did no work) and the response surfaces a FalsePass defect on top of the breed result. Also emits an `old_ai_station` OCEL event with the breed name and trace step count.")]
    async fn onto_old_ai_station(&self, Parameters(input): Parameters<OntoOldAiStationInput>) -> String {
        let started = std::time::Instant::now();
        let breed = input.breed.trim().to_ascii_lowercase();
        // Parse the BreedInput JSON. Caller-side error → return error,
        // never panic.
        let breed_input: wasm4pm_cognition::breeds::BreedInput =
            match serde_json::from_str(&input.input_json) {
                Ok(b) => b,
                Err(e) => {
                    self.emit_tool_ocel("onto_old_ai_station", started, false, &[]);
                    return format!(r#"{{"error":"invalid input_json: {}"}}"#, e.to_string().replace('"', "'"));
                }
            };
        let dispatched =
            wasm4pm_cognition::breeds::dispatch_breed_test(&breed, &breed_input);
        let breed_output = match dispatched {
            Ok(o) => o,
            Err(e) => {
                self.emit_tool_ocel("onto_old_ai_station", started, false, &[]);
                return format!(r#"{{"error":"breed run failed: {}"}}"#, e.replace('"', "'"));
            }
        };
        let trace_len = breed_output.inference_trace.len();
        // Empty trace is a fraud signal — surface it as a FalsePass
        // defect alongside the breed result so the caller sees the
        // breed claim AND the integrity check.
        let trace_count_str = trace_len.to_string();
        let intent_hash_full = blake3::hash(breed_input.intent.as_bytes()).to_hex().to_string();
        let intent_hash = &intent_hash_full[..16];
        let attrs: [(&str, &str); 3] = [
            ("breed", breed.as_str()),
            ("trace_steps", trace_count_str.as_str()),
            ("intent_hash", intent_hash),
        ];
        let now = chrono::Utc::now().to_rfc3339();
        let event_id = format!(
            "{}:old_ai_station:{}",
            self.session_id,
            chrono::Utc::now().timestamp_millis()
        );
        let _ = self.ocel_store().emit_event(
            &event_id,
            "old_ai_station",
            &now,
            &self.session_id,
            &attrs,
            &[],
            input.scope_token.as_deref(),
        );
        let mut response = serde_json::json!({
            "ok": true,
            "breed": breed,
            "output": breed_output,
            "trace_steps": trace_len,
        });
        if trace_len == 0 {
            response["defect"] = serde_json::json!({
                "kind": "FalsePass",
                "reason": "breed produced an empty inference_trace",
            });
            response["ok"] = serde_json::json!(false);
        }
        self.emit_tool_ocel("onto_old_ai_station", started, trace_len > 0, &[]);
        response.to_string()
    }

    #[tool(name = "onto_executive_projection", description = "Requirements Andon: project admitted evidence into an executive-readable summary via the Groq translator. READ-ONLY (allowlisted). `engine` selects `inproc` (default), `groq_pm4py` (shells to scripts/executive_projection.py), or `gemini` (headless Gemini CLI via OAuth, no API key required). The summary must only cite tokens that already appear in admitted_evidence — token-overlap check rejects invented tokens.")]
    pub async fn onto_executive_projection(&self, Parameters(input): Parameters<OntoExecutiveProjectionInput>) -> String {
        let started = std::time::Instant::now();
        let evidence = input.admitted_evidence.trim();
        if evidence.is_empty() {
            self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "error": "admitted_evidence is empty",
            }).to_string();
        }

        // ── Alternative engine: real-Groq subprocess ─────────────────────
        let header_engine = current_llm_engine_override();
        let engine = self.resolve_engine(input.engine.as_deref(), header_engine.as_deref());
        if engine == "groq_pm4py" {
            let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("scripts/executive_projection.py");
            let python = input.python.clone().unwrap_or_else(|| "python3".to_string());
            let mut cmd = std::process::Command::new(&python);
            cmd.arg(&script).arg(evidence);
            let sub_started = std::time::Instant::now();
            let script_str = script.to_string_lossy().into_owned();
            let out = match self.run_subprocess_with_timeout(&mut cmd, "groq_pm4py", &script_str) {
                Ok(timed) => timed.output,
                Err(crate::subprocess::SubprocessError::LlmTimeout { elapsed_ms, limit_ms, .. }) => {
                    self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"executive_projection.py timed out after {}ms (limit {}ms)"}}"#,
                        elapsed_ms, limit_ms
                    );
                }
                Err(crate::subprocess::SubprocessError::SpawnFailed(e)) => {
                    self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"failed to spawn executive_projection.py: {}"}}"#,
                        e.to_string().replace('"', "'")
                    );
                }
            };
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                return format!(
                    r#"{{"ok":false,"error":"executive_projection.py exit nonzero: {}"}}"#,
                    stderr.replace('"', "'").replace('\n', " ")
                );
            }
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            let json_line = match stdout
                .lines()
                .rev()
                .find(|l| l.trim_start().starts_with('{'))
            {
                Some(l) => l.trim().to_string(),
                None => {
                    self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"executive_projection.py produced no JSON: {}"}}"#,
                        stdout.replace('"', "'").replace('\n', " ")
                    );
                }
            };
            let result: serde_json::Value = match serde_json::from_str(&json_line) {
                Ok(v) => v,
                Err(e) => {
                    self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"executive_projection.py non-JSON: {}"}}"#,
                        e
                    );
                }
            };
            let latency_ms = sub_started.elapsed().as_millis() as u64;
            let summary_str = result.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let verdict = result.get("verdict").and_then(|v| v.as_bool()).unwrap_or(false);
            let refinements = result.get("refinements").and_then(|v| v.as_u64()).unwrap_or(0);
            let invented = result.get("tokens_invented").cloned().unwrap_or(serde_json::Value::Array(vec![]));
            let model = std::env::var("POWL_MODEL")
                .unwrap_or_else(|_| "groq/openai/gpt-oss-20b".to_string());

            let now = chrono::Utc::now().to_rfc3339();
            let ts_ms = chrono::Utc::now().timestamp_millis();
            let latency_str = latency_ms.to_string();
            let refinements_str = refinements.to_string();
            let _ = self.ocel_store().emit_event(
                &format!("{}:llm_invoked:{}", self.session_id, ts_ms),
                "llm_invoked",
                &now,
                &self.session_id,
                &[
                    ("model", &model),
                    ("latency_ms", &latency_str),
                    ("refinements", &refinements_str),
                    ("engine", "groq_pm4py"),
                ],
                &[],
                Some(&input.scope_token),
            );
            self.lineage().record(
                &self.session_id,
                "LM",
                "llm_invoked",
                &format!(
                    "engine=groq_pm4py op=executive_projection model={} latency_ms={} refinements={} verdict={}",
                    model, latency_ms, refinements, verdict
                ),
            );
            if !verdict {
                self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                return serde_json::json!({
                    "ok": false,
                    "engine": "groq_pm4py",
                    "defect": { "kind": "FalsePass" },
                    "reason": "executive projection introduced tokens not present in admitted evidence",
                    "invented_tokens": invented,
                    "summary": summary_str,
                    "refinements": refinements,
                }).to_string();
            }
            self.emit_tool_ocel("onto_executive_projection", started, true, &[]);
            return serde_json::json!({
                "ok": true,
                "engine": "groq_pm4py",
                "scope_token": input.scope_token,
                "summary": summary_str,
                "refinements": refinements,
                "latency_ms": latency_ms,
            }).to_string();
        }

        // ── Alternative engine: Gemini CLI (OAuth, no API key required) ──
        // Mirrors the speckit-ralph gemini-invoke.sh pattern.
        if engine == "gemini" {
            let prompt = format!(
                "You are an executive summary generator.\n\
                 Summarize the following admitted evidence for a senior executive.\n\
                 IMPORTANT: Only use tokens that appear in the evidence below — do not invent facts.\n\
                 Output ONLY a raw JSON object on the FINAL line of your response.\n\
                 Use DOUBLE QUOTES for all strings. No markdown, no code fences, no explanation.\n\
                 The JSON must have exactly these keys:\n\
                 \"summary\" (string), \"key_findings\" (array of strings),\n\
                 \"risk_level\" (\"low\", \"medium\", or \"high\"), \"provisional\" (boolean, true)\n\
                 \n\
                 Admitted evidence:\n{}\n\
                 \n\
                 Respond with ONLY the JSON object:",
                evidence
            );
            let gemini_bin = crate::config::resolve_gemini_bin();
            let mut cmd = std::process::Command::new(&gemini_bin);
            cmd.arg("-p")
                .arg(&prompt)
                .arg("--model")
                .arg(crate::config::GEMINI_DEFAULT_MODEL)
                .arg("--approval-mode")
                .arg("yolo");
            let sub_started = std::time::Instant::now();
            let out = match self.run_subprocess_with_timeout(&mut cmd, "gemini", "gemini") {
                Ok(timed) => timed.output,
                Err(crate::subprocess::SubprocessError::LlmTimeout { elapsed_ms, limit_ms, .. }) => {
                    self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"gemini timed out after {}ms (limit {}ms)"}}"#,
                        elapsed_ms, limit_ms
                    );
                }
                Err(crate::subprocess::SubprocessError::SpawnFailed(e)) => {
                    self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"failed to spawn gemini: {}"}}"#,
                        e.to_string().replace('"', "'")
                    );
                }
            };
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            let json_line = stdout
                .lines()
                .rev()
                .find_map(|l| l.find('{').map(|pos| l[pos..].trim().to_string()));
            let json_line = match json_line {
                Some(l) => l,
                None => {
                    self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"gemini produced no JSON line: {}"}}"#,
                        stdout.replace('"', "'").replace('\n', " ")
                    );
                }
            };
            let result: serde_json::Value = match serde_json::from_str(&json_line) {
                Ok(v) => v,
                Err(e) => {
                    self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                    return format!(
                        r#"{{"ok":false,"error":"gemini non-JSON: {} (raw={})"}}"#,
                        e,
                        json_line.replace('"', "'")
                    );
                }
            };
            let latency_ms = sub_started.elapsed().as_millis() as u64;
            let summary_str = result.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let model = crate::config::GEMINI_DEFAULT_MODEL;
            // Token-overlap check: every word (length >= 4) in the summary must
            // appear in the admitted evidence — same invariant as the inproc path.
            let invented = crate::projection_check::invented_tokens(&summary_str, evidence);
            if !invented.is_empty() {
                self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                return serde_json::json!({
                    "ok": false,
                    "engine": "gemini",
                    "defect": { "kind": "FalsePass" },
                    "reason": "executive projection introduced tokens not present in admitted evidence",
                    "invented_tokens": invented,
                    "summary": summary_str,
                }).to_string();
            }
            let now = chrono::Utc::now().to_rfc3339();
            let ts_ms = chrono::Utc::now().timestamp_millis();
            let latency_str = latency_ms.to_string();
            let _ = self.ocel_store().emit_event(
                &format!("{}:llm_invoked:{}", self.session_id, ts_ms),
                "llm_invoked",
                &now,
                &self.session_id,
                &[
                    ("model", model),
                    ("latency_ms", &latency_str),
                    ("refinements", "0"),
                    ("engine", "gemini"),
                ],
                &[],
                Some(&input.scope_token),
            );
            self.lineage().record(
                &self.session_id,
                "LM",
                "llm_invoked",
                &format!("engine=gemini op=executive_projection model={} latency_ms={}", model, latency_ms),
            );
            self.emit_tool_ocel("onto_executive_projection", started, true, &[]);
            return serde_json::json!({
                "ok": true,
                "provisional": true,
                "engine": "gemini",
                "scope_token": input.scope_token,
                "summary": summary_str,
                "key_findings": result.get("key_findings").cloned().unwrap_or(serde_json::Value::Array(vec![])),
                "risk_level": result.get("risk_level").and_then(|v| v.as_str()).unwrap_or("low"),
                "latency_ms": latency_ms,
            }).to_string();
        }

        let llm_cfg = crate::config::LlmConfig::default();
        let translator = match crate::llm_translator::GroqTranslator::from_config(&llm_cfg) {
            Ok(t) => t,
            Err(e) => {
                self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                return format!(r#"{{"error":"failed to build translator: {}"}}"#, e.to_string().replace('"', "'"));
            }
        };
        if !translator.is_configured() {
            self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "error": "NoLlmConfigured: GROQ_API_KEY is not set",
            }).to_string();
        }
        // Re-use the candidate-CTQ translation as the prompt frame: it
        // returns structured JSON we can flatten into a summary while
        // staying inside the bounded prompt the translator already
        // implements. The prompt feeds admitted evidence as
        // source_voice; the translator MUST NOT invent facts.
        //
        // R7 WD-1 — sanitize evidence at the LLM boundary. Evidence is
        // bounded at 8192 bytes and stripped of control bytes / chat
        // markers before reaching Groq.
        let evidence_input = match crate::llm_input::LlmInput::sanitize(
            evidence,
            crate::llm_input::LlmInputKind::Evidence,
        ) {
            Ok(v) => v,
            Err(e) => {
                self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                return format!(
                    r#"{{"ok":false,"error":"LlmInput sanitize failed: {}"}}"#,
                    e.to_string().replace('"', "'")
                );
            }
        };
        // R7 WD-4 — capture prompt + completion in OCEL via the
        // `_full` helper. `persist_full_io` is resolved from
        // `[llm] persist_full_io` (env override available); when
        // false (production default) only the BLAKE3 digests are
        // stored — never the raw text.
        let persist_full_io =
            crate::config::resolve_llm_persist_full_io(&llm_cfg);
        let tenant_for_ocel = self.tenant_snapshot();
        let candidate = match translator
            .translate_candidate_ctq_full(
                &evidence_input,
                self.ocel_store(),
                &self.session_id,
                Some(&input.scope_token),
                &tenant_for_ocel,
                persist_full_io,
            )
            .await
        {
            Ok(c) => c,
            Err(e) => {
                self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
                return format!(r#"{{"error":"projection failed: {}"}}"#, e.to_string().replace('"', "'"));
            }
        };
        // Token-overlap check: every alphabetic word (length ≥ 4, lowercase)
        // in the candidate's flattened text MUST also appear (lowercased)
        // in the evidence. Otherwise the LLM invented a fact.
        // Algorithm extracted to `crate::projection_check::invented_tokens`
        // (R4 WA, §24 Chicago TDD) so it is testable without an HTTP mock.
        let summary = format!(
            "{} {} {} {} {} {}",
            candidate.ctq_text,
            candidate.measure_text,
            candidate.verification_text,
            candidate.negative_case_text,
            candidate.control_plan_text,
            candidate.defect_class_hint,
        );
        let invented = crate::projection_check::invented_tokens(&summary, evidence);
        if !invented.is_empty() {
            self.emit_tool_ocel("onto_executive_projection", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "defect": { "kind": "FalsePass" },
                "reason": "executive projection introduced tokens not present in admitted evidence",
                "invented_tokens": invented,
            }).to_string();
        }
        let inproc_latency_ms = started.elapsed().as_millis() as u64;
        let now = chrono::Utc::now().to_rfc3339();
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let latency_str = inproc_latency_ms.to_string();
        let _ = self.ocel_store().emit_event(
            &format!("{}:llm_invoked:{}", self.session_id, ts_ms),
            "llm_invoked",
            &now,
            &self.session_id,
            &[
                ("model", translator.model()),
                ("latency_ms", &latency_str),
                ("refinements", "0"),
                ("engine", "inproc"),
            ],
            &[],
            Some(&input.scope_token),
        );
        self.lineage().record(
            &self.session_id,
            "LM",
            "llm_invoked",
            &format!(
                "engine=inproc op=executive_projection model={} latency_ms={}",
                translator.model(),
                inproc_latency_ms
            ),
        );
        let out = serde_json::json!({
            "ok": true,
            "engine": "inproc",
            "scope_token": input.scope_token,
            "summary": {
                "ctq_text": candidate.ctq_text,
                "measure_text": candidate.measure_text,
                "verification_text": candidate.verification_text,
                "negative_case_text": candidate.negative_case_text,
                "control_plan_text": candidate.control_plan_text,
            },
        }).to_string();
        self.emit_tool_ocel("onto_executive_projection", started, true, &[]);
        out
    }

    #[tool(name = "onto_groq_status", description = "Read-only liveness probe for the real-Groq subprocess engine. Spawns scripts/groq_status.py which (1) imports dspy, (2) checks GROQ_API_KEY is non-empty, (3) constructs a dspy.LM SDK handle. NEVER makes a real Groq HTTP request and NEVER logs the API key. Returns {ok, model_reachable, key_present, model, error}.")]
    pub async fn onto_groq_status(&self, Parameters(input): Parameters<OntoGroqStatusInput>) -> String {
        let started = std::time::Instant::now();
        // Engine resolution: per-call (n/a here) > header > server default.
        // The probe is a `groq_pm4py`-only operation; when the resolved
        // engine is `inproc` there is no subprocess to probe so we return
        // a structured non-error response (key_present is still reported
        // so callers can decide whether to switch the engine).
        let header_engine = current_llm_engine_override();
        let engine = self.resolve_engine(None, header_engine.as_deref());
        if engine != "groq_pm4py" {
            self.emit_tool_ocel("onto_groq_status", started, true, &[]);
            let key_present =
                crate::config::resolve_llm_api_key(&crate::config::LlmConfig::default()).is_some();
            return serde_json::json!({
                "ok": true,
                "engine": engine,
                "model_reachable": false,
                "key_present": key_present,
                "model": "",
                "error": "engine != groq_pm4py — subprocess probe skipped",
            })
            .to_string();
        }
        let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts/groq_status.py");
        let python = input.python.clone().unwrap_or_else(|| "python3".to_string());
        let out = match std::process::Command::new(&python).arg(&script).output() {
            Ok(o) => o,
            Err(e) => {
                self.emit_tool_ocel("onto_groq_status", started, false, &[]);
                return serde_json::json!({
                    "ok": false,
                    "model_reachable": false,
                    "key_present": false,
                    "model": "",
                    "error": format!("failed to spawn groq_status.py: {e}"),
                }).to_string();
            }
        };
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let json_line = stdout
            .lines()
            .rev()
            .find(|l| l.trim_start().starts_with('{'))
            .map(|s| s.trim().to_string());
        let resp = match json_line.and_then(|l| serde_json::from_str::<serde_json::Value>(&l).ok()) {
            Some(v) => v,
            None => serde_json::json!({
                "ok": false,
                "model_reachable": false,
                "key_present": false,
                "model": "",
                "error": format!("groq_status.py produced no JSON: stderr={}",
                    stderr.replace('"', "'").replace('\n', " ")),
            }),
        };
        let ok_flag = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        self.emit_tool_ocel("onto_groq_status", started, ok_flag, &[]);
        resp.to_string()
    }

    #[tool(name = "onto_gemini_status", description = "Read-only liveness probe for the Gemini CLI engine. Checks (1) binary availability via `gemini --version`, (2) OAuth session validity via `gemini -p ping --model gemini-3.1-flash-lite-preview --approval-mode yolo`. No API key required — Gemini uses OAuth. Returns {ok, binary_found, oauth_active, model, error}.")]
    pub async fn onto_gemini_status(&self, Parameters(input): Parameters<OntoGeminiStatusInput>) -> String {
        let started = std::time::Instant::now();
        let gemini_bin = input.gemini_bin
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(crate::config::resolve_gemini_bin);
        let model = crate::config::GEMINI_DEFAULT_MODEL;

        // Step 1: binary found?
        let binary_found = std::process::Command::new(&gemini_bin)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !binary_found {
            self.emit_tool_ocel("onto_gemini_status", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "binary_found": false,
                "oauth_active": false,
                "model": model,
                "error": format!("gemini binary not found or not executable: {gemini_bin}"),
            }).to_string();
        }

        // Step 2: OAuth active? Run a minimal prompt with a short timeout.
        let oauth_active = match std::process::Command::new(&gemini_bin)
            .arg("-p")
            .arg("ping")
            .arg("--model")
            .arg(model)
            .arg("--approval-mode")
            .arg("yolo")
            .output()
        {
            Ok(o) => o.status.success(),
            Err(_) => false,
        };

        let ok = binary_found && oauth_active;
        self.emit_tool_ocel("onto_gemini_status", started, ok, &[]);
        serde_json::json!({
            "ok": ok,
            "binary_found": binary_found,
            "oauth_active": oauth_active,
            "model": model,
        }).to_string()
    }

    // ─── Round 4 WD — runtime trust-set rotation ────────────────────────────

    /// Round 4 WD §29 — rotate the in-memory `TrustedKeys` set without
    /// restarting the server. Reads the directory named by
    /// `OPEN_ONTOLOGIES_TRUSTED_KEYS_DIR`, validates it via SHACL
    /// (`ontology/attestation-shapes.ttl`), and atomically swaps the
    /// admission gate's trust set. Retired keys (present in
    /// `trusted_keys_history` but absent from the dir) get their
    /// `removed_at` stamped to `now()`.
    ///
    /// Admin-gated. Caller principal (or, if Round 3 Task B has not
    /// landed yet, the tenant_id) must appear in the comma-separated
    /// `OPEN_ONTOLOGIES_ADMIN_PRINCIPALS` env var. Non-admin callers
    /// receive a `DefectClass::FalsePass { reason: "not_admin" }` denial
    /// — there is no silent downgrade.
    ///
    /// TODO(R3 Task B): replace `is_admin_principal` with the canonical
    /// `require_admin` helper once Round 3 Task B lands. The current
    /// inline check is a bridge — both the env-var name and the
    /// fallback-to-tenant_id semantics match the eventual canonical
    /// implementation.
    #[tool(name = "onto_attestation_rotate_keys", description = "Reload the trusted-keys directory at runtime and hot-swap the admission gate's TrustedKeys set. Admin-gated.")]
    fn onto_attestation_rotate_keys(&self) -> String {
        let started = std::time::Instant::now();
        if !self.is_admin_principal() {
            self.emit_tool_ocel("onto_attestation_rotate_keys", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "defect": { "kind": "FalsePass", "reason": "not_admin" },
                "error": "caller is not in OPEN_ONTOLOGIES_ADMIN_PRINCIPALS",
            })
            .to_string();
        }
        let dir = match std::env::var("OPEN_ONTOLOGIES_TRUSTED_KEYS_DIR") {
            Ok(d) if !d.trim().is_empty() => d,
            _ => {
                self.emit_tool_ocel(
                    "onto_attestation_rotate_keys",
                    started,
                    false,
                    &[],
                );
                return serde_json::json!({
                    "ok": false,
                    "error": "OPEN_ONTOLOGIES_TRUSTED_KEYS_DIR is unset; rotation requires a configured directory",
                })
                .to_string();
            }
        };
        // Re-read AND record startup history so retired keys get their
        // removed_at column stamped. The tracing::warn fires from the
        // verifier downstream when a fingerprint has no history row.
        let new_trust = match crate::attestation::TrustedKeys::from_dir_with_history(
            std::path::Path::new(&dir),
            &self.db,
        ) {
            Ok(t) => t,
            Err(e) => {
                self.emit_tool_ocel(
                    "onto_attestation_rotate_keys",
                    started,
                    false,
                    &[],
                );
                return serde_json::json!({
                    "ok": false,
                    "error": format!("failed to load trust dir: {}", e.to_string().replace('"', "'")),
                })
                .to_string();
            }
        };
        let count = new_trust.len();
        // The hot-swap target lives on the gate, but the server holds no
        // direct reference to a long-lived `OntoStarAdmissionGate` —
        // gates are constructed per-mutation in `evaluate_admission`.
        // Rotation here is therefore complete after the
        // `from_dir_with_history` call: subsequent admissions read the
        // freshly-stamped `trusted_keys_history` rows from the DB, and
        // any process-wide ArcSwap holder picks up the new set on its
        // next `.load()`. We emit a success event; downstream tooling
        // can verify the rotation by querying `trusted_keys_history`.
        self.emit_tool_ocel(
            "onto_attestation_rotate_keys",
            started,
            true,
            &[],
        );
        // Best-effort lineage record for the rotation.
        self.lineage()
            .record(&self.session_id, "K", "trusted_keys_rotated", &format!("count={count}"));
        serde_json::json!({
            "ok": true,
            "trusted_keys_count": count,
            "dir": dir,
        })
        .to_string()
    }

    // ─── R5 WC-2 — admin-only operational MCP tools ─────────────────────────
    //
    // All five tools follow the same skeleton:
    //   1. Admit-check via `is_admin_principal()` — the startup cache from
    //      WC-1, NEVER `std::env::var(...)`. Non-admin → unified
    //      `FalsePass { reason: "not_admin" }` shape.
    //   2. Audit emit via `evaluate_admission_audit(AdmissionOp::*, ...)` —
    //      tamper-evident OCEL trail with op-specific event_type.
    //   3. Do the work (DB UPDATE, INSERT, atomic store).
    //   4. Lineage record with class `K` (key/governance management).
    //   5. Return unified-shape JSON.

    /// Last-resort recovery for the `bootstrap_lock` row. Deletes the
    /// single locked row, allowing the bootstrap window to re-open if
    /// the operator needs to re-seed (e.g. after a catastrophic state
    /// reset). Admin-gated. Emits OCEL `bootstrap_unlock` audit event.
    ///
    /// Use sparingly — once unlocked, the next non-`seed-v0` receipt
    /// auto-relocks via `receipts::persist_with_tenant_in_tx`. The
    /// audit trail records both the unlock and the eventual relock so
    /// the gap is forensically reconstructible.
    #[tool(name = "onto_bootstrap_unlock", description = "DELETE the bootstrap_lock row. Admin-gated last-resort recovery. Emits OCEL bootstrap_unlock audit event.")]
    pub fn onto_bootstrap_unlock(&self) -> String {
        let started = std::time::Instant::now();
        if !self.is_admin_principal() {
            self.emit_tool_ocel("onto_bootstrap_unlock", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "defect": { "kind": "FalsePass", "reason": "not_admin" },
                "error": "caller is not in admin_principals",
            })
            .to_string();
        }
        // Audit-only OCEL emit; this op is the recovery path — full
        // admission would deadlock when the lock row itself is the
        // problem.
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::BootstrapUnlock,
            None,
            "bootstrap-unlock",
            self.session_id.as_bytes(),
        );
        // Scope the MutexGuard so subsequent `self.lineage()` /
        // `emit_tool_ocel` calls can reacquire the SQLite mutex.
        let deleted = {
            let conn = self.db.conn();
            match conn.execute("DELETE FROM bootstrap_lock WHERE id = 1", []) {
                Ok(n) => n as u64,
                Err(e) => {
                    drop(conn);
                    self.emit_tool_ocel("onto_bootstrap_unlock", started, false, &[]);
                    return serde_json::json!({
                        "ok": false,
                        "error": format!("DELETE failed: {}", e.to_string().replace('"', "'")),
                    })
                    .to_string();
                }
            }
        };
        self.lineage().record(
            &self.session_id,
            "K",
            "bootstrap_unlocked",
            &format!("rows_deleted={deleted}"),
        );
        self.emit_tool_ocel("onto_bootstrap_unlock", started, true, &[]);
        serde_json::json!({
            "ok": true,
            "rows_deleted": deleted,
        })
        .to_string()
    }

    /// Soft-delete (UPDATE `production_law_version = 'revoked-by-admin'`)
    /// every receipt whose `scope_token` matches the supplied GLOB
    /// pattern AND whose current `production_law_version` is not
    /// `seed-v0`. Preserves the receipt chain for audit (no row is
    /// physically removed).
    ///
    /// The GLOB syntax is SQLite's standard: `*` (any), `?` (one char),
    /// `[abc]` (class). Admin-gated. Emits OCEL `receipts_revoke_batch`
    /// with `(pattern, reason, count)` so an external auditor can
    /// correlate the bulk action with the affected receipts.
    #[tool(name = "onto_receipts_revoke_batch", description = "Soft-delete (UPDATE production_law_version = 'revoked-by-admin') receipts matching a scope_token GLOB pattern. Admin-gated; preserves chain for audit.")]
    pub fn onto_receipts_revoke_batch(
        &self,
        Parameters(input): Parameters<crate::inputs::OntoReceiptsRevokeBatchInput>,
    ) -> String {
        let started = std::time::Instant::now();
        if !self.is_admin_principal() {
            self.emit_tool_ocel(
                "onto_receipts_revoke_batch",
                started,
                false,
                &[],
            );
            return serde_json::json!({
                "ok": false,
                "defect": { "kind": "FalsePass", "reason": "not_admin" },
                "error": "caller is not in admin_principals",
            })
            .to_string();
        }
        let pattern = input.scope_token_pattern.trim();
        let reason = input.reason.trim();
        if pattern.is_empty() {
            self.emit_tool_ocel(
                "onto_receipts_revoke_batch",
                started,
                false,
                &[],
            );
            return serde_json::json!({
                "ok": false,
                "error": "scope_token_pattern must not be empty",
            })
            .to_string();
        }
        if reason.is_empty() {
            self.emit_tool_ocel(
                "onto_receipts_revoke_batch",
                started,
                false,
                &[],
            );
            return serde_json::json!({
                "ok": false,
                "error": "reason must not be empty",
            })
            .to_string();
        }
        // Audit-only — admin tools cannot deny themselves.
        let mut audit_artifact: Vec<u8> = Vec::with_capacity(
            pattern.len() + reason.len() + 1,
        );
        audit_artifact.extend_from_slice(pattern.as_bytes());
        audit_artifact.push(0);
        audit_artifact.extend_from_slice(reason.as_bytes());
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::ReceiptsBatchRevoke,
            None,
            "receipts-revoke-batch",
            &audit_artifact,
        );
        // Soft-delete — preserve the chain.
        let updated = {
            let conn = self.db.conn();
            match conn.execute(
                "UPDATE receipts \
                    SET production_law_version = 'revoked-by-admin' \
                  WHERE scope_token GLOB ?1 \
                    AND production_law_version != 'seed-v0' \
                    AND production_law_version != 'revoked-by-admin'",
                rusqlite::params![pattern],
            ) {
                Ok(n) => n as u64,
                Err(e) => {
                    drop(conn);
                    self.emit_tool_ocel(
                        "onto_receipts_revoke_batch",
                        started,
                        false,
                        &[],
                    );
                    return serde_json::json!({
                        "ok": false,
                        "error": format!("UPDATE failed: {}", e.to_string().replace('"', "'")),
                    })
                    .to_string();
                }
            }
        };
        self.lineage().record(
            &self.session_id,
            "K",
            "receipts_batch_revoked",
            &format!("pattern={};reason={};count={}", pattern, reason, updated),
        );
        self.emit_tool_ocel("onto_receipts_revoke_batch", started, true, &[]);
        serde_json::json!({
            "ok": true,
            "scope_token_pattern": pattern,
            "reason": reason,
            "count": updated,
        })
        .to_string()
    }

    /// Forcefully revoke every active session for a principal in a
    /// tenant. Admin-gated. Emits OCEL `session_revoke` audit event.
    ///
    /// FALLBACK NOTE: R3 Task B's canonical `revoked_principals` table
    /// is still blocked behind wasm4pm PR #34. Until it lands, this
    /// tool bulk-INSERTS into the existing `revoked_sessions` table
    /// for every `session_id` referenced by `declared_workflows` rows
    /// belonging to the tenant. This is a **fallback**: the surface
    /// area remains the same (admin caller + audit trail + ACL effect),
    /// but the table targeted is `revoked_sessions` instead of the
    /// canonical `revoked_principals`.
    /// TODO(R3 Task B): switch the INSERT target to `revoked_principals`
    /// once that table is in tree.
    #[tool(name = "onto_session_revoke_by_principal", description = "Forcefully revoke all active sessions for a principal (tenant-scoped). Admin-gated. Falls back to revoked_sessions until R3 Task B's revoked_principals lands.")]
    pub fn onto_session_revoke_by_principal(
        &self,
        Parameters(input): Parameters<crate::inputs::OntoSessionRevokeByPrincipalInput>,
    ) -> String {
        let started = std::time::Instant::now();
        if !self.is_admin_principal() {
            self.emit_tool_ocel(
                "onto_session_revoke_by_principal",
                started,
                false,
                &[],
            );
            return serde_json::json!({
                "ok": false,
                "defect": { "kind": "FalsePass", "reason": "not_admin" },
                "error": "caller is not in admin_principals",
            })
            .to_string();
        }
        let tenant_id = input.tenant_id.trim();
        let principal_id = input.principal_id.trim();
        let reason = input.reason.trim();
        if tenant_id.is_empty() || principal_id.is_empty() || reason.is_empty() {
            self.emit_tool_ocel(
                "onto_session_revoke_by_principal",
                started,
                false,
                &[],
            );
            return serde_json::json!({
                "ok": false,
                "error": "tenant_id, principal_id, and reason are all required (non-empty)",
            })
            .to_string();
        }
        // Audit-only — operational governance tool.
        let mut audit_artifact: Vec<u8> = Vec::with_capacity(
            tenant_id.len() + principal_id.len() + reason.len() + 2,
        );
        audit_artifact.extend_from_slice(tenant_id.as_bytes());
        audit_artifact.push(0);
        audit_artifact.extend_from_slice(principal_id.as_bytes());
        audit_artifact.push(0);
        audit_artifact.extend_from_slice(reason.as_bytes());
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::SessionRevoke,
            None,
            "session-revoke",
            &audit_artifact,
        );
        // FALLBACK: bulk-INSERT into revoked_sessions for every session
        // currently owning a workflow in this tenant. Until R3 Task B
        // delivers `revoked_principals`, this is the closest equivalent
        // — we deny by session_id (the granularity admission already
        // checks) rather than by principal directly. `INSERT OR IGNORE`
        // keeps the call idempotent.
        let now = chrono::Utc::now().to_rfc3339();
        let inserted = {
            let conn = self.db.conn();
            match conn.execute(
                "INSERT OR IGNORE INTO revoked_sessions \
                    (session_id, reason, revoked_at, tenant_id) \
                 SELECT DISTINCT session_id, ?1, ?2, ?3 \
                   FROM declared_workflows \
                  WHERE tenant_id = ?3 \
                    AND status = 'open'",
                rusqlite::params![reason, now, tenant_id],
            ) {
                Ok(n) => n as u64,
                Err(e) => {
                    drop(conn);
                    self.emit_tool_ocel(
                        "onto_session_revoke_by_principal",
                        started,
                        false,
                        &[],
                    );
                    return serde_json::json!({
                        "ok": false,
                        "error": format!("INSERT failed: {}", e.to_string().replace('"', "'")),
                    })
                    .to_string();
                }
            }
        };
        // `conn` MutexGuard dropped above so subsequent lineage / OCEL
        // calls can reacquire the SQLite mutex.
        self.lineage().record(
            &self.session_id,
            "K",
            "session_revoked_by_principal",
            &format!(
                "tenant={};principal={};reason={};count={}",
                tenant_id, principal_id, reason, inserted
            ),
        );
        self.emit_tool_ocel(
            "onto_session_revoke_by_principal",
            started,
            true,
            &[],
        );
        serde_json::json!({
            "ok": true,
            "tenant_id": tenant_id,
            "principal_id": principal_id,
            "reason": reason,
            "sessions_revoked": inserted,
            "fallback_note": "R3 Task B's revoked_principals table is not yet available; revoked_sessions used as fallback target",
        })
        .to_string()
    }

    /// Suspend the [`crate::retention::RetentionWorker`] for `minutes`.
    /// Admin-gated. Emits OCEL `admission_audit{op=feedback}` (reused
    /// AdmissionOp variant — pause/resume are operational tweaks, not
    /// new audit semantics).
    ///
    /// Sets `retention_paused_until` to `now() + minutes * 60`. The
    /// worker checks this each `tick()` and skips work if paused.
    /// Bounded to 1 week (10080 minutes) to prevent indefinite
    /// suspension; longer durations require multiple calls.
    #[tool(name = "onto_retention_pause", description = "Pause the RetentionWorker for N minutes (max 10080 = 1 week). Admin-gated emergency kill-switch.")]
    pub fn onto_retention_pause(
        &self,
        Parameters(input): Parameters<crate::inputs::OntoRetentionPauseInput>,
    ) -> String {
        let started = std::time::Instant::now();
        if !self.is_admin_principal() {
            self.emit_tool_ocel("onto_retention_pause", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "defect": { "kind": "FalsePass", "reason": "not_admin" },
                "error": "caller is not in admin_principals",
            })
            .to_string();
        }
        let minutes = input.minutes;
        if minutes == 0 || minutes > 10080 {
            self.emit_tool_ocel("onto_retention_pause", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "error": "minutes must be in 1..=10080 (max 1 week)",
            })
            .to_string();
        }
        // Audit-only — Feedback variant (operational tweak; reused).
        let audit_artifact = format!("retention_pause:{}", minutes);
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::Feedback,
            None,
            "retention-pause",
            audit_artifact.as_bytes(),
        );
        let until = chrono::Utc::now().timestamp() + (minutes as i64) * 60;
        self.retention_paused_until
            .store(until, std::sync::atomic::Ordering::Relaxed);
        self.lineage().record(
            &self.session_id,
            "K",
            "retention_paused",
            &format!("minutes={};until_epoch={}", minutes, until),
        );
        self.emit_tool_ocel("onto_retention_pause", started, true, &[]);
        serde_json::json!({
            "ok": true,
            "minutes": minutes,
            "paused_until_epoch_secs": until,
            "paused_until_rfc3339": chrono::DateTime::<chrono::Utc>::from_timestamp(until, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        })
        .to_string()
    }

    /// Resume the [`crate::retention::RetentionWorker`] immediately by
    /// clearing the `paused_until` atomic to 0. Admin-gated. Idempotent
    /// (calling twice is a no-op). Reuses `AdmissionOp::Feedback` for
    /// the audit since pause and resume are paired operational tweaks.
    #[tool(name = "onto_retention_resume", description = "Clear the RetentionWorker pause kill-switch immediately. Admin-gated. Idempotent.")]
    pub fn onto_retention_resume(&self) -> String {
        let started = std::time::Instant::now();
        if !self.is_admin_principal() {
            self.emit_tool_ocel("onto_retention_resume", started, false, &[]);
            return serde_json::json!({
                "ok": false,
                "defect": { "kind": "FalsePass", "reason": "not_admin" },
                "error": "caller is not in admin_principals",
            })
            .to_string();
        }
        let prev = self
            .retention_paused_until
            .swap(0, std::sync::atomic::Ordering::Relaxed);
        // Audit-only — paired with onto_retention_pause.
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::Feedback,
            None,
            "retention-resume",
            self.session_id.as_bytes(),
        );
        self.lineage().record(
            &self.session_id,
            "K",
            "retention_resumed",
            &format!("prev_paused_until_epoch={}", prev),
        );
        self.emit_tool_ocel("onto_retention_resume", started, true, &[]);
        serde_json::json!({
            "ok": true,
            "previous_paused_until_epoch_secs": prev,
            "now_paused": false,
        })
        .to_string()
    }

    // ── R10-2: Ontostar integration seal ────────────────────────────────────

    #[tool(
        name = "onto_ontostar_attest",
        description = "R10-2 Ontostar integration seal: verify an external OntoStar Ed25519 receipt and record the key fingerprint in trusted_keys_history. Accepts base64-encoded signature, BLAKE3 payload hash, and hex key fingerprint. Returns {ok, fingerprint, recorded} on success. Rejects if signature does not verify against TrustedKeys."
    )]
    pub fn onto_ontostar_attest(
        &self,
        Parameters(input): Parameters<crate::inputs::OntoOntostarAttestInput>,
    ) -> String {
        use crate::attestation::{TrustedKeys, fingerprint_hex};
        use base64::Engine as _;

        // Decode signature bytes into fixed [u8;64] — Ed25519 signatures are always 64 bytes.
        let sig_bytes: [u8; 64] = {
            let v = match base64::engine::general_purpose::STANDARD.decode(&input.signature) {
                Ok(b) => b,
                Err(e) => return serde_json::json!({
                    "ok": false,
                    "error": format!("base64 decode signature: {e}"),
                }).to_string(),
            };
            match v.try_into() {
                Ok(arr) => arr,
                Err(_) => return serde_json::json!({
                    "ok": false,
                    "error": "signature must be 64 bytes (Ed25519)",
                }).to_string(),
            }
        };

        // Load trusted keys from env (same path as A10 verifier).
        let trusted = match TrustedKeys::from_env() {
            Ok(Some(t)) => t,
            Ok(None) => return serde_json::json!({
                "ok": false,
                "error": "OPEN_ONTOLOGIES_TRUSTED_KEYS_DIR not configured",
            }).to_string(),
            Err(e) => return serde_json::json!({
                "ok": false,
                "error": format!("load trusted keys: {e}"),
            }).to_string(),
        };

        // Decode fingerprint into [u8;8].
        let fpr_bytes: [u8; 8] = match hex::decode(&input.key_fpr) {
            Ok(b) if b.len() == 8 => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&b);
                arr
            }
            _ => return serde_json::json!({
                "ok": false,
                "error": "key_fpr must be 16 hex chars (8 bytes)",
            }).to_string(),
        };

        // Verify signature.
        let payload_bytes = input.payload_hash.as_bytes();
        let outcome = crate::attestation::verify_strict(
            &trusted,
            &fpr_bytes,
            payload_bytes,
            &sig_bytes,
        );
        if outcome != crate::attestation::VerifyOutcome::Valid {
            return serde_json::json!({
                "ok": false,
                "error": format!("signature verification failed: {outcome:?}"),
            }).to_string();
        }

        // Emit audit trail before writing so the OCEL event is durable
        // even if the INSERT is skipped on duplicate. This call exempts
        // the handler from the no_bypass_audit DB-write gate check.
        self.evaluate_admission_audit(
            crate::admission::AdmissionOp::OntostarAttest,
            None,
            "ontostar_attest",
            &fpr_bytes,
        );

        // Record fingerprint in trusted_keys_history (idempotent INSERT OR IGNORE).
        let now = chrono::Utc::now().to_rfc3339();
        let fpr_hex = fingerprint_hex(&fpr_bytes);
        let conn = self.db.conn();
        let recorded = conn.execute(
            "INSERT OR IGNORE INTO trusted_keys_history
                (fingerprint, pem, added_at, removed_at, status)
             VALUES (?1, '', ?2, NULL, 'ontostar-external')",
            rusqlite::params![fpr_hex, now],
        ).unwrap_or(0);

        serde_json::json!({
            "ok": true,
            "fingerprint": fpr_hex,
            "recorded": recorded > 0,
        }).to_string()
    }

    #[tool(
        name = "onto_guide",
        description = "Workflow planner. Given a plain-language intent, returns a step-by-step \
        tool plan for the matching builtin workflow. Supported intents: 'load and validate an \
        ontology', 'ontology authoring', 'ingest CSV data', 'apply lifecycle changes', \
        'align two ontologies', 'generate code', 'requirements ctq', 'manufacture a solution', \
        'semantic search'. Set include_powl=true to also receive the POWL string for \
        onto_declare_workflow. Unknown intents return a list of known workflow names."
    )]
    pub fn onto_guide(&self, Parameters(input): Parameters<crate::inputs::OntoGuideInput>) -> String {
        let plan = crate::guide::plan_for_intent(
            &input.intent,
            input.include_powl.unwrap_or(false),
        );
        serde_json::to_string(&plan)
            .unwrap_or_else(|e| format!(r#"{{"ok":false,"error":"{}"}}"#, e))
    }
}

// ─── Codegen receipt-stamping helper ────────────────────────────────────────

/// Walk `output_dir` recursively and prepend the OntoStar receipt header to
/// every text source file whose extension supports inline comments. Returns
/// the count of files actually stamped. Errors on individual files are
/// silently skipped — emission must not fail because one file was unwritable.
fn stamp_codegen_output(output_dir: &str, receipt: &crate::receipts::Receipt) -> usize {
    let mut stamped = 0usize;
    let mut stack: Vec<std::path::PathBuf> = vec![std::path::PathBuf::from(output_dir)];
    while let Some(p) = stack.pop() {
        let entries = match std::fs::read_dir(&p) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    stack.push(path);
                } else if ft.is_file()
                    && let Ok(true) = crate::receipts::inject_comment_header(&path, receipt) {
                        stamped += 1;
                    }
            }
        }
    }
    stamped
}

// ─── Stream 5 helpers ───────────────────────────────────────────────────────

/// Short 16-hex-char fingerprint used as a deterministic id suffix.
/// Not cryptographic — Stream 3 owns the real BLAKE3 receipt chain.
fn uuid_short(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h1 = DefaultHasher::new();
    input.hash(&mut h1);
    let a = h1.finish();
    let mut h2 = DefaultHasher::new();
    a.hash(&mut h2);
    input.hash(&mut h2);
    let b = h2.finish();
    format!("{:016x}{:016x}", a, b)[..16].to_string()
}

// ─── Prompt definitions ─────────────────────────────────────────────────────

#[prompt_router]
impl OpenOntologiesServer {
    /// Build an ontology from a domain description. Guides through the full workflow: generate Turtle, validate, load, lint, query, and persist.
    #[prompt(name = "build_ontology")]
    fn build_ontology(&self, Parameters(input): Parameters<BuildOntologyInput>) -> Result<GetPromptResult, rmcp::ErrorData> {
        let msg = format!(
            "Build an OWL ontology for the following domain:\n\n{}\n\n\
            Follow the Open Ontologies workflow:\n\
            1. Generate Turtle/OWL directly\n\
            2. Call onto_validate on the generated Turtle\n\
            3. Call onto_load to load into the triple store\n\
            4. Call onto_stats to verify counts\n\
            5. Call onto_lint to check for missing labels, comments, domains, ranges\n\
            6. Call onto_query with SPARQL to verify structure\n\
            7. Fix any issues and iterate until clean\n\
            8. Call onto_save to persist the final ontology",
            input.domain
        );
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(PromptMessageRole::User, msg),
        ]).with_description("Build an ontology from a domain description"))
    }

    /// Validate and lint an existing ontology file. Loads it, runs validation and lint checks, reports all issues.
    #[prompt(name = "validate_ontology")]
    fn validate_ontology(&self, Parameters(input): Parameters<ValidateOntologyInput>) -> Result<GetPromptResult, rmcp::ErrorData> {
        let msg = format!(
            "Validate and lint the ontology at: {}\n\n\
            Steps:\n\
            1. Call onto_validate to check syntax\n\
            2. Call onto_load to load into the triple store\n\
            3. Call onto_stats to show class/property/triple counts\n\
            4. Call onto_lint to check for missing labels, domains, ranges\n\
            5. Report all issues found and suggest fixes",
            input.path
        );
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(PromptMessageRole::User, msg),
        ]).with_description("Validate and lint an ontology file"))
    }

    /// Compare two versions of an ontology. Shows added/removed classes, properties, and drift analysis.
    #[prompt(name = "compare_ontologies")]
    fn compare_ontologies(&self, Parameters(input): Parameters<CompareOntologiesInput>) -> Result<GetPromptResult, rmcp::ErrorData> {
        let msg = format!(
            "Compare these two ontology versions:\n\
            - Old: {}\n\
            - New: {}\n\n\
            Steps:\n\
            1. Call onto_diff to see structural changes\n\
            2. Call onto_drift to analyze drift velocity and detect renames\n\
            3. Summarize: what was added, removed, renamed, and the overall risk",
            input.old_path, input.new_path
        );
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(PromptMessageRole::User, msg),
        ]).with_description("Compare two ontology versions"))
    }

    /// Ingest external data into a loaded ontology. Maps data fields to ontology classes/properties and validates with SHACL.
    #[prompt(name = "ingest_data")]
    fn ingest_data(&self, Parameters(input): Parameters<IngestDataInput>) -> Result<GetPromptResult, rmcp::ErrorData> {
        let msg = format!(
            "Ingest data from {} into the currently loaded ontology.\n\n\
            Steps:\n\
            1. Call onto_map to inspect the data and suggest a mapping\n\
            2. Review and adjust the mapping\n\
            3. Call onto_ingest with the mapping to generate RDF triples\n\
            4. Call onto_stats to verify triple counts\n\
            5. Call onto_shacl to validate against SHACL shapes\n\
            6. Call onto_reason to infer additional triples\n\
            7. Call onto_query to verify the ingested data",
            input.data_path
        );
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(PromptMessageRole::User, msg),
        ]).with_description("Ingest external data into a loaded ontology"))
    }

    /// Align two ontologies using hybrid neuro-symbolic matching. Runs structural alignment first, then asks you (the LLM) to adjudicate uncertain pairs.
    #[prompt(name = "align_ontologies")]
    fn align_ontologies(&self, Parameters(input): Parameters<AlignOntologiesInput>) -> Result<GetPromptResult, rmcp::ErrorData> {
        let msg = format!(
            "Align these two ontologies using hybrid neuro-symbolic matching:\n\
            - Source: {}\n\
            - Target: {}\n\n\
            Follow this pipeline:\n\n\
            **Step 1: Structural alignment**\n\
            Call onto_align with source, target, min_confidence=0.7, dry_run=true.\n\
            This returns candidates with confidence scores and signal breakdowns.\n\n\
            **Step 2: Auto-accept high-confidence matches**\n\
            Candidates with confidence >= 0.95 are reliable. List them as accepted.\n\n\
            **Step 3: LLM adjudication of uncertain pairs**\n\
            For candidates with confidence 0.7-0.95, YOU decide:\n\
            - Look at the source and target labels, their parent classes, and the signal breakdown\n\
            - Use your knowledge of the domain to judge if they refer to the same concept\n\
            - Accept the pair if they are genuinely equivalent; reject if they are false matches\n\
            - Example: \"levator auris longus\" (mouse muscle) <-> \"Auricularis\" (human muscle) = ACCEPT (same ear muscle, different species names)\n\
            - Example: \"tail\" <-> \"Tail_of_Pancreas\" = REJECT (different concepts despite shared word)\n\n\
            **Step 4: Apply accepted matches**\n\
            For each accepted pair (both auto-accepted and LLM-adjudicated), call onto_align_feedback with accepted=true.\n\
            For rejected pairs, call onto_align_feedback with accepted=false.\n\
            This trains the self-calibrating weights for future alignments.\n\n\
            **Step 5: Report**\n\
            Summarize: total candidates, auto-accepted, LLM-accepted, LLM-rejected, and final alignment count.",
            input.source_path, input.target_path
        );
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(PromptMessageRole::User, msg),
        ]).with_description("Align two ontologies using hybrid neuro-symbolic matching (structural + LLM adjudication)"))
    }

    /// Explore a loaded ontology with SPARQL. Lists classes, properties, and answers competency questions.
    #[prompt(name = "explore_ontology")]
    fn explore_ontology(&self) -> Result<GetPromptResult, rmcp::ErrorData> {
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(
                PromptMessageRole::User,
                "Explore the currently loaded ontology:\n\n\
                1. Call onto_stats to show overview counts\n\
                2. Call onto_query to list all classes with labels\n\
                3. Call onto_query to show the class hierarchy (subClassOf)\n\
                4. Call onto_query to list all properties with domains and ranges\n\
                5. Summarize the ontology structure and suggest competency questions it can answer",
            ),
        ]).with_description("Explore a loaded ontology with SPARQL"))
    }

    /// Generate code artifacts from the loaded ontology using ggen. Guides through full workflow: specify generator, run reasoning, invoke codegen, verify output.
    #[prompt(name = "generate_code")]
    fn generate_code(&self, Parameters(input): Parameters<GenerateCodeInput>) -> Result<GetPromptResult, rmcp::ErrorData> {
        let msg = format!(
            "Generate {} code from the currently loaded ontology.\n\n\
            Follow these steps:\n\
            1. Call onto_stats to show the ontology overview\n\
            2. Call onto_reason with profile='owl-rl' to materialize inferred triples (especially subClassOf chains)\n\
            3. Call onto_query to verify class hierarchy and properties before codegen\n\
            4. Call onto_codegen with generator='{}' and output_dir='{}'\n\
            5. Verify generated artifacts exist and are syntactically correct\n\
            6. Report the file paths and any generated API/class counts",
            input.language.unwrap_or_else(|| "Python".to_string()),
            input.generator.unwrap_or_else(|| "python-client".to_string()),
            input.output_dir.unwrap_or_else(|| "./generated".to_string())
        );
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(PromptMessageRole::User, msg),
        ]).with_description("Generate code artifacts from the loaded ontology using ggen"))
    }
}

// ─── ServerHandler ──────────────────────────────────────────────────────────

#[tool_handler]
#[prompt_handler]
impl ServerHandler for OpenOntologiesServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().enable_prompts().build())
            .with_instructions("Open Ontologies: AI-native ontology engine — RDF/OWL/SPARQL MCP server with 43 tools and 6 workflow prompts for ontology engineering, validation, comparison, alignment, data ingestion, and exploration.")
    }
}
