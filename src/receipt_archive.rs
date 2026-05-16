//! Round 4 WD — receipt cold-storage archival.
//!
//! Older receipts are moved out of the hot `receipts` SQL table into
//! monthly Parquet shards (`receipts-YYYY-MM.parquet`) under `archive_dir`.
//! A sidecar SQLite index (`archive_index.db`) maps `receipt_hash → shard`
//! so [`onto_verify`](crate::verify) can resolve archived hashes in O(1)
//! without scanning every shard.
//!
//! Doctrine: §29 retirement closure. Without this path, the chain walker
//! eventually fails on receipts that were pruned-without-trace, and the
//! external verifier cannot prove a historical artifact was admitted.
//!
//! # Schema
//!
//! Each Parquet shard contains the columns of the live `receipts` table
//! (plus `key_valid_at`). The sidecar index has a single table:
//!
//! ```sql
//! CREATE TABLE archive_index (
//!     receipt_hash TEXT PRIMARY KEY,
//!     shard_file   TEXT NOT NULL,
//!     archived_at  TEXT NOT NULL
//! );
//! ```
//!
//! # Note
//!
//! `archive_receipts` requires a [`StateDb`] backed by a real SQLite file
//! (not an in-memory `:memory:` connection) because it uses WAL mode and
//! multi-statement batches. Doctests below use `tempfile::TempDir` to
//! satisfy the file requirement while remaining hermetic: the temp directory
//! is cleaned up automatically when the guard drops.

use crate::state::StateDb;
use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use std::path::Path;

/// Aggregate stats from one [`archive_receipts`] run.
///
/// # Examples
///
/// A fresh run against an empty (or all-recent) hot table produces all
/// zeroes — the caller can branch on `rows_archived == 0` to skip
/// downstream telemetry.
///
/// ```
/// use open_ontologies::receipt_archive::ArchiveStats;
///
/// let stats = ArchiveStats::default();
/// assert_eq!(stats.rows_archived, 0);
/// assert_eq!(stats.rows_pruned_from_hot, 0);
/// assert_eq!(stats.shards_written, 0);
///
/// // Fields are public so callers can construct test doubles.
/// let partial = ArchiveStats {
///     rows_archived: 5,
///     rows_pruned_from_hot: 5,
///     shards_written: 1,
/// };
/// assert!(partial.rows_archived > 0);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ArchiveStats {
    pub rows_archived: u64,
    pub rows_pruned_from_hot: u64,
    pub shards_written: u64,
}

/// One archived receipt as it appears in the hot table (and the
/// Parquet shard). Mirrors the column set persisted by
/// [`crate::receipts::persist_with_tenant_in_tx`].
#[derive(Debug, Clone)]
pub struct ArchivedReceipt {
    pub receipt_hash: String,
    pub scope_token: String,
    pub session_id: String,
    pub artifact_hash: String,
    pub declared_powl_hash: String,
    pub ocel_canonical_hash: String,
    pub gate_config_hash: String,
    pub prior_receipt_hash: Option<String>,
    pub production_law_version: String,
    pub granted_at: String,
    pub sequence: i64,
    pub tenant_id: String,
    pub key_valid_at: String,
    pub shard_file: String,
}

