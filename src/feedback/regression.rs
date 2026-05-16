//! OntoStar Stream 4 — Loop 5: Conformance regression detection.
//!
//! After every `conformance_runs` insert, compute the rolling-window mean
//! fitness (window K=10 by default) per `workflow_class`. Compare to the
//! baseline (the K runs immediately preceding the rolling window). If
//! `baseline - rolling_mean ≥ REGRESSION_DELTA` (default 0.10), emit an OCEL
//! event `conformance_regression_detected` with attributes:
//!   { workflow_class, baseline, current, delta, window_k }.
//!
//! A built-in `onto_monitor` watcher kind `conformance_regression` scans for
//! these events and supports the existing notify/block/rollback action menu.

use crate::ocel_store::OcelStore;
use anyhow::Result;
use chrono::Utc;

/// Default rolling-window size used by [`check_after_insert`].
///
/// # Example
///
/// ```
/// assert_eq!(open_ontologies::feedback::regression::DEFAULT_WINDOW_K, 10);
/// ```
pub const DEFAULT_WINDOW_K: usize = 10;

/// Minimum drop in rolling-mean fitness that triggers a regression event.
///
/// # Example
///
/// ```
/// assert!((open_ontologies::feedback::regression::REGRESSION_DELTA - 0.10).abs() < f64::EPSILON);
/// ```
pub const REGRESSION_DELTA: f64 = 0.10;

/// Outcome returned by [`check_after_insert`] and [`check_after_insert_with`].
///
/// When there are fewer than `2 * window_k` rows the `emitted` flag is `false`
/// and all numeric fields are `0.0`.
///
/// # Example
///
/// ```
/// use open_ontologies::state::StateDb;
/// use open_ontologies::ocel_store::OcelStore;
/// use open_ontologies::feedback::regression::check_after_insert;
///
/// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
/// let store = OcelStore::new(db);
///
/// let v = check_after_insert(&store, "invoicing").unwrap();
/// // Struct fields are publicly accessible.
/// assert_eq!(v.workflow_class, "invoicing");
/// assert!(!v.emitted);
/// assert_eq!(v.window_k, open_ontologies::feedback::regression::DEFAULT_WINDOW_K);
/// assert!((v.baseline - 0.0).abs() < f64::EPSILON);
/// assert!((v.current - 0.0).abs() < f64::EPSILON);
/// assert!((v.delta - 0.0).abs() < f64::EPSILON);
/// ```
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegressionVerdict {
    pub workflow_class: String,
    pub baseline: f64,
    pub current: f64,
    pub delta: f64,
    pub window_k: usize,
    pub emitted: bool,
}

/// Hook invoked after a `conformance_runs` row is inserted. Computes the
/// rolling-vs-baseline diff for the given workflow class and emits a
/// `conformance_regression_detected` event if the regression delta is breached.
///
/// # Example
///
/// ```
/// use open_ontologies::state::StateDb;
/// use open_ontologies::ocel_store::OcelStore;
/// use open_ontologies::feedback::regression::check_after_insert;
///
/// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
/// let store = OcelStore::new(db);
///
/// // Fewer than 2*K rows — verdict is emitted=false, deltas are zero.
/// let verdict = check_after_insert(&store, "billing").unwrap();
/// assert!(!verdict.emitted);
/// assert_eq!(verdict.workflow_class, "billing");
/// assert!((verdict.delta - 0.0).abs() < f64::EPSILON);
/// ```
pub fn check_after_insert(
    store: &OcelStore,
    workflow_class: &str,
) -> Result<RegressionVerdict> {
    check_after_insert_with(store, workflow_class, DEFAULT_WINDOW_K)
}

