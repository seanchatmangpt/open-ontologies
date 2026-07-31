# autonomics.ttl

_Generated 2026-05-18T19:57:13Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/autonomics.ttl`
- **Triples:** 111
- **Classes:** 2 · **Properties:** 5 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `AutonomicActionClass` | Autonomic Action Class | A risk classification tier for autonomic actions performed at route stages. Instances are the five A0-A4 individuals. De |
| `AutonomicPolicy` | Autonomic Policy | A named policy governing the admission sequence and action-class escalation rules for an autonomic route. Each policy de |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `appliesTo` | AutonomicPolicy | ServiceRoute | Links an AutonomicPolicy to the ServiceRoute it governs. A route may have at mos |
| `defaultAllowedClass` | AutonomicPolicy | AutonomicActionClass | The AutonomicActionClass granted when all gates in the policy's gate sequence pa |
| `gateSequencePosition` | AdmissionGate | positiveInteger | Ordinal position (1-based) of this gate in the ordered ConnectGroupAdmissionPoli |
| `hasGateSequence` | AutonomicPolicy | AdmissionGate | Links an AutonomicPolicy to the ordered AdmissionGates that must be evaluated be |
| `onFailureClass` | AdmissionGate | AutonomicActionClass | The AutonomicActionClass that applies when this gate fails admission evaluation. |
