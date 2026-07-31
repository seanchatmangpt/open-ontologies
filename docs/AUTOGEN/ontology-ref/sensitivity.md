# sensitivity.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/sensitivity.ttl`
- **Triples:** 124
- **Classes:** 2 · **Properties:** 5 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `DataSensitivityLevel` | Data Sensitivity Level | A sensitivity tier applied to a ZOE LA data class to govern access control, consent requirements, and audit obligations. |
| `SensitivityClassification` | Sensitivity Classification | Associates a ZOE LA data class with a sensitivity level, consent requirements, and the roles permitted to access it. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `classificationAccessRoles` | SensitivityClassification | string | Pipe-delimited list of role labels permitted to access data under this classific |
| `classificationRequiresConsent` | SensitivityClassification | boolean | Whether explicit informed consent must be obtained before processing data under  |
| `protectsClass` | SensitivityClassification | Class | The OWL class whose instances are governed by this sensitivity classification. |
| `requiresGuardianConsent` | SensitivityClassification | boolean | Whether guardian (parent or legal guardian) consent is required in addition to o |
| `sensitivityLevel` | SensitivityClassification | DataSensitivityLevel | The data sensitivity tier assigned to this classification. |
