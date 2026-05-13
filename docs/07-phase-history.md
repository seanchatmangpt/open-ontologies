# 07 — Phase History

Each phase shipped one capability and closed one gap. Commit hashes are real; cross-reference with `git log ontostar-integration`.

## Phase 1 — Stream integration foundation

Commits `571a6c0`, `8c3537a`, `1a587c7`. Integrated streams 1–5 into a single working tree on the `ontostar-integration` branch, fixed three post-merge test issues, and shipped an end-to-end DoD test plus a `build_ocel` session-filter fix. The gap closed: prior streams could each pass in isolation but together produced runtime defects that no test caught. The integration commit forced every stream to be co-tested.

## Phase 2 — Real PowlBridge wired into admission gate

Commits `33feda7`, `e4db225`. Replaced the `NoopPowlReplay` stub with `PowlBridgeReplay::new(store)` in production-path admission. The gap closed: 12 admission tests had been validating against a fitness=1.0 stub. Production now routes through `wasm4pm` for every admission; only four gate-semantics tests retain the noop with an explicit `// INTENTIONAL` annotation.

## Phase 3 — Requirements Andon (CTQ Forge)

Commits `c33086f`, `fb419f5`, `5f60633`, `41c5696`, `663d924`, `53f9713`, `5f808d2`, `233f99c`, `1e57bd4`, `d3cfdc1`. Built the four-tier recursive admission claim: requirements → CTQs → work orders → manufacture. Wired six MCP handlers, an `RequirementsManufacturing` workflow, the Fortune-5 RevOps governed-release path, eight negative tests, four Groq boundary tests, nine station tests, counterfactual binding, and the Fortune-5 E2E. The gap closed: stakeholder voice could enter the system without typed admission; now `RequirementWithoutSource` and `CtqIncomplete` defects deny on the proposal path.

## Phase 4 — Solution manufacturing (IaC + Rust + Erlang + AtomVM)

Commit `eb0b8ca`. Autonomic multi-target deterministic generators for Terraform JSON, Rust crate, Erlang/OTP supervision tree, and AtomVM module. Every emitted file carries an OntoStar receipt header (Terraform uses sidecar — fixed in `c4e0035` after an adversarial audit caught broken IaC). The gap closed: admission gate could grant a "manufacture" operation without producing artifacts that compile under real toolchains. Now `cargo check`, `erlc`, and `terraform validate` are run by the validators in `tests/manufacturing_validators.rs`.

## Phase 5 — DSPy signature shapes

Commit `286d47b`. Closed the LLM-to-manufacturing gap with DSPy-style signature shapes — the language-to-contract boundary. `SignatureShape` defines field descriptions, demos, and post-hoc validation. The shaped translator molds Groq's output before generation and gauges it after. Refine loop retries on typed `ValidationFailure`. The gap closed: LLM output had been entering the system as free text with no constraint; now it is shaped before generation and gauged after.

## Phase 5.5 — Real-LLM cascade

Commits `1b7d6cc`, `619c3b1`, `b5cdca7`. Ported the pm4py POWL example with REAL Groq calls, then propagated the pattern across five LLM boundaries (CTQ, executive projection, plan workflow, POWL refine, solution spec), then enforced Chicago-TDD: real Groq at every human interaction point, no mocks. The gap closed: mocked LLM tests proved the test harness, not the boundary.

## Phase swarm — 9 cognition breeds + Hearsay-II

Commit `a2c2a56`. Manufactures nine AtomVM cognition nodes (one per wasm4pm breed: ELIZA, CBR, DENDRAL, STRIPS, Prolog, MYCIN, GPS, SOAR, Hearsay) using the deterministic `manufacture()` pipeline; runs each breed against a shared scenario; fuses outputs via Hearsay-II. The gap closed: the manufacturing pipeline had no consumer that *used* multi-target output; the swarm proves the bundle's Erlang half actually runs and the cognition breeds can fuse.

## Phase 6 — Adversarial hardening cascade

Commits `9bd0611`, `0527773`, `f367fed`, `3ed427a`, `063d540`, `bea21b4`. Five-task fix-forward against a 5-Explore + 5-Plan audit. Task A switched 12 tests to real `PowlBridgeReplay`. Task B repaired 25 silently-broken CLI subprocess tests broken at commit `362fd6b`. Task C added per-session sequence column with adversarial tests. Task D added deny-path tests for 12 production-active variants and removed 10 dead variants (taxonomy v3.0.0). Task E hardened the no-bypass and secret-grep ratchets and fixed three lying allowlist entries. The gap closed: Phase 1–5 had been validating against stubs and silent test breakage. Phase 6 surfaced and closed every audit finding.

## Phase 7 — Receipt chain hardening + #[ignore] closure

Commits `0ab7577`, `f44ec7e`, `4eb2dfb`. Closed all four Phase-6 `#[ignore]` tags (happy-path admission, push verb, file-backed Oxigraph) and added atomic persist+emit transaction with orphan rollback. The gap closed: orphan receipts from emit-failure-after-persist could ghost the chain. Now persist+emit is one transaction.

## Phase 8 — Live Groq subprocess engine

Commit `c8d5588`. `onto_translate_candidate` and `onto_executive_projection` MCP handlers invoke a live Groq subprocess. The gap closed: tests had real-Groq coverage but the MCP handlers themselves did not — clients calling the tool got mock output.

## Phase 9 — External verifier CLI + library

Commit `9a4a277`. `onto verify <bundle>` (CLI) plus `crate::verify` (library API) — pure read-only walker, re-implementable in any language. The gap closed: receipts were chained but no external tool could prove the chain without booting the full server.

## Phase 10 — Cell8 13-gate conformance

Commit `f44ec7e` (chain hardening) + the in-progress Cell8 work in `src/cell8.rs` and `tests/cell8_thirteen_gates.rs`. Expands `cell_ready` from 8 to 13 conjuncts, adds Cell8 EARL emitter with hand-written byte-stable Turtle. The gap closed: external auditors had no SHACL-validatable conformance attestation per receipt.

## Phase 11 — Multi-tenant session isolation

Commit `cd8b3b2`. `TenantContext` (env: `OPEN_ONTOLOGIES_TENANT_ID`) carried alongside `session_id`. ACL enforced by admission gate via `TenantBoundary` defect when caller's tenant ≠ scope's owning tenant. The gap closed: a single server could serve multiple tenants but had no ACL on cross-tenant scope access.
