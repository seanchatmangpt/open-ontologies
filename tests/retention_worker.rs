//! Round 4 WD — `RetentionWorker::tick()` retirement-path proof.
//!
//! Each test seeds a single table with rows whose timestamp is in the
//! past, runs `tick()` synchronously with `*_days = 0`, and asserts the
//! expected rows were pruned. The cascade-order test is the load-bearing
//! one: if `ocel_event_attrs` were not pruned BEFORE `ocel_events`, the
//! foreign-key cascade would either fail or leak orphan attrs.
//!
//! Counterfactual proof (§19): mutate `RetentionConfig { ocel_days: 9999 }`
//! and rerun — every row must remain. Without the worker, the database
//! grows without bound (§29 retirement absence).

use open_ontologies::config::RetentionConfig;
use open_ontologies::retention::RetentionWorker;
use open_ontologies::state::StateDb;
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("retention-worker-test.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn aggressive_cfg() -> RetentionConfig {
    RetentionConfig {
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
    }
}

fn count(db: &StateDb, sql: &str) -> i64 {
    db.conn().query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap()
}

fn one_hour_ago() -> String {
    (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339()
}

#[test]
fn tick_prunes_ocel_events_with_cascade_order() {
    let db = fresh_db();
    // Seed 100 OCEL events with time = now - 1 hour, plus child attrs and
    // relationships referencing them. The foreign-key cascade (children
    // first, parents last) is the load-bearing invariant.
    {
        let conn = db.conn();
        for i in 0..100 {
            let event_id = format!("evt-{i}");
            conn.execute(
                "INSERT INTO ocel_events (event_id, event_type, time, session_id, scope_token, tenant_id)
                 VALUES (?1, 'tested', ?2, 'session-x', 'scope-y', 'default')",
                rusqlite::params![event_id, one_hour_ago()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO ocel_event_attrs (event_id, name, value)
                 VALUES (?1, 'k', 'v')",
                rusqlite::params![event_id],
            )
            .unwrap();
            // Need an object first for the FK on relationships.
            let obj_id = format!("obj-{i}");
            conn.execute(
                "INSERT OR IGNORE INTO ocel_objects (object_id, object_type, created_at)
                 VALUES (?1, 'test_obj', ?2)",
                rusqlite::params![obj_id, one_hour_ago()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO ocel_relationships (event_id, object_id, qualifier)
                 VALUES (?1, ?2, 'declares')",
                rusqlite::params![event_id, obj_id],
            )
            .unwrap();
        }
    }
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ocel_events"), 100);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ocel_event_attrs"), 100);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ocel_relationships"), 100);

    let worker = RetentionWorker::new(db.clone(), aggressive_cfg());
    let report = worker.tick().expect("tick");

    assert_eq!(report.ocel_events_pruned, 100);
    assert_eq!(report.ocel_event_attrs_pruned, 100);
    assert_eq!(report.ocel_relationships_pruned, 100);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ocel_events"), 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ocel_event_attrs"), 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ocel_relationships"), 0);
}

#[test]
fn tick_prunes_lineage_events() {
    let db = fresh_db();
    {
        let conn = db.conn();
        for i in 0..10 {
            conn.execute(
                "INSERT INTO lineage_events (session_id, seq, timestamp, event_type, operation, details)
                 VALUES ('s', ?1, ?2, 't', 'op', 'd')",
                rusqlite::params![i, one_hour_ago()],
            )
            .unwrap();
        }
    }
    assert_eq!(count(&db, "SELECT COUNT(*) FROM lineage_events"), 10);
    let worker = RetentionWorker::new(db.clone(), aggressive_cfg());
    let report = worker.tick().expect("tick");
    assert_eq!(report.lineage_events_pruned, 10);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM lineage_events"), 0);
}

