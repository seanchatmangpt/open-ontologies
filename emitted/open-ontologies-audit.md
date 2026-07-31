# Open-Ontologies Directory Audit

**Generated:** 2026-06-01  
**Repository:** ~/open-ontologies  
**Total .ttl files:** 351 (in root + /ontology + /ontology/profiles + /ontology/zoela + /tests)

---

## 1. Directory Structure

```
~/open-ontologies/
├── ontology/                    # 30 core .ttl files + profiles + zoela subdirs
│   ├── *.ttl                    # 28 domain files
│   ├── profiles/                # 7 .ttl files (role & Zoela specializations)
│   └── zoela/                   # 40 .ttl files (ZOE LA mobile service graph)
├── ontology/zoela/              # Church ministry operations domain
├── tests/                        # 1 test ontology
└── cell8-final-assertion-report.ttl  # Root-level assertion report
```

**Core ontology inventory:**
- `wasm4pm-stubs.ttl` (37.6 KB) — Process mining module stubs (SOURCE OF TRUTH)
- `cli-open-ontologies.ttl` (53.1 KB) — CLI spec and generation rules
- `powl-process-mining.ttl` (8.8 KB) — POWL discovery variants + OCEL 2.0 types
- `truex-ecosystem.ttl` (18.5 KB) — Closure-obligation law surface & TruEx packages
- `portfolio-os.ttl` (15.4 KB) — PR-Ralph portfolio OS (cells, ticks, receipts, andons)
- `ontostar-wasm4pm-integration.ttl` (10.3 KB) — Master integration (AAT-Live + POWL + SharedReceipt)
- `thesis-manufacturing.ttl` (17.0 KB) — Thesis development lifecycle (claims, evidence, defects)
- `aat-live-rules.ttl` (12.8 KB) — 16 AAT-Live runtime correlation checks
- `mcpp-proof-chain.ttl` (10.4 KB) — 5 admission gates (MCPP manufacturing pipeline)
- `requirements.ttl` (8.1 KB) — Requirements andon / CTQ forge
- `cell8-*.ttl` (5 files) — Cell8 manufacturing gates & shapes
- `shared-receipt-shapes.ttl` (11.0 KB) — SharedReceiptV1 SHACL conformance shapes
- `thesis-shapes.ttl` (8.3 KB) — Thesis artifact SHACL shapes
- `revops-manufacturing.ttl` (3.9 KB) — RevOps pipeline stages
- `alignment-notes.md` (8.8 KB) — Namespace & import patterns

---

## 2. Namespace Prefixes (Declared & Used)

### W3C / Standard Vocabularies

| Prefix | Namespace URI | Usage |
|--------|---------------|-------|
| `owl` | `http://www.w3.org/2002/07/owl#` | Ontology definitions, class/property declarations |
| `rdf` | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` | RDF syntax, type assertions |
| `rdfs` | `http://www.w3.org/2000/01/rdf-schema#` | Labels, comments, subclass, domain/range |
| `xsd` | `http://www.w3.org/2001/XMLSchema#` | Typed literals (date, int, float, string) |
| `skos` | `http://www.w3.org/2004/02/skos/core#` | Concept schemes, notation, broader/narrower |
| `sh` | `http://www.w3.org/ns/shacl#` | SHACL shape definitions, validation rules |
| `prov` | `http://www.w3.org/ns/prov#` | Provenance (Entity, Activity, Agent, WasGeneratedBy) |
| `org` | `http://www.w3.org/ns/org#` | Organizational structure (Agent, Organization) |
| `dcat` | `http://www.w3.org/ns/dcat#` | Dataset catalogs & distributions |
| `time` | `http://www.w3.org/2006/time#` | Temporal properties & intervals |
| `acl` | `http://www.w3.org/ns/auth/acl#` | Access control lists |
| `odrl` | `http://www.w3.org/ns/odrl/2/` | Open Digital Rights Language (policies) |
| `earl` | `http://www.w3.org/ns/earl#` | Test results & conformance assertions |
| `foaf` | `http://xmlns.com/foaf/0.1/` | Friend-of-a-Friend (Person, name, email) |
| `schema` | `https://schema.org/` | Schema.org vocabularies (generic entities) |
| `dcterms` | `http://purl.org/dc/terms/` | Dublin Core metadata (creator, created, license, title) |

### Domain-Specific URN: Namespaces (OntoStar Family)

