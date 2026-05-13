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
