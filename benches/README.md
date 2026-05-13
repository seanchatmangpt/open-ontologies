# Benches — performance characterization for the admission + manufacturing path

**These are ENGINE benchmarks, not application throughput benchmarks.** Each
bench isolates the cost of a single component with all setup hoisted out of
the timed window. A regression in one stage cannot hide behind setup churn in
another. If you want application-level throughput numbers (sessions/sec, ops
behind a real RPC) measure them at the integration layer — these microbenches
will mislead you if read as such.

Criterion-based microbenchmarks for the OntoStar pipeline:

| Bench file               | Group                | What it measures                                            |
|--------------------------|----------------------|-------------------------------------------------------------|
| `admission_bench.rs`     | `evaluate_admission/powl_bridge_construct`     | `PowlBridgeReplay::new(&store)` only      |
| `admission_bench.rs`     | `evaluate_admission/powl_bridge_parse`         | `PowlBridge::new()` + `parse(powl)`        |
| `admission_bench.rs`     | `evaluate_admission/powl_bridge_replay_full`   | `replay(token, powl)` (setup hoisted)      |
| `admission_bench.rs`     | `evaluate_admission/full_noop`                 | full `evaluate` with `NoopPowlReplay`      |
| `admission_bench.rs`     | `evaluate_admission/full_real`                 | full `evaluate` with `PowlBridgeReplay`    |
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
cargo bench --bench admission_bench -- --quick full_real
```

## How to interpret

Criterion reports three numbers per bench: a point estimate, a 95 % confidence
interval, and (if `Throughput` was set) ns/elem or MiB/s.

* **Point estimate** — read this as the headline.
* **Confidence interval** — narrow CI ⇒ stable measurement; wide CI ⇒ noisy
  environment. Re-run on a quiet machine before publishing numbers.
* **Throughput** — for `validate_bundle` and `strip_header` it's bytes/s; for
  the other ops/s benches it's one element per call.

## Baseline targets

Approximate targets on Apple-Silicon class hardware (M-series, release build).
Anything substantially slower than these on the same class of hardware is a
regression worth investigating. Numbers are point estimates; expect ±15 %
variance in CI.

| Bench                                                  | Target (release, M-series)         |
|--------------------------------------------------------|------------------------------------|
| `evaluate_admission/powl_bridge_construct`             | ≤ 200 ns / op                      |
| `evaluate_admission/powl_bridge_parse`                 | ≤ 500 µs / op                      |
| `evaluate_admission/powl_bridge_replay_full`           | ≤ 2 ms / op                        |
| `evaluate_admission/full_noop`                         | ≤ 1 ms / op                        |
| `evaluate_admission/full_real`                         | ≤ 3 ms / op                        |
| `latest_for_session/10`                                | ≤ 100 µs                           |
| `latest_for_session/10000`                             | ≤ 5 ms                             |
| `manufacture/canonical_spec`                           | ≤ 2 ms / op                        |
| `validate_bundle/canonical_spec`                       | ≤ 100 µs / op                      |
| `strip_header/1kb_rust_plus_blake3`                    | ≥ 200 MiB/s                        |
| `swarm/run_breeds_all_nine`                            | ≤ 50 ms / op                       |
| `swarm/fuse_via_hearsay`                               | ≤ 20 ms / op                       |
| `swarm/manufacture_swarm`                              | ≤ 50 ms / op                       |
| `receipts/persist_with_tenant_in_tx`                   | ≤ 100 µs / op                      |
| `receipts/chain_walk_depth_100`                        | ≤ 5 ms / op                        |

> **Footnote on prior baselines.** A previous revision of this README cited
> 6.78 ms / 7.04 ms for `happy_path_real_replay` / `happy_path_noop_replay`.
> Those numbers measured `iter_batched` closures that re-built
> `PowlBridgeReplay`, opened a fresh sqlite DB, opened a `WorkflowScope`, and
> re-parsed the POWL string **inside the timed window**. Hoisting that setup
> out of the iter loop yields the values above (`full_noop ≤ 1 ms`,
> `full_real ≤ 3 ms`). The previous numbers are **not regressions** — they
> were measuring the wrong thing. They reflected setup churn, not engine
> cost. The new five-bench split exists so a regression in `parse`, `replay`,
> or the gate's alphabet check shows up where it lives instead of being
> averaged into a single confounded number.
>
> Likewise `receipts/persist_with_tenant_in_tx` previously showed ~4.6 ms
> because `iter_batched` opened a fresh tempdir + sqlite DB on every iter.
> The DB open is now hoisted; the iter mints a unique session id so each
> INSERT remains a sequence=1 first-row insert against an empty per-session
> chain. Expected ~100 µs.

## Constraints

* No mocks. Real `evaluate_admission`, real `manufacture()`, real
  `wasm4pm-cognition` breed dispatch, real `oxigraph`-backed `OcelStore`.
* Each bench's `--quick` mode finishes in < 30 s.
* Benches must not modify any existing `.rs` file in the crate; they live
  entirely under `benches/`.

## Hardware-independent invariants

The shape of the curve matters more than the absolute number:

* `evaluate_admission/powl_bridge_construct` < 1 µs on any modern hardware —
  it is a struct wrapper around a borrowed reference. A regression past 1 µs
  means an allocation crept in.
* `latest_for_session` should be near-flat as `N` grows (it uses the
  `(session_id, sequence)` index — confirmed by the `EXPLAIN QUERY PLAN`
  assertion that runs before the bench loop, which fails if the planner
  doesn't report `USING INDEX` or `USING COVERING INDEX`). Flat numbers
  across N are *correct* — the lookup is O(1) index seek, not O(N) scan.
  An O(N) regression here is a missing or ignored index.
* `chain_walk_depth_100` should be roughly 100× a single receipt fetch (the
  walker is O(depth)). A super-linear regression indicates an N+1 query.
* `manufacture/canonical_spec` should be byte-deterministic: re-running the
  bench on the same git revision should yield identical artifact bytes.
