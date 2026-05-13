//! Criterion harness — admission gate hot paths.
//!
//! These are **engine** benchmarks, not application throughput benchmarks.
//! They isolate the cost of individual admission components so a regression
//! in one stage cannot hide behind setup churn in another.
//!
//! Five focused benches measure the admission engine:
//!   - `evaluate_admission/powl_bridge_construct`      — `PowlBridgeReplay::new(&store)` only
//!   - `evaluate_admission/powl_bridge_parse`          — `PowlBridge::parse(powl)` only
//!   - `evaluate_admission/powl_bridge_replay_full`    — `replay(token, powl)` with hoisted setup
//!   - `evaluate_admission/full_noop`                  — full `evaluate` with `NoopPowlReplay`,
//!                                                       all setup hoisted out of the iter loop
//!   - `evaluate_admission/full_real`                  — full `evaluate` with `PowlBridgeReplay`,
//!                                                       all setup hoisted out of the iter loop
//!
//! Plus the chained-receipt index probe:
//!   - `latest_for_session/<N>` for N in {10, 100, 1000, 10000}
//!
//! Phase-10 constraint: this file must NOT modify any existing `.rs` file in
//! the crate. It uses only public API from `open_ontologies::admission`,
//! `open_ontologies::receipts`, and friends.
//!
//! ## Why setup is hoisted
//!
//! Prior versions of this harness called `b.iter_batched(setup_one, …)` where
//! `setup_one` opened a fresh sqlite DB, built an `OcelStore`, opened a
//! `WorkflowScope`, emitted three OCEL events, and reloaded `observed_event_types`.
//! Criterion times the closure body but the setup cost dominated and varied
//! iteration-to-iteration, polluting the measurement. The new layout pays
//! setup once and measures the engine call alone.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, NoopPowlReplay, OntoStarAdmissionGate, PowlBridgeReplay, PowlReplay,
};
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::powl_bridge::PowlBridge;
use open_ontologies::production_record::ProductionRecord;
use open_ontologies::receipts::{self, Receipt};
use open_ontologies::state::StateDb;
use open_ontologies::workflows::{by_name, WorkflowScope};
use tempfile::tempdir;

const WORKFLOW: &str = "DataExtensionFastPath";

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("admission-bench.db");
    // Leak the tempdir so the file outlives the bench iteration.
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn emit_stage(store: &OcelStore, session: &str, scope: &str, stage: &str, seq: u64) {
    let now = chrono::Utc::now().to_rfc3339();
    let event_id = format!("{session}:{seq:012}:{stage}");
    store
        .emit_event(&event_id, stage, &now, session, &[], &[], Some(scope))
        .unwrap();
}

/// Build (db, store, scope_token, session_id, observed_stages) ONCE for the
/// full-admission benches. Hoisted out of `b.iter()` so the timed window
/// measures only the engine call, not sqlite open + scope open + event emit.
fn setup_once() -> (StateDb, OcelStore, String, String, Vec<String>) {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "bench-sess-hoisted".to_string();
    let scope = WorkflowScope::new(&db, &session);
    let token = scope.open(Some(WORKFLOW), None, None).unwrap();
    scope.close(&token).unwrap();
    let mut seq = 0u64;
    for stage in &["load", "extend", "query"] {
        emit_stage(&store, &session, &token, stage, seq);
        seq += 1;
    }
    let observed = store.observed_event_types_for_session(&session).unwrap();
    (db, store, token, session, observed)
}

fn make_gate() -> OntoStarAdmissionGate {
    OntoStarAdmissionGate::new(
        0.95,
        0.85,
        by_name(WORKFLOW)
            .unwrap()
            .required_stages
            .iter()
            .map(|s| s.to_string())
            .collect(),
        "ontostar-1.0.0",
    )
}

