//! R5 WC-2 — End-to-end coverage for the 5 new admin-only MCP tools.
//!
//! Each test follows the same skeleton:
//!   1. Build a server with NO admin allowlist (closed-by-default).
//!   2. Invoke the tool — assert `defect.kind == "FalsePass"` with
//!      `reason == "not_admin"`.
//!   3. Build a server WITH admin allowlist that matches the tenant.
//!   4. Invoke the tool — assert success and verify the durable state
//!      change (DB row, atomic value, OCEL event).
//!
//! The §28 invariant under test: every admin tool gates on the
//! `is_admin_principal()` cache from R5 WC-1, NOT on `std::env::var(...)`.
//! Subsequent env mutations after server construction have no effect.
//!
//! Counterfactual proof (§19): each "admin path" assertion would FAIL
//! against a server that did NOT install the allowlist, because the
//! tool would correctly refuse with `FalsePass`. Δ > 0.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use open_ontologies::admission::AdmissionOp;
use open_ontologies::config::{CacheConfig, EmbeddingsConfig, RetentionConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::production_record::ProductionRecord;
use open_ontologies::receipts::{self, Receipt};
use open_ontologies::retention::RetentionWorker;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;
use rmcp::handler::server::wrapper::Parameters;
use tempfile::TempDir;

const ADMIN_TENANT: &str = "ops-admin";

fn build_server_with_admin(admin: bool) -> (TempDir, StateDb, OpenOntologiesServer) {
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
    let principals = if admin {
        vec![ADMIN_TENANT.to_string()]
    } else {
        Vec::new()
    };
    let server = OpenOntologiesServer::new_with_registry_options(
        db.clone(),
        graph,
        None,
        EmbeddingsConfig::default(),
        cache,
        ToolFilter::default(),
    )
    .with_admin_principals(principals)
    .with_tenant(ADMIN_TENANT);
    (tmp, db, server)
}

fn build_test_receipt(law_version: &str, scope: &str) -> Receipt {
    let record = ProductionRecord {
        artifact_hash: [0u8; 32],
        scope_token: scope.to_string(),
        declared_powl_hash: [0u8; 32],
        ocel_canonical_hash: [0u8; 32],
        conformance_run_id: "test-run".to_string(),
        gate_config_hash: [0u8; 32],
        production_law_version: law_version.to_string(),
        defects_taxonomy_version: "ontostar-defects-4.4.0".to_string(),
        gates_passed: vec![],
        gates_refused: vec![],
        prior_receipt: None,
        signature: None,
        signing_key_fpr: None,
    };
    receipts::build(record)
}

fn count(db: &StateDb, sql: &str) -> i64 {
    db.conn().query_row(sql, [], |r| r.get(0)).unwrap_or(-1)
}

// ─── 1. onto_bootstrap_unlock ──────────────────────────────────────────

#[test]
fn bootstrap_unlock_admin_only() {
    // Non-admin server — refuses with FalsePass.
    let (_tmp, db, server) = build_server_with_admin(false);
    let prod = build_test_receipt("ontostar-1.0.0", "scope-prod");
    receipts::persist_with_tenant(&prod, &db, "session-prod", "tenant-prod").unwrap();
    assert_eq!(count(&db, "SELECT COUNT(*) FROM bootstrap_lock"), 1);

    let response = server.onto_bootstrap_unlock();
    let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(parsed["ok"].as_bool(), Some(false));
    assert_eq!(parsed["defect"]["kind"].as_str(), Some("FalsePass"));
    assert_eq!(parsed["defect"]["reason"].as_str(), Some("not_admin"));
    // DB unchanged — non-admin call must not delete the lock.
    assert_eq!(count(&db, "SELECT COUNT(*) FROM bootstrap_lock"), 1);

    // Admin server — unlocks.
    let (_tmp2, db2, admin_server) = build_server_with_admin(true);
    let prod2 = build_test_receipt("ontostar-1.0.0", "scope-prod");
    receipts::persist_with_tenant(&prod2, &db2, "session-prod", "tenant-prod").unwrap();
    assert_eq!(count(&db2, "SELECT COUNT(*) FROM bootstrap_lock"), 1);

    let response2 = admin_server.onto_bootstrap_unlock();
    let parsed2: serde_json::Value = serde_json::from_str(&response2).unwrap();
    assert_eq!(parsed2["ok"].as_bool(), Some(true), "admin must unlock; got: {response2}");
    assert_eq!(parsed2["rows_deleted"].as_u64(), Some(1));
    assert_eq!(count(&db2, "SELECT COUNT(*) FROM bootstrap_lock"), 0);
}

// ─── 2. onto_receipts_revoke_batch ─────────────────────────────────────

#[test]
fn receipts_revoke_batch_admin_only_emits_event() {
    use open_ontologies::inputs::OntoReceiptsRevokeBatchInput;
    // Non-admin path.
    let (_tmp, db, server) = build_server_with_admin(false);
    for i in 0..3 {
        let r = build_test_receipt("ontostar-1.0.0", &format!("alpha-{i}"));
        receipts::persist_with_tenant(&r, &db, &format!("s-{i}"), "tenant-prod").unwrap();
    }
    // Seed receipt — must NOT be touched even by admin call.
    let seed = build_test_receipt("seed-v0", "alpha-seed");
    receipts::persist_with_tenant(&seed, &db, "s-seed", "tenant-prod").unwrap();

    let resp = server.onto_receipts_revoke_batch(Parameters(OntoReceiptsRevokeBatchInput {
        scope_token_pattern: "alpha-*".to_string(),
        reason: "test".to_string(),
    }));
    let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["ok"].as_bool(), Some(false));
    assert_eq!(parsed["defect"]["reason"].as_str(), Some("not_admin"));
    let revoked_count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM receipts WHERE production_law_version = 'revoked-by-admin'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(revoked_count, 0, "non-admin must not soft-delete");

    // Admin path.
    let (_tmp2, db2, admin_server) = build_server_with_admin(true);
    for i in 0..3 {
        let r = build_test_receipt("ontostar-1.0.0", &format!("alpha-{i}"));
        receipts::persist_with_tenant(&r, &db2, &format!("s-{i}"), "tenant-prod").unwrap();
    }
    let seed2 = build_test_receipt("seed-v0", "alpha-seed");
    receipts::persist_with_tenant(&seed2, &db2, "s-seed", "tenant-prod").unwrap();
    let resp2 = admin_server.onto_receipts_revoke_batch(Parameters(OntoReceiptsRevokeBatchInput {
        scope_token_pattern: "alpha-*".to_string(),
        reason: "regulatory-clawback".to_string(),
    }));
    let parsed2: serde_json::Value = serde_json::from_str(&resp2).unwrap();
    assert_eq!(parsed2["ok"].as_bool(), Some(true), "admin must revoke; got: {resp2}");
    assert_eq!(parsed2["count"].as_u64(), Some(3), "exactly 3 non-seed rows updated");
    assert_eq!(parsed2["reason"].as_str(), Some("regulatory-clawback"));
    // Seed receipt unchanged.
    let seed_lv: String = db2.conn().query_row(
        "SELECT production_law_version FROM receipts WHERE scope_token = 'alpha-seed'",
        [],
        |r| r.get(0),
    ).unwrap();
    assert_eq!(seed_lv, "seed-v0", "seed-v0 must be excluded from soft-delete");
    // Audit OCEL event recorded under the dedicated AdmissionOp variant.
    let audit_rows: i64 = db2.conn().query_row(
        "SELECT COUNT(*) FROM ocel_events WHERE event_type = 'admission_audit'",
        [],
        |r| r.get(0),
    ).unwrap();
    assert!(audit_rows >= 1, "admission_audit OCEL event must be recorded");
    // The dedicated op string lives on AdmissionOp::ReceiptsBatchRevoke.
    assert_eq!(
        AdmissionOp::ReceiptsBatchRevoke.as_str(),
        "receipts_batch_revoke",
        "AdmissionOp variant must be wired to the audit name"
    );
}