#[test]
fn tick_prunes_conformance_runs() {
    let db = fresh_db();
    {
        let conn = db.conn();
        for i in 0..5 {
            conn.execute(
                "INSERT INTO conformance_runs (run_id, scope_token, fitness, precision,
                 verdict, defects_json, trace_canonical_hash, ran_at)
                 VALUES (?1, 'scope', 0.99, 0.99, 'conform', '[]', '', ?2)",
                rusqlite::params![format!("r-{i}"), one_hour_ago()],
            )
            .unwrap();
        }
    }
    let worker = RetentionWorker::new(db.clone(), aggressive_cfg());
    let r = worker.tick().expect("tick");
    assert_eq!(r.conformance_runs_pruned, 5);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM conformance_runs"), 0);
}

#[test]
fn tick_prunes_revoked_sessions() {
    let db = fresh_db();
    {
        let conn = db.conn();
        for i in 0..3 {
            conn.execute(
                "INSERT INTO revoked_sessions (session_id, reason, revoked_at, tenant_id)
                 VALUES (?1, 'test', ?2, 'default')",
                rusqlite::params![format!("rev-{i}"), one_hour_ago()],
            )
            .unwrap();
        }
    }
    let worker = RetentionWorker::new(db.clone(), aggressive_cfg());
    let r = worker.tick().expect("tick");
    assert_eq!(r.revoked_sessions_pruned, 3);
}

#[test]
fn tick_prunes_mined_exemplars() {
    let db = fresh_db();
    {
        let conn = db.conn();
        for i in 0..4 {
            conn.execute(
                "INSERT INTO mined_exemplars (id, domain, problem_context, powl_string,
                 fitness, receipt_hash, mined_at)
                 VALUES (?1, 'd', 'pc', 'p', 0.9, 'rh', ?2)",
                rusqlite::params![format!("e-{i}"), one_hour_ago()],
            )
            .unwrap();
        }
    }
    let worker = RetentionWorker::new(db.clone(), aggressive_cfg());
    let r = worker.tick().expect("tick");
    assert_eq!(r.mined_exemplars_pruned, 4);
}

#[test]
fn tick_prunes_align_feedback() {
    let db = fresh_db();
    {
        let conn = db.conn();
        for _ in 0..6 {
            conn.execute(
                "INSERT INTO align_feedback (source_iri, target_iri, predicted_relation,
                 accepted, timestamp)
                 VALUES ('s', 't', 'p', 1, ?1)",
                rusqlite::params![one_hour_ago()],
            )
            .unwrap();
        }
    }
    let worker = RetentionWorker::new(db.clone(), aggressive_cfg());
    let r = worker.tick().expect("tick");
    assert_eq!(r.align_feedback_pruned, 6);
}

#[test]
fn tick_prunes_tool_feedback() {
    let db = fresh_db();
    {
        let conn = db.conn();
        for _ in 0..2 {
            conn.execute(
                "INSERT INTO tool_feedback (tool, rule_id, entity, accepted, timestamp)
                 VALUES ('lint', 'r', 'e', 1, ?1)",
                rusqlite::params![one_hour_ago()],
            )
            .unwrap();
        }
    }
    let worker = RetentionWorker::new(db.clone(), aggressive_cfg());
    let r = worker.tick().expect("tick");
    assert_eq!(r.tool_feedback_pruned, 2);
}

#[test]
fn tick_does_not_prune_when_window_excludes_rows() {
    // Counterfactual proof — long retention windows mean nothing is pruned.
    let db = fresh_db();
    {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO ocel_events (event_id, event_type, time, session_id, scope_token, tenant_id)
             VALUES ('e', 't', ?1, 's', 'k', 'default')",
            rusqlite::params![one_hour_ago()],
        )
        .unwrap();
    }
    let cfg = RetentionConfig {
        ocel_days: 9999,
        ..aggressive_cfg()
    };
    let worker = RetentionWorker::new(db.clone(), cfg);
    let report = worker.tick().expect("tick");
    assert_eq!(report.ocel_events_pruned, 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM ocel_events"), 1);
}
