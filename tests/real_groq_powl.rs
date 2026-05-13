//! REAL Groq LLM end-to-end test.
//!
//! Spawns scripts/powl_from_text.py against the chatmangpt/pm4py fork
//! and the chatmangpt/ostar venv, makes an actual Groq API call with
//! the real GROQ_API_KEY, and asserts the JSON response carries a
//! non-empty `powl` field plus the expected sibling keys.
//!
//! No mocks. No tokio HTTP listener. No canned JSON. This is the test
//! the user demanded: real LLM call, real network, real provider.
//!
//! Gating:
//!   - Requires GROQ_API_KEY in the process environment OR in
//!     ./.env (loaded best-effort here so `cargo test` works without
//!     a shell export).
//!   - Requires the chatmangpt/ostar venv at the canonical path.
//!   - Requires the chatmangpt/pm4py fork at the canonical path.
//!   - When any of these are missing the test SKIPs with eprintln.
//!     It does NOT fail — that would brick CI for anyone without
//!     the local setup. Local development boxes run the real call.

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

fn run_powl(description: &str, key: &str) -> serde_json::Value {
    let script =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/powl_from_text.py");
    let output = Command::new(VENV_PYTHON)
        .arg(&script)
        .arg(description)
        .env("GROQ_API_KEY", key)
        .env("PM4PY_FORK_PATH", PM4PY_FORK)
        .output()
        .expect("spawn python subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        panic!(
            "powl_from_text.py exited with {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status.code()
        );
    }
    // Last line of stdout is the JSON object. DSPy / pm4py log warning
    // lines to stderr (or sometimes stdout); we extract the trailing
    // JSON line.
    let json_line = stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or_else(|| panic!(
            "no JSON line in stdout:\nstdout:\n{stdout}\nstderr:\n{stderr}"
        ));
    serde_json::from_str(json_line.trim()).unwrap_or_else(|e| {
        panic!(
            "JSON parse failed: {e}\nline: {json_line}\nstderr: {stderr}"
        )
    })
}

#[test]
fn real_groq_call_produces_powl_for_simple_sequence() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };
    // Description tightly matches a few-shot demo so the LLM is most
    // likely to emit valid POWL syntax. Tests that the integration
    // works end-to-end against the real Groq endpoint.
    let result = run_powl("Submit expense report, then manager reviews it", &key);

    // Required keys present. The pm4py contract:
    for key in &["powl", "verdict", "reasoning", "refinements"] {
        assert!(
            result.get(*key).is_some(),
            "missing key `{key}` in pm4py response: {result}"
        );
    }
    // POWL field must be a non-empty string.
    let powl = result["powl"].as_str().unwrap_or("");
    assert!(
        !powl.is_empty(),
        "real Groq call returned empty `powl` field: {result}"
    );
    // The description maps to a SEQ pattern in the demos. Allow either
    // verdict — the assertion is that REAL output came back, not that
    // the LLM happened to validate. We still print verdict for visibility.
    eprintln!(
        "REAL GROQ POWL: {} (verdict={}, refinements={})",
        powl, result["verdict"], result["refinements"]
    );
    // Receipts pinned to the specific behaviour we just observed in
    // the live run that prompted this test:
    //   - The model produces a string containing one of POWL's
    //     canonical operators OR the SEQ token (which the validator
    //     accepts under the demo).
    let has_powl_token = powl.contains("SEQ")
        || powl.contains("->")
        || powl.contains("X (")
        || powl.contains("X(")
        || powl.contains("PO=")
        || powl.contains("* (")
        || powl.contains("+ (")
        || powl.contains("<>");
    assert!(
        has_powl_token,
        "Groq returned a string with no recognizable POWL token: {powl}"
    );
}

#[test]
fn real_groq_call_returns_typed_failure_on_empty_input() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };
    // Empty description should be rejected by the script's input
    // validation BEFORE any Groq call is made — this proves the
    // contract surface and keeps API quota off the floor for
    // pathological inputs.
    let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/powl_from_text.py");
    let output = Command::new(VENV_PYTHON)
        .arg(&script)
        .arg("")
        .env("GROQ_API_KEY", &key)
        .env("PM4PY_FORK_PATH", PM4PY_FORK)
        .output()
        .expect("spawn python");
    assert!(!output.status.success(), "empty input should fail");
    assert_eq!(output.status.code(), Some(2), "expected exit 2 for usage error");
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("empty process description") || err.contains("usage:"),
        "expected typed usage error, got: {err}"
    );
}
