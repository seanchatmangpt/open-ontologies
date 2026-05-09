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

use crate::attestation::{ArcSwap, Signer, TrustedKeys};
use crate::cell_ready::{cell_ready, CellReadyInputs, PowlOpRef};
use crate::defects::{DefectClass, Deviation};
use crate::ocel_store::OcelStore;
use crate::production_record::hex32_pub;
use crate::receipts::{self, Receipt};
use crate::state::StateDb;

// R5 WB-1 — §15 A13 ReplayProof tautology closure.
//
// Test-only hook fired BETWEEN the line-519 OCEL hash and the independent
// re-snapshot computed in `re_snapshot_ocel_for_replay_proof`. Tests inject
// a synthetic OCEL mutation here to provoke a real ReplayDivergence rather
// than relying on flaky timing-based race tests.
//
// Gated on `debug_assertions` so release builds (`cargo build --release`)
// strip the entire thread_local plus the `with(...)` call inside
// `re_snapshot_ocel_for_replay_proof`. Integration tests in `tests/` build
// the lib WITHOUT `#[cfg(test)]` so we cannot use that gate; but
// `debug_assertions` IS set for `cargo test`, `cargo build`, and
// integration tests, and unset for `cargo build --release` — exactly the
// envelope we need.
//
// `#[doc(hidden)]` keeps the symbol out of public docs even though it is
// `pub` (required for integration-test visibility).
//
// Single-threaded by virtue of `thread_local!`; tests that want
// cross-thread races must wrap their own synchronisation primitives
// inside the closure they install.
#[cfg(debug_assertions)]
#[doc(hidden)]
pub type A13BetweenSnapshotFn =
    Box<dyn Fn(&OcelStore, &str, &str) + Send + 'static>;

#[cfg(debug_assertions)]
thread_local! {
    #[doc(hidden)]
    pub static A13_BETWEEN_SNAPSHOT_HOOK:
        std::cell::RefCell<Option<A13BetweenSnapshotFn>>
        = const { std::cell::RefCell::new(None) };
}

/// What kind of mutation is being requested at the gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionOp {
    // Full-admission ops (graph or external mutation).
    Apply,
    Codegen,
    Save,
    Push,
    Ingest,        // CSV/JSON/SQL ingest, pipeline extend
    ImportSchema,  // DB schema → OWL
    Align,         // auto-applied equivalentClass / subClassOf
    Rollback,      // restore from snapshot
    Version,       // create snapshot
    // Audit-only ops (operator-tier maintenance; logged but never denied).
    Clear,         // clear / unload / cache-remove
    Feedback,      // align_feedback / monitor_clear
    // Requirements-Andon / CTQ-Forge ops (full admission).
    RequirementProposed, // capture source signal + voice
    CtqAdmitted,         // deterministic CTQ admission gate
    WorkOrderAdmitted,   // bind admitted CTQ + counterfactual
    // Audit-only LLM boundary translation (Groq).
    LlmTranslate,
    // Audit-only Loop 3 workflow discovery (inserts discovered_workflows row).
    Discovery,
    // Audit-only Loop 2 threshold-calibration sweep
    // (inserts ocel_events + workflow_thresholds rows).
    ThresholdSweep,
    // Full admission for multi-target solution manufacturing
    // (IaC + Rust + Erlang + AtomVM emitted as a coherent stack).
    SolutionManufactured,
    // Audit-only: a server session changed its tenant context mid-stream.
    // Emits a loud OCEL `tenant_switch` event so a downstream auditor can
    // detect any rotation of effective tenant identity within a session.
    TenantSwitch,
    // R4 WE — §14 mutation gate purity: 5 new variants for handlers that
    // were falsely allowlisted as read-only or whose mutations had no
    // self-attribution.
    /// Full admission. Wraps `WorkflowScope::open(...)` from
    /// `onto_declare_workflow`. The artifact bytes are
    /// `name + "\0" + powl + "\0" + tenant_id`.
    /// TODO(R3 W3): add `op_class()` arm "governance" once the method lands.
    WorkflowDeclared,
    /// Full admission. Wraps `WorkflowScope::close(...)` from
    /// `onto_close_workflow`. The artifact bytes are the raw
    /// `scope_token` bytes.
    /// TODO(R3 W3): add `op_class()` arm "governance" once the method lands.
    WorkflowClosed,
    /// Full admission. Wraps the planner's INSERT into `workflow_scopes`
    /// from `onto_plan_workflow` (both groq_powl and mustar paths).
    /// TODO(R3 W3): add `op_class()` arm "data" once the method lands.
    WorkflowPlanned,
    /// Audit-only. Wraps `OcelStore::seed_from_ocel_bytes` invoked from
    /// `onto_exemplar_seed`. Bootstrap-only — gated by
    /// [`crate::bootstrap::BootstrapState::is_bootstrap`].
    /// TODO(R3 W3): add `op_class()` arm "bootstrap" once the method lands.
    ExemplarSeeded,
    /// Audit-only. Self-attribution for the `bypass_admission` branch
    /// before `revoked_sessions` is written. Pairs with the existing
    /// `admission_bypass` event for backward compat.
    /// TODO(R3 W3): add `op_class()` arm "governance" once the method lands.
    Bypass,
    // ── R5 WC-2 — admin-only operational tools ────────────────────────
    /// Audit-only. Last-resort recovery for the `bootstrap_lock` row.
    /// Distinct OCEL audit name (`bootstrap_unlock`) so an external
    /// auditor reviewing the trail sees a different event_type from
    /// any normal operation. Always admin-gated at the handler.
    /// TODO(R3 W3): add `op_class()` arm "governance" once the method lands.
    BootstrapUnlock,
    /// Audit-only. Bulk soft-delete (UPDATE production_law_version)
    /// over a `scope_token` GLOB pattern. Audit event name
    /// `receipts_revoke_batch` carries the pattern + reason + count
    /// so auditors can correlate against the affected receipts table.
    /// TODO(R3 W3): add `op_class()` arm "data" once the method lands.
    ReceiptsBatchRevoke,
    /// Audit-only. Bulk-INSERT into `revoked_sessions` for every active
    /// scope owned by a principal. Distinct from generic `Bypass`:
    /// `Bypass` tags self-attributed bypass ops by the caller; this
    /// tag marks an admin forcefully revoking *another* principal's
    /// sessions.
    /// TODO(R3 Task B): switch from `revoked_sessions` fallback to the
    /// canonical `revoked_principals` table once Task B lands.
    /// TODO(R3 W3): add `op_class()` arm "governance" once the method lands.
    SessionRevoke,
}

