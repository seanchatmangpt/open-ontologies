# Nehemiah 52 Operating Grammar

This document describes the formal operating grammar derived from the historical
reconstruction of the Jerusalem wall as recorded in Nehemiah 3 and the surrounding
narrative (Nehemiah 1–7). The grammar provides a structural framework for coordinating
solution design, accountability boundaries, verification gates, and adversarial response.

Prefix: `bos: <https://open-ontologies.org/bible-o-star#>`

---

## Doctrine

| Concept | Definition |
|---------|-----------|
| Gate | Admissibility boundary — not metaphor, not department |
| Builder | Accountable named worker — anonymity is an anti-pattern |
| Mocker | Adversarial feedback source |
| Courier | Carrier channel — not a gate |
| FalseReport | Poisoned payload — must be refused, not routed |
| Prayer | Upward appeal — not a routing event |
| Interest | Internal extraction audit (UsuryLedger) — not a gate |
| Nations | External witness field (NationsLedger) — not a gate |
| Census | Muster, not vanity metric |
| Prophet | Proclamation office — not a gate |

---

## 1. The 10 Sanctioned Gates

Exactly 10 gate instances are sanctioned. No additional gates may be introduced.

| Gate | Canonical Reference | Operational Role |
|------|---------------------|-----------------|
| `bos:SheepGate` | Neh.3.1 | Consecrated entry point; high-priest dedication |
| `bos:FishGate` | Neh.3.3 | External intake and supply sorting |
| `bos:OldGate` | Neh.3.6 | Foundational standards; canonical precedents |
| `bos:ValleyGate` | Neh.3.13 | Lower-bound transitions; humility boundary |
| `bos:DungGate` | Neh.3.14 | Validation refutation; purging of unsound structures |
| `bos:FountainGate` | Neh.3.15 | Cross-reference flow; coordination routing |
| `bos:WaterGate` | Neh.3.26 | Truth distribution; public proclamation coordination |
| `bos:HorseGate` | Neh.3.28 | Dynamic defense; rapid response |
| `bos:EastGate` | Neh.3.29 | Governance; change management; return alignment |
| `bos:InspectionGate` | Neh.3.31 | Final validation; ALIVE/PARTIAL/BLOCKED verdicts |

`bos:InspectionGate` is the sole instance of the `bos:InspectionGate` class
(which is a subclass of `bos:Gate`). It emits `bos:InspectionReceipt` individuals
bearing `bos:Verdict` values.

---

## 2. Non-Gate Offices and Layers

These are operational structures that are NOT gates. They carry out specific
functions but must not be promoted to gate status.

| Individual | Class | Doctrine Note |
|-----------|-------|--------------|
| `bos:PrayerLayer` | `bos:Prayer` | Systemic upward-appeal layer; converts adversarial signals to appeals |
| `bos:CourierLayer` | `bos:Courier` | Systemic carrier layer routing payloads to admissibility gates |
| `bos:Mockers` | `bos:Mocker` | Collective adversarial feedback group (Sanballat, Tobiah, Geshem) |
| `bos:UsuryLedger` | `bos:UsuryLedger` | Internal extraction audit; prohibits oppressive interest |
| `bos:MusterLedger` | `bos:MusterLedger` | Census = muster, not vanity metric |
| `bos:ProphetOffice` | `bos:ProphetOffice` | Proclamation counsel; not a gate |
| `bos:NationsLedger` | `bos:NationsLedger` | External witness field; nations observe, they do not admit |

---

## 3. Scripture Classes

| Class | Description |
|-------|-------------|
| `bos:ScriptureWork` | Canonical scripture work (OSIS osisIDWork) |
| `bos:Book` | Canonical book (OSIS three-letter abbreviation) |
| `bos:Chapter` | Chapter division (e.g. Neh.3) |
| `bos:Verse` | Verse address (e.g. Neh.3.1) |
| `bos:Passage` | Named passage spanning one or more verses |
| `bos:Person` | Named human or divine agent |
| `bos:PeopleGroup` | Nation, tribe, or named collective |
| `bos:Place` | Geographic location or structure |
| `bos:Pericope` | Discrete coherent narrative or teaching unit |

