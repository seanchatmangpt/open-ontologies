//! REAL Groq LLM end-to-end test for the POWL *refiner*.
//!
//! Spawns scripts/powl_refine.py against the chatmangpt/pm4py fork and
//! the chatmangpt/ostar venv, makes an actual Groq API call with the
//! real GROQ_API_KEY, and asserts the JSON response carries a
//! refined_powl that has been tightened relative to the input POWL.
//!
//! No mocks. No tokio listener. No canned JSON. Real provider only.
//!
//! Gating mirrors tests/real_groq_powl.rs:
//!   - Requires GROQ_API_KEY in env or in ./.env
//!   - Requires the chatmangpt/ostar venv at the canonical path
//!   - Requires the chatmangpt/pm4py fork at the canonical path
//!   - When any are missing, the test SKIPs with eprintln (does not fail).

use std::process::Command;

const VENV_PYTHON: &str = "/Users/sac/chatmangpt/ostar/.venv/bin/python";
const PM4PY_FORK: &str = "/Users/sac/chatmangpt/pm4py";

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
    if !std::path::Path::new(PM4PY_FORK).exists() {
        eprintln!("SKIP: pm4py fork not at {PM4PY_FORK}");
        return None;
    }
    let key = read_groq_key()?;
    if key.is_empty() {
        eprintln!("SKIP: GROQ_API_KEY not set in env or .env");
        return None;
    }
    Some(key)
}

fn script_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/powl_refine.py")
}

fn run_refine(
    original_powl: &str,
    description: &str,
    issues: &str,
    key: &str,
) -> serde_json::Value {
    let output = Command::new(VENV_PYTHON)
        .arg(script_path())
        .arg("--original-powl")
        .arg(original_powl)
        .arg("--description")
        .arg(description)
        .arg("--issues")
        .arg(issues)
        .env("GROQ_API_KEY", key)
        .env("PM4PY_FORK_PATH", PM4PY_FORK)
        .output()
        .expect("spawn python subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        panic!(
            "powl_refine.py exited with {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
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
fn real_groq_refines_xor_into_sequence_when_description_demands_it() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };

    // Original POWL says XOR (exclusive choice between the two steps),
    // but the natural-language description is clearly a sequence.
    // The refiner should rewrite this into a SEQ / -> form.
    let original = "X (Submit, Manager review)";
    let description = "Submit expense report, then manager reviews it";
    let issues = "should be sequential not exclusive choice";

    let result = run_refine(original, description, issues, &key);

    // Required keys present.
    for k in &[
        "original_powl",
        "refined_powl",
        "changed",
        "reasoning",
        "refinements",
    ] {
        assert!(
            result.get(*k).is_some(),
            "missing key `{k}` in refine response: {result}"
        );
    }

    let refined = result["refined_powl"].as_str().unwrap_or("");
    let original_out = result["original_powl"].as_str().unwrap_or("");
    let changed = result["changed"].as_bool().unwrap_or(false);
    let refinements = result["refinements"].as_i64().unwrap_or(0);

    assert_eq!(original_out, original, "original_powl should be echoed back");
    assert!(!refined.is_empty(), "refined_powl must not be empty");
    assert!(refinements >= 1, "expected at least one refine attempt");

    eprintln!(
        "REAL GROQ POWL REFINE:\n  original: {original}\n  refined:  {refined}\n  changed:  {changed}\n  refinements: {refinements}\n  reasoning: {}",
        result["reasoning"]
    );

    // Tightening assertion: the refined POWL must contain a
    // sequential-flavoured token. The original used X(...) for XOR;
    // we expect SEQ or -> to appear in the output.
    let has_seq_token = refined.contains("->") || refined.contains("SEQ");
    assert!(
        has_seq_token,
        "expected refined POWL to contain a sequential token (-> or SEQ), got: {refined}"
    );

    // The refiner exists to *change* the model. Identical output here
    // means no real refinement happened.
    assert!(
        changed,
        "refined POWL should differ from original; got original={original} refined={refined}"
    );
}

#[test]
fn real_groq_refine_rejects_empty_original_powl() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };

    // Empty original_powl must be rejected by the script's input
    // validation BEFORE any Groq call happens — keeps quota off the
    // floor for pathological inputs.
    let output = Command::new(VENV_PYTHON)
        .arg(script_path())
        .arg("--original-powl")
        .arg("")
        .arg("--description")
        .arg("Submit expense report, then manager reviews it")
        .arg("--issues")
        .arg("anything")
        .env("GROQ_API_KEY", &key)
        .env("PM4PY_FORK_PATH", PM4PY_FORK)
        .output()
        .expect("spawn python");

    assert!(!output.status.success(), "empty original_powl should fail");
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit 2 for usage error"
    );
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("empty original_powl"),
        "expected typed usage error, got: {err}"
    );
}
