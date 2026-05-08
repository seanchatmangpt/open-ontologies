//! Phase 4 — Recursive admission E2E: WorkOrderAdmitted → SolutionManufactured.
//!
//! Proves the full vertical stack admits in sequence:
//!
//!   RequirementProposed -> CtqAdmitted -> WorkOrderAdmitted ->
//!     SolutionManufactured (IaC + Rust + Erlang + AtomVM)
//!
//! AND the receipt chain is intact end-to-end:
//!
//!   solution_receipt.prior_receipt == work_order_receipt.bytes
//!   solution_bundle.files[*].contents.contains(work_order_receipt.hex())
//!
//! This is the recursive admission claim made concrete: every artifact
//! is admitted, and every admitted artifact carries the upstream
//! receipt that authorized it.

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, NoopPowlReplay, OntoStarAdmissionGate,
};
use open_ontologies::manufacturing::{self, ManufacturedFile, SolutionSpec};
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

const REQ_WORKFLOW: &str = "RequirementsManufacturing";
const SMFG_WORKFLOW: &str = "SolutionManufacturing";

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("recursive-admission.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn emit(store: &OcelStore, session: &str, scope: &str, stage: &str) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    let now = chrono::Utc::now().to_rfc3339();
    let event_id = format!("{session}:{n:012}:{stage}");
    store
        .emit_event(&event_id, stage, &now, session, &[], &[], Some(scope))
        .unwrap();
}

#[test]
fn full_recursive_admission_chain_from_requirement_to_manufactured_stack() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "recursive-e2e";

    // ── Layer 1: Requirements Andon ──────────────────────────────────────
    let req_scope_mgr = WorkflowScope::new(&db, session);
    let req_scope = req_scope_mgr.open(Some(REQ_WORKFLOW), None, None).unwrap();
    req_scope_mgr.close(&req_scope).unwrap();

    for stage in &[
        "requirement_proposed",
        "llm_candidate_translated",
        "ctq_admitted",
        "verification_bound",
        "negative_case_bound",
        "control_plan_bound",
        "work_order_admitted",
    ] {
        emit(&store, session, &req_scope, stage);
    }
    let observed: Vec<String> = store.observed_event_types_for_session(session).unwrap();

    let req_gate = OntoStarAdmissionGate::new(
        0.95,
        0.85,
        by_name(REQ_WORKFLOW)
            .map(|w| w.required_stages.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default(),
        "ontostar-1.0.0",
    );
    let req_powl = by_name(REQ_WORKFLOW).unwrap().powl_string;

    // 3 admissions in the Requirements layer (chain via prior_receipt).
    let req_artifact = ArtifactRef { kind: "req", bytes: b"voice" };
    let req_receipt = req_gate
        .evaluate(&req_scope, AdmissionOp::RequirementProposed, &req_artifact,
            &store, &NoopPowlReplay, session, req_powl, &observed)
        .expect("RequirementProposed admits");
    let ctq_artifact = ArtifactRef { kind: "ctq", bytes: b"ctq-canonical" };
    let ctq_receipt = req_gate
        .evaluate(&req_scope, AdmissionOp::CtqAdmitted, &ctq_artifact,
            &store, &NoopPowlReplay, session, req_powl, &observed)
        .expect("CtqAdmitted admits");
    let wo_artifact = ArtifactRef { kind: "wo", bytes: b"work-order-canonical" };
    let wo_receipt = req_gate
        .evaluate(&req_scope, AdmissionOp::WorkOrderAdmitted, &wo_artifact,
            &store, &NoopPowlReplay, session, req_powl, &observed)
        .expect("WorkOrderAdmitted admits");

    // Receipt chain inside Requirements layer.
    assert_eq!(ctq_receipt.record.prior_receipt, Some(req_receipt.bytes));
    assert_eq!(wo_receipt.record.prior_receipt, Some(ctq_receipt.bytes));

    // ── Layer 2: Solution Manufacturing ─────────────────────────────────
    // Open a SECOND scope for manufacturing — the work-order receipt
    // from layer 1 is the binding.
    let smfg_scope = req_scope_mgr.open(Some(SMFG_WORKFLOW), None, None).unwrap();
    req_scope_mgr.close(&smfg_scope).unwrap();

    let spec = SolutionSpec {
        name: "revops_pipeline".into(),
        description: "End-to-end RevOps stack manufactured from admitted CTQ".into(),
        iac_target: "aws".into(),
        region: "us-east-1".into(),
        supervisor_children: 4,
        mcu_target: "esp32".into(),
        // **The recursive binding: the manufactured stack carries the
        // upstream WorkOrderAdmitted receipt hash.**
        work_order_receipt_hash: wo_receipt.hex(),
    };

    let bundle = manufacturing::manufacture(&spec).expect("manufacture must succeed");

    // Emit the 7 SolutionManufacturing stages.
    for stage in &[
        "work_order_received",
        "architecture_decided",
        "iac_generated",
        "rust_generated",
        "erlang_generated",
        "atomvm_generated",
        "receipt_chain_sealed",
    ] {
        emit(&store, session, &smfg_scope, stage);
    }
    let observed_smfg: Vec<String> = store.observed_event_types_for_session(session).unwrap();

    // Compute canonical bundle bytes.
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

    let smfg_gate = OntoStarAdmissionGate::new(
        0.95,
        0.85,
        by_name(SMFG_WORKFLOW)
            .map(|w| w.required_stages.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default(),
        "ontostar-1.0.0",
    );
    let smfg_powl = by_name(SMFG_WORKFLOW).unwrap().powl_string;
    let smfg_artifact = ArtifactRef {
        kind: "solution-bundle",
        bytes: canonical.as_bytes(),
    };
    let smfg_receipt = smfg_gate
        .evaluate(&smfg_scope, AdmissionOp::SolutionManufactured, &smfg_artifact,
            &store, &NoopPowlReplay, session, smfg_powl, &observed_smfg)
        .expect("SolutionManufactured admits");

    // ── Recursive admission claim: receipt chain crosses layers ─────────
    // Within the same session, the SolutionManufactured receipt's
    // prior_receipt is the most recent admission — which is the
    // WorkOrderAdmitted receipt from layer 1.
    assert_eq!(
        smfg_receipt.record.prior_receipt,
        Some(wo_receipt.bytes),
        "SolutionManufactured receipt must chain to WorkOrderAdmitted receipt"
    );

    // Every manufactured file carries the upstream WorkOrderAdmitted
    // receipt hash. The entire stack is provably bound to the work
    // order.
    let wo_hex = wo_receipt.hex();
    for f in &bundle.files {
        assert!(
            f.contents.contains(&wo_hex),
            "{} missing upstream WorkOrderAdmitted receipt hex",
            f.path
        );
    }

    // The 4 targets are all populated.
    assert!(!bundle.files_for("iac").is_empty());
    assert!(!bundle.files_for("rust").is_empty());
    assert!(!bundle.files_for("erlang").is_empty());
    assert!(!bundle.files_for("atomvm").is_empty());

    // The DoD claim, materialized as a structured assertion.
    let admitted_chain_length = 4; // req + ctq + wo + smfg
    let _layer_count = 2;          // Requirements + Solution Manufacturing
    let receipt_chain_intact =
        ctq_receipt.record.prior_receipt == Some(req_receipt.bytes)
        && wo_receipt.record.prior_receipt == Some(ctq_receipt.bytes)
        && smfg_receipt.record.prior_receipt == Some(wo_receipt.bytes);
    assert!(receipt_chain_intact);
    assert_eq!(admitted_chain_length, 4);
}
