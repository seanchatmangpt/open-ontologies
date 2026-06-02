# Open Ontologies - RDF Command Vocabulary

RDF-based ontology for the `clap-noun-verb` CLI framework version 26.6.1.

**Test totals:** 714 `#[test]` functions across `tests/`

## Files

### Core Ontology

**`cli-commands.ttl`** (238 triples)
- Defines the base CLI command ontology
- Classes: `cli:Command`, `cli:Verb`, `cli:Noun`, `cli:Parameter`, `cli:OutputType`, `cli:ErrorType`
- Properties for command metadata: name, handler, docstring, parameters, return types, error types
- Enumerates all error types and output types used by v26.6.1

### Version-Specific Commands

**`v26.6.1-commands.ttl`** (153 triples)
- Command definitions for all 6 verbs in clap-noun-verb v26.6.1
- Three noun namespaces:
  - **graph** (3 verbs): load, query, validate
  - **pack** (2 verbs): add, remove
  - **doctor** (1 verb): check
- Full metadata for each verb: docstring, handler, parameters, return types, errors, examples

### SPARQL Queries

**`queries/cli-commands.sparql`**
- 15 query patterns for discovering and analyzing CLI commands
- Examples:
  - `list_all_verbs` - Get all commands by namespace
  - `get_verb_definition` - Full metadata for a specific verb
  - `required_parameters` - Input validation rules
  - `output_type_schema` - Output structure and serialization
  - `error_reference` - Error handling guide
  - `namespace_parameters` - Per-namespace parameter definitions
  - `commands_per_namespace` - Capacity metrics
  - `parameter_types` - Type system analysis

## Usage

### Query All Verbs

```sparql
PREFIX cli: <http://example.org/cli/>
PREFIX cnv: <http://example.org/clap-noun-verb/v26.6.1/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?namespace ?verb ?name ?docstring
WHERE {
  ?v a cli:Verb ;
     cli:name ?name ;
     cli:namespace ?noun ;
     cli:docstring ?docstring .
  ?noun cli:name ?namespace .
}
ORDER BY ?namespace ?name
```

### Get Verb Parameters for Code Generation

```sparql
PREFIX cli: <http://example.org/cli/>

SELECT ?verb ?param ?type ?required
WHERE {
  ?verb cli:name "load" ;
        cli:hasParameter ?param .
  ?param cli:name ?param_name ;
         cli:type ?type ;
         cli:isRequired ?required .
}
```

### Find Commands by Handler Function

```sparql
PREFIX cli: <http://example.org/cli/>

SELECT ?verb ?name
WHERE {
  ?verb a cli:Verb ;
        cli:handler ?handler ;
        cli:name ?name .
}
```

## Validation

Both files have been validated with the Raptor RDF parser:

```bash
rapper -i turtle -c cli-commands.ttl        # 238 triples ✓
rapper -i turtle -c v26.6.1-commands.ttl    # 153 triples ✓
```

## Namespaces

| Prefix | URI |
|--------|-----|
| `cli:` | `http://example.org/cli/` |
| `cnv:` | `http://example.org/clap-noun-verb/v26.6.1/` |
| `rdf:` | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` |
| `rdfs:` | `http://www.w3.org/2000/01/rdf-schema#` |
| `xsd:` | `http://www.w3.org/2001/XMLSchema#` |

## Version 26.6.1 Command Summary

### graph namespace
1. **load** - Load RDF file (path parameter)
2. **query** - Query RDF data (query_string parameter)
3. **validate** - Validate RDF syntax (path parameter)

### pack namespace
4. **add** - Add capability (pack_name, capability_id parameters)
5. **remove** - Remove capability (pack_name, capability_id parameters)

### doctor namespace
6. **check** - System diagnostics (no parameters)

## Output Types

| Type | Properties |
|------|-----------|
| `GraphLoadedOutput` | triples_loaded, source, status |
| `QueryResultOutput` | query_type, pattern, results, match_count |
| `ValidationResultOutput` | valid, errors, total_triples, valid_triples |
| `PackAddOutput` | pack_name, capability_count, status |
| `PackRemoveOutput` | pack_name, removed_capabilities, status |
| `DoctorCheckOutput` | healthy, diagnostics, warnings, errors |

## Error Types

- `FileNotFoundError` - File does not exist
- `InvalidFormatError` - File format not recognized
- `ValidationError` - RDF content validation failed
- `ExecutionError` - Command execution failed
- `ParseError` - Query parsing failed

## Integration with Code Generation

These ontologies are designed for use with code generators like `clap-noun-verb-gen`:

```bash
# Generate CLI scaffolding from RDF vocabulary
clap-noun-verb-gen --from-ontology=v26.6.1-commands.ttl --output=generated-cli.rs
```

The SPARQL queries provide structured data extraction for code generation, validation, and documentation.
