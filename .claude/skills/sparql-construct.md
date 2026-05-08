---
name: SPARQL CONSTRUCT Patterns
description: SPARQL CONSTRUCT queries for ggen pipeline, result triples
paths: [".specify/**/*.rq", "cell8-ggen/**/*.rq"]
type: skill
---

# Skill: SPARQL CONSTRUCT Patterns

## Purpose

Write SPARQL CONSTRUCT queries that transform RDF ontologies into intermediate representations for the ggen code generation pipeline.

## CONSTRUCT Query Basics

CONSTRUCT queries transform RDF data into new triple patterns:

```sparql
PREFIX onto: <https://ggen.io/onto/cli/open-ontologies/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX owl: <http://www.w3.org/2002/07/owl#>

CONSTRUCT {
  ?class rdfs:label ?label ;
         owl:subClassOf ?superClass .
}
WHERE {
  ?class a owl:Class ;
         rdfs:label ?label ;
         rdfs:subClassOf ?superClass .
}
```

Returns: New triple set matching the CONSTRUCT pattern for all ?class, ?label, ?superClass bindings.

## Pipeline Integration

The ggen pipeline uses SPARQL CONSTRUCT to:

1. **μ₁ (Load)** — Query TTL files
2. **μ₂ (Extract)** — CONSTRUCT intermediate facts
3. **μ₃ (Generate)** — Tera templates consume CONSTRUCT results
4. **μ₄ (Validate)** — SHACL shapes validate intermediate
5. **μ₅ (Emit)** — Write generated files

Example flow:

```
cli-open-ontologies.ttl (source)
  ↓ (SPARQL CONSTRUCT)
command-facts.rdf (intermediate)
  ↓ (Tera template)
src/cmds/generated.rs (output)
```

## Common Query Patterns

### Extract All Classes with Descriptions

```sparql
PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

CONSTRUCT {
  ?class rdfs:label ?label ;
         rdfs:comment ?comment ;
         a owl:Class .
}
WHERE {
  ?class a owl:Class ;
         rdfs:label ?label .
  OPTIONAL { ?class rdfs:comment ?comment }
}
```

### Extract Properties with Domain/Range

```sparql
PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

CONSTRUCT {
  ?prop a owl:ObjectProperty ;
        rdfs:domain ?domain ;
        rdfs:range ?range ;
        rdfs:label ?label .
}
WHERE {
  ?prop a owl:ObjectProperty ;
        rdfs:domain ?domain ;
        rdfs:range ?range ;
        rdfs:label ?label .
}
```

### Extract Hierarchy (Infer Transitive)

```sparql
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

CONSTRUCT {
  ?child rdfs:subClassOf ?parent .
}
WHERE {
  {
    ?child rdfs:subClassOf ?parent .
  }
  UNION
  {
    ?child rdfs:subClassOf ?mid .
    ?mid rdfs:subClassOf+ ?parent .
  }
}
```

## Filtering in CONSTRUCT

Use FILTER to constrain results:

```sparql
PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

CONSTRUCT {
  ?class rdfs:label ?label .
}
WHERE {
  ?class a owl:Class ;
         rdfs:label ?label .
  FILTER (regex(?label, "^Onto", "i"))  # Only classes with label starting "Onto"
}
```

## Empty Result Handling

**Problem**: CONSTRUCT returns empty set if no matches.

```sparql
# Returns nothing if no owl:Class found
CONSTRUCT {
  ?class rdfs:label ?label .
}
WHERE {
  ?class a owl:Class ;
         rdfs:label ?label .
}
```

**Solution**: Use OPTIONAL with a default graph pattern:

```sparql
CONSTRUCT {
  ?result rdfs:label "empty" .
}
WHERE {
  OPTIONAL {
    ?class a owl:Class ;
           rdfs:label ?label .
  }
  BIND (IF(BOUND(?class), ?class, <urn:default:empty>) AS ?result)
}
```

## SPARQL in ggen Pipeline (.specify/)

All queries live in `.specify/queries/`:

```
.specify/queries/
├── extract-commands.rq         # Extract CLI commands from ontology
├── extract-options.rq          # Extract CLI options
├── extract-subcommands.rq      # Extract subcommand hierarchy
└── cell8-gates.rq              # Extract Cell8 gate definitions
```

**Example: extract-commands.rq**

```sparql
PREFIX onto: <https://ggen.io/onto/cli/open-ontologies/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

CONSTRUCT {
  ?cmd rdfs:label ?name ;
       rdfs:comment ?desc ;
       onto:hasOption ?opt .
}
WHERE {
  ?cmd a onto:OntologyCommand ;
       rdfs:label ?name ;
       rdfs:comment ?desc .
  OPTIONAL {
    ?cmd onto:hasOption ?opt .
  }
}
```

## Validation

After CONSTRUCT execution, validate:

```bash
# Check result is valid RDF
onto validate --input-format ntriples result.nt

# Check result contains expected triples
rdfquery result.nt "SELECT ?s WHERE { ?s rdfs:label ?o }" | wc -l
# Should match expected count
```

## Forbidden Patterns

❌ CONSTRUCT without WHERE clause
❌ OPTIONAL without BIND default handling
❌ Unbounded queries (risk of cartesian explosion)
❌ Typos in namespace prefixes (prefix not declared)

## Commands

```bash
# Run single CONSTRUCT query
ggen query --sparql @.specify/queries/extract-commands.rq

# Run full pipeline (includes CONSTRUCT)
ggen sync --dry-run true

# Validate CONSTRUCT output
onto validate --input-format ntriples queries/output.nt
```
