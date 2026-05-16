//! Typed defect taxonomy for OntoStar admission.
//!
//! Free-text denials are forbidden. Every denial path returns a typed
//! [`DefectClass`] paired with [`Deviation`] evidence. No string error
//! messages as authority.
//!
//! # Versioning
//!
//! The taxonomy carries an explicit semver. External auditors read the
//! `defects_taxonomy_version` attribute on every admission OCEL event and
//! every persisted Receipt to know which set of variants ruled.
//!
//! - Bump MAJOR for renamed/removed variants (breaks existing auditors).
//! - Bump MINOR for added variants (forward-compatible).
//! - Bump PATCH for doc-only changes.
//!
//! [`DEFECTS_TAXONOMY_DISCRIMINANT_HASH`] is the BLAKE3 hash of the
//! concatenated tags (in declaration order, NUL-separated). The CI test
//! [`tests::taxonomy_discriminant_hash_pinned`] forces any variant
//! add/rename/remove to trigger a deliberate version bump.

use serde::{Deserialize, Serialize};

/// Current defect taxonomy semver. Stored on every Receipt and emitted as
/// an attribute on every `admission_granted` / `admission_denied` /
/// `admission_audit` OCEL event.
///
/// Bumped from `2.1.0` → `3.0.0` in Phase 6 after deletion of 10
/// zero-emission speculative variants (`LawZero`, `MissingGatewayChoice`,
/// `UnreachableTask`, `ShaclSkipped`, `ProjectionAsAuthority`, `StubGate`,
/// `UnreplayableClaim`, `FalsePass`, `SecretLeak`,
/// `GeneratedArtifactDirectEdit`).
///
/// Bumped from `3.0.0` → `3.1.0` in Phase 10 after addition of five Phase-10
/// A9–A13 conjunct variants (`ProvenanceMissing`, `AttestationMissing`,
/// `TemporalSkew`, `DependencyClosureBroken`, `ReplayDivergence`).
///
/// Bumped from `3.1.0` → `3.2.0` in Phase 11 after addition of the
/// `TenantBoundary` variant (multi-tenant session isolation defect class).
///
/// Bumped from `3.2.0` → `4.0.0` in Phase 10 final after the A9–A13 conjunct
/// variants were enriched with structured evidence fields (`artifact_hash`,
/// `observed_skew_ms`, `missing_hash`, `expected`, `observed`). The variant
/// enum shape changed (added fields) so external auditors must re-deserialize
/// — MAJOR bump.
///
/// Bumped from `4.0.0` → `4.1.0` after the addition of
/// [`DefectClass::AttestationInvalid`] for the real-Ed25519 A10 path
/// (replaces the digest-equality tautology stub). Forward-compatible —
/// existing variants are unchanged, only one variant is added.
///
/// Bumped from `4.1.0` → `4.2.0` (Round 4 WC) after wiring
/// `LlmAuthorityClaimed` from theatrical-only into a load-bearing
/// emission. The `signature_shape::parse_and_validate` gauge now
/// detects the LLM's `provisional: false` / `authoritative: true`
/// claim, surfaces it via `ParsedFields::llm_claimed_authority`, and
/// `onto_translate_candidate` emits an `llm_authority_claimed` OCEL
/// audit event before lifting the fields into a `CandidateCtq`. The
/// tag set is unchanged (no new variants, no renames) — the
/// discriminant hash carries forward unchanged. Forward-compatible.
///
/// Bumped from `4.2.0` → `4.3.0` (Round 4 WE) after addition of
/// [`DefectClass::BootstrapClosed`] for the `onto_exemplar_seed`
/// bootstrap-window precondition. Forward-compatible — only one
/// variant added, no renames or removals. Discriminant hash changes
/// (a new tag joins `all_tags()`), so [`DEFECTS_TAXONOMY_DISCRIMINANT_HASH`]
/// is updated in lockstep.
///
/// Bumped from `4.3.0` → `4.4.0` (Round 5 WC-1) after enriching
/// [`DefectClass::BypassRevoked`] with a structured `reason` field so
/// the unified bypass denial JSON can surface the operator's reason
/// without auditors parsing free text. The variant tag is unchanged
/// (`bypass_revoked`), so [`DEFECTS_TAXONOMY_DISCRIMINANT_HASH`]
/// remains stable; the version bump is forward-compatible (the new
/// field defaults to an empty string for legacy emitters).
///
/// Bumped from `4.5.0` → `4.7.0` (Round 6 WA-2 + WA-3) after the §15
/// A11 TemporalValidity and A12 DependencyClosure tautology closures.
/// A11: `granted_at_chain` now sourced from an independent re-read of
/// `receipts ORDER BY sequence ASC` rather than a single-element
/// `vec![Utc::now()]` (R6 WA-2). A12: `admitted_receipts` now sourced
/// from a `receipts WHERE receipt_hash = prior_hex` point-lookup rather
/// than `vec![hex(prior_receipt)]` derived from the same Option (R6 WA-3).
/// No variant additions, removals, or renames — `TemporalSkew` and
/// `DependencyClosureBroken` already had correct shapes.
/// [`DEFECTS_TAXONOMY_DISCRIMINANT_HASH`] remains pinned; forward-compatible.
///
/// Bumped from `4.7.0` → `4.8.0` (R8-1) after addition of
/// [`DefectClass::BootstrapChainTooShort`] for the post-bootstrap-lock
/// chain-length gate. When `bootstrap_lock` is active (production mode),
/// `granted_at_chain.len() < 2` is denied — a single-entry chain post-lock
/// indicates admission manufactured without prior history. Forward-compatible.
pub const DEFECTS_TAXONOMY_VERSION: &str = "ontostar-defects-4.8.0";

