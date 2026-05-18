# PhD Program in Autonomic Socio-Technical Systems
**Curriculum & Research Progression for the MCP+, wasm4pm, Prolog8, and MCPP Ecosystem**

## Program Overview
This doctoral program is designed to produce scholar-engineers capable of designing, verifying, and deploying combinatorial maximalist systems. Graduates will master the intersection of formal logic, cryptographic admissibility, process mining, and socio-technical deployment. The curriculum is directly mapped to the 7-layer theoretical backbone of the `open-ontologies` ecosystem.

---

## Year 1: Foundations of Evidence and Logic
*The first year establishes the core theoretical competencies: how we govern research, how we extract evidence from reality, and how we apply bounded logic to that evidence.*

### Core Coursework
*   **SYS 801: Design Science Research & Systems Governance (L1)**
    *   *Focus:* Hevner's DSR methodology, ISO/IEC/IEEE 15288 life-cycle control, and NASA TRL scoring.
    *   *Outcome:* Students must frame a software artifact as a valid epistemological research contribution.
*   **MIN 810: Object-Centric Process Evidence (L2)**
    *   *Focus:* The van der Aalst canon, OCEL 2.0 standards, event log ingestion, and process discovery.
    *   *Lab:* Building ingestion pipelines into `src/ocel_store.rs` and `src/lineage.rs`.
*   **LOG 820: Bounded Decision Engines (L5)**
    *   *Focus:* Datalog/negation, SLG tabling, ASP, OWL2-RL fixpoint rules, and Description Logic (DL).
    *   *Lab:* Implementing causal consistency checks within `src/tableaux.rs` and `src/reason.rs`.

---

## Year 2: Architecture, Law, and Cryptographic Integrity
*The second year shifts from theory to architectural enforcement. Students learn how to constrain execution, prove state transitions, and compile logic into portable high-speed kernels.*

### Core Coursework
*   **RTE 830: Route Law and Actor Topologies (L3)**
    *   *Focus:* Partially Ordered Workflow Language (POWL), workflow nets, supervised execution, and the Actor model.
    *   *Lab:* Extending `src/powl_bridge.rs` and managing workflow scope alphabets.
*   **CRY 840: Admissibility and Proof-Carrying Code (L4)**
    *   *Focus:* Necula/Appel's PCC, append-only logs, Certificate Transparency, BLAKE3 hashing, and Ed25519 attestation.
    *   *Lab:* Auditing the `src/receipt_chain.rs` JSONL continuity and building zero-knowledge verification in the `VerifierWorker`.
*   **KRN 850: High-Speed Kernels and Stream Processing (L6)**
    *   *Focus:* WebAssembly (Wasm) architecture, streaming process discovery, and multi-target manufacturing (AtomVM/Erlang/Rust).
    *   *Lab:* Executing the `wasm4pm` stream-2 stub bindings via the `src/swarm.rs` cognition swarm.

---

## Year 3: The Thickest Braid & Socio-Technical Deployment
*The third year is a crucible. Students engage in "Combinatorial Maximalist" practicums, intentionally breaking the system to understand where the theoretical layers intersect, before studying the societal impact of what they are building.*

### Practicums & Seminars
*   **SAB 900: The Saboteur Labs (Combinatorial Intersections)**
    *   *Focus:* Attacking the absolute epicenter of the architecture: `src/cell_ready.rs`.
    *   *Exercise 1:* The Temporal-Causal Paradox (Spoofing L4 timestamps while violating L5 DL constraints).
    *   *Exercise 2:* LLM Hallucination vs. Cryptographic Roots (Bypassing L3 sanitization against L4 receipt verification in `src/admission.rs`).
    *   *Outcome:* Students must author a patch that strengthens a failure mode discovered during adversarial testing.
*   **SOC 910: Civic Deployment & Automation Displacement (L7)**
    *   *Focus:* Acemoglu & Restrepo on labor displacement, Chaves & Tsitsos on congregational/local provision networks, and civic resilience.
    *   *Seminar:* Framing `MCPP` not just as enterprise software, but as critical civic infrastructure capable of absorbing automation shocks.

---

## Year 4: Original Research & Dissertation
*The final year is dedicated to advancing the state of the art in autonomic generative pipelines.*

### Dissertation Milestones
1.  **The Braid Proposal:** Propose an original architectural enhancement that spans at least three of the seven layers (e.g., A novel Wasm-compiled DL reasoner (L5+L6) whose outputs are inherently proof-carrying (L4)).
2.  **The Autonomic Feedback Defense:** Demonstrate empirical proof of a closed-loop autonomic feedback cycle (e.g., using L2 PM drift detection to autonomously rewrite L3 route laws without human intervention).
3.  **Final Defense:** A public defense of the engineered artifact, evaluated against both formal academic criteria (DSR) and strict adversarial load-bearing tests (make adversarial).

---

## Technical Prerequisites for Admission
*   Fluency in Rust (trait bounds, lifecycles, macros).
*   Familiarity with SPARQL, RDF, and semantic web technologies.
*   Understanding of cryptographic hash functions and digital signatures.
*   A "Combinatorial Maximalist" mindset: the belief that the system is only as strong as its most complex, overlapping edge case.