//! R7 WB-1 — Subprocess timeout enforcement.
//!
//! Wraps `std::process::Command::output()` with a hard wall-clock
//! deadline using `wait_timeout`. Closes the active wedge risk
//! identified by R7 Explore-2: the default `groq_pm4py` engine shells
//! out to `scripts/*.py`, and a hung Groq HTTP request inside that
//! script wedged the Tokio worker indefinitely. Until WB-1 the
//! `[llm] subprocess_timeout_secs` config field was *dead* — wired
//! into the config struct but never read by any call site.
//!
//! Authority discipline (`coding-agent-mistakes.md` §6):
//!   * Deepens authority — every subprocess site must funnel through
//!     `run_with_timeout`. The companion AST gate
//!     [`tests/wb1_no_naked_subprocess.rs`] forbids `.output()` /
//!     `.wait_with_output()` outside this module.
//!   * Reduces drift — on timeout the caller emits an
//!     `llm_subprocess_timeout` OCEL event with `model`, `elapsed_ms`,
//!     `tenant_id`, `script_path` so retention-driven cost analysis
//!     can see hung subprocesses for what they are (silent budget burn)
//!     rather than discovering them via worker exhaustion.
//!
//! Failure mode classes blocked:
//!   1.5 (Contract drift) — receipt timing previously claimed
//!     completion in the success path even when the subprocess was
//!     still running. Hard timeout collapses the variance.
//!   1.3 (Fail-open) — `.output()` had no timeout: a hung child
//!     held the tokio worker forever and looked like work. Now
//!     reflective Err.

use std::io;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use thiserror::Error;
use wait_timeout::ChildExt;

