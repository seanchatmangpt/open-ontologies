# AutoReceiptPipeline Operational Runbook

_Generated from `ontology/zoela/autoreceipt.ttl` via `ggen` template pattern._

This runbook provides operational guidance for the **AutoReceiptPipeline**, the core mechanism for verifying Jobs To Be Done (JTBD) across the open-ontologies ecosystem. It ensures that every architectural claim is backed by physical execution evidence (OCEL) and a cryptographic receipt (BLAKE3).

## 1. Overview
The `AutoReceiptPipeline` enforces the **Chatman Equation** ($A = \mu(O)$) by processing high-level architectural receipts into verifiable execution traces. It is responsible for the end-to-end validation of 64 JTBDs across 8 operational personas.

### Persona Goals
1.  **Ontology Architect (OA)**: Ensure public-grounded models manufacture valid artifacts and receipts.
2.  **Compliance / Assurance Lead (CL)**: Prove that execution is visible, reproducible, and policy-gated.
3.  **AI Coding Agent Supervisor (AS)**: Ensure agents act only through admitted plans and verified receipts.
4.  **Release / Platform Engineer (RE)**: Maintain a clean admission path from source to published artifact.
5.  **Process Intelligence Analyst (PI)**: Convert work into object-centric route evidence (OCEL).
6.  **Product / UX Operator (UX)**: Translate deep proof states into honest, user-visible application states.
7.  **Domain Steward (DS)**: Ensure every need has an owner, route, status, and evidence of closure.
8.  **Scientific Strategist (IS)**: Represent large-scale R&D as verifiable micro-interventions.

---

## 2. The AutoReceipt Law (Architecture)
The pipeline is governed by the `AutoReceiptPipeline` law, which defines a strict sequence of 6 states. A failure at any stage prevents the emission of the final receipt.

| State ID | State Name | Description |
|---|---|---|
| **0** | `ArchitecturalReceiptParsed` | Input plan is validated and parsed into internal structures. |
| **1** | `ExpectedOcelManufactured` | The target "Expected OCEL" is synthesized from the JTBD requirements. |
| **2** | `ExecutionRegistryBound` | JTBDs are bound to actual commands, harnesses, or WASM algorithms. |
| **3** | `ObservedOcelCaptured` | The actual execution is recorded as an "Observed OCEL" log. |
| **4** | `AlignmentVerified` | Conformance checking proves the Observed matches the Expected. |
| **5** | `ReceiptEmitted` | A cryptographically anchored BLAKE3 receipt is committed to the registry. |

---

## 3. Operational Lifecycle & Monitoring

### Stage 0: ArchitecturalReceiptParsed
*   **Action**: `ggen` parses the `AUTORECEIPT_BUNDLE` and individual JTBD actuation plans.
*   **Monitor**: Check logs for "Parsing Architectural Receipt" events.
*   **Troubleshooting**: If this fails, verify the input JSON schema in `artifacts/autoreceipt/`. Ensure all `jtbd_id`s are unique and valid.

### Stage 1: ExpectedOcelManufactured
*   **Action**: Generates expected log patterns (JSON-OCEL) in `artifacts/autoreceipt/expected-ocel/`.
*   **Monitor**: Verify that `.expected.ocel.json` files exist for each active JTBD.
*   **Troubleshooting**: Missing files indicate a failure in the `.specify/templates` projection. Check for missing mapping rules in the ontology.

### Stage 2: ExecutionRegistryBound
*   **Action**: Links JTBD requirements to executable code.
*   **Monitor**: Inspect `artifacts/autoreceipt/JTBD_EXECUTION_BINDING_REGISTRY.json`.
*   **Troubleshooting**: Status `ExecutableAfterCommandBinding` means the JTBD lacks a physical command. Ensure `command_or_harness` is populated for all critical paths.

### Stage 3: ObservedOcelCaptured
*   **Action**: Executes the bound commands and captures OTel/OCEL traces.
*   **Monitor**: Real-time execution logs and `artifacts/autoreceipt/observed-ocel/`.
*   **Troubleshooting**: Handle non-zero exit codes. If execution fails, check `execution-failed` receipts for the specific error code.

### Stage 4: AlignmentVerified
*   **Action**: Runs `pm4py` conformance checking (Fitness/Precision).
*   **Monitor**: `artifacts/autoreceipt/alignment/` for `alignment.receipt.json` files.
*   **Troubleshooting**: Status `OcelAlignmentFailed` means the software behavior deviated from the architectural law. Review the "observed vs expected" diff to find the defect.

### Stage 5: ReceiptEmitted
*   **Action**: Final BLAKE3 hash generation and commit to `.ggen/receipts/`.
*   **Monitor**: `ls .ggen/receipts/` for new `.receipt.json` files.
*   **Troubleshooting**: If receipts are missing despite alignment passing, verify the BLAKE3 hashing service and filesystem write permissions.

---

## 4. Troubleshooting Matrix

| Symptom | Stage | Root Cause | Remediation |
|---|---|---|---|
| `ReceiptMissing` | 5 | Hashing failed or Disk I/O | Check `.ggen/` permissions. |
| `OcelAlignmentFailed` | 4 | Code deviation | Fix implementation to match Law. |
| `CommandNotBound` | 2 | Missing harness | Update `binding_registry.json`. |
| `MalformedReceipt` | 0 | Schema drift | Run `ggen sync` to refresh schemas. |
| `FalsePass` | 4 | Weak alignment rules | Tighten SHACL shapes in `.specify/`. |

---

## 5. Tools & Resources
- **`ggen`**: The generative muscle. Use `ggen sync` to align code with these states.
- **`open-ontologies` CLI**: Main entry point for running the pipeline gauntlet.
- **`pm4py`**: Underlying process mining engine used for alignment verification.
- **`artifacts/autoreceipt/`**: Central storage for all pipeline evidence.
