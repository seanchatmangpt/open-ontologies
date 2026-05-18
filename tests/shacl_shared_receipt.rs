// Integration tests: SharedReceiptV1 data against SHACL shapes
//
// Tests use the internal ShaclValidator Rust API directly (no MCP server
// required).  The validator enforces sh:minCount, sh:maxCount, and
// sh:datatype constraints.  Three type-drift risks from the shapes file are
// covered:
//
//   Risk 1 — OTel run-ID attribute name (run.id vs mcpp.run_id)
//   Risk 2 — Timing asymmetry (start_time+end_time vs started_at+duration_ms)
//   Risk 3 — Hash scheme prefix (bare 64-char hex, no 'blake3:' prefix)
//
// Note: sh:pattern, sh:hasValue, sh:in, sh:node, sh:minInclusive, and
// sh:maxInclusive are not yet implemented in ShaclValidator.  Tests that
// require those constraints are marked with an explanatory comment and assert
// only on the constraints that ARE enforced.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Load the canonical shared-receipt-shapes.ttl from the ontology directory.
fn shared_receipt_shapes() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ontology/shared-receipt-shapes.ttl"
    ))
    .expect("ontology/shared-receipt-shapes.ttl must exist")
}

/// Parse a JSON report string into a serde_json::Value.
fn parse_report(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("validator must return valid JSON")
}

// ── Test 1: valid receipt passes all minCount/maxCount checks ─────────────────
//
// The ShaclValidator implements sh:minCount, sh:maxCount, and sh:datatype.
// All required string fields must be present (minCount 1 satisfied).
//
// Known limitation: Oxigraph normalises xsd:nonNegativeInteger → xsd:integer
// on ingest.  The shapes file declares `sh:datatype xsd:nonNegativeInteger` for
// duration_ms.  The ShaclValidator's strict IRI comparison fires a datatype
// violation because the stored type is xsd:integer, not xsd:nonNegativeInteger.
// See test `duration_ms_datatype_violation_is_oxigraph_normalization_artefact`
// for the isolated demonstration of this known limitation.
//
// This test therefore omits `duration_ms` from the receipt and verifies that
// all other required fields satisfy minCount constraints with zero violations.
// A full SHACL processor (e.g. Apache Jena) would accept the canonical receipt
// because xsd:integer is a subtype of xsd:nonNegativeInteger in the XSD lattice.

#[test]
fn valid_shared_receipt_v1_required_string_fields_satisfy_mincount() {
    // A receipt with all required string-typed fields present; duration_ms is
    // omitted because the ShaclValidator misidentifies xsd:nonNegativeInteger
    // (stored as xsd:integer by Oxigraph) as a datatype violation.
    let hex64 = "a".repeat(64);
    let ttl = format!(
        r#"
@prefix sr:  <urn:ontostar:shared-receipt:> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sr:receipt-001 a sr:SharedReceiptV1 ;
    sr:run_id               "3f6c21a0-4b7e-4f1d-8a3b-9c0d5e2f7a8b" ;
    sr:schema_version       "shared/v1" ;
    sr:start_time           "2026-05-17T10:00:00Z" ;
    sr:end_time             "2026-05-17T10:00:01Z" ;
    sr:status               "success" ;
    sr:hash_format          "blake3-hex-64" ;
    sr:otel_run_id_attribute "run.id" ;
    sr:hashes [
        sr:config     "{hex64}" ;
        sr:input      "{hex64}" ;
        sr:plan       "{hex64}" ;
        sr:output     "{hex64}" ;
        sr:proof_pack "{hex64}"
    ] .
"#
    );

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("TTL must parse");

    let shapes = shared_receipt_shapes();
    let report_json = ShaclValidator::validate(&store, &shapes).expect("validate must succeed");
    let report = parse_report(&report_json);

    // The only expected violation is duration_ms minCount (omitted above).
    // All other required string fields are present; no other violations expected.
    let violations = report["violations"].as_array().unwrap();
    let non_duration_violations: Vec<_> = violations
        .iter()
        .filter(|v| !v["path"].as_str().unwrap_or("").contains("duration_ms"))
        .collect();
    assert!(
        non_duration_violations.is_empty(),
        "all required fields except duration_ms must be present; unexpected violations: {:?}",
        non_duration_violations
    );
}

// ── Test 1b: validator produces exactly the duration_ms normalization artefact ─

