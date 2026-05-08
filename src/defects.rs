//! Typed defect taxonomy for OntoStar admission.
//!
//! Free-text denials are forbidden. Every denial path returns a typed
//! [`DefectClass`] paired with [`Deviation`] evidence. No string error
//! messages as authority.

use serde::{Deserialize, Serialize};

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
        }
    }
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