fn bench_admission_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluate_admission");
    group.throughput(Throughput::Elements(1));

    // -------- powl_bridge_construct: PowlBridgeReplay::new(&store) only --------
    // Expectation: ~ns. This is just a struct wrapper around a borrowed &OcelStore.
    {
        let (_db, store, _token, _session, _observed) = setup_once();
        group.bench_function("powl_bridge_construct", |b| {
            b.iter(|| {
                let replay = PowlBridgeReplay::new(black_box(&store));
                black_box(replay);
            })
        });
    }

    // -------- powl_bridge_parse: PowlBridge::parse(powl) only --------
    // Expectation: ~µs. Re-builds the arena each iter (parse is the work being
    // measured); the wasm4pm parser walk is the costly step.
    {
        let powl = by_name(WORKFLOW).unwrap().powl_string;
        group.bench_function("powl_bridge_parse", |b| {
            b.iter(|| {
                let mut bridge = PowlBridge::new();
                let root = bridge.parse(black_box(powl)).unwrap();
                black_box(root);
            })
        });
    }

    // -------- powl_bridge_replay_full: replay(token, powl) with hoisted setup --------
    // Expectation: ~ms. Pure b.iter() — replay does an internal parse + project
    // + token-replay against the OCEL stream tagged with scope_token.
    {
        let (_db, store, token, _session, _observed) = setup_once();
        let powl = by_name(WORKFLOW).unwrap().powl_string;
        let replay = PowlBridgeReplay::new(&store);
        group.bench_function("powl_bridge_replay_full", |b| {
            b.iter(|| {
                let res = replay.replay(black_box(&token), black_box(powl));
                black_box(res);
            })
        });
    }

    // -------- full_noop: full evaluate against NoopPowlReplay, ALL setup hoisted --------
    // Expectation: ≤ 1 ms. NoopPowlReplay returns fitness=1.0/precision=1.0;
    // the rest of the gate (alphabet check, missing/extra stages, governance)
    // is what we are measuring.
    {
        let (_db, store, token, session, observed) = setup_once();
        let gate = make_gate();
        let powl = by_name(WORKFLOW).unwrap().powl_string;
        let artifact = ArtifactRef {
            kind: "bench",
            bytes: b"happy-bytes",
        };
        let replay = NoopPowlReplay;
        group.bench_function("full_noop", |b| {
            b.iter(|| {
                let res = gate.evaluate(
                    black_box(&token),
                    AdmissionOp::Apply,
                    black_box(&artifact),
                    &store,
                    &replay,
                    &session,
                    powl,
                    &observed,
                );
                black_box(res.ok());
            })
        });
    }

    // -------- full_real: full evaluate against PowlBridgeReplay, ALL setup hoisted --------
    // Expectation: ≤ 3 ms. Identical to full_noop except the replay impl is
    // the wasm4pm-backed bridge, which parses POWL + token-replays the OCEL.
    {
        let (_db, store, token, session, observed) = setup_once();
        let gate = make_gate();
        let powl = by_name(WORKFLOW).unwrap().powl_string;
        let artifact = ArtifactRef {
            kind: "bench",
            bytes: b"happy-bytes-real",
        };
        let replay = PowlBridgeReplay::new(&store);
        group.bench_function("full_real", |b| {
            b.iter(|| {
                let res = gate.evaluate(
                    black_box(&token),
                    AdmissionOp::Apply,
                    black_box(&artifact),
                    &store,
                    &replay,
                    &session,
                    powl,
                    &observed,
                );
                black_box(res.ok());
            })
        });
    }

    group.finish();
}

/// Pre-populate `n` chained receipts under one session, then time
/// `latest_for_session` lookup. The chain length stresses the
/// per-session ORDER BY sequence index in the receipts table.
///
/// Before timing, dump `EXPLAIN QUERY PLAN` for the lookup query and
/// assert at least one row uses an index. Flat numbers across N values
/// are *correct*: the lookup is O(1) — index seek to the tip — independent
/// of chain depth. A super-linear curve here means the index is missing.
fn bench_latest_for_session(c: &mut Criterion) {
    // Verify the query plan once, against the largest N. The same query
    // string is used for every N so a single check suffices.
    {
        let db = fresh_db();
        let session = "explain-probe";
        let r: Receipt = receipts::build(ProductionRecord {
            artifact_hash: [0u8; 32],
            scope_token: "scope-0".to_string(),
            declared_powl_hash: [0u8; 32],
            ocel_canonical_hash: [0u8; 32],
            conformance_run_id: "run-0".to_string(),
            gate_config_hash: [0u8; 32],
            production_law_version: "ontostar-1.0.0".into(),
            defects_taxonomy_version: open_ontologies::defects::DEFECTS_TAXONOMY_VERSION.into(),
            gates_passed: vec!["g".into()],
            gates_refused: vec![],
            prior_receipt: None,
            signature: None,
            signing_key_fpr: None,
        });
        receipts::persist(&r, &db, session).unwrap();

        let conn = db.conn();
        let mut stmt = conn
            .prepare(
                "EXPLAIN QUERY PLAN \
                 SELECT receipt_hash FROM receipts \
                 WHERE session_id = ?1 AND tenant_id = ?2 \
                 ORDER BY sequence DESC LIMIT 1",
            )
            .expect("prepare EXPLAIN QUERY PLAN");
        let mut rows = stmt
            .query(rusqlite::params![session, "default"])
            .expect("query EXPLAIN QUERY PLAN");
        let mut detail_blob = String::new();
        while let Some(row) = rows.next().expect("next row") {
            let detail: String = row.get(3).expect("plan detail column");
            eprintln!("EXPLAIN QUERY PLAN: {}", detail);
            detail_blob.push_str(&detail);
            detail_blob.push('\n');
        }
        assert!(
            detail_blob.contains("USING INDEX") || detail_blob.contains("USING COVERING INDEX"),
            "latest_for_session is not using an index — plan: {detail_blob}"
        );
    }

    let mut group = c.benchmark_group("latest_for_session");
    for &n in &[10u64, 100, 1000, 10000] {
        let db = fresh_db();
        let session = format!("chain-{n}");
        let mut prior: Option<[u8; 32]> = None;
        for i in 0..n {
            let record = ProductionRecord {
                artifact_hash: [(i & 0xff) as u8; 32],
                scope_token: format!("scope-{i}"),
                declared_powl_hash: [0u8; 32],
                ocel_canonical_hash: [0u8; 32],
                conformance_run_id: format!("run-{i}"),
                gate_config_hash: [0u8; 32],
                production_law_version: "ontostar-1.0.0".into(),
                defects_taxonomy_version: open_ontologies::defects::DEFECTS_TAXONOMY_VERSION
                    .into(),
                gates_passed: vec!["g".into()],
                gates_refused: vec![],
                prior_receipt: prior,
                signature: None,
                signing_key_fpr: None,
            };
            let r: Receipt = receipts::build(record);
            receipts::persist(&r, &db, &session).unwrap();
            prior = Some(r.bytes);
        }

        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let h = receipts::latest_for_session(black_box(&db), black_box(&session));
                black_box(h);
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_admission_components,
    bench_latest_for_session
);
criterion_main!(benches);