/// BLAKE3 hex of `tag1\0tag2\0...\0` for [`DefectClass::all_tags()`].
/// CI-pinned. Adding/renaming/removing a variant changes this, forcing a
/// taxonomy version bump.
pub const DEFECTS_TAXONOMY_DISCRIMINANT_HASH: &str =
    "14e16b0cef4527edb0368f6a50fa4b6cce8e5f94c6b54597f5c70f4847765b24";

/// Typed denial classes. Every `Denied` outcome in admission/cell-ready
/// machinery short-circuits on the first failing variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DefectClass {
    /// Required stage missing in OCEL.
    CapabilityZero,
    SkippedTask { stage: String },
    ExtraTask { stage: String },
    WrongOrder { expected: String, got: String },
    /// Session under `bypass_admission` revocation. R5 WC-1: enriched
    /// with a structured `reason` field so the unified bypass denial
    /// JSON (`Err({ok:false, admission:"bypassed_session_revoked",
    /// defect:{kind:"BypassRevoked", reason:..}})`) surfaces the
    /// operator's reason without auditors parsing free text. The tag
    /// remains `"bypass_revoked"` so [`all_tags`] / discriminant hash
    /// are stable; only the variant shape changed (additive, with
    /// `#[serde(default)]` for forward compat).
    BypassRevoked {
        #[serde(default)]
        reason: String,
    },
    ReceiptMissing,
    ScopeUnclosed,
    OcelIncomplete,
    ThresholdFailed {
        metric: String,
        observed: f64,
        required: f64,
    },
    /// No successful replay against declared POWL.
    ReplayFailed,
    DeadParameter { param: String },
    // --- Requirements-Andon / CTQ-Forge taxonomy v2.0.0 ---
    /// A `RequirementProposed` op was attempted with no source-voice signal.
    RequirementWithoutSource,
    /// CTQ admission denied because a mandatory field is missing or empty.
    /// `missing` carries one of: "measure", "verification", "negative_case",
    /// "control_plan", "source_voice".
    CtqIncomplete { missing: String },
    /// Work-order admission denied because no naked-craft counterfactual
    /// delta was bound.
    WorkOrderMissingCounterfactual,
    /// LLM (Groq) output was treated as authoritative without passing the
    /// deterministic CTQ admission gate.
    ///
    /// Phase 8 (Plan 4): the variant carries structured `reason` /
    /// `remediation` strings so external auditors can distinguish
    /// transient subprocess failures from configuration mistakes
    /// without parsing free text. The shape change is additive at the
    /// tag level (`tag()` still returns `"llm_authority_claimed"`),
    /// hence no taxonomy hash bump.
    ///
    /// Recognised `reason` values:
    /// - `"subprocess_unavailable"` — `scripts/*.py` could not be spawned.
    /// - `"key_invalid"` — the API key was missing or rejected upstream.
    /// - `"timeout"` — the subprocess exceeded `subprocess_timeout_secs`.
    LlmAuthorityClaimed {
        #[serde(default)]
        reason: String,
        #[serde(default)]
        remediation: String,
    },
    /// Export contains a restricted raw-data field (e.g. customer email,
    /// real account name).
    RawDataLeak { field: String },
    // --- Solution Manufacturing taxonomy v2.1.0 ---
    /// A target generator (iac/rust/erlang/atomvm) emitted no bytes —
    /// the manufacturing pipeline cannot ship an empty artifact.
    GeneratorEmpty { target: String },
    /// Generated IaC (Terraform/Pulumi) failed deterministic validation
    /// (e.g. unbalanced braces, missing required block, illegal IRI).
    IacInvalid { reason: String },
    /// Generated Rust failed deterministic validation (no `pub fn main`,
    /// missing receipt header, unbalanced braces).
    RustInvalid { reason: String },
    /// Generated Erlang failed deterministic validation (missing -module
    /// declaration, missing -export, unmatched parens).
    ErlangInvalid { reason: String },
    /// Generated AtomVM target failed deterministic validation (missing
    /// `start/0`, no AVM-loadable shape).
    AtomVmInvalid { reason: String },
    /// One or more required manufacturing stages (architecture decided,
    /// IaC generated, Rust generated, etc.) is missing — the chain is
    /// broken and cannot ship.
    ManufacturingChainBroken { missing: String },
    /// Solution architecture was not bound to an admitted work order.
    /// Without an upstream WorkOrderAdmitted receipt, no architecture
    /// may be manufactured.
    ArchitectureUnbound,
    // --- Multi-tenant taxonomy v3.1.0 (Phase 11) ---
    /// A request crossed a tenant boundary: a caller in tenant `from`
    /// attempted to read or mutate resources owned by tenant `to`. The
    /// admission gate refuses cross-tenant access regardless of any other
    /// authority the caller may hold within their own tenant.
    TenantBoundary { from: String, to: String },
    // --- Cell8 Phase-10 13-conjunct expansion (Phase 7 / cell_ready.rs) ---
    /// A9 ProvenanceChain failed: the `artifact_hash` was not present in
    /// `provenance_evidence`, so the `prov:wasGeneratedBy` lineage cannot
    /// be closed.
    ProvenanceMissing { artifact_hash: String },
    /// A10 ExternalAttestation failed: no external attestation digest
    /// matches the artifact bit-for-bit. (Phase-10 stub: digest-equality
    /// stand-in for Ed25519. See `src/cell_ready.rs` A10 conjunct.)
    AttestationMissing,
    /// A11 TemporalValidity failed: the `granted_at` chain is empty or
    /// not monotonically non-decreasing. `observed_skew_ms` is the worst
    /// negative delta between adjacent timestamps in milliseconds (or 0
    /// when the chain is empty).
    TemporalSkew { observed_skew_ms: i64 },
    /// A12 DependencyClosure failed: the `prior_receipt` is referenced
    /// but does not appear in the admitted-receipts set. `missing_hash`
    /// is the hex of the absent prior receipt.
    DependencyClosureBroken { missing_hash: String },
    /// A13 ReplayProof failed: deterministic POWL replay produced an OCEL
    /// canonical hash that diverges from the recorded `ocel_trace_hash`.
    ReplayDivergence { expected: String, observed: String },
    /// A10 ExternalAttestation failed under the real-Ed25519 path: a
    /// signature was supplied but `verify_strict` rejected it. `reason`
    /// distinguishes "signature_invalid" (key found, signature did not
    /// verify), "unknown_signing_key" (`signing_key_fpr` not in the
    /// trust set), and "no_trust_set" (admission gate had no trust set
    /// loaded). The legacy `AttestationMissing` is reserved for the
    /// signature-absent path.
    AttestationInvalid { reason: String },
    /// R4 WE — §14: a bootstrap-only handler (e.g. `onto_exemplar_seed`)
    /// was invoked after the bootstrap window closed (i.e. at least one
    /// non-`seed-v0` receipt has been admitted, and the
    /// `OPEN_ONTOLOGIES_BOOTSTRAP_MODE=1` env override is not set).
    BootstrapClosed,
    /// R8-1: post-bootstrap-lock admission with a `granted_at_chain` shorter
    /// than 2 entries. During the bootstrap window a single-entry chain is
    /// acceptable (no prior history exists); once `bootstrap_lock` is set the
    /// admission pipeline must see at least the seed grant followed by the
    /// current in-flight timestamp. A chain of length 1 post-lock means the
    /// history lookup produced no prior rows — suspicious.
    BootstrapChainTooShort,
}

