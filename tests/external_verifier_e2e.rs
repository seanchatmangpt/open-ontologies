//! Phase 9 — external verifier end-to-end tests.
//!
//! Exercises the four file formats (Rust / Erlang / TTL / IaC sidecar)
//! and the four verdict variants (Admitted / Tampered / Orphaned /
//! UnknownChain), plus chain walking and ASCII rendering.

use open_ontologies::manufacturing::{self, SolutionSpec};
use open_ontologies::production_record::{hex32_pub, ProductionRecord};
use open_ontologies::receipts;
use open_ontologies::state::StateDb;
use open_ontologies::verify::{
    render_chain_ascii, verify_artifact, verify_iac_bundle, walk_receipt_chain, Verdict,
};
use tempfile::tempdir;

fn ok_spec() -> SolutionSpec {
    SolutionSpec {
        name: "verifier_e2e".into(),
        description: "Phase 9 verifier test".into(),
        iac_target: "aws".into(),
        region: "us-east-1".into(),
        supervisor_children: 4,
        mcu_target: "esp32".into(),
        work_order_receipt_hash: "a".repeat(64),
    }
}

/// Materialize the bundle to disk under `root` so we can hand the
/// verifier a real Path.
fn write_bundle_to_disk(
    bundle: &manufacturing::SolutionBundle,
    root: &std::path::Path,
) {
    for f in &bundle.files {
        let dst = root.join(&f.path);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&dst, &f.contents).unwrap();
    }
}

#[test]
fn admits_clean_rust_file() {
    let dir = tempdir().unwrap();
    let bundle = manufacturing::manufacture(&ok_spec()).unwrap();
    write_bundle_to_disk(&bundle, dir.path());

    let p = dir.path().join("rust/src/lib.rs");
    let v = verify_artifact(&p, None);
    assert!(matches!(v, Verdict::Admitted { .. }), "got {v:?}");
}

#[test]
fn admits_clean_erlang_file() {
    let dir = tempdir().unwrap();
    let bundle = manufacturing::manufacture(&ok_spec()).unwrap();
    write_bundle_to_disk(&bundle, dir.path());

    // Find any .erl file (sup/app/worker/atomvm).
    let erl = bundle
        .files
        .iter()
        .find(|f| f.path.ends_with(".erl"))
        .expect("erlang file");
    let p = dir.path().join(&erl.path);
    let v = verify_artifact(&p, None);
    assert!(matches!(v, Verdict::Admitted { .. }), "got {v:?}");
}

#[test]
fn admits_clean_ttl_file_with_receipts_header() {
    // TTL files use the `# ostar-` header form. We construct one
    // manually here using `receipts::ttl_header` — the same path
    // that `portability_save.rs` exercises end-to-end.
    let dir = tempdir().unwrap();
    let body = "@prefix ex: <http://example.com/> .\nex:a ex:p ex:b .\n";

    // Build a synthetic receipt whose `artifact_hash` is the BLAKE3
    // of the (header-stripped) body. This is what
    // `inject_comment_header` would do in production.
    let body_hash = blake3::hash(body.as_bytes());
    let record = ProductionRecord {
        artifact_hash: *body_hash.as_bytes(),
        scope_token: "ttl-scope".into(),
        declared_powl_hash: [0u8; 32],
        ocel_canonical_hash: [0u8; 32],
        conformance_run_id: "run-1".into(),
        gate_config_hash: [0u8; 32],
        production_law_version: "ontostar-1.0.0".into(),
        defects_taxonomy_version: open_ontologies::defects::DEFECTS_TAXONOMY_VERSION.into(),
        gates_passed: vec!["g".into()],
        gates_refused: vec![],
        prior_receipt: None,
    };
    let receipt = receipts::build(record);
    let header = receipts::ttl_header(&receipt);

    let p = dir.path().join("ont.ttl");
    let mut full = header.clone();
    full.push_str(body);
    std::fs::write(&p, &full).unwrap();

    let v = verify_artifact(&p, None);
    assert!(
        matches!(v, Verdict::Admitted { .. }),
        "TTL header round-trip should admit, got {v:?}"
    );
}

#[test]
fn admits_iac_bundle_via_sidecar() {
    let dir = tempdir().unwrap();
    let bundle = manufacturing::manufacture(&ok_spec()).unwrap();
    write_bundle_to_disk(&bundle, dir.path());

    let iac_dir = dir.path().join("iac");
    let v = verify_iac_bundle(&iac_dir, None);
    assert!(matches!(v, Verdict::Admitted { .. }), "got {v:?}");

    // Direct .tf.json verification dispatches to the same sidecar path.
    let tf = dir.path().join("iac/main.tf.json");
    let v2 = verify_artifact(&tf, None);
    assert!(matches!(v2, Verdict::Admitted { .. }), "got {v2:?}");
}

#[test]
fn detects_tampered_body_byte_in_rust_file() {
    let dir = tempdir().unwrap();
    let bundle = manufacturing::manufacture(&ok_spec()).unwrap();
    write_bundle_to_disk(&bundle, dir.path());

    let p = dir.path().join("rust/src/lib.rs");
    let original = std::fs::read_to_string(&p).unwrap();
    // Append a single comment line AFTER the receipt header — body
    // changed, header untouched.
    let tampered = format!("{original}\n// EVIL INJECTED LINE\n");
    std::fs::write(&p, tampered).unwrap();

    let v = verify_artifact(&p, None);
    match v {
        Verdict::Tampered { mismatch_at, expected, actual } => {
            assert!(mismatch_at.contains("lib.rs"));
            assert_ne!(expected, actual);
            assert_eq!(expected.len(), 64);
            assert_eq!(actual.len(), 64);
        }
        other => panic!("expected Tampered, got {other:?}"),
    }
}

