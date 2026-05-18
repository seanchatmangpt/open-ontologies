# Comprehensive Syllabi: PhD in Autonomic Socio-Technical Systems

This document contains the detailed, 14-week syllabi for the core coursework and practicums of the PhD program. Each course bridges formal academic literature from the 7-Layer Research Backbone with rigorous, hands-on engineering within the `open-ontologies` Rust ecosystem.

---

## SYS 801: Design Science Research & Systems Governance (L1)
**Course Description:**
This course establishes the epistemological foundation for the program. Students learn how to frame software artifacts as rigorous research contributions and manage their evolution using formal systems-engineering lifecycles. 

**Core Literature:**
*   Hevner et al. (2004). *Design Science in Information Systems Research*.
*   Peffers et al. (2007). *A Design Science Research Methodology*.
*   ISO/IEC/IEEE 15288:2023. *System life cycle processes*.
*   Yin, R. K. (2018). *Case Study Research and Applications*.

**Weekly Schedule:**
*   **Weeks 1-3: The Design Science Paradigm:** Introduction to DSR. Moving beyond "build and evaluate" to formal artifact-centric knowledge creation (Hevner).
*   **Weeks 4-6: Methodological Frameworks:** Operationalizing DSR (Peffers). Mapping research cycles to software release cycles.
*   **Weeks 7-9: Systems Engineering Lifecycle:** Applying ISO 15288. Requirements, architecture, verification, and validation processes in practice.
*   **Weeks 10-12: Technology Readiness & Case Studies:** Using NASA TRL to score artifact maturity. Designing analytic case studies for real-world deployments (Yin).
*   **Weeks 13-14: Final Project:** Draft a formal DSR proposal for a novel sub-component of the `open-ontologies` architecture, complete with evaluation criteria and a case-study design.

---

## MIN 810: Object-Centric Process Evidence (L2)
**Course Description:**
A deep dive into process mining, focusing on moving from flat event logs to rich, object-centric execution data. Students will map theoretical process evidence concepts to the system's OCEL 2.0 telemetry and storage.

**Core Literature:**
*   van der Aalst, W. M. P. (2022). *Process Mining: A 360 Degree Overview*.
*   van der Aalst, W. M. P. (2019). *Object-Centric Process Mining*.
*   OCEL 2.0 Standard.
*   Carmona et al. (2018). *Conformance Checking: Relating Processes and Models*.

**Weekly Schedule:**
*   **Weeks 1-3: Process Mining Foundations:** The 360-degree overview. Discovery, conformance, and enhancement.
*   **Weeks 4-6: Object-Centricity:** The divergence and convergence problem in traditional event logs. The OCEL 2.0 meta-model.
*   **Weeks 7-9: Conformance Checking:** Relating event streams to formal models. "Deny with evidence."
*   **Weeks 10-12: The `ocel_store.rs` Lab:** Hands-on lab analyzing the Rust implementation of OCEL 2.0 storage. Emitting object-centric events during state transitions.
*   **Weeks 13-14: Final Project:** Implement a new conformance checking heuristic or drift detection metric in `lineage.rs` that leverages OCEL relationships.

---

## LOG 820: Bounded Decision Engines (L5)
**Course Description:**
Explores the formal logic systems that dictate state admissibility. Students learn Datalog, SLG tabling, and Description Logic, bridging them to the codebase's fixpoint and DL reasoners.

**Core Literature:**
*   Abiteboul & Hull (1988). *Data Functions, Datalog and Negation*.
*   Chen & Warren (1996). *Tabled Evaluation with Delaying for General Logic Programs*.
*   Clarke et al. (2001). *Bounded Model Checking*.
*   Arias et al. (2018). *Constraint Answer Set Programming without Grounding*.

**Weekly Schedule:**
*   **Weeks 1-3: Datalog and Negation:** Foundations of database-oriented logic. Bounded reasoning principles.
*   **Weeks 4-6: Tabling and SLG Resolution:** Ensuring termination and efficient subgoal evaluation (Chen & Warren).
*   **Weeks 7-9: Bounded Model Checking & ASP:** Counterfactual search and constraint Answer Set Programming (s(CASP)).
*   **Weeks 10-12: The `reason.rs` and `tableaux.rs` Lab:** Auditing the OWL2-RL fixpoint rules and DL causal consistency checks. Understanding how semantic laws bound LLM outputs.
*   **Weeks 13-14: Final Project:** Author a complex, counterfactual constraint in `ontology/requirements.ttl` and trace its bounded evaluation through the DL reasoner.

---

## RTE 830: Route Law and Actor Topologies (L3)
**Course Description:**
Focuses on the orchestration and routing of actors. Moving beyond flat BPMN diagrams, this course covers Partially Ordered Workflow Languages (POWL) and supervised actor topologies.

**Core Literature:**
*   Kourani & van Zelst (2023). *POWL: Partially Ordered Workflow Language*.
*   Hewitt et al. (1973). *A Universal Modular ACTOR Formalism*.
*   Armstrong, J. (2003). *Reliable Distributed Systems in the Presence of Software Errors* (Erlang thesis).

**Weekly Schedule:**
*   **Weeks 1-3: The Actor Model:** Hewitt's formalism. Distributed, message-based orchestration.
*   **Weeks 4-6: Erlang Reliability & Supervision:** Fault isolation, supervision trees, and "let it crash" philosophy (Armstrong).
*   **Weeks 7-9: Route Law & POWL:** Extending partial orders with choice and loops. Moving from graphs to semantic execution (Kourani).
*   **Weeks 10-12: The `powl_bridge.rs` Lab:** Interfacing POWL semantics with Rust. Tracing workflow net execution within the architecture.
*   **Weeks 13-14: Final Project:** Design a robust, supervised actor topology for a new cognitive breed in `src/swarm.rs` that explicitly handles temporal faults.

