# revops-manufacturing.ttl

_Generated 2026-05-18T19:57:13Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/revops-manufacturing.ttl`
- **Triples:** 50
- **Classes:** 1 · **Properties:** 3 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `ManufacturingStage` | Manufacturing Stage | A discrete gate in the CodeManufactory RevOps pipeline. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `hasStage` | RevOpsManufacturingProfile | ManufacturingStage | Links a manufacturing profile to its constituent stages. |
| `stageName` | ManufacturingStage | string | Machine-readable stage identifier used in OCEL event log emissions. |
| `stageOrder` | ManufacturingStage | integer | Monotonic execution order (1-indexed). Stages must execute in ascending order. |
