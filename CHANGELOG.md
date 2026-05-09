# Changelog

All notable changes to OntoStar / open-ontologies are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions are organized per OntoStar phase on the `ontostar-integration` branch.
Pre-OntoStar history (the original `open-ontologies` MCP server, releases
0.1.x) is summarized at the bottom.

## [Unreleased]

### Round 5 WC-2 — §28 admin tools + HTTP header allowlists

Five new admin-only MCP tools, two new HTTP header allowlists, and a
shared kill-switch for the retention worker. All admin tools gate on
the `is_admin_principal()` cache from R5 WC-1 (TOCTOU-immune).

#### [Added]

- **`onto_bootstrap_unlock`** (`src/server.rs`). DELETEs the
  `bootstrap_lock` row. Last-resort recovery for the one-shot
  bootstrap window. Audit-only via
  `AdmissionOp::BootstrapUnlock` (NEW variant). Lineage class `K`.
- **`onto_receipts_revoke_batch`**. Soft-deletes (UPDATE
  `production_law_version = 'revoked-by-admin'`) every receipt whose
  `scope_token` matches a SQLite GLOB pattern AND whose
  `production_law_version` is not `seed-v0`. Preserves the chain for
  audit. New `AdmissionOp::ReceiptsBatchRevoke` variant +
  `receipts_batch_revoke` audit name.
- **`onto_session_revoke_by_principal`**. Bulk-INSERT into
  `revoked_sessions` for every active scope owned by a principal in
  a tenant. Falls back to `revoked_sessions` until R3 Task B's
  `revoked_principals` table lands (TODO marker in code). New
  `AdmissionOp::SessionRevoke` variant + `session_revoke` audit name.
- **`onto_retention_pause { minutes }`** and
  **`onto_retention_resume`**. Emergency kill-switch for the
  `RetentionWorker` (R4 WD). Both reuse `AdmissionOp::Feedback`
  (operational tweaks, not new audit semantics). Bounded to 1 week
  (10080 minutes); idempotent resume.
- **`AuthorityConfig::known_tenants: Vec<String>`** (`src/config.rs`)
  and **`resolve_known_tenants(cfg)`**. Tenant allowlist for the
  `X-Ontostar-Tenant` HTTP header, resolved ONCE at startup. Empty
  list preserves backwards-compat (any well-formed tenant accepted);
  non-empty enforces strict 403.
- **`tenant_extract_layer_with_allowlist`** (`src/cmds/server.rs`).
  Strict-allowlist variant of the existing `tenant_extract_layer`.
  Unknown tenants → HTTP 403 with
  `FalsePass { reason: "tenant_not_in_allowlist" }` instead of silent
  fallback to `"default"`.
- **`principal_extract_layer`** (`src/cmds/server.rs`). Admin-gated
  acceptance of the `X-Ontostar-Principal` HTTP header. Wired AFTER
  the bearer-token layer so the caller is authenticated first.
  Non-admin caller presenting the header → HTTP 403 with
  `FalsePass { reason: "principal_override_requires_admin" }`.
  Closed-by-default: empty admin allowlist rejects ANY use of the
  header (not just non-admins).
- **`RetentionWorker::paused_until: Arc<AtomicI64>`** field +
  **`spawn_with_pause`** + **`is_paused`** + **`new_with_pause`**.
  `tick()` returns an empty `RetentionReport` without touching the
  database when paused. Shared `Arc<AtomicI64>` so the server's
  pause/resume tools and the worker's `tick()` agree atomically.
  Backwards-compat shim: `spawn(db, cfg)` still works (returns just
  the JoinHandle, internally allocates a fresh atomic).
- **`OpenOntologiesServer::retention_paused_until: Arc<AtomicI64>`**
  field + **`with_retention_pause(...)`** builder. Bootstrap in
  `src/cmds/server.rs::run_stdio_server` and `build_http_axum_router`
  allocates one atomic and hands it to both the server and the
  worker.
- **`OntoReceiptsRevokeBatchInput`**, **`OntoSessionRevokeByPrincipalInput`**,
  **`OntoRetentionPauseInput`** (`src/inputs.rs`). MCP tool input
  structs with documented field semantics.
- **`tests/admin_tools_e2e.rs`** (NEW) — end-to-end coverage for the
  5 new admin tools: admin-vs-non-admin paths, durable state changes
  (DB rows, atomic values, OCEL audit events), bound checks, fallback
  notes for `onto_session_revoke_by_principal`.
- **`tests/http_header_allowlists.rs`** (NEW) — counterfactuals for
  both new middleware layers + `resolve_known_tenants` env-var
  precedence + dedup/trim. Replicates the middleware logic locally
  (the production closures live in `cmds/server.rs` which is binary-
  only) so the contract is pinned by behaviour, not import.
- 5 new entries in
  `tests/no_bypass_audit.rs::audit_only_handlers_present` — each new
  admin tool must call `evaluate_admission_audit(...)`.
- Test count 541 → 558 (5 admin-tool E2E + 12 HTTP-allowlist
  counterfactuals).

#### [Changed]

- **`AdmissionOp` enum** — 3 new variants
  (`BootstrapUnlock`, `ReceiptsBatchRevoke`, `SessionRevoke`)
  with distinct audit name strings; updated `as_str()` and
  `is_full_admission()` (all three are audit-only because admin
  tools cannot deny themselves — full admission would deadlock
  `onto_bootstrap_unlock` when the lock row is the problem the
  operator is repairing). `Feedback` reused for the paired
  `retention_pause` / `retention_resume` tools.
- **`src/cmds/server.rs::run_stdio_server`** — uses
  `RetentionWorker::spawn_with_pause` and pipes the same
  `Arc<AtomicI64>` to `OpenOntologiesServer::with_retention_pause`.
  Stdio MCP gains the kill-switch transparently.
- **`src/cmds/server.rs::build_http_axum_router`** — adds the
  principal_extract_layer (after bearer-token layer) and replaces
  `tenant_extract_layer` with `tenant_extract_layer_with_allowlist`.
  The original `tenant_extract_layer` is retained but `#[allow(dead_code)]`
  for tests that wish to exercise the open-mode path.
- **`AuthorityConfig`** gains `known_tenants` field
  (`#[serde(default)]`). Existing config files load unchanged
  (empty list = open mode = backwards-compat).

#### [Notes]

