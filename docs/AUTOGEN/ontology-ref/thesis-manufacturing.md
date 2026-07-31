# thesis-manufacturing.ttl

_Generated 2026-05-18T19:57:13Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/thesis-manufacturing.ttl`
- **Triples:** 262
- **Classes:** 10 · **Properties:** 21 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `AdmittedClaim` | Admitted Claim | A claim that passed the deterministic admission gate. Carries a Receipt and is bound to a support status, domain, and sc |
| `ArtifactRef` | Artifact Reference | A reference to an external artifact (paper, dataset, code, simulation). Carries locator, checksum, and access timestamp. |
| `Chapter` | Chapter | A section of the thesis organizing claims, evidence, and laws around a research question or topic. |
| `Claim` | Claim | A factual proposition made by a thesis. Not authoritative until admitted through deterministic gate. |
| `Defect` | Defect | A structural or substantive defect found in a claim, evidence, or thesis. Blocks admission until resolved. |
| `Evidence` | Evidence | An empirical artifact, measurement, or citation supporting a claim. Must trace to published or reproducible source. |
| `Law` | Law | An enforced constraint or principle within the thesis. Must be derived from admitted claims and evidence. |
| `Projection` | Projection | An LLM-projected or derived summary of evidence. Authority is NOT granted; projections are provisional and require admis |
| `ResearchQuestion` | Research Question | A research question that frames a thesis domain. Origin of all claims and evidence collection. |
| `VerifiedEvidence` | Verified Evidence | Evidence that passed verification gates: source validation, measurement conformance, citation resolution, reproducibilit |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `accessedAt` | ArtifactRef | dateTime | Timestamp when the artifact was last accessed/verified. |
| `artifactChecksum` | ArtifactRef | string | BLAKE3 or SHA256 hash of the artifact for integrity verification. |
| `artifactLocator` | ArtifactRef | string | URL, DOI, file path, or other locator for the artifact. |
| `claimCoverage` | Chapter | integer | Number of admitted claims in the chapter. |
| `claimText` | Claim | string | The full text of the claim proposition. |
| `defectClass` | Defect | Concept | Classification of the defect (ProjectionSubstitutedForProof, EvidenceMissing, et |
| `defectDescription` | Defect | string | Narrative explanation of the defect and remediation guidance. |
| `derivesFromLaws` | Claim | Law | Laws that constrain or enforce properties of this claim. |
| `domain` | Claim | string | Domain or field (e.g., 'machine learning', 'manufacturing', 'ontology engineerin |
| `enforcedBy` | Law | Defect | Defects that trigger enforcement of this law. |
| `evidenceType` | Evidence | string | Type of evidence: empirical-data, measurement, simulation, citation, code, datas |
| `hasResearchQuestion` | Chapter | ResearchQuestion | Links a chapter to its guiding research question. |
| `projectionBoundary` | Projection | string | Description of what admitted claims and evidence the projection cites. Required  |
| `researchQuestionText` | ResearchQuestion | string | The textual statement of the research question. |
| `scope` | Claim | string | Textual description of the claim scope: what is asserted and what is not. |
| `severity` | Defect | string | Defect severity: red (blocking admission), orange (requires resolution), yellow  |
| `supportedByClaim` | Evidence | Claim | Links evidence to the specific claim it supports. |
| `supportStatus` | Claim | Concept | One of: supported, partially-supported, unproven, overclaimed, contradicted, pro |
| `targetClaim` | Defect | Claim | The claim that the defect targets. |
| `unsupportedClaimCount` | Projection | integer | Number of unproven or projection-only claims in the projection. Must be 0 for ad |
| `wasDerivedFromPublication` | Evidence | ArtifactRef | Evidence traces to a published paper, dataset, or code artifact. Required for ve |