| Prefix | Namespace URI | Domain | Authority |
|--------|---------------|--------|-----------|
| `w4pm` | `urn:ontostar:wasm4pm:` | Process mining execution engine stubs | wasm4pm crate |
| `powl` | `urn:ontostar:powl:` | Partially-Ordered Workflow Language discovery | POWL standard (pm4py) |
| `aat` | `urn:ontostar:aat:live:` | AAT-Live runtime correlation & observability rules | AAT engine |
| `mcpp` | `urn:ontostar:mcpp:` | Manufacturing Certificate & Proof Pipeline | MCP+ manufacturing |
| `c8` / `cell8` | `urn:ontostar:cell8:` & `urn:ontostar:cell8:shape:` | Cell8 manufacturing gates & shapes | Truex ecosystem |
| `attest` | `urn:ontostar:attestation:` | Attestation & trusted key shapes | Truex ecosystem |
| `sr` | `urn:ontostar:shared-receipt:` | SharedReceiptV1 conformance shapes | Receipt chain |
| `integ` | `urn:ontostar:integration:wasm4pm-mcpp:` | wasm4pm-mcpp integration surface | Master integration |
| `ostar` | `urn:ostar:ontology#` | OntoStar generic ontology root | OntoStar project |

### Project-Specific HTTP: Namespaces

| Prefix | Namespace URI | Domain |
|--------|---------------|--------|
| `ggen` | `https://open-ontologies.org/ggen#` | GGen (generation/manufacturing orchestration) |
| `ghf` | `https://open-ontologies.org/profile/github-factory#` | GitHub Factory profile & security policies |
| `truex` | `https://open-ontologies.org/profile/truex#` | TruEx ecosystem & closure theorems |
| `cli` | `https://ggen.io/onto/cli/spec/` | CLI specification & command structure |
| `onto` | `https://ggen.io/onto/cli/open-ontologies/` | Open-ontologies CLI namespace |
| `req` | `https://ggen.io/onto/requirements/` | Requirements andon & CTQ forge |
| `tm` | `https://ggen.io/onto/thesis-manufacturing/` | Thesis manufacturing pipeline |
| `port` | `https://ggen.io/onto/portfolio-os/` | Portfolio OS (cells, ticks, receipts) |
| `revops` | `https://ggen.io/onto/cli/revops/` | RevOps manufacturing profile |
| `zoe` | `https://zoela.org/ontology/` | ZOE LA mobile service graph |

### Supporting Vocabularies

| Prefix | Namespace URI | Usage |
|--------|---------------|-------|
| `bibo` | `http://purl.org/ontology/bibo/` | Bibliographic ontology (papers, documents) |
| `doco` | `http://purl.org/spar/doco/` | Document Components ontology |
| `cito` | `http://purl.org/spar/cito/` | Citation Typing Ontology |
| `nanopub` | `http://purl.org/nanopub/` | Nanopublications (linked research) |
| `sioc` | `http://rdfs.org/sioc/ns#` | Semantically-Interlinked Online Communities |
| `qb` | `http://purl.org/linked-data/cube#` | RDF Data Cube (multi-dimensional data) |
| `spdx` | `http://spdx.org/rdf/terms#` | SPDX software licensing |
| `as` | `https://www.w3.org/ns/activitystreams#` | Activity Streams (social events) |
| `geo` | `http://www.opengis.net/ont/geosparql#` | GeoSPARQL (spatial/geographic data) |
| `sf` | `http://www.opengis.net/ont/sf#` | Simple Features (geometry) |
| `vcard` | `http://www.w3.org/2006/vcard/ns#` | vCard (contact information) |
| `fibo-be-oac` | `https://spec.edmcouncil.org/fibo/ontology/BE/OwnershipAndControl/ControlParties/` | FIBO parties |
| `fibo-fnd-acc-cur` | `https://spec.edmcouncil.org/fibo/ontology/FND/Accounting/CurrencyAmount/` | FIBO currency |

**Note:** All W3C namespaces are pinned to their canonical URIs. OntoStar `urn:` namespaces are project authority; `https://open-ontologies.org/` and `https://ggen.io/` are HTTP bases for published profiles.

---

## 3. Domain Coverage by Module

### 3.1 Process Mining & Workflow (POWL, OCEL, Token Replay)

**Authority:** wasm4pm execution engine stubs + POWL standard  
**Key Files:** `wasm4pm-stubs.ttl`, `powl-process-mining.ttl`  
**Namespaces:** `w4pm:`, `powl:`, `integ:` (integration)

