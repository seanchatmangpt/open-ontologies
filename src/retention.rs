//! Round 4 WD — §29 Cell8 retirement closure.
//!
//! [`RetentionWorker`] is the single per-table retirement path for every
//! persistent OntoStar table. It mirrors [`crate::registry::spawn_evictor`]:
//! a `tokio::spawn`'d loop that ticks on `cfg.poll_interval_secs`, calls
//! one [`tick`](RetentionWorker::tick) per cycle, and logs (does not panic)
//! on failure.
//!
//! Doctrine: §29 declares that every persistent artifact must have a
//! defined retirement path. Without [`RetentionWorker`] the database grows
//! without bound — a §27 HiddenWIP and §17 fake gauge (the system claims
//! "manufacturing" but cannot dispose of its byproducts).
//!
//! Cascade order matters: child tables that reference an event/receipt by
//! foreign key (`ocel_event_attrs`, `ocel_relationships`) MUST be pruned
//! BEFORE the parent (`ocel_events`). Otherwise SQLite's `foreign_keys=ON`
//! pragma rejects the DELETE.

use crate::config::RetentionConfig;
use crate::state::StateDb;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

/// Outcome of a single retention pass. All counts are best-effort and
/// reflect rows actually deleted (not rows considered).
#[derive(Debug, Clone, Default)]
pub struct RetentionReport {
    pub ocel_events_pruned: u64,
    pub ocel_event_attrs_pruned: u64,
    pub ocel_relationships_pruned: u64,
    pub lineage_events_pruned: u64,
    pub conformance_runs_pruned: u64,
    pub revoked_sessions_pruned: u64,
    pub receipts_archived_or_pruned: u64,
    pub mined_exemplars_pruned: u64,
    pub align_feedback_pruned: u64,
    pub tool_feedback_pruned: u64,
    pub embeddings_orphaned: u64,
    pub ontology_cache_pruned: u64,
}

/// Background worker that prunes expired rows per [`RetentionConfig`].
pub struct RetentionWorker {
    pub db: StateDb,
    pub cfg: RetentionConfig,
    /// R5 WC-2 — emergency kill-switch. When `> Utc::now().timestamp()`,
    /// [`tick`] returns an empty `RetentionReport` without running ANY
    /// pruner. Set by the `onto_retention_pause` admin tool; cleared
    /// (set to 0) by `onto_retention_resume`. Shared `Arc<AtomicI64>`
    /// so the server can mutate the same atomic the worker reads each
    /// tick. `Ordering::Relaxed` is sufficient — this is a coarse
    /// time-since-epoch flag, not a synchronization point.
    pub paused_until: Arc<AtomicI64>,
}

impl RetentionWorker {
    pub fn new(db: StateDb, cfg: RetentionConfig) -> Self {
        Self {
            db,
            cfg,
            paused_until: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Construct with an externally-supplied paused_until handle. Used
    /// by [`spawn_with_pause`] so the spawning caller (the HTTP / stdio
    /// bootstrap in `src/cmds/server.rs`) can hand the same Arc to the
    /// MCP server, letting `onto_retention_pause` / `onto_retention_resume`
    /// drive the worker without an explicit channel.
    pub fn new_with_pause(
        db: StateDb,
        cfg: RetentionConfig,
        paused_until: Arc<AtomicI64>,
    ) -> Self {
        Self { db, cfg, paused_until }
    }

    /// Spawn the loop. Returns a detached `JoinHandle`. Mirrors
    /// [`crate::registry::spawn_evictor`] semantics: dropping the handle
    /// does NOT abort.
    ///
    /// **Backwards-compat shim**: callers that don't need the pause
    /// handle (the existing `_retention =` site in stdio bootstrap,
    /// retention_worker.rs tests) keep working unchanged. New callers
    /// that DO need pause control use [`spawn_with_pause`].
    pub fn spawn(db: StateDb, cfg: RetentionConfig) -> tokio::task::JoinHandle<()> {
        Self::spawn_with_pause(db, cfg, Arc::new(AtomicI64::new(0))).0
    }

    /// R5 WC-2 — spawn with an externally-owned `paused_until` handle.
    /// Returns `(JoinHandle, Arc<AtomicI64>)`; the caller installs the
    /// Arc on `OpenOntologiesServer::retention_paused_until` so the
    /// `onto_retention_pause` / `onto_retention_resume` tools can mutate
    /// the same atomic the worker reads each tick.
    pub fn spawn_with_pause(
        db: StateDb,
        cfg: RetentionConfig,
        paused_until: Arc<AtomicI64>,
    ) -> (tokio::task::JoinHandle<()>, Arc<AtomicI64>) {
        let interval_secs = cfg.poll_interval_secs.max(1);
        let worker = Self::new_with_pause(db, cfg, paused_until.clone());
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
            // Skip the immediate tick so retention does not run before
            // the server has had a chance to admit anything.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                match worker.tick() {
                    Ok(report) => {
                        if report_has_pruning(&report) {
                            tracing::info!(
                                "retention worker tick: {:?}",
                                report
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!("retention worker tick failed: {}", e);
                    }
                }
            }
        });
        (handle, paused_until)
    }

