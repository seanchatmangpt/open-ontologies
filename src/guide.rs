//! Workflow planner for the `onto_guide` MCP tool.
//!
//! Accepts a plain-language intent string and returns a step-by-step plan
//! mapping to the canonical builtin workflow catalog. Intent matching is
//! lowercase substring — no LLM call. Calling an LLM here would constitute
//! `LlmAuthorityClaimed` without CTQ admission, which is a typed defect.

use serde::Serialize;

/// One step in a workflow plan.
#[derive(Debug, Clone, Serialize)]
pub struct PlanStep {
    pub step: u32,
    pub tool: &'static str,
    pub params: serde_json::Value,
    pub reason: &'static str,
    /// When true this step requires a scope_token from onto_declare_workflow.
    pub requires_scope: bool,
}

/// A complete workflow plan returned by `onto_guide`.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowPlan {
    pub ok: bool,
    pub intent: String,
    pub workflow_name: Option<&'static str>,
    pub plan: Vec<PlanStep>,
    pub estimated_steps: usize,
    /// False when any step writes state (onto_save, onto_apply, onto_version).
    pub can_auto_execute: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub powl: Option<&'static str>,
    /// Populated when the intent does not match a known workflow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub known_intents: Option<Vec<&'static str>>,
}

/// Resolve an intent string to a structured workflow plan.
///
/// Intent matching: lowercase substring against each cluster's keyword list.
/// First match wins. Unknown intents return an empty plan with `known_intents`.
///
/// # Examples
///
/// A recognized intent returns a non-empty ordered plan:
/// ```
/// # use open_ontologies::guide::plan_for_intent;
/// let plan = plan_for_intent("load and validate", false);
/// assert!(plan.ok);
/// assert!(!plan.plan.is_empty());
/// assert_eq!(plan.plan[0].tool, "onto_validate");
/// ```
///
/// An unrecognized intent returns the known workflow names for the caller
/// to choose from:
/// ```
/// # use open_ontologies::guide::plan_for_intent;
/// let plan = plan_for_intent("do something unknown", false);
/// assert!(plan.ok);
/// assert!(plan.plan.is_empty());
/// assert!(plan.known_intents.is_some());
/// ```
pub fn plan_for_intent(intent: &str, include_powl: bool) -> WorkflowPlan {
    let lower = intent.to_lowercase();

    for cluster in CLUSTERS {
        if cluster.keywords.iter().any(|k| lower.contains(k)) {
            let steps: Vec<PlanStep> = (cluster.steps)();
            let n = steps.len();
            return WorkflowPlan {
                ok: true,
                intent: intent.to_string(),
                workflow_name: Some(cluster.name),
                estimated_steps: n,
                can_auto_execute: cluster.can_auto_execute,
                powl: if include_powl { cluster.powl } else { None },
                known_intents: None,
                plan: steps,
            };
        }
    }

    // Unknown intent — return the list of known workflow names so the agent
    // can select the right one without reading documentation.
    WorkflowPlan {
        ok: true,
        intent: intent.to_string(),
        workflow_name: None,
        plan: vec![],
        estimated_steps: 0,
        can_auto_execute: true,
        powl: None,
        known_intents: Some(CLUSTERS.iter().map(|c| c.name).collect()),
    }
}

struct Cluster {
    name: &'static str,
    keywords: &'static [&'static str],
    can_auto_execute: bool,
    powl: Option<&'static str>,
    steps: fn() -> Vec<PlanStep>,
}

macro_rules! step {
    ($n:expr, $tool:expr, $params:tt, $reason:expr) => {
        PlanStep {
            step: $n,
            tool: $tool,
            params: serde_json::json!($params),
            reason: $reason,
            requires_scope: false,
        }
    };
    ($n:expr, $tool:expr, $params:tt, $reason:expr, scope) => {
        PlanStep {
            step: $n,
            tool: $tool,
            params: serde_json::json!($params),
            reason: $reason,
            requires_scope: true,
        }
    };
}