impl DefectClass {
    /// Stable short tag suitable for OCEL `defect` attribute strings.
    /// Short stable string identifier for this variant.
    ///
    /// Stored in OCEL events and receipts; used by auditors to identify
    /// defect classes without parsing the full JSON shape.
    ///
    /// # Examples
    /// ```
    /// # use open_ontologies::defects::DefectClass;
    /// assert_eq!(DefectClass::CapabilityZero.tag(), "capability_zero");
    /// assert_eq!(
    ///     DefectClass::SkippedTask { stage: "enforce_run".into() }.tag(),
    ///     "skipped_task"
    /// );
    /// assert_eq!(DefectClass::BootstrapChainTooShort.tag(), "bootstrap_chain_too_short");
    /// ```
    pub fn tag(&self) -> &'static str {
        match self {
            DefectClass::CapabilityZero => "capability_zero",
            DefectClass::SkippedTask { .. } => "skipped_task",
            DefectClass::ExtraTask { .. } => "extra_task",
            DefectClass::WrongOrder { .. } => "wrong_order",
            DefectClass::BypassRevoked { .. } => "bypass_revoked",
            DefectClass::ReceiptMissing => "receipt_missing",
            DefectClass::ScopeUnclosed => "scope_unclosed",
            DefectClass::OcelIncomplete => "ocel_incomplete",
            DefectClass::ThresholdFailed { .. } => "threshold_failed",
            DefectClass::ReplayFailed => "replay_failed",
            DefectClass::DeadParameter { .. } => "dead_parameter",
            DefectClass::RequirementWithoutSource => "requirement_without_source",
            DefectClass::CtqIncomplete { .. } => "ctq_incomplete",
            DefectClass::WorkOrderMissingCounterfactual => "work_order_missing_counterfactual",
            DefectClass::LlmAuthorityClaimed { .. } => "llm_authority_claimed",
            DefectClass::RawDataLeak { .. } => "raw_data_leak",
            DefectClass::GeneratorEmpty { .. } => "generator_empty",
            DefectClass::IacInvalid { .. } => "iac_invalid",
            DefectClass::RustInvalid { .. } => "rust_invalid",
            DefectClass::ErlangInvalid { .. } => "erlang_invalid",
            DefectClass::AtomVmInvalid { .. } => "atomvm_invalid",
            DefectClass::ManufacturingChainBroken { .. } => "manufacturing_chain_broken",
            DefectClass::ArchitectureUnbound => "architecture_unbound",
            DefectClass::TenantBoundary { .. } => "tenant_boundary",
            DefectClass::ProvenanceMissing { .. } => "provenance_missing",
            DefectClass::AttestationMissing => "attestation_missing",
            DefectClass::TemporalSkew { .. } => "temporal_skew",
            DefectClass::DependencyClosureBroken { .. } => "dependency_closure_broken",
            DefectClass::ReplayDivergence { .. } => "replay_divergence",
            DefectClass::AttestationInvalid { .. } => "attestation_invalid",
            DefectClass::BootstrapClosed => "bootstrap_closed",
            DefectClass::BootstrapChainTooShort => "bootstrap_chain_too_short",
        }
    }

    /// Tag list in declaration order. The hash of this list (NUL-separated)
    /// is pinned in [`DEFECTS_TAXONOMY_DISCRIMINANT_HASH`]. Any variant
    /// add/rename/remove changes the hash and forces a taxonomy version bump.
    pub const fn all_tags() -> &'static [&'static str] {
        &[
            "capability_zero",
            "skipped_task",
            "extra_task",
            "wrong_order",
            "bypass_revoked",
            "receipt_missing",
            "scope_unclosed",
            "ocel_incomplete",
            "threshold_failed",
            "replay_failed",
            "dead_parameter",
            "requirement_without_source",
            "ctq_incomplete",
            "work_order_missing_counterfactual",
            "llm_authority_claimed",
            "raw_data_leak",
            "generator_empty",
            "iac_invalid",
            "rust_invalid",
            "erlang_invalid",
            "atomvm_invalid",
            "manufacturing_chain_broken",
            "architecture_unbound",
            "tenant_boundary",
            "provenance_missing",
            "attestation_missing",
            "temporal_skew",
            "dependency_closure_broken",
            "replay_divergence",
            "attestation_invalid",
            "bootstrap_closed",
            "bootstrap_chain_too_short",
        ]
    }
}

