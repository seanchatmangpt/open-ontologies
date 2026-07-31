# Prophetic Proclamation Model — Bible O*

## Doctrine: Prophet as Office, Not Gate
The Prophet is a proclamation office. It is not a gate.
There is no "ProphetGate" in the Nehemiah 52 operating grammar.
Any reference to a "ProphetGate" is an anti-pattern. See nehemiah-52.ttl bos:ProphetGate (deprecated).

## bos:ProphetOffice
The ProphetOffice emits proclamations into the system.
Proclamations are not binding authority over gates.
They are inputs to the Prayer Layer and the Inspection Gate.

## bos:PropheticProclamation fields
- bos:proclamationText — the declared word
- bos:proclamationSource — which prophetic office
- bos:targetGate — which gate receives this proclamation
- bos:hasPrayer — upward appeal accompanying the proclamation
- bos:hasReceipt — evidence of proclamation emission

## False Prophets in the Nehemiah System
False prophets in Nehemiah 6 are modeled as FalseReport emitters, not ProphetOffice instances.
A false prophet produces a FalseReport (bos:FalseReport) routed to the Courier layer.
The Courier layer classifies payload and may refuse poison.

## Water Gate Reading
Ezra reads the law at the Water Gate (Nehemiah 8).
Water Gate = public law reading, community formation, covenant renewal.
Not a content filter. A formation boundary.

## Operating Rule
Prophet speaks. Gate decides. Inspection Gate witnesses. Prayer carries.

---

## Extended Doctrine

### 1. What the Prophet Office Is

The prophet office declares what has been commissioned. It does not control ingress or
egress. It does not issue InspectionReceipts. It does not block or admit payloads.

`bos:PropheticProclamation` records what was declared — by whom, from what source, at what
operational moment — and no more.

The prophet office in Nehemiah is present but subordinate to the building mission:
- Neh.6.14: Nehemiah prays against Noadiah the prophetess and other prophets who tried to
  frighten him. The prophet office is subject to audit because it can be captured.
- Neh.6.12-13: "I perceived that God had not sent him, but he had pronounced the prophecy
  against me because Tobiah and Sanballat had hired him." — An adversary can install a
  fraudulent prophet to issue false proclamations at a critical gate moment.

### 2. PropheticProclamation Records What Was Declared

A `bos:PropheticProclamation` instance includes:
- The proclaiming agent (`bos:hasProphet`)
- The text of the proclamation (`bos:proclamationText` / as `rdfs:comment`)
- The source verse (`bos:proclamationSource` / `bos:hasSource`)
- The target gate receiving the proclamation (`bos:targetGate`)
- An accompanying upward appeal (`bos:hasPrayer`)
- Evidence of proclamation emission (`bos:hasReceipt`)
- A validity signal: genuine or adversarially captured (`bos:isAdversarial` as boolean)

The validity signal does not come from the prophet — it comes from outcome alignment with
the canonical mission. A prophet who declared fear when courage was required is marked as
adversarial regardless of claimed authority.

### 3. No ProphetGate

There is no `bos:ProphetGate` in this ontology. This design decision is not arbitrary:

- Gates control physical or semantic ingress/egress. Prophets do not stand at gates.
- Gate admissibility is determined by structural criteria (receipts, verdicts, named laws).
  A prophetic proclamation is not a structural criterion.
- Allowing a prophet to function as a gate would create a path for adversarial capture of
  the admission function — exactly the attack Tobiah and Sanballat attempted in Neh.6.

If you are designing a system and you find yourself tempted to create a `ProphetGate`,
the correct response is: that is not a gate feature request. That is a FalseReport
attempting to install itself as gate logic.

### 4. Genuine vs. Adversarially Captured Proclamations

| Signal | Genuine Proclamation | Adversarially Captured Proclamation |
|---|---|---|
| Source | Commission-aligned | Hired by opponent |
| Direction | Toward mission completion | Away from mission (fear, pause, retreat) |
| Nehemiah's test | "I perceived that God had not sent him" | "Tobiah and Sanballat had hired him" |
| Ontological mark | `bos:isAdversarial false` | `bos:isAdversarial true` |
| What happens | Recorded as proclamation evidence | Recorded as FalseReport and routed to CourierRecord |

### 5. Ontological Role Summary

| Property | Value |
|---|---|
| Class | `bos:PropheticProclamation` |
| Is it a gate? | No |
| Can a prophet block a wall section? | No |
| Can a prophet issue an InspectionReceipt? | No |
| Can the prophet office be adversarially captured? | Yes — this is the central risk |
| Audit mechanism | Source alignment + mission direction |
