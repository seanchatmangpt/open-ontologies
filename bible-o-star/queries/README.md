# Bible O* SPARQL Query Library

Ontology base: `https://open-ontologies.org/bible-o-star#` (prefix `bos:`)
Source files: `ontology/nehemiah-52.ttl`, `ontology/bible-o-star.ttl`
License: CC BY 4.0

All queries are in `bible-o-star.sparql`. Each query is delimited by a `# query_name` comment block and includes its own PREFIX declarations for standalone executability.

---

## Query Patterns

### `list_all_gates`

Returns the 10 sanctioned `bos:Gate` named individuals with `rdfs:label`, `rdfs:comment`, and `bos:hasCanonicalReference` (OSIS reference string). Results are ordered by canonical reference to match Nehemiah chapter 3 reading order. The `bos:InspectionGate` subclass instance is included because it is a member of `bos:Gate` by subclass inference.

### `list_builders_by_gate`

Joins `bos:Builder` instances to gate assignments via two paths: `bos:assignedToGate` (direct assignment) and `bos:repairsGate` (repair assignment). Both paths are UNIONed so a builder who only has a repair relationship still appears. Results are ordered by gate label then builder label.

### `list_wall_sections`

Returns `bos:WallSection` instances with optional builder (`bos:hasBuilder`) and gate. Gate lookup uses a UNION of the forward property `bos:assignedToGate` on the section and the inverse `bos:hasWallSection` from the gate, covering both assertion directions.

### `find_false_reports`

Returns `bos:FalseReport` instances. The query surfaces `bos:refusesPoison` (boolean), `bos:classifiesPayload` (string payload classification), and joins to any `bos:CourierRecord` that carries a matching classified payload to the routing gate. A FalseReport must be refused, not routed — this query exposes both the payload and any gate it was attempted to be routed through.

### `list_mocker_feedback`

Returns `bos:MockerFeedback` instances with `bos:extractsSignal` (the adversarial signal text) and the source `bos:Mocker` that emitted it via `bos:hasMockerFeedback`. Ordered by mocker label then feedback label for audit grouping.

### `list_inspection_receipts`

Returns `bos:InspectionReceipt` instances with `bos:hasVerdict` (pointing to a `bos:Verdict` individual: ALIVE, PARTIAL, or BLOCKED) and `bos:hasSource` (the originating work or evidence). This is the primary ALIVE gate query pattern.

### `find_cross_references_for_passage`

Returns cross-reference links for `bos:Passage` instances via `bos:hasCrossReference`. To filter to a specific passage, uncomment the `BIND` line and supply the passage IRI. Without binding, the query returns all cross-reference pairs in the loaded graph. Cross-reference is evidence-level only — not a doctrinal relation.

### `list_courier_records`

Returns `bos:CourierRecord` instances joined to their carrier `bos:Courier` (via `bos:hasCourierRecord`) and their routing destination `bos:Gate` (via `bos:routesToGate`). Payload classification (`bos:classifiesPayload`) is included. Courier = carrier channel, not a gate.

### `list_usury_audit_records`

Returns `bos:UsuryLedgerRecord` instances joined to their parent `bos:UsuryLedger` (via `bos:hasUsuryAudit`), optional source (`bos:hasSource`), and optional builder (`bos:hasBuilder`). Interest = internal extraction audit — the Usury Ledger is not a gate. Ordered by ledger then record for audit grouping.

### `list_muster_records`

Returns `bos:MusterLedgerRecord` instances joined to their parent `bos:MusterLedger` (via `bos:hasMusterRecord`) and the builders they enumerate (`bos:hasBuilder`), with gate assignments for those builders. Census = muster, not a vanity metric.

### `find_refused_fake_gates`

Returns all named individuals marked `owl:deprecated true`. These are the 7 refused fake gates: `InterestGate`, `PeopleGate`, `MessengerGate`, `NationsGate`, `ProphetGate`, `RumorGate`, `ReportGate`. None are members of `bos:Gate`. This query is the canonical refusal audit pattern.

### `list_prophetic_proclamations`

Returns `bos:PropheticProclamation` instances with label, comment text, and the `bos:ProphetOffice` that emitted them (via `bos:hasProclamation`). Prophet = proclamation office — not a gate. Ordered by office then proclamation label.

---

## Prefix Reference

| Prefix | IRI |
|--------|-----|
| `bos:` | `https://open-ontologies.org/bible-o-star#` |
| `rdfs:` | `http://www.w3.org/2000/01/rdf-schema#` |
| `owl:` | `http://www.w3.org/2002/07/owl#` |
| `rdf:` | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` |
| `xsd:` | `http://www.w3.org/2001/XMLSchema#` |

## Key Class and Property Reference

| Term | IRI | Kind |
|------|-----|------|
| `bos:Gate` | `…#Gate` | Class |
| `bos:InspectionGate` | `…#InspectionGate` | Class (subClassOf Gate) + NamedIndividual |
| `bos:Builder` | `…#Builder` | Class |
| `bos:WallSection` | `…#WallSection` | Class |
| `bos:GateSwarm` | `…#GateSwarm` | Class |
| `bos:Courier` | `…#Courier` | Class |
| `bos:CourierRecord` | `…#CourierRecord` | Class |
| `bos:FalseReport` | `…#FalseReport` | Class |
| `bos:Mocker` | `…#Mocker` | Class |
| `bos:MockerFeedback` | `…#MockerFeedback` | Class |
| `bos:InspectionReceipt` | `…#InspectionReceipt` | Class |
| `bos:Verdict` | `…#Verdict` | Class |
| `bos:PropheticProclamation` | `…#PropheticProclamation` | Class |
| `bos:UsuryLedger` | `…#UsuryLedger` | Class |
| `bos:UsuryLedgerRecord` | `…#UsuryLedgerRecord` | Class |
| `bos:MusterLedger` | `…#MusterLedger` | Class |
| `bos:MusterLedgerRecord` | `…#MusterLedgerRecord` | Class |
| `bos:Passage` | `…#Passage` | Class |
| `bos:assignedToGate` | `…#assignedToGate` | ObjectProperty |
| `bos:repairsGate` | `…#repairsGate` | ObjectProperty |
| `bos:buildsWallSection` | `…#buildsWallSection` | ObjectProperty |
| `bos:hasBuilder` | `…#hasBuilder` | ObjectProperty |
| `bos:hasWallSection` | `…#hasWallSection` | ObjectProperty |
| `bos:hasVerdict` | `…#hasVerdict` | ObjectProperty |
| `bos:hasProclamation` | `…#hasProclamation` | ObjectProperty |
| `bos:hasMusterRecord` | `…#hasMusterRecord` | ObjectProperty |
| `bos:hasUsuryAudit` | `…#hasUsuryAudit` | ObjectProperty |
| `bos:hasCourierRecord` | `…#hasCourierRecord` | ObjectProperty |
| `bos:hasMockerFeedback` | `…#hasMockerFeedback` | ObjectProperty |
| `bos:routesToGate` | `…#routesToGate` | ObjectProperty |
| `bos:hasCrossReference` | `…#hasCrossReference` | ObjectProperty |
| `bos:hasSource` | `…#hasSource` | ObjectProperty |
| `bos:hasCanonicalReference` | `…#hasCanonicalReference` | DatatypeProperty (xsd:string) |
| `bos:classifiesPayload` | `…#classifiesPayload` | DatatypeProperty (xsd:string) |
| `bos:extractsSignal` | `…#extractsSignal` | DatatypeProperty (xsd:string) |
| `bos:refusesPoison` | `…#refusesPoison` | DatatypeProperty (xsd:boolean) |
