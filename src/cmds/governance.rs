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

/// External verifier — strip-and-rehash an artifact, optionally walk
/// its receipt chain. Read-only. Non-zero exit on any non-Admitted
/// verdict so CI can gate on it directly.
#[verb]
fn verify(
    path: String,
    db: Option<String>,
    ascii_tree: Option<bool>,
) -> NounVerbResult<serde_json::Value> {
    do_verify(&path, db.as_deref(), ascii_tree.unwrap_or(false))
}

/// Domain helper for the `verify` verb — extracted to keep the verb body
/// thin (Poka-Yoke FM-1.1: complexity ≤ 5).
fn do_verify(
    path: &str,
    db_path: Option<&str>,
    ascii_tree: bool,
) -> NounVerbResult<serde_json::Value> {
    let db_handle = open_db_handle(db_path)?;
    let verdict = compute_verdict(path, db_handle.as_ref());
    let mut out = serde_json::to_value(&verdict).map_err(to_verb_err)?;
    maybe_attach_chain(&mut out, &verdict, db_handle.as_ref(), ascii_tree);
    if !verdict.is_admitted() {
        return Err(clap_noun_verb::NounVerbError::execution_error(format!(
            "verify failed: {}",
            serde_json::to_string(&out).unwrap_or_default()
        )));
    }
    Ok(out)
}

fn open_db_handle(db_path: Option<&str>) -> NounVerbResult<Option<open_ontologies::state::StateDb>> {
    match db_path {
        Some(p) => Ok(Some(
            open_ontologies::state::StateDb::open(std::path::Path::new(p)).map_err(to_verb_err)?,
        )),
        None => Ok(None),
    }
}

fn compute_verdict(
    path: &str,
    db: Option<&open_ontologies::state::StateDb>,
) -> open_ontologies::verify::Verdict {
    use open_ontologies::verify as v;
    let p = std::path::Path::new(path);
    if p.is_dir() {
        v::verify_iac_bundle(p, db)
    } else {
        v::verify_artifact(p, db)
    }
}

fn maybe_attach_chain(
    out: &mut serde_json::Value,
    verdict: &open_ontologies::verify::Verdict,
    db: Option<&open_ontologies::state::StateDb>,
    ascii_tree: bool,
) {
    use open_ontologies::verify as v;
    if !ascii_tree {
        return;
    }
    let (Some(db), v::Verdict::Admitted { receipt_hash, .. }) = (db, verdict) else {
        return;
    };
    let Some(rh) = hex_to_32(receipt_hash) else { return };
    let chain = v::walk_receipt_chain(db, &rh);
    let ascii = v::render_chain_ascii(&chain);
    if let Some(obj) = out.as_object_mut() {
        obj.insert("chain".into(), serde_json::json!(chain));
        obj.insert("ascii".into(), serde_json::Value::String(ascii));
    }
}

fn hex_to_32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
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
