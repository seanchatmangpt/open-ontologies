//! `OntoStarAdmissionGate` — admission gate that fires before any
//! ontology mutation. Wires together (in this order):
//!
//! 1. resolve declared workflow → POWL via Stream 2's `PowlBridge`
//!    (TODO Stream 2: replace [`PowlReplay`] stub with the real bridge),
//! 2. build a canonical OCEL projection of scope and BLAKE3 it,
//! 3. run conformance via [`PowlReplay`],
//! 4. call [`cell_ready`],
//! 5. on Ok: build [`ProductionRecord`], persist [`Receipt`], emit
//!    `admission_granted` OCEL event with `receipt_hash` attribute,
//! 6. on Err: emit `admission_denied` with typed `defect` attribute and
//!    return an MCP-shaped error tuple.
//!
//! **No `bail!`, no `anyhow!`, no string error authority.** Every denial
//! path returns a typed `(DefectClass, Vec<Deviation>)`.

use crate::cell_ready::{cell_ready, CellReadyInputs, PowlOpRef};
use crate::defects::{DefectClass, Deviation};
use crate::ocel_store::OcelStore;
use crate::production_record::hex32_pub;
use crate::receipts::{self, Receipt};
use crate::state::StateDb;

/// What kind of mutation is being requested at the gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionOp {
    Apply,
    Codegen,
    Save,
    Push,
}

impl AdmissionOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            AdmissionOp::Apply => "apply",
            AdmissionOp::Codegen => "codegen",
            AdmissionOp::Save => "save",
            AdmissionOp::Push => "push",
        }
    }
}

/// Pointer to the artifact bytes the operation would produce. The gate
/// hashes these bytes into the production record. For operations that
/// don't yet have artifact bytes (push, save-with-not-yet-serialized),
/// the gate uses a deterministic stand-in (the current canonical graph
/// or the SPARQL endpoint URL) so the receipt is still chained.
pub struct ArtifactRef<'a> {
    pub kind: &'a str,
    pub bytes: &'a [u8],
}

impl<'a> ArtifactRef<'a> {
    pub fn hash(&self) -> [u8; 32] {
        *blake3::hash(self.bytes).as_bytes()
    }
}

/// Stream-2-stub trait. The real implementation lives in `powl_bridge.rs`
/// and delegates to `wasm4pm`. Until Stream 2 lands, [`NoopPowlReplay`]
/// returns fitness=1.0 and precision=1.0 so the gate is exercisable.
///
/// TODO(stream-2): swap to the wasm4pm-backed bridge. This stub MUST be
/// replaced before production.
pub trait PowlReplay {
    fn replay(&self, scope_token: &str, powl_string: &str) -> ConformanceResult;
}

#[derive(Clone, Debug)]
pub struct ConformanceResult {
    pub fitness: f64,
    pub precision: f64,
    pub verdict: String,
    pub run_id: String,
}

/// **Stream-2 stub.** Returns a perfect-fit verdict. Flagged with a
/// deliberate constant so a CI grep can find this when Stream 2 lands.
pub struct NoopPowlReplay;

pub const STREAM3_STUB_POWL_REPLAY_MARKER: &str = "TODO(stream-2): replace NoopPowlReplay";

impl PowlReplay for NoopPowlReplay {
    fn replay(&self, scope_token: &str, _powl_string: &str) -> ConformanceResult {
        ConformanceResult {
            fitness: 1.0,
            precision: 1.0,
            verdict: "conform".to_string(),
            run_id: format!("stub-run-{}", scope_token),
        }
    }
}

/// Configuration for the admission gate. Field bytes feed `gate_config_hash`.
#[derive(Clone, Debug)]
pub struct OntoStarAdmissionGate {
    pub f_min: f64,
    pub p_min: f64,
    pub required_stages: Vec<String>,
    pub taxonomy_version: String,
    pub gate_config_hash: [u8; 32],
}