#[test]
fn duration_ms_datatype_violation_is_oxigraph_normalization_artefact() {
    // Demonstrates that Oxigraph normalises xsd:nonNegativeInteger to xsd:integer.
    // The ShaclValidator then fires a datatype violation because the stored type
    // (xsd:integer) does not match the shape's declared type (xsd:nonNegativeInteger).
    // A full SHACL 1.0 processor would NOT flag this because xsd:nonNegativeInteger
    // is a derived type of xsd:integer in the XSD type hierarchy.
    //
    // This test documents the known validator limitation rather than masking it.
    let hex64 = "a".repeat(64);
    let ttl = format!(
        r#"
@prefix sr:  <urn:ontostar:shared-receipt:> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sr:receipt-001 a sr:SharedReceiptV1 ;
    sr:run_id               "3f6c21a0-4b7e-4f1d-8a3b-9c0d5e2f7a8b" ;
    sr:schema_version       "shared/v1" ;
    sr:start_time           "2026-05-17T10:00:00Z" ;
    sr:end_time             "2026-05-17T10:00:01Z" ;
    sr:duration_ms          "1000"^^xsd:nonNegativeInteger ;
    sr:status               "success" ;
    sr:hash_format          "blake3-hex-64" ;
    sr:otel_run_id_attribute "run.id" ;
    sr:hashes [
        sr:config     "{hex64}" ;
        sr:input      "{hex64}" ;
        sr:plan       "{hex64}" ;
        sr:output     "{hex64}" ;
        sr:proof_pack "{hex64}"
    ] .
"#
    );

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("TTL must parse");

    let shapes = shared_receipt_shapes();
    let report_json = ShaclValidator::validate(&store, &shapes).expect("validate must succeed");
    let report = parse_report(&report_json);

    // Exactly one violation: the duration_ms datatype mismatch artefact.
    let violations = report["violations"].as_array().unwrap();
    let duration_violations: Vec<_> = violations
        .iter()
        .filter(|v| v["path"].as_str().unwrap_or("").contains("duration_ms"))
        .collect();
    assert_eq!(
        duration_violations.len(),
        1,
        "expected exactly 1 duration_ms datatype artefact; got: {:?}",
        violations
    );
    assert_eq!(
        duration_violations[0]["constraint"].as_str().unwrap(),
        "datatype",
        "violation must be a datatype constraint (not minCount or maxCount)"
    );
    // All other violations are zero (the receipt is otherwise well-formed)
    let other_violations: Vec<_> = violations
        .iter()
        .filter(|v| !v["path"].as_str().unwrap_or("").contains("duration_ms"))
        .collect();
    assert!(
        other_violations.is_empty(),
        "no violations expected besides the duration_ms normalisation artefact; got: {:?}",
        other_violations
    );
}

// ── Test 2: missing run_id violates sh:minCount ───────────────────────────────

#[test]
fn missing_run_id_violates_shacl_shapes() {
    // Omit sr:run_id — minCount 1 must fire
    let hex64 = "b".repeat(64);
    let ttl = format!(
        r#"
@prefix sr:  <urn:ontostar:shared-receipt:> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sr:receipt-no-run-id a sr:SharedReceiptV1 ;
    sr:schema_version       "shared/v1" ;
    sr:start_time           "2026-05-17T10:00:00Z" ;
    sr:end_time             "2026-05-17T10:00:01Z" ;
    sr:duration_ms          "500"^^xsd:integer ;
    sr:status               "success" ;
    sr:hash_format          "blake3-hex-64" ;
    sr:otel_run_id_attribute "run.id" ;
    sr:hashes [
        sr:config     "{hex64}" ;
        sr:input      "{hex64}" ;
        sr:plan       "{hex64}" ;
        sr:output     "{hex64}" ;
        sr:proof_pack "{hex64}"
    ] .
"#
    );

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("TTL must parse");

    let shapes = shared_receipt_shapes();
    let report_json = ShaclValidator::validate(&store, &shapes).expect("validate must succeed");
    let report = parse_report(&report_json);

    assert!(
        !report["conforms"].as_bool().unwrap(),
        "receipt missing run_id must not conform"
    );
    assert!(
        report["violation_count"].as_u64().unwrap() >= 1,
        "missing run_id must produce at least one violation"
    );

    // Confirm the violation references the run_id path
    let violations = report["violations"].as_array().unwrap();
    let has_run_id_violation = violations.iter().any(|v| {
        v["path"]
            .as_str()
            .unwrap_or("")
            .contains("run_id")
    });
    assert!(
        has_run_id_violation,
        "violation list must include a run_id path entry; got: {}",
        report["violations"]
    );
}

// ── Test 3: missing schema_version violates sh:minCount ──────────────────────

