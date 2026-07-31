# TOGAF Mapping for Nehemiah 52 Operating Grammar

## Overview
This document maps the Nehemiah 52 (Bible O*) operating grammar to the TOGAF Enterprise Architecture framework. Nehemiah 52 provides the semantic operating model (the "grammar"), while TOGAF provides the enterprise architecture lifecycle shell.

## Mapping Table

| TOGAF Concept | Nehemiah 52 Object | Description / Rationale |
|---------------|-------------------|--------------------------|
| **ADM (Architecture Development Method)** | `bos:Wall` Construction | The ADM represents the continuous cycle of building, repairing, and maintaining the structural integrity of the enterprise architecture. |
| **Architecture Building Blocks (ABB)** | `bos:Gate`, `bos:Wall`, `bos:ProphetOffice` | Logical definitions of admissibility boundaries, containment, and state declaration mechanisms. |
| **Solution Building Blocks (SBB)** | `bos:Builder`, `bos:Courier`, `bos:Receipt` | The physical implementations: accountable workers (agents), message carriers, and cryptographic proofs. |
| **Architecture Board** | `bos:ProphetOffice`, `bos:MusterLedger` | The governing authority that declares the "law" and maintains the census of authorized builders. |
| **Contracts** | `bos:InspectionVerdict`, `bos:WallSection` | Compliance results and assigned scopes of work that define the "terms" of architecture implementation. |
| **Stakeholders** | `bos:Builder`, `bos:NationsLedger`, `bos:Mocker` | Internal builders, external witnesses (the Nations), and adversarial stakeholders who provide feedback through testing. |
| **Communications** | `bos:Courier`, `bos:Prayer`, `bos:ProphetOffice` | Data transport, upward telemetry/appeal, and system-wide state broadcasts. |
| **Governance** | `bos:InspectionVerdict`, `bos:UsuryLedger`, `bos:MusterLedger` | Validation of integrity, resource extraction auditing, and identity/authorization management. |
| **Change Management** | `bos:WallSection` Maintenance | The process of identifying segments in need of repair and assigning builders to perform surgical updates. |

---

## Per-ADM-Phase Mapping

The TOGAF ADM defines a set of iterative phases (Preliminary through H) governing the full enterprise architecture lifecycle. Each phase maps to a specific Nehemiah 52 structure, gate, or operational layer. The mappings below are interpretive — structurally analogous to TOGAF ADM phases but not certified by The Open Group. TOGAF is a registered trademark of The Open Group. This mapping is not endorsed or certified by The Open Group and does not imply conformance with any TOGAF standard.

---

### Preliminary Phase — Dung Gate (`bos:DungGate`)

**TOGAF purpose:** Establish architecture capability; define governance framework; remove legacy impediments before work begins.

