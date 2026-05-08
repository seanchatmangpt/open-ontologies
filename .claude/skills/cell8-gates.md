---
name: Cell8 Conformance Gates
description: Cell8 proof-carrying interchangeable WASM operating part conformance
paths: ["ontology/cell8-*.ttl", "src/cell8/**"]
type: skill
---

# Skill: Cell8 Conformance Gates

## Purpose

Understand and implement the 13 Cell8 conformance gates (A1-A13) that govern WASM operating part verification and manufacturing.

## The 13 Gates

| Gate | Name | Requirement |
|------|------|-------------|
| **A1** | Seed | Artifact exists in seeded state |
| **A2** | Breed | RDF evidence graph properly formed |
| **A3** | Validate | SHACL shapes pass (syntactic correctness) |
| **A4** | Reason | OWL reasoning produces no unsatisfiable classes |
| **A5** | Prove | Receipt chain valid (BLAKE3 hashes match) |
| **A6** | Seal | Ed25519 signature verifies |
| **A7** | Emit | Artifacts written to specified paths |
| **A8** | Journal | Event log entries match declared transitions |
| **A9** | Causal | Cross-artifact causality is mutually consistent |
| **A10** | Temporal | No impossible time overlaps in event log |
| **A11** | Governance | Operator (agent, human) is authorized |
| **A12** | Rollback | Previous version snapshot is available for revert |
| **A13** | Attest | External (non-Cell8) witness approves |

## Where Gates Live

Gates are defined in `ontology/cell8-manufacturing.ttl`:

```turtle
@prefix cell8: <urn:cell8:gate:> .
@prefix skos: <http://www.w3.org/2004/02/skos/core#> .

cell8:gate_a1
  a skos:Concept ;
  skos:prefLabel "Seed" ;
  skos:definition "..." ;
  schema:position "1"^^xsd:integer ;
  schema:execTime "100000"^^xsd:long .  # nanoseconds
```

## SHACL Shapes for Gate Validation

Gates A1-A3 are enforced via `ontology/cell8-shapes.ttl`:

```turtle
@prefix sh: <http://www.w3.org/ns/shacl#> .

cell8:GateA1Shape
  a sh:NodeShape ;
  sh:targetNode cell8:gate_a1 ;
  sh:property [
    sh:path rdfs:label ;
    sh:minCount 1 ;
    sh:datatype xsd:string
  ] .
```

Run: `onto validate ontology/cell8-shapes.ttl`

## Manufacturing Gate Chain

When applying Cell8:

1. Artifact enters "seeded" (A1)
2. RDF evidence generated (A2)
3. SHACL validation (A3)
4. OWL reasoning check (A4)
5. BLAKE3 receipt chain (A5)
6. Ed25519 signature (A6)
7. All artifacts emitted (A7)
8. Event log journaled (A8)
9. Cross-artifact causal chain (A9)
10. Temporal consistency (A10)
11. Operator authorization check (A11)
12. Snapshot for rollback (A12)
13. External attestation (A13)

**All 13 must pass** for Cell8 conformance.

## Commands

```bash
# Generate Cell8 gates from TTL
ggen sync --audit true

# Validate all gates
onto validate ontology/cell8-shapes.ttl

# Check specific gate
onto query --sparql 'SELECT ?label WHERE { <urn:cell8:gate:a1> rdfs:label ?label }'
```
