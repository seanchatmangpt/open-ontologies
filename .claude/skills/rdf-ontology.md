---
name: RDF Ontology Authoring
description: RDF/OWL/TTL authoring patterns for ontology engineering
paths: ["ontology/**/*.ttl"]
type: skill
---

# Skill: RDF Ontology Authoring

## Purpose

Write and maintain RDF/OWL ontologies in Turtle (.ttl) format. These are the source of truth for all code generation and reasoning tasks.

## Turtle Syntax Essentials

```turtle
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix onto: <https://ggen.io/onto/cli/open-ontologies/> .

# Classes
onto:OntologyCommand
  a owl:Class ;
  rdfs:label "Ontology Command" ;
  rdfs:comment "A command for ontology operations" .

# Properties
onto:hasVerb
  a owl:ObjectProperty ;
  rdfs:domain onto:OntologyCommand ;
  rdfs:range onto:Verb .

# Instances
onto:load_cmd
  a onto:OntologyCommand ;
  onto:hasVerb onto:load ;
  rdfs:label "Load Ontology" .
```

## Common Patterns

### Hierarchies

```turtle
onto:LoadOntology
  a owl:Class ;
  rdfs:subClassOf onto:OntologyCommand .
```

### Property Axioms

```turtle
onto:hasIRI
  a owl:DataProperty ;
  rdfs:domain onto:Ontology ;
  rdfs:range xsd:anyURI ;
  owl:minQualifiedCardinality "1"^^xsd:nonNegativeInteger .
```

### Union Types

```turtle
onto:ValidationTarget
  owl:unionOf (
    onto:Ontology
    onto:DataSource
  ) .
```

## Validation

Always validate after editing:

```bash
onto validate ontology/my-ontology.ttl
# Must exit 0
```

## CRITICAL: Don't Generate, Edit TTL

- ✅ Edit `.ttl` files directly
- ✅ These are the source of truth
- ❌ Do NOT generate `.ttl` from code
- ❌ Do NOT edit generated `.md` documentation

When in doubt: **edit the TTL, regenerate the docs.**
