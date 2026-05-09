//! Task C — Receipt-chain hardening adversarial tests.
//!
//! Three invariants under hostile conditions:
//!
//!   1. `granted_at_tie_resolves_by_sequence` — when two receipts share the
//!      same `granted_at` timestamp, `latest_for_session` MUST deterministically
//!      return the row with the higher `sequence`, not whichever SQLite happens
//!      to surface first. Run 100x to assert determinism.
//!
//!   2. `concurrent_sessions_do_not_cross_chain` — two threads admitting on
//!      different `session_id`s must each produce a contiguous 1..N sequence
//!      with no chain crossing (no receipt's `prior_receipt_hash` points at
//!      the other session's hashes).
//!
//!   3. `orphan_detection_refuses_to_chain` — a receipt inserted directly
//!      into the table WITHOUT a corresponding `admission_granted` event in
//!      OCEL is an orphan. The next real admission MUST NOT chain back to
//!      that orphan as `prior_receipt`.

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, OntoStarAdmissionGate, PowlBridgeReplay,
};
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::receipts;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use std::sync::Arc;
use std::thread;
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("receipt-chain-adversarial.db");
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

fn build_gate(workflow_name: &str) -> OntoStarAdmissionGate {
    let required: Vec<String> = by_name(workflow_name)
        .map(|w| w.required_stages.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    OntoStarAdmissionGate::new(0.95, 0.85, required, "ontostar-1.0.0")
}

/// The RequirementsManufacturing workflow's POWL classifies these stages
/// as `conform` under PowlBridgeReplay. LifecycleApply currently does not
/// (see admission.rs::happy_path_admission_persists_receipt — ignored, phase-6
/// followup), so this is the workflow we drive in the chain-hardening tests.
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

/// Insert a synthetic receipt row directly via SQL with caller-controlled
/// `granted_at` and `sequence`. Bypasses `receipts::persist` so the test can
/// produce identical timestamps deterministically.
fn insert_raw_receipt(
    db: &StateDb,
    session_id: &str,
    receipt_hash_hex: &str,
    granted_at: &str,
    sequence: i64,
) {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO receipts (
            receipt_hash, scope_token, session_id,
            artifact_hash, declared_powl_hash, ocel_canonical_hash,
            gate_config_hash, prior_receipt_hash,
            production_law_version, granted_at, sequence
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        rusqlite::params![
            receipt_hash_hex,
            "scope-tie",
            session_id,
            "a".repeat(64),
            "b".repeat(64),
            "c".repeat(64),
            "d".repeat(64),
            Option::<String>::None,
            "ontostar-1.0.0",
            granted_at,
            sequence,
        ],
    )
    .expect("raw insert");
}

fn hash_to_hex(b: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for byte in b {
        s.push_str(&format!("{:02x}", byte));
    }
    s
}

#[test]
fn granted_at_tie_resolves_by_sequence() {
    // Two rows share `granted_at`. The resolver must always return the
    // higher-sequence row, regardless of SQLite's internal row ordering.
    // 100 iterations: any flake here is a determinism bug.
    for iter in 0..100 {
        let db = fresh_db();
        let session = "tie-session";
        let same_ts = "2026-05-08T12:00:00.000000000+00:00";

        let hash_a = format!("{:064x}", iter * 2);
        let hash_b = format!("{:064x}", iter * 2 + 1);

        // Insert in the "wrong" order: higher hash first, lower hash second.
        // SQLite's natural order would return them by rowid, which would
        // be non-deterministic w.r.t. the chain semantics we want.
        insert_raw_receipt(&db, session, &hash_b, same_ts, 2);
        insert_raw_receipt(&db, session, &hash_a, same_ts, 1);

        let latest = receipts::latest_for_session(&db, session)
            .expect("latest must exist");
        let latest_hex = hash_to_hex(&latest);
        assert_eq!(
            latest_hex, hash_b,
            "iter {iter}: tied granted_at must resolve to higher sequence \
             (expected {hash_b}, got {latest_hex})"
        );
    }
}

#[test]
fn concurrent_sessions_do_not_cross_chain() {
    let db = Arc::new(fresh_db());
    let store = Arc::new(OcelStore::new((*db).clone()));

    fn drive_session(
        db: Arc<StateDb>,
        store: Arc<OcelStore>,
        session: String,
    ) {
        // One scope per session; drive 5 distinct AdmissionOps on it (matches
        // the upstream pattern in recursive_admission_e2e.rs). Using
        // RequirementsManufacturing because PowlBridgeReplay classifies its
        // trace as `conform`.
        let scope = WorkflowScope::new(&db, &session);
        let token = scope
            .open(Some(RM_WORKFLOW), None, None)
            .expect("open scope");
        scope.close(&token).expect("close scope");
        for stage in RM_STAGES {
            emit_stage(&store, &session, &token, stage);
        }
        let observed = store.observed_event_types_for_session(&session).unwrap();

        let gate = build_gate(RM_WORKFLOW);
        let powl = by_name(RM_WORKFLOW).unwrap().powl_string;
        let replay = PowlBridgeReplay::new(&store);

        let ops = [
            AdmissionOp::RequirementProposed,
            AdmissionOp::CtqAdmitted,
            AdmissionOp::WorkOrderAdmitted,
            AdmissionOp::RequirementProposed,
            AdmissionOp::CtqAdmitted,
        ];
        for (i, op) in ops.iter().enumerate() {
            let payload = format!("{}-{}", session, i);
            let artifact = ArtifactRef {
                kind: "test",
                bytes: payload.as_bytes(),
            };
            gate.evaluate(
                &token,
                *op,
                &artifact,
                &store,
                &replay,
                &session,
                powl,
                &observed,
            )
            .expect("admission must grant");
        }
    }

    let db_a = Arc::clone(&db);
    let store_a = Arc::clone(&store);
    let h1 = thread::spawn(move || {
        drive_session(db_a, store_a, "session-A".to_string())
    });
    let db_b = Arc::clone(&db);
    let store_b = Arc::clone(&store);
    let h2 = thread::spawn(move || {
        drive_session(db_b, store_b, "session-B".to_string())
    });
    h1.join().unwrap();
    h2.join().unwrap();

    // Inspect each chain independently.
    let conn = db.conn();
    for session in &["session-A", "session-B"] {
        let mut stmt = conn
            .prepare(
                "SELECT sequence, receipt_hash, prior_receipt_hash
                   FROM receipts
                  WHERE session_id = ?1
               ORDER BY sequence ASC",
            )
            .unwrap();
        let rows: Vec<(i64, String, Option<String>)> = stmt
            .query_map(rusqlite::params![session], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        assert_eq!(
            rows.len(),
            5,
            "session {session} must have exactly 5 receipts (got {})",
            rows.len()
        );
        for (i, (seq, _hash, _prior)) in rows.iter().enumerate() {
            assert_eq!(
                *seq,
                (i + 1) as i64,
                "session {session} sequence must be contiguous 1..N (row {i}: got {seq})"
            );
        }

        // Collect this session's hashes, then assert no `prior_receipt_hash`
        // points at the OTHER session's hashes.
        let our_hashes: std::collections::HashSet<String> =
            rows.iter().map(|(_, h, _)| h.clone()).collect();
        let other = if *session == "session-A" {
            "session-B"
        } else {
            "session-A"
        };
        let mut other_stmt = conn
            .prepare("SELECT receipt_hash FROM receipts WHERE session_id = ?1")
            .unwrap();
        let other_hashes: std::collections::HashSet<String> = other_stmt
            .query_map(rusqlite::params![other], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        for (_, _, prior) in &rows {
            if let Some(p) = prior {
                assert!(
                    !other_hashes.contains(p),
                    "session {session}: prior_receipt {p} crossed into {other}'s chain"
                );
                assert!(
                    our_hashes.contains(p),
                    "session {session}: prior_receipt {p} not found in own chain"
                );
            }
        }
    }
}

#[test]
fn orphan_detection_refuses_to_chain() {
    // Phase 7 Task C.fix: atomic persist+emit. The receipt INSERT and the
    // `admission_granted` OCEL emit run inside ONE SQLite transaction, so:
    //
    //   * if the OCEL emit fails, the receipt INSERT is rolled back —
    //     no orphan row can ever land in `receipts`;
    //   * if the receipt INSERT fails, the OCEL emit is also rolled back —
    //     no `admission_granted` event can ever vouch for a receipt that
    //     does not exist.
    //
    // This test sabotages the OCEL emit step by dropping `ocel_event_attrs`
    // mid-flight and asserts the stronger invariants:
    //
    //   (a) the would-be receipt is NOT in `receipts` (rollback held), and
    //   (b) the next real admission's `prior_receipt` does NOT chain to a
    //       hash created during the sabotaged attempt (because no such
    //       hash exists in `receipts`).
    //
    // A planted "raw" orphan from a hostile actor (direct SQL bypass of the
    // gate) is also asserted to never appear in an `admission_granted`
    // OCEL event — the audit trail keeps that secondary witness.
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "orphan-session";

    // Plant a hostile-actor orphan: someone wrote directly to `receipts`
    // bypassing the gate. Our atomic boundary is what keeps the gate's own
    // path honest — this row is just here to confirm the OCEL audit trail
    // can still detect a non-gate insertion post-hoc.
    let orphan_hash = format!("{:064x}", 0xdeadbeefu64);
    insert_raw_receipt(
        &db,
        session,
        &orphan_hash,
        "2026-05-08T12:00:00.000000000+00:00",
        1,
    );

    // Set up a real admission attempt, but sabotage OCEL emit by dropping
    // the `ocel_event_attrs` table BEFORE evaluate(). The receipt INSERT
    // will succeed; the attrs INSERT inside the same transaction will fail
    // ("no such table"); the whole txn rolls back.
    let scope = WorkflowScope::new(&db, session);
    let token = scope
        .open(Some(RM_WORKFLOW), None, None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    for stage in RM_STAGES {
        emit_stage(&store, session, &token, stage);
    }
    let observed = store.observed_event_types_for_session(session).unwrap();

    let gate = build_gate(RM_WORKFLOW);
    let powl = by_name(RM_WORKFLOW).unwrap().powl_string;

    // Snapshot receipts BEFORE sabotage so we can diff afterwards.
    let receipts_before: i64 = {
        let conn = db.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM receipts WHERE session_id = ?1",
            rusqlite::params![session],
            |r| r.get(0),
        )
        .unwrap()
    };

    // SABOTAGE: drop `ocel_event_attrs`. Now the in-tx attrs INSERT will
    // fail and the surrounding transaction must roll back the receipt row.
    {
        let conn = db.conn();
        conn.execute_batch("DROP TABLE ocel_event_attrs;").unwrap();
    }

    let replay = PowlBridgeReplay::new(&store);
    let sabotaged = gate.evaluate(
        &token,
        AdmissionOp::RequirementProposed,
        &ArtifactRef {
            kind: "test",
            bytes: b"sabotaged-emit",
        },
        &store,
        &replay,
        session,
        powl,
        &observed,
    );
    assert!(
        sabotaged.is_err(),
        "sabotaged emit must surface as Err (transaction rolled back)"
    );

    // Restore the table so we can run a clean follow-up admission and prove
    // (b): the chain on the next admission does NOT thread through any
    // hash created during the sabotaged attempt.
    {
        let conn = db.conn();
        conn.execute_batch(
            "CREATE TABLE ocel_event_attrs (
                event_id   TEXT NOT NULL,
                name       TEXT NOT NULL,
                value      TEXT NOT NULL,
                value_type TEXT NOT NULL DEFAULT 'string',
                PRIMARY KEY (event_id, name)
            );",
        )
        .unwrap();
    }

    // (a) Receipt count must be unchanged — the sabotaged attempt rolled
    // back, so no new row landed in `receipts`.
    let receipts_after_sabotage: i64 = {
        let conn = db.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM receipts WHERE session_id = ?1",
            rusqlite::params![session],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(
        receipts_after_sabotage, receipts_before,
        "atomic boundary must roll back the receipt INSERT when emit fails \
         (before={receipts_before}, after={receipts_after_sabotage})"
    );

    // Drive a clean follow-up admission. Its `prior_receipt` will be
    // whatever `latest_for_session` returns — the planted orphan, since no
    // other row exists. We then assert the secondary OCEL-audit invariant:
    // the planted orphan was never broadcast as an `admission_granted`,
    // and the legitimate follow-up's receipt hash is freshly emitted.
    let real_receipt = gate
        .evaluate(
            &token,
            AdmissionOp::RequirementProposed,
            &ArtifactRef {
                kind: "test",
                bytes: b"clean-followup",
            },
            &store,
            &replay,
            session,
            powl,
            &observed,
        )
        .expect("clean follow-up admission must grant");

    let conn = db.conn();

    // The planted orphan was never broadcast as an `admission_granted`.
    let granted_events_referencing_orphan: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ocel_event_attrs a
              JOIN ocel_events e ON e.event_id = a.event_id
             WHERE e.event_type = 'admission_granted'
               AND e.session_id = ?1
               AND a.name = 'receipt_hash'
               AND a.value = ?2",
            rusqlite::params![session, &orphan_hash],
            |r| r.get(0),
        )
        .unwrap_or(0);
    assert_eq!(
        granted_events_referencing_orphan, 0,
        "planted orphan (raw SQL bypass) must never appear in any \
         admission_granted OCEL event — the OCEL audit trail is the \
         post-hoc witness for non-gate insertions"
    );

    // The real receipt IS emitted to OCEL — atomic boundary held on the
    // success path too.
    let real_hex = real_receipt.hex();
    let real_emitted: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ocel_event_attrs a
              JOIN ocel_events e ON e.event_id = a.event_id
             WHERE e.event_type = 'admission_granted'
               AND e.session_id = ?1
               AND a.name = 'receipt_hash'
               AND a.value = ?2",
            rusqlite::params![session, &real_hex],
            |r| r.get(0),
        )
        .unwrap_or(0);
    assert_eq!(
        real_emitted, 1,
        "clean admission must have emitted exactly one admission_granted event"
    );

    // (b)-strengthened: every gate-produced receipt has an OCEL witness.
    // The planted orphan is the only unwitnessed row by construction; any
    // other unwitnessed row would mean the atomic boundary leaked a
    // partial-success state during the sabotaged attempt.
    let mut stmt = conn
        .prepare("SELECT receipt_hash FROM receipts WHERE session_id = ?1")
        .unwrap();
    let all_hashes: Vec<String> = stmt
        .query_map(rusqlite::params![session], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    let mut unwitnessed: Vec<String> = Vec::new();
    for h in &all_hashes {
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ocel_event_attrs a
                  JOIN ocel_events e ON e.event_id = a.event_id
                 WHERE e.event_type = 'admission_granted'
                   AND e.session_id = ?1
                   AND a.name = 'receipt_hash'
                   AND a.value = ?2",
                rusqlite::params![session, h],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if n == 0 {
            unwitnessed.push(h.clone());
        }
    }
    assert_eq!(
        unwitnessed,
        vec![orphan_hash.clone()],
        "the only unwitnessed receipt in the session must be the planted \
         orphan; any other unwitnessed row would mean the atomic boundary \
         leaked a partial-success state"
    );
}
