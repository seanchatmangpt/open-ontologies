//! OntoStar Stream 4 — Loop 2: Threshold calibration.
//!
//! Trigger: a `bypass_admission` OCEL event aging out N days (default 7) without
//! any monitor alerts decrements that workflow class's `precision_threshold`
//! by δ (default 0.02; floor 0.70). With alerts in the window, increment by δ
//! (ceiling 0.99). The delta math mirrors `align::AlignmentEngine::record_feedback`
//! / `feedback::record_tool_feedback` — small additive nudges driven by signal
//! presence.
//!
//! Sweep is invoked by the admin handler `onto_threshold_sweep` and by every
//! `onto_monitor` tick.

use crate::ocel_store::OcelStore;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

/// Minimum age in days before a `bypass_admission` event is acted on.
///
/// # Example
///
/// ```
/// assert_eq!(open_ontologies::feedback::thresholds::DEFAULT_AGE_DAYS, 7);
/// ```
pub const DEFAULT_AGE_DAYS: i64 = 7;

/// Amount by which the precision threshold is nudged per calibration cycle.
///
/// # Example
///
/// ```
/// assert!((open_ontologies::feedback::thresholds::DEFAULT_DELTA - 0.02).abs() < f64::EPSILON);
/// ```
pub const DEFAULT_DELTA: f64 = 0.02;

/// Lowest precision threshold that calibration will ever set.
///
/// # Example
///
/// ```
/// assert!((open_ontologies::feedback::thresholds::PRECISION_FLOOR - 0.70).abs() < f64::EPSILON);
/// ```
pub const PRECISION_FLOOR: f64 = 0.70;

/// Highest precision threshold that calibration will ever set.
///
/// # Example
///
/// ```
/// assert!((open_ontologies::feedback::thresholds::PRECISION_CEIL - 0.99).abs() < f64::EPSILON);
/// ```
pub const PRECISION_CEIL: f64 = 0.99;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ThresholdSweepResult {
    pub examined: usize,
    pub adjusted: usize,
    pub adjustments: Vec<ThresholdAdjustment>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ThresholdAdjustment {
    pub workflow_class: String,
    pub before: f64,
    pub after: f64,
    pub direction: &'static str, // "decrement" | "increment"
    pub reason: &'static str,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ThresholdRow {
    pub workflow_class: String,
    pub precision_threshold: f64,
    pub fitness_threshold: f64,
    pub sample_count: i64,
    pub updated_at: String,
}

/// Sweep all aged-out `bypass_admission` events and adjust per-class
/// precision thresholds by ±δ. Idempotent: each event is acted on once,
/// tracked by inserting a synthetic OCEL event `threshold_calibrated` keyed
/// to the original event id.
///
/// # Example
///
/// ```
/// use open_ontologies::state::StateDb;
/// use open_ontologies::ocel_store::OcelStore;
/// use open_ontologies::feedback::thresholds::sweep;
///
/// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
/// let store = OcelStore::new(db);
///
/// // No bypass_admission events — sweep examines zero events.
/// let result = sweep(&store).unwrap();
/// assert_eq!(result.examined, 0);
/// assert_eq!(result.adjusted, 0);
/// assert!(result.adjustments.is_empty());
/// ```
pub fn sweep(store: &OcelStore) -> Result<ThresholdSweepResult> {
    sweep_with(store, DEFAULT_AGE_DAYS, DEFAULT_DELTA)
}

/// Like [`sweep`] but with explicit age and delta parameters.
///
/// # Example
///
/// ```
/// use open_ontologies::state::StateDb;
/// use open_ontologies::ocel_store::OcelStore;
/// use open_ontologies::feedback::thresholds::sweep_with;
///
/// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
/// let store = OcelStore::new(db);
///
/// // Sweep with a 1-day window and 0.05 delta — still zero events in empty DB.
/// let result = sweep_with(&store, 1, 0.05).unwrap();
/// assert_eq!(result.examined, 0);
/// ```
pub fn sweep_with(
    store: &OcelStore,
    age_days: i64,
    delta: f64,
) -> Result<ThresholdSweepResult> {
    let db = store.db();
    let conn = db.conn();
    let cutoff = (Utc::now() - Duration::days(age_days)).to_rfc3339();

    let events: Vec<(String, String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT e.event_id, e.time, COALESCE(wc.value, '__unknown__')
             FROM ocel_events e
             LEFT JOIN ocel_event_attrs wc
               ON wc.event_id = e.event_id AND wc.name = 'workflow_class'
             WHERE e.event_type = 'bypass_admission' AND e.time <= ?1
               AND NOT EXISTS (
                   SELECT 1 FROM ocel_events c
                   WHERE c.event_type = 'threshold_calibrated'
                     AND c.event_id = 'tc_' || e.event_id
               )",
        )?;
        stmt.query_map(rusqlite::params![cutoff], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };

    let mut adjustments = Vec::new();
    let examined = events.len();

    for (event_id, time_iso, workflow_class) in events {
        let event_time: DateTime<Utc> = DateTime::parse_from_rfc3339(&time_iso)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let window_end = (event_time + Duration::days(age_days)).to_rfc3339();
        let alert_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ocel_events
             WHERE event_type IN ('monitor_alert', 'conformance_regression_detected')
               AND time BETWEEN ?1 AND ?2",
            rusqlite::params![time_iso, window_end],
            |r| r.get(0),
        ).unwrap_or(0);

        let (before, after, direction, reason) =
            adjust_one(&conn, &workflow_class, alert_count, delta)?;

        adjustments.push(ThresholdAdjustment {
            workflow_class: workflow_class.clone(),
            before,
            after,
            direction,
            reason,
        });

        let now = Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT OR IGNORE INTO ocel_events (event_id, event_type, time, session_id)
             VALUES (?1, 'threshold_calibrated', ?2, 'feedback')",
            rusqlite::params![format!("tc_{}", event_id), now],
        );
        for (name, value) in [
            ("workflow_class", workflow_class.as_str()),
            ("direction", direction),
        ] {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO ocel_event_attrs (event_id, name, value, value_type)
                 VALUES (?1, ?2, ?3, 'string')",
                rusqlite::params![format!("tc_{}", event_id), name, value],
            );
        }
    }

    let adjusted = adjustments
        .iter()
        .filter(|a| (a.before - a.after).abs() > f64::EPSILON)
        .count();
    Ok(ThresholdSweepResult { examined, adjusted, adjustments })
}

