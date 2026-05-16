use crate::graph::GraphStore;
use crate::ontology::OntologyService;
use crate::state::StateDb;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatcherAction {
    #[serde(rename = "notify")]
    Notify,
    #[serde(rename = "block_next_apply")]
    BlockNextApply,
    #[serde(rename = "auto_rollback")]
    AutoRollback,
    #[serde(rename = "log")]
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watcher {
    pub id: String,
    pub check_type: String,
    pub threshold: f64,
    pub severity: String,
    pub action: WatcherAction,
    pub query: Option<String>,
    pub message: Option<String>,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub webhook_headers: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub watcher: String,
    pub severity: String,
    pub value: f64,
    pub threshold: f64,
    pub action: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorResult {
    pub status: String,
    pub alerts: Vec<Alert>,
    pub passed: Vec<String>,
}

pub struct Monitor {
    db: StateDb,
    graph: Arc<GraphStore>,
}

impl Monitor {
    /// Create a new `Monitor` backed by the given SQLite state store and
    /// in-memory Oxigraph graph.
    ///
    /// Both arguments are cheap to construct in tests:
    ///
    /// ```
    /// use open_ontologies::monitor::Monitor;
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::graph::GraphStore;
    /// use std::path::Path;
    /// use std::sync::Arc;
    ///
    /// let db    = StateDb::open(Path::new(":memory:")).unwrap();
    /// let graph = Arc::new(GraphStore::new());
    /// let _monitor = Monitor::new(db, graph);
    /// ```
    pub fn new(db: StateDb, graph: Arc<GraphStore>) -> Self {
        Self { db, graph }
    }

    /// Register a watcher that fires when a measured value exceeds its
    /// threshold. The watcher is persisted to the SQLite state store so
    /// it survives across `Monitor` instances that share the same `StateDb`.
    ///
    /// Because `StateDb` is `Clone`, a second `Monitor` built from the same
    /// `StateDb` can read back the watcher to confirm persistence.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::monitor::{Monitor, Watcher, WatcherAction};
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::graph::GraphStore;
    /// use std::path::Path;
    /// use std::sync::Arc;
    ///
    /// let db    = StateDb::open(Path::new(":memory:")).unwrap();
    /// let graph = Arc::new(GraphStore::new());
    /// let monitor = Monitor::new(db.clone(), graph.clone());
    ///
    /// monitor.add_watcher(Watcher {
    ///     id:              "class-count-gate".to_string(),
    ///     check_type:      "sparql".to_string(),
    ///     threshold:       100.0,
    ///     severity:        "warning".to_string(),
    ///     action:          WatcherAction::Notify,
    ///     query:           Some(
    ///         "SELECT (COUNT(?c) AS ?count) WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> }"
    ///             .to_string(),
    ///     ),
    ///     message:         Some("Class count exceeded threshold".to_string()),
    ///     webhook_url:     None,
    ///     webhook_headers: None,
    /// });
    ///
    /// // The watcher is stored in the shared SQLite state. Verify via the
    /// // raw connection that `StateDb` exposes for inspection.
    /// let stored_id: String = db.conn().query_row(
    ///     "SELECT id FROM monitor_watchers WHERE id = 'class-count-gate'",
    ///     [],
    ///     |row| row.get(0),
    /// ).unwrap();
    /// assert_eq!(stored_id, "class-count-gate");
    /// ```
    pub fn add_watcher(&self, watcher: Watcher) {
        let conn = self.db.conn();
        let action_str = serde_json::to_string(&watcher.action).unwrap_or_default();
        let action_str = action_str.trim_matches('"');
        let _ = conn.execute(
            "INSERT OR REPLACE INTO monitor_watchers (id, check_type, threshold, severity, action, query, message, webhook_url, webhook_headers, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1)",
            rusqlite::params![
                watcher.id, watcher.check_type, watcher.threshold,
                watcher.severity, action_str, watcher.query, watcher.message,
                watcher.webhook_url, watcher.webhook_headers,
            ],
        );
    }

    /// Evaluate every enabled watcher and return a [`MonitorResult`] that
    /// reports the overall status, any alerts that fired, and the list of
    /// watcher IDs that passed.
    ///
    /// When no watchers are registered the result has `status = "ok"` and
    /// empty `alerts` and `passed` vectors. This makes the zero-watcher case
    /// safe for hermetic tests that need to exercise downstream status
    /// handling without standing up a live SPARQL endpoint.
    ///
    /// For watchers whose `check_type` is `"sparql"` the implementation
    /// issues a SELECT query against the underlying Oxigraph store; those
    /// watchers require a populated store to return meaningful values. The
    /// example below covers the zero-watcher path only.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::monitor::Monitor;
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::graph::GraphStore;
    /// use std::path::Path;
    /// use std::sync::Arc;
    ///
    /// let db    = StateDb::open(Path::new(":memory:")).unwrap();
    /// let graph = Arc::new(GraphStore::new());
    /// let monitor = Monitor::new(db, graph);
    ///
    /// // With no watchers registered the monitor reports "ok".
    /// let result = monitor.run_watchers();
    /// assert_eq!(result.status, "ok");
    /// assert!(result.alerts.is_empty());
    /// assert!(result.passed.is_empty());
    /// ```
    pub fn run_watchers(&self) -> MonitorResult {
        // OntoStar Stream 4 — Loop 2: opportunistic threshold calibration sweep.
        // Failure here is non-fatal — the sweep is best-effort housekeeping.
        let store = crate::ocel_store::OcelStore::new(self.db.clone());
        let _ = crate::feedback::thresholds::sweep(&store);

        let watchers = self.load_watchers();
        let mut alerts = Vec::new();
        let mut passed = Vec::new();
        let mut blocked = false;
        let mut rolled_back = false;

        for w in &watchers {
            let value = self.evaluate_watcher(w);
            if value > w.threshold {
                let action_str = serde_json::to_string(&w.action).unwrap_or_default();
                let action_str = action_str.trim_matches('"').to_string();
                if matches!(w.action, WatcherAction::BlockNextApply) {
                    blocked = true;
                    self.set_blocked(true);
                }
                // Auto-rollback: revert to the most recent saved version
                if matches!(w.action, WatcherAction::AutoRollback) && !rolled_back {
                    if let Some(label) = self.latest_version_label() {
                        match OntologyService::rollback_version(&self.db, &self.graph, &label) {
                            Ok(_) => {
                                eprintln!("[watch] auto-rollback to '{}' triggered by watcher '{}'", label, w.id);
                                rolled_back = true;
                            }
                            Err(e) => {
                                eprintln!("[watch] auto-rollback failed for watcher '{}': {}", w.id, e);
                            }
                        }
                    } else {
                        eprintln!("[watch] auto-rollback requested by watcher '{}' but no saved versions exist", w.id);
                    }
                }
                // Fire webhook for Notify actions
                if matches!(w.action, WatcherAction::Notify)
                    && let Some(ref url) = w.webhook_url {
                        let url = url.clone();
                        let headers = w.webhook_headers.clone();
                        let payload = serde_json::json!({
                            "source": "open-ontologies",
                            "watcher_id": w.id,
                            "severity": w.severity,
                            "value": value,
                            "threshold": w.threshold,
                            "message": w.message.clone().unwrap_or_default(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });
                        tokio::spawn(async move {
                            let _ = crate::webhook::deliver_webhook(&url, headers.as_deref(), &payload).await;
                        });
                }
                alerts.push(Alert {
                    watcher: w.id.clone(),
                    severity: w.severity.clone(),
                    value,
                    threshold: w.threshold,
                    action: action_str,
                    detail: w.message.clone().unwrap_or_default(),
                });
            } else {
                passed.push(w.id.clone());
            }
        }

        let status = if rolled_back {
            "auto_rolled_back".to_string()
        } else if blocked {
            "blocked".to_string()
        } else if !alerts.is_empty() {
            "alert".to_string()
        } else {
            "ok".to_string()
        };

        MonitorResult { status, alerts, passed }
    }

    /// Return `true` when a watcher action has raised the block flag, meaning
    /// the next `onto_apply` call should be refused until the issue is
    /// resolved and [`clear_blocked`][Monitor::clear_blocked] is called.
    ///
    /// The blocked state is persisted in SQLite so it survives process
    /// restarts when the same `StateDb` path is reused.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::monitor::Monitor;
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::graph::GraphStore;
    /// use std::path::Path;
    /// use std::sync::Arc;
    ///
    /// let db    = StateDb::open(Path::new(":memory:")).unwrap();
    /// let graph = Arc::new(GraphStore::new());
    /// let monitor = Monitor::new(db, graph);
    ///
    /// // A freshly created monitor is never blocked.
    /// assert!(!monitor.is_blocked());
    /// ```
    pub fn is_blocked(&self) -> bool {
        let conn = self.db.conn();
        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM monitor_state WHERE key = 'blocked'",
                [],
                |r| r.get(0),
            )
            .ok();
        result.as_deref() == Some("true")
    }

    /// Raise or lower the block flag. Pass `true` to block and `false` to
    /// unblock. Prefer [`clear_blocked`][Monitor::clear_blocked] for the
    /// unblock case — it is more expressive at call sites.
    ///
    /// This is called automatically by [`run_watchers`][Monitor::run_watchers]
    /// when a watcher with `WatcherAction::BlockNextApply` exceeds its
    /// threshold.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::monitor::Monitor;
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::graph::GraphStore;
    /// use std::path::Path;
    /// use std::sync::Arc;
    ///
    /// let db    = StateDb::open(Path::new(":memory:")).unwrap();
    /// let graph = Arc::new(GraphStore::new());
    /// let monitor = Monitor::new(db, graph);
    ///
    /// // Starts unblocked.
    /// assert!(!monitor.is_blocked());
    ///
    /// // Raise the block flag.
    /// monitor.set_blocked(true);
    /// assert!(monitor.is_blocked());
    ///
    /// // Lower it again.
    /// monitor.set_blocked(false);
    /// assert!(!monitor.is_blocked());
    /// ```
    pub fn set_blocked(&self, blocked: bool) {
        let conn = self.db.conn();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO monitor_state (key, value) VALUES ('blocked', ?1)",
            rusqlite::params![if blocked { "true" } else { "false" }],
        );
    }

    /// Clear the block flag, allowing the next `onto_apply` call to proceed.
    ///
    /// This is the standard resolution path after a practitioner has
    /// investigated a monitor alert: fix the root cause in the ontology, then
    /// call `clear_blocked` to re-open the apply gate.
    ///
    /// # Examples
    ///
    /// Full blocked → unblocked lifecycle:
    ///
    /// ```
    /// use open_ontologies::monitor::Monitor;
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::graph::GraphStore;
    /// use std::path::Path;
    /// use std::sync::Arc;
    ///
    /// let db    = StateDb::open(Path::new(":memory:")).unwrap();
    /// let graph = Arc::new(GraphStore::new());
    /// let monitor = Monitor::new(db, graph);
    ///
    /// // Initially not blocked.
    /// assert!(!monitor.is_blocked());
    ///
    /// // A threshold breach blocks the apply gate.
    /// monitor.set_blocked(true);
    /// assert!(monitor.is_blocked());
    ///
    /// // After resolving the issue, clear the block.
    /// monitor.clear_blocked();
    /// assert!(!monitor.is_blocked());
    /// ```
    pub fn clear_blocked(&self) {
        self.set_blocked(false);
    }

    fn load_watchers(&self) -> Vec<Watcher> {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare("SELECT id, check_type, threshold, severity, action, query, message, webhook_url, webhook_headers FROM monitor_watchers WHERE enabled = 1")
            .unwrap();
        stmt.query_map([], |row| {
            let action_str: String = row.get(4)?;
            let action = match action_str.as_str() {
                "block_next_apply" => WatcherAction::BlockNextApply,
                "auto_rollback" => WatcherAction::AutoRollback,
                "log" => WatcherAction::Log,
                _ => WatcherAction::Notify,
            };
            Ok(Watcher {
                id: row.get(0)?,
                check_type: row.get(1)?,
                threshold: row.get(2)?,
                severity: row.get(3)?,
                action,
                query: row.get(5)?,
                message: row.get(6)?,
                webhook_url: row.get(7)?,
                webhook_headers: row.get(8)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    fn evaluate_watcher(&self, watcher: &Watcher) -> f64 {
        match watcher.check_type.as_str() {
            "sparql" => self.eval_sparql_watcher(watcher),
            // OntoStar Stream 4 — Loop 5 surface. The watcher's `query` field
            // (when present) is interpreted as the lower-bound RFC3339
            // timestamp; otherwise we look back 24h. The returned value is
            // the count of `conformance_regression_detected` events.
            "conformance_regression" => self.eval_conformance_regression(watcher),
            _ => 0.0,
        }
    }

    fn eval_conformance_regression(&self, watcher: &Watcher) -> f64 {
        let store = crate::ocel_store::OcelStore::new(self.db.clone());
        let since = watcher
            .query
            .clone()
            .unwrap_or_else(|| (chrono::Utc::now() - chrono::Duration::hours(24)).to_rfc3339());
        crate::feedback::regression::count_regressions_since(&store, &since)
            .map(|n| n as f64)
            .unwrap_or(0.0)
    }

    fn eval_sparql_watcher(&self, watcher: &Watcher) -> f64 {
        let query = match &watcher.query {
            Some(q) => q,
            None => return 0.0,
        };
        // Expect a SELECT query returning a ?count binding
        match self.graph.sparql_select(query) {
            Ok(json) => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json)
                    && let Some(results) = parsed["results"].as_array()
                        && let Some(first) = results.first()
                            && let Some(count_str) = first["count"].as_str() {
                                // Oxigraph returns literal like "\"1\"^^<http://...>"
                                let cleaned = count_str
                                    .trim_matches('"')
                                    .split("^^")
                                    .next()
                                    .unwrap_or("0")
                                    .trim_matches('"');
                                return cleaned.parse().unwrap_or(0.0);
                            }
                0.0
            }
            Err(_) => 0.0,
        }
    }

    /// Get the label of the most recently saved ontology version.
    fn latest_version_label(&self) -> Option<String> {
        let conn = self.db.conn();
        conn.query_row(
            "SELECT label FROM ontology_versions ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok()
    }
}

/// Spawn a background task that runs all enabled watchers every `interval` seconds.
/// Actions (auto_rollback, block_next_apply, notify) fire automatically on threshold breach.
/// Returns a `JoinHandle` that can be aborted to stop the loop.
pub fn start_background_loop(
    db: StateDb,
    graph: Arc<GraphStore>,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        // The first tick fires immediately — skip it so we don't run before any ontology is loaded.
        tick.tick().await;

        eprintln!("[watch] background monitor started (interval: {}s)", interval.as_secs());

        loop {
            tick.tick().await;

            let monitor = Monitor::new(db.clone(), graph.clone());
            let watchers = monitor.load_watchers();
            if watchers.is_empty() {
                continue;
            }

            let result = monitor.run_watchers();
            match result.status.as_str() {
                "ok" => {} // silent on healthy sweeps
                status => {
                    eprintln!(
                        "[watch] sweep: status={}, alerts={}, passed={}",
                        status,
                        result.alerts.len(),
                        result.passed.len(),
                    );
                    for alert in &result.alerts {
                        eprintln!(
                            "[watch]   {} ({}): value={} > threshold={} → {}",
                            alert.watcher, alert.severity, alert.value, alert.threshold, alert.action,
                        );
                    }
                }
            }
        }
    })
}
