//! R7 WA2 — A2 V1 Receipt-Chain Verifier worker.
//!
//! ZERO LLM by invariant. Crypto verdicts must be reproducible bit-for-bit
//! from `(receipt_row, trusted_keys_history_row)`. An LLM in this code
//! path would be a §22 regression.
//!
//! Mirrors [`crate::retention::RetentionWorker`]: a `tokio::spawn`'d loop
//! that ticks on `cfg.tick_secs`, calls one [`tick`](VerifierWorker::tick)
//! per cycle, and logs (does not panic) on failure.
//!
//! # Tick algorithm
//!
//! 1. Snapshot the cursor `last_verified_seq` (Arc<AtomicI64>).
//! 2. SELECT a batch of `receipts` rows where `sequence > cursor` ORDER BY
//!    `sequence ASC LIMIT batch_limit`.
//! 3. For each row, dispatch [`crate::verify::crypto_verify`]:
//!    - `Ok` → continue.
//!    - `SignatureExpiredKey` → emit `verifier_warning` (no retention pause).
//!    - `SignatureCorrupted | UnknownKey | BodyHashMismatch` → emit
//!      `verifier_failure`, advance `retention_paused_until` via
//!      `fetch_max(now + pause_secs)`, and tracing::error with the
//!      `andon` target so log scrapers can stop the line.
//! 4. Advance the cursor to the largest `sequence` we processed.
//! 5. Emit `verifier_tick_completed` with scanned/warnings/failures
//!    counters and the new cursor.
//!
//! # Idempotency
//!
//! Every emit uses `event_id = format!("verifier_<kind>::<receipt_hash>")`.
//! `ocel_events.event_id` is a primary key, and we INSERT OR IGNORE the
//! row directly — re-running the same tick (or restarting the worker
//! before it persisted the cursor) cannot produce duplicate OCEL rows.

use crate::config::VerifierConfig;
use crate::ocel_store::OcelStore;
use crate::state::StateDb;
use crate::verify::{crypto_verify, VerifierError, VerifierReceiptRow};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Outcome of a single verifier pass. All counts are best-effort — they
/// reflect rows actually examined and verdicts actually emitted (after
/// the INSERT OR IGNORE idempotency filter).
///
/// # Examples
///
/// ```
/// use open_ontologies::verifier_worker::VerifierReport;
///
/// // Default report: all counters zero, cursors at origin.
/// let report = VerifierReport::default();
/// assert_eq!(report.scanned, 0);
/// assert_eq!(report.warnings, 0);
/// assert_eq!(report.failures, 0);
/// assert_eq!(report.cursor_before, 0);
/// assert_eq!(report.cursor_after, 0);
///
/// // Clone and mutate independently.
/// let mut pass = report.clone();
/// pass.scanned = 17;
/// pass.cursor_after = 42;
/// assert_eq!(pass.scanned, 17);
/// assert_eq!(pass.cursor_after, 42);
///
/// // A clean pass: all scanned, no warnings or failures.
/// let clean = VerifierReport { scanned: 100, cursor_before: 50, cursor_after: 150, ..Default::default() };
/// assert_eq!(clean.warnings, 0);
/// assert_eq!(clean.failures, 0);
/// assert_eq!(clean.cursor_after - clean.cursor_before, 100);
/// ```
///
/// The `Debug` implementation includes all field names so log output is
/// human-readable without additional formatting:
///
/// ```
/// use open_ontologies::verifier_worker::VerifierReport;
///
/// let report = VerifierReport {
///     scanned: 3,
///     warnings: 1,
///     failures: 0,
///     cursor_before: 10,
///     cursor_after: 13,
/// };
/// let debug_str = format!("{:?}", report);
/// // All public field names must appear in the Debug output.
/// assert!(debug_str.contains("scanned"), "missing scanned in: {debug_str}");
/// assert!(debug_str.contains("warnings"), "missing warnings in: {debug_str}");
/// assert!(debug_str.contains("failures"), "missing failures in: {debug_str}");
/// assert!(debug_str.contains("cursor_before"), "missing cursor_before in: {debug_str}");
/// assert!(debug_str.contains("cursor_after"), "missing cursor_after in: {debug_str}");
/// ```
///
/// A report with only failures set (warnings=0) models a hard corruption
/// verdict where expired-key warnings were not triggered:
///
/// ```
/// use open_ontologies::verifier_worker::VerifierReport;
///
/// let corrupt = VerifierReport {
///     scanned: 1,
///     failures: 1,
///     warnings: 0,
///     cursor_before: 99,
///     cursor_after: 100,
/// };
/// // failures is accessible and non-zero; warnings stays at zero.
/// assert!(corrupt.failures > 0);
/// assert_eq!(corrupt.warnings, 0);
/// // scanned == failures means every examined receipt was corrupt.
/// assert_eq!(corrupt.scanned, corrupt.failures);
/// ```
#[derive(Debug, Clone, Default)]
pub struct VerifierReport {
    pub scanned: u64,
    pub warnings: u64,
    pub failures: u64,
    pub cursor_before: i64,
    pub cursor_after: i64,
}

