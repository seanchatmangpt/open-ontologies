# Chapter 4 — Implementation

## 4.1 Overview

The portfolio system operates as a closed-loop manufacturing ecosystem where operational law, plant execution, and conformance oracles are tightly coupled through cryptographic and process-mining evidence. The implementation is partitioned into four primary cells: *speckit-ralph* functions as the portfolio orchestrator and autonomous implementation loop [OVERVIEW.md]; *mcpp* acts as the admissible-work plant, executing route-based transitions and emitting telemetry [OVERVIEW.md]; *wasm4pm* serves as the conformance oracle, replaying event logs against process models to calculate fitness and precision [OVERVIEW.md]; and *open-ontologies* provides the manufacturing substrate, housing the typestate ontologies and SPARQL-Tera pipelines that generate service runtimes [OVERVIEW.md]. This chapter details the technical realization of these cells and their integration into a unified, receipt-bound admission system.

## 4.2 Receipt Chain

The system achieves operational immutability through an append-only receipt chain, anchored by BLAKE3 cryptographic hashes and Ed25519 signatures [memory:project_adversary_test_suite]. Each receipt entry, recorded in `chain.jsonl` (or repository-specific equivalents like `.ggen/receipts/`), contains a `tick_id`, a link to the previous hash, and the state-transition evidence [memory:project_adversary_test_suite]. This structure satisfies the requirement for a non-tautological audit trail: an agent cannot claim a task is complete without an independent, disk-verified cryptographic proof packet [memory:project_hooks_proof_gate].

The validation logic, enforced by the `wpm proof audit` tool, rejects any chain where the `CHAIN_HEAD` external anchor mismatches or where event-level hash mismatches occur, preventing unauthorized modifications to the history [memory:project_adversary_test_suite]. The integration of these chains across the portfolio allows for transitively verifiable cross-cell consequences, where an action admitted in *mcpp* can be traced back to its specific ontology law in *open-ontologies* and its conformance replay in *wasm4pm* [memory:project_mcpp_weaver_architecture].

## 4.3 Object-Centric Event Log

Telemetry emission follows the OCEL 2.0 standard, treating process execution as a continuous operational memory rather than a post-hoc analytical export [memory:project_mcpp_weaver_architecture]. Every state transition within the *mcpp* plant triggers an OCEL event, which is schema-enforced at the point of writing [memory:project_mcpp_weaver_architecture]. This log is not batched but tail-replayed, allowing the system to maintain a cursor on its own execution history [memory:project_mcpp_weaver_architecture]. The event structure includes required attributes—`mcpp.ocel.event_id`, `mcpp.route.id`, and `mcpp.status`—which allow the *wasm4pm* oracle to perform object-centric replay against partially ordered workflow models [memory:project_mcpp_weaver_architecture]. The integrity of these logs is preserved via the same receipt-chain mechanism, ensuring that an emitted OCEL event is always bound to the authority that admitted the underlying action [memory:project_mcpp_weaver_architecture].

## 4.4 Plant — mcpp

The *mcpp* repository functions as the admissible-work plant. It utilizes POWL v2 (Partially Ordered Workflow Language) to define route-based logic, allowing for non-sequential but deterministically constrained execution [memory:project_mcpp_route_success]. The plant is engineered to support multiple heterogeneous runtimes, including Erlang, AtomVM, and WebAssembly (WASM), using *mcpp* as the central MCP+ manufacturing orchestrator [memory:project_mcpp_wasm4pm_boundary].

Execution within the plant is governed by canonical admission gates, such as those verified during Wave 50 [commit:756741d]. These gates interpret outputs from the *wasm4pm* oracle; for instance, a route conformance failure (where fitness falls below the required threshold) results in a `RouteConformanceFailed` refusal, emitting the appropriate OCEL error telemetry and blocking the issuance of subsequent receipts [memory:project_mcpp_wasm4pm_boundary].

## 4.5 Oracle — wasm4pm

The *wasm4pm* oracle is the core process-mining and benchmarking engine, designed as an algorithmic gauge for the manufacturing plant [memory:project_mcpp_wasm4pm_boundary]. Its primary function is the token-based replay of OCEL logs against POWL models to compute fitness, precision, and predictive perspectives [memory:project_algo_validation_goal]. 

