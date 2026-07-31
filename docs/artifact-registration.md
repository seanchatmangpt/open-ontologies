# Artifact Registration Procedure

After ggen emits artifacts (μ₅ stage), the artifact registration subsystem automatically registers them in the open-ontologies RDF store with full traceability.

## Overview

The registration procedure:

1. **Hash each artifact** using BLAKE3 (immutable proof of content)
2. **Parse ggen receipt** JSON to extract operation metadata
3. **Build SPARQL INSERT** query with artifact and receipt assertions
4. **Load into RDF store** in `onto:artifact-registry` named graph
5. **Link to source** ontology, SPARQL queries, and Tera templates

Result: Queryable artifact provenance with cryptographic proof of content and authorship.

## Architecture

```
ggen execution (μ₁–μ₅)
  ├─ μ₁: Load ontology
  ├─ μ₂: Execute SPARQL queries
  ├─ μ₃: Render Tera templates
  ├─ μ₄: Validate with SHACL
  ├─ μ₅: Emit artifacts
  └─ Generate receipt (.ggen/receipts/latest.json)
      │
      ↓
  Artifact Registry
  ├─ Hash each artifact (BLAKE3)
  ├─ Parse receipt metadata
  ├─ Build SPARQL INSERT
  ├─ Load into onto:artifact-registry
  └─ Emit lineage trace (OTEL span)
      │
      ↓
  Artifact RDF Triples
  ├─ urn:ggen:artifact:<path> — artifact IRI
  ├─ ggen:path — file path
  ├─ ggen:fileHash — BLAKE3 hash
  ├─ ggen:receipt — linked receipt IRI
  ├─ ggen:generatedAt — timestamp
  ├─ ggen:sourceTTL — source ontology file(s)
  ├─ ggen:queryUsed — SPARQL query file
  ├─ ggen:templateUsed — Tera template file
  └─ ggen:mimeType — MIME type of artifact
```

## SPARQL Query

Location: `.specify/queries/register-ggen-artifacts.rq`

**Purpose:** INSERT artifact and receipt RDF assertions into `onto:artifact-registry` named graph.

**Input bindings** (provided by registration procedure):
- `?path` — artifact file path (e.g., `"src/cmds/generated.rs"`)
- `?hash` — BLAKE3 hash of artifact (hex string, 64 chars)
- `?timestamp` — generation timestamp (xsd:dateTime)
- `?sourceTTL` — source TTL file(s) used (pipe-separated)
- `?query` — SPARQL query file path
- `?template` — Tera template file path
- `?mimeType` — MIME type of artifact
- `?operationId` — ggen operation UUID
- `?signature` — Ed25519 signature (base64)
- `?previousReceiptHash` — BLAKE3 hash of previous receipt (or `undef`)
- `?artifactCount` — total artifacts in operation

**Output bindings:**
- `?artifact` — generated artifact IRI: `urn:ggen:artifact:<path-with-colons>`
- `?receipt` — generated receipt IRI: `urn:ggen:receipt:<operation-id>`

**Assertions created:**
```turtle
# Artifact node
?artifact a ggen:Artifact ;
  ggen:path ?path ;
  ggen:fileHash ?hash ;
  ggen:hashAlgorithm "blake3" ;
  ggen:receipt ?receipt ;
  ggen:generatedAt ?timestamp ;
  ggen:sourceTTL ?sourceTTL ;
  ggen:queryUsed ?query ;
  ggen:templateUsed ?template ;
  ggen:mimeType ?mimeType ;
  ggen:registeredAt ?registeredAt ;
  rdfs:label "Artifact: <path>" ;
  rdfs:comment "Generated artifact registered with BLAKE3 hash..." .

# Receipt node
?receipt a ggen:Receipt ;
  ggen:operationId ?operationId ;
  ggen:signature ?signature ;
  ggen:previousReceiptHash ?previousReceiptHash ;
  ggen:artifactCount ?artifactCount ;
  dct:issued ?timestamp .
```

## Rust Module

Location: `src/artifact_registry.rs`

### Public API

#### `hash_artifact(path: &Path) -> Result<String>`

Compute BLAKE3 hash of an artifact file.

```rust
use open_ontologies::artifact_registry::hash_artifact;
use std::path::Path;

let hash = hash_artifact(Path::new("src/cmds/generated.rs"))?;
assert_eq!(hash.len(), 64); // BLAKE3 hex is 64 chars
```

#### `build_artifact_record(...) -> Result<ArtifactRecord>`

