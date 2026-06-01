# Open-Ontologies: Quick Reference Guide

**Last Updated:** 2026-06-01  
**Audit Scope:** ~/open-ontologies (351 .ttl files)

---

## 1. What Is This Repository?

A **modular RDF ontology ecosystem** containing:
- **Process mining** (POWL, OCEL 2.0, token replay conformance)
- **Manufacturing gates** (Cell8: SEED → BREED → SEAL → SHEET → SELL)
- **Proof chains** (MCPP 5-gate admission, SharedReceiptV1 conformance)
- **Thesis lifecycle** (research questions, claims, evidence, defects, laws)
- **Portfolio state machines** (cells, ticks, receipts, andons)
- **Church ministry operations** (40 ZOE LA domain modules)
- **GitHub CI/CD** (GitHub Factory profile, contribution receipts)

---

## 2. Most Important Files

| File | Purpose | Audience |
|------|---------|----------|
| `ontology/wasm4pm-stubs.ttl` | **SOURCE OF TRUTH** for process mining engine API | wasm4pm crate users, integration teams |
| `ontology/powl-process-mining.ttl` | POWL discovery variants + OCEL 2.0 types | Process mining engineers |
| `ontology/ontostar-wasm4pm-integration.ttl` | Master integration (3-way coupling) | Integration architects |
| `ontology/cell8-*.ttl` | Manufacturing gates (5 files) | Truex ecosystem users |
| `ontology/shared-receipt-shapes.ttl` | SharedReceiptV1 SHACL validation | Proof chain maintainers |
| `ontology/thesis-manufacturing.ttl` | Thesis lifecycle (claims → evidence → laws) | Thesis authors |
| `ontology/portfolio-os.ttl` | PR-Ralph portfolio OS (cells, ticks, andons) | Portfolio operators |
| `ontology/zoela/core.ttl` | Master ZOE LA integration (40+ imports) | ZOE LA mobile developers |

---

## 3. Namespace Quick Look

### Core OntoStar (urn:ontostar:*)

```turtle
@prefix w4pm: <urn:ontostar:wasm4pm:> .       # Process mining stubs
@prefix powl: <urn:ontostar:powl:> .         # POWL discovery
@prefix aat: <urn:ontostar:aat:live:> .      # AAT-Live rules
@prefix mcpp: <urn:ontostar:mcpp:> .         # MCPP proof chain
@prefix c8: <urn:ontostar:cell8:> .          # Cell8 gates
@prefix sr: <urn:ontostar:shared-receipt:> . # SharedReceipt shapes
@prefix integ: <urn:ontostar:integration:wasm4pm-mcpp:> . # Master integration
```

### Published Profiles (https://)

```turtle
@prefix ggen: <https://open-ontologies.org/ggen#> .      # GGen orchestration
@prefix ghf: <https://open-ontologies.org/profile/github-factory#> .    # GitHub Factory
@prefix truex: <https://open-ontologies.org/profile/truex#> .           # TruEx
@prefix cli: <https://ggen.io/onto/cli/spec/> .          # CLI spec
@prefix tm: <https://ggen.io/onto/thesis-manufacturing/> . # Thesis mfg
@prefix port: <https://ggen.io/onto/portfolio-os/> .     # Portfolio OS
@prefix zoe: <https://zoela.org/ontology/> .             # ZOE LA
```

### W3C (Always available)

```turtle
@prefix owl: <http://www.w3.org/2002/07/owl#> .      # OWL 2
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> . # RDF
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .  # RDFS
@prefix sh: <http://www.w3.org/ns/shacl#> .          # SHACL
@prefix prov: <http://www.w3.org/ns/prov#> .         # Provenance
@prefix org: <http://www.w3.org/ns/org#> .           # Organizations
@prefix skos: <http://www.w3.org/2004/02/skos/core#> . # Concepts
@prefix dcterms: <http://purl.org/dc/terms/> .       # Metadata
@prefix earl: <http://www.w3.org/ns/earl#> .         # Conformance
```

---

## 4. Domain Coverage

