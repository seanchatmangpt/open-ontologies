# DFLSS/DMEDI arXiv Coverage — Definition of Done

This Definition of Done is the admission boundary for the DFLSS/DMEDI arXiv coverage package. “Done” is not a prose assertion: it is the conjunction of the executable gates below plus an exact-head repository verification receipt.

## Status semantics

- `UNKNOWN` — not inspected or not executed.
- `PARTIAL_ALIVE` — some gates executed successfully, but the complete admitted subject was not verified.
- `ALIVE` — every DOD gate below executed successfully against the exact admitted package revision.
- `BLOCKED` — verification could not complete because of an external or capability boundary.
- `BUILD_BROKEN` — the executable verification path ran and failed because the repository cannot build or test.
- `UNSUPPORTED` — the requested proof cannot be manufactured by the available verifier.

## Executable gates

1. **DOD-01 Artifact closure** — ontology, SHACL shapes, SPARQL projection, operating guide, and this DoD all exist at canonical paths.
2. **DOD-02 Phase closure** — the exact six-phase spine is present once each: Define, Measure, Explore, Develop, Implement, Capstone; each phase has deterministic order.
3. **DOD-03 Curriculum closure** — the exact 45 supplied Black Belt topics are represented once each as `dflss:Topic` or `dflss:Tool`.
4. **DOD-04 Structural closure** — every topic has exactly one admitted phase owner, deterministic topic order, and a normalized research-subject mapping.
5. **DOD-05 Bibliographic closure** — every admitted arXiv seed carries a canonical arXiv identifier and URL; seed cardinality is deterministic.
6. **DOD-06 Gap transparency** — bounded sparse/no-direct arXiv coverage is explicit, never silently omitted or promoted to direct evidence.
7. **DOD-07 Claim closure** — coverage claims use the bounded relevance ontology: Direct, StronglyRelated, Adjacent, Sparse, or NoDirectArxivCoverage.
8. **DOD-08 Constraint closure** — SHACL shapes cover phases, topics/tools, arXiv papers, and coverage claims, including key relationship paths.
9. **DOD-09 Query closure** — the canonical SPARQL projection exposes phase/topic order, relevance, paper identity/URL, and sparse reason in deterministic order.
10. **DOD-10 Operational closure** — the operating guide points to this DoD and the executable verifier; the verifier emits a machine-readable receipt.

## Replay

From repository root:

```bash
bash .github/scripts/dflss-dod.sh
```

The script executes the static DoD verifier, emits `target/verifier/dflss-dmedi-dod.json`, and then executes the repository's own ontology parser against both Turtle artifacts.

## Merge standing

A pull request may be merged only when:

- the DoD script reports `ALIVE` on the exact PR head;
- the repository verification matrix succeeds on that same head; and
- no review or mergeability blocker remains.

The DoD deliberately does **not** claim that arXiv is the complete authority for every DFLSS subject. Sparse/no-direct coverage remains first-class evidence rather than fabricated completeness.
