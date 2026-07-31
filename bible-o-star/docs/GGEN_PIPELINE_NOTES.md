# GGEN Pipeline Integration Notes — Bible O*

**Assessment date:** 2026-06-02
**Ontology:** `bible-o-star/ontology/bible-o-star.ttl` + `nehemiah-52.ttl`
**Pipeline:** open-ontologies ggen v5 (TTL → SPARQL → Tera → generated artifacts)
**Status: PARTIAL**

---

## Can bible-o-star.ttl feed into the ggen pipeline?

**Short answer:** Not in the current pipeline configuration. The ggen pipeline at `/Users/sac/open-ontologies/ggen.toml` is tightly bound to `ontology/cli-open-ontologies.ttl` as its single source TTL, with all import paths relative to the open-ontologies root. `bible-o-star.ttl` lives in a subdirectory (`bible-o-star/ontology/`) and uses a separate namespace (`bos: <https://open-ontologies.org/bible-o-star#>`).

The pipeline's vocabulary namespace must be in ggen's internal allowlist (the `ggen.toml` note reads: "ggen v5 binary requires vocabulary namespaces to be in its internal allowlist. The cli: and onto: namespaces use https://ggen.io/onto/cli/ base URIs (allowed domain)"). The `bos:` namespace (`https://open-ontologies.org/bible-o-star#`) is not an allowed domain — it would require explicit registration or a namespace bridge.

Additionally, the `ggen.toml` documents a scaling bottleneck: imports beyond 39 blow up pre-flight (confirmed across multiple sync runs). Adding bible-o-star.ttl and nehemiah-52.ttl as additional imports to the existing 39-import manifest would push the pipeline past the known failure threshold.

---

## Gaps That Prevent ggen Integration

### Gap 1 — Namespace not in ggen allowlist
The `bos:` prefix (`https://open-ontologies.org/bible-o-star#`) is not a `ggen.io`-domain URI. The ggen v5 binary enforces an allowlist of recognized vocabulary namespaces. Without `bos:` in that allowlist, SPARQL queries against `bos:` classes will produce no bindings and templates will render empty.

**Required to close:** Either register `https://open-ontologies.org/bible-o-star#` with ggen's allowlist (requires ggen configuration or a custom namespace bridge TTL that maps `bos:` terms to `https://ggen.io/onto/` equivalents), or stand up a separate `ggen-bos.toml` manifest with a dedicated source TTL.

### Gap 2 — No ggen manifest for bible-o-star
There is no `ggen-bos.toml` (compare: `ggen.toml`, `ggen-revops.toml`, `ggen-zoela-mobile.toml` at the open-ontologies root). The bible-o-star ontology is not registered as a ggen pipeline target.

**Required to close:** Create `ggen-bos.toml` at `/Users/sac/open-ontologies/ggen-bos.toml` pointing at `bible-o-star/ontology/bible-o-star.ttl` with `owl:imports` of `nehemiah-52.ttl`. Define `[generators]` entries for each code generation target.

### Gap 3 — No SPARQL CONSTRUCT queries defined
The `.specify/queries/` directory contains queries for `wasm4pm`, `revops`, `zoela`, `ghf`, `thesis`, and `portfolio` domains. There are no queries for the `bos:` namespace — no `extract-gates.rq`, no `extract-builders.rq`, no `extract-wall-sections.rq`.

**Required to close:** Author SPARQL CONSTRUCT queries targeting `bos:` classes. Examples below.

### Gap 4 — No Tera templates for bos: output
The `.specify/templates/` directory contains templates for `wasm4pm`, `revops`, `zoela`, and `truex` domains. There are no templates for bible-o-star artifacts.

**Required to close:** Author `.tera` templates for the target output format (Rust structs, TypeScript types, or SQL schema depending on intended use).

### Gap 5 — Import count ceiling
The existing `ggen.toml` is at the 39-import ceiling confirmed to cause pre-flight hangs. Adding bible-o-star imports to the existing manifest is blocked. A separate manifest (`ggen-bos.toml`) avoids this constraint since it would start with a fresh import count.

---

## SPARQL Queries That Would Generate Code from the Ontology

The following queries illustrate what would be needed. These are design specifications, not runnable queries (Gap 1 and Gap 2 must be closed first).

### Query 1 — Extract Gate Definitions

```sparql
# .specify/queries/bos/extract-gates.rq
PREFIX bos: <https://open-ontologies.org/bible-o-star#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT
  ?gate_iri
  ?gate_label
  ?gate_comment
  ?canonical_ref
WHERE {
  ?gate_iri a bos:Gate ;
            rdfs:label ?gate_label ;
            rdfs:comment ?gate_comment .
  OPTIONAL { ?gate_iri bos:hasCanonicalReference ?canonical_ref . }
}
ORDER BY ?gate_label
```

