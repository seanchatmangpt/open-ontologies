//! Phase 4 — Solution Manufacturing full E2E.
//!
//! Drives the complete autonomic-manufacturing chain:
//!
//!   1. Open a SolutionManufacturing scope.
//!   2. Build a SolutionSpec bound to a synthetic work-order receipt.
//!   3. Emit all 7 required SolutionManufacturing stages upfront.
//!   4. Drive AdmissionOp::SolutionManufactured.
//!   5. Verify every target (iac/rust/erlang/atomvm) emitted at least
//!      one file with the receipt header / JSON-embedded receipt.
//!   6. Verify external-verifier round-trip: strip the receipt header
//!      from a Rust file, recompute BLAKE3, compare to the
//!      `ostar-artifact-hash` line.
//!   7. Verify deterministic re-manufacture produces byte-identical
//!      output (no time / random in the generators).
//!   8. Verify the work-order receipt hash appears in EVERY generated
//!      file — the entire stack is provably bound to one work order.

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, NoopPowlReplay, OntoStarAdmissionGate,
};
use open_ontologies::defects::DefectClass;
use open_ontologies::manufacturing::{
    self, validators, ManufacturedFile, SolutionBundle, SolutionSpec,
};
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

const WORKFLOW: &str = "SolutionManufacturing";

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("solution-mfg.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn ok_spec() -> SolutionSpec {
    SolutionSpec {
        name: "fortune5_revops".into(),
        description: "Fortune-5 RevOps revenue trust manufactured stack".into(),
        iac_target: "aws".into(),
        region: "us-east-1".into(),
        supervisor_children: 6,
        mcu_target: "esp32".into(),
        // 64-char hex stand-in for an upstream WorkOrderAdmitted receipt.
        work_order_receipt_hash: "a".repeat(64),
    }
}

fn emit(store: &OcelStore, session: &str, scope: &str, stage: &str, attrs: &[(&str, &str)]) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    let now = chrono::Utc::now().to_rfc3339();
    let event_id = format!("{session}:{n:012}:{stage}");
    store
        .emit_event(&event_id, stage, &now, session, attrs, &[], Some(scope))
        .unwrap();
}

#[test]
fn solution_manufacturing_e2e_admits_full_stack() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "smfg-e2e";
    let scope_mgr = WorkflowScope::new(&db, session);
    let token = scope_mgr.open(Some(WORKFLOW), None, None).unwrap();
    scope_mgr.close(&token).unwrap();

    let spec = ok_spec();
    // Manufacture the bundle BEFORE the gate so we can compute the
    // canonical artifact bytes deterministically.
    let bundle = manufacturing::manufacture(&spec).expect("manufacture must succeed");

    // Emit the 7 required stages.
    for stage in &[
        "work_order_received",
        "architecture_decided",
        "iac_generated",
        "rust_generated",
        "erlang_generated",
        "atomvm_generated",
        "receipt_chain_sealed",
    ] {
        emit(
            &store,
            session,
            &token,
            stage,
            &[
                ("solution_name", spec.name.as_str()),
                ("work_order_receipt", spec.work_order_receipt_hash.as_str()),
            ],
        );
    }
    let observed: Vec<String> = store.observed_event_types_for_session(session).unwrap();
    for required in by_name(WORKFLOW).unwrap().required_stages {
        assert!(
            observed.iter().any(|s| s == required),
            "required stage `{required}` missing from observed trace"
        );
    }

    // Canonical bundle bytes — same as the handler computes.
    let mut digest = blake3::Hasher::new();
    let mut sorted: Vec<&ManufacturedFile> = bundle.files.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    for f in &sorted {
        digest.update(f.path.as_bytes());
        digest.update(b"\0");
        digest.update(f.contents.as_bytes());
        digest.update(b"\x1e");
    }
    let canonical = digest.finalize().to_hex().to_string();

    let gate = OntoStarAdmissionGate::new(
        0.95,
        0.85,
        by_name(WORKFLOW)
            .map(|w| w.required_stages.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default(),
        "ontostar-1.0.0",
    );
    let powl = by_name(WORKFLOW).unwrap().powl_string;
    let artifact = ArtifactRef {
        kind: "solution-bundle",
        bytes: canonical.as_bytes(),
    };
    let receipt = gate
        .evaluate(
            &token,
            AdmissionOp::SolutionManufactured,
            &artifact,
            &store,
            &NoopPowlReplay,
            session,
            powl,
            &observed,
        )
        .expect("SolutionManufactured admission must succeed");

    // Every target produced files.
    assert!(!bundle.files_for("iac").is_empty(), "no iac files");
    assert!(!bundle.files_for("rust").is_empty(), "no rust files");
    assert!(!bundle.files_for("erlang").is_empty(), "no erlang files");
    assert!(!bundle.files_for("atomvm").is_empty(), "no atomvm files");

    // Every file binds to the work-order receipt.
    for f in &bundle.files {
        assert!(
            f.contents.contains(&spec.work_order_receipt_hash),
            "{} missing work-order binding",
            f.path
        );
    }

    // Receipt persisted with correct law version + scope.
    assert_eq!(receipt.record.scope_token, token);
    assert_eq!(receipt.record.production_law_version, "ontostar-1.0.0");
}

