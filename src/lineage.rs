use crate::state::StateDb;
use chrono::Utc;
use wasm4pm_types::{Attribute, Attributes, Event, EventLog, Trace, AttributeValue};

/// The SQLite table name used to store lineage events.
///
/// All SQL statements in this module target this table by name.  Consumers
/// that query the database directly (e.g. for process mining) should use this
/// constant rather than hardcoding the string `"lineage_events"`.
///
/// # Examples
///
/// ```
/// use open_ontologies::lineage::LINEAGE_EVENTS_TABLE;
///
/// // Pin the contract: the table name is exactly "lineage_events".
/// assert_eq!(LINEAGE_EVENTS_TABLE, "lineage_events");
/// ```
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
    /// Create a new `LineageLog` backed by the given [`StateDb`].
    ///
    /// No governance webhook is configured; lineage events are written only to
    /// the local SQLite store. To add webhook delivery, use
    /// [`LineageLog::with_governance_webhook`].
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
    ///
    /// // The log starts empty; get_compact returns only the trailing newline.
    /// let compact = log.get_compact("no-events-yet");
    /// assert_eq!(compact, "\n");
    /// ```
    pub fn new(db: StateDb) -> Self {
        Self { db, governance_webhook: None }
    }

    /// Create a `LineageLog` that also POSTs every event to a governance webhook.
    ///
    /// When `webhook_url` is `Some`, each call to [`LineageLog::record`] (and all
    /// typed helpers) fires an asynchronous HTTP POST to the given URL after the
    /// event is persisted to SQLite. The payload is a JSON object containing the
    /// session ID, sequence number, event type, operation, details, and an RFC-3339
    /// timestamp.
    ///
    /// Pass `None` to get the same behaviour as [`LineageLog::new`].
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::lineage::LineageLog;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    ///
    /// // Construction with a webhook URL — the URL is stored but not dispatched
    /// // until the first event is recorded inside a Tokio runtime.
    /// let _log_with_hook = LineageLog::with_governance_webhook(
    ///     db.clone(),
    ///     Some("http://localhost:9900/api/enforcer/event".to_string()),
    /// );
    ///
    /// // Passing None disables webhook delivery. Events are written to SQLite
    /// // only, exactly like LineageLog::new.
    /// let log_no_hook = LineageLog::with_governance_webhook(db, None);
    /// let session = "webhook-doctest-01";
    /// log_no_hook.record(session, "S", "session_reset", "");
    /// let compact = log_no_hook.get_compact(session);
    /// assert!(compact.contains("session_reset"),
    ///     "event must appear in lineage, got: {compact}");
    /// ```
    pub fn with_governance_webhook(db: StateDb, webhook_url: Option<String>) -> Self {
        Self { db, governance_webhook: webhook_url }
    }

    /// Generate a new session ID (short hex).
    ///
    /// Session IDs are 16-character lowercase hexadecimal strings derived from
    /// a monotonic nanosecond counter. Successive calls are guaranteed to
    /// produce distinct values within a single process.
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
    ///
    /// let s1 = log.new_session();
    /// let s2 = log.new_session();
    ///
    /// // IDs are exactly 16 hex characters.
    /// assert_eq!(s1.len(), 16);
    /// assert!(s1.chars().all(|c| c.is_ascii_hexdigit()));
    ///
    /// // Successive calls produce distinct IDs.
    /// assert_ne!(s1, s2);
    /// ```
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
    ///
    /// Multiple events in the same session are stored with monotonically
    /// increasing sequence numbers and appear in that order in the compact view:
    ///
    /// ```
    /// use open_ontologies::lineage::LineageLog;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let log = LineageLog::new(db);
    /// let sid = "seq-order-doctest";
    ///
    /// log.record(sid, "S", "session_reset", "");
    /// log.record(sid, "G", "admission_granted", "h1");
    /// log.record(sid, "R", "powl_replay", "fitness=1.0;precision=1.0");
    ///
    /// let compact = log.get_compact(sid);
    /// let lines: Vec<&str> = compact.lines().collect();
    /// assert_eq!(lines.len(), 3, "three events must produce three lines");
    ///
    /// // Sequence numbers are 1-based and strictly increasing.
    /// let seq_of = |line: &str| -> u64 {
    ///     line.splitn(6, ':').nth(1).unwrap().parse().unwrap()
    /// };
    /// assert_eq!(seq_of(lines[0]), 1);
    /// assert_eq!(seq_of(lines[1]), 2);
    /// assert_eq!(seq_of(lines[2]), 3);
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
    ///
    /// The event type is always `"R"` — this is the single-letter code consumed
    /// by process-mining queries to identify replay events:
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::lineage::LineageLog;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let log = LineageLog::new(db);
    /// let session = "replay-type-doctest";
    ///
    /// log.record_powl_replay(session, 1.0, 1.0);
    ///
    /// // The event type field (index 3 in the colon-delimited line) must be "R".
    /// let compact = log.get_compact(session);
    /// let first_line = compact.lines().next().unwrap();
    /// let parts: Vec<&str> = first_line.splitn(6, ':').collect();
    /// assert_eq!(parts[3], "R", "powl_replay event type must be 'R'");
    /// assert_eq!(parts[4], "powl_replay", "operation must be 'powl_replay'");
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

    /// Persist an admission bypass event for the given session.
    ///
    /// Records event type `"B"` (bypass) with operation `"admission_bypass"`.
    /// An admission bypass occurs when a conformance gate is explicitly overridden
    /// by an authorised operator — for example, when a known-good artifact is
    /// promoted without re-running the full proof chain. The `reason` field makes
    /// the override auditable: a practitioner can later mine bypass events to
    /// measure how often the authoritative path is circumvented, which is a
    /// direct measure of process generalisation risk.
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
    /// let session = "bypass-doctest-01";
    ///
    /// log.record_admission_bypass(session, "emergency-hotfix-approved-by-alice");
    ///
    /// // The compact view encodes the B event with the reason as details.
    /// let compact = log.get_compact(session);
    /// assert!(compact.contains("admission_bypass"), "operation must appear in lineage");
    /// assert!(
    ///     compact.contains("emergency-hotfix-approved-by-alice"),
    ///     "reason must be persisted, got: {compact}",
    /// );
    /// ```
    pub fn record_admission_bypass(&self, session_id: &str, reason: &str) {
        self.record(session_id, "B", "admission_bypass", reason);
    }

    /// Persist a session-revocation event.
    ///
    /// Records event type `"V"` (revoked) with operation `"session_revoked"`.
    /// Session revocation is a governance action: an authorised operator or
    /// automated policy engine invalidates an active session — for example, when
    /// a principal's credentials are compromised or a running job must be halted
    /// mid-flight. The `reason` field provides the audit trail entry that
    /// satisfies Cell8 gate A11 (Governance): downstream process-mining queries
    /// can count revocation events per principal to detect anomalous patterns.
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
    /// let session = "revoke-doctest-01";
    ///
    /// log.record_session_revoked(session, "policy-violation:credential-expired");
    ///
    /// // The compact view encodes the V event with the reason as details.
    /// let compact = log.get_compact(session);
    /// assert!(compact.contains("session_revoked"), "operation must appear in lineage");
    /// assert!(
    ///     compact.contains("policy-violation:credential-expired"),
    ///     "reason must be persisted, got: {compact}",
    /// );
    /// ```
    pub fn record_session_revoked(&self, session_id: &str, reason: &str) {
        self.record(session_id, "V", "session_revoked", reason);
    }

    /// Persist a session-reset event.
    ///
    /// Records event type `"S"` (reset) with operation `"session_reset"` and no
    /// additional detail payload. A session reset clears all transient state
    /// accumulated during a session — for example, a practitioner who loaded a
    /// wrong ontology and wants to start fresh. Unlike revocation, a reset is
    /// operator-initiated and does not imply a policy violation; it is a normal
    /// part of the PM lifecycle loop (van der Aalst, 2016) when iterating between
    /// discovery and conformance phases.
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
    /// let session = "reset-doctest-01";
    ///
    /// log.record_session_reset(session);
    ///
    /// // The compact view encodes the S event; the details field is empty.
    /// let compact = log.get_compact(session);
    /// assert!(compact.contains("session_reset"), "operation must appear in lineage");
    /// // Details are empty, so the trailing colon is the last character before newline.
    /// assert!(compact.contains(":S:session_reset:"), "event type and operation must be encoded");
    /// ```
    pub fn record_session_reset(&self, session_id: &str) {
        self.record(session_id, "S", "session_reset", "");
    }

    /// Get compact lineage for a session.
    ///
    /// Returns one line per recorded event in the format:
    /// `session_id:seq:timestamp:event_type:operation:details\n`.
    ///
    /// When the session has no events the return value is a single newline
    /// character (`"\n"`), which makes it safe to call unconditionally and
    /// split on newlines without special-casing the empty case.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::lineage::LineageLog;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db  = StateDb::open(Path::new(":memory:")).unwrap();
    /// let log = LineageLog::new(db);
    /// let sid = "compact-doctest-01";
    ///
    /// // Empty session returns a single newline.
    /// assert_eq!(log.get_compact(sid), "\n");
    ///
    /// // After recording, the compact view has exactly one colon-delimited line.
    /// log.record(sid, "G", "admission_granted", "cafe0123");
    /// let compact = log.get_compact(sid);
    ///
    /// // Line format: session:seq:timestamp:event_type:operation:details
    /// let first_line = compact.lines().next().unwrap();
    /// let parts: Vec<&str> = first_line.splitn(6, ':').collect();
    /// assert_eq!(parts[0], sid,           "session_id must be first");
    /// assert_eq!(parts[1], "1",           "first event has seq=1");
    /// assert_eq!(parts[3], "G",           "event_type must be G");
    /// assert_eq!(parts[4], "admission_granted", "operation must match");
    /// ```
    ///
    /// Events recorded under a different session ID do not appear in the compact
    /// view of an unrelated session (session isolation invariant):
    ///
    /// ```
    /// use open_ontologies::lineage::LineageLog;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db  = StateDb::open(Path::new(":memory:")).unwrap();
    /// let log = LineageLog::new(db);
    ///
    /// log.record("session-x", "G", "admission_granted", "hash-x");
    /// log.record("session-y", "D", "admission_denied",  "defect-y");
    ///
    /// // session-x compact must not contain anything from session-y.
    /// let x_compact = log.get_compact("session-x");
    /// assert!(!x_compact.contains("admission_denied"),  "session-y event must not leak into session-x");
    /// assert!(!x_compact.contains("defect-y"),          "session-y detail must not leak");
    ///
    /// // session-y compact must not contain anything from session-x.
    /// let y_compact = log.get_compact("session-y");
    /// assert!(!y_compact.contains("admission_granted"), "session-x event must not leak into session-y");
    /// assert!(!y_compact.contains("hash-x"),            "session-x detail must not leak");
    /// ```
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
/// ## lifecycle:transition invariant
///
/// Every event in the returned log carries a `lifecycle:transition` attribute
/// with value `"complete"` — this satisfies the XES standard and makes the log
/// directly consumable by pm4py conformance checkers:
///
/// ```
/// use open_ontologies::lineage::{LineageLog, lineage_to_event_log};
/// use open_ontologies::state::StateDb;
/// use std::path::Path;
///
/// let db = StateDb::open(Path::new(":memory:")).unwrap();
/// let log = LineageLog::new(db.clone());
/// log.record("lc-session", "G", "admission_granted", "h42");
///
/// let conn = db.conn();
/// let event_log = lineage_to_event_log(&conn, Some("lc-session")).unwrap();
/// let event = &event_log.traces[0].events[0];
///
/// // Every event must carry lifecycle:transition = "complete".
/// let lc = event.attributes.iter()
///     .find(|a| a.key == "lifecycle:transition")
///     .expect("lifecycle:transition attribute must be present on every event");
/// assert!(matches!(&lc.value,
///     wasm4pm_types::AttributeValue::String(v) if v == "complete"
/// ), "lifecycle:transition must be 'complete', got: {:?}", lc.value);
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
