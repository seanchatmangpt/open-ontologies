use crate::state::StateDb;
use chrono::Utc;
use wasm4pm_types::{Attribute, Attributes, Event, EventLog, Trace, AttributeValue};

pub const LINEAGE_EVENTS_TABLE: &str = "lineage_events";

/// Append-only lineage log. Compressed format for AI consumption.
///
/// Each `LineageLog` is backed by an in-memory or on-disk SQLite database via
/// [`StateDb`]. Events are written in append-only fashion and tagged with a
/// session identifier so that the PM lifecycle loop can isolate and mine one
/// session at a time.
///
/// # Examples
///
/// ```
/// use open_ontologies::lineage::LineageLog;
/// use open_ontologies::state::StateDb;
/// use std::path::Path;
///
/// let db = StateDb::open(Path::new(":memory:")).unwrap();
/// let log = LineageLog::new(db);
/// let session_id = log.new_session();
///
/// // session IDs are 16-character lowercase hex strings.
/// assert_eq!(session_id.len(), 16);
/// assert!(session_id.chars().all(|c| c.is_ascii_hexdigit()));
/// ```
pub struct LineageLog {
    db: StateDb,
    governance_webhook: Option<String>,
}

impl LineageLog {
    pub fn new(db: StateDb) -> Self {
        Self { db, governance_webhook: None }
    }

    pub fn with_governance_webhook(db: StateDb, webhook_url: Option<String>) -> Self {
        Self { db, governance_webhook: webhook_url }
    }

    /// Generate a new session ID (short hex).
    pub fn new_session(&self) -> String {
        format!("{:016x}", rand_id())
    }

    /// Record a lineage event.
    ///
    /// Events are stored in insertion order within the session. The `event_type`
    /// is a single-letter code (`R`, `G`, `D`, `B`, `V`, `S`) and `operation`
    /// is the human-readable activity name. Together they form the
    /// `concept:name` attribute consumed by [`lineage_to_event_log`].
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::lineage::LineageLog;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let log = LineageLog::new(db);
    /// let sid = "doctestSession01".to_string();
    ///
    /// log.record(&sid, "G", "admission_granted", "abc123");
    ///
    /// // The compact representation captures every field in order.
    /// let compact = log.get_compact(&sid);
    /// assert!(compact.contains("admission_granted"));
    /// assert!(compact.contains("abc123"));
    /// ```
    /// Format: session:seq:timestamp:event_type:operation:details
    pub fn record(&self, session_id: &str, event_type: &str, operation: &str, details: &str) {
        let conn = self.db.conn();
        // Get next seq for this session
        let seq: i64 = conn
            .query_row(
                &format!("SELECT COALESCE(MAX(seq), 0) + 1 FROM {LINEAGE_EVENTS_TABLE} WHERE session_id = ?1"),
                rusqlite::params![session_id],
                |r| r.get(0),
            )
            .unwrap_or(1);
        let ts = Utc::now().timestamp();
        let _ = conn.execute(
            &format!("INSERT INTO {LINEAGE_EVENTS_TABLE} (session_id, seq, timestamp, event_type, operation, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"),
            rusqlite::params![session_id, seq, ts.to_string(), event_type, operation, details],
        );

        // Fire governance webhook if configured
        if let Some(ref url) = self.governance_webhook {
            let url = url.clone();
            let payload = serde_json::json!({
                "source": "open-ontologies",
                "session_id": session_id,
                "seq": seq,
                "event_type": event_type,
                "operation": operation,
                "details": details,
                "timestamp": Utc::now().to_rfc3339(),
            });
            tokio::spawn(async move {
                let _ = crate::webhook::deliver_webhook(&url, None, &payload).await;
            });
        }
    }

    // ─── Stream 3 typed event kinds ─────────────────────────────────────
    //
    // The plan calls for an event-kind enum; in this codebase lineage is
    // string-shaped. We expose typed helpers so callers never hand-write
    // the strings, and the values flow through `record` unchanged.
    //
    // Event types: `R` = replay, `G` = admission_granted, `D` = admission_denied,
    // `B` = admission_bypass, `V` = session_revoked, `S` = session_reset.

    /// Persist a POWL conformance-replay result for the given session.
    ///
    /// The event is stored with type `"R"` and operation `"powl_replay"`. The
    /// `fitness` and `precision` values are the two most informative PM quality
    /// dimensions: fitness measures whether the model allows all observed
    /// behaviour; precision penalises unnecessary permissiveness. Both are
    /// encoded in the details field as `"fitness=<f>;precision=<p>"` so any
    /// downstream process-mining query can extract and compare them.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::lineage::LineageLog;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let log = LineageLog::new(db);
    /// let session = "replay-doctest-01";
    ///
    /// log.record_powl_replay(session, 0.95, 0.90);
    ///
    /// // The compact view encodes the event type and quality dimensions.
    /// let compact = log.get_compact(session);
    /// assert!(compact.contains("powl_replay"), "operation must appear in lineage");
    /// assert!(compact.contains("fitness=0.95;precision=0.9"),
    ///     "fitness and precision details must be persisted, got: {compact}");
    /// ```
    pub fn record_powl_replay(&self, session_id: &str, fitness: f64, precision: f64) {
        let details = format!("fitness={};precision={}", fitness, precision);
        self.record(session_id, "R", "powl_replay", &details);
    }

