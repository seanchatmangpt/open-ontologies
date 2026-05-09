//! Real Ed25519 attestation for OntoStar receipts.
//!
//! Replaces the Phase-10 A10 tautology stub (which compared
//! `external_attestation == artifact_hash`, a vacuous self-check) with
//! cryptographic signatures over the canonical bytes of a
//! [`crate::production_record::ProductionRecord`].
//!
//! # Trust model
//!
//! - The admission gate holds a single `Signer` loaded from a PEM-encoded
//!   PKCS#8 Ed25519 private key at `OPEN_ONTOLOGIES_SIGNING_KEY_PATH`.
//!   It signs every receipt's canonical bytes (`canonical_bytes_for_signing`)
//!   before persistence.
//! - The Cell8 A10 verifier holds a [`TrustedKeys`] set loaded from a
//!   directory of PEM-encoded `*.pub.pem` SubjectPublicKeyInfo files at
//!   `OPEN_ONTOLOGIES_TRUSTED_KEYS_DIR`. Each public key is fingerprinted
//!   by BLAKE3-prefix (8 bytes) so receipts can name a signer-of-record
//!   without leaking the full key material.
//! - Verification uses [`ed25519_dalek::VerifyingKey::verify_strict`],
//!   which rejects malleable / non-canonical signatures (RFC 8032 §5.1.7
//!   strict mode).
//!
//! # Receipt-replay defence
//!
//! The signature is computed over `canonical_bytes_for_signing` — the
//! canonical record bytes EXCLUDING the `signature` and `signing_key_fpr`
//! fields. This means pasting the signature from receipt A onto receipt B
//! (with a different `artifact_hash`) re-derives a different message and
//! `verify_strict` returns `Err(_)`.
//!
//! # Backwards compatibility
//!
//! Records persisted before this commit have no `signature` field. They
//! deserialize with `signature = None`. Cell8 A10 admits them only when
//! `[admission] verify_legacy_receipts = true`; otherwise it raises
//! `DefectClass::AttestationMissing`.

use ed25519_dalek::pkcs8::{DecodePrivateKey, DecodePublicKey};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 8-byte BLAKE3-prefix fingerprint of an Ed25519 verifying key. Stored
/// on every signed [`crate::production_record::ProductionRecord`] so the
/// verifier can pick the right `VerifyingKey` from the trust set without
/// trial-decrypting against every key.
pub type KeyFingerprint = [u8; 8];

/// Compute the fingerprint of a verifying key: `BLAKE3(verifying_key_bytes)[..8]`.
pub fn fingerprint(vk: &VerifyingKey) -> KeyFingerprint {
    let h = blake3::hash(vk.as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&h.as_bytes()[..8]);
    out
}

/// Hex-encode a key fingerprint for human-readable error messages.
pub fn fingerprint_hex(fpr: &KeyFingerprint) -> String {
    let mut s = String::with_capacity(16);
    for byte in fpr {
        s.push_str(&format!("{:02x}", byte));
    }
    s
}

/// Single Ed25519 signer used by the admission gate. Wraps a
/// [`SigningKey`] loaded from a PEM-encoded PKCS#8 file.
#[derive(Debug)]
pub struct Signer {
    key: SigningKey,
    fpr: KeyFingerprint,
}

impl Signer {
    /// Load a signer from `OPEN_ONTOLOGIES_SIGNING_KEY_PATH`. Returns
    /// `None` when the env var is unset or empty (the admission gate then
    /// emits unsigned receipts; A10 falls back to the legacy path
    /// controlled by `[admission] verify_legacy_receipts`).
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let path = match std::env::var("OPEN_ONTOLOGIES_SIGNING_KEY_PATH") {
            Ok(p) if !p.trim().is_empty() => p,
            _ => return Ok(None),
        };
        Self::from_pem_file(Path::new(&path)).map(Some)
    }

    /// Load a signer from a PEM-encoded PKCS#8 Ed25519 private key file.
    pub fn from_pem_file(path: &Path) -> anyhow::Result<Self> {
        let pem = std::fs::read_to_string(path).map_err(|e| {
            anyhow::anyhow!("read signing key {}: {e}", path.display())
        })?;
        Self::from_pem_str(&pem)
    }

    /// Parse a PEM-encoded PKCS#8 Ed25519 private key.
    pub fn from_pem_str(pem: &str) -> anyhow::Result<Self> {
        let key = SigningKey::from_pkcs8_pem(pem)
            .map_err(|e| anyhow::anyhow!("parse pkcs8 ed25519 private key: {e}"))?;
        let fpr = fingerprint(&key.verifying_key());
        Ok(Self { key, fpr })
    }

    /// Construct a signer directly from key bytes (test/utility helper).
    pub fn from_bytes(secret: &[u8; 32]) -> Self {
        let key = SigningKey::from_bytes(secret);
        let fpr = fingerprint(&key.verifying_key());
        Self { key, fpr }
    }

    /// Sign the given bytes (typically the canonical record bytes excluding
    /// the signature + fingerprint fields).
    pub fn sign(&self, msg: &[u8]) -> Signature {
        use ed25519_dalek::Signer as _;
        self.key.sign(msg)
    }

    pub fn fingerprint(&self) -> KeyFingerprint {
        self.fpr
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }
}

/// Trust set of public Ed25519 verifying keys, indexed by fingerprint.
#[derive(Debug, Default, Clone)]
pub struct TrustedKeys {
    keys: BTreeMap<KeyFingerprint, VerifyingKey>,
}