- The `onto_session_revoke_by_principal` SQL fallback uses
  `revoked_sessions` until R3 Task B's `revoked_principals` table is
  in tree. Documented in the tool docstring + CHANGELOG; switch
  point is a single SQL change once R3 lands.
- HTTP header tests run middleware closures via
  `tower::ServiceExt::oneshot`, NOT a full
  `StreamableHttpService` stack — full HTTP-layer integration is
  too invasive for this scope. Contract drift between production
  and the in-test replication is the cost; the trade-off is
  documented in `tests/http_header_allowlists.rs` file-level
  comment.



Three §28 leaks closed: success-shaped bypass denial, volatile
bootstrap-window override, and admin-allowlist TOCTOU race.

#### [Breaking]

- **`bypass_admission` denial JSON shape changed.** The previous
  shape `Err({"ok": true, "admission": "bypassed", "reason": <...>})`
  claimed success while the internal state was a denial — auditors
  keying on `ok` were misled. The new unified denial shape:
  ```json
  {
    "ok": false,
    "admission": "bypassed_session_revoked",
    "defect": { "kind": "BypassRevoked", "reason": "<reason>" },
    "principal_revoked_at": "<RFC3339 timestamp>"
  }
  ```
  Tooling that read `result.reason` at the top level must migrate to
  `result.defect.reason`. The OCEL `admission_audit{op=bypass}` and
  `admission_bypass` events still emit (audit precedence unchanged);
  only the JSON response shape is breaking. Pinned by
  `tests/bypass_response_shape_unified.rs`.
- **Admin operations require explicit configuration.** The new
  `[authority] admin_principals = [...]` config section (or
  `OPEN_ONTOLOGIES_ADMIN_PRINCIPALS` env var, env wins) is now
  resolved ONCE at startup instead of being re-read per call. Servers
  built without `with_admin_principals(...)` get an empty cache —
  closed-by-default. Operators that previously relied on setting the
  env var post-startup will see admin tools refuse: set the env BEFORE
  starting the server, or add `[authority]` to `config.toml`.

#### [Added]

- `bootstrap_lock` table (`src/state.rs`) — single-row
  (`CHECK (id = 1)`) DB-level enforcement of the bootstrap window.
  Auto-inserted via `INSERT OR IGNORE` on the first non-`seed-v0`
  receipt persisted by `receipts::persist_with_tenant_in_tx`. Excluded
  from `RetentionWorker` pruning by design — pruning would re-open
  the window and reintroduce the §28 leak. Pinned by
  `tests/bootstrap_lock_persists.rs`.
- `BootstrapState::is_bootstrap` rewritten with lock-row precedence:
  (1) lock row → CLOSED, (2) `OPEN_ONTOLOGIES_BOOTSTRAP_MODE=1` →
  advisory open (only when lock absent), (3) zero non-seed receipts →
  bootstrap, (4) else CLOSED. The env var is now demoted to advisory.
- `DefectClass::BypassRevoked { reason: String }` — promoted from a
  unit variant to a struct variant carrying the operator's reason.
  Tag (`"bypass_revoked"`) unchanged; `all_tags()` ordering unchanged;
  `DEFECTS_TAXONOMY_DISCRIMINANT_HASH` therefore stable. The variant
  shape change is forward-compatible (`#[serde(default)]` on the new
  field). `DEFECTS_TAXONOMY_VERSION` bumped 4.3.0 → 4.4.0 (MINOR).
- `AuthorityConfig` (`src/config.rs`) — new `[authority]` section
  exposing `admin_principals: Vec<String>`. `#[serde(default)]` on the
  field for silent upgrade of existing config files.
- `resolve_admin_principals(cfg)` (`src/config.rs`) — startup-time
  resolver: env var > config file > empty. Reads
  `OPEN_ONTOLOGIES_ADMIN_PRINCIPALS` ONCE; the only place in
  production code that reads that env var.
- `OpenOntologiesServer::admin_principals: Arc<Vec<String>>` and
  `OpenOntologiesServer::with_admin_principals(self, Vec<String>)
  -> Self` — startup-cached allowlist + builder method. Mirrors
  `with_default_llm_engine` wiring in `src/cmds/server.rs`.
  `is_admin_principal` reads from the cache, not from
  `std::env::var(...)`. Pinned by
  `tests/admin_principals_cache_immune.rs`.
- 10 new counterfactual `#[test]` / `#[tokio::test]` functions across
  three new test files. Each proves Δ > 0 versus the pre-fix
  implementation (the test would FAIL against the previous code).
  Test count 531 → 541.

#### [Changed]

- `src/admission.rs::evaluate_in_tenant` — when `session_is_revoked`
  is true, the `BypassRevoked` denial now carries the original reason
  read from `revoked_sessions.reason` (read-only `SELECT`; no schema
  change). Auditors no longer need to JOIN to recover the reason.
- `src/cell_ready.rs` — `BypassRevoked` construction at the
  `session_revoked` conjunct uses the new struct variant with empty
  reason (the upstream call site populates the reason; cell_ready
  inputs do not currently surface it).

### Round 5 WB-2 — §15 OCEL anchor closure

3 emit swallows replaced with primary+fallback+log pattern. New OCEL
event types `admission_denied_ocel_failed`,
`workflow_declared_emit_failed`, `conformance_recorded`. Conformance
runs writes are now OCEL-witnessed atomically.

- `src/admission.rs::emit_with_fallback` (NEW, private): encapsulates
  primary emit → on-failure fallback emit → on-double-failure
  `tracing::error!` for sites 1 and 2. The fallback `event_type` is
  supplied explicitly (not derived) so historical names like
  `admission_denied_ocel_failed` are preserved. Fallback events carry
  the original attrs plus `intended_event_type` and `primary_emit_error`
  so an OCEL projector can reconstruct the lost primary witness.
- `src/admission.rs::emit_denied_for_scope` (Site 1): replaces
  `let _ = store.emit_event(..., "admission_denied", ...)` with an
  `emit_with_fallback` call. Phantom denials no longer possible — the
  caller's `Err(...)` is now mirrored by an OCEL anchor (primary
  `admission_denied` or fallback `admission_denied_ocel_failed`).
