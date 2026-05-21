use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use open_ontologies::plan::{Planner, autoreceipt};
use open_ontologies::ocel_manufacturer::OcelManufacturer;
use open_ontologies::registry::ExecutionRegistry;
use open_ontologies::align::AlignmentEngine;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn test_autoreceipt_pipeline_rejects_synthetic() {
    let db_dir = tempdir().unwrap();
    let db = StateDb::open(&db_dir.path().join("state.db")).unwrap();
    let store = Arc::new(GraphStore::new());
    let planner = Planner::new(db.clone(), store.clone());

    // 1. Plan Architecture (ArchitecturalReceiptParsed)
    let c4_markdown = r#"
C4Context
    System(oo, "open-ontologies")
C4Container
    Container(ar_compiler, "AutoReceipt Compiler")
    Container(ocel_aligner, "OCEL Aligner")
"#;
    let intent = planner.plan_architecture(c4_markdown).unwrap();
    assert_eq!(intent["containers"].as_array().unwrap().len(), 2);

    let pipeline = autoreceipt::AutoReceiptPipeline::new();
    let _pipeline = pipeline.admit();

    // 2. Manufacture Expected OCEL (ExpectedOcelManufactured)
    let expected_events = OcelManufacturer::manufacture(&intent);
    assert!(expected_events.len() >= 2);

    // 3. Bind Execution Registry (ExecutionRegistryBound)
    let registry = ExecutionRegistry::new()
        .bind("AutoReceipt Compiler", "open-ontologies plan")
        .bind("OCEL Aligner", "open-ontologies align")
        .transition();
    assert_eq!(registry.resolve("OCEL Aligner").unwrap(), "open-ontologies align");

    // 4. Capture Observed OCEL (ObservedOcelCaptured)
    // Simulate observed events matching the expected ones by cloning
    let observed_events = expected_events.clone(); 
    let _execution_hash = blake3::hash(b"simulated execution").to_hex().to_string();

    // 5. Verify Alignment (AlignmentVerified)
    let engine = AlignmentEngine::new(db.clone(), store.clone());
    let (fitness, precision, verdict) = engine.verify_alignment(&expected_events, &observed_events);
    
    // Assert the system blocks synthetic closure
    assert_eq!(fitness, 0.0);
    assert_eq!(precision, 0.0);
    assert_eq!(verdict, "SyntheticObservedOcelRejected");
    
    // The test explicitly marks itself as synthetic and invalid for AutoReceipt
    println!("execution_mode = synthetic");
    println!("valid_for_autoreceipt_closure = false");
    println!("state = SyntheticObservedOcelRejected");
}

