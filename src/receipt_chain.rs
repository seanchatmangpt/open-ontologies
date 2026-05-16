//! Append-only JSONL receipt chain — supplementary to the SQL `receipts` table.
//!
//! Provides a greppable, replayable chain log alongside the authoritative SQLite
//! record. Pattern adapted from `~/mcpp/crates/mcpp-core/src/chain.rs`.
//!
//! # Design
//!
//! - `.ontostar/chain.jsonl` — one JSON object per line, each representing one
//!   receipt. Appended atomically via `writeln!` (POSIX line-atomic for ≤4 KiB).
//! - `.ontostar/CHAIN_HEAD` — single-line hex of the most recently appended
//!   receipt hash. Updated via `tempfile::persist` for tear-free reads.
//! - A process-global `OnceLock<ChainStore>` is initialized from the env var
//!   `OPEN_ONTOLOGIES_CHAIN_PATH` (directory). When the env var is absent,
//!   `maybe_append` is a no-op — no failure, no overhead.
//!
//! # Authoritativeness
//!
//! The SQL `receipts` table is authoritative. The JSONL chain is additive:
//! - Appends happen AFTER `tx.commit()` succeeds in `receipts::persist_with_tenant`
//! - A JSONL write failure logs a warning but does NOT fail the receipt persist
//! - `verify_chain` checks structural integrity; it does NOT replace SHACL/Cell8 gates

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use ulid::Ulid;

/// One line of the JSONL chain log.
///
/// # Examples
///
/// Construct a genesis link (no prior receipt) and check its fields:
/// ```
/// # use open_ontologies::receipt_chain::ChainLink;
/// let link = ChainLink {
///     receipt_hash: "deadbeef".to_string(),
///     prev_receipt_hash: None,
///     scope_token: "scope-abc".to_string(),
///     ts: "2025-01-01T00:00:00Z".to_string(),
///     session_id: "sess-1".to_string(),
///     tenant_id: "tenant-default".to_string(),
/// };
/// assert_eq!(link.receipt_hash, "deadbeef");
/// assert!(link.prev_receipt_hash.is_none());
/// assert_eq!(link.scope_token, "scope-abc");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainLink {
    /// BLAKE3 hex of this receipt's canonical bytes (receipt's unique ID).
    pub receipt_hash: String,
    /// BLAKE3 hex of the prior receipt in this session, or `None` for genesis.
    pub prev_receipt_hash: Option<String>,
    /// Scope token the receipt belongs to.
    pub scope_token: String,
    /// RFC-3339 timestamp the link was appended.
    pub ts: String,
    /// Session ID — diagnostic, for auditor correlation.
    pub session_id: String,
    /// Tenant ID — diagnostic, for auditor cross-referencing.
    pub tenant_id: String,
}

/// Structural integrity report from [`ChainStore::verify_chain`].
///
/// # Examples
///
/// A clean report has `is_contiguous = true` and no gap:
/// ```
/// # use open_ontologies::receipt_chain::ChainVerifyReport;
/// let report = ChainVerifyReport {
///     links_walked: 42,
///     is_contiguous: true,
///     first_gap_at: None,
/// };
/// assert!(report.is_contiguous);
/// assert!(report.first_gap_at.is_none());
/// assert_eq!(report.links_walked, 42);
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct ChainVerifyReport {
    /// Total links walked (scanned) during verification.
    pub links_walked: usize,
    /// Whether any link's `prev_receipt_hash` referenced a hash not found
    /// earlier in the log. `true` means clean (no gaps found).
    pub is_contiguous: bool,
    /// Hash of the first gap found, if any.
    pub first_gap_at: Option<String>,
}

/// JSONL-backed receipt chain store.
pub struct ChainStore {
    chain_path: PathBuf,
    head_path: PathBuf,
    write_lock: Mutex<()>,
}

static GLOBAL_CHAIN: OnceLock<ChainStore> = OnceLock::new();

/// Initialize the global chain store from `OPEN_ONTOLOGIES_CHAIN_PATH` env var.
///
/// The env var should be a directory path; `chain.jsonl` and `CHAIN_HEAD` are
/// placed inside it. Idempotent — subsequent calls after the first are no-ops.
/// Returns `Ok(true)` if the chain was initialized, `Ok(false)` if the env var
/// is absent (chain disabled), or `Err` if initialization failed.
pub fn init_from_env() -> Result<bool> {
    let dir = match std::env::var("OPEN_ONTOLOGIES_CHAIN_PATH") {
        Ok(d) if !d.trim().is_empty() => PathBuf::from(d.trim()),
        _ => return Ok(false),
    };
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating chain directory {dir:?}"))?;
    let chain_path = dir.join("chain.jsonl");
    let head_path = dir.join("CHAIN_HEAD");
    let store = ChainStore {
        chain_path,
        head_path,
        write_lock: Mutex::new(()),
    };
    // OnceLock::set fails silently if already set — idempotent.
    let _ = GLOBAL_CHAIN.set(store);
    Ok(true)
}

