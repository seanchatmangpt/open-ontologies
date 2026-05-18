# Combinatorial Maximalist Execution Plan

This document details the exact research scenarios, test vectors, and theoretical bounds we will evaluate to execute the combinatorial maximalist strategy across the 7-layer framework. 

## Phase 2: The "Saboteur" Intersection Analysis
**Target:** `src/cell_ready.rs` and `src/admission.rs`
**Goal:** Prove that the system gracefully handles cascading failures where multiple theoretical layers are attacked simultaneously.

### Scenario 2.1: The Temporal-Causal Paradox (L4 vs. L5)
*   **Attack Vector:** Submit a request that is perfectly signed and cryptographically intact (L4 passes), but the requested state transition violates the Description Logic causal consistency bounds in `tableaux.rs` (L5), while simultaneously spoofing the `granted_at` timestamp to exploit temporal race conditions (violating A11).
*   **Execution:** 
    *   Create a mock `CellReadyInputs` with a manipulated `prior_tenant_receipt_count` and a crafted `LlmInput` payload.
    *   Inject a DL contradiction into the `check_causal_consistency` pipeline.
    *   **Success Criterion:** The `cell_ready` gate must reject the transition with a compound `DefectClass` that accurately prioritizes the L4 temporal violation over the L5 logic violation (or vice versa, depending on the established strict-evaluation order). It must *not* panic or silently drop either evidence trail.

### Scenario 2.2: LLM Hallucination vs. Cryptographic Roots (L3 vs. L4)
*   **Attack Vector:** Force the generative engine to output a syntactically valid POWL route (L3) that dictates an impossible workflow step. Attempt to bypass the admission gate by providing a valid BLAKE3 hash of a *different*, legitimate route (L4).
*   **Execution:**
    *   Bypass `LlmInput::sanitize` maliciously in a test harness to feed raw hallucinated JSON into `powl_bridge.rs`.
    *   Attach an Ed25519 signature from an authorized key that corresponds to a different payload.
    *   **Success Criterion:** The system must emit a `receipt_tampered` OCEL event (L2) and reject the admission at the `verify_replay_hash` (A13) boundary, proving that L3 structural validity cannot override L4 cryptographic mismatch.

---

## Phase 3: Autonomic Feedback Loop Amplification
**Target:** `src/defects.rs`, `src/health_guardian.rs`, and pm4py integration.
**Goal:** Push the self-healing and remediation pipelines into infinite loops or race conditions to verify their bounded termination logic.

### Scenario 3.1: The Infinite Remediation Trap
*   **Attack Vector:** Trigger a defect (e.g., `ScopeUnclosed`) that suggests a remediation command (e.g., `onto_declare_workflow`). Rig the system so that executing the remediation command *re-triggers* the same defect, creating a theoretical infinite loop for an autonomous agent.
*   **Execution:**
    *   Configure a LangChain/AutoGPT mock agent that automatically executes the `hint` provided in the JSON error response.
    *   Mutate the database state so that `onto_declare_workflow` consistently fails with `ScopeUnclosed`.
    *   **Success Criterion:** The `HealthGuardian` or the `DefectClass::remediation` logic must feature a backoff, quarantine, or `auto_rollback` state that forces a hard termination after $N$ cycles, preventing execution starvation.

### Scenario 3.2: Recursive Conformance Drift
*   **Attack Vector:** Inject synthetic OCEL logs that simulate severe process drift over a 24-hour period.
*   **Execution:**
    *   Run `pm4py_drift` via the MCP tool to detect the drift.
    *   Assert that the system triggers `record_tool_feedback` automatically.
    *   **Success Criterion:** The feedback must correctly mutate the `structembed.rs` (L5) vectors, fundamentally altering the next LLM-generated plan via `onto_guide` (L3). The test must prove that L2 evidence directly closes the loop to alter L5 reasoning.

---

## Phase 4: Extreme Generative Manufacturing
**Target:** `ostar-governor`, `ostar-doctor`, `ggen sync`, and `.specify/` templates.
**Goal:** Stress-test the combinatorial explosion of the `ggen` pipeline and the semantic route laws.

### Scenario 4.1: Contradictory Semantic Laws
*   **Attack Vector:** Inject fundamentally opposed semantic rules into `ontology/revops-manufacturing.ttl`. For example, Rule A mandates that State $X$ must transition to State $Y$. Rule B mandates that State $X$ must *never* transition to State $Y$.
*   **Execution:**
    *   Run the `ostar-governor` to parse the rules.
    *   Run `ggen sync` to aggregate the commands.
    *   Run `ostar-doctor` to verify the generated code.
    *   **Success Criterion:** The pipeline must *fail to compile* the generated Rust/Erlang code, or `ostar-doctor` must throw a formal refutation during the SPARQL extraction phase. The contradiction must not make it into the executable L6 kernels.

### Scenario 4.2: Maximum Target Complexity
*   **Attack Vector:** Expand the ontology to define 10,000 distinct CLI commands and workflow routes.
*   **Execution:**
    *   Run `ggen sync` targeting the `commands_aggregated.rq` query.
    *   **Success Criterion:** Measure the performance of the single-row `GROUP_CONCAT` Tera template rendering. The system must render the 10,000-route artifact in under 5 seconds, proving the scalability of the L3 route-law generation pipeline.