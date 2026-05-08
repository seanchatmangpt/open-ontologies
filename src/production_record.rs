//! `ProductionRecord` — portable, byte-canonical record of a gated mutation.
//!
//! Storage in OCEL events under attributes; canonical bytes are BLAKE3-hashed
//! and chained via `prior_receipt`. Every gated mutation produces one of
//! these, regardless of the operation kind.

use crate::defects::DefectClass;
use serde::{Deserialize, Serialize};

/// Portable record of a single admitted (or refused) manufacturing operation.
///
/// All hash fields are 32-byte BLAKE3 outputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProductionRecord {
    /// BLAKE3 of artifact bytes (turtle / wasm / code).
    pub artifact_hash: [u8; 32],
    pub scope_token: String,
    /// BLAKE3 of canonical POWL string.
    pub declared_powl_hash: [u8; 32],
    /// BLAKE3 of canonical OCEL projection of scope.
    pub ocel_canonical_hash: [u8; 32],
    /// FK into conformance_runs.
    pub conformance_run_id: String,
    /// BLAKE3 of (f_min, p_min, required_stages, taxonomy version).
    pub gate_config_hash: [u8; 32],
    /// e.g. "ontostar-1.0.0".
    pub production_law_version: String,
    pub gates_passed: Vec<String>,
    pub gates_refused: Vec<DefectClass>,
    /// Receipt chain: previous receipt hash in same session, if any.
    pub prior_receipt: Option<[u8; 32]>,
}

impl ProductionRecord {
    /// Deterministic canonical serialization. Uses `serde_json::to_vec` against
    /// a `BTreeMap`-backed JSON value so map keys are sorted lexicographically.
    /// All `[u8; 32]` fields are emitted as lowercase hex strings; this gives
    /// us a stable byte representation suitable for BLAKE3 hashing.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        // Build the canonical JSON value with a BTreeMap-backed map so keys
        // serialize in sorted order. `serde_json::Map` is insertion-ordered
        // when its `preserve_order` feature is on; here we explicitly sort.
        let v = serde_json::json!({
            "artifact_hash": hex32(&self.artifact_hash),
            "conformance_run_id": self.conformance_run_id,
            "declared_powl_hash": hex32(&self.declared_powl_hash),
            "gate_config_hash": hex32(&self.gate_config_hash),
            "gates_passed": self.gates_passed,
            "gates_refused": self.gates_refused,
            "ocel_canonical_hash": hex32(&self.ocel_canonical_hash),
            "prior_receipt": self.prior_receipt.as_ref().map(|h| hex32(h)),
            "production_law_version": self.production_law_version,
            "scope_token": self.scope_token,
        });
        // serde_json::to_vec preserves insertion order; we built the literal
        // above with keys in sorted order, so the output is canonical.
        serde_json::to_vec(&v).expect("canonical record serializes")
    }
}

fn hex32(b: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for byte in b {
        s.push_str(&format!("{:02x}", byte));
    }
    s
}

/// Helper: hex-encode a 32-byte hash.
pub fn hex32_pub(b: &[u8; 32]) -> String {
    hex32(b)
}