**Nehemiah 52 mapping:** `bos:DungGate` (Neh. 3:14, Malkijah's boundary)

The Dung Gate is the purge boundary. Before the wall can be built, unsound material must be cleared. In TOGAF terms, the Preliminary Phase requires removing architectural debt, expired constraints, and legacy governance artifacts that would contaminate the new cycle. No phase may begin until the Dung Gate has processed its intake.

**Artifacts:**
- `bos:UnsoundArtifact` — flagged legacy structures that fail the purge evaluation
- `bos:DungGate` admission record — proof that the workspace was cleared
- `bos:CourierRecord` with `bos:UnroutablePayload` flags — identifies artifacts that could not be routed to any valid gate

**Operational law:** The Dung Gate does not route; it refuses. Artifacts that enter the Preliminary Phase and are confirmed unsound receive a `bos:DungGate` refusal receipt, not a construction assignment. Attempting to build on uncleared ground is a conformance violation.

---

### Phase A — Architecture Vision (`bos:InspectionGate` charter + `bos:PrayerLayer`)

**TOGAF purpose:** Define the scope, constraints, and high-level vision for the architecture engagement. Obtain approval to proceed. Establish the Statement of Architecture Work.

**Nehemiah 52 mapping:** `bos:InspectionGate` charter issuance + `bos:PrayerLayer` activation

The Architecture Vision phase corresponds to the moment Nehemiah surveys the ruined wall at night (Neh. 2:11–16) before declaring the vision to the people. The vision is not a plan — it is a sanctioned scope declaration backed by prayer (upward appeal) and sealed with an inspection charter.

The `bos:InspectionGate` in Phase A does not yet emit a verdict receipt. It issues a **charter** — a pre-commitment that defines what a passing inspection will require. The `bos:PrayerLayer` is activated simultaneously: adversarial signals (Sanballat's mockery; `bos:Mockers`) are converted to upward appeals rather than routed as threats.

**Artifacts:**
- `bos:InspectionGate` charter (`bos:hasVerdictCriteria`) — defines ALIVE/PARTIAL/BLOCKED criteria for this cycle
- `bos:PrayerLayer` activation record — logs that adversarial input is being processed as appeal
- `bos:PropheticProclamation` from `bos:ProphetOffice` — declares the vision statement

**Operational law:** A vision statement that has not been countersigned by the Inspection Gate charter is not admissible. The `bos:SheepGate` (Neh. 3:1, consecrated entry) governs the formal opening of new work lifecycles that follow from the vision.

---

### Phase B — Business Architecture (`bos:MusterLedger` + `bos:BuilderRegistry`)

**TOGAF purpose:** Develop the baseline and target Business Architecture. Define business processes, organizational structures, and capability gaps.

**Nehemiah 52 mapping:** `bos:MusterLedger` + `bos:Builder` registry enumeration

Phase B is the muster. TOGAF asks: who does what, in what organizational unit, under which capability? Nehemiah 52 answers with the `bos:MusterLedger` — the accountability register of named builders and family units (Neh. 3). Each `bos:Builder` instance is an accountable named worker. Anonymity is an anti-pattern; unregistered labor is a conformance violation.

The "Builder Registry" is not a separate class — it is the `bos:MusterLedger` populated with `bos:MusterLedgerRecord` instances. The Business Architecture is complete when every wall section (`bos:WallSection`) has a named builder and every builder appears in the muster ledger.

**Artifacts:**
- `bos:MusterLedger` instance populated with `bos:MusterLedgerRecord` entries
- `bos:WallSection` assignments — each section carries `bos:assignedToGate` and `bos:buildsWallSection` triples linking builders to scope
- `bos:GateSwarm` cohort definitions — coordinated builder cohorts assigned to specific gates

**Operational law:** A Business Architecture that contains anonymous builders (no `rdfs:label`, no muster entry) is incomplete. The `bos:MusterLedger` is the census, not a vanity metric.

---

### Phase C — Information Systems Architecture (`bos:ScriptureWork`, `bos:Book`, `bos:Chapter`, `bos:Verse`)

**TOGAF purpose:** Develop the Data Architecture and Application Architecture. Define the information assets and application portfolio.

**Nehemiah 52 mapping:** Scripture spine — `bos:ScriptureWork`, `bos:Book`, `bos:Chapter`, `bos:Verse`, `bos:Passage`, `bos:Pericope`

Phase C governs information structure. In Bible O*, the canonical information system is the Scripture spine: a hierarchical address space from `bos:ScriptureWork` (the source work, e.g. Westminster Leningrad Codex) down through `bos:Book`, `bos:Chapter`, `bos:Verse`, and `bos:Passage`. This is the Data Architecture — a stable, version-controlled canonical reference system.

The Application Architecture layer maps to `bos:Pericope` (discrete coherent narrative units) and `bos:Person`/`bos:Place`/`bos:PeopleGroup` — the entities that application-level queries resolve against. Cross-references (`bos:hasCrossReference`) are the application-level links between information assets.

**Artifacts:**
- `bos:ScriptureWork` instances — the canonical source works (OSIS osisIDWork references)
- `bos:Book` → `bos:Chapter` → `bos:Verse` hierarchy — the normalized data architecture
- `bos:Passage` and `bos:Pericope` — application-level named units with canonical references (`bos:hasCanonicalReference` via OSIS format strings)
- `bos:Person`, `bos:Place`, `bos:PeopleGroup` — entity resolution targets

**Operational law:** All canonical references use OSIS format (e.g. `Neh.3.1`, `Gen.1.1`). Non-OSIS reference strings are inadmissible. Cross-references are evidence-level links, not doctrinal assertions.

---

### Phase D — Technology Architecture (`bos:CourierLayer` + VectorClock semantics)

**TOGAF purpose:** Develop the Technology Architecture — the hardware, software, and infrastructure substrate. Define the technology capabilities and platform architecture.

**Nehemiah 52 mapping:** `bos:CourierLayer` + `bos:VectorClock` temporal ordering

Phase D maps to the Courier Layer: the carrier infrastructure that routes payloads between gates. The `bos:Courier` class defines the transport channel; `bos:CourierRecord` is the transmission log. In technology architecture terms, the Courier Layer is the message fabric — not a gate, not an admissibility boundary, but the carrier substrate that connects gates.

VectorClock semantics (implied by the ordered construction sequence of Neh. 3, where sections are assigned with directional sequence and temporal ordering) govern the causal ordering of `bos:CourierRecord` events. No two courier records may be out of causal order without a `bos:FalseReport` flag.

**Artifacts:**
- `bos:CourierLayer` instance — the systemic carrier layer
- `bos:CourierRecord` instances — per-transmission logs with source gate, target gate, payload type, and timestamp
- `bos:FalseReport` refusals — records of poisoned payloads detected during transport
- `bos:VectorClock` ordering evidence — causal sequence of courier records

**Operational law:** The Courier Layer routes; it does not admit. A courier that admits or refuses an artifact has exceeded its mandate — the artifact must be forwarded to the target gate for admission evaluation. `bos:FalseReport` payloads must be refused at the Courier Layer before reaching any gate.

---

### Phase E — Opportunities and Solutions (`bos:Gate` admission criteria)

**TOGAF purpose:** Identify the portfolio of projects and initiatives that implement the target architecture. Define the Solution Building Blocks and assess implementation options.

**Nehemiah 52 mapping:** Gate admission criteria — per-gate law definitions

Phase E asks: what specific solutions will realize the target architecture? In Nehemiah 52, this translates to gate admission criteria — the per-gate laws that determine which artifacts are admissible and which are refused. Each gate's admission condition is a specific law, not a general evaluation.

The ten gates each carry a defined admission condition (see GATE_ASSIGNMENT_MODEL.md). Phase E work is the formal specification of these conditions as implementable criteria. The output is a gate covenant — a structured admission law that Solution Building Blocks must satisfy.

**Artifacts:**
- `bos:Gate` admission condition specifications (one per gate, structured as `bos:hasAdmissionCriteria` assertions)
- `bos:SheepGate` dedication marker requirements — the entry-point SBB spec
- `bos:DungGate` unsoundness typology — what constitutes `bos:UnsoundArtifact` for this engagement
- `bos:EastGate` change proposal format — the Gate Covenant draft that change proposals must carry

**Operational law:** Admission criteria must be named laws, not informal guidelines. A gate that admits without evaluating against a named law is a conformance violation. Phase E deliverables are gate covenant drafts — precursors to the architecture contracts formalized in Phases F and G.

---

### Phase F — Migration Planning (`bos:HorseGate` deployment records)

**TOGAF purpose:** Finalize the Architecture Roadmap and Migration Plan. Sequence the implementation projects and transitions.

**Nehemiah 52 mapping:** `bos:HorseGate` (Neh. 3:28) — dynamic defense and deployment sequencing

Phase F maps to the Horse Gate: the dynamic defense boundary responsible for rapid response and active perimeter management. In TOGAF terms, migration planning requires active sequencing — knowing which sections of the wall to repair first based on threat posture and resource availability, not arbitrary scheduling.

The Horse Gate governs deployment records: the ordered sequence of `bos:WallSection` repair assignments that constitute the migration roadmap. Each deployment record is a `bos:CourierRecord` with a phase marker, linking builder assignments to temporal sequence.

**Artifacts:**
- `bos:HorseGate` deployment sequence records — ordered `bos:WallSection` repair assignments with phase markers
- `bos:GateSwarm` deployment cohorts — cohorts assigned to specific migration phases
- Migration risk classification — which sections face adversarial threat (`bos:Mocker` activity) and require Horse Gate prioritization
- `bos:Receipt` instances for completed migration phase transitions

**Operational law:** Migration sequencing must reflect actual threat posture. A migration plan that sequences by convenience rather than defense priority is a Phase F conformance violation. The Horse Gate is the authority for resequencing when adversarial signals (`bos:MockerFeedback`) require it.

---

### Phase G — Implementation Governance (`bos:InspectionGate` receipts)

**TOGAF purpose:** Provide architectural oversight of the implementation. Issue Architecture Contracts. Ensure conformance during construction.

**Nehemiah 52 mapping:** `bos:InspectionGate` receipt issuance — the ALIVE/PARTIAL/BLOCKED verdict surface

Phase G is where the Inspection Gate operates at full authority. Each `bos:WallSection` completion triggers an inspection request. The Inspection Gate evaluates conformance against the charter established in Phase A and emits a `bos:InspectionReceipt` carrying a `bos:Verdict` (ALIVE, PARTIAL, or BLOCKED).

This is the primary receipt-issuing phase. No section is declared complete without an Inspection Gate receipt. The Architecture Contracts of TOGAF correspond to the `bos:InspectionReceipt` instances — formal, sealed proof that a builder fulfilled their wall section assignment.

**Artifacts:**
- `bos:InspectionReceipt` instances — one per `bos:WallSection` evaluation
- `bos:Verdict` values — ALIVE (full conformance), PARTIAL (conditional pass with named gaps), BLOCKED (refusal with named law)
- `bos:InspectionGate` audit log — complete record of all inspection evaluations in this ADM cycle
- Architecture Contract instances — `bos:InspectionReceipt` + `bos:WallSection` + `bos:Builder` triples constituting the formal contract

**Operational law:** A PARTIAL verdict is not a passing grade. It is a time-bounded conditional that requires gap closure before ALIVE can be issued. A BLOCKED verdict triggers an immediate `bos:DungGate` referral — the section is treated as unsound until the named law violation is resolved.

---

### Phase H — Architecture Change Management (`bos:EastGate` + `bos:OldGate`)

**TOGAF purpose:** Manage changes to the architecture in a controlled manner. Monitor for change drivers and determine whether formal ADM re-entry is required.

**Nehemiah 52 mapping:** `bos:EastGate` (forward governance) + `bos:OldGate` (foundational standards preservation)

Phase H is the dual-gate change management surface. The `bos:EastGate` (Neh. 3:29, sunrise-facing) governs forward change admission — all change proposals must pass East Gate evaluation before being admitted to the wall. The `bos:OldGate` (Neh. 3:6, Joiada and Meshullam's boundary) enforces foundational standards preservation — change proposals that violate prior canonical law are refused at the Old Gate regardless of East Gate approval.

This dual-gate structure enforces TOGAF's change management principle: changes must be both forward-viable (East Gate) and backward-compatible with foundational law (Old Gate). A change that passes only one gate is inadmissible.

**Artifacts:**
- `bos:EastGate` change admission records — `bos:CourierRecord` entries for each change proposal evaluated
- `bos:OldGate` preservation check results — foundational law compliance evaluations
- Change driver log — `bos:NationsLedgerRecord` entries recording external signals that triggered change consideration
- ADM re-entry trigger records — `bos:InspectionGate` BLOCKED verdicts that require full ADM cycle restart

**Operational law:** A change proposal that has not passed both `bos:EastGate` and `bos:OldGate` evaluation is inadmissible. There is no fast-path bypass. Emergency changes that skip Old Gate evaluation are Phase H conformance violations and must be retrospectively audited via the `bos:UsuryLedger`.

---

## Detailed Mapping Analysis

### 1. TOGAF ADM & The Wall
The **ADM** is the engine of TOGAF. In Nehemiah 52, this is the act of rebuilding the wall. Each phase of the ADM (Preliminary through H) corresponds to a specific gate, ledger, or operational layer — not a generic "wall section." The phase-to-structure mapping above is normative.

### 2. Architecture & Solution Building Blocks (ABB/SBB)
- **ABB:** These are the "what" without the "how". `bos:Gate` is an ABB defining *that* an admissibility boundary must exist. `bos:Wall` is an ABB defining *that* a containment boundary must exist.
- **SBB:** These are the "how". A specific `bos:Builder` implementation (e.g., a Rust-based agent registered in the `bos:MusterLedger`) is an SBB. A deployed `bos:CourierLayer` instance carrying signed `bos:CourierRecord` payloads is an SBB.

### 3. Governance & The Usury Ledger
TOGAF **Governance** ensures that the architecture is followed. `bos:UsuryLedger` acts as a specific governance tool for monitoring resource consumption and preventing the "usury" of system resources — internal extraction that exceeds what the wall covenant permits. Phase H changes that bypass Old Gate review are logged as usury entries.

### 4. Stakeholders & The Nations Ledger
**Stakeholders** in TOGAF include anyone affected by the architecture. `bos:NationsLedger` represents the "External Witness," capturing the perspective of external stakeholders who observe the system's integrity from the outside. Nations observe; they do not admit. Routing a `bos:NationsLedgerRecord` to a gate for admission evaluation is a routing error.

### 5. Contracts & Inspection Verdicts
An **Architecture Contract** is a joint agreement between development partners and sponsors. In Nehemiah 52, the `bos:InspectionReceipt` is the formal artifact proving that a `bos:Builder` has fulfilled their contract for a specific `bos:WallSection`. A receipt carrying a BLOCKED verdict is a contract breach record, not a passing contract.
