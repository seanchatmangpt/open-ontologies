//! `Receipt` — BLAKE3 hash bound to a `ProductionRecord`.
//!
//! Receipts chain across operations within a session: each new admission
//! references the previous receipt. The chain is replayable from the OCEL
//! trace alone (no out-of-band state).

use crate::production_record::{hex32_pub, ProductionRecord};
use crate::receipt_chain;
use crate::state::StateDb;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Stream-3 backstop: a no-op so callers may pass it to `execute_batch` without
/// risking a conflict against Stream 1's authoritative schema (which now lives
/// in `state.rs`). Kept as a public constant for compatibility with earlier
/// branches of Stream 3 that referenced it.
///
/// # Examples
///
/// ```
/// use open_ontologies::receipts::STREAM3_STUB_MIGRATION;
///
/// // The constant is a valid SQL comment — starts with `--`.
/// assert!(STREAM3_STUB_MIGRATION.starts_with("--"));
/// // It is non-empty and safe to pass to `execute_batch`.
/// assert!(!STREAM3_STUB_MIGRATION.trim().is_empty());
/// ```
pub const STREAM3_STUB_MIGRATION: &str = "-- stream-3 backstop: schema lives in state.rs\n";

/// A sealed proof of a manufacturing step.
///
/// Constructed by [`build`]. Carries the 32-byte BLAKE3 hash of the
/// [`ProductionRecord`] and the record itself. Two receipts built from
/// identical records yield identical `bytes`.
///
/// # Examples
///
/// ```
/// use open_ontologies::receipts::build;
/// use open_ontologies::production_record::ProductionRecord;
///
/// let record = ProductionRecord {
///     artifact_hash:            [1u8; 32],
///     scope_token:              "order-to-cash".into(),
///     declared_powl_hash:       [0u8; 32],
///     ocel_canonical_hash:      [0u8; 32],
///     conformance_run_id:       "run-struct".into(),
///     gate_config_hash:         [0u8; 32],
///     production_law_version:   "ontostar-1.0.0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed:             vec!["A1_WorkflowDeclared".into()],
///     gates_refused:            vec![],
///     prior_receipt:            None,
///     signature:                None,
///     signing_key_fpr:          None,
/// };
/// let receipt = build(record);
///
/// // Field access: bytes is the raw 32-byte BLAKE3 digest.
/// assert_eq!(receipt.bytes.len(), 32);
/// // record field carries back the original production record.
/// assert_eq!(receipt.record.scope_token, "order-to-cash");
/// assert_eq!(receipt.record.gates_passed, vec!["A1_WorkflowDeclared"]);
/// // prior_receipt is None for a seed receipt (first in chain).
/// assert!(receipt.record.prior_receipt.is_none());
/// ```
///
/// Chain link construction — seed → child:
///
/// ```
/// use open_ontologies::receipts::{build, is_valid_hex_hash};
/// use open_ontologies::production_record::ProductionRecord;
///
/// fn record(scope: &str, prior: Option<[u8; 32]>) -> ProductionRecord {
///     ProductionRecord {
///         artifact_hash:            [0u8; 32],
///         scope_token:              scope.into(),
///         declared_powl_hash:       [0u8; 32],
///         ocel_canonical_hash:      [0u8; 32],
///         conformance_run_id:       "run-link".into(),
///         gate_config_hash:         [0u8; 32],
///         production_law_version:   "ontostar-1.0.0".into(),
///         defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///         gates_passed:             vec![],
///         gates_refused:            vec![],
///         prior_receipt:            prior,
///         signature:                None,
///         signing_key_fpr:          None,
///     }
/// }
///
/// // Seed: first link has no parent.
/// let seed = build(record("seed", None));
/// assert!(seed.record.prior_receipt.is_none());
/// assert!(is_valid_hex_hash(&seed.hex()));
///
/// // Child: references seed's bytes as its parent_hash.
/// let child = build(record("child", Some(seed.bytes)));
/// assert_eq!(child.record.prior_receipt, Some(seed.bytes));
/// assert_ne!(child.bytes, seed.bytes);
/// assert!(is_valid_hex_hash(&child.hex()));
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    /// BLAKE3 over the canonical `ProductionRecord` bytes.
    pub bytes: [u8; 32],
    pub record: ProductionRecord,
}

