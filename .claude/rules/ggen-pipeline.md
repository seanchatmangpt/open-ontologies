---
name: ggen Code Generation Pipeline (open-ontologies)
description: .specify/ pipeline, TTL→SPARQL→Tera→generated.rs, safe editing practices
type: rules
---

# ggen Code Generation Pipeline

## Golden Rule

**ALWAYS edit source TTL files. NEVER edit generated.rs directly.**

Generated files are artifacts produced by the ggen pipeline. Editing them:
- Will be overwritten by the next `ggen sync`
- Violates separation of concerns (source vs artifact)
- Makes changes unreproducible and unmaintainable

## The Pipeline (μ₁–μ₅)

```
ontology/cli-open-ontologies.ttl (SOURCE OF TRUTH)
  ↓
μ₁ (Load)
  Validate TTL syntax
  ↓
.specify/queries/*.rq (SPARQL CONSTRUCT)
  ↓
μ₂ (Extract)
  Execute CONSTRUCT queries
  Produce intermediate RDF facts
  ↓
intermediate-facts.rdf
  ↓
.specify/templates/*.tera (Tera templates)
  ↓
μ₃ (Generate)
  Consume intermediate RDF
  Render Tera templates
  Produce code
  ↓
*.rs (generated Rust code)
  ↓
μ₄ (Validate)
  SHACL shape validation
  cargo check
  ↓
src/cmds/generated.rs (ARTIFACT)
  ↓
μ₅ (Emit)
  Write artifact files
  Sign with receipt (BLAKE3 + Ed25519)
```

## Safe Editing Workflow

### Adding a New CLI Command

**Step 1: Edit source TTL**

```turtle
# ontology/cli-open-ontologies.ttl

onto:MyNewCommand
  a onto:OntologyCommand ;
  rdfs:label "My New Command" ;
  rdfs:comment "Description of what the command does" ;
  onto:hasVerb onto:mynew ;
  onto:hasOption onto:optMyOption .

onto:optMyOption
  a onto:Option ;
  rdfs:label "my-option" ;
  rdfs:comment "Option description" ;
  onto:isRequired false .
```

**Step 2: Validate TTL syntax**

```bash
onto validate ontology/cli-open-ontologies.ttl
# Must exit 0
```

**Step 3: Preview code generation**

```bash
ggen sync --dry-run true
# Shows what will be generated, doesn't write files
```

**Step 4: Run full pipeline**

```bash
ggen sync --audit true
# Executes μ₁–μ₅, writes .ggen/receipts/latest.json
```

**Step 5: Verify compilation**

```bash
cargo make check
# Must compile without errors
```

**Step 6: Verify generated code looks correct**

```bash
# Check that new command appears in generated.rs
grep -A 5 "MyNewCommand" src/cmds/generated.rs
```

**Step 7: Run tests**

```bash
cargo make test
# All tests must pass
```

**Step 8: Commit**

```bash
git add ontology/cli-open-ontologies.ttl src/cmds/generated.rs
git commit -m "feat(ontology): Add MyNewCommand via ggen"
```

## SPARQL CONSTRUCT Queries

All queries in `.specify/queries/`:

```sparql
# extract-commands.rq
PREFIX onto: <https://ggen.io/onto/cli/open-ontologies/>

CONSTRUCT {
  ?cmd rdfs:label ?label ;
       rdfs:comment ?comment ;
       onto:hasVerb ?verb ;
       onto:hasOption ?opt .
}
WHERE {
  ?cmd a onto:OntologyCommand ;
       rdfs:label ?label ;
       rdfs:comment ?comment ;
       onto:hasVerb ?verb .
  OPTIONAL {
    ?cmd onto:hasOption ?opt .
  }
}
```

**Rule**: If you need to add a new query step, define it here. Don't hardcode logic in templates.

## Tera Templates

All templates in `.specify/templates/`:

**cli.tera** (generates CLI commands):

```jinja2
// Generated from ontology/cli-open-ontologies.ttl
// DO NOT EDIT — run `ggen sync` to regenerate

{% for cmd in commands -%}
pub struct {{ cmd.label | pascal }}Cmd {
    {%- for opt in cmd.options %}
    pub {{ opt.label | snake }}: {{ opt.type_hint }},
    {%- endfor %}
}
{% endfor -%}
```

**Rule**: Only modify templates to change output format. Don't add business logic—move that to SPARQL.

## What NOT to Edit

### ❌ DO NOT EDIT: src/cmds/generated.rs

This file is a ggen artifact. Any changes will be lost on the next `ggen sync`.

If you see a bug in generated code:
1. Don't fix it in generated.rs
2. Identify the root cause (TTL definition, SPARQL query, or template)
3. Fix the root cause
4. Run `ggen sync` to regenerate

### ❌ DO NOT EDIT: cell8-ggen/src/cell8/generated/

Same rule as above—this is also a ggen artifact.

### ❌ DO NOT EDIT: .specify/templates/old-*.tera

Archived templates from prior versions. If you need to roll back, use git history.

## Verification Checklist

Before claiming code generation is done:

```
[ ] Source TTL edited (ontology/*.ttl)
[ ] onto validate ontology/cli-open-ontologies.ttl passes
[ ] ggen sync --dry-run true shows expected changes
[ ] ggen sync --audit true runs successfully
[ ] cargo make check passes (no compilation errors)
[ ] Generated file contains expected code (grep check)
[ ] cargo make test passes (all tests pass)
[ ] Commit includes both TTL and generated files
```

## Error Recovery

**Error: `ggen sync` fails with SPARQL syntax error**

```
Solution:
1. Check .specify/queries/*.rq for typos in SPARQL
2. Test query manually: onto query construct --sparql @.specify/queries/extract-commands.rq
3. Fix query
4. Re-run ggen sync
```

**Error: Generated code doesn't compile**

```
Solution:
1. Don't fix the generated code
2. Check ontology/cli-open-ontologies.ttl for incomplete definitions
3. Check .specify/templates/*.tera for template syntax errors
4. Fix the root cause
5. Re-run ggen sync
```

**Error: `cargo check` passes but generated code is wrong**

```
Solution:
1. Don't edit generated.rs to work around the issue
2. The generated code accurately reflects the ontology input
3. Fix the ontology definition or template
4. Re-run ggen sync
```

## Forbidden Workflow

❌ Edit generated.rs directly (changes will be lost)
❌ Fix a bug in generated code without fixing source TTL
❌ Commit generated code changes without corresponding TTL changes
❌ Run ggen sync without first validating source TTL
❌ Commit before running cargo make check

## Commands

```bash
# Validate source
onto validate ontology/cli-open-ontologies.ttl

# Preview generation
ggen sync --dry-run true

# Full pipeline with receipt
ggen sync --audit true

# Check compilation
cargo make check

# Run tests
cargo make test

# Verify generated file exists
ls -la src/cmds/generated.rs
```