---

## 4. Core Operational Classes

| Class | Description |
|-------|-------------|
| `bos:Gate` | Admissibility boundary (exactly 10 instances) |
| `bos:Wall` | Complete protective boundary structure |
| `bos:WallSection` | Bounded work segment assigned to a builder |
| `bos:Builder` | Accountable named worker |
| `bos:GateSwarm` | Coordinated cohort assigned to a gate |
| `bos:Courier` | Communication carrier channel |
| `bos:CourierRecord` | Carrier transmission log |
| `bos:FalseReport` | Adversarial poisoned payload |
| `bos:Mocker` | Adversarial feedback agent |
| `bos:MockerFeedback` | Adversarial feedback record |
| `bos:Prayer` | Upward appeal |
| `bos:UsuryLedger` | Internal extraction audit ledger |
| `bos:UsuryLedgerRecord` | Single extraction audit record |
| `bos:MusterLedger` | Builder accountability register |
| `bos:MusterLedgerRecord` | Single muster accountability record |
| `bos:ProphetOffice` | Proclamation office |
| `bos:PropheticProclamation` | Declaration emitted by the Prophet Office |
| `bos:NationsLedger` | External witness observation ledger |
| `bos:NationsLedgerRecord` | Single external observation record |
| `bos:InspectionGate` | subClassOf Gate; the final validation boundary class |
| `bos:InspectionReceipt` | Verification receipt from the InspectionGate |
| `bos:Verdict` | ALIVE / PARTIAL / BLOCKED status assessment |
| `bos:Receipt` | Durable proof of a building act or transition |

---

## 5. Required Properties

### Scripture Properties

| Property | Type | Description |
|----------|------|-------------|
| `bos:hasBook` | ObjectProperty | ScriptureWork → Book |
| `bos:hasChapter` | ObjectProperty | Book → Chapter |
| `bos:hasVerse` | ObjectProperty | Chapter → Verse |
| `bos:hasPassage` | ObjectProperty | ScriptureWork → Passage |
| `bos:hasCanonicalReference` | DatatypeProperty | OSIS canonical reference string |
| `bos:mentionsPerson` | ObjectProperty | passage/verse → Person |
| `bos:mentionsPlace` | ObjectProperty | passage/verse → Place |
| `bos:hasCrossReference` | ObjectProperty | Evidence-level passage link |

### Operational Properties

| Property | Type | Description |
|----------|------|-------------|
| `bos:assignedToGate` | ObjectProperty | Builder/Swarm/Section → Gate |
| `bos:buildsWallSection` | ObjectProperty | Builder/Swarm → WallSection |
| `bos:repairsGate` | ObjectProperty | Builder/Swarm → Gate |
| `bos:hasBuilder` | ObjectProperty | Swarm/Section → Builder |
| `bos:hasGateSwarm` | ObjectProperty | Gate → GateSwarm |
| `bos:hasWallSection` | ObjectProperty | Gate/Wall → WallSection |
| `bos:hasVerdict` | ObjectProperty | InspectionReceipt/Gate → Verdict |
| `bos:hasReceipt` | ObjectProperty | artifact → Receipt |
| `bos:hasSource` | ObjectProperty | component → provenance |
| `bos:hasPrayer` | ObjectProperty | action → Prayer |
| `bos:hasProclamation` | ObjectProperty | event → PropheticProclamation |
| `bos:hasMusterRecord` | ObjectProperty | gate/swarm → MusterLedgerRecord |
| `bos:hasUsuryAudit` | ObjectProperty | community → UsuryLedgerRecord |
| `bos:hasNationsSignal` | ObjectProperty | timeline → NationsLedgerRecord |
| `bos:hasCourierRecord` | ObjectProperty | event → CourierRecord |
| `bos:hasMockerFeedback` | ObjectProperty | builder/gate → MockerFeedback |
| `bos:classifiesPayload` | DatatypeProperty | payload classification string |
| `bos:routesToGate` | ObjectProperty | CourierRecord → Gate |
| `bos:extractsSignal` | DatatypeProperty | extracted signal description |
| `bos:refusesPoison` | DatatypeProperty | xsd:boolean — false report refused |
| `bos:requiresRepair` | DatatypeProperty | xsd:boolean — gate/section in disrepair |
| `bos:witnessesCompletion` | DatatypeProperty | xsd:boolean — external witness of completion |

