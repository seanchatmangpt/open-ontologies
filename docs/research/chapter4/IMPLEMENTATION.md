# Chapter 4: Implementation - The Ostar Generative Pipeline and Typestate Enforcement

## 4.1 Tier 1: Semantic Modeling in RDF/Turtle

The Ostar pipeline begins with the formalization of the system's operational domain in `ontology/cli-open-ontologies.ttl`. We utilize the Resource Description Framework (RDF) to model commands, subcommands, and constraints as a graph.

### 4.1.1 Class Hierarchy and Object Properties
The core CLI vocabulary defines `cli:Command` as a fundamental class. Subcommands are modeled via the `cli:hasSubcommand` object property. This allows for the recursive definition of complex command structures (e.g., `onto -> workflow -> discover`). By using Turtle as the serialization format, the ontology remains both human-readable and machine-processable, serving as the immutable blueprint for all downstream artifacts.

## 4.2 Tier 2: Deterministic Manufacturing via `ggen`

The transition from a declarative graph to executable code is managed by the `ggen` synchronization engine. 

### 4.2.1 SPARQL-Driven Context Extraction
To eliminate the non-determinism inherent in probabilistic code generation, `ggen` utilizes SPARQL queries (e.g., `.specify/queries/cli/commands_aggregated.rq`) to extract a deterministic context from the Tier 1 ontology. The `GROUP_CONCAT` operator is employed to collapse multi-row graph results into single-row aggregates, ensuring that the rendering engine receives a stable, flattened view of the command hierarchy.

### 4.2.2 Template Rendering and Formal Closure
The extracted context is passed to Tera templates (`.specify/templates/cli/cmds.rs.tera`). This rendering process is purely functional: given the same ontology and query, it will produce the same Rust source code. The project achieved **Formal Closure** in R6-WC3 by permanently deleting all imperative fallbacks (e.g., `manufacture_cli.py`), establishing that code can only exist if it is explicitly derived from the ontology.

## 4.3 Tier 3: Rust Typestate Enforcement

The generated artifacts are implemented in Rust to leverage its strong type system and ownership guarantees.

### 4.3.1 Mapping Ontological States to Rust Enums
Each command and state transition defined in the ontology is mapped to a Rust `enum` (e.g., `AdmissionOp`). This ensures that only valid, ontology-defined operations can be instantiated. The `ProductionRecord` serves as a cryptographically sealed container for these operations, preventing any modification after the admission gate has granted approval.

### 4.3.2 Telemetry and Witnessing
In `src/telemetry.rs`, we initialize a tracing subscriber that captures the evaluation of the 13 Cell8 gates. Every admission decision is logged with its associated OTLP spans, creating a tamper-evident chain of custody that bridges the gap between the Rust runtime and the OCEL audit log.

## 4.4 The Studio: A Multi-Modal Engineering Interface

The Open Ontologies Studio (`studio/`) provides a visual environment for manipulating the Ostar pipeline. Built using Tauri and React, it utilizes a process sidecar model to execute the core engine in a local, sandboxed environment. The studio's `GraphCanvas` renders the ontology as an interactive 3D force-directed graph, allowing engineers to visualize the structural impact of their semantic modifications before they are committed to the manufacturing pipeline.

---
*End of Chapter 4*
