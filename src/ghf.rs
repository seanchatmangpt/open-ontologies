use serde::{Deserialize, Serialize};

/// Foundational "8-basis" structs mapping to specific ontological concepts.
/// These provide the semantic grounding for all GitHub Factory operations.

/// Represents a role (mapping to `org:Role`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Role8(pub String);

/// Represents a purpose (mapping to `dpv:Purpose`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Purpose8(pub String);

/// Represents a scope or constraint (mapping to `odrl:Constraint`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scope8(pub String);

/// Represents a disclosure or processing type (mapping to `dpv:Processing`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Disclosure8(pub String);

// --- Typestates for RouteState8 ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpectedOcelManufactured;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionBindingReady;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RealBoundaryExecuted;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservedOcelCaptured;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OcelAlignmentPassed;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OcelAlignmentFailed;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptEmitted;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptVerified;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoReceiptClosed;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoReceiptBlocked;

/// Runtime representation of RouteState8.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RouteState8 {
    ExpectedOcelManufactured,
    ExecutionBindingReady,
    RealBoundaryExecuted,
    ObservedOcelCaptured,
    OcelAlignmentPassed,
    OcelAlignmentFailed,
    ReceiptEmitted,
    ReceiptVerified,
    AutoReceiptClosed,
    AutoReceiptBlocked,
}

// --- Typestates for RefusalState8 ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceIncomplete;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntheticObservedOcelRejected;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntheticClosureLie;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirtyTreeUnclassified;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionMismatch;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandFailureUnresolved;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptSchemaInvalid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HashBindingFailed;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoundaryEvidenceMissing;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyConformanceFailed;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OCELAlignmentFailed;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayFailed;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FleetDriftDetected;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemporalConformanceFailed;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalVerificationFailed;

/// Runtime representation of RefusalState8.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RefusalState8 {
    EvidenceIncomplete,
    SyntheticObservedOcelRejected,
    SyntheticClosureLie,
    DirtyTreeUnclassified,
    VersionMismatch,
    CommandFailureUnresolved,
    // V0-V8 Refusal States
    ReceiptSchemaInvalid,
    HashBindingFailed,
    BoundaryEvidenceMissing,
    PolicyConformanceFailed,
    OCELAlignmentFailed,
    ReplayFailed,
    FleetDriftDetected,
    TemporalConformanceFailed,
    ExternalVerificationFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationState {
    Admitted,
    Refused,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub state: VerificationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<RefusalState8>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub missing: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_hash: Option<String>,
}

/// A unit of contribution within the GitHub Factory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContributionUnit {
    /// The Job To Be Done (JTBD) identifier.
    pub jtbd_id: String,
    /// The persona performing the contribution.
    pub persona: String,
    /// The target GitHub repository.
    pub target_repo: String,
}

/// A receipt proving a contribution has been made and verified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionReceipt {
    #[serde(flatten)]
    pub core: crate::autoreceipt::Receipt,
    /// Additional GHF-specific metadata can be placed here
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ghf_specific_refusal: Option<RefusalState8>,
}

/// Verifies a contribution receipt against the OpenOntologyReceipt core laws.
pub fn verify_receipt(receipt: &ContributionReceipt) -> ValidationResult {
    match crate::autoreceipt::validate_core_receipt(&receipt.core) {
        Ok(_) => ValidationResult {
            state: VerificationState::Admitted,
            refusal: None,
            missing: vec![],
            receipt_hash: receipt.core.receipt_hash.clone(),
        },
        Err(e) => {
            // Map the core refusal state to the GHF validation result.
            // For now, we will serialize the core error into the missing array or a string representation,
            // or we could map them directly if we unify the enums. Let's map it roughly.
            let mapped_refusal = match e {
                crate::autoreceipt::OpenOntologyRefusalState8::ExpectedOCELMissing => RefusalState8::EvidenceIncomplete,
                crate::autoreceipt::OpenOntologyRefusalState8::ObservedOCELMissing => RefusalState8::BoundaryEvidenceMissing,
                crate::autoreceipt::OpenOntologyRefusalState8::ObservedOCELSynthetic => RefusalState8::SyntheticClosureLie,
                crate::autoreceipt::OpenOntologyRefusalState8::OCELAlignmentFailed => RefusalState8::OCELAlignmentFailed,
                crate::autoreceipt::OpenOntologyRefusalState8::BoundaryEvidenceMissing => RefusalState8::BoundaryEvidenceMissing,
                crate::autoreceipt::OpenOntologyRefusalState8::ArtifactHashMismatch => RefusalState8::HashBindingFailed,
                crate::autoreceipt::OpenOntologyRefusalState8::ReceiptHashMismatch => RefusalState8::HashBindingFailed,
                crate::autoreceipt::OpenOntologyRefusalState8::ClosureOverclaimed => RefusalState8::CommandFailureUnresolved,
                crate::autoreceipt::OpenOntologyRefusalState8::FleetDriftDetected => RefusalState8::FleetDriftDetected,
                crate::autoreceipt::OpenOntologyRefusalState8::RulesetObservedMismatch => RefusalState8::PolicyConformanceFailed,
                crate::autoreceipt::OpenOntologyRefusalState8::OutOfMembraneMutation => RefusalState8::ExternalVerificationFailed,
                crate::autoreceipt::OpenOntologyRefusalState8::RawBoundaryEvidenceMissing => RefusalState8::BoundaryEvidenceMissing,
                crate::autoreceipt::OpenOntologyRefusalState8::ObservedOCELNotBoundaryDerived => RefusalState8::SyntheticClosureLie,
                crate::autoreceipt::OpenOntologyRefusalState8::ObservedOCELFormattedFromSummary => RefusalState8::SyntheticClosureLie,
                crate::autoreceipt::OpenOntologyRefusalState8::BoundaryEvidenceHashOnly => RefusalState8::BoundaryEvidenceMissing,
                crate::autoreceipt::OpenOntologyRefusalState8::AlignmentReceiptSelfAuthored => RefusalState8::SyntheticClosureLie,
                crate::autoreceipt::OpenOntologyRefusalState8::ExpectedObservedCloneDetected => RefusalState8::SyntheticClosureLie,
            };
            
            ValidationResult {
                state: VerificationState::Refused,
                refusal: Some(mapped_refusal),
                missing: vec![format!("{:?}", e)],
                receipt_hash: receipt.core.receipt_hash.clone(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contribution_unit_serialization() {
        let unit = ContributionUnit {
            jtbd_id: "jtbd-123".to_string(),
            persona: "architect".to_string(),
            target_repo: "open-ontologies".to_string(),
        };
        let json = serde_json::to_string(&unit).unwrap();
        let decoded: ContributionUnit = serde_json::from_str(&json).unwrap();
        assert_eq!(unit, decoded);
    }

    #[test]
    fn test_8basis_structs() {
        let role = Role8("org:Role".to_string());
        let purpose = Purpose8("dpv:Purpose".to_string());
        let scope = Scope8("odrl:Constraint".to_string());
        let disclosure = Disclosure8("dpv:Processing".to_string());

        assert_eq!(role.0, "org:Role");
        assert_eq!(purpose.0, "dpv:Purpose");
        assert_eq!(scope.0, "odrl:Constraint");
        assert_eq!(disclosure.0, "dpv:Processing");
    }
}