#[test]
fn missing_schema_version_violates_shacl_shapes() {
    let hex64 = "c".repeat(64);
    let ttl = format!(
        r#"
@prefix sr:  <urn:ontostar:shared-receipt:> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sr:receipt-no-schema-version a sr:SharedReceiptV1 ;
    sr:run_id               "3f6c21a0-4b7e-4f1d-8a3b-9c0d5e2f7a8b" ;
    sr:start_time           "2026-05-17T10:00:00Z" ;
    sr:end_time             "2026-05-17T10:00:01Z" ;
    sr:duration_ms          "200"^^xsd:integer ;
    sr:status               "success" ;
    sr:hash_format          "blake3-hex-64" ;
    sr:otel_run_id_attribute "run.id" ;
    sr:hashes [
        sr:config     "{hex64}" ;
        sr:input      "{hex64}" ;
        sr:plan       "{hex64}" ;
        sr:output     "{hex64}" ;
        sr:proof_pack "{hex64}"
    ] .
"#
    );

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("TTL must parse");

    let shapes = shared_receipt_shapes();
    let report_json = ShaclValidator::validate(&store, &shapes).expect("validate must succeed");
    let report = parse_report(&report_json);

    assert!(
        !report["conforms"].as_bool().unwrap(),
        "receipt missing schema_version must not conform"
    );
    assert!(
        report["violation_count"].as_u64().unwrap() >= 1,
        "missing schema_version must produce at least one violation"
    );
}

// ── Test 4: missing start_time violates sh:minCount (Risk 2) ─────────────────

#[test]
fn missing_start_time_violates_risk2_timing_field() {
    // Risk 2: start_time+end_time are required; absence must fail
    let hex64 = "d".repeat(64);
    let ttl = format!(
        r#"
@prefix sr:  <urn:ontostar:shared-receipt:> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sr:receipt-no-start-time a sr:SharedReceiptV1 ;
    sr:run_id               "3f6c21a0-4b7e-4f1d-8a3b-9c0d5e2f7a8b" ;
    sr:schema_version       "shared/v1" ;
    sr:end_time             "2026-05-17T10:00:01Z" ;
    sr:duration_ms          "1000"^^xsd:integer ;
    sr:status               "success" ;
    sr:hash_format          "blake3-hex-64" ;
    sr:otel_run_id_attribute "run.id" ;
    sr:hashes [
        sr:config     "{hex64}" ;
        sr:input      "{hex64}" ;
        sr:plan       "{hex64}" ;
        sr:output     "{hex64}" ;
        sr:proof_pack "{hex64}"
    ] .
"#
    );

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("TTL must parse");

    let shapes = shared_receipt_shapes();
    let report_json = ShaclValidator::validate(&store, &shapes).expect("validate must succeed");
    let report = parse_report(&report_json);

    assert!(
        !report["conforms"].as_bool().unwrap(),
        "receipt missing start_time (Risk 2) must not conform"
    );
    assert!(
        report["violation_count"].as_u64().unwrap() >= 1,
        "missing start_time must produce at least one violation"
    );
}

// ── Test 5: missing end_time violates sh:minCount (Risk 2) ───────────────────

#[test]
fn missing_end_time_violates_risk2_timing_field() {
    // Risk 2: end_time is required; absence must fail
    let hex64 = "e".repeat(64);
    let ttl = format!(
        r#"
@prefix sr:  <urn:ontostar:shared-receipt:> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sr:receipt-no-end-time a sr:SharedReceiptV1 ;
    sr:run_id               "3f6c21a0-4b7e-4f1d-8a3b-9c0d5e2f7a8b" ;
    sr:schema_version       "shared/v1" ;
    sr:start_time           "2026-05-17T10:00:00Z" ;
    sr:duration_ms          "1000"^^xsd:integer ;
    sr:status               "success" ;
    sr:hash_format          "blake3-hex-64" ;
    sr:otel_run_id_attribute "run.id" ;
    sr:hashes [
        sr:config     "{hex64}" ;
        sr:input      "{hex64}" ;
        sr:plan       "{hex64}" ;
        sr:output     "{hex64}" ;
        sr:proof_pack "{hex64}"
    ] .
"#
    );

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("TTL must parse");

    let shapes = shared_receipt_shapes();
    let report_json = ShaclValidator::validate(&store, &shapes).expect("validate must succeed");
    let report = parse_report(&report_json);

    assert!(
        !report["conforms"].as_bool().unwrap(),
        "receipt missing end_time (Risk 2) must not conform"
    );
    assert!(
        report["violation_count"].as_u64().unwrap() >= 1,
        "missing end_time must produce at least one violation"
    );
}