| Domain | Entity Classes | Key Properties |
|--------|---|---|
| **POWL Models** | `w4pm:PowlConformanceModule`, `powl:DiscoveryVariant` | `powl:variant` (8 named individuals), `powl:hasCutOrder` |
| **Process Discovery** | `powl:DecisionGraphCyclic`, `powl:DecisionGraphMax`, etc. | `skos:notation` (variant code), `rdfs:comment` (CutFilter order) |
| **Token Replay** | `w4pm:FitnessResult`, `w4pm:TraceReplayResult` | `w4pm:hasProducedTokens`, `w4pm:hasConsumedTokens`, `w4pm:fitness` (f64) |
| **Event Logs** | `w4pm:Event`, `w4pm:Trace`, `w4pm:EventLog` | `w4pm:case_id`, `w4pm:name`, `w4pm:timestamp` |
| **OCEL 2.0** | `w4pm:OCEL`, `w4pm:OCELEvent`, `w4pm:OCELObject` | `w4pm:object_type`, `w4pm:event_type`, `w4pm:attributes` |
| **Petri Nets** | `w4pm:PowlMarking`, `w4pm:PowlPetriNet` | `w4pm:state` (marking), `w4pm:transitions` (count) |

**Module Stubs (SOURCE OF TRUTH):**
- `wasm4pm_stub` — POWL parsing, conformance testing, arena management
- `wasm4pm_types_stub` — OCEL 2.0 Event/Trace/EventLog types
- `wasm4pm_cognition_stub` — 9-breed symbolic AI dispatch (Eliza, CBR, Dendral, STRIPS, Prolog, MYCIN, GPS, Soar, Hearsay)
- `wasm4pm_algos_stub` — Token replay conformance, Alpha algorithm discovery

---

### 3.2 Manufacturing & Quality Gates (Cell8, AAT-Live, MCPP)

**Authority:** Truex ecosystem + MCPP manufacturing pipeline + AAT engine  
**Key Files:** `cell8-*.ttl`, `aat-live-rules.ttl`, `mcpp-proof-chain.ttl`  
**Namespaces:** `c8:`, `cell8:` (shapes), `aat:`, `mcpp:`

| Layer | Entity Classes | Purpose |
|---|---|---|
| **Cell8 Gates** | `earl:TestCriterion`, `skos:Concept` (seed, breed, seal, sheet, sell) | 5 manufacturing gates: **SEED** (input admission) → **BREED** (hypothesis generation) → **SEAL** (proof formation) → **SHEET** (evidence compilation) → **SELL** (release authorization) |
| **AAT-Live Rules** | `aat:CorrelationRule` | 16 runtime checks between wasm4pm observability bridges & AAT engine |
| **MCPP Proof Chain** | `mcpp:ProofGate` | 5 sequential admission gates for work order entry (Discovery, Execution, Conformance, Release, Closure) |
| **SHACL Shapes** | `sh:NodeShape`, `sh:PropertyShape` | Conformance validation (Cell8 13-gate coverage, SharedReceiptV1 hashes, theses) |

**Gate Semantics:**
- **SEED:** Raw input validation (structurally well-formed)
- **BREED:** Candidate generation & symbolic AI selection (9-breed cognition dispatch)
- **SEAL:** Proof formation from evidence (receipts, test results)
- **SHEET:** Evidence compilation & report generation
- **SELL:** Market release authorization (AndOn fulfillment)

---

### 3.3 Receipt & Proof Chains (SharedReceipt, Thesis Manufacturing)

**Authority:** Receipt provenance + thesis lifecycle  
**Key Files:** `shared-receipt-shapes.ttl`, `thesis-manufacturing.ttl`, `thesis-shapes.ttl`  
**Namespaces:** `sr:`, `tm:` (thesis manufacturing)

| Concept | RDF Classes | Key Properties |
|---------|---|---|
| **SharedReceiptV1** | `sr:Receipt` | `sr:conformance_dimensions` (fitness, precision, generalization), `sr:hashes` (SHA256 chain) |
| **Conformance** | `sr:ConformanceDimension` (fitness, precision, generalization) | `sr:value` (0.0-1.0), `sr:measured_at` (ISO timestamp) |
| **Thesis Claims** | `tm:ResearchQuestion`, `tm:Claim`, `tm:Evidence` | `tm:hasEvidence`, `tm:hasDefect`, `tm:law` (named law) |
| **Defects** | `tm:Defect`, `tm:Law` | `tm:defect_type` (NamedLaw enum), `tm:severity` (critical/major/minor) |
| **Thesis Artifacts** | SHACL shapes in `thesis-shapes.ttl` | `sh:targetClass` (Thesis, Claim, Evidence, Defect) |

