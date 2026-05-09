# Changelog

All notable changes to OntoStar / open-ontologies are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions are organized per OntoStar phase on the `ontostar-integration` branch.
Pre-OntoStar history (the original `open-ontologies` MCP server, releases
0.1.x) is summarized at the bottom.

## [Unreleased] — Phase 10 — Cell8 13-gate attestation

Source and tests for the Cell8 A1–A13 conformance gates plus the EARL emitter
are in tree (`src/cell8.rs`, `tests/cell8_thirteen_gates.rs` — 8 tests).
Awaiting the Phase-10 consolidation commit; this entry will be retagged as
`[0.10.0]` with the hash once it lands.

### Added (in tree, not yet tagged)
- `src/cell8.rs` — A1 Seed, A2 Breed, A3 Validate, A4 Reason, A5 Prove, A6 Seal, A7 Emit, A8 Journal, A9 Causal, A10 Temporal, A11 Governance, A12 Rollback, A13 Attest.
- EARL `earl:Assertion` emitter with `earl:passed` / `earl:failed` outcomes.
- Gate dependency ordering (A1 → A7 sequential; A8–A13 parallel).
- `tests/cell8_thirteen_gates.rs` — one test per gate plus full-cascade.

## [0.11.0] — Phase 11 — Multi-tenant isolation

- [`cd8b3b2`](../../commit/cd8b3b2) `feat(tenant): multi-tenant session isolation + scope-token ACLs (Phase 11)`

### Added
- `src/tenant.rs` — `TenantBoundary` defect class; per-tenant receipt chains; scope-token ACLs gating every mutation handler.
- `tests/multi_tenant_isolation.rs` — 7 isolation tests covering cross-tenant leak, scope escalation, chain bleed-through, and session-stealing attacks.

### Security
- All mutation handlers now require a tenant-scoped token; absence is `TenantBoundary::MissingScope` (hard deny, not warn).

## [0.9.0] — Phase 9 — External verifier

- [`9a4a277`](../../commit/9a4a277) `feat(verifier): external receipt verifier CLI + library API (Phase 9)`

### Added
- `src/verify.rs` + `onto verify` CLI verb — replays a receipt chain offline, no network, no shared state with the producer.
- ASCII chain visualization (`onto verify --visualize`) showing BLAKE3 link integrity at each hop.
- Library API (`open_ontologies::verify::verify_chain`) for embedding in CI.
- `tests/external_verifier_e2e.rs` — 10 tests including corrupted-signature, broken-link, and key-rotation sabotage scenarios.

## [0.8.0] — Phase 8 — Live MCP-Groq integration

- [`c8d5588`](../../commit/c8d5588) `feat(mcp-groq): live Groq subprocess engine for translate_candidate + executive_projection (Phase 8)`

### Added
- `engine="groq_pm4py"` subprocess transport — MCP handlers now spawn the real DSPy/pm4py-backed translator instead of the in-process stub.
- Live execution path covers `translate_candidate` and `executive_projection`.

### Changed
- `tests/real_groq_*.rs` (14 tests across 8 files) now exercise the live subprocess instead of the mock.

## [0.7.0] — Phase 7 — Phase 6 consolidation

- [`f44ec7e`](../../commit/f44ec7e) `feat(receipts): atomic persist+emit transaction; orphan rollback (Phase 7 C.fix)`
- [`0ab7577`](../../commit/0ab7577) `test(phase-7): close all 4 Phase-6 #[ignore] tags — happy-path admission, push verb, file-backed Oxigraph`

### Fixed
- Receipt persist + emit is now a single atomic transaction; partial-write orphans roll back instead of leaving an unsigned skeleton on disk (closes Phase-6 finding 3.3).
- All 5 Phase-6 `#[ignore]` markers removed; happy-path admission, push verb, and file-backed Oxigraph re-enter the regular test run.

## [0.6.0] — Phase 6 — Adversarial hardening cascade

Five parallel hardening tasks (A–E) closing the findings of the 5-Explore + 5-Plan
adversarial audit.

- [`9bd0611`](../../commit/9bd0611) **Task A** — `test(real-replay): switch admission tests from NoopPowlReplay to PowlBridgeReplay`. Replaces fitness=1.0 stubs with the real wasm4pm bridge across 7 test files.
- [`0527773`](../../commit/0527773) **Task B** — `test(cli): adapt CLI subprocess tests to noun-verb structure`. Revives 25 CLI subprocess tests broken by the `362fd6b` flat→noun-verb refactor.
- [`f367fed`](../../commit/f367fed) **Task C** — `feat(receipts): per-session sequence column + 3 adversarial tests`. Closes receipt-chain silent-failure modes (granted_at tie / concurrent sessions / orphaned receipt).
- [`3ed427a`](../../commit/3ed427a) **Task D part 1** — `test(defects): deny-path tests for 12 production-active variants`.
- [`bea21b4`](../../commit/bea21b4) **Task D part 2** — `feat(defects)!: bump taxonomy to 3.0.0 — remove 10 speculative dead variants`. **BREAKING**: `DefectClass` enum loses 10 unused variants.
- [`063d540`](../../commit/063d540) **Task E** — `feat(ratchets)!: harden no_bypass_audit + secret_grep_ratchet, fix 3 allowlist lies`. **BREAKING**: read-only allowlist contract tightened; `onto_workflow_discover` reclassified as mutating.
- [`4eb2dfb`](../../commit/4eb2dfb) `test(ratchet): char/string-literal-aware brace walker in no_bypass_audit (v2)` — follow-up fix to ratchet false-positives on string literals containing braces.

