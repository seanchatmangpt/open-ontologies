use schemars::JsonSchema;
use serde::Deserialize;

// ─── MCP tool input structs ─────────────────────────────────────────────────

/// Input for [`onto_validate`](https://docs.rs/open-ontologies).
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoValidateInput;
/// let inp = OntoValidateInput {
///     input: "ontology/pizza.ttl".to_string(),
///     inline: None,
/// };
/// assert_eq!(inp.input, "ontology/pizza.ttl");
/// assert!(inp.inline.is_none());
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoValidateInput {
    /// Path to an RDF file OR inline Turtle content
    pub input: String,
    /// If true, treat input as inline content rather than a file path
    pub inline: Option<bool>,
}

/// Input for format conversion between RDF serializations.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoConvertInput;
/// let inp = OntoConvertInput {
///     path: "ontology/pizza.ttl".to_string(),
///     to: "ntriples".to_string(),
///     output: None,
/// };
/// assert_eq!(inp.to, "ntriples");
/// assert!(inp.output.is_none());
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoConvertInput {
    /// Path to source RDF file
    pub path: String,
    /// Target format: turtle, ntriples, rdfxml, nquads, trig
    pub to: String,
    /// Optional output file path (if omitted, returns content)
    pub output: Option<String>,
}

/// Input for loading an ontology into the in-memory store.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoLoadInput;
/// let inp = OntoLoadInput {
///     path: Some("ontology/pizza.ttl".to_string()),
///     turtle: None,
///     name: None,
///     auto_refresh: None,
///     force_recompile: None,
/// };
/// assert!(inp.path.is_some());
/// assert!(inp.turtle.is_none());
/// assert!(inp.name.is_none());
/// assert!(inp.auto_refresh.is_none());
/// assert!(inp.force_recompile.is_none());
/// ```
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

/// Input for removing an ontology from the in-memory store.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoUnloadInput;
/// let inp = OntoUnloadInput { delete_cache: Some(false), name: None };
/// assert_eq!(inp.delete_cache, Some(false));
/// assert!(inp.name.is_none());
/// ```
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

/// Input for forcing a recompile of an ontology from its source file.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoRecompileInput;
/// let inp = OntoRecompileInput { name: Some("pizza".to_string()) };
/// assert_eq!(inp.name.as_deref(), Some("pizza"));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoRecompileInput {
    /// Optional ontology name. When omitted, recompiles the active ontology.
    /// When provided, recompiles that cached entry from its recorded source
    /// path; if the entry is not active, the active in-memory store is left
    /// untouched.
    pub name: Option<String>,
}

/// Input for listing ontology files in configured repository directories.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoRepoListInput;
/// let inp = OntoRepoListInput {
///     dir: None,
///     recursive: Some(true),
///     glob: Some("*.ttl".to_string()),
///     limit: Some(50),
///     offset: None,
/// };
/// assert_eq!(inp.recursive, Some(true));
/// assert_eq!(inp.glob.as_deref(), Some("*.ttl"));
/// assert_eq!(inp.limit, Some(50));
/// assert!(inp.offset.is_none());
/// ```
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

/// Input for inspecting the compile cache state (no parameters needed).
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoCacheStatusInput;
/// let inp = OntoCacheStatusInput {};
/// // Unit struct — construction always succeeds.
/// drop(inp);
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoCacheStatusInput {}

/// Input for listing all cached ontologies (no parameters needed).
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoCacheListInput;
/// let inp = OntoCacheListInput {};
/// drop(inp);
/// ```
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

/// Input for executing a SPARQL query against the loaded ontology.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoQueryInput;
/// let inp = OntoQueryInput {
///     query: "SELECT ?s WHERE { ?s a owl:Class }".to_string(),
/// };
/// assert!(inp.query.contains("owl:Class"));
/// ```
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

/// Input for generating a mapping config from a data schema and the loaded ontology.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoMapInput;
/// let inp = OntoMapInput {
///     data_path: "data/customers.csv".to_string(),
///     format: Some("csv".to_string()),
///     save_path: None,
/// };
/// assert_eq!(inp.format.as_deref(), Some("csv"));
/// assert!(inp.save_path.is_none());
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoMapInput {
    /// Path to sample data file to generate mapping for
    pub data_path: String,
    /// Data format (auto-detected if omitted)
    pub format: Option<String>,
    /// Optional path to save the generated mapping config
    pub save_path: Option<String>,
}

/// Input for validating loaded data against SHACL constraints.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoShaclInput;
/// let inp = OntoShaclInput {
///     shapes: "ontology/cell8-shapes.ttl".to_string(),
///     inline: Some(false),
/// };
/// assert_eq!(inp.inline, Some(false));
/// assert!(inp.shapes.ends_with(".ttl"));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoShaclInput {
    /// Path to SHACL shapes file OR inline SHACL Turtle content
    pub shapes: String,
    /// If true, treat shapes as inline Turtle content
    pub inline: Option<bool>,
}

