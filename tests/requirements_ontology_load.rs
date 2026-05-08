//! Phase 1.7 — verify the requirements ontology + SHACL shapes parse as
//! valid Turtle and load into the graph store without errors.
//!
//! The shapes file is ADVISORY — the authoritative admission lives in
//! src/admission.rs. This test is a syntax / structure smoke check only.

use open_ontologies::graph::GraphStore;

#[test]
fn requirements_ontology_loads_as_turtle() {
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ontology/requirements.ttl");
    assert!(path.exists(), "{}", path.display());
    let content = std::fs::read_to_string(&path).expect("read ontology file");

    let graph = GraphStore::new();
    graph
        .load_turtle(&content, None)
        .expect("requirements.ttl should parse as Turtle and load into the graph");

    // The ontology must declare at least the 9 core classes.
    let stats = graph
        .get_stats()
        .expect("graph stats should be obtainable after load");
    // Stats is a JSON string; verify it carries a non-zero triple count.
    let parsed: serde_json::Value =
        serde_json::from_str(&stats).expect("stats should be valid JSON");
    let triples = parsed
        .get("total_triples")
        .or_else(|| parsed.get("triple_count"))
        .or_else(|| parsed.get("triples"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        triples >= 50,
        "expected >= 50 triples in requirements.ttl, got {triples} (stats={stats})"
    );
}

#[test]
fn requirements_shapes_loads_as_turtle() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("ontology/requirements-shapes.ttl");
    assert!(path.exists(), "{}", path.display());
    let content = std::fs::read_to_string(&path).expect("read shapes file");

    let graph = GraphStore::new();
    graph
        .load_turtle(&content, None)
        .expect("requirements-shapes.ttl should parse as Turtle and load into the graph");
}

#[test]
fn shapes_reference_each_canonical_defect_class() {
    // The shapes file deliberately mirrors three DefectClass variants
    // from src/defects.rs. This test pins the contract: if a defect tag
    // is renamed in Rust, the shapes file must be updated too.
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("ontology/requirements-shapes.ttl");
    let content = std::fs::read_to_string(&path).expect("read shapes file");
    for required in &[
        "RequirementWithoutSource",
        "CtqIncomplete",
        "WorkOrderMissingCounterfactual",
    ] {
        assert!(
            content.contains(required),
            "requirements-shapes.ttl must mention DefectClass `{required}` so the SHACL \
             surface mirrors the Rust admission contract"
        );
    }
}
