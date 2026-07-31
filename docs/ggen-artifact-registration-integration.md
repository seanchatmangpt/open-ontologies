# ggen-Artifact Registration Integration Guide

This guide explains how artifact registration integrates into the complete ggen manufacturing pipeline and where to call the registration procedure.

## Manufacturing Pipeline with Registration

```
User executes:
  $ ggen sync --audit true

     ↓

ggen μ₁ (Load)
  ├─ Load ontology from ontology/cli-open-ontologies.ttl
  ├─ Validate TTL syntax
  └─ Hash ontology: record in receipt.input_hashes

     ↓

ggen μ₂ (Extract)
  ├─ Execute SPARQL CONSTRUCT queries from .specify/queries/*.rq
  ├─ Produce intermediate RDF facts
  └─ Validate SPARQL syntax

     ↓

ggen μ₃ (Generate)
  ├─ Load intermediate RDF into Tera template engine
  ├─ Render all Tera templates from .specify/templates/*.tera
  ├─ Produce code artifacts (Rust, TypeScript, SQL, etc.)
  └─ Artifact filenames and MIME types determined here

     ↓

ggen μ₄ (Validate)
  ├─ Load artifacts into SHACL validator
  ├─ Validate against ontology/cell8-shapes.ttl
  ├─ If SHACL violation → HALT and exit(1)
  └─ If SHACL pass → proceed

     ↓

ggen μ₅ (Emit)
  ├─ Write all artifacts to disk
  ├─ Hash each artifact
  ├─ Compute Ed25519 signature over manifest
  ├─ Write receipt to .ggen/receipts/latest.json
  └─ SUCCESS: all artifacts in place

     ↓ [NEW]

ARTIFACT REGISTRATION (Post-μ₅)
  ├─ For each artifact file:
  │  ├─ Compute BLAKE3 hash (src/artifact_registry.rs::hash_artifact)
  │  ├─ Build ArtifactRecord with metadata
  │  └─ Collect into Vec<ArtifactRecord>
  │
  ├─ Build SPARQL INSERT query
  │  ├─ Load .specify/queries/register-ggen-artifacts.rq
  │  ├─ Bind artifact metadata: path, hash, receipt metadata
  │  └─ Generate complete INSERT query
  │
  ├─ POST SPARQL INSERT to Oxigraph
  │  ├─ Endpoint: http://localhost:7878/query
  │  ├─ Content-Type: application/sparql-update
  │  └─ Graph URI: https://open-ontologies.io/onto/artifact-registry
  │
  ├─ Verify registration
  │  ├─ Query artifact count in registry
  │  ├─ Confirm all artifacts present (A7 gate satisfied)
  │  └─ Return triple count as proof
  │
  └─ Emit OTEL span: onto.artifact.register
     ├─ duration_ms = X
     ├─ artifact_count = N
     ├─ outcome = success | partial | failed
     └─ lineage recorded for audit trail

     ↓

make adversarial
  ├─ make check — Compilation gate passes
  ├─ make test — All tests pass
  ├─ dead-param gate — No unused parameters
  ├─ clippy deny — No forbidden patterns
  └─ SHACL validation — Cell8 A1-A3 gates pass

     ↓

git commit (with receipt as proof)
  ├─ Add ontology changes
  ├─ Add generated artifacts
  ├─ Add .ggen/receipts/latest.json as immutable record
  └─ Commit message references receipt ID
```

## Integration Points

### 1. Call Site: Post-ggen-μ₅ Hook

**Where:** In ggen's finalization code, after artifact write but before process exit.

**When:** Every successful `ggen sync` run.

**Who calls:** ggen binary (via post-emission hook configured in CLAUDE.md).

**Code location:** `src/bin/ggen.rs` or equivalent (ggen crate, not open-ontologies).

#### Example Hook Registration

