use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::process::Command;
use crate::subprocess::{run_with_timeout, SubprocessContext, SubprocessError};

/// Gemini CLI Actuation Plan. Governs the execution of an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuationPlan {
    pub action_id: String,
    pub emitted_by: String,
    pub policy_id: String,
    pub allowed: bool,
    pub working_directory: String,
    pub command: String,
}

/// Result of an actuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuationResult {
    pub action_id: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub ocel_events: Vec<serde_json::Value>,
    pub execution_hash: String,
}

pub fn capture_git_state(dir: &str) -> String {
    let output = Command::new("git")
        .arg("status")
        .arg("--short")
        .current_dir(dir)
        .output()
        .unwrap_or_else(|_| std::process::Output {
            status: std::os::unix::process::ExitStatusExt::from_raw(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        });
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn run_real_boundary(command: &str, args: &[&str], dir: &str) -> serde_json::Value {
    let git_before = capture_git_state(dir);
    
    let started_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    
    let mut cmd = Command::new(command);
    cmd.args(args).current_dir(dir);
    let output = cmd.output().unwrap_or_else(|_| std::process::Output {
        status: std::os::unix::process::ExitStatusExt::from_raw(1),
        stdout: b"command failed to start".to_vec(),
        stderr: Vec::new(),
    });
    
    let finished_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    
    let git_after = capture_git_state(dir);
    let files_changed = git_before != git_after;
    
    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    
    let stdout_hash = blake3::hash(&output.stdout).to_hex().to_string();
    let stderr_hash = blake3::hash(&output.stderr).to_hex().to_string();
    
    let mut execution_hasher = blake3::Hasher::new();
    execution_hasher.update(command.as_bytes());
    execution_hasher.update(&output.stdout);
    execution_hasher.update(&output.stderr);
    let execution_receipt_hash = execution_hasher.finalize().to_hex().to_string();

    serde_json::json!({
        "ocel:activity": "execute_boundary",
        "command": format!("{} {}", command, args.join(" ")),
        "working_directory": dir,
        "stdout_hash": stdout_hash,
        "stderr_hash": stderr_hash,
        "exit_code": exit_code,
        "started_at": started_at,
        "finished_at": finished_at,
        "git_before": git_before,
        "git_after": git_after,
        "files_changed": files_changed,
        "execution_receipt_hash": execution_receipt_hash,
        "boundary_type": "shell",
        "actor_basis": "system",
        "policy_epoch": "latest",
        "proof_hash": execution_receipt_hash
    })
}

/// Capture the observed OCEL by executing the actuation plan.
pub fn capture_observed_ocel(
    plan: &ActuationPlan,
    tenant_id: &str,
) -> Result<ActuationResult, SubprocessError> {
    let mut cmd = Command::new("npx");
    cmd.args(&[
        "-y",
        "@google/gemini-cli",
        "-p",
        &plan.command,
        "--approval-mode",
        "yolo",
    ]);
    cmd.current_dir(&plan.working_directory);

    let ctx = SubprocessContext {
        model: "gemini-cli",
        tenant_id,
        script_path: "npx @google/gemini-cli",
    };

    // Use a hard 10-minute timeout for actuation.
    let timed_output = run_with_timeout(&mut cmd, Duration::from_secs(600), ctx)?;
    
    let stdout = String::from_utf8_lossy(&timed_output.output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&timed_output.output.stderr).to_string();
    let exit_code = timed_output.output.status.code().unwrap_or(-1);

    // Extract OCEL events from stdout. 
    // Gemini CLI might emit OCEL events as JSON lines.
    let ocel_events = extract_ocel_events(&stdout);

    // Secure capture includes hashes of the execution state (action_id + outputs).
    let mut hasher = blake3::Hasher::new();
    hasher.update(plan.action_id.as_bytes());
    hasher.update(&timed_output.output.stdout);
    hasher.update(&timed_output.output.stderr);
    let execution_hash = hasher.finalize().to_hex().to_string();

    Ok(ActuationResult {
        action_id: plan.action_id.clone(),
        stdout,
        stderr,
        exit_code,
        ocel_events,
        execution_hash,
    })
}

fn extract_ocel_events(stdout: &str) -> Vec<serde_json::Value> {
    let mut events = Vec::new();
    for line in stdout.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            // Check if it's an OCEL event (has event_id or ocel:id)
            if v.get("ocel:id").is_some() || v.get("event_id").is_some() || v.get("ocel:activity").is_some() || v.get("activity").is_some() {
                events.push(v);
            }
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ocel_events() {
        let stdout = r#"
Some random text
{"event_id": "e1", "activity": "act1"}
More random text
{"ocel:id": "e2", "ocel:activity": "act2"}
{"not_ocel": "true"}
"#;
        let events = extract_ocel_events(stdout);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["event_id"], "e1");
        assert_eq!(events[1]["ocel:id"], "e2");
    }
}