/// Compute the BLAKE3 hex hash of the concatenated tag list.
///
/// The result must equal [`DEFECTS_TAXONOMY_DISCRIMINANT_HASH`]. The CI gate
/// [`tests::taxonomy_discriminant_hash_pinned`] enforces this; any variant
/// add/rename/remove will break it and force a taxonomy version bump.
///
/// # Examples
/// ```
/// # use open_ontologies::defects::{discriminant_hash, DEFECTS_TAXONOMY_DISCRIMINANT_HASH};
/// let h = discriminant_hash();
/// assert_eq!(h.len(), 64);                        // BLAKE3 hex is always 64 chars
/// assert_eq!(h, DEFECTS_TAXONOMY_DISCRIMINANT_HASH); // must match the pinned constant
/// ```
pub fn discriminant_hash() -> String {
    let mut h = blake3::Hasher::new();
    for tag in DefectClass::all_tags() {
        h.update(tag.as_bytes());
        h.update(b"\0");
    }
    h.finalize().to_hex().to_string()
}

/// Severity of an actionable remediation hint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemediationSeverity {
    Blocking,
    Warning,
    Info,
}

/// Structured remediation block attached to every defect JSON response.
///
/// Enables AI agents (LangChain, CrewAI) to self-correct without human
/// intervention: the agent reads `next_tool` + `next_params`, calls that
/// tool, and retries. `auto_retry = true` means the agent may do so
/// without user confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationBlock {
    pub explanation: String,
    pub next_tool: Option<String>,
    pub next_params: Option<serde_json::Value>,
    pub severity: RemediationSeverity,
    pub auto_retry: bool,
}