---

## CRY 840: Admissibility and Proof-Carrying Code (L4)
**Course Description:**
Teaches the cryptographic foundations of the system. Students learn how decisions are packaged as independently checkable receipts, moving from generic logging to mathematical proof of admissibility.

**Core Literature:**
*   Necula, G. C. (1997). *Proof-Carrying Code*.
*   Appel, A. W. (2001). *Foundational Proof-Carrying Code*.
*   Laurie et al. (2013). *RFC 6962: Certificate Transparency*.
*   O’Connor et al. (2020). *The BLAKE3 paper*.

**Weekly Schedule:**
*   **Weeks 1-3: Proof-Carrying Code (PCC):** The theory of requiring checkable proofs before execution (Necula/Appel).
*   **Weeks 4-6: Hash Chains and Transparency:** Certificate transparency, Merkle trees, and append-only audit logs.
*   **Weeks 7-9: Modern Cryptographic Primitives:** The design and deployment of BLAKE3 and Ed25519 signatures.
*   **Weeks 10-12: The `receipt_chain.rs` Lab:** Analyzing the JSONL append-only log. Writing parsers to independently verify the chain's BLAKE3 continuity outside of the core server.
*   **Weeks 13-14: Final Project:** Extend the `VerifierWorker` to support distributed consensus checks on the receipt chain, emitting advanced `receipt_tampered` OCEL telemetry.

---

## KRN 850: High-Speed Kernels and Stream Processing (L6)
**Course Description:**
Bridges theoretical bounds with high-performance execution. Students focus on WebAssembly as a portable substrate and the demands of streaming process discovery.

**Core Literature:**
*   Haas et al. (2017). *Bringing the Web Up to Speed with WebAssembly*.
*   Burattin, A. (2022). *Streaming Process Mining*.
*   Wasm 2.0 Specifications.

**Weekly Schedule:**
*   **Weeks 1-3: WebAssembly Foundations:** The PLDI architecture. Portable, safe execution across edge and cloud.
*   **Weeks 4-6: Streaming Process Mining:** Real-time event stream processing vs. static log analysis (Burattin).
*   **Weeks 7-9: Multi-Target Manufacturing:** Generating artifacts for diverse runtimes (AtomVM, Erlang, Rust) using Tera templates and SPARQL queries.
*   **Weeks 10-12: The `wasm4pm` Lab:** Integrating and evaluating the stream-2 stub bindings. Profiling kernel execution speed.
*   **Weeks 13-14: Final Project:** Implement a new `ggen sync` target that compiles a specific POWL route directly into an optimized, standalone `.wasm` module.

---

## SAB 900: The Saboteur Labs (Combinatorial Intersections)
**Course Description:**
An advanced practicum where students actively attack the system. This course forces students to understand the thickest architectural braids by orchestrating cascading failures.

**Lab Execution Focus:** `src/cell_ready.rs` and `src/admission.rs`

**Weekly Schedule:**
*   **Weeks 1-4: The Temporal-Causal Paradox:** Students design exploits that pair valid cryptography with invalid Description Logic states, testing the order of operations in Gate A9 vs A11.
*   **Weeks 5-8: LLM Hallucination vs. Cryptographic Roots:** Students bypass `LlmInput` sanitization in harnesses to inject maliciously compliant JSON, testing the resilience of `verify_replay_hash` (A13).
*   **Weeks 9-12: The Infinite Remediation Trap:** Students rig the `DefectClass::remediation()` logic to force LangChain agents into infinite loops, forcing the implementation of quarantine circuits in the `HealthGuardian`.
*   **Weeks 13-14: Post-Mortem & Patching:** Students submit formal patches to the `open-ontologies` repository to seal the vulnerabilities exposed during the practicum.

---

## SOC 910: Civic Deployment & Automation Displacement (L7)
**Course Description:**
A socio-technical capstone that situates the architecture within the broader civilizational context. Students study the impact of automation on labor and the vital role of local, resilient provision networks.

**Core Literature:**
*   Acemoglu & Restrepo (2020). *Robots and Jobs: Evidence from U.S. Labor Markets*.
*   Chaves & Tsitsos (2001). *Congregations and Social Services*.
*   Klinenberg, E. (2018). *Palaces for the People* (Social Infrastructure).
*   Autor, D. H. (2015). *Why Are There Still So Many Jobs?*

**Weekly Schedule:**
*   **Weeks 1-3: The Automation Economy:** Labor displacement, task reconfiguration, and wage inequality (Acemoglu, Autor).
*   **Weeks 4-6: Civic Provision Networks:** The role of local institutions (churches, schools, community centers) as service providers (Chaves, Cnaan).
*   **Weeks 7-9: Social Infrastructure and Resilience:** Understanding how local networks activate under stress (Klinenberg, Abramson).
*   **Weeks 10-12: The MCPP Alignment Lab:** Mapping the requirements of a local congregational service network to the formal POWL and OCEL constructs in `open-ontologies`.
*   **Weeks 13-14: Final Capstone:** Present a formal architecture blueprint for deploying `wasm4pm` kernels to coordinate a local, high-trust civic distribution network, demonstrating how cryptographic receipts ensure accountability in volunteer labor.