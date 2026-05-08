---
name: MCP Server Configuration and Tooling
description: 43 onto_* tool catalog, registration, rmcp transport, service deployment
type: rules
---

# MCP Server Configuration and Tooling

## Tool Categories (43 Tools Total)

### 1. Ontology Management (6 tools)
- `onto_load` — Load TTL file into Oxigraph store
- `onto_load_remote` — Load from remote HTTP URL
- `onto_unload` — Remove ontology from store
- `onto_list_ontologies` — Show all loaded ontologies
- `onto_export` — Serialize store to TTL/NTriples/RDF-XML
- `onto_import_namespace` — Import namespace definitions

### 2. SPARQL Query (8 tools)
- `onto_query_select` — Execute SELECT query
- `onto_query_construct` — Execute CONSTRUCT query
- `onto_query_ask` — Execute ASK (boolean) query
- `onto_query_describe` — Execute DESCRIBE query
- `onto_query_load_file` — Load and execute query from file
- `onto_query_list_saved` — List saved query templates
- `onto_query_save` — Save query as reusable template
- `onto_query_delete_saved` — Delete saved query

### 3. SHACL Validation (5 tools)
- `onto_validate` — Validate store against SHACL shapes
- `onto_validate_artifact` — Validate single entity
- `onto_validate_report` — Get JSON validation report
- `onto_shapes_list` — List loaded SHACL shapes
- `onto_shapes_load` — Load additional shapes

### 4. OWL Reasoning (4 tools)
- `onto_reason_infer` — Run OWL 2 reasoner
- `onto_reason_check_consistency` — Check consistency
- `onto_reason_unsatisfiable_classes` — Find unsatisfiable classes
- `onto_reason_equivalence` — Find equivalent entities

### 5. Entity Management (6 tools)
- `onto_entity_get` — Get full entity description
- `onto_entity_search` — Search by label/comment
- `onto_entity_create` — Create new class/property/instance
- `onto_entity_update` — Update entity properties
- `onto_entity_delete` — Remove entity
- `onto_entity_instances` — List instances of class

### 6. Class Operations (4 tools)
- `onto_class_hierarchy` — Get class hierarchy tree
- `onto_class_properties` — List class properties
- `onto_class_restrictions` — Get OWL restrictions
- `onto_class_equivalent` — Find equivalent definitions

### 7. Property Operations (3 tools)
- `onto_property_get` — Get property definition
- `onto_property_domain_range` — Get domain/range constraints
- `onto_property_inverse` — Get inverse property

### 8. Graph Operations (7 tools)
- `onto_graph_triples` — Get all triples (optionally filtered)
- `onto_graph_triple_count` — Count triples
- `onto_graph_pattern_match` — Find matching triples
- `onto_graph_subgraph_extract` — Extract connected subgraph
- `onto_graph_merge` — Merge graphs
- `onto_graph_diff` — Find differences
- `onto_graph_visualize` — Generate Mermaid/DOT diagram

## Tool Router Implementation

All tools registered in `src/server.rs`:

