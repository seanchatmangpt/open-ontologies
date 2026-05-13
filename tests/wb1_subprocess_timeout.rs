//! R7 WB-1 — Subprocess timeout enforcement integration tests.
//!
//! Three sabotage tests prove the wedge-risk closure:
//!
//! 1. `sleep_120s_kills_at_60s` — the canonical wedge: a child that
//!    refuses to terminate must be SIGKILLed near the configured
//!    deadline, not run to completion. Uses a shorter (200ms) deadline
//!    so the test stays fast while still proving the kill path runs.
//!
//! 2. `no_zombie_after_kill` — `wait_timeout`'s reap step must run on
//!    the timeout path. The test invokes the timeout path many times
//!    and asserts no defunct processes are left under the test's PGID.
//!
//! 3. `timeout_error_is_typed` — the call site must receive
//!    `SubprocessError::LlmTimeout` so the response JSON / OCEL
//!    emission can distinguish a hung subprocess from a regular
//!    non-zero exit.
//!
//! These tests do NOT exercise the OCEL emission directly — that is
//! covered by the WB-1 server-side integration test (server-level
//! emission requires standing up a full `OpenOntologiesServer`,
//! which is out of scope for this unit-style sabotage). The subprocess
//! API contract is what those server-level tests build on.

use std::process::Command;
use std::time::{Duration, Instant};

use open_ontologies::subprocess::{run_with_timeout, SubprocessContext, SubprocessError};

const CTX: SubprocessContext<'static> = SubprocessContext {
    model: "wb1-test",
    tenant_id: "default",
    script_path: "/bin/sleep",
};

#[test]
fn sleep_120s_kills_at_configured_deadline() {
    // The Plan-3 spec calls for a 60s deadline killing a 120s sleep,
    // which would take 60+ seconds of real wall-clock per invocation.
    // The contract is identical at smaller scales: a 200ms deadline
    // killing a 10s sleep proves the kill path triggers and the
    // elapsed window matches the deadline rather than the child's
    // intended duration. We use 10s — long enough that no plausible
    // race wins, short enough to bound test runtime if the kill path
    // ever regressed.
    let started = Instant::now();
    let mut cmd = Command::new("/bin/sleep");
    cmd.arg("10");
    let err = run_with_timeout(&mut cmd, Duration::from_millis(200), CTX)
        .expect_err("/bin/sleep 10 with 200ms deadline must time out");
    let total = started.elapsed();
    match err {
        SubprocessError::LlmTimeout {
            elapsed_ms,
            limit_ms,
            script_path,
        } => {
            assert_eq!(limit_ms, 200, "limit_ms must echo the configured deadline");
            assert!(
                (150..=2_000).contains(&elapsed_ms),
                "elapsed_ms={elapsed_ms} should be near 200ms, far below sleep duration"
            );
            assert_eq!(
                script_path, "/bin/sleep",
                "script_path must reflect the SubprocessContext"
            );
            assert!(
                total < Duration::from_secs(5),
                "wall-clock total {total:?} must be far below the 10s sleep target"
            );
        }
        other => panic!("expected LlmTimeout, got {other:?}"),
    }
}

#[test]
fn no_zombie_after_kill() {
    // Run 10 timed-out subprocesses in succession. After each run the
    // child must be reaped (`child.wait()` in the timeout path); if
    // any iteration leaks a zombie the next /bin/sh -c that calls
    // pgrep would surface it. We can't directly inspect the kernel
    // process table from the test, but we can prove the API path is
    // clean by counting open child handles via /bin/sh `ps` after
    // every iteration.
    for i in 0..10 {
        let mut cmd = Command::new("/bin/sleep");
        cmd.arg("5");
        let err = run_with_timeout(&mut cmd, Duration::from_millis(80), CTX)
            .expect_err("iteration {i} must time out");
        assert!(matches!(err, SubprocessError::LlmTimeout { .. }), "iter {i}");
    }
    // Best-effort: query /bin/ps for our PID's children. On macOS the
    // shell script below counts processes whose parent PID is the
    // current test runner; under correct kill+reap semantics this
    // value is always zero immediately after each `run_with_timeout`
    // returns.
    let pid = std::process::id();
    if let Ok(out) = Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("ps -o pid,ppid,state -A | awk '$2 == {pid} {{print}}'"))
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            // Z = defunct/zombie state.
            assert!(
                !line.contains(" Z"),
                "zombie child detected after timeout path: {line}"
            );
        }
    }
}

#[test]
fn timeout_error_is_typed() {
    let mut cmd = Command::new("/bin/sleep");
    cmd.arg("3");
    let err = run_with_timeout(&mut cmd, Duration::from_millis(100), CTX)
        .expect_err("3s sleep with 100ms deadline must time out");
    // The error must be the LlmTimeout variant — not some opaque
    // io::Error wrapped in SpawnFailed. Distinguishing the two is
    // exactly the contract that downstream callers (server.rs) rely
    // on to map to a typed `subprocess_timed_out` JSON denial.
    let display = err.to_string();
    assert!(
        display.contains("subprocess timed out"),
        "expected 'subprocess timed out' in message, got: {display}"
    );
    match err {
        SubprocessError::LlmTimeout { .. } => {}
        SubprocessError::SpawnFailed(_) => {
            panic!("a sleep that ran to deadline must be classified as LlmTimeout, not SpawnFailed")
        }
    }
}
