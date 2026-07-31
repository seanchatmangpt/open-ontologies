use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn verifier() -> PathBuf {
    repo_root().join("tools/verify_ggen_standards.py")
}

fn run_contract(root: &Path) -> Output {
    Command::new("python3")
        .arg(verifier())
        .arg("--root")
        .arg(root)
        .arg("--contract-only")
        .arg("--output")
        .arg(root.join("ggen-standards-report.json"))
        .output()
        .expect("python3 must execute the ggen standards verifier")
}

fn copy_contract_fixture() -> TempDir {
    let source = repo_root();
    let fixture = TempDir::new().expect("temporary contract fixture");
    for relative in [
        "AGENTS.md",
        "Cargo.toml",
        ".chatmangpt/namespace.toml",
        "standards/ggen-v26.7.31.toml",
    ] {
        let from = source.join(relative);
        let to = fixture.path().join(relative);
        fs::create_dir_all(to.parent().expect("fixture parent"))
            .expect("create fixture directories");
        fs::copy(&from, &to).unwrap_or_else(|error| {
            panic!("copy {} into fixture: {error}", from.display())
        });
    }
    fixture
}

fn refusal_id(output: &Output) -> String {
    let document: Value = serde_json::from_slice(&output.stdout)
        .expect("verifier stdout must be a machine-readable JSON report");
    document["refusal"]["id"]
        .as_str()
        .expect("failed verification must carry a typed refusal id")
        .to_owned()
}

#[test]
fn admitted_repository_contract_is_a_positive_witness() {
    let output = run_contract(&repo_root());
    assert!(
        output.status.success(),
        "contract verifier refused the admitted repository:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value =
        serde_json::from_slice(&output.stdout).expect("positive report must be JSON");
    assert_eq!(document["bounded_standing"], "PARTIAL_ALIVE");
    assert_eq!(document["external_release_standing"], "UNKNOWN");
    assert_eq!(document["actuation_performed"], false);
}

#[test]
fn direct_actuation_authority_is_refused() {
    let fixture = copy_contract_fixture();
    let profile = fixture.path().join("standards/ggen-v26.7.31.toml");
    let text = fs::read_to_string(&profile).expect("read fixture profile");
    fs::write(
        &profile,
        text.replace("required_broker = \"BRCE\"", "required_broker = \"DIRECT\""),
    )
    .expect("mutate broker authority");

    let output = run_contract(fixture.path());
    assert!(!output.status.success(), "DIRECT authority must fail closed");
    assert_eq!(refusal_id(&output), "GGEN-STD-AUTHORITY-001");
}

#[test]
fn workstation_only_dependency_is_refused() {
    let fixture = copy_contract_fixture();
    let manifest = fixture.path().join("Cargo.toml");
    let mut text = fs::read_to_string(&manifest).expect("read fixture manifest");
    text.push_str("\n[dependencies.ggen_falsifier]\npath = \"/Users/example/private-crate\"\n");
    fs::write(&manifest, text).expect("mutate dependency boundary");

    let output = run_contract(fixture.path());
    assert!(
        !output.status.success(),
        "absolute workstation dependency must fail closed"
    );
    assert_eq!(refusal_id(&output), "GGEN-STD-PORTABILITY-001");
}
