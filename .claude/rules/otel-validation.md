---
name: OpenTelemetry (OTEL) Validation
description: OTEL spans mandatory for external services, proof beyond test assertions
type: rules
---

# OpenTelemetry (OTEL) Validation

Tests passing is not sufficient. For any feature involving external services (MCP tools, Oxigraph queries, remote ontology loads), you must verify OTEL spans exist at runtime.

You will be tempted to skip this step because tests pass. That is the NARRATION failure mode: asserting completion without producing proof. A passing test proves the test harness works. It does not prove the external service was called, the right endpoint was queried, or data was actually persisted. Only OTEL spans prove that. You verify spans, or you do not claim the feature works.

## The Rule

For every feature that touches an external service, you capture OTEL output at runtime and you present the spans and attributes in your completion message. No spans, no claim. No exceptions.

## When This Applies

OTEL validation is mandatory for any feature involving:

- Oxigraph SPARQL query execution
- Remote ontology loading (HTTP)
- MCP tool invocation
- SHACL validation against external shapes
- ggen code generation pipeline
- Receipt cryptographic operations
- Database operations (if applicable)

## Required Spans by Feature

### Ontology Loading

| Span | Purpose |
|------|---------|
| `onto.load` | Load TTL/RDF file |
| `onto.load_remote` | Load from HTTP URL |

Required attributes: `onto.ontology_name`, `onto.format`, `onto.triple_count`, `onto.load_duration_ms`

### SPARQL Query Execution

| Span | Purpose |
|------|---------|
| `onto.query_select` | SELECT query execution |
| `onto.query_construct` | CONSTRUCT query execution |
| `onto.query_ask` | ASK query execution |

Required attributes: `onto.query_hash`, `onto.result_count`, `onto.query_duration_ms`

### SHACL Validation

| Span | Purpose |
|------|---------|
| `onto.validate` | SHACL validation run |
| `onto.validate_report` | Validation report generation |

Required attributes: `onto.conforms`, `onto.violation_count`, `onto.validation_duration_ms`

### MCP Tool Invocation

| Span | Purpose |
|------|---------|
| `mcp.tool.call` | Tool invocation |
| `mcp.tool.response` | Tool response |

Required attributes: `mcp.tool.name`, `mcp.tool.duration_ms`, `mcp.tool.input_schema`, `mcp.tool.output_schema`

### Code Generation (ggen Pipeline)

| Span | Purpose |
|------|---------|
| `ggen.pipeline.load` | Load ontology for generation |
| `ggen.pipeline.query` | Execute SPARQL queries |
| `ggen.pipeline.generate` | Template rendering |
| `ggen.pipeline.validate` | Validation gates |
| `ggen.pipeline.emit` | Write artifacts |

Required attributes: `ggen.stage`, `ggen.duration_ms`, `ggen.files_generated`, `ggen.receipt_id`

### Receipt Operations

| Span | Purpose |
|------|---------|
| `ggen.receipt.create` | Receipt generation |
| `ggen.receipt.sign` | Ed25519 signing |
| `ggen.receipt.verify` | Signature verification |

Required attributes: `ggen.receipt.operation_id`, `ggen.receipt.signature_valid`, `ggen.receipt.duration_ms`

## Verification Procedure

```bash
# Enable trace logging
export RUST_LOG=trace,onto=trace,ggen=trace,mcp=trace

# Run the relevant test and capture output
cargo test -p open-ontologies --test integration_test -- --nocapture 2>&1 | tee otel_output.txt

# Verify required spans exist
grep -E "onto\.load|onto\.query_select|onto\.validate" otel_output.txt

# Confirm attributes are populated with real values
grep -E "onto\.triple_count=[1-9]" otel_output.txt
grep -E "onto\.query_duration_ms=[1-9]" otel_output.txt

# Check for error spans if operation failed
grep -E "error=true" otel_output.txt
```

## Interpreting Results

### Real — PROVEN