Build a single artifact registration record from file + receipt.

```rust
use open_ontologies::artifact_registry::build_artifact_record;
use std::path::Path;

let record = build_artifact_record(
    "src/cmds/generated.rs",
    Path::new(".ggen/receipts/latest.json"),
    "ontology/cli-open-ontologies.ttl",
    ".specify/queries/extract-commands.rq",
    ".specify/templates/cli.tera",
)?;

println!("Path: {}", record.path);
println!("Hash: {}", record.hash);
println!("MIME: {}", record.mime_type);
```

#### `build_sparql_registration_query(artifacts: &[ArtifactRecord]) -> Result<String>`

Generate SPARQL INSERT query for batch registration.

```rust
use open_ontologies::artifact_registry::{build_artifact_record, build_sparql_registration_query};

let records = vec![
    build_artifact_record(...)?,
    build_artifact_record(...)?,
];

let sparql = build_sparql_registration_query(&records)?;
// sparql can be POSTed to Oxigraph
```

#### `async register_artifacts(...) -> Result<usize>`

Register all artifacts into Oxigraph RDF store.

```rust
use open_ontologies::artifact_registry::{build_artifact_record, register_artifacts};

let artifacts = vec![
    build_artifact_record(...)?,
];

let triple_count = register_artifacts(
    &artifacts,
    "http://localhost:7878",  // Oxigraph endpoint
    "https://open-ontologies.io/onto/artifact-registry",
).await?;

println!("Registered {} triples", triple_count);
```

### Data Structures

#### `ArtifactRecord`

```rust
pub struct ArtifactRecord {
    pub path: String,           // "src/cmds/generated.rs"
    pub hash: String,           // BLAKE3 hex (64 chars)
    pub receipt: ReceiptMetadata,
    pub generated_at: String,   // RFC3339 timestamp
    pub source_ttl: String,     // "ontology/cli-open-ontologies.ttl|ontology/cell8-core.ttl"
    pub query_path: String,     // ".specify/queries/extract-commands.rq"
    pub template_path: String,  // ".specify/templates/cli.tera"
    pub mime_type: String,      // "text/x-rust"
}
```

#### `ReceiptMetadata`

```rust
pub struct ReceiptMetadata {
    pub operation_id: String,        // UUID of ggen operation
    pub signature: String,           // Ed25519 signature (base64)
    pub previous_receipt_hash: Option<String>,  // Chain link (hex)
    pub timestamp: String,           // RFC3339
    pub artifact_count: usize,       // Total artifacts in operation
}
```

## Integration with ggen Pipeline

### Step 1: ggen Emits Artifacts

```bash
$ ggen sync --audit true
```

**Output:**
- `src/cmds/generated.rs` (and other artifacts)
- `.ggen/receipts/latest.json` with metadata

### Step 2: Post-Emission Registration

Called automatically after ggen μ₅ completes:

```rust
// In ggen pipeline post-emission hook
let receipt_path = Path::new(".ggen/receipts/latest.json");
let artifacts = vec![
    build_artifact_record(
        "src/cmds/generated.rs",
        receipt_path,
        "ontology/cli-open-ontologies.ttl",
        ".specify/queries/extract-commands.rq",
        ".specify/templates/cli.tera",
    )?,
];

let triple_count = register_artifacts(
    &artifacts,
    "http://localhost:7878",
    "https://open-ontologies.io/onto/artifact-registry",
).await?;

println!("Registered {} triples in artifact-registry", triple_count);
```

## Querying Registered Artifacts

### Find artifacts by path

```sparql
PREFIX ggen: <https://ggen.io/onto/ggen/>
PREFIX onto: <https://open-ontologies.io/onto/>

SELECT ?artifact ?hash ?timestamp WHERE {
  GRAPH onto:artifact-registry {
    ?artifact
      a ggen:Artifact ;
      ggen:path "src/cmds/generated.rs"^^xsd:string ;
      ggen:fileHash ?hash ;
      ggen:generatedAt ?timestamp .
  }
}
```

### Find artifacts by source ontology

```sparql
PREFIX ggen: <https://ggen.io/onto/ggen/>
PREFIX onto: <https://open-ontologies.io/onto/>

SELECT ?artifact ?path WHERE {
  GRAPH onto:artifact-registry {
    ?artifact
      a ggen:Artifact ;
      ggen:path ?path ;
      ggen:sourceTTL ?source .
  }
  FILTER(CONTAINS(?source, "cli-open-ontologies"))
}
```

