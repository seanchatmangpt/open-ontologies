# GitHub Factory (GHF)

The GitHub Factory is an AI-native infrastructure-as-code and contribution-tracking system for the `open-ontologies` ecosystem. It leverages the Ostar Generative Pipeline to ensure that all GitHub activities are lawful, auditable, and backed by cryptographic proofs.

## Architecture

GHF operates on three layers:
1.  **Ontology (Source of Truth)**: `ontology/ghf-core.ttl` and `ghf-shacl.ttl` define the repositories, labels, and contribution units.
2.  **Manufactured Artifacts**: `ggen sync` generates Terraform files, GitHub workflows, and OCEL templates.
3.  **Actuation & Proof**: The Gemini CLI executes infrastructure changes and emits BLAKE3 receipts binding execution to the ontology.

## Human-in-the-Loop: Lifting an Andon

An **Andon** is triggered when the automated pipeline encounters a deviation or failure that it cannot resolve autonomously. This might happen due to:
- A failed OCEL alignment (observed behavior doesn't match expected law).
- A missing receipt for a critical infrastructure change.
- A manual change on GitHub that drifts from the ontology.

### How to Resolve

When an andon is triggered, a human operator must:

1.  **Identify the Blocker**: Check the `docs/ghf-proof-matrix.md` and the most recent `weekly-ledger`. Look for `[PENDING]` or `[FAILED]` status.
2.  **Inspect the Evidence**:
    - Review the expected OCEL in `artifacts/ghf/ocel/expected.ocel.jsonl`.
    - Review the observed OCEL in `artifacts/ghf/ocel/observed/`.
    - Check the alignment report using `gemini alignment verify`.
3.  **Correct the State**:
    - **If the Ontology is wrong**: Update `ontology/ghf-core.ttl` to reflect the new desired state, then run `ggen sync`.
    - **If the Execution failed**: Manually remediate the GitHub state (e.g., via the GitHub UI or `gh` CLI) to match the ontology, then re-run the evidence collection script.
    - **If the Receipt is missing**: Force an emission if the state is confirmed lawful: `gemini auditor emit --force`.
4.  **Clear the Andon**:
    - Once the state is aligned and a valid receipt exists, run `gemini auditor verify` to confirm closure.
    - Update the `docs/ghf-proof-matrix.md` with the new hashes.

## Key Commands

- `ggen sync`: Regenerate all GHF artifacts from the ontology.
- `scripts/collect-github-evidence.sh`: Pull the current state of GitHub as observed OCEL.
- `gemini alignment verify`: Compare observed OCEL against the expected law.
- `terraform -chdir=artifacts/ghf/terraform apply`: Apply infrastructure changes.

---
*For more details on the underlying philosophy, see [ADR 0001: GitHub Factory Implementation](./adr/0001-github-factory-implementation.md).*