/// Input for running RDFS or OWL-RL inference over the loaded ontology.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoReasonInput;
/// let inp = OntoReasonInput {
///     profile: Some("rdfs".to_string()),
///     materialize: Some(true),
/// };
/// assert_eq!(inp.profile.as_deref(), Some("rdfs"));
/// assert_eq!(inp.materialize, Some(true));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoReasonInput {
    /// Reasoning profile: rdfs (default), owl-rl
    pub profile: Option<String>,
    /// If true (default), add inferred triples to the store. If false, dry-run only.
    pub materialize: Option<bool>,
}

/// Input for explaining why a class is unsatisfiable using DL tableaux reasoning.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoDlExplainInput;
/// let inp = OntoDlExplainInput {
///     class_iri: "http://example.org/UnsatisfiableClass".to_string(),
/// };
/// assert!(inp.class_iri.starts_with("http://"));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoDlExplainInput {
    /// IRI of the class to explain unsatisfiability for
    pub class_iri: String,
}

/// Input for checking subsumption between two classes via DL tableaux reasoning.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoDlCheckInput;
/// let inp = OntoDlCheckInput {
///     sub_class: "http://example.org/Car".to_string(),
///     super_class: "http://example.org/Vehicle".to_string(),
/// };
/// assert_ne!(inp.sub_class, inp.super_class);
/// ```
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

/// Input for applying ontology changes in safe or migrate mode.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoApplyInput;
/// let inp = OntoApplyInput {
///     mode: Some("safe".to_string()),
///     scope_token: None,
///     bypass_admission: None,
///     bypass_reason: None,
/// };
/// assert_eq!(inp.mode.as_deref(), Some("safe"));
/// assert!(inp.scope_token.is_none());
/// ```
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

/// Input for retrieving the session lineage trail.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoLineageInput;
/// let inp = OntoLineageInput {
///     session_id: None,
///     format: Some("ocel".to_string()),
/// };
/// assert!(inp.session_id.is_none());
/// assert_eq!(inp.format.as_deref(), Some("ocel"));
/// ```
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
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCloseWorkflowInput {
    /// The scope token returned by `onto_declare_workflow`.
    pub scope_token: String,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
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

/// Input for detecting alignment candidates between two ontologies.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoAlignInput;
/// let inp = OntoAlignInput {
///     source: "ontology/source.ttl".to_string(),
///     target: Some("ontology/target.ttl".to_string()),
///     min_confidence: Some(0.85),
///     dry_run: Some(true),
///     scope_token: None,
///     bypass_admission: None,
///     bypass_reason: None,
/// };
/// assert_eq!(inp.min_confidence, Some(0.85));
/// assert_eq!(inp.dry_run, Some(true));
/// ```
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

/// Input for providing feedback on an alignment candidate to tune confidence weights.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoAlignFeedbackInput;
/// let inp = OntoAlignFeedbackInput {
///     source_iri: "http://example.org/Vehicle".to_string(),
///     target_iri: "http://schema.org/Vehicle".to_string(),
///     accepted: true,
///     signals: None,
/// };
/// assert!(inp.accepted);
/// assert!(inp.signals.is_none());
/// ```
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

/// Input for accepting or dismissing a lint issue to teach the linter.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoLintFeedbackInput;
/// let inp = OntoLintFeedbackInput {
///     rule_id: "missing_label".to_string(),
///     entity: "http://example.org/MyClass".to_string(),
///     accepted: false,
/// };
/// assert_eq!(inp.rule_id, "missing_label");
/// assert!(!inp.accepted);
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoLintFeedbackInput {
    /// The lint rule ID (e.g. "missing_label", "missing_comment", "missing_domain", "missing_range")
    pub rule_id: String,
    /// The entity IRI that triggered the lint issue
    pub entity: String,
    /// true = this is a real issue, false = dismiss/ignore
    pub accepted: bool,
}

/// Input for accepting or dismissing an enforce violation to teach the enforcer.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoEnforceFeedbackInput;
/// let inp = OntoEnforceFeedbackInput {
///     rule_id: "orphan_class".to_string(),
///     entity: "http://example.org/OrphanClass".to_string(),
///     accepted: true,
/// };
/// assert_eq!(inp.rule_id, "orphan_class");
/// assert!(inp.accepted);
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoEnforceFeedbackInput {
    /// The enforce rule ID (e.g. "orphan_class", "missing_domain", "missing_range", "missing_label", or custom rule ID)
    pub rule_id: String,
    /// The entity IRI that triggered the violation
    pub entity: String,
    /// true = this is a real violation, false = dismiss/override
    pub accepted: bool,
}

/// Input for generating text + Poincaré structural embeddings for all classes.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoEmbedInput;
/// let inp = OntoEmbedInput {
///     struct_dim: Some(64),
///     struct_epochs: Some(200),
/// };
/// assert_eq!(inp.struct_dim, Some(64));
/// assert_eq!(inp.struct_epochs, Some(200));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoEmbedInput {
    /// Structural embedding dimension. Default: 32
    pub struct_dim: Option<usize>,
    /// Structural training epochs. Default: 100
    pub struct_epochs: Option<usize>,
}