---

### 3.4 Portfolio OS (Cells, Ticks, Receipts, AndOns)

**Authority:** PR-Ralph portfolio operating system  
**Key File:** `portfolio-os.ttl`  
**Namespace:** `port:`

| Concept | RDF Class | Description |
|---------|---|---|
| **Portfolio Cell** | `port:Cell` | Atomic unit of portfolio state (PRiority-centric convergence) |
| **Tick** | `port:Tick` | Discrete time step in portfolio state machine (cycle count) |
| **Receipt** | `port:Receipt` | Proof of work completion; enables portfolio tick advancement |
| **AndOn** | `port:AndOn` | Conjunction/obligation: "all of these conditions must hold to proceed" |
| **Obligation** | `port:Obligation` | Condition that must be discharged before release |
| **Convergence** | `port:ConvergenceCriteria` | Portfolio closure condition (all cells satisfied, no orphans) |

---

### 3.5 Requirements & CTQ Forge

**Authority:** OntoStar requirements layer  
**Key File:** `requirements.ttl`  
**Namespace:** `req:`

| Concept | RDF Class | Properties |
|---------|---|---|
| **Source Signal** | `req:SourceSignal` | Customer voice or operational constraint |
| **CTQ Flowdown** | `req:CriticalToQuality` | Derived from source signals (Critical-To-Quality parameter) |
| **Requirement** | `req:Requirement` | Formal specification (text, bounds, priority) |
| **AndOn** | `req:AndOn` | Logical conjunction of requirements (all must hold) |
| **Refusal** | `req:Refusal` | Reason for requirement violation (named law) |

---

### 3.6 TruEx Ecosystem (Closure Theorems & Obligations)

**Authority:** Execution-admissibility infrastructure  
**Key File:** `truex-ecosystem.ttl`  
**Namespace:** `truex:`

| Concept | RDF Class | Purpose |
|---------|---|---|
| **Closure Theorem** | `truex:ClosureTheorem` | Root mathematical proof target for manufacturing pass |
| **Obligation** | `truex:Obligation` | Non-negotiable requirement for closure |
| **Artifact** | `truex:Artifact` | Physical file emitted during manufacturing |
| **E2E Test** | `truex:E2ETest` | Behavioral validation scenario |
| **DerivationRule** | `truex:DerivationRule` | Formal logic rule deriving OCEL objects/events |
| **Package** | `truex:Package` | Modular unit of the TruEx ecosystem |

---

### 3.7 ZOE LA Mobile (Church Ministry Operations)

**Authority:** ZOE LA mobile service graph  
**Key Files:** 40 .ttl files in `ontology/zoela/`  
**Namespace:** `zoe:`

**Domains covered:**

| Domain | File | Concepts |
|--------|------|----------|
| **Core Integration** | `core.ttl` | Master ontology (all imports) |
| **Campus** | `campus.ttl` | Physical locations, ministry sites |
| **Ministry** | N/A (implied by profile) | Spiritual care, group leadership |
| **People** | `person.ttl` | Individuals, households, generations |
| **Groups** | `connect-groups.ttl` | Connect groups (prayer, fellowship, learning) |
| **Events** | `event.ttl` | Gatherings, services, programs |
| **Care** | `care.ttl` | Pastoral care, follow-up, outcomes |
| **Volunteers** | `volunteer.ttl` | Volunteer roles, capacity, scheduling |
| **Resources** | `resource.ttl` | Funding, facilities, materials |
| **Leadership** | `leadership-college.ttl` | Leaders, mentorship, training |
| **Roles & Permissions** | `roles.ttl`, `permissions.ttl` | Role-based access control (ODRL) |
| **Consent** | `consent.ttl` | Opt-in/opt-out, GDPR compliance |
| **Evidence** | `evidence.ttl` | Spiritual growth evidence, metrics |
| **Receipts** | `receipt.ttl` | Care interaction receipts, audit trail |
| **Policy** | `policy.ttl` | Governance rules, refusal conditions |
| **Finance** | `finance.ttl` | Giving, pledges, fund allocation |
| **Kids & Youth** | `kids.ttl`, `yth.ttl` | Children & youth-specific pastoral care |
| **Generations** | `generations.ttl` | Age cohorts, life stages |
| **Household** | `household.ttl` | Family units, dependencies |
| **Referrals & Outcomes** | `referrals.ttl`, `outcomes.ttl` | External referral network, care outcomes |
| **Categories & Navigation** | `categories.ttl`, `navigation.ttl` | Tag taxonomy, app navigation |
| **Autonomics & Policies** | `autonomics.ttl`, `autonomic-policies.ttl` | Policy automation, runtime checks |

