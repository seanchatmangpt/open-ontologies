// CLI subprocess tests adapted to noun-verb structure (Task B).
//
// Post-refactor invariants:
//   - All verbs are nested under a noun group (ontology, data, governance, alignment, ...)
//   - There is no global `--data-dir`; each verb takes `--data_dir <path>` (snake_case)
//   - Most inputs are named flags (`--input`, `--path`, `--label`, `--source`,
//     `--target`, `--sparql_query`, `--rule_id`, `--entity`, `--min_confidence`,
//     `--dry_run`, `--graph_name`, `--file`, `--file_a`, `--file_b`, `--pack`)
//   - `oo_isolated` returns a builder that appends `--data_dir <dir>` AFTER
//     the verb args, since `--data_dir` is verb-scoped post-refactor.

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

fn oo() -> Command {
    Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
}

/// Builder that produces a `Command` with `--data_dir <tmp>` appended AFTER
/// the verb arguments. Use `.verb([...])` to set the noun-verb prefix and
/// optionally `.flags([...])` for additional named flags.
struct Iso<'a> {
    dir: &'a Path,
    verb: Vec<OsString>,
    extra: Vec<OsString>,
}

impl<'a> Iso<'a> {
    fn new(dir: &'a tempfile::TempDir) -> Self {
        Self {
            dir: dir.path(),
            verb: Vec::new(),
            extra: Vec::new(),
        }
    }

    fn verb<I, S>(mut self, parts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        for p in parts {
            self.verb.push(p.as_ref().to_owned());
        }
        self
    }

    fn flags<I, S>(mut self, parts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        for p in parts {
            self.extra.push(p.as_ref().to_owned());
        }
        self
    }

    fn build(self) -> Command {
        let mut cmd = oo();
        cmd.args(&self.verb);
        cmd.args(&self.extra);
        cmd.arg("--data_dir").arg(self.dir);
        cmd
    }
}

#[test]
fn test_cli_help() {
    let out = oo().arg("--help").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Post-refactor top-level surface lists noun groups.
    assert!(stdout.contains("ontology"));
    assert!(stdout.contains("governance"));
    assert!(stdout.contains("data"));
}

