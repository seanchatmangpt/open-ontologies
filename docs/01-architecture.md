# 01 — Architecture

## The layer stack

```
┌─────────────────────────────────────────────────────────────────┐
│                    External Verifier (onto verify)              │
│  Pure read-only walker; re-implementable in any language.       │
│  src/verify.rs · src/cmds/governance.rs                         │
├─────────────────────────────────────────────────────────────────┤
│                Multi-tenant isolation (Phase 11)                │
│  TenantContext + scope-token ACLs · src/tenant.rs               │
├─────────────────────────────────────────────────────────────────┤
│              Cell8 13-gate EARL attestation (Phase 10)          │
│  src/cell8.rs · ontology/cell8-conformance-shapes.ttl           │
├─────────────────────────────────────────────────────────────────┤
│             Receipt chain — atomic persist+emit (Phase 7)       │
│  Per-session sequence; BLAKE3 + Ed25519 · src/receipts.rs       │
├─────────────────────────────────────────────────────────────────┤
│       Swarm — 9 cognition breeds + Hearsay-II (Phase swarm)     │
│  src/swarm.rs · wasm4pm_cognition::breeds                       │
├─────────────────────────────────────────────────────────────────┤
│      Manufacturing — IaC + Rust + Erlang + AtomVM (Phase 4)     │
│  Deterministic generators · src/manufacturing/                  │
├─────────────────────────────────────────────────────────────────┤
│    LLM boundary — DSPy signature shapes + Groq (Phase 5/8)      │
│  src/signature_shape.rs · src/llm_translator.rs                 │
├─────────────────────────────────────────────────────────────────┤
│      OntoStar admission gate — cell_ready 13 conjuncts          │
│  src/admission.rs · src/cell_ready.rs · src/defects.rs          │
├─────────────────────────────────────────────────────────────────┤
│        wasm4pm bridge — POWL replay / conformance               │
│  src/powl_bridge.rs (PowlBridge, PowlBridgeReplay)              │
├─────────────────────────────────────────────────────────────────┤
│           Requirements Andon — CTQ Forge (Phase 3)              │
│  ontology/ requirements-andon · src/inputs.rs · workflows       │
├─────────────────────────────────────────────────────────────────┤
│            Oxigraph store + OCEL event log (foundation)         │
│  src/graph.rs · src/ocel_store.rs · src/state.rs                │
└─────────────────────────────────────────────────────────────────┘
```

## Per-layer prose

### Requirements Andon (CTQ Forge)

Stakeholder voice enters as `RequirementProposed`, must cite a `source_evidence_uri` (Defect: `RequirementWithoutSource`), then is translated by Groq into a structured `CandidateCtq`. The CTQ admission gate (`onto_admit_ctq`) demands an OCEL fixture, a quantitative threshold, and a counterfactual binding before promoting the candidate to a Critical-To-Quality fact. Defects: `CtqIncomplete`, `WorkOrderMissingCounterfactual`.

### OntoStar admission gate (`cell_ready` 13 conjuncts)

The single function `cell_ready` in `src/cell_ready.rs` certifies thirteen conjuncts in order, short-circuiting to the first failing typed `DefectClass`. Phase 10 expanded the original 8 conjuncts (Workflow / Scope / OCEL / Replay / Threshold / Stages / Bypass / Receipt) with five Cell8 gates: `ProvenanceChain`, `ExternalAttestation`, `TemporalValidity`, `DependencyClosure`, `ReplayProof`. No string errors. No `bail!`.

### wasm4pm bridge (POWL replay; conformance)

`PowlBridgeReplay` parses declared POWL strings via the `wasm4pm` crate, projects the OCEL trace tagged with `scope_token`, and returns a fitness/precision verdict. Production admission uses this; a `NoopPowlReplay` stub remains for gate-semantics unit tests that need a deterministic pass-through. Defects: `ReplayFailed`, `SkippedTask`, `ExtraTask`, `WrongOrder`, `CapabilityZero`, `ReplayDivergence`.

### Manufacturing (IaC + Rust + Erlang + AtomVM)

`src/manufacturing/mod.rs` takes a `SolutionSpec` (derived from an admitted work order) and deterministically emits a four-target bundle: Terraform JSON, a Rust crate, an Erlang/OTP supervision tree, and an AtomVM module. Every file carries an inline OntoStar receipt header (Terraform JSON uses `iac/.ontostar-receipt.json` sidecar — its schema is closed). Validators (`onto manufacturing validators`) compile each artifact under real `cargo check`, real `erlc`, real `terraform validate`. Defects: `GeneratorEmpty`, `IacInvalid`, `RustInvalid`, `ErlangInvalid`, `AtomVmInvalid`, `ManufacturingChainBroken`, `ArchitectureUnbound`.

### Swarm (9 cognition breeds + Hearsay-II)

`src/swarm.rs` manufactures nine AtomVM nodes — one per wasm4pm cognition breed (ELIZA, CBR, DENDRAL, STRIPS, Prolog, MYCIN, GPS, SOAR, Hearsay) — runs each against a shared `BreedInput`, then fuses the outputs through Hearsay-II as the consensus engine. Each node's manufactured artifacts pass real toolchain validators before any breed runs.

### Receipt chain (per-session sequence; atomic persist+emit)

Phase 7 added a per-session monotonic `sequence` column (`receipts_session_sequence_uniq` unique index) and wrapped persist+emit in a single SQLite transaction. Effect: ties on `granted_at` resolve deterministically; concurrent sessions cannot cross-contaminate; an emit failure rolls the persist back so no orphan receipt exists.

### Cell8 13-gate attestation + EARL emitter

`src/cell8.rs` emits hand-written Turtle EARL reports (one `earl:Assertion` per Cell8 gate A1–A13, all bound to a single receipt URN). Reports are byte-stable and SHACL-validatable against `ontology/cell8-conformance-shapes.ttl`.

### Multi-tenant isolation (Phase 11)

`TenantContext` (read from `OPEN_ONTOLOGIES_TENANT_ID`, defaults to `"default"`) is carried alongside `session_id`. The admission gate compares `TenantContext::current()` against the scope's `tenant_id` recorded in `declared_workflows`. Defect: `TenantBoundary`.

### External verifier

`onto verify <bundle>` strips inline-comment receipt headers (or sidecar JSON for IaC), recomputes BLAKE3 over the stripped body, walks `prior_receipt` back to the seed, and returns `is_valid: bool`. Pure read-only — no Oxigraph access required.

### LLM boundary (Groq via DSPy through pm4py)

`src/signature_shape.rs` defines DSPy-style signature shapes (input/output field semantics, demos, post-hoc validation). `src/llm_translator.rs` wraps Groq with bearer-auth secret-hygiene invariants. The pm4py POWL pattern was ported to five LLM boundaries (commits `1b7d6cc`, `619c3b1`) — every boundary has a real-Groq Chicago-TDD test.
