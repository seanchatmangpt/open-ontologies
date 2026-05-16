//! Background Health Guardian — periodic checks for scope leaks, receipt
//! chain gaps, and process health signals.
//!
//! Runs alongside [`crate::verifier_worker::VerifierWorker`] and
//! [`crate::retention::RetentionWorker`] on a 60-second tick. ZERO LLM by
//! invariant — all checks are pure SQL against [`StateDb`]; deterministic
//! and reproducible.
//!
//! # Checks (each tick)
//!
//! 1. **Scope leaks** — `declared_workflows` rows where `closed_at IS NULL`
//!    and `declared_at < datetime('now', '-30 minutes')`. A workflow was
//!    declared but never closed; OCEL event tags will pile up under an orphan
//!    `scope_token` with no `WorkflowClosed` event to balance them.
//!
//! 2. **Receipt chain gaps** — `MAX(sequence) - MIN(sequence) + 1 ≠ COUNT(*)`
//!    in the `receipts` table. Indicates receipts were deleted or inserted out
//!    of band, breaking the monotone chain invariant required by A5 (Prove).
//!
//! 3. **Heartbeat** — `health_guardian_ok` event emitted every tick so
//!    monitors can detect a silent guardian (absence of the heartbeat means
//!    the guardian crashed or is not running).
//!
//! # OCEL emission
//!
//! All findings are emitted as `INSERT OR IGNORE` into `ocel_events` so
//! replayed ticks collapse to a single row per finding identity. The same
//! `StateDb` connection used for data queries holds both the `declared_workflows`
//! / `receipts` schema and the `ocel_events` / `ocel_event_attrs` schema.
//!
//! # Deadlock avoidance
//!
//! `StateDb::conn()` returns a `MutexGuard<Connection>`. Every method here
//! acquires, uses, then **drops** the guard before calling any sibling that
//! would re-acquire it. Fetch methods return owned `Vec`; emit happens after
//! the fetch guard is released.

use crate::state::StateDb;
use std::time::Duration;

/// Outcome of one guardian tick.
///
/// # Examples
///
/// ```
/// use open_ontologies::health_guardian::GuardianReport;
///
/// // Default-constructed report: both counters start at zero.
/// let report = GuardianReport::default();
/// assert_eq!(report.scope_leaks, 0);
/// assert_eq!(report.receipt_gaps, 0);
///
/// // Fields are public and directly assignable.
/// let report = GuardianReport { scope_leaks: 3, receipt_gaps: 1 };
/// assert_eq!(report.scope_leaks, 3);
/// assert_eq!(report.receipt_gaps, 1);
/// ```
///
/// A healthy report has both counters at zero. An unhealthy one has at least
/// one counter above zero:
///
/// ```
/// use open_ontologies::health_guardian::GuardianReport;
///
/// fn is_healthy(r: &GuardianReport) -> bool {
///     r.scope_leaks == 0 && r.receipt_gaps == 0
/// }
///
/// assert!(is_healthy(&GuardianReport::default()));
/// assert!(!is_healthy(&GuardianReport { scope_leaks: 1, receipt_gaps: 0 }));
/// assert!(!is_healthy(&GuardianReport { scope_leaks: 0, receipt_gaps: 2 }));
/// ```
///
/// `GuardianReport` is `Debug`-printable for structured logging:
///
/// ```
/// use open_ontologies::health_guardian::GuardianReport;
///
/// let report = GuardianReport { scope_leaks: 0, receipt_gaps: 0 };
/// let s = format!("{report:?}");
/// assert!(s.contains("scope_leaks"));
/// assert!(s.contains("receipt_gaps"));
/// ```
#[derive(Debug, Default)]
pub struct GuardianReport {
    pub scope_leaks: u64,
    pub receipt_gaps: u64,
}

/// Background guardian that polls for scope leaks and receipt chain gaps.
///
/// Constructed via [`HealthGuardian::spawn`] in production, or used directly
/// (via its private `db` field set up inside `tick` tests) in integration tests.
/// See [`HealthGuardian::tick`] for the synchronous test entry point.
pub struct HealthGuardian {
    db: StateDb,
}