impl OntoStarAdmissionGate {
    /// Construct a gate and compute its config hash from inputs.
    pub fn new(
        f_min: f64,
        p_min: f64,
        required_stages: Vec<String>,
        taxonomy_version: impl Into<String>,
    ) -> Self {
        let taxonomy_version = taxonomy_version.into();
        let mut hasher = blake3::Hasher::new();
        hasher.update(&f_min.to_le_bytes());
        hasher.update(&p_min.to_le_bytes());
        for s in &required_stages {
            hasher.update(s.as_bytes());
            hasher.update(&[0u8]);
        }
        hasher.update(taxonomy_version.as_bytes());
        let gate_config_hash = *hasher.finalize().as_bytes();
        Self {
            f_min,
            p_min,
            required_stages,
            taxonomy_version,
            gate_config_hash,
        }
    }

    /// Run admission. On Ok: persist the receipt and emit `admission_granted`.
    /// On Err: emit `admission_denied` with a typed `defect` attribute.
    pub fn evaluate<R: PowlReplay>(
        &self,
        scope_token: &str,
        op: AdmissionOp,
        artifact: &ArtifactRef<'_>,
        store: &OcelStore,
        replay: &R,
        session_id: &str,
        powl_string: &str,
        observed_stages: &[String],
    ) -> Result<Receipt, (DefectClass, Vec<Deviation>)> {
        // No scope_token? Refuse before touching the store.
        if scope_token.is_empty() {
            self.emit_denied(store, session_id, op, &DefectClass::ScopeUnclosed);
            return Err((DefectClass::ScopeUnclosed, vec![]));
        }

        // Bypass-revoked sessions auto-deny.
        if store.session_is_revoked(session_id).unwrap_or(false) {
            self.emit_denied(store, session_id, op, &DefectClass::BypassRevoked);
            return Err((DefectClass::BypassRevoked, vec![]));
        }

        // Hash the canonical POWL string.
        let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
        let powl_ref = PowlOpRef {
            powl_string,
            powl_hash,
        };

        // Build canonical OCEL projection of scope. Until Stream 1's
        // scope_token column lands on ocel_events, project by session.
        let ocel_canonical = canonical_ocel_projection(store, session_id, scope_token);
        let ocel_canonical_hash_bytes = *blake3::hash(&ocel_canonical).as_bytes();
        let ocel_trace_hash_hex = hex32_pub(&ocel_canonical_hash_bytes);
        let artifact_hash_bytes = artifact.hash();
        let artifact_hash_hex = hex32_pub(&artifact_hash_bytes);
        let gate_config_hash_hex = hex32_pub(&self.gate_config_hash);

        // Run conformance via wasm4pm bridge (or stub).
        let conf = replay.replay(scope_token, powl_string);
        // Persist conformance row so cell_ready's `replay_pass` conjunct can read it.
        persist_conformance_run(store, scope_token, &conf, &ocel_trace_hash_hex);

        let prior_receipt = receipts::latest_for_session(store.db(), session_id);

        let inputs = CellReadyInputs {
            scope_token,
            declared_powl: &powl_ref,
            ocel_trace_hash: &ocel_trace_hash_hex,
            artifact_hash: &artifact_hash_hex,
            gate_config_hash: &gate_config_hash_hex,
            session_revoked: false, // already checked above
            fitness_observed: conf.fitness,
            precision_observed: conf.precision,
            fitness_required: self.f_min,
            precision_required: self.p_min,
            required_stages: &self.required_stages,
            observed_stages,
            conformance_run_id: &conf.run_id,
            production_law_version: "ontostar-1.0.0",
            prior_receipt,
            session_id,
        };

        match cell_ready(inputs, store) {
            Ok(receipt) => {
                if let Err(_e) = receipts::persist(&receipt, store.db(), session_id) {
                    // Persistence failure is itself a typed defect.
                    self.emit_denied(store, session_id, op, &DefectClass::ReceiptMissing);
                    return Err((DefectClass::ReceiptMissing, vec![]));
                }
                self.emit_granted(store, session_id, op, &receipt);
                Ok(receipt)
            }
            Err(defect) => {
                self.emit_denied(store, session_id, op, &defect);
                Err((defect, vec![]))
            }
        }
    }