/// Input for finding ontology classes by natural language description.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoSearchInput;
/// let inp = OntoSearchInput {
///     query: "a vehicle with four wheels".to_string(),
///     top_k: Some(5),
///     mode: Some("text".to_string()),
///     alpha: Some(0.7),
/// };
/// assert_eq!(inp.top_k, Some(5));
/// assert_eq!(inp.mode.as_deref(), Some("text"));
/// assert!((inp.alpha.unwrap() - 0.7_f32).abs() < 1e-6);
/// ```
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

/// Input for computing embedding similarity between two class IRIs.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoSimilarityInput;
/// let inp = OntoSimilarityInput {
///     iri_a: "http://example.org/Vehicle".to_string(),
///     iri_b: "http://example.org/Automobile".to_string(),
/// };
/// assert!(inp.iri_a.starts_with("http://"));
/// assert!(inp.iri_b.starts_with("http://"));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoSimilarityInput {
    /// First IRI
    pub iri_a: String,
    /// Second IRI
    pub iri_b: String,
}

/// Input for browsing or installing ontologies from the curated catalogue.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoMarketplaceInput;
/// let inp = OntoMarketplaceInput {
///     action: "list".to_string(),
///     id: None,
///     domain: Some("foundational".to_string()),
/// };
/// assert_eq!(inp.action, "list");
/// assert_eq!(inp.domain.as_deref(), Some("foundational"));
/// ```
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

/// Input for building a new ontology from a domain description.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::BuildOntologyInput;
/// let inp = BuildOntologyInput {
///     domain: "A pizza ontology with toppings, bases, and named pizzas".to_string(),
/// };
/// assert!(inp.domain.contains("pizza"));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct BuildOntologyInput {
    /// Description of the domain to model (e.g. "A pizza ontology with toppings, bases, and named pizzas")
    pub domain: String,
}

/// Input for validating an ontology file against SHACL shapes.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::ValidateOntologyInput;
/// let inp = ValidateOntologyInput {
///     path: "ontology/my-ontology.ttl".to_string(),
/// };
/// assert!(inp.path.ends_with(".ttl"));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct ValidateOntologyInput {
    /// Path to the ontology file to validate
    pub path: String,
}

/// Input for comparing two ontology versions.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::CompareOntologiesInput;
/// let inp = CompareOntologiesInput {
///     old_path: "ontology/v1.ttl".to_string(),
///     new_path: "ontology/v2.ttl".to_string(),
/// };
/// assert_ne!(inp.old_path, inp.new_path);
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct CompareOntologiesInput {
    /// Path to the old/original ontology file
    pub old_path: String,
    /// Path to the new/modified ontology file
    pub new_path: String,
}

/// Input for ingesting structured data into the RDF store.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::IngestDataInput;
/// let inp = IngestDataInput {
///     data_path: "data/patients.csv".to_string(),
/// };
/// assert!(inp.data_path.ends_with(".csv"));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct IngestDataInput {
    /// Path to the data file (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet)
    pub data_path: String,
}

/// Input for aligning two ontologies using weighted similarity signals.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::AlignOntologiesInput;
/// let inp = AlignOntologiesInput {
///     source_path: "ontology/source.ttl".to_string(),
///     target_path: "ontology/target.ttl".to_string(),
/// };
/// assert_ne!(inp.source_path, inp.target_path);
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct AlignOntologiesInput {
    /// Path to the source ontology file
    pub source_path: String,
    /// Path to the target ontology file
    pub target_path: String,
}

// ─── Process Mining / WvdA Agent ────────────────────────────────────────────

/// Input for validating a process-mining claim against OTel traces.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoProcessValidateClaimInput;
/// let inp = OntoProcessValidateClaimInput {
///     claim: "manufacturing pipeline executed lawfully".to_string(),
///     artifact_id: None,
///     time_range_hours: Some(48),
/// };
/// assert_eq!(inp.time_range_hours, Some(48));
/// assert!(inp.artifact_id.is_none());
/// ```
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

