# 02 — Quickstart

This guide takes you from a clean checkout to a real admission, a real Groq translation, and a real swarm consensus run. Allow about 20 minutes for first-time toolchain installation.

## Prerequisites

| Tool | Why | Install |
|------|-----|---------|
| Rust 1.90+ | Build the server and run tests | `rustup toolchain install stable` |
| `erlc` (OTP 26+) | Real Erlang/AtomVM validator (Phase 4) | `brew install erlang` / `apt install erlang` |
| `terraform` 1.6+ | Real IaC validator (Phase 4) | `brew install terraform` |
| `make` | Wraps cargo with SHACL/dead-param gates | preinstalled |
| `GROQ_API_KEY` | Real-LLM tests (Phase 5/8) | get key at console.groq.com |

## Build

```bash
git clone <repo> open-ontologies
cd open-ontologies
git checkout ontostar-integration
make check        # cargo check via Makefile (NEVER use cargo directly)
make test         # full test suite
```

`make adversarial` is the gate that must pass before claiming any feature complete. It runs the dead-param gate, clippy deny list, the JTBD adversarial tests, and the full test suite.

## Start the server

```bash
# stdio transport (Claude Code integration)
cargo run --bin onto -- mcp start --transport stdio

# HTTP transport (remote)
cargo run --bin onto -- mcp start --transport http --port 3050
```

Configure `OPEN_ONTOLOGIES_TENANT_ID` to scope all admissions to a tenant; otherwise tenant defaults to `"default"`.

## First admission (recursive admission claim, end-to-end)

```bash
# 1. Propose a requirement (must cite source_evidence_uri)
onto requirements propose --voice "lead-time should drop below 2 days" \
    --source-evidence-uri "interview://stakeholder-7"

# 2. Translate via Groq into a CandidateCtq (LLM proposes)
GROQ_API_KEY=$GROQ_API_KEY onto translate candidate --requirement-id <id>

# 3. Admit the CTQ (deterministic gate; can deny with CtqIncomplete)
onto ctq admit --candidate-id <id>

# 4. Propose and admit a work order (must cite counterfactual)
onto work-order propose --ctq-id <id>
onto work-order admit --work-order-id <id>

# 5. Manufacture solution bundle (IaC + Rust + Erlang + AtomVM)
onto manufacturing solution --work-order-id <id>

# 6. Verify the chain externally — pure read-only
onto verify ./out/bundle/
```

## Real Groq translation example

```bash
export GROQ_API_KEY="gsk_..."
cargo test --test real_groq_powl -- --test-threads=1 --nocapture
```

Expected output: a candidate POWL string molded by a `SignatureShape`, then validated by `wasm4pm::parse`. If the LLM emits invalid POWL, the refine loop retries with a typed `ValidationFailure` until the shape gauges pass or the budget is exhausted. See `tests/real_groq_powl.rs`.

## Swarm test

```bash
cargo test --test real_swarm_e2e -- --nocapture --test-threads=1
```

This manufactures nine AtomVM nodes (one per cognition breed), runs each against a shared `BreedInput` scenario, and fuses the outputs via Hearsay-II. Every node's bundle passes real `cargo check` + real `erlc` + real `terraform validate` before any breed dispatches.

## Verify a bundle

```bash
onto verify ./out/bundle/
# {"is_valid": true, "chain_length": 4, "seed_receipt": "blake3:abcd..."}
```

The verifier walks `prior_receipt` back to the seed without touching the live triple store. Implementable in any language; the protocol is documented in `docs/05-receipt-chain.md`.

## Next steps

- Read `docs/03-mcp-tools.md` for the full tool catalogue.
- Read `docs/06-llm-boundary.md` to understand why the LLM is a transducer, not an authority.
- Run `make adversarial` and read `docs/08-running-tests.md` to wire CI.
