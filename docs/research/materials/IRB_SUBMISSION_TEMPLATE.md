# IRB Submission Form Template: Autonomic Civic Provision Networks
**SOC 910: Civic Deployment & Automation Displacement**

*This template must be submitted to the University Institutional Review Board (IRB) prior to initiating any edge-node deployment involving human subjects or community partner organizations (e.g., local congregations).*

## 1. Project Overview
*   **Project Title:** Autonomic Verification of Volunteer Labor via Object-Centric Edge Logging
*   **Principal Investigator:** [PhD Candidate Name]
*   **Faculty Advisor:** [Committee Chair]
*   **Dates of Proposed Research:** [Start Date] to [End Date]

## 2. Research Objectives
This study evaluates the socio-technical feasibility of deploying WebAssembly (`wasm4pm`) process mining kernels on low-power edge devices to coordinate and cryptographically verify emergency food dispatch in local congregations. The study aims to prove that autonomic receipt chains can reduce administrative burden in civic networks under automation stress.

## 3. Human Subjects & PII
*   **Participant Population:** 20-50 civilian volunteers acting as drivers and intake coordinators at [Partner Organization].
*   **Data Collected (PII):** 
    *   Volunteer names/IDs (pseudonymized at the edge).
    *   Geolocation data (only at designated delivery/intake nodes).
    *   Timestamps of object custody transitions (`FoodBox` handoffs).
*   **Data Minimization Strategy:** The `open-ontologies` OCEL 2.0 implementation (`src/ocel_store.rs`) has been modified to generate BLAKE3 hashes of volunteer IDs. Only zero-knowledge proofs and hashes leave the local edge node. No raw PII is transmitted to the central cloud.

## 4. Risks & Algorithmic Harm Mitigation
*   **Potential Risk:** Algorithmic bias in dispatch routing. If the Description Logic (DL) reasoner improperly constrains volunteer task allocation, certain demographics may be structurally excluded.
*   **Mitigation:** We have implemented a counterfactual constraint audit (L5) to prove that the routing laws are mathematically invariant to demographic metadata. The `HealthGuardian` actively monitors for starvation loops.

## 5. Informed Consent
Volunteers will be provided a plain-language summary of the cryptographic logging system. They will use a simplified mobile interface to sign Ed25519 "receipts" acknowledging custody transfer of physical goods. Consent forms emphasize that algorithmic telemetry is used strictly for operational verification, not surveillance.

---
*IRB Committee Use Only:*
[ ] Approved
[ ] Revisions Required
[ ] Denied