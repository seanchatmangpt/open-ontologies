//! Deny-path coverage for `manufacturing::validators::validate_bundle`.
//!
//! Each test calls `manufacture(&ok_spec()).unwrap()` to obtain a known-good
//! bundle, then mutates exactly one byte/file/header to exercise one
//! `DefectClass` arm. Asserts the typed defect is returned.

use open_ontologies::defects::DefectClass;
use open_ontologies::manufacturing::{
    manufacture, manufacture_with_override, validators::validate_bundle, SolutionSpec,
};

fn ok_spec() -> SolutionSpec {
    SolutionSpec {
        name: "revops_pipeline".into(),
        description: "Fortune-5 RevOps pipeline manufactured stack".into(),
        iac_target: "aws".into(),
        region: "us-east-1".into(),
        supervisor_children: 4,
        mcu_target: "esp32".into(),
        work_order_receipt_hash: "0".repeat(64),
    }
}

// ─── IacInvalid ───────────────────────────────────────────────────────────

#[test]
fn iac_invalid_when_sidecar_missing_artifact_hash() {
    let mut bundle = manufacture(&ok_spec()).expect("manufacture ok");
    let sidecar = bundle
        .files
        .iter_mut()
        .find(|f| f.path == "iac/.ontostar-receipt.json")
        .expect("sidecar present");
    let mut v: serde_json::Value =
        serde_json::from_str(&sidecar.contents).expect("sidecar parses");
    v.as_object_mut().unwrap().remove("artifact_hash");
    sidecar.contents = serde_json::to_string_pretty(&v).unwrap();

    match validate_bundle(&bundle) {
        Err(DefectClass::IacInvalid { reason }) => {
            assert!(
                reason.contains("artifact_hash"),
                "expected reason to mention artifact_hash, got: {reason}"
            );
        }
        other => panic!("expected IacInvalid, got {other:?}"),
    }
}

#[test]
fn iac_invalid_rejects_inline_receipt_in_tf_json() {
    let mut bundle = manufacture(&ok_spec()).expect("manufacture ok");
    let main_tf = bundle
        .files
        .iter_mut()
        .find(|f| f.path.ends_with("main.tf.json"))
        .expect("main.tf.json present");
    let mut v: serde_json::Value =
        serde_json::from_str(&main_tf.contents).expect("main.tf.json parses");
    v.as_object_mut().unwrap().insert(
        "_ontostar_receipt".to_string(),
        serde_json::json!({"injected": true}),
    );
    main_tf.contents = serde_json::to_string_pretty(&v).unwrap();

    match validate_bundle(&bundle) {
        Err(DefectClass::IacInvalid { reason }) => {
            assert!(
                reason.contains("_ontostar_receipt"),
                "expected reason to mention _ontostar_receipt, got: {reason}"
            );
        }
        other => panic!("expected IacInvalid, got {other:?}"),
    }
}

// ─── RustInvalid ──────────────────────────────────────────────────────────

#[test]
fn rust_invalid_when_main_rs_lacks_fn_main() {
    let mut bundle = manufacture(&ok_spec()).expect("manufacture ok");
    let main_rs = bundle
        .files
        .iter_mut()
        .find(|f| f.path.ends_with("main.rs"))
        .expect("main.rs present");
    // Strip out `fn main` while keeping the receipt header so we hit
    // RustInvalid (not ManufacturingChainBroken).
    main_rs.contents = main_rs.contents.replace("fn main", "fn not_main");

    match validate_bundle(&bundle) {
        Err(DefectClass::RustInvalid { reason }) => {
            assert!(
                reason.contains("fn main") || reason.contains("main.rs"),
                "expected reason to mention main, got: {reason}"
            );
        }
        other => panic!("expected RustInvalid, got {other:?}"),
    }
}

