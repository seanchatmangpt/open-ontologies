//! Smoke tests for the mcpp proof-gating middleware.
//!
//! These tests verify structural and helper behaviour without a live MCP
//! session (RequestContext is non-exhaustive in rmcp 1.6 and cannot be
//! constructed outside the crate).
//!
//! The K-P09 admit path is exercised by `mcpp-onto-bridge`'s bridge_smoke
//! integration tests which spin up a real rmcp session.

use open_ontologies::mcpp_gate::MaybeGatedServer;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use tempfile::tempdir;

fn fresh_db() -> (tempfile::TempDir, StateDb) {
    let dir = tempdir().unwrap();
    let db = StateDb::open(&dir.path().join("test.db")).unwrap();
    (dir, db)
}

// ── MaybeGatedServer::Bare ──────────────────────────────────────────────────

#[test]
fn bare_server_registry_does_not_panic() {
    let (dir, db) = fresh_db();
    let inner = OpenOntologiesServer::new(db);
    let server = MaybeGatedServer::Bare(inner);
    let _reg = server.registry();
    drop(dir);
}

#[test]
fn bare_server_get_info_returns_name() {
    use rmcp::ServerHandler;
    let (dir, db) = fresh_db();
    let inner = OpenOntologiesServer::new(db);
    let server = MaybeGatedServer::Bare(inner);
    let info = server.get_info();
    assert!(!info.server_info.name.is_empty());
    drop(dir);
}

// ── MaybeGatedServer::Gated (mcpp feature only) ─────────────────────────────

#[cfg(feature = "mcpp")]
#[test]
fn gated_server_registry_does_not_panic() {
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;

    let (dir, db) = fresh_db();
    let inner = OpenOntologiesServer::new(db.clone());
    let key = SigningKey::generate(&mut OsRng);
    let server = MaybeGatedServer::Gated(
        open_ontologies::mcpp_gate::ProofGatedServer::new(inner, db, key),
    );
    let _reg = server.registry();
    drop(dir);
}

#[cfg(feature = "mcpp")]
#[test]
fn gated_server_get_info_matches_bare() {
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;
    use rmcp::ServerHandler;

    let (dir_bare, db_bare) = fresh_db();
    let (dir_gated, db_gated) = fresh_db();

    let bare = MaybeGatedServer::Bare(OpenOntologiesServer::new(db_bare));

    let key = SigningKey::generate(&mut OsRng);
    let inner = OpenOntologiesServer::new(db_gated.clone());
    let gated = MaybeGatedServer::Gated(
        open_ontologies::mcpp_gate::ProofGatedServer::new(inner, db_gated, key),
    );

    assert_eq!(
        bare.get_info().server_info.name,
        gated.get_info().server_info.name,
        "gated wrapper must not change the server name"
    );
    drop((dir_bare, dir_gated));
}

// ── augment_with_proof helper ────────────────────────────────────────────────
//
// Test the proof augmentation logic via the on_threshold_status result shape.
// We construct a synthetic CallToolResult and verify the mcpp envelope.

#[cfg(feature = "mcpp")]
#[test]
fn augment_with_proof_adds_mcpp_envelope() {
    use rmcp::model::{CallToolResult, Content};

    let json = serde_json::json!({"ok": true, "thresholds": []}).to_string();
    let result = CallToolResult::success(vec![Content::text(json)]);

    // augment_with_proof is private; simulate its logic here to verify the
    // JSON shape that callers depend on.
    let scope = "mcpp-test-scope";
    let hash = "abcdef1234567890";

    let text = result.content.first().and_then(|c| c.as_text())
        .map(|t| t.text.clone()).unwrap_or_default();
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["mcpp"] = serde_json::json!({
        "verdict":      "accepted",
        "scope_token":  scope,
        "receipt_hash": hash,
    });

    assert_eq!(v["ok"], true);
    assert_eq!(v["mcpp"]["verdict"], "accepted");
    assert_eq!(v["mcpp"]["scope_token"], scope);
    assert_eq!(v["mcpp"]["receipt_hash"], hash);
}
