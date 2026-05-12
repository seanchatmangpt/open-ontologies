<!-- mcp-name: io.github.fabio-rovai/open-ontologies -->

# OntoStar

**Receipt-bound recursive admission for AI-manufactured software.**

[![CI](https://img.shields.io/badge/CI-pending-lightgrey?style=flat-square)](https://github.com/fabio-rovai/open-ontologies/actions)
[![docs](https://img.shields.io/badge/docs-docs%2F-blue?style=flat-square)](docs/00-overview.md)
[![crate](https://img.shields.io/badge/crate-pending-lightgrey?style=flat-square)](https://crates.io/crates/open-ontologies)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)

---

## Why this exists

LLMs cannot be trusted as authority. They produce plausible artifacts at high speed,
but plausibility is not provenance, and a passing test does not prove that a lawful
process happened. OntoStar is the layer that turns an LLM into a *manufacturing
operator* whose every output must pass through admission gates before it is allowed
to exist.

Requirements, work orders, mutations, and emitted artifacts are not the same kind
of object — they break in different ways and they need different gates.
Requirements need CTQ admission. Work orders need ontological alignment. Mutations
need conformance replay. Emitted artifacts need cryptographic receipts. OntoStar
gives each its own gate and refuses to let upstream admission excuse downstream
failure.

Every artifact carries its receipt of admission. The receipt is a BLAKE3-chained
record of (a) which gate granted it, (b) what the inputs hashed to, (c) which
session and tenant it belongs to, and (d) what came before it in the chain.
When a signing key is configured (`OPEN_ONTOLOGIES_SIGNING_KEY_PATH`), each
record is additionally Ed25519-signed; the Cell8 A10 conjunct then verifies the
signature with `verify_strict` against a trust set loaded from
`OPEN_ONTOLOGIES_TRUSTED_KEYS_DIR`. Without a signing key, receipts are emitted
unsigned and admitted by A10 only when `[admission] verify_legacy_receipts =
true`; otherwise A10 raises `DefectClass::AttestationMissing`. An external
verifier can replay the chain offline. Fix-forward is the only way out.

## Architecture

```
                    Requirement (NL / DSPy signature)
                            │
                            ▼
      ┌────────────────────────────────────────────────┐
      │  Groq LLM boundary  (translate_candidate)       │
      │  pm4py POWL pattern, DSPy-shaped signatures     │
      └────────────────────────────────────────────────┘
                            │  POWL candidate
                            ▼
      ┌────────────────────────────────────────────────┐
      │  CTQ admission gate     [requirements-andon]    │
      │  conjuncts: ocel_complete · powl_replay ·       │
      │             precision≥0.7 · stages_present      │
      └────────────────────────────────────────────────┘
                            │  WorkOrder (admitted)
                            ▼
      ┌────────────────────────────────────────────────┐
      │  Manufacturing       [IaC · Rust · Erlang ·     │
      │                       AtomVM]                   │
      │  9-breed cognition swarm fused via Hearsay-II   │
      └────────────────────────────────────────────────┘
                            │  Artifact + provenance
                            ▼
      ┌────────────────────────────────────────────────┐
      │  Cell8 13-gate attestation (A1…A13)             │
      │  EARL emitter · BLAKE3 chain · Ed25519 seal     │
      └────────────────────────────────────────────────┘
                            │  Receipt
                            ▼
                  External verifier (`onto verify`)
```

## Quickstart

```bash
git clone https://github.com/fabio-rovai/open-ontologies && cd open-ontologies
cargo build --release
./target/release/open-ontologies mcp start --transport stdio       # serve
./target/release/open-ontologies verify --receipt .ggen/receipts/latest.json
```

A receipt that fails to verify exits non-zero and prints the broken link in the
chain. There is no `--force` flag.

**Multi-tenancy.** Single-tenant by default. Multi-tenant isolation requires
explicit tenant declaration: header `X-Ontostar-Tenant: <id>` for HTTP, or
`OPEN_ONTOLOGIES_TENANT_ID=<id>` for stdio. Tenants must match
`^[a-z][a-z0-9_-]{0,63}$`. Cross-tenant scope access is denied with
`DefectClass::TenantBoundary`.

## What's in the box

| Capability | Module | Tests |
|---|---|---|
| Real Groq LLM via DSPy / pm4py POWL | `src/llm_translator.rs` + `scripts/*.py` | 14 real-Groq tests (`tests/real_groq_*.rs`) |
| 9-breed cognition swarm (Rust + AtomVM, Hearsay-II) | `src/swarm.rs` + `wasm4pm-cognition` | 4 (`tests/real_swarm_e2e.rs`) |
| Manufacturing (IaC / Rust / Erlang / AtomVM) | `src/manufacturing/` | 8 + 5 real-toolchain (`tests/manufacturing_validators.rs`, `tests/adversarial_real_toolchains.rs`) |
| External receipt verifier | `src/verify.rs` + `cmds/governance.rs` | 10 (`tests/external_verifier_e2e.rs`) |
| Multi-tenant isolation + scope-token ACLs | `src/tenant.rs` | 7 (`tests/multi_tenant_isolation.rs`) |
| Cell8 13-gate attestation (A1–A13) + EARL | `src/cell8.rs` | 8 (`tests/cell8_thirteen_gates.rs`) |
| Recursive admission (CTQ → WorkOrder → Manufacturing) | `src/admission.rs` + `src/cell_ready.rs` | `tests/admission*.rs`, `tests/recursive_admission_e2e.rs` |
| Receipt chain (BLAKE3 + opt-in Ed25519, atomic persist+emit) | `src/receipts.rs`, `src/attestation.rs` | `tests/receipt_chain_adversarial.rs`, `tests/ed25519_attestation.rs` |
| RDF / OWL / SPARQL / SHACL via Oxigraph | `src/graph.rs` `src/shacl.rs` | `tests/graph_test.rs`, `tests/shacl_test.rs` |
| 50+ MCP tools (`onto_*`) over stdio / HTTP | `src/server.rs` + `src/cmds/` | `tests/onto_integration_test.rs` |

**Test totals:** 597 `#[test]` functions across `tests/` (regenerated by
`tools/check-test-count.sh`). 20 of them call real Groq (require
`GROQ_API_KEY`); 5 run real toolchains (`terraform`, `cargo`, `erlc`, AtomVM);
the rest run hermetically.

## Status

Branch: `ontostar-integration`. Phases 1–9 + 11 landed; Phase 10 (Cell8 13-gate)
has source + tests in tree, awaiting consolidation commit. See
[`CHANGELOG.md`](CHANGELOG.md) for the per-phase commit map.

## Documentation

- [`docs/00-overview.md`](docs/00-overview.md) — system overview
- [`docs/02-quickstart.md`](docs/quickstart.md) — quickstart walkthrough
- [`docs/lifecycle.md`](docs/lifecycle.md) — Terraform-style lifecycle
- [`CHANGELOG.md`](CHANGELOG.md) — phase-by-phase history with commit hashes
- [`CLAUDE.md`](CLAUDE.md) — agent instructions and tool catalog

## License & contribution

MIT. Issues and PRs welcome on
[github.com/fabio-rovai/open-ontologies](https://github.com/fabio-rovai/open-ontologies).
Read [`.claude/rules/coding-agent-mistakes.md`](.claude/rules/coding-agent-mistakes.md)
before submitting — every patch must either deepen authority or reduce drift.