The engine implements several Tier-1 mining and conformance algorithms—including DFG, heuristic miner, and alpha++—which are rigorously validated against real-world XES and OCEL datasets [memory:project_algo_validation_goal]. As of May 18, 2026, the oracle maintains 144 real-data algorithm validation tests, ensuring that conformance results are not based on synthetic toys but on datasets derived from pm4py ground truth [memory:project_algo_validation_goal]. The oracle's output is strict; precision gates are enforced identically to fitness gates, requiring exactly 1.0 for admission, thereby preventing "informational" conformance gaps [memory:project_precision_gating].

## 4.6 Typestate Layer — open-ontologies

*open-ontologies* acts as the manufacturing substrate, where the ontology serves as the ultimate source of operational law [memory:feedback_manufacturing_doctrine]. It employs a structured pipeline—TTL ontologies $\rightarrow$ SPARQL extraction $\rightarrow$ Tera manufacturing—to generate runtimes, database migrations, and validation logic [memory:project_autonomics_architecture]. 

The system enforces a strict public-vocabulary alignment; ZOE LA specific ontologies exist only as local specialization profiles, specializing public anchors (e.g., `schema:Action`, `prov:Activity`) to inherit their interoperability and persistence contracts [memory:feedback_public_vocab_materialization]. Namespace singularity is enforced via CI-wired gates (e.g., `tools/validate-namespace-singularity.sh`), which prevent the emergence of the banned legacy URN form (a `urn:` scheme prefixed with the local domain shorthand) or the banned shortened HTTPS variant, ensuring the entire manufacturing graph remains validatable [memory:feedback_namespace_singularity].

## 4.7 Portfolio OS — speckit-ralph

*speckit-ralph* provides the portfolio orchestrator, managing the lifecycle of the other three repositories through a structured tasking system (`tasks.md`) and autonomous implementation loops [speckit-ralph/GEMINI.md]. It acts as the "doorkeeper," initiating work via PR-Ralph, which manages the granular logic and implementation obligations [speckit-ralph/GEMINI.md]. This repository does not implement process logic itself but instead manages the manufacturing manifests, ontologies, and ggen templates required by *open-ontologies* to scaffold new capabilities [speckit-ralph/GEMINI.md]. It serves as the single source of truth for repository state, as evidenced by the convergence reports and the registry of portfolio cells [speckit-ralph/OVERVIEW.md].

## 4.8 Cross-Cell Integration

Boundary coupling between cells is strictly managed via CLI and component interfaces to prevent the dilution of autonomic law [memory:project_mcpp_wasm4pm_boundary]. The integration between *mcpp* and *wasm4pm*, for example, is constrained to path dependencies and command-line execution, explicitly forbidding the merger of the two repositories [memory:project_mcpp_wasm4pm_boundary]. 

When *mcpp* route logic requires conformance verification, it invokes the *wasm4pm* oracle as a capability, receives a structured conformance JSON envelope, and delegates the refusal classification to the *mcpp* doctor [memory:project_mcpp_wasm4pm_boundary]. This boundary preservation ensures that *wasm4pm* remains a gauge, while *mcpp* remains the plant, with the *open-ontologies* layer providing the unifying typestate that all cells must respect [memory:project_mcpp_weaver_architecture].

## 4.9 Threats to Implementation Validity

The implementation faces three primary validity threats:

1. **Semantic Drift:** The persistent risk of namespace drift in *open-ontologies*, where hand-authored artifacts begin to deviate from ontology-driven laws, necessitating continuous maintenance of the namespace singularity gates [memory:feedback_namespace_singularity].
2. **Oracle Contention:** As the number of autonomic routes increases, the computational overhead of continuous conformance replay in *wasm4pm* may create performance bottlenecks during the benchmark-gate enforcement [memory:project_wasm4pm_workspace_issue].
3. **Ghost-Implementation:** The persistent temptation to instantiate "fake" implementations (stubs or mocks) that bypass admission-gate telemetry. The current adversarial defense, centered on `wpm proof audit`, must be continually hardened to detect structural gaps that appear "accepted" by the tooling but lack actual process-mining evidence [evidence gap].