/// Archive every receipt with `granted_at < now - older_than_days` into
/// monthly Parquet shards under `archive_dir`. Returns the row counts
/// archived and pruned from the hot table.
///
/// Idempotency: re-running with the same `older_than_days` re-archives
/// only rows newly past the threshold. A receipt already in a Parquet
/// shard is `INSERT OR IGNORE`'d into the sidecar index.
///
/// Cascade: rows are written to Parquet AND to the sidecar index BEFORE
/// the hot row is deleted. If shard-write fails, the hot row stays. If
/// hot-delete fails, the shard write is durable (so the chain walker
/// still finds the receipt) and the next run will retry the prune.
///
/// # Examples
///
/// Run against an empty hot table: no rows are archived.
///
/// ```
/// use open_ontologies::receipt_archive::archive_receipts;
/// use open_ontologies::state::StateDb;
/// use tempfile::tempdir;
///
/// let db_dir  = tempdir().unwrap();
/// let arc_dir = tempdir().unwrap();
/// let db = StateDb::open(&db_dir.path().join("state.db")).unwrap();
///
/// let stats = archive_receipts(&db, 365, arc_dir.path()).unwrap();
/// assert_eq!(stats.rows_archived, 0);
/// assert_eq!(stats.shards_written, 0);
/// ```
///
/// Insert one receipt that is 400 days old and archive it.  The hot
/// table must be empty after the call and the sidecar index must contain
/// the hash.
///
/// ```
/// use open_ontologies::receipt_archive::{archive_receipts, lookup_archived};
/// use open_ontologies::state::StateDb;
/// use tempfile::tempdir;
///
/// let db_dir  = tempdir().unwrap();
/// let arc_dir = tempdir().unwrap();
/// let db = StateDb::open(&db_dir.path().join("state.db")).unwrap();
///
/// // Stamp granted_at 400 days in the past so it clears the 365-day gate.
/// let old_ts = (chrono::Utc::now() - chrono::Duration::days(400)).to_rfc3339();
/// let hash   = format!("{:064x}", 0xdeadbeef_u64);
/// db.conn().execute(
///     "INSERT INTO receipts (
///         receipt_hash, scope_token, session_id,
///         artifact_hash, declared_powl_hash, ocel_canonical_hash,
///         gate_config_hash, prior_receipt_hash,
///         production_law_version, granted_at, sequence, tenant_id,
///         key_valid_at
///      ) VALUES (?1,'s','s',?1,?1,?1,?1,NULL,'ontostar-1.0.0',?2,1,'default','')",
///     rusqlite::params![hash, old_ts],
/// ).unwrap();
///
/// let stats = archive_receipts(&db, 365, arc_dir.path()).unwrap();
/// assert_eq!(stats.rows_archived, 1);
/// assert_eq!(stats.rows_pruned_from_hot, 1);
/// assert_eq!(stats.shards_written, 1);
///
/// // Hot table is now empty.
/// let hot: i64 = db.conn()
///     .query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))
///     .unwrap();
/// assert_eq!(hot, 0);
///
/// // Archived hash is recoverable via the sidecar index.
/// let found = lookup_archived(arc_dir.path(), &hash).unwrap();
/// assert!(found.is_some());
/// assert_eq!(found.unwrap().receipt_hash, hash);
/// ```
pub fn archive_receipts(
    db: &StateDb,
    older_than_days: u64,
    archive_dir: &Path,
) -> Result<ArchiveStats> {
    std::fs::create_dir_all(archive_dir)
        .with_context(|| format!("create archive_dir {}", archive_dir.display()))?;

    let cutoff = (chrono::Utc::now() - chrono::Duration::days(older_than_days as i64))
        .to_rfc3339();

    // Read every eligible row in a single query. We do not stream because
    // monthly receipt counts in the hot table are bounded (the worker
    // archives them every cycle).
    let rows = read_eligible_rows(db, &cutoff)?;
    if rows.is_empty() {
        return Ok(ArchiveStats::default());
    }

    // Partition by YYYY-MM of `granted_at` so each shard is a single
    // monthly Parquet file. Within a month order by sequence so the
    // Parquet shard is replayable in admission order.
    let mut by_month: std::collections::BTreeMap<String, Vec<ArchivedReceipt>> =
        std::collections::BTreeMap::new();
    for r in rows {
        let month = month_key_of(&r.granted_at);
        by_month.entry(month).or_default().push(r);
    }

    let mut stats = ArchiveStats::default();
    let index_path = archive_dir.join("archive_index.db");
    let index = open_index(&index_path)?;

    for (month, mut bucket) in by_month {
        bucket.sort_by(|a, b| {
            a.granted_at
                .cmp(&b.granted_at)
                .then(a.sequence.cmp(&b.sequence))
        });
        let shard_name = format!("receipts-{month}.parquet");
        let shard_path = archive_dir.join(&shard_name);
        write_parquet_shard(&shard_path, &bucket)?;
        stats.shards_written += 1;

        for r in &bucket {
            index.execute(
                "INSERT OR IGNORE INTO archive_index
                    (receipt_hash, shard_file, archived_at)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    r.receipt_hash,
                    shard_name,
                    chrono::Utc::now().to_rfc3339()
                ],
            )?;
            stats.rows_archived += 1;
        }
    }

    // Now prune the hot table. Each row that we successfully indexed
    // above is safe to delete.
    let pruned = {
        let conn = db.conn();
        conn.execute(
            "DELETE FROM receipts WHERE granted_at < ?1",
            rusqlite::params![cutoff],
        )? as u64
    };
    stats.rows_pruned_from_hot = pruned;
    Ok(stats)
}