---

## 6. Refused Fake Gates

The following are explicitly refused as gates. Each is declared as
`owl:NamedIndividual` (NOT `owl:Class`) with `rdfs:comment "REFUSED: not a gate"`
and `owl:deprecated true`.

| Refused Individual | Reason |
|-------------------|--------|
| `bos:InterestGate` | Economic checks belong in UsuryLedger |
| `bos:PeopleGate` | Human registries belong in MusterLedger |
| `bos:MessengerGate` | Communication belongs in CourierLayer |
| `bos:NationsGate` | Nations are external witnesses, not gatekeepers |
| `bos:ProphetGate` | Prophets are proclamation counsel, not gates |
| `bos:RumorGate` | Rumors are FalseReports refused at InspectionGate |
| `bos:ReportGate` | Reports are processed at InspectionGate already |

**Rule:** Promoting a ledger, layer, or office to gate status collapses the
admissibility/function distinction that makes the grammar coherent. When in doubt,
ask: "Does this thing decide what enters the city?" If no, it is not a gate.

---

## 7. Adversarial Response Pattern

Nehemiah's response to adversarial pressure is encoded in four steps:

1. `bos:hasMockerFeedback` — record the adversarial signal
2. `bos:hasPrayer` — submit an upward appeal
3. `bos:refusesPoison true` — mark the FalseReport as refused
4. `bos:hasReceipt` — emit a Receipt proving the refusal

No adversarial feedback routes directly into the building swarm. All adversarial
payloads are classified as `bos:FalseReport`, routed via `bos:CourierLayer`, and
refused at the nearest gate boundary.

---

## 8. Inspection Gate Protocol

The `bos:InspectionGate` individual is the terminal validation boundary. It:

1. Receives a `bos:CourierRecord` from the building swarm
2. Evaluates wall integrity across all 10 sections
3. Emits a `bos:InspectionReceipt` bearing a `bos:Verdict`
4. Verdict values: `ALIVE` (fully operational), `PARTIAL` (in progress), `BLOCKED`

No wall section or gate boundary may declare itself ALIVE. All ALIVE verdicts must
originate from `bos:InspectionGate`.

---

## 9. Gate-by-Gate Operating Doctrine

Each gate has a distinct operating doctrine. Routing an artifact to the wrong gate
is a conformance violation, not a shortcut. The following sections document, for
each gate: the canonical reference, the responsible builder(s), the wall section
covered, admission conditions, refusal conditions, and the false-report interception
role (if any).

---

### Gate 1 — Sheep Gate (`bos:SheepGate`)

**Canonical reference:** Neh. 3:1
**Builder(s):** Eliashib the high priest and his fellow priests
**Wall section:** Sheep Gate structure + Tower of the Hundred + Tower of Hananel (Neh. 3:1)
**Facing:** North

**Operating role:** Consecrated entry point. The Sheep Gate is rebuilt first and
consecrated (dedicated) before all other work begins. It governs new-work lifecycle
initiation: before any builder may begin their section, the initiating work unit must
carry a valid dedication marker traceable to the Sheep Gate consecration.

**Admission conditions:**
- Artifact carries a valid dedication marker (`bos:DedicationMarker`) issued at or
  after the Sheep Gate consecration event.
- The initiating builder is registered in the Muster Ledger with a Sheep Gate
  Gate Covenant.
- No prior refusal for `bos:DedicationMissing` is outstanding against the artifact.

**Refusal conditions:**
- `bos:DedicationMissing` — Work declaration lacks a dedication marker.
- `bos:BuilderNotMustered` — The initiating builder is absent from the Muster Ledger.
- `bos:PrematureWork` — Work is declared before the Sheep Gate consecration receipt exists.

