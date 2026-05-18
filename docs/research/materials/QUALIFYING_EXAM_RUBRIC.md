# Qualifying Examination Rubric (Year 2)
**Doctoral Program in Autonomic Socio-Technical Systems**

The Qualifying Examination (Comps) occurs at the end of Year 2. It assesses the candidate's ability to synthesize the 7 theoretical layers and mathematically reason about the architecture before beginning original dissertation research.

## Component 1: Written Examination (72 Hours)
The candidate is provided a sealed, air-gapped system running a compromised fork of `open-ontologies`.

*   **Task A (Process Evidence & Route Law):** The system is suffering from severe process drift. The candidate must write a streaming process mining heuristic to isolate the divergent object-centric (OCEL) patterns and propose a POWL structural revision.
*   **Task B (Admissibility & Logic):** The candidate is given a set of Description Logic (DL) statements that contain a hidden tautology loop. They must mathematically prove why the current `tableaux.rs` implementation will fail to terminate, and write the necessary bounds to fix the engine.

## Component 2: Oral Defense (2 Hours)
The candidate defends their written solutions before the Interdisciplinary Faculty Committee.

**Evaluation Matrix:**

| Area | Fail (Remediation Req.) | Pass | Pass with Distinction |
| :--- | :--- | :--- | :--- |
| **Layer Interlocking** | Treats cryptography and DL logic as isolated systems. | Correctly identifies how L4 signatures depend on L5 bounds. | Articulates a zero-knowledge proof mechanism that bridges L4 and L5 natively. |
| **Socio-Technical Awareness** | Evaluates the system purely on speed and uptime. | Acknowledges how automated bounds impact civilian users. | Proposes a civic deployment architecture that mitigates algorithmic harm using structural embeddings. |
| **Combinatorial Maximalism** | Only tests single-variable failure modes. | Tests multi-variable edge cases (e.g., latency + LLM hallucination). | Constructs an exploit that leverages 3+ theoretical layers simultaneously, then mathematically seals it. |

*Candidates who pass this examination are elevated to PhD Candidate status (ABD) and proceed to SAB 900 (The Saboteur Labs).*