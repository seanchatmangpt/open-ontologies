# Case Studies and Practical Exercises
**Operationalizing the 7-Layer Autonomic Architecture**

This document outlines the core case studies and hands-on exercises integrated into the PhD curriculum. These scenarios force students to bridge academic theory with the `open-ontologies` codebase, navigating the tension between cryptographic integrity, bounded logic, and socio-technical reality.

---

## Case Study 1: The Civic Provision Network (L7, L3, L2)
**Context (SOC 910 & RTE 830):**
As automation displaces traditional labor markets (Acemoglu & Restrepo), local institutions like churches and community centers (Chaves & Tsitsos) are forced to scale their social service provision (food distribution, emergency shelter, volunteer coordination). However, they lack the administrative overhead to manage complex logistics or prove compliance to municipal funders.

**The Scenario:**
A coalition of local congregations has adopted the MCPP architecture. They need a system to coordinate volunteer dispatch, track physical inventory distribution, and generate cryptographically undeniable proof of service delivery for state audits. The system must operate on low-power edge devices (via WebAssembly) and gracefully handle network partitions.

### Exercise 1.1: POWL Route Design
*   **Task:** Define a Partially Ordered Workflow Language (POWL) model in `ontology/requirements.ttl` that governs the "Emergency Food Dispatch" route. The route must handle concurrent choice (e.g., dispatching multiple drivers) and loops (re-attempting failed deliveries).
*   **Deliverable:** A validated Turtle `.ttl` file and a successful parse through `src/powl_bridge.rs`.

### Exercise 1.2: OCEL 2.0 Telemetry
*   **Task:** Map the physical objects (`FoodBox`, `VolunteerVehicle`, `Recipient`) to the OCEL 2.0 meta-model. 
*   **Deliverable:** Modify `src/ocel_store.rs` to emit an `object_transition` event when a `FoodBox` changes custody from the congregation to the `VolunteerVehicle`, ensuring the event is appended to the JSONL log.

---

## Case Study 2: The Hallucinated Workflow (L3 vs. L4)
**Context (SAB 900 & CRY 840):**
An enterprise is using an autonomous LangChain agent to orchestrate complex data ETL pipelines. An attacker has successfully executed a prompt injection attack against the agent, forcing the LLM to output a maliciously compliant but structurally invalid workflow plan.

**The Scenario:**
The attacker has crafted a request that bypasses the agent's internal checks. They attempt to submit a mathematically impossible state transition to the `open-ontologies` backend, attaching a valid Ed25519 signature from a *previously captured*, legitimate request to bypass the cryptographic gate.

### Exercise 2.1: The Cryptographic Bypass
*   **Task:** Write a Rust test harness that deliberately skips `LlmInput::sanitize`. Inject a hallucinated JSON payload that dictates a workflow transition violating the configured POWL route.
*   **Deliverable:** A failing test demonstrating that the system panics or accepts the payload.

### Exercise 2.2: The Admission Patch
*   **Task:** Patch `src/admission.rs` and `src/cell_ready.rs`. Ensure that `verify_replay_hash` (A13) strictly validates the hash of the *current* structural payload against the signature, rather than just verifying the signature's mathematical validity in isolation.
*   **Deliverable:** A patched `cell_ready.rs` that emits a `DefectClass::SignatureExpiredKey` or `DefectClass::CapabilityZero` and safely denies admission.

---

## Case Study 3: The Infinite Remediation Trap (L5 & L1)
**Context (SAB 900 & LOG 820):**
To provide Autonomic Developer Experience (DX), the system returns structured `hint` fields detailing the exact CLI commands needed to unblock a failed state (e.g., `onto_declare_workflow`).

**The Scenario:**
A poorly configured AutoGPT agent receives a `ScopeUnclosed` defect. The system suggests `onto_declare_workflow`. The agent executes it, but due to a database lock or semantic contradiction in the OWL2-RL rules, the remediation command *also* fails with `ScopeUnclosed`. The agent enters an infinite loop, starving the server of resources.

### Exercise 3.1: Rigging the Trap
*   **Task:** Mutate a local SQLite `StateDb` to permanently lock the workflow scope table. Write a script simulating the AutoGPT agent that blindly follows the `hint` field in a tight loop.
*   **Deliverable:** A demonstration of the server experiencing resource starvation via infinite remediation loops.

### Exercise 3.2: The Health Guardian Backoff
*   **Task:** Implement a stateful quarantine circuit in `src/health_guardian.rs`. The guardian must track the frequency of remediation failures per tenant/agent. 
*   **Deliverable:** Code that forces a `auto_rollback` or hard termination (e.g., `DefectClass::SessionRevoke`) after 5 consecutive identical remediation failures, breaking the loop.

---

## Case Study 4: Autonomic Law Generation (L6 & L2)
**Context (KRN 850 & MIN 810):**
The `ggen sync` pipeline translates semantic laws into executable Rust and Erlang code. However, the system currently assumes the ontology is static. 

**The Scenario:**
Over 30 days, the `pm4py` batch analysis tool (`pm4py_drift`) has detected severe behavioral drift in how users are executing the "RevOps Manufacturing" pipeline. The formal ontology demands Route A, but empirical OCEL evidence shows 90% of users take Route B due to an unmodeled physical constraint.

### Exercise 4.1: Evidence-Driven Mutation
*   **Task:** Ingest a synthetic OCEL log demonstrating the drift. Trigger the `record_tool_feedback` mechanism.
*   **Deliverable:** A script that automatically reads the drift metrics and modifies the structural embeddings (`src/structembed.rs`) to decrease the semantic distance between the current state and Route B.

### Exercise 4.2: Generative Re-Compilation
*   **Task:** Based on the shifted embeddings, prompt the `onto_guide` to output a new `commands_aggregated.rq` configuration that legitimizes Route B. Run `ggen sync`.
*   **Deliverable:** An autonomously generated `src/cmds/generated_revops.rs` file that compiles successfully, proving that process evidence (L2) can autonomously rewrite the executing kernel (L6) without human intervention.