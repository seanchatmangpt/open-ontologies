# Mockers and Adversarial Feedback

## What a Mocker Is (and Is Not)

A **mocker** in the Bible O* ontology is an **adversarial feedback agent** — not a gate, not a
person-type insult, and not an identity label. The term maps to a structural role: an entity that
emits signals intended to destabilize the construction process.

In the Nehemiah 52 operating grammar, Sanballat and Tobiah exemplify this role. Their mockery
("Even what they build — if a fox goes up on it, he will break down their stone wall") is not
merely noise; it is an adversarial probe carrying a real structural hypothesis. The mocker pattern
is therefore modeled as a **source of incoming adversarial signals that must be captured, analyzed,
and routed — not ignored and not accepted as authority.**

## The MockerFeedback Type

`bos:MockerFeedback` is a first-class ontology node with the following required properties:

| Property | Requirement | Purpose |
|---|---|---|
| `rdfs:comment` | minCount 1 | Human-readable description of the extracted signal |
| `bos:hasSource` | minCount 1 | Citation of the adversarial origin (scripture ref, system source) |

A `MockerFeedback` node without `rdfs:comment` is an unanalyzed noise record — the signal has not
been extracted and the feedback cannot be routed. This is a structural defect, not an acceptable
open record.

## How Mocker Feedback Flows Through InspectionGate

The InspectionGate is the terminal inspection stage where adversarial feedback receives its verdict.
Flow:

```
Adversarial source (mocker)
    |
    v  [bos:hasSource]
bos:MockerFeedback
    |
    +-- rdfs:comment  ──────> signal extracted and described
    |
    v  [routes through InspectionGate logic]
bos:InspectionReceipt
    |
    +-- bos:hasVerdict  ────> ALIVE | PARTIAL | BLOCKED
    +-- bos:hasReceipt  ────> hash chain / provenance record
    +-- bos:hasSource   ────> evaluation source citation
```

The InspectionGate does not absorb mocker feedback directly — it receives a `MockerFeedback` record
that has already been processed by the gate assignment layer and carries an extracted signal in
`rdfs:comment`. The gate produces an `InspectionReceipt` with a final verdict.

A `MockerFeedback` record that never reaches an `InspectionReceipt` is an **open threat** — it
exists in the log but has no resolved verdict. The SHACL gate (`bos:InspectionReceiptShape`) enforces
that every receipt carries `bos:hasVerdict`, `bos:hasReceipt`, and `bos:hasSource`, so unresolved
mocker records are detectable by conformance checking.

## Mocker Rejection Patterns

Three canonical rejection patterns are defined. Each pattern names a specific reason for refusal
rather than a generic "invalid input" verdict.

### 1. FalseReport Refusal

The mocker issues a `bos:FalseReport` — a fabricated threat or distorted intelligence payload.

- Rejection signal: `bos:refusesPoison = true`
- Shape enforced by: `bos:FalseReportShape`
- Named law: the false report was **refused** (not ignored, not accepted)
- Evidence: `bos:hasSource` names the adversarial origin

A `FalseReport` with `bos:refusesPoison = false` or missing remains a live threat in the log.
Conformance checking detects this as an unresolved structural defect.

### 2. Signal Extraction Without Panic

The mocker issues a probing statement that contains a real structural hypothesis (e.g., "a fox
will break down the wall"). The correct response is **not** panic and **not** dismissal.

- Signal is extracted into `rdfs:comment` on the `MockerFeedback` node
- The structural claim is routed to the relevant `bos:WallSection` for re-inspection
- If the wall section is sound: `bos:InspectionReceipt` verdict = ALIVE
- If the section has a weakness: verdict = PARTIAL or BLOCKED, triggering reinforcement

The pattern enforces that mocker feedback is never lost — it must produce an `InspectionReceipt`
or remain an open defect in the conformance log.

### 3. Nations Ledger Closure

After mocker pressure peaks and wall completion is achieved, external witnesses (nations) observe
the outcome. This is captured in `bos:NationsLedgerRecord`:

- `bos:hasNationsSignal`: the nations' observation (e.g., "this work was done by our God")
- `bos:hasSource`: the scripture or historical record
- This record closes the adversarial feedback loop with external attestation

The `bos:NationsLedgerRecordShape` enforces that nations records carry both the signal and the
source. A nations ledger record without these properties cannot serve as closure evidence.

## Mapping to Process-Evidence Terms

| Bible O* Term | Process-Evidence Term |
|---|---|
| Mocker | adversarial feedback agent |
| MockerFeedback | adversarial signal record |
| FalseReport | poisoned payload / fabricated threat |
| bos:refusesPoison | refusal receipt (named law) |
| InspectionReceipt | proof gate artifact |
| bos:hasVerdict | verdict field (ALIVE / PARTIAL / BLOCKED) |
| NationsLedgerRecord | external attestation / closure evidence |

## Invariants

- A mocker is never a gate. Mocker feedback routes **through** gates; it does not define them.
- A mocker is not a person-type insult. The role is structural: any entity that emits adversarial
  signals to destabilize construction occupies this role, regardless of identity.
- Signal extraction is mandatory. Dismissing mocker feedback without extracting its structural
  signal is the same defect as ignoring it. Both leave an open record in the conformance log.
- Every mocker feedback record that enters the system must produce a terminal `InspectionReceipt`
  or remain an explicit open defect. There is no "close without verdict" path.