**False-report interception:** The Sheep Gate evaluates dedication claims. Any payload
claiming that work was "already dedicated" without a traceable receipt is refused as
a `bos:FabricatedDedication` false report. Eliashib, as high priest, holds the
canonical dedication authority; no other party may issue a valid dedication marker.

---

### Gate 2 — Fish Gate (`bos:FishGate`)

**Canonical reference:** Neh. 3:3
**Builder(s):** Sons of Hassenaah; also Meremoth son of Uriah received adjacent sections
**Wall section:** Fish Gate structure (Neh. 3:3-5); Meremoth's two adjacent sections
  (Neh. 3:4, 3:21) run alongside this boundary
**Facing:** North

**Operating role:** External intake and supply sorting. The Fish Gate faces outward
toward trade and supply routes. It is the canonical entry point for external data
streams, externally-sourced artifacts, and cross-boundary categorization requests.

**Admission conditions:**
- Artifact origin is external and verifiable (origin identity present in payload header).
- Payload conforms to the categorization schema declared at this gate.
- No `bos:ExternalOriginUnverified` flag is outstanding.

**Refusal conditions:**
- `bos:ExternalOriginUnverified` — External payload origin cannot be confirmed.
- `bos:CategorizationSchemaMismatch` — Payload does not conform to the external intake schema.
- `bos:UndeclaredExternalSource` — Payload carries no source ledger entry.

**False-report interception:** External payloads are structurally the highest-risk
vector for false reports (Sanballat and Tobiah operated from outside the wall). The
Fish Gate applies a `bos:SourceLedgerCheck` before admission. Any payload asserting
an external fact that has no corroboration in the Nations Ledger or Source Ledger is
flagged `bos:UnverifiedExternalClaim` and refused.

---

### Gate 3 — Old Gate (`bos:OldGate`)

**Canonical reference:** Neh. 3:6
**Builder(s):** Joiada son of Paseah and Meshullam son of Besodeiah
**Wall section:** Old Gate structure (Neh. 3:6); associated with the Broad Wall section
  (Neh. 3:8, attributed to Uzziel son of Harhaiah and Hananiah son of the perfumers)
**Facing:** Northwest (also called the Jeshanah Gate)

**Operating role:** Foundational standards preservation. The Old Gate maintains
canonical precedents. Any artifact asserting a change to a foundational standard, or
invoking a prior law as authority, must pass through the Old Gate before proceeding
to the East Gate for change admission.

**Admission conditions:**
- Artifact does not violate a prior foundational law held in the Old Gate canonical archive.
- If the artifact invokes a prior law as authority, the law reference resolves in
  the Old Gate record.
- Change proposals must pass an Old Gate preservation check before East Gate evaluation.

**Refusal conditions:**
- `bos:FoundationalLawViolation` — Artifact violates a prior foundational standard.
- `bos:UnresolvableCanonicalReference` — The prior law cited does not resolve.
- `bos:ChangeWithoutPreservationCheck` — A change proposal bypassed Old Gate review.

**False-report interception:** The Old Gate is the primary interception point for
payloads that falsely invoke foundational authority. "This was always the standard"
or "the prior law permits this" are claims evaluated here. If the claim cannot be
corroborated by the Old Gate canonical archive, it is refused as
`bos:FabricatedCanonicalClaim`.

---

### Gate 4 — Valley Gate (`bos:ValleyGate`)

**Canonical reference:** Neh. 3:13
**Builder(s):** Hanun and the inhabitants of Zanoah
**Wall section:** Valley Gate structure + one thousand cubits of wall to the Dung Gate (Neh. 3:13, Hebrew: aleph ammah = 1000 cubits)
  This is the longest single builder assignment in Nehemiah 3.
**Facing:** West (lowest elevation point in the wall circuit)