/// Input for solving a problem via the MuStar multi-step reasoning algorithm.
/// `OntoAlphastarSolveInput` is a type alias for this struct.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoMustarSolveInput;
/// let inp = OntoMustarSolveInput {
///     problem_statement: "Sort a list of integers in O(n log n)".to_string(),
///     domain: Some("ALGORITHM".to_string()),
///     constraints: Some("must be in-place".to_string()),
///     title: None,
/// };
/// assert_eq!(inp.domain.as_deref(), Some("ALGORITHM"));
/// assert!(inp.title.is_none());
/// ```
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

/// Input for dry-running the admission gate for an operation.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoAdmissionCheckInput;
/// let inp = OntoAdmissionCheckInput {
///     op: "apply".to_string(),
///     scope_token: Some("scope-001".to_string()),
/// };
/// assert_eq!(inp.op, "apply");
/// assert!(inp.scope_token.is_some());
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoAdmissionCheckInput {
    /// Operation to dry-run admission for: "apply", "codegen", "save", "push".
    pub op: String,
    /// Optional explicit scope token; falls back to the latest open scope for
    /// the session.
    pub scope_token: Option<String>,
}

/// Input for resetting a session by clearing its revoked-sessions row.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoSessionResetInput;
/// let inp = OntoSessionResetInput {
///     session_id: "sess-abc-123".to_string(),
/// };
/// assert!(!inp.session_id.is_empty());
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoSessionResetInput {
    /// Session id whose `revoked_sessions` row should be cleared.
    pub session_id: String,
}

/// Input for running Cell8 A1-A13 conformance attestation on a scope's receipt.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoCell8AttestInput;
/// let inp = OntoCell8AttestInput {
///     scope_token: "scope-cell8-attest-001".to_string(),
/// };
/// assert!(inp.scope_token.starts_with("scope-"));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoCell8AttestInput {
    /// Scope token whose latest receipt (or current admission dry-run state)
    /// should be attested via the Cell8 13-gate EARL report. Read-only:
    /// emits no OCEL events, performs no mutation, returns the EARL Turtle
    /// plus pass/fail counts and any typed defect classes observed.
    pub scope_token: String,
}

/// Input for generating code artifacts from the loaded ontology via ggen.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoCodegenInput;
/// let inp = OntoCodegenInput {
///     generator: "rust-structs".to_string(),
///     output_dir: Some("./generated".to_string()),
///     dry_run: Some(true),
///     manifest_path: None,
///     queries_dir: None,
///     scope_token: None,
///     bypass_admission: None,
///     bypass_reason: None,
/// };
/// assert_eq!(inp.generator, "rust-structs");
/// assert_eq!(inp.dry_run, Some(true));
/// ```
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

/// Input for retrieving warm-start planner exemplars for a domain.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoPlannerDemosInput;
/// let inp = OntoPlannerDemosInput {
///     domain: "OntologyAuthoring".to_string(),
///     min_fitness: Some(0.95),
///     limit: Some(10),
/// };
/// assert_eq!(inp.domain, "OntologyAuthoring");
/// assert_eq!(inp.min_fitness, Some(0.95));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoPlannerDemosInput {
    /// Domain key (workflow class name, e.g. "OntologyAuthoring").
    pub domain: String,
    /// Minimum fitness floor for returned exemplars. Default 0.95.
    pub min_fitness: Option<f64>,
    /// Maximum number of exemplars. Default 10.
    pub limit: Option<usize>,
}

/// Input for discovering the actual process from event logs for a domain.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoWorkflowDiscoverInput;
/// let inp = OntoWorkflowDiscoverInput {
///     domain: "DataExtension".to_string(),
/// };
/// assert_eq!(inp.domain, "DataExtension");
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoWorkflowDiscoverInput {
    /// Domain key (workflow class name) to run discovery for.
    pub domain: String,
}

/// Input for providing feedback on a discovered workflow variant.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoWorkflowFeedbackInput;
/// let inp = OntoWorkflowFeedbackInput {
///     id: "wf-42".to_string(),
///     accepted: true,
/// };
/// assert_eq!(inp.id, "wf-42");
/// assert!(inp.accepted);
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoWorkflowFeedbackInput {
    /// `discovered_workflows.id` row to flip.
    pub id: String,
    /// true = mark accepted, false = mark rejected.
    pub accepted: bool,
}

/// Prompt input for generating code from the loaded ontology.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::GenerateCodeInput;
/// let inp = GenerateCodeInput {
///     language: Some("Rust".to_string()),
///     generator: Some("rust-structs".to_string()),
///     output_dir: Some("./generated".to_string()),
/// };
/// assert_eq!(inp.language.as_deref(), Some("Rust"));
/// assert_eq!(inp.generator.as_deref(), Some("rust-structs"));
/// ```
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

