---
name: SHACL Shape Validation
description: SHACL NodeShape, property constraints, validation execution
paths: ["ontology/cell8-shapes.ttl", "src/shacl.rs"]
type: skill
---

# Skill: SHACL Validation

## Purpose

Define SHACL shapes that validate RDF data against structural and semantic constraints, and execute validation reports.

## SHACL Shape Basics

```turtle
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# Define a shape
:PersonShape
  a sh:NodeShape ;
  sh:targetClass :Person ;
  sh:property [
    sh:path rdfs:label ;
    sh:minCount 1 ;
    sh:maxCount 1 ;
    sh:datatype xsd:string ;
  ] .
```

Validates: Every `:Person` must have exactly one `rdfs:label` of type `xsd:string`.

## Common Constraints

| Constraint | Meaning | Example |
|-----------|---------|---------|
| `sh:minCount` | Minimum property occurrences | `sh:minCount 1` |
| `sh:maxCount` | Maximum property occurrences | `sh:maxCount 1` |
| `sh:datatype` | Required data type | `sh:datatype xsd:string` |
| `sh:nodeKind` | Node type (IRI, Literal, BlankNode) | `sh:nodeKind sh:IRI` |
| `sh:pattern` | Regex pattern | `sh:pattern "^[A-Za-z0-9]+$"` |
| `sh:minInclusive` | Minimum numeric value | `sh:minInclusive 0` |
| `sh:maxInclusive` | Maximum numeric value | `sh:maxInclusive 100` |
| `sh:in` | Allowed values (enum) | `sh:in ("red" "green" "blue")` |
| `sh:hasValue` | Required exact value | `sh:hasValue :someValue` |
| `sh:closed` | Only declared properties | `sh:closed true` |

## Cell8 Gates Shapes

Example shapes for Cell8 conformance gates (from `ontology/cell8-shapes.ttl`):

```turtle
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix cell8: <urn:cell8:gate:> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

# Gate A1 (Seed): Artifact must exist in seeded state
cell8:GateA1Shape
  a sh:NodeShape ;
  sh:targetClass cell8:Artifact ;
  sh:property [
    sh:path rdfs:label ;
    sh:minCount 1 ;
    sh:datatype xsd:string ;
  ] ;
  sh:property [
    sh:path cell8:hasState ;
    sh:hasValue cell8:seeded ;
  ] .

# Gate A6 (Seal): Ed25519 signature must be present and non-empty
cell8:GateA6Shape
  a sh:NodeShape ;
  sh:targetClass cell8:Receipt ;
  sh:property [
    sh:path cell8:hasSignature ;
    sh:minCount 1 ;
    sh:datatype xsd:string ;
    sh:minLength 1 ;  # non-empty
  ] .

# Gate A13 (Attest): External witness approval recorded
cell8:GateA13Shape
  a sh:NodeShape ;
  sh:targetClass cell8:Artifact ;
  sh:property [
    sh:path cell8:approvedBy ;
    sh:minCount 1 ;  # at least one witness required
    sh:nodeKind sh:IRI ;
  ] .
```

## Validation Execution

```bash
# Validate TTL file against shapes
onto validate ontology/cell8-manufacturing.ttl --shapes ontology/cell8-shapes.ttl
# Exit 0 = valid | Exit 1+ = violations

# Get detailed report
onto validate ontology/cell8-manufacturing.ttl --shapes ontology/cell8-shapes.ttl --format json
```

Output (JSON format):

```json
{
  "conforms": false,
  "results": [
    {
      "resultSeverity": "sh:Violation",
      "focusNode": "cell8:artifact-001",
      "resultPath": "cell8:hasSignature",
      "resultMessage": "Property cell8:hasSignature has 0 values, expected at least 1",
      "sourceShape": "cell8:GateA6Shape"
    }
  ]
}
```

## Validation Report Interpretation

| Field | Meaning |
|-------|---------|
| `conforms` | Boolean: true if all shapes pass |
| `results[]` | Array of violations or warnings |
| `resultSeverity` | `sh:Violation` (blocks), `sh:Warning` (advisory) |
| `focusNode` | IRI that failed the shape |
| `resultPath` | Property path that violated the constraint |
| `resultMessage` | Human-readable explanation |
| `sourceShape` | Shape definition that detected the violation |

## Rust Integration

```rust
// src/shacl.rs

use oxigraph::store::Store;

pub fn validate_shapes(
    ontology: &str,
    shapes: &str,
) -> Result<ValidationReport> {
    let store = Store::new()?;
    
    // Load ontology and shapes into store
    store.load_graph(ontology, GraphFormat::Turtle)?;
    store.load_graph(shapes, GraphFormat::Turtle)?;
    
    // Execute SHACL validation
    let report = store.validate()?;
    
    Ok(report)
}
```

## Forbidden Patterns

❌ SHACL shape with no sh:targetClass (undefined scope)
❌ sh:minCount 0 and sh:maxCount 0 (always fails)
❌ sh:closed true without documenting all allowed properties
❌ Numeric constraints with wrong datatype (minInclusive on string)
❌ Validation skipped in pipeline (must always validate before emit)

## Required Patterns

✅ Every shape has sh:targetClass or sh:targetNode
✅ Every constraint has clear sh:message documentation
✅ Validation run before code generation (μ₄ stage)
✅ Violations block artifact emission (fail-fast)
✅ Validation report logged as evidence

## Commands

```bash
# Validate ontology against shapes
onto validate ontology/cell8-manufacturing.ttl

# Validate with detailed JSON output
onto validate ontology/cell8-manufacturing.ttl --format json > validation-report.json

# Run validation as part of build
make check        # includes shape validation
make adversarial  # full validation + dead-param-gate
```
