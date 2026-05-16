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
///
/// # Examples
///
/// Construct a minimal record and verify JSON round-trip via serde:
///
/// ```
/// use open_ontologies::production_record::ProductionRecord;
///
/// let rec = ProductionRecord {
///     artifact_hash:            [0xaau8; 32],
///     scope_token:              "order-to-cash".into(),
///     declared_powl_hash:       [0xbbu8; 32],
///     ocel_canonical_hash:      [0xccu8; 32],
///     conformance_run_id:       "run-42".into(),
///     gate_config_hash:         [0xddu8; 32],
///     production_law_version:   "ontostar-1.0.0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed:             vec!["A1".into(), "A3".into()],
///     gates_refused:            vec![],
///     prior_receipt:            None,
///     signature:                None,
///     signing_key_fpr:          None,
/// };
///
/// // JSON round-trip: serialize then deserialize produces an equal struct.
/// let json = serde_json::to_string(&rec).unwrap();
/// let restored: ProductionRecord = serde_json::from_str(&json).unwrap();
/// assert_eq!(rec, restored);
/// assert_eq!(restored.scope_token, "order-to-cash");
/// assert_eq!(restored.gates_passed, vec!["A1", "A3"]);
/// ```
///
/// Chain two records via `prior_receipt`:
///
/// ```
/// use open_ontologies::production_record::ProductionRecord;
/// use open_ontologies::receipts::build;
///
/// let parent = ProductionRecord {
///     artifact_hash:            [1u8; 32],
///     scope_token:              "p2p".into(),
///     declared_powl_hash:       [0u8; 32],
///     ocel_canonical_hash:      [0u8; 32],
///     conformance_run_id:       "run-1".into(),
///     gate_config_hash:         [0u8; 32],
///     production_law_version:   "ontostar-1.0.0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed:             vec!["A1".into()],
///     gates_refused:            vec![],
///     prior_receipt:            None,
///     signature:                None,
///     signing_key_fpr:          None,
/// };
/// let parent_receipt = build(parent);
///
/// // Child record references parent receipt hash as its prior_receipt.
/// let child = ProductionRecord {
///     artifact_hash:            [2u8; 32],
///     scope_token:              "o2c".into(),
///     declared_powl_hash:       [0u8; 32],
///     ocel_canonical_hash:      [0u8; 32],
///     conformance_run_id:       "run-2".into(),
///     gate_config_hash:         [0u8; 32],
///     production_law_version:   "ontostar-1.0.0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed:             vec!["A1".into(), "A2".into()],
///     gates_refused:            vec![],
///     prior_receipt:            Some(parent_receipt.bytes),
///     signature:                None,
///     signing_key_fpr:          None,
/// };
///
/// // The child's prior_receipt matches the parent's hash — chain is linked.
/// assert_eq!(child.prior_receipt, Some(parent_receipt.bytes));
/// // The child hash is different from the parent hash.
/// let child_receipt = build(child);
/// assert_ne!(child_receipt.bytes, parent_receipt.bytes);
/// ```
///
/// `defects_taxonomy_version` defaults to empty string when absent (legacy records):
///
/// ```
/// use open_ontologies::production_record::ProductionRecord;
///
/// // Pre-Level-5 JSON without the defects_taxonomy_version field.
/// let legacy = r#"{
///     "artifact_hash": "0000000000000000000000000000000000000000000000000000000000000000",
///     "scope_token": "legacy",
///     "declared_powl_hash": "0000000000000000000000000000000000000000000000000000000000000000",
///     "ocel_canonical_hash": "0000000000000000000000000000000000000000000000000000000000000000",
///     "conformance_run_id": "run-0",
///     "gate_config_hash": "0000000000000000000000000000000000000000000000000000000000000000",
///     "production_law_version": "ontostar-0.9.0",
///     "gates_passed": [],
///     "gates_refused": [],
///     "prior_receipt": null,
///     "signature": null,
///     "signing_key_fpr": null
/// }"#;
///
/// let rec: ProductionRecord = serde_json::from_str(legacy).unwrap();
/// // Missing field defaults to empty — not an error.
/// assert_eq!(rec.defects_taxonomy_version, "");
/// ```
///
/// Records differing only in `scope_token` are not equal:
///
/// ```
/// use open_ontologies::production_record::ProductionRecord;
///
/// let make = |scope: &str| ProductionRecord {
///     artifact_hash: [0u8; 32],
///     scope_token: scope.into(),
///     declared_powl_hash: [0u8; 32],
///     ocel_canonical_hash: [0u8; 32],
///     conformance_run_id: "run-1".into(),
///     gate_config_hash: [0u8; 32],
///     production_law_version: "ontostar-1.0.0".into(),
///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
///     gates_passed: vec![],
///     gates_refused: vec![],
///     prior_receipt: None,
///     signature: None,
///     signing_key_fpr: None,
/// };
///
/// assert_ne!(make("order-to-cash"), make("procure-to-pay"));
/// assert_eq!(make("o2c"), make("o2c"));
/// ```
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

