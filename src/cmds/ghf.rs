//! GitHub Factory Commands

use clap_noun_verb::Result as NounVerbResult;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::path::PathBuf;
use std::fs;

use super::helpers::to_verb_err;
use open_ontologies::ghf::{ContributionReceipt, verify_receipt};

#[derive(Serialize)]
pub struct VerifyOutput {
    pub ok: bool,
    pub receipt_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Verifies a GitHub Factory artifact.
///
/// This command validates the cryptographic integrity and policy conformance of GHF artifacts.
#[verb]
fn verify(target_type: String, target: Option<String>) -> NounVerbResult<()> {
    if target_type == "receipt" {
        let target_val = target.ok_or_else(|| to_verb_err("Missing receipt path".to_string()))?;
        let receipt_path = PathBuf::from(&target_val);
        let receipt_json = fs::read_to_string(&receipt_path)
            .map_err(|e| to_verb_err(format!("Failed to read receipt file: {}", e)))?;
        
        let receipt: ContributionReceipt = serde_json::from_str(&receipt_json)
            .map_err(|e| to_verb_err(format!("Failed to parse receipt JSON: {}", e)))?;

        // Default locations for evidence and expected OCEL
        let observed_path = PathBuf::from("artifacts/ghf/ocel/observed.ocel.jsonl");
        let expected_path = PathBuf::from("artifacts/ghf/ocel/expected.ocel.jsonl");

        let evidence = fs::read(&observed_path).unwrap_or_default();
        let expected_ocel = fs::read(&expected_path).unwrap_or_default();

        let result = verify_receipt(&receipt, &evidence, &expected_ocel);
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
        
        if result.state == open_ontologies::ghf::VerificationState::Admitted {
            Ok(())
        } else {
            Err(to_verb_err("Verification failed or incomplete".to_string()))
        }
    } else if target_type == "fleet" {
        // Fleet Sentinel mode
        let status = std::process::Command::new("python3")
            .arg("scripts/ghf/fleet_sentinel.py")
            .status()
            .map_err(|e| to_verb_err(format!("Failed to run fleet sentinel: {}", e)))?;
            
        if !status.success() {
            return Err(to_verb_err("Fleet Sentinel execution failed".to_string()));
        }

        let receipt_json = fs::read_to_string("artifacts/ghf/fleet/fleet-health.receipt.json")
            .unwrap_or_else(|_| "{}".to_string());
        
        println!("Fleet Sentinel Report:");
        println!("{}", receipt_json);

        Ok(())
    } else {
        Err(to_verb_err(format!("Unsupported target type: {}", target_type)))
    }
}
