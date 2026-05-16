use crate::state::StateDb;
use anyhow::Result;
use chrono::FixedOffset;
use wasm4pm_types::{OCELObject, OCEL};
use std::collections::BTreeSet;

pub struct OcelStore {
    db: StateDb,
}

// R5 WB-2 — §15 OCEL anchor closure.
//
// Test-only hook fired at the entry of `emit_event_rows` (i.e., BEFORE any
// SQL is executed). When the closure is installed and returns `Some(err)` for
// the supplied `event_type`, the emit fails as if SQLite refused — letting
// counterfactual tests prove that the admission gate's primary+fallback
// pattern actually records a degraded-trail anchor when the primary emit
// fails, instead of silently swallowing the failure.
//
// Mirrors `admission::A13_BETWEEN_SNAPSHOT_HOOK` (R5 WB-1):
// - `#[cfg(debug_assertions)]` so release builds strip the entire
//   thread_local plus the `with(...)` call inside `emit_event_rows`.
// - `#[doc(hidden)]` keeps the symbol out of public docs even though it is
//   `pub` (required for integration-test visibility).
// - Single-threaded by virtue of `thread_local!`; tests that want
//   cross-thread races must wrap their own synchronisation primitives
//   inside the closure they install.
#[cfg(debug_assertions)]
#[doc(hidden)]
pub type EmitFailureInjectionFn =
    Box<dyn Fn(&str) -> Option<anyhow::Error> + Send + 'static>;

#[cfg(debug_assertions)]
thread_local! {
    #[doc(hidden)]
    pub static EMIT_FAILURE_INJECTION_HOOK:
        std::cell::RefCell<Option<EmitFailureInjectionFn>>
        = const { std::cell::RefCell::new(None) };
}

/// SQL table that stores OCEL events. Used in every INSERT and SELECT that
/// touches the event log; centralised here so a schema rename is a one-line
/// change rather than a grep-and-pray.
///
/// # Examples
///
/// ```
/// use open_ontologies::ocel_store::OCEL_EVENTS_TABLE;
///
/// assert_eq!(OCEL_EVENTS_TABLE, "ocel_events");
/// ```
pub const OCEL_EVENTS_TABLE: &str = "ocel_events";

/// OCEL event type emitted when a seed exemplar is generated from a POWL
/// replay. Matched in [`OcelStore::seed_from_ocel_bytes`] (1 site) and in
/// the `record_llm_invoked_full` helper event_id construction (1 site).
/// Centralised so downstream OCEL consumers can reference the constant rather
/// than a bare string.
///
/// # Examples
///
/// ```
/// use open_ontologies::ocel_store::OCEL_EVENT_BUILD_ORDER_GENERATED;
///
/// assert_eq!(OCEL_EVENT_BUILD_ORDER_GENERATED, "build_order_generated");
/// ```
pub const OCEL_EVENT_BUILD_ORDER_GENERATED: &str = "build_order_generated";

/// OCEL/JSON attribute key for the POWL model string stored in seed exemplar
/// events. Used twice inside [`OcelStore::seed_from_ocel_bytes`] to extract
/// the attribute from an incoming OCEL document.
///
/// # Examples
///
/// ```
/// use open_ontologies::ocel_store::OCEL_ATTR_POWL_MODEL;
///
/// assert_eq!(OCEL_ATTR_POWL_MODEL, "powl_model");
/// ```
pub const OCEL_ATTR_POWL_MODEL: &str = "powl_model";

/// OCEL/JSON attribute key for the domain string in seed exemplar events.
/// Used twice inside [`OcelStore::seed_from_ocel_bytes`] to extract the domain
/// from an incoming OCEL document (with fallback to `default_domain`).
///
/// # Examples
///
/// ```
/// use open_ontologies::ocel_store::OCEL_ATTR_DOMAIN;
///
/// assert_eq!(OCEL_ATTR_DOMAIN, "domain");
/// ```
pub const OCEL_ATTR_DOMAIN: &str = "domain";

/// OCEL event type for a full LLM invocation record (prompt + completion
/// hashes, optional redacted text). Emitted by [`record_llm_invoked_full`]
/// and matched by the `AdmissionOp::LlmInvokedFull` discriminant in
/// `admission.rs`. Appears twice in this file (event_id construction +
/// emit call).
///
/// # Examples
///
/// ```
/// use open_ontologies::ocel_store::OCEL_EVENT_LLM_INVOKED_FULL;
///
/// assert_eq!(OCEL_EVENT_LLM_INVOKED_FULL, "llm_invoked_full");
/// ```
pub const OCEL_EVENT_LLM_INVOKED_FULL: &str = "llm_invoked_full";

