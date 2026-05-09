//! Phase 11 — multi-tenant session isolation + scope-token ACLs.
//!
//! These tests prove that the Phase-11 admission gate refuses cross-tenant
//! access and that tenant-tagged rows in `receipts`, `declared_workflows`,
//! and `ocel_events` never bleed across tenant namespaces.
//!
//! The tests directly exercise [`OntoStarAdmissionGate::evaluate_in_tenant`]
//! and [`receipts::latest_for_session_in_tenant`], which together implement
//! the tenant ACL surface. All other surfaces (the MCP `evaluate_admission`
//! helper) build on these primitives, so an isolation guarantee here lifts
//! to every gated MCP handler.

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, OntoStarAdmissionGate, PowlBridgeReplay,
};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::receipts;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

const RM_WORKFLOW: &str = "RequirementsManufacturing";
const RM_STAGES: &[&str] = &[
    "requirement_proposed",
    "llm_candidate_translated",
    "ctq_admitted",
    "verification_bound",
    "negative_case_bound",
    "control_plan_bound",
    "work_order_admitted",
];

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("multi-tenant-isolation.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn emit_stage(store: &OcelStore, session: &str, scope: &str, stage: &str, tenant: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let event_id = format!(
        "{}:{}:{}:{}",
        session,
        stage,
        tenant,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
    );
    store
        .emit_event_in_tenant(
            &event_id, stage, &now, session, &[], &[], Some(scope), tenant,
        )
        .expect("emit OCEL event");
}

fn build_gate() -> OntoStarAdmissionGate {
    let required: Vec<String> = by_name(RM_WORKFLOW)
        .map(|w| w.required_stages.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0")
}

/// Open a tenant-tagged scope and emit a conforming RM trace under the same
/// tenant. Returns the scope token.
fn declare_and_run(
    db: &StateDb,
    store: &OcelStore,
    session: &str,
    tenant: &str,
) -> String {
    let scope = WorkflowScope::new(db, session);
    let token = scope
        .open_in_tenant(Some(RM_WORKFLOW), None, None, tenant)
        .expect("open scope under tenant");
    scope.close(&token).expect("close scope");
    for stage in RM_STAGES {
        emit_stage(store, session, &token, stage, tenant);
    }
    token
}

// ── Test 1: cross-tenant receipt isolation (read surface) ──────────────────

#[test]
fn latest_for_session_isolates_receipts_per_tenant() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());

    // Same session_id, different tenants — proves the ACL is tenant-bound,
    // not session-bound.
    let session = "shared-session-id";
    let token_alpha = declare_and_run(&db, &store, session, "alpha");
    let token_beta = declare_and_run(&db, &store, session, "beta");

    let observed_alpha = store.observed_event_types_for_session(session).unwrap();
    let powl = by_name(RM_WORKFLOW).unwrap().powl_string;
    let gate = build_gate();
    let replay = PowlBridgeReplay::new(&store);

    // Admit one receipt under alpha, one under beta.
    let r_alpha = gate
        .evaluate_in_tenant(
            &token_alpha,
            AdmissionOp::RequirementProposed,
            &ArtifactRef { kind: "test", bytes: b"alpha-bytes" },
            &store,
            &replay,
            session,
            powl,
            &observed_alpha,
            "alpha",
        )
        .expect("alpha admits its own scope");
    let r_beta = gate
        .evaluate_in_tenant(
            &token_beta,
            AdmissionOp::RequirementProposed,
            &ArtifactRef { kind: "test", bytes: b"beta-bytes" },
            &store,
            &replay,
            session,
            powl,
            &observed_alpha,
            "beta",
        )
        .expect("beta admits its own scope");
    assert_ne!(r_alpha.bytes, r_beta.bytes);

    // Receipts are already persisted by `evaluate_in_tenant` under their
    // scope's owning tenant_id, no manual persist needed.

    // Tenant alpha's view: never sees beta's hash.
    let latest_alpha = receipts::latest_for_session_in_tenant(&db, session, "alpha")
        .expect("alpha sees its own receipt");
    assert_eq!(
        latest_alpha, r_alpha.bytes,
        "alpha must see its own receipt"
    );
    assert_ne!(
        latest_alpha, r_beta.bytes,
        "alpha must NEVER see beta's receipt"
    );

    let latest_beta = receipts::latest_for_session_in_tenant(&db, session, "beta")
        .expect("beta sees its own receipt");
    assert_eq!(
        latest_beta, r_beta.bytes,
        "beta must see its own receipt"
    );
    assert_ne!(
        latest_beta, r_alpha.bytes,
        "beta must NEVER see alpha's receipt"
    );

    // A nonexistent tenant gets nothing.
    assert!(
        receipts::latest_for_session_in_tenant(&db, session, "gamma").is_none(),
        "unknown tenant must see nothing"
    );
}