### Process Mining (w4pm + powl)
- **Discovery Variants:** 8 named individuals (DecisionGraphCyclic, DecisionGraphMax, etc.)
- **OCEL 2.0:** Event, Trace, EventLog, Object, Relationship types
- **Token Replay:** FitnessResult, TraceReplayResult, Marking, PetriNet
- **Conformance:** fitness (0.0-1.0), precision, generalization

### Manufacturing Gates (c8 + cell8)
- **SEED:** Input admission (raw validation)
- **BREED:** Hypothesis generation (9-breed cognition dispatch)
- **SEAL:** Proof formation (receipts, test results)
- **SHEET:** Evidence compilation (reports)
- **SELL:** Release authorization (AndOn fulfillment)

### Proof Chains (mcpp + sr)
- **5 Admission Gates:** Discovery, Execution, Conformance, Release, Closure
- **SharedReceiptV1:** Fitness, precision, generalization bounds (0.0-1.0)
- **SHACL Shapes:** Node & property validation (sh:NodeShape, sh:PropertyShape)

### Thesis Lifecycle (tm + thesis-shapes)
- **Claims:** Research questions, hypotheses
- **Evidence:** Observations, experiments, papers
- **Defects:** Structural failures (NamedLaw enum)
- **Laws:** Named invariants that must hold

### Portfolio OS (port)
- **Cells:** Atomic portfolio units
- **Ticks:** Discrete time steps (cycle count)
- **Receipts:** Proof of work completion
- **AndOns:** Conjunction obligations (all must hold)
- **Convergence:** Portfolio closure (all cells satisfied)

### Church Ministry (zoe)
- **40 domain modules:** campus, ministry, groups, roles, permissions, consent, care, volunteers, leadership, finance, policy, generations, kids, youth, households, etc.
- **Key pattern:** `prov:wasGeneratedBy`, `dcterms:created`, `foaf:name`, `org:hasMember`, `odrl:assigner`

---

## 5. Import Chains (How Files Reference Each Other)

```
Master Integration: ontostar-wasm4pm-integration.ttl
  ├─ w4pm:* (wasm4pm-stubs.ttl)
  │   └─ powl:* (powl-process-mining.ttl)
  ├─ aat:* (aat-live-rules.ttl)
  └─ sr:* (shared-receipt-shapes.ttl)

Manufacturing: cell8-*.ttl (5 files)
  └─ sh:* (SHACL validation)

Portfolio: portfolio-os.ttl
  ├─ prov:*, org:*, earl:*, odrl:*, qb:*
  └─ [W3C imports]

Thesis: thesis-manufacturing.ttl + thesis-shapes.ttl
  ├─ dcterms:*, skos:*, earl:*
  ├─ bibo:*, doco:*, cito:*, nanopub:*
  └─ [W3C + bibliography imports]

Church: zoela/core.ttl (master)
  ├─ zoe:campus, zoe:ministry, zoe:connect-groups, …
  └─ [40 module imports]
```

---

## 6. Authority Hierarchy

**Tier 1 (Canonical):** OntoStar `urn:` namespaces
- wasm4pm-stubs.ttl (SOURCE OF TRUTH marker)
- powl-process-mining.ttl
- cell8-*.ttl
- aat-live-rules.ttl

**Tier 2 (Well-Integrated):** Master integration surfaces
- ontostar-wasm4pm-integration.ttl (3-way coupling)
- shared-receipt-shapes.ttl
- zoela/core.ttl

**Tier 3 (Modular but Isolated):** Self-contained domains
- portfolio-os.ttl
- thesis-manufacturing.ttl
- truex-ecosystem.ttl

**Tier 4 (Emerging):** ZOE LA modules (40 files, light import chains to OntoStar)

---

## 7. Key Observations

✅ **Strengths:**
- Clear authority separation via `urn:ontostar:*` namespaces
- Multi-level metadata (RDFS comments + SHACL shapes)
- W3C foundation (Dublin Core, Prov, Skos, Earl, Shacl)
- Modular domain coverage (8 Cell8 gates, 40 ZOE LA modules)

⚠️ **Gaps:**
- Prefix duplication (dcterms vs dct) — standardize to dcterms
- Missing conformance shaping for some domains (requirements.ttl)
- ZOE LA import chain circularity risk (40+ module imports)
- No versioning on `urn:ontostar:*` ontologies (add owl:versionInfo)
- Limited crosslinking between thesis & truex ecosystems

