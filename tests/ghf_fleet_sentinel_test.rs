use std::fs;
use std::path::PathBuf;
use std::process::Command;
use serde_json::Value;

// Integration test for the GHF Fleet Sentinel closed loop
#[test]
fn test_fleet_sentinel_detects_missing_security_md_then_passes_after_ggen_remediation() {
    let security_md_path = PathBuf::from("SECURITY.md");
    let sentinel_script = PathBuf::from("scripts/ghf/fleet_sentinel.py");
    let fleet_receipt_path = PathBuf::from("artifacts/ghf/fleet/fleet-health.receipt.json");

    // 1. Setup: Ensure SECURITY.md is missing to trigger drift
    let backup_path = PathBuf::from("SECURITY.md.backup");
    if security_md_path.exists() {
        fs::rename(&security_md_path, &backup_path).unwrap();
    }

    // 2. Scan 1: Verify missing SECURITY.md produces FleetDriftDetected
    let status = Command::new("python3")
        .arg(&sentinel_script)
        .status()
        .expect("Failed to run sentinel script");
    assert!(status.success());

    let receipt_json = fs::read_to_string(&fleet_receipt_path).unwrap();
    let refusal_receipt: Value = serde_json::from_str(&receipt_json).unwrap();
    
    assert_eq!(refusal_receipt["receipt_type"], "OutOfMembraneReceipt");
    assert_eq!(refusal_receipt["refusal_state"], "FleetDriftDetected");
    assert!(refusal_receipt["receipt_hash"].as_str().is_some());
    let missing_items = refusal_receipt["missing"].as_array().unwrap();
    assert!(missing_items.iter().any(|v| v.as_str().unwrap() == "SECURITY.md is missing"));

    // 3. Remediation: ggen emits SECURITY.md from policy
    let ggen_status = Command::new("ggen")
        .arg("sync")
        .arg("--manifest")
        .arg("ggen.toml")
        .status()
        .expect("Failed to run ggen sync");
    assert!(ggen_status.success());
    assert!(security_md_path.exists(), "ggen failed to emit SECURITY.md");

    // 4. Scan 2: Re-scan produces FleetHealthReceipt
    let status2 = Command::new("python3")
        .arg(&sentinel_script)
        .status()
        .expect("Failed to run sentinel script");
    assert!(status2.success());

    let health_json = fs::read_to_string(&fleet_receipt_path).unwrap();
    let health_receipt: Value = serde_json::from_str(&health_json).unwrap();
    
    assert_eq!(health_receipt["receipt_type"], "FleetHealthReceipt");
    assert_eq!(health_receipt["DriftRiskScore"].as_f64().unwrap(), 0.0);
    assert_eq!(health_receipt["TopologyConformanceScore"].as_f64().unwrap(), 1.0);
    assert!(health_receipt["receipt_hash"].as_str().is_some());
    assert_ne!(
        refusal_receipt["receipt_hash"].as_str().unwrap(),
        health_receipt["receipt_hash"].as_str().unwrap(),
        "Hashes must be distinct between refusal and health receipts"
    );

    // 5. Teardown: Restore backup if we moved it
    if backup_path.exists() {
        fs::rename(&backup_path, &security_md_path).unwrap();
    }
}