/// Input for planning a workflow from a natural-language problem statement.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoPlanWorkflowInput;
/// let inp = OntoPlanWorkflowInput {
///     problem_statement: "Build a pizza ontology with toppings and bases".to_string(),
///     domain: "ONTOLOGY".to_string(),
///     constraints: None,
///     python: None,
///     planner_script: None,
///     engine: Some("mustar".to_string()),
///     bypass_admission: None,
///     bypass_reason: None,
/// };
/// assert_eq!(inp.domain, "ONTOLOGY");
/// assert_eq!(inp.engine.as_deref(), Some("mustar"));
/// ```
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
    /// Planner engine: "mustar" (default, MuStar+PowlPredictor subprocess)
    /// or "groq_powl" (real Groq via pm4py.algo.dspy.powl through
    /// `scripts/powl_from_text.py`). Unknown values are treated as "mustar".
    pub engine: Option<String>,
    /// Bypass admission gate. Requires `bypass_reason`. Revokes the session.
    pub bypass_admission: Option<bool>,
    /// Required when `bypass_admission` is true.
    pub bypass_reason: Option<String>,
}

/// Input for seeding exemplar demonstrations from an OCEL JSON file.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoExemplarSeedInput;
/// let inp = OntoExemplarSeedInput {
///     path: None,
///     domain: Some("ONTOLOGY".to_string()),
/// };
/// assert!(inp.path.is_none());
/// assert_eq!(inp.domain.as_deref(), Some("ONTOLOGY"));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoExemplarSeedInput {
    /// Path to the seed OCEL JSON file. Default:
    /// `~/chatmangpt/ostar/artifacts/ocel/mu_star/ONTOLOGY.oceljson`.
    pub path: Option<String>,
    /// Domain to assign to seeded exemplars when the OCEL event omits it.
    pub domain: Option<String>,
}

/// Input for generating counterfactual explanations for a workflow scope.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoCounterfactualInput;
/// let inp = OntoCounterfactualInput {
///     scope_token: "scope-counterfactual-007".to_string(),
/// };
/// assert!(!inp.scope_token.is_empty());
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoCounterfactualInput {
    /// Scope token returned by `onto_declare_workflow` / `onto_plan_workflow`.
    pub scope_token: String,
}

// ── Requirements-Andon / CTQ-Forge inputs (Phase 1.5) ────────────────────

/// Capture a source-voice signal and propose a requirement. The
/// admission gate denies with `RequirementWithoutSource` if
/// `source_voice` is empty.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoProposeRequirementInput;
/// let inp = OntoProposeRequirementInput {
///     source_voice: "Customer reports checkout latency exceeds 3 seconds".to_string(),
///     voice_kind: Some("customer".to_string()),
///     scope_token: None,
///     bypass_admission: None,
///     bypass_reason: None,
/// };
/// assert!(!inp.source_voice.is_empty());
/// assert_eq!(inp.voice_kind.as_deref(), Some("customer"));
/// assert!(inp.scope_token.is_none());
/// ```
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
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoTranslateCandidateInput;
/// let inp = OntoTranslateCandidateInput {
///     scope_token: "scope-abc-123".to_string(),
///     source_voice: "Customer complains latency exceeds 200ms".to_string(),
///     engine: Some("inproc".to_string()),
///     python: None,
/// };
/// assert_eq!(inp.engine.as_deref(), Some("inproc"));
/// assert!(inp.python.is_none());
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoTranslateCandidateInput {
    /// Scope token returned by `onto_propose_requirement`.
    pub scope_token: String,
    /// Source-voice signal to translate (re-echoed for self-contained
    /// audit; must match the value provided to `onto_propose_requirement`).
    pub source_voice: String,
    /// Translation engine. `"inproc"` (default) — drives the in-process
    /// `GroqTranslator` shaped-signature path. `"groq_pm4py"` — shells
    /// out to `scripts/ctq_from_voice.py`, the same pm4py/dspy subprocess
    /// proven against real Groq in `tests/real_groq_ctq.rs`. `"gemini"` —
    /// headless Gemini CLI via OAuth (`gemini -p … --approval-mode yolo`);
    /// no API key required, binary resolved via `GEMINI_BIN` env or `"gemini"`.
    /// Unknown values are treated as `"inproc"`.
    pub engine: Option<String>,
    /// Override the python interpreter used by the `groq_pm4py` engine
    /// (default: `"python3"`). Ignored by the in-process engine.
    pub python: Option<String>,
}

