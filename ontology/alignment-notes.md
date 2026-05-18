# Alignment Notes: aat-live-rules ↔ powl-process-mining

**Generated:** 2026-05-17  
**Tool chain:** `open-ontologies alignment align` (structural) + SPARQL analysis  
**Source:** `ontology/aat-live-rules.ttl` (199 triples) · `ontology/powl-process-mining.ttl` (80 triples)

---

## 1. Automated Alignment Result

`onto_align --source aat-live-rules.ttl --target powl-process-mining.ttl --min_confidence 0.1`

```json
{"applied_count":0,"candidates":[],"threshold":0.1,"total_candidates":0}
```

**Why zero candidates:** Both ontologies use flat named-individual structures (no `rdfs:subClassOf` hierarchy beyond leaf coverage subclasses). The 6 structural signals (label overlap, property overlap, hierarchy depth, sibling similarity, range overlap, individual overlap) all score near zero because:
- AAT classes: `LiveRule` + coverage subclasses (`CoveredRule`, `PartialRule`, `UncoveredRule`)
- POWL classes: `DiscoveryVariant`, `ConformanceDimension`, `OcelObjectType`
- No shared labels, no shared property domains, no shared superclasses

Structural alignment is not applicable here. The cross-ontology connection is **semantic** (attribute string → class individual) and requires SPARQL-based analysis.

---

## 2. SPARQL-Discovered Alignment Candidates

### 2.1 LIVE-02 → powl:Fitness + powl:Precision (skos:exactMatch candidates)

| AAT term | Relation | POWL term | Confidence |
|---|---|---|---|
| `aat:requiredAttribute "mcpp.conformance.fitness"` | `skos:exactMatch` | `powl:Fitness` | HIGH |
| `aat:requiredAttribute "mcpp.conformance.precision"` | `skos:exactMatch` | `powl:Precision` | HIGH |

**Rationale:** LIVE-02 (`LiveAcceptedWithoutProof`) requires `proof.aggregate` spans to carry `mcpp.conformance.fitness` and `mcpp.conformance.precision` as OTel attributes. These attribute keys are direct runtime projections of the `powl:Fitness` and `powl:Precision` named individuals (Van der Aalst conformance dimensions). The mcpp Doctrine (fitness = 1.0 required, 0.999 = Andon pull) establishes that `mcpp.conformance.fitness` is the observable manifestation of `powl:Fitness` at runtime.

**Proposed triple to add to `aat-live-rules.ttl`:**
```turtle
# Cross-ontology link: LIVE-02 conformance attributes map to POWL dimensions
aat:LIVE-02 aat:measuresFitness powl:Fitness ;
            aat:measuresPrecision powl:Precision .
```
Or alternatively add `skos:exactMatch` on the OTel attribute string concept if modelled as a named individual.

### 2.2 powl:satisfiesLiveRule — Existing Cross-Ontology Bridge (Already Present)

`powl:satisfiesLiveRule` is defined in the POWL ontology with `rdfs:domain powl:DiscoveryVariant`. This property already creates a directed link: **a DiscoveryVariant can satisfy a LiveRule**. This is the authoritative cross-ontology bridge; the alignment candidates above are supplementary.

### 2.3 LIVE-01, LIVE-05, LIVE-09, LIVE-11 — POWL Span References

Four LIVE rules reference POWL-namespaced OTel spans or attributes directly:

| Rule | Connection | Type |
|---|---|---|
| LIVE-01 | `requiredSpan` includes `powl.route.evaluate` | Span name references POWL routing concept |
| LIVE-05 | `requiredAttribute` includes `powl.activity.id`, `powl.activity.predecessors_satisfied`, `powl.activity.objects_valid` | Attributes reference POWL activity lifecycle |
| LIVE-09 | `requiredSpan` includes `powl.gap.*` spans; `requiredAttribute` includes `powl.gap.correlation_id`, `powl.gap.activity_id` | Gap closure spans reference POWL gap detection |
| LIVE-11 | `requiredSpan` includes `powl.route.evaluate` (latency delta check) | Timing reference to POWL routing |

These are **rdfs:subClassOf candidates** or **skos:related** links between `aat:LiveRule` instances and POWL routing/gap concepts. No formal POWL classes for `Route`, `Gap`, or `Activity` are defined in `powl-process-mining.ttl` — these span names are OTel vocabulary, not OWL classes.

**Recommendation:** Extend `powl-process-mining.ttl` with classes for `powl:Route`, `powl:Gap`, and `powl:Activity` to give these span references OWL anchors.

### 2.4 ConformanceDimension → hasMinFitness / hasMinPrecision (Domain Gap)

`powl:hasMinFitness` and `powl:hasMinPrecision` have `rdfs:domain powl:DiscoveryVariant` but their values (float thresholds) express constraints on `powl:Fitness` and `powl:Precision` respectively. A cleaner model would add:

