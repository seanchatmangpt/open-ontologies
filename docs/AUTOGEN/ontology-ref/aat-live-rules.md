# aat-live-rules.ttl

_Generated 2026-05-18T19:57:13Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/aat-live-rules.ttl`
- **Triples:** 199
- **Classes:** 1 · **Properties:** 5 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `urn:ontostar:aat:live:LiveRule` | Live Correlation Rule | An AAT-Live correlation check that inspects captured OTel TraceRecords to verify runtime behaviour matches the declared  |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `urn:ontostar:aat:live:coverageStatus` | urn:ontostar:aat:live:LiveRule | string | The current wasm4pm bridge coverage status: 'covered' (fully implemented), 'part |
| `urn:ontostar:aat:live:coveredBy` | urn:ontostar:aat:live:LiveRule | string | The wasm4pm module and function that implements the bridge for this rule, as dec |
| `urn:ontostar:aat:live:requiredAttribute` | urn:ontostar:aat:live:LiveRule | string | A single OTel attribute key that must be present and non-empty on the relevant s |
| `urn:ontostar:aat:live:requiredSpan` | urn:ontostar:aat:live:LiveRule | string | The OTel span name (or pipe-separated alternatives) that must be present in the  |
| `urn:ontostar:aat:live:rustCheckId` | urn:ontostar:aat:live:LiveRule | string | The symbolic identifier of the corresponding CheckOutcome variant in mcpp-server |