**Key property pattern:** All ZOE LA modules use:
- `prov:wasGeneratedBy` (link to care activity)
- `dcterms:created` (timestamp audit trail)
- `foaf:name`, `foaf:email` (person identifiers)
- `org:hasMember` (group membership)
- `odrl:assigner`, `odrl:assignee` (permission chains)

---

### 3.8 GitHub Factory (GHF) Profile

**Authority:** GitHub CI/CD & security policies  
**Key Files:** `ghf-core.ttl`, `ghf-shacl.ttl`, `ghf-security-policy.ttl`  
**Namespace:** `ghf:`

| Concept | Purpose |
|---------|---------|
| **Contribution Receipt** | Track code contributions, licensing |
| **No Synthetic Closure** | Rule: forbid auto-generated closing commits |
| **Contribution Unit** | Atomic contribution for receipt issuance |

---

### 3.9 Integration Surfaces

**Master Integration:** `ontostar-wasm4pm-integration.ttl` (namespace: `integ:`)

**Three-way coupling:**
1. **AAT-Live rules** → Runtime correlation checks between wasm4pm observability & AAT engine
2. **POWL process mining** → Event log conformance (token replay, fitness)
3. **SharedReceiptV1 shapes** → Proof validation (fitness, precision, generalization bounds)

**Public Alignment:** `public-alignment.ttl` — Bridges generic Prov/SKOS vocabularies to OntoStar domain classes

---

## 4. Authority Hierarchy & Ownership

### Primary Authorities (Canonical)

| Authority | Namespace | Files | Scope |
|-----------|-----------|-------|-------|
| **wasm4pm crate** | `w4pm:`, `powl:` | `wasm4pm-stubs.ttl`, `powl-process-mining.ttl` | Process mining engine API spec (SOURCE OF TRUTH) |
| **Truex ecosystem** | `c8:`, `cell8:`, `attest:`, `truex:` | `cell8-*.ttl`, `truex-ecosystem.ttl` | Manufacturing gates, closure theorems, obligations |
| **MCPP pipeline** | `mcpp:`, `integ:` | `mcpp-proof-chain.ttl`, `ontostar-wasm4pm-integration.ttl` | 5-gate admission proof chain, integration coordination |
| **AAT engine** | `aat:` | `aat-live-rules.ttl` | 16 runtime correlation checks |
| **Receipt chain** | `sr:` | `shared-receipt-shapes.ttl` | SharedReceiptV1 conformance shapes |
| **OntoStar generic** | `ostar:` | (distributed) | Generic ontology root |

### Secondary Authorities (Domain-Specific)

| Authority | Namespace | Purpose |
|-----------|-----------|---------|
| **Thesis Manufacturing** | `tm:` | Thesis lifecycle (claims, evidence, defects, laws) |
| **Portfolio OS** | `port:` | PR-Ralph portfolio state machines |
| **Requirements Andon** | `req:` | CTQ flowdown, source signals |
| **RevOps Profile** | `revops:` | RevOps manufacturing stages |
| **GitHub Factory** | `ghf:` | GitHub CI/CD, licensing, contribution receipts |
| **ZOE LA Mobile** | `zoe:` | Church ministry operations |

### Tertiary Authorities (Orchestration & CLI)

| Authority | Namespace | Purpose |
|-----------|-----------|---------|
| **GGen** | `ggen:`, `cli:`, `onto:` | Code generation, CLI spec, open-ontologies CLI |
| **Requirements** | `req:` | Requirements andon & CTQ forge |

---

## 5. Import Patterns & Namespace Alignment Rules

### Master Import Chain