---

## 8. How to Add a New Ontology

**Step 1: Choose namespace**
```turtle
@prefix mydom: <urn:ontostar:mydomain:> .  # For canonical domains
@prefix mydom: <https://ggen.io/onto/mydomain/> . # For published profiles
```

**Step 2: Declare ontology**
```turtle
<urn:ontostar:mydomain:ontology>
    a owl:Ontology ;
    rdfs:label "My Domain Ontology"@en ;
    rdfs:comment "Formal model of my domain."@en ;
    dcterms:created "2026-06-01"^^xsd:date ;
    owl:versionInfo "1.0.0" ;
    owl:imports <urn:ontostar:powl:ontology> .  # if needed
```

**Step 3: Define classes & properties**
```turtle
mydom:MyClass
    a owl:Class ;
    rdfs:label "My Class"@en ;
    rdfs:comment "A thing in my domain."@en .

mydom:myProperty
    a owl:ObjectProperty ;
    rdfs:label "My Property"@en ;
    rdfs:domain mydom:MyClass ;
    rdfs:range mydom:OtherClass .
```

**Step 4: If validation needed, create SHACL shapes**
```turtle
<mydom:MyClassShape>
    a sh:NodeShape ;
    sh:targetClass mydom:MyClass ;
    sh:property [
        sh:path mydom:myProperty ;
        sh:minCount 1 ;
        sh:class mydom:OtherClass
    ] .
```

**Step 5: If master integration needed, update ontostar-wasm4pm-integration.ttl**
```turtle
owl:imports <urn:ontostar:mydomain:ontology> .
```

---

## 9. Common Patterns

### Metadata (Applied to all files)
```turtle
dcterms:creator "Sean Chatman" ;
dcterms:created "2026-06-01"^^xsd:date ;
dcterms:license <https://opensource.org/licenses/MIT> ;
owl:versionInfo "1.0.0" ;
```

### Witness/Authority Markers (OntoStar pattern)
```turtle
myontology:SomeClass
    a owl:Class ;
    rdfs:isDefinedBy mydom:AuthorityClass ;
    skos:notation "some_code" ;
    rdfs:comment "Clear, machine-readable description."@en .
```

### SHACL Validation
```turtle
mydom:MyShape
    a sh:NodeShape ;
    sh:targetClass mydom:MyClass ;
    sh:closed true ;
    sh:property [
        sh:path mydom:requiredProp ;
        sh:minCount 1 ;
        sh:maxCount 1 ;
        sh:datatype xsd:string
    ] .
```

### Proof / Receipt Pattern
```turtle
mydom:Receipt
    a owl:Class ;
    rdfs:comment "Proof of conformance."@en ;
    rdfs:subClassOf sr:ReceiptBase .

mydom:hasConformanceFitness
    a owl:DatatypeProperty ;
    rdfs:domain mydom:Receipt ;
    rdfs:range [
        a rdfs:Datatype ;
        owl:onDatatype xsd:float ;
        owl:withRestrictions (
            [ xsd:minInclusive "0.0"^^xsd:float ] 
            [ xsd:maxInclusive "1.0"^^xsd:float ]
        )
    ] .
```

---

## 10. Files Generated in This Audit

1. **open-ontologies-audit.md** — Full inventory, domains, authority hierarchy, import patterns
2. **namespace-hierarchy.txt** — Visual 5-tier namespace dependency graph
3. **QUICK-REFERENCE.md** — This document

---

## 11. Next Steps

1. **Audit ZOE LA circularity:** Run transitive closure on `zoela/core.ttl` imports
2. **Standardize prefixes:** Replace all `dct:` → `dcterms:` globally
3. **Add version info:** Annotate all `urn:ontostar:*` with `owl:versionInfo`
4. **Link thesis & truex:** Create bridge ontology (thesis-truex-alignment.ttl)
5. **Expand conformance:** Add SHACL shapes for `requirements.ttl`
6. **Document GGen:** Update `cli-open-ontologies.ttl` with explicit import chains

---

**Audit Complete**  
Generated: 2026-06-01  
Source: ~/open-ontologies (351 .ttl files)
