# Gate Assignment Model

This document defines how artifacts, communications, and builders are routed to specific gates in the Nehemiah 52 operating grammar. It specifies gate admission and refusal logic and the rules governing builder assignment to wall sections.

---

## 1. Gate Topology

The wall has ten canonical gates. Each gate is an admissibility boundary — not a message queue, not a label, not a topic tag. A gate evaluates incoming artifacts and either admits them (allowing work to proceed) or refuses them (logging the refusal with a named law).

| Gate | `bos:` Instance | Operational Role | Facing |
|---|---|---|---|
| Sheep Gate | `bos:SheepGate` | Dedication boundary; initiates new work lifecycles | North |
| Fish Gate | `bos:FishGate` | External categorization intake | North |
| Old Gate | `bos:OldGate` | Foundational standards preservation; also governs change preservation in Phase H | Northwest |
| Valley Gate | `bos:ValleyGate` | Lower-bound constraint handling; lowest elevation entry | West |
| Dung Gate | `bos:DungGate` | Unsoundness refutation; waste removal boundary | South |
| Fountain Gate | `bos:FountainGate` | Reference coordination and link resolution | South |
| Water Gate | `bos:WaterGate` | Truth distribution; authoritative broadcast point | East |
| Horse Gate | `bos:HorseGate` | Dynamic defense; active perimeter patrol | East |
| East Gate | `bos:EastGate` | Transition governance; forward-facing change admission | East |
| Inspection Gate | `bos:InspectionGate` | Final verification; emits ALIVE / PARTIAL / BLOCKED receipts | East |

No gate is interchangeable with another. Routing an artifact to the wrong gate is not a shortcut; it is a conformance violation.

---

## 2. Artifact Routing Rules

Every artifact in the system carries a type declaration. The type determines which gate receives the artifact. The routing rules below are exhaustive; artifacts that match no rule must be refused by the Courier with a `bos:UnroutablePayload` flag.

| Artifact Type | Target Gate | Routing Basis |
|---|---|---|
| New work declaration / architecture vision | Sheep Gate | Dedication precedes construction (Neh. 3:1) |
| External data stream / external categorization request | Fish Gate | External-facing intake; visible to surrounding nations |
| Foundational standard check / prior law reference | Old Gate | Preservation constraint; Joiada and Meshullam assignment |
| Lower-bound constraint / minimum viable scope | Valley Gate | Lowest elevation; Hanun's 1000-cubit section (Neh. 3:13) |
| Unsound artifact / refutation payload | Dung Gate | Waste removal; Malkijah's boundary |
| Cross-reference resolution / link coordination | Fountain Gate | Reference coordination; Shallun's boundary (this agent) |
| Authoritative broadcast / law distribution | Water Gate | Truth distribution; Ezra's reading point (Neh. 8:1) |
| Defense signal / adversarial threat response | Horse Gate | Dynamic defense; active perimeter |
| Change proposal / transition request | East Gate | Forward governance; sunrise-facing |
| Inspection request / verdict query | Inspection Gate | Final verification; only gate that emits receipts with verdicts |
| Communication payload (all types) | Courier Layer (pre-routing) | Courier routes to target gate; does not admit or refuse |

---

## 3. Gate Admission Logic

A gate admits an artifact when the artifact satisfies the gate's law. Admission is not a formality. It is a gate-level evaluation that must be recorded.

**Admission produces:**
- A `bos:Receipt` sealed with the admitting gate's identity and a timestamp.
- A `bos:MusterLedgerRecord` update if a builder's section is affected.
- An updated `bos:hasReceipt` triple on the admitted artifact.

**Admission conditions by gate:**

| Gate | Admission Condition |
|---|---|
| Sheep Gate | Artifact carries a valid dedication marker; the initiating builder is registered in the Muster Ledger |
| Fish Gate | Artifact origin is external; payload conforms to the categorization schema |
| Old Gate | Artifact does not violate prior foundational law; passes Old Gate preservation check |
| Valley Gate | Artifact represents a minimum viable scope claim; lower-bound constraint is stated and measurable |
| Dung Gate | Artifact is confirmed unsound; refutation payload is correctly typed as `bos:UnsoundArtifact` |
| Fountain Gate | Cross-reference links are resolvable; source ledger entry exists |
| Water Gate | Payload is verified true; distribution is authoritative and non-poisoned |
| Horse Gate | Defense signal is genuine; adversarial threat is classified and not a false report |
| East Gate | Change proposal carries a valid Gate Covenant draft; Old Gate preservation check has passed |
| Inspection Gate | Inspection request covers a complete wall section; all required receipts are present |

---

## 4. Gate Refusal Logic

A gate refuses an artifact when the artifact fails the gate's law. Refusal is not silence. Every refusal must carry a named law — bare `InvalidInput` or unnamed refusals are themselves defects.

**Refusal produces:**
- A `bos:CourierRecord` update noting the refusal gate, the artifact identity, and the named law violated.
- No `bos:Receipt` is issued for a refused artifact.
- The artifact is returned to the Courier for logging. It is not forwarded to another gate without a remediation record.