/// Typed timeout error. Carries enough context for OCEL emission
/// without exposing PII (the script path is `env!("CARGO_MANIFEST_DIR")`-
/// derived; arguments are NOT included).
///
/// # Examples
///
/// ```
/// use open_ontologies::subprocess::SubprocessError;
///
/// // Construct an LlmTimeout variant and check its Display message.
/// let err = SubprocessError::LlmTimeout {
///     elapsed_ms: 5_200,
///     limit_ms: 5_000,
///     script_path: "/opt/scripts/groq_pm4py.py".into(),
/// };
/// let msg = err.to_string();
/// assert!(msg.contains("5200ms"), "elapsed must appear: {msg}");
/// assert!(msg.contains("5000ms"), "limit must appear: {msg}");
/// assert!(msg.contains("groq_pm4py.py"), "path must appear: {msg}");
/// ```
///
/// ```
/// use open_ontologies::subprocess::SubprocessError;
/// use std::io;
///
/// // Construct a SpawnFailed variant from a raw io::Error and verify Display.
/// let io_err = io::Error::new(io::ErrorKind::NotFound, "no such file or directory");
/// let err = SubprocessError::SpawnFailed(io_err);
/// let msg = err.to_string();
/// assert!(msg.contains("spawn failed"), "display must say spawn failed: {msg}");
/// assert!(msg.contains("no such file"), "display must include io::Error message: {msg}");
///
/// // Verify debug formatting is available (derived).
/// let timeout_err = SubprocessError::LlmTimeout {
///     elapsed_ms: 100,
///     limit_ms: 50,
///     script_path: "test.py".into(),
/// };
/// assert!(format!("{timeout_err:?}").contains("LlmTimeout"));
/// ```
#[derive(Debug, Error)]
pub enum SubprocessError {
    #[error("subprocess timed out after {elapsed_ms}ms (limit {limit_ms}ms): {script_path}")]
    LlmTimeout {
        elapsed_ms: u64,
        limit_ms: u64,
        script_path: String,
    },
    #[error("subprocess spawn failed: {0}")]
    SpawnFailed(#[from] io::Error),
}

/// Result of a successful timed run. Same shape as `std::process::Output`
/// but carries `elapsed_ms` so OCEL emitters don't have to re-time.
///
/// # Examples
///
/// ```
/// use open_ontologies::subprocess::TimedOutput;
/// use std::process::Output;
/// use std::os::unix::process::ExitStatusExt;
///
/// // Construct a TimedOutput from raw parts (hermetic — no subprocess needed).
/// let status = std::process::ExitStatus::from_raw(0);
/// let timed = TimedOutput {
///     output: Output { status, stdout: b"hello\n".to_vec(), stderr: vec![] },
///     elapsed_ms: 42,
/// };
/// assert!(timed.output.status.success());
/// assert_eq!(timed.elapsed_ms, 42);
/// assert_eq!(&timed.output.stdout, b"hello\n");
/// ```
///
/// ```
/// use open_ontologies::subprocess::TimedOutput;
/// use std::process::Output;
/// use std::os::unix::process::ExitStatusExt;
///
/// // Non-zero exit status: success() returns false, stderr is populated.
/// // On Unix, from_raw(256) encodes exit code 1 (upper byte is the exit code).
/// let status = std::process::ExitStatus::from_raw(256);
/// let timed = TimedOutput {
///     output: Output {
///         status,
///         stdout: vec![],
///         stderr: b"error: no input files\n".to_vec(),
///     },
///     elapsed_ms: 7,
/// };
/// assert!(!timed.output.status.success());
/// assert_eq!(timed.elapsed_ms, 7);
/// assert_eq!(&timed.output.stderr, b"error: no input files\n");
/// assert!(timed.output.stdout.is_empty());
///
/// // Debug formatting is available (derived).
/// let dbg = format!("{timed:?}");
/// assert!(dbg.contains("elapsed_ms"), "debug must include field name: {dbg}");
/// ```
#[derive(Debug)]
pub struct TimedOutput {
    pub output: Output,
    pub elapsed_ms: u64,
}

/// Identification metadata for OCEL emission and error formatting.
/// Pulled from the call site so per-tool / per-tenant attribution is
/// preserved when several handlers share this wrapper.
///
/// # Examples
///
/// ```
/// use open_ontologies::subprocess::SubprocessContext;
///
/// // Construct the context struct (pure data — hermetic).
/// let ctx = SubprocessContext {
///     model: "groq_pm4py",
///     tenant_id: "acme-corp",
///     script_path: "/opt/scripts/groq_pm4py.py",
/// };
/// assert_eq!(ctx.model, "groq_pm4py");
/// assert_eq!(ctx.tenant_id, "acme-corp");
///
/// // Copy semantics are derived — the struct is Copy.
/// let ctx2 = ctx;
/// assert_eq!(ctx2.script_path, ctx.script_path);
/// ```
///
/// ```
/// use open_ontologies::subprocess::SubprocessContext;
///
/// // Demonstrate Clone: both copies are independent (though values point to
/// // the same string literals since lifetime is 'static here).
/// let model_name = String::from("powl_from_text");
/// let tenant = String::from("beta-corp");
/// let script = String::from("/scripts/powl.py");
/// let ctx = SubprocessContext {
///     model: &model_name,
///     tenant_id: &tenant,
///     script_path: &script,
/// };
/// let ctx_clone = ctx; // Copy
/// assert_eq!(ctx.model, ctx_clone.model);
/// assert_eq!(ctx.tenant_id, ctx_clone.tenant_id);
/// assert_eq!(ctx.script_path, ctx_clone.script_path);
///
/// // Debug formatting is available (derived).
/// let dbg = format!("{ctx:?}");
/// assert!(dbg.contains("powl_from_text"), "debug must include model: {dbg}");
/// assert!(dbg.contains("beta-corp"), "debug must include tenant_id: {dbg}");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SubprocessContext<'a> {
    /// Logical tool / engine model string (e.g. `"groq_pm4py"`,
    /// `"powl_from_text"`). Stored as the OCEL `model` attribute on
    /// timeout.
    pub model: &'a str,
    /// Tenant ID at the call site. Stored as `tenant_id`.
    pub tenant_id: &'a str,
    /// Path to the script being invoked, for diagnostics. Stored as
    /// `script_path`.
    pub script_path: &'a str,
}