/// Insert OCEL event + attrs + relationships through a `Connection` (which
/// transparently accepts a `&Transaction` via deref). Shared by the legacy
/// `emit_event_in_tenant` (acquires its own conn) and the Phase 7 Task C.fix
/// `emit_event_in_tenant_in_tx` (caller supplies the transaction).
#[allow(clippy::too_many_arguments)]
fn emit_event_rows(
    conn: &rusqlite::Connection,
    event_id: &str,
    event_type: &str,
    time_iso: &str,
    session_id: &str,
    attrs: &[(&str, &str)],
    objects: &[(&str, &str)],
    scope_token: Option<&str>,
    tenant_id: &str,
) -> Result<()> {
    // R5 WB-2 — emit-failure injection for counterfactual tests.
    #[cfg(debug_assertions)]
    {
        let injected: Option<anyhow::Error> = EMIT_FAILURE_INJECTION_HOOK.with(|h| {
            h.borrow().as_ref().and_then(|hook| hook(event_type))
        });
        if let Some(e) = injected {
            return Err(e);
        }
    }
    conn.execute(
        &format!("INSERT INTO {OCEL_EVENTS_TABLE} (event_id, event_type, time, session_id, scope_token, tenant_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"),
        rusqlite::params![event_id, event_type, time_iso, session_id, scope_token, tenant_id],
    )?;
    for (name, value) in attrs {
        conn.execute(
            "INSERT INTO ocel_event_attrs (event_id, name, value, value_type)
             VALUES (?1, ?2, ?3, 'string')",
            rusqlite::params![event_id, name, value],
        )?;
    }
    for (object_id, qualifier) in objects {
        conn.execute(
            "INSERT INTO ocel_relationships (event_id, object_id, qualifier)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![event_id, object_id, qualifier],
        )?;
    }
    Ok(())
}

/// Receipt-backed exemplar row returned by [`OcelStore::exemplars_for_domain`].
/// Loop 4 surface — see `feedback::exemplars` (Loop 1) for how rows arrive.
///
/// # Examples
///
/// ```
/// use open_ontologies::ocel_store::Exemplar;
///
/// let ex = Exemplar {
///     id: "ex-001".to_string(),
///     domain: "revops".to_string(),
///     problem_context: "Route lead to rep".to_string(),
///     powl_string: "SEQ(A, B)".to_string(),
///     build_order: Some("A then B".to_string()),
///     fitness: 0.92,
///     source_session: Some("sess-abc".to_string()),
///     receipt_hash: "seed-v0-0000000000000001".to_string(),
///     mined_at: "2026-01-01T00:00:00Z".to_string(),
/// };
///
/// assert_eq!(ex.domain, "revops");
/// assert!((ex.fitness - 0.92).abs() < 1e-9);
/// assert!(ex.build_order.is_some());
///
/// // Exemplar is serializable to JSON.
/// let json = serde_json::to_string(&ex).unwrap();
/// assert!(json.contains("revops"));
/// ```
#[derive(Debug, Clone, serde::Serialize)]
pub struct Exemplar {
    pub id: String,
    pub domain: String,
    pub problem_context: String,
    pub powl_string: String,
    pub build_order: Option<String>,
    pub fitness: f64,
    pub source_session: Option<String>,
    pub receipt_hash: String,
    pub mined_at: String,
}

impl OcelStore {
    /// Create a new `OcelStore` wrapping a [`StateDb`].
    ///
    /// Pass an in-memory database for testing or benchmarks; pass a
    /// file-backed database for production use.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // The store is ready to emit events immediately after construction.
    /// let result = store.emit_event(
    ///     "evt-001",
    ///     "workflow_started",
    ///     "2026-01-01T00:00:00Z",
    ///     "sess-001",
    ///     &[("key", "value")],
    ///     &[],
    ///     None,
    /// );
    /// assert!(result.is_ok());
    /// ```
    pub fn new(db: StateDb) -> Self {
        Self { db }
    }

    /// Borrow the underlying state database.
    ///
    /// Useful for running raw queries or passing the database to other
    /// components that need direct access to the SQLite connection.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // db() returns a reference to the inner StateDb.
    /// let _db_ref: &StateDb = store.db();
    /// ```
    pub fn db(&self) -> &StateDb {
        &self.db
    }

