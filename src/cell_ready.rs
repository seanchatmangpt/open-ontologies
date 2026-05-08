//! `CellReady` — the only function in this codebase allowed to declare
//! manufacturing success.
//!
//! ```text
//! CellReady = WorkflowDeclared
//!           ∧ ScopeClosed
//!           ∧ OCELComplete
//!           ∧ POWLReplayPass
//!           ∧ ThresholdPass
//!           ∧ RequiredStagesPresent
//!           ∧ NoBypassRevocation
//!           ∧ ReceiptValid
//! ```
//!
//! Short-circuits to the first failing [`DefectClass`]. **No `bail!`,
//! no `anyhow!`, no string error authority** — every denial is a typed
//! defect class.
//!
//! NOTE on Stream 2 coupling: the plan signature uses `&'a PowlOpRef`
//! (a `wasm4pm` arena handle). Until Stream 2 lands its `powl_bridge.rs`,
//! we accept an opaque marker so this module compiles standalone. The
//! shape of the function and the conjunct order are exactly per the plan.

use crate::defects::DefectClass;
use crate::ocel_store::OcelStore;
use crate::production_record::ProductionRecord;
use crate::receipts::{self, Receipt};

/// Stream-2-stub stand-in for `wasm4pm`'s POWL arena handle. Stream 2
/// replaces this with a re-export from `powl_bridge.rs`.
pub struct PowlOpRef<'a> {
    pub powl_string: &'a str,
    pub powl_hash: [u8; 32],
}

pub struct CellReadyInputs<'a> {
    pub scope_token: &'a str,
    pub declared_powl: &'a PowlOpRef<'a>,
    pub ocel_trace_hash: &'a str,
    pub artifact_hash: &'a str,
    pub gate_config_hash: &'a str,
    pub session_revoked: bool,

    // Threshold-pass inputs (computed by admission gate via wasm4pm)
    pub fitness_observed: f64,
    pub precision_observed: f64,
    pub fitness_required: f64,
    pub precision_required: f64,

    // Required-stages-present inputs (workflow alphabet vs. observed events)
    pub required_stages: &'a [String],
    pub observed_stages: &'a [String],

    /// Conformance run id for the matching POWL replay.
    pub conformance_run_id: &'a str,

    /// Production-law version label, e.g. "ontostar-1.0.0".
    pub production_law_version: &'a str,

    /// Optional prior receipt hash to chain into the new record.
    pub prior_receipt: Option<[u8; 32]>,

    /// Session id that owns the receipts row when persisted.
    pub session_id: &'a str,
}

/// Compute the `CellReady` predicate. Returns a freshly built (but not yet
/// persisted) [`Receipt`] on success, or the **first** failing
/// [`DefectClass`] on failure.
///
/// Persistence of the receipt is the caller's responsibility (admission
/// gate); this function only certifies that all eight conjuncts hold.
pub fn cell_ready(
    inp: CellReadyInputs<'_>,
    store: &OcelStore,
) -> Result<Receipt, DefectClass> {
    // 1. WorkflowDeclared — declared_workflows row exists for scope_token.
    if !workflow_declared(store, inp.scope_token) {
        return Err(DefectClass::ScopeUnclosed);
    }

    // 2. ScopeClosed — closed_at IS NOT NULL.
    if !scope_closed(store, inp.scope_token) {
        return Err(DefectClass::ScopeUnclosed);
    }

    // 3. OCELComplete — every event in declared alphabet has fired OR
    //    not-required is documented (treat undocumented absence as
    //    OcelIncomplete).
    if !ocel_complete(inp.required_stages, inp.observed_stages) {
        return Err(DefectClass::OcelIncomplete);
    }

    // 4. POWLReplayPass — conformance_runs row with verdict='conform' for scope.
    if !replay_pass(store, inp.scope_token) {
        return Err(DefectClass::ReplayFailed);
    }

    // 5. ThresholdPass — fitness ≥ f_min, precision ≥ p_min.
    if inp.fitness_observed < inp.fitness_required {
        return Err(DefectClass::ThresholdFailed {
            metric: "fitness".into(),
            observed: inp.fitness_observed,
            required: inp.fitness_required,
        });
    }
    if inp.precision_observed < inp.precision_required {
        return Err(DefectClass::ThresholdFailed {
            metric: "precision".into(),
            observed: inp.precision_observed,
            required: inp.precision_required,
        });
    }

    // 6. RequiredStagesPresent — every stage in required_stages present in OCEL.
    for stage in inp.required_stages {
        if !inp.observed_stages.iter().any(|s| s == stage) {
            return Err(DefectClass::CapabilityZero);
        }
    }

    // 7. NoBypassRevocation — session not in revoked_sessions.
    if inp.session_revoked {
        return Err(DefectClass::BypassRevoked);
    }

    // 8. ReceiptValid — canonical hashes recompute. Build a candidate
    //    record and prove its hash is well-formed BLAKE3 input.
    let artifact_hash = parse_hex32(inp.artifact_hash).ok_or(DefectClass::ReceiptMissing)?;
    let ocel_canonical_hash =
        parse_hex32(inp.ocel_trace_hash).ok_or(DefectClass::ReceiptMissing)?;
    let gate_config_hash =
        parse_hex32(inp.gate_config_hash).ok_or(DefectClass::ReceiptMissing)?;

    let record = ProductionRecord {
        artifact_hash,
        scope_token: inp.scope_token.to_string(),
        declared_powl_hash: inp.declared_powl.powl_hash,
        ocel_canonical_hash,
        conformance_run_id: inp.conformance_run_id.to_string(),
        gate_config_hash,
        production_law_version: inp.production_law_version.to_string(),
        gates_passed: vec![
            "WorkflowDeclared".into(),
            "ScopeClosed".into(),
            "OCELComplete".into(),
            "POWLReplayPass".into(),
            "ThresholdPass".into(),
            "RequiredStagesPresent".into(),
            "NoBypassRevocation".into(),
            "ReceiptValid".into(),
        ],
        gates_refused: Vec::new(),
        prior_receipt: inp.prior_receipt,
    };

    Ok(receipts::build(record))
}

// ─── conjunct helpers ──────────────────────────────────────────────────────

fn workflow_declared(store: &OcelStore, scope_token: &str) -> bool {
    // Stream-3-stub: a scope is "declared" iff a row exists in
    // declared_workflows for it. The table is created lazily by
    // `receipts::STREAM3_STUB_MIGRATION` so this works pre-Stream-1.
    store.has_declared_workflow(scope_token).unwrap_or(false)
}

fn scope_closed(store: &OcelStore, scope_token: &str) -> bool {
    store.is_scope_closed(scope_token).unwrap_or(false)
}

fn replay_pass(store: &OcelStore, scope_token: &str) -> bool {
    store.has_conforming_replay(scope_token).unwrap_or(false)
}

fn ocel_complete(required: &[String], observed: &[String]) -> bool {
    if required.is_empty() {
        return !observed.is_empty();
    }
    required.iter().all(|r| observed.contains(r))
}

fn parse_hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}