impl Receipt {
    /// Hex-encode the 32-byte BLAKE3 receipt hash as a 64-character lowercase string.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::receipts::build;
    /// use open_ontologies::production_record::ProductionRecord;
    ///
    /// let record = ProductionRecord {
    ///     artifact_hash:            [0u8; 32],
    ///     scope_token:              "test-scope".into(),
    ///     declared_powl_hash:       [0u8; 32],
    ///     ocel_canonical_hash:      [0u8; 32],
    ///     conformance_run_id:       "run-1".into(),
    ///     gate_config_hash:         [0u8; 32],
    ///     production_law_version:   "ontostar-1.0.0".into(),
    ///     defects_taxonomy_version: "ontostar-defects-1.0.0".into(),
    ///     gates_passed:             vec![],
    ///     gates_refused:            vec![],
    ///     prior_receipt:            None,
    ///     signature:                None,
    ///     signing_key_fpr:          None,
    /// };
    /// let receipt = build(record);
    /// let hex = receipt.hex();
    ///
    /// // A BLAKE3 output is always 32 bytes → 64 lowercase hex characters.
    /// assert_eq!(hex.len(), 64);
    /// assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    /// // Identical call returns same string (deterministic).
    /// assert_eq!(hex, receipt.hex());
    /// ```
    pub fn hex(&self) -> String {
        hex32_pub(&self.bytes)
    }
}

/// Build a `Receipt` by hashing the canonical bytes of the production record.
///
/// The receipt hash is a BLAKE3 digest of the deterministic JSON serialisation
/// produced by [`ProductionRecord::canonical_bytes`]. Two `ProductionRecord`
/// values that differ in any field will produce distinct receipt hashes.
///
/// # Examples
///
/// ```
/// use open_ontologies::receipts::build;
/// use open_ontologies::production_record::ProductionRecord;
///
/// fn zero_record(scope: &str) -> ProductionRecord {
///     ProductionRecord {
///         artifact_hash:            [0u8; 32],
///         scope_token:              scope.into(),
///         declared_powl_hash:       [0u8; 32],
///         ocel_canonical_hash:      [0u8; 32],
///         conformance_run_id:       "run-1".into(),
///         gate_config_hash:         [0u8; 32],
///         production_law_version:   "ontostar-1.0.0".into(),
///         defects_taxonomy_version: "ontostar-defects-1.0.0".into(),
///         gates_passed:             vec![],
///         gates_refused:            vec![],
///         prior_receipt:            None,
///         signature:                None,
///         signing_key_fpr:          None,
///     }
/// }
///
/// let r1 = build(zero_record("scope-a"));
/// let r2 = build(zero_record("scope-b"));
///
/// // Each receipt carries a 32-byte hash.
/// assert_eq!(r1.bytes.len(), 32);
/// // Different scope_token → different receipt hash.
/// assert_ne!(r1.bytes, r2.bytes);
/// // hex() is the lowercase hex rendering of those 32 bytes.
/// assert_eq!(r1.hex().len(), 64);
/// ```
///
/// ## Linked chain — parent→child relationship
///
/// ```
/// use open_ontologies::receipts::{build, is_valid_hex_hash};
/// use open_ontologies::production_record::ProductionRecord;
///
/// fn make_record(scope: &str, prior: Option<[u8; 32]>) -> ProductionRecord {
///     ProductionRecord {
///         artifact_hash:            [0u8; 32],
///         scope_token:              scope.into(),
///         declared_powl_hash:       [0u8; 32],
///         ocel_canonical_hash:      [0u8; 32],
///         conformance_run_id:       "run-chain".into(),
///         gate_config_hash:         [0u8; 32],
///         production_law_version:   "ontostar-1.0.0".into(),
///         defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///         gates_passed:             vec![],
///         gates_refused:            vec![],
///         prior_receipt:            prior,
///         signature:                None,
///         signing_key_fpr:          None,
///     }
/// }
///
/// let seed = build(make_record("seed", None));
/// assert!(seed.record.prior_receipt.is_none());
/// assert!(is_valid_hex_hash(&seed.hex()));
///
/// let link1 = build(make_record("link-1", Some(seed.bytes)));
/// assert_eq!(link1.record.prior_receipt, Some(seed.bytes));
/// assert_ne!(link1.bytes, seed.bytes);
/// assert!(is_valid_hex_hash(&link1.hex()));
///
/// let link2 = build(make_record("link-2", Some(link1.bytes)));
/// assert_eq!(link2.record.prior_receipt, Some(link1.bytes));
/// assert_ne!(link2.bytes, link1.bytes);
/// assert_ne!(link2.bytes, seed.bytes);
///
/// let hashes = [seed.hex(), link1.hex(), link2.hex()];
/// assert!(hashes.iter().all(|h| is_valid_hex_hash(h)));
/// let unique: std::collections::HashSet<_> = hashes.iter().collect();
/// assert_eq!(unique.len(), 3, "chain links must have distinct hashes");
/// ```
pub fn build(record: ProductionRecord) -> Receipt {
    let bytes_in = record.canonical_bytes();
    let h = blake3::hash(&bytes_in);
    Receipt {
        bytes: *h.as_bytes(),
        record,
    }
}

