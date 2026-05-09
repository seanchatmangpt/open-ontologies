# Benches — performance characterization for the admission + manufacturing path

Criterion-based microbenchmarks for the four hot paths of the OntoStar pipeline:

| Bench file               | Group                | What it measures                                            |
|--------------------------|----------------------|-------------------------------------------------------------|
| `admission_bench.rs`     | `evaluate_admission` | full `evaluate_admission` (Noop replay + real PowlBridge)   |
| `admission_bench.rs`     | `latest_for_session` | per-session receipt-tip lookup over chains of 10..10 000    |
| `manufacturing_bench.rs` | `manufacture`        | deterministic generator end-to-end (`SolutionSpec → bundle`) |
| `manufacturing_bench.rs` | `validate_bundle`    | post-generation validators                                   |
| `manufacturing_bench.rs` | `strip_header`       | header strip + BLAKE3 rehash of a 1 KB Rust file             |
| `swarm_bench.rs`         | `swarm`              | `run_breeds`, `fuse_via_hearsay`, `manufacture_swarm`        |
| `receipts_bench.rs`      | `receipts`           | `persist_with_tenant_in_tx`, `walk_receipt_chain` (depth 100) |

## How to run

The benches do **not** run under `make test`. Invoke Criterion directly:

```bash
# Smoke run — 1 sample per bench, finishes in seconds.
cargo bench --bench admission_bench     -- --quick
cargo bench --bench manufacturing_bench -- --quick
cargo bench --bench swarm_bench         -- --quick
cargo bench --bench receipts_bench      -- --quick

# Full statistical run with HTML reports under target/criterion/.
cargo bench --bench admission_bench
cargo bench --bench manufacturing_bench
cargo bench --bench swarm_bench
cargo bench --bench receipts_bench
```

Filter to one bench:

```bash
cargo bench --bench admission_bench -- --quick happy_path_real_replay
```

## How to interpret

Criterion reports three numbers per bench: a point estimate, a 95 % confidence
interval, and (if `Throughput` was set) ns/elem or MiB/s.

* **Point estimate** — read this as the headline.
* **Confidence interval** — narrow CI ⇒ stable measurement; wide CI ⇒ noisy
  environment. Re-run on a quiet machine before publishing numbers.
* **Throughput** — for `validate_bundle` and `strip_header` it's bytes/s; for
  `evaluate_admission`, `manufacture`, `run_breeds`, `fuse_via_hearsay`,
  `manufacture_swarm`, `persist_with_tenant_in_tx`, `chain_walk_depth_100` it's
  ops/s (one element per call).

## Baseline targets

Approximate targets on Apple-Silicon class hardware (M-series, release build).
Anything substantially slower than these on the same class of hardware is a
regression worth investigating. Numbers are point estimates; expect ±15 %
variance in CI.

| Bench                                          | Target (release, M-series)         |
|------------------------------------------------|------------------------------------|
| `evaluate_admission/happy_path_noop_replay`    | ≤ 5 ms / op                        |
| `evaluate_admission/happy_path_real_replay`    | ≤ 30 ms / op                       |
| `latest_for_session/10`                        | ≤ 100 µs                           |
| `latest_for_session/10000`                     | ≤ 5 ms                             |
| `manufacture/canonical_spec`                   | ≤ 2 ms / op                        |
| `validate_bundle/canonical_spec`               | ≤ 100 µs / op                      |
| `strip_header/1kb_rust_plus_blake3`            | ≥ 200 MiB/s                        |
| `swarm/run_breeds_all_nine`                    | ≤ 50 ms / op                       |
| `swarm/fuse_via_hearsay`                       | ≤ 20 ms / op                       |
| `swarm/manufacture_swarm`                      | ≤ 50 ms / op                       |
| `receipts/persist_with_tenant_in_tx`           | ≤ 2 ms / op (incl. fresh sqlite)   |
| `receipts/chain_walk_depth_100`                | ≤ 5 ms / op                        |

## Constraints

* No mocks. Real `evaluate_admission`, real `manufacture()`, real
  `wasm4pm-cognition` breed dispatch, real `oxigraph`-backed `OcelStore`.
* Each bench's `--quick` mode finishes in < 30 s.
* Benches must not modify any existing `.rs` file in the crate; they live
  entirely under `benches/`.

## Hardware-independent invariants

The shape of the curve matters more than the absolute number:

* `latest_for_session` should be near-flat as `N` grows (it uses the
  `(session_id, sequence)` index; an O(N) regression here is a missing index).
* `chain_walk_depth_100` should be roughly 100× a single receipt fetch (the
  walker is O(depth)). A super-linear regression indicates an N+1 query.
* `manufacture/canonical_spec` should be byte-deterministic: re-running the
  bench on the same git revision should yield identical artifact bytes.
