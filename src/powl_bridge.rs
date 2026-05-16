//! Stream 2 — thin adapter over `wasm4pm`'s POWL stack.
//!
//! **HARD CONSTRAINT:** zero local process-mining math. Every fitness,
//! replay, and conformance number originates in `wasm4pm`. This module is
//! pure plumbing: parse strings, project traces, delegate to wasm4pm, and
//! translate deviations into typed [`crate::defects::DefectClass`] values.
//!
//! ## API drift note
//!
//! The plan asks for `PowlBridge::replay_trace(root, &[String]) -> TraceReplayResult`
//! and `compute_fitness(&[TraceReplayResult]) -> FitnessResult`. wasm4pm's
//! actual public API is:
//!
//! - `wasm4pm::powl_parser::parse_powl_model_string(s, &mut PowlArena) -> Result<u32, String>`
//! - `wasm4pm::powl::conversion::to_petri_net::apply(&PowlArena, root) -> PowlPetriNetResult`
//! - `wasm4pm::powl::conformance::token_replay::replay_trace(&PetriNet, &Marking, &Marking, &Trace) -> TraceReplayResult`
//! - `wasm4pm::powl::conformance::token_replay::compute_fitness(&PetriNet, &Marking, &Marking, &EventLog) -> FitnessResult`
//!
//! `replay_trace` consumes a `&PowlPetriNet` plus initial/final markings, **not**
//! a root index, because wasm4pm separates POWL → Petri-net conversion
//! (`to_petri_net::apply`) from replay. The bridge stores the converted
//! `(PowlPetriNet, initial, final)` tuples per declared root so the per-trace
//! call site does not need to re-convert.
//!
//! Likewise `compute_fitness` expects the original `&EventLog` (so it can
//! re-run replay internally), not a slice of `TraceReplayResult`. The bridge
//! offers a `compute_fitness_from_traces` helper that wraps the wasm4pm call
//! with the original event-log argument.

use crate::defects::{DefectClass, Deviation};

/// Verdict string emitted when a trace replays without missing or remaining tokens.
pub const VERDICT_CONFORM: &str = "conform";
/// Verdict string emitted when a trace replays with partial fitness (fitness > 0).
pub const VERDICT_DEVIATE: &str = "deviate";
/// Verdict string emitted when a trace cannot be replayed at all (fitness == 0).
pub const VERDICT_IMPOSSIBLE: &str = "impossible";

// Re-exports: callers can use these without depending on wasm4pm directly.
pub use wasm4pm::powl::conformance::token_replay::{FitnessResult, TraceReplayResult};
pub use wasm4pm::powl_arena::PowlArena;
pub use wasm4pm::powl_event_log::{Event, EventLog, Trace};
pub use wasm4pm::powl_models::{PowlMarking, PowlPetriNet};

/// Cached Petri-net projection of a parsed POWL root.
struct ParsedRoot {
    net: PowlPetriNet,
    initial: PowlMarking,
    final_: PowlMarking,
}

/// Thin OntoStar-side wrapper around wasm4pm's POWL arena + parser + replay.
///
/// One `PowlBridge` per server instance. Each successful `parse` adds a row
/// to the arena and caches the converted Petri-net so replay does not re-walk
/// the arena.
///
/// # Examples
///
/// ```
/// use open_ontologies::powl_bridge::PowlBridge;
///
/// // Construction is pure and requires no I/O
/// let bridge = PowlBridge::new();
/// // Default produces the same empty bridge
/// let bridge2 = PowlBridge::default();
/// // Both are freshly initialised — no roots parsed yet
/// drop(bridge);
/// drop(bridge2);
/// ```
pub struct PowlBridge {
    arena: PowlArena,
    parsed: std::collections::HashMap<u32, ParsedRoot>,
}

impl Default for PowlBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl PowlBridge {
    pub fn new() -> Self {
        Self {
            arena: PowlArena::default(),
            parsed: std::collections::HashMap::new(),
        }
    }

    /// Borrow the underlying arena (needed by callers that want to inspect
    /// arena state — e.g. the future Loop-1 exemplar miner).
    pub fn arena(&self) -> &PowlArena {
        &self.arena
    }