/// Look up an archived receipt by hash. Returns `Ok(None)` when the
/// hash is not in the sidecar index (caller should treat this as
/// "receipt was never admitted, OR has not yet been archived").
///
/// # Examples
///
/// When no `archive_index.db` exists in `archive_dir`, the function
/// returns `Ok(None)` immediately — no error.
///
/// ```
/// use open_ontologies::receipt_archive::lookup_archived;
/// use tempfile::tempdir;
///
/// let arc_dir = tempdir().unwrap();
/// // Directory exists but contains no archive_index.db yet.
/// let result = lookup_archived(arc_dir.path(), "nonexistent-hash").unwrap();
/// assert!(result.is_none());
/// ```
///
/// After a receipt has been archived by [`archive_receipts`], the same
/// hash is resolved in O(1) via the sidecar index.
///
/// ```
/// use open_ontologies::receipt_archive::{archive_receipts, lookup_archived};
/// use open_ontologies::state::StateDb;
/// use tempfile::tempdir;
///
/// let db_dir  = tempdir().unwrap();
/// let arc_dir = tempdir().unwrap();
/// let db = StateDb::open(&db_dir.path().join("state.db")).unwrap();
///
/// let old_ts = (chrono::Utc::now() - chrono::Duration::days(400)).to_rfc3339();
/// let hash   = format!("{:064x}", 0xcafe_u64);
/// db.conn().execute(
///     "INSERT INTO receipts (
///         receipt_hash, scope_token, session_id,
///         artifact_hash, declared_powl_hash, ocel_canonical_hash,
///         gate_config_hash, prior_receipt_hash,
///         production_law_version, granted_at, sequence, tenant_id,
///         key_valid_at
///      ) VALUES (?1,'s','s',?1,?1,?1,?1,NULL,'ontostar-1.0.0',?2,1,'default','')",
///     rusqlite::params![hash, old_ts],
/// ).unwrap();
/// archive_receipts(&db, 365, arc_dir.path()).unwrap();
///
/// let archived = lookup_archived(arc_dir.path(), &hash).unwrap().unwrap();
/// assert_eq!(archived.receipt_hash, hash);
/// assert_eq!(archived.tenant_id, "default");
/// assert_eq!(archived.sequence, 1);
/// ```
pub fn lookup_archived(
    archive_dir: &Path,
    receipt_hash: &str,
) -> Result<Option<ArchivedReceipt>> {
    let index_path = archive_dir.join("archive_index.db");
    if !index_path.exists() {
        return Ok(None);
    }
    let conn = Connection::open(&index_path)
        .with_context(|| format!("open archive index {}", index_path.display()))?;
    // The schema may not exist on first lookup; create idempotently.
    conn.execute_batch(ARCHIVE_INDEX_SCHEMA)?;
    let row: Option<(String, String)> = conn
        .query_row(
            "SELECT shard_file, archived_at FROM archive_index WHERE receipt_hash = ?1",
            rusqlite::params![receipt_hash],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .ok();
    let Some((shard_file, _archived_at)) = row else {
        return Ok(None);
    };
    let shard_path = archive_dir.join(&shard_file);
    let receipts = read_parquet_shard(&shard_path)?;
    let found = receipts.into_iter().find(|r| r.receipt_hash == receipt_hash);
    if let Some(mut r) = found {
        r.shard_file = shard_file;
        Ok(Some(r))
    } else {
        // Index says it's there, shard says no — surface as None and
        // let the caller decide whether to refresh the index. We do not
        // bail because cold storage is read-only at lookup time.
        Ok(None)
    }
}

const ARCHIVE_INDEX_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS archive_index (
    receipt_hash TEXT PRIMARY KEY,
    shard_file   TEXT NOT NULL,
    archived_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_archive_index_shard
    ON archive_index(shard_file);
";

fn open_index(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)
        .with_context(|| format!("open archive index {}", path.display()))?;
    conn.execute_batch(ARCHIVE_INDEX_SCHEMA)?;
    Ok(conn)
}

fn read_eligible_rows(db: &StateDb, cutoff_rfc3339: &str) -> Result<Vec<ArchivedReceipt>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT receipt_hash, scope_token, session_id, artifact_hash,
                declared_powl_hash, ocel_canonical_hash, gate_config_hash,
                prior_receipt_hash, production_law_version, granted_at,
                sequence, tenant_id, key_valid_at
         FROM receipts WHERE granted_at < ?1
         ORDER BY granted_at ASC, sequence ASC",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![cutoff_rfc3339], |row| {
            Ok(ArchivedReceipt {
                receipt_hash: row.get(0)?,
                scope_token: row.get(1)?,
                session_id: row.get(2)?,
                artifact_hash: row.get(3)?,
                declared_powl_hash: row.get(4)?,
                ocel_canonical_hash: row.get(5)?,
                gate_config_hash: row.get(6)?,
                prior_receipt_hash: row.get(7)?,
                production_law_version: row.get(8)?,
                granted_at: row.get(9)?,
                sequence: row.get(10)?,
                tenant_id: row.get(11)?,
                key_valid_at: row.get(12)?,
                shard_file: String::new(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn month_key_of(rfc3339: &str) -> String {
    // Best-effort: take the first 7 chars (YYYY-MM). RFC-3339 always
    // begins with `YYYY-MM-DD…`, so this is byte-safe; if the timestamp
    // is malformed we fall back to "unknown" so the row is not lost.
    if rfc3339.len() >= 7 {
        let head = &rfc3339[..7];
        if head
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-')
        {
            return head.to_string();
        }
    }
    "unknown".to_string()
}

// ─── Parquet I/O ──────────────────────────────────────────────────────────
//
// Each shard is a single Arrow RecordBatch serialized as Parquet with
// SNAPPY compression. We use `parquet`'s in-memory `arrow::ArrowWriter`
// because the row counts per month are small (thousands, not millions).

use arrow::array::{Array, ArrayRef, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder};
use std::fs::File;
use std::sync::Arc;

fn shard_schema() -> Schema {
    Schema::new(vec![
        Field::new("receipt_hash", DataType::Utf8, false),
        Field::new("scope_token", DataType::Utf8, false),
        Field::new("session_id", DataType::Utf8, false),
        Field::new("artifact_hash", DataType::Utf8, false),
        Field::new("declared_powl_hash", DataType::Utf8, false),
        Field::new("ocel_canonical_hash", DataType::Utf8, false),
        Field::new("gate_config_hash", DataType::Utf8, false),
        Field::new("prior_receipt_hash", DataType::Utf8, true),
        Field::new("production_law_version", DataType::Utf8, false),
        Field::new("granted_at", DataType::Utf8, false),
        Field::new("sequence", DataType::Int64, false),
        Field::new("tenant_id", DataType::Utf8, false),
        Field::new("key_valid_at", DataType::Utf8, false),
    ])
}

fn write_parquet_shard(path: &Path, rows: &[ArchivedReceipt]) -> Result<()> {
    let schema = Arc::new(shard_schema());

    let receipt_hash: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.receipt_hash.as_str()),
    ));
    let scope_token: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.scope_token.as_str()),
    ));
    let session_id: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.session_id.as_str()),
    ));
    let artifact_hash: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.artifact_hash.as_str()),
    ));
    let declared_powl_hash: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.declared_powl_hash.as_str()),
    ));
    let ocel_canonical_hash: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.ocel_canonical_hash.as_str()),
    ));
    let gate_config_hash: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.gate_config_hash.as_str()),
    ));
    let prior: ArrayRef = Arc::new(StringArray::from_iter(
        rows.iter().map(|r| r.prior_receipt_hash.clone()),
    ));
    let production_law_version: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.production_law_version.as_str()),
    ));
    let granted_at: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.granted_at.as_str()),
    ));
    let sequence: ArrayRef = Arc::new(Int64Array::from_iter_values(
        rows.iter().map(|r| r.sequence),
    ));
    let tenant_id: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.tenant_id.as_str()),
    ));
    let key_valid_at: ArrayRef = Arc::new(StringArray::from_iter_values(
        rows.iter().map(|r| r.key_valid_at.as_str()),
    ));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            receipt_hash,
            scope_token,
            session_id,
            artifact_hash,
            declared_powl_hash,
            ocel_canonical_hash,
            gate_config_hash,
            prior,
            production_law_version,
            granted_at,
            sequence,
            tenant_id,
            key_valid_at,
        ],
    )
    .map_err(|e| anyhow!("build RecordBatch: {e}"))?;

    let file = File::create(path)
        .with_context(|| format!("create shard {}", path.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, None)
        .map_err(|e| anyhow!("open ArrowWriter: {e}"))?;
    writer
        .write(&batch)
        .map_err(|e| anyhow!("write batch: {e}"))?;
    writer.close().map_err(|e| anyhow!("close writer: {e}"))?;
    Ok(())
}