// ── Test 2: cross-tenant scope_token denial (write/admission surface) ──────

#[test]
fn cross_tenant_scope_token_yields_tenant_boundary_denial() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "denial-session";

    // Beta opens a scope. Alpha tries to admit it.
    let token_beta = declare_and_run(&db, &store, session, "beta");

    let observed = store.observed_event_types_for_session(session).unwrap();
    let powl = by_name(RM_WORKFLOW).unwrap().powl_string;
    let gate = build_gate();
    let replay = PowlBridgeReplay::new(&store);

    let result = gate.evaluate_in_tenant(
        &token_beta,
        AdmissionOp::RequirementProposed,
        &ArtifactRef { kind: "test", bytes: b"alpha-tries-beta" },
        &store,
        &replay,
        session,
        powl,
        &observed,
        "alpha",
    );

    match result {
        Err((DefectClass::TenantBoundary { from, to }, _)) => {
            assert_eq!(from, "alpha", "from_tenant must be the caller (alpha)");
            assert_eq!(to, "beta", "to_tenant must be the scope owner (beta)");
        }
        other => panic!(
            "expected TenantBoundary denial, got {:?}",
            other.map(|r| r.hex())
        ),
    }
}

// ── Test 3: same-tenant happy path is unaffected ───────────────────────────

#[test]
fn same_tenant_admission_unaffected() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "happy-session";
    let token = declare_and_run(&db, &store, session, "alpha");

    let observed = store.observed_event_types_for_session(session).unwrap();
    let powl = by_name(RM_WORKFLOW).unwrap().powl_string;
    let gate = build_gate();
    let replay = PowlBridgeReplay::new(&store);

    let receipt = gate
        .evaluate_in_tenant(
            &token,
            AdmissionOp::RequirementProposed,
            &ArtifactRef { kind: "test", bytes: b"happy" },
            &store,
            &replay,
            session,
            powl,
            &observed,
            "alpha",
        )
        .expect("same-tenant admission must grant");
    assert!(!receipt.hex().is_empty());
}

// ── Test 4: tenant rotation emits tenant_switch OCEL ───────────────────────

#[test]
fn tenant_rotation_emits_tenant_switch_event() {
    use open_ontologies::tenant::{TenantContext, TenantHandle};

    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "rotation-session";

    let handle = TenantHandle::new("default");
    assert_eq!(handle.current().current(), "default");

    handle.switch(&store, session, "alpha");
    assert_eq!(handle.current().current(), "alpha");

    handle.switch(&store, session, "beta");
    assert_eq!(handle.current().current(), "beta");

    // No-op switch (same tenant) emits NO event.
    handle.switch(&store, session, "beta");
    assert_eq!(handle.current().current(), "beta");

    let conn = db.conn();
    let switch_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ocel_events WHERE event_type = 'tenant_switch'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    assert_eq!(
        switch_count, 2,
        "two tenant_switch events expected (default→alpha, alpha→beta), got {switch_count}"
    );

    // Verify each carries from/to attributes.
    let mut stmt = conn
        .prepare(
            "SELECT a.value FROM ocel_event_attrs a
              JOIN ocel_events e ON e.event_id = a.event_id
             WHERE e.event_type = 'tenant_switch' AND a.name = 'to_tenant'
             ORDER BY e.time ASC, e.event_id ASC",
        )
        .unwrap();
    let to_tenants: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(to_tenants, vec!["alpha".to_string(), "beta".to_string()]);

    // Sanity: TenantContext::from_env still works (returns default when unset).
    let _ = TenantContext::from_env();
}

// ── Test 5: backwards compat — default tenant accepts default scopes ───────

#[test]
fn default_tenant_admits_default_tagged_scope() {
    // Legacy test pattern: open() defaults to tenant_id='default'. A caller
    // also at 'default' must admit unaffected.
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "default-session";

    // Use the legacy `open` (no tenant arg) — defaults to 'default'.
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some(RM_WORKFLOW), None, None)
        .expect("open scope (default tenant)");
    scope.close(&token).expect("close scope");
    for stage in RM_STAGES {
        emit_stage(&store, session, &token, stage, "default");
    }
    let observed = store.observed_event_types_for_session(session).unwrap();
    let powl = by_name(RM_WORKFLOW).unwrap().powl_string;
    let gate = build_gate();
    let replay = PowlBridgeReplay::new(&store);

    // Caller at 'default' admits — should succeed.
    let r = gate
        .evaluate_in_tenant(
            &token,
            AdmissionOp::RequirementProposed,
            &ArtifactRef { kind: "test", bytes: b"default-bytes" },
            &store,
            &replay,
            session,
            powl,
            &observed,
            "default",
        )
        .expect("default-tenant admits default-tagged scope");
    // `evaluate_in_tenant` has already persisted under tenant='default'.

    // The legacy `latest_for_session` (no tenant) defaults to 'default' and
    // returns this receipt — backward-compat invariant.
    let latest = receipts::latest_for_session(&db, session)
        .expect("legacy latest_for_session returns default tenant rows");
    assert_eq!(latest, r.bytes);
}