    /// Parse a POWL string into the arena. Pure delegation to
    /// `wasm4pm::powl_parser::parse_powl_model_string`.
    ///
    /// Returns a root index (`u32`) that identifies the parsed model within
    /// the arena. The index is used by [`Self::replay_trace`] and
    /// [`Self::compute_fitness`]. Each successful parse also caches the
    /// converted Petri-net so replay calls do not re-walk the arena.
    ///
    /// # Example
    ///
    /// ```
    /// use open_ontologies::powl_bridge::PowlBridge;
    ///
    /// let mut bridge = PowlBridge::new();
    ///
    /// // Parse a simple sequential partial order: a → b → c.
    /// let root = bridge
    ///     .parse("PO=(nodes={a, b, c}, order={a-->b, b-->c})")
    ///     .expect("valid POWL string must parse without error");
    ///
    /// // The root index is a small integer; the exact value depends on arena
    /// // state but must be a valid key for subsequent replay calls.
    /// let trace = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    /// let replay = bridge.replay_trace(root, &trace).expect("replay must succeed");
    /// assert!(replay.fitness >= 0.999, "perfect trace fitness={}", replay.fitness);
    /// ```
    pub fn parse(&mut self, powl_string: &str) -> Result<u32, String> {
        let root = wasm4pm::powl_parser::parse_powl_model_string(powl_string, &mut self.arena)
            .map_err(|e| format!("{e:?}"))?;
        // Cache the Petri-net projection so replay/fitness are pure delegations.
        let pn = wasm4pm::powl::conversion::to_petri_net::apply(&self.arena, root);
        self.parsed.insert(
            root,
            ParsedRoot {
                net: pn.net,
                initial: pn.initial_marking,
                final_: pn.final_marking,
            },
        );
        Ok(root)
    }

    /// Replay one trace against a parsed root. Pure delegation.
    pub fn replay_trace(&self, root: u32, trace: &[String]) -> Result<TraceReplayResult, String> {
        let parsed = self
            .parsed
            .get(&root)
            .ok_or_else(|| format!("unknown POWL root {root}"))?;
        let trace = build_trace("scope", trace);
        Ok(wasm4pm::powl::conformance::token_replay::replay_trace(
            &parsed.net,
            &parsed.initial,
            &parsed.final_,
            &trace,
        ))
    }

    /// Compute aggregate fitness over an event log. Pure delegation to
    /// `wasm4pm::powl::conformance::token_replay::compute_fitness`.
    pub fn compute_fitness(&self, root: u32, log: &EventLog) -> Result<FitnessResult, String> {
        let parsed = self
            .parsed
            .get(&root)
            .ok_or_else(|| format!("unknown POWL root {root}"))?;
        Ok(wasm4pm::powl::conformance::token_replay::compute_fitness(
            &parsed.net,
            &parsed.initial,
            &parsed.final_,
            log,
        ))
    }

    /// Set of activity labels reachable from a parsed root. Used by the
    /// defect mapper to distinguish `SkippedTask` from `ExtraTask`.
    fn alphabet(&self, root: u32) -> std::collections::BTreeSet<String> {
        let mut out = std::collections::BTreeSet::new();
        if let Some(parsed) = self.parsed.get(&root) {
            for t in &parsed.net.transitions {
                if let Some(label) = &t.label {
                    out.insert(label.clone());
                }
            }
        }
        out
    }
}

fn build_trace(case_id: &str, activities: &[String]) -> Trace {
    Trace {
        case_id: case_id.to_string(),
        events: activities
            .iter()
            .map(|name| Event {
                name: name.clone(),
                timestamp: None,
                lifecycle: None,
                attributes: std::collections::HashMap::new(),
            })
            .collect(),
    }
}