fn read_parquet_shard(path: &Path) -> Result<Vec<ArchivedReceipt>> {
    let file = File::open(path)
        .with_context(|| format!("open shard {}", path.display()))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| anyhow!("open ParquetReader: {e}"))?;
    let reader = builder
        .build()
        .map_err(|e| anyhow!("build batch reader: {e}"))?;
    let mut out: Vec<ArchivedReceipt> = Vec::new();
    for batch in reader {
        let batch = batch.map_err(|e| anyhow!("read batch: {e}"))?;
        let s = |i: usize| -> &StringArray {
            batch
                .column(i)
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("string column")
        };
        let n = |i: usize| -> &Int64Array {
            batch
                .column(i)
                .as_any()
                .downcast_ref::<Int64Array>()
                .expect("int64 column")
        };
        for i in 0..batch.num_rows() {
            let prior_idx = 7;
            let prior_col = s(prior_idx);
            let prior_value: Option<String> = if prior_col.is_null(i) {
                None
            } else {
                Some(prior_col.value(i).to_string())
            };
            out.push(ArchivedReceipt {
                receipt_hash: s(0).value(i).to_string(),
                scope_token: s(1).value(i).to_string(),
                session_id: s(2).value(i).to_string(),
                artifact_hash: s(3).value(i).to_string(),
                declared_powl_hash: s(4).value(i).to_string(),
                ocel_canonical_hash: s(5).value(i).to_string(),
                gate_config_hash: s(6).value(i).to_string(),
                prior_receipt_hash: prior_value,
                production_law_version: s(8).value(i).to_string(),
                granted_at: s(9).value(i).to_string(),
                sequence: n(10).value(i),
                tenant_id: s(11).value(i).to_string(),
                key_valid_at: s(12).value(i).to_string(),
                shard_file: String::new(),
            });
        }
    }
    Ok(out)
}