// ─── 3. onto_session_revoke_by_principal ───────────────────────────────

#[test]
fn session_revoke_by_principal_admin_only_inserts_revoked_sessions() {
    use open_ontologies::inputs::OntoSessionRevokeByPrincipalInput;
    let (_tmp, db, server) = build_server_with_admin(true);
    // Seed a couple of declared_workflows rows for the target tenant.
    {
        let conn = db.conn();
        for i in 0..3 {
            conn.execute(
                "INSERT INTO declared_workflows
                    (scope_token, session_id, name, powl_string, powl_hash, alphabet_json, declared_at, status, tenant_id)
                 VALUES (?1, ?2, 'wf', '', '', '{}', ?3, 'open', 'target-tenant')",
                rusqlite::params![
                    format!("scope-{i}"),
                    format!("session-target-{i}"),
                    chrono::Utc::now().to_rfc3339(),
                ],
            )
            .unwrap();
        }
    }
    let pre: i64 = count(&db, "SELECT COUNT(*) FROM revoked_sessions");
    let resp = server.onto_session_revoke_by_principal(Parameters(
        OntoSessionRevokeByPrincipalInput {
            tenant_id: "target-tenant".to_string(),
            principal_id: "target-tenant".to_string(),
            reason: "operator-eviction".to_string(),
        },
    ));
    let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["ok"].as_bool(), Some(true), "admin path: {resp}");
    let post: i64 = count(&db, "SELECT COUNT(*) FROM revoked_sessions");
    assert_eq!(
        post - pre,
        3,
        "3 new revoked_sessions rows for the 3 target sessions"
    );
    // Non-admin counterfactual — different server, no allowlist.
    let (_tmp2, db2, non_admin) = build_server_with_admin(false);
    {
        let conn = db2.conn();
        conn.execute(
            "INSERT INTO declared_workflows
                (scope_token, session_id, name, powl_string, powl_hash, alphabet_json, declared_at, status, tenant_id)
             VALUES ('s-x', 'session-x', 'wf', '', '', '{}', ?1, 'open', 'target-tenant')",
            rusqlite::params![chrono::Utc::now().to_rfc3339()],
        )
        .unwrap();
    }
    let resp_n = non_admin.onto_session_revoke_by_principal(Parameters(
        OntoSessionRevokeByPrincipalInput {
            tenant_id: "target-tenant".to_string(),
            principal_id: "target-tenant".to_string(),
            reason: "x".to_string(),
        },
    ));
    let parsed_n: serde_json::Value = serde_json::from_str(&resp_n).unwrap();
    assert_eq!(parsed_n["ok"].as_bool(), Some(false));
    assert_eq!(
        parsed_n["defect"]["reason"].as_str(),
        Some("not_admin"),
        "non-admin must be denied"
    );
    assert_eq!(
        count(&db2, "SELECT COUNT(*) FROM revoked_sessions"),
        0,
        "non-admin must not insert"
    );
}