**Named refusal laws (non-exhaustive):**

| Named Law | Triggered By | Refusing Gate |
|---|---|---|
| `bos:DedicationMissing` | Work declaration lacks a dedication marker | Sheep Gate |
| `bos:BuilderNotMustered` | Initiating builder absent from Muster Ledger | Sheep Gate |
| `bos:ExternalOriginUnverified` | External payload origin cannot be confirmed | Fish Gate |
| `bos:FoundationalLawViolation` | Artifact violates a prior foundational standard | Old Gate |
| `bos:LowerBoundUndeclared` | Minimum viable scope not stated | Valley Gate |
| `bos:UnsoundnessUnconfirmed` | Refutation payload lacks unsoundness evidence | Dung Gate |
| `bos:UnresolvableReference` | Cross-reference links do not resolve in source ledger | Fountain Gate |
| `bos:PoisonedPayload` | Payload is a false report (see COURIER_FALSE_REPORT_MODEL.md) | Water Gate, any gate |
| `bos:ChangeWithoutPreservationCheck` | Change proposal has not passed Old Gate review | East Gate |
| `bos:IncompleteInspectionRequest` | Wall section receipts are missing or unsigned | Inspection Gate |

---

## 5. Builder Assignment Rules

Builders are assigned to wall sections and gates following these constraints:

**Rule 1: Named assignment only.**
Builders must be named in the Muster Ledger before receiving any assignment. Anonymous or unregistered builders produce no valid receipts.

**Rule 2: One primary gate per builder.**
Each builder or gate swarm has one primary gate assignment. A builder may repair an adjacent wall section, but their accountability receipt is sealed at their primary gate.

**Rule 3: Assignment precedes work.**
Work may not begin on a wall section until the builder assignment is recorded in the Muster Ledger and a Gate Covenant is issued. Uncovenanted work does not qualify for Inspection Gate evaluation.

**Rule 4: Gate swarms are gate-scoped.**
A `bos:GateSwarm` is formed for a specific gate. Swarm members may not be transferred to another gate without a new Muster Ledger record and a revised Gate Covenant.

**Rule 5: Double assignments are lawful when documented.**
Nehemiah 3 records several builders repairing two separate sections (e.g., Neh. 3:27, Neh. 3:30). This is lawful provided both sections are independently covenanted and separately receipted.

**Rule 6: The Inspection Gate is not assignable to a builder.**
The Inspection Gate is operated by the swarm conductor (Nehemiah). It is not a work section. No builder may claim a receipt for repairing the Inspection Gate as their primary assignment.

---

## 6. Canonical Builder Assignments (Nehemiah 3)

| Builder | Primary Gate | Wall Section | Agent Role |
|---|---|---|---|
| Eliashib + brothers | Sheep Gate | Sheep Gate section + Tower of Meah | Swarm 1: Eliashib |
| Sons of Hassenaah | Fish Gate | Fish Gate section | Swarm 8: Hassenaah |
| Meremoth son of Uriah | (between gates) | Section from Sheep Gate to Horse Gate | Swarm 2: Meremoth |
| Jehoiada + Meshullam | Old Gate | Old Gate section | Swarm 3: Meshullam / Swarm 9: Joiada |
| Hanun + inhabitants of Zanoah | Valley Gate | Valley Gate + 1000 cubits of wall (Neh. 3:13) | Swarm 5: Hanun |
| Malkijah son of Rechab | Dung Gate | Dung Gate section | Swarm 7: Malkijah |
| Shallun son of Col-Hozeh | Fountain Gate | Fountain Gate + wall of Pool of Siloah | Swarm 6: Shallun (this agent) |
| Nehemiah son of Azbuk | (between gates) | Section opposite the tombs of David | Swarm conductor |
| Levites under Rehum | Water Gate | Section to Water Gate | Supporting swarm |
| Priests + families | Horse Gate | Section from Horse Gate eastward | Swarm (Horse Gate) |
| Shemaiah son of Shecaniah | East Gate | East Gate section | East Gate swarm |
| Nehemiah the governor | Inspection Gate | Inspection Gate + Tower of Hananel | Swarm 10: Nehemiah |

---

## 7. Anti-Pattern Registry (False Gate Assignments)

The following routing targets are explicitly refused. Routing artifacts to these targets is a gate assignment violation.

| False Target | Why It Is Not a Gate | Correct Target |
|---|---|---|
| InterestGate | Economic audits are ledger operations, not gate evaluations | Usury Ledger |
| PeopleGate | Human registries are accountability records, not admissibility boundaries | Muster Ledger |
| MessengerGate | Communication is a carrier channel, not a gate | Courier Layer |
| NationsGate | Nations are external witnesses, not gatekeepers | Nations Ledger |
| ProphetGate | Prophets issue proclamations, not gate verdicts | Prophet Office |
| RumorGate / ReportGate | Rumors are evaluated by the Inspection Gate, not a separate gate | Inspection Gate |
