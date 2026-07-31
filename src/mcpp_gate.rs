//! mcpp proof gating middleware for open-ontologies MCP server.
//!
//! `MaybeGatedServer` is the public entry point — it wraps
//! `OpenOntologiesServer` and is transparent unless the `mcpp` feature is
//! enabled AND `MCPP_SIGNING_KEY_PATH` is set at startup.
//!
//! With gating active, every successful tool call receives a canonical
//! BLAKE3 + Ed25519 proof envelope and the JSON response gains an `"mcpp"` field:
//!
//! ```json
//! { "ok": true, "thresholds": [], "mcpp": { "verdict": "accepted", ... } }
//! ```
//!
//! K-P09: the portable proof-envelope boundary lives in
//! `ProofGatedServer::call_tool` below.

use std::sync::Arc;

use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, GetPromptRequestParams, GetPromptResult,
        InitializeRequestParams, InitializeResult, ListPromptsResult, ListToolsResult,
        PaginatedRequestParams, ServerInfo, Tool,
    },
    service::RequestContext,
};

use crate::registry::OntologyRegistry;
use crate::server::OpenOntologiesServer;
#[cfg(feature = "mcpp")]
use crate::state::StateDb;

#[cfg(feature = "mcpp")]
use rmcp::model::RawContent;

// ─── MaybeGatedServer (always compiled) ────────────────────────────────────

/// Runtime-selectable wrapper: `Bare` runs without proof gating; `Gated`
/// intercepts every successful tool call through a portable signed proof envelope.
///
/// In production, the correct variant is selected at startup based on whether
/// `MCPP_SIGNING_KEY_PATH` is set. The `Bare` variant is always available;
/// `Gated` is compiled only with the `mcpp` feature flag.
///
/// # Examples
///
/// ```
/// use open_ontologies::mcpp_gate::MaybeGatedServer;
/// use open_ontologies::server::OpenOntologiesServer;
/// use open_ontologies::state::StateDb;
/// use std::path::Path;
///
/// let db = StateDb::open(Path::new(":memory:")).unwrap();
/// let server = OpenOntologiesServer::new(db);
/// let gated = MaybeGatedServer::Bare(server);
///
/// // The registry is reachable through the wrapper.
/// let _registry = gated.registry();
/// ```
///
/// Pattern-matching on the variant tells callers whether proof gating is
/// active without needing to inspect environment variables:
///
/// ```
/// use open_ontologies::mcpp_gate::MaybeGatedServer;
/// use open_ontologies::server::OpenOntologiesServer;
/// use open_ontologies::state::StateDb;
/// use std::path::Path;
///
/// let db = StateDb::open(Path::new(":memory:")).unwrap();
/// let wrapper = MaybeGatedServer::Bare(OpenOntologiesServer::new(db));
///
/// // Without the `mcpp` feature, only the `Bare` variant is reachable.
/// let is_bare = matches!(wrapper, MaybeGatedServer::Bare(_));
/// assert!(is_bare, "default build uses the Bare (ungated) variant");
/// ```
pub enum MaybeGatedServer {
    Bare(OpenOntologiesServer),
    #[cfg(feature = "mcpp")]
    Gated(ProofGatedServer<OpenOntologiesServer>),
}

impl MaybeGatedServer {
    /// Return the shared [`OntologyRegistry`] for the wrapped server.
    ///
    /// The returned `Arc` is the same registry that the underlying server uses
    /// for all tool calls; callers may clone it to share ownership.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::mcpp_gate::MaybeGatedServer;
    /// use open_ontologies::server::OpenOntologiesServer;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let wrapper = MaybeGatedServer::Bare(OpenOntologiesServer::new(db));
    /// // registry() always returns a valid Arc regardless of the active variant.
    /// let registry = wrapper.registry();
    /// // The Arc reference count is at least 1 (the server itself holds one).
    /// assert!(std::sync::Arc::strong_count(&registry) >= 1);
    /// ```
    ///
    /// Two callers receive the same underlying registry (shared ownership):
    ///
    /// ```
    /// use open_ontologies::mcpp_gate::MaybeGatedServer;
    /// use open_ontologies::server::OpenOntologiesServer;
    /// use open_ontologies::state::StateDb;
    /// use std::sync::Arc;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let wrapper = MaybeGatedServer::Bare(OpenOntologiesServer::new(db));
    ///
    /// let r1 = wrapper.registry();
    /// let r2 = wrapper.registry();
    /// // Both Arcs point at the same allocation.
    /// assert!(Arc::ptr_eq(&r1, &r2), "registry() returns the same Arc each call");
    /// ```
    pub fn registry(&self) -> Arc<OntologyRegistry> {
        match self {
            Self::Bare(s) => s.registry(),
            #[cfg(feature = "mcpp")]
            Self::Gated(s) => s.inner.registry(),
        }
    }
}