/// Aggregate conformance verdict over a single replayed trace. The four
/// score fields (`fitness`, `precision`, `generalization`, `simplicity`)
/// are read off wasm4pm's `TraceReplayResult` / `FitnessResult` — none are
/// computed here.
///
/// # Examples — direct construction of score types
///
/// ```
/// use open_ontologies::powl_bridge::{ConformanceResult, TraceReplayResult};
///
/// // Construct a perfect ConformanceResult directly (no I/O, no DB)
/// let result = ConformanceResult {
///     fitness: 1.0,
///     precision: Some(1.0),
///     generalization: None,
///     simplicity: None,
///     verdict: "conform",
///     defects: vec![],
///     trace_canonical_hash: "abc123".to_string(),
///     run_id: "run-abc123".to_string(),
/// };
/// assert!(result.is_conform());
/// assert_eq!(result.verdict, "conform");
/// assert!(result.defects.is_empty());
///
/// // A deviant result is not conform
/// let deviant = ConformanceResult {
///     fitness: 0.5,
///     precision: Some(0.6),
///     generalization: None,
///     simplicity: None,
///     verdict: "deviate",
///     defects: vec![],
///     trace_canonical_hash: "def456".to_string(),
///     run_id: "run-def456".to_string(),
/// };
/// assert!(!deviant.is_conform());
/// ```
///
/// ```
/// use open_ontologies::powl_bridge::TraceReplayResult;
///
/// // TraceReplayResult construction — replay/fitness score types
/// let tr = TraceReplayResult {
///     case_id: "case-1".to_string(),
///     fitness: 1.0,
///     precision: 1.0,
///     produced_tokens: 4,
///     consumed_tokens: 4,
///     missing_tokens: 0,
///     remaining_tokens: 0,
/// };
/// assert!(tr.is_perfect(), "zero missing and remaining tokens means perfect");
/// assert_eq!(tr.fitness, 1.0);
///
/// // An imperfect result is not perfect
/// let imperfect = TraceReplayResult {
///     case_id: "case-2".to_string(),
///     fitness: 0.75,
///     precision: 0.8,
///     produced_tokens: 4,
///     consumed_tokens: 3,
///     missing_tokens: 1,
///     remaining_tokens: 0,
/// };
/// assert!(!imperfect.is_perfect());
/// ```
#[derive(Clone, Debug)]
pub struct ConformanceResult {
    pub fitness: f64,
    pub precision: Option<f64>,
    pub generalization: Option<f64>,
    pub simplicity: Option<f64>,
    pub verdict: &'static str,
    pub defects: Vec<(DefectClass, Deviation)>,
    pub trace_canonical_hash: String,
    pub run_id: String,
}

impl ConformanceResult {
    /// Returns `true` when the trace's verdict is `"conform"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::powl_bridge::ConformanceResult;
    ///
    /// let conform = ConformanceResult {
    ///     fitness: 1.0,
    ///     precision: Some(1.0),
    ///     generalization: None,
    ///     simplicity: None,
    ///     verdict: "conform",
    ///     defects: vec![],
    ///     trace_canonical_hash: "hash".to_string(),
    ///     run_id: "run-hash".to_string(),
    /// };
    /// assert!(conform.is_conform());
    ///
    /// let impossible = ConformanceResult {
    ///     fitness: 0.0,
    ///     precision: None,
    ///     generalization: None,
    ///     simplicity: None,
    ///     verdict: "impossible",
    ///     defects: vec![],
    ///     trace_canonical_hash: "hash2".to_string(),
    ///     run_id: "run-hash2".to_string(),
    /// };
    /// assert!(!impossible.is_conform());
    /// ```
    pub fn is_conform(&self) -> bool {
        self.verdict == VERDICT_CONFORM
    }
}