**Operating role:** Lower-bound constraint handling. The Valley Gate sits at the
lowest elevation of the wall. It governs minimum viable scope claims: any artifact
that asserts a lower-bound constraint (minimum acceptable state, baseline requirement,
floor condition) must be evaluated here. This agent is the Valley Gate operator.

**Admission conditions:**
- Artifact represents a minimum viable scope claim with a stated, measurable lower bound.
- The lower bound is declared (`bos:LowerBoundDeclared = true`).
- The one-thousand-cubit rule: the scope claim covers at least the declared minimum extent (Neh. 3:13).

**Refusal conditions:**
- `bos:LowerBoundUndeclared` — Minimum viable scope not stated or not measurable.
- `bos:ScopeBelowFloor` — Scope claim falls below the declared minimum viable extent.
- `bos:UndocumentedShrinkage` — Scope was reduced after admission without a revised covenant.

**False-report interception:** The Valley Gate intercepts fear payloads designed to
compress scope below what is structurally sound. "You cannot build all one thousand cubits;
reduce your scope" is a fear payload unless it carries a valid structural assessment
from the Inspection Gate. Hanun's one-thousand-cubit covenant is the counter-receipt: any
payload attempting to shrink the scope below that covenant is refused as
`bos:FearPayload`.

---

### Gate 5 — Dung Gate (`bos:DungGate`)

**Canonical reference:** Neh. 3:14
**Builder(s):** Malkijah son of Rechab, ruler of Beth-hakkerem district
**Wall section:** Dung Gate structure (Neh. 3:14)
**Facing:** South (waste removal direction)

**Operating role:** Validation refutation and purging. The Dung Gate is the canonical
removal boundary. Unsound artifacts — those that have failed structural validation —
are processed here. The gate does not build; it removes. It is the only gate whose
primary admission criterion is that the artifact is confirmed unsound.

**Admission conditions:**
- Artifact is confirmed unsound and correctly typed as `bos:UnsoundArtifact`.
- The unsoundness evidence is present and references a named structural law.
- The referring gate (whichever gate identified the unsoundness) has issued a
  `bos:UnsoundnessReceipt`.

**Refusal conditions:**
- `bos:UnsoundnessUnconfirmed` — Refutation payload lacks unsoundness evidence.
- `bos:MisnamedUnsoundness` — The artifact is typed as `bos:UnsoundArtifact` but
  no named structural law is cited.
- `bos:SoundArtifactMisrouted` — A sound artifact was sent to the Dung Gate in error;
  this is refused and rerouted.