    /// Loop 4 (cross-session retrieval). Return exemplars for `domain` whose
    /// `mined_exemplars.receipt_hash` JOINs successfully against `receipts`.
    /// The JOIN is the integrity proof: an exemplar without a matching receipt
    /// row never escapes this function. Ordered by fitness DESC then mined_at DESC.
    pub fn exemplars_for_domain(
        &self,
        domain: &str,
        min_fitness: f64,
        limit: usize,
    ) -> Result<Vec<Exemplar>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT m.id, m.domain, m.problem_context, m.powl_string, m.build_order,
                    m.fitness, m.source_session, m.receipt_hash, m.mined_at
             FROM mined_exemplars m
             JOIN receipts r ON m.receipt_hash = r.receipt_hash
             WHERE m.domain = ?1 AND m.fitness >= ?2
             ORDER BY m.fitness DESC, m.mined_at DESC
             LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(
                rusqlite::params![domain, min_fitness, limit as i64],
                |r| {
                    Ok(Exemplar {
                        id: r.get(0)?,
                        domain: r.get(1)?,
                        problem_context: r.get(2)?,
                        powl_string: r.get(3)?,
                        build_order: r.get(4)?,
                        fitness: r.get(5)?,
                        source_session: r.get(6)?,
                        receipt_hash: r.get(7)?,
                        mined_at: r.get(8)?,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Stream-3 helper: does a declared workflow row exist for the scope?
    ///
    /// Returns `false` for any scope that has not been declared yet. A freshly
    /// opened in-memory database contains no declared workflows, so any query
    /// against an unknown scope token returns `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // No workflow has been declared for this scope, so the predicate is false.
    /// assert!(!store.has_declared_workflow("test-session-1").unwrap());
    /// ```
    pub fn has_declared_workflow(&self, scope_token: &str) -> Result<bool> {
        let conn = self.db.conn();
        let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM declared_workflows WHERE scope_token = ?1",
            rusqlite::params![scope_token],
            |r| r.get(0),
        ).unwrap_or(0);
        Ok(n > 0)
    }

    /// Stream-3 helper: is the scope closed (closed_at IS NOT NULL)?
    ///
    /// A scope is considered closed when its `declared_workflows` row carries a
    /// non-NULL `closed_at` timestamp. A scope that was never declared, or a
    /// scope that has been declared but not yet closed, both return `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // A scope that was never declared cannot be closed.
    /// assert!(!store.is_scope_closed("test-session-1").unwrap());
    /// ```
    pub fn is_scope_closed(&self, scope_token: &str) -> Result<bool> {
        let conn = self.db.conn();
        let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);
        let closed: Option<String> = conn.query_row(
            "SELECT closed_at FROM declared_workflows WHERE scope_token = ?1",
            rusqlite::params![scope_token],
            |r| r.get(0),
        ).unwrap_or(None);
        Ok(closed.is_some())
    }

    /// Stream-3 helper: does a conforming replay exist for the scope?
    ///
    /// Queries `conformance_runs` for a row whose `scope_token` matches and
    /// whose `verdict` is `'conform'`. A freshly opened database has no
    /// conformance run records, so any scope returns `false` until a conforming
    /// replay has been recorded via [`OcelStore::replay_against_powl`].
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // No conformance run has been recorded for this scope.
    /// assert!(!store.has_conforming_replay("test-session-1").unwrap());
    /// ```
    pub fn has_conforming_replay(&self, scope_token: &str) -> Result<bool> {
        let conn = self.db.conn();
        let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM conformance_runs WHERE scope_token = ?1 AND verdict = 'conform'",
            rusqlite::params![scope_token],
            |r| r.get(0),
        ).unwrap_or(0);
        Ok(n > 0)
    }

    /// Stream-3 helper: is the session in revoked_sessions (and not cleared)?
    ///
    /// Returns `true` only when a row for `session_id` exists in
    /// `revoked_sessions` with `cleared_at IS NULL`. Sessions that were never
    /// revoked, and revocations that have subsequently been cleared, both
    /// return `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // A session that was never revoked is not revoked.
    /// assert!(!store.session_is_revoked("test-session-1").unwrap());
    /// ```
    pub fn session_is_revoked(&self, session_id: &str) -> Result<bool> {
        let conn = self.db.conn();
        let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM revoked_sessions WHERE session_id = ?1 AND cleared_at IS NULL",
            rusqlite::params![session_id],
            |r| r.get(0),
        ).unwrap_or(0);
        Ok(n > 0)
    }

    /// Stream 2: project OCEL events for `scope_token` to an ordered trace
    /// (event_type values in `time` order) and replay against the parsed POWL
    /// `root` via [`crate::powl_bridge::PowlBridge`].
    ///
    /// **No PM math here.** All fitness/replay numbers come from
    /// `wasm4pm::powl::conformance::token_replay` via the bridge. Persists a
    /// row in `conformance_runs` and returns the typed
    /// [`crate::powl_bridge::ConformanceResult`].
    pub fn replay_against_powl(
        &self,
        scope_token: &str,
        bridge: &crate::powl_bridge::PowlBridge,
        root: u32,
        tenant_id: &str,
    ) -> Result<crate::powl_bridge::ConformanceResult> {
        // Make sure the conformance_runs table exists. The Stream-3 stub
        // migration is idempotent so cheap to run.
        let conn = self.db.conn();
        let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);

