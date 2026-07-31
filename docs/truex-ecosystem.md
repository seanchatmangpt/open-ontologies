# Truex Ecosystem Documentation

> **"Unproven consequence has no operational authority."**

## Closure Theorem: TruexEcosystemClosed

The manufacture of this ecosystem is only complete when the following obligations are discharged:


- **AdmissibleGraphProjection**

- **ReleaseReceiptEmitted**

- **FraudulentPathRefused**

- **ValidPathAdmitted**

- **OCELLaunderingRefused**

- **SummaryOnlyProofRefused**

- **RawBoundaryMandatory**

- **VerifierIndependent**

- **AllSchemasMaterialized**

- **AllRequiredPackagesEmitted**



## Package Manifest

| Name | Purpose |
|---|---|
| @truex/vkg | Virtual knowledge graph projection over execution traces and state mutations. |
| @truex/cli | Command-line interface for Truex (verify, admit, replay). |
| @truex/examples | Valid and fraudulent checkout demonstrations. |
| @truex/conformance | Golden corpus and adversarial conformance tests. |
| @truex/replay | Replay artifact generation (Markdown, Mermaid, Audit). |
| @truex/otel | OpenTelemetry (OTLP) egress adapter. |
| @truex/capture | Proxyable state mutation capture and evidence collection. |
| @truex/verifier | Independent receipt verification and refusal authority. |
| @truex/receipt | Truex receipt schema and hash-chain management. |
| @truex/canonical | Canonical OCEL ordering and stable digest generation. |
| @truex/ocel2 | OCEL 2.0 logical model + validators. |



## Rules of Engagement

### Raw Boundary Mandatory
Closure requires raw boundary evidence (stdout/stderr/exit), not summary hashes.
### Independent Verifier
The verifier must be logically and physically distinct from the producer.
### No Receipt, No Closure
Physical invariant: closure is not admitted without a cryptographic receipt.



## Failure Taxonomy

- **BoundaryProjectionFailure**: Failed to project raw boundary evidence into a valid OCEL structure.
- **TemporalOrderingViolation**: Observed event sequence violates the causal/temporal laws defined in the expected path.
- **ArtifactOriginMismatch**: Emitted artifact hash or path does not match the derivation origin in the boundary evidence.
- **StateTransitionMismatch**: Observed object state transitions do not match the expected state graph mutations.
- **NonDerivableExecution**: The claimed execution path cannot be physically derived from raw boundary evidence.
- **MissingBoundary**: Absence of required physical execution evidence.
- **CloneTrace**: Using expected OCEL as observed evidence (cloning).
- **SummaryOnlyProof**: Attempting closure with only high-level hashes (stdout_hash) without raw evidence.
- **OCELLaundering**: Formatting OCEL from summary data without raw boundary derivation.



## Derivation Calculus Rules

### MaximalDerivation
none
### FilesystemMutationDerivesArtifactObject
none
### StdoutHashDerivesArtifactEmission
none
### RawExitCodeDerivesExecutionStatus
ExecutionComplete

