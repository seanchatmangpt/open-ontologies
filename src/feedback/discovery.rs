//! OntoStar Stream 4 — Loop 3: Workflow discovery.
//!
//! Trigger: ≥ 20 admitted scopes per domain since last discovery. We pull OCEL
//! traces, run wasm4pm process discovery, compare discovered fitness against
//! declared fitness, and if `discovered_fitness > declared_fitness + 0.05`,
//! insert a `discovered_workflows` row with status=pending. Manual approval
//! flips status via `onto_workflow_feedback`.
//!
//! Engine binding (v26.9.26): the former `wasm4pm-algos` crate no longer exists
//! at any public revision (absent at wasm4pm@057d5ba2, on wasm4pm main and on
//! crates.io), so discovery is bound to the public `wasm4pm` engine surface:
//!   1. `wasm4pm::algorithms::discover_alpha_plus_plus_from_log` → Petri net
//!      (replayable model; Alpha++ also handles length-1/2 loops)
//!   2. `wasm4pm::conformance::token_replay_pure` → fitness against the same log
//!
//! Every process-mining computation stays inside wasm4pm; this module only
//! projects OCEL rows into a `wasm4pm::models::EventLog` and records the verdict.

use crate::ocel_store::OcelStore;
use anyhow::Result;
use chrono::Utc;
use std::collections::{BTreeMap, HashMap};
use wasm4pm::models::{AttributeValue, Event, EventLog, PetriNet, Trace};
use wasm4pm_types::admission::Admission;

/// Activity attribute key used for both discovery and replay.
pub const ACTIVITY_KEY: &str = "concept:name";

/// Alpha++ minimum support; 0.0 keeps every observed directly-follows relation
/// (the log is already scoped to admitted traces of one domain).
pub const ALPHA_PP_MIN_SUPPORT: f64 = 0.0;

/// Minimum number of admitted scopes per domain before discovery runs.
///
/// # Example
///
/// ```
/// assert_eq!(open_ontologies::feedback::discovery::ADMITTED_SCOPES_THRESHOLD, 20);
/// ```
pub const ADMITTED_SCOPES_THRESHOLD: i64 = 20;

/// Minimum fitness improvement required over declared fitness to emit a
/// discovery suggestion.
///
/// # Example
///
/// ```
/// assert!((open_ontologies::feedback::discovery::FITNESS_LIFT - 0.05).abs() < f64::EPSILON);
/// ```
pub const FITNESS_LIFT: f64 = 0.05;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscoveredWorkflow {
    pub id: String,
    pub domain: String,
    pub powl_string: String,
    pub discovered_fitness: f64,
    pub declared_fitness: f64,
    pub status: String,
    pub suggested_at: String,
}

