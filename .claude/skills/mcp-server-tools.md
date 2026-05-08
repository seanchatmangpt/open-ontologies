---
name: MCP Server Tools (43 onto_* tools)
description: onto_* tool patterns, tool registration in tool_router, RPC protocol
paths: ["src/server.rs", "src/cmds/**"]
type: skill
---

# Skill: MCP Server Tools

## Purpose

Understand and implement the 43 onto_* tools that expose open-ontologies functionality via MCP protocol.

## The 43 Tools (by category)

### Ontology Management (6 tools)
- `onto_load` — Load TTL ontology into store
- `onto_load_remote` — Load ontology from remote URL
- `onto_unload` — Unload ontology from store
- `onto_list_ontologies` — List loaded ontologies
- `onto_export` — Export ontology to TTL/NTriples/RDF/XML
- `onto_import_namespace` — Import namespace from remote registry

### SPARQL Query (8 tools)
- `onto_query_select` — Execute SPARQL SELECT
- `onto_query_construct` — Execute SPARQL CONSTRUCT
- `onto_query_ask` — Execute SPARQL ASK
- `onto_query_describe` — Execute SPARQL DESCRIBE
- `onto_query_load_file` — Load and execute SPARQL from file
- `onto_query_list_saved` — List saved SPARQL queries
- `onto_query_save` — Save SPARQL query for reuse
- `onto_query_delete_saved` — Delete saved query

### SHACL Validation (5 tools)
- `onto_validate` — Validate ontology against SHACL shapes
- `onto_validate_artifact` — Validate specific artifact
- `onto_validate_report` — Get detailed validation report (JSON)
- `onto_shapes_list` — List loaded SHACL shapes
- `onto_shapes_load` — Load additional SHACL shapes

### OWL Reasoning (4 tools)
- `onto_reason_infer` — Run OWL 2 reasoner, infer new facts
- `onto_reason_check_consistency` — Check ontology consistency
- `onto_reason_unsatisfiable_classes` — List unsatisfiable classes
- `onto_reason_equivalence` — Find equivalent classes/properties

### Entity Management (6 tools)
- `onto_entity_get` — Get full description of entity (IRI)
- `onto_entity_search` — Search entities by label/comment
- `onto_entity_create` — Create new entity (class, property, instance)
- `onto_entity_update` — Update entity properties
- `onto_entity_delete` — Delete entity
- `onto_entity_instances` — List all instances of class

### Class Operations (4 tools)
- `onto_class_hierarchy` — Get class hierarchy tree
- `onto_class_properties` — List properties defined on class
- `onto_class_restrictions` — Get OWL restrictions on class
- `onto_class_equivalent` — Find equivalent class definitions

### Property Operations (3 tools)
- `onto_property_get` — Get property definition and constraints
- `onto_property_domain_range` — Get domain and range
- `onto_property_inverse` — Get inverse property if defined

### Graph Operations (7 tools)
- `onto_graph_triples` — Get all triples (optionally filtered)
- `onto_graph_triple_count` — Count triples in store
- `onto_graph_pattern_match` — Find triples matching pattern
- `onto_graph_subgraph_extract` — Extract connected subgraph
- `onto_graph_merge` — Merge two graphs/stores
- `onto_graph_diff` — Find differences between graphs
- `onto_graph_visualize` — Get graph visualization (Mermaid/DOT)

## Tool Router Registration

All tools registered in `src/server.rs`:

```rust
macro_rules! tool_router {
    ($tool_name:expr, $input:expr) => {
        match $tool_name {
            "onto_load" => onto_load($input).await,
            "onto_query_select" => onto_query_select($input).await,
            "onto_validate" => onto_validate($input).await,
            "onto_reason_infer" => onto_reason_infer($input).await,
            "onto_entity_get" => onto_entity_get($input).await,
            "onto_class_hierarchy" => onto_class_hierarchy($input).await,
            "onto_property_get" => onto_property_get($input).await,
            "onto_graph_triples" => onto_graph_triples($input).await,
            // ... 35+ more tools
            _ => Err(format!("Unknown tool: {}", $tool_name)),
        }
    };
}
```

Every tool MUST be:
1. Defined in `src/cmds/<category>.rs`
2. Registered in the `tool_router!` macro
3. Have input schema documented
4. Have output schema documented

## Input/Output Schema Pattern

```rust
// src/cmds/query.rs

pub struct QuerySelectInput {
    pub sparql: String,
    pub format: Option<String>,  // json, xml, csv
}

pub struct QuerySelectOutput {
    pub results: Vec<HashMap<String, String>>,
    pub bindings_count: usize,
}

pub async fn onto_query_select(input: QuerySelectInput) -> Result<QuerySelectOutput> {
    // Implementation
}
```

MCP exposure:

```json
{
  "name": "onto_query_select",
  "description": "Execute SPARQL SELECT query against loaded ontologies",
  "inputSchema": {
    "type": "object",
    "properties": {
      "sparql": {
        "type": "string",
        "description": "SPARQL SELECT query string"
      },
      "format": {
        "type": "string",
        "enum": ["json", "xml", "csv"],
        "description": "Result format (default: json)"
      }
    },
    "required": ["sparql"]
  }
}
```

## Forbidden Patterns

❌ Tool defined but NOT registered in tool_router!
❌ Tool that modifies store state without documenting side effects
❌ Tool that returns success with partial results (must return all or error)
❌ Tool with undocumented input/output schema
❌ Tool that skips validation of input parameters

## Required Patterns

✅ All 43 tools registered in tool_router!
✅ Each tool has Rust function + MCP schema
✅ Input validation before processing
✅ Error messages contain actionable details
✅ Output serializable to JSON
✅ Integration test for each tool

## Commands

```bash
# List all available tools
onto tools list

# Get tool schema
onto tools schema onto_validate

# Call tool via CLI
onto query select --sparql "SELECT ?s WHERE { ?s rdf:type owl:Class }"

# Start MCP server (serves all 43 tools)
onto mcp start --transport stdio
```