#[test]
fn rust_invalid_when_cargo_missing_package_section() {
    let mut bundle = manufacture(&ok_spec()).expect("manufacture ok");
    let cargo = bundle
        .files
        .iter_mut()
        .find(|f| f.path.ends_with("Cargo.toml"))
        .expect("Cargo.toml present");
    cargo.contents = cargo.contents.replace("[package]", "[notpackage]");

    match validate_bundle(&bundle) {
        Err(DefectClass::RustInvalid { reason }) => {
            assert!(
                reason.contains("[package]"),
                "expected reason to mention [package], got: {reason}"
            );
        }
        other => panic!("expected RustInvalid, got {other:?}"),
    }
}

#[test]
fn rust_invalid_when_lib_missing_solution_name_fn() {
    let mut bundle = manufacture(&ok_spec()).expect("manufacture ok");
    let lib = bundle
        .files
        .iter_mut()
        .find(|f| f.path.ends_with("lib.rs"))
        .expect("lib.rs present");
    lib.contents = lib
        .contents
        .replace("pub fn manufactured_solution_name", "pub fn renamed_solution");

    match validate_bundle(&bundle) {
        Err(DefectClass::RustInvalid { reason }) => {
            assert!(
                reason.contains("manufactured_solution_name"),
                "expected reason to mention manufactured_solution_name, got: {reason}"
            );
        }
        other => panic!("expected RustInvalid, got {other:?}"),
    }
}

// ─── ManufacturingChainBroken ─────────────────────────────────────────────

#[test]
fn manufacturing_chain_broken_when_receipt_header_stripped() {
    let mut bundle = manufacture(&ok_spec()).expect("manufacture ok");
    // Strip the inline `ostar-artifact-hash:` header from a non-tf.json
    // file (e.g. main.rs or any erlang file). The validator must refuse
    // to bind that file and surface ManufacturingChainBroken.
    let target = bundle
        .files
        .iter_mut()
        .find(|f| {
            f.contents.contains("ostar-artifact-hash:")
                && !f.path.ends_with(".tf.json")
                && f.path != "iac/.ontostar-receipt.json"
        })
        .expect("a non-tf.json file with the inline header");
    // Remove the entire receipt-header block: any line starting with
    // `<prefix> ostar-` is dropped.
    let stripped: String = target
        .contents
        .lines()
        .filter(|line| !line.contains("ostar-"))
        .collect::<Vec<_>>()
        .join("\n");
    target.contents = stripped;

    match validate_bundle(&bundle) {
        Err(DefectClass::ManufacturingChainBroken { missing }) => {
            assert!(
                missing.contains("receipt binding"),
                "expected missing to mention receipt binding, got: {missing}"
            );
        }
        other => panic!("expected ManufacturingChainBroken, got {other:?}"),
    }
}

#[test]
fn manufacturing_chain_broken_when_work_order_hash_absent() {
    let mut bundle = manufacture(&ok_spec()).expect("manufacture ok");
    let wo_hash = bundle.spec.work_order_receipt_hash.clone();
    // Wipe the WO hash from every file that mentions it. We have to
    // keep a receipt-binding header on each file (otherwise we trip
    // the per-file binding check first), so we replace ONLY the
    // 64-char WO hash literal with a placeholder of equal length.
    let placeholder: String = "f".repeat(wo_hash.len());
    for f in &mut bundle.files {
        f.contents = f.contents.replace(&wo_hash, &placeholder);
    }

    match validate_bundle(&bundle) {
        Err(DefectClass::ManufacturingChainBroken { missing }) => {
            assert!(
                missing.contains("work-order receipt hash"),
                "expected missing to mention work-order receipt hash, got: {missing}"
            );
        }
        other => panic!("expected ManufacturingChainBroken, got {other:?}"),
    }
}

// ─── GeneratorEmpty ───────────────────────────────────────────────────────

#[test]
fn generator_empty_iac() {
    let res = manufacture_with_override(&ok_spec(), "iac");
    match res {
        Err(DefectClass::GeneratorEmpty { target }) => {
            assert_eq!(target, "iac");
        }
        other => panic!("expected GeneratorEmpty{{target=iac}}, got {other:?}"),
    }
}
