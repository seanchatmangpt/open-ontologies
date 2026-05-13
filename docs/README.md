# OntoStar Documentation

This directory documents the OntoStar layer of the `open-ontologies` repository — the admission gate, recursive admission claim, manufacturing pipeline, swarm, receipt chain, and external verifier that have been built across Phases 1–11 on the `ontostar-integration` branch.

## Audience

A senior engineer joining the project who needs to understand the system in roughly one hour. The docs are written to be read in order, but each is self-contained.

## Reading order

1. **[00-overview.md](00-overview.md)** — One-page elevator pitch. Start here.
2. **[01-architecture.md](01-architecture.md)** — Layer stack, ASCII diagram, per-layer prose.
3. **[02-quickstart.md](02-quickstart.md)** — Install, build, first admission, real-Groq translation.
4. **[03-mcp-tools.md](03-mcp-tools.md)** — Catalogue of every `onto_*` MCP tool by admission tier.
5. **[04-defect-taxonomy.md](04-defect-taxonomy.md)** — `DefectClass` taxonomy v3.0.0, deny paths, tests.
6. **[05-receipt-chain.md](05-receipt-chain.md)** — How receipts are computed, chained, externally verified.
7. **[06-llm-boundary.md](06-llm-boundary.md)** — DSPy signature shapes, molded LLM, real-Groq tests.
8. **[07-phase-history.md](07-phase-history.md)** — Phases 1–11 with commit hashes.
9. **[08-running-tests.md](08-running-tests.md)** — Verification matrix, real-toolchain tests, Groq tests.
10. **[09-troubleshooting.md](09-troubleshooting.md)** — Known stale subprocess state, ignored tests, Tokio issues.

## Doctrine

> **LLMs translate. Gates admit. Receipts prove.**

If a line of code claims success but no admission gate fired and no receipt chained, the success did not happen. The Phase 6 adversarial audit treated optimistic narration as a defect class, and Phases 7–11 closed every audit finding with a fix-forward commit. Read the docs in that spirit: every claim in the system must be backed by a typed `DefectClass` deny path or a chained `Receipt`.

## Pre-existing docs

The original ontology-engineering docs (`alignment.md`, `cache-and-registry.md`, `clinical.md`, `data-pipeline.md`, `embeddings.md`, `ies-*`, `lifecycle.md`, `quickstart.md`, `reasoning.md`) document the broader `open-ontologies` MCP server. Those are still authoritative for the non-OntoStar feature surface (embeddings, IES alignment, clinical crosswalks, etc.). The OntoStar docs (`00-`–`09-`) layer on top.
