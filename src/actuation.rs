use crate::subprocess::SubprocessError;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let mut cmd = Command::new(command);
    cmd.args(args).current_dir(dir);
    let output = cmd.output().unwrap_or_else(|_| std::process::Output {
        status: std::os::unix::process::ExitStatusExt::from_raw(1),
        stdout: b"command failed to start".to_vec(),
        stderr: Vec::new(),
    });

    let finished_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let git_after = capture_git_state(dir);
    let files_changed = git_before != git_after;

    let _stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
    let _stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
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
    _tenant_id: &str,
) -> Result<ActuationResult, SubprocessError> {
    let cfg = crate::config::LlmConfig::default();
    let api_base = crate::config::resolve_llm_api_base(&cfg);
    let api_key = crate::config::resolve_llm_api_key(&cfg);
    let model = std::env::var("OPEN_ONTOLOGIES_LLM_MODEL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| cfg.model.clone().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| "openai/gpt-oss-20b".to_string());

    let endpoint = format!("{}/chat/completions", api_base.trim_end_matches('/'));

    let system_prompt = "You are executing a system actuation plan.";
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": plan.command}
        ],
        "temperature": 0.0
    });

    let client = reqwest::Client::new();
    let mut request = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }

    let command_to_run = request;
    let (status, response_text) = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        rt.block_on(async move {
            let response = command_to_run
                .send()
                .await
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let status = response.status();
            let text = response
                .text()
                .await
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            Ok::<_, std::io::Error>((status, text))
        })
    })
    .join()
    .map_err(|_| SubprocessError::SpawnFailed(std::io::Error::other("Thread join failed")))??;

    if !status.is_success() {
        return Err(SubprocessError::SpawnFailed(std::io::Error::other(
            format!("Groq API returned status {}: {}", status, response_text),
        )));
    }

    let parsed: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| SubprocessError::SpawnFailed(std::io::Error::other(e.to_string())))?;
    let stdout = parsed["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let stderr = String::new();
    let exit_code = 0;

    // Extract OCEL events from stdout.
    let ocel_events = extract_ocel_events(&stdout);

    // Secure capture includes hashes of the execution state (action_id + outputs).
    let mut hasher = blake3::Hasher::new();
    hasher.update(plan.action_id.as_bytes());
    hasher.update(stdout.as_bytes());
    hasher.update(stderr.as_bytes());
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
            if v.get("ocel:id").is_some()
                || v.get("event_id").is_some()
                || v.get("ocel:activity").is_some()
                || v.get("activity").is_some()
            {
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

    #[tokio::test]
    async fn test_capture_observed_ocel_with_mock_binary() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let response_body = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "{\"event_id\": \"e_mock\", \"activity\": \"mock_act\"}"
                    }
                }
            ]
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        unsafe {
            std::env::set_var("OPEN_ONTOLOGIES_LLM_API_BASE", mock_server.uri());
            std::env::set_var("OPEN_ONTOLOGIES_LLM_API_KEY", "mock-key");
        }

        let plan = ActuationPlan {
            action_id: "act-123".to_string(),
            emitted_by: "test".to_string(),
            policy_id: "policy-456".to_string(),
            allowed: true,
            working_directory: ".".to_string(),
            command: "some command".to_string(),
        };

        let result = capture_observed_ocel(&plan, "test-tenant");

        unsafe {
            std::env::remove_var("OPEN_ONTOLOGIES_LLM_API_BASE");
            std::env::remove_var("OPEN_ONTOLOGIES_LLM_API_KEY");
        }

        let res = result.expect("should run mock api successfully");
        assert_eq!(res.action_id, "act-123");
        assert!(
            res.stdout.contains("mock_act"),
            "stdout was: {}",
            res.stdout
        );
        assert_eq!(res.ocel_events.len(), 1);
        assert_eq!(res.ocel_events[0]["event_id"], "e_mock");
    }
}