### Find artifacts by Tera template

```sparql
PREFIX ggen: <https://ggen.io/onto/ggen/>
PREFIX onto: <https://open-ontologies.io/onto/>

SELECT ?artifact ?path ?timestamp WHERE {
  GRAPH onto:artifact-registry {
    ?artifact
      a ggen:Artifact ;
      ggen:path ?path ;
      ggen:templateUsed "/.specify/templates/cli.tera"^^xsd:string ;
      ggen:generatedAt ?timestamp .
  }
  ORDER BY DESC(?timestamp)
}
```

### Verify receipt chain integrity

```sparql
PREFIX ggen: <https://ggen.io/onto/ggen/>
PREFIX onto: <https://open-ontologies.io/onto/>

SELECT ?receipt ?operationId ?signature ?previousHash WHERE {
  GRAPH onto:artifact-registry {
    ?artifact
      a ggen:Artifact ;
      ggen:receipt ?receipt .
    
    ?receipt
      a ggen:Receipt ;
      ggen:operationId ?operationId ;
      ggen:signature ?signature ;
      ggen:previousReceiptHash ?previousHash .
  }
}
ORDER BY ?timestamp
```

## OTEL Tracing

Registration emits OTEL spans for observability:

```
onto.artifact.register
  ├─ onto.artifact.count = 5
  ├─ onto.artifact.total_size_bytes = 234567
  ├─ onto.artifact.hash_duration_ms = 45
  ├─ onto.artifact.sparql_insert_duration_ms = 120
  ├─ onto.artifact.registration_duration_ms = 165
  └─ outcome = success | partial | failed
```

### Enabling OTEL Output

```bash
export RUST_LOG=trace,onto=trace
cargo test --test integration_test -- --nocapture 2>&1 | tee otel.txt
grep "onto.artifact.register" otel.txt
```

## Failure Modes

### Missing Artifact File

```
Error: Failed to read artifact src/cmds/missing.rs: No such file or directory
```

**Fix:** Verify artifact path is correct relative to project root.

### Invalid Receipt JSON

```
Error: Invalid receipt JSON: missing field `operation_id` at line 1 column 0
```

**Fix:** Check `.ggen/receipts/latest.json` is valid and not corrupted.

### Oxigraph Connection Failure

```
Error: Failed to POST SPARQL to Oxigraph: error sending request for url
```

**Fix:** Verify Oxigraph is running on `http://localhost:7878` or update endpoint URL.

### SPARQL Query Error

```
Error: Oxigraph returned 400: Parse error at line 5
```

**Fix:** Check SPARQL INSERT syntax in `.specify/queries/register-ggen-artifacts.rq`.

## Cell8 Conformance

Artifact registration enforces Cell8 Gate A7 (Emit):

```
Gate A7: Emit
  ├─ Artifact exists at declared path: ✓ (verified by hash_artifact)
  ├─ BLAKE3 hash computed and stored: ✓ (computed during registration)
  ├─ Format correct (MIME type set): ✓ (inferred from extension)
  ├─ Receipt linked (operation_id present): ✓ (from ggen receipt)
  ├─ Previous receipt linked (chain): ✓ (previous_receipt_hash from ggen)
  └─ RDF assertions in artifact-registry: ✓ (via SPARQL INSERT)
```

All A7 requirements are satisfied by the registration procedure.

## Maintenance & Cleanup

### Export Artifact Registry

```bash
# Query all artifacts registered
onto query select --sparql '
SELECT * WHERE {
  GRAPH <https://open-ontologies.io/onto/artifact-registry> {
    ?s ?p ?o .
  }
}' > artifacts.nt
```

### Clear Registry (Full Reset)

```bash
onto clear --graph https://open-ontologies.io/onto/artifact-registry
```

### Inspect Registry Statistics

```bash
onto stats
```

## Versioning

The artifact registry module follows semantic versioning:

- **v1.0.0** — Initial release with BLAKE3 hashing, receipt parsing, SPARQL insertion
- **v1.1.0** (planned) — Add signature verification via Ed25519 public key
- **v1.2.0** (planned) — Add OTEL metrics and distributed tracing integration

---

**See also:**
- `src/artifact_registry.rs` — Implementation
- `.specify/queries/register-ggen-artifacts.rq` — SPARQL query
- `ontology/ggen-integration-law.ttl` — Manufacturing contract
- `.ggen/receipts/latest.json` — Receipt structure