    /// Persist a successful admission decision for the given session.
    ///
    /// Records event type `"G"` (granted) with operation `"admission_granted"`.
    /// The `receipt_hash` is the hex-encoded BLAKE3 receipt proving the artifact
    /// passed all conformance gates — storing it here closes the audit trail
    /// from process discovery through to release, enabling the practitioner to
    /// answer "Can I reproduce this result next week?" with a concrete hash.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::lineage::LineageLog;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let log = LineageLog::new(db);
    /// let session = "granted-doctest-01";
    ///
    /// log.record_admission_granted(session, "deadbeef01234567");
    ///
    /// // The compact view encodes the G event with the receipt hash as details.
    /// let compact = log.get_compact(session);
    /// assert!(compact.contains("admission_granted"), "operation must appear in lineage");
    /// assert!(compact.contains("deadbeef01234567"),
    ///     "receipt hash must be persisted, got: {compact}");
    /// ```
    pub fn record_admission_granted(&self, session_id: &str, receipt_hash: &str) {
        self.record(session_id, "G", "admission_granted", receipt_hash);
    }

    /// Persist a failed admission decision for the given session.
    ///
    /// Records event type `"D"` (denied) with operation `"admission_denied"`.
    /// The `defect_tag` identifies the specific conformance violation that caused
    /// the gate to close — making the denial auditable and reproducible. A model
    /// that only replays the training log will fail on new cases; recording the
    /// defect tag here exposes that generalisation failure explicitly.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::lineage::LineageLog;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let log = LineageLog::new(db);
    /// let session = "denied-doctest-01";
    ///
    /// log.record_admission_denied(session, "fitness_below_threshold");
    ///
    /// // The compact view encodes the D event with the defect tag as details.
    /// let compact = log.get_compact(session);
    /// assert!(compact.contains("admission_denied"), "operation must appear in lineage");
    /// assert!(compact.contains("fitness_below_threshold"),
    ///     "defect tag must be persisted, got: {compact}");
    /// ```
    pub fn record_admission_denied(&self, session_id: &str, defect_tag: &str) {
        self.record(session_id, "D", "admission_denied", defect_tag);
    }

    pub fn record_admission_bypass(&self, session_id: &str, reason: &str) {
        self.record(session_id, "B", "admission_bypass", reason);
    }

    pub fn record_session_revoked(&self, session_id: &str, reason: &str) {
        self.record(session_id, "V", "session_revoked", reason);
    }

    pub fn record_session_reset(&self, session_id: &str) {
        self.record(session_id, "S", "session_reset", "");
    }

