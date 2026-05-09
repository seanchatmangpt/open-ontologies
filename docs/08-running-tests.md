# 08 — Running Tests

The full verification matrix. Every gate listed here must be green for an honest "feature complete" claim.

## Always use `make`, never `cargo` directly

`make check` / `make test` / `make adversarial` wrap cargo with the dead-param gate, clippy deny list, SHACL validation, and lineage recording. Direct `cargo` bypasses these. (See `CLAUDE.md` rule §6.)

## Standard gates

```bash
make check            # cargo check, dead-param-gate, clippy
make test             # full integration sweep
make adversarial      # all of the above + JTBD adversarial tests
```

## Real-toolchain tests (Phase 4)

These shell out to real `cargo`, `erlc`, and `terraform`. Required before claiming Phase 4 manufacturing works.

```bash
cargo test --test adversarial_real_toolchains -- --nocapture --test-threads=1
cargo test --test manufacturing_validators
cargo test --test solution_manufacturing_e2e
```

`erlc` must be on `$PATH` (check with `command -v erlc`). `terraform` must be ≥ 1.6. Tests are serialized (`--test-threads=1`) so concurrent compilation does not race over `target/`.

## Real-Groq tests (Phase 5/8)

Set `GROQ_API_KEY`, then run with `--test-threads=1` to respect rate limits. Each test makes at least one live LLM call.

```bash
export GROQ_API_KEY="gsk_..."

for t in real_groq_powl real_groq_ctq real_groq_executive_projection \
         real_groq_plan_workflow real_groq_solution_spec real_groq_powl_refine \
         real_groq_chicago_e2e real_groq_mcp_handler; do
  cargo test --test $t -- --test-threads=1
  sleep 2
done
```

The fourteen real-LLM tests (counting sub-tests inside the binaries above) cover every human-interaction point. Mocked variants exist for fast CI, but Chicago-TDD discipline says "real Groq or it didn't happen" at the boundary.

## Real-swarm tests (Phase swarm)

```bash
cargo test --test real_swarm_e2e -- --nocapture --test-threads=1
```

Manufactures all nine cognition nodes, runs each breed, fuses via Hearsay-II. Requires `erlc` on `$PATH`. Four sub-tests cover breed dispatch, manufactured-bundle compile, fusion consensus, and per-node receipt chain.

## Adversarial ratchets (Phase 6 Task E)

```bash
cargo test --test no_bypass_audit       # every MCP handler is gated/audited/explicit-RO
cargo test --test secret_grep_ratchet   # alias + tracing-field + format-string detection
cargo test --test ratchet_red_team      # 8 known bypass patterns; each must be caught
```

The ratchets are static-text scanners over `src/server.rs`. They reject `let _ = self.evaluate_admission`, string-literal `"evaluate_admission("`, dead-code blocks (`if false { ... }`, `cfg!(any())`), and one level of helper transitive scanning.

## Phase-6 task gates (regression)

```bash
cargo test --test admission --test admission_real_replay     # Task A
cargo test --test cli_test --test adversarial_jtbd_test      # Task B
cargo test --test receipt_chain_adversarial                  # Task C
cargo test --test manufacturing_validators                   # Task D
cargo test --test cell_ready_deny_paths                      # Task D
cargo test --lib                                             # taxonomy pin test
```

## Cell8 13-gate conformance (Phase 10)

```bash
cargo test --test cell8_thirteen_gates
```

Each test asserts that one Cell8 gate (A1–A13) passes on a known-good fixture and fails on a known-bad fixture, emitting an EARL `outcome` of pass/fail respectively.

## Multi-tenant (Phase 11)

```bash
cargo test --test multi_tenant_isolation
```

Cross-tenant scope access must produce `TenantBoundary { caller_tenant, scope_tenant }`.

## External verifier (Phase 9)

```bash
cargo test --test external_verifier_e2e
```

Strip-and-rehash protocol round-trips for Rust, Erlang, TTL, and Terraform-sidecar carrier formats.

## OTEL validation (per `~/.claude/rules/otel-validation.md`)

Tests passing is not sufficient for any feature touching an external service. Run with tracing enabled:

```bash
export RUST_LOG=trace,onto=trace,ggen=trace
cargo test --test admission_real_replay -- --nocapture 2>&1 | tee /tmp/otel.txt
grep -E "onto\.load|onto\.query|onto\.validate" /tmp/otel.txt
```

A passing test without span output is unverified. See the rule file for required attributes per span type.

## What "green" means

`make adversarial` exits 0 **and** every real-toolchain test in this doc has been run within the last commit. Skipping real-toolchain tests under "they're slow" is the NARRATION failure mode. The Andon protocol applies — stop the line and fix.
