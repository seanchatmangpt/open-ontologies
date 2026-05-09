//! `Receipt` — BLAKE3 hash bound to a `ProductionRecord`.
//!
//! Receipts chain across operations within a session: each new admission
//! references the previous receipt. The chain is replayable from the OCEL
//! trace alone (no out-of-band state).

use crate::production_record::{hex32_pub, ProductionRecord};
use crate::state::StateDb;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Stream-3 backstop: a no-op so callers may pass it to `execute_batch` without
/// risking a conflict against Stream 1's authoritative schema (which now lives
/// in `state.rs`). Kept as a public constant for compatibility with earlier
/// branches of Stream 3 that referenced it.
pub const STREAM3_STUB_MIGRATION: &str = "-- stream-3 backstop: schema lives in state.rs\n";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    /// BLAKE3 over the canonical `ProductionRecord` bytes.
    pub bytes: [u8; 32],
    pub record: ProductionRecord,
}

impl Receipt {
    pub fn hex(&self) -> String {
        hex32_pub(&self.bytes)
    }
}

/// Build a `Receipt` by hashing the canonical bytes of the production record.
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
pub fn persist(receipt: &Receipt, db: &StateDb, session_id: &str) -> Result<()> {
    persist_with_tenant(receipt, db, session_id, "default")
}

/// Tenant-aware variant of [`persist`].
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
    Ok(())
}

/// Phase 7 Task C.fix: shared-transaction variant. Performs the receipt INSERT
/// on a caller-supplied transaction WITHOUT committing. Lets the admission
/// gate wrap `persist` + OCEL `emit_event` in a single atomic boundary so a
/// receipt is never durable without its corresponding `admission_granted`
/// event (or vice-versa).
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
        "SELECT COALESCE(MAX(sequence), 0) + 1 FROM receipts WHERE session_id = ?1",
        rusqlite::params![session_id],
        |r| r.get(0),
    )?;
    tx.execute(
        "INSERT INTO receipts (
            receipt_hash, scope_token, session_id,
            artifact_hash, declared_powl_hash, ocel_canonical_hash,
            gate_config_hash, prior_receipt_hash,
            production_law_version, granted_at, sequence, tenant_id
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
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
        ],
    )?;
    Ok(())
}

/// Look up the most recent receipt for a session; used to chain `prior_receipt`.
///
/// Phase 11: backwards-compat shim — defaults to `tenant_id = "default"`.
/// Tenant-aware callers must use [`latest_for_session_in_tenant`].
pub fn latest_for_session(db: &StateDb, session_id: &str) -> Option<[u8; 32]> {
    latest_for_session_in_tenant(db, session_id, "default")
}

/// Tenant-scoped variant of [`latest_for_session`]. Cross-tenant rows are
/// invisible to the chain — a receipt persisted under `tenant_id = "alpha"`
/// will NEVER be returned to a caller asking under `tenant_id = "beta"`.
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
