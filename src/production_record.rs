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
    /// e.g. "ontostar-defects-1.0.0". Source of truth: `defects::DEFECTS_TAXONOMY_VERSION`.
    /// Old persisted records (pre-Level-5) deserialize with empty string and
    /// are interpreted as "unversioned-legacy" by external auditors.
    #[serde(default)]
    pub defects_taxonomy_version: String,
    pub gates_passed: Vec<String>,
    pub gates_refused: Vec<DefectClass>,
    /// Receipt chain: previous receipt hash in same session, if any.
    pub prior_receipt: Option<[u8; 32]>,
    /// Ed25519 signature over [`ProductionRecord::canonical_bytes_for_signing`].
    /// `None` for legacy records persisted before real attestation landed —
    /// Cell8 A10 admits these only when `[admission] verify_legacy_receipts = true`.
    #[serde(default, with = "serde_opt_sig")]
    pub signature: Option<[u8; 64]>,
    /// 8-byte BLAKE3-prefix fingerprint of the public key that produced
    /// `signature`. `None` for legacy unsigned records.
    #[serde(default, with = "serde_opt_fpr")]
    pub signing_key_fpr: Option<[u8; 8]>,
}

impl ProductionRecord {
    /// Deterministic canonical serialization. Uses `serde_json::to_vec` against
    /// a `BTreeMap`-backed JSON value so map keys are sorted lexicographically.
    /// All `[u8; 32]` fields are emitted as lowercase hex strings; this gives
    /// us a stable byte representation suitable for BLAKE3 hashing.
    ///
    /// The output INCLUDES `signature` and `signing_key_fpr` so the receipt
    /// hash binds them. To produce the bytes that get signed, use
    /// [`ProductionRecord::canonical_bytes_for_signing`] instead.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let v = serde_json::json!({
            "artifact_hash": hex32(&self.artifact_hash),
            "conformance_run_id": self.conformance_run_id,
            "declared_powl_hash": hex32(&self.declared_powl_hash),
            "defects_taxonomy_version": self.defects_taxonomy_version,
            "gate_config_hash": hex32(&self.gate_config_hash),
            "gates_passed": self.gates_passed,
            "gates_refused": self.gates_refused,
            "ocel_canonical_hash": hex32(&self.ocel_canonical_hash),
            "prior_receipt": self.prior_receipt.as_ref().map(|h| hex32(h)),
            "production_law_version": self.production_law_version,
            "scope_token": self.scope_token,
            "signature": self.signature.as_ref().map(hex_n::<64>),
            "signing_key_fpr": self.signing_key_fpr.as_ref().map(hex_n::<8>),
        });
        serde_json::to_vec(&v).expect("canonical record serializes")
    }

    /// Bytes that the Ed25519 signer commits to. Identical to
    /// [`ProductionRecord::canonical_bytes`] but EXCLUDES the `signature`
    /// and `signing_key_fpr` fields — otherwise the signer would be
    /// signing-over-itself.
    ///
    /// This is the receipt-replay defence: the same `signature` value
    /// pasted onto a record with a different `artifact_hash` produces a
    /// different signing-input, and `verify_strict` returns `Err(_)`.
    pub fn canonical_bytes_for_signing(&self) -> Vec<u8> {
        let v = serde_json::json!({
            "artifact_hash": hex32(&self.artifact_hash),
            "conformance_run_id": self.conformance_run_id,
            "declared_powl_hash": hex32(&self.declared_powl_hash),
            "defects_taxonomy_version": self.defects_taxonomy_version,
            "gate_config_hash": hex32(&self.gate_config_hash),
            "gates_passed": self.gates_passed,
            "gates_refused": self.gates_refused,
            "ocel_canonical_hash": hex32(&self.ocel_canonical_hash),
            "prior_receipt": self.prior_receipt.as_ref().map(|h| hex32(h)),
            "production_law_version": self.production_law_version,
            "scope_token": self.scope_token,
        });
        serde_json::to_vec(&v).expect("canonical record serializes")
    }
}

fn hex_n<const N: usize>(b: &[u8; N]) -> String {
    let mut s = String::with_capacity(N * 2);
    for byte in b {
        s.push_str(&format!("{:02x}", byte));
    }
    s
}

fn parse_hex_n<const N: usize>(s: &str) -> Option<[u8; N]> {
    if s.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for i in 0..N {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

mod serde_opt_sig {
    use super::{hex_n, parse_hex_n};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        v: &Option<[u8; 64]>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match v {
            Some(b) => s.serialize_some(&hex_n::<64>(b)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<[u8; 64]>, D::Error> {
        let opt: Option<String> = Option::deserialize(d)?;
        match opt {
            Some(s) => parse_hex_n::<64>(&s)
                .map(Some)
                .ok_or_else(|| serde::de::Error::custom("signature not 128-hex")),
            None => Ok(None),
        }
    }
}

mod serde_opt_fpr {
    use super::{hex_n, parse_hex_n};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        v: &Option<[u8; 8]>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match v {
            Some(b) => s.serialize_some(&hex_n::<8>(b)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<[u8; 8]>, D::Error> {
        let opt: Option<String> = Option::deserialize(d)?;
        match opt {
            Some(s) => parse_hex_n::<8>(&s)
                .map(Some)
                .ok_or_else(|| serde::de::Error::custom("fpr not 16-hex")),
            None => Ok(None),
        }
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