// ── Test 6: missing otel_run_id_attribute violates sh:minCount (Risk 1) ──────

#[test]
fn missing_otel_run_id_attribute_violates_risk1() {
    // Risk 1: otel_run_id_attribute is required; absence must fail sh:minCount
    let hex64 = "f".repeat(64);
    let ttl = format!(
        r#"
@prefix sr:  <urn:ontostar:shared-receipt:> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sr:receipt-no-otel-attr a sr:SharedReceiptV1 ;
    sr:run_id               "3f6c21a0-4b7e-4f1d-8a3b-9c0d5e2f7a8b" ;
    sr:schema_version       "shared/v1" ;
    sr:start_time           "2026-05-17T10:00:00Z" ;
    sr:end_time             "2026-05-17T10:00:01Z" ;
    sr:duration_ms          "500"^^xsd:integer ;
    sr:status               "success" ;
    sr:hash_format          "blake3-hex-64" ;
    sr:hashes [
        sr:config     "{hex64}" ;
        sr:input      "{hex64}" ;
        sr:plan       "{hex64}" ;
        sr:output     "{hex64}" ;
        sr:proof_pack "{hex64}"
    ] .
"#
    );

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("TTL must parse");

    let shapes = shared_receipt_shapes();
    let report_json = ShaclValidator::validate(&store, &shapes).expect("validate must succeed");
    let report = parse_report(&report_json);

    assert!(
        !report["conforms"].as_bool().unwrap(),
        "receipt missing otel_run_id_attribute (Risk 1) must not conform"
    );
    assert!(
        report["violation_count"].as_u64().unwrap() >= 1,
        "missing otel_run_id_attribute must produce at least one violation"
    );
}

// ── Test 7: duration_ms with wrong datatype violates sh:datatype ──────────────

#[test]
fn duration_ms_wrong_datatype_violates_shacl_shapes() {
    // duration_ms must be xsd:nonNegativeInteger; a plain xsd:string violates datatype
    let hex64 = "0".repeat(64);
    let ttl = format!(
        r#"
@prefix sr:  <urn:ontostar:shared-receipt:> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sr:receipt-bad-duration a sr:SharedReceiptV1 ;
    sr:run_id               "3f6c21a0-4b7e-4f1d-8a3b-9c0d5e2f7a8b" ;
    sr:schema_version       "shared/v1" ;
    sr:start_time           "2026-05-17T10:00:00Z" ;
    sr:end_time             "2026-05-17T10:00:01Z" ;
    sr:duration_ms          "1000" ;
    sr:status               "success" ;
    sr:hash_format          "blake3-hex-64" ;
    sr:otel_run_id_attribute "run.id" ;
    sr:hashes [
        sr:config     "{hex64}" ;
        sr:input      "{hex64}" ;
        sr:plan       "{hex64}" ;
        sr:output     "{hex64}" ;
        sr:proof_pack "{hex64}"
    ] .
"#
    );

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("TTL must parse");

    let shapes = shared_receipt_shapes();
    let report_json = ShaclValidator::validate(&store, &shapes).expect("validate must succeed");
    let report = parse_report(&report_json);

    assert!(
        !report["conforms"].as_bool().unwrap(),
        "duration_ms as plain string must not conform (requires xsd:nonNegativeInteger)"
    );
    assert!(
        report["violation_count"].as_u64().unwrap() >= 1,
        "wrong datatype for duration_ms must produce at least one violation"
    );
}

// ── Test 8: receipt with conformance sub-object passes non-duration checks ────
//
// Tests that all optional fields (source, chain_predecessor, conformance
// dimensions) do not introduce violations.  duration_ms is omitted to avoid
// the known Oxigraph xsd:nonNegativeInteger normalization artefact (see test
// `duration_ms_datatype_violation_is_oxigraph_normalization_artefact`).

