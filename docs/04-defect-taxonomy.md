# 04 — DefectClass Taxonomy v4.1.0

`DEFECTS_TAXONOMY_VERSION = "ontostar-defects-4.1.0"` (commit `bea21b4`; bumped from `3.0.0` in Round-2 cascade Plan 1 — Ed25519 attestation introduced `AttestationInvalid`).

Phase 6 Task D Part 2 removed ten zero-emission speculative variants (`LawZero`, `MissingGatewayChoice`, `UnreachableTask`, `ShaclSkipped`, `ProjectionAsAuthority`, `StubGate`, `UnreplayableClaim`, `FalsePass`, `SecretLeak`, `GeneratedArtifactDirectEdit`). Every variant in the current taxonomy has at least one production emission site and a deny-path test.

The taxonomy is pinned by `DEFECTS_TAXONOMY_DISCRIMINANT_HASH` — a BLAKE3 over `tag1\0tag2\0...\0`. Changing the variant set is a breaking change and must bump the version.

## Variants

| Tag | Variant | Fires when | Triggered by | Deny-path test |
|-----|---------|------------|--------------|----------------|
| `capability_zero` | `CapabilityZero` | POWL replay yields fitness=0 / precision=0 | Empty trace; trace bears no required activities | `tests/admission_real_replay.rs` |
| `skipped_task` | `SkippedTask { task }` | Required activity absent from observed trace | Trace omits a `required_stages` entry | `tests/admission.rs::skipped_stage_denial` |
| `extra_task` | `ExtraTask { task }` | Trace contains an activity not in the declared POWL | Test fixture inserts unknown stage | `tests/powl_bridge_extras.rs` |
| `wrong_order` | `WrongOrder { expected, observed }` | Trace activities violate POWL ordering | Stages emitted in reversed sequence | `tests/admission.rs::wrong_order_denial` |
| `bypass_revoked` | `BypassRevoked` | A prior denial revoked the session's bypass capability | Operation attempted after a deny in the same session | `tests/admission.rs::bypass_revokes_subsequent_operations` |
| `receipt_missing` | `ReceiptMissing` | Predecessor receipt in the chain cannot be loaded | DB row deleted; sequence gap | `tests/cell_ready_deny_paths.rs` |
| `scope_unclosed` | `ScopeUnclosed` | Scope token has no closing event | OCEL projection still has an open scope | `tests/cell_ready_deny_paths.rs` |
| `ocel_incomplete` | `OcelIncomplete` | Required event types not all present | Test fixture omits a required event class | `tests/admission.rs::ocel_incomplete_denial` |
| `threshold_failed` | `ThresholdFailed { metric, observed, required }` | A quantitative threshold is below required level | Precision under p_min=0.7 | `tests/cell_ready_deny_paths.rs` |
| `replay_failed` | `ReplayFailed` | wasm4pm parser refused the POWL string | Malformed POWL grammar | `tests/admission_real_replay.rs::replay_enforcement_after_corruption` |
| `dead_parameter` | `DeadParameter { name }` | A handler accepts a parameter it never reads | Static check via dead-param-gate.sh | `tools/dead-param-gate.sh` |
| `requirement_without_source` | `RequirementWithoutSource` | Requirement proposed without `source_evidence_uri` | Empty URI in requirement record | `tests/revops_negative.rs` |
| `ctq_incomplete` | `CtqIncomplete { missing }` | CTQ candidate lacks fixture / threshold / counterfactual | Translation result missing one of the three | `tests/revops_ctq_admission.rs` |
| `work_order_missing_counterfactual` | `WorkOrderMissingCounterfactual` | Work order admitted without counterfactual binding | No counterfactual in record | `tests/revops_counterfactual.rs` |
| `llm_authority_claimed` | `LlmAuthorityClaimed` | An LLM output is recorded as authoritative (without admission) | Promotion attempt of `CandidateCtq` to fact directly | `tests/revops_negative.rs` |
| `raw_data_leak` | `RawDataLeak { field }` | Raw stakeholder text leaks into a structured field | OCEL attribute carries verbatim voice | `tests/revops_negative.rs` |
| `generator_empty` | `GeneratorEmpty { target }` | A manufacturing target produced zero files | Forced via `manufacture_with_override` | `tests/manufacturing_validators.rs` |
| `iac_invalid` | `IacInvalid { reason }` | `terraform validate` rejects emitted JSON | Generator bug / schema drift | `tests/manufacturing_validators.rs` (4 sub-tests) |
| `rust_invalid` | `RustInvalid { reason }` | `cargo check` fails on emitted crate | Generator bug | `tests/manufacturing_validators.rs` (3 sub-tests) |
| `erlang_invalid` | `ErlangInvalid { reason }` | `erlc` rejects emitted module | Generator bug | `tests/manufacturing_validators.rs` |
| `atomvm_invalid` | `AtomVmInvalid { reason }` | AtomVM bytecode build fails | Generator bug | `tests/manufacturing_validators.rs` |
| `manufacturing_chain_broken` | `ManufacturingChainBroken { stage }` | Cross-target reference cannot be resolved | Erlang refers to Rust symbol that wasn't generated | `tests/manufacturing_validators.rs` (2 sub-tests) |
| `architecture_unbound` | `ArchitectureUnbound` | SolutionSpec lacks bound architecture template | Spec built without architecture | `tests/solution_manufacturing_e2e.rs` |
| `tenant_boundary` | `TenantBoundary { caller_tenant, scope_tenant }` | Caller's `TenantContext` ≠ scope's owning tenant | Cross-tenant scope access attempt | `tests/multi_tenant_isolation.rs` |
| `provenance_missing` | `ProvenanceMissing { stage }` | Cell8 A9 `ProvenanceChain` conjunct fails | Receipt not bound to upstream evidence | `tests/cell8_thirteen_gates.rs` |
| `attestation_missing` | `AttestationMissing` | Cell8 A10 `ExternalAttestation` conjunct fails | Stub digest mismatch | `tests/cell8_thirteen_gates.rs` |
| `attestation_invalid` | `AttestationInvalid { reason }` | Cell8 A10 verifier rejects the Ed25519 signature | `signature_invalid` / `unknown_signing_key:<fpr>` / `missing_signing_key_fpr` / `no_trust_set` / `no_signer_configured` | `tests/ed25519_attestation.rs` |
| `temporal_skew` | `TemporalSkew { skew_ms }` | Cell8 A11 `TemporalValidity` conjunct fails | Receipt timestamp earlier than predecessor | `tests/cell8_thirteen_gates.rs` |
| `dependency_closure_broken` | `DependencyClosureBroken { missing }` | Cell8 A12 `DependencyClosure` conjunct fails | Cited dependency receipt absent | `tests/cell8_thirteen_gates.rs` |
| `replay_divergence` | `ReplayDivergence { expected, observed }` | Cell8 A13 `ReplayProof` conjunct fails | Replay produces different artifact bytes | `tests/cell8_thirteen_gates.rs` |

## Pin test

`tests/lib_taxonomy_pin.rs::taxonomy_discriminant_hash_pinned` recomputes the BLAKE3 over `all_tags()` and compares against the pinned constant. Adding/removing/renaming any variant breaks this test until both the version constant and the hash constant are updated together.