impl RemediationBlock {
    fn blocking(
        explanation: impl Into<String>,
        next_tool: Option<&str>,
        next_params: Option<serde_json::Value>,
        auto_retry: bool,
    ) -> Self {
        Self {
            explanation: explanation.into(),
            next_tool: next_tool.map(|s| s.to_string()),
            next_params,
            severity: RemediationSeverity::Blocking,
            auto_retry,
        }
    }
}

impl DefectClass {
    /// Returns a structured remediation hint for AI agents. Every variant
    /// maps to the exact next tool to call (and suggested params) so that
    /// agents can self-correct without human intervention.
    ///
    /// This method is pure — no I/O, no DB calls, deterministic.
    ///
    /// # Examples
    /// ```
    /// # use open_ontologies::defects::{DefectClass, RemediationSeverity};
    /// let rem = DefectClass::ScopeUnclosed.remediation();
    /// assert_eq!(rem.severity,  RemediationSeverity::Blocking);
    /// assert!(rem.auto_retry);
    /// assert_eq!(rem.next_tool.as_deref(), Some("onto_declare_workflow"));
    /// ```
    pub fn remediation(&self) -> RemediationBlock {
        use serde_json::json;
        match self {
            DefectClass::ScopeUnclosed => RemediationBlock::blocking(
                "No open workflow scope. Call onto_declare_workflow to open one, \
                 then retry this operation with the returned scope_token.",
                Some("onto_declare_workflow"),
                Some(json!({"name": "OntologyAuthoring", "description": "auto-opened by remediation"})),
                true,
            ),
            DefectClass::CapabilityZero => RemediationBlock::blocking(
                "Required stage missing in OCEL event log. Declare and open a \
                 workflow scope before the first tool call.",
                Some("onto_declare_workflow"),
                Some(json!({"name": "OntologyAuthoring"})),
                true,
            ),
            DefectClass::SkippedTask { stage } => {
                let next = stage_to_tool(stage);
                RemediationBlock::blocking(
                    format!(
                        "Stage '{}' was required by the workflow but missing from the OCEL trace. \
                         Complete that stage first.",
                        stage
                    ),
                    next,
                    None,
                    false,
                )
            }
            DefectClass::ExtraTask { stage } => RemediationBlock::blocking(
                format!(
                    "Stage '{}' appeared in the OCEL trace but is not declared in the workflow. \
                     Close the current scope and declare a workflow that includes this stage.",
                    stage
                ),
                Some("onto_close_workflow"),
                None,
                false,
            ),
            DefectClass::WrongOrder { expected, got } => {
                let next = stage_to_tool(expected);
                RemediationBlock::blocking(
                    format!(
                        "Stages out of order: expected '{}' before '{}'. \
                         Complete the prerequisite stage first.",
                        expected, got
                    ),
                    next,
                    None,
                    false,
                )
            }
            DefectClass::BypassRevoked { reason } => RemediationBlock::blocking(
                format!(
                    "Session revoked by bypass_admission (reason: '{}'). \
                     The session cannot be reused. Start a new session.",
                    reason
                ),
                None,
                None,
                false,
            ),
            DefectClass::ReceiptMissing => RemediationBlock::blocking(
                "No admission receipt found. Open a workflow scope via \
                 onto_declare_workflow and complete the required stages before \
                 this operation.",
                Some("onto_declare_workflow"),
                Some(json!({})),
                false,
            ),
            DefectClass::OcelIncomplete => RemediationBlock::blocking(
                "OCEL event log is incomplete for the current scope. Ensure all \
                 required stages have been executed and their events are present.",
                Some("onto_declare_workflow"),
                Some(json!({})),
                false,
            ),
            DefectClass::ThresholdFailed { metric, observed, required } => {
                RemediationBlock::blocking(
                    format!(
                        "Threshold check failed: {} is {:.3} but must be ≥ {:.3}. \
                         Run onto_threshold_sweep to see which parameters meet the threshold.",
                        metric, observed, required
                    ),
                    Some("onto_threshold_sweep"),
                    None,
                    false,
                )
            }
            DefectClass::ReplayFailed => RemediationBlock::blocking(
                "POWL replay against the declared workflow failed. Check that all \
                 required stages were executed in order, then retry.",
                Some("onto_conformance_check"),
                None,
                false,
            ),
            DefectClass::DeadParameter { param } => RemediationBlock::blocking(
                format!(
                    "Parameter '{}' was declared but never consumed. Remove it from \
                     the function signature or use it.",
                    param
                ),
                None,
                None,
                false,
            ),
            DefectClass::RequirementWithoutSource => RemediationBlock::blocking(
                "A requirement was proposed with no source_voice signal. \
                 Provide a non-empty source_voice when calling onto_propose_requirement.",
                Some("onto_propose_requirement"),
                Some(json!({"source_voice": "<stakeholder voice — required>"})),
                false,
            ),
            DefectClass::CtqIncomplete { missing } => RemediationBlock::blocking(
                format!(
                    "CTQ admission denied: mandatory field '{}' is missing or empty. \
                     Provide all required CTQ fields before calling onto_admit_ctq.",
                    missing
                ),
                Some("onto_admit_ctq"),
                None,
                false,
            ),
            DefectClass::WorkOrderMissingCounterfactual => RemediationBlock::blocking(
                "Work-order admission denied: no naked-craft counterfactual delta was bound. \
                 Bind a counterfactual via onto_counterfactual before calling \
                 onto_admit_work_order.",
                Some("onto_propose_work_order"),
                None,
                false,
            ),
            DefectClass::LlmAuthorityClaimed { reason, .. } => RemediationBlock::blocking(
                format!(
                    "LLM output was treated as authoritative without passing the CTQ gate \
                     (reason: '{}'). LLM output must be validated by onto_admit_ctq before \
                     any downstream tool can act on it.",
                    reason
                ),
                Some("onto_translate_candidate"),
                None,
                false,
            ),
            DefectClass::RawDataLeak { field } => RemediationBlock::blocking(
                format!(
                    "Export contains restricted raw-data field '{}'. \
                     Remove or redact the field before retrying.",
                    field
                ),
                None,
                None,
                false,
            ),
            DefectClass::GeneratorEmpty { target } => RemediationBlock::blocking(
                format!(
                    "Generator '{}' emitted no bytes. Check the work order inputs and \
                     retry onto_manufacture_solution.",
                    target
                ),
                Some("onto_manufacture_solution"),
                None,
                false,
            ),
            DefectClass::IacInvalid { reason } => RemediationBlock::blocking(
                format!(
                    "Generated IaC failed validation: {}. \
                     Fix the work order inputs and retry onto_manufacture_solution.",
                    reason
                ),
                Some("onto_manufacture_solution"),
                None,
                false,
            ),
            DefectClass::RustInvalid { reason } => RemediationBlock::blocking(
                format!(
                    "Generated Rust failed validation: {}. \
                     Fix the work order inputs and retry onto_manufacture_solution.",
                    reason
                ),
                Some("onto_manufacture_solution"),
                None,
                false,
            ),
            DefectClass::ErlangInvalid { reason } => RemediationBlock::blocking(
                format!(
                    "Generated Erlang failed validation: {}. \
                     Fix the work order inputs and retry onto_manufacture_solution.",
                    reason
                ),
                Some("onto_manufacture_solution"),
                None,
                false,
            ),
            DefectClass::AtomVmInvalid { reason } => RemediationBlock::blocking(
                format!(
                    "Generated AtomVM target failed validation: {}. \
                     Fix the work order inputs and retry onto_manufacture_solution.",
                    reason
                ),
                Some("onto_manufacture_solution"),
                None,
                false,
            ),
            DefectClass::ManufacturingChainBroken { missing } => RemediationBlock::blocking(
                format!(
                    "Manufacturing stage '{}' is missing — the chain cannot ship. \
                     Complete the missing stage via onto_admit_work_order.",
                    missing
                ),
                Some("onto_admit_work_order"),
                None,
                false,
            ),
            DefectClass::ArchitectureUnbound => RemediationBlock::blocking(
                "Solution architecture was not bound to an admitted work order. \
                 Admit a work order via onto_admit_work_order before manufacturing.",
                Some("onto_admit_work_order"),
                None,
                false,
            ),
            DefectClass::TenantBoundary { from, to } => RemediationBlock::blocking(
                format!(
                    "Cross-tenant access denied: caller in tenant '{}' cannot access \
                     resources in tenant '{}'. Use resources within your own tenant.",
                    from, to
                ),
                None,
                None,
                false,
            ),
            DefectClass::ProvenanceMissing { artifact_hash } => RemediationBlock::blocking(
                format!(
                    "Provenance chain broken: artifact hash '{}' not found in \
                     provenance evidence. View lineage via onto_lineage.",
                    artifact_hash
                ),
                Some("onto_lineage"),
                None,
                false,
            ),
            DefectClass::AttestationMissing => RemediationBlock::blocking(
                "No external attestation found for this artifact. \
                 Provide an Ed25519 receipt via onto_ontostar_attest.",
                Some("onto_ontostar_attest"),
                None,
                false,
            ),
            DefectClass::TemporalSkew { observed_skew_ms } => RemediationBlock::blocking(
                format!(
                    "Temporal validity failed: granted_at chain has a negative skew \
                     of {}ms. Check system clock synchronization and retry.",
                    observed_skew_ms
                ),
                None,
                None,
                false,
            ),
            DefectClass::DependencyClosureBroken { missing_hash } => RemediationBlock::blocking(
                format!(
                    "Dependency closure broken: prior receipt '{}' is referenced but \
                     absent from the admitted-receipts set. View lineage via onto_lineage.",
                    missing_hash
                ),
                Some("onto_lineage"),
                None,
                false,
            ),
            DefectClass::ReplayDivergence { expected, observed } => RemediationBlock::blocking(
                format!(
                    "POWL replay divergence: expected OCEL hash '{}', observed '{}'. \
                     Run onto_conformance_check to diagnose.",
                    expected, observed
                ),
                Some("onto_conformance_check"),
                None,
                false,
            ),
            DefectClass::AttestationInvalid { reason } => RemediationBlock::blocking(
                format!(
                    "Ed25519 attestation invalid ({}). \
                     Rotate signing keys via onto_attestation_rotate_keys and re-attest.",
                    reason
                ),
                Some("onto_attestation_rotate_keys"),
                None,
                false,
            ),
            DefectClass::BootstrapClosed => RemediationBlock::blocking(
                "Bootstrap window is closed — bootstrap-only tools (e.g. onto_exemplar_seed) \
                 can no longer be called. Use production admission via onto_declare_workflow.",
                None,
                None,
                false,
            ),
            DefectClass::BootstrapChainTooShort => RemediationBlock::blocking(
                "Post-bootstrap chain is too short (< 2 entries). \
                 Ensure the seed grant was admitted before the current operation.",
                Some("onto_admission_check"),
                Some(json!({"op": "apply"})),
                true,
            ),
        }
    }
}