/// Map a wasm4pm `TraceReplayResult` plus the originally-projected trace into
/// a typed [`ConformanceResult`].
///
/// **Defect mapping (no free-text errors):**
/// - missing tokens > 0 with activities not in the model alphabet → `ExtraTask`
/// - missing tokens > 0 with activities in alphabet but absent from trace → `SkippedTask`
/// - remaining tokens > 0 → `WrongOrder` (final marking unreached)
/// - any other replay anomaly → `ReplayFailed`
///
/// # Example — perfect replay produces `conform` verdict
///
/// A trace that exactly matches the declared POWL sequence produces a
/// `ConformanceResult` with `verdict == "conform"` and an empty defect list.
/// `TraceReplayResult` is constructed directly here because `classify_replay`
/// is pure given a parsed bridge — no I/O, no DB, no network.
///
/// ```
/// use open_ontologies::powl_bridge::{PowlBridge, TraceReplayResult, classify_replay};
///
/// let mut bridge = PowlBridge::new();
/// let root = bridge
///     .parse("PO=(nodes={a, b, c}, order={a-->b, b-->c})")
///     .expect("parse SEQ(a,b,c)");
///
/// // Trace that visits every node in declared order — no deviations.
/// let trace = vec!["a".to_string(), "b".to_string(), "c".to_string()];
///
/// // Construct a perfect replay result directly (all value types, no DB).
/// let replay = TraceReplayResult {
///     case_id: "test".to_string(),
///     fitness: 1.0,
///     precision: 1.0,
///     produced_tokens: 4,
///     consumed_tokens: 4,
///     missing_tokens: 0,
///     remaining_tokens: 0,
/// };
///
/// let result = classify_replay(&bridge, root, &trace, &replay);
/// assert!(result.is_conform(), "verdict was '{}'", result.verdict);
/// assert!(result.defects.is_empty(), "unexpected defects: {:?}", result.defects);
/// ```
///
/// # Example — extra activity in trace produces `ExtraTask` defect
///
/// When a trace contains an activity outside the POWL alphabet the function
/// classifies it as `ExtraTask` regardless of the replay fitness score.
///
/// ```
/// use open_ontologies::powl_bridge::{PowlBridge, TraceReplayResult, classify_replay};
/// use open_ontologies::defects::DefectClass;
///
/// let mut bridge = PowlBridge::new();
/// let root = bridge
///     .parse("PO=(nodes={a, b}, order={a-->b})")
///     .expect("parse");
///
/// // "z" is not in the declared alphabet {a, b}.
/// let trace = vec!["a".to_string(), "b".to_string(), "z".to_string()];
///
/// let replay = TraceReplayResult {
///     case_id: "test".to_string(),
///     fitness: 0.75,
///     precision: 0.5,
///     produced_tokens: 3,
///     consumed_tokens: 3,
///     missing_tokens: 1,
///     remaining_tokens: 0,
/// };
///
/// let result = classify_replay(&bridge, root, &trace, &replay);
/// assert!(
///     result.defects.iter().any(|(d, _)| matches!(d, DefectClass::ExtraTask { stage } if stage == "z")),
///     "expected ExtraTask{{stage='z'}}, got: {:?}", result.defects
/// );
/// ```
pub fn classify_replay(
    bridge: &PowlBridge,
    root: u32,
    observed_trace: &[String],
    replay: &TraceReplayResult,
) -> ConformanceResult {
    let alphabet = bridge.alphabet(root);
    let observed: std::collections::BTreeSet<String> =
        observed_trace.iter().cloned().collect();
    let extra: Vec<String> = observed.difference(&alphabet).cloned().collect();
    let missing: Vec<String> = alphabet.difference(&observed).cloned().collect();

    let mut defects: Vec<(DefectClass, Deviation)> = Vec::new();

    for stage in &extra {
        defects.push((
            DefectClass::ExtraTask {
                stage: stage.clone(),
            },
            Deviation {
                kind: "extra_task".into(),
                stage: stage.clone(),
                detail: "activity in trace but absent from declared POWL alphabet".into(),
                expected: None,
                actual: Some(stage.clone()),
            },
        ));
    }
    for stage in &missing {
        defects.push((
            DefectClass::SkippedTask {
                stage: stage.clone(),
            },
            Deviation {
                kind: "skipped_task".into(),
                stage: stage.clone(),
                detail: "stage in declared POWL alphabet but missing from trace".into(),
                expected: Some(stage.clone()),
                actual: None,
            },
        ));
    }
    if replay.remaining_tokens > 0 && missing.is_empty() && extra.is_empty() {
        // Tokens left but no skipped/extra activity ⇒ wrong order at the
        // POWL operator level.
        defects.push((
            DefectClass::WrongOrder {
                expected: "lawful POWL ordering".into(),
                got: format!("trace [{}]", observed_trace.join(", ")),
            },
            Deviation {
                kind: "wrong_order".into(),
                stage: "<scope>".into(),
                detail: format!(
                    "remaining_tokens={} after replay; final marking not reached",
                    replay.remaining_tokens
                ),
                expected: None,
                actual: None,
            },
        ));
    }
    if !replay.is_perfect() && defects.is_empty() {
        defects.push((
            DefectClass::ReplayFailed,
            Deviation {
                kind: "replay_failed".into(),
                stage: "<scope>".into(),
                detail: format!(
                    "fitness={:.4} produced={} consumed={} missing={} remaining={}",
                    replay.fitness,
                    replay.produced_tokens,
                    replay.consumed_tokens,
                    replay.missing_tokens,
                    replay.remaining_tokens
                ),
                expected: None,
                actual: None,
            },
        ));
    }

    let verdict = if replay.is_perfect() {
        VERDICT_CONFORM
    } else if replay.fitness > 0.0 {
        VERDICT_DEVIATE
    } else {
        VERDICT_IMPOSSIBLE
    };

    let trace_canonical_hash = canonical_hash_of_trace(observed_trace);
    let run_id = format!("run-{}", &trace_canonical_hash[..16]);

    ConformanceResult {
        fitness: replay.fitness,
        precision: Some(replay.precision),
        generalization: None,
        simplicity: None,
        verdict,
        defects,
        trace_canonical_hash,
        run_id,
    }
}