// ─── 4 & 5. onto_retention_pause / onto_retention_resume ───────────────

#[test]
fn retention_pause_skips_tick() {
    use open_ontologies::inputs::OntoRetentionPauseInput;
    // Use a server WITH a wired pause handle that's also the worker's
    // `paused_until`. The worker is constructed inline (not spawned as
    // a tokio task) so we can drive `tick()` synchronously.
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
    let pause_handle = Arc::new(AtomicI64::new(0));
    let server = OpenOntologiesServer::new_with_registry_options(
        db.clone(),
        graph,
        None,
        EmbeddingsConfig::default(),
        cache,
        ToolFilter::default(),
    )
    .with_admin_principals(vec![ADMIN_TENANT.to_string()])
    .with_tenant(ADMIN_TENANT)
    .with_retention_pause(pause_handle.clone());
    let cfg = RetentionConfig {
        poll_interval_secs: 1,
        ocel_days: 0,
        lineage_days: 0,
        conformance_days: 0,
        revocation_grace_days: 0,
        receipt_files_days: 0,
        exemplar_days: 0,
        feedback_days: 0,
        archive_path: None,
        hot_receipt_days: 0,
    };
    let worker = RetentionWorker::new_with_pause(db.clone(), cfg, pause_handle.clone());
    // Pause via the admin tool BEFORE seeding the past row. This way the
    // tool's own lineage rows have current timestamps (NOT in the past),
    // so the pruner with cfg.lineage_days=0 (cutoff = now) only targets
    // the explicitly-seeded past row.
    let resp = server.onto_retention_pause(Parameters(OntoRetentionPauseInput { minutes: 60 }));
    let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["ok"].as_bool(), Some(true), "{resp}");
    assert!(pause_handle.load(Ordering::Relaxed) > chrono::Utc::now().timestamp());
    assert!(worker.is_paused(), "worker.is_paused() must agree with the atomic");

    // Seed a lineage event in the past — this is the row the pruner
    // would target if it were running. We track it by the unique
    // session_id 'past-lineage-seed'.
    let past_ts = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    db.conn()
        .execute(
            "INSERT INTO lineage_events (session_id, seq, timestamp, event_type, operation, details, tenant_id) \
             VALUES ('past-lineage-seed', 1, ?1, 'X', 'op', '', 'default')",
            rusqlite::params![past_ts],
        )
        .unwrap();
    let past_count_before: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM lineage_events WHERE session_id = 'past-lineage-seed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(past_count_before, 1);

    // Tick — must skip work, our seeded past row preserved.
    let report = worker.tick().expect("tick must succeed");
    assert_eq!(report.lineage_events_pruned, 0, "paused tick must not prune");
    let past_after_pause: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM lineage_events WHERE session_id = 'past-lineage-seed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        past_after_pause, 1,
        "seeded past lineage row must survive paused tick"
    );

    // Resume.
    let resp_r = server.onto_retention_resume();
    let parsed_r: serde_json::Value = serde_json::from_str(&resp_r).unwrap();
    assert_eq!(parsed_r["ok"].as_bool(), Some(true));
    assert_eq!(pause_handle.load(Ordering::Relaxed), 0);
    assert!(!worker.is_paused());

    // Tick again — pruner runs and removes the seeded past row.
    let report2 = worker.tick().expect("post-resume tick");
    assert!(report2.lineage_events_pruned >= 1, "post-resume must prune ≥1");
    let past_after_resume: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM lineage_events WHERE session_id = 'past-lineage-seed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(past_after_resume, 0, "seeded past row must be pruned post-resume");
}

