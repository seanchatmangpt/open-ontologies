//! Round 4 WC — compile-time saboteur ratchet.
//!
//! Pins `swarm::fuse_via_hearsay`'s return type to the typed
//! `SwarmConsensus` struct. If a future PR ever weakens the signature
//! to `serde_json::Value` (or any other untyped JSON shape), this test
//! file refuses to compile.
//!
//! Δ>0 PROOF: pre-R4-WC, nothing forced the swarm consensus path to
//! return a typed struct. A refactor that returned
//! `serde_json::Value` (e.g. "for flexibility") would silently drop
//! the structural guarantees on `node_reports`, `consensus_selected`,
//! and `consensus_trace_steps`. Post-R4-WC, that refactor would fail
//! to compile this test file.
//!
//! Zero-runtime: the assertion is a `let _: fn(...) -> SwarmConsensus
//! = fuse_via_hearsay;` binding which the compiler resolves at type
//! check time. The `#[test]` body never runs the function — it just
//! takes the function pointer so the type checker compares the
//! declared signature against the actual one.

use open_ontologies::swarm::{fuse_via_hearsay, SwarmConsensus};
use wasm4pm_cognition::breeds::{BreedInput, BreedOutput};

#[test]
fn fuse_via_hearsay_signature_is_typed() {
    // Compile-time pin. If `fuse_via_hearsay`'s return type changes to
    // anything other than `SwarmConsensus`, the type-checker rejects
    // this binding.
    //
    // We accept the function as a function pointer with the exact
    // signature the swarm module declares: `(&BreedInput,
    // &[(String, BreedOutput)]) -> SwarmConsensus`.
    let _: fn(&BreedInput, &[(String, BreedOutput)]) -> SwarmConsensus = fuse_via_hearsay;

    // Spot-check at runtime: SwarmConsensus has the structured fields
    // we depend on. (Reflection-free; a missing field would have
    // tripped the compiler well before this point.)
    let empty = SwarmConsensus {
        node_reports: vec![],
        consensus_explanation: String::new(),
        consensus_selected: None,
        consensus_trace_steps: 0,
    };
    assert_eq!(empty.node_reports.len(), 0);
    assert!(empty.consensus_selected.is_none());
}
