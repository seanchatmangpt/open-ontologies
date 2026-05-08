use schemars::JsonSchema;
use serde::Deserialize;

// ─── MCP tool input structs ─────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct OntoValidateInput {
    /// Path to an RDF file OR inline Turtle content
    pub input: String,
    /// If true, treat input as inline content rather than a file path
    pub inline: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoConvertInput {
    /// Path to source RDF file
    pub path: String,
    /// Target format: turtle, ntriples, rdfxml, nquads, trig
    pub to: String,
    /// Optional output file path (if omitted, returns content)
    pub output: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLoadInput {
    /// Path to RDF file, OR inline Turtle/RDF content
    pub path: Option<String>,
    /// Inline Turtle content to load (alternative to path)
    pub turtle: Option<String>,
    /// Optional name for this ontology in the registry. Defaults to the file
    /// stem of `path`. When omitted for inline turtle, defaults to "default".
    pub name: Option<String>,
    /// When true, every subsequent read tool checks the source file's mtime
    /// and recompiles if it changed. Has no effect for inline turtle.
    pub auto_refresh: Option<bool>,
    /// When true, ignore the on-disk compile cache and re-parse from source.
    pub force_recompile: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoUnloadInput {
    /// When true, also delete the on-disk compile cache file.
    pub delete_cache: Option<bool>,
    /// Optional ontology name. When omitted, operates on the currently active
    /// ontology. When provided, targets that named cache entry — if it is the
    /// active slot the in-memory store is cleared; otherwise only the on-disk
    /// cache is touched (and only when `delete_cache` is true).
    pub name: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoRecompileInput {
    /// Optional ontology name. When omitted, recompiles the active ontology.
    /// When provided, recompiles that cached entry from its recorded source
    /// path; if the entry is not active, the active in-memory store is left
    /// untouched.
    pub name: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoRepoListInput {
    /// Optional subdirectory to scan instead of every configured ontology
    /// repo. Must resolve under one of the configured `ontology_dirs`
    /// entries; arbitrary host paths are rejected (path-traversal guard).
    pub dir: Option<String>,
    /// Walk subdirectories recursively. Defaults to false (top-level only).
    pub recursive: Option<bool>,
    /// Optional filename glob filter (e.g. `*.ttl`, `foo*`). Matches the
    /// filename only, not the full path.
    pub glob: Option<String>,
    /// Maximum number of entries to return. Default 1000.
    pub limit: Option<usize>,
    /// Skip the first `offset` entries (for pagination). Default 0.
    pub offset: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoRepoLoadInput {
    /// Identifier of the ontology to load. Accepts:
    ///   - a bare name (e.g. `pizza`) matching a file stem under any
    ///     configured `ontology_dirs`,
    ///   - a relative path (e.g. `subdir/pizza.ttl`) resolved against the
    ///     configured directories,
    ///   - an absolute path inside one of the configured directories.
    ///
    /// Paths outside the configured `ontology_dirs` are rejected.
    pub name: String,
    /// Optional registry name override (defaults to the file stem).
    pub registry_name: Option<String>,
    /// When true, every subsequent read tool checks the source file's mtime
    /// and recompiles if it changed.
    pub auto_refresh: Option<bool>,
    /// When true, ignore the on-disk compile cache and re-parse from source.
    pub force_recompile: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCacheStatusInput {}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCacheListInput {}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCacheRemoveInput {
    /// Name of the cached ontology to remove.
    pub name: String,
    /// When true (default), also delete the on-disk N-Triples cache file.
    /// When false, only the metadata row is removed.
    pub delete_file: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoQueryInput {
    /// SPARQL query string
    pub query: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoSaveInput {
    /// Output file path
    pub path: String,
    /// Format: turtle, ntriples, rdfxml, nquads, trig
    pub format: Option<String>,
    /// Optional explicit scope token for admission. Falls back to the latest
    /// open scope for the session.
    pub scope_token: Option<String>,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoDiffInput {
    /// Path to the old/original ontology file
    pub old_path: String,
    /// Path to the new/modified ontology file
    pub new_path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLintInput {
    /// Path to RDF file to lint, OR inline Turtle content
    pub input: String,
    /// If true, treat input as inline content
    pub inline: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoPullInput {
    /// Remote URL or SPARQL endpoint to fetch ontology from
    pub url: String,
    /// If true, treat url as a SPARQL endpoint and run a CONSTRUCT query
    pub sparql: Option<bool>,
    /// Optional SPARQL CONSTRUCT query (required if sparql=true)
    pub query: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoPushInput {
    /// Remote SPARQL endpoint URL
    pub endpoint: String,
    /// Optional named graph IRI
    pub graph: Option<String>,
    /// Optional explicit scope token for admission. Falls back to the latest
    /// open scope for the session.
    pub scope_token: Option<String>,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoImportInput {
    /// Resolve and load all owl:imports from the currently loaded ontology
    pub max_depth: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoVersionInput {
    /// Version label (e.g. "v1.0", "draft-2026-03-09")
    pub label: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoRollbackInput {
    /// Version label to restore
    pub label: String,
    /// Optional explicit scope token; falls back to the latest open scope.
    pub scope_token: Option<String>,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoIngestInput {
    /// Path to the data file (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet)
    pub path: String,
    /// Data format (auto-detected from extension if omitted): csv, json, ndjson, xml, yaml, xlsx, parquet
    pub format: Option<String>,
    /// Mapping config as JSON string or path to mapping JSON file
    pub mapping: Option<String>,
    /// If true, treat mapping as inline JSON (default: false = file path)
    pub inline_mapping: Option<bool>,
    /// Base IRI for generated instances (default: http://example.org/data/)
    pub base_iri: Option<String>,
    /// Optional explicit scope token; falls back to the latest open scope.
    pub scope_token: Option<String>,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoMapInput {
    /// Path to sample data file to generate mapping for
    pub data_path: String,
    /// Data format (auto-detected if omitted)
    pub format: Option<String>,
    /// Optional path to save the generated mapping config
    pub save_path: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoShaclInput {
    /// Path to SHACL shapes file OR inline SHACL Turtle content
    pub shapes: String,
    /// If true, treat shapes as inline Turtle content
    pub inline: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoReasonInput {
    /// Reasoning profile: rdfs (default), owl-rl
    pub profile: Option<String>,
    /// If true (default), add inferred triples to the store. If false, dry-run only.
    pub materialize: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoDlExplainInput {
    /// IRI of the class to explain unsatisfiability for
    pub class_iri: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoDlCheckInput {
    /// IRI of the sub-class (the more specific class)
    pub sub_class: String,
    /// IRI of the super-class (the more general class)
    pub super_class: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoExtendInput {
    /// Path to the data file
    pub data_path: String,
    /// Data format (auto-detected if omitted)
    pub format: Option<String>,
    /// Mapping config (inline JSON or file path)
    pub mapping: Option<String>,
    /// If true, treat mapping as inline JSON
    pub inline_mapping: Option<bool>,
    /// Base IRI for generated instances
    pub base_iri: Option<String>,
    /// Path to SHACL shapes file or inline Turtle
    pub shapes: Option<String>,
    /// If true, treat shapes as inline Turtle
    pub inline_shapes: Option<bool>,
    /// Reasoning profile (rdfs, owl-rl). Omit to skip reasoning.
    pub reason_profile: Option<String>,
    /// If true (default), stop pipeline on SHACL violations
    pub stop_on_violations: Option<bool>,
    /// Optional explicit scope token; falls back to the latest open scope.
    pub scope_token: Option<String>,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
}

// ─── v2 input structs ───────────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct OntoPlanInput {
    /// New ontology as inline Turtle content
    pub new_turtle: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoApplyInput {
    /// Apply mode: "safe" (default), "force" (skip monitor watchers), "migrate" (adds bridges).
    ///
    /// NOTE on `force` semantics: the legacy `force` mode is retained but its
    /// meaning is now narrowed — it only skips monitor watchers. It does NOT
    /// bypass the OntoStar admission gate. To bypass admission, use the
    /// explicit `bypass_admission` field below (which requires a `bypass_reason`
    /// and revokes the session for subsequent operations until
    /// `onto_session_reset`).
    pub mode: Option<String>,
    /// Optional explicit scope token; if omitted, the latest open scope for
    /// the session is used. If no scope is open, admission denies with
    /// `ScopeUnclosed`.
    pub scope_token: Option<String>,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true. Free text but must be non-empty.
    pub bypass_reason: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLockInput {
    /// IRIs to lock (prevent removal)
    pub iris: Vec<String>,
    /// Reason for locking
    pub reason: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoDriftInput {
    /// First version as inline Turtle
    pub version_a: String,
    /// Second version as inline Turtle
    pub version_b: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoEnforceInput {
    /// Rule pack to enforce: "generic", "boro", "value_partition", or custom pack name
    pub rule_pack: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoMonitorInput {
    /// Inline JSON array of watchers to add, or omit to just run existing watchers
    pub watchers: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCrosswalkInput {
    /// Clinical code to look up (e.g. "I10")
    pub code: String,
    /// Source system (e.g. "ICD10", "SNOMED", "MeSH")
    pub source_system: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoEnrichInput {
    /// IRI of the ontology class to enrich
    pub class_iri: String,
    /// Clinical code to map to
    pub code: String,
    /// Code system (e.g. "ICD10")
    pub system: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLineageInput {
    /// Session ID to query (omit for current session)
    pub session_id: Option<String>,
    /// Export format: "text" (default) or "ocel" (Object-Centric Event Log JSON)
    pub format: Option<String>,
}

// ─── OntoStar Stream 1 — workflow scope ─────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct OntoDeclareWorkflowInput {
    /// Built-in workflow name (e.g. "OntologyAuthoring", "DataExtension"). If
    /// omitted, `powl` must be provided.
    pub name: Option<String>,
    /// Inline POWL string. Overrides catalog lookup when both are supplied.
    pub powl: Option<String>,
    /// Reuse an existing scope token (idempotent re-open). If omitted, a new
    /// ULID is minted.
    pub scope_token: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCloseWorkflowInput {
    /// The scope token returned by `onto_declare_workflow`.
    pub scope_token: String,
}

// ─── OntoStar Stream 2 — conformance check ──────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct OntoConformanceCheckInput {
    /// Scope token returned by `onto_declare_workflow`. Events tagged with
    /// this scope are projected to a trace and replayed against the declared
    /// POWL via `wasm4pm` (zero local PM math).
    pub scope_token: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoImportSchemaInput {
    /// Database connection string. Supported:
    ///   - `postgres://user:pass@host/db` (requires `postgres` feature)
    ///   - `duckdb:///path/to/file.duckdb` or bare `/path/to/file.duckdb` (requires `duckdb` feature)
    ///   - `:memory:` for an in-memory DuckDB database (requires `duckdb` feature)
    pub connection: String,
    /// Base IRI for generated classes (default: http://example.org/db/)
    pub base_iri: Option<String>,
    /// Optional explicit scope token; falls back to the latest open scope.
    pub scope_token: Option<String>,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoSqlIngestInput {
    /// Database connection string. Same forms as `onto_import_schema`:
    /// `postgres://…`, `duckdb:///path/to.duckdb`, `:memory:`, or a bare
    /// `*.duckdb` file path.
    pub connection: String,
    /// SQL SELECT statement to run. Returned rows are converted to RDF using
    /// the supplied mapping (or an auto-generated one).
    pub sql: String,
    /// Mapping config as JSON string or path to a mapping JSON file.
    /// Same shape as `onto_ingest`. Optional — if omitted, an auto-mapping
    /// is generated from the column names.
    pub mapping: Option<String>,
    /// If true, treat `mapping` as inline JSON (default: false = file path).
    pub inline_mapping: Option<bool>,
    /// Base IRI for generated instances (default: http://example.org/data/)
    pub base_iri: Option<String>,
    /// Optional explicit scope token; falls back to the latest open scope.
    pub scope_token: Option<String>,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoAlignInput {
    /// Source ontology: inline Turtle content or file path
    pub source: String,
    /// Target ontology: inline Turtle content or file path. If omitted, aligns against loaded store
    pub target: Option<String>,
    /// Minimum confidence threshold for auto-apply (default 0.85)
    pub min_confidence: Option<f64>,
    /// If true, return candidates only without inserting triples (default false)
    pub dry_run: Option<bool>,
    /// Optional explicit scope token; falls back to the latest open scope.
    /// Only consulted when `dry_run=false` (auto-apply path requires admission).
    pub scope_token: Option<String>,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoAlignFeedbackInput {
    /// Source class IRI from the alignment candidate
    pub source_iri: String,
    /// Target class IRI from the alignment candidate
    pub target_iri: String,
    /// Whether the alignment candidate was correct
    pub accepted: bool,
    /// Signal values from the alignment candidate (copied from the "signals" field in align output)
    pub signals: Option<std::collections::HashMap<String, f64>>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLintFeedbackInput {
    /// The lint rule ID (e.g. "missing_label", "missing_comment", "missing_domain", "missing_range")
    pub rule_id: String,
    /// The entity IRI that triggered the lint issue
    pub entity: String,
    /// true = this is a real issue, false = dismiss/ignore
    pub accepted: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoEnforceFeedbackInput {
    /// The enforce rule ID (e.g. "orphan_class", "missing_domain", "missing_range", "missing_label", or custom rule ID)
    pub rule_id: String,
    /// The entity IRI that triggered the violation
    pub entity: String,
    /// true = this is a real violation, false = dismiss/override
    pub accepted: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoEmbedInput {
    /// Structural embedding dimension. Default: 32
    pub struct_dim: Option<usize>,
    /// Structural training epochs. Default: 100
    pub struct_epochs: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoSearchInput {
    /// Natural language query
    pub query: String,
    /// Number of results. Default: 10
    pub top_k: Option<usize>,
    /// Search mode: "text", "structure", or "product". Default: "product"
    pub mode: Option<String>,
    /// Weight for text vs structure in product mode (0.0-1.0). Default: 0.5
    pub alpha: Option<f32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoSimilarityInput {
    /// First IRI
    pub iri_a: String,
    /// Second IRI
    pub iri_b: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoMarketplaceInput {
    /// Action: "list" to browse available ontologies, "install" to fetch and load one
    pub action: String,
    /// Ontology ID to install (e.g. "prov-o", "schema-org", "foaf"). Required for "install".
    pub id: Option<String>,
    /// Filter list by domain (e.g. "foundational", "metadata", "iot", "geospatial")
    pub domain: Option<String>,
}

// ─── Prompt input structs ───────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct BuildOntologyInput {
    /// Description of the domain to model (e.g. "A pizza ontology with toppings, bases, and named pizzas")
    pub domain: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ValidateOntologyInput {
    /// Path to the ontology file to validate
    pub path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct CompareOntologiesInput {
    /// Path to the old/original ontology file
    pub old_path: String,
    /// Path to the new/modified ontology file
    pub new_path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct IngestDataInput {
    /// Path to the data file (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet)
    pub data_path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct AlignOntologiesInput {
    /// Path to the source ontology file
    pub source_path: String,
    /// Path to the target ontology file
    pub target_path: String,
}

// ─── Process Mining / WvdA Agent ────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct OntoProcessValidateClaimInput {
    /// The claim to validate (e.g., 'manufacturing pipeline executed lawfully')
    pub claim: String,
    /// Artifact ID to filter traces (optional)
    pub artifact_id: Option<String>,
    /// Time range to search for traces (hours, default: 24)
    pub time_range_hours: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoProcessCheckSoundnessInput {
    /// Path to OCEL event log file
    pub event_log_path: String,
}

// ─── Semantic Lowering / MuStar & AlphaStar ─────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct OntoMustarSolveInput {
    /// The problem statement to solve or intent to lower
    pub problem_statement: String,
    /// Domain context (e.g., ALGORITHM, API, DB) - defaults to ALGORITHM
    pub domain: Option<String>,
    /// Technical constraints to adhere to
    pub constraints: Option<String>,
    /// Title for the generated artifact
    pub title: Option<String>,
}

pub type OntoAlphastarSolveInput = OntoMustarSolveInput;

#[derive(Deserialize, JsonSchema)]
pub struct OntoAdmissionCheckInput {
    /// Operation to dry-run admission for: "apply", "codegen", "save", "push".
    pub op: String,
    /// Optional explicit scope token; falls back to the latest open scope for
    /// the session.
    pub scope_token: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoSessionResetInput {
    /// Session id whose `revoked_sessions` row should be cleared.
    pub session_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCodegenInput {
    /// Generator name / target language (e.g., "python-client" → python, "rust-structs" → rust, "typescript-types" → typescript, or accepted values: python, rust, typescript, go, elixir)
    pub generator: String,
    /// Optional output directory where generated code will be written. Default: ./generated
    pub output_dir: Option<String>,
    /// If true, run in dry-run mode (preview without writing files)
    pub dry_run: Option<bool>,
    /// Path to a ggen.toml with generation.rules (manifest mode). Either this or queries_dir must be provided.
    pub manifest_path: Option<String>,
    /// Path to a directory of SPARQL .rq query files (low-level pipeline mode). Either this or manifest_path must be provided.
    pub queries_dir: Option<String>,
    /// Optional explicit scope token for admission. Falls back to the latest
    /// open scope for the session.
    pub scope_token: Option<String>,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
}

// ─── OntoStar Stream 4 — feedback handler inputs ─────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct OntoPlannerDemosInput {
    /// Domain key (workflow class name, e.g. "OntologyAuthoring").
    pub domain: String,
    /// Minimum fitness floor for returned exemplars. Default 0.95.
    pub min_fitness: Option<f64>,
    /// Maximum number of exemplars. Default 10.
    pub limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoWorkflowDiscoverInput {
    /// Domain key (workflow class name) to run discovery for.
    pub domain: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoWorkflowFeedbackInput {
    /// `discovered_workflows.id` row to flip.
    pub id: String,
    /// true = mark accepted, false = mark rejected.
    pub accepted: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct GenerateCodeInput {
    /// Programming language (e.g., "Python", "Rust", "TypeScript"). Default: "Python"
    pub language: Option<String>,
    /// ggen generator name (e.g., "python-client", "rust-structs", "typescript-types"). Default: "python-client"
    pub generator: Option<String>,
    /// Output directory where generated code will be written. Default: "./generated"
    pub output_dir: Option<String>,
}

// ─── Stream 5 inputs ────────────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct OntoPlanWorkflowInput {
    /// Natural-language description of the workflow to plan.
    pub problem_statement: String,
    /// Domain bucket used to look up warm-start exemplars (e.g. "ONTOLOGY").
    pub domain: String,
    /// Optional technical constraints passed to the planner.
    pub constraints: Option<String>,
    /// Override the python interpreter (default: "python3").
    pub python: Option<String>,
    /// Override the planner script path. Default:
    /// `~/chatmangpt/ostar/src/ostar/process/ontostar_planner.py`.
    pub planner_script: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoExemplarSeedInput {
    /// Path to the seed OCEL JSON file. Default:
    /// `~/chatmangpt/ostar/artifacts/ocel/mu_star/ONTOLOGY.oceljson`.
    pub path: Option<String>,
    /// Domain to assign to seeded exemplars when the OCEL event omits it.
    pub domain: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCounterfactualInput {
    /// Scope token returned by `onto_declare_workflow` / `onto_plan_workflow`.
    pub scope_token: String,
}

// ── Requirements-Andon / CTQ-Forge inputs (Phase 1.5) ────────────────────

/// Capture a source-voice signal and propose a requirement. The
/// admission gate denies with `RequirementWithoutSource` if
/// `source_voice` is empty.
#[derive(Deserialize, JsonSchema)]
pub struct OntoProposeRequirementInput {
    /// Stakeholder voice / process evidence / defect log line. The
    /// translator echoes this back; the gate refuses on empty / whitespace.
    pub source_voice: String,
    /// Voice category — one of `customer`, `operator`, `process`,
    /// `defect`, `control_plan`, `counterfactual`, `business`, `policy`.
    pub voice_kind: Option<String>,
    /// Optional explicit scope token for admission. Falls back to the
    /// latest open scope for the session.
    pub scope_token: Option<String>,
    pub bypass_admission: Option<bool>,
    pub bypass_reason: Option<String>,
}

/// Call the Groq LLM boundary translator on a previously-proposed
/// requirement. **Audit-only.** The translator output is provisional
/// and must pass through `onto_admit_ctq` before any work order is
/// admitted.
#[derive(Deserialize, JsonSchema)]
pub struct OntoTranslateCandidateInput {
    /// Scope token returned by `onto_propose_requirement`.
    pub scope_token: String,
    /// Source-voice signal to translate (re-echoed for self-contained
    /// audit; must match the value provided to `onto_propose_requirement`).
    pub source_voice: String,
}

/// Admit a CTQ. The deterministic gate denies with
/// `CtqIncomplete{missing}` if any of the 5 mandatory fields are
/// empty / whitespace.
#[derive(Deserialize, JsonSchema)]
pub struct OntoAdmitCtqInput {
    /// Scope token from the propose / translate phase.
    pub scope_token: String,
    /// Source-voice the CTQ derives from.
    pub source_voice: String,
    /// CTQ statement.
    pub ctq_text: String,
    /// Measurement description.
    pub measure_text: String,
    /// Verification method.
    pub verification_text: String,
    /// Negative case.
    pub negative_case_text: String,
    /// Control plan.
    pub control_plan_text: String,
    pub bypass_admission: Option<bool>,
    pub bypass_reason: Option<String>,
}

/// Bind an admitted CTQ to a draft work order with a counterfactual
/// delta. **Read-only / allowlisted** — no mutation; admission happens
/// at `onto_admit_work_order`.
#[derive(Deserialize, JsonSchema)]
pub struct OntoProposeWorkOrderInput {
    pub scope_token: String,
    /// Receipt hash returned by `onto_admit_ctq`.
    pub ctq_receipt_hash: String,
    /// Naked-craft path: what an unadmitted prompt-to-code workflow
    /// would have allowed.
    pub naked_craft_path: String,
    /// Manufacturing path: what OntoStar admission/replay enforces.
    pub manufacturing_path: String,
    /// Material delta — the defect or risk this work order prevents.
    pub counterfactual_delta: String,
}

/// Admit a work order. Denies with `WorkOrderMissingCounterfactual` if
/// the counterfactual fields are absent.
#[derive(Deserialize, JsonSchema)]
pub struct OntoAdmitWorkOrderInput {
    pub scope_token: String,
    pub ctq_receipt_hash: String,
    pub naked_craft_path: String,
    pub manufacturing_path: String,
    pub counterfactual_delta: String,
    pub bypass_admission: Option<bool>,
    pub bypass_reason: Option<String>,
}

/// Project admitted evidence into an executive-readable summary via
/// the Groq translator. **Read-only / allowlisted.** The summary must
/// only cite tokens that already appear in the admitted evidence.
#[derive(Deserialize, JsonSchema)]
pub struct OntoExecutiveProjectionInput {
    pub scope_token: String,
    /// Pre-rendered evidence text (assembled by the caller from
    /// admitted OCEL events / receipts). The translator's summary
    /// must be a token-overlap subset of this.
    pub admitted_evidence: String,
}

/// Run a single old-AI cognition breed (ELIZA / CBR / DENDRAL / STRIPS
/// / Prolog / MYCIN / GPS / SOAR / Hearsay) against the supplied
/// `BreedInput` JSON. Read-only / allowlisted — breeds are pure
/// functions over inputs.
#[derive(Deserialize, JsonSchema)]
pub struct OntoOldAiStationInput {
    /// Breed name: one of `eliza`, `cbr`, `dendral`, `strips`, `prolog`,
    /// `mycin`, `gps`, `soar`, `hearsay` (case-insensitive).
    pub breed: String,
    /// `wasm4pm_cognition::breeds::BreedInput` JSON. Must contain at
    /// minimum the `intent` field; other fields default to empty
    /// vectors. Optional `scope_token` field on the wrapper is honoured
    /// when emitting the OCEL trace step.
    pub input_json: String,
    /// Optional scope token for OCEL trace emission. The breed step is
    /// recorded as an `old_ai_station` event with the breed name and
    /// trace step count.
    pub scope_token: Option<String>,
}