/// Run `cmd` with a hard wall-clock timeout.
///
/// On success: returns `TimedOutput` carrying the child's stdout/stderr
/// and the elapsed milliseconds.
///
/// On timeout: SIGKILLs the child, reaps it, drains any pending pipes
/// (best-effort — the child may already be gone) and returns
/// `SubprocessError::LlmTimeout`. The caller is expected to emit an
/// OCEL `llm_subprocess_timeout` event.
///
/// On spawn / IO failure: returns `SubprocessError::SpawnFailed`.
///
/// The function does NOT itself emit OCEL — emission is the caller's
/// responsibility because the OCEL store + tenant context live on the
/// server struct. Returning a typed error keeps this module
/// dependency-free for unit testing.
///
/// # Examples
///
/// ```no_run
/// use open_ontologies::subprocess::{run_with_timeout, SubprocessContext};
/// use std::process::Command;
/// use std::time::Duration;
///
/// let ctx = SubprocessContext {
///     model: "groq_pm4py",
///     tenant_id: "default",
///     script_path: "/bin/echo",
/// };
/// let mut cmd = Command::new("/bin/echo");
/// cmd.arg("hello");
/// let result = run_with_timeout(&mut cmd, Duration::from_secs(5), ctx)
///     .expect("echo should succeed within 5 s");
/// assert!(result.output.status.success());
/// assert!(result.elapsed_ms < 5_000);
/// ```
pub fn run_with_timeout(
    cmd: &mut Command,
    dur: Duration,
    _ctx: SubprocessContext<'_>,
) -> Result<TimedOutput, SubprocessError> {
    // Pipe stdout/stderr so we can drain after kill.
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let started = Instant::now();
    let mut child = cmd.spawn().map_err(SubprocessError::SpawnFailed)?;

    match child.wait_timeout(dur).map_err(SubprocessError::SpawnFailed)? {
        Some(status) => {
            // Child finished within the deadline. Drain stdout/stderr
            // explicitly because we set them to piped above.
            let elapsed_ms = started.elapsed().as_millis() as u64;
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut s) = child.stdout.take() {
                let _ = io::Read::read_to_end(&mut s, &mut stdout);
            }
            if let Some(mut s) = child.stderr.take() {
                let _ = io::Read::read_to_end(&mut s, &mut stderr);
            }
            Ok(TimedOutput {
                output: Output { status, stdout, stderr },
                elapsed_ms,
            })
        }
        None => {
            // Deadline exceeded. SIGKILL and drain.
            let _ = child.kill();
            // Reap so we don't leave a zombie. Ignore the status — we've
            // already classified the run as timed-out.
            let _ = child.wait();
            // Best-effort drain: the kill may have produced partial
            // output. Ignore errors.
            if let Some(mut s) = child.stdout.take() {
                let mut buf = Vec::new();
                let _ = io::Read::read_to_end(&mut s, &mut buf);
            }
            if let Some(mut s) = child.stderr.take() {
                let mut buf = Vec::new();
                let _ = io::Read::read_to_end(&mut s, &mut buf);
            }
            let elapsed_ms = started.elapsed().as_millis() as u64;
            Err(SubprocessError::LlmTimeout {
                elapsed_ms,
                limit_ms: dur.as_millis() as u64,
                script_path: _ctx.script_path.to_string(),
            })
        }
    }
}

/// Variant of [`run_with_timeout`] that writes `stdin_payload` to the
/// child's stdin before waiting. Used by the `ontostar_planner.py`
/// site in `src/server.rs` which feeds a JSON payload over stdin
/// rather than CLI args. Mirrors the same SIGKILL-on-timeout semantics.
///
/// # Examples
///
/// ```no_run
/// use open_ontologies::subprocess::{run_with_timeout_stdin, SubprocessContext};
/// use std::process::Command;
/// use std::time::Duration;
///
/// let ctx = SubprocessContext {
///     model: "ontostar_planner",
///     tenant_id: "default",
///     script_path: "/usr/bin/cat",
/// };
/// let payload = br#"{"query": "list classes"}"#;
/// let mut cmd = Command::new("/usr/bin/cat");
/// let result = run_with_timeout_stdin(&mut cmd, payload, Duration::from_secs(5), ctx)
///     .expect("cat should echo stdin within 5 s");
/// assert!(result.output.status.success());
/// assert_eq!(&result.output.stdout, payload);
/// ```
pub fn run_with_timeout_stdin(
    cmd: &mut Command,
    stdin_payload: &[u8],
    dur: Duration,
    _ctx: SubprocessContext<'_>,
) -> Result<TimedOutput, SubprocessError> {
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let started = Instant::now();
    let mut child = cmd.spawn().map_err(SubprocessError::SpawnFailed)?;

    // Write the stdin payload, then drop the handle so the child sees EOF.
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(stdin_payload);
        // drop stdin closes the pipe
    }

    match child.wait_timeout(dur).map_err(SubprocessError::SpawnFailed)? {
        Some(status) => {
            let elapsed_ms = started.elapsed().as_millis() as u64;
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut s) = child.stdout.take() {
                let _ = io::Read::read_to_end(&mut s, &mut stdout);
            }
            if let Some(mut s) = child.stderr.take() {
                let _ = io::Read::read_to_end(&mut s, &mut stderr);
            }
            Ok(TimedOutput {
                output: Output { status, stdout, stderr },
                elapsed_ms,
            })
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            if let Some(mut s) = child.stdout.take() {
                let mut buf = Vec::new();
                let _ = io::Read::read_to_end(&mut s, &mut buf);
            }
            if let Some(mut s) = child.stderr.take() {
                let mut buf = Vec::new();
                let _ = io::Read::read_to_end(&mut s, &mut buf);
            }
            let elapsed_ms = started.elapsed().as_millis() as u64;
            Err(SubprocessError::LlmTimeout {
                elapsed_ms,
                limit_ms: dur.as_millis() as u64,
                script_path: _ctx.script_path.to_string(),
            })
        }
    }
}

