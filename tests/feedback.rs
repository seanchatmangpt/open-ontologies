//! OntoStar Stream 4 — feedback loop tests.
//!
//! (a) Loop 1: an apply with no `receipts` row cannot enter `mined_exemplars`.
//! (b) Loop 4: only receipt-backed exemplars come out of `exemplars_for_domain`.
//! (c) Loop 5: 20 declining conformance_runs emit `conformance_regression_detected` exactly once.

use open_ontologies::feedback::{exemplars, regression};
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;

fn fresh_store() -> OcelStore {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    OcelStore::new(db)
}

fn insert_admission_granted(store: &OcelStore, scope: &str, event_id: &str) {
    let conn = store.db().conn();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO ocel_events (event_id, event_type, time, session_id) VALUES (?1, 'admission_granted', ?2, 'sess')",
        rusqlite::params![event_id, now],
    ).unwrap();
    conn.execute(
        "INSERT INTO ocel_event_attrs (event_id, name, value, value_type) VALUES (?1, 'scope_token', ?2, 'string')",
        rusqlite::params![event_id, scope],
    ).unwrap();
}

fn insert_conformance_run(store: &OcelStore, scope: &str, run_id: &str, fitness: f64, workflow_class: &str) {
    let conn = store.db().conn();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO conformance_runs
            (run_id, scope_token, workflow_class, fitness, precision, generalization, simplicity,
             verdict, defects_json, trace_canonical_hash, ran_at)
         VALUES (?1, ?2, ?3, ?4, 1.0, 1.0, 1.0, 'conform', '[]', '', ?5)",
        rusqlite::params![run_id, scope, workflow_class, fitness, now],
    ).unwrap();
}

fn insert_receipt(store: &OcelStore, scope: &str, hash: &str) {
    let conn = store.db().conn();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO receipts
            (receipt_hash, scope_token, artifact_hash, declared_powl_hash,
             ocel_canonical_hash, gate_config_hash, prior_receipt_hash,
             production_law_version, granted_at)
         VALUES (?1, ?2, 'art', 'pwl', 'ocel', 'gate', NULL, 'ontostar-1.0.0', ?3)",
        rusqlite::params![hash, scope, now],
    ).unwrap();
}

fn insert_declared_workflow(store: &OcelStore, scope: &str, name: &str) {
    let conn = store.db().conn();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO declared_workflows
            (scope_token, session_id, name, powl_string, powl_hash, alphabet_json, declared_at, status)
         VALUES (?1, 'sess', ?2, 'SEQ(a,b)', 'h', '[]', ?3, 'open')",
        rusqlite::params![scope, name, now],
    ).unwrap();
}

#[test]
fn loop1_force_apply_without_receipt_cannot_enter_registry() {
    let store = fresh_store();
    let scope = "scope-forced";

    // Setup: admission_granted + conformance_runs at fitness 0.99 BUT no receipt.
    insert_admission_granted(&store, scope, "ev1");
    insert_conformance_run(&store, scope, "run1", 0.99, "OntologyAuthoring");
    insert_declared_workflow(&store, scope, "OntologyAuthoring");

    // Mining MUST refuse — no receipt row.
    let mined = exemplars::maybe_mine_exemplar(scope, &store).unwrap();
    assert!(mined.is_none(), "mining must be refused without a receipt");

    let count: i64 = store.db().conn()
        .query_row("SELECT COUNT(*) FROM mined_exemplars", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0, "mined_exemplars table must remain empty");

    // Now add receipt and retry — mining should succeed.
    insert_receipt(&store, scope, "rh1");
    let mined = exemplars::maybe_mine_exemplar(scope, &store).unwrap();
    assert!(mined.is_some(), "mining should succeed once receipt is present");
}

#[test]
fn loop4_join_filters_orphan_exemplars() {
    let store = fresh_store();

    // Exemplar A — has matching receipt (legitimate flow via the API).
    let scope_a = "scope-a";
    insert_admission_granted(&store, scope_a, "ev_a");
    insert_conformance_run(&store, scope_a, "run_a", 0.97, "DomainX");
    insert_declared_workflow(&store, scope_a, "DomainX");
    insert_receipt(&store, scope_a, "rh_a");
    let mined_a = exemplars::maybe_mine_exemplar(scope_a, &store).unwrap();
    assert!(mined_a.is_some());

    // Exemplar B — direct SQL bypass injecting an orphan row pointing at a
    // non-existent receipt_hash. Loop 4 must NOT return this row.
    {
        let conn = store.db().conn();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO mined_exemplars
                (id, domain, problem_context, powl_string, fitness, source_session, receipt_hash, mined_at, promoted)
             VALUES ('orphan_b', 'DomainX', 'ctx', 'powl', 0.99, NULL, 'NO_SUCH_RECEIPT', ?1, 0)",
            rusqlite::params![now],
        ).unwrap();
    }

    // Sanity: both rows now exist in the table.
    let total: i64 = store.db().conn()
        .query_row("SELECT COUNT(*) FROM mined_exemplars", [], |r| r.get(0))
        .unwrap();
    assert_eq!(total, 2, "fixture should have both rows in raw table");

    // The JOIN in exemplars_for_domain must drop the orphan.
    let rows = store.exemplars_for_domain("DomainX", 0.0, 100).unwrap();
    assert_eq!(rows.len(), 1, "only the receipt-backed exemplar should be returned");
    assert_eq!(rows[0].receipt_hash, "rh_a");
}

#[test]
fn loop5_declining_fitness_emits_one_regression_event() {
    let store = fresh_store();
    let class = "RegressionClass";

    // 10 baseline runs at fitness=0.95, then 10 current runs at fitness=0.60.
    // baseline mean = 0.95, current mean = 0.60, delta = 0.35 → emit.
    let mut last_verdict = None;
    for i in 0..20 {
        let run_id = format!("run_{:02}", i);
        let scope = format!("scope_{:02}", i);
        // Older inserts first; ran_at uses Utc::now() so insertion order ⇒ time order.
        let fitness = if i < 10 { 0.95 } else { 0.60 };
        insert_conformance_run(&store, &scope, &run_id, fitness, class);
        // Spread timestamps so ORDER BY ran_at DESC works deterministically.
        std::thread::sleep(std::time::Duration::from_millis(2));
        last_verdict = Some(regression::check_after_insert(&store, class).unwrap());
    }

    // The final verdict should report emitted=true (or no-op due to already-emitted idempotency).
    let final_v = last_verdict.unwrap();
    let emit_count: i64 = store.db().conn()
        .query_row(
            "SELECT COUNT(*) FROM ocel_events WHERE event_type = 'conformance_regression_detected'",
            [], |r| r.get(0)
        )
        .unwrap();
    assert_eq!(emit_count, 1, "exactly one regression event must be emitted");
    assert!(final_v.delta >= 0.10 || !final_v.emitted,
        "delta should clear regression threshold once both windows fill: {:?}", final_v);
}
