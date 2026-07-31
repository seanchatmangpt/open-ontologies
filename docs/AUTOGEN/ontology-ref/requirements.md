# requirements.ttl

_Generated 2026-05-18T19:57:13Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/requirements.ttl`
- **Triples:** 124
- **Classes:** 18 · **Properties:** 14 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `AdmittedCtq` | Admitted CTQ | A CTQ that passed the deterministic admission gate. Carries a Receipt and is bound to all 5 mandatory fields (source, me |
| `CandidateCtq` | Candidate CTQ | An LLM-translated provisional Critical-To-Quality structure with the 5 mandatory fields. Authority is not granted by the |
| `CandidateRequirement` | Candidate Requirement | A provisional requirement proposed from a source signal. Not authoritative until admitted. |
| `ControlPlan` | Control Plan | Regression-prevention plan. Required for CTQ admission. |
| `Counterfactual` | Counterfactual Delta | What naked-craft (unadmitted prompt-to-code) would have allowed vs what the manufacturing path enforces. Required for wo |
| `ExecutiveProjection` | Executive Projection | An LLM-projected summary of admitted evidence. Bound by a token-overlap check: the summary may only cite tokens already  |
| `NegativeCase` | Negative Case | What the gate must refuse. Required for CTQ admission. |
| `SourceSignal` | Source Signal | An origin of requirement-relevant evidence: customer voice, operator complaint, defect log, control-plan gap, runtime ab |
| `VerificationMethod` | Verification Method | How the CTQ is checked: positive case, missing-evidence case, replay test, etc. |
| `VoiceOfBusiness` | Voice of Business | — |
| `VoiceOfControlPlan` | Voice of Control Plan | — |
| `VoiceOfCounterfactual` | Voice of Counterfactual Waste | — |
| `VoiceOfCustomer` | Voice of Customer | — |
| `VoiceOfDefect` | Voice of Defect | — |
| `VoiceOfOperator` | Voice of Operator | — |
| `VoiceOfPolicy` | Voice of Policy | — |
| `VoiceOfProcess` | Voice of Process | — |
| `WorkOrder` | Work Order | An admitted unit of work. Bound to an admitted CTQ AND to a counterfactual delta. Implementation workers receive only ad |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `bindsToWorkflow` | CandidateRequirement | Thing | Reference to the declared POWL workflow that this requirement is bound to (typic |
| `counterfactualDelta` | Counterfactual | string | The defect or risk this work order prevents — what makes the manufacturing path  |
| `derivesFromCtq` | WorkOrder | AdmittedCtq | — |
| `hasControlPlan` | CandidateCtq | string | — |
| `hasCounterfactual` | WorkOrder | Counterfactual | — |
| `hasCtqText` | CandidateCtq | string | — |
| `hasMeasure` | CandidateCtq | string | — |
| `hasNegativeCase` | CandidateCtq | string | — |
| `hasSourceVoice` | CandidateRequirement | string | The verbatim source-voice text. Required and non-empty for proposal admission. |
| `hasVerification` | CandidateCtq | string | — |
| `isProvisional` | CandidateRequirement | boolean | True until the deterministic admission gate produces a receipt. Set by the syste |
| `manufacturingPath` | Counterfactual | string | What OntoStar admission/replay enforces. |
| `nakedCraftPath` | Counterfactual | string | What unadmitted prompt-to-code would have allowed. |
| `voiceKind` | SourceSignal | string | One of: customer, operator, process, defect, control_plan, counterfactual, busin |