/// Admit a CTQ. The deterministic gate denies with
/// `CtqIncomplete{missing}` if any of the 5 mandatory fields are
/// empty / whitespace.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoAdmitCtqInput;
/// let inp = OntoAdmitCtqInput {
///     scope_token: "scope-xyz".to_string(),
///     source_voice: "Customer: response time > 500ms is unacceptable".to_string(),
///     ctq_text: "API p99 latency < 200ms".to_string(),
///     measure_text: "p99 response time from load balancer logs".to_string(),
///     verification_text: "k6 load test at 1000 RPS".to_string(),
///     negative_case_text: "p99 > 200ms under normal load".to_string(),
///     control_plan_text: "Alert on SLO breach; auto-scale triggered".to_string(),
///     bypass_admission: None,
///     bypass_reason: None,
/// };
/// assert_eq!(inp.ctq_text, "API p99 latency < 200ms");
/// assert!(inp.bypass_admission.is_none());
/// ```
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
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoProposeWorkOrderInput;
/// let inp = OntoProposeWorkOrderInput {
///     scope_token: "scope-wo-001".to_string(),
///     ctq_receipt_hash: "deadbeef".to_string(),
///     naked_craft_path: "prompt → code (no validation)".to_string(),
///     manufacturing_path: "CTQ → admit → manufacture → attest".to_string(),
///     counterfactual_delta: "Without admission: silent data corruption possible".to_string(),
/// };
/// assert_eq!(inp.scope_token, "scope-wo-001");
/// assert!(!inp.counterfactual_delta.is_empty());
/// ```
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
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoAdmitWorkOrderInput;
/// let inp = OntoAdmitWorkOrderInput {
///     scope_token: "scope-admit-002".to_string(),
///     ctq_receipt_hash: "cafebabe".to_string(),
///     naked_craft_path: "raw LLM generation".to_string(),
///     manufacturing_path: "CTQ-gated manufacture".to_string(),
///     counterfactual_delta: "admission prevents unvalidated artifact release".to_string(),
///     bypass_admission: None,
///     bypass_reason: None,
/// };
/// assert_eq!(inp.scope_token, "scope-admit-002");
/// assert!(inp.bypass_admission.is_none());
/// assert!(!inp.counterfactual_delta.is_empty());
/// ```
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
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoExecutiveProjectionInput;
/// let inp = OntoExecutiveProjectionInput {
///     scope_token: "scope-exec-001".to_string(),
///     admitted_evidence: "Receipt R1 admits CTQ: latency < 200ms".to_string(),
///     engine: Some("inproc".to_string()),
///     python: None,
/// };
/// assert!(!inp.admitted_evidence.is_empty());
/// assert_eq!(inp.engine.as_deref(), Some("inproc"));
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoExecutiveProjectionInput {
    pub scope_token: String,
    /// Pre-rendered evidence text (assembled by the caller from
    /// admitted OCEL events / receipts). The translator's summary
    /// must be a token-overlap subset of this.
    pub admitted_evidence: String,
    /// Projection engine. `"inproc"` (default) — uses the in-process
    /// `GroqTranslator`. `"groq_pm4py"` — shells out to
    /// `scripts/executive_projection.py`. `"gemini"` — headless Gemini CLI
    /// via OAuth (`gemini -p … --approval-mode yolo`); no API key required.
    /// Unknown values treated as `"inproc"`.
    pub engine: Option<String>,
    /// Override the python interpreter used by the `groq_pm4py` engine
    /// (default: `"python3"`).
    pub python: Option<String>,
}

/// Read-only liveness probe for the real-Groq subprocess engine.
/// Invokes a tiny Python harness that imports `dspy` and inspects the
/// `GROQ_API_KEY` env var. Never logs the key.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoGroqStatusInput;
/// let inp = OntoGroqStatusInput::default();
/// assert!(inp.python.is_none());
/// ```
#[derive(Deserialize, JsonSchema, Default)]
pub struct OntoGroqStatusInput {
    /// Override the python interpreter (default: `"python3"`).
    pub python: Option<String>,
}

/// Read-only liveness probe for the Gemini CLI engine.
/// Checks binary availability (`--version`) and OAuth session validity
/// (`gemini -p ping … --approval-mode yolo`). No API key required.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoGeminiStatusInput;
/// let inp = OntoGeminiStatusInput::default();
/// assert!(inp.gemini_bin.is_none());
/// ```
///
/// # Auto-instinct: `gemini_bin` override is stored verbatim — no path normalization
///
/// When supplied, the binary path is forwarded as-is to the subprocess.
/// `None` signals that the handler should fall back to the `GEMINI_BIN`
/// environment variable or the bare `"gemini"` name on `$PATH`.
///
/// ```
/// use open_ontologies::inputs::OntoGeminiStatusInput;
/// let inp = OntoGeminiStatusInput {
///     gemini_bin: Some("/usr/local/bin/gemini-cli".to_string()),
/// };
/// assert_eq!(inp.gemini_bin.as_deref(), Some("/usr/local/bin/gemini-cli"));
/// // Field is Option<String>; None means "use default resolution".
/// assert!(OntoGeminiStatusInput::default().gemini_bin.is_none());
/// ```
#[derive(Deserialize, JsonSchema, Default)]
pub struct OntoGeminiStatusInput {
    /// Override the Gemini binary path (default: `GEMINI_BIN` env or `"gemini"`).
    pub gemini_bin: Option<String>,
}

