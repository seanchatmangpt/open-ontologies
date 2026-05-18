# 3.1 The PowlBridge: Deterministic Conformance via wasm4pm

The methodology for behavioral enforcement in the Ostar pipeline hinges on the `PowlBridge` (`src/powl_bridge.rs`). The system imposes a **HARD CONSTRAINT: zero local process-mining math.** Every conformance calculation is delegated entirely to the `wasm4pm` engine.

## 3.1.1 Token Replay Delegation
The bridge acts as an adapter. It parses the declarative POWL string into a `PowlArena` using `wasm4pm::powl_parser::parse_powl_model_string`. It then converts the POWL AST into a Petri Net (`wasm4pm::powl::conversion::to_petri_net::apply`).

When evaluating a trace, the bridge projects the OCEL log into a flat string slice and calls `wasm4pm::powl::conformance::token_replay::replay_trace`. 

## 3.1.2 Alignment-to-Defect Mapping (`classify_replay`)
Instead of rejecting traces with generic errors, the bridge maps the $O(n)$ token replay artifacts into typed defects (`src/powl_bridge.rs`, lines 135-212):

1. **`DefectClass::ExtraTask`:** Triggered when the trace contains activities not present in the declared POWL alphabet.
2. **`DefectClass::SkippedTask`:** Triggered when an activity in the POWL alphabet is missing from the trace.
3. **`DefectClass::WrongOrder`:** Triggered when `replay.remaining_tokens > 0` and there are no missing or extra activities. This proves the activities fired, but not in the sequence dictated by the Petri net markings.
4. **`DefectClass::ReplayFailed`:** A catch-all for fitness < 1.0 when not captured by the above.

## 3.2 Anti-Tautology Hooks: The Independent Witness

A major vulnerability in naive validation gates is tautological reasoning (where the system trusts the very memory structures it just mutated). `src/admission.rs` mitigates this via independent re-reads and Saboteur Hooks.

### 3.2.1 A13 Between-Snapshot Hook
The ReplayProof gate (A13) ensures the OCEL trace used for conformance hasn't changed. To prevent time-of-check to time-of-use (TOCTOU) vulnerabilities, `admission.rs` defines `re_snapshot_ocel_for_replay_proof`.

```rust
fn re_snapshot_ocel_for_replay_proof(store: &OcelStore, session_id: &str, scope_token: &str) -> String {
    #[cfg(debug_assertions)]
    A13_BETWEEN_SNAPSHOT_HOOK.with(|h| {
        if let Some(hook) = h.borrow().as_ref() {
            hook(store, session_id, scope_token);
        }
    });
    let projection = canonical_ocel_projection(store, session_id, scope_token);
    let bytes = *blake3::hash(&projection).as_bytes();
    hex32_pub(&bytes)
}
```
The `A13_BETWEEN_SNAPSHOT_HOOK` is an adversarial test vector. It allows the test suite to inject a synthetic OCEL mutation precisely between the first projection and this second projection, proving that the A13 gate is load-bearing and will throw a `ReplayDivergence` defect if the state mutates mid-flight.

### 3.2.2 Atomic Conformance Persistence
To close "orphan-evidence" windows, `persist_conformance_run` (`src/admission.rs`, line 883) wraps the SQLite `INSERT OR REPLACE INTO conformance_runs` and the OCEL `emit_event` (`conformance_recorded`) inside a single `rusqlite::Transaction`. If the OCEL emit fails, the SQLite record rolls back, ensuring the system never records a conformance pass that lacks an immutable audit witness.
