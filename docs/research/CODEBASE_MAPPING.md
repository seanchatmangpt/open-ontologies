# Combinatorial Maximalist Codebase Mapping

Based on the 7-layer framework defined in `RESEARCH_BACKBONE.md`, the `open-ontologies` codebase manifests these theoretical layers in the following concrete modules and combinatorial intersections.

## Layer Mapping

*   **L2 Process Evidence:** Anchored in `src/ocel_store.rs` (OCEL 2.0 storage and event emission) and `src/lineage.rs` (RDF-to-OCEL transformation).
*   **L3 Route Law:** Implemented in `src/powl_bridge.rs` (POWL semantics) and the `src/workflows/` directory (alphabet and scope management).
*   **L4 Admissibility & Integrity:** Centralized in `src/admission.rs` (the gatekeeper), `src/receipt_chain.rs` (the append-only JSONL log), and `src/attestation.rs` (Ed25519 verification).
*   **L5 Bounded Decision Engine:** Powered by `src/reason.rs` (OWL2-RL fixpoint) and `src/tableaux.rs` (DL reasoning), driving bounded checks.
*   **L6 High-Speed Kernels:** Represented by the `wasm4pm` stream-2 stub bindings (POWL and cognition kernels) and the `src/manufacturing/` pipeline that targets AtomVM, Erlang, and Rust.

## Combinatorial Intersections

The "combinatorial maximalist" perspective requires understanding where these layers braid together to enforce the system's global properties.

### 1. The "Absolute Thickest Braid": `src/cell_ready.rs`
This module is the single most critical intersection in the entire architecture. The `cell_ready` function enforces the "Cell8" suite—13 interdependent gates (A1-A13) that must *all* pass for a mutation to be admissible. 
*   **L2 (Evidence):** It consumes the prior state and emits admissibility verdicts.
*   **L3 (Route Law):** It verifies workflow constraints and POWL conformance (`replay_against_powl`).
*   **L4 (Integrity):** It asserts the cryptographic continuity of the receipt chain (`re_read_granted_at_chain`).
*   **L5 (Decision Engine):** It delegates to `tableaux.rs` to ensure Description Logic causal consistency (`check_causal_consistency`).

### 2. The Admission Pipeline: `src/admission.rs`
This module wires L2 evidence collection, L3 POWL replay (via L6 kernels), and L5 reasoning to generate L4 receipts. It is the architectural bridge where theoretical route limits meet practical execution.

### 3. The Cognition Swarm: `src/swarm.rs`
Intersects L6 (portable kernels) with L3/L5 (breed-specific logic), utilizing a deterministic manufacturing pipeline to ensure kernel integrity across multiple target platforms.

### 4. MCPP Gating: `src/mcpp_gate.rs`
Bridges L4 (Proof-Carrying Code) and L2 (OCEL evidence) by wrapping MCP tool calls in a proof-generation middleware (`ProofGatedServer`) that verifies evidence before signing a receipt.