/// Manufacture a complete multi-target solution stack (IaC + Rust +
/// Erlang + AtomVM) from a SolutionSpec. Full admission. The gate
/// denies with `ArchitectureUnbound` when the work-order receipt is
/// not a valid 64-char hex; with `IacInvalid` / `RustInvalid` /
/// `ErlangInvalid` / `AtomVmInvalid` when a target generator fails;
/// with `ManufacturingChainBroken` when receipt headers are missing.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoManufactureSolutionInput;
/// let inp = OntoManufactureSolutionInput {
///     scope_token: "scope-mfg-001".to_string(),
///     name: "latency_reducer".to_string(),
///     description: "Reduce p99 API latency below 200ms".to_string(),
///     iac_target: "aws".to_string(),
///     region: "us-east-1".to_string(),
///     supervisor_children: 4,
///     mcu_target: "esp32".to_string(),
///     work_order_receipt_hash: "a".repeat(64),
///     output_dir: None,
///     bypass_admission: None,
///     bypass_reason: None,
/// };
/// assert_eq!(inp.iac_target, "aws");
/// assert_eq!(inp.supervisor_children, 4);
/// assert_eq!(inp.work_order_receipt_hash.len(), 64);
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoManufactureSolutionInput {
    pub scope_token: String,
    /// Solution name. Must match `[a-z][a-z0-9_]*`.
    pub name: String,
    pub description: String,
    /// Cloud target for IaC. Currently `"aws"` only.
    pub iac_target: String,
    pub region: String,
    /// Number of supervisor children for the Erlang/OTP tree (1..=64).
    pub supervisor_children: u32,
    /// MCU for AtomVM. One of `"esp32"`, `"stm32"`, `"rp2040"`.
    pub mcu_target: String,
    /// 64-char hex BLAKE3 of the upstream WorkOrderAdmitted receipt.
    pub work_order_receipt_hash: String,
    /// Optional: write the manufactured bundle to this directory. When
    /// absent, the bundle is returned in the JSON response only.
    pub output_dir: Option<String>,
    pub bypass_admission: Option<bool>,
    pub bypass_reason: Option<String>,
}

/// Run a single old-AI cognition breed (ELIZA / CBR / DENDRAL / STRIPS
/// / Prolog / MYCIN / GPS / SOAR / Hearsay) against the supplied
/// `BreedInput` JSON. Read-only / allowlisted — breeds are pure
/// functions over inputs.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoOldAiStationInput;
/// let inp = OntoOldAiStationInput {
///     breed: "eliza".to_string(),
///     input_json: r#"{"intent": "sort a list"}"#.to_string(),
///     scope_token: None,
/// };
/// assert_eq!(inp.breed, "eliza");
/// assert!(inp.scope_token.is_none());
/// ```
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

// ─── R5 WC-2 — admin-only operational tool inputs ──────────────────────

/// `onto_receipts_revoke_batch` input. Soft-deletes (UPDATE
/// `production_law_version`) every receipt whose `scope_token` matches
/// `scope_token_pattern` and whose `production_law_version` is not the
/// `seed-v0` sentinel. The chain is preserved for audit (no row is
/// physically removed).
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoReceiptsRevokeBatchInput;
/// let inp = OntoReceiptsRevokeBatchInput {
///     scope_token_pattern: "scope-prod-*".to_string(),
///     reason: "emergency rollback after incident INC-4821".to_string(),
/// };
/// assert!(inp.scope_token_pattern.contains('*'));
/// assert!(!inp.reason.is_empty());
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoReceiptsRevokeBatchInput {
    /// SQLite GLOB pattern matched against `receipts.scope_token`.
    /// Examples: `"scope-prod-*"`, `"tenant:foo:*"`. Pure GLOB syntax
    /// (`*`, `?`, `[abc]`); does NOT use SQL `LIKE` placeholders.
    pub scope_token_pattern: String,
    /// Operator-supplied free-text reason. Recorded verbatim in the
    /// `receipts_revoke_batch` OCEL audit event so an external auditor
    /// can correlate the bulk action with a justification.
    pub reason: String,
}

/// `onto_session_revoke_by_principal` input. Forcefully revokes all
/// active sessions owned by a principal in a tenant. Falls back to
/// bulk-INSERT into `revoked_sessions` until R3 Task B's
/// `revoked_principals` table lands.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoSessionRevokeByPrincipalInput;
/// let inp = OntoSessionRevokeByPrincipalInput {
///     tenant_id: "tenant-acme".to_string(),
///     principal_id: "tenant-acme".to_string(),
///     reason: "key compromise detected".to_string(),
/// };
/// assert_eq!(inp.tenant_id, inp.principal_id);
/// assert!(!inp.reason.is_empty());
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoSessionRevokeByPrincipalInput {
    /// Tenant the principal belongs to. Must match the `tenant_id`
    /// column on `declared_workflows` for the affected workflows.
    pub tenant_id: String,
    /// Principal identifier (typically the same as `tenant_id` until
    /// R3 Task B's principal helper lands). Currently an alias.
    pub principal_id: String,
    /// Operator-supplied free-text reason. Recorded as
    /// `revoked_sessions.reason` for every inserted row and in the
    /// `session_revoke` OCEL audit event.
    pub reason: String,
}

