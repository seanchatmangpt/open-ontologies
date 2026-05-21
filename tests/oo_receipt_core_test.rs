use open_ontologies::autoreceipt::{
    validate_core_receipt, AlignmentProof, AlignmentState, BoundaryEvidence, Claim, OcelReference,
    OpenOntologyRefusalState8, Receipt,
};

fn dummy_receipt() -> Receipt {
    Receipt {
        receipt_type: "OpenOntologyReceipt".to_string(),
        receipt_schema: "oo.receipt.v1".to_string(),
        version: "26.5.21".to_string(),
        hash_algorithm: "BLAKE3".to_string(),
        claim: Claim {
            artifact_id: "art-1".to_string(),
            operator_id: "op-1".to_string(),
            closure_id: "cls-1".to_string(),
            route_id: "rt-1".to_string(),
        },
        expected_ocel: Some(OcelReference {
            schema: "oo.expected_ocel.v1".into(),
            canonical_hash: "hash_exp".into(),
        }),
        observed_ocel: Some(OcelReference {
            schema: "oo.observed_ocel.v1".into(),
            canonical_hash: "hash_obs".into(),
        }),
        alignment: AlignmentProof {
            state: AlignmentState::Pass,
            missing_events: vec![],
            unexpected_events: vec![],
            refusal_state: None,
        },
        boundary_evidence: Some(BoundaryEvidence {
            git_before: Some("a".into()),
            git_after: Some("b".into()),
            stdout_hash: Some("stdout_h".into()),
            stderr_hash: None,
            exit_code: Some(0),
            files_changed_hash: None,
        }),
        previous_receipt_hash: None,
        receipt_hash: None,
    }
}

#[test]
fn test_refuses_receipt_without_observed_ocel() {
    let mut r = dummy_receipt();
    r.observed_ocel = None;
    assert_eq!(
        validate_core_receipt(&r),
        Err(OpenOntologyRefusalState8::ObservedOCELMissing)
    );
}

#[test]
fn test_refuses_receipt_with_cloned_expected_as_observed() {
    let mut r = dummy_receipt();
    let exp_hash = r.expected_ocel.as_ref().unwrap().canonical_hash.clone();
    r.observed_ocel.as_mut().unwrap().canonical_hash = exp_hash;
    r.boundary_evidence = None; // Missing bounds + matching hashes = cloned synthetic
    assert_eq!(
        validate_core_receipt(&r),
        Err(OpenOntologyRefusalState8::ObservedOCELSynthetic)
    );
}

#[test]
fn test_refuses_receipt_with_exit_code_only_proof() {
    let mut r = dummy_receipt();
    let bounds = BoundaryEvidence {
        git_before: None,
        git_after: None,
        stdout_hash: None,
        stderr_hash: None,
        exit_code: Some(0),
        files_changed_hash: None,
    };
    r.boundary_evidence = Some(bounds);
    assert_eq!(
        validate_core_receipt(&r),
        Err(OpenOntologyRefusalState8::BoundaryEvidenceMissing)
    );
}

#[test]
fn test_refuses_receipt_with_stdout_only_proof_if_alignment_fails() {
    let mut r = dummy_receipt();
    r.alignment.state = AlignmentState::Refused;
    r.alignment.refusal_state = Some(OpenOntologyRefusalState8::OCELAlignmentFailed);
    assert_eq!(
        validate_core_receipt(&r),
        Err(OpenOntologyRefusalState8::OCELAlignmentFailed)
    );
}

#[test]
fn test_admits_receipt_with_full_evidence() {
    let r = dummy_receipt();
    assert_eq!(validate_core_receipt(&r), Ok(()));
}