/// Persist a receipt to the `receipts` SQL table. The receipts table is
/// created by Stream 1's migration; until that lands we fall back to the
/// stub migration in `STREAM3_STUB_MIGRATION`.
///
/// Phase 11: defaults `tenant_id = "default"` for backwards compat. Use
/// [`persist_with_tenant`] to associate the receipt with a non-default tenant.
///
/// # Examples
///
/// ```
/// use open_ontologies::receipts::{build, persist, latest_for_session};
/// use open_ontologies::production_record::ProductionRecord;
/// use open_ontologies::state::StateDb;
/// use std::path::Path;
///
/// let db = StateDb::open(Path::new(":memory:")).unwrap();
///
/// let record = ProductionRecord {
///     artifact_hash:            [1u8; 32],
///     scope_token:              "order-to-cash".into(),
///     declared_powl_hash:       [2u8; 32],
///     ocel_canonical_hash:      [3u8; 32],
///     conformance_run_id:       "run-42".into(),
///     gate_config_hash:         [4u8; 32],
///     production_law_version:   "seed-v0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed:             vec!["A1_WorkflowDeclared".into()],
///     gates_refused:            vec![],
///     prior_receipt:            None,
///     signature:                None,
///     signing_key_fpr:          None,
/// };
/// let receipt = build(record);
/// let expected_hex = receipt.hex();
///
/// persist(&receipt, &db, "session-1").unwrap();
///
/// // After persisting, the receipt can be retrieved as the chain tip.
/// let tip = latest_for_session(&db, "session-1").unwrap();
/// assert_eq!(tip.len(), 32);
/// // The bytes retrieved from the DB round-trip through hex encoding.
/// let tip_hex: String = tip.iter().map(|b| format!("{:02x}", b)).collect();
/// assert_eq!(tip_hex, expected_hex);
/// ```
pub fn persist(receipt: &Receipt, db: &StateDb, session_id: &str) -> Result<()> {
    persist_with_tenant(receipt, db, session_id, "default")
}

/// Tenant-aware variant of [`persist`].
///
/// # Examples
///
/// ```
/// use open_ontologies::receipts::{build, persist_with_tenant, latest_for_session_in_tenant};
/// use open_ontologies::production_record::ProductionRecord;
/// use open_ontologies::state::StateDb;
/// use std::path::Path;
///
/// let db = StateDb::open(Path::new(":memory:")).unwrap();
///
/// let record = ProductionRecord {
///     artifact_hash:            [5u8; 32],
///     scope_token:              "p2p".into(),
///     declared_powl_hash:       [0u8; 32],
///     ocel_canonical_hash:      [0u8; 32],
///     conformance_run_id:       "run-99".into(),
///     gate_config_hash:         [0u8; 32],
///     production_law_version:   "seed-v0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed:             vec![],
///     gates_refused:            vec![],
///     prior_receipt:            None,
///     signature:                None,
///     signing_key_fpr:          None,
/// };
/// let receipt = build(record);
///
/// persist_with_tenant(&receipt, &db, "session-x", "tenant-alpha").unwrap();
///
/// // Visible under the correct tenant.
/// assert!(latest_for_session_in_tenant(&db, "session-x", "tenant-alpha").is_some());
/// // Invisible to a different tenant — cross-tenant isolation.
/// assert!(latest_for_session_in_tenant(&db, "session-x", "tenant-beta").is_none());
/// ```
pub fn persist_with_tenant(
    receipt: &Receipt,
    db: &StateDb,
    session_id: &str,
    tenant_id: &str,
) -> Result<()> {
    let mut conn = db.conn();
    let tx = conn.transaction()?;
    persist_with_tenant_in_tx(&tx, receipt, session_id, tenant_id)?;
    tx.commit()?;
    // After SQL commit: append to the supplementary JSONL chain (best-effort).
    let receipt_hash = receipt.hex();
    let prior = receipt.record.prior_receipt.as_ref().map(hex32_pub);
    receipt_chain::maybe_append(
        &receipt_hash,
        prior.as_deref(),
        &receipt.record.scope_token,
        session_id,
        tenant_id,
    );
    Ok(())
}

