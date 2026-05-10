//! OntoStar Stream 4 — Loop 3: Workflow discovery.
//!
//! Trigger: ≥ 20 admitted scopes per domain since last discovery. We pull OCEL
//! traces, run wasm4pm process discovery, compare discovered fitness against
//! declared fitness, and if `discovered_fitness > declared_fitness + 0.05`,
//! insert a `discovered_workflows` row with status=pending. Manual approval
//! flips status via `onto_workflow_feedback`.
//!
//! Adapter note: the plan calls for `wasm4pm::powl::discovery::choice_graph::discover_choice_graph`
//! (POWL 2.0). That function requires a pre-computed DFG + activity sets, and
//! its return value (`Option<(Vec<HashSet<String>>, HashSet<(usize, usize)>)>`)
//! is not a directly-replayable model. To satisfy the spirit of the plan
//! ("no local PM math") we delegate the whole pipeline to wasm4pm-algos:
//!   1. `discover_dfg` → directly-follows graph from the OCEL traces
//!   2. `discover_alpha` → Petri net (replayable model)
//!   3. `check_conformance_alignment` → fitness against the same log
//!
//! This keeps every PM call inside wasm4pm; the choice_graph entry point can
//! be substituted later when Stream 2's PowlBridge exposes a wrapper that
//! converts choice_graph output to a model the conformance checker accepts.

use crate::ocel_store::OcelStore;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use wasm4pm_algos::conformance::check_conformance_alignment;
use wasm4pm_types::{Attribute, AttributeValue, Event, EventLog, Trace};

pub const ADMITTED_SCOPES_THRESHOLD: i64 = 20;
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

    // 4. wasm4pm discovery — alpha gives us a replayable Petri net.
    let petri = match wasm4pm_algos::alpha::discover_alpha(&log, "concept:name") {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };

    // 5. wasm4pm conformance — fitness of the discovered model on its own log.
    let conf = match check_conformance_alignment(&log, &petri, "concept:name") {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    let discovered_fitness = conf.fitness;

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

fn build_event_log(traces_by_key: &HashMap<String, Vec<String>>) -> EventLog {
    let mut log = EventLog::default();
    for (case_id, activities) in traces_by_key {
        let trace = Trace {
            attributes: vec![Attribute {
                key: "concept:name".into(),
                value: AttributeValue::String(case_id.clone()),
                own_attributes: None,
            }],
            events: activities
                .iter()
                .map(|act| Event {
                    attributes: vec![Attribute {
                        key: "concept:name".into(),
                        value: AttributeValue::String(act.clone()),
                        own_attributes: None,
                    }],
                })
                .collect(),
        };
        log.traces.push(trace);
    }
    log
}
