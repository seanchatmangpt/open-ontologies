//! REAL Groq LLM end-to-end test for `onto_executive_projection`.
//!
//! Spawns scripts/executive_projection.py against the chatmangpt/ostar venv
//! and the real Groq endpoint, using the GROQ_API_KEY in the environment or
//! .env file. No mocks, no canned JSON, no HTTP fake.
//!
//! Gating:
//!   - Requires GROQ_API_KEY in the process environment OR in
//!     ./.env (best-effort load).
//!   - Requires the chatmangpt/ostar venv at the canonical path.
//!   - When any of these are missing the test SKIPs with eprintln.
//!     It does NOT fail — that would brick CI for anyone without
//!     the local setup.
//!
//! Run serially:
//!     cargo test --test real_groq_executive_projection -- --test-threads=1

use std::process::Command;

const VENV_PYTHON: &str = "/Users/sac/chatmangpt/ostar/.venv/bin/python";

fn read_groq_key() -> Option<String> {
    // Prefer the project .env so the test runs against the pinned key
    // even when a stale GROQ_API_KEY is exported in the developer's shell.
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

fn run_projection(
    evidence: &str,
    key: &str,
    extra_env: &[(&str, &str)],
) -> serde_json::Value {
    let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/executive_projection.py");
    let mut cmd = Command::new(VENV_PYTHON);
    cmd.arg(&script)
        .arg(evidence)
        .env("GROQ_API_KEY", key);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("spawn python subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        panic!(
            "executive_projection.py exited with {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status.code()
        );
    }
    let json_line = stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or_else(|| {
            panic!(
                "no JSON line in stdout:\nstdout:\n{stdout}\nstderr:\n{stderr}"
            )
        });
    serde_json::from_str(json_line.trim()).unwrap_or_else(|e| {
        panic!(
            "JSON parse failed: {e}\nline: {json_line}\nstderr: {stderr}"
        )
    })
}

#[test]
fn real_groq_projection_grounded_in_admitted_evidence() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };
    let evidence = "Reconciliation completeness rate is 83%. Forecast risk \
                    explainable. Nightly report ran. Refuse missing contract. \
                    Block partial chain.";
    let result = run_projection(evidence, &key, &[]);

    for k in &[
        "summary",
        "tokens_used",
        "tokens_invented",
        "verdict",
        "refinements",
    ] {
        assert!(
            result.get(*k).is_some(),
            "missing key `{k}` in projection response: {result}"
        );
    }

    let summary = result["summary"].as_str().unwrap_or("");
    assert!(
        !summary.is_empty(),
        "real Groq call returned empty `summary` field: {result}"
    );

    let verdict = result["verdict"].as_bool().unwrap_or(false);
    let invented = result["tokens_invented"].as_array().cloned().unwrap_or_default();
    eprintln!(
        "REAL GROQ EXEC PROJECTION:\n  summary: {summary}\n  verdict: {verdict}\n  refinements: {}\n  tokens_used: {}\n  tokens_invented: {}",
        result["refinements"], result["tokens_used"], result["tokens_invented"]
    );

    assert!(
        verdict,
        "expected verdict=true after up to 2 refinements; invented={invented:?}; summary={summary}"
    );
    assert!(
        invented.is_empty(),
        "verdict was true but tokens_invented non-empty: {invented:?}"
    );

    // The summary must contain at least one substantive evidence token, so
    // we know the LLM actually grounded in the input rather than emitting
    // boilerplate.
    let summary_lc = summary.to_lowercase();
    let candidates = [
        "reconciliation",
        "forecast",
        "nightly",
        "report",
        "contract",
        "chain",
        "explainable",
        "partial",
        "missing",
        "refuse",
        "block",
    ];
    let hit = candidates.iter().any(|c| summary_lc.contains(c));
    assert!(
        hit,
        "summary does not contain any substantive evidence token: {summary}"
    );
}

#[test]
fn real_groq_projection_audits_invented_tokens() {
    let key = match skip_unless_available() {
        Some(k) => k,
        None => return,
    };
    // Evidence so generic the LLM is likely to invent. Disable refinement
    // so the audit verdict reflects the raw first pass. The contract under
    // test is the audit pipeline itself, not LLM cooperation: if the LLM
    // happens to stay grounded, verdict=true is also acceptable as long as
    // tokens_invented is consistent (empty iff verdict=true).
    let evidence = "alpha beta gamma delta";
    let result = run_projection(
        evidence,
        &key,
        &[("POWL_MAX_REFINEMENTS", "0")],
    );

    let summary = result["summary"].as_str().unwrap_or("");
    let verdict = result["verdict"].as_bool().unwrap_or(false);
    let invented = result["tokens_invented"].as_array().cloned().unwrap_or_default();
    let used = result["tokens_used"].as_array().cloned().unwrap_or_default();
    let refinements = result["refinements"].as_i64().unwrap_or(-1);

    eprintln!(
        "REAL GROQ EXEC PROJECTION (audit):\n  summary: {summary}\n  verdict: {verdict}\n  refinements: {refinements}\n  tokens_used: {used:?}\n  tokens_invented: {invented:?}"
    );

    assert!(!summary.is_empty(), "summary must be non-empty: {result}");
    assert_eq!(
        refinements, 0,
        "POWL_MAX_REFINEMENTS=0 must short-circuit the refine loop"
    );
    // Audit consistency invariant: verdict=true iff invented is empty.
    assert_eq!(
        verdict,
        invented.is_empty(),
        "verdict/invented inconsistent: verdict={verdict} invented={invented:?}"
    );
    // We expect the LLM to invent for such generic evidence; if so, verify
    // negative-path semantics. If the LLM stayed grounded, the consistency
    // invariant above is the contract being asserted.
    if !verdict {
        assert!(
            !invented.is_empty(),
            "verdict=false must imply non-empty tokens_invented"
        );
    }
}