```rust
pub async fn handle_tool_call(name: &str, input: ToolInput) -> Result<ToolOutput> {
    match name {
        // Ontology Management
        "onto_load" => onto_load(input).await,
        "onto_load_remote" => onto_load_remote(input).await,
        "onto_unload" => onto_unload(input).await,
        "onto_list_ontologies" => onto_list_ontologies(input).await,
        "onto_export" => onto_export(input).await,
        "onto_import_namespace" => onto_import_namespace(input).await,

        // SPARQL Query
        "onto_query_select" => onto_query_select(input).await,
        "onto_query_construct" => onto_query_construct(input).await,
        "onto_query_ask" => onto_query_ask(input).await,
        "onto_query_describe" => onto_query_describe(input).await,
        "onto_query_load_file" => onto_query_load_file(input).await,
        "onto_query_list_saved" => onto_query_list_saved(input).await,
        "onto_query_save" => onto_query_save(input).await,
        "onto_query_delete_saved" => onto_query_delete_saved(input).await,

        // SHACL Validation
        "onto_validate" => onto_validate(input).await,
        "onto_validate_artifact" => onto_validate_artifact(input).await,
        "onto_validate_report" => onto_validate_report(input).await,
        "onto_shapes_list" => onto_shapes_list(input).await,
        "onto_shapes_load" => onto_shapes_load(input).await,

        // OWL Reasoning
        "onto_reason_infer" => onto_reason_infer(input).await,
        "onto_reason_check_consistency" => onto_reason_check_consistency(input).await,
        "onto_reason_unsatisfiable_classes" => onto_reason_unsatisfiable_classes(input).await,
        "onto_reason_equivalence" => onto_reason_equivalence(input).await,

        // Entity Management
        "onto_entity_get" => onto_entity_get(input).await,
        "onto_entity_search" => onto_entity_search(input).await,
        "onto_entity_create" => onto_entity_create(input).await,
        "onto_entity_update" => onto_entity_update(input).await,
        "onto_entity_delete" => onto_entity_delete(input).await,
        "onto_entity_instances" => onto_entity_instances(input).await,

        // Class Operations
        "onto_class_hierarchy" => onto_class_hierarchy(input).await,
        "onto_class_properties" => onto_class_properties(input).await,
        "onto_class_restrictions" => onto_class_restrictions(input).await,
        "onto_class_equivalent" => onto_class_equivalent(input).await,

        // Property Operations
        "onto_property_get" => onto_property_get(input).await,
        "onto_property_domain_range" => onto_property_domain_range(input).await,
        "onto_property_inverse" => onto_property_inverse(input).await,

        // Graph Operations
        "onto_graph_triples" => onto_graph_triples(input).await,
        "onto_graph_triple_count" => onto_graph_triple_count(input).await,
        "onto_graph_pattern_match" => onto_graph_pattern_match(input).await,
        "onto_graph_subgraph_extract" => onto_graph_subgraph_extract(input).await,
        "onto_graph_merge" => onto_graph_merge(input).await,
        "onto_graph_diff" => onto_graph_diff(input).await,
        "onto_graph_visualize" => onto_graph_visualize(input).await,

        _ => Err(format!("Unknown tool: {}", name)),
    }
}
```

## RMCP Transport Configuration

Rusty Model Context Protocol (rmcp) supports two transports:

### stdio (for Claude integration)

```bash
# Start server on stdio (default)
onto mcp start --transport stdio

# In Claude Code settings:
{
  "mcp": {
    "servers": {
      "open-ontologies": {
        "command": "onto",
        "args": ["mcp", "start", "--transport", "stdio"]
      }
    }
  }
}
```

### HTTP (for remote access)

```bash
# Start server on port 3050
onto mcp start --transport http --port 3050

# Client request
curl -X POST http://localhost:3050/tools/call \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "onto_query_select",
      "arguments": {
        "sparql": "SELECT ?s WHERE { ?s a owl:Class }"
      }
    }
  }'
```

## Tool Requirements

Every tool MUST:

1. **Be registered in tool_router!** — No orphan tool implementations
2. **Have input validation** — Check required fields, validate IRI syntax
3. **Have output schema** — Serializable to JSON
4. **Handle errors gracefully** — Return detailed error messages
5. **Emit OTEL spans** — Track execution time and tool name
6. **Have integration tests** — Test successful path and error cases

### Implementation Checklist

```
For each tool:
[ ] Function defined in src/cmds/<category>.rs
[ ] Registered in tool_router! macro in src/server.rs
[ ] Input struct with documentation
[ ] Output struct with documentation
[ ] Input validation (required fields, format)
[ ] Error handling (don't swallow errors)
[ ] OTEL span emission (mcp.tool.name, mcp.tool.duration_ms)
[ ] Integration test (happy path + error case)
[ ] Schema document (for MCP spec)
```

## Service Deployment

For production deployment:

1. **Configure HTTP transport** for remote access
2. **Add authentication** (API key or OAuth2)
3. **Enable OTEL export** to observability backend
4. **Set up rate limiting** on tool invocations
5. **Monitor tool performance** — track slow tools
6. **Backup ontology store** — daily snapshots
7. **Health checks** — `/health` endpoint

## Forbidden Patterns

❌ Tool not registered in tool_router!
❌ Tool that modifies store without documenting side effects
❌ Tool that returns success with partial results
❌ Tool with undocumented input/output
❌ Tool that catches errors silently

## Commands

```bash
# List all tools
onto tools list

# Get tool schema
onto tools schema onto_validate

# Test tool locally
onto query select --sparql "SELECT ?s WHERE { ?s a owl:Class }"

# Start server
onto mcp start --transport stdio

# Health check (requires server running)
curl http://localhost:3050/health
```