/// Like [`check_after_insert`] but with an explicit window size.
///
/// # Example
///
/// ```
/// use open_ontologies::state::StateDb;
/// use open_ontologies::ocel_store::OcelStore;
/// use open_ontologies::feedback::regression::check_after_insert_with;
///
/// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
/// let store = OcelStore::new(db);
///
/// // With a window of 2, we need at least 4 rows; an empty DB returns early.
/// let verdict = check_after_insert_with(&store, "shipping", 2).unwrap();
/// assert!(!verdict.emitted);
/// assert_eq!(verdict.window_k, 2);
/// ```
///
/// The verdict is [`serde::Serialize`] so it can be turned into JSON:
///
/// ```
/// use open_ontologies::state::StateDb;
/// use open_ontologies::ocel_store::OcelStore;
/// use open_ontologies::feedback::regression::check_after_insert_with;
///
/// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
/// let store = OcelStore::new(db);
///
/// let verdict = check_after_insert_with(&store, "claims", 5).unwrap();
/// let json = serde_json::to_value(&verdict).unwrap();
/// assert_eq!(json["workflow_class"], "claims");
/// assert_eq!(json["emitted"], false);
/// assert_eq!(json["window_k"], 5);
/// ```
pub fn check_after_insert_with(
    store: &OcelStore,
    workflow_class: &str,
    window_k: usize,
) -> Result<RegressionVerdict> {
    let conn = store.db().conn();

    // Pull the most recent 2*K fitness values for this class, newest first.
    let values: Vec<f64> = {
        let mut stmt = conn.prepare(
            "SELECT COALESCE(fitness, 0.0) FROM conformance_runs
             WHERE workflow_class = ?1
             ORDER BY ran_at DESC, run_id DESC
             LIMIT ?2",
        )?;
        let limit = (2 * window_k) as i64;
        stmt.query_map(rusqlite::params![workflow_class, limit], |r| {
            r.get::<_, f64>(0)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };

    if values.len() < 2 * window_k {
        return Ok(RegressionVerdict {
            workflow_class: workflow_class.to_string(),
            baseline: 0.0,
            current: 0.0,
            delta: 0.0,
            window_k,
            emitted: false,
        });
    }

    let current_mean = mean(&values[..window_k]);
    let baseline_mean = mean(&values[window_k..2 * window_k]);
    let delta = baseline_mean - current_mean;

    if delta < REGRESSION_DELTA {
        return Ok(RegressionVerdict {
            workflow_class: workflow_class.to_string(),
            baseline: baseline_mean,
            current: current_mean,
            delta,
            window_k,
            emitted: false,
        });
    }

    // Idempotency: collapse the (class, current_mean rounded) tuple. If the
    // same regression state is observed twice in succession we only emit once.
    let event_id = format!(
        "crd_{}_{}_{:.4}_{:.4}",
        workflow_class, window_k, baseline_mean, current_mean
    );
    let already: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ocel_events WHERE event_id = ?1",
            rusqlite::params![event_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if already > 0 {
        return Ok(RegressionVerdict {
            workflow_class: workflow_class.to_string(),
            baseline: baseline_mean,
            current: current_mean,
            delta,
            window_k,
            emitted: false,
        });
    }

    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO ocel_events (event_id, event_type, time, session_id)
         VALUES (?1, 'conformance_regression_detected', ?2, 'feedback')",
        rusqlite::params![event_id, now],
    )?;
    for (name, value) in [
        ("workflow_class", workflow_class.to_string()),
        ("baseline", format!("{:.6}", baseline_mean)),
        ("current", format!("{:.6}", current_mean)),
        ("delta", format!("{:.6}", delta)),
        ("window_k", window_k.to_string()),
    ] {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO ocel_event_attrs (event_id, name, value, value_type)
             VALUES (?1, ?2, ?3, 'string')",
            rusqlite::params![event_id, name, value],
        );
    }

    Ok(RegressionVerdict {
        workflow_class: workflow_class.to_string(),
        baseline: baseline_mean,
        current: current_mean,
        delta,
        window_k,
        emitted: true,
    })
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

/// Built-in `onto_monitor` watcher kind `conformance_regression`. Returns the
/// count of regression events whose `time >= since_iso`. Watchers compare
/// this count against the watcher's `threshold` to decide notify/block/rollback.
///
/// # Example
///
/// ```
/// use open_ontologies::state::StateDb;
/// use open_ontologies::ocel_store::OcelStore;
/// use open_ontologies::feedback::regression::count_regressions_since;
///
/// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
/// let store = OcelStore::new(db);
///
/// // A fresh store has no regression events.
/// let count = count_regressions_since(&store, "2020-01-01T00:00:00Z").unwrap();
/// assert_eq!(count, 0);
/// ```
pub fn count_regressions_since(store: &OcelStore, since_iso: &str) -> Result<i64> {
    let conn = store.db().conn();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ocel_events
             WHERE event_type = 'conformance_regression_detected' AND time >= ?1",
            rusqlite::params![since_iso],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(n)
}
