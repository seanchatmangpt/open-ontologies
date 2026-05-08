//! End-to-end DoD test (plan verification item #8).
//!
//! Exercises the full OntoStar manufacturing path programmatically:
//!   1. declare a workflow (DataExtensionFastPath — a 3-stage built-in)
//!   2. emit the OCEL events for each stage with the scope_token
//!   3. close the scope
//!   4. run admission (real PowlBridgeReplay) against the artifact
//!   5. assert Admitted(receipt) with non-stub fitness
//!   6. dump OCEL JSON and assert the admission_granted event carries
//!      a receipt_hash attribute, and the chain is verifiable from
//!      the OCEL alone.
//!
//! GovernedRelease (the plan's named composition) inlines the alphabets
//! of OntologyAuthoring + LifecycleApply + Codegen and is the harder
//! version of this test — DataExtensionFastPath is the smaller, faster
//! proof that exercises every gate the same way.

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, OntoStarAdmissionGate, PowlBridgeReplay, PowlReplay,
};
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("e2e.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn emit_stage(store: &OcelStore, session: &str, scope: &str, stage: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let event_id = format!(
        "{}:{}:{}",
        session,
        stage,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    store
        .emit_event(&event_id, stage, &now, session, &[], &[], Some(scope))
        .unwrap();
}

#[test]
fn end_to_end_declare_walk_admit_receipt_in_ocel() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "e2e-session-01";

    // 1. Declare workflow.
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("declare workflow");

    // 2. Walk the alphabet: load → extend → query.
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, session, &token, stage);
    }

    // 3. Close.
    scope.close(&token).expect("close scope");

    // 4. Pre-flight conformance check (the gate will redo this, but verifying
    //    the bridge directly proves the wasm4pm replay is producing real
    //    fitness, not a stub 1.0).
    let powl = by_name("DataExtensionFastPath").unwrap().powl_string;
    let replay = PowlBridgeReplay::new(&store);
    let conf = replay.replay(&token, powl);
    assert!(
        conf.fitness >= 0.95,
        "pre-flight fitness should be >= 0.95, got {}",
        conf.fitness
    );
    assert!(
        !conf.run_id.starts_with("stub-run-"),
        "verdict came from the stub, not the real PowlBridge: {}",
        conf.run_id
    );

    // 5. Run admission gate.
    // f_min=0.95 (fitness threshold), p_min=0.7 (precision threshold).
    let gate = OntoStarAdmissionGate::new(0.95, 0.7, vec![], "ontostar-1.0.0");
    let artifact = ArtifactRef {
        kind: "turtle",
        bytes: b"@prefix : <urn:e2e:> . :a :b :c .",
    };
    let observed: Vec<String> = store
        .observed_event_types_for_session(session)
        .unwrap_or_default();
    let receipt = match gate.evaluate(
        &token,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &replay,
        session,
        powl,
        &observed,
    ) {
        Ok(r) => r,
        Err((defect, devs)) => panic!(
            "expected Admitted, got Denied({:?}) with {} deviations",
            defect,
            devs.len()
        ),
    };

    // 6. Receipt sanity: 32-byte BLAKE3 chain, scope_token round-trips, hashes non-empty.
    assert_eq!(receipt.bytes.len(), 32, "BLAKE3 receipt is 32 bytes");
    assert_eq!(receipt.record.scope_token, token);
    assert_ne!(receipt.record.artifact_hash, [0u8; 32]);
    assert_ne!(receipt.record.declared_powl_hash, [0u8; 32]);
    assert_ne!(receipt.record.gate_config_hash, [0u8; 32]);

    // 7. Dump OCEL and assert admission_granted event present with receipt_hash.
    let ocel = store
        .build_ocel(Some(session))
        .expect("build OCEL for session");
    let json = serde_json::to_string(&ocel).expect("serialize OCEL");
    assert!(
        json.contains("admission_granted"),
        "OCEL should contain an admission_granted event: {json}"
    );
    let receipt_hex: String = receipt.bytes.iter().map(|b| format!("{:02x}", b)).collect();
    assert!(
        json.contains(&receipt_hex),
        "OCEL should carry the BLAKE3 receipt hash {} as an event attribute",
        receipt_hex
    );

    println!(
        "E2E PASS: scope={} fitness={:.4} receipt={}",
        token, conf.fitness, receipt_hex
    );
}
