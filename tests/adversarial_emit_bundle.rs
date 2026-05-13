//! Adversarial: write the manufactured bundle to /tmp so external
//! toolchains (rustc, erlc, terraform) can be run against it.

use open_ontologies::manufacturing::{manufacture, SolutionSpec};

#[test]
#[ignore] // run on demand: cargo test --test adversarial_emit_bundle -- --ignored --nocapture
fn emit_bundle_to_tmp() {
    let spec = SolutionSpec {
        name: "audit_test".into(),
        description: "audit harness".into(),
        iac_target: "aws".into(),
        region: "us-east-1".into(),
        supervisor_children: 4,
        mcu_target: "esp32".into(),
        work_order_receipt_hash: "a".repeat(64),
    };
    let bundle = manufacture(&spec).expect("manufacture");
    let root = std::path::PathBuf::from("/tmp/adversarial-mfg");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for f in &bundle.files {
        let full = root.join(&f.path);
        if let Some(p) = full.parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(&full, &f.contents).unwrap();
        println!("wrote {} ({} bytes)", full.display(), f.contents.len());
    }
}
