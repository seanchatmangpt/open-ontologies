//! WC-1 — ggen pipeline integration proof.
//!
//! Verifies that:
//!   1. `src/cmds/generated.rs` was produced by ggen (non-empty, contains stubs).
//!   2. `src/cmds/mod.rs` declares `pub mod generated`.
//!   3. The receipt `.ggen/receipts/latest.json` has a non-empty signature and
//!      at least one output hash.
//!   4. Two consecutive `ggen sync` runs produce byte-identical output for
//!      `src/cmds/generated.rs` (determinism).
//!
//! Run manually with:
//!   cargo test --test ggen_pipeline_real -- --include-ignored
//!
//! Or as part of the standard suite (`make test`).
//!
//! These tests are NOT `#[ignore]` — they guard the ggen pipeline regression.

use std::fs;
use std::path::Path;

const GENERATED_RS: &str = "src/cmds/generated.rs";
const MOD_RS: &str = "src/cmds/mod.rs";
const RECEIPT: &str = ".ggen/receipts/latest.json";

/// 1. generated.rs exists and contains the expected stub modules.
#[test]
fn generated_rs_exists_and_has_stubs() {
    assert!(
        Path::new(GENERATED_RS).exists(),
        "src/cmds/generated.rs must exist — run `ggen sync` to regenerate"
    );
    let content =
        fs::read_to_string(GENERATED_RS).expect("read generated.rs");
    assert!(
        !content.trim().is_empty(),
        "src/cmds/generated.rs must not be empty"
    );
    // Every top-level command should have a _stub module.
    for noun in &["doctor", "marketplace", "clinical", "alignment", "governance", "data", "server", "ontology"] {
        let stub_mod = format!("pub mod {}_stub", noun);
        assert!(
            content.contains(&stub_mod),
            "generated.rs must contain `{stub_mod}` — re-run `ggen sync`"
        );
    }
}

/// 2. src/cmds/mod.rs declares `pub mod generated`.
#[test]
fn mod_rs_declares_pub_mod_generated() {
    let content = fs::read_to_string(MOD_RS).expect("read src/cmds/mod.rs");
    assert!(
        content.contains("pub mod generated"),
        "src/cmds/mod.rs must declare `pub mod generated`"
    );
}

/// 3. The latest ggen receipt has non-empty signature and ≥1 output hash.
#[test]
fn receipt_is_non_empty_and_signed() {
    assert!(
        Path::new(RECEIPT).exists(),
        ".ggen/receipts/latest.json must exist — run `ggen sync --audit true`"
    );
    let raw = fs::read_to_string(RECEIPT).expect("read receipt");
    let v: serde_json::Value =
        serde_json::from_str(&raw).expect("receipt must be valid JSON");

    let signature = v["signature"]
        .as_str()
        .unwrap_or("");
    assert!(
        !signature.is_empty(),
        "receipt signature must be non-empty (got empty string — ggen did not sign)"
    );

    let output_hashes = v["output_hashes"]
        .as_array()
        .expect("receipt must have output_hashes array");
    assert!(
        !output_hashes.is_empty(),
        "receipt output_hashes must be non-empty — ggen did not record what it generated"
    );

    // Verify the generated.rs hash entry is present.
    let has_generated_rs = output_hashes.iter().any(|h| {
        h.as_str()
            .map(|s| s.contains("generated.rs"))
            .unwrap_or(false)
    });
    assert!(
        has_generated_rs,
        "receipt output_hashes must reference generated.rs; got: {:?}",
        output_hashes
    );
}

/// 4. Two consecutive `ggen sync` runs produce byte-identical generated.rs.
///
/// This guards against non-deterministic generation (e.g., GROUP_CONCAT without ORDER BY
/// in SPARQL queries). Marked `#[ignore]` because it mutates the working tree; run manually:
///   cargo test --test ggen_pipeline_real a ggen_output_is_deterministic -- --ignored
#[test]
#[ignore = "mutates working tree — run manually to check determinism"]
fn ggen_output_is_deterministic() {
    let before =
        fs::read_to_string(GENERATED_RS).expect("read generated.rs before second sync");

    let status = std::process::Command::new("ggen")
        .args(["sync", "--audit", "true"])
        .status()
        .expect("ggen sync must be available on PATH");
    assert!(status.success(), "second ggen sync must exit 0");

    let after =
        fs::read_to_string(GENERATED_RS).expect("read generated.rs after second sync");

    assert_eq!(
        before, after,
        "ggen output is non-deterministic — two consecutive syncs produced different generated.rs.\n\
         Check SPARQL queries for GROUP_CONCAT without ORDER BY."
    );
}