impl TrustedKeys {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load every `*.pub.pem` file from the directory named by
    /// `OPEN_ONTOLOGIES_TRUSTED_KEYS_DIR`. Returns `None` when the env var
    /// is unset or empty. Returns an error when the directory exists but
    /// is unreadable, or when any file present in it fails to parse — we
    /// fail closed: a misconfigured trust set must NOT silently downgrade
    /// to "trust nothing".
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let dir = match std::env::var("OPEN_ONTOLOGIES_TRUSTED_KEYS_DIR") {
            Ok(p) if !p.trim().is_empty() => p,
            _ => return Ok(None),
        };
        Self::from_dir(Path::new(&dir)).map(Some)
    }

    /// Load every `*.pub.pem` file from `dir`. Subdirectories are ignored.
    pub fn from_dir(dir: &Path) -> anyhow::Result<Self> {
        let mut out = Self::new();
        let entries = std::fs::read_dir(dir).map_err(|e| {
            anyhow::anyhow!("read trusted keys dir {}: {e}", dir.display())
        })?;
        for entry in entries {
            let entry = entry?;
            let path: PathBuf = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if !name.ends_with(".pub.pem") {
                continue;
            }
            let pem = std::fs::read_to_string(&path)
                .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
            out.insert_pem(&pem).map_err(|e| {
                anyhow::anyhow!("parse trusted key {}: {e}", path.display())
            })?;
        }
        Ok(out)
    }

    /// Add a verifying key from a PEM-encoded SubjectPublicKeyInfo string.
    pub fn insert_pem(&mut self, pem: &str) -> anyhow::Result<KeyFingerprint> {
        let vk = VerifyingKey::from_public_key_pem(pem)
            .map_err(|e| anyhow::anyhow!("parse pkcs8 ed25519 public key: {e}"))?;
        let fpr = fingerprint(&vk);
        self.keys.insert(fpr, vk);
        Ok(fpr)
    }

    /// Add a verifying key by fingerprint (test/utility).
    pub fn insert(&mut self, vk: VerifyingKey) -> KeyFingerprint {
        let fpr = fingerprint(&vk);
        self.keys.insert(fpr, vk);
        fpr
    }

    pub fn get(&self, fpr: &KeyFingerprint) -> Option<&VerifyingKey> {
        self.keys.get(fpr)
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

/// Verdict from verifying a signed message against a trust set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyOutcome {
    /// Strict verification succeeded.
    Valid,
    /// `signing_key_fpr` is not present in the trust set.
    UnknownKey,
    /// Key was found but `verify_strict` rejected the signature
    /// (tampered message, malleable signature, or wrong key).
    SignatureInvalid,
}

/// Verify a 64-byte signature over `msg` using the verifying key with
/// fingerprint `fpr` from `trust`. Uses [`VerifyingKey::verify_strict`]
/// (RFC 8032 §5.1.7 strict mode) so non-canonical signatures are
/// rejected.
pub fn verify_strict(
    trust: &TrustedKeys,
    fpr: &KeyFingerprint,
    msg: &[u8],
    sig: &[u8; 64],
) -> VerifyOutcome {
    let vk = match trust.get(fpr) {
        Some(v) => v,
        None => return VerifyOutcome::UnknownKey,
    };
    let signature = Signature::from_bytes(sig);
    match vk.verify_strict(msg, &signature) {
        Ok(()) => VerifyOutcome::Valid,
        Err(_) => VerifyOutcome::SignatureInvalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
    use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey};
    use rand_core::OsRng;

    fn gen_key_pair() -> (Signer, VerifyingKey) {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();
        let fpr = fingerprint(&vk);
        (
            Signer { key: sk, fpr },
            vk,
        )
    }

    #[test]
    fn roundtrip_pem_signer_then_verify() {
        let (signer, vk) = gen_key_pair();
        // Encode signer to PEM, reload, and check fingerprints match.
        let sk_pem = signer
            .key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("encode pkcs8 pem")
            .to_string();
        let reloaded = Signer::from_pem_str(&sk_pem).expect("reload");
        assert_eq!(reloaded.fingerprint(), signer.fingerprint());

        let mut trust = TrustedKeys::new();
        let vk_pem = vk
            .to_public_key_pem(LineEnding::LF)
            .expect("encode pubkey pem");
        trust.insert_pem(&vk_pem).expect("insert pem");
        assert_eq!(trust.len(), 1);
        assert!(trust.get(&signer.fingerprint()).is_some());

        let msg = b"the quick brown fox";
        let sig = signer.sign(msg);
        assert_eq!(
            verify_strict(&trust, &signer.fingerprint(), msg, &sig.to_bytes()),
            VerifyOutcome::Valid
        );
    }

    #[test]
    fn verify_rejects_tampered_message() {
        let (signer, vk) = gen_key_pair();
        let mut trust = TrustedKeys::new();
        trust.insert(vk);
        let msg = b"original";
        let sig = signer.sign(msg);
        let tampered = b"tampered";
        assert_eq!(
            verify_strict(&trust, &signer.fingerprint(), tampered, &sig.to_bytes()),
            VerifyOutcome::SignatureInvalid
        );
    }

    #[test]
    fn verify_unknown_key_when_fpr_absent() {
        let (signer, _vk) = gen_key_pair();
        let trust = TrustedKeys::new();
        let sig = signer.sign(b"x");
        assert_eq!(
            verify_strict(&trust, &signer.fingerprint(), b"x", &sig.to_bytes()),
            VerifyOutcome::UnknownKey
        );
    }
}
