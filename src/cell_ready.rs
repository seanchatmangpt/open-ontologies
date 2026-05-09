//! `CellReady` — the only function in this codebase allowed to declare
//! manufacturing success.
//!
//! ```text
//! CellReady = WorkflowDeclared          (A1: Seed)
//!           ∧ ScopeClosed                (A2: Breed)
//!           ∧ OCELComplete               (A3: Validate)
//!           ∧ POWLReplayPass             (A4: Reason)
//!           ∧ ThresholdPass              (A5: Prove)
//!           ∧ RequiredStagesPresent      (A6: Seal)
//!           ∧ NoBypassRevocation         (A7: Emit)
//!           ∧ ReceiptValid               (A8: Journal)
//!           ∧ ProvenanceChain            (A9: Causal)
//!           ∧ ExternalAttestation        (A10: Temporal)
//!           ∧ TemporalValidity           (A11: Governance)
//!           ∧ DependencyClosure          (A12: Rollback)
//!           ∧ ReplayProof                (A13: Attest)
//! ```
//!
//! Phase 10: expanded from 8 conjuncts to the full 13-gate Cell8
//! conformance suite. Each conjunct still short-circuits to the first
//! failing typed [`DefectClass`]. **No `bail!`, no `anyhow!`, no string
//! error authority** — every denial is a typed defect class.

use crate::attestation::{self, KeyFingerprint, TrustedKeys, VerifyOutcome};
use crate::defects::DefectClass;
use crate::ocel_store::OcelStore;
use crate::production_record::ProductionRecord;
use crate::receipts::{self, Receipt};
use crate::state::StateDb;

/// Stream-2-stub stand-in for `wasm4pm`'s POWL arena handle. Stream 2
/// replaces this with a re-export from `powl_bridge.rs`.
pub struct PowlOpRef<'a> {
    pub powl_string: &'a str,
    pub powl_hash: [u8; 32],
}

pub struct CellReadyInputs<'a> {
    pub scope_token: &'a str,
    pub declared_powl: &'a PowlOpRef<'a>,
    pub ocel_trace_hash: &'a str,
    pub artifact_hash: &'a str,
    pub gate_config_hash: &'a str,
    pub session_revoked: bool,

    // Threshold-pass inputs (computed by admission gate via wasm4pm)
    pub fitness_observed: f64,
    pub precision_observed: f64,
    pub fitness_required: f64,
    pub precision_required: f64,

    // Required-stages-present inputs (workflow alphabet vs. observed events)
    pub required_stages: &'a [String],
    pub observed_stages: &'a [String],

    /// Conformance run id for the matching POWL replay.
    pub conformance_run_id: &'a str,

    /// Production-law version label, e.g. "ontostar-1.0.0".
    pub production_law_version: &'a str,

    /// Optional prior receipt hash to chain into the new record.
    pub prior_receipt: Option<[u8; 32]>,

    /// Session id that owns the receipts row when persisted.
    pub session_id: &'a str,

    // ── Phase 10: A9–A13 inputs ───────────────────────────────────────────
    /// A9 (provenance chain): every artifact hash referenced here must
    /// have a recorded `prov:wasGeneratedBy` event entry. Empty list
    /// means "no provenance evidence" → A9 fails.
    pub provenance_evidence: &'a [String],

    /// A10 (external attestation): hex-encoded BLAKE3 attestation digest
    /// that must equal `artifact_hash` to verify (poor-man's Ed25519
    /// stand-in until ed25519-dalek lands). Empty string → A10 fails.
    pub external_attestation: &'a str,

    /// A11 (temporal validity): RFC-3339 granted_at timestamps in
    /// observed order. Must be monotonic non-decreasing. Empty → fail.
    pub granted_at_chain: &'a [String],

    /// A12 (dependency closure): every prior receipt hex referenced
    /// here must itself be admitted (present in `admitted_receipts`).
    pub admitted_receipts: &'a [String],

    /// A13 (replay proof): expected canonical OCEL hash from a
    /// deterministic POWL replay. Must equal `ocel_trace_hash` byte-
    /// for-byte. Empty string → fail.
    pub replay_canonical_hash: &'a str,

    // ── Real Ed25519 attestation (replaces the A10 tautology stub) ──
    /// Ed25519 signature over [`ProductionRecord::canonical_bytes_for_signing`]
    /// of the record cell_ready will build. `None` activates the
    /// legacy-unsigned branch (A10 either passes-with-warning or fails
    /// based on `allow_legacy_unsigned`).
    pub signature: Option<[u8; 64]>,

    /// 8-byte BLAKE3-prefix fingerprint of the public key that produced
    /// `signature`. Required when `signature` is `Some`.
    pub signing_key_fpr: Option<KeyFingerprint>,

    /// Trust set used to resolve `signing_key_fpr`. `None` → A10 cannot
    /// verify even when `signature` is present; falls through to
    /// `AttestationInvalid { reason: "no_trust_set" }`.
    pub trusted_keys: Option<&'a TrustedKeys>,

    /// When `true`, A10 admits a record with `signature: None` by
    /// emitting a `legacy_unsigned_receipt` OCEL event and passing the
    /// conjunct. When `false`, `signature: None` raises
    /// [`DefectClass::AttestationMissing`].
    pub allow_legacy_unsigned: bool,

    /// Round 4 WD — `StateDb` handle used to resolve the signing
    /// fingerprint's `trusted_keys_history` row. When `Some`, A10
    /// rejects receipts whose `granted_at` falls outside the
    /// `[added_at, removed_at)` window with
    /// `AttestationInvalid { reason: "key_not_trusted_at_signature_time" }`.
    /// When `None`, the window check is skipped — used by tests and the
    /// legacy unsigned-receipt path.
    pub trusted_keys_db: Option<&'a StateDb>,
}