### Added
- `tests/admission_real_replay.rs`, `tests/receipt_chain_adversarial.rs`, `tests/ratchet_red_team.rs`, `tests/cell_ready_deny_paths.rs`.

### Removed
- 10 speculative `DefectClass` variants with zero production emissions.

### Security
- `secret_grep_ratchet` now scans format strings and `tracing::*!` macro literals (previously bypassable).

## [0.5.0] — Phase 5 — DSPy-style signature shapes

- [`286d47b`](../../commit/286d47b) `feat(signatures): Phase 5 — DSPy-style signature shapes close the LLM-to-manufacturing gap`
- [`a2c2a56`](../../commit/a2c2a56) `feat(swarm): 9 Rust+AtomVM cognition nodes fused via Hearsay-II`
- [`b5cdca7`](../../commit/b5cdca7) `test(chicago-tdd): real Groq at every human interaction point`
- [`619c3b1`](../../commit/619c3b1) `feat(real-llm): port pm4py POWL pattern across 5 LLM boundaries — REAL Groq calls`
- [`1b7d6cc`](../../commit/1b7d6cc) `feat(real-llm): port pm4py POWL example with REAL Groq calls`
- [`c4e0035`](../../commit/c4e0035) `fix(audit): adversarial audit caught broken Terraform IaC; receipt moved to sidecar`
- [`da1c115`](../../commit/da1c115) `tools(security): add untrack-secret.sh for fix-forward .env / secret removal`
- [`e27d418`](../../commit/e27d418) `fix(security): untrack .env + non-project files; tighten .gitignore`

### Added
- `src/signature_shape.rs` — DSPy-shaped signatures bridging LLM output → admission input.
- `src/swarm.rs` — 9-breed Rust+AtomVM cognition node fused via Hearsay-II blackboard.
- Chicago-TDD discipline: every human-interaction point covered by a real-Groq test.

### Security
- `.env` and other secret-bearing files untracked; `.gitignore` tightened; `tools/untrack-secret.sh` added for fix-forward removal.

## [0.4.0] — Phase 4 — Autonomic multi-target solution manufacturing

- [`eb0b8ca`](../../commit/eb0b8ca) `feat(manufacturing): Phase 4 — autonomic multi-target solution manufacturing`
- [`fe838f2`](../../commit/fe838f2) `fix(test): rebind capability_rollup to DEFECTS_TAXONOMY_VERSION constant`

### Added
- `src/manufacturing/` — `iac.rs` (Terraform), `rust_target.rs`, `erlang.rs`, `atomvm.rs`, `validators.rs`.
- Multi-target work-order routing with per-target validators and toolchain probes.
- `tests/manufacturing_validators.rs` (8 tests), `tests/adversarial_real_toolchains.rs` (5 real-toolchain tests).

## [0.3.0] — Phase 3 — RevOps test case (CTQ admission)

- [`d3cfdc1`](../../commit/d3cfdc1) `test(revops): Phase 3.5 + 3.6 + 3.7 — 9 station tests + counterfactual + Fortune-5 E2E`
- [`1e57bd4`](../../commit/1e57bd4) `test(revops): Phase 3.3 + 3.4 — 8 negative tests + 4 Groq boundary tests`
- [`233f99c`](../../commit/233f99c) `test(revops): Phase 3.1 + 3.2 — fake OCEL fixture + 5 CTQ admission tests`
- [`5f808d2`](../../commit/5f808d2) `test(requirements-andon): small-first E2E — gate of Phase 3`
- [`53f9713`](../../commit/53f9713) `feat(requirements-andon): ontology + advisory SHACL shapes (CLI surface deferred)`
- [`663d924`](../../commit/663d924) `feat(old-ai-stations): wire wasm4pm-cognition breeds via onto_old_ai_station`
- [`41c5696`](../../commit/41c5696) `feat(requirements-andon): wire 6 MCP handlers + no-bypass ratchet update`
- [`e20a8c4`](../../commit/e20a8c4) `test(secret-hygiene): canary leak scan + textual log/format ratchet`
- [`5f60633`](../../commit/5f60633) `feat(requirements-andon): Groq LLM boundary translator + [llm] config + .env loading`
- [`c33086f`](../../commit/c33086f) `feat(requirements-andon): add RequirementsManufacturing + Fortune5RevOpsGovernedRelease workflows`
- [`fb419f5`](../../commit/fb419f5) `feat(requirements-andon): extend AdmissionOp + DefectClass for CTQ-Forge layer`

