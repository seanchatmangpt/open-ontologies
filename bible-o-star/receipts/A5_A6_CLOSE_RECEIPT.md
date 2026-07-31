# Cell8 Gate A5/A6 Close Receipt

**Date:** 2026-06-02T00:00:00Z
**Status:** ALIVE

## A5 — BLAKE3 Hash Chain

### Files Hashed

| File | BLAKE3 Hash |
|------|-------------|
| `ontology/bible-o-star.ttl` | `440dbbd6a4c0097bc2741ee1b4aed45b8cdc578b987c2462c4d37431177252bf` |
| `ontology/nehemiah-52.ttl` | `8542c2705dc2fda203f6bb4626222d1fe5df6abab4ef4d960979acac7d5c833c` |
| `ontology/nehemiah-52-shapes.ttl` | `b484638b5e552fab0a10e523f2d348c0e54889b453210f37268e99054b0076de` |
| `ontology/source-ledger.ttl` | `37de03b9299a7dd6910213b5ab9e05bd9a0237504f477a4ea8b689c1aaa9700b` |

Chain order: `Receipt_BibleOStar` → `Receipt_Nehemiah52` → `Receipt_Nehemiah52Shapes` → `Receipt_SourceLedger`

Each receipt carries `cell8:previousReceipt` linking to the prior node, forming a tamper-evident chain.

## A6 — Ed25519 Seal

**Algorithm:** Ed25519 (ephemeral keypair, generated at gate-close time)

**Signed payload:** concatenated `b3sum` output for all 4 ontology files (as UTF-8 bytes)

| Field | Value |
|-------|-------|
| Public Key (hex) | `477300eac25501eb3ce6923998b7595a16b19c5bc2df584090293528dcdf954c` |
| Signature (hex) | `7a47e9cd8105f10dfb49e8a4aa0ae6ea8c89a1931c114bedac42da76cc4d41a7d927fb68f415d63d280d83fa18627a210343e1969b0a710fe6ee217cded1eb02` |

## RDF Parse Verification

```
python3 -c "from rdflib import Graph; g=Graph(); g.parse('receipts/receipt-chain.ttl', format='turtle'); print(len(g), 'triples')"
29 triples
```

Receipt chain parses to **29 triples** with no errors.

## Artifact

- `receipts/receipt-chain.ttl` — Turtle-serialized BLAKE3 receipt chain + Ed25519 seal assertion
