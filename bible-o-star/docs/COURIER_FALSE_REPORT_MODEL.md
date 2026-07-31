# Courier and False Report Model

This document defines the Courier Layer as a carrier channel, specifies the structure and detection of false reports (poisoned payloads), and documents how false reports are refused at gates.

---

## 1. The Courier Layer

### What the Courier Is

The Courier (`bos:Courier`) is a carrier channel. It transmits payloads across gate boundaries. It is not a gate. It does not evaluate admissibility. It does not emit verdicts. It does not issue receipts.

Every transmission the Courier carries is logged as a `bos:CourierRecord`. The Courier Record contains:
- The origin identity (who sent the payload)
- The path taken (which nodes the payload traversed)
- The target gate (which gate is the intended recipient)
- The payload hash (a tamper-evident fingerprint of the payload content)
- A transmission timestamp

The Courier Record is not a receipt. It proves transmission occurred. It does not prove admission occurred. Admission proof requires a `bos:Receipt` from the receiving gate.

### What the Courier Is Not

| Claim | Status |
|---|---|
| The Courier is a gate | Refused. See `bos:MessengerGate` anti-pattern in GATE_ASSIGNMENT_MODEL.md. |
| The Courier can admit artifacts | Refused. Admission is gate-exclusive. |
| The Courier can refuse artifacts | Refused. Refusal is gate-exclusive. |
| A Courier Record is equivalent to a Receipt | Refused. These are distinct artifact types with distinct semantics. |
| An unlogged transmission is valid | Refused. Transmissions without `bos:CourierRecord` entries are inadmissible as evidence. |

### Courier Routing Obligations

The Courier must:
1. Log every transmission as a `bos:CourierRecord` before delivery.
2. Route each payload to the gate declared in the payload's routing header.
3. Flag any payload lacking a routing header as `bos:UnroutablePayload`.
4. Deliver unroutable payloads to the Inspection Gate for adjudication — not to the Water Gate for broadcast.
5. Record the gate's refusal response in the Courier Record when a gate refuses a payload.

The Courier must not:
- Execute payload instructions.
- Modify payload content in transit.
- Suppress a payload without logging the suppression.
- Route a payload to a different gate than declared without a documented escalation record.

---

## 2. False Reports

### Definition

A False Report (`bos:FalseReport`) is a poisoned payload transmitted through the Courier Layer that contains unverified rumors, fabricated accusations, or adversarial instructions designed to halt, distract, or discredit the building swarm.

A False Report is not an error. It is not a malformed message. It is not an unroutable payload. A False Report is a deliberately constructed adversarial artifact that may be syntactically well-formed and correctly routed — and is still poison.

### Historical Basis

Nehemiah 6:5-7 documents the canonical false report incident:

> Sanballat sent his servant to Nehemiah with an open letter containing accusations that Nehemiah was planning a rebellion, appointing prophets to proclaim him king, and thereby defying the Persian king.