impl AdmissionOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            AdmissionOp::Apply => "apply",
            AdmissionOp::Codegen => "codegen",
            AdmissionOp::Save => "save",
            AdmissionOp::Push => "push",
            AdmissionOp::Ingest => "ingest",
            AdmissionOp::ImportSchema => "import_schema",
            AdmissionOp::Align => "align",
            AdmissionOp::Rollback => "rollback",
            AdmissionOp::Version => "version",
            AdmissionOp::Clear => "clear",
            AdmissionOp::Feedback => "feedback",
            AdmissionOp::RequirementProposed => "requirement_proposed",
            AdmissionOp::CtqAdmitted => "ctq_admitted",
            AdmissionOp::WorkOrderAdmitted => "work_order_admitted",
            AdmissionOp::LlmTranslate => "llm_translate",
            AdmissionOp::Discovery => "discovery",
            AdmissionOp::ThresholdSweep => "threshold_sweep",
            AdmissionOp::SolutionManufactured => "solution_manufactured",
            AdmissionOp::TenantSwitch => "tenant_switch",
            AdmissionOp::WorkflowDeclared => "workflow_declared",
            AdmissionOp::WorkflowClosed => "workflow_closed",
            AdmissionOp::WorkflowPlanned => "workflow_planned",
            AdmissionOp::ExemplarSeeded => "exemplar_seeded",
            AdmissionOp::Bypass => "bypass",
            // R5 WC-2 — distinct OCEL audit names per admin tool.
            AdmissionOp::BootstrapUnlock => "bootstrap_unlock",
            AdmissionOp::ReceiptsBatchRevoke => "receipts_batch_revoke",
            AdmissionOp::SessionRevoke => "session_revoke",
        }
    }

    /// True for ops handled by the full admission gate (replay + receipt).
    /// False for audit-only ops (logged, never denied). `Version` is audit-only
    /// because snapshot creation is non-destructive metadata — taking a
    /// snapshot can never make the system worse, only more recoverable.
    pub fn is_full_admission(&self) -> bool {
        !matches!(
            self,
            AdmissionOp::Clear
                | AdmissionOp::Feedback
                | AdmissionOp::Version
                | AdmissionOp::LlmTranslate
                | AdmissionOp::Discovery
                | AdmissionOp::ThresholdSweep
                | AdmissionOp::TenantSwitch
                | AdmissionOp::ExemplarSeeded
                | AdmissionOp::Bypass
                // R5 WC-2 — admin tools are audit-only: they emit a
                // tamper-evident OCEL trail but cannot deny themselves
                // (they are the recovery path; full admission would
                // create a deadlock for `onto_bootstrap_unlock` in
                // particular).
                | AdmissionOp::BootstrapUnlock
                | AdmissionOp::ReceiptsBatchRevoke
                | AdmissionOp::SessionRevoke
        )
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

/// **Stream-2 stub.** Returns a perfect-fit verdict. Retained because some
/// admission unit tests (`tests/admission.rs`) need a deterministic
/// pass-through to exercise the gate's other defect classes in isolation
/// from the wasm4pm parser. Production code uses [`PowlBridgeReplay`].
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

/// Production-grade [`PowlReplay`] — parses the declared POWL via
/// [`crate::powl_bridge::PowlBridge`], projects the OCEL trace tagged
/// with `scope_token`, and returns the wasm4pm-derived fitness /
/// precision verdict. Falls back to a `ReplayFailed`-shaped result
/// (verdict="non_conform", fitness=0.0) when the POWL string is
/// syntactically invalid or replay errors.
pub struct PowlBridgeReplay<'a> {
    store: &'a OcelStore,
}

impl<'a> PowlBridgeReplay<'a> {
    pub fn new(store: &'a OcelStore) -> Self {
        Self { store }
    }
}