```turtle
powl:hasMinFitness rdfs:range xsd:decimal ;
    # Note: constrains the Fitness dimension
    rdfs:comment "Minimum Fitness score threshold for admission" .
```

with an `owl:onProperty powl:hasMinFitness / owl:onClass powl:Fitness` restriction on `DiscoveryVariant`.

---

## 3. Enforcement Results (generic pack)

```json
{
  "compliance": 0.75,
  "passed_rules": 3,
  "total_rules": 4,
  "violations": [
    {
      "entity": "urn:ontostar:powl:OcelObjectType",
      "message": "Class has no parent, children, or property references",
      "rule": "orphan_class",
      "severity": "warning"
    },
    {
      "entity": "urn:ontostar:powl:ConformanceDimension",
      "message": "Class has no parent, children, or property references",
      "rule": "orphan_class",
      "severity": "warning"
    }
  ]
}
```

### Violation 1: `powl:OcelObjectType` — orphan class

**Root cause:** `OcelObjectType` is declared as an `owl:Class` with label and comment but has:
- No `rdfs:subClassOf` parent
- No child classes
- No property with `rdfs:domain powl:OcelObjectType`

`OcelObjectType` is a typing class for OCEL 2.0 instances (e.g. Order, Item, Package). In the current TTL it is a vocabulary stub — the actual object type individuals (Order, Item, etc.) are not defined. The enforcer correctly flags it as orphaned.

**Fix:** Either (a) add `rdfs:subClassOf owl:Thing` explicitly and a note that subclasses are defined per-deployment, or (b) add `powl:hasObjectType` property with `rdfs:range powl:OcelObjectType` linking `DiscoveryVariant` to the object types it can discover over.

### Violation 2: `powl:ConformanceDimension` — orphan class

**Root cause:** `ConformanceDimension` has four named individuals (`Fitness`, `Precision`, `Generalization`, `Simplicity`) that declare `a powl:ConformanceDimension` — but the enforcer's `orphan_class` rule checks for structural graph connections (parent/child subclass or domain), not individual membership.

**Fix:** Add `owl:hasKey` or a `rdfs:subClassOf` bridge, or annotate with `owl:oneOf (powl:Fitness powl:Precision powl:Generalization powl:Simplicity)` to make the closed-world membership explicit:

```turtle
powl:ConformanceDimension owl:oneOf (
  powl:Fitness powl:Precision powl:Generalization powl:Simplicity
) .
```

This eliminates the orphan warning and formally closes the enumeration.

---

## 4. Lint Results

Both ontologies: **0 issues** (all classes and properties have `rdfs:label`, `rdfs:comment`, `rdfs:domain`, `rdfs:range` where applicable).

---

## 5. Coverage Summary

| Rule | Coverage | POWL connection |
|---|---|---|
| LIVE-01 | covered | `powl.route.evaluate` span |
| LIVE-02 | covered | `powl:Fitness`, `powl:Precision` (key alignment) |
| LIVE-03 | covered | none |
| LIVE-04 | covered | none |
| LIVE-05 | covered | `powl.activity.*` attributes |
| LIVE-06 | **none** | none |
| LIVE-07 | partial | none |
| LIVE-08 | covered | none |
| LIVE-09 | covered | `powl.gap.*` spans and attributes |
| LIVE-10 | partial | none |
| LIVE-11 | partial | `powl.route.evaluate` span (latency) |
| LIVE-12 | covered | none |
| LIVE-13 | covered | none |
| LIVE-14 | partial | none |
| LIVE-15 | partial | none |
| LIVE-16 | partial | none |

Coverage summary: 9 covered · 6 partial · 1 none (LIVE-06: `wasm.part.invoke` / part manifest binding).

---

## 6. Recommended Actions

1. **Fix `powl:ConformanceDimension` orphan** — add `owl:oneOf` enumeration closure (one-line TTL change, resolves enforce violation).
2. **Fix `powl:OcelObjectType` orphan** — add `rdfs:subClassOf owl:Thing` + comment noting deployment-specific subclasses, or add a `powl:hasObjectType` property with this as range.
3. **Add cross-ontology exactMatch triples** — link `aat:LIVE-02`'s conformance attributes to `powl:Fitness` and `powl:Precision` via a bridge property or `skos:exactMatch` on concept nodes.
4. **Extend POWL with `powl:Route`, `powl:Gap`, `powl:Activity`** — to give LIVE-01, LIVE-05, LIVE-09, LIVE-11 span references formal OWL anchors rather than just string literals.
5. **Add `powl:hasMinFitness`/`powl:hasMinPrecision` individuals** — populate with the mcpp Doctrine threshold (1.0 required, 0.999 = Andon pull) so admission criteria are machine-readable.
6. **Bridge LIVE-06** — `wasm.part.invoke` / part manifest binding has zero coverage; this is the only completely uncovered rule.
