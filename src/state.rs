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
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
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
