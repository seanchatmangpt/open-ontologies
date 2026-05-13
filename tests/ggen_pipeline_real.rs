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

/// 3. A signed ggen receipt referencing generated.rs exists in the receipts directory.
///
/// Scans all receipt files (not just latest.json) because the revops pipeline
/// overwrites latest.json with its own receipt — the CLI receipt may be a
/// non-latest but still-valid sibling file.
#[test]
fn receipt_is_non_empty_and_signed() {
    let receipts_dir = Path::new(".ggen/receipts");
    assert!(
        receipts_dir.exists(),
        ".ggen/receipts/ must exist — run `ggen sync` to create it"
    );

    // Collect all .json receipt files in the directory.
    let entries: Vec<_> = fs::read_dir(receipts_dir)
        .expect("read .ggen/receipts/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();
    assert!(!entries.is_empty(), ".ggen/receipts/ must contain at least one .json receipt");

    // Find a receipt that (a) has a non-empty signature and (b) references generated.rs.
    let cli_receipt = entries.iter().find(|e| {
        let Ok(raw) = fs::read_to_string(e.path()) else { return false };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else { return false };
        let sig_ok = v["signature"].as_str().map(|s| !s.is_empty()).unwrap_or(false);
        let hashes_ok = v["output_hashes"]
            .as_array()
            .map(|arr| arr.iter().any(|h| {
                h.as_str().map(|s| s.contains("generated.rs") && !s.contains("generated_revops.rs")).unwrap_or(false)
            }))
            .unwrap_or(false);
        sig_ok && hashes_ok
    });

    assert!(
        cli_receipt.is_some(),
        "no signed receipt referencing generated.rs found in .ggen/receipts/ — run `ggen sync`"
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
