# Edge Lab Infrastructure Specification
**KRN 850 & SOC 910 Testbed Requirements**

To empirically test WebAssembly (`wasm4pm`) kernels and partition tolerance in civic deployment scenarios, the PhD program requires a physical edge-computing testbed. This lab simulates the degraded network environments of a local civic distribution center.

## Hardware Specifications
*   **Cluster Nodes:** 4x Raspberry Pi 5 (8GB RAM) serving as edge compute nodes.
*   **Storage:** 256GB NVMe SSDs via PCIe HATs (for SQLite/StateDb write-ahead log stability).
*   **Networking:** An isolated, air-gapped Gigabit switch. A programmable router to simulate packet loss, high latency, and network partitions (Split-Brain testing).
*   **Client Devices:** 2x Android Tablets to simulate volunteer edge-clients submitting Ed25519 signed receipts.

## Software Architecture
*   **Orchestration:** K3s (Lightweight Kubernetes) optimized for ARM64 edge deployments.
*   **Wasm Runtime:** WasmEdge runtime configured with the `open-ontologies` `wasm4pm` stream-2 stub bindings. 
*   **Database:** Local SQLite utilizing the `StateDb` configurations established in `open-ontologies/src/config.rs`.
*   **Synchronization:** The JSONL receipt chain (`chain.jsonl`) will utilize an eventual-consistency sync protocol when the network partition is resolved.

## Practical Lab Objectives
1.  **Partition Survival:** Sever the uplink to the main campus network. Prove that the local Wasm kernel can continue to emit `admission_granted` events locally using offline Ed25519 verification.
2.  **Chain Reconciliation:** Restore the uplink. Prove that the local `chain.jsonl` cleanly merges with the central root chain without cryptographic contradictions.
3.  **Power Draw:** Profile the CPU and wattage footprint of the `VerifierWorker` daemon polling the receipt chain every 30 seconds on ARM64.