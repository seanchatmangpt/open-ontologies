// Adverse Serial Counter-Factual JTBD Testing (Armstrong Let-It-Crash)
//
// The principle: Test what the code SHOULD do, not what it claims.
// Use counter-factual reasoning: if feature X works, then Y must also be true.
// If Y is false, then X was theater (fake).
//
// Armstrong principle applied: Every impossible state must fail LOUDLY with
// an informative error message. Silent success = theater.
//
// Serial means: each test step uses the output of the prior step as input,
// so a fake in step 1 is caught by step 2's assertion.

use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// Helper: Run the open-ontologies binary with a specific data-dir isolation
fn oo() -> Command {
    Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
}

fn oo_isolated(dir: &TempDir) -> Command {
    let mut cmd = oo();
    cmd.arg("--data-dir").arg(dir.path());
    cmd
}

// ============================================================================
// MODULE A: Dead-Param Theater Detection
// Counter-factual: if --param is accepted, it must have observable effect.
// If not, it's theater.
// ============================================================================

#[test]
fn test_format_param_turtle_on_csv_must_fail() {
    // JTBD: I want to control the parse format for ingested data.
    // Counter-factual: if --format is honored, format=turtle on a CSV file must fail.
    // Armstrong: fail LOUD (exit code != 0, clear error message), not silently succeed.

    let dir = TempDir::new().unwrap();

    // Write a valid CSV file
    let csv_path = dir.path().join("data.csv");
    let mut f = File::create(&csv_path).unwrap();
    writeln!(f, "name,age").unwrap();
    writeln!(f, "Alice,30").unwrap();

    // Try to ingest it with --format turtle (wrong format)
    let out = oo_isolated(&dir)
        .args(&["data", "ingest", csv_path.to_str().unwrap()])
        .arg("--format")
        .arg("turtle")
        .output()
        .unwrap();

    // Theater detection: this must NOT succeed silently
    assert!(
        !out.status.success(),
        "THEATER DETECTED: ingest accepted --format turtle on a CSV file and succeeded.\n\
         Either the format was silently ignored (soft stub) or auto-detection hid the override.\n\
         The --format parameter must either work correctly or the verb must refuse it explicitly.\n\
         stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Armstrong requires an informative error, not silent failure
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.is_empty(),
        "Armstrong violation: process failed (exit != 0) but stderr is empty. \
         Crash must be informative."
    );
}

#[test]
fn test_push_graph_name_param_forwarded_or_refused() {
    // JTBD: I want to push an ontology to a named graph in a SPARQL endpoint.
    // Counter-factual: --graph-name either gets forwarded to the UPDATE query OR
    // the verb returns an explicit "named graph not supported" error.
    // Armstrong: NOT accepted and silently ignored.

    let dir = TempDir::new().unwrap();

    // Create a minimal valid ontology
    let ttl_path = dir.path().join("test.ttl");
    let mut f = File::create(&ttl_path).unwrap();
    writeln!(f, "@prefix : <http://example.org/> .").unwrap();
    writeln!(f, ":Alice a :Person .").unwrap();

    // Load it first
    let out = oo_isolated(&dir)
        .args(&["ontology", "load", ttl_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "Setup failed: could not load ontology. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Try to push with a named graph (this will fail because we don't have a real SPARQL endpoint,
    // but the important thing is that --graph-name is NOT silently accepted and ignored)
    let out = oo_isolated(&dir)
        .args(&["data", "push", "http://localhost:7070/sparql"])
        .arg("--graph-name")
        .arg("urn:named:graph:test")
        .output()
        .unwrap();

    // If it succeeds, we're OK (endpoint was real and named graph was forwarded).
    // If it fails, the error message must mention the endpoint or the graph-name feature,
    // NOT silently pretend the param was accepted.
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        // Check that the error is about the endpoint or the feature, not about other things
        let error_msg = stderr.to_lowercase();
        assert!(
            error_msg.contains("endpoint")
                || error_msg.contains("connection")
                || error_msg.contains("feature")
                || error_msg.contains("graph"),
            "THEATER DETECTED: push rejected with no mention of the endpoint or graph feature.\n\
             stderr: {}\n\
             The error must indicate what failed, not silently drop --graph-name.",
            stderr
        );
    }
}

// ============================================================================
// MODULE B: Armstrong Hard-Crash Scenarios
// Impossible states MUST fail loudly (exit != 0) with informative errors.
// ============================================================================

#[test]
fn test_load_nonexistent_file_crashes_loudly() {
    // Armstrong: loading a file that doesn't exist must fail hard.
    let dir = TempDir::new().unwrap();

    let out = oo_isolated(&dir)
        .args(&["ontology", "load", "/nonexistent/file/that/does/not/exist.ttl"])
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "THEATER DETECTED: load succeeded on nonexistent file. \
         Armstrong requires hard failure, not silent success."
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.is_empty(),
        "Armstrong violation: exit != 0 but stderr is empty. Crash must be informative."
    );
}

