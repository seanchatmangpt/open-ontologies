//! Criterion harness — swarm hot paths.
//!
//! Measures:
//!   - `swarm/run_breeds_all_nine` — wasm4pm-cognition dispatch over 9 breeds
//!   - `swarm/fuse_via_hearsay`    — Hearsay-II fusion of 9 breed outputs
//!   - `swarm/manufacture_swarm`   — 9 × deterministic `manufacture()`

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use open_ontologies::swarm::{
    fuse_via_hearsay, manufacture_swarm, run_breeds, SWARM_BREEDS,
};
use wasm4pm_cognition::breeds::{
    BreedInput, Candidate, Case, Fact, Goal, Rule, StateAtom,
};

fn revops_scenario() -> BreedInput {
    BreedInput {
        intent: "RevOps revenue leakage detection across the booking pipeline at Fortune-5 scale"
            .to_string(),
        candidates: vec![
            Candidate {
                id: "centralized-revenue-engine".into(),
                score: 0.5,
                eliminated: false,
                elimination_reason: None,
            },
            Candidate {
                id: "edge-distributed-reconciliation".into(),
                score: 0.5,
                eliminated: false,
                elimination_reason: None,
            },
            Candidate {
                id: "hybrid-dual-write".into(),
                score: 0.4,
                eliminated: false,
                elimination_reason: None,
            },
        ],
        facts: vec![
            Fact { key: "scale".into(), value: "billion".into() },
            Fact { key: "leakage".into(), value: "detected".into() },
            Fact { key: "compliance".into(), value: "strict".into() },
            Fact { key: "current".into(), value: "no-architecture".into() },
        ],
        cases: vec![
            Case {
                id: "case-rev-001".into(),
                intent: "Booking reconciliation gap, contract chain partial".into(),
                architecture: "centralized-revenue-engine".into(),
                outcome_score: 0.92,
                facts: vec![Fact { key: "scale".into(), value: "billion".into() }],
            },
            Case {
                id: "case-rev-002".into(),
                intent: "Late partner attribution, edge-distributed handled it".into(),
                architecture: "edge-distributed-reconciliation".into(),
                outcome_score: 0.78,
                facts: vec![Fact { key: "leakage".into(), value: "detected".into() }],
            },
        ],
        rules: vec![
            Rule {
                id: "r1".into(),
                premise: vec!["scale=billion".into()],
                conclusion: "favor=centralized-revenue-engine".into(),
                certainty: 0.9,
            },
            Rule {
                id: "r2".into(),
                premise: vec!["leakage=detected".into()],
                conclusion: "risk=high".into(),
                certainty: 0.85,
            },
            Rule {
                id: "establish-arch".into(),
                premise: vec!["current=no-architecture".into()],
                conclusion: "performance=high".into(),
                certainty: 1.0,
            },
        ],
        goals: vec![Goal {
            id: "g_perf".into(),
            predicate: "performance".into(),
            value: "high".into(),
        }],
        state: vec![StateAtom {
            predicate: "current".into(),
            value: "no-architecture".into(),
        }],
    }
}

fn bench_run_breeds_all_nine(c: &mut Criterion) {
    let scenario = revops_scenario();
    let mut group = c.benchmark_group("swarm");
    group.throughput(Throughput::Elements(SWARM_BREEDS.len() as u64));
    group.bench_function("run_breeds_all_nine", |b| {
        b.iter(|| {
            let reports = run_breeds(black_box(&scenario));
            black_box(reports.len());
        })
    });
    group.finish();
}

fn bench_fuse_via_hearsay(c: &mut Criterion) {
    let scenario = revops_scenario();
    let reports = run_breeds(&scenario);

    let mut group = c.benchmark_group("swarm");
    group.throughput(Throughput::Elements(reports.len() as u64));
    group.bench_function("fuse_via_hearsay", |b| {
        b.iter(|| {
            let consensus = fuse_via_hearsay(black_box(&scenario), black_box(&reports));
            black_box(consensus.consensus_trace_steps);
        })
    });
    group.finish();
}

fn bench_manufacture_swarm(c: &mut Criterion) {
    let work_order_hash = "a".repeat(64);
    let mut group = c.benchmark_group("swarm");
    group.throughput(Throughput::Elements(SWARM_BREEDS.len() as u64));
    group.bench_function("manufacture_swarm", |b| {
        b.iter(|| {
            let nodes = manufacture_swarm(black_box("bench"), black_box(&work_order_hash))
                .expect("swarm manufacture ok");
            black_box(nodes.len());
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_run_breeds_all_nine,
    bench_fuse_via_hearsay,
    bench_manufacture_swarm
);
criterion_main!(benches);
