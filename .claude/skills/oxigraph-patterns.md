---
name: Oxigraph RDF Store Patterns
description: GraphStore API, SPARQL execution, triple patterns, query optimization
paths: ["src/graph.rs", "src/query.rs"]
type: skill
---

# Skill: Oxigraph Patterns

## Purpose

Use the Oxigraph triple store for RDF storage, querying, and SPARQL execution in Rust code.

## Oxigraph Core API

```rust
use oxigraph::store::Store;
use oxigraph::model::{NamedNode, Literal, Triple, Subject, Predicate, Object};
use oxigraph::io::GraphFormat;

// Create in-memory store
let store = Store::new()?;

// Load TTL file
let ttl_data = std::fs::read_to_string("ontology.ttl")?;
store.load_graph(
    ttl_data.as_bytes(),
    GraphFormat::Turtle,
    None,
)?;

// Query and iterate
let query = r#"
    SELECT ?s ?o WHERE {
        ?s rdfs:label ?o .
    }
"#;

for binding in store.query(query)? {
    let binding = binding?;
    let s = binding.get("s")?;
    let o = binding.get("o")?;
    println!("{} -> {}", s, o);
}
```

## Triple Construction

```rust
use oxigraph::model::*;

// Create IRI
let class_iri = NamedNode::new("https://example.com/MyClass")?;

// Create literal
let label = Literal::new_simple_literal("My Class");

// Create triple
let triple = Triple::new(
    class_iri.clone(),
    NamedNode::new("http://www.w3.org/2000/01/rdf-schema#label")?,
    label,
);

// Insert into store
store.insert(&triple)?;
```

## Query Patterns

### SELECT (get specific bindings)

```rust
let query = r#"
    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
    PREFIX owl: <http://www.w3.org/2002/07/owl#>
    
    SELECT ?class ?label WHERE {
        ?class a owl:Class ;
               rdfs:label ?label .
    }
"#;

let results = store.query(query)?;
for row in results {
    let row = row?;
    println!("Class: {}, Label: {}", row.get("class")?, row.get("label")?);
}
```

### CONSTRUCT (create new RDF data)

```rust
let query = r#"
    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
    
    CONSTRUCT {
        ?s rdfs:comment "Generated comment" .
    }
    WHERE {
        ?s a owl:Class .
    }
"#;

// CONSTRUCT returns triples
let quads = store.query(query)?;
for quad in quads {
    let triple: Triple = quad?.into();
    println!("{}", triple);
}
```

### ASK (boolean existence check)

```rust
let query = r#"
    ASK WHERE {
        ?s a owl:Class ;
           rdfs:label "MyClass" .
    }
"#;

let exists = store.query(query)?;
// exists: bool
```

### DESCRIBE (get all triples about a subject)

```rust
let query = r#"
    DESCRIBE ?s WHERE {
        ?s a owl:Class .
        LIMIT 1
    }
"#;

let triples = store.query(query)?;
```

## Graph Operations

```rust
// Count triples
let count: u64 = store.query(
    "SELECT (COUNT(*) AS ?count) WHERE { ?s ?p ?o }"
)?
    .next()
    .unwrap()
    .get("count")?
    .as_literal()?
    .value()
    .parse()?;

// Get all subjects of a type
let query = r#"
    SELECT DISTINCT ?s WHERE {
        ?s a owl:Class .
    }
"#;

let subjects = store.query(query)?
    .collect::<Result<Vec<_>>>()?;

// Pattern matching
let store_iter = store.quads_for_pattern(
    Some(&class_iri.into()),
    Some(&NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?.into()),
    Some(&NamedNode::new("http://www.w3.org/2002/07/owl#Class")?.into()),
    None,
)?;

for quad in store_iter {
    println!("{}", quad?);
}
```

## Validation with SHACL

```rust
use oxigraph::store::Store;
use oxigraph::sparql::EvaluationError;

// Load ontology and shapes
store.load_graph(ontology, GraphFormat::Turtle, None)?;
store.load_graph(shapes, GraphFormat::Turtle, None)?;

// Run SHACL validation
let conforms = store.query(
    r#"
    ASK WHERE {
        [] sh:conforms true .
    }
    "#
)?;

if !conforms {
    // Get violation details
    let violations = store.query(
        r#"
        SELECT ?focusNode ?resultMessage WHERE {
            ?report a sh:ValidationReport ;
                    sh:result ?result .
            ?result sh:focusNode ?focusNode ;
                    sh:resultMessage ?resultMessage .
        }
        "#
    )?;
    
    for row in violations {
        let row = row?;
        eprintln!("Violation: {}: {}", 
            row.get("focusNode")?, 
            row.get("resultMessage")?
        );
    }
}
```

## Performance Optimization

### Index Key Patterns

Oxigraph automatically indexes:
- Triple patterns with bound subject
- Triple patterns with bound predicate
- Query results (no table scans)

### Query Optimization Tips

```rust
// ✅ GOOD: Bind specific subjects early
let query = r#"
    SELECT ?p ?o WHERE {
        <urn:example:specific-subject> ?p ?o .
    }
"#;

// ❌ INEFFICIENT: Unbounded subject
let query = r#"
    SELECT ?s ?p ?o WHERE {
        ?s ?p ?o .
    }
"#;
// Avoid on large graphs; use LIMIT or filter

// ✅ GOOD: Filter before construct
let query = r#"
    CONSTRUCT {
        ?s rdfs:comment "Active" .
    }
    WHERE {
        ?s a owl:Class ;
           owl:deprecated false .
    }
"#;
```

## Error Handling

```rust
use oxigraph::sparql::EvaluationError;

match store.query(sparql) {
    Ok(results) => {
        for row in results {
            match row {
                Ok(binding) => { /* process */ },
                Err(e) => eprintln!("Binding error: {}", e),
            }
        }
    }
    Err(EvaluationError::ParseError(_)) => {
        eprintln!("SPARQL syntax error");
    }
    Err(EvaluationError::NotImplemented(feature)) => {
        eprintln!("Feature not supported: {}", feature);
    }
    Err(e) => eprintln!("Evaluation error: {}", e),
}
```

## Commands

```bash
# Query store via CLI
onto query select --sparql "SELECT ?s WHERE { ?s a owl:Class }"

# Validate against shapes
onto validate ontology.ttl --shapes shapes.ttl

# Export store
onto export --format ntriples > dump.nt

# Load from remote
onto load remote https://example.com/ontology.ttl
```