impl<'a> PowlReplay for PowlBridgeReplay<'a> {
    fn replay(&self, scope_token: &str, powl_string: &str) -> ConformanceResult {
        let mut bridge = crate::powl_bridge::PowlBridge::new();
        let root = match bridge.parse(powl_string) {
            Ok(r) => r,
            Err(e) => {
                return ConformanceResult {
                    fitness: 0.0,
                    precision: 0.0,
                    verdict: format!("non_conform:parse:{e}"),
                    run_id: format!("powl-bridge-parse-fail-{scope_token}"),
                };
            }
        };
        match self.store.replay_against_powl(scope_token, &bridge, root) {
            Ok(r) => ConformanceResult {
                fitness: r.fitness,
                precision: r.precision.unwrap_or(0.0),
                verdict: r.verdict.to_string(),
                run_id: r.run_id,
            },
            Err(e) => ConformanceResult {
                fitness: 0.0,
                precision: 0.0,
                verdict: format!("non_conform:replay_error:{e}"),
                run_id: format!("powl-bridge-replay-fail-{scope_token}"),
            },
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
    /// Real-Ed25519 attestation: when `Some`, the gate signs every
    /// admitted [`crate::production_record::ProductionRecord`] before
    /// persistence. When `None`, receipts are emitted unsigned (and
    /// `verify_legacy_receipts` controls whether downstream A10 admits
    /// them). Loaded from `OPEN_ONTOLOGIES_SIGNING_KEY_PATH`.
    pub signer: Option<std::sync::Arc<Signer>>,
    /// Trust set used by A10. Loaded from
    /// `OPEN_ONTOLOGIES_TRUSTED_KEYS_DIR`. Required to verify any signed
    /// receipt; unset turns A10 into legacy-only mode.
    ///
    /// Round 4 WD — wrapped in `ArcSwap` for runtime hot-swap. The
    /// `onto_attestation_rotate_keys` MCP tool reads a fresh trust dir,
    /// validates against `ontology/attestation-shapes.ttl`, builds a new
    /// [`TrustedKeys`], and `.store()`s it here without taking any lock.
    /// Readers call `.load()` and get a `Guard<Arc<TrustedKeys>>` whose
    /// deref is the trust set under that snapshot.
    pub trusted_keys: Option<std::sync::Arc<ArcSwap<TrustedKeys>>>,
    /// `[admission] require_attestation`. When `true` (default) and no
    /// signer is configured, admission ALSO refuses to run — a missing
    /// signing key in production is a configuration defect, not a
    /// silent downgrade.
    pub require_attestation: bool,
    /// `[admission] verify_legacy_receipts`. When `true`, A10 admits
    /// receipts with `signature: None` (emits a `legacy_unsigned_receipt`
    /// audit event). When `false` (default), unsigned receipts fail A10
    /// with `DefectClass::AttestationMissing`.
    pub verify_legacy_receipts: bool,
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
            signer: None,
            trusted_keys: None,
            require_attestation: false,
            verify_legacy_receipts: true,
        }
    }

    /// Attach an Ed25519 signer (used to sign every admitted receipt).
    /// Builder-style; returns self.
    pub fn with_signer(mut self, signer: std::sync::Arc<Signer>) -> Self {
        self.signer = Some(signer);
        self
    }

    /// Attach a trust set (used by A10 to verify signatures).
    ///
    /// Round 4 WD — accepts the hot-swap-capable `ArcSwap` wrapper. Use
    /// [`crate::attestation::into_swap`] to build one from a plain
    /// `TrustedKeys`. The deprecated test path that just had an
    /// `Arc<TrustedKeys>` should construct the swap explicitly:
    ///
    /// ```ignore
    /// let swap = open_ontologies::attestation::into_swap(trust);
    /// gate = gate.with_trusted_keys(swap);
    /// ```
    pub fn with_trusted_keys(
        mut self,
        trust: std::sync::Arc<ArcSwap<TrustedKeys>>,
    ) -> Self {
        self.trusted_keys = Some(trust);
        self
    }

    /// Set the `require_attestation` flag. When `true`, admission refuses
    /// to run unless a signer is configured.
    pub fn require_attestation(mut self, v: bool) -> Self {
        self.require_attestation = v;
        self
    }

    /// Set the `verify_legacy_receipts` flag. When `true`, A10 admits
    /// receipts that lack a signature.
    pub fn verify_legacy_receipts(mut self, v: bool) -> Self {
        self.verify_legacy_receipts = v;
        self
    }

    /// Phase 11 — tenant-aware admission. Looks up the scope's tenant_id
    /// from `declared_workflows` and refuses cross-tenant access with a
    /// typed [`DefectClass::TenantBoundary`] defect before any other
    /// admission machinery runs. Same-tenant calls are forwarded to
    /// [`evaluate`] unchanged. Backwards compatible: rows whose
    /// `tenant_id` defaulted to `"default"` are accessible to callers
    /// in `tenant = "default"`.
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_in_tenant<R: PowlReplay>(
        &self,
        scope_token: &str,
        op: AdmissionOp,
        artifact: &ArtifactRef<'_>,
        store: &OcelStore,
        replay: &R,
        session_id: &str,
        powl_string: &str,
        observed_stages: &[String],
        caller_tenant: &str,
    ) -> Result<Receipt, (DefectClass, Vec<Deviation>)> {
        // Look up scope's owning tenant. Missing scope → fall through; the
        // existing `evaluate` will raise the appropriate defect.
        let owner_tenant: String = store
            .db()
            .conn()
            .query_row(
                "SELECT tenant_id FROM declared_workflows WHERE scope_token = ?1",
                rusqlite::params![scope_token],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "default".to_string());
        if owner_tenant != caller_tenant {
            let defect = DefectClass::TenantBoundary {
                from: caller_tenant.to_string(),
                to: owner_tenant.clone(),
            };
            self.emit_denied_for_scope(store, session_id, op, &defect, Some(scope_token));
            return Err((defect, vec![]));
        }
        self.evaluate(
            scope_token,
            op,
            artifact,
            store,
            replay,
            session_id,
            powl_string,
            observed_stages,
        )
    }