/// Build a minimal `ProductionRecord` with zero hashes and no optional fields.
///
/// Used in doctests to avoid repeating the full struct literal.
#[cfg(doctest)]
fn minimal_record() -> ProductionRecord {
    ProductionRecord {
        artifact_hash: [0u8; 32],
        scope_token: "scope-1".into(),
        declared_powl_hash: [0u8; 32],
        ocel_canonical_hash: [0u8; 32],
        conformance_run_id: "run-1".into(),
        gate_config_hash: [0u8; 32],
        production_law_version: "ontostar-1.0.0".into(),
        defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
        gates_passed: vec!["A1".into(), "A2".into()],
        gates_refused: vec![],
        prior_receipt: None,
        signature: None,
        signing_key_fpr: None,
    }
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
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::production_record::ProductionRecord;
    ///
    /// let rec = ProductionRecord {
    ///     artifact_hash: [0u8; 32],
    ///     scope_token: "scope-1".into(),
    ///     declared_powl_hash: [0u8; 32],
    ///     ocel_canonical_hash: [0u8; 32],
    ///     conformance_run_id: "run-1".into(),
    ///     gate_config_hash: [0u8; 32],
    ///     production_law_version: "ontostar-1.0.0".into(),
    ///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
    ///     gates_passed: vec!["A1".into(), "A2".into()],
    ///     gates_refused: vec![],
    ///     prior_receipt: None,
    ///     signature: None,
    ///     signing_key_fpr: None,
    /// };
    /// let bytes = rec.canonical_bytes();
    /// assert!(!bytes.is_empty());
    /// // Output is valid JSON.
    /// let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    /// assert_eq!(parsed["scope_token"], "scope-1");
    /// assert_eq!(parsed["production_law_version"], "ontostar-1.0.0");
    /// ```
    ///
    /// Different `artifact_hash` values yield different canonical bytes (content-addressed):
    ///
    /// ```
    /// use open_ontologies::production_record::ProductionRecord;
    ///
    /// let make = |h: u8| ProductionRecord {
    ///     artifact_hash: [h; 32],
    ///     scope_token: "s".into(),
    ///     declared_powl_hash: [0u8; 32],
    ///     ocel_canonical_hash: [0u8; 32],
    ///     conformance_run_id: "r".into(),
    ///     gate_config_hash: [0u8; 32],
    ///     production_law_version: "ontostar-1.0.0".into(),
    ///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
    ///     gates_passed: vec![],
    ///     gates_refused: vec![],
    ///     prior_receipt: None,
    ///     signature: None,
    ///     signing_key_fpr: None,
    /// };
    ///
    /// assert_ne!(make(0x00).canonical_bytes(), make(0xff).canonical_bytes());
    /// // Same input → identical bytes (deterministic).
    /// assert_eq!(make(0xab).canonical_bytes(), make(0xab).canonical_bytes());
    /// ```
    ///
    /// The `gates_passed` field appears as an ordered JSON array:
    ///
    /// ```
    /// use open_ontologies::production_record::ProductionRecord;
    ///
    /// let rec = ProductionRecord {
    ///     artifact_hash: [0u8; 32],
    ///     scope_token: "s".into(),
    ///     declared_powl_hash: [0u8; 32],
    ///     ocel_canonical_hash: [0u8; 32],
    ///     conformance_run_id: "r".into(),
    ///     gate_config_hash: [0u8; 32],
    ///     production_law_version: "ontostar-1.0.0".into(),
    ///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
    ///     gates_passed: vec!["A1".into(), "A2".into(), "A3".into()],
    ///     gates_refused: vec![],
    ///     prior_receipt: None,
    ///     signature: None,
    ///     signing_key_fpr: None,
    /// };
    /// let parsed: serde_json::Value = serde_json::from_slice(&rec.canonical_bytes()).unwrap();
    /// let gates = parsed["gates_passed"].as_array().unwrap();
    /// assert_eq!(gates.len(), 3);
    /// assert_eq!(gates[0], "A1");
    /// assert_eq!(gates[2], "A3");
    /// ```
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
            "prior_receipt": self.prior_receipt.as_ref().map(hex32),
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
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::production_record::ProductionRecord;
    ///
    /// let rec = ProductionRecord {
    ///     artifact_hash: [1u8; 32],
    ///     scope_token: "scope-x".into(),
    ///     declared_powl_hash: [0u8; 32],
    ///     ocel_canonical_hash: [0u8; 32],
    ///     conformance_run_id: "run-x".into(),
    ///     gate_config_hash: [0u8; 32],
    ///     production_law_version: "ontostar-1.0.0".into(),
    ///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
    ///     gates_passed: vec!["A1".into()],
    ///     gates_refused: vec![],
    ///     prior_receipt: None,
    ///     signature: None,
    ///     signing_key_fpr: None,
    /// };
    /// let signing_bytes = rec.canonical_bytes_for_signing();
    /// let full_bytes = rec.canonical_bytes();
    /// // Signing bytes are a strict subset — they omit signature/signing_key_fpr.
    /// assert!(signing_bytes.len() < full_bytes.len());
    /// let parsed: serde_json::Value = serde_json::from_slice(&signing_bytes).unwrap();
    /// assert!(parsed.get("signature").is_none());
    /// assert!(parsed.get("signing_key_fpr").is_none());
    /// ```
    ///
    /// Records differing only in `scope_token` produce different signing bytes:
    ///
    /// ```
    /// use open_ontologies::production_record::ProductionRecord;
    ///
    /// let make = |scope: &str| ProductionRecord {
    ///     artifact_hash: [0u8; 32],
    ///     scope_token: scope.into(),
    ///     declared_powl_hash: [0u8; 32],
    ///     ocel_canonical_hash: [0u8; 32],
    ///     conformance_run_id: "r".into(),
    ///     gate_config_hash: [0u8; 32],
    ///     production_law_version: "ontostar-1.0.0".into(),
    ///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
    ///     gates_passed: vec![],
    ///     gates_refused: vec![],
    ///     prior_receipt: None,
    ///     signature: None,
    ///     signing_key_fpr: None,
    /// };
    ///
    /// assert_ne!(
    ///     make("order-to-cash").canonical_bytes_for_signing(),
    ///     make("procure-to-pay").canonical_bytes_for_signing(),
    /// );
    /// ```
    ///
    /// Swapped `artifact_hash` produces different signing input — replay is detectable:
    ///
    /// ```
    /// use open_ontologies::production_record::ProductionRecord;
    ///
    /// let make = |h: u8| ProductionRecord {
    ///     artifact_hash: [h; 32],
    ///     scope_token: "o2c".into(),
    ///     declared_powl_hash: [0u8; 32],
    ///     ocel_canonical_hash: [0u8; 32],
    ///     conformance_run_id: "r".into(),
    ///     gate_config_hash: [0u8; 32],
    ///     production_law_version: "ontostar-1.0.0".into(),
    ///     defects_taxonomy_version: "ontostar-defects-4.8.0".into(),
    ///     gates_passed: vec!["A1".into()],
    ///     gates_refused: vec![],
    ///     prior_receipt: None,
    ///     signature: None,
    ///     signing_key_fpr: None,
    /// };
    ///
    /// // Honest record and replayed record differ in signing input.
    /// assert_ne!(make(0x00).canonical_bytes_for_signing(),
    ///            make(0xff).canonical_bytes_for_signing());
    ///
    /// // artifact_hash is embedded as hex in the signing bytes.
    /// let parsed: serde_json::Value =
    ///     serde_json::from_slice(&make(0x00).canonical_bytes_for_signing()).unwrap();
    /// assert!(parsed["artifact_hash"].as_str().unwrap().chars().all(|c| c == '0'));
    /// ```
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
            "prior_receipt": self.prior_receipt.as_ref().map(hex32),
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
///
/// Returns a 64-character lowercase hex string. Useful for embedding
/// BLAKE3 digests into OCEL event attributes or SPARQL literals.
///
/// # Examples
///
/// ```
/// use open_ontologies::production_record::hex32_pub;
///
/// let digest = [0xabu8; 32];
/// let hex = hex32_pub(&digest);
/// assert_eq!(hex.len(), 64);
/// assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
/// // All bytes are 0xab → each pair is "ab".
/// assert!(hex.chars().collect::<String>().starts_with("ab"));
/// ```
///
/// All-zero digest encodes as 64 `'0'` characters:
///
/// ```
/// use open_ontologies::production_record::hex32_pub;
///
/// let zero = [0u8; 32];
/// assert_eq!(hex32_pub(&zero), "0".repeat(64));
/// ```
///
/// All-`0xff` digest encodes as 64 `'f'` characters:
///
/// ```
/// use open_ontologies::production_record::hex32_pub;
///
/// let ones = [0xffu8; 32];
/// let s = hex32_pub(&ones);
/// assert_eq!(s.len(), 64);
/// assert!(s.chars().all(|c| c == 'f'));
/// ```
pub fn hex32_pub(b: &[u8; 32]) -> String {
    hex32(b)
}