/// Phase 7 Task C.fix: shared-transaction variant. Performs the receipt INSERT
/// on a caller-supplied transaction WITHOUT committing. Lets the admission
/// gate wrap `persist` + OCEL `emit_event` in a single atomic boundary so a
/// receipt is never durable without its corresponding `admission_granted`
/// event (or vice-versa).
///
/// # Examples
///
/// Two receipts written on the same transaction share an atomic commit
/// boundary. Both become visible after `commit()`, or neither does on rollback.
///
/// ```
/// use open_ontologies::receipts::{build, persist_with_tenant_in_tx, latest_for_session_in_tenant};
/// use open_ontologies::production_record::ProductionRecord;
/// use open_ontologies::state::StateDb;
/// use std::path::Path;
///
/// let db  = StateDb::open(Path::new(":memory:")).unwrap();
///
/// let make_record = |scope: &str| ProductionRecord {
///     artifact_hash:            [0u8; 32],
///     scope_token:              scope.into(),
///     declared_powl_hash:       [0u8; 32],
///     ocel_canonical_hash:      [0u8; 32],
///     conformance_run_id:       "run-tx".into(),
///     gate_config_hash:         [0u8; 32],
///     production_law_version:   "seed-v0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed:             vec![],
///     gates_refused:            vec![],
///     prior_receipt:            None,
///     signature:                None,
///     signing_key_fpr:          None,
/// };
///
/// let r1 = build(make_record("p2p"));
/// let r2 = build(make_record("o2c"));
///
/// {
///     let mut conn = db.conn();
///     let tx = conn.transaction().unwrap();
///     persist_with_tenant_in_tx(&tx, &r1, "sess-tx", "alpha").unwrap();
///     persist_with_tenant_in_tx(&tx, &r2, "sess-tx", "alpha").unwrap();
///     tx.commit().unwrap();
/// }
///
/// // Both receipts are now visible; the chain tip is the last one inserted.
/// assert!(latest_for_session_in_tenant(&db, "sess-tx", "alpha").is_some());
/// // Cross-tenant isolation: beta sees nothing.
/// assert!(latest_for_session_in_tenant(&db, "sess-tx", "beta").is_none());
/// ```
pub fn persist_with_tenant_in_tx(
    tx: &rusqlite::Transaction<'_>,
    receipt: &Receipt,
    session_id: &str,
    tenant_id: &str,
) -> Result<()> {
    let granted_at = chrono::Utc::now().to_rfc3339();
    let prior = receipt.record.prior_receipt.as_ref().map(hex32_pub);
    // Task C: per-session monotonic sequence; concurrent admissions on the
    // same session_id cannot race the (session_id, sequence) unique index
    // because the surrounding transaction serializes the read+write.
    let next_sequence: i64 = tx.query_row(
        "SELECT COALESCE(MAX(sequence), 0) + 1 FROM receipts WHERE session_id = ?1 AND tenant_id = ?2",
        rusqlite::params![session_id, tenant_id],
        |r| r.get(0),
    )?;

    // Round 4 WD — populate `key_valid_at` from the signing fingerprint's
    // `trusted_keys_history.added_at`. When the receipt is unsigned, or
    // when no history row exists for the fingerprint (legacy databases),
    // we leave it as the empty-string default. The Cell8 A10 verifier
    // reads this column at chain-walk time and rejects receipts whose
    // `granted_at` falls outside `[added_at, removed_at)`.
    let key_valid_at: String = match receipt.record.signing_key_fpr.as_ref() {
        Some(fpr) => {
            let fpr_hex = crate::attestation::fingerprint_hex(fpr);
            tx.query_row(
                "SELECT added_at FROM trusted_keys_history WHERE fingerprint = ?1",
                rusqlite::params![fpr_hex],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_default()
        }
        None => String::new(),
    };

    tx.execute(
        "INSERT INTO receipts (
            receipt_hash, scope_token, session_id,
            artifact_hash, declared_powl_hash, ocel_canonical_hash,
            gate_config_hash, prior_receipt_hash,
            production_law_version, granted_at, sequence, tenant_id,
            key_valid_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        rusqlite::params![
            hex32_pub(&receipt.bytes),
            receipt.record.scope_token,
            session_id,
            hex32_pub(&receipt.record.artifact_hash),
            hex32_pub(&receipt.record.declared_powl_hash),
            hex32_pub(&receipt.record.ocel_canonical_hash),
            hex32_pub(&receipt.record.gate_config_hash),
            prior,
            receipt.record.production_law_version,
            granted_at,
            next_sequence,
            tenant_id,
            key_valid_at,
        ],
    )?;

    // R5 WC-1 — §28 HiddenWIP closure. The first non-seed receipt across
    // the whole DB closes the bootstrap window at the DB level. The
    // `INSERT OR IGNORE` makes this idempotent (the table has a single
    // PK row enforced by `CHECK (id = 1)`); subsequent non-seed receipts
    // never overwrite the original `locked_at` / `locked_by`. Retention
    // pruning explicitly excludes `bootstrap_lock` (see state.rs schema
    // comment) so the window cannot be silently re-opened.
    if receipt.record.production_law_version != "seed-v0" {
        tx.execute(
            "INSERT OR IGNORE INTO bootstrap_lock (id, locked_at, locked_by) \
             VALUES (1, ?1, ?2)",
            rusqlite::params![granted_at, tenant_id],
        )?;
    }
    Ok(())
}

/// Look up the most recent receipt for a session; used to chain `prior_receipt`.
///
/// Phase 11: backwards-compat shim — defaults to `tenant_id = "default"`.
/// Tenant-aware callers must use [`latest_for_session_in_tenant`].
///
/// Returns `None` when no receipt has been persisted for the session yet.
///
/// # Examples
///
/// ```
/// use open_ontologies::receipts::{build, persist, latest_for_session};
/// use open_ontologies::production_record::ProductionRecord;
/// use open_ontologies::state::StateDb;
/// use std::path::Path;
///
/// let db = StateDb::open(Path::new(":memory:")).unwrap();
///
/// // Before any receipt is persisted, the session has no chain tip.
/// assert!(latest_for_session(&db, "session-new").is_none());
///
/// let record = ProductionRecord {
///     artifact_hash:            [7u8; 32],
///     scope_token:              "hire-to-retire".into(),
///     declared_powl_hash:       [0u8; 32],
///     ocel_canonical_hash:      [0u8; 32],
///     conformance_run_id:       "run-7".into(),
///     gate_config_hash:         [0u8; 32],
///     production_law_version:   "seed-v0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed:             vec![],
///     gates_refused:            vec![],
///     prior_receipt:            None,
///     signature:                None,
///     signing_key_fpr:          None,
/// };
/// persist(&build(record), &db, "session-new").unwrap();
///
/// // Now the chain tip is present and is exactly 32 bytes.
/// let tip = latest_for_session(&db, "session-new").unwrap();
/// assert_eq!(tip.len(), 32);
/// ```
pub fn latest_for_session(db: &StateDb, session_id: &str) -> Option<[u8; 32]> {
    latest_for_session_in_tenant(db, session_id, "default")
}

/// Tenant-scoped variant of [`latest_for_session`]. Cross-tenant rows are
/// invisible to the chain — a receipt persisted under `tenant_id = "alpha"`
/// will NEVER be returned to a caller asking under `tenant_id = "beta"`.
///
/// # Examples
///
/// ```
/// use open_ontologies::receipts::{build, persist_with_tenant, latest_for_session_in_tenant};
/// use open_ontologies::production_record::ProductionRecord;
/// use open_ontologies::state::StateDb;
/// use std::path::Path;
///
/// let db = StateDb::open(Path::new(":memory:")).unwrap();
///
/// // Empty session: no tip for any tenant.
/// assert!(latest_for_session_in_tenant(&db, "s1", "alpha").is_none());
/// assert!(latest_for_session_in_tenant(&db, "s1", "beta").is_none());
///
/// let record = ProductionRecord {
///     artifact_hash:            [9u8; 32],
///     scope_token:              "procure-to-pay".into(),
///     declared_powl_hash:       [0u8; 32],
///     ocel_canonical_hash:      [0u8; 32],
///     conformance_run_id:       "run-9".into(),
///     gate_config_hash:         [0u8; 32],
///     production_law_version:   "seed-v0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed:             vec![],
///     gates_refused:            vec![],
///     prior_receipt:            None,
///     signature:                None,
///     signing_key_fpr:          None,
/// };
/// persist_with_tenant(&build(record), &db, "s1", "alpha").unwrap();
///
/// // Visible only to tenant "alpha".
/// assert!(latest_for_session_in_tenant(&db, "s1", "alpha").is_some());
/// assert!(latest_for_session_in_tenant(&db, "s1", "beta").is_none());
/// ```
pub fn latest_for_session_in_tenant(
    db: &StateDb,
    session_id: &str,
    tenant_id: &str,
) -> Option<[u8; 32]> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare(
            "SELECT receipt_hash FROM receipts \
             WHERE session_id = ?1 AND tenant_id = ?2 \
             ORDER BY sequence DESC LIMIT 1",
        )
        .ok()?;
    let mut rows = stmt.query(rusqlite::params![session_id, tenant_id]).ok()?;
    let row = rows.next().ok()??;
    let hex: String = row.get(0).ok()?;
    hex_to_32(&hex)
}

