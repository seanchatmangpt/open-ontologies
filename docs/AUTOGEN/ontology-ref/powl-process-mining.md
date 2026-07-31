# powl-process-mining.ttl

_Generated 2026-05-18T19:57:13Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/powl-process-mining.ttl`
- **Triples:** 80
- **Classes:** 3 · **Properties:** 2 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `urn:ontostar:powl:ConformanceDimension` | Conformance Dimension | One of the four quality dimensions defined by Wil van der Aalst for evaluating how well a discovered process model repre |
| `urn:ontostar:powl:DiscoveryVariant` | Discovery Variant | An inductive-miner algorithm variant used to discover a POWL model from an event log. Each variant defines a CutFilter — |
| `urn:ontostar:powl:OcelObjectType` | OCEL Object Type | A typed category of business object in an Object-Centric Event Log (OCEL 2.0). Each event in the log is associated with  |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `urn:ontostar:powl:hasMinFitness` | urn:ontostar:powl:DiscoveryVariant | decimal | The minimum acceptable Fitness score (in [0.0, 1.0]) for a discovered POWL model |
| `urn:ontostar:powl:hasMinPrecision` | urn:ontostar:powl:DiscoveryVariant | decimal | The minimum acceptable Precision score (in [0.0, 1.0]) for a discovered POWL mod |
