# 4.1 The Cell8 Formal Conformance Suite (`src/cell_ready.rs`)

The `cell_ready` function is the singular authority in the codebase permitted to certify manufacturing success. It enforces the 13 canonical gates (A1-A13). It is strictly typed; it returns `Result<Receipt, DefectClass>` and never utilizes string-based errors.

## 4.1.1 Cryptographic Attestation (Gate A10)
A10 guarantees that a system mutation was authorized by a known principal. The gate performs strict Ed25519 signature verification:

```rust
let msg = preview.canonical_bytes_for_signing();
match attestation::verify_strict(trust, fpr, &msg, &sig) {
    VerifyOutcome::Valid => { /* Pass */ }
    VerifyOutcome::UnknownKey => return Err(DefectClass::AttestationInvalid { ... }),
    VerifyOutcome::SignatureInvalid => return Err(DefectClass::AttestationInvalid { ... }),
}
```
Crucially, `preview` is the exact `ProductionRecord` that the gate is about to build. This "Receipt-Replay Defense" ensures the signature covers the canonical JSON representation of the final state, preventing a valid signature from being replayed on a modified record.

## 4.1.2 Temporal Validity and The Bootstrap Gate (Gate A11)
A11 (`cell_ready.rs`, line 363) enforces strict monotonicity of the receipt chain. It evaluates `granted_at_chain.windows(2)` and computes the millisecond skew. If the later timestamp is older than the earlier, it returns `DefectClass::TemporalSkew`.

**R8-1 Extension: The Bootstrap Lock:**
To prevent attackers from injecting synthetic history after the initial system bootstrap, the gate checks `inp.post_bootstrap`.
```rust
if inp.post_bootstrap
    && inp.granted_at_chain.len() < 2
    && inp.prior_tenant_receipt_count > 0
{
    return Err(DefectClass::BootstrapChainTooShort);
}
```
If the system is locked (`post_bootstrap = true`), any chain shorter than 2 (meaning the independent DB query failed to find the required historical seed receipt) is immediately rejected. This closes the history-spoofing vulnerability.

## 4.2 Independent Evidence Gathering (`src/admission.rs`)

To feed `cell_ready`, the `OntoStarAdmissionGate::evaluate` function meticulously avoids tautologies. For A9 (Provenance) and A12 (Dependency Closure), it does not pass locally generated vectors.

For example, `re_read_admitted_receipts` runs a completely independent SQLite query:
```sql
SELECT receipt_hash FROM receipts WHERE receipt_hash = ?1 AND tenant_id = ?2
```
If the prior receipt row was deleted mid-flight (simulated via the `A12_ADMITTED_RECEIPTS_REREAD_HOOK`), this query returns an empty set. `cell_ready` then correctly fails Gate A12 with `DefectClass::DependencyClosureBroken`. This structural separation between the "gauge" (in-flight state) and the "witness" (DB state) is the foundation of the pipeline's zero-trust architecture.
