//! Level-5 portability codegen sabotage test.
//!
//! Asserts that `receipts::inject_comment_header` correctly stamps OntoStar
//! receipt headers onto generated source files, using the right comment-prefix
//! per file extension, while leaving unsupported extensions untouched.

use open_ontologies::production_record::{hex32_pub, ProductionRecord};
use open_ontologies::receipts::{self, Receipt};
use std::fs;
use std::path::Path;

fn build_test_receipt(artifact_bytes: &[u8]) -> Receipt {
    let artifact_hash = *blake3::hash(artifact_bytes).as_bytes();
    let record = ProductionRecord {
        artifact_hash,
        scope_token: "scope-codegen-test".to_string(),
        declared_powl_hash: [0u8; 32],
        ocel_canonical_hash: [0u8; 32],
        conformance_run_id: "test-run-codegen".to_string(),
        gate_config_hash: [0u8; 32],
        production_law_version: "ontostar-1.0.0".to_string(),
        defects_taxonomy_version: "ontostar-defects-1.0.0".to_string(),
        gates_passed: vec!["WorkflowDeclared".into()],
        gates_refused: vec![],
        prior_receipt: None,
    };
    receipts::build(record)
}

fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(path, bytes).expect("write file");
}

fn assert_header_lines_present(text: &str, prefix: &str, receipt: &Receipt) {
    for tag in &[
        "ostar-production-law:",
        "ostar-defects-taxonomy:",
        "ostar-receipt-hash:",
        "ostar-artifact-hash:",
        "ostar-scope-token:",
        "ostar-prior-receipt:",
    ] {
        let needle = format!("{} {}", prefix, tag);
        assert!(
            text.contains(&needle),
            "expected `{}` line in header, got:\n{}",
            needle,
            text
        );
    }
    assert!(
        text.contains(&receipt.hex()),
        "header must embed the receipt hex {}",
        receipt.hex()
    );
    assert!(
        text.contains(&hex32_pub(&receipt.record.artifact_hash)),
        "header must embed the artifact hash hex"
    );
}

#[test]
fn inject_comment_header_stamps_supported_extensions_and_skips_others() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("out");

    // Diverse extensions:
    let rs_path = root.join("foo.rs");
    let py_path = root.join("bar.py");
    let ts_path = root.join("sub").join("baz.ts");
    let ttl_path = root.join("data.ttl");
    let png_path = root.join("image.png");
    let no_ext_path = root.join("README");

    let rs_body = b"pub fn hello() {}\n";
    let py_body = b"def hello():\n    pass\n";
    let ts_body = b"export const x = 1;\n";
    let ttl_body = b"@prefix : <urn:t:> . :a :p :b .\n";
    let png_body: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0xFF];
    let no_ext_body = b"this is a plain readme\n";

    write(&rs_path, rs_body);
    write(&py_path, py_body);
    write(&ts_path, ts_body);
    write(&ttl_path, ttl_body);
    write(&png_path, png_body);
    write(&no_ext_path, no_ext_body);

    let receipt = build_test_receipt(b"some-artifact-placeholder");

    // Supported extensions => Ok(true), file rewritten with header.
    assert_eq!(
        receipts::inject_comment_header(&rs_path, &receipt).expect("rs ok"),
        true,
        ".rs should be stamped"
    );
    assert_eq!(
        receipts::inject_comment_header(&py_path, &receipt).expect("py ok"),
        true,
        ".py should be stamped"
    );
    assert_eq!(
        receipts::inject_comment_header(&ts_path, &receipt).expect("ts ok"),
        true,
        ".ts should be stamped"
    );
    assert_eq!(
        receipts::inject_comment_header(&ttl_path, &receipt).expect("ttl ok"),
        true,
        ".ttl should be stamped"
    );

    // Unsupported => Ok(false), file untouched.
    assert_eq!(
        receipts::inject_comment_header(&png_path, &receipt).expect("png ok"),
        false,
        ".png should NOT be stamped"
    );
    assert_eq!(
        receipts::inject_comment_header(&no_ext_path, &receipt).expect("no-ext ok"),
        false,
        "extensionless file should NOT be stamped"
    );

    // Verify .rs uses `//` prefix.
    let rs_after = fs::read_to_string(&rs_path).expect("read rs");
    assert!(
        rs_after.starts_with("// ostar-production-law:"),
        ".rs must begin with `// ostar-production-law:`, got: {:?}",
        &rs_after[..rs_after.len().min(80)]
    );
    assert_header_lines_present(&rs_after, "//", &receipt);
    assert!(
        rs_after.contains("pub fn hello() {}"),
        ".rs body must be preserved below the header"
    );

    // Verify .py uses `#` prefix.
    let py_after = fs::read_to_string(&py_path).expect("read py");
    assert!(
        py_after.starts_with("# ostar-production-law:"),
        ".py must begin with `# ostar-production-law:`, got: {:?}",
        &py_after[..py_after.len().min(80)]
    );
    assert_header_lines_present(&py_after, "#", &receipt);
    assert!(py_after.contains("def hello():"), ".py body preserved");

    // Verify .ts uses `//` prefix.
    let ts_after = fs::read_to_string(&ts_path).expect("read ts");
    assert!(
        ts_after.starts_with("// ostar-production-law:"),
        ".ts must begin with `// ostar-production-law:`"
    );
    assert_header_lines_present(&ts_after, "//", &receipt);

    // Verify .ttl uses `#` prefix.
    let ttl_after = fs::read_to_string(&ttl_path).expect("read ttl");
    assert!(
        ttl_after.starts_with("# ostar-production-law:"),
        ".ttl must begin with `# ostar-production-law:`"
    );
    assert_header_lines_present(&ttl_after, "#", &receipt);

    // Verify .png and README are byte-identical to before.
    let png_after = fs::read(&png_path).expect("read png");
    assert_eq!(png_after, png_body, ".png must be byte-identical");
    let no_ext_after = fs::read(&no_ext_path).expect("read README");
    assert_eq!(no_ext_after, no_ext_body, "README must be byte-identical");
}

