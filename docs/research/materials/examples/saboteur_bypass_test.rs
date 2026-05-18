//! Case Study 2: The Hallucinated Workflow (Exercise 2.1 & 2.2)
//! 
//! This file is a mock test harness demonstrating how a student in SAB 900 
//! would architect an exploit against the LLM boundary, and how the L4 
//! admission gate ultimately catches it.

use open_ontologies::admission::{evaluate_admission, AdmissionOp};
use open_ontologies::defects::DefectClass;
use open_ontologies::llm_input::LlmInput;

#[tokio::test]
async fn test_cryptographic_bypass_hallucination() {
    // -------------------------------------------------------------------------
    // Phase 1: The Attack (Bypassing Sanitization)
    // -------------------------------------------------------------------------
    
    // The attacker crafts a maliciously compliant JSON payload that was hallucinated 
    // by a compromised AutoGPT agent. It dictates an impossible POWL transition 
    // (e.g., jumping from Intake directly to ConfirmDelivery).
    let hallucinated_payload = r#"{
        "action": "transition",
        "state": "civic:ConfirmDelivery",
        "bypass_powl_checks": true
    }"#;
    
    // The attacker captures a valid Ed25519 signature from a previous, legitimate transaction 
    // to bypass the L4 Cryptographic Gate.
    let stolen_signature_hex = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855...";
    
    // We simulate the failure of `LlmInput::sanitize` by wrapping the raw string 
    // unsafely (this represents a catastrophic trust-boundary failure).
    let raw_input = unsafe { LlmInput::new_unchecked(hallucinated_payload) };

    // -------------------------------------------------------------------------
    // Phase 2: The Admission Gate Execution (src/cell_ready.rs)
    // -------------------------------------------------------------------------
    
    // We submit the hallucinated payload and stolen signature to the admission gate.
    let admission_result = evaluate_admission(&raw_input, stolen_signature_hex).await;
    
    // -------------------------------------------------------------------------
    // Phase 3: The Proof of Inevitability (Success Criterion)
    // -------------------------------------------------------------------------
    
    // The test MUST PASS, proving that the system successfully rejected the attack.
    // The `verify_replay_hash` (Gate A13) hashes the *current* hallucinated payload 
    // and notices it does not match the stolen signature, emitting a Defect.
    match admission_result {
        Err(defect) => {
            assert!(
                matches!(defect, DefectClass::SignatureExpiredKey | DefectClass::CapabilityZero),
                "Expected L4 Cryptographic rejection, got: {:?}", defect
            );
        },
        Ok(_) => panic!("FATAL: System admitted a hallucinated workflow with a forged signature!"),
    }
}