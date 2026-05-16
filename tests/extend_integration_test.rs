use open_ontologies::graph::GraphStore;
use open_ontologies::ingest::DataIngester;
use open_ontologies::mapping::{MappingConfig, FieldMapping};
use open_ontologies::shacl::ShaclValidator;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

/// Edge case: ingest succeeds but SHACL validation finds violations.
///
/// Mirrors the `onto_extend_inner` path where `stop_on_violations = true`
/// (the default): after a successful ingest, SHACL shapes are applied and
/// the validation report must reflect `conforms = false`, preventing the
/// pipeline from proceeding to the reasoning stage.
#[test]
fn onto_extend_shacl_violation_returns_error() {
    // 1. Ingest CSV where one row is intentionally missing the required label field.
    //    Row b1 has a name, row b2 does not — b2 will violate the SHACL minCount 1
    //    constraint on rdfs:label.
    let csv = "id,name\nb1,Tower Bridge\nb2,\n";
    let rows = DataIngester::parse_csv(csv).unwrap();
    assert_eq!(rows.len(), 2, "should parse both rows");

    // 2. Map rows to RDF: id → subject IRI, name → rdfs:label.
    //    b2's name is an empty string — the mapper will emit an empty literal,
    //    which means rdfs:label is present but empty (still satisfies minCount 1).
    //    To produce a true violation we omit the label field mapping for the
    //    shape target class altogether by using a separate class for b2.
    //
    //    Simpler: map only the id field so that NO rdfs:label triple is emitted
    //    for either subject, then assert minCount 1 on rdfs:label fails for both.
    let store = Arc::new(GraphStore::new());
    let mapping = MappingConfig {
        base_iri: "http://example.org/".to_string(),
        id_field: "id".to_string(),
        class: "http://example.org/Building".to_string(),
        // Only map the id field — rdfs:label intentionally excluded so the
        // SHACL shape's minCount 1 on rdfs:label will fire on every subject.
        mappings: vec![FieldMapping {
            field: "id".to_string(),
            predicate: "http://example.org/id".to_string(),
            datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
            class: None,
            lookup: false,
        }],
    };
    let ntriples = mapping.rows_to_ntriples(&rows);
    let triples_loaded = store.load_ntriples(&ntriples).unwrap();
    assert!(triples_loaded > 0, "ingest must succeed and load triples");

    // 3. Apply a SHACL shape that requires every Building to carry rdfs:label.
    //    Since no label triples were ingested, both b1 and b2 violate the shape.
    let shapes = r#"
        @prefix sh:   <http://www.w3.org/ns/shacl#> .
        @prefix ex:   <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:BuildingLabelShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
                sh:message "Building must have a rdfs:label" ;
            ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&report).unwrap();

    // 4. Assert the validation path that onto_extend_inner acts on:
    //    conforms must be false and at least one violation must be reported.
    //    When stop_on_violations is true (the default), onto_extend_inner returns
    //    {"stage":"shacl","stopped":true,...} instead of {"ok":true,...}.
    assert_eq!(
        parsed["conforms"], false,
        "SHACL must not conform — no rdfs:label was ingested; got: {parsed}"
    );
    let violation_count = parsed["violation_count"].as_u64().unwrap_or(0);
    assert!(
        violation_count >= 1,
        "expected at least 1 SHACL violation, got {violation_count}; report: {parsed}"
    );
}

#[test]
fn test_full_pipeline_csv_to_validated_rdf() {
    // 1. Load an ontology
    let store = Arc::new(GraphStore::new());
    let ontology = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .

        ex:Building a owl:Class ;
            rdfs:label "Building" .
        ex:Landmark a owl:Class ;
            rdfs:subClassOf ex:Building ;
            rdfs:label "Landmark" .
        ex:hasName a owl:DatatypeProperty ;
            rdfs:domain ex:Building ;
            rdfs:label "has name" .
    "#;
    store.load_turtle(ontology, None).unwrap();
    let onto_triples = store.triple_count();
    assert!(onto_triples > 0);

    // 2. Parse CSV data
    let csv = "id,name,type\nb1,Tower Bridge,Landmark\nb2,Big Ben,Landmark\n";
    let rows = DataIngester::parse_csv(csv).unwrap();
    assert_eq!(rows.len(), 2);

    // 3. Create explicit mapping
    let mapping = MappingConfig {
        base_iri: "http://example.org/".to_string(),
        id_field: "id".to_string(),
        class: "http://example.org/Landmark".to_string(),
        mappings: vec![
            FieldMapping {
                field: "id".to_string(),
                predicate: "http://example.org/id".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
                class: None,
                lookup: false,
            },
            FieldMapping {
                field: "name".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#label".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
                class: None,
                lookup: false,
            },
        ],
    };

    // 4. Ingest — convert rows to N-Triples and load
    let ntriples = mapping.rows_to_ntriples(&rows);
    let loaded = store.load_ntriples(&ntriples).unwrap();
    assert!(loaded > 0);

    // 5. Validate with SHACL — Landmarks must have labels
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:LandmarkShape a sh:NodeShape ;
            sh:targetClass ex:Landmark ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
                sh:message "Landmark must have a label" ;
            ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(parsed["conforms"], true, "SHACL should pass — both landmarks have labels");

    // 6. Run RDFS reasoning — Landmarks should be inferred as Buildings
    let reason_report = Reasoner::run(&store, "rdfs", true).unwrap();
    let reason_parsed: serde_json::Value = serde_json::from_str(&reason_report).unwrap();
    let inferred = reason_parsed["inferred_count"].as_u64().unwrap();
    assert!(inferred >= 2, "Should infer at least 2 triples (b1 and b2 are Buildings)");

    // 7. Verify inference worked via SPARQL
    let check = store.sparql_select(
        "ASK { <http://example.org/b1> a <http://example.org/Building> }"
    ).unwrap();
    assert!(check.contains("true"), "b1 should be inferred as a Building via subClassOf");

    let check2 = store.sparql_select(
        "ASK { <http://example.org/b2> a <http://example.org/Building> }"
    ).unwrap();
    assert!(check2.contains("true"), "b2 should be inferred as a Building via subClassOf");
}