    /// Phase 11 — tenant-aware audit emit. Audit-only ops (Clear, Feedback,
    /// LlmTranslate, Discovery, ThresholdSweep, Version, TenantSwitch) cannot
    /// deny, but their `admission_audit` OCEL event must still carry the
    /// caller's tenant_id so an external auditor can scope the trail per
    /// tenant. This method emits the audit event tagged with `caller_tenant`
    /// and never returns an error. Mirrors [`evaluate_in_tenant`] in spirit
    /// but without the gate machinery.
    pub fn evaluate_audit_in_tenant(
        &self,
        op: AdmissionOp,
        artifact: &ArtifactRef<'_>,
        store: &OcelStore,
        session_id: &str,
        scope_token: Option<&str>,
        caller_tenant: &str,
    ) {
        debug_assert!(
            !op.is_full_admission(),
            "evaluate_audit_in_tenant only accepts audit-only ops; got {:?}",
            op
        );
        let artifact_hash = blake3::hash(artifact.bytes);
        let ts = chrono::Utc::now().to_rfc3339();
        let event_id = format!(
            "{}:admission_audit:{}:{}",
            session_id,
            caller_tenant,
            chrono::Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() * 1_000_000),
        );
        let _ = store.emit_event_in_tenant(
            &event_id,
            "admission_audit",
            &ts,
            session_id,
            &[
                ("op", op.as_str()),
                ("artifact_kind", artifact.kind),
                ("artifact_hash", &artifact_hash.to_hex().to_string()),
                ("production_law_version", "ontostar-1.0.0"),
                (
                    "defects_taxonomy_version",
                    crate::defects::DEFECTS_TAXONOMY_VERSION,
                ),
                ("caller_tenant", caller_tenant),
            ],
            &[],
            scope_token,
            caller_tenant,
        );
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

        // require_attestation: if the gate was configured to demand a
        // signer and none is loaded, refuse loud-and-early — a missing
        // signing key is a configuration defect, not a silent downgrade.
        if self.require_attestation && self.signer.is_none() {
            let defect = DefectClass::AttestationInvalid {
                reason: "no_signer_configured".into(),
            };
            self.emit_denied(store, session_id, op, &defect);
            return Err((defect, vec![]));
        }

        // Bypass-revoked sessions auto-deny. R5 WC-1: variant gained a
        // `reason` field; we read it from `revoked_sessions` so the
        // denial carries the original bypass reason all the way through
        // to the auditor (read-only query — no schema change).
        if store.session_is_revoked(session_id).unwrap_or(false) {
            let reason: String = {
                let conn = store.db().conn();
                conn.query_row(
                    "SELECT reason FROM revoked_sessions \
                     WHERE session_id = ?1 AND cleared_at IS NULL \
                     ORDER BY revoked_at DESC LIMIT 1",
                    rusqlite::params![session_id],
                    |r| r.get::<_, String>(0),
                )
                .unwrap_or_default()
            };
            let defect = DefectClass::BypassRevoked { reason };
            self.emit_denied(store, session_id, op, &defect);
            return Err((defect, vec![]));
        }

        // Hash the canonical POWL string.
        let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
        let powl_hash_hex = hex32_pub(&powl_hash);
        let powl_ref = PowlOpRef {
            powl_string,
            powl_hash,
        };

        // Replay portability anchor: emit a `workflow_declared` event once
        // per scope carrying powl_hash + powl_string as OCEL attributes. An
        // external observer with only the OCEL stream can then reconstruct
        // the declared model without reading `declared_workflows`. Idempotent
        // via deterministic event_id keyed on powl_hash.
        //
        // R5 WB-2 — §15 OCEL anchor closure. Plan B identifies this as a
        // load-bearing replay-portability anchor — NOT informational. A
        // downstream `replay_from_ocel_alone` cannot reconstruct the
        // declared model without it. Previously `let _ = store.emit_event(...)`
        // swallowed failure silently and admission proceeded to write a
        // receipt whose model could not be reconstructed from OCEL alone.
        // Now: primary emit; on failure, `workflow_declared_emit_failed`
        // fallback carrying the same powl_hash + powl_string attrs so an
        // auditor can still rebuild the model from a degraded trail.
        let anchor_event_id = format!("workflow_declared:{}:{}", scope_token, &powl_hash_hex[..16]);
        emit_with_fallback(
            store,
            &anchor_event_id,
            "workflow_declared",
            "workflow_declared_emit_failed",
            &chrono::Utc::now().to_rfc3339(),
            session_id,
            &[
                ("powl_hash", &powl_hash_hex),
                ("powl_string", powl_string),
                ("production_law_version", "ontostar-1.0.0"),
                ("defects_taxonomy_version", crate::defects::DEFECTS_TAXONOMY_VERSION),
            ],
            &[],
            Some(scope_token),
            "ontostar.admission.workflow_declared_emit_lost",
        );

        // Build canonical OCEL projection of scope. Until Stream 1's
        // scope_token column lands on ocel_events, project by session.
        let ocel_canonical = canonical_ocel_projection(store, session_id, scope_token);
        let ocel_canonical_hash_bytes = *blake3::hash(&ocel_canonical).as_bytes();
        let ocel_trace_hash_hex = hex32_pub(&ocel_canonical_hash_bytes);
        let artifact_hash_bytes = artifact.hash();
        let artifact_hash_hex = hex32_pub(&artifact_hash_bytes);
        let gate_config_hash_hex = hex32_pub(&self.gate_config_hash);

        // Run conformance via wasm4pm bridge (or stub).
        let mut conf = replay.replay(scope_token, powl_string);
        // Phase 7 Task C.fix: namespace `run_id` with the scope_token. The
        // bridge derives `run_id` from the trace canonical hash alone, which
        // is identical across two scopes that share the same `event_type`
        // sequence. Without scope-prefixing, two concurrent admissions on
        // distinct scopes collide on the `run_id` PRIMARY KEY of
        // `conformance_runs` — `INSERT OR REPLACE` overwrites one scope's
        // row with the other's `scope_token`, and the loser's
        // `has_conforming_replay(scope_token)` lookup returns false →
        // spurious `ReplayFailed` defect. Scope-prefixing makes the key
        // disjoint so concurrent scopes both retain their own row.
        conf.run_id = format!("{}:{}", scope_token, conf.run_id);
        // Persist conformance row so cell_ready's `replay_pass` conjunct can read it.
        persist_conformance_run(store, scope_token, &conf, &ocel_trace_hash_hex);

