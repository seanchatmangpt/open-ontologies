use std::process::Command;

fn cicd_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cargo-cicd"))
}

#[test]
fn test_cargo_cicd_help() {
    let out = cicd_bin().arg("--help").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("verify"));
    assert!(stdout.contains("doctor"));
    assert!(stdout.contains("explain"));
    assert!(stdout.contains("fix"));
    assert!(stdout.contains("release"));
    assert!(stdout.contains("task"));
}

#[test]
fn test_cargo_cicd_doctor() {
    let out = cicd_bin().arg("doctor").output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok") || stdout.contains("message"));
}

#[test]
fn test_cargo_cicd_verify() {
    let out = cicd_bin().arg("verify").output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("state") || stdout.contains("message"));
}

#[test]
fn test_cargo_cicd_explain() {
    let out = cicd_bin().args(["explain", "--rule_id", "DirtyTreeUnclassified"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("DirtyTreeUnclassified"));
    assert!(stdout.contains("unclassified"));
    assert!(stdout.contains("GEMINI.md"));
}

#[test]
fn test_cargo_cicd_fix() {
    let out = cicd_bin().args(["fix", "--rule_id", "VersionMismatch"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("actions_taken"));
    assert!(stdout.contains("Validated version consistency"));
}

#[test]
fn test_cargo_cicd_release() {
    let out = cicd_bin().args(["release", "--version", "26.7.3"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("standing"));
    assert!(stdout.contains("receipt_hash"));
    // Since git status is dirty during development / test run, standing should be REFUSED
    assert!(stdout.contains("REFUSED"));
}

#[test]
fn test_cargo_cicd_task() {
    let out = cicd_bin().arg("task").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("tasks"));
    // Git is dirty during tests, so task list should contain the stash/commit suggestion
    assert!(stdout.contains("Commit or stash local modifications"));
}