/// Background worker that crypto-verifies receipts on a continuous tick.
pub struct VerifierWorker {
    pub db: StateDb,
    pub ocel: Arc<OcelStore>,
    pub cfg: VerifierConfig,
    /// Shared with [`crate::retention::RetentionWorker`]. On a corruption
    /// verdict the worker calls `fetch_max(now + pause_secs)` so the
    /// retention loop sees an imminent cutoff and skips its next tick.
    /// Monotone — never shortens an existing pause.
    pub retention_paused_until: Arc<AtomicI64>,
    /// Per-worker checkpoint. The next SELECT WHERE `sequence > cursor`.
    /// Survives across ticks. On startup it begins at 0 (back-fill all
    /// historical receipts on first run); production deployments persist
    /// this externally if back-fill is undesirable.
    pub last_verified_seq: Arc<AtomicI64>,
    /// Wall-clock RFC-3339 of the most recently completed tick. Used by
    /// tests and observability.
    pub last_run_unix: Arc<AtomicI64>,
}

impl VerifierWorker {
    /// Construct a new worker. The worker starts with `last_verified_seq = 0`
    /// so the first tick back-fills all historical receipts.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicI64, Ordering};
    /// use open_ontologies::verifier_worker::VerifierWorker;
    /// use open_ontologies::config::VerifierConfig;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use open_ontologies::state::StateDb;
    ///
    /// let db  = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let ocel = Arc::new(OcelStore::new(
    ///     StateDb::open(std::path::Path::new(":memory:")).unwrap(),
    /// ));
    /// let cfg = VerifierConfig::default();
    /// let pause = Arc::new(AtomicI64::new(0));
    ///
    /// let worker = VerifierWorker::new(db, ocel, cfg, pause);
    /// // Cursor starts at 0 — will back-fill all receipts on first tick.
    /// assert_eq!(worker.last_verified_seq.load(Ordering::Relaxed), 0);
    /// // last_run_unix also starts at 0 — no tick has run yet.
    /// assert_eq!(worker.last_run_unix.load(Ordering::Relaxed), 0);
    /// // retention_paused_until starts at 0 — retention not paused.
    /// assert_eq!(worker.retention_paused_until.load(Ordering::Relaxed), 0);
    /// ```
    ///
    /// # Custom batch limit
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::sync::atomic::AtomicI64;
    /// use open_ontologies::verifier_worker::VerifierWorker;
    /// use open_ontologies::config::VerifierConfig;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use open_ontologies::state::StateDb;
    ///
    /// let db   = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let ocel = Arc::new(OcelStore::new(
    ///     StateDb::open(std::path::Path::new(":memory:")).unwrap(),
    /// ));
    /// // Low batch limit for high-throughput sharding use-cases.
    /// let cfg = VerifierConfig { batch_limit: 100, ..VerifierConfig::default() };
    /// let pause = Arc::new(AtomicI64::new(0));
    ///
    /// let worker = VerifierWorker::new(db, ocel, cfg, pause);
    /// assert_eq!(worker.cfg.batch_limit, 100);
    /// ```
    ///
    /// Batch limit of 5 — the smallest meaningful sharding unit for tests
    /// where each tick must process at most 5 receipts at a time:
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicI64, Ordering};
    /// use open_ontologies::verifier_worker::VerifierWorker;
    /// use open_ontologies::config::VerifierConfig;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use open_ontologies::state::StateDb;
    ///
    /// let db   = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let ocel = Arc::new(OcelStore::new(
    ///     StateDb::open(std::path::Path::new(":memory:")).unwrap(),
    /// ));
    /// let cfg = VerifierConfig { batch_limit: 5, ..VerifierConfig::default() };
    /// let pause = Arc::new(AtomicI64::new(0));
    ///
    /// let worker = VerifierWorker::new(db, ocel, cfg, pause);
    /// // batch_limit is stored verbatim.
    /// assert_eq!(worker.cfg.batch_limit, 5);
    /// // Cursor and run-clock still start at zero.
    /// assert_eq!(worker.last_verified_seq.load(Ordering::Relaxed), 0);
    /// assert_eq!(worker.last_run_unix.load(Ordering::Relaxed), 0);
    /// ```
    pub fn new(
        db: StateDb,
        ocel: Arc<OcelStore>,
        cfg: VerifierConfig,
        retention_paused_until: Arc<AtomicI64>,
    ) -> Self {
        Self {
            db,
            ocel,
            cfg,
            retention_paused_until,
            last_verified_seq: Arc::new(AtomicI64::new(0)),
            last_run_unix: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Spawn the loop. Returns `(JoinHandle, last_verified_seq)`. The
    /// caller can read the cursor at any time.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() {
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicI64, Ordering};
    /// use open_ontologies::verifier_worker::VerifierWorker;
    /// use open_ontologies::config::VerifierConfig;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use open_ontologies::state::StateDb;
    ///
    /// let db  = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let ocel = Arc::new(OcelStore::new(
    ///     StateDb::open(std::path::Path::new(":memory:")).unwrap(),
    /// ));
    /// let mut cfg = VerifierConfig::default();
    /// cfg.tick_secs = 60;
    /// let pause = Arc::new(AtomicI64::new(0));
    ///
    /// let (_handle, cursor) = VerifierWorker::spawn_with_cursor(db, ocel, cfg, pause);
    /// // Cursor is still 0 — the first tick hasn't fired yet.
    /// assert_eq!(cursor.load(Ordering::Relaxed), 0);
    /// # }
    /// ```
    pub fn spawn_with_cursor(
        db: StateDb,
        ocel: Arc<OcelStore>,
        cfg: VerifierConfig,
        retention_paused_until: Arc<AtomicI64>,
    ) -> (tokio::task::JoinHandle<()>, Arc<AtomicI64>) {
        let interval_secs = cfg.tick_secs.max(1);
        let worker = Self::new(db, ocel, cfg, retention_paused_until);
        let cursor = worker.last_verified_seq.clone();
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
            // Skip the immediate tick so the verifier does not run before
            // the server has admitted anything.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                if !worker.cfg.enabled {
                    continue;
                }
                match worker.tick() {
                    Ok(report) => {
                        if report.scanned > 0 || report.failures > 0 || report.warnings > 0 {
                            tracing::info!(
                                target: "verifier",
                                scanned = report.scanned,
                                warnings = report.warnings,
                                failures = report.failures,
                                cursor = report.cursor_after,
                                "verifier tick"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(target: "verifier", "verifier tick failed: {e}");
                    }
                }
            }
        });
        (handle, cursor)
    }

    /// Run a single verifier pass synchronously. Tests drive this
    /// directly with a fresh tempdir StateDb.
    ///
    /// OTEL: emits `verifier.tick` span with `verifier.scanned`,
    /// `verifier.failures`, `verifier.cursor` attributes.
    ///
    /// # Examples
    ///
    /// Empty DB — zero rows scanned, all counters at origin after the tick:
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::sync::atomic::AtomicI64;
    /// use open_ontologies::verifier_worker::VerifierWorker;
    /// use open_ontologies::config::VerifierConfig;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use open_ontologies::state::StateDb;
    ///
    /// let db  = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let ocel = Arc::new(OcelStore::new(
    ///     StateDb::open(std::path::Path::new(":memory:")).unwrap(),
    /// ));
    /// let cfg = VerifierConfig::default();
    /// let pause = Arc::new(AtomicI64::new(0));
    ///
    /// let worker = VerifierWorker::new(db, ocel, cfg, pause);
    /// // Empty receipts table → scanned = 0, no failures, no warnings.
    /// let report = worker.tick().unwrap();
    /// assert_eq!(report.scanned, 0);
    /// assert_eq!(report.warnings, 0);
    /// assert_eq!(report.failures, 0);
    /// assert_eq!(report.cursor_before, 0);
    /// assert_eq!(report.cursor_after, 0);
    /// ```
    ///
    /// Multiple consecutive ticks on an empty DB are idempotent — cursor
    /// stays at 0 and no failures are emitted:
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::sync::atomic::AtomicI64;
    /// use open_ontologies::verifier_worker::VerifierWorker;
    /// use open_ontologies::config::VerifierConfig;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use open_ontologies::state::StateDb;
    ///
    /// let db  = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let ocel = Arc::new(OcelStore::new(
    ///     StateDb::open(std::path::Path::new(":memory:")).unwrap(),
    /// ));
    /// let pause = Arc::new(AtomicI64::new(0));
    /// let worker = VerifierWorker::new(db, ocel, VerifierConfig::default(), pause);
    ///
    /// for _ in 0..3 {
    ///     let r = worker.tick().unwrap();
    ///     assert_eq!(r.cursor_after, 0);
    ///     assert_eq!(r.failures, 0);
    /// }
    /// ```
    ///
    /// `last_run_unix` advances from 0 to a positive Unix timestamp after
    /// the first tick — confirming the worker recorded wall-clock evidence:
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicI64, Ordering};
    /// use open_ontologies::verifier_worker::VerifierWorker;
    /// use open_ontologies::config::VerifierConfig;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use open_ontologies::state::StateDb;
    ///
    /// let db   = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let ocel = Arc::new(OcelStore::new(
    ///     StateDb::open(std::path::Path::new(":memory:")).unwrap(),
    /// ));
    /// let pause = Arc::new(AtomicI64::new(0));
    /// let worker = VerifierWorker::new(db, ocel, VerifierConfig::default(), pause);
    ///
    /// // Before any tick, the wall-clock is uninitialised (zero).
    /// assert_eq!(worker.last_run_unix.load(Ordering::Relaxed), 0);
    ///
    /// worker.tick().unwrap();
    ///
    /// // After the first tick, last_run_unix holds a plausible Unix epoch
    /// // value (greater than the year-2020 epoch floor: 1_577_836_800).
    /// let run_ts = worker.last_run_unix.load(Ordering::Relaxed);
    /// assert!(run_ts > 1_577_836_800, "last_run_unix={run_ts} should be post-2020");
    /// ```
    ///
    /// Empty DB: each tick returns a `VerifierReport` where `scanned`,
    /// `warnings`, and `failures` are all zero and both cursor fields agree:
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::sync::atomic::AtomicI64;
    /// use open_ontologies::verifier_worker::VerifierWorker;
    /// use open_ontologies::config::VerifierConfig;
    /// use open_ontologies::ocel_store::OcelStore;
    /// use open_ontologies::state::StateDb;
    ///
    /// let db   = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let ocel = Arc::new(OcelStore::new(
    ///     StateDb::open(std::path::Path::new(":memory:")).unwrap(),
    /// ));
    /// let pause = Arc::new(AtomicI64::new(0));
    /// let worker = VerifierWorker::new(db, ocel, VerifierConfig::default(), pause);
    ///
    /// let r = worker.tick().unwrap();
    /// // Both cursors are the same on an empty DB — nothing was advanced.
    /// assert_eq!(r.cursor_before, r.cursor_after);
    /// // scanned == 0 means no receipts were fetched from the DB.
    /// assert_eq!(r.scanned, 0);
    /// // The report satisfies the no-failure invariant: warnings + failures == 0.
    /// assert_eq!(r.warnings + r.failures, 0);
    /// ```
    pub fn tick(&self) -> anyhow::Result<VerifierReport> {
        let span = tracing::info_span!(
            target: "verifier",
            "verifier.tick",
            verifier.cursor = tracing::field::Empty,
            verifier.scanned = tracing::field::Empty,
            verifier.failures = tracing::field::Empty,
            verifier.warnings = tracing::field::Empty,
        );
        let _enter = span.enter();

        let cursor_before = self.last_verified_seq.load(Ordering::Relaxed);
        let mut report = VerifierReport {
            cursor_before,
            cursor_after: cursor_before,
            ..Default::default()
        };

        // Tick-started OCEL anchor. Idempotent across worker restarts via
        // a wall-clock event_id (one per second ceiling is plenty —
        // collisions inside a single second collapse to one row).
        let now_iso = chrono::Utc::now().to_rfc3339();
        self.emit_idempotent(
            &format!(
                "verifier_tick_started::{}",
                chrono::Utc::now().timestamp_millis()
            ),
            "verifier_tick_started",
            &now_iso,
            &[("cursor", &cursor_before.to_string())],
        ).ok();

        // Single SELECT batch — no `.await` between the conn() acquire
        // and conn drop (rusqlite::MutexGuard is !Send).
        let rows = self.fetch_batch(cursor_before, self.cfg.batch_limit)?;
        report.scanned = rows.len() as u64;

        let mut max_seq = cursor_before;
        for row in &rows {
            if row.sequence > max_seq {
                max_seq = row.sequence;
            }
            // Per-row OTEL span: deterministic verdict on receipt_hash.
            let verify_span = tracing::debug_span!(
                target: "verifier",
                "verifier.verify_one",
                verifier.receipt_hash = %row.receipt_hash,
                verifier.sequence = row.sequence,
            );
            let _v = verify_span.enter();

            match crypto_verify(row, &self.db) {
                Ok(()) => {}
                Err(VerifierError::SignatureExpiredKey {
                    granted_at,
                    removed_at,
                }) => {
                    report.warnings += 1;
                    let event_id =
                        format!("verifier_warning::{}", row.receipt_hash);
                    self.emit_idempotent(
                        &event_id,
                        "verifier_warning",
                        &now_iso,
                        &[
                            ("receipt_hash", row.receipt_hash.as_str()),
                            ("kind", "signature_expired_key"),
                            ("granted_at", granted_at.as_str()),
                            ("removed_at", removed_at.as_str()),
                            ("session_id", row.session_id.as_str()),
                        ],
                    ).ok();
                }
                Err(err) => {
                    report.failures += 1;
                    let kind = err.kind();
                    let event_id =
                        format!("verifier_failure::{}", row.receipt_hash);
                    let mut attrs: Vec<(&str, &str)> = vec![
                        ("receipt_hash", row.receipt_hash.as_str()),
                        ("kind", kind),
                        ("granted_at", row.granted_at.as_str()),
                        ("key_valid_at", row.key_valid_at.as_str()),
                        ("session_id", row.session_id.as_str()),
                    ];
                    let removed_at_owned: Option<String> = match &err {
                        VerifierError::SignatureCorrupted {
                            removed_at, ..
                        } => removed_at.clone(),
                        _ => None,
                    };
                    if let Some(ref ra) = removed_at_owned {
                        attrs.push(("removed_at", ra.as_str()));
                    }
                    self.emit_idempotent(
                        &event_id,
                        "verifier_failure",
                        &now_iso,
                        &attrs,
                    ).ok();
                    if self.cfg.pause_retention_on_failure {
                        let pause_until = chrono::Utc::now().timestamp()
                            .saturating_add(
                                (self.cfg.pause_minutes_on_failure
                                    .saturating_mul(60))
                                    as i64,
                            );
                        // Monotone — fetch_max never shortens an
                        // already-set pause.
                        self.retention_paused_until
                            .fetch_max(pause_until, Ordering::Relaxed);
                    }
                    if self.cfg.andon_on_failure {
                        tracing::error!(
                            target: "andon",
                            kind = %kind,
                            receipt_hash = %row.receipt_hash,
                            session_id = %row.session_id,
                            granted_at = %row.granted_at,
                            key_valid_at = %row.key_valid_at,
                            "receipt-chain verifier failure"
                        );
                    }
                }
            }
        }

        // Persist the cursor only after the batch processed; if
        // emit_idempotent succeeded for failures, the OCEL row has the
        // tamper evidence. If it crashed mid-batch the cursor isn't
        // bumped, so the next tick re-examines those rows — and the
        // INSERT OR IGNORE on the deterministic event_id collapses
        // duplicates.
        self.last_verified_seq.store(max_seq, Ordering::Relaxed);
        report.cursor_after = max_seq;
        self.last_run_unix
            .store(chrono::Utc::now().timestamp(), Ordering::Relaxed);

        // Tick-completed OCEL anchor.
        self.emit_idempotent(
            &format!(
                "verifier_tick_completed::{}",
                chrono::Utc::now().timestamp_millis()
            ),
            "verifier_tick_completed",
            &now_iso,
            &[
                ("scanned", &report.scanned.to_string()),
                ("warnings", &report.warnings.to_string()),
                ("failures", &report.failures.to_string()),
                ("cursor", &report.cursor_after.to_string()),
            ],
        ).ok();

        span.record("verifier.cursor", report.cursor_after);
        span.record("verifier.scanned", report.scanned);
        span.record("verifier.failures", report.failures);
        span.record("verifier.warnings", report.warnings);

        Ok(report)
    }

    /// Fetch one batch of receipts past the cursor.
    fn fetch_batch(
        &self,
        cursor: i64,
        limit: i64,
    ) -> anyhow::Result<Vec<VerifierReceiptRow>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT receipt_hash, sequence, session_id, scope_token,
                    granted_at, key_valid_at
             FROM receipts
             WHERE sequence > ?1
             ORDER BY sequence ASC
             LIMIT ?2",
        )?;
        let rows: Vec<VerifierReceiptRow> = stmt
            .query_map(rusqlite::params![cursor, limit], |r| {
                Ok(VerifierReceiptRow {
                    receipt_hash: r.get(0)?,
                    sequence: r.get(1)?,
                    session_id: r.get(2)?,
                    scope_token: r.get(3)?,
                    granted_at: r.get(4)?,
                    key_valid_at: r.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// INSERT OR IGNORE the OCEL event row directly. This bypasses
    /// [`OcelStore::emit_event`] (which uses plain INSERT and would fail
    /// on the deterministic event_id PK clash) so duplicate ticks for
    /// the same `receipt_hash` collapse to a single row.
    ///
    /// `attrs` are stored as `ocel_event_attrs` rows (also INSERT OR
    /// IGNORE so the second tick doesn't double-write attributes).
    fn emit_idempotent(
        &self,
        event_id: &str,
        event_type: &str,
        time_iso: &str,
        attrs: &[(&str, &str)],
    ) -> anyhow::Result<bool> {
        let conn = self.ocel.db().conn();
        let inserted = conn.execute(
            "INSERT OR IGNORE INTO ocel_events
                (event_id, event_type, time, session_id, scope_token, tenant_id)
             VALUES (?1, ?2, ?3, '', NULL, 'default')",
            rusqlite::params![event_id, event_type, time_iso],
        )?;
        if inserted == 0 {
            // Already present from a prior tick — idempotent no-op.
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