    /// Get compact lineage for a session.
    /// Returns: "session:seq:timestamp:type:operation:details\n" per event.
    pub fn get_compact(&self, session_id: &str) -> String {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare(
                &format!("SELECT seq, timestamp, event_type, operation, details
                 FROM {LINEAGE_EVENTS_TABLE} WHERE session_id = ?1 ORDER BY seq ASC"),
            )
            .unwrap();
        let rows: Vec<String> = stmt
            .query_map(rusqlite::params![session_id], |row| {
                let seq: i64 = row.get(0)?;
                let ts: String = row.get(1)?;
                let etype: String = row.get(2)?;
                let op: String = row.get(3)?;
                let details: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();
                Ok(format!("{}:{}:{}:{}:{}:{}", session_id, seq, ts, etype, op, details))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        rows.join("\n") + "\n"
    }
}

fn rand_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::SystemTime;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let d = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    (d.as_nanos() as u64).wrapping_add(seq)
}

/// Convert lineage events to a wasm4pm [`EventLog`] for process mining.
///
/// Each unique `session_id` in the [`LINEAGE_EVENTS_TABLE`] becomes one
/// [`Trace`] (case) in the returned log. Events within a session are ordered
/// by their monotonic `seq` column. The function is the OCEL bridge: after
/// calling it, the caller can hand the [`EventLog`] to any wasm4pm discovery
/// or conformance algorithm — Alpha Miner, Heuristic Miner, Inductive Miner —
/// without any additional transformation.
///
/// Pass `Some(session_id)` to mine a single session; pass `None` to include
/// all sessions in the log (cross-session process discovery).
///
/// # Examples
///
/// ```
/// use open_ontologies::lineage::{LineageLog, lineage_to_event_log};
/// use open_ontologies::state::StateDb;
/// use std::path::Path;
///
/// // Open an in-memory database — hermetic, no filesystem side-effects.
/// let db = StateDb::open(Path::new(":memory:")).unwrap();
///
/// // Record two events in session "s1" and one event in session "s2".
/// let log = LineageLog::new(db.clone());
/// log.record("s1", "G", "admission_granted", "hash-a");
/// log.record("s1", "R", "powl_replay", "fitness=1.0;precision=0.9");
/// log.record("s2", "D", "admission_denied", "low-fitness");
///
/// // Bridge: convert lineage into a mineable event log.
/// let conn = db.conn();
/// let event_log = lineage_to_event_log(&conn, None).unwrap();
///
/// // Two sessions → two traces.
/// assert_eq!(event_log.traces.len(), 2);
///
/// // Session "s1" has two events; session "s2" has one.
/// let s1_trace = event_log.traces.iter()
///     .find(|t| {
///         t.attributes.iter().any(|a| {
///             a.key == "case:concept:name"
///             && matches!(&a.value, wasm4pm_types::AttributeValue::String(v) if v == "s1")
///         })
///     })
///     .expect("trace for session s1 must exist");
/// assert_eq!(s1_trace.events.len(), 2);
///
/// // Each event carries a concept:name formed as "event_type:operation".
/// let first_event = &s1_trace.events[0];
/// let concept = first_event.attributes.iter()
///     .find(|a| a.key == "concept:name")
///     .expect("concept:name attribute must be present");
/// assert!(matches!(&concept.value,
///     wasm4pm_types::AttributeValue::String(v) if v == "G:admission_granted"
/// ));
/// ```
///
/// ## Session-filtered variant
///
/// ```
/// use open_ontologies::lineage::{LineageLog, lineage_to_event_log};
/// use open_ontologies::state::StateDb;
/// use std::path::Path;
///
/// let db = StateDb::open(Path::new(":memory:")).unwrap();
/// let log = LineageLog::new(db.clone());
/// log.record("session-alpha", "S", "session_reset", "");
/// log.record("session-beta", "V", "session_revoked", "policy-violation");
///
/// // Filter to a single session — only one trace is returned.
/// let conn = db.conn();
/// let event_log = lineage_to_event_log(&conn, Some("session-alpha")).unwrap();
/// assert_eq!(event_log.traces.len(), 1);
/// assert_eq!(event_log.traces[0].events.len(), 1);
/// ```
///
/// Groups events by session_id (each session becomes a trace/case).
pub fn lineage_to_event_log(
    conn: &rusqlite::Connection,
    session_id_filter: Option<&str>,
) -> anyhow::Result<EventLog> {
    let mut stmt = if session_id_filter.is_some() {
        conn.prepare(
            &format!("SELECT session_id, timestamp, event_type, operation, details
             FROM {LINEAGE_EVENTS_TABLE} WHERE session_id = ?1 ORDER BY session_id ASC, seq ASC"),
        )?
    } else {
        conn.prepare(
            &format!("SELECT session_id, timestamp, event_type, operation, details
             FROM {LINEAGE_EVENTS_TABLE} ORDER BY session_id ASC, seq ASC"),
        )?
    };

    let mut traces: std::collections::BTreeMap<String, Vec<Event>> = std::collections::BTreeMap::new();

    let rows = if let Some(sid) = session_id_filter {
        stmt.query_map(rusqlite::params![sid], map_lineage_row)?
    } else {
        stmt.query_map(rusqlite::params![], map_lineage_row)?
    };

    for row_result in rows {
        let (sid, event): (String, Event) = row_result?;
        traces.entry(sid).or_default().push(event);
    }

    let traces_vec: Vec<Trace> = traces
        .into_iter()
        .map(|(case_id, events)| {
            let trace_attrs = vec![Attribute::new(
                "case:concept:name".to_string(),
                AttributeValue::String(case_id),
            )];
            Trace { events, attributes: trace_attrs }
        })
        .collect();

    Ok(EventLog::new(traces_vec, Attributes::new()))
}

fn map_lineage_row(row: &rusqlite::Row) -> rusqlite::Result<(String, Event)> {
    let session_id: String = row.get(0)?;
    let timestamp_str: String = row.get(1)?;
    let event_type: String = row.get(2)?;
    let operation: String = row.get(3)?;
    let details: Option<String> = row.get(4)?;

    let concept_name = format!("{}:{}", event_type, operation);

    let mut attributes = Attributes::new();
    attributes.push(Attribute::new(
        "concept:name".to_string(),
        AttributeValue::String(concept_name),
    ));
    attributes.push(Attribute::new(
        "lifecycle:transition".to_string(),
        AttributeValue::String("complete".to_string()),
    ));
    attributes.push(Attribute::new(
        "time:timestamp".to_string(),
        AttributeValue::String(timestamp_str),
    ));

    if let Some(d) = details
        && !d.is_empty() {
            attributes.push(Attribute::new(
                "details".to_string(),
                AttributeValue::String(d),
            ));
        }

    let event = Event { attributes };
    Ok((session_id, event))
}
