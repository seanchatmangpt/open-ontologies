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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryEvidence {
    pub git_before: Option<String>,
    pub git_after: Option<String>,
    pub stdout_hash: Option<String>,
    pub stderr_hash: Option<String>,
    pub exit_code: Option<i32>,
    pub files_changed_hash: Option<String>,
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
    if receipt.expected_ocel.is_none() {
        return Err(OpenOntologyRefusalState8::ExpectedOCELMissing);
    }
    
    // 2. Missing Observed OCEL (No world evidence)
    let obs = receipt.observed_ocel.as_ref().ok_or(OpenOntologyRefusalState8::ObservedOCELMissing)?;
    
    // 3. Synthetic Cloning Check (Expected hash cannot equal Observed hash directly without real boundary execution)
    let exp = receipt.expected_ocel.as_ref().unwrap();
    if exp.canonical_hash == obs.canonical_hash {
        // If they match perfectly but boundary evidence is missing or trivial, it's a clone lie.
        if receipt.boundary_evidence.is_none() {
            return Err(OpenOntologyRefusalState8::ObservedOCELSynthetic);
        }
    }
    
    // 4. Missing Boundary Evidence (Command smoke alone cannot close JTBDs)
    let bounds = receipt.boundary_evidence.as_ref().ok_or(OpenOntologyRefusalState8::BoundaryEvidenceMissing)?;
    if bounds.stdout_hash.is_none() && bounds.exit_code == Some(0) {
        // Exit code 0 without output hashing is not enough.
        return Err(OpenOntologyRefusalState8::BoundaryEvidenceMissing);
    }
    
    // 5. Alignment State
    match receipt.alignment.state {
        AlignmentState::Pass => Ok(()),
        AlignmentState::Refused => Err(receipt.alignment.refusal_state.clone().unwrap_or(OpenOntologyRefusalState8::OCELAlignmentFailed)),
        AlignmentState::Incomplete => Err(OpenOntologyRefusalState8::ClosureOverclaimed),
    }
}