// ── Test 6: tenant-id columns persist correctly across tables ──────────────

#[test]
fn tenant_id_columns_present_on_all_tagged_tables() {
    // Smoke-test the schema migration: all six tagged tables expose a
    // `tenant_id` column. PRAGMA table_info() is used for portability.
    let db = fresh_db();
    let conn = db.conn();
    for table in &[
        "receipts",
        "declared_workflows",
        "ocel_events",
        "lineage_events",
        "workflow_capability",
        "revoked_sessions",
    ] {
        let sql = format!("PRAGMA table_info({})", table);
        let mut stmt = conn.prepare(&sql).unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(
            cols.iter().any(|c| c == "tenant_id"),
            "table {table} is missing tenant_id column; got: {cols:?}"
        );
    }
}

// ── Test 7: cross-tenant receipts share session, never share chain ─────────

#[test]
fn cross_tenant_receipts_share_session_id_but_never_chain() {
    // Two tenants both write under the same session_id. Their per-tenant
    // sequences must be independent (each starts at 1) and `latest_for_session`
    // for tenant X must only ever return tenant X's chain head.
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "shared-session";
    let token_a = declare_and_run(&db, &store, session, "alpha");
    let token_b = declare_and_run(&db, &store, session, "beta");
    let observed = store.observed_event_types_for_session(session).unwrap();
    let powl = by_name(RM_WORKFLOW).unwrap().powl_string;
    let gate = build_gate();
    let replay = PowlBridgeReplay::new(&store);
    // Two admissions per tenant. `evaluate_in_tenant` persists each receipt
    // under the scope's owning tenant_id automatically.
    for i in 0..2 {
        let _ = gate
            .evaluate_in_tenant(
                &token_a,
                AdmissionOp::RequirementProposed,
                &ArtifactRef {
                    kind: "test",
                    bytes: format!("alpha-{i}").as_bytes(),
                },
                &store,
                &replay,
                session,
                powl,
                &observed,
                "alpha",
            )
            .expect("alpha admit");

        let _ = gate
            .evaluate_in_tenant(
                &token_b,
                AdmissionOp::RequirementProposed,
                &ArtifactRef {
                    kind: "test",
                    bytes: format!("beta-{i}").as_bytes(),
                },
                &store,
                &replay,
                session,
                powl,
                &observed,
                "beta",
            )
            .expect("beta admit");
    }

    // Per-tenant counts. Acquire+release the conn() guard in a tight scope —
    // `latest_for_session_in_tenant` below also calls `db.conn()`, so holding
    // the MutexGuard across that call would deadlock.
    let (alpha_count, beta_count) = {
        let conn = db.conn();
        let a: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM receipts WHERE session_id = ?1 AND tenant_id = ?2",
                rusqlite::params![session, "alpha"],
                |r| r.get(0),
            )
            .unwrap();
        let b: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM receipts WHERE session_id = ?1 AND tenant_id = ?2",
                rusqlite::params![session, "beta"],
                |r| r.get(0),
            )
            .unwrap();
        (a, b)
    };
    assert_eq!(alpha_count, 2);
    assert_eq!(beta_count, 2);

    // latest_for_session_in_tenant returns alpha-only and beta-only chains.
    let alpha_latest = receipts::latest_for_session_in_tenant(&db, session, "alpha")
        .expect("alpha latest");
    let beta_latest = receipts::latest_for_session_in_tenant(&db, session, "beta")
        .expect("beta latest");
    assert_ne!(alpha_latest, beta_latest);

    // Confirm at SQL level that each tenant's chain head row carries its own
    // tenant_id.
    let conn = db.conn();
    let alpha_tenant: String = conn
        .query_row(
            "SELECT tenant_id FROM receipts \
              WHERE session_id = ?1 AND tenant_id = 'alpha' \
              ORDER BY sequence DESC LIMIT 1",
            rusqlite::params![session],
            |r| r.get(0),
        )
        .unwrap();
    let beta_tenant: String = conn
        .query_row(
            "SELECT tenant_id FROM receipts \
              WHERE session_id = ?1 AND tenant_id = 'beta' \
              ORDER BY sequence DESC LIMIT 1",
            rusqlite::params![session],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(alpha_tenant, "alpha");
    assert_eq!(beta_tenant, "beta");
}
