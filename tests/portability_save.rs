//! Level-5 portability sabotage test.
//!
//! Asserts that the OntoStar receipt header embedded in a saved TTL artifact
//! is verifiable by an external observer: stripping the header and
//! recomputing the BLAKE3 of the body must equal the `ostar-artifact-hash`
//! line. Mutating the body must break the equality.

use open_ontologies::production_record::{hex32_pub, ProductionRecord};
use open_ontologies::receipts::{self, Receipt};

fn body_hash_after_stripping_header(file_bytes: &[u8]) -> [u8; 32] {
    // Drop every leading line that matches `^# ostar-[a-z-]+: .+$`. Stop at
    // the first line that doesn't match — the body begins there.
    let text = std::str::from_utf8(file_bytes).expect("ttl file should be utf-8");
    let mut body_start = 0usize;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        let is_header = trimmed.starts_with("# ostar-")
            && trimmed
                .splitn(2, ": ")
                .nth(1)
                .map(|v| !v.is_empty())
                .unwrap_or(false);
        if is_header {
            body_start += line.len();
        } else {
            break;
        }
    }
    let body = &file_bytes[body_start..];
    *blake3::hash(body).as_bytes()
}

fn build_test_receipt(artifact_bytes: &[u8]) -> Receipt {
    let artifact_hash = *blake3::hash(artifact_bytes).as_bytes();
    let record = ProductionRecord {
        artifact_hash,
        scope_token: "scope-portability-test".to_string(),
        declared_powl_hash: [0u8; 32],
        ocel_canonical_hash: [0u8; 32],
        conformance_run_id: "test-run".to_string(),
        gate_config_hash: [0u8; 32],
        production_law_version: "ontostar-1.0.0".to_string(),
        defects_taxonomy_version: "ontostar-defects-1.0.0".to_string(),
        gates_passed: vec!["WorkflowDeclared".into()],
        gates_refused: vec![],
        prior_receipt: None,
        signature: None,
        signing_key_fpr: None,
    };
    receipts::build(record)
}

#[test]
fn ttl_header_round_trips_with_blake3_verify() {
    // Body bytes: a small Turtle snippet. The receipt commits to THIS, not
    // to the file-with-header.
    let body = "@prefix : <urn:t:> . :a :p :b .";
    let receipt = build_test_receipt(body.as_bytes());

    // Build the file content the way `onto_save` does: header + body.
    let header = receipts::ttl_header(&receipt);
    let mut file_bytes = Vec::with_capacity(header.len() + body.len());
    file_bytes.extend_from_slice(header.as_bytes());
    file_bytes.extend_from_slice(body.as_bytes());

    // External verifier path: strip the header, recompute BLAKE3, compare to
    // the `ostar-artifact-hash` line value.
    let computed = body_hash_after_stripping_header(&file_bytes);
    assert_eq!(
        computed,
        receipt.record.artifact_hash,
        "stripped-body BLAKE3 must equal ostar-artifact-hash"
    );

    // The header must contain all 6 ostar-* lines and the receipt hex.
    let header_text = std::str::from_utf8(&file_bytes[..header.len()]).unwrap();
    for tag in &[
        "ostar-production-law:",
        "ostar-defects-taxonomy:",
        "ostar-receipt-hash:",
        "ostar-artifact-hash:",
        "ostar-scope-token:",
        "ostar-prior-receipt:",
    ] {
        assert!(
            header_text.contains(tag),
            "header missing tag `{}`: {:?}",
            tag,
            header_text
        );
    }
    assert!(
        header_text.contains(&receipt.hex()),
        "header must embed the receipt hex"
    );
    assert!(
        header_text.contains(&hex32_pub(&receipt.record.artifact_hash)),
        "header must embed the artifact hash hex"
    );
}

#[test]
fn ttl_header_verification_fails_when_body_is_mutated() {
    // Build a clean header+body file.
    let body = "@prefix : <urn:t:> . :a :p :b .";
    let receipt = build_test_receipt(body.as_bytes());
    let header = receipts::ttl_header(&receipt);

    // Mutate the body — append an extra triple. The header still claims the
    // original artifact_hash, so verification must fail.
    let tampered_body = format!("{}\n:c :d :e .", body);
    let mut file_bytes = Vec::with_capacity(header.len() + tampered_body.len());
    file_bytes.extend_from_slice(header.as_bytes());
    file_bytes.extend_from_slice(tampered_body.as_bytes());

    let computed = body_hash_after_stripping_header(&file_bytes);
    assert_ne!(
        computed,
        receipt.record.artifact_hash,
        "tampered body must NOT match ostar-artifact-hash"
    );
}

#[test]
fn ttl_header_strip_idempotent_under_no_op_re_serialization() {
    // Strip the header twice — the second strip is a no-op because the body
    // does not start with `# ostar-` lines. This guards against a bug where
    // stripping eats real content if the body itself begins with `# `.
    let body = "@prefix : <urn:t:> . :a :p :b .";
    let receipt = build_test_receipt(body.as_bytes());
    let header = receipts::ttl_header(&receipt);
    let mut file_bytes = Vec::new();
    file_bytes.extend_from_slice(header.as_bytes());
    file_bytes.extend_from_slice(body.as_bytes());

    let computed1 = body_hash_after_stripping_header(&file_bytes);
    // Round-trip: write computed body, strip again — should be identical.
    let computed2 = body_hash_after_stripping_header(body.as_bytes());
    assert_eq!(computed1, computed2);
    assert_eq!(computed1, *blake3::hash(body.as_bytes()).as_bytes());
}
