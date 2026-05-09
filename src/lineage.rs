use crate::state::StateDb;
use chrono::Utc;
use wasm4pm_types::{Attribute, Attributes, Event, EventLog, Trace, AttributeValue};

/// Append-only lineage log. Compressed format for AI consumption.
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
    /// Format: session:seq:timestamp:event_type:operation:details
    pub fn record(&self, session_id: &str, event_type: &str, operation: &str, details: &str) {
        let conn = self.db.conn();
        // Get next seq for this session
        let seq: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM lineage_events WHERE session_id = ?1",
                rusqlite::params![session_id],
                |r| r.get(0),
            )
            .unwrap_or(1);
        let ts = Utc::now().timestamp();
        let _ = conn.execute(
            "INSERT INTO lineage_events (session_id, seq, timestamp, event_type, operation, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
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

    pub fn record_powl_replay(&self, session_id: &str, fitness: f64, precision: f64) {
        let details = format!("fitness={};precision={}", fitness, precision);
        self.record(session_id, "R", "powl_replay", &details);
    }

    pub fn record_admission_granted(&self, session_id: &str, receipt_hash: &str) {
        self.record(session_id, "G", "admission_granted", receipt_hash);
    }

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
                "SELECT seq, timestamp, event_type, operation, details
                 FROM lineage_events WHERE session_id = ?1 ORDER BY seq ASC",
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

/// Convert lineage events to a wasm4pm EventLog for process mining.
/// Groups events by session_id (each session becomes a trace/case).
pub fn lineage_to_event_log(
    conn: &rusqlite::Connection,
    session_id_filter: Option<&str>,
) -> anyhow::Result<EventLog> {
    let mut stmt = if session_id_filter.is_some() {
        conn.prepare(
            "SELECT session_id, timestamp, event_type, operation, details
             FROM lineage_events WHERE session_id = ?1 ORDER BY session_id ASC, seq ASC",
        )?
    } else {
        conn.prepare(
            "SELECT session_id, timestamp, event_type, operation, details
             FROM lineage_events ORDER BY session_id ASC, seq ASC",
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
        traces.entry(sid).or_insert_with(Vec::new).push(event);
    }

    let traces_vec: Vec<Trace> = traces
        .into_iter()
        .map(|(case_id, events)| {
            let mut trace_attrs = Attributes::new();
            trace_attrs.push(Attribute::new(
                "case:concept:name".to_string(),
                AttributeValue::String(case_id),
            ));
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

    if let Some(d) = details {
        if !d.is_empty() {
            attributes.push(Attribute::new(
                "details".to_string(),
                AttributeValue::String(d),
            ));
        }
    }

    let event = Event { attributes };
    Ok((session_id, event))
}