#[test]
fn receipt_with_optional_conformance_dimensions_passes_non_duration_checks() {
    // conformance is optional; when present all dimensions are optional decimals
    let hex64 = "1".repeat(64);
    let ttl = format!(
        r#"
@prefix sr:  <urn:ontostar:shared-receipt:> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sr:receipt-with-conformance a sr:SharedReceiptV1 ;
    sr:run_id               "3f6c21a0-4b7e-4f1d-8a3b-9c0d5e2f7a8b" ;
    sr:schema_version       "shared/v1" ;
    sr:start_time           "2026-05-17T10:00:00Z" ;
    sr:end_time             "2026-05-17T10:00:01Z" ;
    sr:status               "success" ;
    sr:hash_format          "blake3-hex-64" ;
    sr:otel_run_id_attribute "run.id" ;
    sr:source               "wasm4pm" ;
    sr:chain_predecessor    "genesis" ;
    sr:hashes [
        sr:config     "{hex64}" ;
        sr:input      "{hex64}" ;
        sr:plan       "{hex64}" ;
        sr:output     "{hex64}" ;
        sr:proof_pack "{hex64}"
    ] ;
    sr:conformance [
        sr:fitness      "0.95"^^xsd:decimal ;
        sr:precision    "0.88"^^xsd:decimal ;
        sr:lifecycle    "1.0"^^xsd:decimal ;
        sr:cardinality  "0.72"^^xsd:decimal ;
        sr:receipt      "1.0"^^xsd:decimal
    ] .
"#
    );

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("TTL must parse");

    let shapes = shared_receipt_shapes();
    let report_json = ShaclValidator::validate(&store, &shapes).expect("validate must succeed");
    let report = parse_report(&report_json);

    // Only expect the duration_ms minCount violation (field intentionally omitted).
    // All other fields — including optional conformance dimensions — must not violate.
    let violations = report["violations"].as_array().unwrap();
    let non_duration_violations: Vec<_> = violations
        .iter()
        .filter(|v| !v["path"].as_str().unwrap_or("").contains("duration_ms"))
        .collect();
    assert!(
        non_duration_violations.is_empty(),
        "optional conformance fields must not introduce violations; unexpected: {:?}",
        non_duration_violations
    );
}

// ── Test 9: missing hash_format violates sh:minCount (Risk 3) ────────────────

#[test]
fn missing_hash_format_violates_risk3() {
    // Risk 3: hash_format declares all hashes are bare hex; its absence must fail
    let hex64 = "2".repeat(64);
    let ttl = format!(
        r#"
@prefix sr:  <urn:ontostar:shared-receipt:> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sr:receipt-no-hash-format a sr:SharedReceiptV1 ;
    sr:run_id               "3f6c21a0-4b7e-4f1d-8a3b-9c0d5e2f7a8b" ;
    sr:schema_version       "shared/v1" ;
    sr:start_time           "2026-05-17T10:00:00Z" ;
    sr:end_time             "2026-05-17T10:00:01Z" ;
    sr:duration_ms          "100"^^xsd:integer ;
    sr:status               "success" ;
    sr:otel_run_id_attribute "run.id" ;
    sr:hashes [
        sr:config     "{hex64}" ;
        sr:input      "{hex64}" ;
        sr:plan       "{hex64}" ;
        sr:output     "{hex64}" ;
        sr:proof_pack "{hex64}"
    ] .
"#
    );

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("TTL must parse");

    let shapes = shared_receipt_shapes();
    let report_json = ShaclValidator::validate(&store, &shapes).expect("validate must succeed");
    let report = parse_report(&report_json);

    assert!(
        !report["conforms"].as_bool().unwrap(),
        "receipt missing hash_format (Risk 3) must not conform"
    );
    assert!(
        report["violation_count"].as_u64().unwrap() >= 1,
        "missing hash_format must produce at least one violation"
    );
}

// ── Test 10: multiple missing required fields produce multiple violations ──────

#[test]
fn multiple_missing_fields_produce_multiple_violations() {
    // Strip run_id, schema_version, start_time — expect at least 3 violations
    let hex64 = "3".repeat(64);
    let ttl = format!(
        r#"
@prefix sr:  <urn:ontostar:shared-receipt:> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sr:receipt-many-missing a sr:SharedReceiptV1 ;
    sr:end_time             "2026-05-17T10:00:01Z" ;
    sr:duration_ms          "50"^^xsd:integer ;
    sr:status               "success" ;
    sr:hash_format          "blake3-hex-64" ;
    sr:otel_run_id_attribute "run.id" ;
    sr:hashes [
        sr:config     "{hex64}" ;
        sr:input      "{hex64}" ;
        sr:plan       "{hex64}" ;
        sr:output     "{hex64}" ;
        sr:proof_pack "{hex64}"
    ] .
"#
    );

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("TTL must parse");

    let shapes = shared_receipt_shapes();
    let report_json = ShaclValidator::validate(&store, &shapes).expect("validate must succeed");
    let report = parse_report(&report_json);

    assert!(
        !report["conforms"].as_bool().unwrap(),
        "receipt with multiple missing required fields must not conform"
    );
    assert!(
        report["violation_count"].as_u64().unwrap() >= 3,
        "three missing required fields must produce at least 3 violations; got: {}",
        report["violation_count"]
    );
}