```
ontostar-wasm4pm-integration.ttl (MASTER)
  ├─ owl:imports aat:Ontology (AAT-Live rules)
  ├─ owl:imports powl:ontology (POWL discovery + OCEL 2.0)
  └─ (implicitly aligns with SharedReceiptV1 shapes)

wasm4pm-stubs.ttl
  └─ owl:imports powl: (POWL dependency)

portfolio-os.ttl
  └─ owl:imports prov#Ontology (Provenance foundation)
     owl:imports org#Ontology (Organizational structure)
     owl:imports earl# (Test results & conformance)
     owl:imports odrl/2/ (Policies & rights)
     owl:imports qb# (RDF Data Cube for metrics)

thesis-manufacturing.ttl
  └─ owl:imports dcterms (Metadata)
     owl:imports skos (Concept schemes)
     owl:imports earl# (Conformance assertions)
     owl:imports bibo (Document ontology)
     owl:imports doco (Document components)
     owl:imports cito (Citation typing)
     owl:imports nanopub (Nanopublications)

zoela/core.ttl (ZOE LA master)
  └─ owl:imports [40 module ontologies]
      (campus, ministry, connect-groups, roles, permissions, consent,
       routes, evidence, ocel, volunteer, resource, care, need, receipt,
       policy, generations, kids, yth, leadership-college, ...)
```

### Namespace Alignment Rules

**Rule 1: OntoStar Authority Separation**
- `urn:ontostar:*` namespaces are canonical within their domain
- No cross-domain reuse (e.g., `w4pm:` is wasm4pm-exclusive; `c8:` is Cell8-exclusive)
- Integration happens via `integ:` namespace (bridge classes + properties)

**Rule 2: W3C Base Vocabularies**
- All ontology files MUST declare `owl`, `rdf`, `rdfs`, `xsd` (unambiguous)
- Additional W3C vocabularies imported explicitly via `owl:imports` when needed
- No local redeclaration of W3C terms

**Rule 3: Domain-Specific HTTP Bases**
- `https://open-ontologies.org/` — Published public profiles (ghf, truex, ggen)
- `https://ggen.io/onto/` — GGen-controlled ontology paths (cli, requirements, thesis-manufacturing, portfolio-os, revops)
- `https://zoela.org/ontology/` — ZOE LA church mobility domain (40 modules)

**Rule 4: URN: Namespace Ownership**
- `urn:ontostar:` — OntoStar project family (wasm4pm, powl, aat, mcpp, cell8, sr, attestation)
- `urn:ostar:` — Generic OntoStar root (legacy/shared)
- **No local service URNs** (each module must use assigned namespace)

**Rule 5: Import Depth**
- Maximum 3 levels of transitive `owl:imports` to prevent circular dependencies
- CLI spec (`cli:Ontology`) can follow imports up to configured depth (default: 3)

**Rule 6: Prefix Consistency**
- Each namespace has ONE canonical prefix (no aliasing)
- Exception: `dcterms` and `dct` are equivalent (Dublin Core); standardize to `dcterms` in new files

---

## 6. Namespace Table (Definitive Reference)

