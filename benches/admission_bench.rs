//! Criterion harness — admission gate hot paths.
//!
//! Measures `evaluate_admission` end-to-end against a pre-staged DataExtensionFastPath
//! workflow (which is a pure SEQ workflow whose alphabet equals its required stages —
//! see `tests/admission.rs::happy_path_admission_persists_receipt` for rationale).
//!
//! Runs three benches:
//!   - `evaluate_admission/happy_path_noop_replay`
//!   - `evaluate_admission/happy_path_real_replay`
//!   - `latest_for_session/<N>` for N in {10, 100, 1000, 10000}
//!
//! Phase-10 constraint: this file must NOT modify any existing `.rs` file in the
//! crate. It uses only public API from `open_ontologies::admission`,
//! `open_ontologies::receipts`, and friends.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use open_ontologies::admission::{
    AdmissionOp, ArtifactRef, NoopPowlReplay, OntoStarAdmissionGate, PowlBridgeReplay,
};
use open_ontologies::ocel_store::OcelStore;
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

/// Build a fresh (db, store, scope_token, session_id, observed_stages) tuple
/// every time. Each iteration of the bench needs a clean session because
/// the gate persists a receipt on success and we don't want chain growth
/// to bias the measurement.
fn setup_one() -> (StateDb, OcelStore, String, String, Vec<String>) {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = format!("bench-sess-{n}");
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

fn bench_evaluate_admission_happy_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluate_admission");
    group.throughput(Throughput::Elements(1));

    group.bench_function("happy_path_noop_replay", |b| {
        b.iter_batched(
            setup_one,
            |(_db, store, token, session, observed)| {
                let gate = OntoStarAdmissionGate::new(
                    0.95,
                    0.85,
                    by_name(WORKFLOW)
                        .unwrap()
                        .required_stages
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    "ontostar-1.0.0",
                );
                let powl = by_name(WORKFLOW).unwrap().powl_string;
                let artifact = ArtifactRef {
                    kind: "bench",
                    bytes: b"happy-bytes",
                };
                let res = gate.evaluate(
                    black_box(&token),
                    AdmissionOp::Apply,
                    black_box(&artifact),
                    &store,
                    &NoopPowlReplay,
                    &session,
                    powl,
                    &observed,
                );
                black_box(res.ok());
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("happy_path_real_replay", |b| {
        b.iter_batched(
            setup_one,
            |(_db, store, token, session, observed)| {
                let gate = OntoStarAdmissionGate::new(
                    0.95,
                    0.85,
                    by_name(WORKFLOW)
                        .unwrap()
                        .required_stages
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    "ontostar-1.0.0",
                );
                let powl = by_name(WORKFLOW).unwrap().powl_string;
                let artifact = ArtifactRef {
                    kind: "bench",
                    bytes: b"happy-bytes-real",
                };
                let replay = PowlBridgeReplay::new(&store);
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
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// Pre-populate `n` chained receipts under one session, then time
/// `latest_for_session` lookup. The chain length stresses the
/// per-session ORDER BY sequence index in the receipts table.
fn bench_latest_for_session(c: &mut Criterion) {
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
    bench_evaluate_admission_happy_path,
    bench_latest_for_session
);
criterion_main!(benches);