/// Render a deterministic header block for embedding in a TTL/Turtle artifact.
///
/// The block is five `# ostar-…:` comment lines. An external verifier strips
/// any line matching `^# ostar-[a-z-]+: .+$` from the file head, BLAKE3-hashes
/// the remainder, and asserts equality with `ostar-artifact-hash`. Receipts
/// commit to the **header-less** body, so the verifier's stripping is sound.
///
/// # Examples
///
/// ```
/// use open_ontologies::receipts::{build, ttl_header};
/// use open_ontologies::production_record::ProductionRecord;
///
/// let record = ProductionRecord {
///     artifact_hash:            [0u8; 32],
///     scope_token:              "test-scope".into(),
///     declared_powl_hash:       [0u8; 32],
///     ocel_canonical_hash:      [0u8; 32],
///     conformance_run_id:       "run-1".into(),
///     gate_config_hash:         [0u8; 32],
///     production_law_version:   "ontostar-1.0.0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed:             vec![],
///     gates_refused:            vec![],
///     prior_receipt:            None,
///     signature:                None,
///     signing_key_fpr:          None,
/// };
/// let receipt = build(record);
/// let header = ttl_header(&receipt);
///
/// // Six mandatory lines, each prefixed with `# ostar-`.
/// let lines: Vec<&str> = header.lines().collect();
/// assert_eq!(lines.len(), 6);
/// assert!(lines.iter().all(|l| l.starts_with("# ostar-")));
///
/// // When there is no prior receipt the sentinel value is "none".
/// assert!(header.contains("# ostar-prior-receipt: none"));
///
/// // The receipt hash embedded in the header matches Receipt::hex().
/// let expected = format!("# ostar-receipt-hash: {}", receipt.hex());
/// assert!(header.contains(&expected));
/// ```
pub fn ttl_header(r: &Receipt) -> String {
    let prior = r
        .record
        .prior_receipt
        .as_ref()
        .map(hex32_pub)
        .unwrap_or_else(|| "none".to_string());
    format!(
        "# ostar-production-law: {}\n\
         # ostar-defects-taxonomy: {}\n\
         # ostar-receipt-hash: {}\n\
         # ostar-artifact-hash: {}\n\
         # ostar-scope-token: {}\n\
         # ostar-prior-receipt: {}\n",
        r.record.production_law_version,
        r.record.defects_taxonomy_version,
        r.hex(),
        hex32_pub(&r.record.artifact_hash),
        r.record.scope_token,
        prior,
    )
}

