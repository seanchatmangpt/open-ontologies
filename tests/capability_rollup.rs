//! Level-5 Capability-evidence gate sabotage test.
//!
//! Proves that `workflow_capability` accumulates per-class admission
//! statistics (admission_count, success_count, failure_count, sum_fitness,
//! sum_precision, first/last_admitted_at, defects_taxonomy_version) across
//! repeated admissions of the same workflow class. If the UPSERT in
//! `StateDb::record_capability` regresses to insert-only or stops being
//! invoked from `OntoStarAdmissionGate::evaluate`, these tests fail loudly.

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, OntoStarAdmissionGate, PowlBridgeReplay,
};
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("capability_rollup.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn emit_stage(store: &OcelStore, session: &str, scope: &str, stage: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let event_id = format!(
        "{}:{}:{}",
        session,
        stage,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    store
        .emit_event(&event_id, stage, &now, session, &[], &[], Some(scope))
        .unwrap();
}

#[allow(clippy::type_complexity)]
fn read_capability(
    db: &StateDb,
    workflow_name: &str,
) -> (i64, i64, i64, f64, f64, Option<String>, Option<String>, String) {
    let conn = db.conn();
    conn.query_row(
        "SELECT admission_count, success_count, failure_count,
                sum_fitness, sum_precision,
                first_admitted_at, last_admitted_at,
                defects_taxonomy_version
         FROM workflow_capability
         WHERE workflow_name = ?1",
        rusqlite::params![workflow_name],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        },
    )
    .expect("workflow_capability row must exist")
}

fn admit_once(db: &StateDb, store: &OcelStore, session: &str) -> Result<(), String> {
    let scope = WorkflowScope::new(db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");
    for stage in &["load", "extend", "query"] {
        emit_stage(store, session, &token, stage);
    }
    scope.close(&token).expect("close scope");

    let powl = by_name("DataExtensionFastPath").unwrap().powl_string;
    let replay = PowlBridgeReplay::new(store);
    let gate = OntoStarAdmissionGate::new(0.0, 0.0, vec![], "ontostar-1.0.0");
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"a",
    };
    let observed: Vec<String> = store
        .observed_event_types_for_session(session)
        .unwrap_or_default();
    gate.evaluate(
        &token,
        AdmissionOp::Apply,
        &artifact,
        store,
        &replay,
        session,
        powl,
        &observed,
    )
    .map(|_| ())
    .map_err(|e| format!("{:?}", e))
}

#[test]
fn admission_count_accumulates_across_three_successful_admissions() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());

    for i in 0..3 {
        let session = format!("cap-rollup-ok-{}", i);
        // Ensure last_admitted_at strictly advances across iterations.
        std::thread::sleep(std::time::Duration::from_millis(10));
        admit_once(&db, &store, &session).expect("admission must succeed");
    }

    let (
        admission_count,
        success_count,
        failure_count,
        sum_fitness,
        _sum_precision,
        first_at,
        last_at,
        taxonomy,
    ) = read_capability(&db, "DataExtensionFastPath");

    assert_eq!(admission_count, 3, "admission_count must accumulate to 3");
    assert_eq!(success_count, 3, "success_count must be 3");
    assert_eq!(failure_count, 0, "failure_count must remain 0 on all-success");
    assert!(
        sum_fitness >= 2.85,
        "sum_fitness should be ≥ 3 × 0.95 = 2.85, got {}",
        sum_fitness
    );
    let first = first_at.expect("first_admitted_at must be populated");
    let last = last_at.expect("last_admitted_at must be populated");
    assert!(
        last >= first,
        "last_admitted_at ({}) must be >= first_admitted_at ({})",
        last,
        first
    );
    assert_eq!(
        taxonomy, "ontostar-defects-1.0.0",
        "defects_taxonomy_version must match canonical taxonomy"
    );
}

#[test]
fn failure_count_increments_on_denial() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "cap-rollup-deny";
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some("DataExtensionFastPath"), None, None)
        .expect("open scope");
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, session, &token, stage);
    }
    scope.close(&token).expect("close scope");

    let powl = by_name("DataExtensionFastPath").unwrap().powl_string;
    let replay = PowlBridgeReplay::new(&store);
    // High thresholds AND a required stage that never appears in the trace
    // — the gate must deny.
    let gate = OntoStarAdmissionGate::new(
        0.99,
        0.99,
        vec!["NEVER_FIRES".to_string()],
        "ontostar-1.0.0",
    );
    let artifact = ArtifactRef {
        kind: "test",
        bytes: b"x",
    };
    let observed: Vec<String> = store
        .observed_event_types_for_session(session)
        .unwrap_or_default();
    let result = gate.evaluate(
        &token,
        AdmissionOp::Apply,
        &artifact,
        &store,
        &replay,
        session,
        powl,
        &observed,
    );
    assert!(result.is_err(), "admission must be denied");

    let (admission_count, _success, failure_count, _sf, _sp, _first, _last, taxonomy) =
        read_capability(&db, "DataExtensionFastPath");

    assert!(
        admission_count >= 1,
        "admission_count must increment on denial, got {}",
        admission_count
    );
    assert!(
        failure_count >= 1,
        "failure_count must be ≥ 1 after denial, got {}",
        failure_count
    );
    assert_eq!(
        taxonomy, "ontostar-defects-1.0.0",
        "defects_taxonomy_version must be canonical"
    );
}