This would generate a Rust enum of the 10 sanctioned gates, or a TypeScript discriminated union, or a PostgreSQL lookup table.

### Query 2 — Extract Builder-WallSection Assignments

```sparql
# .specify/queries/bos/extract-builder-assignments.rq
PREFIX bos: <https://open-ontologies.org/bible-o-star#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT
  ?builder_label
  ?section_label
  ?gate_label
WHERE {
  ?builder a bos:Builder ;
           rdfs:label ?builder_label ;
           bos:buildsWallSection ?section .
  ?section rdfs:label ?section_label .
  OPTIONAL {
    ?builder bos:assignedToGate ?gate .
    ?gate rdfs:label ?gate_label .
  }
}
ORDER BY ?builder_label
```

This would generate builder-to-section assignment tables, routing configuration, or accountability matrices.

### Query 3 — Extract Scripture Address Hierarchy

```sparql
# .specify/queries/bos/extract-scripture-spine.rq
PREFIX bos: <https://open-ontologies.org/bible-o-star#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX owl: <http://www.w3.org/2002/07/owl#>

SELECT
  ?class_iri
  ?class_label
  ?parent_label
  ?class_comment
WHERE {
  VALUES ?class_iri {
    bos:ScriptureWork bos:Book bos:Chapter bos:Verse bos:Passage bos:Pericope
  }
  ?class_iri rdfs:label ?class_label ;
             rdfs:comment ?class_comment .
  OPTIONAL {
    ?class_iri rdfs:subClassOf ?parent .
    ?parent rdfs:label ?parent_label .
  }
}
```

This would generate Rust structs for the canonical address hierarchy, or TypeScript interfaces for a scripture reference API.

### Query 4 — Extract Verdict Taxonomy

```sparql
# .specify/queries/bos/extract-verdict-taxonomy.rq
PREFIX bos: <https://open-ontologies.org/bible-o-star#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT
  ?verdict_class
  ?label
  ?comment
WHERE {
  VALUES ?verdict_class { bos:Verdict bos:InspectionReceipt bos:Receipt }
  ?verdict_class rdfs:label ?label ;
                 rdfs:comment ?comment .
}
```

This would generate verdict enum types (`ALIVE | PARTIAL | BLOCKED`) usable in generated admission logic.

---

## What the Ontology Already Has (Positives)

The bible-o-star ontology is structurally well-formed for ggen consumption:

- All classes have `rdfs:label` and `rdfs:comment` — the label/comment pair is what ggen templates consume for identifier generation
- Properties have `rdfs:domain` and `rdfs:range` — enabling type-safe code generation
- The ontology uses standard OWL constructs (no exotic extensions that would confuse SPARQL reasoning)
- `owl:imports` is declared in `nehemiah-52.ttl` (`owl:imports <https://open-ontologies.org/bible-o-star>`) — the import chain is well-formed
- SHACL shapes exist in `nehemiah-52-shapes.ttl` — usable for `onto_shacl` validation before code generation
- The namespace is stable and collision-free with existing open-ontologies namespaces

---

## Integration Path (If Pursued)

To move from PARTIAL to READY, the following must be completed in order:

1. Create `ggen-bos.toml` with `source = "bible-o-star/ontology/bible-o-star.ttl"` and `imports = ["bible-o-star/ontology/nehemiah-52.ttl"]`
2. Register `bos:` namespace with ggen allowlist or create a `bos-to-ggen-bridge.ttl` that mints `https://ggen.io/onto/bos/` equivalents via `owl:equivalentClass`
3. Author SPARQL queries in `.specify/queries/bos/` (gate definitions, builder assignments, verdict taxonomy, scripture spine)
4. Author Tera templates in `.specify/templates/bos/` for target output (Rust, TypeScript, or SQL)
5. Validate with `ggen sync --dry-run true` against `--config ggen-bos.toml`
6. Run `ggen sync --audit true --config ggen-bos.toml` and verify receipt emission

**Do not add bible-o-star imports to the existing `ggen.toml`** — it is at the import ceiling.

---

## Current Status: PARTIAL

The bible-o-star ontology is structurally ready (well-formed TTL, labels, comments, domains, ranges, SHACL shapes). The ggen pipeline infrastructure requires three additions before code generation is possible: a dedicated manifest file, namespace allowlist registration, and SPARQL/Tera artifacts. None of these gaps are blockers in principle — they are missing work items, not architectural incompatibilities.
