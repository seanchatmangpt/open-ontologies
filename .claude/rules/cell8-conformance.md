---
name: Cell8 Conformance Validation
description: 13 gates A1-A13, EARL assertion patterns, SHACL enforcement
type: rules
---

# Cell8 Conformance Validation

## The 13 Gates (A1-A13)

Cell8 is a proof-carrying interchangeable conformance framework. All 13 gates MUST pass for certification.

| Gate | Name | What it proves | Enforcement |
|------|------|----------------|------------|
| **A1** | Seed | Artifact exists in seeded state with metadata | SHACL NodeShape on artifact object type |
| **A2** | Breed | RDF evidence graph properly formed with valid triples | SHACL property constraints |
| **A3** | Validate | SHACL shapes pass (syntactic/semantic correctness) | `onto validate` with shapes.ttl |
| **A4** | Reason | OWL reasoning produces no unsatisfiable classes | OWL 2 reasoner consistency check |
| **A5** | Prove | Receipt chain valid: BLAKE3 hashes match predecessors | Chain linkage verification |
| **A6** | Seal | Ed25519 signature verifies against public key | Cryptographic signature check |
| **A7** | Emit | Artifacts written to specified paths with correct format | File existence + format check |
| **A8** | Journal | Event log entries match declared state transitions | OCEL event log conformance |
| **A9** | Causal | Cross-artifact causality mutually consistent | Causal chain closure check |
| **A10** | Temporal | No impossible time overlaps in event log | Timestamp ordering validation |
| **A11** | Governance | Operator (agent, human) is authorized | ACL check against governance policy |
| **A12** | Rollback | Previous version snapshot available for revert | Snapshot manifest verification |
| **A13** | Attest | External (non-Cell8) witness approves | External attestation signature + timestamp |

## SHACL Shapes for Gates A1-A3

All shapes defined in `ontology/cell8-shapes.ttl`:

```turtle
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix cell8: <urn:cell8:gate:> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# A1: Seed gate
cell8:GateA1Shape
  a sh:NodeShape ;
  sh:targetClass cell8:Artifact ;
  sh:property [
    sh:path rdfs:label ;
    sh:minCount 1 ;
    sh:maxCount 1 ;
    sh:datatype xsd:string ;
  ] ;
  sh:property [
    sh:path cell8:hasState ;
    sh:hasValue cell8:seeded ;
  ] ;
  sh:property [
    sh:path cell8:createdAt ;
    sh:minCount 1 ;
    sh:datatype xsd:dateTime ;
  ] .

# A2: Breed gate (proper RDF formation)
cell8:GateA2Shape
  a sh:NodeShape ;
  sh:targetClass cell8:RdfEvidence ;
  sh:property [
    sh:path rdfs:label ;
    sh:minCount 1 ;
  ] ;
  sh:closed true ;  # Only declared properties allowed
  sh:ignoredProperties (rdf:type) ;
  sh:minCount 1 .

# A3: Validate gate (SHACL shapes pass)
cell8:GateA3Shape
  a sh:NodeShape ;
  sh:targetNode cell8:conformance-report ;
  sh:property [
    sh:path cell8:conforms ;
    sh:hasValue true ;
  ] ;
  sh:property [
    sh:path cell8:violationCount ;
    sh:hasValue 0 ;
    sh:datatype xsd:integer ;
  ] .
```

Run validation:

```bash
onto validate ontology/cell8-manufacturing.ttl --shapes ontology/cell8-shapes.ttl
```

## EARL Assertion Patterns

Evaluation and Reporting Language (EARL) records conformance results:

```turtle
@prefix earl: <http://www.w3.org/ns/earl#> .
@prefix cell8: <urn:cell8:gate:> .
@prefix dct: <http://purl.org/dc/terms/> .

# Assertion: Gate A1 passed
[] a earl:Assertion ;
   earl:subject <urn:cell8:artifact:a1> ;
   earl:test cell8:GateA1Shape ;
   earl:result [
     a earl:TestResult ;
     earl:outcome earl:passed ;
     dct:issued "2026-05-07T14:23:45Z"^^xsd:dateTime ;
   ] .

# Assertion: Gate A5 passed
[] a earl:Assertion ;
   earl:subject <urn:cell8:receipt:r1> ;
   earl:test cell8:GateA5Shape ;
   earl:result [
     a earl:TestResult ;
     earl:outcome earl:passed ;
     dct:issued "2026-05-07T14:24:00Z"^^xsd:dateTime ;
   ] .

# Assertion: Gate A6 failed (signature missing)
[] a earl:Assertion ;
   earl:subject <urn:cell8:receipt:r2> ;
   earl:test cell8:GateA6Shape ;
   earl:result [
     a earl:TestResult ;
     earl:outcome earl:failed ;
     earl:info "Signature field is empty" ;
     dct:issued "2026-05-07T14:24:15Z"^^xsd:dateTime ;
   ] .
```

## Gate Execution Order

Gates run in dependency order; all must pass:

```
A1 (Seed)              ← Artifact exists in correct state
  ↓
A2 (Breed)             ← RDF triples well-formed
  ↓
A3 (Validate)          ← SHACL shapes pass (uses A2 output)
  ↓
A4 (Reason)            ← OWL consistency (uses A3 RDF)
  ↓
A5 (Prove)             ← BLAKE3 chain valid
  ↓
A6 (Seal)              ← Ed25519 signature verifies (uses A5 receipt)
  ↓
A7 (Emit)              ← Artifacts written
  ↓
A8 (Journal)           ← Event log entries match
A9 (Causal)            ← Cross-artifact causality
A10 (Temporal)         ← Timestamp ordering
A11 (Governance)       ← Operator authorized
A12 (Rollback)         ← Snapshot available
A13 (Attest)           ← External witness approves
```

(A8-A13 run in parallel after A7)

## Validation Workflow

```bash
# 1. Validate gates are defined
ggen sync --audit true   # Checks all 13 gates present in ontology

# 2. Run full conformance check
onto validate ontology/cell8-manufacturing.ttl --shapes ontology/cell8-shapes.ttl

# 3. Get EARL assertion report
onto validate --format earl > cell8-assertion-report.ttl

# 4. Check all 13 gates passed
jq '.gates[] | select(.passed == false)' cell8-assertion-report.json
# Should return empty (all gates passed)

# 5. Commit conformance evidence
git add ontology/cell8-*.ttl cell8-assertion-report.ttl
git commit -m "chore(cell8): Cell8 A1-A13 gates all passing"
```

## Forbidden Patterns

❌ Running gates out of order (A4 before A2)
❌ Skipping gate validation ("we're confident")
❌ SHACL shapes that don't validate Gate A1-A3 requirements
❌ Empty signature in receipt (violates A6)
❌ Artifact with mismatched state (violates A1)

## Required Patterns

✅ All 13 gates defined in `ontology/cell8-manufacturing.ttl`
✅ SHACL shapes enforcing A1-A3 in `ontology/cell8-shapes.ttl`
✅ EARL assertions recording all results
✅ `make adversarial` includes Cell8 gate validation
✅ Conformance evidence saved before release

## Commands

```bash
# List Cell8 gates
onto query select --sparql 'SELECT ?gate WHERE { ?gate a cell8:Gate }'

# Validate gates
onto validate ontology/cell8-manufacturing.ttl --shapes ontology/cell8-shapes.ttl

# Get EARL report
onto validate --format earl > report.ttl

# Full pipeline with Cell8 validation
ggen sync --audit true
make adversarial
```