/// Comment-prefix style for a generated source file. Returns `None` for
/// extensions that do not support inline comments (binaries, JSON without
/// `JSON5`, etc.) — caller skips those.
fn comment_prefix_for(path: &std::path::Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()) {
        Some(ref ext) if ext == "rs" => Some("//"),
        Some(ref ext) if ext == "ts" => Some("//"),
        Some(ref ext) if ext == "tsx" => Some("//"),
        Some(ref ext) if ext == "js" => Some("//"),
        Some(ref ext) if ext == "go" => Some("//"),
        Some(ref ext) if ext == "java" => Some("//"),
        Some(ref ext) if ext == "kt" => Some("//"),
        Some(ref ext) if ext == "swift" => Some("//"),
        Some(ref ext) if ext == "py" => Some("#"),
        Some(ref ext) if ext == "rb" => Some("#"),
        Some(ref ext) if ext == "ex" => Some("#"),
        Some(ref ext) if ext == "exs" => Some("#"),
        Some(ref ext) if ext == "sh" => Some("#"),
        Some(ref ext) if ext == "ttl" => Some("#"),
        Some(ref ext) if ext == "n3" => Some("#"),
        Some(ref ext) if ext == "trig" => Some("#"),
        _ => None,
    }
}

/// Prepend the OntoStar receipt header to a generated text file. Skips files
/// whose extension does not support inline comments (returns `Ok(false)` for
/// those, `Ok(true)` when the file was stamped). Best-effort: I/O errors
/// surface to the caller.
///
/// # Examples
///
/// A `.rs` file is stamped (returns `true`); an extension-less binary blob
/// is skipped (returns `false`).
///
/// ```
/// use open_ontologies::receipts::{build, inject_comment_header};
/// use open_ontologies::production_record::ProductionRecord;
/// use tempfile::NamedTempFile;
/// use std::io::Write;
///
/// let record = ProductionRecord {
///     artifact_hash:            [0u8; 32],
///     scope_token:              "test-inject".into(),
///     declared_powl_hash:       [0u8; 32],
///     ocel_canonical_hash:      [0u8; 32],
///     conformance_run_id:       "run-inject".into(),
///     gate_config_hash:         [0u8; 32],
///     production_law_version:   "ontostar-1.0.0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed:             vec![],
///     gates_refused:            vec![],
///     prior_receipt:            None,
///     signature:                None,
///     signing_key_fpr:          None,
/// };
/// let receipt = build(record);
///
/// // Write a minimal Rust source file.
/// let mut rs_file = NamedTempFile::with_suffix(".rs").unwrap();
/// rs_file.write_all(b"fn main() {}\n").unwrap();
/// let stamped = inject_comment_header(rs_file.path(), &receipt).unwrap();
/// assert!(stamped, "`.rs` files should be stamped");
///
/// // The injected content begins with `// ostar-` comment lines.
/// let contents = std::fs::read_to_string(rs_file.path()).unwrap();
/// assert!(contents.starts_with("// ostar-production-law:"));
/// assert!(contents.contains("fn main() {}"));
///
/// // A file with an unsupported extension is silently skipped.
/// let bin_file = NamedTempFile::with_suffix(".bin").unwrap();
/// let skipped = inject_comment_header(bin_file.path(), &receipt).unwrap();
/// assert!(!skipped, "`.bin` files should be skipped");
/// ```
pub fn inject_comment_header(path: &std::path::Path, r: &Receipt) -> std::io::Result<bool> {
    let Some(prefix) = comment_prefix_for(path) else {
        return Ok(false);
    };
    let body = std::fs::read(path)?;
    let prior = r
        .record
        .prior_receipt
        .as_ref()
        .map(hex32_pub)
        .unwrap_or_else(|| "none".to_string());
    let header = format!(
        "{prefix} ostar-production-law: {}\n\
         {prefix} ostar-defects-taxonomy: {}\n\
         {prefix} ostar-receipt-hash: {}\n\
         {prefix} ostar-artifact-hash: {}\n\
         {prefix} ostar-scope-token: {}\n\
         {prefix} ostar-prior-receipt: {}\n",
        r.record.production_law_version,
        r.record.defects_taxonomy_version,
        r.hex(),
        hex32_pub(&r.record.artifact_hash),
        r.record.scope_token,
        prior,
        prefix = prefix,
    );
    let mut out = Vec::with_capacity(header.len() + body.len());
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(&body);
    std::fs::write(path, &out)?;
    Ok(true)
}