#[test]
fn external_verifier_round_trips_rust_file_via_header_strip() {
    // External-verifier protocol: strip leading `// ostar-…` lines,
    // BLAKE3 the remainder, compare to the `ostar-artifact-hash` line
    // value embedded in the header. This is the same protocol used
    // for portability_save.rs (TTL files).
    let bundle = manufacturing::manufacture(&ok_spec()).unwrap();
    let lib_rs = bundle
        .files
        .iter()
        .find(|f| f.path == "rust/src/lib.rs")
        .expect("rust/src/lib.rs must exist");
    let body = validators::strip_header(&lib_rs.contents, "//");
    let expected_hash = blake3::hash(body.as_bytes()).to_hex().to_string();

    // Pull out the artifact_hash line from the header.
    let actual_hash = lib_rs
        .contents
        .lines()
        .find_map(|l| l.strip_prefix("// ostar-artifact-hash: "))
        .expect("ostar-artifact-hash line must be present");
    assert_eq!(
        expected_hash, actual_hash,
        "external-verifier round-trip failed for rust/src/lib.rs"
    );
}

#[test]
fn external_verifier_round_trips_erlang_file_via_header_strip() {
    let bundle = manufacturing::manufacture(&ok_spec()).unwrap();
    let sup = bundle
        .files
        .iter()
        .find(|f| f.path.ends_with("_sup.erl"))
        .expect("erlang sup file must exist");
    let body = validators::strip_header(&sup.contents, "%%");
    let expected_hash = blake3::hash(body.as_bytes()).to_hex().to_string();
    let actual_hash = sup
        .contents
        .lines()
        .find_map(|l| l.strip_prefix("%% ostar-artifact-hash: "))
        .expect("ostar-artifact-hash line must be present");
    assert_eq!(expected_hash, actual_hash);
}

#[test]
fn iac_files_have_embedded_json_receipt() {
    let bundle = manufacturing::manufacture(&ok_spec()).unwrap();
    for f in bundle.files_for("iac") {
        let parsed: serde_json::Value =
            serde_json::from_str(&f.contents).expect("iac file is valid JSON");
        let receipt = parsed
            .get("_ontostar_receipt")
            .expect("iac file must carry _ontostar_receipt");
        assert_eq!(
            receipt.get("solution_name").and_then(|v| v.as_str()),
            Some("fortune5_revops")
        );
        assert_eq!(
            receipt.get("work_order_receipt").and_then(|v| v.as_str()),
            Some(bundle.spec.work_order_receipt_hash.as_str())
        );
    }
}

#[test]
fn manufacture_is_deterministic_e2e() {
    // Two independent calls with the same SolutionSpec must produce
    // byte-identical output (no time / random / nondeterminism in
    // the generators).
    let a = manufacturing::manufacture(&ok_spec()).unwrap();
    let b = manufacturing::manufacture(&ok_spec()).unwrap();
    assert_eq!(a.files.len(), b.files.len());
    for (x, y) in a.files.iter().zip(b.files.iter()) {
        assert_eq!(x.path, y.path);
        assert_eq!(x.contents, y.contents);
    }
}

#[test]
fn empty_work_order_receipt_denies_with_architecture_unbound() {
    let mut spec = ok_spec();
    spec.work_order_receipt_hash = "".into();
    match manufacturing::manufacture(&spec) {
        Err(DefectClass::ArchitectureUnbound) => {}
        other => panic!("expected ArchitectureUnbound, got {other:?}"),
    }
}

#[test]
fn unsupported_mcu_denies_with_atom_vm_invalid() {
    let mut spec = ok_spec();
    spec.mcu_target = "msp430".into();
    match manufacturing::manufacture(&spec) {
        Err(DefectClass::AtomVmInvalid { .. }) => {}
        other => panic!("expected AtomVmInvalid, got {other:?}"),
    }
}

#[test]
fn supervisor_zero_denies_with_erlang_invalid() {
    let mut spec = ok_spec();
    spec.supervisor_children = 0;
    match manufacturing::manufacture(&spec) {
        Err(DefectClass::ErlangInvalid { .. }) => {}
        other => panic!("expected ErlangInvalid, got {other:?}"),
    }
}

#[test]
fn bundle_files_can_be_written_to_disk_and_read_back() {
    let bundle: SolutionBundle = manufacturing::manufacture(&ok_spec()).unwrap();
    let tmp = tempdir().unwrap();
    for f in &bundle.files {
        let full = tmp.path().join(&f.path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, &f.contents).unwrap();
        // Read back and verify byte-identity.
        let read_back = std::fs::read_to_string(&full).unwrap();
        assert_eq!(read_back, f.contents, "round-trip mismatch on {}", f.path);
    }
}

#[test]
fn rust_lib_includes_self_test_on_solution_name() {
    // The generated rust/src/lib.rs ships its own #[cfg(test)] mod
    // that asserts manufactured_solution_name() returns the same
    // string we configured. Verify the test code is present.
    let bundle = manufacturing::manufacture(&ok_spec()).unwrap();
    let lib = bundle
        .files
        .iter()
        .find(|f| f.path == "rust/src/lib.rs")
        .unwrap();
    assert!(
        lib.contents.contains("#[cfg(test)]"),
        "lib.rs must carry a self-test"
    );
    assert!(
        lib.contents.contains("solution_name_matches"),
        "lib.rs self-test must include solution_name_matches"
    );
}