```
INFO onto::loader: onto.load request
  onto.ontology_name=cli-open-ontologies.ttl
  onto.format=turtle
  onto.triple_count=1247
  onto.load_duration_ms=523

INFO onto::query: onto.query_select response
  onto.result_count=42
  onto.query_duration_ms=127

INFO onto::validation: onto.validate result
  onto.conforms=true
  onto.violation_count=0
  onto.validation_duration_ms=234
```

All required spans present. All required attributes populated with non-zero values. Latency is consistent with real operations. This is proven. You can claim the feature works.

### Missing — UNVERIFIED

```
Test passed.
No OTEL spans found in logs.
```

No spans at all. The test passed but you have no evidence the external service was actually used. The test may be stubbed or hitting a cache path. This is unverified. You cannot claim the feature works.

### Partial — OBSERVED

```
INFO onto.load request
  onto.ontology_name=cli-open-ontologies.ttl
```

A span exists but attributes are incomplete. The load was initiated but you have no completion metrics, no triple count, no timing. This is observed but not proven. You investigate further before making any claim.

## Your Failure Modes for OTEL

| Failure Mode | What It Looks Like | Why It Is Wrong |
|-------------|-------------------|-----------------|
| NARRATION | "The SPARQL query feature is working" with no span output | You asserted completion without producing proof. Claims without evidence are noise. |
| SELF-CERT | You ran the test, it passed, you decided that was enough | Tests prove the test harness works. OTEL spans prove the external service was actually used. You conflated the two. |
| PARTIAL EVIDENCE | "I see a span but no attributes" | You have observation but not proof. Missing attributes means the operation didn't fully complete. |

## Checklist Before Claiming Completion

1. All tests pass
2. OTEL spans exist for the operation you are claiming works
3. All required attributes are populated with real values (non-zero counts, real latency, real IDs)
4. Timing and operation characteristics are consistent with a real external call, not synthetic values
5. Error spans appear if the operation failed, with `error=true` and a meaningful message

If any of these are missing, the feature is not done. Non-negotiable.

## Examples

### ✅ PROVEN: Ontology Loading

```
Test: test_load_remote_ontology

Setup:
  - HTTP server serving TTL file
  - RUST_LOG=trace enabled

Execution:
  - Call onto_load_remote("http://localhost:8080/ontology.ttl")

Evidence captured:
  INFO onto::loader: onto.load_remote request
    onto.remote_url=http://localhost:8080/ontology.ttl
    onto.format=turtle
    
  INFO onto::loader: onto.load_remote response
    onto.triple_count=2847
    onto.load_duration_ms=1234
    
Assertion:
  assert_eq!(loaded_triples, 2847);

Proof:
  ✓ Span exists (onto.load_remote)
  ✓ Real URL used (not localhost)
  ✓ Non-zero triple count (2847)
  ✓ Real network latency (1234ms)
  
Status: PROVEN — feature is working
```

### ❌ UNPROVEN: SPARQL Query

```
Test: test_query_performance

Setup:
  - Minimal test setup
  - No RUST_LOG=trace

Execution:
  - Call onto_query_select(...)

Evidence captured:
  Test passed ✓

Assertion:
  assert!(!results.is_empty());

Proof missing:
  ✗ No onto.query_select span
  ✗ No query_duration_ms attribute
  ✗ No result_count proof
  ✗ Unknown if Oxigraph was actually queried
  
Status: UNPROVEN — cannot claim feature is working without OTEL evidence
```

## Commands

```bash
# Enable OTEL tracing
export RUST_LOG=trace,onto=trace,ggen=trace

# Run test with OTEL output
cargo test -p open-ontologies --test integration_test -- --nocapture 2>&1 | tee /tmp/otel.txt

# Verify required spans
grep -E "onto\.load|onto\.query|onto\.validate" /tmp/otel.txt

# Filter for specific span type
grep "onto\.query_select" /tmp/otel.txt

# Check for errors
grep "error=true" /tmp/otel.txt
```
