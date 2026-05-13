//! R5 WC-1 — §22 success-shaped denial closure.
//!
//! Counterfactual: when a caller invokes a mutating handler with
//! `bypass_admission=true` plus a non-empty `bypass_reason`, the handler
//! MUST return the unified denial JSON shape:
//!
//! ```json
//! {
//!   "ok": false,
//!   "admission": "bypassed_session_revoked",
//!   "defect": { "kind": "BypassRevoked", "reason": "<reason>" },
//!   "principal_revoked_at": "<RFC3339 timestamp>"
//! }
//! ```
//!
//! Pre-fix behaviour: `Err({"ok": true, "admission": "bypassed", ...})` —
//! a JSON object claiming `ok: true` while `revoked_sessions` was being
//! written and the session was being killed. External auditors keying on
//! `ok` were misled into treating denials as successes.
//!
//! Post-fix behaviour: every field of the new shape is asserted here, so
//! any drift back to the old shape (or partial drift) is caught at test
//! time. **Δ > 0**: this test would FAIL against pre-fix server.rs.

use std::sync::Arc;

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::inputs::OntoSaveInput;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;
use rmcp::handler::server::wrapper::Parameters;
use tempfile::TempDir;

fn build_server() -> (TempDir, StateDb, OpenOntologiesServer) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("server.db")).unwrap();
    let graph = Arc::new(GraphStore::new());
    let cache = CacheConfig {
        enabled: true,
        dir: tmp.path().join("cache").to_string_lossy().into_owned(),
        idle_ttl_secs: 0,
        evictor_interval_secs: 30,
        auto_refresh: false,
        hash_prefix_bytes: 64 * 1024,
    };
    let server = OpenOntologiesServer::new_with_registry_options(
        db.clone(),
        graph,
        None,
        EmbeddingsConfig::default(),
        cache,
        ToolFilter::default(),
    );
    (tmp, db, server)
}

#[tokio::test]
async fn bypass_response_is_unified_denial_shape() {
    let (_tmp, _db, server) = build_server();

    // Drive a bypass via onto_save (a public mutating handler that
    // funnels through evaluate_admission; with no active ontology
    // loaded, ensure_loaded is a no-op so the bypass branch is hit
    // cleanly without exercising the planner / graph serializer).
    let response = server
        .onto_save(Parameters(OntoSaveInput {
            path: "/tmp/r5-wc-1-bypass-shape-test.ttl".to_string(),
            format: Some("turtle".into()),
            scope_token: None,
            bypass_admission: Some(true),
            bypass_reason: Some("r5-wc-1-unified-bypass-shape-test".into()),
        }))
        .await;

    let parsed: serde_json::Value = serde_json::from_str(&response).unwrap_or_else(|e| {
        panic!("response is not valid JSON: {}\nresponse: {}", e, &response)
    });

    // 1. `ok` is FALSE — the previous shape claimed `ok: true` while the
    //    internal state was a denial. This is the load-bearing flip.
    assert_eq!(
        parsed["ok"].as_bool(),
        Some(false),
        "bypass denial must report ok=false; got: {}",
        &response
    );

    // 2. `admission` is the new sentinel that distinguishes this denial
    //    path from gate refusal (`"denied"`) and admit (`"granted"`).
    assert_eq!(
        parsed["admission"].as_str(),
        Some("bypassed_session_revoked"),
        "bypass denial admission tag drift; got: {}",
        &response
    );

    // 3. `defect.kind` is the structured DefectClass tag — auditors drive
    //    workflows on this, not on free text.
    assert_eq!(
        parsed["defect"]["kind"].as_str(),
        Some("BypassRevoked"),
        "defect.kind must be BypassRevoked; got: {}",
        &response
    );

    // 4. `defect.reason` echoes the operator's reason verbatim, no
    //    weasel substitution.
    assert_eq!(
        parsed["defect"]["reason"].as_str(),
        Some("r5-wc-1-unified-bypass-shape-test"),
        "defect.reason must echo the bypass_reason; got: {}",
        &response
    );

    // 5. `principal_revoked_at` is RFC3339-shaped. We use chrono's
    //    parser rather than a regex to avoid coupling to formatting
    //    minutiae; auditors can rely on the same parser.
    let revoked_at = parsed["principal_revoked_at"]
        .as_str()
        .unwrap_or_else(|| panic!("missing principal_revoked_at: {}", &response));
    chrono::DateTime::parse_from_rfc3339(revoked_at).unwrap_or_else(|e| {
        panic!(
            "principal_revoked_at is not RFC3339: {} — error: {}\nresponse: {}",
            revoked_at, e, &response
        )
    });

    // 6. Pre-fix shape's `reason` (NOT under defect) is GONE. Any tooling
    //    that read the old top-level `reason` MUST migrate to
    //    `defect.reason`. Asserting absence keeps the migration honest.
    assert!(
        parsed.get("reason").is_none(),
        "top-level `reason` field must NOT appear in unified denial shape \
         (auditors should read defect.reason); got: {}",
        &response
    );
}