- `src/admission.rs::evaluate` workflow_declared anchor (Site 2): same
  fallback pattern. Plan B identifies this as a load-bearing
  replay-portability anchor — downstream `replay_from_ocel_alone`
  cannot reconstruct the declared model without it. New event type
  `workflow_declared_emit_failed` carries the same `powl_hash` and
  `powl_string` attrs so the model is recoverable from a degraded
  trail.
- `src/admission.rs::persist_conformance_run` (Site 3): refactored to
  wrap the `INSERT OR REPLACE INTO conformance_runs` and a NEW
  `conformance_recorded` OCEL emit in a single
  `rusqlite::Transaction`. The two commit together or roll back
  together — closing the orphan-evidence window where a verifier
  joining `receipts` ↔ `ocel_events` ↔ `conformance_runs` could not
  prove the conformance row was used at admission. Best-effort
  post-conditions (workflow_class stamping, Loop 5 regression hook)
  run AFTER commit, unchanged. Atomic-block failure logged under
  `tracing::error!` target `ontostar.admission.conformance_witness_lost`.
- `src/ocel_store.rs::EMIT_FAILURE_INJECTION_HOOK` (NEW,
  `#[cfg(debug_assertions)]`-gated thread_local): test-only injection
  point fired at the entry of `emit_event_rows`. When a closure is
  installed and returns `Some(error)` for a target `event_type`, the
  emit fails as if SQLite refused. Lets counterfactual tests prove
  the fallback pattern actually records a degraded anchor, and lets
  the conformance-rollback test prove the transaction is a real
  atomic boundary. Mirrors WB-1's `A13_BETWEEN_SNAPSHOT_HOOK`
  pattern. `#[doc(hidden)]`.
- `tests/ocel_emit_swallow_closure.rs` (NEW): five counterfactual
  proofs.
  `denial_witness_survives_primary_emit_failure` sabotages the
  `admission_denied` emit and asserts the fallback lands.
  `denial_witness_normal_path_no_fallback` is the baseline (no
  fallback when the primary succeeds).
  `workflow_declared_witness_survives_primary_emit_failure`
  sabotages the workflow_declared anchor and asserts the fallback
  carries the same powl_hash/powl_string attrs.
  `conformance_runs_writes_atomic_with_ocel_witness` is the
  positive-path joiner-readiness proof.
  `conformance_runs_rollback_on_ocel_witness_failure` is the
  load-bearing atomicity proof — sabotage the conformance OCEL emit
  and assert the conformance_runs row is NOT visible.
- `README.md`: test count 526 → 531.

External verifiers SHOULD treat `admission_denied_ocel_failed` and
`workflow_declared_emit_failed` as equivalent-to their primary event
type plus a `degraded_trail = true` flag (carried in the
`intended_event_type` attribute). The conformance row now carries an
OCEL witness pinned by `(scope_token, run_id)` — joining
`conformance_runs.run_id` to `ocel_events` via
`event_type='conformance_recorded'` is the canonical lookup.

### Round 5 WB-1 — §15 A13 ReplayProof tautology closure

A13 ReplayProof gate now uses an independent OCEL re-snapshot; previously
both inputs were derived from the same line-519 hash. The gate can now
actually fail under concurrent OCEL mutation. A9/A11/A12 marked as
TODO(R6) — same disease, deferred fix template.

- `src/admission.rs::re_snapshot_ocel_for_replay_proof` (NEW, private):
  re-runs `canonical_ocel_projection` and re-hashes via `blake3::hash` +
  `hex32_pub`. Wired into `OntoStarAdmissionGate::evaluate` immediately
  before the `CellReadyInputs` struct literal so
  `replay_canonical_hash` is now byte-independent of the line-519
  `ocel_trace_hash_hex`. Closes the structural twin to the §15 A10
  tautology that R2 fixed.
- `src/admission.rs::A13_BETWEEN_SNAPSHOT_HOOK` (NEW,
  `#[cfg(debug_assertions)]`-gated thread_local): test-only injection
  point fired between the first and second OCEL snapshots. Release
  builds (`cargo build --release` strips `debug_assertions`) cannot
  reach the hook — production has zero overhead and zero test
  surface. `#[doc(hidden)]` keeps the symbol out of public docs.
- `tests/cell_ready_a13_deny_path.rs` (NEW): two-test integration
  binary. `a13_replay_divergence_under_concurrent_mutation` drives a
  full `OntoStarAdmissionGate::evaluate` on a `DataExtensionFastPath`
  scope, installs the hook to emit a NEW OCEL event_type
  (`a13_test_concurrent_mutation`) between snapshots, and asserts
  `Err(DefectClass::ReplayDivergence { expected, observed })` with two
  64-char DISTINCT BLAKE3 hashes. `a13_re_snapshot_quiescent_store_still_grants`
  asserts that with no hook installed, the same flow grants — proving
  the hook is the only thing producing divergence and the
  re-snapshot is not a permanent denial.
- `tests/saboteur_a13_replay_proof_load_bearing.rs` (NEW, `#[ignore]`):
  documentation/saboteur test with extensive header comments
  describing the saboteur matrix (with-fix vs without-fix) and step-by-step
  manual verification: comment out the new local in
  `re_snapshot_ocel_for_replay_proof`'s call site, restore the
  pre-R5-WB-1 alias, re-run → test MUST FAIL, proving the fix is
  load-bearing.
- `src/admission.rs`: `provenance_evidence`, `granted_at_chain`,
  `admitted_receipts` construction sites (lines ~563–568) now carry
  `TODO(R6 §15.A9)`, `TODO(R6 §15.A11)`, `TODO(R6 §15.A12)` comments
  — same caller-trust-burden disease A13 had, deferred to R6 with
  explicit fix templates (independent verification against receipt
  chain / receipt-chain reconstruction for monotonicity / DB lookup
  against `receipts WHERE record_hash = prior_receipt`).
- `README.md`: test totals 523 → 526 (2 active + 1 ignored).

## [Unreleased] — Round 4 WD: §29 Cell8 retirement closure

### Added

- `src/retention.rs` (NEW): `RetentionWorker` background task. Mirrors
  `registry::spawn_evictor` semantics. Per-table pruners
  (`prune_ocel`, `prune_lineage`, `prune_conformance`, `prune_revoked`,
  `prune_receipt_files`, `prune_exemplars`, `prune_align_feedback`,
  `prune_tool_feedback`, `prune_embeddings_orphans`, `prune_cache`)
  each accept their respective `*_days` window from `RetentionConfig`.
  Cascade order: `ocel_event_attrs` and `ocel_relationships` are
  pruned BEFORE `ocel_events` (foreign-key parents last).