        // Phase 11: look up the scope's owning tenant. Default to "default"
        // for legacy rows. The chain head is then read PER TENANT so cross-
        // tenant chains are invisible to one another even when they share a
        // session_id.
        let scope_tenant: String = store
            .db()
            .conn()
            .query_row(
                "SELECT tenant_id FROM declared_workflows WHERE scope_token = ?1",
                rusqlite::params![scope_token],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "default".to_string());
        let prior_receipt =
            receipts::latest_for_session_in_tenant(store.db(), session_id, &scope_tenant);

        // Phase-10 13-conjunct evidence — values produced by admission so the
        // gate is self-sufficient. A9 binds the artifact's own hash as its
        // provenance witness (the admission gate IS the generator); A10
        // self-attests with the same hash (placeholder until ed25519-dalek
        // lands); A11 stamps a single monotonic granted_at; A12 admits the
        // prior receipt if any; A13 is now an INDEPENDENT re-snapshot (see
        // `re_snapshot_ocel_for_replay_proof` below) — was previously a
        // tautology that aliased `ocel_trace_hash_hex` for both A13 inputs.
        // TODO(R6 §15.A9): caller-trust-burden — `provenance_evidence` is
        // supplied by caller against own artifact_hash; needs independent
        // verification against receipt chain.
        let provenance_evidence: Vec<String> = vec![artifact_hash_hex.clone()];
        // TODO(R6 §15.A11): caller-trust-burden — `granted_at_chain` only
        // contains a single timestamp at admission; windows(2) check
        // trivially passes; needs receipt-chain reconstruction for
        // monotonicity.
        let granted_at_chain: Vec<String> = vec![chrono::Utc::now().to_rfc3339()];
        // TODO(R6 §15.A12): caller-trust-burden — `admitted_receipts` is
        // `vec![hex(prior_receipt)]` constructed from the same value being
        // checked; needs DB lookup against `receipts WHERE record_hash =
        // prior_receipt`.
        let admitted_receipts: Vec<String> = match prior_receipt.as_ref() {
            Some(h) => vec![hex32_pub(h)],
            None => Vec::new(),
        };

        // Real-Ed25519: when a signer is configured, sign the would-be
        // record (canonical_bytes_for_signing) and pass the signature +
        // fingerprint into cell_ready so its A10 conjunct can
        // `verify_strict` against the trust set. The preview record we
        // sign here MUST match the record cell_ready will build on the
        // ok-path; cell_ready rebuilds the same preview internally for
        // verification, so the bytes are identical by construction.
        let (signature_opt, fpr_opt) = if let Some(signer) = self.signer.as_ref() {
            let preview = crate::production_record::ProductionRecord {
                artifact_hash: artifact_hash_bytes,
                scope_token: scope_token.to_string(),
                declared_powl_hash: powl_hash,
                ocel_canonical_hash: ocel_canonical_hash_bytes,
                conformance_run_id: conf.run_id.clone(),
                gate_config_hash: self.gate_config_hash,
                production_law_version: "ontostar-1.0.0".into(),
                defects_taxonomy_version: crate::defects::DEFECTS_TAXONOMY_VERSION
                    .to_string(),
                gates_passed: vec![
                    "A1_WorkflowDeclared".into(),
                    "A2_ScopeClosed".into(),
                    "A3_OCELComplete".into(),
                    "A4_POWLReplayPass".into(),
                    "A5_ThresholdPass".into(),
                    "A6_RequiredStagesPresent".into(),
                    "A7_NoBypassRevocation".into(),
                    "A8_ReceiptValid".into(),
                    "A9_ProvenanceChain".into(),
                    "A10_ExternalAttestation".into(),
                    "A11_TemporalValidity".into(),
                    "A12_DependencyClosure".into(),
                    "A13_ReplayProof".into(),
                ],
                gates_refused: Vec::new(),
                prior_receipt,
                signature: None,
                signing_key_fpr: None,
            };
            let msg = preview.canonical_bytes_for_signing();
            let sig = signer.sign(&msg);
            (Some(sig.to_bytes()), Some(signer.fingerprint()))
        } else {
            (None, None)
        };
        // Round 4 WD — load the current trust-set snapshot. The guard
        // outlives `inputs` (and the `cell_ready` call below) because we
        // bind it to `_trust_guard` for the duration of the function.
        // Readers see a consistent snapshot even if a concurrent
        // `onto_attestation_rotate_keys` swaps the inner Arc.
        let _trust_guard = self.trusted_keys.as_ref().map(|s| s.load_full());
        let trust_ref: Option<&TrustedKeys> = _trust_guard.as_deref();

