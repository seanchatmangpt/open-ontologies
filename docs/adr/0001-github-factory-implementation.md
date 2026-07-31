# ADR 0001: GitHub Factory Implementation

## Status
Accepted

## Context
We need a scalable, auditable system for managing GitHub infrastructure (repositories, labels, branch protection) and tracking contributions within the `open-ontologies` ecosystem. Previous attempts at manual configuration or simple script-based management lacked the cryptographic rigor and architectural alignment required for autonomous operation.

## Decision
We will implement the GitHub Factory (GHF) using a combination of public ontologies and the Ostar Generative Pipeline.

### 1. Public Vocabularies with `ghf:` Profile
Instead of reinventing infrastructure concepts, we anchor GHF in established public vocabularies:
- **PROV-O**: For tracking the provenance of artifacts and execution.
- **IES 4D**: For modeling state transitions and consequences.
- **Dublin Core**: For basic metadata.

A thin `ghf:` profile (defined in `ontology/ghf-core.ttl`) will map these general concepts to specific GitHub primitives (e.g., `ghf:GitHubRepository`, `ghf:BranchProtection`).

### 2. Autonomic Execution + Cryptographic Governance
We adopt a "best of both worlds" architecture:
- **Autonomic Execution**: The Gemini CLI acts as the actuation membrane. It executes commands based on the semantic laws defined in the ontology and synced via `ggen`.
- **Cryptographic Governance**: Every action taken by the CLI must produce a BLAKE3 receipt. These receipts bind the expected state (O*), the execution operator (μ), and the observed artifact (A).

This ensures that while the system can operate autonomously, every change is verifiable and traces back to a lawful requirement.

## Consequences
- **High Observability**: Every infrastructure change is backed by an OCEL (Object-Centric Event Log) trace and a cryptographic receipt.
- **Strict Compliance**: Changes that do not align with the ontology (O*) will fail the verification gate.
- **Reduced Manual Overhead**: Repetitive tasks like label creation and branch protection are managed through Terraform artifacts generated directly from the ontology.
- **Clear Audit Trail**: The weekly ledger and proof matrix provide a human-readable summary of the system's state and history.