static CLUSTERS: &[Cluster] = &[
    // ── 1. Quick load + validate ────────────────────────────────────────────
    Cluster {
        name: "LoadAndValidate",
        keywords: &["load and validate", "validate and load", "load ttl", "load turtle",
                    "load ontology", "quick load", "validate syntax"],
        can_auto_execute: true,
        powl: None,
        steps: || vec![
            step!(1, "onto_validate", {"input": "<path-to.ttl>"}, "validate syntax before loading"),
            step!(2, "onto_load",     {"path": "<path-to.ttl>"}, "load into triple store"),
            step!(3, "onto_stats",    {}, "verify triple count matches expectations"),
        ],
    },

    // ── 2. Full ontology authoring lifecycle ────────────────────────────────
    Cluster {
        name: "OntologyAuthoring",
        keywords: &["ontology authoring", "build ontology", "create ontology",
                    "author ontology", "new ontology", "develop ontology",
                    "full ontology", "ontology lifecycle"],
        can_auto_execute: false,
        powl: Some("SEQ(load,validate,reason,enforce_run,lint,save,version)"),
        steps: || vec![
            step!(1, "onto_validate", {"input": "<path-to.ttl>"}, "validate syntax first"),
            step!(2, "onto_load",     {"path": "<path-to.ttl>"}, "load into triple store"),
            step!(3, "onto_reason",   {"profile": "rdfs"}, "materialize inferred triples"),
            step!(4, "onto_lint",     {"input": "<path-to.ttl>"}, "check missing labels, domains, ranges"),
            step!(5, "onto_enforce",  {"rule_pack": "generic"}, "check design pattern compliance"),
            step!(6, "onto_save",     {"path": "<output.ttl>", "format": "turtle"},
                  "persist validated ontology"),
            step!(7, "onto_version",  {"label": "<version-label>"}, "create named snapshot for rollback"),
        ],
    },

    // ── 3. Data extension / ingest pipeline ─────────────────────────────────
    Cluster {
        name: "DataExtension",
        keywords: &["data extension", "ingest", "csv", "json data", "excel",
                    "parquet", "load data", "import data", "extend with data",
                    "data pipeline"],
        can_auto_execute: false,
        powl: Some("SEQ(map,ingest,shacl,reason,query)"),
        steps: || vec![
            step!(1, "onto_map",    {"data_path": "<path-to-data.csv>"},
                  "inspect data and generate RDF mapping"),
            step!(2, "onto_ingest", {"path": "<path-to-data.csv>"},
                  "generate RDF triples from data"),
            step!(3, "onto_stats",  {}, "verify triple count after ingest"),
            step!(4, "onto_shacl",  {"shapes_turtle": "<shapes.ttl>"},
                  "validate ingested data against SHACL constraints"),
            step!(5, "onto_reason", {"profile": "rdfs"}, "infer additional triples"),
            step!(6, "onto_query",  {"sparql": "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10"},
                  "verify ingested knowledge"),
        ],
    },

    // ── 4. Lifecycle: plan → enforce → apply → monitor ──────────────────────
    Cluster {
        name: "LifecycleApply",
        keywords: &["lifecycle", "apply", "production", "plan and apply",
                    "deploy ontology", "promote", "apply changes"],
        can_auto_execute: false,
        powl: Some("SEQ(plan,enforce_run,apply,monitor,drift)"),
        steps: || vec![
            step!(1, "onto_plan",    {"new_turtle": "<proposed.ttl>"},
                  "preview blast radius and risk score"),
            step!(2, "onto_enforce", {"rule_pack": "generic"},
                  "check design pattern compliance before apply"),
            step!(3, "onto_apply",   {"mode": "safe"},
                  "apply changes (safe = clear+reload with monitor check)"),
            step!(4, "onto_monitor", {},
                  "run SPARQL watchers — alerts trigger notify or auto-rollback"),
            step!(5, "onto_drift",   {"version_a": "<before>", "version_b": "<after>"},
                  "analyze drift velocity and detect renames"),
        ],
    },

    // ── 5. Ontology alignment ────────────────────────────────────────────────
    Cluster {
        name: "Alignment",
        keywords: &["align", "alignment", "match ontologies", "ontology matching",
                    "merge ontologies", "equivalentclass", "similar classes"],
        can_auto_execute: false,
        powl: Some("SEQ(load,embed,align_run)"),
        steps: || vec![
            step!(1, "onto_load",           {"path": "<source.ttl>"},
                  "load source ontology"),
            step!(2, "onto_embed",          {},
                  "generate text + Poincaré structural embeddings for semantic alignment"),
            step!(3, "onto_align",          {"source": "<src.ttl>", "target": "<tgt.ttl>",
                                             "dry_run": true},
                  "detect alignment candidates using 7 weighted signals"),
            step!(4, "onto_align_feedback", {"source_iri": "<iri-a>", "target_iri": "<iri-b>",
                                             "accepted": true},
                  "accept/reject pairs to self-calibrate confidence weights"),
        ],
    },

    // ── 6. Code generation ───────────────────────────────────────────────────
    Cluster {
        name: "Codegen",
        keywords: &["codegen", "code generation", "generate code", "python client",
                    "rust structs", "typescript types", "generate client"],
        can_auto_execute: false,
        powl: Some("SEQ(load,validate,reason,codegen_run)"),
        steps: || vec![
            step!(1, "onto_validate", {"input": "<path.ttl>"}, "validate syntax before generation"),
            step!(2, "onto_load",     {"path": "<path.ttl>"},  "load into triple store"),
            step!(3, "onto_reason",   {"profile": "owl-rl"},
                  "materialize inferred triples (important for codegen accuracy)"),
            step!(4, "onto_codegen",  {"generator": "python-client"},
                  "generate code artifacts — set generator to your target language"),
        ],
    },

    // ── 7. Requirements / CTQ / work-order admission ─────────────────────────
    Cluster {
        name: "RequirementsManufacturing",
        keywords: &["requirements", "ctq", "work order", "requirement admission",
                    "critical to quality", "ctq gate", "admit requirement",
                    "propose requirement"],
        can_auto_execute: false,
        powl: Some("SEQ(declare,propose_req,translate,ctq,work_order,admit_wo,close)"),
        steps: || vec![
            step!(1, "onto_declare_workflow",  {"name": "RequirementsManufacturing",
                                                "description": "CTQ admission workflow"},
                  "open admission scope — required before any CTQ operation"),
            step!(2, "onto_propose_requirement",
                  {"source_voice": "<stakeholder voice>", "scope_token": "<token>"},
                  "capture raw stakeholder requirement", scope),
            step!(3, "onto_translate_candidate",
                  {"scope_token": "<token>", "source_voice": "<voice>"},
                  "LLM translation to POWL candidate (audit-only, non-authoritative)", scope),
            step!(4, "onto_admit_ctq",
                  {"scope_token": "<token>", "measure": "<metric>",
                   "verification": "<method>", "negative_case": "<negative>",
                   "control_plan": "<plan>"},
                  "deterministic CTQ gate — must pass for work order to proceed", scope),
            step!(5, "onto_propose_work_order",
                  {"scope_token": "<token>", "source_voice": "<voice>",
                   "candidate_json": "<json>"},
                  "bind CTQ to a work order", scope),
            step!(6, "onto_admit_work_order",
                  {"scope_token": "<token>"},
                  "admit work order with counterfactual delta", scope),
            step!(7, "onto_close_workflow", {"scope_token": "<token>"},
                  "close scope — seals the OCEL trace and releases the scope lock"),
        ],
    },

    // ── 8. Solution manufacturing ────────────────────────────────────────────
    Cluster {
        name: "SolutionManufacturing",
        keywords: &["manufacture", "solution", "manufacturing", "generate iac",
                    "generate rust", "generate erlang", "atomvm", "manufacture solution",
                    "cognition swarm", "hearsay"],
        can_auto_execute: false,
        powl: Some("SEQ(declare,propose_req,ctq,work_order,manufacture,close)"),
        steps: || vec![
            step!(1, "onto_declare_workflow",  {"name": "SolutionManufacturing",
                                                "description": "Solution manufacturing workflow"},
                  "open admission scope"),
            step!(2, "onto_propose_requirement",
                  {"source_voice": "<stakeholder voice>", "scope_token": "<token>"},
                  "capture requirement", scope),
            step!(3, "onto_admit_ctq",
                  {"scope_token": "<token>", "measure": "<metric>",
                   "verification": "<method>", "negative_case": "<negative>",
                   "control_plan": "<plan>"},
                  "CTQ gate", scope),
            step!(4, "onto_admit_work_order", {"scope_token": "<token>"},
                  "admit work order", scope),
            step!(5, "onto_manufacture_solution",
                  {"scope_token": "<token>", "architecture": "iac+rust"},
                  "manufacture via 9-breed cognition swarm (IaC + Rust + Erlang + AtomVM)", scope),
            step!(6, "onto_close_workflow", {"scope_token": "<token>"},
                  "close scope and seal the manufacturing receipt"),
        ],
    },

    // ── 9. Semantic search / embedding ──────────────────────────────────────
    Cluster {
        name: "SemanticSearch",
        keywords: &["semantic search", "embedding", "search classes", "natural language search",
                    "find similar", "similarity", "embed"],
        can_auto_execute: true,
        powl: None,
        steps: || vec![
            step!(1, "onto_load",       {"path": "<path.ttl>"}, "load ontology"),
            step!(2, "onto_embed",      {},
                  "generate text + Poincaré embeddings for all classes"),
            step!(3, "onto_search",     {"query": "<natural language query>"},
                  "find classes by semantic description"),
            step!(4, "onto_similarity", {"iri_a": "<iri-1>", "iri_b": "<iri-2>"},
                  "compare cosine + Poincaré distance between two specific classes"),
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_intents_returned_for_unknown_input() {
        let plan = plan_for_intent("do something completely unknown xyz", false);
        assert!(plan.ok);
        assert!(plan.plan.is_empty());
        assert!(plan.known_intents.is_some());
        let intents = plan.known_intents.unwrap();
        assert!(!intents.is_empty(), "must list at least one known workflow");
    }

    #[test]
    fn load_and_validate_resolves() {
        let plan = plan_for_intent("load and validate an ontology", false);
        assert!(plan.ok);
        assert_eq!(plan.workflow_name, Some("LoadAndValidate"));
        assert_eq!(plan.plan[0].tool, "onto_validate");
        assert_eq!(plan.plan[1].tool, "onto_load");
        assert_eq!(plan.plan[2].tool, "onto_stats");
    }

    #[test]
    fn manufacturing_resolves() {
        let plan = plan_for_intent("manufacture a solution", false);
        assert!(plan.ok);
        assert_eq!(plan.workflow_name, Some("SolutionManufacturing"));
        assert!(!plan.plan.is_empty());
    }

    #[test]
    fn data_ingest_resolves() {
        let plan = plan_for_intent("ingest csv data", false);
        assert_eq!(plan.workflow_name, Some("DataExtension"));
        assert_eq!(plan.plan[0].tool, "onto_map");
    }

    #[test]
    fn powl_included_when_requested() {
        let plan = plan_for_intent("ontology authoring", true);
        assert!(plan.powl.is_some());
    }

    #[test]
    fn powl_excluded_when_not_requested() {
        let plan = plan_for_intent("ontology authoring", false);
        assert!(plan.powl.is_none());
    }

    #[test]
    fn all_clusters_have_nonempty_steps() {
        for cluster in CLUSTERS {
            let steps = (cluster.steps)();
            assert!(
                !steps.is_empty(),
                "cluster '{}' has no steps",
                cluster.name
            );
        }
    }
}
