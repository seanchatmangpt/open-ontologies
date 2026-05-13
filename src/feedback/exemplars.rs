//! OntoStar Stream 4 — Loop 1: Exemplar mining.
//!
//! Trigger: post-`onto_apply`. An `admission_granted` event must exist for the
//! scope, the corresponding `conformance_runs.fitness` must be ≥ 0.95, and a
//! row in `receipts` MUST be present. The receipt-join is a hard rule:
//! a mined exemplar without a receipt cannot enter the registry. Loop 4 then
//! retrieves only receipt-backed exemplars via [`crate::ocel_store::OcelStore::exemplars_for_domain`].
//!
//! All process-mining math is delegated to wasm4pm (`replay_trace`,
//! `compute_fitness`); this module is a pure orchestration hook that reads
//! existing `conformance_runs` rows and persists the link.

use crate::ocel_store::OcelStore;
use anyhow::Result;
use rusqlite::OptionalExtension;

pub const EXEMPLAR_FITNESS_FLOOR: f64 = 0.95;

#[derive(Debug, Clone)]
pub struct MinedExemplar {
    pub id: String,
    pub domain: String,
    pub problem_context: String,
    pub powl_string: String,
    pub fitness: f64,
    pub source_session: Option<String>,
    pub receipt_hash: String,
    pub mined_at: String,
}

/// Insert a row into `mined_exemplars` if and only if the scope has both an
/// `admission_granted` OCEL event AND a row in `receipts` AND a
/// `conformance_runs` row with `fitness >= EXEMPLAR_FITNESS_FLOOR`.
///
/// Returns `Ok(None)` (no exemplar mined, silently skipped) when any
/// precondition fails. Returns `Ok(Some(MinedExemplar))` on success.
///
/// The hard rule "no receipt ⇒ no exemplar" is enforced at the SQL layer:
/// the `receipt_hash` column is NOT NULL and joined to `receipts` on the
/// retrieval side (see `OcelStore::exemplars_for_domain`).
pub fn maybe_mine_exemplar(
    scope_token: &str,
    store: &OcelStore,
) -> Result<Option<MinedExemplar>> {
    let db = store.db();
    let conn = db.conn();

    // 1. admission_granted event for scope?
    let admitted: Option<String> = conn
        .query_row(
            "SELECT e.event_id FROM ocel_events e
             JOIN ocel_event_attrs a ON a.event_id = e.event_id
             WHERE e.event_type = 'admission_granted'
               AND a.name = 'scope_token' AND a.value = ?1
             LIMIT 1",
            rusqlite::params![scope_token],
            |r| r.get(0),
        )
        .optional()?;
    if admitted.is_none() {
        return Ok(None);
    }

    // 2. receipt row for scope (HARD GATE)
    let receipt: Option<String> = conn
        .query_row(
            "SELECT receipt_hash FROM receipts WHERE scope_token = ?1
             ORDER BY granted_at DESC LIMIT 1",
            rusqlite::params![scope_token],
            |r| r.get(0),
        )
        .optional()?;
    let Some(receipt_hash) = receipt else {
        return Ok(None);
    };

    // 3. conformance_runs row with fitness ≥ floor
    let run: Option<(f64, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT fitness, workflow_class, scope_token FROM conformance_runs
             WHERE scope_token = ?1 ORDER BY ran_at DESC LIMIT 1",
            rusqlite::params![scope_token],
            |r| Ok((r.get::<_, f64>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<String>>(2)?)),
        )
        .optional()?;
    let Some((fitness, workflow_class, _)) = run else {
        return Ok(None);
    };
    if fitness < EXEMPLAR_FITNESS_FLOOR {
        return Ok(None);
    }

    // 4. Find a powl_string + domain. Best-effort: pull declared workflow if Stream 1 schema is present.
    let (domain, powl_string, problem_context, source_session): (String, String, String, Option<String>) = conn
        .query_row(
            "SELECT name, powl_string, COALESCE(name, 'unknown'), session_id
             FROM declared_workflows WHERE scope_token = ?1",
            rusqlite::params![scope_token],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .unwrap_or_else(|| {
            (
                workflow_class.clone().unwrap_or_else(|| "unknown".into()),
                String::new(),
                workflow_class.clone().unwrap_or_else(|| "unknown".into()),
                None,
            )
        });

    let id = format!("ex_{}", &receipt_hash[..receipt_hash.len().min(16)]);
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT OR IGNORE INTO mined_exemplars
            (id, domain, problem_context, powl_string, fitness, source_session, receipt_hash, mined_at, promoted)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
        rusqlite::params![
            id, domain, problem_context, powl_string, fitness,
            source_session, receipt_hash, now,
        ],
    )?;

    Ok(Some(MinedExemplar {
        id,
        domain,
        problem_context,
        powl_string,
        fitness,
        source_session,
        receipt_hash,
        mined_at: now,
    }))
}
