# 00 — Overview

## What is OntoStar

OntoStar is the proof-carrying admission layer of the `open-ontologies` MCP server. It refuses to apply any ontology mutation, codegen, push, or release until a 13-conjunct conformance predicate (`cell_ready`) holds, and it records every admitted operation as a chained, BLAKE3-bound `Receipt`.

The wider repository is a Rust MCP server exposing 70+ `onto_*` tools over Oxigraph (RDF store), DSPy-style signature shapes (LLM molding), wasm4pm (POWL conformance replay), and a deterministic multi-target manufacturing pipeline (IaC + Rust + Erlang + AtomVM). OntoStar is the part of that server that decides what is allowed to happen and what proof must exist afterwards.

## The recursive admission claim

OntoStar applies the same admission discipline at four progressively higher altitudes, and the receipt of each tier becomes input evidence for the next:

```
Requirements  ──admit──▶  receipt_R
       │
       ▼
Work Orders  ──admit──▶  receipt_W   (cites receipt_R as scope basis)
       │
       ▼
Mutations    ──admit──▶  receipt_M   (cites receipt_W as workflow basis)
       │
       ▼
Artifacts    ──admit──▶  receipt_A   (cites receipt_M as production basis)
```

Every receipt above the leaf binds to the receipt below it via `prior_receipt` plus a per-session monotonic `sequence` column. An external verifier (`onto verify`) can walk the chain back to the seed without ever loading the live triple store. This is what "recursive admission" means: the same gate, the same defect taxonomy, the same chained receipt at each tier.

## The doctrine

> **LLMs translate. Gates admit. Receipts prove.**

- **LLMs translate.** Groq-backed translators (Phase 5/8) propose `CandidateCtq`, candidate POWL strings, executive projections. They are *proposers* — never authorities. Their output is provisional until the deterministic admission gate certifies it.
- **Gates admit.** `OntoStarAdmissionGate::evaluate` runs `cell_ready` (13 conjuncts: workflow declared, scope closed, OCEL complete, POWL replay passes, threshold met, required stages present, no bypass revocation, receipt valid, provenance chain, external attestation, temporal validity, dependency closure, replay proof). The first failing conjunct returns a typed `DefectClass`. There are no `bail!`, no `anyhow!`, no string-error authorities anywhere on the admission path.
- **Receipts prove.** Every admitted operation produces a `ProductionRecord`, a chained `Receipt` (BLAKE3 over canonical bytes, Ed25519-signed in Phase 10's stub-of-record form), and an `admission_granted` OCEL event. Denied operations produce `admission_denied` with a typed `defect` attribute. No claim of success exists outside this chain.

## Why this matters

The Phase 6 audit found 25 silently-broken CLI tests, 12 stub-validated admission tests, 21 dead defect variants, and 5 textual-ratchet bypass patterns. Phases 7–11 closed every finding fix-forward. The system now refuses to claim a feature works unless the receipt and the OCEL event log prove it ran. That refusal is the product.