/// Run the discovery loop for a domain. Returns `Ok(None)` when no
/// statistically-better workflow is found (or threshold not met).
///
/// # Example
///
/// ```
/// use open_ontologies::state::StateDb;
/// use open_ontologies::ocel_store::OcelStore;
/// use open_ontologies::feedback::discovery::discover_for_domain;
///
/// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
/// let store = OcelStore::new(db);
///
/// // An empty store has no admitted scopes, so discovery returns None.
/// let result = discover_for_domain("billing", &store).unwrap();
/// assert!(result.is_none());
/// ```
pub fn discover_for_domain(
    domain: &str,
    store: &OcelStore,
) -> Result<Option<DiscoveredWorkflow>> {
    let db = store.db();
    let conn = db.conn();

    // 1. Count admitted scopes for this domain.
    let admitted: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT m.source_session) FROM mined_exemplars m
             JOIN receipts r ON m.receipt_hash = r.receipt_hash
             WHERE m.domain = ?1",
            rusqlite::params![domain],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if admitted < ADMITTED_SCOPES_THRESHOLD {
        return Ok(None);
    }

    // 2. Pull OCEL traces tagged to this domain via declared_workflows.name.
    let mut stmt = conn.prepare(
        "SELECT e.event_id, e.event_type, COALESCE(st.value, e.session_id) as scope_or_session
         FROM ocel_events e
         LEFT JOIN ocel_event_attrs st ON st.event_id = e.event_id AND st.name = 'scope_token'
         WHERE EXISTS (
             SELECT 1 FROM declared_workflows dw
             WHERE dw.name = ?1
               AND (st.value = dw.scope_token OR e.session_id = dw.session_id)
         )
         ORDER BY e.time ASC",
    )?;
    let event_rows: Vec<(String, String, String)> = stmt
        .query_map(rusqlite::params![domain], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(stmt);

    if event_rows.len() < 2 {
        return Ok(None);
    }

    // 3. Group into per-trace event sequences (one trace per scope_token/session).
    let mut traces_by_key: HashMap<String, Vec<String>> = HashMap::new();
    for (_eid, etype, key) in event_rows {
        traces_by_key.entry(key).or_default().push(etype);
    }
    let log = build_event_log(&traces_by_key);
    if log.traces.is_empty() {
        return Ok(None);
    }

    // 4+5. wasm4pm discovery + conformance on the same log.
    let (petri, discovered_fitness) = match mine_fitness(&log) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    // 6. Read declared fitness — average of recent conformance_runs for this class.
    let declared_fitness: f64 = conn
        .query_row(
            "SELECT COALESCE(AVG(fitness), 0.0) FROM conformance_runs
             WHERE workflow_class = ?1
             AND ran_at >= datetime('now', '-30 days')",
            rusqlite::params![domain],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    if discovered_fitness <= declared_fitness + FITNESS_LIFT {
        return Ok(None);
    }

    // 7. Synthesize a POWL-shaped string from the DFG (best-effort serialization).
    let powl_string = format!("DISCOVERED_DFG{{transitions={}, places={}}}",
        petri.transitions.len(), petri.places.len());
    let id = format!("dw_{}_{}", domain, Utc::now().timestamp_millis());
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO discovered_workflows
            (id, domain, powl_string, discovered_fitness, declared_fitness, status, suggested_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
        rusqlite::params![id, domain, powl_string, discovered_fitness, declared_fitness, now],
    )?;

    Ok(Some(DiscoveredWorkflow {
        id,
        domain: domain.to_string(),
        powl_string,
        discovered_fitness,
        declared_fitness,
        status: "pending".into(),
        suggested_at: now,
    }))
}

/// Flip `discovered_workflows.status` based on user feedback.
/// Returns the final status string ("accepted" or "rejected").
///
/// # Example
///
/// ```
/// use open_ontologies::state::StateDb;
/// use open_ontologies::ocel_store::OcelStore;
/// use open_ontologies::feedback::discovery::record_feedback;
///
/// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
/// let store = OcelStore::new(db);
///
/// // Accepting a non-existent id is a no-op UPDATE — still returns "accepted".
/// let status = record_feedback(&store, "dw_nonexistent", true).unwrap();
/// assert_eq!(status, "accepted");
///
/// // Rejecting a non-existent id likewise returns "rejected".
/// let status = record_feedback(&store, "dw_nonexistent", false).unwrap();
/// assert_eq!(status, "rejected");
/// ```
pub fn record_feedback(store: &OcelStore, id: &str, accepted: bool) -> Result<String> {
    let conn = store.db().conn();
    let status = if accepted { "accepted" } else { "rejected" };
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE discovered_workflows SET status = ?1, decided_at = ?2 WHERE id = ?3",
        rusqlite::params![status, now, id],
    )?;
    Ok(status.to_string())
}

/// Discover a Petri net from `log` with wasm4pm Alpha++ and replay the same
/// log on it with wasm4pm token replay. Returns the model and its average
/// fitness in `[0, 1]`. An empty log is refused with an error rather than
/// reported as a vacuous fitness.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use open_ontologies::feedback::discovery::{build_event_log, mine_fitness};
///
/// let mut traces = HashMap::new();
/// traces.insert("c1".to_string(), vec!["a".to_string(), "b".to_string()]);
/// traces.insert("c2".to_string(), vec!["a".to_string(), "b".to_string()]);
/// let (net, fitness) = mine_fitness(&build_event_log(&traces)).unwrap();
/// assert!(!net.transitions.is_empty());
/// assert!((0.0..=1.0).contains(&fitness));
/// ```
pub fn mine_fitness(log: &EventLog) -> Result<(PetriNet, f64)> {
    if log.traces.is_empty() {
        anyhow::bail!("REFUSED(EMPTY_LOG): discovery requires at least one trace");
    }
    let admitted = Admission::<_, ()>::new(log.clone()).into_evidence();
    let petri = wasm4pm::algorithms::discover_alpha_plus_plus_from_log(
        &admitted,
        ACTIVITY_KEY,
        ALPHA_PP_MIN_SUPPORT,
    )
    .map_err(|e| anyhow::anyhow!("wasm4pm alpha++ discovery failed: {e}"))?;
    let conf = wasm4pm::conformance::token_replay_pure(log, &petri, ACTIVITY_KEY);
    let fitness = conf.avg_fitness;
    if !fitness.is_finite() {
        anyhow::bail!("wasm4pm token replay returned non-finite fitness");
    }
    Ok((petri, fitness))
}

/// Project per-case activity sequences into a wasm4pm `EventLog`. Traces are
/// emitted in case-id order so the projection is deterministic.
pub fn build_event_log(traces_by_key: &HashMap<String, Vec<String>>) -> EventLog {
    let ordered: BTreeMap<&String, &Vec<String>> = traces_by_key.iter().collect();
    let mut log = EventLog::default();
    for (case_id, activities) in ordered {
        let mut attributes = BTreeMap::new();
        attributes.insert(ACTIVITY_KEY.to_string(), AttributeValue::String(case_id.clone()));
        let events = activities
            .iter()
            .map(|act| {
                let mut ev = Event::new();
                ev.attributes
                    .insert(ACTIVITY_KEY.to_string(), AttributeValue::String(act.clone()));
                ev
            })
            .collect();
        log.traces.push(Trace { attributes, events });
    }
    log
}