#[test]
fn detects_tampered_artifact_hash_line() {
    let dir = tempdir().unwrap();
    let bundle = manufacturing::manufacture(&ok_spec()).unwrap();
    write_bundle_to_disk(&bundle, dir.path());

    let p = dir.path().join("rust/src/lib.rs");
    let original = std::fs::read_to_string(&p).unwrap();
    // Replace the artifact-hash line with a bogus value of the same
    // length. The body is unchanged but the header now lies.
    let tampered: String = original
        .lines()
        .map(|l| {
            if l.starts_with("// ostar-artifact-hash: ") {
                format!("// ostar-artifact-hash: {}", "f".repeat(64))
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&p, tampered).unwrap();

    let v = verify_artifact(&p, None);
    assert!(matches!(v, Verdict::Tampered { .. }), "got {v:?}");
}

#[test]
fn detects_tampered_iac_sidecar_artifact_hash() {
    let dir = tempdir().unwrap();
    let bundle = manufacturing::manufacture(&ok_spec()).unwrap();
    write_bundle_to_disk(&bundle, dir.path());

    let sidecar_path = dir.path().join("iac/.ontostar-receipt.json");
    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&sidecar_path).unwrap()).unwrap();
    json["artifact_hash"] = serde_json::Value::String("0".repeat(64));
    std::fs::write(
        &sidecar_path,
        serde_json::to_string_pretty(&json).unwrap(),
    )
    .unwrap();

    let v = verify_iac_bundle(&dir.path().join("iac"), None);
    assert!(matches!(v, Verdict::Tampered { .. }), "got {v:?}");
}

#[test]
fn unknown_chain_when_no_header_present() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("plain.rs");
    std::fs::write(&p, "fn main() {}\n").unwrap();
    let v = verify_artifact(&p, None);
    assert!(matches!(v, Verdict::UnknownChain { .. }), "got {v:?}");
}

#[test]
fn walks_receipt_chain_in_correct_order_and_renders_ascii() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("chain.db");
    let db = StateDb::open(&db_path).unwrap();
    let session = "chain-test";

    // Insert three receipts, each chained to the prior. The first has
    // no prior. We persist via `receipts::persist` and `latest_for_session`.
    let mk_record = |i: u8, prior: Option<[u8; 32]>| ProductionRecord {
        artifact_hash: [i; 32],
        scope_token: format!("scope-{i}"),
        declared_powl_hash: [0u8; 32],
        ocel_canonical_hash: [0u8; 32],
        conformance_run_id: format!("run-{i}"),
        gate_config_hash: [0u8; 32],
        production_law_version: "ontostar-1.0.0".into(),
        defects_taxonomy_version: open_ontologies::defects::DEFECTS_TAXONOMY_VERSION
            .into(),
        gates_passed: vec!["g".into()],
        gates_refused: vec![],
        prior_receipt: prior,
    };

    let r1 = receipts::build(mk_record(1, None));
    receipts::persist(&r1, &db, session).unwrap();
    let r2 = receipts::build(mk_record(2, Some(r1.bytes)));
    receipts::persist(&r2, &db, session).unwrap();
    let r3 = receipts::build(mk_record(3, Some(r2.bytes)));
    receipts::persist(&r3, &db, session).unwrap();

    let chain = walk_receipt_chain(&db, &r3.bytes);
    assert_eq!(chain.len(), 3, "expected 3 links, got {}", chain.len());
    assert_eq!(chain[0].receipt_hash, hex32_pub(&r3.bytes));
    assert_eq!(chain[1].receipt_hash, hex32_pub(&r2.bytes));
    assert_eq!(chain[2].receipt_hash, hex32_pub(&r1.bytes));
    assert!(chain[2].prior.is_none(), "origin must have no prior");

    let ascii = render_chain_ascii(&chain);
    eprintln!("=== ASCII CHAIN ===\n{ascii}=== END ===");
    assert!(ascii.contains("↓"), "ASCII tree must show downward arrows");
    assert!(ascii.contains("origin"), "must mark origin link");
    assert!(ascii.contains("seq=3"));
    assert!(ascii.contains("seq=1"));
}

#[test]
fn walks_chain_terminates_on_missing_link() {
    // Insert only r2 (which references a missing r1). Chain should
    // terminate at r2 and the renderer should mark it as missing.
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("orphan.db");
    let db = StateDb::open(&db_path).unwrap();
    let phantom_prior = [9u8; 32];
    let r2 = receipts::build(ProductionRecord {
        artifact_hash: [2u8; 32],
        scope_token: "orphan-scope".into(),
        declared_powl_hash: [0u8; 32],
        ocel_canonical_hash: [0u8; 32],
        conformance_run_id: "run".into(),
        gate_config_hash: [0u8; 32],
        production_law_version: "ontostar-1.0.0".into(),
        defects_taxonomy_version: open_ontologies::defects::DEFECTS_TAXONOMY_VERSION
            .into(),
        gates_passed: vec!["g".into()],
        gates_refused: vec![],
        prior_receipt: Some(phantom_prior),
    });
    receipts::persist(&r2, &db, "orphan-sess").unwrap();

    let chain = walk_receipt_chain(&db, &r2.bytes);
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].prior, Some(hex32_pub(&phantom_prior)));
    let ascii = render_chain_ascii(&chain);
    assert!(
        ascii.contains("MISSING"),
        "renderer must flag missing prior, got:\n{ascii}"
    );
}