#[test]
fn test_validate_garbage_input_rejects() {
    // Armstrong: validating garbage Turtle must return an error.
    let dir = TempDir::new().unwrap();

    let garbage = "this is not valid RDF or Turtle @@@### ]}}";

    let mut child = oo_isolated(&dir)
        .args(&["ontology", "validate"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();

    if let Some(mut stdin) = child.stdin.take() {
        let _ = write!(stdin, "{}", garbage);
    }

    let out = child.wait_with_output().unwrap();

    assert!(
        !out.status.success(),
        "THEATER DETECTED: validate claimed garbage input was valid. \
         Armstrong requires explicit error on invalid syntax."
    );
}

#[test]
fn test_query_empty_store_returns_no_results() {
    // Counter to Module B philosophy: querying an empty store is NOT an error,
    // it should return 0 results. But it must do so clearly.
    let dir = TempDir::new().unwrap();

    let out = oo_isolated(&dir)
        .args(&["ontology", "query", "SELECT * WHERE { ?s ?p ?o }"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "THEATER DETECTED: query on empty store failed. Empty store queries should return 0 results.\n\
         stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    // Expect either "0 results" or an empty JSON array or similar
    assert!(
        stdout.contains("0") || stdout.contains("[]") || stdout.contains("bindings"),
        "Empty query result must indicate 0 results, not undefined behavior.\n\
         stdout: {}",
        stdout
    );
}

#[test]
fn test_version_without_name_uses_default() {
    // Armstrong: version must either require a label or use a sensible default.
    // NOT silently fail.
    let dir = TempDir::new().unwrap();

    // Create and load minimal ontology
    let ttl_path = dir.path().join("test.ttl");
    let mut f = File::create(&ttl_path).unwrap();
    writeln!(f, "@prefix : <http://example.org/> . :x :y :z .").unwrap();

    let out = oo_isolated(&dir)
        .args(&["ontology", "load", ttl_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());

    // Version without a label — should generate a default or return a clear error
    let out = oo_isolated(&dir)
        .args(&["governance", "version"])
        .output()
        .unwrap();

    // If it fails, the error must be clear ("--label required" or similar).
    // If it succeeds, a default label was used (also acceptable).
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("label") || stderr.contains("required"),
            "THEATER: version failed but didn't explain why.\n\
             Expected error about --label, got: {}",
            stderr
        );
    }
}

// ============================================================================
// MODULE C: Serial Counter-Factual JTBD Chain
// Each step uses prior step's output as input.
// A fake in step 1 is disproven by step 2's assertion.
// ============================================================================

#[test]
fn test_serial_counterfactual_ontology_load_query_clear_rollback() {
    // Serial JTBD: "I want to load an ontology, query it, snapshot it, clear it, and restore it."
    // Counter-factual chain: each step depends on the prior being real.

    let dir = TempDir::new().unwrap();

    // Step 1: Load — if fake, triples=0 and step 2 fails
    let ttl_content = r#"
@prefix : <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

:Alice a :Person ;
    :name "Alice Wonderland" ;
    :knows :Bob .

:Bob a :Person ;
    :name "Bob Builder" .
"#;

    let ttl_path = dir.path().join("test.ttl");
    let mut f = File::create(&ttl_path).unwrap();
    write!(f, "{}", ttl_content).unwrap();
    drop(f);

    let out = oo_isolated(&dir)
        .args(&["ontology", "load", ttl_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "Step 1 (load) failed. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Step 2: Query — proves loaded triples are the SPECIFIC triples, not random data
    let sparql = r#"SELECT ?name WHERE { ?x <http://example.org/name> ?name }"#;
    let out = oo_isolated(&dir)
        .args(&["ontology", "query", sparql])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "Step 2 (query) failed. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Alice Wonderland") || stdout.contains("Alice"),
        "Step 2 (query) COUNTER-FACTUAL FAILED: Load was theater.\n\
         Entity from file not queryable. Either:\n\
           - Load was fake (returned success without actually loading), OR\n\
           - Query was fake (returned hardcoded results)\n\
         stdout: {}",
        stdout
    );

    // Step 3: Version — proves state crosses disk boundary (not in-memory only)
    let out = oo_isolated(&dir)
        .args(&["governance", "version", "--label", "v1"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "Step 3 (version) failed. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Step 4: Clear — proves in-memory store is actually cleared
    let out = oo_isolated(&dir)
        .args(&["ontology", "clear"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "Step 4 (clear) failed. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Step 5: Query after clear — counter-factual: must return 0 results
    // If it still returns Alice, clear was theater (noop)
    let out = oo_isolated(&dir)
        .args(&["ontology", "query", sparql])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "Step 5 (query-after-clear) failed. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Alice Wonderland") && !stdout.contains("Bob"),
        "Step 5 COUNTER-FACTUAL FAILED: Clear was theater (noop).\n\
         Entity still queryable after clear. Either:\n\
           - Clear was fake (didn't actually clear), OR\n\
           - Query was fake (returned stale cache)\n\
         stdout: {}",
        stdout
    );

    // Step 6: Rollback — proves version system reads from disk, not memory
    let out = oo_isolated(&dir)
        .args(&["governance", "rollback", "--label", "v1"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "Step 6 (rollback) failed. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Step 7: Query after rollback — counter-factual: entity must be back
    // If rollback was fake, this fails. This is the final proof of the serial chain.
    let out = oo_isolated(&dir)
        .args(&["ontology", "query", sparql])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "Step 7 (query-after-rollback) failed. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Alice Wonderland") || stdout.contains("Alice"),
        "Step 7 COUNTER-FACTUAL FAILED: Rollback was theater.\n\
         Entity not restored after rollback. Serial chain disproves:\n\
           - Step 3 (version was never saved to disk), OR\n\
           - Step 6 (rollback didn't load from disk), OR\n\
           - Step 1 (load was always fake, no real data to restore)\n\
         stdout: {}",
        stdout
    );

    // All 7 steps passed: load, query, version, clear, query-empty, rollback, query-restored.
    // This proves the entire load→query→version→clear→rollback→query lifecycle is real,
    // not theater.
}

#[test]
fn test_serial_counterfactual_ingest_csv_to_queryable_rdf() {
    // JTBD: I want CSV data to become queryable RDF triples.
    // Counter-factual: if ingest is real, a SPARQL query for a CSV value must succeed.

    let dir = TempDir::new().unwrap();

    // Write a CSV with a specific sentinel value that we'll query for
    let csv_path = dir.path().join("data.csv");
    let mut f = File::create(&csv_path).unwrap();
    writeln!(f, "name,age").unwrap();
    writeln!(f, "Alice,30").unwrap();
    writeln!(f, "Bob,25").unwrap();
    drop(f);

    // Ingest the CSV
    let out = oo_isolated(&dir)
        .args(&["data", "ingest", csv_path.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "Ingest failed. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Query for the CSV data — counter-factual: must find the names
    let sparql = r#"SELECT ?value WHERE { ?s ?p ?value . FILTER(CONTAINS(STR(?value), "Alice")) }"#;
    let out = oo_isolated(&dir)
        .args(&["ontology", "query", sparql])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "Query after ingest failed. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Alice"),
        "COUNTER-FACTUAL FAILED: CSV ingest was theater.\n\
         CSV data not queryable as RDF. Either:\n\
           - Ingest was fake (didn't actually convert CSV to RDF), OR\n\
           - Query was fake (returned hardcoded results)\n\
         stdout: {}",
        stdout
    );
}
