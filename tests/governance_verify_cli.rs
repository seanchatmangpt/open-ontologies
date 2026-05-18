//! T3-1 Governance verify CLI subprocess tests.
//!
//! Tests the `governance verify` CLI subcommand for artifact receipt chain verification.
//! Uses the same subprocess spawn pattern as existing CLI tests.

use std::process::Command;

fn oo() -> Command {
    Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
}

#[test]
fn governance_verify_with_help_flag() {
    let output = oo()
        .args(&["governance", "verify", "--help"])
        .output()
        .expect("spawn governance verify --help");

    assert!(output.status.success(), "governance verify --help should succeed");

    let help_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        help_text.contains("verify") || help_text.contains("path"),
        "help output should mention verify or path"
    );
}

#[test]
fn governance_verify_rejects_nonexistent_path() {
    let output = oo()
        .args(&["governance", "verify", "--path", "/tmp/nonexistent_artifact_xyz.bin"])
        .output()
        .expect("spawn governance verify with nonexistent path");

    assert!(
        !output.status.success(),
        "governance verify should fail for nonexistent artifact"
    );
}

#[test]
fn governance_verify_with_invalid_args() {
    let output = oo()
        .args(&["governance", "verify"])
        .output()
        .expect("spawn governance verify with no path");

    assert!(
        !output.status.success(),
        "governance verify should fail without --path"
    );
}