#[test]
fn inject_comment_header_is_not_idempotent_double_stamp_prepends_twice() {
    // The function does not detect an existing header — calling it twice
    // prepends two headers. Idempotence is the caller's responsibility, and
    // this test pins that contract so a future "helpful" change doesn't
    // silently make the function dedupe.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("twice.rs");
    let body = b"fn main() {}\n";
    write(&path, body);

    let receipt = build_test_receipt(b"twice-artifact");

    assert!(receipts::inject_comment_header(&path, &receipt).expect("first stamp"));
    let after_first = fs::read_to_string(&path).expect("read first");
    let count_first = after_first.matches("ostar-receipt-hash:").count();
    assert_eq!(count_first, 1, "first stamp emits exactly one header");

    assert!(receipts::inject_comment_header(&path, &receipt).expect("second stamp"));
    let after_second = fs::read_to_string(&path).expect("read second");
    let count_second = after_second.matches("ostar-receipt-hash:").count();
    assert_eq!(
        count_second, 2,
        "second stamp prepends another header (NOT idempotent)"
    );
    assert!(
        after_second.contains("fn main() {}"),
        "original body still present after double stamp"
    );
}

#[test]
fn inject_comment_header_preserves_existing_file_contents_below_header() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("keep.rs");
    let original = "pub fn main() {}\n// some trailing comment\n";
    write(&path, original.as_bytes());

    let receipt = build_test_receipt(b"keep-artifact");
    assert!(receipts::inject_comment_header(&path, &receipt).expect("stamp"));

    let after = fs::read_to_string(&path).expect("read after");
    // Header must be at the top.
    assert!(
        after.starts_with("// ostar-production-law:"),
        "header must be at the very top"
    );
    // Original content must appear below the header, unmodified.
    assert!(
        after.ends_with(original),
        "original body must be preserved verbatim at the tail. got tail: {:?}",
        &after[after.len().saturating_sub(original.len() + 20)..]
    );
    // And there must be exactly one header block.
    assert_eq!(
        after.matches("ostar-receipt-hash:").count(),
        1,
        "single stamp = single header"
    );
}
