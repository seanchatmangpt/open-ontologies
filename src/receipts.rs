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
pub fn persist(receipt: &Receipt, db: &StateDb, session_id: &str) -> Result<()> {
    let conn = db.conn();
    let granted_at = chrono::Utc::now().to_rfc3339();
    let prior = receipt.record.prior_receipt.as_ref().map(hex32_pub);
    conn.execute(
        "INSERT OR REPLACE INTO receipts (
            receipt_hash, scope_token, session_id,
            artifact_hash, declared_powl_hash, ocel_canonical_hash,
            gate_config_hash, prior_receipt_hash,
            production_law_version, granted_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
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
        ],
    )?;
    Ok(())
}

/// Look up the most recent receipt for a session; used to chain `prior_receipt`.
pub fn latest_for_session(db: &StateDb, session_id: &str) -> Option<[u8; 32]> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare(
            "SELECT receipt_hash FROM receipts WHERE session_id = ?1 ORDER BY granted_at DESC LIMIT 1",
        )
        .ok()?;
    let mut rows = stmt.query(rusqlite::params![session_id]).ok()?;
    let row = rows.next().ok()??;
    let hex: String = row.get(0).ok()?;
    hex_to_32(&hex)
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
