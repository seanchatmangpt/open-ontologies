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

/// Runtime representation of RefusalState8.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RefusalState8 {
    EvidenceIncomplete,
    SyntheticObservedOcelRejected,
    SyntheticClosureLie,
    DirtyTreeUnclassified,
    VersionMismatch,
    CommandFailureUnresolved,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContributionReceipt {
    /// The activity that generated this receipt (prov:wasGeneratedBy).
    pub generating_activity: String,
    /// BLAKE3 hash of the observed evidence (ghf:observedEvidenceHash).
    pub observed_evidence_hash: String,
    /// Hash of the expected closure (ghf:expectedClosureHash).
    pub expected_closure_hash: String,
    /// Optional refusal state if the contribution was rejected.
    pub refusal_state: Option<RefusalState8>,
}

/// Verifies a contribution receipt against provided evidence and expected OCEL.
pub fn verify_receipt(
    receipt: &ContributionReceipt,
    evidence: &[u8],
    expected_ocel: &[u8],
) -> anyhow::Result<()> {
    // 1. Recompute BLAKE3 hash of the provided evidence
    let mut hasher = blake3::Hasher::new();
    hasher.update(evidence);
    let evidence_hash = hasher.finalize().to_hex().to_string();

    // 2. Match it against the observed_evidence_hash
    if evidence_hash != receipt.observed_evidence_hash {
        return Err(anyhow::anyhow!(
            "Observed evidence hash mismatch: expected {}, got {}",
            receipt.observed_evidence_hash,
            evidence_hash
        ));
    }

    // 3. Verify that the refusal_state is not SyntheticClosureLie
    if let Some(RefusalState8::SyntheticClosureLie) = receipt.refusal_state {
        return Err(anyhow::anyhow!(
            "Verification failed: Receipt is marked with SyntheticClosureLie"
        ));
    }

    // 4. Check the expected_closure_hash against the expected OCEL manifest
    let mut expected_hasher = blake3::Hasher::new();
    expected_hasher.update(expected_ocel);
    let expected_hash = expected_hasher.finalize().to_hex().to_string();

    if expected_hash != receipt.expected_closure_hash {
        return Err(anyhow::anyhow!(
            "Expected closure hash mismatch: receipt expects {}, but manifest is {}",
            receipt.expected_closure_hash,
            expected_hash
        ));
    }

    Ok(())
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
    fn test_contribution_receipt_serialization() {
        let receipt = ContributionReceipt {
            generating_activity: "activity-456".to_string(),
            observed_evidence_hash: "a".repeat(64),
            expected_closure_hash: "b".repeat(64),
            refusal_state: Some(RefusalState8::SyntheticClosureLie),
        };
        let json = serde_json::to_string(&receipt).unwrap();
        let decoded: ContributionReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(receipt, decoded);
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

    #[test]
    fn test_verify_receipt_success() {
        let evidence = b"observed evidence";
        let expected_ocel = b"expected ocel manifest";
        
        let evidence_hash = blake3::hash(evidence).to_hex().to_string();
        let expected_hash = blake3::hash(expected_ocel).to_hex().to_string();

        let receipt = ContributionReceipt {
            generating_activity: "activity-1".to_string(),
            observed_evidence_hash: evidence_hash,
            expected_closure_hash: expected_hash,
            refusal_state: None,
        };

        assert!(verify_receipt(&receipt, evidence, expected_ocel).is_ok());
    }

    #[test]
    fn test_verify_receipt_evidence_mismatch() {
        let evidence = b"observed evidence";
        let expected_ocel = b"expected ocel manifest";
        
        let evidence_hash = "wrong hash".to_string();
        let expected_hash = blake3::hash(expected_ocel).to_hex().to_string();

        let receipt = ContributionReceipt {
            generating_activity: "activity-1".to_string(),
            observed_evidence_hash: evidence_hash,
            expected_closure_hash: expected_hash,
            refusal_state: None,
        };

        let result = verify_receipt(&receipt, evidence, expected_ocel);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Observed evidence hash mismatch"));
    }

    #[test]
    fn test_verify_receipt_synthetic_lie() {
        let evidence = b"observed evidence";
        let expected_ocel = b"expected ocel manifest";
        
        let evidence_hash = blake3::hash(evidence).to_hex().to_string();
        let expected_hash = blake3::hash(expected_ocel).to_hex().to_string();

        let receipt = ContributionReceipt {
            generating_activity: "activity-1".to_string(),
            observed_evidence_hash: evidence_hash,
            expected_closure_hash: expected_hash,
            refusal_state: Some(RefusalState8::SyntheticClosureLie),
        };

        let result = verify_receipt(&receipt, evidence, expected_ocel);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Receipt is marked with SyntheticClosureLie"));
    }

    #[test]
    fn test_verify_receipt_expected_mismatch() {
        let evidence = b"observed evidence";
        let expected_ocel = b"expected ocel manifest";
        
        let evidence_hash = blake3::hash(evidence).to_hex().to_string();
        let expected_hash = "wrong hash".to_string();

        let receipt = ContributionReceipt {
            generating_activity: "activity-1".to_string(),
            observed_evidence_hash: evidence_hash,
            expected_closure_hash: expected_hash,
            refusal_state: None,
        };

        let result = verify_receipt(&receipt, evidence, expected_ocel);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Expected closure hash mismatch"));
    }
}
// Verified GHF closure
