//! Criterion harness — receipt persistence + chain-walk hot paths.
//!
//! Measures:
//!   - `receipts/persist_with_tenant_in_tx` — atomic INSERT under a transaction
//!   - `receipts/chain_walk_depth_100`      — `walk_receipt_chain` over a 100-deep chain

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use open_ontologies::production_record::ProductionRecord;
use open_ontologies::receipts::{self, persist_with_tenant_in_tx, Receipt};
use open_ontologies::state::StateDb;
use open_ontologies::verify::walk_receipt_chain;
use tempfile::tempdir;

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("receipts-bench.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn mk_record(i: u64, prior: Option<[u8; 32]>) -> ProductionRecord {
    ProductionRecord {
        artifact_hash: [(i & 0xff) as u8; 32],
        scope_token: format!("scope-{i}"),
        declared_powl_hash: [0u8; 32],
        ocel_canonical_hash: [0u8; 32],
        conformance_run_id: format!("run-{i}"),
        gate_config_hash: [0u8; 32],
        production_law_version: "ontostar-1.0.0".into(),
        defects_taxonomy_version: open_ontologies::defects::DEFECTS_TAXONOMY_VERSION.into(),
        gates_passed: vec!["g".into()],
        gates_refused: vec![],
        prior_receipt: prior,
        signature: None,
        signing_key_fpr: None,
    }
}

fn bench_persist_with_tenant_in_tx(c: &mut Criterion) {
    let mut group = c.benchmark_group("receipts");
    group.throughput(Throughput::Elements(1));
    // Hoist the sqlite open out of the timed window. Each iter mints a
    // unique session id so the INSERT is always sequence=1 in a fresh
    // (session_id, tenant_id) chain — comparable iteration-to-iteration
    // without paying tempdir + sqlite open cost (~4.5 ms) on every iter.
    let db = fresh_db();
    let counter = std::sync::atomic::AtomicU64::new(0);
    group.bench_function("persist_with_tenant_in_tx", |b| {
        b.iter(|| {
            let n = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let session = format!("persist-bench-{n}");
            let r: Receipt = receipts::build(mk_record(n, None));
            let mut conn = db.conn();
            let tx = conn.transaction().unwrap();
            persist_with_tenant_in_tx(
                &tx,
                black_box(&r),
                black_box(&session),
                black_box("default"),
            )
            .unwrap();
            tx.commit().unwrap();
        })
    });
    group.finish();
}

fn bench_chain_walk_depth_100(c: &mut Criterion) {
    // Build one shared 100-deep chain.
    let db = fresh_db();
    let session = "walk-bench";
    let mut prior: Option<[u8; 32]> = None;
    let mut tip: [u8; 32] = [0u8; 32];
    for i in 0..100u64 {
        let r: Receipt = receipts::build(mk_record(i, prior));
        receipts::persist(&r, &db, session).unwrap();
        prior = Some(r.bytes);
        tip = r.bytes;
    }

    let mut group = c.benchmark_group("receipts");
    group.throughput(Throughput::Elements(100));
    group.bench_function("chain_walk_depth_100", |b| {
        b.iter(|| {
            let chain = walk_receipt_chain(black_box(&db), black_box(&tip));
            black_box(chain.len());
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_persist_with_tenant_in_tx,
    bench_chain_walk_depth_100
);
criterion_main!(benches);
