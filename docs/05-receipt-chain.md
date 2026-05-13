# 05 — Receipt Chain

## What a receipt is

A `Receipt` is a typed record certifying that one admission decision happened. It is **not** a hash of stdout. Every field is load-bearing.

```rust
pub struct Receipt {
    pub receipt_hash: [u8; 32],          // BLAKE3 over canonical bytes (this row)
    pub prior_receipt: Option<[u8; 32]>, // BLAKE3 of the predecessor in the chain
    pub session_id: String,              // session this admission belongs to
    pub sequence: i64,                   // per-session monotonic counter (Phase 7)
    pub op: AdmissionOp,                 // typed operation (Apply / Codegen / ...)
    pub scope_token: [u8; 32],           // BLAKE3 of canonical OCEL projection
    pub artifact_hash: [u8; 32],         // BLAKE3 of artifact bytes (or stand-in)
    pub gate_config_hash: [u8; 32],      // BLAKE3 of admission gate config
    pub defects_taxonomy_version: String,
    pub granted_at: DateTime<Utc>,       // RFC-3339, UTC
    pub tenant_id: String,               // Phase 11
    pub signature: Option<Vec<u8>>,      // Ed25519 (Phase 10 stub-of-record)
}
```

## How `receipt_hash` is computed

BLAKE3 over the canonical byte serialization of all other fields, in declaration order, with `\0` separators and lower-case hex for nested hashes. The serializer is in `src/receipts.rs::canonical_bytes` and is round-trip tested. The point of canonicalization is that two implementations in any two languages agree on the same hash.

## How the chain links

Each receipt names its predecessor's `receipt_hash` in the `prior_receipt` field. The first receipt in a session has `prior_receipt = None`. The chain is therefore a per-session linked list, totally ordered by `sequence`.

```
session "abc":
  seq=1  hash=H1  prior=None     op=RequirementProposed
  seq=2  hash=H2  prior=H1       op=CtqAdmitted
  seq=3  hash=H3  prior=H2       op=WorkOrderAdmitted
  seq=4  hash=H4  prior=H3       op=SolutionManufactured
```

## Per-session sequence (Phase 7 Task C)

```sql
ALTER TABLE receipts ADD COLUMN sequence INTEGER NOT NULL DEFAULT 0;
CREATE UNIQUE INDEX receipts_session_sequence_uniq ON receipts(session_id, sequence);
CREATE INDEX receipts_session_seq_desc ON receipts(session_id, sequence DESC);
```

The unique index makes a chain fork structurally impossible. `latest_for_session` orders by `sequence DESC`, not `granted_at`, so identical timestamps cannot deadlock determinism.

## Atomic persist+emit (Phase 7 C.fix)

`OntoStarAdmissionGate::evaluate` opens one SQLite transaction, computes the next sequence under lock, persists the receipt, and emits the `admission_granted` OCEL event in the **same** transaction. If the OCEL emit fails the receipt is rolled back. There is no orphan-receipt failure mode — `tests/receipt_chain_adversarial.rs::orphan_detection_refuses_to_chain` proves this.

## Worked example — verifying externally

```bash
$ onto verify ./out/bundle/
{
  "is_valid": true,
  "chain_length": 4,
  "seed_receipt": "blake3:b1d4e5f0...",
  "head_receipt": "blake3:9f3a2c11...",
  "chain": [
    {"seq": 1, "op": "requirement_proposed", "hash": "b1d4e5f0..."},
    {"seq": 2, "op": "ctq_admitted",         "hash": "7c20a9b1...", "prior": "b1d4e5f0..."},
    {"seq": 3, "op": "work_order_admitted",  "hash": "ee45ff03...", "prior": "7c20a9b1..."},
    {"seq": 4, "op": "solution_manufactured","hash": "9f3a2c11...", "prior": "ee45ff03..."}
  ]
}
```

## Strip-and-rehash protocol

The verifier strips the receipt header from the artifact body before re-hashing. Two carrier formats are supported:

1. **Inline header** (Rust `//`, Erlang `%%`, TTL/Python/shell `#`). The first contiguous block of lines matching `^<prefix> ostar-[a-z-]+: .+$` is the header. The line `ostar-artifact-hash: <hex>` names the BLAKE3 of the file body **after** the entire header block has been stripped.
2. **Sidecar JSON** (Terraform `iac/.ontostar-receipt.json`). Used when the carrier format's schema is closed (Terraform's top-level JSON schema rejects unknown keys; commit `c4e0035` documents this fix-forward).

The verifier protocol is documented in `src/verify.rs` so any external auditor can reimplement it.

## What you can prove from the chain alone

- Every operation in the session is admitted (no gaps in `sequence`).
- Every artifact's bytes hash to its receipt's `artifact_hash`.
- Every receipt's `prior_receipt` is the `receipt_hash` of `sequence - 1`.
- Every `defects_taxonomy_version` agrees with the constant baked into the verifier binary at build time (mismatched taxonomies surface as a verification failure rather than a silent drift).