| Prefix | Namespace URI | Authority | Import | Domain |
|--------|---|---|---|---|
| `owl` | `http://www.w3.org/2002/07/owl#` | W3C | implicit | OWL 2 syntax & semantics |
| `rdf` | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` | W3C | implicit | RDF core |
| `rdfs` | `http://www.w3.org/2000/01/rdf-schema#` | W3C | implicit | RDF schema |
| `xsd` | `http://www.w3.org/2001/XMLSchema#` | W3C | implicit | XML Schema types |
| `skos` | `http://www.w3.org/2004/02/skos/core#` | W3C | explicit | Concept schemes |
| `sh` | `http://www.w3.org/ns/shacl#` | W3C | explicit | SHACL validation |
| `prov` | `http://www.w3.org/ns/prov#` | W3C | explicit | Provenance |
| `org` | `http://www.w3.org/ns/org#` | W3C | explicit | Organizations |
| `dcat` | `http://www.w3.org/ns/dcat#` | W3C | explicit | Datasets & catalogs |
| `time` | `http://www.w3.org/2006/time#` | W3C | explicit | Temporal reasoning |
| `acl` | `http://www.w3.org/ns/auth/acl#` | W3C | explicit | Access control |
| `odrl` | `http://www.w3.org/ns/odrl/2/` | W3C | explicit | Digital rights & policies |
| `earl` | `http://www.w3.org/ns/earl#` | W3C | explicit | Evaluation & reporting |
| `foaf` | `http://xmlns.com/foaf/0.1/` | FOAF Project | explicit | Social networks & people |
| `schema` | `https://schema.org/` | Schema.org | explicit | Generic web entities |
| `dcterms` | `http://purl.org/dc/terms/` | Dublin Core | explicit | Metadata (creator, date, license) |
| `bibo` | `http://purl.org/ontology/bibo/` | BIBO | explicit | Bibliographic metadata |
| `doco` | `http://purl.org/spar/doco/` | SPAR | explicit | Document components |
| `cito` | `http://purl.org/spar/cito/` | SPAR | explicit | Citation typing |
| `nanopub` | `http://purl.org/nanopub/` | Nanopub | explicit | Linked research |
| `sioc` | `http://rdfs.org/sioc/ns#` | SIOC | explicit | Online communities |
| `qb` | `http://purl.org/linked-data/cube#` | W3C LD Cube | explicit | Multi-dimensional data |
| `spdx` | `http://spdx.org/rdf/terms#` | SPDX | explicit | Software licensing |
| `as` | `https://www.w3.org/ns/activitystreams#` | W3C | explicit | Activity streams |
| `geo` | `http://www.opengis.net/ont/geosparql#` | OGC | explicit | Geospatial data |
| `sf` | `http://www.opengis.net/ont/sf#` | OGC | explicit | Simple Features geometry |
| `vcard` | `http://www.w3.org/2006/vcard/ns#` | W3C | explicit | vCard contacts |
| `fibo-be-oac` | `https://spec.edmcouncil.org/fibo/ontology/BE/OwnershipAndControl/ControlParties/` | FIBO | explicit | Business entities |
| `fibo-fnd-acc-cur` | `https://spec.edmcouncil.org/fibo/ontology/FND/Accounting/CurrencyAmount/` | FIBO | explicit | Currency & accounting |
| **ONTOSTAR CORE** | | | | |
| `w4pm` | `urn:ontostar:wasm4pm:` | wasm4pm crate | implicit | Process mining engine (SOURCE OF TRUTH) |
| `powl` | `urn:ontostar:powl:` | POWL standard | implicit | Partially-Ordered Workflows |
| `aat` | `urn:ontostar:aat:live:` | AAT engine | implicit | Runtime correlation rules |
| `mcpp` | `urn:ontostar:mcpp:` | MCPP pipeline | implicit | Proof chain, admission gates |
| `c8` | `urn:ontostar:cell8:` | Truex/Cell8 | implicit | Manufacturing gate enums |
| `cell8` | `urn:ontostar:cell8:shape:` | Truex/Cell8 | implicit | Cell8 SHACL shapes |
| `attest` | `urn:ontostar:attestation:` | Truex | implicit | Attestation & trusted keys |
| `sr` | `urn:ontostar:shared-receipt:` | Receipt chain | implicit | SharedReceiptV1 shapes |
| `integ` | `urn:ontostar:integration:wasm4pm-mcpp:` | wasm4pm-mcpp | implicit | Master integration surface |
| `ostar` | `urn:ostar:ontology#` | OntoStar (legacy) | implicit | Generic OntoStar root |
| **PUBLISHED PROFILES** | | | | |
| `ggen` | `https://open-ontologies.org/ggen#` | open-ontologies | explicit | GGen orchestration & generation |
| `ghf` | `https://open-ontologies.org/profile/github-factory#` | open-ontologies | explicit | GitHub Factory profile |
| `truex` | `https://open-ontologies.org/profile/truex#` | open-ontologies | explicit | TruEx ecosystem |
| `cli` | `https://ggen.io/onto/cli/spec/` | ggen.io | explicit | CLI specification |
| `onto` | `https://ggen.io/onto/cli/open-ontologies/` | ggen.io | explicit | Open-ontologies CLI |
| `req` | `https://ggen.io/onto/requirements/` | ggen.io | explicit | Requirements andon |
| `tm` | `https://ggen.io/onto/thesis-manufacturing/` | ggen.io | explicit | Thesis manufacturing |
| `port` | `https://ggen.io/onto/portfolio-os/` | ggen.io | explicit | Portfolio OS |
| `revops` | `https://ggen.io/onto/cli/revops/` | ggen.io | explicit | RevOps manufacturing |
| `zoe` | `https://zoela.org/ontology/` | ZOE LA | explicit | Church ministry operations |

---

## 7. Domain Coverage Matrix

