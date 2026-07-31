# ontostar-wasm4pm-integration.ttl

_Generated 2026-05-18T19:57:13Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/ontostar-wasm4pm-integration.ttl`
- **Triples:** 108
- **Classes:** 7 · **Properties:** 4 · **SHACL shapes:** 1

## Classes

| name | label | comment |
|---|---|---|
| `nca25e78bf3be4594b691c3f2a8c509afb1` | — | — |
| `nca25e78bf3be4594b691c3f2a8c509afb5` | — | — |
| `nca25e78bf3be4594b691c3f2a8c509afb9` | — | — |
| `urn:ontostar:aat:live:CoveredRule` | Covered Live Rule | An AAT-Live correlation rule whose wasm4pm bridge is fully implemented (coverageStatus = 'covered'). All required OTel s |
| `urn:ontostar:aat:live:PartialRule` | Partially Covered Live Rule | An AAT-Live correlation rule whose wasm4pm bridge emits the required span but one or more required attributes depend on  |
| `urn:ontostar:aat:live:UncoveredRule` | Uncovered Live Rule | An AAT-Live correlation rule for which no wasm4pm bridge exists yet (coverageStatus = 'none'). The required OTel spans a |
| `urn:ontostar:shared-receipt:ReceiptConformance` | Receipt Conformance | A conformance record that associates a SharedReceiptV1 instance with the AAT-Live correlation rules that its captured OT |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `urn:ontostar:powl:satisfiesLiveRule` | urn:ontostar:powl:DiscoveryVariant | urn:ontostar:aat:live:LiveRule | Relates a POWL DiscoveryVariant to an AAT-Live LiveRule that the variant's disco |
| `urn:ontostar:shared-receipt:conformanceScore` | urn:ontostar:shared-receipt:ReceiptConformance | decimal | The fraction of AAT-Live LIVE rules (out of 16) that passed for the associated r |
| `urn:ontostar:shared-receipt:forReceipt` | urn:ontostar:shared-receipt:ReceiptConformance | urn:ontostar:shared-receipt:SharedReceiptV1 | Links a sr:ReceiptConformance to the sr:SharedReceiptV1 instance it evaluates. |
| `urn:ontostar:shared-receipt:hasLiveRuleCoverage` | urn:ontostar:shared-receipt:ReceiptConformance | urn:ontostar:aat:live:LiveRule | Points from a sr:ReceiptConformance instance to each aat:LiveRule individual who |

## SHACL shapes

| shape | targetClass |
|---|---|
| `urn:ontostar:integration:wasm4pm-mcpp:ReceiptConformanceShape` | urn:ontostar:shared-receipt:ReceiptConformance |