/// Map a workflow stage name to the tool that produces it.
fn stage_to_tool(stage: &str) -> Option<&'static str> {
    match stage {
        "load" | "repo_load" => Some("onto_load"),
        "validate" => Some("onto_validate"),
        "reason" => Some("onto_reason"),
        "enforce_run" => Some("onto_enforce"),
        "save" => Some("onto_save"),
        "version" => Some("onto_version"),
        "map" => Some("onto_map"),
        "ingest" => Some("onto_ingest"),
        "shacl" => Some("onto_shacl"),
        "embed" => Some("onto_embed"),
        "align_run" => Some("onto_align"),
        "codegen_run" => Some("onto_codegen"),
        "apply" => Some("onto_apply"),
        "plan" => Some("onto_plan"),
        "drift" => Some("onto_drift"),
        _ => None,
    }
}

/// Evidence carried alongside a [`DefectClass`] explaining the deviation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deviation {
    pub kind: String,
    pub stage: String,
    pub detail: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defect_class_round_trips_through_serde() {
        let d = DefectClass::WrongOrder {
            expected: "load".into(),
            got: "save".into(),
        };
        let s = serde_json::to_string(&d).expect("serialize");
        let back: DefectClass = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(d, back);
    }

    #[test]
    fn taxonomy_discriminant_hash_pinned() {
        let got = discriminant_hash();
        assert_eq!(
            got, DEFECTS_TAXONOMY_DISCRIMINANT_HASH,
            "DefectClass tag set changed.\n\
             Expected: {}\n\
             Got:      {}\n\
             If this is intentional, bump DEFECTS_TAXONOMY_VERSION and update \
             DEFECTS_TAXONOMY_DISCRIMINANT_HASH to the 'Got' value above.",
            DEFECTS_TAXONOMY_DISCRIMINANT_HASH, got
        );
    }

    #[test]
    fn deviation_round_trips_through_serde() {
        let dev = Deviation {
            kind: "skipped_task".into(),
            stage: "enforce_run".into(),
            detail: "stage missing in OCEL trace".into(),
            expected: Some("enforce_run".into()),
            actual: None,
        };
        let s = serde_json::to_string(&dev).expect("serialize");
        let back: Deviation = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(dev, back);
    }
}
