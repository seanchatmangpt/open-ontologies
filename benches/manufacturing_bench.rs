//! Criterion harness — solution manufacturing hot paths.
//!
//! Measures:
//!   - `manufacture/canonical_spec` — full multi-target generation
//!   - `validate_bundle/canonical_spec` — post-generation validators
//!   - `strip_header/1kb_rust` — receipt-header strip + BLAKE3 rehash
//!
//! All measurements use real `manufacture()` from `open_ontologies::manufacturing`.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use open_ontologies::manufacturing::{self, validators, SolutionSpec};

fn ok_spec() -> SolutionSpec {
    SolutionSpec {
        name: "bench_revops".into(),
        description: "Bench RevOps manufactured stack".into(),
        iac_target: "aws".into(),
        region: "us-east-1".into(),
        supervisor_children: 6,
        mcu_target: "esp32".into(),
        work_order_receipt_hash: "a".repeat(64),
    }
}

fn bench_manufacture_canonical_spec(c: &mut Criterion) {
    let spec = ok_spec();
    let mut group = c.benchmark_group("manufacture");
    group.throughput(Throughput::Elements(1));
    group.bench_function("canonical_spec", |b| {
        b.iter(|| {
            let bundle = manufacturing::manufacture(black_box(&spec)).expect("manufacture ok");
            black_box(bundle.files.len());
        })
    });
    group.finish();
}

fn bench_validate_bundle(c: &mut Criterion) {
    let spec = ok_spec();
    let bundle = manufacturing::manufacture(&spec).expect("manufacture ok");

    let mut group = c.benchmark_group("validate_bundle");
    group.throughput(Throughput::Bytes(bundle.total_bytes() as u64));
    group.bench_function("canonical_spec", |b| {
        b.iter(|| {
            let r = validators::validate_bundle(black_box(&bundle));
            black_box(r.ok());
        })
    });
    group.finish();
}

fn bench_strip_header(c: &mut Criterion) {
    // Build a synthetic 1KB Rust file with a real-looking ostar header.
    let header = "\
// ostar-production-law: ontostar-1.0.0\n\
// ostar-defects-taxonomy: ontostar-defects-1.0.0\n\
// ostar-receipt-hash: 1111111111111111111111111111111111111111111111111111111111111111\n\
// ostar-artifact-hash: 2222222222222222222222222222222222222222222222222222222222222222\n\
// ostar-scope-token: scope-bench\n\
// ostar-prior-receipt: none\n";
    let mut body = String::with_capacity(1024);
    while body.len() < 1024 {
        body.push_str("pub fn answer() -> u32 { 42 }\n");
    }
    let contents = format!("{header}{body}");
    let bytes_total = contents.len();

    let mut group = c.benchmark_group("strip_header");
    group.throughput(Throughput::Bytes(bytes_total as u64));
    group.bench_function("1kb_rust_plus_blake3", |b| {
        b.iter(|| {
            let stripped = validators::strip_header(black_box(&contents), "//");
            let h = blake3::hash(stripped.as_bytes());
            black_box(*h.as_bytes());
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_manufacture_canonical_spec,
    bench_validate_bundle,
    bench_strip_header
);
criterion_main!(benches);
