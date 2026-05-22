use serde::{Deserialize, Serialize};

// --- Core Open-Ontologies Refusal States ---
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpenOntologyRefusalState8 {
    ExpectedOCELMissing,
    ObservedOCELMissing,
    ObservedOCELSynthetic,
    OCELAlignmentFailed,
    BoundaryEvidenceMissing,
    ArtifactHashMismatch,
    ReceiptHashMismatch,
    ClosureOverclaimed,
    // Anti-Laundering Refusals
    RawBoundaryEvidenceMissing,
    ObservedOCELNotBoundaryDerived,
    ObservedOCELFormattedFromSummary,
    BoundaryEvidenceHashOnly,
    AlignmentReceiptSelfAuthored,
    ExpectedObservedCloneDetected,
    // Domain-specific ones
    FleetDriftDetected,
    RulesetObservedMismatch,
    OutOfMembraneMutation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlignmentState {
    Pass,
    Refused,
    Incomplete,
}

// --- Structural OCEL References ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcelReference {
    pub schema: String,
    pub canonical_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentProof {
    pub state: AlignmentState,
    pub missing_events: Vec<String>,
    pub unexpected_events: Vec<String>,
    pub refusal_state: Option<OpenOntologyRefusalState8>,
    pub verifier_derived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryEvidence {
    pub git_before: Option<String>,
    pub git_after: Option<String>,
    pub stdout_hash: Option<String>,
    pub stderr_hash: Option<String>,
    pub exit_code: Option<i32>,
    pub files_changed_hash: Option<String>,
    pub raw_evidence_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub artifact_id: String,
    pub operator_id: String,
    pub closure_id: String,
    pub route_id: String,
}

// --- The Parent Receipt Schema (v1) ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub receipt_type: String,
    pub receipt_schema: String, // "oo.receipt.v1"
    pub version: String,        // "26.5.21"
    pub hash_algorithm: String, // "BLAKE3"
    
    pub claim: Claim,
    
    pub expected_ocel: Option<OcelReference>,
    pub observed_ocel: Option<OcelReference>,
    pub alignment: AlignmentProof,
    pub boundary_evidence: Option<BoundaryEvidence>,
    
    pub previous_receipt_hash: Option<String>,
    pub receipt_hash: Option<String>,
}

/// Core Validation Law: No embedded OCEL path -> no receipt closure.
pub fn validate_core_receipt(receipt: &Receipt) -> Result<(), OpenOntologyRefusalState8> {
    // 1. Missing Expected OCEL
    let exp = receipt.expected_ocel.as_ref().ok_or(OpenOntologyRefusalState8::ExpectedOCELMissing)?;
    
    // 2. Missing Observed OCEL (No world evidence)
    let obs = receipt.observed_ocel.as_ref().ok_or(OpenOntologyRefusalState8::ObservedOCELMissing)?;
    
    // 3. Expected/Observed Clone Detected (Expected hash cannot equal Observed hash)
    if exp.canonical_hash == obs.canonical_hash {
        return Err(OpenOntologyRefusalState8::ExpectedObservedCloneDetected);
    }
    
    // 4. Missing Boundary Evidence
    let bounds = receipt.boundary_evidence.as_ref().ok_or(OpenOntologyRefusalState8::BoundaryEvidenceMissing)?;
    
    // 5. Raw Boundary Evidence Missing (Hash-only proofs are forbidden)
    if bounds.raw_evidence_hash.is_none() {
        if bounds.stdout_hash.is_some() || bounds.stderr_hash.is_some() {
            return Err(OpenOntologyRefusalState8::BoundaryEvidenceHashOnly);
        } else if bounds.exit_code.is_some() {
            return Err(OpenOntologyRefusalState8::RawBoundaryEvidenceMissing);
        }
        return Err(OpenOntologyRefusalState8::BoundaryEvidenceMissing);
    }
    
    // 6. Alignment Receipt Self-Authored Check
    if !receipt.alignment.verifier_derived {
        return Err(OpenOntologyRefusalState8::AlignmentReceiptSelfAuthored);
    }
    
    // 7. Alignment State
    match receipt.alignment.state {
        AlignmentState::Pass => Ok(()),
        AlignmentState::Refused => Err(receipt.alignment.refusal_state.clone().unwrap_or(OpenOntologyRefusalState8::OCELAlignmentFailed)),
        AlignmentState::Incomplete => Err(OpenOntologyRefusalState8::ClosureOverclaimed),
    }
}