        // R5 WB-1 — §15 A13 ReplayProof tautology closure. Previously this
        // struct literal aliased `&ocel_trace_hash_hex` into BOTH the
        // `ocel_trace_hash` and `replay_canonical_hash` fields, so the A13
        // equality check at `cell_ready.rs:378` was vacuously true by
        // construction and the gate could never fail. We now compute an
        // INDEPENDENT re-snapshot of the OCEL projection between the two
        // hashes; under the `#[cfg(test)] A13_BETWEEN_SNAPSHOT_HOOK` a
        // synthetic mutation fired between snapshots produces a real
        // ReplayDivergence — proving the gate is load-bearing.
        let replay_canonical_hash_hex =
            re_snapshot_ocel_for_replay_proof(store, session_id, scope_token);

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
            provenance_evidence: &provenance_evidence,
            external_attestation: "",
            granted_at_chain: &granted_at_chain,
            admitted_receipts: &admitted_receipts,
            replay_canonical_hash: &replay_canonical_hash_hex,
            signature: signature_opt,
            signing_key_fpr: fpr_opt,
            trusted_keys: trust_ref,
            allow_legacy_unsigned: self.verify_legacy_receipts,
            trusted_keys_db: Some(store.db()),
        };

        // Look up workflow_name once so both Ok and Err branches can record
        // capability evidence on `workflow_capability` and update
        // `declared_workflows` per-scope outcome columns.
        let workflow_name: String = store
            .db()
            .conn()
            .query_row(
                "SELECT name FROM declared_workflows WHERE scope_token = ?1",
                rusqlite::params![scope_token],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_default();
        let taxonomy_v = crate::defects::DEFECTS_TAXONOMY_VERSION;

        match cell_ready(inputs, store) {
            Ok(receipt) => {
                // Phase 7 Task C.fix: persist receipt + emit `admission_granted`
                // under a SINGLE SQLite transaction. If either step fails, the
                // whole boundary rolls back: a receipt never lands in the DB
                // without its OCEL witness, and an `admission_granted` event
                // never lands without a backing receipt row. Closes the
                // orphan window documented in tests/receipt_chain_adversarial.
                let atomic_result: Result<(), anyhow::Error> = (|| {
                    let mut conn = store.db().conn();
                    let tx = conn.transaction()?;
                    receipts::persist_with_tenant_in_tx(
                        &tx,
                        &receipt,
                        session_id,
                        &scope_tenant,
                    )?;
                    let event_id = format!(
                        "{}:admission_granted:{}",
                        session_id,
                        chrono::Utc::now().timestamp_millis()
                    );
                    let powl_hash_hex = hex32_pub(&receipt.record.declared_powl_hash);
                    let receipt_hex = receipt.hex();
                    OcelStore::emit_event_in_tenant_in_tx(
                        &tx,
                        &event_id,
                        "admission_granted",
                        &chrono::Utc::now().to_rfc3339(),
                        session_id,
                        &[
                            ("op", op.as_str()),
                            ("receipt_hash", &receipt_hex),
                            ("scope_token", &receipt.record.scope_token),
                            ("production_law_version", &receipt.record.production_law_version),
                            ("defects_taxonomy_version", &receipt.record.defects_taxonomy_version),
                            ("powl_hash", &powl_hash_hex),
                        ],
                        &[],
                        Some(&receipt.record.scope_token),
                        &scope_tenant,
                    )?;
                    tx.commit()?;
                    Ok(())
                })();
                if let Err(_e) = atomic_result {
                    // Either persist or emit failed; transaction was dropped
                    // (rolled back). Surface as ReceiptMissing — neither side
                    // is durable, so the admission is not granted.
                    self.emit_denied(store, session_id, op, &DefectClass::ReceiptMissing);
                    if !workflow_name.is_empty() {
                        let _ = store.db().record_capability(
                            &workflow_name,
                            false,
                            conf.fitness,
                            conf.precision,
                            taxonomy_v,
                        );
                        let _ = store.db().record_workflow_outcome(
                            scope_token, false,
                            conf.fitness, conf.precision,
                            "[\"receipt_missing\"]", "[]",
                            "[]", "[\"ReceiptValid\"]",
                            "{}",
                        );
                    }
                    return Err((DefectClass::ReceiptMissing, vec![]));
                }
                if !workflow_name.is_empty() {
                    let _ = store.db().record_capability(
                        &workflow_name,
                        true,
                        conf.fitness,
                        conf.precision,
                        taxonomy_v,
                    );
                    let gates_fired_json = serde_json::to_string(&receipt.record.gates_passed)
                        .unwrap_or_else(|_| "[]".into());
                    let manufacturing_delta_json = serde_json::json!({
                        "fired_only_under_ontostar": receipt.record.gates_passed,
                        "naked_craft_verdict": "granted_by_force",
                    })
                    .to_string();
                    let _ = store.db().record_workflow_outcome(
                        scope_token, true,
                        conf.fitness, conf.precision,
                        "[]", "[]",
                        &gates_fired_json, "[]",
                        &manufacturing_delta_json,
                    );
                }
                Ok(receipt)
            }
            Err(defect) => {
                self.emit_denied(store, session_id, op, &defect);
                if !workflow_name.is_empty() {
                    let _ = store.db().record_capability(
                        &workflow_name,
                        false,
                        conf.fitness,
                        conf.precision,
                        taxonomy_v,
                    );
                    let denied_tag = defect.tag();
                    let defects_json = serde_json::to_string(&vec![&defect])
                        .unwrap_or_else(|_| "[]".into());
                    let gates_denied_json = serde_json::to_string(&vec![denied_tag])
                        .unwrap_or_else(|_| "[]".into());
                    let _ = store.db().record_workflow_outcome(
                        scope_token, false,
                        conf.fitness, conf.precision,
                        &defects_json, "[]",
                        "[]", &gates_denied_json,
                        "{}",
                    );
                }
                Err((defect, vec![]))
            }
        }
    }

    // Phase 7 Task C.fix: `emit_granted` was inlined into `evaluate` so the
    // receipt persist + OCEL emit run under a single transaction. The
    // standalone helper was removed because it was the seam through which
    // partial-success orphans could appear (receipt durable, emit failed).

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
        // R5 WB-2 — §15 OCEL anchor closure. Previously this was
        // `let _ = store.emit_event(...)` — a phantom-denial swallow
        // (the caller saw `Err(...)` but OCEL had no witness, so a
        // downstream auditor mining only OCEL could not see the deny).
        // Now: primary emit; on failure, secondary `admission_denied_ocel_failed`
        // emit; on double-failure, tracing::error so OTEL still surfaces
        // the loss.
        emit_with_fallback(
            store,
            &event_id,
            "admission_denied",
            "admission_denied_ocel_failed",
            &chrono::Utc::now().to_rfc3339(),
            session_id,
            &[
                ("op", op.as_str()),
                ("defect", defect.tag()),
                ("production_law_version", "ontostar-1.0.0"),
                ("defects_taxonomy_version", crate::defects::DEFECTS_TAXONOMY_VERSION),
            ],
            &[],
            scope_token,
            "ontostar.admission.denied_emit_lost",
        );
    }
}

