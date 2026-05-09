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

        // Bypass-revoked sessions auto-deny.
        if store.session_is_revoked(session_id).unwrap_or(false) {
            self.emit_denied(store, session_id, op, &DefectClass::BypassRevoked);
            return Err((DefectClass::BypassRevoked, vec![]));
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
        let anchor_event_id = format!("workflow_declared:{}:{}", scope_token, &powl_hash_hex[..16]);
        let _ = store.emit_event(
            &anchor_event_id,
            "workflow_declared",
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
        // prior receipt if any; A13 echoes the OCEL canonical hash as the
        // deterministic replay output (POWL bridge is deterministic).
        let provenance_evidence: Vec<String> = vec![artifact_hash_hex.clone()];
        let granted_at_chain: Vec<String> = vec![chrono::Utc::now().to_rfc3339()];
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
            replay_canonical_hash: &ocel_trace_hash_hex,
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
        let _ = store.emit_event(
            &event_id,
            "admission_denied",
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
