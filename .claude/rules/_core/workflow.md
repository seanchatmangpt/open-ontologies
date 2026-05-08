---
name: open-ontologies Ontology Engineering Workflow
description: 5-step workflow for RDF/OWL ontology evolution
type: rules
---

# open-ontologies Workflow

Follow this cycle when building or modifying ontologies.

## Step 1: Edit Source TTL

The source of truth is RDF/TTL files in `ontology/`:

```bash
vim ontology/cli-open-ontologies.ttl
# Or:
vim ontology/cell8-*.ttl
# Or:
vim ontology/pizza.ttl (example)
```

These are **NOT** generated files. Edit them directly.

## Step 2: Validate Syntax and SHACL

Validate the TTL file:

```bash
onto validate ontology/<file>.ttl
# Must exit 0 for all syntax and shape validation
```

If validation fails, fix the TTL and re-validate.

## Step 3: Run ggen Pipeline (If CLI Changed)

If you edited `ontology/cli-open-ontologies.ttl`, regenerate the CLI:

```bash
ggen sync
# This reads cli-open-ontologies.ttl
# Runs .specify/ SPARQL queries
# Runs Tera templates
# Outputs src/cmds/generated.rs
```

Do NOT hand-edit `generated.rs` — it will be overwritten.

## Step 4: Test and Adversarial Gate

Run full quality gate:

```bash
make adversarial
# Runs: make check → make test → dead-param gate → clippy deny
# Must exit 0
```

If any step fails, fix and re-run.

## Step 5: Version and Approval

If the ontology is production-ready:

```bash
onto version <name>  # Save snapshot
# Record SHACL validation receipt
# Request approval via governance webhook
```

---

**Summary:**
1. Edit `.ttl` file (source of truth)
2. Validate: `onto validate`
3. If CLI changed: `ggen sync`
4. Test: `make adversarial`
5. Version: `onto version`

**Then** claim done.