    fn emit_granted(
        &self,
        store: &OcelStore,
        session_id: &str,
        op: AdmissionOp,
        receipt: &Receipt,
    ) {
        let event_id = format!(
            "{}:admission_granted:{}",
            session_id,
            chrono::Utc::now().timestamp_millis()
        );
        let _ = store.emit_event(
            &event_id,
            "admission_granted",
            &chrono::Utc::now().to_rfc3339(),
            session_id,
            &[
                ("op", op.as_str()),
                ("receipt_hash", &receipt.hex()),
                ("scope_token", &receipt.record.scope_token),
            ],
            &[],
            Some(&receipt.record.scope_token),
        );
    }

    fn emit_denied(
        &self,
        store: &OcelStore,
        session_id: &str,
        op: AdmissionOp,
        defect: &DefectClass,
    ) {
        self.emit_denied_for_scope(store, session_id, op, defect, None);
    }

    fn emit_denied_for_scope(
        &self,
        store: &OcelStore,
        session_id: &str,
        op: AdmissionOp,
        defect: &DefectClass,
        scope_token: Option<&str>,
    ) {
        let event_id = format!(
            "{}:admission_denied:{}",
            session_id,
            chrono::Utc::now().timestamp_millis()
        );
        let _ = store.emit_event(
            &event_id,
            "admission_denied",
            &chrono::Utc::now().to_rfc3339(),
            session_id,
            &[("op", op.as_str()), ("defect", defect.tag())],
            &[],
            scope_token,
        );
    }
}

/// Canonical OCEL projection of a scope. Concatenates `event_type` strings
/// in stable order, separated by NULs, to produce a deterministic byte
/// vector suitable for BLAKE3 hashing.
fn canonical_ocel_projection(store: &OcelStore, session_id: &str, scope_token: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(scope_token.as_bytes());
    out.push(0);
    if let Ok(events) = store.observed_event_types_for_session(session_id) {
        for et in events {
            out.extend_from_slice(et.as_bytes());
            out.push(0);
        }
    }
    out
}

fn persist_conformance_run(
    store: &OcelStore,
    scope_token: &str,
    conf: &ConformanceResult,
    trace_hash_hex: &str,
) {
    let conn = store.db().conn();
    let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);
    let _ = conn.execute(
        "INSERT OR REPLACE INTO conformance_runs (
            run_id, scope_token, fitness, precision, generalization, simplicity,
            verdict, defects_json, trace_canonical_hash, ran_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![
            conf.run_id,
            scope_token,
            conf.fitness,
            conf.precision,
            Option::<f64>::None,
            Option::<f64>::None,
            conf.verdict,
            "[]",
            trace_hash_hex,
            chrono::Utc::now().to_rfc3339(),
        ],
    );
    // Best-effort: stamp workflow_class from declared_workflows so Loop 5
    // (regression detection) can group rolling means by class.
    let workflow_class: Option<String> = conn
        .query_row(
            "SELECT name FROM declared_workflows WHERE scope_token = ?1",
            rusqlite::params![scope_token],
            |r| r.get(0),
        )
        .ok();
    if let Some(ref cls) = workflow_class {
        let _ = conn.execute(
            "UPDATE conformance_runs SET workflow_class = ?1 WHERE run_id = ?2",
            rusqlite::params![cls, conf.run_id],
        );
    }
    drop(conn);
    // Loop 5 hook — best-effort.
    if let Some(cls) = workflow_class {
        let _ = crate::feedback::regression::check_after_insert(store, &cls);
    }
}

/// Convenience: revoke a session by writing to `revoked_sessions`.
pub fn revoke_session(db: &StateDb, session_id: &str, reason: &str) -> anyhow::Result<()> {
    let conn = db.conn();
    conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION)?;
    conn.execute(
        "INSERT OR REPLACE INTO revoked_sessions (session_id, reason, revoked_at, cleared_at)
         VALUES (?1, ?2, ?3, NULL)",
        rusqlite::params![session_id, reason, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Convenience: clear a revocation (sets `cleared_at`).
pub fn clear_revocation(db: &StateDb, session_id: &str) -> anyhow::Result<()> {
    let conn = db.conn();
    conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION)?;
    conn.execute(
        "UPDATE revoked_sessions SET cleared_at = ?1 WHERE session_id = ?2",
        rusqlite::params![chrono::Utc::now().to_rfc3339(), session_id],
    )?;
    Ok(())
}