**False-report interception:** Adversaries may attempt to route sound work to the
Dung Gate by fabricating unsoundness claims ("their wall is so weak a fox would
break it through" — Neh. 4:3, Tobiah's fox test). Malkijah's gate evaluates every
claim against the `bos:UnsoundnessReceipt` requirement. A payload asserting
unsoundness without a receipt is refused as `bos:FabricatedUnsoundnessClaim`.

---

### Gate 6 — Fountain Gate (`bos:FountainGate`)

**Canonical reference:** Neh. 3:15
**Builder(s):** Shallun son of Col-Hozeh, ruler of the Mizpah district
**Wall section:** Fountain Gate structure + wall of the Pool of Siloah (Shelah) +
  wall to the stairs descending from the City of David (Neh. 3:15). This agent.
**Facing:** South

**Operating role:** Cross-reference flow and coordination routing. The Fountain Gate
sits adjacent to the Pool of Siloah, a primary water source. It governs reference
resolution: cross-reference links, source ledger entries, and coordination routing
payloads must pass through this gate. An unresolvable reference that reaches any
other gate is escalated back to the Fountain Gate.

**Admission conditions:**
- Cross-reference links are resolvable; source ledger entry exists for every reference.
- Coordination routing payload has a valid target gate declaration.
- All referenced artifacts are present in the receipt archive.

**Refusal conditions:**
- `bos:UnresolvableReference` — Cross-reference links do not resolve in the source ledger.
- `bos:OrphanedCrossReference` — A reference points to an artifact with no receipt.
- `bos:CircularReference` — A reference chain loops back to itself without a grounding receipt.

**False-report interception:** Poisoned cross-reference chains are a structural attack
vector: fabricate a reference to an authority that does not exist, and route payloads
through it. The Fountain Gate breaks the chain by requiring every reference to resolve
against the source ledger. A cross-reference that resolves to a non-existent or
unreceipted artifact is refused as `bos:PoisonedReference`.

---

### Gate 7 — Water Gate (`bos:WaterGate`)

**Canonical reference:** Neh. 3:26
**Builder(s):** Nethinim (temple servants) living in Ophel; also Zadok son of Immer
  (Neh. 3:29) worked in the adjacent east section
**Wall section:** Nethinim section opposite the Water Gate, as far as the projecting tower
  (Neh. 3:26-27); also Tekoites' second section (Neh. 3:27)
**Facing:** East (toward the Kidron Valley)

**Operating role:** Truth distribution and public proclamation coordination. The Water
Gate is the canonical broadcast point. Ezra read the Law to the assembly at the Water
Gate (Neh. 8:1). Payloads routed to the Water Gate are declared authoritative for
distribution. Only verified-true payloads may pass through here.

**Admission conditions:**
- Payload is verified true against the receipt archive.
- Distribution is non-poisoned (no `bos:PoisonedPayload` flag outstanding).
- The broadcasting entity is registered and holds a valid proclamation authority.

**Refusal conditions:**
- `bos:PoisonedPayload` — Payload contains false or unverified content.
- `bos:UnauthorizedBroadcast` — Broadcasting entity lacks proclamation authority.
- `bos:PreReviewDistribution` — Payload was distributed before Inspection Gate review
  (the open-letter pattern).

**False-report interception:** The open-letter pattern (Neh. 6:5-7) is the canonical
Water Gate attack: distribute a false accusation to the Nations before review, forcing
the builder to respond to a public lie. The Water Gate identifies `bos:OpenLetterPattern`
on any payload whose Nations Ledger distribution timestamp precedes its Inspection
Gate review timestamp, and refuses it as `bos:PoisonedPayload`.

---

### Gate 8 — Horse Gate (`bos:HorseGate`)

**Canonical reference:** Neh. 3:28
**Builder(s):** Priests, each repairing opposite their own house (Neh. 3:28)
**Wall section:** Section from the Horse Gate eastward (Neh. 3:28); also Zadok son
  of Immer (Neh. 3:29) repaired the section opposite his house in this area
**Facing:** East

**Operating role:** Dynamic defense and rapid response. The Horse Gate historically
served as a military sally port — a high-clearance gate for mounted patrols and rapid
sortie. In the operating grammar, it governs defense signals and adversarial threat
responses. Genuine defense signals are admitted here; false alarms are refused.

**Admission conditions:**
- Defense signal is genuine: adversarial threat is classified with a named threat type.
- Payload is not a false report (corroboration check required).
- The threat classification references a specific, observable adversarial action.

**Refusal conditions:**
- `bos:UnclassifiedThreat` — Defense signal carries no named threat type.
- `bos:FalseAlarm` — Threat claim cannot be corroborated by observable evidence.
- `bos:PanicPayload` — Payload is a fear signal designed to trigger defensive halt
  rather than a genuine threat response.

**False-report interception:** Fear payloads often masquerade as legitimate defense
signals. "They are coming to kill you; stop building" (Neh. 6:10, the Shemaiah trap)
is a `bos:FearPayload` disguised as a protective warning. The Horse Gate evaluates
defense signals against the corroboration requirement: if the claimed threat has no
observable evidence in the Nations Ledger or receipt archive, it is refused as
`bos:FabricatedThreat`.

---

### Gate 9 — East Gate (`bos:EastGate`)

**Canonical reference:** Neh. 3:29
**Builder(s):** Shemaiah son of Shecaniah, keeper of the East Gate
**Wall section:** East Gate section (Neh. 3:29)
**Facing:** East (sunrise-facing)

**Operating role:** Governance, change management, and return alignment. The East Gate
faces the sunrise — the direction of return from exile. It governs change proposals and
transition requests. No change to a sealed Gate Covenant or wall section assignment
may proceed without East Gate review.

**Admission conditions:**
- Change proposal carries a valid Gate Covenant draft.
- Old Gate preservation check has passed (no `bos:FoundationalLawViolation` outstanding).
- The proposed change is forward-facing (does not retroactively alter sealed receipts).

**Refusal conditions:**
- `bos:ChangeWithoutPreservationCheck` — Change bypassed Old Gate review.
- `bos:RetroactiveReceiptAlteration` — Change attempts to modify a sealed receipt.
- `bos:UngroundedTransition` — Transition has no traceable return alignment.

**False-report interception:** Fabricated change proposals are a governance attack:
claim that a change was already approved, force the builder to respond to a fait
accompli. The East Gate requires a valid Gate Covenant draft for every change proposal.
A change claim without a draft is refused as `bos:FabricatedChangeApproval`.

---

### Gate 10 — Inspection Gate (`bos:InspectionGate`)

**Canonical reference:** Neh. 3:31
**Builder(s):** Nehemiah the governor (swarm conductor; not a section builder)
**Wall section:** Section from the house of Malkijah (goldsmiths' and merchants' section)
  to the Inspection Gate, including the upper room at the corner (Neh. 3:31-32)
**Facing:** East

**Operating role:** Final validation. The Inspection Gate is the terminal boundary.
It emits `bos:InspectionReceipt` individuals bearing `bos:Verdict` values
(ALIVE / PARTIAL / BLOCKED). No wall section or gate boundary may self-certify as
ALIVE. All ALIVE verdicts originate from and are sealed by this gate alone.

**Admission conditions:**
- Inspection request covers a complete, named wall section.
- All required receipts for the section are present, non-forged, and traceable to
  named builders.
- No outstanding `bos:FalseReport` or `bos:OpenLetterPattern` flags are unresolved.
- The Muster Ledger contains a complete record of all builders on the section.

**Refusal conditions:**
- `bos:IncompleteInspectionRequest` — Wall section receipts are missing or unsigned.
- `bos:UnnamedBuilder` — A builder on the section has no Muster Ledger record.
- `bos:UnresolvedFalseReport` — An outstanding false report against the section has
  not been refused and logged.
- `bos:SelfCertifiedALIVE` — A wall section or gate has declared itself ALIVE without
  Inspection Gate review; this is itself a defect.

**False-report interception:** All unresolved false reports — regardless of which gate
originally encountered them — are escalated to the Inspection Gate before a verdict is
emitted. An ALIVE verdict cannot be issued while any `bos:FalseReport` against the
section remains without a `bos:refusesPoison = true` record. Nehemiah's role is not
to be deceived by a clean-looking count of completed sections; he checks the receipt
archive and the false-report log before sealing any verdict.

---

## 10. Wall Section to Gate Mapping

The wall of Jerusalem circuits the city. Each section of wall is the bounded work unit
of a named builder or swarm. The mapping below cross-references Nehemiah 3 verse
ranges to the nearest gate boundary and the accountable builder.

| Wall Section | Neh. 3 Reference | Nearest Gate | Builder(s) |
|---|---|---|---|
| Sheep Gate structure + towers | 3:1 | Sheep Gate | Eliashib + priests |
| Section: Fish Gate to Old Gate (north) | 3:3-5 | Fish Gate | Sons of Hassenaah; Meremoth (two sections, 3:4 + 3:21) |
| Old Gate + Broad Wall | 3:6-8 | Old Gate | Joiada + Meshullam; Uzziel + Hananiah |
| Section: Broad Wall to Valley Gate | 3:9-12 | Old Gate / Valley Gate boundary | Rephaiah son of Hur; Jedaiah; Hattush; Malkijah son of Harim; Hashub |
| Valley Gate + 1000-cubit section to Dung Gate (Neh. 3:13) | 3:13 | Valley Gate | Hanun + inhabitants of Zanoah |
| Dung Gate structure | 3:14 | Dung Gate | Malkijah son of Rechab |
| Fountain Gate + Pool of Siloah wall | 3:15 | Fountain Gate | Shallun son of Col-Hozeh |
| Section: Fountain Gate to Water Gate | 3:16-25 | Fountain Gate / Water Gate boundary | Nehemiah son of Azbuk; Levites under Rehum and Hashabiah; Binnui; Ezer; Baruch; Meremoth (second section) |
| Water Gate section (Nethinim + Tekoites) | 3:26-27 | Water Gate | Nethinim (Ophel); Tekoites (second section) |
| Horse Gate section (priests) | 3:28 | Horse Gate | Priests opposite their houses |
| East Gate section | 3:29 | East Gate | Shemaiah son of Shecaniah; Hananiah son of Shelemiah; Hanun (sixth section) |
| Inspection Gate section (goldsmiths + merchants) | 3:31-32 | Inspection Gate | Malkijah the goldsmith; goldsmiths + merchants |

---

## 11. False Report Interception Points — Summary

False reports are refused at the gate closest to their structural attack vector, not
at a single central filter. The following table summarizes which gate intercepts which
type of false report.

| False Report Type | Primary Interception Gate | Named Refusal Law |
|---|---|---|
| Fabricated dedication claim | Sheep Gate | `bos:FabricatedDedication` |
| Unverified external claim | Fish Gate | `bos:UnverifiedExternalClaim` |
| Fabricated canonical authority | Old Gate | `bos:FabricatedCanonicalClaim` |
| Fear payload / scope compression | Valley Gate | `bos:FearPayload` |
| Fabricated unsoundness claim | Dung Gate | `bos:FabricatedUnsoundnessClaim` |
| Poisoned cross-reference chain | Fountain Gate | `bos:PoisonedReference` |
| Open letter (pre-review broadcast) | Water Gate | `bos:OpenLetterPattern` |
| Fabricated threat / fear disguised as defense | Horse Gate | `bos:FabricatedThreat` |
| Fabricated change approval | East Gate | `bos:FabricatedChangeApproval` |
| Any unresolved false report (ALIVE block) | Inspection Gate | `bos:UnresolvedFalseReport` |

**Rule:** No false report passes through a gate unlogged. Every interception produces a
`bos:FalseReportRefusal` record with a named law. The Inspection Gate will not emit an
ALIVE verdict while any false report log entry lacks a `bos:refusesPoison = true`
annotation.

---

## 12. Builder Accountability Model

Each builder in the Muster Ledger is accountable by name. The accountability model has
three binding elements:

1. **Named assignment.** The builder's name, gate assignment, and wall section are
   recorded in the Muster Ledger before work begins. Anonymous work produces no valid
   receipts.

2. **Covenanted scope.** A Gate Covenant is issued for each assignment, declaring the
   exact extent of the wall section (in cubits where Nehemiah 3 records them) and the
   admission gate. Work that exceeds the covenanted scope without a revised covenant is
   not receipted.

3. **Traceable receipt chain.** Every building act — not just completion — produces a
   `bos:Receipt` traceable to the builder's Muster Ledger entry. The chain from raw
   stone to ALIVE verdict must have no unattributed links.

**Failure modes that break accountability:**
- Builder completes work but is not in the Muster Ledger → receipts are void.
- Scope is expanded without a revised covenant → excess work is unreceipted.
- Builder is present in the Muster Ledger but completes no work → absence is recorded
  (cf. Neh. 3:5, the nobles of Tekoa who "did not put their necks to the work").
- Receipts are emitted from outside the covenanted gate → receipt is refused as
  `bos:UncovenantedReceipt`.

The accountability model closes the loop: every ALIVE verdict from the Inspection Gate
can be traced back through the receipt chain to a named builder at a named gate in a
named wall section. If that trace breaks, the verdict is PARTIAL or BLOCKED.

---

## Source Ontology

File: `ontology/nehemiah-52.ttl`
Prefix: `@prefix bos: <https://open-ontologies.org/bible-o-star#>`
License: CC0 1.0
