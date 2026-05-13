//! R4 WE — §14: bypass branch self-attribution proofs.
//!
//! When a caller invokes a mutating handler with `bypass_admission=true`
//! plus a non-empty `bypass_reason`, the OCEL trail must record an
//! `admission_audit` row carrying `op=bypass` BEFORE the
//! `revoked_sessions` row is written. This file pins both invariants:
//!
//!   1. `bypass_emits_admission_audit_before_revoke` — the OCEL row
//!      ordering is `admission_audit{op=bypass}` then `admission_bypass`,
//!      and the `revoked_sessions.revoked_at` timestamp is monotonically
//!      `>=` the audit row's timestamp.
//!   2. `bypass_audit_carries_op_bypass_attribute` — the audit row's
//!      `op` attribute is exactly `"bypass"`, not the underlying
//!      operation's tag (the audit MUST self-attribute as a Bypass
//!      event so the auditor can distinguish the bypass branch from
//!      the underlying operation's normal admission events).

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
async fn bypass_emits_admission_audit_before_revoke() {
    let (_tmp, db, server) = build_server();

    // Drive a bypass via onto_save (any mutating handler funnels through
    // evaluate_admission, which is where the bypass branch lives).
    let _ = server
        .onto_save(Parameters(OntoSaveInput {
            path: "/tmp/round4-bypass-test-output.ttl".to_string(),
            format: Some("turtle".into()),
            scope_token: None,
            bypass_admission: Some(true),
            bypass_reason: Some("r4-we-bypass-self-attribution-test".into()),
        }))
        .await;

    // Read OCEL events ordered by timestamp ASC. The first row in the
    // bypass branch MUST be an `admission_audit` row with op=bypass; the
    // `admission_bypass` row (legacy event_type) follows. There MUST be
    // at least one `admission_audit{op=bypass}` row — the new self-
    // attribution.
    let conn = db.conn();
    let mut stmt = conn
        .prepare(
            "SELECT e.event_id, e.event_type, e.time, IFNULL(a.value, '')
             FROM ocel_events e
             LEFT JOIN ocel_event_attrs a
               ON a.event_id = e.event_id AND a.name = 'op'
             WHERE e.event_type IN ('admission_audit', 'admission_bypass')
             ORDER BY e.time ASC, e.event_id ASC",
        )
        .unwrap();
    let rows: Vec<(String, String, String, String)> = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    drop(stmt);

    // Find the audit row carrying op=bypass.
    let audit_idx = rows
        .iter()
        .position(|(_, ty, _, op)| ty == "admission_audit" && op == "bypass")
        .expect(
            "expected an admission_audit row with op=bypass before any admission_bypass row",
        );
    let bypass_idx = rows
        .iter()
        .position(|(_, ty, _, _)| ty == "admission_bypass")
        .expect("expected an admission_bypass row to exist");

    assert!(
        audit_idx <= bypass_idx,
        "admission_audit{{op=bypass}} must precede admission_bypass in OCEL stream;\n\
         audit_idx={} bypass_idx={}\nrows={:#?}",
        audit_idx,
        bypass_idx,
        rows
    );

    // The revoked_sessions.revoked_at timestamp must be >= the audit
    // row's timestamp (the bypass branch emits the audit first, THEN
    // writes revoked_sessions).
    let audit_ts = &rows[audit_idx].2;
    let revoked_at: Option<String> = conn
        .query_row(
            "SELECT revoked_at FROM revoked_sessions ORDER BY revoked_at ASC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok();
    let revoked_at = revoked_at.expect("revoked_sessions row must exist after bypass");
    assert!(
        revoked_at.as_str() >= audit_ts.as_str(),
        "revoked_at ({}) must be >= admission_audit.time ({})",
        revoked_at,
        audit_ts
    );
}

#[tokio::test]
async fn bypass_audit_carries_op_bypass_attribute() {
    let (_tmp, db, server) = build_server();

    let _ = server
        .onto_save(Parameters(OntoSaveInput {
            path: "/tmp/round4-bypass-attr-test-output.ttl".to_string(),
            format: Some("turtle".into()),
            scope_token: None,
            bypass_admission: Some(true),
            bypass_reason: Some("r4-we-bypass-attr-test".into()),
        }))
        .await;

    let conn = db.conn();
    // Find every admission_audit row's op attribute. At least one MUST
    // be exactly the literal string "bypass" (the canonical R4 WE
    // self-attribution).
    let mut stmt = conn
        .prepare(
            "SELECT a.value
             FROM ocel_events e
             JOIN ocel_event_attrs a
               ON a.event_id = e.event_id AND a.name = 'op'
             WHERE e.event_type = 'admission_audit'",
        )
        .unwrap();
    let ops: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    drop(stmt);

    assert!(
        ops.iter().any(|s| s == "bypass"),
        "expected at least one admission_audit row with op='bypass'; got: {:#?}",
        ops
    );
}