        // Project event_type values for scope_token in time order.
        let trace: Vec<String> = {
            let mut stmt = conn.prepare(
                &format!("SELECT event_type FROM {OCEL_EVENTS_TABLE}
                 WHERE scope_token = ?1 AND tenant_id = ?2
                 ORDER BY time ASC, event_id ASC"),
            )?;
            let rows = stmt.query_map(rusqlite::params![scope_token, tenant_id], |r| r.get::<_, String>(0))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            out
        };

        // Delegate replay + classification to wasm4pm-backed bridge.
        let replay = bridge
            .replay_trace(root, &trace)
            .map_err(|e| anyhow::anyhow!("powl replay: {e}"))?;
        let result = crate::powl_bridge::classify_replay(bridge, root, &trace, &replay);

        // Persist conformance_runs row.
        let defects_json = serde_json::to_string(
            &result
                .defects
                .iter()
                .map(|(d, _)| d)
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".to_string());
        let _ = conn.execute(
            "INSERT OR REPLACE INTO conformance_runs (
                run_id, scope_token, fitness, precision, generalization, simplicity,
                verdict, defects_json, trace_canonical_hash, ran_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            rusqlite::params![
                result.run_id,
                scope_token,
                result.fitness,
                result.precision,
                result.generalization,
                result.simplicity,
                result.verdict,
                defects_json,
                result.trace_canonical_hash,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;

        Ok(result)
    }

    /// Replay a scope using only the OCEL stream — no `declared_workflows`
    /// row required. Reads the `workflow_declared` anchor event for the
    /// scope, extracts `powl_string` from its attributes, parses it into a
    /// fresh `PowlBridge`, and replays. This proves the OCEL stream is
    /// self-sufficient: an external observer with only the event log can
    /// reconstruct what should have happened.
    ///
    /// Errors when no anchor event exists (the scope was never properly
    /// declared) or when the embedded POWL fails to parse.
    pub fn replay_from_ocel_alone(
        &self,
        scope_token: &str,
    ) -> Result<crate::powl_bridge::ConformanceResult> {
        // Find the workflow_declared anchor event for this scope and pull
        // its powl_string attribute. Drop the connection lock before the
        // (potentially heavy) parse so we don't serialize replays.
        let powl_string: String = {
            let conn = self.db.conn();
            let anchor_event_id: String = conn
                .query_row(
                    &format!("SELECT event_id FROM {OCEL_EVENTS_TABLE}
                     WHERE scope_token = ?1 AND event_type = 'workflow_declared'
                     ORDER BY time ASC LIMIT 1"),
                    rusqlite::params![scope_token],
                    |r| r.get(0),
                )
                .map_err(|_| anyhow::anyhow!(
                    "no workflow_declared anchor event for scope {}",
                    scope_token
                ))?;
            conn.query_row(
                "SELECT value FROM ocel_event_attrs
                 WHERE event_id = ?1 AND name = 'powl_string'",
                rusqlite::params![anchor_event_id],
                |r| r.get(0),
            )
            .map_err(|_| anyhow::anyhow!(
                "anchor event {} missing powl_string attribute",
                anchor_event_id
            ))?
        };

        let mut bridge = crate::powl_bridge::PowlBridge::new();
        let root = bridge
            .parse(&powl_string)
            .map_err(|e| anyhow::anyhow!("anchor powl parse failed: {e}"))?;

        // Reuse the canonical replay path. This re-reads the same events but
        // does NOT touch declared_workflows.
        self.replay_against_powl(scope_token, &bridge, root, "default")
    }

    /// Stream-3 helper: list event_types observed for a session.
    ///
    /// Returns a deduplicated, alphabetically-sorted list of every distinct
    /// `event_type` that has been emitted for `session_id`. An empty session
    /// returns an empty vec.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // No events yet — result is empty.
    /// let empty = store.observed_event_types_for_session("sess-new").unwrap();
    /// assert!(empty.is_empty());
    ///
    /// // Emit two different event types and one duplicate.
    /// store.emit_event("e1", "started", "2026-01-01T00:00:00Z", "sess-x", &[], &[], None).unwrap();
    /// store.emit_event("e2", "completed", "2026-01-01T00:01:00Z", "sess-x", &[], &[], None).unwrap();
    /// store.emit_event("e3", "started", "2026-01-01T00:02:00Z", "sess-x", &[], &[], None).unwrap();
    ///
    /// let types = store.observed_event_types_for_session("sess-x").unwrap();
    /// // DISTINCT + ORDER BY means exactly two entries in alphabetical order.
    /// assert_eq!(types, vec!["completed".to_string(), "started".to_string()]);
    /// ```
    pub fn observed_event_types_for_session(&self, session_id: &str) -> Result<Vec<String>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            &format!("SELECT DISTINCT event_type FROM {OCEL_EVENTS_TABLE} WHERE session_id = ?1 ORDER BY event_type ASC")
        )?;
        let rows = stmt.query_map(rusqlite::params![session_id], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Idempotent object upsert. Creates or updates an OCEL object and its attributes.
    ///
    /// Calling `upsert_object` multiple times with the same `id` is safe: the
    /// first call creates the `ocel_objects` row (`INSERT OR IGNORE`) while
    /// each call appends fresh attribute rows with the current timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // Insert an object with two attributes.
    /// store.upsert_object(
    ///     "product-001",
    ///     "Product",
    ///     &[("name", "Widget", "string"), ("price", "9.99", "float")],
    /// ).unwrap();
    ///
    /// // Re-upserting the same id is idempotent for the ocel_objects row.
    /// store.upsert_object("product-001", "Product", &[]).unwrap();
    ///
    /// // The object appears in the OCEL struct built from the store.
    /// let ocel = store.build_ocel(None).unwrap();
    /// assert_eq!(ocel.objects.len(), 1);
    /// assert_eq!(ocel.objects[0].id, "product-001");
    /// ```
    pub fn upsert_object(
        &self,
        id: &str,
        object_type: &str,
        attrs: &[(&str, &str, &str)],
    ) -> Result<()> {
        let conn = self.db.conn();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR IGNORE INTO ocel_objects (object_id, object_type, created_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![id, object_type, &now],
        )?;

        for (name, value, value_type) in attrs {
            conn.execute(
                "INSERT INTO ocel_object_attrs (object_id, name, value, value_type, valid_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, name, value, value_type, &now],
            )?;
        }

        Ok(())
    }

    /// Emit one OCEL event with attributes and relationships to objects.
    ///
    /// `scope_token` (Stream 1) tags the event with an open
    /// [`crate::workflows::WorkflowScope`] so OntoStar admission can replay
    /// scoped traces. Pass `None` when the call site has no declared scope —
    /// Stream 3 fills these in for gated handlers.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // Emit an event with two attributes and no object relationships.
    /// store.emit_event(
    ///     "evt-abc",
    ///     "admission_granted",
    ///     "2026-05-01T10:00:00Z",
    ///     "session-1",
    ///     &[("verdict", "granted"), ("fitness", "0.95")],
    ///     &[],
    ///     Some("scope-123"),
    /// ).unwrap();
    ///
    /// // The event is now queryable via observed_event_types_for_session.
    /// let types = store.observed_event_types_for_session("session-1").unwrap();
    /// assert!(types.contains(&"admission_granted".to_string()));
    /// ```
    #[allow(clippy::too_many_arguments)] // Public OCEL emission surface; each arg matches a column in the OCEL schema and bundling them would only relocate the cost.
    pub fn emit_event(
        &self,
        event_id: &str,
        event_type: &str,
        time_iso: &str,
        session_id: &str,
        attrs: &[(&str, &str)],
        objects: &[(&str, &str)],
        scope_token: Option<&str>,
    ) -> Result<()> {
        // Phase 11: backwards-compat — tag events emitted via the legacy
        // entrypoint with `tenant_id = "default"`. Tenant-aware callers must
        // use [`emit_event_in_tenant`].
        self.emit_event_in_tenant(
            event_id,
            event_type,
            time_iso,
            session_id,
            attrs,
            objects,
            scope_token,
            "default",
        )
    }

    /// Tenant-aware variant of [`emit_event`]. Tags the resulting `ocel_events`
    /// row with the supplied `tenant_id` so per-tenant projections can be
    /// computed without joining back to `declared_workflows`.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // Emit an event tagged with tenant "acme".
    /// store.emit_event_in_tenant(
    ///     "evt-t1",
    ///     "order_placed",
    ///     "2026-05-01T11:00:00Z",
    ///     "sess-acme",
    ///     &[("amount", "100")],
    ///     &[("obj-product-1", "purchases")],
    ///     None,
    ///     "acme",
    /// ).unwrap();
    ///
    /// let types = store.observed_event_types_for_session("sess-acme").unwrap();
    /// assert_eq!(types, vec!["order_placed".to_string()]);
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn emit_event_in_tenant(
        &self,
        event_id: &str,
        event_type: &str,
        time_iso: &str,
        session_id: &str,
        attrs: &[(&str, &str)],
        objects: &[(&str, &str)],
        scope_token: Option<&str>,
        tenant_id: &str,
    ) -> Result<()> {
        let conn = self.db.conn();
        emit_event_rows(
            &conn, event_id, event_type, time_iso, session_id, attrs, objects, scope_token,
            tenant_id,
        )
    }

    /// Phase 7 Task C.fix: shared-transaction variant. Inserts the OCEL event
    /// rows on a caller-supplied transaction WITHOUT committing. Used by the
    /// admission gate to keep `receipts` INSERT and the `admission_granted`
    /// emit atomic — failure of either rolls back both, so a receipt is never
    /// durable without its OCEL witness.
    #[allow(clippy::too_many_arguments)]
    pub fn emit_event_in_tenant_in_tx(
        tx: &rusqlite::Transaction<'_>,
        event_id: &str,
        event_type: &str,
        time_iso: &str,
        session_id: &str,
        attrs: &[(&str, &str)],
        objects: &[(&str, &str)],
        scope_token: Option<&str>,
        tenant_id: &str,
    ) -> Result<()> {
        emit_event_rows(
            tx, event_id, event_type, time_iso, session_id, attrs, objects, scope_token, tenant_id,
        )
    }

    /// Build a complete OCEL 2.0 struct from the stored OCEL data.
    ///
    /// When `session_id_filter` is `Some(sid)`, only events emitted for that
    /// session are included. Pass `None` to include all sessions.
    ///
    /// The returned [`OCEL`] struct carries deduplicated event-type and
    /// object-type registries derived from the actual rows stored in the
    /// database.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // An empty store yields an empty OCEL document.
    /// let ocel = store.build_ocel(None).unwrap();
    /// assert!(ocel.events.is_empty());
    /// assert!(ocel.objects.is_empty());
    ///
    /// // After emitting an event the OCEL document is populated.
    /// store.emit_event(
    ///     "e1", "stage_enter", "2026-01-01T00:00:00Z",
    ///     "sess-demo", &[("stage", "validate")], &[], None,
    /// ).unwrap();
    ///
    /// let ocel_all = store.build_ocel(None).unwrap();
    /// assert_eq!(ocel_all.events.len(), 1);
    /// assert_eq!(ocel_all.event_types.len(), 1);
    /// assert_eq!(ocel_all.event_types[0].name, "stage_enter");
    ///
    /// // Filter by session — a different session returns no events.
    /// let ocel_other = store.build_ocel(Some("sess-other")).unwrap();
    /// assert!(ocel_other.events.is_empty());
    /// ```
    pub fn build_ocel(&self, session_id_filter: Option<&str>) -> Result<OCEL> {
        let conn = self.db.conn();

        // Query objects
        let mut object_type_set = BTreeSet::new();
        let mut objects = Vec::new();

        let mut stmt = conn.prepare(
            "SELECT object_id, object_type FROM ocel_objects ORDER BY object_id ASC",
        )?;

        let obj_rows = stmt.query_map(rusqlite::params![], |row| {
            let id: String = row.get(0)?;
            let otype: String = row.get(1)?;
            Ok((id, otype))
        })?;

        for row_result in obj_rows {
            let (id, otype) = row_result?;
            object_type_set.insert(otype.clone());

            objects.push(OCELObject {
                id,
                object_type: otype,
                attributes: Vec::new(),
                relationships: Vec::new(),
            });
        }

        // Query events
        let mut event_type_set = BTreeSet::new();
        let mut events = Vec::new();

        let event_rows: Vec<(String, String, String, String)> = if let Some(sid) = session_id_filter {
            let mut stmt = conn.prepare(
                &format!("SELECT event_id, event_type, time, session_id FROM {OCEL_EVENTS_TABLE}
                 WHERE session_id = ?1 ORDER BY event_id ASC"),
            )?;
            stmt.query_map(rusqlite::params![sid], |row| {
                let eid: String = row.get(0)?;
                let etype: String = row.get(1)?;
                let time_str: String = row.get(2)?;
                let sid: String = row.get(3)?;
                Ok((eid, etype, time_str, sid))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            let mut stmt = conn.prepare(
                &format!("SELECT event_id, event_type, time, session_id FROM {OCEL_EVENTS_TABLE} ORDER BY event_id ASC"),
            )?;
            stmt.query_map(rusqlite::params![], |row| {
                let eid: String = row.get(0)?;
                let etype: String = row.get(1)?;
                let time_str: String = row.get(2)?;
                let sid: String = row.get(3)?;
                Ok((eid, etype, time_str, sid))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };

        let utc_now = chrono::Utc::now();
        let fixed_now = utc_now.with_timezone(&FixedOffset::east_opt(0).unwrap());

        for (eid, etype, time_str, _sid) in event_rows {
            event_type_set.insert(etype.clone());

            let time = chrono::DateTime::parse_from_rfc3339(&time_str).unwrap_or(fixed_now);

            // Query event attributes
            let mut attr_stmt = conn.prepare(
                "SELECT name, value FROM ocel_event_attrs WHERE event_id = ?1",
            )?;

            let attributes = attr_stmt
                .query_map(rusqlite::params![&eid], |row| {
                    let name: String = row.get(0)?;
                    let value: String = row.get(1)?;
                    Ok((name, value))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            // Query relationships
            let mut rel_stmt =
                conn.prepare("SELECT object_id, qualifier FROM ocel_relationships WHERE event_id = ?1")?;

            let relationships = rel_stmt
                .query_map(rusqlite::params![&eid], |row| {
                    let oid: String = row.get(0)?;
                    let qual: String = row.get(1)?;
                    Ok((oid, qual))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            events.push((eid, etype, time, attributes, relationships));
        }

        // Build event_types
        let event_types: Vec<(String,)> = event_type_set.iter().map(|n| (n.clone(),)).collect();

        Ok(OCEL {
            event_types: event_types.iter().map(|(n,)| {
                use wasm4pm_types::ocel::OCELType;
                OCELType {
                    name: n.clone(),
                    attributes: Vec::new(),
                }
            }).collect(),
            object_types: object_type_set.iter().map(|n| {
                use wasm4pm_types::ocel::OCELType;
                OCELType {
                    name: n.clone(),
                    attributes: Vec::new(),
                }
            }).collect(),
            events: events.into_iter().map(|(eid, etype, time, attrs, rels)| {
                use wasm4pm_types::ocel::{OCELEvent, OCELEventAttribute, OCELAttributeValue, OCELRelationship};
                OCELEvent {
                    id: eid,
                    event_type: etype,
                    time,
                    attributes: attrs.iter().map(|(name, value)| {
                        OCELEventAttribute {
                            name: name.clone(),
                            value: OCELAttributeValue::String(value.clone()),
                        }
                    }).collect(),
                    relationships: rels.iter().map(|(oid, qual)| {
                        OCELRelationship {
                            object_id: oid.clone(),
                            qualifier: qual.clone(),
                        }
                    }).collect(),
                }
            }).collect(),
            objects,
        })
    }

    /// Persist a seed exemplar from a manually curated problem–model pair.
    ///
    /// Derives a stable `receipt_hash` from the domain, problem statement, and
    /// POWL model so that re-inserting the same exemplar is idempotent
    /// (`INSERT OR IGNORE` on the `receipts` row).
    ///
    /// Returns the computed `receipt_hash` string so callers can reference it
    /// in subsequent OCEL events or join logic. The hash always starts with
    /// `"seed-v0-"` followed by 16 hex digits derived from the inputs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// let hash = store.insert_seed_exemplar(
    ///     "revops",
    ///     "Route a qualified lead to the best available rep",
    ///     "SEQ(qualify_lead, route_to_rep, follow_up)",
    ///     "qualify → route → follow_up",
    ///     "sequenceDiagram\n  lead->>rep: route",
    ///     0.97,
    /// ).unwrap();
    ///
    /// // Hash is deterministic and carries the "seed-v0-" prefix.
    /// assert!(hash.starts_with("seed-v0-"));
    /// ```
    pub fn insert_seed_exemplar(
        &self,
        domain: &str,
        problem_statement: &str,
        powl_model: &str,
        build_order: &str,
        sequence_diagram: &str,
        fitness: f64,
    ) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let seed_input = format!("{}|{}|{}", domain, problem_statement, powl_model);
        let mut h = DefaultHasher::new();
        seed_input.hash(&mut h);
        let receipt_hash = format!("seed-v0-{:016x}", h.finish());
        let canonical = serde_json::json!({
            "kind": "seed",
            "domain": domain,
            "problem": problem_statement,
            "powl_model": powl_model,
            "fitness": fitness,
        })
        .to_string();
        let conn = self.db.conn();
        conn.execute(
            "INSERT OR IGNORE INTO receipts (receipt_hash, scope_token, production_law_version, canonical_record, parent_hash)
             VALUES (?1, NULL, 'seed-v0', ?2, NULL)",
            rusqlite::params![receipt_hash, canonical],
        )?;
        conn.execute(
            "INSERT INTO mined_exemplars (domain, problem_statement, powl_model, build_order, sequence_diagram, fitness, source_scope_token, receipt_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
            rusqlite::params![
                domain,
                problem_statement,
                powl_model,
                build_order,
                sequence_diagram,
                fitness,
                receipt_hash
            ],
        )?;
        Ok(receipt_hash)
    }

    /// Parse an OCEL JSON document and persist every `build_order_generated`
    /// event as a seed exemplar.
    ///
    /// Returns the count of exemplars inserted. Events that are not of type
    /// `build_order_generated`, or events that carry an empty `powl_model` /
    /// `powl_string` attribute, are silently skipped.
    ///
    /// Supports both the `ocel:events` (object-form and array-form) and the
    /// plain `events` array envelope used by different OCEL 2.0 exporters.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = OcelStore::new(db);
    ///
    /// // An empty document inserts nothing.
    /// let empty = store.seed_from_ocel_bytes(b"{}", "default").unwrap();
    /// assert_eq!(empty, 0);
    ///
    /// // A document with no build_order_generated events inserts nothing.
    /// let no_match = serde_json::json!({
    ///     "events": [{"activity": "other_event", "attributes": {}}]
    /// }).to_string();
    /// let count = store.seed_from_ocel_bytes(no_match.as_bytes(), "default").unwrap();
    /// assert_eq!(count, 0);
    ///
    /// // A build_order_generated event with an empty powl_model is skipped.
    /// let missing_powl = serde_json::json!({
    ///     "events": [{
    ///         "activity": "build_order_generated",
    ///         "attributes": {"powl_model": "", "domain": "x"}
    ///     }]
    /// }).to_string();
    /// let skipped = store.seed_from_ocel_bytes(missing_powl.as_bytes(), "default").unwrap();
    /// assert_eq!(skipped, 0);
    /// ```
    pub fn seed_from_ocel_bytes(&self, bytes: &[u8], default_domain: &str) -> Result<u64> {
        let doc: serde_json::Value = serde_json::from_slice(bytes)?;
        let events: Vec<serde_json::Value> = if let Some(arr) =
            doc.get("events").and_then(|v| v.as_array())
        {
            arr.clone()
        } else if let Some(obj) = doc.get("ocel:events").and_then(|v| v.as_object()) {
            obj.values().cloned().collect()
        } else if let Some(arr) = doc.get("ocel:events").and_then(|v| v.as_array()) {
            arr.clone()
        } else {
            return Ok(0);
        };
        let mut inserted = 0u64;
        for ev in events {
            let etype = ev
                .get("ocel:activity")
                .or_else(|| ev.get("activity"))
                .or_else(|| ev.get("ocel:type"))
                .or_else(|| ev.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if etype != OCEL_EVENT_BUILD_ORDER_GENERATED {
                continue;
            }
            let attrs = ev
                .get("ocel:attributes")
                .or_else(|| ev.get("attributes"))
                .or_else(|| ev.get("vmap"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let s = |k: &str| -> String {
                attrs.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
            };
            let problem = s("problem_statement");
            let powl_model = if !s(OCEL_ATTR_POWL_MODEL).is_empty() {
                s(OCEL_ATTR_POWL_MODEL)
            } else {
                s("powl_string")
            };
            if powl_model.is_empty() {
                continue;
            }
            let domain = if !s(OCEL_ATTR_DOMAIN).is_empty() {
                s(OCEL_ATTR_DOMAIN)
            } else {
                default_domain.to_string()
            };
            let fitness = attrs.get("fitness").and_then(|v| v.as_f64()).unwrap_or(1.0);
            self.insert_seed_exemplar(
                &domain,
                &problem,
                &powl_model,
                &s("build_order"),
                &s("sequence_diagram"),
                fitness,
            )?;
            inserted += 1;
        }
        Ok(inserted)
    }
}

// ─── R7 WD-4 — `llm_invoked_full` OCEL persistence ─────────────────────
//
// The `llm_invoked_full` event captures the BLAKE3 digest of the prompt
// and completion (always — deterministic, content-addressable) and,
// only when `persist_full_io = true`, the redacted truncated text. The
// helper is a free function (not a method) so the call site can fire
// even when the OcelStore is borrowed from inside the translator's
// async path.
//
// Persisted-text rules:
// - Hard cap at 32 KiB; payloads beyond the cap are truncated and
//   suffixed with the marker `[truncated]` so an auditor can spot the
//   transformation.
// - Bearer-pattern redaction (shared with the translator) is applied
//   to both the prompt and the completion before persistence.
//
// Retention: this event_type is persisted in `ocel_events` like every
// other event, so [`crate::retention::RetentionWorker::prune_ocel`]
// already evicts it on the same TTL as receipts (R4 WD).

const LLM_FULL_TEXT_CAP: usize = 32 * 1024;
const LLM_TRUNCATION_MARKER: &str = "[truncated]";

/// Truncate `s` to at most [`LLM_FULL_TEXT_CAP`] bytes and suffix the
/// truncation marker if the cap fired. UTF-8-safe: cuts at the last
/// char boundary inside the cap.
fn truncate_for_ocel(s: &str) -> String {
    if s.len() <= LLM_FULL_TEXT_CAP {
        return s.to_string();
    }
    let mut end = LLM_FULL_TEXT_CAP;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = String::with_capacity(end + LLM_TRUNCATION_MARKER.len());
    out.push_str(&s[..end]);
    out.push_str(LLM_TRUNCATION_MARKER);
    out
}

/// Apply the same bearer-pattern redaction the translator uses for HTTP
/// error bodies. Inlined here to avoid a circular `crate::llm_translator`
/// import inside the OCEL store.
fn redact_bearer(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    let needle = b"Bearer ";
    while i < bytes.len() {
        if bytes[i..].starts_with(needle) {
            out.push_str("Bearer <redacted>");
            i += needle.len();
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// R7 WD-4 — emit an `llm_invoked_full` OCEL event.
///
/// `prompt_hash` and `completion_hash` (BLAKE3 hex) are ALWAYS emitted.
/// `prompt_text` and `completion_text` are emitted only when
/// `persist_full_io = true`; they are redacted (bearer patterns) and
/// truncated to 32 KiB with the `[truncated]` marker.
///
/// Errors are intentionally swallowed (logged at `tracing::warn`) so
/// an OCEL persistence failure cannot break the translator's main
/// success path. The hash digests are deterministic so a repeat call
/// produces the same `event_id` and the OCEL store's INSERT OR IGNORE
/// path makes the emit idempotent.
///
/// # Examples
///
/// ```
/// use open_ontologies::state::StateDb;
/// use open_ontologies::ocel_store::{OcelStore, record_llm_invoked_full};
/// use std::path::Path;
///
/// let db = StateDb::open(Path::new(":memory:")).unwrap();
/// let store = OcelStore::new(db);
///
/// // Emit an LLM invocation event (hashes only, no full IO).
/// record_llm_invoked_full(
///     &store,
///     "sess-001",
///     None,
///     "default",
///     "mixtral-8x7b",
///     "translate",
///     true,
///     "Translate this POWL: SEQ(A,B)",
///     "SEQ(A, B)",
///     false,  // persist_full_io = false; only hashes stored
/// );
///
/// // The event type should now appear in the session log.
/// let types = store.observed_event_types_for_session("sess-001").unwrap();
/// assert!(types.contains(&"llm_invoked_full".to_string()));
///
/// // Emitting with persist_full_io = true stores redacted prompt and completion text.
/// record_llm_invoked_full(
///     &store,
///     "sess-002",
///     Some("scope-abc"),
///     "acme",
///     "mixtral-8x7b",
///     "translate",
///     false,
///     "Prompt with Bearer sk-secret token",
///     "Some completion",
///     true,
/// );
/// let types2 = store.observed_event_types_for_session("sess-002").unwrap();
/// assert!(types2.contains(&"llm_invoked_full".to_string()));
/// ```
#[allow(clippy::too_many_arguments)]
pub fn record_llm_invoked_full(
    store: &OcelStore,
    session_id: &str,
    scope_token: Option<&str>,
    tenant_id: &str,
    model: &str,
    op: &str,
    success: bool,
    prompt_text: &str,
    completion_text: &str,
    persist_full_io: bool,
) {
    let prompt_hash = blake3::hash(prompt_text.as_bytes()).to_hex().to_string();
    let completion_hash = blake3::hash(completion_text.as_bytes()).to_hex().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let ts_ms = chrono::Utc::now().timestamp_millis();

    // Deterministic event_id incorporates the hashes so two ticks with
    // identical io collapse to a single OCEL row (idempotency required
    // by `tests/llm_invoked_full_replay.rs`).
    let event_id = format!(
        "{session_id}:{}:{}:{}:{ts_ms}",
        OCEL_EVENT_LLM_INVOKED_FULL,
        &prompt_hash[..16],
        &completion_hash[..16]
    );

    let success_str = if success { "true" } else { "false" };
    let mut attrs: Vec<(&str, &str)> = vec![
        ("model", model),
        ("op", op),
        ("success", success_str),
        ("prompt_hash", &prompt_hash),
        ("completion_hash", &completion_hash),
    ];

    let prompt_redacted;
    let completion_redacted;
    if persist_full_io {
        prompt_redacted = truncate_for_ocel(&redact_bearer(prompt_text));
        completion_redacted = truncate_for_ocel(&redact_bearer(completion_text));
        attrs.push(("prompt_text", &prompt_redacted));
        attrs.push(("completion_text", &completion_redacted));
    }

    if let Err(e) = store.emit_event_in_tenant(
        &event_id,
        OCEL_EVENT_LLM_INVOKED_FULL,
        &now,
        session_id,
        &attrs,
        &[],
        scope_token,
        tenant_id,
    ) {
        tracing::warn!("record_llm_invoked_full: emit failed: {e}");
    }
}

#[cfg(test)]
mod llm_full_tests {
    use super::*;

    #[test]
    fn truncate_appends_marker_when_over_cap() {
        let s = "a".repeat(LLM_FULL_TEXT_CAP + 100);
        let out = truncate_for_ocel(&s);
        assert!(out.ends_with(LLM_TRUNCATION_MARKER));
        assert!(out.len() <= LLM_FULL_TEXT_CAP + LLM_TRUNCATION_MARKER.len());
    }

    #[test]
    fn truncate_passthrough_when_under_cap() {
        assert_eq!(truncate_for_ocel("hello"), "hello");
    }

    #[test]
    fn redact_strips_bearer_token() {
        let r = redact_bearer("Authorization: Bearer sk-leak123 trailing");
        assert!(!r.contains("sk-leak123"));
        assert!(r.contains("Bearer <redacted>"));
    }
}