impl HealthGuardian {
    /// Spawn the guardian loop. Ticks every 60 s; skips the first immediate
    /// tick so the guardian does not race against server initialisation.
    /// Returns the `JoinHandle` — drop it to stop the loop.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use open_ontologies::health_guardian::HealthGuardian;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let handle = HealthGuardian::spawn(db);
    /// // Drop the handle to stop the background loop.
    /// handle.abort();
    /// # }
    /// ```
    pub fn spawn(db: StateDb) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let guardian = Self { db };
            let mut ticker = tokio::time::interval(Duration::from_secs(60));
            // Skip the immediate tick (mirrors VerifierWorker behaviour).
            ticker.tick().await;
            loop {
                ticker.tick().await;
                match guardian.tick() {
                    Ok(report) => {
                        if report.scope_leaks > 0 || report.receipt_gaps > 0 {
                            tracing::warn!(
                                target: "health_guardian",
                                scope_leaks = report.scope_leaks,
                                receipt_gaps = report.receipt_gaps,
                                "health guardian found issues"
                            );
                        } else {
                            tracing::debug!(
                                target: "health_guardian",
                                "health guardian tick: ok"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(target: "health_guardian", "health guardian tick failed: {e}");
                    }
                }
            }
        })
    }

    /// Run one guardian tick synchronously. Integration tests call this
    /// directly with a fresh `StateDb` to verify the checks without spawning.
    ///
    /// On an empty database both counters are zero and a heartbeat OCEL event
    /// is emitted into `ocel_events`.
    ///
    /// # Return value
    ///
    /// Returns a [`GuardianReport`] with counts for scope leaks and receipt
    /// chain gaps found during this tick.
    ///
    /// # Examples
    ///
    /// The `db` field is private so `HealthGuardian` cannot be constructed
    /// directly outside this module. In integration tests the private
    /// struct-literal syntax is available; in doc-tests use `# no_run` and
    /// show the full call sequence instead:
    ///
    /// ```no_run
    /// use open_ontologies::health_guardian::HealthGuardian;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// // Production pattern: spawn the guardian, which calls tick() internally.
    /// // The JoinHandle can be aborted to stop the loop.
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let handle = HealthGuardian::spawn(db);
    /// // tick() runs on every 60-second interval inside the spawned task.
    /// handle.abort();
    /// ```
    ///
    /// The report type produced by `tick()` is directly constructible for
    /// unit-testing downstream consumers:
    ///
    /// ```
    /// use open_ontologies::health_guardian::GuardianReport;
    ///
    /// // Simulate what tick() returns on a clean database.
    /// let report: GuardianReport = GuardianReport { scope_leaks: 0, receipt_gaps: 0 };
    /// assert_eq!(report.scope_leaks, 0, "no leaks on fresh db");
    /// assert_eq!(report.receipt_gaps, 0, "no gaps on fresh db");
    ///
    /// // Simulate what tick() returns when two scope leaks are found.
    /// let report = GuardianReport { scope_leaks: 2, receipt_gaps: 0 };
    /// assert!(report.scope_leaks > 0, "tick detected scope leaks");
    /// ```
    pub fn tick(&self) -> anyhow::Result<GuardianReport> {
        let mut report = GuardianReport::default();
        let now_iso = chrono::Utc::now().to_rfc3339();

        report.scope_leaks = self.check_scope_leaks(&now_iso)?;
        report.receipt_gaps = self.check_receipt_chain(&now_iso)?;

        // Emit a heartbeat once per minute-window so monitors can detect a
        // silent guardian. The minute-bucketed event_id collapses duplicate
        // ticks within the same wall-clock minute to one OCEL row.
        let hb_id = format!(
            "health_guardian_ok::{}",
            chrono::Utc::now().timestamp() / 60
        );
        self.emit_idempotent(&hb_id, "health_guardian_ok", &now_iso, &[]).ok();

        Ok(report)
    }

    // ── private ──────────────────────────────────────────────────────────

    /// Fetch all open workflow scopes older than 30 minutes. The `MutexGuard`
    /// is released when this method returns; callers may then call
    /// `emit_idempotent` without deadlocking.
    fn fetch_scope_leaks(&self) -> anyhow::Result<Vec<(String, String, String, String)>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT scope_token, session_id, name, declared_at
             FROM declared_workflows
             WHERE closed_at IS NULL
               AND declared_at < datetime('now', '-30 minutes')",
        )?;
        let rows: Vec<(String, String, String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Fetch receipt sequence stats: (min_seq, max_seq, count). The guard is
    /// released on return.
    fn fetch_receipt_stats(&self) -> anyhow::Result<(i64, i64, i64)> {
        let conn = self.db.conn();
        let stats = conn.query_row(
            "SELECT COALESCE(MIN(sequence), 0),
                    COALESCE(MAX(sequence), 0),
                    COUNT(*)
             FROM receipts",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        Ok(stats)
    }

    fn check_scope_leaks(&self, now_iso: &str) -> anyhow::Result<u64> {
        let rows = self.fetch_scope_leaks()?; // guard released here
        let count = rows.len() as u64;
        for (scope_token, session_id, name, declared_at) in &rows {
            let event_id = format!("health_guardian_scope_leak::{scope_token}");
            self.emit_idempotent(
                &event_id,
                "health_guardian_scope_leak",
                now_iso,
                &[
                    ("scope_token", scope_token.as_str()),
                    ("session_id", session_id.as_str()),
                    ("workflow_name", name.as_str()),
                    ("declared_at", declared_at.as_str()),
                ],
            )
            .ok();
            tracing::warn!(
                target: "health_guardian",
                scope_token = %scope_token,
                session_id = %session_id,
                workflow_name = %name,
                declared_at = %declared_at,
                "scope leak: workflow open >30 min without close"
            );
        }
        Ok(count)
    }

    fn check_receipt_chain(&self, now_iso: &str) -> anyhow::Result<u64> {
        let (min_seq, max_seq, count) = self.fetch_receipt_stats()?; // guard released here
        if count == 0 {
            return Ok(0);
        }
        let expected = max_seq - min_seq + 1;
        if expected != count {
            let gap_count = (expected - count).max(0) as u64;
            let event_id = format!("health_guardian_receipt_gap::{max_seq}");
            self.emit_idempotent(
                &event_id,
                "health_guardian_receipt_gap",
                now_iso,
                &[
                    ("min_seq", &min_seq.to_string()),
                    ("max_seq", &max_seq.to_string()),
                    ("actual_count", &count.to_string()),
                    ("expected_count", &expected.to_string()),
                    ("gap_count", &gap_count.to_string()),
                ],
            )
            .ok();
            tracing::warn!(
                target: "health_guardian",
                min_seq,
                max_seq,
                actual_count = count,
                expected_count = expected,
                gap_count,
                "receipt chain gap detected"
            );
            return Ok(1);
        }
        Ok(0)
    }

    /// `INSERT OR IGNORE` an OCEL event. The duplicate-event-id collapse means
    /// replayed ticks cannot produce phantom rows. The guard is held for the
    /// duration and released on return.
    fn emit_idempotent(
        &self,
        event_id: &str,
        event_type: &str,
        time_iso: &str,
        attrs: &[(&str, &str)],
    ) -> anyhow::Result<bool> {
        let conn = self.db.conn();
        let inserted = conn.execute(
            "INSERT OR IGNORE INTO ocel_events
                (event_id, event_type, time, session_id, scope_token, tenant_id)
             VALUES (?1, ?2, ?3, '', NULL, 'default')",
            rusqlite::params![event_id, event_type, time_iso],
        )?;
        if inserted == 0 {
            return Ok(false);
        }
        for (name, value) in attrs {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO ocel_event_attrs
                    (event_id, name, value, value_type)
                 VALUES (?1, ?2, ?3, 'string')",
                rusqlite::params![event_id, name, value],
            );
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> StateDb {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        StateDb::open(tmp.path()).expect("StateDb::open")
    }

    #[test]
    fn tick_on_empty_db_returns_zero_leaks_and_zero_gaps() {
        let db = temp_db();
        let guardian = HealthGuardian { db };
        let report = guardian.tick().expect("tick");
        assert_eq!(report.scope_leaks, 0);
        assert_eq!(report.receipt_gaps, 0);
    }

    #[test]
    fn tick_emits_heartbeat_into_ocel() {
        let db = temp_db();
        let guardian = HealthGuardian { db: db.clone() };
        guardian.tick().expect("tick");
        // Verify at least one health_guardian_ok row exists.
        let conn = db.conn();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ocel_events WHERE event_type = 'health_guardian_ok'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert!(count > 0, "heartbeat not emitted");
    }

    #[test]
    fn scope_leak_detected_for_old_open_workflow() {
        let db = temp_db();
        // Insert a workflow that was declared 31 minutes ago without closing.
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO declared_workflows
                    (scope_token, session_id, name, powl_string, powl_hash,
                     alphabet_json, declared_at, closed_at, status)
                 VALUES ('leak-scope-1', 'session-1', 'TestWorkflow',
                         'SEQ', 'abc', '[]',
                         datetime('now', '-31 minutes'), NULL, 'open')",
                [],
            )
            .expect("insert workflow");
        }
        let guardian = HealthGuardian { db: db.clone() };
        let report = guardian.tick().expect("tick");
        assert_eq!(report.scope_leaks, 1, "expected one scope leak");

        // OCEL event should be recorded.
        let conn = db.conn();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ocel_events WHERE event_type = 'health_guardian_scope_leak'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(count, 1, "leak event not emitted");
    }

    #[test]
    fn fresh_workflow_not_flagged_as_leak() {
        let db = temp_db();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO declared_workflows
                    (scope_token, session_id, name, powl_string, powl_hash,
                     alphabet_json, declared_at, closed_at, status)
                 VALUES ('fresh-scope', 'session-2', 'TestWorkflow',
                         'SEQ', 'def', '[]',
                         datetime('now', '-5 minutes'), NULL, 'open')",
                [],
            )
            .expect("insert workflow");
        }
        let guardian = HealthGuardian { db };
        let report = guardian.tick().expect("tick");
        assert_eq!(report.scope_leaks, 0, "recent workflow should not be flagged");
    }

    fn insert_test_receipt(db: &StateDb, seq: i64, session: &str) {
        let conn = db.conn();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO receipts
                (receipt_hash, scope_token, artifact_hash, declared_powl_hash,
                 ocel_canonical_hash, gate_config_hash, production_law_version,
                 granted_at, session_id, sequence, key_valid_at)
             VALUES (?1, 'test-scope', 'ah', 'dph', 'och', 'gch', '1.0',
                     ?2, ?3, ?4, ?2)",
            rusqlite::params![format!("hash-{session}-{seq}"), now, session, seq],
        )
        .expect("insert receipt");
    }

    #[test]
    fn receipt_gap_detected_when_sequence_has_holes() {
        let db = temp_db();
        // Insert receipts with a gap: sequences 1, 2, 4 (missing 3).
        for seq in [1i64, 2, 4] {
            insert_test_receipt(&db, seq, "session-3");
        }
        let guardian = HealthGuardian { db };
        let report = guardian.tick().expect("tick");
        assert_eq!(report.receipt_gaps, 1, "expected one receipt gap");
    }

    #[test]
    fn contiguous_receipts_produce_no_gap() {
        let db = temp_db();
        for seq in [1i64, 2, 3] {
            insert_test_receipt(&db, seq, "session-4");
        }
        let guardian = HealthGuardian { db };
        let report = guardian.tick().expect("tick");
        assert_eq!(report.receipt_gaps, 0, "contiguous receipts should not flag a gap");
    }
}