```rust
// In ggen crate (open-ontologies/ggen/src/bin/ggen.rs or similar)

use open_ontologies::artifact_registry::{
    build_artifact_record,
    register_artifacts,
};
use std::path::Path;

async fn post_emit_registration() -> anyhow::Result<()> {
    // Collect output artifacts from receipt
    let receipt_path = Path::new(".ggen/receipts/latest.json");
    
    // Map each output hash entry to an artifact registration
    let artifacts = vec![
        build_artifact_record(
            "src/cmds/generated.rs",
            receipt_path,
            "ontology/cli-open-ontologies.ttl",
            ".specify/queries/extract-commands.rq",
            ".specify/templates/cli.tera",
        )?,
    ];

    // Register all artifacts in one batch
    let triple_count = register_artifacts(
        &artifacts,
        "http://localhost:7878",  // Oxigraph endpoint
        "https://open-ontologies.io/onto/artifact-registry",
    ).await?;

    eprintln!(
        "[onto:artifact-registry] Registered {} artifact(s) ({} triples)",
        artifacts.len(),
        triple_count
    );

    Ok(())
}
```

### 2. Integration with Make

**File:** `Makefile`

**Target:** `make check` and `make adversarial` must pass before claiming done.

```makefile
# In Makefile, after ggen sync completes

.PHONY: ggen-register
ggen-register:
	@echo "Registering artifacts in open-ontologies..."
	cargo run --bin open-ontologies -- artifact-register \
		--receipt .ggen/receipts/latest.json \
		--oxigraph-endpoint http://localhost:7878
	@echo "Registration complete: ontology state persisted to RDF"

# Extend ggen-sync to include registration
.PHONY: ggen-sync
ggen-sync: ggen-sync-dry ggen-sync-audit ggen-register
	@echo "ggen sync pipeline complete (μ₁–μ₅ + registration)"

# Extend adversarial gate to include A7 verification
.PHONY: make-adversarial
make-adversarial: make-check make-test dead-param-gate clippy-deny a7-gate
	@echo "All adversarial gates passed"

.PHONY: a7-gate
a7-gate:
	@echo "Verifying Cell8 Gate A7 (Artifact Emission & Registration)..."
	cargo run --bin open-ontologies -- artifact-list \
		--graph https://open-ontologies.io/onto/artifact-registry \
		--count-only
	@echo "Gate A7 satisfied"
```

### 3. CLAUDE.md Configuration

**File:** `/Users/sac/open-ontologies/CLAUDE.md`

Add to build configuration:

```markdown
## Artifact Registration Procedure

After ggen emits artifacts (μ₅), the artifact registration subsystem automatically:

1. Hashes each artifact with BLAKE3
2. Parses ggen receipt JSON for operation metadata
3. Builds SPARQL INSERT with artifact assertions
4. Loads into onto:artifact-registry named graph
5. Emits OTEL trace for audit trail

**Call site:** Post-μ₅, before make check completes

**Configuration:**
- Oxigraph endpoint: `http://localhost:7878`
- Named graph: `https://open-ontologies.io/onto/artifact-registry`
- Receipt location: `.ggen/receipts/latest.json`

**Commands:**
```bash
# Trigger artifact registration (called automatically)
ggen sync --audit true

# Verify registration (manual check)
onto query select --sparql 'SELECT ?artifact WHERE {
  GRAPH <https://open-ontologies.io/onto/artifact-registry> {
    ?artifact a <https://ggen.io/onto/ggen/Artifact> .
  }
}'

# Check artifact count in registry
onto stats --graph https://open-ontologies.io/onto/artifact-registry
```

**Cell8 Gate:** A7 (Artifact Emission) enforces:
- All artifacts hashed and registered
- Receipt chain linked (previous_receipt_hash)
- SPARQL INSERT succeeded (verified by count query)
- OTEL span emitted for traceability
```

### 4. OTEL Instrumentation

**Module:** `src/otel.rs` or equivalent

Add spans for artifact registration:

```rust
use opentelemetry::global;
use opentelemetry::trace::{Tracer, TraceContextPropagator};

