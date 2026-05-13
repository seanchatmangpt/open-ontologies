# 03 — MCP Tool Catalogue

Every tool registered with `#[tool(name = "onto_*")]` in `src/server.rs`, grouped by admission tier. Tiers are enforced by `tests/no_bypass_audit.rs` — a tool that mutates persistent state without calling `evaluate_admission` or `evaluate_admission_audit` will fail the no-bypass audit (Phase 6 Task E hardened the ratchet to catch dead-code, alias, and string-literal bypasses).

## Tiers

| Tier | Definition | Enforcement |
|------|------------|-------------|
| **Full admission** | Mutates persistent state. Calls `OntoStarAdmissionGate::evaluate(op, ...)`. Returns `(DefectClass, Vec<Deviation>)` on denial. Emits `admission_granted` / `admission_denied` OCEL event. Receipt chained. | `tests/no_bypass_audit.rs::every_mcp_handler_is_gated_audited_or_explicitly_readonly` |
| **Audit-only** | Side-effecting but not a deterministic mutation (LLM proposal, discovery, threshold sweep, feedback). Calls `evaluate_admission_audit(op, ...)`. Receipt is descriptive, not bound to artifact bytes. | Same audit; `is_full_admission()` returns `false`. |
| **Read-only** | Returns information. No persistent state change. Listed in `read_only_allowlist()`. | Phase 6 Task E removed three lying allowlist entries. |

## Full-admission tools (typed `AdmissionOp` variant)

| Tool | `AdmissionOp` | What it does |
|------|---------------|--------------|
| `onto_apply` | `Apply` | Apply ontology mutation to Oxigraph |
| `onto_codegen` | `Codegen` | Run ggen pipeline; emit `src/cmds/generated.rs` |
| `onto_save` | `Save` | Serialize store to TTL file |
| `onto_push` | `Push` | Push to remote SPARQL endpoint |
| `onto_ingest` | `Ingest` | Parse structured data → RDF, load |
| `onto_sql_ingest` | `Ingest` | SQL SELECT → RDF, load |
| `onto_import` | `Ingest` | Resolve owl:imports chains |
| `onto_import_schema` | `ImportSchema` | DB schema → OWL ontology |
| `onto_align` | `Align` | Detect alignment candidates between two ontologies |
| `onto_rollback` | `Rollback` | Restore prior version snapshot |
| `onto_version` | `Version` | Save named snapshot |
| `onto_clear` | `Clear` | Reset store |
| `onto_propose_requirement` | `RequirementProposed` | Propose requirement (CTQ Forge tier 1) |
| `onto_admit_ctq` | `CtqAdmitted` | Admit CTQ candidate (tier 2) |
| `onto_propose_work_order` | — | Stages work-order proposal |
| `onto_admit_work_order` | `WorkOrderAdmitted` | Admit work order (tier 3) |
| `onto_manufacture_solution` | `SolutionManufactured` | Emit IaC+Rust+Erlang+AtomVM (tier 4) |
| `onto_extend` | `Ingest` | Convenience: ingest+SHACL+reason |
| `onto_pull` | `Ingest` | Fetch from remote URL/endpoint |

## Audit-only tools

| Tool | `AdmissionOp` | What it does |
|------|---------------|--------------|
| `onto_translate_candidate` | `LlmTranslate` | Groq translation (LLM proposes) |
| `onto_executive_projection` | `LlmTranslate` | Groq executive summary projection |
| `onto_plan_workflow` | `LlmTranslate` | Groq POWL workflow proposal |
| `onto_workflow_discover` | `Discovery` | Discover workflow from event log |
| `onto_workflow_feedback` | `Feedback` | Operator feedback on discovery |
| `onto_threshold_sweep` | `ThresholdSweep` | Sweep threshold parameters |
| `onto_align_feedback` | `Feedback` | Accept/reject alignment candidate |
| `onto_lint_feedback` | `Feedback` | Accept/dismiss lint issue |
| `onto_enforce_feedback` | `Feedback` | Accept/dismiss enforce violation |
| `onto_counterfactual` | — | Counterfactual binding for work order |
| `onto_exemplar_seed` | — | Seed exemplar fixture |
| `onto_old_ai_station` | — | Dispatch to wasm4pm cognition breed |
| `onto_alphastar_solve` | — | AlphaStar planner |
| `onto_mustar_solve` | — | MuStar planner |
| `onto_planner_demos` | — | List planner demonstrations |
| `onto_enrich` | — | Add skos:exactMatch via crosswalks |

## Read-only tools

| Tool | What it returns |
|------|-----------------|
| `onto_status` | Server health |
| `onto_verify` | Walk the receipt chain for a given hash; return verdict (`Admitted` / `Tampered { reason }` / `Orphan`) and ASCII chain tree. Pure read-only; no admission, no OCEL emission. |
| `onto_groq_status` | Groq API connectivity |
| `onto_stats` | Class/property/triple counts |
| `onto_query` | SPARQL SELECT/CONSTRUCT/ASK/DESCRIBE |
| `onto_validate` | SHACL conforms / violation report |
| `onto_shacl` | SHACL validation against ad-hoc shapes |
| `onto_reason` | RDFS / OWL-RL inference (in-memory only) |
| `onto_lint` | Missing labels/comments/domains/ranges |
| `onto_enforce` | Design-pattern check (read-only when no `--apply`) |
| `onto_diff` | Compare two versions |
| `onto_drift` | Drift velocity + rename detection |
| `onto_lineage` | Session lineage trail |
| `onto_history` | Saved snapshot list |
| `onto_search` | Embedding-based class search |
| `onto_similarity` | Cosine + Poincaré between two IRIs |
| `onto_embed` | Generate embeddings (treated as read-only; output cached not persisted to graph) |
| `onto_dl_check` | DL tableaux subsumption |
| `onto_dl_explain` | DL tableaux clash trace |
| `onto_marketplace` | Browse curated ontologies |
| `onto_repo_list` | Enumerate `ontology_dirs` |
| `onto_repo_load` | Load from configured dir |
| `onto_load` | Load TTL into store (treated read-only-ish via cache; mutation is into ephemeral cache) |
| `onto_unload` | Remove from in-memory store |
| `onto_recompile` | Re-parse source, rebuild cache |
| `onto_cache_status` / `onto_cache_list` / `onto_cache_remove` | Cache introspection |
| `onto_lock` | Mark IRIs production-locked |
| `onto_monitor` | Run SPARQL watchers |
| `onto_monitor_clear` | Clear blocked-state |
| `onto_plan` | Show added/removed/risk score (preview only) |
| `onto_threshold_status` | Read threshold config |
| `onto_validate_clinical` | Clinical-crosswalk label check |
| `onto_crosswalk` | ICD/SNOMED/MeSH lookup |
| `onto_map` | Generate mapping config (no ingest) |
| `onto_convert` | Format conversion |
| `onto_process_check_soundness` | Process-mining soundness check |
| `onto_process_validate_claim` | Validate process claim against OCEL |

## Defects each tool can return

Every full-admission tool can return any of: `CapabilityZero`, `SkippedTask`, `ExtraTask`, `WrongOrder`, `BypassRevoked`, `ReceiptMissing`, `ScopeUnclosed`, `OcelIncomplete`, `ThresholdFailed`, `ReplayFailed`, `DeadParameter`, plus the operation-specific defects (e.g. manufacturing tools also emit `GeneratorEmpty`, `IacInvalid`, `RustInvalid`, `ErlangInvalid`, `AtomVmInvalid`, `ManufacturingChainBroken`). See `docs/04-defect-taxonomy.md`.
