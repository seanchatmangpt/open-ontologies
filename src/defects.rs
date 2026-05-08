//! Typed defect taxonomy for OntoStar admission.
//!
//! Free-text denials are forbidden. Every denial path returns a typed
//! [`DefectClass`] paired with [`Deviation`] evidence. No string error
//! messages as authority.
//!
//! # Versioning
//!
//! The taxonomy carries an explicit semver. External auditors read the
//! `defects_taxonomy_version` attribute on every admission OCEL event and
//! every persisted Receipt to know which set of variants ruled.
//!
//! - Bump MAJOR for renamed/removed variants (breaks existing auditors).
//! - Bump MINOR for added variants (forward-compatible).
//! - Bump PATCH for doc-only changes.
//!
//! [`DEFECTS_TAXONOMY_DISCRIMINANT_HASH`] is the BLAKE3 hash of the
//! concatenated tags (in declaration order, NUL-separated). The CI test
//! [`tests::taxonomy_discriminant_hash_pinned`] forces any variant
//! add/rename/remove to trigger a deliberate version bump.

use serde::{Deserialize, Serialize};

/// Current defect taxonomy semver. Stored on every Receipt and emitted as
/// an attribute on every `admission_granted` / `admission_denied` /
/// `admission_audit` OCEL event.
pub const DEFECTS_TAXONOMY_VERSION: &str = "ontostar-defects-2.1.0";

/// BLAKE3 hex of `tag1\0tag2\0...\0` for [`DefectClass::all_tags()`].
/// CI-pinned. Adding/renaming/removing a variant changes this, forcing a
/// taxonomy version bump.
pub const DEFECTS_TAXONOMY_DISCRIMINANT_HASH: &str =
    "e87b042c865d2dbf35300fa4388c152a10692b0a998746048e4d2b479ceb42b4";

/// Typed denial classes. Every `Denied` outcome in admission/cell-ready
/// machinery short-circuits on the first failing variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DefectClass {
    /// Locked IRI / contract breach.
    LawZero,
    /// Required stage missing in OCEL.
    CapabilityZero,
    SkippedTask { stage: String },
    ExtraTask { stage: String },
    WrongOrder { expected: String, got: String },
    MissingGatewayChoice { branches: Vec<String> },
    UnreachableTask { stage: String },
    /// Session under `bypass_admission` revocation.
    BypassRevoked,
    ReceiptMissing,
    ScopeUnclosed,
    OcelIncomplete,
    ThresholdFailed {
        metric: String,
        observed: f64,
        required: f64,
    },
    ShaclSkipped,
    /// No successful replay against declared POWL.
    ReplayFailed,
    /// JSON consumed as canonical instead of Rust byte record.
    ProjectionAsAuthority,
    /// A gate present but not actually checking.
    StubGate,
    DeadParameter { param: String },
    GeneratedArtifactDirectEdit { path: String },
    UnreplayableClaim,
    /// Claimed success without evidence.
    FalsePass,
    // --- Requirements-Andon / CTQ-Forge taxonomy v2.0.0 ---
    /// A `RequirementProposed` op was attempted with no source-voice signal.
    RequirementWithoutSource,
    /// CTQ admission denied because a mandatory field is missing or empty.
    /// `missing` carries one of: "measure", "verification", "negative_case",
    /// "control_plan", "source_voice".
    CtqIncomplete { missing: String },
    /// Work-order admission denied because no naked-craft counterfactual
    /// delta was bound.
    WorkOrderMissingCounterfactual,
    /// LLM (Groq) output was treated as authoritative without passing the
    /// deterministic CTQ admission gate.
    LlmAuthorityClaimed,
    /// Export contains a restricted raw-data field (e.g. customer email,
    /// real account name).
    RawDataLeak { field: String },
    /// Secret material (e.g. `GROQ_API_KEY` value) detected in an evidence
    /// surface (OCEL attribute, receipt, projection, log).
    SecretLeak { surface: String },
    // --- Solution Manufacturing taxonomy v2.1.0 ---
    /// A target generator (iac/rust/erlang/atomvm) emitted no bytes —
    /// the manufacturing pipeline cannot ship an empty artifact.
    GeneratorEmpty { target: String },
    /// Generated IaC (Terraform/Pulumi) failed deterministic validation
    /// (e.g. unbalanced braces, missing required block, illegal IRI).
    IacInvalid { reason: String },
    /// Generated Rust failed deterministic validation (no `pub fn main`,
    /// missing receipt header, unbalanced braces).
    RustInvalid { reason: String },
    /// Generated Erlang failed deterministic validation (missing -module
    /// declaration, missing -export, unmatched parens).
    ErlangInvalid { reason: String },
    /// Generated AtomVM target failed deterministic validation (missing
    /// `start/0`, no AVM-loadable shape).
    AtomVmInvalid { reason: String },
    /// One or more required manufacturing stages (architecture decided,
    /// IaC generated, Rust generated, etc.) is missing — the chain is
    /// broken and cannot ship.
    ManufacturingChainBroken { missing: String },
    /// Solution architecture was not bound to an admitted work order.
    /// Without an upstream WorkOrderAdmitted receipt, no architecture
    /// may be manufactured.
    ArchitectureUnbound,
}

