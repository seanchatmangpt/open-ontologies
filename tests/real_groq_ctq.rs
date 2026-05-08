//! REAL Groq LLM end-to-end test for the CTQ-Forge.
//!
//! Spawns scripts/ctq_from_voice.py against the chatmangpt/ostar venv,
//! makes an actual Groq API call with the real GROQ_API_KEY, and asserts
//! the JSON response carries a non-empty CTQ structure plus `verdict=true`
//! for the canonical demo-shaped input.
//!
//! No mocks. No tokio HTTP listener. No canned JSON. Real LLM call,
//! real network, real provider.
//!
//! Gating mirrors tests/real_groq_powl.rs: missing venv / missing key
//! triggers a SKIP via eprintln rather than a hard failure, so CI without
//! the local setup does not redden.

use std::process::Command;

const VENV_PYTHON: &str = "/Users/sac/chatmangpt/ostar/.venv/bin/python";

fn read_groq_key() -> Option<String> {
    // Prefer the project-pinned .env file over a possibly-stale shell env,
    // mirroring the user's directive: "real key from .env". The shell var
    // is only used as a fallback when no .env entry is present.
    let env_path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    if let Ok(content) = std::fs::read_to_string(&env_path) {
        for line in content.lines() {
            if let Some(rest) = line.trim().strip_prefix("GROQ_API_KEY=") {
                let v = rest.trim_matches('"').trim_matches('\'').trim();
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    if let Ok(v) = std::env::var("GROQ_API_KEY") {
        if !v.trim().is_empty() {
            return Some(v);
        }
    }
    None
}

fn skip_unless_available() -> Option<String> {
    if !std::path::Path::new(VENV_PYTHON).exists() {
        eprintln!("SKIP: venv python not at {VENV_PYTHON}");
        return None;
    }
    let key = read_groq_key()?;
    if key.is_empty() {
        eprintln!("SKIP: GROQ_API_KEY not set in env or .env");
        return None;
    }
    Some(key)
}

fn run_ctq(voice: &str, key: &str) -> serde_json::Value {
    let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/ctq_from_voice.py");
    let output = Command::new(VENV_PYTHON)
        .arg(&script)
        .arg(voice)
        .env("GROQ_API_KEY", key)
        .output()
        .expect("spawn python subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        panic!(
            "ctq_from_voice.py exited with {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status.code()
        );
    }
    let json_line = stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or_else(|| {
            panic!("no JSON line in stdout:\nstdout:\n{stdout}\nstderr:\n{stderr}")
        });
    serde_json::from_str(json_line.trim()).unwrap_or_else(|e| {
        panic!("JSON parse failed: {e}\nline: {json_line}\nstderr: {stderr}")
    })
}

#[test]
fn real_groq_call_produces_admissible_ctq_for_demo_voice() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };
    // The voice tightly matches the first few-shot demo, so the real
    // Groq call should converge on verdict=true within the refine budget.
    let voice = "Sales says deals are real, Finance can't reconcile bookings";
    let result = run_ctq(voice, &key);

    for k in &[
        "source_voice_echo",
        "ctq_text",
        "measure_text",
        "verification_text",
        "negative_case_text",
        "control_plan_text",
        "defect_class_hint",
        "verdict",
        "refinements",
    ] {
        assert!(
            result.get(*k).is_some(),
            "missing key `{k}` in CTQ response: {result}"
        );
    }

    // source_voice_echo must be a faithful echo (script-side, not LLM).
    assert_eq!(
        result["source_voice_echo"].as_str().unwrap_or(""),
        voice,
        "source_voice_echo must verbatim-echo input"
    );

    // All five mandatory descriptive fields must be non-empty (the
    // deterministic admission gate denies otherwise).
    for k in &[
        "ctq_text",
        "measure_text",
        "verification_text",
        "negative_case_text",
        "control_plan_text",
    ] {
        let s = result[*k].as_str().unwrap_or("");
        assert!(
            !s.trim().is_empty(),
            "field `{k}` must be non-empty for admission: {result}"
        );
    }

    // The min-len constraints from src/signature_shape.rs::ctq_signature.
    assert!(result["ctq_text"].as_str().unwrap_or("").len() >= 20);
    assert!(result["measure_text"].as_str().unwrap_or("").len() >= 8);
    assert!(result["verification_text"].as_str().unwrap_or("").len() >= 8);
    assert!(result["negative_case_text"].as_str().unwrap_or("").len() >= 12);
    assert!(result["control_plan_text"].as_str().unwrap_or("").len() >= 12);

    // Real-Groq receipt: this exact demo-shaped input must yield verdict=true.
    let verdict = result["verdict"].as_bool().unwrap_or(false);
    eprintln!(
        "REAL GROQ CTQ: verdict={} refinements={} ctq_text={:?}",
        verdict,
        result["refinements"],
        result["ctq_text"].as_str().unwrap_or("")
    );
    assert!(
        verdict,
        "real Groq call should validate the demo-shaped CTQ as verdict=true: {result}"
    );
}

#[test]
fn real_groq_call_returns_typed_failure_on_empty_input() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };
    // Empty source_voice must be rejected before any Groq call is made.
    let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/ctq_from_voice.py");
    let output = Command::new(VENV_PYTHON)
        .arg(&script)
        .arg("")
        .env("GROQ_API_KEY", &key)
        .output()
        .expect("spawn python");
    assert!(!output.status.success(), "empty input should fail");
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit 2 for usage error"
    );
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("empty source_voice") || err.contains("usage:"),
        "expected typed usage error, got: {err}"
    );
}