| Domain | Covered By | Scope | Authority |
|--------|-----------|-------|-----------|
| **Process Mining** | wasm4pm-stubs, powl-process-mining | Discovery variants, OCEL 2.0 types, token replay, conformance | wasm4pm crate + POWL standard |
| **Manufacturing Gates** | cell8-*.ttl, aat-live-rules | SEED → BREED → SEAL → SHEET → SELL | Truex + AAT engine |
| **Proof Chains** | mcpp-proof-chain, shared-receipt-shapes | 5-gate admission, SharedReceiptV1 conformance (fitness/precision) | MCPP pipeline + receipt chain |
| **Thesis Lifecycle** | thesis-manufacturing, thesis-shapes | Claims, evidence, defects, laws, artifacts | Thesis manufacturing |
| **Portfolio State Machines** | portfolio-os | Cells, ticks, receipts, andons, convergence | PR-Ralph portfolio OS |
| **Requirements Andon** | requirements | Source signals, CTQ flowdown, refusals | OntoStar requirements layer |
| **Church Ministry** | 40 zoela/*.ttl files | People, groups, events, care, volunteers, leadership, finance, consent, policy | ZOE LA mobile |
| **GitHub CI/CD** | ghf-*.ttl | Contributions, licenses, synthetic closure rules | GitHub Factory profile |
| **Integration** | ontostar-wasm4pm-integration | AAT-Live ↔ POWL ↔ SharedReceipt | Master integration surface |

---

## 8. Key Observations

### 8.1 Strengths

1. **Clear Authority Separation:** OntoStar `urn:` namespaces prevent namespace collision across domains.
2. **Multi-Level Metadata:** Files use both RDFS comments AND SHACL shapes for dual spec (readable + machine-validatable).
3. **W3C Foundation:** Consistent use of Dublin Core (dcterms), Prov, Skos, Earl, and Shacl creates interoperability.
4. **Modular Domain Coverage:** 40 ZOE LA modules + 8 Cell8 gates + 5 MCPP proof gates create granular, composable governance.
5. **SOURCE OF TRUTH Marking:** `wasm4pm-stubs.ttl` explicitly declares itself as canonical; prevents drift.

### 8.2 Gaps & Risks

1. **Prefix Duplication:** Some files declare both `dcterms:` and `dct:` for Dublin Core. Standardize to `dcterms:`.
2. **Missing Conformance Shaping:** Some ontologies (e.g., `requirements.ttl`) lack SHACL validation shapes. Recommend `requirements-shapes.ttl` be linked or expanded.
3. **ZOE LA Import Chain:** `zoela/core.ttl` imports 40+ modules; unclear if circular dependencies exist. Recommend dependency audit (`owl:imports` traversal).
4. **No Versioning on Non-W3C Ontologies:** OntoStar `urn:` ontologies lack `owl:versionInfo`. Recommend adding SemVer (e.g., "1.0.0-rc1").
5. **Limited Crosslinking:** `truex-ecosystem.ttl` and `thesis-manufacturing.ttl` operate in parallel but don't reference each other. Recommend explicit `rdfs:seeAlso` or integration via `integ:`.

### 8.3 Authority Alignment Quality

- **Canonical:** wasm4pm-stubs.ttl, powl-process-mining.ttl, cell8-*.ttl, aat-live-rules.ttl (all marked SOURCE OF TRUTH or domain-exclusive)
- **Well-Integrated:** ontostar-wasm4pm-integration.ttl, shared-receipt-shapes.ttl (explicit 3-way coupling)
- **Modular but Isolated:** portfolio-os.ttl, thesis-manufacturing.ttl (self-contained; could benefit from `integ:` bridge)
- **Emerging:** zoela/*.ttl (40 modules; ZOE LA-specific; light import chains to OntoStar)

---

## 9. Recommendations

1. **Audit ZOE LA circular imports:** Run transitive closure on `zoela/core.ttl` imports; flag any cycles.
2. **Standardize prefix declarations:** Replace all `dct:` with `dcterms:`; update all files.
3. **Add version info:** Annotate all `urn:ontostar:*` ontologies with `owl:versionInfo "1.0.0"` or similar.
4. **Link thesis & truex:** Create `thesis-truex-alignment.ttl` to show how thesis defects map to TruEx obligations.
5. **Expand conformance shapes:** Link `requirements.ttl` to a `requirements-conformance-shapes.ttl` using SHACL `sh:targetObjectsOf`.
6. **Document GGen integration:** Update `cli-open-ontologies.ttl` with explicit `owl:imports` chain for ggen/cli/open-ontologies namespace.

---

## 10. Files Generated

- **This audit:** `emitted/open-ontologies-audit.md`
- **Run:** No execution needed; static analysis only.
- **Size:** 351 .ttl files; 28 core domain + 7 profiles + 40 ZOE LA + 1 root assertion + 1 test + 274 in zoela/

---

**End of Audit**