/// Convenience helper for the OCEL emit attrs vector. Returns four
/// `(&str, &str)` borrow pairs so the caller can build a slice from
/// local references.
///
/// # Examples
///
/// ```
/// use open_ontologies::subprocess::timeout_ocel_attrs;
///
/// let elapsed = "5200";
/// let attrs = timeout_ocel_attrs(
///     "groq_pm4py",
///     elapsed,
///     "acme-corp",
///     "/opt/scripts/groq_pm4py.py",
/// );
///
/// assert_eq!(attrs[0], ("model", "groq_pm4py"));
/// assert_eq!(attrs[1], ("elapsed_ms", "5200"));
/// assert_eq!(attrs[2], ("tenant_id", "acme-corp"));
/// assert_eq!(attrs[3], ("script_path", "/opt/scripts/groq_pm4py.py"));
/// assert_eq!(attrs.len(), 4);
/// ```
///
/// ```
/// use open_ontologies::subprocess::timeout_ocel_attrs;
///
/// // Keys are stable across all calls — the array is always
/// // ["model", "elapsed_ms", "tenant_id", "script_path"] in that order.
/// let attrs = timeout_ocel_attrs("m", "0", "t", "s");
/// let keys: Vec<&str> = attrs.iter().map(|(k, _)| *k).collect();
/// assert_eq!(keys, &["model", "elapsed_ms", "tenant_id", "script_path"]);
///
/// // Values round-trip correctly for all positions.
/// let attrs2 = timeout_ocel_attrs("my-model", "12345", "my-tenant", "/my/script.py");
/// assert_eq!(attrs2[0].1, "my-model");
/// assert_eq!(attrs2[1].1, "12345");
/// assert_eq!(attrs2[2].1, "my-tenant");
/// assert_eq!(attrs2[3].1, "/my/script.py");
/// ```
pub fn timeout_ocel_attrs<'a>(
    model: &'a str,
    elapsed_ms_str: &'a str,
    tenant_id: &'a str,
    script_path: &'a str,
) -> [(&'a str, &'a str); 4] {
    [
        ("model", model),
        ("elapsed_ms", elapsed_ms_str),
        ("tenant_id", tenant_id),
        ("script_path", script_path),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn fast_command_returns_within_window() {
        let mut cmd = Command::new("/bin/echo");
        cmd.arg("hello");
        let result = run_with_timeout(
            &mut cmd,
            Duration::from_secs(5),
            SubprocessContext {
                model: "test",
                tenant_id: "default",
                script_path: "/bin/echo",
            },
        )
        .expect("echo should succeed");
        assert!(result.output.status.success());
        assert!(String::from_utf8_lossy(&result.output.stdout).contains("hello"));
        assert!(result.elapsed_ms < 5_000);
    }

    #[test]
    fn slow_command_times_out_with_typed_error() {
        let mut cmd = Command::new("/bin/sleep");
        cmd.arg("10");
        let err = run_with_timeout(
            &mut cmd,
            Duration::from_millis(200),
            SubprocessContext {
                model: "test",
                tenant_id: "default",
                script_path: "/bin/sleep",
            },
        )
        .expect_err("sleep 10 must time out at 200ms");
        match err {
            SubprocessError::LlmTimeout { elapsed_ms, limit_ms, .. } => {
                assert!(elapsed_ms >= 150, "elapsed_ms={elapsed_ms}");
                assert!(elapsed_ms < 5_000, "elapsed_ms={elapsed_ms} should be near 200, not 10000");
                assert_eq!(limit_ms, 200);
            }
            other => panic!("expected LlmTimeout, got {other:?}"),
        }
    }

    #[test]
    fn spawn_failure_returns_typed_error() {
        let mut cmd = Command::new("/this/binary/does/not/exist/__nope__");
        let err = run_with_timeout(
            &mut cmd,
            Duration::from_secs(1),
            SubprocessContext {
                model: "test",
                tenant_id: "default",
                script_path: "/missing",
            },
        )
        .expect_err("nonexistent binary must fail spawn");
        match err {
            SubprocessError::SpawnFailed(_) => {}
            other => panic!("expected SpawnFailed, got {other:?}"),
        }
    }
}
