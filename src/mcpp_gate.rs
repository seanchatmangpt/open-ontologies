//! mcpp proof gating middleware for open-ontologies MCP server.
//!
//! `MaybeGatedServer` is the public entry point — it wraps
//! `OpenOntologiesServer` and is transparent unless the `mcpp` feature is
//! enabled AND `MCPP_SIGNING_KEY_PATH` is set at startup.
//!
//! With gating active, every successful tool call is admitted through
//! `ProofWriter::admit()` and the JSON response gains an `"mcpp"` field:
//!
//! ```json
//! { "ok": true, "thresholds": [], "mcpp": { "verdict": "accepted", ... } }
//! ```
//!
//! K-P09: the sole `ProofWriter::admit()` call site lives in
//! `ProofGatedServer::call_tool` below.

use std::sync::Arc;

use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, GetPromptRequestParams,
        GetPromptResult, InitializeRequestParams, InitializeResult, ListPromptsResult,
        ListToolsResult, PaginatedRequestParams, ServerInfo, Tool,
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
/// intercepts every tool call through mcpp's `ProofWriter`.
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
    /// Every successful tool call will be admitted through `ProofWriter::admit()`
    /// and the JSON response gains an `"mcpp"` field with verdict and receipt hash.
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
    /// let signing_key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
    /// let gated = ProofGatedServer::new(server, db, signing_key);
    /// // `gated.inner` holds the wrapped server.
    /// let _registry = gated.inner.registry();
    /// # }
    /// ```
    pub fn new(inner: H, db: StateDb, signing_key: ed25519_dalek::SigningKey) -> Self {
        Self { inner, db, signing_key }
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

    /// K-P09: sole `ProofWriter::admit()` call site in this crate.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        use chrono::Utc;
        use ed25519_dalek::Signer;
        use mcpp_core::{
            manifest::PartManifest,
            proof_writer::{AdmissionEvidence, SignedReceipt, new_proof_writer_for_proof_cmd},
            protocol::{request::ConformanceThresholds, verdict::Verdict},
            receipt::BuildReceipt,
        };
        use ulid::Ulid;

        let tool_name = request.name.clone();
        let scope_token = format!("mcpp-{}-{}", tool_name, Ulid::new());
        let started = Utc::now();

        // 1. Synthetic OCEL event — ensures evidence is never empty for
        //    read-only tools. Guard scoped here; released before inner call.
        emit_invocation_event(&self.db, &scope_token, &tool_name)
            .map_err(|e| ErrorData::internal_error(format!("mcpp: ocel emit failed: {e}"), None))?;

        // 2. Delegate to inner server (inner acquires its own DB locks).
        let result = self.inner.call_tool(request, context).await?;

        // 3. Check if the tool reported success; tool errors pass through ungated.
        let text = extract_text(&result);
        let result_json: serde_json::Value =
            serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        let ok = result_json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);

        if !ok {
            return Ok(result);
        }

        // 4. Collect OCEL evidence (scoped lock, released before admit).
        let ocel_ev = collect_ocel(&self.db, &scope_token, started)
            .map_err(|e| ErrorData::internal_error(format!("mcpp: ocel collect failed: {e}"), None))?;

        // 5. Build Ed25519-signed receipt.
        let part_name = format!("onto-gate/{tool_name}");
        let mut build_receipt = BuildReceipt::new_detached(&part_name);
        build_receipt.sign(&self.signing_key)
            .map_err(|e| ErrorData::internal_error(format!("mcpp: receipt sign failed: {e}"), None))?;
        let canonical = build_receipt.canonical_bytes();
        let sig_bytes = self.signing_key.sign(&canonical);
        let signature_hex = hex::encode(sig_bytes.to_bytes());
        let signed_receipt = SignedReceipt { receipt: build_receipt, signature: signature_hex };

        let receipt_hash = {
            use mcpp_core::receipt::hash_bytes;
            hash_bytes(&canonical)
        };

        // 6. Assemble admission evidence.
        let manifest = PartManifest::new("onto-gate", "0.1.0", vec![tool_name.to_string()]);
        let evidence = AdmissionEvidence {
            route: "ontology".to_string(),
            conformance_vector: ConformanceThresholds {
                fitness:     Some(1.0),
                precision:   Some(1.0),
                lifecycle:   Some(1.0),
                cardinality: Some(1.0),
                receipt:     Some(1.0),
            },
            ocel_evidence: ocel_ev,
            manifest,
            signed_receipt,
        };

        // K-P09: sole ProofWriter::admit() call.
        // A gated server that silently degrades is not a gated server.
        let writer = new_proof_writer_for_proof_cmd();
        match writer.admit(evidence) {
            Ok(Verdict::Accept(_)) => {
                Ok(augment_with_proof(result, &scope_token, &receipt_hash))
            }
            Ok(Verdict::Refuse { reason, .. }) => {
                Err(ErrorData::internal_error(format!("mcpp: proof gate refused: {reason:?}"), None))
            }
            Err(refusal) => {
                Err(ErrorData::internal_error(format!("mcpp: proof gate error: {refusal:?}"), None))
            }
        }
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
) -> CallToolResult {
    if let Some(first) = result.content.first_mut() {
        if let RawContent::Text(ref mut t) = first.raw {
            if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&t.text) {
                v["mcpp"] = serde_json::json!({
                    "verdict":      "accepted",
                    "scope_token":  scope_token,
                    "receipt_hash": receipt_hash,
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
fn emit_invocation_event(
    db: &StateDb,
    scope_token: &str,
    tool_name: &str,
) -> anyhow::Result<()> {
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
/// and wrap them as `OcelEvidence` for mcpp's `AdmissionEvidence`.
/// The `db.conn()` guard is scoped to an inner block.
#[cfg(feature = "mcpp")]
fn collect_ocel(
    db: &StateDb,
    scope_token: &str,
    since: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<mcpp_core::proof_writer::OcelEvidence> {
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
    Ok(mcpp_core::proof_writer::OcelEvidence::from_bytes(ocel_json))
}