impl DefectClass {
    /// Stable short tag suitable for OCEL `defect` attribute strings.
    pub fn tag(&self) -> &'static str {
        match self {
            DefectClass::LawZero => "law_zero",
            DefectClass::CapabilityZero => "capability_zero",
            DefectClass::SkippedTask { .. } => "skipped_task",
            DefectClass::ExtraTask { .. } => "extra_task",
            DefectClass::WrongOrder { .. } => "wrong_order",
            DefectClass::MissingGatewayChoice { .. } => "missing_gateway_choice",
            DefectClass::UnreachableTask { .. } => "unreachable_task",
            DefectClass::BypassRevoked => "bypass_revoked",
            DefectClass::ReceiptMissing => "receipt_missing",
            DefectClass::ScopeUnclosed => "scope_unclosed",
            DefectClass::OcelIncomplete => "ocel_incomplete",
            DefectClass::ThresholdFailed { .. } => "threshold_failed",
            DefectClass::ShaclSkipped => "shacl_skipped",
            DefectClass::ReplayFailed => "replay_failed",
            DefectClass::ProjectionAsAuthority => "projection_as_authority",
            DefectClass::StubGate => "stub_gate",
            DefectClass::DeadParameter { .. } => "dead_parameter",
            DefectClass::GeneratedArtifactDirectEdit { .. } => "generated_artifact_direct_edit",
            DefectClass::UnreplayableClaim => "unreplayable_claim",
            DefectClass::FalsePass => "false_pass",
            DefectClass::RequirementWithoutSource => "requirement_without_source",
            DefectClass::CtqIncomplete { .. } => "ctq_incomplete",
            DefectClass::WorkOrderMissingCounterfactual => "work_order_missing_counterfactual",
            DefectClass::LlmAuthorityClaimed => "llm_authority_claimed",
            DefectClass::RawDataLeak { .. } => "raw_data_leak",
            DefectClass::SecretLeak { .. } => "secret_leak",
            DefectClass::GeneratorEmpty { .. } => "generator_empty",
            DefectClass::IacInvalid { .. } => "iac_invalid",
            DefectClass::RustInvalid { .. } => "rust_invalid",
            DefectClass::ErlangInvalid { .. } => "erlang_invalid",
            DefectClass::AtomVmInvalid { .. } => "atomvm_invalid",
            DefectClass::ManufacturingChainBroken { .. } => "manufacturing_chain_broken",
            DefectClass::ArchitectureUnbound => "architecture_unbound",
        }
    }

    /// Tag list in declaration order. The hash of this list (NUL-separated)
    /// is pinned in [`DEFECTS_TAXONOMY_DISCRIMINANT_HASH`]. Any variant
    /// add/rename/remove changes the hash and forces a taxonomy version bump.
    pub const fn all_tags() -> &'static [&'static str] {
        &[
            "law_zero",
            "capability_zero",
            "skipped_task",
            "extra_task",
            "wrong_order",
            "missing_gateway_choice",
            "unreachable_task",
            "bypass_revoked",
            "receipt_missing",
            "scope_unclosed",
            "ocel_incomplete",
            "threshold_failed",
            "shacl_skipped",
            "replay_failed",
            "projection_as_authority",
            "stub_gate",
            "dead_parameter",
            "generated_artifact_direct_edit",
            "unreplayable_claim",
            "false_pass",
            "requirement_without_source",
            "ctq_incomplete",
            "work_order_missing_counterfactual",
            "llm_authority_claimed",
            "raw_data_leak",
            "secret_leak",
            "generator_empty",
            "iac_invalid",
            "rust_invalid",
            "erlang_invalid",
            "atomvm_invalid",
            "manufacturing_chain_broken",
            "architecture_unbound",
        ]
    }
}

/// Compute the BLAKE3 hex hash of the concatenated tag list.
pub fn discriminant_hash() -> String {
    let mut h = blake3::Hasher::new();
    for tag in DefectClass::all_tags() {
        h.update(tag.as_bytes());
        h.update(b"\0");
    }
    h.finalize().to_hex().to_string()
}

/// Evidence carried alongside a [`DefectClass`] explaining the deviation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deviation {
    pub kind: String,
    pub stage: String,
    pub detail: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defect_class_round_trips_through_serde() {
        let d = DefectClass::WrongOrder {
            expected: "load".into(),
            got: "save".into(),
        };
        let s = serde_json::to_string(&d).expect("serialize");
        let back: DefectClass = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(d, back);
    }

    #[test]
    fn taxonomy_discriminant_hash_pinned() {
        let got = discriminant_hash();
        assert_eq!(
            got, DEFECTS_TAXONOMY_DISCRIMINANT_HASH,
            "DefectClass tag set changed.\n\
             Expected: {}\n\
             Got:      {}\n\
             If this is intentional, bump DEFECTS_TAXONOMY_VERSION and update \
             DEFECTS_TAXONOMY_DISCRIMINANT_HASH to the 'Got' value above.",
            DEFECTS_TAXONOMY_DISCRIMINANT_HASH, got
        );
    }

    #[test]
    fn deviation_round_trips_through_serde() {
        let dev = Deviation {
            kind: "skipped_task".into(),
            stage: "enforce_run".into(),
            detail: "stage missing in OCEL trace".into(),
            expected: Some("enforce_run".into()),
            actual: None,
        };
        let s = serde_json::to_string(&dev).expect("serialize");
        let back: Deviation = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(dev, back);
    }
}
