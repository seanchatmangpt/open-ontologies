//! REAL Groq end-to-end test for the SolutionSpec proposer.
//!
//! Spawns scripts/spec_from_ctq.py against the chatmangpt/ostar venv,
//! makes an actual Groq API call with the real GROQ_API_KEY, and asserts
//! the proposer produces a shape-valid SolutionSpec.
//!
//! No mocks. No canned JSON. Real LLM, real network, real provider.
//!
//! Gating:
//!   - Requires GROQ_API_KEY in process env OR in ./.env.
//!   - Requires the chatmangpt/ostar venv at the canonical path.
//!   - When either is missing the test SKIPs with eprintln. It does NOT
//!     fail — that would brick CI for anyone without the local setup.
//!
//! Run with:
//!     cargo test --test real_groq_solution_spec -- --nocapture --test-threads=1

use std::process::Command;

const VENV_PYTHON: &str = "/Users/sac/chatmangpt/ostar/.venv/bin/python";

const FIXED_HASH: &str =
    "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn read_groq_key() -> Option<String> {
    // Prefer the project .env so the test runs against the pinned key
    // even when a stale GROQ_API_KEY is exported in the developer shell.
    let env_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
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

fn ctq_payload(hash: &str) -> String {
    serde_json::json!({
        "ctq_text": "Lead-to-cash latency under 30 days from MQL to closed-won",
        "measure_text": "days elapsed from MQL creation to opportunity close-won in Salesforce",
        "verification_text": "Salesforce stage-change audit log + revops weekly cohort report",
        "negative_case_text": "a Q3 enterprise deal that took 92 days due to extended legal review",
        "control_plan_text": "weekly funnel SLA review with sales ops + automated alerts at 21d",
        "work_order_receipt_hash": hash,
    })
    .to_string()
}

fn run_spec(ctq_json: &str, key: &str) -> serde_json::Value {
    let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/spec_from_ctq.py");
    let output = Command::new(VENV_PYTHON)
        .arg(&script)
        .arg(ctq_json)
        .env("GROQ_API_KEY", key)
        .output()
        .expect("spawn python subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        panic!(
            "spec_from_ctq.py exited with {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
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
fn real_groq_proposes_shape_valid_solution_spec() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };
    let result = run_spec(&ctq_payload(FIXED_HASH), &key);

    eprintln!("REAL GROQ SPEC: {result}");

    // Required top-level keys.
    for k in &["spec", "verdict", "refinements"] {
        assert!(
            result.get(*k).is_some(),
            "missing key `{k}` in response: {result}"
        );
    }
    assert_eq!(
        result["verdict"].as_bool(),
        Some(true),
        "verdict must be true; refinements={}, violations={}, spec={}",
        result["refinements"],
        result.get("violations").unwrap_or(&serde_json::Value::Null),
        result["spec"]
    );

    let spec = &result["spec"];
    // Every spec field is non-empty.
    let name = spec["name"].as_str().unwrap_or("");
    let description = spec["description"].as_str().unwrap_or("");
    let iac_target = spec["iac_target"].as_str().unwrap_or("");
    let region = spec["region"].as_str().unwrap_or("");
    let mcu_target = spec["mcu_target"].as_str().unwrap_or("");
    let supervisor_children = spec["supervisor_children"].as_i64().unwrap_or(0);
    let wor_hash = spec["work_order_receipt_hash"].as_str().unwrap_or("");

    assert!(!name.is_empty(), "name must be non-empty: {spec}");
    assert!(!description.is_empty(), "description must be non-empty: {spec}");
    assert_eq!(iac_target, "aws", "iac_target must be 'aws': {spec}");
    assert!(!region.is_empty(), "region must be non-empty: {spec}");
    assert!(
        matches!(mcu_target, "esp32" | "stm32" | "rp2040"),
        "mcu_target must be one of esp32/stm32/rp2040; got {mcu_target:?}"
    );
    assert!(
        (1..=64).contains(&supervisor_children),
        "supervisor_children must be in [1,64]; got {supervisor_children}"
    );
    // name shape check (mirror of validate_spec).
    let first = name.chars().next().expect("name non-empty");
    assert!(
        first.is_ascii_alphabetic() && first.is_ascii_lowercase(),
        "name must start with a lowercase letter: {name:?}"
    );
    assert!(
        name.chars().all(|c| c.is_ascii_lowercase()
            || c.is_ascii_digit()
            || c == '_'),
        "name must match [a-z][a-z0-9_]*: {name:?}"
    );
    assert_eq!(
        wor_hash, FIXED_HASH,
        "work_order_receipt_hash must be preserved verbatim"
    );
}

#[test]
fn real_groq_preserves_work_order_receipt_hash_verbatim() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };
    // Use a distinct hash so we know the script is forwarding it
    // rather than echoing whatever the LLM emitted (the LLM is
    // explicitly told NOT to emit this field).
    let unique_hash =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    assert_ne!(unique_hash, FIXED_HASH, "test hashes must differ");

    let result = run_spec(&ctq_payload(unique_hash), &key);
    eprintln!("REAL GROQ SPEC (hash-test): {result}");

    let spec = &result["spec"];
    let wor = spec["work_order_receipt_hash"].as_str().unwrap_or("");
    assert_eq!(
        wor, unique_hash,
        "work_order_receipt_hash must be preserved verbatim from input \
         regardless of LLM behavior; got {wor:?}, expected {unique_hash:?}"
    );
}
