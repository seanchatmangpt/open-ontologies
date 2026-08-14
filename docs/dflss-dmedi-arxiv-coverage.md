# DFLSS DMEDI arXiv Coverage Ontology

This package adds a receipt-bearing research map for the two-week Design for Lean Six Sigma Black Belt curriculum using the DMEDI spine: Define, Measure, Explore, Develop, Implement, and Capstone.

The ontology does **not** claim that every course module has direct arXiv coverage. It separates:

- `dflss:Direct` — the paper directly addresses the named course method or tool.
- `dflss:StronglyRelated` — the paper gives a strong methodological or application analogue.
- `dflss:Adjacent` — the paper is useful context but not a direct course source.
- `dflss:Sparse` — arXiv coverage is sparse for this module.
- `dflss:NoDirectArxivCoverage` — no direct arXiv seed was admitted for the module.

## Files

- `ontology/dflss-dmedi.ttl` — OWL/Turtle curriculum, topic, subject, paper, and coverage-claim graph.
- `ontology/dflss-dmedi-shapes.ttl` — SHACL shapes for phases, topics, papers, and claims.
- `sparql/dflss-dmedi-topic-coverage.rq` — ordered phase/topic/paper coverage query.
- `tools/validate_dflss_dmedi.py` — dependency-free static verifier for the seed ontology package.

## Seed coverage

The first admitted seed set covers TRIZ/autonomous ideation, process capability, statistical process control, FMEA, nonlinear DOE/RSM, robust designs, and fractional-polynomial response surfaces. Modules with weak arXiv fit, such as Minitab-specific instruction and classroom catapult/helicopter simulations, are intentionally represented as sparse rather than forced into false direct coverage.

## Competency questions

The SPARQL query is designed to answer:

1. Which DMEDI phase owns each curriculum topic?
2. Which arXiv papers support each topic?
3. Which topics have sparse/no-direct arXiv coverage?
4. Which topics are covered by Direct, Strongly Related, or Adjacent papers?

## Verification

Run the local verifier from the repository root:

```bash
python tools/validate_dflss_dmedi.py
```

For the full repository gate, run the repository-required quality ladder:

```bash
make check
make test
make adversarial
```