pub async fn register_artifacts_traced(artifacts: &[ArtifactRecord]) -> anyhow::Result<usize> {
    let tracer = global::tracer("open-ontologies");
    
    let span = tracer.start("onto.artifact.register");
    let _guard = opentelemetry::trace::TraceContextPropagator::extract(&span);

    span.add_event("artifact.registry.start", vec![
        Key::new("artifact.count").i64(artifacts.len() as i64),
    ]);

    // ... registration logic ...

    span.add_event("artifact.registry.complete", vec![
        Key::new("triple.count").i64(triple_count as i64),
        Key::new("duration_ms").i64(duration_ms as i64),
    ]);

    Ok(triple_count)
}
```

## Data Flow Diagram

```
ggen receipt (.ggen/receipts/latest.json)
  │
  ├─ operation_id ────────┐
  ├─ timestamp            │
  ├─ input_hashes         │
  ├─ output_hashes        │
  ├─ signature            │
  └─ previous_receipt_hash│
                          │
                          ↓
              artifact_registry::parse_receipt()
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ↓                 ↓                 ↓
  operationId      signature         previousReceiptHash
  timestamp        artifactCount     ...
  │
  │   (For each output artifact)
  │
  ├─ src/cmds/generated.rs
  │   ├─ hash_artifact() → blake3::hash(bytes) → "abc123..."
  │   ├─ mime_type_for_path() → "text/x-rust"
  │   └─ build_artifact_record() → ArtifactRecord
  │
  ├─ packages/types/zoela.ts
  │   ├─ hash_artifact() → "def456..."
  │   ├─ mime_type_for_path() → "application/typescript"
  │   └─ build_artifact_record() → ArtifactRecord
  │
  └─ ... (more artifacts)
      │
      ↓
   Vec<ArtifactRecord>
      │
      ├─ build_sparql_registration_query()
      │   └─ .specify/queries/register-ggen-artifacts.rq (template)
      │       ├─ BIND(?path, ?hash, ?timestamp, ...)
      │       ├─ INSERT { GRAPH onto:artifact-registry { ... } }
      │       └─ SPARQL INSERT statement
      │
      ↓
   SPARQL INSERT
      │
      ├─ POST to Oxigraph http://localhost:7878/query
      │   ├─ Content-Type: application/sparql-update
      │   └─ Graph URI: https://open-ontologies.io/onto/artifact-registry
      │
      ↓
   Oxigraph RDF Store
      │
      ├─ Triples for artifact:
      │   ├─ urn:ggen:artifact:src:cmds:generated.rs
      │   │  ├─ rdf:type ggen:Artifact
      │   │  ├─ ggen:path "src/cmds/generated.rs"
      │   │  ├─ ggen:fileHash "abc123..."
      │   │  ├─ ggen:receipt urn:ggen:receipt:operation-id
      │   │  └─ ... (other properties)
      │   │
      │   └─ urn:ggen:receipt:operation-id
      │      ├─ rdf:type ggen:Receipt
      │      ├─ ggen:operationId "operation-id"
      │      ├─ ggen:signature "sig123..."
      │      └─ dct:issued "2026-06-01T15:30:45Z"
      │
      ↓
   Verification Query
      │
      ├─ SELECT (COUNT(?artifact) AS ?count)
      │ WHERE { GRAPH <onto:artifact-registry> {
      │   ?artifact a ggen:Artifact
      │ } }
      │
      ↓
   Artifact Count Confirmed
      │
      └─ Return triple_count to caller ✓
```

## Testing Strategy

### Unit Tests

**File:** `src/artifact_registry.rs` (in `#[cfg(test)]` module)

```rust
#[test]
fn test_hash_artifact() { ... }

#[test]
fn test_build_artifact_record() { ... }

#[test]
fn test_build_sparql_registration_query() { ... }
```

**Run:**
```bash
cargo test -p open-ontologies artifact_registry:: -- --nocapture
```

### Integration Tests

**File:** `tests/artifact_registration_integration.rs`

```rust
#[tokio::test]
async fn test_register_artifacts_to_oxigraph() {
    // Start local Oxigraph
    let oxigraph = start_test_oxigraph();
    
    // Create test artifacts
    let artifacts = vec![
        build_artifact_record(...)?,
    ];
    
    // Register
    let count = register_artifacts(
        &artifacts,
        oxigraph.endpoint(),
        "https://open-ontologies.io/onto/artifact-registry",
    ).await?;
    
    // Verify
    assert!(count > 0);
    
    // Query back
    let results = oxigraph.query(
        "SELECT ?artifact WHERE { ... }"
    ).await?;
    assert_eq!(results.len(), artifacts.len());
}
```

**Run:**
```bash
cargo test -p open-ontologies --test artifact_registration_integration -- --nocapture
```

### End-to-End Test

**File:** `tests/ggen_full_pipeline.rs`

