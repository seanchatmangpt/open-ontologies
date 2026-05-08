//! Adversarial: run REAL toolchains (rustc / erlc / terraform) against
//! the manufactured bundle. Tests are gated on toolchain availability;
//! they SKIP when the tool is not installed rather than fail, so CI
//! still passes on minimal images. Local development boxes with all
//! three installed get the full proof.
//!
//! These tests exist because the adversarial-audit caught the IaC
//! generator emitting Terraform JSON with an extraneous top-level key
//! that fails `terraform validate`. The audit was right; the previous
//! tests were structural-only and could not see it.

use open_ontologies::manufacturing::{manufacture, SolutionSpec};
use std::process::Command;
use tempfile::tempdir;

fn ok_spec(name: &str) -> SolutionSpec {
    SolutionSpec {
        name: name.into(),
        description: "adversarial real-toolchain test".into(),
        iac_target: "aws".into(),
        region: "us-east-1".into(),
        supervisor_children: 4,
        mcu_target: "esp32".into(),
        work_order_receipt_hash: "a".repeat(64),
    }
}

fn write_bundle(spec: &SolutionSpec) -> tempfile::TempDir {
    let bundle = manufacture(spec).expect("manufacture");
    let dir = tempdir().unwrap();
    for f in &bundle.files {
        let full = dir.path().join(&f.path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, &f.contents).unwrap();
    }
    dir
}

fn tool_available(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn rust_crate_compiles_under_real_cargo_check() {
    if !tool_available("cargo") {
        eprintln!("SKIP: cargo not on PATH");
        return;
    }
    let dir = write_bundle(&ok_spec("real_rust"));
    let rust = dir.path().join("rust");
    let out = Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .current_dir(&rust)
        .output()
        .expect("spawn cargo");
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        panic!(
            "generated Rust crate failed `cargo check`:\n{stderr}\n\nDir: {}",
            rust.display()
        );
    }
}

#[test]
fn erlang_modules_compile_under_real_erlc() {
    if !tool_available("erlc") {
        eprintln!("SKIP: erlc not on PATH");
        return;
    }
    let dir = write_bundle(&ok_spec("real_erlang"));
    let src = dir.path().join("erlang/src");
    let ebin = dir.path().join("erlang/ebin");
    std::fs::create_dir_all(&ebin).unwrap();
    for entry in std::fs::read_dir(&src).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().and_then(|s| s.to_str()) != Some("erl") {
            continue;
        }
        let out = Command::new("erlc")
            .arg("-o")
            .arg(&ebin)
            .arg(&p)
            .output()
            .expect("spawn erlc");
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            panic!(
                "generated Erlang module {} failed erlc:\n{stderr}",
                p.display()
            );
        }
    }
}

#[test]
fn atomvm_module_compiles_under_real_erlc() {
    if !tool_available("erlc") {
        eprintln!("SKIP: erlc not on PATH");
        return;
    }
    let dir = write_bundle(&ok_spec("real_atomvm"));
    let avm_dir = dir.path().join("atomvm");
    let entries: Vec<_> = std::fs::read_dir(&avm_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("erl"))
        .collect();
    assert!(!entries.is_empty(), "atomvm dir must contain an .erl module");
    for entry in entries {
        let p = entry.path();
        let out = Command::new("erlc")
            .arg("-o")
            .arg(&avm_dir)
            .arg(&p)
            .output()
            .expect("spawn erlc");
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            panic!(
                "AtomVM module {} failed erlc:\n{stderr}",
                p.display()
            );
        }
    }
}

#[test]
fn terraform_validate_admits_generated_iac() {
    if !tool_available("terraform") {
        eprintln!("SKIP: terraform not on PATH");
        return;
    }
    let dir = write_bundle(&ok_spec("real_terraform"));
    let iac = dir.path().join("iac");

    // `terraform init -backend=false` is required before `validate`
    // so providers can be resolved. We allow init network failures
    // (some CI envs block egress) but treat them as SKIP, not FAIL.
    let init = Command::new("terraform")
        .arg("init")
        .arg("-backend=false")
        .arg("-input=false")
        .current_dir(&iac)
        .output()
        .expect("spawn terraform init");
    if !init.status.success() {
        let stderr = String::from_utf8_lossy(&init.stderr);
        if stderr.contains("Failed to query available provider")
            || stderr.contains("network is unreachable")
            || stderr.contains("could not connect")
        {
            eprintln!("SKIP: terraform init network failure (offline?)");
            return;
        }
        panic!("terraform init failed:\n{stderr}");
    }

    // `terraform validate` is the actual contract: bundle must parse
    // as Terraform, NOT just as JSON.
    let out = Command::new("terraform")
        .arg("validate")
        .arg("-no-color")
        .current_dir(&iac)
        .output()
        .expect("spawn terraform validate");
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        panic!(
            "terraform validate FAILED on generated IaC:\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
    }
}

#[test]
fn iac_sidecar_receipt_is_separate_from_terraform_files() {
    // No tool needed — pure structural assertion against the spec
    // shape. The audit-fix puts the receipt in iac/.ontostar-receipt.json
    // (a sidecar), NOT inside any .tf.json file.
    let bundle = manufacture(&ok_spec("real_sidecar")).expect("manufacture");
    let mut have_sidecar = false;
    for f in bundle.files_for("iac") {
        if f.path == "iac/.ontostar-receipt.json" {
            have_sidecar = true;
            continue;
        }
        // Each .tf.json is parseable JSON with NO _ontostar_receipt key.
        let v: serde_json::Value =
            serde_json::from_str(&f.contents).expect("tf.json parses");
        let obj = v.as_object().expect("tf.json is an object");
        assert!(
            !obj.contains_key("_ontostar_receipt"),
            "{} carries an extraneous _ontostar_receipt key",
            f.path
        );
        for k in obj.keys() {
            assert!(
                matches!(
                    k.as_str(),
                    "terraform" | "provider" | "resource" | "variable"
                    | "output" | "data" | "module" | "locals"
                ),
                "{} has non-Terraform top-level key `{}`",
                f.path,
                k
            );
        }
    }
    assert!(have_sidecar, "iac/.ontostar-receipt.json sidecar missing");
}
