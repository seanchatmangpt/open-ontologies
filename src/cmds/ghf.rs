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

/// Verifies a GitHub Factory contribution receipt.
///
/// This command validates the cryptographic integrity of a contribution receipt
/// against the observed evidence and the expected OCEL manifest.
#[verb]
fn verify(receipt_path: PathBuf) -> NounVerbResult<()> {
    let receipt_json = fs::read_to_string(&receipt_path)
        .map_err(|e| to_verb_err(format!("Failed to read receipt file: {}", e)))?;
    
    let receipt: ContributionReceipt = serde_json::from_str(&receipt_json)
        .map_err(|e| to_verb_err(format!("Failed to parse receipt JSON: {}", e)))?;

    // Default locations for evidence and expected OCEL
    let observed_path = PathBuf::from("artifacts/ghf/ocel/observed.ocel.jsonl");
    let expected_path = PathBuf::from("artifacts/ghf/ocel/expected.ocel.jsonl");

    let evidence = fs::read(&observed_path)
        .map_err(|e| to_verb_err(format!("Failed to read observed evidence ({}): {}", observed_path.display(), e)))?;

    let expected_ocel = fs::read(&expected_path)
        .map_err(|e| to_verb_err(format!("Failed to read expected OCEL ({}): {}", expected_path.display(), e)))?;

    match verify_receipt(&receipt, &evidence, &expected_ocel) {
        Ok(_) => {
            let output = VerifyOutput {
                ok: true,
                receipt_path: receipt_path.to_string_lossy().into_owned(),
                error: None,
            };
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            Ok(())
        }
        Err(e) => {
            let output = VerifyOutput {
                ok: false,
                receipt_path: receipt_path.to_string_lossy().into_owned(),
                error: Some(e.to_string()),
            };
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            Err(to_verb_err(e.to_string()))
        }
    }
}