#[test]
fn test_cli_validate_file() {
    let dir = tempfile::tempdir().unwrap();
    let ttl_path = dir.path().join("test.ttl");
    std::fs::write(
        &ttl_path,
        r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#,
    )
    .unwrap();

    let out = oo()
        .args(["ontology", "validate", "--input", ttl_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("triples"));
}

#[test]
fn test_cli_validate_stdin() {
    use std::io::Write;
    let mut child = oo()
        .args(["ontology", "validate", "--input", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"@prefix ex: <http://example.org/> . ex:Dog a <http://www.w3.org/2002/07/owl#Class> .")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_cli_stats_empty() {
    let dir = tempfile::tempdir().unwrap();
    let out = Iso::new(&dir).verb(["ontology", "stats"]).build().output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("classes"));
}

#[test]
fn test_cli_clear() {
    let dir = tempfile::tempdir().unwrap();
    let out = Iso::new(&dir).verb(["ontology", "clear"]).build().output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_cli_status() {
    let dir = tempfile::tempdir().unwrap();
    let out = Iso::new(&dir).verb(["ontology", "status"]).build().output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"));
}

// ─── Remote + versioning tests ────────────────────────────────────

#[test]
fn test_cli_history_empty() {
    let dir = tempfile::tempdir().unwrap();
    let out = Iso::new(&dir).verb(["ontology", "history"]).build().output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_cli_version_and_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let out = Iso::new(&dir)
        .verb(["ontology", "version"])
        .flags(["--label", "test-v1"])
        .build()
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ─── Data pipeline tests ─────────────────────────────────────────

#[test]
fn test_cli_reason_empty_store() {
    let dir = tempfile::tempdir().unwrap();
    let out = Iso::new(&dir)
        .verb(["ontology", "reason"])
        .flags(["--profile", "rdfs"])
        .build()
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("inferred") || stdout.contains("triples"));
}

#[test]
fn test_cli_ingest_csv() {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("data.csv");
    std::fs::write(&csv_path, "name,age\nAlice,30\nBob,25").unwrap();

    let out = Iso::new(&dir)
        .verb(["data", "ingest"])
        .flags(["--path", csv_path.to_str().unwrap()])
        .build()
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ─── Lifecycle + clinical tests ──────────────────────────────────

#[test]
fn test_cli_enforce_generic() {
    let dir = tempfile::tempdir().unwrap();
    let out = Iso::new(&dir)
        .verb(["governance", "enforce"])
        .flags(["--pack", "generic"])
        .build()
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("compliance") || stdout.contains("violations"));
}

#[test]
fn test_cli_plan() {
    let dir = tempfile::tempdir().unwrap();
    let ttl_path = dir.path().join("new.ttl");
    std::fs::write(
        &ttl_path,
        r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#,
    )
    .unwrap();

    let out = Iso::new(&dir)
        .verb(["governance", "plan"])
        .flags(["--file", ttl_path.to_str().unwrap()])
        .build()
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("risk_score") || stdout.contains("added"));
}

#[test]
fn test_cli_drift() {
    let dir = tempfile::tempdir().unwrap();
    let v1 = dir.path().join("v1.ttl");
    let v2 = dir.path().join("v2.ttl");
    std::fs::write(
        &v1,
        r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#,
    )
    .unwrap();
    std::fs::write(
        &v2,
        r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#,
    )
    .unwrap();

    let out = oo()
        .args([
            "governance",
            "drift",
            "--file_a",
            v1.to_str().unwrap(),
            "--file_b",
            v2.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("drift_velocity"));
}

#[test]
fn test_cli_lineage() {
    let dir = tempfile::tempdir().unwrap();
    let out = Iso::new(&dir).verb(["governance", "lineage"]).build().output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_cli_monitor_clear() {
    let dir = tempfile::tempdir().unwrap();
    // Subcommand is `monitor_clear` (snake_case post-refactor), not `monitor-clear`.
    let out = Iso::new(&dir).verb(["governance", "monitor_clear"]).build().output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_cli_align_two_files() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.ttl");
    let target = dir.path().join("target.ttl");

    std::fs::write(
        &source,
        r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class ; rdfs:label "Dog" .
        ex:Cat a owl:Class ; rdfs:label "Cat" .
    "#,
    )
    .unwrap();

    std::fs::write(
        &target,
        r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix other: <http://other.org/> .
        other:Dog a owl:Class ; rdfs:label "Dog" .
        other:Feline a owl:Class ; rdfs:label "Cat" .
    "#,
    )
    .unwrap();

    let out = Iso::new(&dir)
        .verb(["alignment", "align"])
        .flags([
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
            "--min_confidence",
            "0.5",
            "--dry_run",
            "true",
        ])
        .build()
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("candidates"));
    assert!(stdout.contains("confidence"));
}

#[test]
fn test_cli_align_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let out = Iso::new(&dir)
        .verb(["alignment", "align_feedback"])
        .flags([
            "--source",
            "http://ex.org/Dog",
            "--target",
            "http://other.org/Canine",
            "--accept",
            "true",
        ])
        .build()
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"));
}

#[test]
fn test_cli_lint_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let out = Iso::new(&dir)
        .verb(["alignment", "lint_feedback"])
        .flags([
            "--rule_id",
            "missing_label",
            "--entity",
            "<http://example.org/Dog>",
            "--dismiss",
            "true",
        ])
        .build()
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"));
    assert!(stdout.contains("lint"));
}

#[test]
fn test_cli_enforce_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let out = Iso::new(&dir)
        .verb(["alignment", "enforce_feedback"])
        .flags([
            "--rule_id",
            "orphan_class",
            "--entity",
            "<http://example.org/Thing>",
            "--accept",
            "true",
        ])
        .build()
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"));
    assert!(stdout.contains("enforce"));
}

#[test]
fn test_cli_lint_suppression_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let ttl_path = dir.path().join("test.ttl");
    std::fs::write(
        &ttl_path,
        r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#,
    )
    .unwrap();

    // Lint should report issues initially
    let out = Iso::new(&dir)
        .verb(["ontology", "lint"])
        .flags(["--input", ttl_path.to_str().unwrap()])
        .build()
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();

    // Find the entity string for missing_label on Dog
    let issues = v["issues"].as_array().unwrap();
    let dog_issue = issues.iter().find(|i| {
        i["type"].as_str().unwrap_or("") == "missing_label"
            && i["entity"].as_str().unwrap_or("").contains("example.org/Dog")
    });
    assert!(dog_issue.is_some(), "Should have missing_label for Dog");
    let entity_str = dog_issue.unwrap()["entity"].as_str().unwrap();

    // Dismiss 3 times using exact entity string from lint output
    for _ in 0..3 {
        let out = Iso::new(&dir)
            .verb(["alignment", "lint_feedback"])
            .flags([
                "--rule_id",
                "missing_label",
                "--entity",
                entity_str,
                "--dismiss",
                "true",
            ])
            .build()
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // Lint should now show suppressed_count > 0
    let out = Iso::new(&dir)
        .verb(["ontology", "lint"])
        .flags(["--input", ttl_path.to_str().unwrap()])
        .build()
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert!(
        v["suppressed_count"].as_u64().unwrap() > 0,
        "suppressed_count should be > 0 after 3 dismissals"
    );
}