The letter was:
- Correctly formatted (a letter, not a defaced document)
- Delivered by a named courier (Sanballat's servant)
- Routed to the correct recipient (Nehemiah)
- Syntactically coherent (it made specific, detailed claims)

And it was entirely fabricated.

Nehemiah's response (Neh. 6:8): "No such things as you say have been done, for you are inventing them out of your own mind."

This is the canonical refusal pattern: name the false report as a fabrication, refuse its instructions, and log the refusal. Nehemiah did not ignore the letter, destroy it, or pretend it had not arrived. He responded with an explicit refusal and continued building.

### False Report Taxonomy

| Type | Description | Detection Signal |
|---|---|---|
| Accusation payload | Contains false accusations of rule violation or rebellion | Claims that cannot be corroborated by existing receipts |
| Fear payload | Designed to produce panic and halt work (Neh. 6:9) | Instruction is "stop building"; no lawful gate issued the stop order |
| Fabricated prophecy | A prophet claims a divine word that contradicts the operating grammar (Neh. 6:10-13) | Prophecy contradicts a sealed Gate Covenant or Inspection Receipt |
| Open letter | An accusation broadcast to surrounding nations before delivery to the target | Payload was distributed to Nations Ledger before Inspection Gate review |
| Invitation trap | Invitation to leave the work site for a meeting designed to distract the builder (Neh. 6:2-4) | No Gate Covenant authorizes the meeting; no Muster Ledger record of the meeting host |

---

## 3. False Report Detection

False reports are detected by comparing the payload's claims against the sealed receipt archive. A claim is a false report if:

1. It asserts a state that no existing `bos:Receipt` confirms.
2. It asserts a builder action that no `bos:MusterLedgerRecord` corroborates.
3. It instructs a halt that no `bos:Verdict` from the Inspection Gate authorized.
4. It attributes a proclamation to the Prophet Office that the Prophet Office has no `bos:PropheticProclamation` record for.
5. It was delivered as an open letter (broadcast before review) — this is itself a structural signal of adversarial intent.

Detection is the responsibility of the receiving gate, not the Courier. The Courier must deliver the payload. The gate must evaluate it.

### Detection Algorithm (Conceptual)

```
function evaluatePayload(payload, receiptArchive, musterLedger):
    if payload.type == bos:FalseReport:
        return REFUSE with law = bos:KnownFalseReport

    claims = payload.extractClaims()
    for each claim in claims:
        if not receiptArchive.hasCorroboration(claim):
            flag(payload, bos:UnverifiedClaim, claim)

    if payload.distributedBeforeReview:
        flag(payload, bos:OpenLetterPattern)

    if payload.instructsHalt and not inspectionGate.hasBlockedVerdict(payload.scope):
        flag(payload, bos:UnauthorizedHaltInstruction)

    if flagCount(payload) > 0:
        return REFUSE with law = bos:PoisonedPayload
    else:
        return ADMIT
```

---

## 4. False Report Refusal at Gates

When a gate identifies a false report, it must:

1. Set `bos:refusesPoison = true` on the `bos:CourierRecord` for this transmission.
2. Issue a refusal record with the named law `bos:PoisonedPayload` (or a more specific law from the taxonomy above).
3. Do not execute any instruction embedded in the false report.
4. Do not forward the false report to another gate without the refusal annotation.
5. Log the refusal in the Receipt Archive as evidence that the poison was encountered and refused.
6. Continue building. A refused false report does not halt work.

**The refusal is not a verdict on the sender's intent.** The gate evaluates the payload, not the person. If the payload cannot be corroborated by existing receipts, it is refused. Whether the sender knew it was false is outside the gate's jurisdiction.

### Refusal Record Structure

| Field | Value |
|---|---|
| `rdf:type` | `bos:FalseReportRefusal` |
| `bos:refusesPoison` | `true` |
| `bos:refusalLaw` | `bos:PoisonedPayload` (or specific named law) |
| `bos:refusingGate` | The gate that issued the refusal |
| `bos:originalCourierRecord` | Link to the `bos:CourierRecord` for this transmission |
| `bos:claimCorroborationStatus` | List of claims and their corroboration results |
| `bos:timestamp` | Refusal timestamp |

---

## 5. False Report vs. Mocker Feedback

False reports and Mocker Feedback are distinct artifact types and must not be conflated.

| Dimension | False Report (`bos:FalseReport`) | Mocker Feedback (`bos:MockerFeedback`) |
|---|---|---|
| Source | An adversary claiming a false state of affairs | An adversary emitting discouraging or mocking signals |
| Channel | Delivered via Courier as a routed payload | Emitted directly at the wall boundary |
| Intent | Halt construction by fabricating a crisis | Demoralize builders by questioning the work's feasibility |
| Detection | Corroboration check against receipt archive | Signal extraction — is the criticism structurally true? |
| Response | Refuse the payload; log `bos:refusesPoison = true` | Extract the signal; reinforce the wall section if valid |
| Example | Sanballat's letter (Neh. 6:5-7) | "What are these feeble Jews doing... will a fox break down their wall?" (Neh. 4:3) |

Mocker Feedback is not refused. It is mined for structural signals. If Tobiah's fox test is valid (a weak section would not withstand a fox), the swarm reinforces that section. The mocker's intent is adversarial; the signal may still be useful.

False reports are refused without execution. They carry no valid structural signal because they assert states that cannot be corroborated.

---

## 6. Operational Invariants

1. **The Courier is a carrier, not a gate.** Routing, delivery, and logging are Courier functions. Admission and refusal are gate functions.
2. **Every transmission must have a Courier Record.** Unlogged transmissions are inadmissible as evidence.
3. **Every false report refusal must carry a named law.** Bare `bos:refusesPoison = true` without a named law is an incomplete refusal record.
4. **Refused payloads do not halt construction.** Nehemiah continued building after refusing the open letter.
5. **Gates evaluate payloads, not senders.** A payload is refused because it fails corroboration, not because the sender is an adversary.
6. **Open letters are structurally suspect.** Any payload distributed to the Nations Ledger before Inspection Gate review carries the open letter flag and must be evaluated with heightened scrutiny.
7. **Fabricated prophecy is a false report.** A prophetic proclamation that cannot be corroborated by the Prophet Office record is a `bos:FabricatedProphecy` false report (Neh. 6:12-13).