#[test]
fn retention_resume_idempotent_and_non_admin_denied() {
    use open_ontologies::inputs::OntoRetentionPauseInput;
    let (_tmp, _db, server) = build_server_with_admin(true);
    // Non-admin path for both pause and resume.
    let (_tmp_n, _db_n, non_admin) = build_server_with_admin(false);
    let resp_p = non_admin.onto_retention_pause(Parameters(OntoRetentionPauseInput { minutes: 5 }));
    let parsed_p: serde_json::Value = serde_json::from_str(&resp_p).unwrap();
    assert_eq!(parsed_p["defect"]["reason"].as_str(), Some("not_admin"));
    let resp_r = non_admin.onto_retention_resume();
    let parsed_r: serde_json::Value = serde_json::from_str(&resp_r).unwrap();
    assert_eq!(parsed_r["defect"]["reason"].as_str(), Some("not_admin"));

    // Idempotent resume on admin.
    let r1 = server.onto_retention_resume();
    let r2 = server.onto_retention_resume();
    let p1: serde_json::Value = serde_json::from_str(&r1).unwrap();
    let p2: serde_json::Value = serde_json::from_str(&r2).unwrap();
    assert_eq!(p1["ok"].as_bool(), Some(true));
    assert_eq!(p2["ok"].as_bool(), Some(true));
    assert_eq!(p2["previous_paused_until_epoch_secs"].as_i64(), Some(0));

    // Bound check: minutes=0 is rejected.
    let bad = server.onto_retention_pause(Parameters(OntoRetentionPauseInput { minutes: 0 }));
    let pb: serde_json::Value = serde_json::from_str(&bad).unwrap();
    assert_eq!(pb["ok"].as_bool(), Some(false));
    let huge = server.onto_retention_pause(Parameters(OntoRetentionPauseInput { minutes: 100_000 }));
    let ph: serde_json::Value = serde_json::from_str(&huge).unwrap();
    assert_eq!(ph["ok"].as_bool(), Some(false));
}