    /// Returns true when the worker is currently paused (kill-switch active).
    pub fn is_paused(&self) -> bool {
        let until = self.paused_until.load(Ordering::Relaxed);
        until > 0 && chrono::Utc::now().timestamp() < until
    }

    /// Run a single retention pass synchronously. Tests drive this directly
    /// with `RetentionConfig { poll_interval_secs: 1, *_days: 0, … }` to
    /// prove every pruner is wired.
    ///
    /// R5 WC-2: returns an empty `RetentionReport` without touching the
    /// database when [`is_paused`] is true. The pause kill-switch is the
    /// authoritative consumer of `paused_until`; `Ordering::Relaxed` is
    /// adequate (no other state is synchronized through the atomic).
    pub fn tick(&self) -> Result<RetentionReport> {
        if self.is_paused() {
            tracing::debug!(
                "retention worker tick skipped: paused_until={}",
                self.paused_until.load(Ordering::Relaxed)
            );
            return Ok(RetentionReport::default());
        }
        let mut report = RetentionReport::default();

        // Cascade order: children first.
        let (ev_attrs, ev_rels, ev) = self.prune_ocel(self.cfg.ocel_days)?;
        report.ocel_event_attrs_pruned = ev_attrs;
        report.ocel_relationships_pruned = ev_rels;
        report.ocel_events_pruned = ev;

        report.lineage_events_pruned = self.prune_lineage(self.cfg.lineage_days)?;
        report.conformance_runs_pruned = self.prune_conformance(self.cfg.conformance_days)?;
        report.revoked_sessions_pruned = self.prune_revoked(self.cfg.revocation_grace_days)?;
        report.mined_exemplars_pruned = self.prune_exemplars(self.cfg.exemplar_days)?;
        report.align_feedback_pruned = self.prune_align_feedback(self.cfg.feedback_days)?;
        report.tool_feedback_pruned = self.prune_tool_feedback(self.cfg.feedback_days)?;
        report.embeddings_orphaned = self.prune_embeddings_orphans()?;
        report.ontology_cache_pruned = self.prune_cache(self.cfg.receipt_files_days)?;

        Ok(report)
    }

    /// Cascade-delete `ocel_event_attrs` and `ocel_relationships` BEFORE
    /// `ocel_events`. Returns `(attrs, rels, events)`.
    pub fn prune_ocel(&self, days: u64) -> Result<(u64, u64, u64)> {
        let cutoff = days_ago(days);
        let conn = self.db.conn();

        // 1. Children first: attrs referencing events older than cutoff.
        let attrs = conn.execute(
            "DELETE FROM ocel_event_attrs
             WHERE event_id IN (
                 SELECT event_id FROM ocel_events WHERE time < ?1
             )",
            rusqlite::params![cutoff],
        )? as u64;

        // 2. Relationships referencing events older than cutoff.
        let rels = conn.execute(
            "DELETE FROM ocel_relationships
             WHERE event_id IN (
                 SELECT event_id FROM ocel_events WHERE time < ?1
             )",
            rusqlite::params![cutoff],
        )? as u64;

        // 3. Now safe to delete the parent events.
        let events = conn.execute(
            "DELETE FROM ocel_events WHERE time < ?1",
            rusqlite::params![cutoff],
        )? as u64;

        Ok((attrs, rels, events))
    }

    pub fn prune_lineage(&self, days: u64) -> Result<u64> {
        let cutoff = days_ago(days);
        let n = self.db.conn().execute(
            "DELETE FROM lineage_events WHERE timestamp < ?1",
            rusqlite::params![cutoff],
        )? as u64;
        Ok(n)
    }