fn adjust_one(
    conn: &rusqlite::Connection,
    workflow_class: &str,
    alert_count: i64,
    delta: f64,
) -> Result<(f64, f64, &'static str, &'static str)> {
    let row = conn.query_row(
        "SELECT precision_threshold, fitness_threshold, sample_count
         FROM workflow_thresholds WHERE workflow_class = ?1",
        rusqlite::params![workflow_class],
        |r| Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?, r.get::<_, i64>(2)?)),
    );

    let (current, fitness_threshold, sample_count) = match row {
        Ok(t) => t,
        Err(rusqlite::Error::QueryReturnedNoRows) => (0.85, 0.90, 0),
        Err(e) => return Err(e.into()),
    };

    let (after, direction, reason) = if alert_count == 0 {
        ((current - delta).max(PRECISION_FLOOR), "decrement", "no_alerts_in_window")
    } else {
        ((current + delta).min(PRECISION_CEIL), "increment", "alerts_observed")
    };

    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO workflow_thresholds
            (workflow_class, precision_threshold, fitness_threshold, sample_count, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(workflow_class) DO UPDATE SET
            precision_threshold = excluded.precision_threshold,
            sample_count = workflow_thresholds.sample_count + 1,
            updated_at = excluded.updated_at",
        rusqlite::params![workflow_class, after, fitness_threshold, sample_count + 1, now],
    )?;

    Ok((current, after, direction, reason))
}

/// Read all threshold rows for the `onto_threshold_status` MCP handler.
///
/// # Example
///
/// ```
/// use open_ontologies::state::StateDb;
/// use open_ontologies::ocel_store::OcelStore;
/// use open_ontologies::feedback::thresholds::list_all;
///
/// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
/// let store = OcelStore::new(db);
///
/// // A fresh database has no threshold rows.
/// let rows = list_all(&store).unwrap();
/// assert!(rows.is_empty());
/// ```
pub fn list_all(store: &OcelStore) -> Result<Vec<ThresholdRow>> {
    let conn = store.db().conn();
    let mut stmt = conn.prepare(
        "SELECT workflow_class, precision_threshold, fitness_threshold, sample_count, updated_at
         FROM workflow_thresholds ORDER BY workflow_class ASC",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ThresholdRow {
                workflow_class: r.get(0)?,
                precision_threshold: r.get(1)?,
                fitness_threshold: r.get(2)?,
                sample_count: r.get(3)?,
                updated_at: r.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}