/// `onto_retention_pause` input. Suspends the [`crate::retention::RetentionWorker`]
/// for `minutes` minutes via a shared `Arc<AtomicI64>`.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoRetentionPauseInput;
/// let inp = OntoRetentionPauseInput { minutes: 30 };
/// assert_eq!(inp.minutes, 30);
/// assert!(inp.minutes <= 60 * 24 * 7, "bounded to one week");
/// ```
///
/// # Auto-instinct: one-week ceiling must be validated by the caller
///
/// The struct is a plain `u64` carrier — the handler enforces the upper bound
/// at invocation time. Construction with any value succeeds.
///
/// ```
/// use open_ontologies::inputs::OntoRetentionPauseInput;
/// const ONE_WEEK_MINUTES: u64 = 60 * 24 * 7; // 10 080
///
/// // At-ceiling value is a valid pause duration.
/// let at_limit = OntoRetentionPauseInput { minutes: ONE_WEEK_MINUTES };
/// assert_eq!(at_limit.minutes, 10_080);
///
/// // Zero is constructible but semantically a no-op pause.
/// let noop = OntoRetentionPauseInput { minutes: 0 };
/// assert_eq!(noop.minutes, 0);
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoRetentionPauseInput {
    /// Number of minutes to suspend retention pruning. The pause
    /// expires at `now() + minutes * 60`. Bounded to `60 * 24 * 7`
    /// (one week) to prevent an indefinite forget; longer durations
    /// require multiple calls.
    pub minutes: u64,
}

/// `onto_guide` input. Accepts a plain-language intent and returns a
/// step-by-step tool plan for the matching builtin workflow.
#[derive(Deserialize, JsonSchema)]
pub struct OntoGuideInput {
    /// Plain-language description of what you want to accomplish.
    /// Examples: "load and validate an ontology", "ingest CSV data",
    /// "align two ontologies", "manufacture a solution".
    pub intent: String,
    /// When true, include the POWL string for the matched workflow in the
    /// response (useful for onto_declare_workflow). Default: false.
    pub include_powl: Option<bool>,
}

/// R10-2: `onto_ontostar_attest` input. Verifies an external OntoStar
/// Ed25519 receipt and seals the key fingerprint into `trusted_keys_history`.
///
/// # Example
///
/// ```
/// use open_ontologies::inputs::OntoOntostarAttestInput;
/// let inp = OntoOntostarAttestInput {
///     signature: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
///     payload_hash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
///     key_fpr: "0102030405060708".to_string(),
/// };
/// assert_eq!(inp.key_fpr.len(), 16);
/// assert_eq!(inp.payload_hash.len(), 64);
/// ```
///
/// # Auto-instinct: `key_fpr` encodes 8 bytes as exactly 16 lowercase hex chars
///
/// The handler rejects any `key_fpr` that is not exactly 16 characters long.
/// Callers must zero-pad short fingerprints before submitting.
///
/// ```
/// use open_ontologies::inputs::OntoOntostarAttestInput;
///
/// // 16-char hex string = 8 bytes — the canonical fingerprint width.
/// let fpr = "deadbeefcafebabe";
/// assert_eq!(fpr.len(), 16);
/// assert!(fpr.chars().all(|c| c.is_ascii_hexdigit()));
///
/// // payload_hash must be 64 hex chars (BLAKE3 / SHA-256 output length).
/// let hash = "a".repeat(64);
/// let inp = OntoOntostarAttestInput {
///     signature: "sig".to_string(),
///     payload_hash: hash,
///     key_fpr: fpr.to_string(),
/// };
/// assert_eq!(inp.key_fpr.len(), 16, "key_fpr must be exactly 16 hex chars");
/// assert_eq!(inp.payload_hash.len(), 64, "payload_hash must be 64 hex chars");
/// ```
#[derive(Deserialize, JsonSchema)]
pub struct OntoOntostarAttestInput {
    /// Base64-encoded Ed25519 signature from the external OntoStar receipt.
    pub signature: String,
    /// BLAKE3 hash (hex or UTF-8 string) of the receipt payload being attested.
    pub payload_hash: String,
    /// Key fingerprint hex (exactly 16 hex chars = 8 bytes) identifying the
    /// external signer within the local `TrustedKeys` set.
    pub key_fpr: String,
}