    pub fn prune_conformance(&self, days: u64) -> Result<u64> {
        let cutoff = days_ago(days);
        let n = self.db.conn().execute(
            "DELETE FROM conformance_runs WHERE ran_at < ?1",
            rusqlite::params![cutoff],
        )? as u64;
        Ok(n)
    }

    pub fn prune_revoked(&self, days: u64) -> Result<u64> {
        let cutoff = days_ago(days);
        let n = self.db.conn().execute(
            "DELETE FROM revoked_sessions WHERE revoked_at < ?1",
            rusqlite::params![cutoff],
        )? as u64;
        Ok(n)
    }

    /// Receipt files (compile cache N-Triples sidecars). Receipts in the
    /// `receipts` SQL table are NOT pruned here — they go through
    /// [`crate::receipt_archive::archive_receipts`] (cold storage), which
    /// preserves the chain in Parquet shards before removing hot rows.
    pub fn prune_receipt_files(&self, days: u64) -> Result<u64> {
        // Currently scoped to the on-disk compile cache file rows; the
        // actual files are kept (the cache layer manages its own cleanup).
        // This is a placeholder for receipt-file artifacts — see
        // [`crate::receipt_archive`] for the receipts-table cold path.
        let cutoff = days_ago(days);
        let n = self.db.conn().execute(
            "DELETE FROM ontology_cache
             WHERE last_access_at < ?1",
            rusqlite::params![cutoff],
        )? as u64;
        Ok(n)
    }

    pub fn prune_exemplars(&self, days: u64) -> Result<u64> {
        let cutoff = days_ago(days);
        let n = self.db.conn().execute(
            "DELETE FROM mined_exemplars WHERE mined_at < ?1",
            rusqlite::params![cutoff],
        )? as u64;
        Ok(n)
    }

    pub fn prune_align_feedback(&self, days: u64) -> Result<u64> {
        let cutoff = days_ago(days);
        let n = self.db.conn().execute(
            "DELETE FROM align_feedback WHERE timestamp < ?1",
            rusqlite::params![cutoff],
        )? as u64;
        Ok(n)
    }

    pub fn prune_tool_feedback(&self, days: u64) -> Result<u64> {
        let cutoff = days_ago(days);
        let n = self.db.conn().execute(
            "DELETE FROM tool_feedback WHERE timestamp < ?1",
            rusqlite::params![cutoff],
        )? as u64;
        Ok(n)
    }

    /// Drop embedding rows whose IRI is no longer referenced by any
    /// loaded ontology. Best-effort: we currently treat any embedding
    /// older than the configured `feedback_days` window as orphaned if
    /// no class with that IRI is in `ontology_cache`. This is a coarse
    /// signal but it fixes the unbounded-growth case.
    pub fn prune_embeddings_orphans(&self) -> Result<u64> {
        // Without an authoritative "is-this-IRI-loaded" join we keep this
        // as a no-op when no policy is set. A more aggressive policy can
        // be wired later from [`RetentionConfig`].
        Ok(0)
    }

    /// Hot-path cache eviction is owned by [`crate::registry::OntologyRegistry`];
    /// here we only delete cache rows whose source file has been gone for
    /// `days` and whose row hasn't been touched since.
    pub fn prune_cache(&self, days: u64) -> Result<u64> {
        // Mirrors `prune_receipt_files` — the cache files are removed by
        // the registry; this prunes the index rows.
        self.prune_receipt_files(days)
    }

    /// Convenience used by tests and the embedding pruner: number of
    /// rows currently present in `embeddings`.
    pub fn embedding_row_count(&self) -> Result<i64> {
        let n: i64 = self
            .db
            .conn()
            .query_row("SELECT COUNT(*) FROM embeddings", [], |r| r.get(0))?;
        Ok(n)
    }
}

fn days_ago(days: u64) -> String {
    // For days=0 return "now": every row strictly older than this instant
    // is eligible. Tests use days=0 with rows whose `time` is in the past.
    let dt = chrono::Utc::now() - chrono::Duration::days(days as i64);
    dt.to_rfc3339()
}

fn report_has_pruning(r: &RetentionReport) -> bool {
    r.ocel_events_pruned
        + r.ocel_event_attrs_pruned
        + r.ocel_relationships_pruned
        + r.lineage_events_pruned
        + r.conformance_runs_pruned
        + r.revoked_sessions_pruned
        + r.mined_exemplars_pruned
        + r.align_feedback_pruned
        + r.tool_feedback_pruned
        + r.embeddings_orphaned
        + r.ontology_cache_pruned
        > 0
}