- `src/receipt_archive.rs` (NEW): receipt cold-storage archival.
  `archive_receipts(db, older_than_days, dir)` writes monthly Parquet
  shards (`receipts-YYYY-MM.parquet`) and an `archive_index.db`
  sidecar SQLite index for O(1) `receipt_hash → shard` lookup.
  `lookup_archived(dir, hash)` resolves cold receipts.
- `src/state.rs`: `key_valid_at` column on `receipts`;
  `trusted_keys_history(fingerprint, pem, added_at, removed_at, status)`
  table; per-tenant retention pruning indexes on `ocel_events`,
  `lineage_events`, `conformance_runs`, `revoked_sessions`, `receipts`,
  `mined_exemplars`, `align_feedback`, `tool_feedback`.
- `src/attestation.rs`: `from_dir_with_history(dir, db)` upserts the
  current trust set into `trusted_keys_history` and stamps `removed_at`
  on retired fingerprints. `lookup_history(db, fpr)` returns the row.
  `into_swap(trust)` builds the hot-swap container.
  `pub use arc_swap::ArcSwap` (for the gate's `Arc<ArcSwap<TrustedKeys>>`).
- `src/admission.rs`: `OntoStarAdmissionGate.trusted_keys` is now
  `Arc<ArcSwap<TrustedKeys>>`. The `evaluate*` paths take a
  `load_full()` snapshot guard before calling `cell_ready`, so
  concurrent rotation cannot tear within one admission.
- `src/cell_ready.rs::CellReadyInputs.trusted_keys_db: Option<&StateDb>`.
  When `Some`, A10 looks up the signing fingerprint's
  `trusted_keys_history` row and rejects with
  `AttestationInvalid { reason: "key_not_trusted_at_signature_time" }`
  when `granted_at < added_at` or `granted_at >= removed_at`. Legacy
  fingerprints (no history row) are admitted with a `tracing::warn!`.
- `src/receipts.rs::persist_with_tenant_in_tx`: looks up
  `trusted_keys_history.added_at` by fingerprint and writes
  `key_valid_at` on every receipt.
- `src/server.rs`: new MCP tool `onto_attestation_rotate_keys`,
  admin-gated via the `OPEN_ONTOLOGIES_ADMIN_PRINCIPALS` env var
  (CSV-of-principals; closed-by-default — empty allowlist denies all).
  New `is_admin_principal(&self) -> bool` helper. Reads the configured
  trust dir, validates via SHACL, calls `from_dir_with_history`, and
  records lineage event `K trusted_keys_rotated count=N`.
  Non-admin callers receive `FalsePass { reason: "not_admin" }`.
- `src/cmds/server.rs`: `RetentionWorker::spawn` invocation alongside
  the existing `registry::spawn_evictor`.
- `src/config.rs`: new `[retention]` section
  (`poll_interval_secs=86400`, `ocel_days=90`, `lineage_days=180`,
  `conformance_days=30`, `revocation_grace_days=30`,
  `receipt_files_days=365`, `exemplar_days=365`, `feedback_days=365`,
  `archive_path=None`, `hot_receipt_days=365`).
- `src/verify.rs`: `Verdict::Admitted.source: String`. Falls through to
  `OPEN_ONTOLOGIES_RECEIPT_ARCHIVE_DIR` on hot-table miss; cold hits
  set `source: "archive"`. Hot hits keep the field empty (skipped in
  serialization via `skip_serializing_if = "String::is_empty"`).
- `ontology/attestation-shapes.ttl` (NEW): SHACL shape over
  `attest:TrustedKey` requiring 16-hex-char fingerprint, non-empty
  SubjectPublicKeyInfo PEM, xsd:dateTime `added_at`, and `status` in
  {`active`, `retired`}.
- `Makefile`: `clean-worktrees`, `clean-worktrees-soft` (warn-only;
  wired into `make adversarial`), `gc-build` targets.
- `Cargo.toml`: `arc-swap = "1"` dependency added (parquet was already
  a non-default dep used elsewhere).
- `tests/retention_worker.rs` (NEW, 8 tests): drives `tick()`
  synchronously with 0-day retention; the cascade-order test seeds
  100 events + child rows and asserts FK-safe deletion.
- `tests/key_rotation.rs` (NEW, 4 tests):
  `rotate_replaces_in_memory_set`, `signed_then_rotated_out_rejected`
  (Δ>0 §19 counterfactual: without `key_valid_at`, the receipt would
  verify forever), `additive_rotation_preserves_old_signatures`,
  `non_admin_rejected`.
- `tests/receipt_archival.rs` (NEW, 4 tests): archive→lookup round
  trip, `lookup_returns_none_when_archive_empty`,
  `archive_skips_recent_receipts`, `verify_falls_through_to_archive_on_hot_miss`
  (asserts `source == "archive"`).

### Changed

- `tests/no_bypass_audit.rs::read_only_allowlist`: added
  `("onto_attestation_rotate_keys", "READ-ONLY: admin-gated trust-set
  reload; writes only to trusted_keys_history…")`.
- README test count 507 → 523 (`tools/check-test-count.sh`-checked).

### Δ>0 counterfactual proof (§19)

Compromised Ed25519 key rotated out → `signed_then_rotated_out_rejected`
fails with `AttestationInvalid { reason: "key_not_trusted_at_signature_time" }`.
Without `key_valid_at` and the history-window check in `cell_ready`,
the receipt would verify forever.

## [Unreleased] — Round 4 WE: §14 mutation gate purity + ratchet hardening

### Changed

- `src/server.rs`: 4 falsely-allowlisted mutating handlers reclassified.
  - `onto_declare_workflow` now routes through `evaluate_admission(WorkflowDeclared, …)`
    BEFORE `WorkflowScope::open(...)`. Artifact bytes:
    `name + "\0" + powl + "\0" + tenant_id`.
  - `onto_close_workflow` now routes through `evaluate_admission(WorkflowClosed, …)`
    BEFORE `WorkflowScope::close(...)`. Artifact bytes: raw `scope_token`.
  - `onto_plan_workflow` (both `groq_powl` and `mustar` engine paths) now
    funnels through a new private helper `persist_planned_scope` which
    runs `evaluate_admission(WorkflowPlanned, …)` BEFORE the synthetic
    `INSERT INTO workflow_scopes` row.
  - `onto_exemplar_seed` now requires `BootstrapState::is_bootstrap(&db)`
    to return `true`; otherwise it fails fast with
    `DefectClass::BootstrapClosed`. On the bootstrap-passing path, an
    `evaluate_admission_audit(ExemplarSeeded, …)` event is emitted before
    the `OcelStore::seed_from_ocel_bytes` mutation.
- `src/server.rs::evaluate_admission` bypass branch: now self-attributes
  via `evaluate_admission_audit(AdmissionOp::Bypass, …)` BEFORE writing
  `revoked_sessions`. The pre-existing `admission_bypass` event is retained
  for backward compat with auditors keyed on the old `event_type`.
- `src/server.rs::onto_align`: dry_run no longer leaks an `align_run` OCEL
  event. The `emit_event("align_run", ...)` and `lineage().record("AL", …)`
  calls now live inside the `if !dry_run_flag` apply branch.
- `src/admission.rs`: 5 new `AdmissionOp` variants (`WorkflowDeclared`,
  `WorkflowClosed`, `WorkflowPlanned`, `ExemplarSeeded`, `Bypass`).
  `as_str()` and `is_full_admission()` updated; `ExemplarSeeded` and
  `Bypass` are audit-only.
- `tests/no_bypass_audit.rs`: hardened with three new sub-checks:
  (a) direct DB write detection (`body_writes_db`) catching `.execute(`,
  `.execute_batch(`, `.prepare(`, `INSERT INTO`, `UPDATE `, `DELETE FROM`;
  (b) depth-2 transitive helper scan (`handler_reaches_db_write_bypassing_gate`)
  walking `self.<helper>(` calls; (c) allowlist justification regex
  (`validate_allowlist_justification`) requiring `READ-ONLY: ` prefix and
  rejecting weasel words. The allowlist is now `HashSet<(name, justification)>`
  with proper justifications, and the four reclassified handlers were
  removed.
- Defects taxonomy: `4.2.0` → `4.3.0` (forward-compatible).
  `DefectClass::BootstrapClosed` added. New discriminant hash:
  `6984749a1ef04b4669aa22fa977506d4c0d8b1baf5898e9e7e8d9cf84e92b3d9`
  (was `a0d498dba7d299c8c105a3713186f6d7df79428896fd5133cb4575d3a18fd1f2`).

### Added

- `src/bootstrap.rs` — new module with `BootstrapState::is_bootstrap(&db)`.
  Returns `true` iff `OPEN_ONTOLOGIES_BOOTSTRAP_MODE=1` env var is set OR
  the `receipts` table has zero rows with
  `production_law_version != 'seed-v0'`.
- `tests/round4_no_bypass_red_team.rs` — 6 saboteur tests proving each
  hardened sub-check is load-bearing (depth-1 helper bypass, gated helper
  passes, conditionally-gated path stripped, weak `graph` justification
  rejected, missing `READ-ONLY:` prefix rejected, direct DB write in
  body caught).
- `tests/round4_admission_op_bypass.rs` — 2 tests proving the bypass
  branch emits an `admission_audit{op=bypass}` OCEL row BEFORE
  `revoked_sessions` is written.
- `tests/round4_align_dry_run.rs` — 2 tests proving `dry_run=true` emits
  zero `align_run` OCEL rows and `dry_run=false` emits at most one.
- `tests/no_bypass_audit.rs::read_only_allowlist_justifications_pass_regex`
  — pins every allowlist justification against the §14 regex.
- `OpenOntologiesServer::onto_align` is now `pub` so integration tests
  can drive it directly (it was already public via `#[tool]`; this just
  closes the Rust-visibility gap).
- `OntoDeclareWorkflowInput`, `OntoCloseWorkflowInput`, `OntoPlanWorkflowInput`
  gain `bypass_admission: Option<bool>` and `bypass_reason: Option<String>`
  fields, mirroring the rest of the mutating-handler input contract.

### Counterfactual proof (§19)

Without the depth-2 helper scan, a future PR adding a `self.evil_insert()`
helper inside an allowlisted handler — where `evil_insert` runs
`conn.execute("INSERT INTO …")` — would slip past `no_bypass_audit`.
`r4_red_team_depth1_helper_writes_db_caught` proves the walker catches
this exact pattern. Test count empirically rose from 496 → 507.

## [Earlier Unreleased] — Round 4 WC: §7 + §13 LLMAuthority closure

### Changed

- `signature_shape::parse_and_validate` return type: `Result<BTreeMap<String, String>, Vec<...>>`
  → `Result<ParsedFields { fields, llm_claimed_authority }, Vec<...>>`. The new
  `ParsedFields::llm_claimed_authority` flag is set when the LLM's reply contains
  `"provisional": false` or `"authoritative": true` — the canonical adversarial
  pattern that R3 only forced silently to `true` without auditing. Callers in
  `src/llm_translator.rs` and `src/server.rs` updated; in-tree tests updated.
- `onto_translate_candidate` MCP tool now emits `llm_authority_claimed` OCEL
  audit events **before** lifting the LLM's fields into a `CandidateCtq` when
  `parsed.llm_claimed_authority` is set. Both the `inproc` and `groq_pm4py`
  engines participate; the gate still forces `provisional = true` regardless,
  the OCEL event records the claim independently. Wires
  `DefectClass::LlmAuthorityClaimed` from theatrical (defined-not-emitted) to
  load-bearing.
- `onto_translate_candidate` response JSON now carries `_projection_only: true`
  (§13 JSON-as-authority) and `llm_claimed_authority: <bool>` for downstream
  inspection. The handler doc-comment states the projection-only contract:
  admission flows exclusively through `onto_admit_ctq`.
- `src/batch.rs`: 13 `serde_json::from_str(...).unwrap_or(json!({"raw": s}))`
  fail-open call sites replaced with `parse_subprocess_json(&s).into_value()`,
  backed by a new local `BatchOutcome::{Parsed(Value), Malformed { reason,
  snippet }}` enum. Malformed payloads now surface `error` and
  `subprocess_malformed: true` keys, which the existing `has_error` detector
  picks up — closing the silent fail-open hole. The enum is intentionally
  local; conflating subprocess-CLI parse errors with `DefectClass`
  variants would pollute the typed taxonomy (§21).

- Defects taxonomy bumped 4.1.0 → 4.2.0 (forward-compatible). Discriminant
  hash unchanged (tag set unchanged).

### Added

- `tests/llm_provisional_override.rs` — pins the §7 detection logic against an
  adversarial LLM reply (`"provisional": false` and `"authoritative": true`);
  pure unit-level — no HTTP, no mock — by calling `parse_and_validate`
  directly with hand-crafted JSON.
- `tests/llm_authority_zero.rs` — saboteur ratchet: lexical scan over
  `src/admission.rs`, `src/cell_ready.rs`, `src/receipts.rs`, `src/defects.rs`,
  `src/production_record.rs` ensuring no LLM-output identifier (`fields[`,
  `parsed.fields`, `candidate.ctq_text`, etc.) is assigned into authority
  structures. Self-reference safe: the forbidden patterns are stored as
  byte arrays so the test file's own source does not match.
- `tests/hearsay_returns_typed_consensus.rs` — compile-time `fn(...) ->
  SwarmConsensus` type pin via `let _: fn(...) = fuse_via_hearsay;`. Fails to
  compile if the swarm fusion function ever returns `serde_json::Value`.

### Truth-up

- `docs/06-llm-boundary.md`: new "Translate-vs-admit ratio audit" section
  documenting the projection-only contract and the `llm_authority_claimed`
  OCEL signal.

## [Unreleased] — Real Ed25519 attestation (Round-2 cascade Plan 1)

### Changed (BREAKING)

- Replaced the Phase-10 A10 tautology (`external_attestation == artifact_hash`,
  a vacuous self-check) with real Ed25519 verification using
  `ed25519_dalek::VerifyingKey::verify_strict` over
  `ProductionRecord::canonical_bytes_for_signing` (signature/fpr fields
  excluded from the signed bytes — receipt-replay defence).
- New module `src/attestation.rs` exposes `Signer::from_env()`,
  `TrustedKeys::from_env()`, and an 8-byte BLAKE3-prefix key fingerprint
  for rotation. PEM PKCS#8 keys at `OPEN_ONTOLOGIES_SIGNING_KEY_PATH` and
  `OPEN_ONTOLOGIES_TRUSTED_KEYS_DIR/*.pub.pem`.
- `ProductionRecord` gains `signature: Option<[u8; 64]>` and
  `signing_key_fpr: Option<[u8; 8]>` (additive, serde-default —
  pre-Round-2 receipts deserialize unchanged).
- `OntoStarAdmissionGate` gains `signer`, `trusted_keys`,
  `require_attestation`, `verify_legacy_receipts` knobs (builders:
  `with_signer`, `with_trusted_keys`, `require_attestation`,
  `verify_legacy_receipts`).
- A10 conjunct in `cell_ready` rewritten with three branches:
  - `signature: None` + `allow_legacy_unsigned: true` → emits
    `legacy_unsigned_receipt` OCEL audit event and passes.
  - `signature: None` + `allow_legacy_unsigned: false` →
    `DefectClass::AttestationMissing`.
  - `signature: Some(_)` → `verify_strict` →
    `DefectClass::AttestationInvalid { reason }` on rejection.
- New defect variant `DefectClass::AttestationInvalid { reason }` —
  reasons: `"signature_invalid"`, `"unknown_signing_key:<fpr>"`,
  `"missing_signing_key_fpr"`, `"no_trust_set"`, `"no_signer_configured"`.
- Defects taxonomy bumped 4.0.0 → 4.1.0; discriminant hash repinned to
  `a0d498dba7d299c8c105a3713186f6d7df79428896fd5133cb4575d3a18fd1f2`.
- `Verdict::Tampered` gains `reason: String` (additive serde-default):
  `"body_hash_mismatch"`, `"signature_invalid"`, `"unknown_signing_key"`.

### Added

- `tests/ed25519_attestation.rs` — five tests including the round-2
  receipt-replay attack (sig from receipt A pasted onto receipt B with a
  different `artifact_hash` → `AttestationInvalid { reason:
  "signature_invalid" }`).

### Truth-up

- README receipt-chain wording: removed the unconditional
  "Ed25519-signed" claim; documented the opt-in semantics keyed to
  `OPEN_ONTOLOGIES_SIGNING_KEY_PATH` /
  `OPEN_ONTOLOGIES_TRUSTED_KEYS_DIR` and the `verify_legacy_receipts`
  default.

## [Pre-release] — Phase 10 — Cell8 13-gate attestation

Source and tests for the Cell8 A1–A13 conformance gates plus the EARL emitter
are in tree (`src/cell8.rs`, `tests/cell8_thirteen_gates.rs` — 8 tests).
Awaiting the Phase-10 consolidation commit; this entry will be retagged as
`[0.10.0]` with the hash once it lands.

### Added (in tree, not yet tagged)
- `src/cell8.rs` — A1 Seed, A2 Breed, A3 Validate, A4 Reason, A5 Prove, A6 Seal, A7 Emit, A8 Journal, A9 Causal, A10 Temporal, A11 Governance, A12 Rollback, A13 Attest.
- EARL `earl:Assertion` emitter with `earl:passed` / `earl:failed` outcomes.
- Gate dependency ordering (A1 → A7 sequential; A8–A13 parallel).
- `tests/cell8_thirteen_gates.rs` — one test per gate plus full-cascade.

## [0.11.0] — Phase 11 — Multi-tenant isolation

- [`cd8b3b2`](../../commit/cd8b3b2) `feat(tenant): multi-tenant session isolation + scope-token ACLs (Phase 11)`

### Added
- `src/tenant.rs` — `TenantBoundary` defect class; per-tenant receipt chains; scope-token ACLs gating every mutation handler.
- `tests/multi_tenant_isolation.rs` — 7 isolation tests covering cross-tenant leak, scope escalation, chain bleed-through, and session-stealing attacks.

### Security
- All mutation handlers now require a tenant-scoped token; absence is `TenantBoundary::MissingScope` (hard deny, not warn).

## [0.9.0] — Phase 9 — External verifier

- [`9a4a277`](../../commit/9a4a277) `feat(verifier): external receipt verifier CLI + library API (Phase 9)`

### Added
- `src/verify.rs` + `onto verify` CLI verb — replays a receipt chain offline, no network, no shared state with the producer.
- ASCII chain visualization (`onto verify --visualize`) showing BLAKE3 link integrity at each hop.
- Library API (`open_ontologies::verify::verify_chain`) for embedding in CI.
- `tests/external_verifier_e2e.rs` — 10 tests including corrupted-signature, broken-link, and key-rotation sabotage scenarios.

## [0.8.0] — Phase 8 — Live MCP-Groq integration

- [`c8d5588`](../../commit/c8d5588) `feat(mcp-groq): live Groq subprocess engine for translate_candidate + executive_projection (Phase 8)`

### Added
- `engine="groq_pm4py"` subprocess transport — MCP handlers now spawn the real DSPy/pm4py-backed translator instead of the in-process stub.
- Live execution path covers `translate_candidate` and `executive_projection`.

### Changed
- `tests/real_groq_*.rs` (14 tests across 8 files) now exercise the live subprocess instead of the mock.

## [0.7.0] — Phase 7 — Phase 6 consolidation

- [`f44ec7e`](../../commit/f44ec7e) `feat(receipts): atomic persist+emit transaction; orphan rollback (Phase 7 C.fix)`
- [`0ab7577`](../../commit/0ab7577) `test(phase-7): close all 4 Phase-6 #[ignore] tags — happy-path admission, push verb, file-backed Oxigraph`

### Fixed
- Receipt persist + emit is now a single atomic transaction; partial-write orphans roll back instead of leaving an unsigned skeleton on disk (closes Phase-6 finding 3.3).
- All 5 Phase-6 `#[ignore]` markers removed; happy-path admission, push verb, and file-backed Oxigraph re-enter the regular test run.

## [0.6.0] — Phase 6 — Adversarial hardening cascade

Five parallel hardening tasks (A–E) closing the findings of the 5-Explore + 5-Plan
adversarial audit.

- [`9bd0611`](../../commit/9bd0611) **Task A** — `test(real-replay): switch admission tests from NoopPowlReplay to PowlBridgeReplay`. Replaces fitness=1.0 stubs with the real wasm4pm bridge across 7 test files.
- [`0527773`](../../commit/0527773) **Task B** — `test(cli): adapt CLI subprocess tests to noun-verb structure`. Revives 25 CLI subprocess tests broken by the `362fd6b` flat→noun-verb refactor.
- [`f367fed`](../../commit/f367fed) **Task C** — `feat(receipts): per-session sequence column + 3 adversarial tests`. Closes receipt-chain silent-failure modes (granted_at tie / concurrent sessions / orphaned receipt).
- [`3ed427a`](../../commit/3ed427a) **Task D part 1** — `test(defects): deny-path tests for 12 production-active variants`.
- [`bea21b4`](../../commit/bea21b4) **Task D part 2** — `feat(defects)!: bump taxonomy to 3.0.0 — remove 10 speculative dead variants`. **BREAKING**: `DefectClass` enum loses 10 unused variants.
- [`063d540`](../../commit/063d540) **Task E** — `feat(ratchets)!: harden no_bypass_audit + secret_grep_ratchet, fix 3 allowlist lies`. **BREAKING**: read-only allowlist contract tightened; `onto_workflow_discover` reclassified as mutating.
- [`4eb2dfb`](../../commit/4eb2dfb) `test(ratchet): char/string-literal-aware brace walker in no_bypass_audit (v2)` — follow-up fix to ratchet false-positives on string literals containing braces.

### Added
- `tests/admission_real_replay.rs`, `tests/receipt_chain_adversarial.rs`, `tests/ratchet_red_team.rs`, `tests/cell_ready_deny_paths.rs`.

### Removed
- 10 speculative `DefectClass` variants with zero production emissions.

### Security
- `secret_grep_ratchet` now scans format strings and `tracing::*!` macro literals (previously bypassable).

## [0.5.0] — Phase 5 — DSPy-style signature shapes

- [`286d47b`](../../commit/286d47b) `feat(signatures): Phase 5 — DSPy-style signature shapes close the LLM-to-manufacturing gap`
- [`a2c2a56`](../../commit/a2c2a56) `feat(swarm): 9 Rust+AtomVM cognition nodes fused via Hearsay-II`
- [`b5cdca7`](../../commit/b5cdca7) `test(chicago-tdd): real Groq at every human interaction point`
- [`619c3b1`](../../commit/619c3b1) `feat(real-llm): port pm4py POWL pattern across 5 LLM boundaries — REAL Groq calls`
- [`1b7d6cc`](../../commit/1b7d6cc) `feat(real-llm): port pm4py POWL example with REAL Groq calls`
- [`c4e0035`](../../commit/c4e0035) `fix(audit): adversarial audit caught broken Terraform IaC; receipt moved to sidecar`
- [`da1c115`](../../commit/da1c115) `tools(security): add untrack-secret.sh for fix-forward .env / secret removal`
- [`e27d418`](../../commit/e27d418) `fix(security): untrack .env + non-project files; tighten .gitignore`

### Added
- `src/signature_shape.rs` — DSPy-shaped signatures bridging LLM output → admission input.
- `src/swarm.rs` — 9-breed Rust+AtomVM cognition node fused via Hearsay-II blackboard.
- Chicago-TDD discipline: every human-interaction point covered by a real-Groq test.

### Security
- `.env` and other secret-bearing files untracked; `.gitignore` tightened; `tools/untrack-secret.sh` added for fix-forward removal.

## [0.4.0] — Phase 4 — Autonomic multi-target solution manufacturing

- [`eb0b8ca`](../../commit/eb0b8ca) `feat(manufacturing): Phase 4 — autonomic multi-target solution manufacturing`
- [`fe838f2`](../../commit/fe838f2) `fix(test): rebind capability_rollup to DEFECTS_TAXONOMY_VERSION constant`

### Added
- `src/manufacturing/` — `iac.rs` (Terraform), `rust_target.rs`, `erlang.rs`, `atomvm.rs`, `validators.rs`.
- Multi-target work-order routing with per-target validators and toolchain probes.
- `tests/manufacturing_validators.rs` (8 tests), `tests/adversarial_real_toolchains.rs` (5 real-toolchain tests).

## [0.3.0] — Phase 3 — RevOps test case (CTQ admission)

- [`d3cfdc1`](../../commit/d3cfdc1) `test(revops): Phase 3.5 + 3.6 + 3.7 — 9 station tests + counterfactual + Fortune-5 E2E`
- [`1e57bd4`](../../commit/1e57bd4) `test(revops): Phase 3.3 + 3.4 — 8 negative tests + 4 Groq boundary tests`
- [`233f99c`](../../commit/233f99c) `test(revops): Phase 3.1 + 3.2 — fake OCEL fixture + 5 CTQ admission tests`
- [`5f808d2`](../../commit/5f808d2) `test(requirements-andon): small-first E2E — gate of Phase 3`
- [`53f9713`](../../commit/53f9713) `feat(requirements-andon): ontology + advisory SHACL shapes (CLI surface deferred)`
- [`663d924`](../../commit/663d924) `feat(old-ai-stations): wire wasm4pm-cognition breeds via onto_old_ai_station`
- [`41c5696`](../../commit/41c5696) `feat(requirements-andon): wire 6 MCP handlers + no-bypass ratchet update`
- [`e20a8c4`](../../commit/e20a8c4) `test(secret-hygiene): canary leak scan + textual log/format ratchet`
- [`5f60633`](../../commit/5f60633) `feat(requirements-andon): Groq LLM boundary translator + [llm] config + .env loading`
- [`c33086f`](../../commit/c33086f) `feat(requirements-andon): add RequirementsManufacturing + Fortune5RevOpsGovernedRelease workflows`
- [`fb419f5`](../../commit/fb419f5) `feat(requirements-andon): extend AdmissionOp + DefectClass for CTQ-Forge layer`

### Added
- Requirements-andon ontology, advisory SHACL shapes, 6 MCP handlers.
- CTQ-Forge admission layer (`AdmissionOp`, `DefectClass` extensions).
- Groq LLM boundary translator with `[llm]` config block and `.env` loading.
- 9 RevOps station tests, 8 negative tests, 4 Groq boundary tests, Fortune-5 E2E.
- `tests/secret_hygiene.rs` canary leak scan.

> The product is CodeManufactory; RevOps is merely proof that CodeManufactory works.

## [0.2.0] — Phase 2 — Recursive admission (Level-5)

- [`346ce74`](../../commit/346ce74) `test(level-5): add portability_push, portability_codegen, capability_rollup; fix init() dead tuple`
- [`b70b2ca`](../../commit/b70b2ca) `feat(level-5): close no-bypass gate — gate 12 mutation handlers`
- [`a410e31`](../../commit/a410e31) `feat(level-5): receipt portability into TTL/codegen/push artifacts`
- [`554789a`](../../commit/554789a) `feat(level-5): replay-from-OCEL-alone + counterfactual binding`
- [`c51a29e`](../../commit/c51a29e) `feat(level-5): capability rollup + defect taxonomy versioning`
- [`ee90af9`](../../commit/ee90af9) `fix(no-stub): wire ingest/map/extend/push format params; add named-graph push; remove dead cfg`
- [`623a2e3`](../../commit/623a2e3) `feat(stream-5): wire onto_plan_workflow, onto_exemplar_seed, onto_counterfactual`
- [`2749500`](../../commit/2749500) `feat(admission): wire real precision into admission gate (p_min=0.7)`
- [`d224432`](../../commit/d224432) `fix(cell-ready): ocel_complete checks required ⊆ observed instead of non-empty`

### Added
- 12 mutation handlers gated by no-bypass audit; capability rollup; defect taxonomy versioning.
- Receipt portability into TTL / codegen / push artifacts.
- `replay-from-OCEL-alone` and counterfactual binding.
- Real precision (`p_min=0.7`) wired into the admission gate.

### Fixed
- `cell_ready.ocel_complete` now checks `required ⊆ observed` instead of merely non-empty.

## [0.1.0] — Phase 1 — OntoStar foundation (streams 1–5)

- [`1a587c7`](../../commit/1a587c7) `ontostar: end-to-end DoD test + build_ocel session-filter fix`
- [`8c3537a`](../../commit/8c3537a) `ontostar: fix three post-merge test issues`
- [`571a6c0`](../../commit/571a6c0) `ontostar: integration of streams 1-5 complete (option A: working-tree integration)`
- [`d0fe244`](../../commit/d0fe244) `ontostar(stream 5): no-op vs master per prior audit`
- [`e4db225`](../../commit/e4db225) `ontostar(R3): wire PowlBridgeReplay into admission gate, replace stub`
- [`33feda7`](../../commit/33feda7) `ontostar(R2): DefectClass::ThresholdFailed.metric is String (no-op)`
- [`f4f1f01`](../../commit/f4f1f01) `ontostar(R1): rewrite builtin catalog POWL strings to wasm4pm grammar`
- [`12f3d39`](../../commit/12f3d39) `ontostar(stream 3): admission gate integration tests`
- [`f11ffc0`](../../commit/f11ffc0) `ontostar(stream 4): feedback loop integration tests`
- [`7d52e35`](../../commit/7d52e35) `ontostar(stream 2): wasm4pm POWL replay integration tests`
- [`be04ae0`](../../commit/be04ae0) `ontostar(streams 1-4): integrated foundation for OntoStar manufacturing lifecycle`

### Added
- Streams 1–5 integrated: ontology layer, wasm4pm POWL replay bridge, admission gate, feedback loop.
- `PowlBridgeReplay` replaces the noop stub in the admission gate.
- Builtin catalog rewritten to wasm4pm grammar.
- End-to-end DoD test plus `build_ocel` session-filter fix.

---

## Pre-OntoStar — open-ontologies MCP server (0.1.x)

The `ontostar-integration` branch builds on top of the original `open-ontologies`
project, an AI-native MCP server for RDF/OWL ontology engineering with 50+
`onto_*` tools, an Oxigraph-backed triple store, SHACL validation, OWL-RL
reasoning, semantic embeddings, clinical crosswalks, and Terraform-style
lifecycle management.

- **0.1.13** — Compile cache + TTL eviction + tool-exposure filter; ontology
  repository directories; OpenAI-compatible embeddings provider; surfaced
  operational config.
- **0.1.12** — Virtualized tree view (Studio); 13-step deep builder
  (`/build`); IES-level ontology generation.
- **0.1.11 and earlier** — Initial 50-tool MCP surface, marketplace of 32
  standard ontologies, lineage trail, drift detection, alignment with
  self-calibrating confidence weights, doctor diagnostics, persistent store.

Detail for the pre-OntoStar releases is preserved in the project's git history
on `main`.
