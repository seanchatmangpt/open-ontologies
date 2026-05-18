# Chapter 2: Literature Review - From Petri Nets to Generative Ontologies

## 2.1 The Foundations of Process Science

The theoretical underpinning of the Ostar framework traces back to the formalization of concurrency in **Petri Nets** (Peterson, 1981). The ability to model discrete-event systems with mathematical precision provided the groundwork for the field of **Process Mining** (Van der Aalst, 2011). Historically, Process Mining has been a diagnostic discipline, focused on the *ex-post* analysis of event logs to discover, monitor, and improve real-world processes. 

The transition from flat, case-centric event logs (XES) to **Object-Centric Event Logs (OCEL)** (Van der Aalst et al., 2020) was a pivotal development. OCEL 2.0 allows for the modeling of many-to-many relationships between events and objects, a requirement for capturing the complexity of multi-agent cyber-physical systems. The Ostar methodology extends this lineage by moving from *descriptive* process mining to *prescriptive* behavioral enforcement.

## 2.2 The Rise and Risks of Generative Software Engineering

The emergence of Large Language Models (LLMs) for code synthesis, such as Codex (Chen et al., 2021), promised a revolution in developer productivity. However, research into the safety of LLM-generated code has highlighted significant risks. The "hallucination" problem—where models generate plausible but logically flawed code—presents a critical barrier to adoption in safety-critical domains. 

Existing mitigation strategies, such as unit testing and human-in-the-loop review, are reactive. They operate after the code has been generated. The Ostar hypothesis builds on the concept of **Correct-by-Construction** software (Hall & Chapman, 2002), arguing that the generative process itself must be constrained by formal semantic laws.

## 2.3 Semantic Web and Formal Verification

The use of **RDF (Resource Description Framework)** and **SHACL (Shapes Constraint Language)** for structural validation is well-established in the Semantic Web community. Formal ontologies provide a way to declare domain-specific invariants that can be checked for consistency. 

However, static structural validation is insufficient for enforcing behavioral compliance. Research into **Semantic Process Models** has attempted to bridge this gap, but few systems have successfully integrated these models into the critical path of an admission control gate. The Ostar framework’s use of the **POWL (Partially Ordered Workflow Language)** grammar (Van der Aalst, 2023) provides the necessary mathematical link between declarative ontologies and execution-time conformance.

## 2.4 State of the Art: Online Conformance Checking

Recent work in **Online Conformance Checking** has focused on detecting deviations in real-time as events enter a stream. These systems typically function as monitors, alerting administrators to violations. The Ostar pipeline’s innovation lies in its **Blocking Invariant**: a deviation does not merely trigger an alert; it prevents the state transition from occurring. This integration of alignment-based conformance into the admission gate represents the current frontier of Process Science.

---
*End of Chapter 2*