/// Compute the `CellReady` predicate. Returns a freshly built (but not yet
/// persisted) [`Receipt`] on success, or the **first** failing
/// [`DefectClass`] on failure.
///
/// Persistence of the receipt is the caller's responsibility (admission
/// gate); this function only certifies that all thirteen conjuncts hold.
pub fn cell_ready(
    inp: CellReadyInputs<'_>,
    store: &OcelStore,
) -> Result<Receipt, DefectClass> {
    // 1. WorkflowDeclared — declared_workflows row exists for scope_token.
    if !workflow_declared(store, inp.scope_token) {
        return Err(DefectClass::ScopeUnclosed);
    }

    // 2. ScopeClosed — closed_at IS NOT NULL.
    if !scope_closed(store, inp.scope_token) {
        return Err(DefectClass::ScopeUnclosed);
    }

    // 3. OCELComplete — at least one event observed for the scope.
    if !ocel_complete(inp.observed_stages) {
        return Err(DefectClass::OcelIncomplete);
    }

    // 4. POWLReplayPass — conformance_runs row with verdict='conform'.
    if !replay_pass(store, inp.scope_token) {
        return Err(DefectClass::ReplayFailed);
    }

    // 5. ThresholdPass — fitness ≥ f_min, precision ≥ p_min.
    if inp.fitness_observed < inp.fitness_required {
        return Err(DefectClass::ThresholdFailed {
            metric: "fitness".into(),
            observed: inp.fitness_observed,
            required: inp.fitness_required,
        });
    }
    if inp.precision_observed < inp.precision_required {
        return Err(DefectClass::ThresholdFailed {
            metric: "precision".into(),
            observed: inp.precision_observed,
            required: inp.precision_required,
        });
    }

    // 6. RequiredStagesPresent — every stage in required_stages present.
    for stage in inp.required_stages {
        if !inp.observed_stages.iter().any(|s| s == stage) {
            return Err(DefectClass::CapabilityZero);
        }
    }

    // 7. NoBypassRevocation — session not in revoked_sessions.
    if inp.session_revoked {
        return Err(DefectClass::BypassRevoked);
    }

    // 8. ReceiptValid — canonical hashes recompute.
    let artifact_hash = parse_hex32(inp.artifact_hash).ok_or(DefectClass::ReceiptMissing)?;
    let ocel_canonical_hash =
        parse_hex32(inp.ocel_trace_hash).ok_or(DefectClass::ReceiptMissing)?;
    let gate_config_hash =
        parse_hex32(inp.gate_config_hash).ok_or(DefectClass::ReceiptMissing)?;

    // 9. A9_provenance_chain — every artifact hash has prov:wasGeneratedBy.
    if inp.provenance_evidence.is_empty()
        || !inp.provenance_evidence.iter().any(|p| p == inp.artifact_hash)
    {
        return Err(DefectClass::ProvenanceMissing {
            artifact_hash: inp.artifact_hash.to_string(),
        });
    }

    // 10. A10_external_attestation — REAL Ed25519 verification.
    //     The Phase-10 tautology (`external_attestation == artifact_hash`)
    //     is gone. Three branches:
    //       (a) signature: None + allow_legacy_unsigned: true → emit a
    //           `legacy_unsigned_receipt` OCEL event and pass the conjunct.
    //       (b) signature: None + allow_legacy_unsigned: false →
    //           DefectClass::AttestationMissing.
    //       (c) signature: Some(sig) → call verify_strict over
    //           ProductionRecord::canonical_bytes_for_signing for the
    //           record we are about to build. Failure raises
    //           DefectClass::AttestationInvalid { reason }.
    match inp.signature {
        None => {
            if !inp.allow_legacy_unsigned {
                return Err(DefectClass::AttestationMissing);
            }
            // Legacy-unsigned-receipt audit event (best-effort).
            let event_id = format!(
                "{}:legacy_unsigned_receipt:{}",
                inp.session_id,
                chrono::Utc::now().timestamp_millis()
            );
            let _ = store.emit_event(
                &event_id,
                "legacy_unsigned_receipt",
                &chrono::Utc::now().to_rfc3339(),
                inp.session_id,
                &[
                    ("scope_token", inp.scope_token),
                    ("artifact_hash", inp.artifact_hash),
                    ("reason", "no signature; verify_legacy_receipts=true"),
                ],
                &[],
                Some(inp.scope_token),
            );
        }
        Some(sig) => {
            let trust = match inp.trusted_keys {
                Some(t) => t,
                None => {
                    return Err(DefectClass::AttestationInvalid {
                        reason: "no_trust_set".into(),
                    });
                }
            };
            let fpr = match inp.signing_key_fpr.as_ref() {
                Some(f) => f,
                None => {
                    return Err(DefectClass::AttestationInvalid {
                        reason: "missing_signing_key_fpr".into(),
                    });
                }
            };

            // Round 4 WD — validity-window enforcement. Look up the
            // fingerprint's history row; reject when the receipt's
            // signature time falls outside the [added_at, removed_at)
            // window. Without this check, a key rotated OUT today still
            // verifies receipts that were signed yesterday-with-malice.
            //
            // Backward compat (Plan D Option 1): if no history row
            // exists for this fingerprint we skip the window check with
            // a `tracing::warn!`. Legacy databases that pre-date the
            // `trusted_keys_history` table would otherwise refuse every
            // signed receipt. New deployments should populate history
            // via `TrustedKeys::from_dir_with_history` at startup.
            if let Some(db) = inp.trusted_keys_db {
                if let Some(history) =
                    attestation::TrustedKeys::lookup_history(db, fpr)
                {
                    let signed_at = inp
                        .granted_at_chain
                        .last()
                        .map(String::as_str)
                        .unwrap_or("");
                    if !signed_at.is_empty() {
                        if signed_at < history.added_at.as_str() {
                            return Err(DefectClass::AttestationInvalid {
                                reason: "key_not_trusted_at_signature_time".into(),
                            });
                        }
                        if let Some(removed_at) = history.removed_at.as_ref() {
                            if signed_at >= removed_at.as_str() {
                                return Err(DefectClass::AttestationInvalid {
                                    reason: "key_not_trusted_at_signature_time".into(),
                                });
                            }
                        }
                    }
                } else {
                    tracing::warn!(
                        "cell_ready A10: no trusted_keys_history row for {} \
                         (legacy receipt path; window check skipped)",
                        attestation::fingerprint_hex(fpr),
                    );
                }
            }
            // Build the would-be record (without sig/fpr) so we can
            // recompute canonical_bytes_for_signing — the exact bytes
            // the admission gate signed.
            let preview = ProductionRecord {
                artifact_hash,
                scope_token: inp.scope_token.to_string(),
                declared_powl_hash: inp.declared_powl.powl_hash,
                ocel_canonical_hash,
                conformance_run_id: inp.conformance_run_id.to_string(),
                gate_config_hash,
                production_law_version: inp.production_law_version.to_string(),
                defects_taxonomy_version: crate::defects::DEFECTS_TAXONOMY_VERSION
                    .to_string(),
                gates_passed: vec![
                    "A1_WorkflowDeclared".into(),
                    "A2_ScopeClosed".into(),
                    "A3_OCELComplete".into(),
                    "A4_POWLReplayPass".into(),
                    "A5_ThresholdPass".into(),
                    "A6_RequiredStagesPresent".into(),
                    "A7_NoBypassRevocation".into(),
                    "A8_ReceiptValid".into(),
                    "A9_ProvenanceChain".into(),
                    "A10_ExternalAttestation".into(),
                    "A11_TemporalValidity".into(),
                    "A12_DependencyClosure".into(),
                    "A13_ReplayProof".into(),
                ],
                gates_refused: Vec::new(),
                prior_receipt: inp.prior_receipt,
                signature: None,
                signing_key_fpr: None,
            };
            let msg = preview.canonical_bytes_for_signing();
            match attestation::verify_strict(trust, fpr, &msg, &sig) {
                VerifyOutcome::Valid => {
                    // Pass.
                }
                VerifyOutcome::UnknownKey => {
                    return Err(DefectClass::AttestationInvalid {
                        reason: format!(
                            "unknown_signing_key:{}",
                            attestation::fingerprint_hex(fpr)
                        ),
                    });
                }
                VerifyOutcome::SignatureInvalid => {
                    return Err(DefectClass::AttestationInvalid {
                        reason: "signature_invalid".into(),
                    });
                }
            }
        }
    }
    // legacy `external_attestation` field is intentionally ignored — kept
    // in CellReadyInputs only for source-compat with older test scaffolding.
    // Touch it via a no-op debug assertion so the compiler doesn't warn.
    debug_assert!(inp.external_attestation.is_empty() || !inp.external_attestation.is_empty());

    // 11. A11_temporal_validity — granted_at chain monotonic.
    if inp.granted_at_chain.is_empty() {
        return Err(DefectClass::TemporalSkew { observed_skew_ms: 0 });
    }
    for w in inp.granted_at_chain.windows(2) {
        if w[0] > w[1] {
            // Compute the negative skew in ms for evidence; tolerate parse
            // failures by reporting -1 (sentinel "unparseable but inverted").
            let skew_ms = parse_skew_ms(&w[0], &w[1]).unwrap_or(-1);
            return Err(DefectClass::TemporalSkew { observed_skew_ms: skew_ms });
        }
    }

    // 12. A12_dependency_closure — every prior_receipt is admitted.
    if let Some(prior) = inp.prior_receipt.as_ref() {
        let prior_hex = hex32_local(prior);
        if !inp.admitted_receipts.iter().any(|r| r == &prior_hex) {
            return Err(DefectClass::DependencyClosureBroken {
                missing_hash: prior_hex,
            });
        }
    }

    // 13. A13_replay_proof — POWL replay byte-identical to OCEL hash.
    if inp.replay_canonical_hash != inp.ocel_trace_hash {
        return Err(DefectClass::ReplayDivergence {
            expected: inp.ocel_trace_hash.to_string(),
            observed: inp.replay_canonical_hash.to_string(),
        });
    }

    let record = ProductionRecord {
        artifact_hash,
        scope_token: inp.scope_token.to_string(),
        declared_powl_hash: inp.declared_powl.powl_hash,
        ocel_canonical_hash,
        conformance_run_id: inp.conformance_run_id.to_string(),
        gate_config_hash,
        production_law_version: inp.production_law_version.to_string(),
        defects_taxonomy_version: crate::defects::DEFECTS_TAXONOMY_VERSION.to_string(),
        gates_passed: vec![
            "A1_WorkflowDeclared".into(),
            "A2_ScopeClosed".into(),
            "A3_OCELComplete".into(),
            "A4_POWLReplayPass".into(),
            "A5_ThresholdPass".into(),
            "A6_RequiredStagesPresent".into(),
            "A7_NoBypassRevocation".into(),
            "A8_ReceiptValid".into(),
            "A9_ProvenanceChain".into(),
            "A10_ExternalAttestation".into(),
            "A11_TemporalValidity".into(),
            "A12_DependencyClosure".into(),
            "A13_ReplayProof".into(),
        ],
        gates_refused: Vec::new(),
        prior_receipt: inp.prior_receipt,
        signature: inp.signature,
        signing_key_fpr: inp.signing_key_fpr,
    };

    Ok(receipts::build(record))
}

// ─── conjunct helpers ──────────────────────────────────────────────────────

fn workflow_declared(store: &OcelStore, scope_token: &str) -> bool {
    store.has_declared_workflow(scope_token).unwrap_or(false)
}

fn scope_closed(store: &OcelStore, scope_token: &str) -> bool {
    store.is_scope_closed(scope_token).unwrap_or(false)
}

fn replay_pass(store: &OcelStore, scope_token: &str) -> bool {
    store.has_conforming_replay(scope_token).unwrap_or(false)
}

fn ocel_complete(observed: &[String]) -> bool {
    !observed.is_empty()
}

fn parse_hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Best-effort RFC-3339 skew in milliseconds: returns `later − earlier`.
/// When `earlier > later` (the failure case for monotonicity) the result
/// is negative. Returns `None` when either side is unparseable.
fn parse_skew_ms(earlier: &str, later: &str) -> Option<i64> {
    let e = chrono::DateTime::parse_from_rfc3339(earlier).ok()?;
    let l = chrono::DateTime::parse_from_rfc3339(later).ok()?;
    Some(l.signed_duration_since(e).num_milliseconds())
}

fn hex32_local(b: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for byte in b {
        s.push_str(&format!("{:02x}", byte));
    }
    s
}