/// R5 WB-2 — §15 OCEL anchor closure: primary+fallback+log emit pattern.
///
/// Encapsulates the two-step recovery used by sites that previously did
/// `let _ = store.emit_event(...)` — a phantom-success swallow that lost
/// the OCEL witness whenever SQLite refused (disk full, schema migration
/// in flight, FK violation). Now:
///
/// 1. Try the primary emit. On success, return.
/// 2. On `Err`, attempt a SECONDARY emit with `event_type =
///    <primary>_emit_failed` (or, for `admission_denied`, the historic
///    name `admission_denied_ocel_failed`) so the OCEL trail still has
///    a degraded-but-real anchor an external auditor can mine.
/// 3. If BOTH emits fail (DB is offline / corrupt), log a structured
///    `tracing::error!` so an OTEL collector still records the loss,
///    using the supplied `tracing_target` for namespace clarity.
///
/// External verifiers SHOULD treat `<event_type>_emit_failed` as
/// equivalent-to `<event_type>` plus a `degraded_trail = true` flag.
///
/// Sites:
/// - `admission_denied` (every denial path) — fallback type
///   `admission_denied_ocel_failed`. Phantom denials no longer possible.
/// - `workflow_declared` (replay-portability anchor) — fallback type
///   `workflow_declared_emit_failed`. Load-bearing per Plan B: downstream
///   replay-from-OCEL-alone needs this anchor or the declared model is
///   unrecoverable.
///
/// `fallback_event_type` is supplied explicitly (not derived) so the
/// `admission_denied_ocel_failed` historical name is preserved instead
/// of being broken to `admission_denied_emit_failed` by a naive
/// `format!("{}_emit_failed", primary)`.
#[allow(clippy::too_many_arguments)]
fn emit_with_fallback(
    store: &OcelStore,
    primary_event_id: &str,
    primary_event_type: &str,
    fallback_event_type: &str,
    time_iso: &str,
    session_id: &str,
    attrs: &[(&str, &str)],
    objects: &[(&str, &str)],
    scope_token: Option<&str>,
    tracing_target: &'static str,
) {
    let primary = store.emit_event(
        primary_event_id,
        primary_event_type,
        time_iso,
        session_id,
        attrs,
        objects,
        scope_token,
    );
    if let Err(primary_err) = primary {
        // Build a fresh event_id derived from the primary so an external
        // joiner can correlate the degraded anchor back to the missing
        // primary witness.
        let fallback_event_id = format!("{primary_event_id}:emit_failed");
        let primary_err_str = primary_err.to_string();
        // Carry the primary's attrs forward AND tag the failure cause so
        // a verifier reading only the OCEL stream sees what type was
        // intended and why the primary did not land.
        let mut fallback_attrs: Vec<(&str, &str)> = attrs.to_vec();
        fallback_attrs.push(("intended_event_type", primary_event_type));
        fallback_attrs.push(("primary_emit_error", &primary_err_str));
        let secondary = store.emit_event(
            &fallback_event_id,
            fallback_event_type,
            time_iso,
            session_id,
            &fallback_attrs,
            objects,
            scope_token,
        );
        if let Err(secondary_err) = secondary {
            // Both emits failed. The OCEL trail has lost this anchor —
            // record via `tracing::error!` so OTEL still surfaces it.
            // External operators MUST treat this as a §15 anchor-loss
            // andon: receipts/conformance rows may exist without their
            // OCEL counterpart for the duration of the outage.
            //
            // `tracing::error!`'s `target:` slot must be a string literal
            // (the macro bakes a `DefaultCallsite` static). We dispatch
            // on the supplied namespace so each call site retains its
            // own static callsite.
            let secondary_err_str = secondary_err.to_string();
            match tracing_target {
                "ontostar.admission.denied_emit_lost" => tracing::error!(
                    target: "ontostar.admission.denied_emit_lost",
                    primary_event_id = primary_event_id,
                    primary_event_type = primary_event_type,
                    fallback_event_type = fallback_event_type,
                    primary_error = %primary_err_str,
                    secondary_error = %secondary_err_str,
                    "OCEL emit lost — both primary and fallback failed; \
                     receipts/conformance rows may exist without an OCEL anchor",
                ),
                "ontostar.admission.workflow_declared_emit_lost" => tracing::error!(
                    target: "ontostar.admission.workflow_declared_emit_lost",
                    primary_event_id = primary_event_id,
                    primary_event_type = primary_event_type,
                    fallback_event_type = fallback_event_type,
                    primary_error = %primary_err_str,
                    secondary_error = %secondary_err_str,
                    "OCEL emit lost — both primary and fallback failed; \
                     receipts/conformance rows may exist without an OCEL anchor",
                ),
                _ => tracing::error!(
                    target: "ontostar.admission.emit_lost",
                    namespace = tracing_target,
                    primary_event_id = primary_event_id,
                    primary_event_type = primary_event_type,
                    fallback_event_type = fallback_event_type,
                    primary_error = %primary_err_str,
                    secondary_error = %secondary_err_str,
                    "OCEL emit lost — both primary and fallback failed; \
                     receipts/conformance rows may exist without an OCEL anchor",
                ),
            }
        }
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

/// R5 WB-1 — INDEPENDENT re-snapshot of the OCEL projection used as the
/// A13 ReplayProof witness. Calls `canonical_ocel_projection` a SECOND
/// time and re-hashes via BLAKE3, then converts via `hex32_pub`.
///
/// The line-519 hash is the FIRST snapshot; this is the SECOND. If the
/// store mutates between the two (concurrent OCEL emit, hot-path
/// re-entrancy, time-travel attack), the A13 equality check at
/// `cell_ready.rs:378` will FAIL with `DefectClass::ReplayDivergence`.
/// Previously both inputs aliased the same hex string and A13 was
/// structurally incapable of failing — see `tests/cell_ready_a13_deny_path.rs`
/// for the deterministic deny-path proof.
///
/// Under `#[cfg(test)]`, fires `A13_BETWEEN_SNAPSHOT_HOOK` so tests can
/// inject synthetic mutations without flaky timing — release builds
/// cannot reach the hook.
fn re_snapshot_ocel_for_replay_proof(
    store: &OcelStore,
    session_id: &str,
    scope_token: &str,
) -> String {
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

/// R5 WB-2 — §15 OCEL anchor closure: atomic conformance INSERT + OCEL witness.
///
/// Previously this function did `let _ = conn.execute("INSERT OR REPLACE
/// INTO conformance_runs ...")` and emitted no OCEL witness. A downstream
/// verifier joining `receipts` ↔ `ocel_events` ↔ `conformance_runs` could
/// not prove the conformance row was used at admission — the row was
/// orphan-evidence.
///
/// Now: a single `rusqlite::Transaction` wraps:
///   1. INSERT OR REPLACE into `conformance_runs`
///   2. NEW OCEL event `conformance_recorded` with attrs (run_id, verdict,
///      fitness, precision, scope_token, trace_canonical_hash) emitted via
///      `OcelStore::emit_event_in_tenant_in_tx` on the SAME tx.
///
/// The two commit together or roll back together. If the OCEL emit fails,
/// the conformance row is NOT durable — closing the orphan-evidence
/// window. The INSERT/UPDATE for `workflow_class` and the Loop 5
/// regression hook run AFTER the commit (they are best-effort
/// post-conditions, not part of the atomic anchor).
///
/// We log via `tracing::error!` on the namespace
/// `ontostar.admission.conformance_witness_lost` if the atomic block
/// fails, so OTEL still surfaces the loss; the caller's admission flow
/// continues — `cell_ready`'s `replay_pass` conjunct will refuse with
/// `ReplayFailed` because no row exists for it to read.
fn persist_conformance_run(
    store: &OcelStore,
    scope_token: &str,
    conf: &ConformanceResult,
    trace_hash_hex: &str,
) {
    // Run the stub migration on its own (it is a no-op SQL string but keep
    // the historical call so external diff readers see no behaviour change
    // outside the new tx).
    {
        let conn = store.db().conn();
        let _ = conn.execute_batch(crate::receipts::STREAM3_STUB_MIGRATION);
    }

    // Build the OCEL witness payload up front. Strings live for the tx
    // duration so `&str` slices into them are valid through commit.
    let event_id = format!(
        "conformance_recorded:{}:{}",
        scope_token, conf.run_id,
    );
    let now = chrono::Utc::now().to_rfc3339();
    let fitness_s = format!("{}", conf.fitness);
    let precision_s = format!("{}", conf.precision);

    let atomic: Result<(), anyhow::Error> = (|| {
        let mut conn = store.db().conn();
        let tx = conn.transaction()?;
        tx.execute(
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
                now,
            ],
        )?;
        OcelStore::emit_event_in_tenant_in_tx(
            &tx,
            &event_id,
            "conformance_recorded",
            &now,
            // session_id is not threaded through `persist_conformance_run`;
            // tag the witness with a synthetic session anchored to the
            // run_id so an OCEL projector can recover the linkage via
            // `attrs.run_id` without leaking real session ids into the
            // join.
            "conformance",
            &[
                ("run_id", &conf.run_id),
                ("verdict", &conf.verdict),
                ("fitness", &fitness_s),
                ("precision", &precision_s),
                ("scope_token", scope_token),
                ("trace_canonical_hash", trace_hash_hex),
                ("production_law_version", "ontostar-1.0.0"),
                ("defects_taxonomy_version", crate::defects::DEFECTS_TAXONOMY_VERSION),
            ],
            &[],
            Some(scope_token),
            "default",
        )?;
        tx.commit()?;
        Ok(())
    })();

    if let Err(e) = atomic {
        tracing::error!(
            target: "ontostar.admission.conformance_witness_lost",
            scope_token = scope_token,
            run_id = %conf.run_id,
            error = %e,
            "conformance_runs INSERT + OCEL witness rolled back together; \
             cell_ready replay_pass will refuse with ReplayFailed for this run",
        );
        return;
    }

    // Best-effort post-conditions (NOT part of the atomic anchor). Stamp
    // workflow_class from declared_workflows so Loop 5 (regression
    // detection) can group rolling means by class.
    let conn = store.db().conn();
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
