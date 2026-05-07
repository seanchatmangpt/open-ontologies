//! Governance Commands — lifecycle planning, apply, lock, enforce, monitor, drift, lineage

use clap_noun_verb::Result as NounVerbResult;
use clap_noun_verb_macros::verb;
use serde::Serialize;

use super::helpers::{DEFAULT_DATA_DIR, setup, to_verb_err};

// ── output types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct LockOutput {
    pub ok: bool,
    pub locked: Vec<String>,
    pub reason: String,
}

#[derive(Serialize)]
pub struct MonitorClearOutput {
    pub ok: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct LineageOutput {
    pub session_id: String,
    pub events: String,
}

// ── verbs ─────────────────────────────────────────────────────────────────

/// Plan changes: diff current vs proposed Turtle
#[verb]
fn plan(file: String, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let turtle = std::fs::read_to_string(&file).map_err(to_verb_err)?;
    let planner = open_ontologies::plan::Planner::new(db, graph);
    let result = planner.plan(&turtle).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// Apply planned changes (safe or migrate)
#[verb]
fn apply(mode: Option<String>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let m = mode.unwrap_or_else(|| "safe".to_string());
    let planner = open_ontologies::plan::Planner::new(db, graph);
    let result = planner.apply(&m).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// Lock IRIs to prevent removal (comma-separated list)
#[verb]
fn lock(iris_csv: String, reason: Option<String>, data_dir: Option<String>) -> NounVerbResult<LockOutput> {
    let iris: Vec<String> = iris_csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let (db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let planner = open_ontologies::plan::Planner::new(db, graph);
    let reason_str = reason.unwrap_or_else(|| "locked".to_string());
    for iri in &iris {
        planner.lock_iri(iri, &reason_str);
    }
    Ok(LockOutput { ok: true, locked: iris, reason: reason_str })
}

/// Run design pattern enforcement
#[verb]
fn enforce(pack: Option<String>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let p = pack.unwrap_or_else(|| "generic".to_string());
    let enforcer = open_ontologies::enforce::Enforcer::new(db.clone(), graph);
    let result = enforcer.enforce_with_feedback(&p, Some(&db))
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// Run active SPARQL watchers
#[verb]
fn monitor(data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let m = open_ontologies::monitor::Monitor::new(db, graph);
    let result = m.run_watchers();
    serde_json::to_value(result).map_err(to_verb_err)
}

/// Clear monitor block state
#[verb]
fn monitor_clear(data_dir: Option<String>) -> NounVerbResult<MonitorClearOutput> {
    let (db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let m = open_ontologies::monitor::Monitor::new(db, graph);
    m.clear_blocked();
    Ok(MonitorClearOutput { ok: true, message: "Monitor block cleared".to_string() })
}

/// View lineage trail
#[verb]
fn lineage(session: Option<String>, data_dir: Option<String>) -> NounVerbResult<LineageOutput> {
    let (db, _graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let log = open_ontologies::lineage::LineageLog::new(db);
    let session_id = session.unwrap_or_else(|| "current".to_string());
    let events = log.get_compact(&session_id);
    Ok(LineageOutput { session_id, events: events.trim().to_string() })
}

/// Detect drift between two ontology versions
#[verb]
fn drift(file_a: String, file_b: String, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (db, _graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let v1 = std::fs::read_to_string(&file_a).map_err(to_verb_err)?;
    let v2 = std::fs::read_to_string(&file_b).map_err(to_verb_err)?;
    let detector = open_ontologies::drift::DriftDetector::new(db);
    let result = detector.detect(&v1, &v2).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}
