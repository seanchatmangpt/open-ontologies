use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ontology_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    label TEXT NOT NULL,
    triple_count INTEGER NOT NULL,
    content TEXT NOT NULL,
    format TEXT NOT NULL DEFAULT 'ntriples',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS monitor_watchers (
    id TEXT PRIMARY KEY,
    check_type TEXT NOT NULL,
    threshold REAL NOT NULL DEFAULT 0.0,
    severity TEXT NOT NULL DEFAULT 'warning',
    action TEXT NOT NULL DEFAULT 'notify',
    query TEXT,
    message TEXT,
    webhook_url TEXT,
    webhook_headers TEXT,
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS monitor_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS drift_feedback (
    id TEXT PRIMARY KEY,
    from_iri TEXT NOT NULL,
    to_iri TEXT NOT NULL,
    predicted TEXT NOT NULL,
    confidence REAL NOT NULL,
    actual TEXT,
    signal_domain_range INTEGER NOT NULL DEFAULT 0,
    signal_label_sim REAL NOT NULL DEFAULT 0.0,
    signal_hierarchy INTEGER NOT NULL DEFAULT 0,
    signal_individuals INTEGER NOT NULL DEFAULT 0,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS iri_locks (
    iri TEXT PRIMARY KEY,
    locked_at TEXT NOT NULL DEFAULT (datetime('now')),
    reason TEXT
);

CREATE TABLE IF NOT EXISTS lineage_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    operation TEXT NOT NULL,
    details TEXT
);

CREATE TABLE IF NOT EXISTS enforce_rules (
    id TEXT PRIMARY KEY,
    rule_pack TEXT NOT NULL,
    query TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'warning',
    message TEXT,
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_lineage_session ON lineage_events(session_id);
CREATE INDEX IF NOT EXISTS idx_lineage_seq ON lineage_events(session_id, seq);
CREATE INDEX IF NOT EXISTS idx_enforce_pack ON enforce_rules(rule_pack);

CREATE TABLE IF NOT EXISTS align_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_iri TEXT NOT NULL,
    target_iri TEXT NOT NULL,
    predicted_relation TEXT NOT NULL,
    accepted INTEGER NOT NULL,
    signals_json TEXT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_align_feedback_iris ON align_feedback(source_iri, target_iri);

CREATE TABLE IF NOT EXISTS tool_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool TEXT NOT NULL,
    rule_id TEXT NOT NULL,
    entity TEXT NOT NULL,
    accepted INTEGER NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_tool_feedback ON tool_feedback(tool, rule_id, entity);

CREATE TABLE IF NOT EXISTS embeddings (
    iri TEXT PRIMARY KEY,
    text_vec BLOB NOT NULL,
    struct_vec BLOB NOT NULL,
    text_dim INTEGER NOT NULL,
    struct_dim INTEGER NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Compile cache for loaded ontology files. One row per ontology `name`.
-- See src/cache.rs for the validity policy.
CREATE TABLE IF NOT EXISTS ontology_cache (
    name TEXT PRIMARY KEY,
    source_path TEXT NOT NULL,
    source_mtime INTEGER NOT NULL,
    source_size INTEGER NOT NULL,
    source_sha TEXT NOT NULL,
    cache_path TEXT NOT NULL,
    triple_count INTEGER NOT NULL,
    compiled_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_access_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Object-Centric Event Log (OCEL) tables for native OCEL emission
CREATE TABLE IF NOT EXISTS ocel_objects (
    object_id   TEXT PRIMARY KEY,
    object_type TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ocel_object_attrs (
    object_id  TEXT NOT NULL,
    name       TEXT NOT NULL,
    value      TEXT NOT NULL,
    value_type TEXT NOT NULL DEFAULT 'string',
    valid_at   TEXT NOT NULL,
    PRIMARY KEY (object_id, name, valid_at)
);

CREATE TABLE IF NOT EXISTS ocel_events (
    event_id   TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    time       TEXT NOT NULL,
    session_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ocel_event_attrs (
    event_id   TEXT NOT NULL,
    name       TEXT NOT NULL,
    value      TEXT NOT NULL,
    value_type TEXT NOT NULL DEFAULT 'string',
    PRIMARY KEY (event_id, name)
);

CREATE TABLE IF NOT EXISTS ocel_relationships (
    event_id  TEXT NOT NULL,
    object_id TEXT NOT NULL,
    qualifier TEXT NOT NULL,
    PRIMARY KEY (event_id, object_id, qualifier)
);

-- ─── OntoStar Stream 1 stub migrations (authoritative copies live in Stream 1) ──
-- Receipts for admitted manufactured artifacts (Stream 3 owns inserts).
CREATE TABLE IF NOT EXISTS receipts (
    receipt_hash           TEXT PRIMARY KEY,
    scope_token            TEXT NOT NULL,
    artifact_hash          TEXT NOT NULL,
    declared_powl_hash     TEXT NOT NULL,
    ocel_canonical_hash    TEXT NOT NULL,
    gate_config_hash       TEXT NOT NULL,
    prior_receipt_hash     TEXT,
    production_law_version TEXT NOT NULL,
    granted_at             TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS conformance_runs (
    run_id              TEXT PRIMARY KEY,
    scope_token         TEXT NOT NULL,
    workflow_class      TEXT,
    fitness             REAL,
    precision           REAL,
    generalization      REAL,
    simplicity          REAL,
    verdict             TEXT NOT NULL,
    defects_json        TEXT NOT NULL DEFAULT '[]',
    trace_canonical_hash TEXT NOT NULL DEFAULT '',
    ran_at              TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS mined_exemplars (
    id              TEXT PRIMARY KEY,
    domain          TEXT NOT NULL,
    problem_context TEXT NOT NULL,
    powl_string     TEXT NOT NULL,
    fitness         REAL NOT NULL,
    source_session  TEXT,
    receipt_hash    TEXT NOT NULL,
    mined_at        TEXT NOT NULL,
    promoted        INTEGER DEFAULT 0,
    build_order     TEXT
);

CREATE TABLE IF NOT EXISTS workflow_thresholds (
    workflow_class       TEXT PRIMARY KEY,
    precision_threshold  REAL NOT NULL,
    fitness_threshold    REAL NOT NULL,
    sample_count         INTEGER NOT NULL,
    updated_at           TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS discovered_workflows (
    id                  TEXT PRIMARY KEY,
    domain              TEXT NOT NULL,
    powl_string         TEXT NOT NULL,
    discovered_fitness  REAL NOT NULL,
    declared_fitness    REAL NOT NULL,
    status              TEXT NOT NULL,
    suggested_at        TEXT NOT NULL,
    decided_at          TEXT
);

CREATE INDEX IF NOT EXISTS idx_mined_exemplars_domain ON mined_exemplars(domain);
CREATE INDEX IF NOT EXISTS idx_discovered_workflows_status ON discovered_workflows(status);

-- R5 WC-1 — §28 HiddenWIP closure: one-shot bootstrap-window enforcement.
--
-- The legacy `OPEN_ONTOLOGIES_BOOTSTRAP_MODE` env var (and the
-- receipt-count heuristic) are volatile: a retention worker that prunes
-- seed receipts can silently re-open the bootstrap window. This table
-- enforces one-shot semantics at the DB level:
--
--   * `id INTEGER PRIMARY KEY DEFAULT 1, CHECK (id = 1)` — single-row
--     enforcement; only one row can ever exist.
--   * Inserted (idempotently via `INSERT OR IGNORE`) on the first
--     non-`seed-v0` receipt persisted by `receipts::persist_with_tenant_in_tx`.
--   * Read by `BootstrapState::is_bootstrap` — if this row exists the
--     window is CLOSED, regardless of receipt counts or env vars.
--
-- DO NOT add to RetentionWorker pruning — this is one-shot enforcement
-- state. Pruning it would re-open the bootstrap window and reintroduce
-- the §28 hidden-WIP leak. (Verified: no DELETE statement in
-- src/retention.rs targets `bootstrap_lock`.)
CREATE TABLE IF NOT EXISTS bootstrap_lock (
    id        INTEGER PRIMARY KEY DEFAULT 1,
    locked_at TEXT NOT NULL,
    locked_by TEXT NOT NULL,
    CHECK (id = 1)
);
";

/// Minimal SQLite state store for ontology versioning.
#[derive(Clone)]
pub struct StateDb {
    conn: Arc<Mutex<Connection>>,
}

impl StateDb {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(SCHEMA)?;
        // Safe migration: add webhook columns if upgrading from older schema
        let _ = conn.execute_batch(
            "ALTER TABLE monitor_watchers ADD COLUMN webhook_url TEXT;
             ALTER TABLE monitor_watchers ADD COLUMN webhook_headers TEXT;"
        );
        // Safe migration: add signals_json column for feedback-based weight learning
        let _ = conn.execute_batch(
            "ALTER TABLE align_feedback ADD COLUMN signals_json TEXT;"
        );
        // OntoStar Stream 4: defensive ALTERs for older databases that may
        // have created `conformance_runs` / `mined_exemplars` before Stream 4
        // landed. Wrapped in `let _ =` because re-applying is idempotent.
        let _ = conn.execute_batch(
            "ALTER TABLE conformance_runs ADD COLUMN workflow_class TEXT;"
        );
        let _ = conn.execute_batch(
            "ALTER TABLE mined_exemplars ADD COLUMN build_order TEXT;"
        );
        let _ = conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_conformance_runs_workflow
                 ON conformance_runs(workflow_class, ran_at);"
        );
        // OntoStar Stream 3: receipts.session_id for per-session chaining.
        // Stream 1's `receipts` table has no session_id column; we ALTER it in
        // additively so cell_ready/admission can chain per session.
        let _ = conn.execute_batch(
            "ALTER TABLE receipts ADD COLUMN session_id TEXT NOT NULL DEFAULT '';"
        );
        // Task C: per-session monotonic sequence column for deterministic chain
        // ordering. The ALTER is wrapped in `let _` so it is idempotent on
        // databases that already have the column (SQLite returns
        // "duplicate column name"). The UPDATE backfill is run unconditionally
        // but is safe — it only touches rows where sequence = 0 (the default).
        let _ = conn.execute_batch(
            "ALTER TABLE receipts ADD COLUMN sequence INTEGER NOT NULL DEFAULT 0;"
        );
        // Backfill BEFORE creating the unique index so legacy rows (all
        // defaulted to sequence=0) get distinct values per session first.
        let _ = conn.execute_batch(
            "UPDATE receipts SET sequence = (
               SELECT COUNT(*) FROM receipts r2
                WHERE r2.session_id = receipts.session_id
                  AND r2.granted_at < receipts.granted_at
             ) + 1 WHERE sequence = 0;"
        );
        let _ = conn.execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS receipts_session_sequence_uniq
                 ON receipts(session_id, sequence);
             CREATE INDEX IF NOT EXISTS receipts_session_seq_desc
                 ON receipts(session_id, sequence DESC);"
        );
        // OntoStar Stream 1: workflow scope, OCEL scope tagging, revoked sessions.
        // Run as separate batches so additive ALTER on an existing DB does not
        // poison sibling CREATEs.
        let _ = conn.execute_batch(
            "ALTER TABLE ocel_events ADD COLUMN scope_token TEXT;"
        );
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_ocel_scope ON ocel_events(scope_token);

             CREATE TABLE IF NOT EXISTS declared_workflows (
                 scope_token   TEXT PRIMARY KEY,
                 session_id    TEXT NOT NULL,
                 name          TEXT NOT NULL,
                 powl_string   TEXT NOT NULL,
                 powl_hash     TEXT NOT NULL,
                 alphabet_json TEXT NOT NULL,
                 declared_at   TEXT NOT NULL,
                 closed_at     TEXT,
                 status        TEXT NOT NULL
             );

             CREATE TABLE IF NOT EXISTS revoked_sessions (
                 session_id  TEXT PRIMARY KEY,
                 reason      TEXT NOT NULL,
                 revoked_at  TEXT NOT NULL,
                 cleared_at  TEXT
             );

             CREATE INDEX IF NOT EXISTS idx_declared_workflows_session
                 ON declared_workflows(session_id, status);"
        )?;

        // OntoStar Level 5 — capability evidence + per-scope outcome columns.
        // ALTER TABLE … ADD COLUMN runs are additive on existing DBs and
        // idempotent across sessions (errors on existing column are ignored).
        for stmt in [
            "ALTER TABLE declared_workflows ADD COLUMN admitted INTEGER;",
            "ALTER TABLE declared_workflows ADD COLUMN fitness REAL;",
            "ALTER TABLE declared_workflows ADD COLUMN precision REAL;",
            "ALTER TABLE declared_workflows ADD COLUMN defects_json TEXT;",
            "ALTER TABLE declared_workflows ADD COLUMN deviations_json TEXT;",
            "ALTER TABLE declared_workflows ADD COLUMN gates_fired_json TEXT;",
            "ALTER TABLE declared_workflows ADD COLUMN gates_denied_json TEXT;",
            "ALTER TABLE declared_workflows ADD COLUMN naked_craft_verdict TEXT DEFAULT 'granted_by_force';",
            "ALTER TABLE declared_workflows ADD COLUMN manufacturing_delta_json TEXT;",
            "ALTER TABLE declared_workflows ADD COLUMN admission_decided_at TEXT;",
        ] {
            let _ = conn.execute_batch(stmt);
        }

        // workflow_scopes view: Stream 5 handlers reference this name; alias to
        // declared_workflows so we keep one canonical table. The `domain`
        // column is extracted from alphabet_json's optional `domain` field.
        // Drop and recreate the view so it picks up the tenant_id column when
        // upgrading from a pre-Phase-11 database. CREATE VIEW IF NOT EXISTS
        // would otherwise leave the legacy schema in place.
        let _ = conn.execute_batch("DROP VIEW IF EXISTS workflow_scopes;");
        conn.execute_batch(
            "CREATE VIEW IF NOT EXISTS workflow_scopes AS
                SELECT
                    scope_token,
                    name AS workflow_name,
                    COALESCE(json_extract(alphabet_json,'$.domain'),'') AS domain,
                    powl_string,
                    admitted,
                    fitness,
                    defects_json,
                    deviations_json,
                    gates_fired_json,
                    tenant_id
                FROM declared_workflows;

             CREATE TABLE IF NOT EXISTS workflow_capability (
                 workflow_name             TEXT PRIMARY KEY,
                 admission_count           INTEGER NOT NULL DEFAULT 0,
                 success_count             INTEGER NOT NULL DEFAULT 0,
                 failure_count             INTEGER NOT NULL DEFAULT 0,
                 sum_fitness               REAL    NOT NULL DEFAULT 0.0,
                 sum_precision             REAL    NOT NULL DEFAULT 0.0,
                 first_admitted_at         TEXT,
                 last_admitted_at          TEXT,
                 defects_taxonomy_version  TEXT NOT NULL
             );"
        )?;

        // ─── Phase 11: multi-tenant ALTERs (after all CREATEs) ─────────
        // Idempotent additive migrations. Each row defaults to tenant_id =
        // 'default', which preserves single-tenant deployments. New rows
        // inserted by a tenant-aware code path carry a real tenant_id.
        // The `sessions` table called out in the deliverable maps to
        // `revoked_sessions` (the closest session-scoped table that exists).
        for stmt in [
            "ALTER TABLE receipts ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';",
            "ALTER TABLE declared_workflows ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';",
            "ALTER TABLE ocel_events ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';",
            "ALTER TABLE lineage_events ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';",
            "ALTER TABLE workflow_capability ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';",
            "ALTER TABLE revoked_sessions ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';",
        ] {
            let _ = conn.execute_batch(stmt);
        }
        for stmt in [
            "CREATE INDEX IF NOT EXISTS idx_receipts_tenant ON receipts(tenant_id);",
            "CREATE INDEX IF NOT EXISTS idx_declared_workflows_tenant ON declared_workflows(tenant_id);",
            "CREATE INDEX IF NOT EXISTS idx_ocel_events_tenant ON ocel_events(tenant_id);",
            "CREATE INDEX IF NOT EXISTS idx_lineage_events_tenant ON lineage_events(tenant_id);",
            "CREATE INDEX IF NOT EXISTS idx_workflow_capability_tenant ON workflow_capability(tenant_id);",
            "CREATE INDEX IF NOT EXISTS idx_revoked_sessions_tenant ON revoked_sessions(tenant_id);",
        ] {
            let _ = conn.execute_batch(stmt);
        }

        // ─── R9-3: tenant-scoped receipt sequence unique index ─────────────
        // The original index `receipts_session_sequence_uniq` keys on
        // (session_id, sequence), which prevents two tenants from sharing the
        // same session_id with independent per-tenant counters. Replace it with
        // a (session_id, tenant_id, sequence) triple to allow per-tenant
        // sequence namespaces while still enforcing no duplicate within a
        // single tenant's session.
        let _ = conn.execute_batch(
            "DROP INDEX IF EXISTS receipts_session_sequence_uniq;\
             DROP INDEX IF EXISTS receipts_session_seq_desc;\
             CREATE UNIQUE INDEX IF NOT EXISTS receipts_session_tenant_seq_uniq \
                 ON receipts(session_id, tenant_id, sequence);\
             CREATE INDEX IF NOT EXISTS receipts_session_tenant_seq_desc \
                 ON receipts(session_id, tenant_id, sequence DESC);"
        );

        // ─── Round 4 WD — §29 Cell8 retirement closure ────────────────────
        // Trust-set rotation history + receipt validity-window column.
        // Additive/idempotent — safe on existing databases.

        // 1) `key_valid_at` on receipts. Empty default → legacy receipts
        //    pass A10 without window check (with a tracing::warn from the
        //    verifier). Plan D Option 1.
        let _ = conn.execute_batch(
            "ALTER TABLE receipts ADD COLUMN key_valid_at TEXT NOT NULL DEFAULT '';"
        );

        // 2) `trusted_keys_history` — every key the gate ever accepted.
        //    `removed_at` NULL means the key is still active. The
        //    `status` column is informational ('active' | 'retired').
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS trusted_keys_history (
                fingerprint TEXT PRIMARY KEY,
                pem         TEXT NOT NULL,
                added_at    TEXT NOT NULL,
                removed_at  TEXT,
                status      TEXT NOT NULL DEFAULT 'active'
             );
             CREATE INDEX IF NOT EXISTS idx_trusted_keys_history_status
                 ON trusted_keys_history(status);
             CREATE INDEX IF NOT EXISTS idx_trusted_keys_history_added_at
                 ON trusted_keys_history(added_at);"
        )?;

        // 3) Retention pruning indexes. Each (tenant_id, time/created) DESC
        //    pair lets the RetentionWorker run `DELETE … WHERE time < cutoff`
        //    without a full-table scan. Wrap individually so an existing
        //    column-name conflict on a sibling table can't poison the rest.
        for stmt in [
            "CREATE INDEX IF NOT EXISTS idx_ocel_events_tenant_time \
                 ON ocel_events(tenant_id, time DESC);",
            "CREATE INDEX IF NOT EXISTS idx_lineage_events_tenant_ts \
                 ON lineage_events(tenant_id, timestamp DESC);",
            "CREATE INDEX IF NOT EXISTS idx_conformance_runs_ran_at \
                 ON conformance_runs(ran_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_revoked_sessions_revoked_at \
                 ON revoked_sessions(revoked_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_receipts_granted_at \
                 ON receipts(granted_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_mined_exemplars_mined_at \
                 ON mined_exemplars(mined_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_align_feedback_ts \
                 ON align_feedback(timestamp DESC);",
            "CREATE INDEX IF NOT EXISTS idx_tool_feedback_ts \
                 ON tool_feedback(timestamp DESC);",
        ] {
            let _ = conn.execute_batch(stmt);
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Record a capability event for a workflow class. Called from the
    /// admission gate after both Ok and Err branches. Atomic UPSERT.
    ///
    /// Accumulates per-workflow `admission_count`, `sum_fitness`, and
    /// `sum_precision` so that the practitioner can later compute the
    /// average fitness-precision trade-off across all admission runs for a
    /// given workflow class — answering "which algorithm gives the best
    /// fitness-precision balance for my log?" without re-running discovery.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    ///
    /// // A successful admission: fitness=0.95, precision=0.88.
    /// db.record_capability("order-to-cash", true, 0.95, 0.88, "v1").unwrap();
    ///
    /// // The row is queryable via the raw connection for inspection.
    /// let conn = db.conn();
    /// let (count, sum_f, sum_p): (i64, f64, f64) = conn.query_row(
    ///     "SELECT admission_count, sum_fitness, sum_precision \
    ///      FROM workflow_capability WHERE workflow_name = 'order-to-cash'",
    ///     [],
    ///     |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    /// ).unwrap();
    ///
    /// assert_eq!(count, 1);
    /// assert!((sum_f - 0.95).abs() < 1e-9, "fitness must be stored exactly");
    /// assert!((sum_p - 0.88).abs() < 1e-9, "precision must be stored exactly");
    /// ```
    pub fn record_capability(
        &self,
        workflow_name: &str,
        admitted: bool,
        fitness: f64,
        precision: f64,
        taxonomy_version: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let success: i64 = if admitted { 1 } else { 0 };
        let failure: i64 = if admitted { 0 } else { 1 };
        self.conn().execute(
            "INSERT INTO workflow_capability(
                 workflow_name, admission_count, success_count, failure_count,
                 sum_fitness, sum_precision, first_admitted_at, last_admitted_at,
                 defects_taxonomy_version)
             VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?6, ?7)
             ON CONFLICT(workflow_name) DO UPDATE SET
                 admission_count = admission_count + 1,
                 success_count   = success_count + excluded.success_count,
                 failure_count   = failure_count + excluded.failure_count,
                 sum_fitness     = sum_fitness + excluded.sum_fitness,
                 sum_precision   = sum_precision + excluded.sum_precision,
                 last_admitted_at = excluded.last_admitted_at,
                 defects_taxonomy_version = excluded.defects_taxonomy_version",
            rusqlite::params![workflow_name, success, failure, fitness, precision, now, taxonomy_version],
        )?;
        Ok(())
    }

    /// Update the per-scope outcome columns on a `declared_workflows` row.
    /// Called from admission with the verdict, fitness/precision, and the
    /// JSON-serialized gates/defects/manufacturing-delta payloads.
    ///
    /// The caller must have already inserted a row for `scope_token` via
    /// `declared_workflows`; this method is a pure UPDATE that stamps the
    /// conformance result onto that existing row, preserving the PM lifecycle
    /// principle that every scope has a single, authoritative outcome record.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    ///
    /// // Pre-condition: a declared_workflows row must exist for the scope.
    /// db.conn().execute(
    ///     "INSERT INTO declared_workflows \
    ///      (scope_token, session_id, name, powl_string, powl_hash, \
    ///       alphabet_json, declared_at, status) \
    ///      VALUES ('scope-abc', 'sess-1', 'wf', 'SEQ(a,b)', 'hash1', \
    ///              '{}', datetime('now'), 'open')",
    ///     [],
    /// ).unwrap();
    ///
    /// // Record the outcome: admitted, fitness=0.92, precision=0.85.
    /// db.record_workflow_outcome(
    ///     "scope-abc", true, 0.92, 0.85,
    ///     "[]", "[]", "[\"gate_A\"]", "[]", "{}",
    /// ).unwrap();
    ///
    /// // Read back the stamped fitness/precision from the row.
    /// let (admitted, fitness, precision): (i64, f64, f64) = db.conn().query_row(
    ///     "SELECT admitted, fitness, precision FROM declared_workflows \
    ///      WHERE scope_token = 'scope-abc'",
    ///     [],
    ///     |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    /// ).unwrap();
    ///
    /// assert_eq!(admitted, 1);
    /// assert!((fitness  - 0.92).abs() < 1e-9, "fitness must be stored exactly");
    /// assert!((precision - 0.85).abs() < 1e-9, "precision must be stored exactly");
    /// ```
    #[allow(clippy::too_many_arguments)] // Each arg maps 1-to-1 to a `declared_workflows` column written atomically; a struct would just shadow the schema.
    pub fn record_workflow_outcome(
        &self,
        scope_token: &str,
        admitted: bool,
        fitness: f64,
        precision: f64,
        defects_json: &str,
        deviations_json: &str,
        gates_fired_json: &str,
        gates_denied_json: &str,
        manufacturing_delta_json: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let admitted_i: i64 = if admitted { 1 } else { 0 };
        self.conn().execute(
            "UPDATE declared_workflows SET
                admitted = ?1,
                fitness = ?2,
                precision = ?3,
                defects_json = ?4,
                deviations_json = ?5,
                gates_fired_json = ?6,
                gates_denied_json = ?7,
                manufacturing_delta_json = ?8,
                admission_decided_at = ?9
             WHERE scope_token = ?10",
            rusqlite::params![
                admitted_i, fitness, precision,
                defects_json, deviations_json,
                gates_fired_json, gates_denied_json,
                manufacturing_delta_json, now, scope_token,
            ],
        )?;
        Ok(())
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

    pub fn get_last_active_path(&self) -> Result<Option<String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT value FROM monitor_state WHERE key = 'last_active_ontology_path'")?;
        let mut rows = stmt.query([])?;
        Ok(rows.next()?.map(|r| r.get(0)).transpose()?)
    }

    pub fn set_last_active_path(&self, path: &str) -> Result<()> {
        self.conn().execute(
            "INSERT OR REPLACE INTO monitor_state (key, value) VALUES ('last_active_ontology_path', ?1)",
            rusqlite::params![path],
        )?;
        Ok(())
    }

    pub fn clear_last_active_path(&self) -> Result<()> {
        self.conn().execute(
            "DELETE FROM monitor_state WHERE key = 'last_active_ontology_path'",
            [],
        )?;
        Ok(())
    }

    pub fn last_cache_entry(&self) -> Result<Option<(String, i64)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT name, triple_count FROM ontology_cache ORDER BY last_access_at DESC LIMIT 1"
        )?;
        let mut rows = stmt.query([])?;
        Ok(rows.next()?.map(|r| Ok::<_, rusqlite::Error>((r.get(0)?, r.get(1)?))).transpose()?)
    }
}
