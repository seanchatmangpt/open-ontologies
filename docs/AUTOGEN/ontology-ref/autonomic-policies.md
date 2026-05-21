# autonomic-policies.ttl

_Generated 2026-05-18T19:57:13Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/autonomic-policies.ttl`
- **Triples:** 120
- **Classes:** 4 · **Properties:** 6 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `AdmissionCondition` | Admission Condition | A named conjunct in the AutonomicActionAllowed equation. Each condition corresponds to one of the 8 boolean predicates t |
| `AdmissionConditionSet` | Admission Condition Set | A set of AdmissionConditions that jointly constitute the AutonomicActionAllowed equation for a specific route. An Autono |
| `AutonomicActionAllowed` | Autonomic Action Allowed | Class of execution decisions that satisfy all required conjuncts in the AutonomicActionAllowed equation. An individual i |
| `EscalationRule` | Escalation Rule | A named rule specifying which AutonomicActionClass applies when a specific AdmissionGate fails. Escalation rules overrid |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `escalatesOnGate` | EscalationRule | AdmissionGate | The AdmissionGate whose failure triggers this escalation rule. |
| `escalatesToClass` | EscalationRule | AutonomicActionClass | The AutonomicActionClass that applies when the referenced gate fails. For most g |
| `hasEscalationRule` | AutonomicPolicy | EscalationRule | Links an AutonomicPolicy to an EscalationRule that overrides the default A4_REFU |
| `requiresCondition` | AdmissionConditionSet | AdmissionCondition | Links an AdmissionConditionSet to one of its required conjunct AdmissionConditio |
| `satisfiedByGate` | AdmissionCondition | AdmissionGate | Links an AdmissionCondition to the runtime AdmissionGate whose evaluation determ |
| `satisfiesConditionSet` | AutonomicActionAllowed | AdmissionConditionSet | Links an autonomic action execution decision to the AdmissionConditionSet it sat |