### Added
- Requirements-andon ontology, advisory SHACL shapes, 6 MCP handlers.
- CTQ-Forge admission layer (`AdmissionOp`, `DefectClass` extensions).
- Groq LLM boundary translator with `[llm]` config block and `.env` loading.
- 9 RevOps station tests, 8 negative tests, 4 Groq boundary tests, Fortune-5 E2E.
- `tests/secret_hygiene.rs` canary leak scan.

> The product is CodeManufactory; RevOps is merely proof that CodeManufactory works.

## [0.2.0] — Phase 2 — Recursive admission (Level-5)

- [`346ce74`](../../commit/346ce74) `test(level-5): add portability_push, portability_codegen, capability_rollup; fix init() dead tuple`
- [`b70b2ca`](../../commit/b70b2ca) `feat(level-5): close no-bypass gate — gate 12 mutation handlers`
- [`a410e31`](../../commit/a410e31) `feat(level-5): receipt portability into TTL/codegen/push artifacts`
- [`554789a`](../../commit/554789a) `feat(level-5): replay-from-OCEL-alone + counterfactual binding`
- [`c51a29e`](../../commit/c51a29e) `feat(level-5): capability rollup + defect taxonomy versioning`
- [`ee90af9`](../../commit/ee90af9) `fix(no-stub): wire ingest/map/extend/push format params; add named-graph push; remove dead cfg`
- [`623a2e3`](../../commit/623a2e3) `feat(stream-5): wire onto_plan_workflow, onto_exemplar_seed, onto_counterfactual`
- [`2749500`](../../commit/2749500) `feat(admission): wire real precision into admission gate (p_min=0.7)`
- [`d224432`](../../commit/d224432) `fix(cell-ready): ocel_complete checks required ⊆ observed instead of non-empty`

### Added
- 12 mutation handlers gated by no-bypass audit; capability rollup; defect taxonomy versioning.
- Receipt portability into TTL / codegen / push artifacts.
- `replay-from-OCEL-alone` and counterfactual binding.
- Real precision (`p_min=0.7`) wired into the admission gate.

### Fixed
- `cell_ready.ocel_complete` now checks `required ⊆ observed` instead of merely non-empty.

## [0.1.0] — Phase 1 — OntoStar foundation (streams 1–5)

- [`1a587c7`](../../commit/1a587c7) `ontostar: end-to-end DoD test + build_ocel session-filter fix`
- [`8c3537a`](../../commit/8c3537a) `ontostar: fix three post-merge test issues`
- [`571a6c0`](../../commit/571a6c0) `ontostar: integration of streams 1-5 complete (option A: working-tree integration)`
- [`d0fe244`](../../commit/d0fe244) `ontostar(stream 5): no-op vs master per prior audit`
- [`e4db225`](../../commit/e4db225) `ontostar(R3): wire PowlBridgeReplay into admission gate, replace stub`
- [`33feda7`](../../commit/33feda7) `ontostar(R2): DefectClass::ThresholdFailed.metric is String (no-op)`
- [`f4f1f01`](../../commit/f4f1f01) `ontostar(R1): rewrite builtin catalog POWL strings to wasm4pm grammar`
- [`12f3d39`](../../commit/12f3d39) `ontostar(stream 3): admission gate integration tests`
- [`f11ffc0`](../../commit/f11ffc0) `ontostar(stream 4): feedback loop integration tests`
- [`7d52e35`](../../commit/7d52e35) `ontostar(stream 2): wasm4pm POWL replay integration tests`
- [`be04ae0`](../../commit/be04ae0) `ontostar(streams 1-4): integrated foundation for OntoStar manufacturing lifecycle`

### Added
- Streams 1–5 integrated: ontology layer, wasm4pm POWL replay bridge, admission gate, feedback loop.
- `PowlBridgeReplay` replaces the noop stub in the admission gate.
- Builtin catalog rewritten to wasm4pm grammar.
- End-to-end DoD test plus `build_ocel` session-filter fix.

---

## Pre-OntoStar — open-ontologies MCP server (0.1.x)

The `ontostar-integration` branch builds on top of the original `open-ontologies`
project, an AI-native MCP server for RDF/OWL ontology engineering with 50+
`onto_*` tools, an Oxigraph-backed triple store, SHACL validation, OWL-RL
reasoning, semantic embeddings, clinical crosswalks, and Terraform-style
lifecycle management.

- **0.1.13** — Compile cache + TTL eviction + tool-exposure filter; ontology
  repository directories; OpenAI-compatible embeddings provider; surfaced
  operational config.
- **0.1.12** — Virtualized tree view (Studio); 13-step deep builder
  (`/build`); IES-level ontology generation.
- **0.1.11 and earlier** — Initial 50-tool MCP surface, marketplace of 32
  standard ontologies, lineage trail, drift detection, alignment with
  self-calibrating confidence weights, doctor diagnostics, persistent store.

Detail for the pre-OntoStar releases is preserved in the project's git history
on `main`.
