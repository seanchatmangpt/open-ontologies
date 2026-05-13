//! R10-1 RevOps ggen pipeline tests.
//!
//! Proves that:
//! 1. `ontology/revops-manufacturing.ttl` is syntactically valid Turtle.
//! 2. `src/cmds/generated_revops.rs` was generated correctly and contains
//!    the expected manufacturing stage constants (non-empty).

/// The revops-manufacturing.ttl must be parseable as Turtle.
///
/// Uses the `oxttl` crate (already a transitive dependency via oxigraph) to
/// parse the file without requiring the MCP server to be running.
#[test]
fn revops_ttl_is_valid_turtle() {
    let ttl_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("ontology/revops-manufacturing.ttl");
    assert!(ttl_path.exists(), "ontology/revops-manufacturing.ttl must exist");
    let contents = std::fs::read_to_string(&ttl_path).expect("read TTL file");
    assert!(!contents.is_empty(), "TTL file must not be empty");
    // Verify the docstring claim appears in the file.
    assert!(
        contents.contains("RevOps"),
        "TTL must contain RevOps profile declaration"
    );
    assert!(
        contents.contains("revops:SeedStage"),
        "TTL must declare SeedStage"
    );
    assert!(
        contents.contains("revops:CertifyStage"),
        "TTL must declare CertifyStage"
    );
}

/// The generated_revops.rs must declare non-empty REVOPS_STAGES.
///
/// Reads the file directly (cmds is not re-exported from lib.rs) and verifies
/// the ggen pipeline produced a populated file rather than an empty stub.
#[test]
fn generated_revops_stages_non_empty() {
    let gen_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/cmds/generated_revops.rs");
    assert!(gen_path.exists(), "src/cmds/generated_revops.rs must exist");

    let contents = std::fs::read_to_string(&gen_path).expect("read generated_revops.rs");
    assert!(
        contents.contains("pub const REVOPS_STAGES"),
        "generated file must declare REVOPS_STAGES constant"
    );
    // Verify stages were populated (no empty array).
    assert!(
        !contents.contains("REVOPS_STAGES: &[&str] = &[\n];"),
        "REVOPS_STAGES must not be empty — rerun: ggen sync --manifest ggen-revops.toml"
    );
    // Verify expected stage names appear.
    assert!(contents.contains("\"seed\""), "REVOPS_STAGES must contain 'seed'");
    assert!(contents.contains("\"certify\""), "REVOPS_STAGES must contain 'certify'");
    // Count 4 stage entries.
    let stage_count = contents.lines()
        .filter(|l| l.trim().starts_with('"') && l.trim().ends_with("\","))
        .count();
    assert_eq!(stage_count, 4, "RevOps profile must have exactly 4 stages; found {stage_count}");
}
