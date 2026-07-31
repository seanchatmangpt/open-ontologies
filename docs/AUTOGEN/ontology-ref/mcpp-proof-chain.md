# mcpp-proof-chain.ttl

_Generated 2026-05-18T19:57:13Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/mcpp-proof-chain.ttl`
- **Triples:** 79
- **Classes:** 3 · **Properties:** 4 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `urn:ontostar:mcpp:ProofGate` | Proof Gate | A single verifiable admission check in the MCPP proof chain. A work order is admitted only when every ProofGate it requi |
| `urn:ontostar:mcpp:Verdict` | Verdict | The admission decision produced by evaluating all required ProofGates for a WorkOrder. A verdict is emitted exactly once |
| `urn:ontostar:mcpp:WorkOrder` | Work Order | A request to manufacture a compiled part from observed MCP/A2A sessions. A WorkOrder packages the OCEL log reference, th |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `urn:ontostar:mcpp:hasDiscoveryVariant` | urn:ontostar:mcpp:WorkOrder | urn:ontostar:powl:DiscoveryVariant | The POWL inductive-miner algorithm variant used to discover the route model for  |
| `urn:ontostar:mcpp:hasRunId` | urn:ontostar:mcpp:WorkOrder | string | The unique identifier of the manufacturing run that produced this work order. Fo |
| `urn:ontostar:mcpp:hasVerdict` | urn:ontostar:mcpp:WorkOrder | urn:ontostar:mcpp:Verdict | The admission verdict assigned to this work order after all required ProofGates  |
| `urn:ontostar:mcpp:requiresGate` | urn:ontostar:mcpp:WorkOrder | urn:ontostar:mcpp:ProofGate | Links a WorkOrder to each ProofGate that must pass for the work order to receive |