impl ServerHandler for MaybeGatedServer {
    fn get_info(&self) -> ServerInfo {
        match self {
            Self::Bare(s) => s.get_info(),
            #[cfg(feature = "mcpp")]
            Self::Gated(s) => s.get_info(),
        }
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        match self {
            Self::Bare(s) => s.get_tool(name),
            #[cfg(feature = "mcpp")]
            Self::Gated(s) => s.get_tool(name),
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        match self {
            Self::Bare(s) => s.call_tool(request, context).await,
            #[cfg(feature = "mcpp")]
            Self::Gated(s) => s.call_tool(request, context).await,
        }
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        match self {
            Self::Bare(s) => s.list_tools(request, context).await,
            #[cfg(feature = "mcpp")]
            Self::Gated(s) => s.list_tools(request, context).await,
        }
    }

    async fn list_prompts(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        match self {
            Self::Bare(s) => s.list_prompts(request, context).await,
            #[cfg(feature = "mcpp")]
            Self::Gated(s) => s.list_prompts(request, context).await,
        }
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, ErrorData> {
        match self {
            Self::Bare(s) => s.get_prompt(request, context).await,
            #[cfg(feature = "mcpp")]
            Self::Gated(s) => s.get_prompt(request, context).await,
        }
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        match self {
            Self::Bare(s) => s.initialize(request, context).await,
            #[cfg(feature = "mcpp")]
            Self::Gated(s) => s.initialize(request, context).await,
        }
    }
}

// ─── ProofGatedServer (mcpp feature only) ──────────────────────────────────

#[cfg(feature = "mcpp")]
pub struct ProofGatedServer<H: ServerHandler> {
    pub inner: H,
    db: StateDb,
    signing_key: ed25519_dalek::SigningKey,
}

#[cfg(feature = "mcpp")]
impl<H: ServerHandler> ProofGatedServer<H> {
    /// Wrap `inner` with proof gating backed by `db` and `signing_key`.
    ///
    /// Every successful tool call receives a canonical BLAKE3 + Ed25519 envelope,
    /// and the JSON response gains an `"mcpp"` field with its receipt and signature.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "mcpp")]
    /// # {
    /// use open_ontologies::mcpp_gate::ProofGatedServer;
    /// use open_ontologies::server::OpenOntologiesServer;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let server = OpenOntologiesServer::new(db.clone());
    /// let signing_key = ed25519_dalek::SigningKey::from_bytes(&[7_u8; 32]);
    /// let gated = ProofGatedServer::new(server, db, signing_key);
    /// // `gated.inner` holds the wrapped server.
    /// let _registry = gated.inner.registry();
    /// # }
    /// ```
    pub fn new(inner: H, db: StateDb, signing_key: ed25519_dalek::SigningKey) -> Self {
        Self {
            inner,
            db,
            signing_key,
        }
    }
}

#[cfg(feature = "mcpp")]
impl<H: ServerHandler> ServerHandler for ProofGatedServer<H> {
    fn get_info(&self) -> ServerInfo {
        self.inner.get_info()
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.inner.get_tool(name)
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        self.inner.list_tools(request, context).await
    }

    async fn list_prompts(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        self.inner.list_prompts(request, context).await
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, ErrorData> {
        self.inner.get_prompt(request, context).await
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        self.inner.initialize(request, context).await
    }

    /// K-P09: portable proof-envelope boundary; no private workspace dependency.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        use chrono::Utc;
        use ed25519_dalek::Signer;
        use ulid::Ulid;

        let tool_name = request.name.clone();
        let scope_token = format!("mcpp-{}-{}", tool_name, Ulid::new());
        let started = Utc::now();

        emit_invocation_event(&self.db, &scope_token, &tool_name)
            .map_err(|e| ErrorData::internal_error(format!("mcpp: ocel emit failed: {e}"), None))?;

        let result = self.inner.call_tool(request, context).await?;
        let text = extract_text(&result);
        let result_json: serde_json::Value =
            serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        let ok = result_json
            .get("ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !ok {
            return Ok(result);
        }

        let ocel_bytes = collect_ocel(&self.db, &scope_token, started).map_err(|e| {
            ErrorData::internal_error(format!("mcpp: ocel collect failed: {e}"), None)
        })?;
        let canonical = serde_json::to_vec(&serde_json::json!({
            "protocol": "mcpp-compat/v1",
            "route": "ontology",
            "tool": tool_name,
            "scope_token": scope_token,
            "tool_result": result_json,
            "ocel_blake3": blake3::hash(&ocel_bytes).to_hex().to_string(),
        }))
        .map_err(|e| {
            ErrorData::internal_error(format!("mcpp: canonicalization failed: {e}"), None)
        })?;
        let receipt_hash = blake3::hash(&canonical).to_hex().to_string();
        let signature_hex = hex::encode(self.signing_key.sign(&canonical).to_bytes());

        Ok(augment_with_proof(
            result,
            &scope_token,
            &receipt_hash,
            &signature_hex,
        ))
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Extract the JSON text from the first text content item of a `CallToolResult`.
#[cfg(feature = "mcpp")]
fn extract_text(result: &CallToolResult) -> String {
    result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.clone())
        .unwrap_or_default()
}

/// Augment the first text content item with `"mcpp": {...}` fields.
/// Returns original result unchanged if JSON parse fails.
#[cfg(feature = "mcpp")]
fn augment_with_proof(
    mut result: CallToolResult,
    scope_token: &str,
    receipt_hash: &str,
    signature: &str,
) -> CallToolResult {
    if let Some(first) = result.content.first_mut() {
        if let RawContent::Text(ref mut t) = first.raw {
            if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&t.text) {
                v["mcpp"] = serde_json::json!({
                    "verdict":      "accepted",
                    "scope_token":  scope_token,
                    "receipt_hash": receipt_hash,
                    "signature": signature,
                    "protocol": "mcpp-compat/v1",
                });
                t.text = v.to_string();
            }
        }
    }
    result
}

/// Insert a synthetic OCEL event so the evidence log is never empty for
/// read-only tools. The `db.conn()` guard is scoped to this function.
#[cfg(feature = "mcpp")]
fn emit_invocation_event(db: &StateDb, scope_token: &str, tool_name: &str) -> anyhow::Result<()> {
    use ulid::Ulid;
    let conn = db.conn();
    let event_id = Ulid::new().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO ocel_events \
         (event_id, event_type, time, scope_token, session_id, tenant_id) \
         VALUES (?1, ?2, ?3, ?4, 'mcpp-gate', 'default')",
        rusqlite::params![event_id, format!("gate:{tool_name}"), now, scope_token],
    )?;
    Ok(())
}

/// Collect all OCEL events tagged with `scope_token` at or after `since`
/// and encode them as deterministic OCEL evidence bytes.
/// The `db.conn()` guard is scoped to an inner block.
#[cfg(feature = "mcpp")]
fn collect_ocel(
    db: &StateDb,
    scope_token: &str,
    since: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<Vec<u8>> {
    let tuples: Vec<(String, String, String)> = {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT event_id, event_type, time FROM ocel_events \
             WHERE scope_token = ?1 AND time >= ?2 ORDER BY time ASC",
        )?;
        let since_str = since.to_rfc3339();
        let rows: Vec<(String, String, String)> = stmt
            .query_map(rusqlite::params![scope_token, since_str], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        rows
        // conn guard drops here
    };

    if tuples.is_empty() {
        return Err(anyhow::anyhow!("no OCEL events for scope {scope_token}"));
    }

    let events: Vec<serde_json::Value> = tuples
        .into_iter()
        .map(|(id, act, ts)| {
            serde_json::json!({
                "ocel:id":        id,
                "ocel:activity":  act,
                "ocel:timestamp": ts,
            })
        })
        .collect();

    let ocel_json = serde_json::to_vec(&serde_json::json!({
        "ocel:version": "2.0",
        "ocel:events":  events,
    }))?;
    Ok(ocel_json)
}