/// Append a receipt to the global chain. No-op if the chain was not initialized.
/// Failures are logged as warnings; never propagated to callers.
///
/// Must be called AFTER the SQL transaction has committed.
pub fn maybe_append(
    receipt_hash: &str,
    prev_receipt_hash: Option<&str>,
    scope_token: &str,
    session_id: &str,
    tenant_id: &str,
) {
    let Some(store) = GLOBAL_CHAIN.get() else {
        return;
    };
    let link = ChainLink {
        receipt_hash: receipt_hash.to_string(),
        prev_receipt_hash: prev_receipt_hash.map(str::to_string),
        scope_token: scope_token.to_string(),
        ts: chrono::Utc::now().to_rfc3339(),
        session_id: session_id.to_string(),
        tenant_id: tenant_id.to_string(),
    };
    if let Err(e) = store.append(&link) {
        tracing::warn!(
            target: "ontostar.receipt_chain",
            receipt_hash,
            error = %e,
            "JSONL chain append failed — SQL receipt is durable, chain audit trail has a gap"
        );
    }
}

impl ChainStore {
    /// Append one link. Serializes concurrent appends via mutex.
    pub fn append(&self, link: &ChainLink) -> Result<()> {
        let line = serde_json::to_string(link).context("serializing chain link")?;
        let _guard = self.write_lock.lock().expect("chain write lock poisoned");
        // Line-atomic append on POSIX (writeln produces ≤4 KiB).
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.chain_path)
            .with_context(|| format!("opening chain log {:?}", self.chain_path))?;
        writeln!(f, "{line}").context("writing chain link")?;
        // Update the HEAD anchor atomically via tempfile rename.
        self.write_head(&link.receipt_hash)?;
        Ok(())
    }

    /// Read the current head hash (most recently appended receipt).
    pub fn current_head(&self) -> Result<Option<String>> {
        match std::fs::read_to_string(&self.head_path) {
            Ok(s) => Ok(Some(s.trim().to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context("reading CHAIN_HEAD"),
        }
    }

    /// Walk the JSONL log and report structural integrity.
    ///
    /// O(N) scan. Checks that every `prev_receipt_hash` references a hash
    /// seen earlier in the file (genesis-ordered). Does not verify BLAKE3
    /// correctness — that is Cell8 A5/A6's job.
    pub fn verify_chain(&self) -> Result<ChainVerifyReport> {
        let f = match File::open(&self.chain_path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ChainVerifyReport {
                    links_walked: 0,
                    is_contiguous: true,
                    first_gap_at: None,
                })
            }
            Err(e) => return Err(e).context("opening chain log for verify"),
        };

        let mut seen = std::collections::HashSet::new();
        let mut links_walked = 0usize;
        for (i, line) in BufReader::new(f).lines().enumerate() {
            let line = line.with_context(|| format!("reading chain log line {i}"))?;
            if line.trim().is_empty() {
                continue;
            }
            let link: ChainLink =
                serde_json::from_str(&line).with_context(|| format!("parsing chain link {i}"))?;
            links_walked += 1;
            if let Some(ref prev) = link.prev_receipt_hash
                && !seen.contains(prev.as_str())
            {
                return Ok(ChainVerifyReport {
                    links_walked,
                    is_contiguous: false,
                    first_gap_at: Some(prev.clone()),
                });
            }
            seen.insert(link.receipt_hash);
        }

        Ok(ChainVerifyReport {
            links_walked,
            is_contiguous: true,
            first_gap_at: None,
        })
    }

    /// Expose chain path for diagnostics.
    pub fn chain_path(&self) -> &Path {
        &self.chain_path
    }

    /// Expose head path for diagnostics.
    pub fn head_path(&self) -> &Path {
        &self.head_path
    }

    fn write_head(&self, hash_hex: &str) -> Result<()> {
        // Atomic write via temp-file + rename (POSIX rename is atomic).
        let ulid = Ulid::new().to_string();
        let tmp_path = self.head_path.with_file_name(format!("CHAIN_HEAD_{ulid}.tmp"));
        std::fs::write(&tmp_path, format!("{hash_hex}\n"))
            .context("writing CHAIN_HEAD temp file")?;
        std::fs::rename(&tmp_path, &self.head_path)
            .context("renaming CHAIN_HEAD temp file into place")?;
        Ok(())
    }
}

/// Public accessor for diagnostics (e.g., `onto_receipts_revoke_batch` reporting chain path).
pub fn global() -> Option<&'static ChainStore> {
    GLOBAL_CHAIN.get()
}