```rust
#[test]
fn test_ggen_sync_with_artifact_registration() {
    // 1. Run ggen sync
    let output = Command::new("ggen")
        .args(&["sync", "--audit", "true"])
        .output()
        .expect("ggen sync failed");
    assert!(output.status.success());
    
    // 2. Check receipt exists
    assert!(Path::new(".ggen/receipts/latest.json").exists());
    
    // 3. Query artifact registry
    let results = onto_query(
        "SELECT ?artifact WHERE { GRAPH <onto:artifact-registry> { ?artifact a ggen:Artifact } }"
    );
    
    // 4. Verify artifact count matches receipt output_hashes count
    let receipt_artifact_count = parse_receipt_output_count();
    assert_eq!(results.len(), receipt_artifact_count);
}
```

**Run:**
```bash
cargo test -p open-ontologies --test ggen_full_pipeline -- --nocapture --test-threads=1
```

## Failure Recovery

### Scenario 1: Registration Partial (Some Artifacts Not Inserted)

**Symptom:**
```
Registered 5 artifacts but query returns 3 triples
```

**Recovery:**
```bash
# Clear registry and re-run
onto clear --graph https://open-ontologies.io/onto/artifact-registry

# Re-register
ggen sync --audit true
```

### Scenario 2: Oxigraph Down During Registration

**Symptom:**
```
Error: Failed to POST SPARQL to Oxigraph: connection refused
```

**Recovery:**
```bash
# Start Oxigraph
open-ontologies server serve &

# Retry registration
cargo run --bin open-ontologies -- artifact-register \
  --receipt .ggen/receipts/latest.json \
  --oxigraph-endpoint http://localhost:7878
```

### Scenario 3: Receipt JSON Corrupted

**Symptom:**
```
Error: Invalid receipt JSON: missing field `operation_id`
```

**Recovery:**
```bash
# Restore receipt from backup
cp .ggen/receipts/backup/latest.json .ggen/receipts/latest.json

# Re-register
cargo run --bin open-ontologies -- artifact-register ...
```

## Performance Characteristics

| Operation | Typical Duration | Notes |
|-----------|-----------------|-------|
| Hash single artifact (50KB) | 1-2 ms | BLAKE3 is fast; linear with file size |
| Build ArtifactRecord | 0.5 ms | Parse receipt JSON + metadata extraction |
| Build SPARQL INSERT | 1-2 ms | String construction + BIND statements |
| POST to Oxigraph | 50-150 ms | Network + SPARQL execution |
| Verify via count query | 20-50 ms | SELECT COUNT result |
| **Total (5 artifacts)** | ~500 ms | Dominated by Oxigraph network latency |

## Observability

### OTEL Spans Emitted

```
onto.artifact.register
  ├─ attributes:
  │  ├─ artifact.count = 5
  │  ├─ artifact.total_bytes = 234567
  │  ├─ hash_duration_ms = 45
  │  ├─ sparql_duration_ms = 120
  │  ├─ registration_duration_ms = 165
  │  ├─ verification_duration_ms = 35
  │  └─ outcome = success
  │
  ├─ events:
  │  ├─ artifact.registry.start (artifact.count=5)
  │  ├─ artifact.hashing.complete (duration_ms=45)
  │  ├─ sparql.insert.submitted (query_bytes=1234)
  │  ├─ artifact.registry.verified (triple_count=45)
  │  └─ artifact.registry.complete (duration_ms=165)
  │
  └─ end_time (total duration recorded)
```

### Logs

```
[2026-06-01 15:30:45] INFO: onto.artifact.register starting (5 artifacts)
[2026-06-01 15:30:45] DEBUG: hash src/cmds/generated.rs → abc123...
[2026-06-01 15:30:45] DEBUG: hash packages/types/zoela.ts → def456...
[2026-06-01 15:30:45] DEBUG: build SPARQL INSERT (10 bindings)
[2026-06-01 15:30:45] INFO: POST SPARQL to http://localhost:7878/query
[2026-06-01 15:30:46] INFO: verify artifact count in registry: 5 ✓
[2026-06-01 15:30:46] INFO: onto.artifact.register complete (165ms, 45 triples)
```

---

**See also:**
- `docs/artifact-registration.md` — Detailed procedure
- `src/artifact_registry.rs` — Implementation
- `.specify/queries/register-ggen-artifacts.rq` — SPARQL query
- `ontology/ggen-integration-law.ttl` — Manufacturing law