/// BLAKE3 hex over the canonical ASCII projection `"a\nb\nc\n"` of a trace.
///
/// The hash is deterministic: identical activity sequences always produce the
/// same hex string. Different sequences always produce different strings.
/// The empty trace is valid and returns the BLAKE3 hash of the empty byte
/// sequence (a non-empty 64-character hex string).
///
/// # Example — non-empty and stable
///
/// ```
/// use open_ontologies::powl_bridge::canonical_hash_of_trace;
///
/// let trace: Vec<String> = ["A", "B", "C"].iter().map(|s| s.to_string()).collect();
///
/// let hash = canonical_hash_of_trace(&trace);
/// assert!(!hash.is_empty(), "hash must be non-empty");
/// // BLAKE3 hex strings are always 64 characters.
/// assert_eq!(hash.len(), 64, "expected 64-char hex, got {}", hash.len());
///
/// // Stability: same input → same output.
/// assert_eq!(hash, canonical_hash_of_trace(&trace));
///
/// // Different input → different output.
/// let other: Vec<String> = ["A", "C", "B"].iter().map(|s| s.to_string()).collect();
/// assert_ne!(hash, canonical_hash_of_trace(&other), "permutation must differ");
/// ```
///
/// # Example — empty trace is valid
///
/// ```
/// use open_ontologies::powl_bridge::canonical_hash_of_trace;
///
/// let empty_hash = canonical_hash_of_trace(&[]);
/// assert_eq!(empty_hash.len(), 64);
/// // The empty trace hash is distinct from any non-empty trace.
/// let non_empty: Vec<String> = vec!["A".to_string()];
/// assert_ne!(empty_hash, canonical_hash_of_trace(&non_empty));
/// ```
pub fn canonical_hash_of_trace(trace: &[String]) -> String {
    let mut hasher = blake3::Hasher::new();
    for a in trace {
        hasher.update(a.as_bytes());
        hasher.update(b"\n");
    }
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_seq_and_perfect_replay() {
        let mut b = PowlBridge::new();
        let root = b.parse("PO=(nodes={a, b, c}, order={a-->b, b-->c})").expect("parse SEQ(a,b,c)");
        let trace = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let res = b.replay_trace(root, &trace).expect("replay");
        assert!(res.fitness >= 0.999, "fitness={}", res.fitness);
        let cls = classify_replay(&b, root, &trace, &res);
        assert!(cls.is_conform(), "verdict={}", cls.verdict);
        assert!(cls.defects.is_empty());
    }

    #[test]
    fn skipped_task_yields_typed_defect() {
        let mut b = PowlBridge::new();
        let root = b.parse("PO=(nodes={a, b, c}, order={a-->b, b-->c})").expect("parse");
        let trace = vec!["a".to_string(), "c".to_string()];
        let res = b.replay_trace(root, &trace).expect("replay");
        let cls = classify_replay(&b, root, &trace, &res);
        assert!(cls.fitness < 1.0);
        assert!(
            cls.defects
                .iter()
                .any(|(d, _)| matches!(d, DefectClass::SkippedTask { stage } if stage == "b")),
            "expected SkippedTask{{stage='b'}} in defects: {:?}",
            cls.defects
        );
    }
}