fn hex_to_32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Returns `true` if `s` is a valid lowercase 64-character hex string
/// that could represent a BLAKE3 receipt hash.
///
/// A valid receipt hash is exactly 64 ASCII hexadecimal digits
/// (`0-9`, `a-f`). Uppercase hex digits, strings of wrong length, and
/// strings containing non-hex characters all return `false`.
///
/// Also demonstrates a three-link chain: seed → link-1 → link-2.  Each
/// link's `prior_receipt` field carries the previous receipt's `bytes`,
/// creating a cryptographic chain that can be replayed from the OCEL log.
///
/// # Examples
///
/// ```
/// use open_ontologies::receipts::is_valid_hex_hash;
///
/// // Exactly 64 lowercase hex characters — valid.
/// assert!(is_valid_hex_hash(&"a".repeat(64)));
/// assert!(is_valid_hex_hash(&"0".repeat(64)));
/// assert!(is_valid_hex_hash(&"f".repeat(64)));
///
/// // Wrong length — invalid.
/// assert!(!is_valid_hex_hash("abc123"));
/// assert!(!is_valid_hex_hash(""));
/// assert!(!is_valid_hex_hash(&"a".repeat(63)));
/// assert!(!is_valid_hex_hash(&"a".repeat(65)));
///
/// // Uppercase is rejected — hashes are lowercase canonical.
/// assert!(!is_valid_hex_hash(&"A".repeat(64)));
///
/// // Non-hex character.
/// let bad = "0".repeat(63) + "g";
/// assert!(!is_valid_hex_hash(&bad));
/// ```
///
/// Three-link chain — seed → link-1 → link-2:
///
/// ```
/// use open_ontologies::receipts::{build, is_valid_hex_hash};
/// use open_ontologies::production_record::ProductionRecord;
///
/// fn make_record(scope: &str, prior: Option<[u8; 32]>) -> ProductionRecord {
///     ProductionRecord {
///         artifact_hash:            [0u8; 32],
///         scope_token:              scope.into(),
///         declared_powl_hash:       [0u8; 32],
///         ocel_canonical_hash:      [0u8; 32],
///         conformance_run_id:       "run-chain".into(),
///         gate_config_hash:         [0u8; 32],
///         production_law_version:   "ontostar-1.0.0".into(),
///         defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///         gates_passed:             vec![],
///         gates_refused:            vec![],
///         prior_receipt:            prior,
///         signature:                None,
///         signing_key_fpr:          None,
///     }
/// }
///
/// let seed  = build(make_record("seed",   None));
/// let link1 = build(make_record("link-1", Some(seed.bytes)));
/// let link2 = build(make_record("link-2", Some(link1.bytes)));
///
/// // Each hash in the chain is a valid 64-char hex string.
/// assert!(is_valid_hex_hash(&seed.hex()));
/// assert!(is_valid_hex_hash(&link1.hex()));
/// assert!(is_valid_hex_hash(&link2.hex()));
///
/// // Chain pointers: seed has no prior, link-1 points to seed, link-2 to link-1.
/// assert!(seed.record.prior_receipt.is_none());
/// assert_eq!(link1.record.prior_receipt, Some(seed.bytes));
/// assert_eq!(link2.record.prior_receipt, Some(link1.bytes));
///
/// // All three hashes are distinct.
/// assert_ne!(seed.bytes,  link1.bytes);
/// assert_ne!(link1.bytes, link2.bytes);
/// assert_ne!(seed.bytes,  link2.bytes);
/// ```
pub fn is_valid_hex_hash(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}
