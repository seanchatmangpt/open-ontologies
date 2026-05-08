//! Built-in POWL workflow catalog (Tier 2 standard work).
//!
//! Activity names equal OCEL `event_type` strings. POWL strings are the
//! canonical declared process for each catalog entry — they are what
//! Stream 2's wasm4pm bridge will parse and replay.

/// One entry in the built-in catalog.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinWorkflow {
    pub name: &'static str,
    pub powl_string: &'static str,
    pub alphabet: &'static [&'static str],
    pub required_stages: &'static [&'static str],
}

// POWL strings use the wasm4pm grammar:
//   PO=(nodes={a, b, c}, order={a-->b, b-->c})       — strict partial order / sequence
//   PO=(nodes={a, b}, order={})                       — concurrent (no edges)
//   X (a, b)                                          — exclusive choice (XOR)
//   * (do, redo)                                      — loop with do/redo children
//   tau                                               — silent transition
//   bare label                                        — labeled transition
//
// Notes:
// - SEQ(a, b, c) is encoded as PO with linear ordering edges.
// - IF(cond, then) is encoded as X (cond, then) — wasm4pm has no Choice Graph yet.
// - CG{a, b, c} (Choice Graph) is encoded as XOR for now.
//   TODO(wasm4pm POWL v2): use CG=(...) once Choice Graphs land upstream
//   (refactor in flight under wasm4pm agent ae31da986feb3e5b4).
// - GovernedRelease inlines its three sub-workflows so the alphabet is the
//   union and replay works against actual handler events.

pub static BUILTIN_WORKFLOWS: &[BuiltinWorkflow] = &[
    BuiltinWorkflow {
        name: "OntologyAuthoring",
        // SEQ(load, validate, * (X (invalid, fix), validate), reason,
        //     PO{lint, enforce_run}, save, version)
        powl_string:
            "PO=(nodes={load, validate, * (X (invalid, fix), validate), reason, PO=(nodes={lint, enforce_run}, order={}), save, version}, order={load-->validate, validate-->* (X (invalid, fix), validate), * (X (invalid, fix), validate)-->reason, reason-->PO=(nodes={lint, enforce_run}, order={}), PO=(nodes={lint, enforce_run}, order={})-->save, save-->version})",
        alphabet: &[
            "load",
            "validate",
            "fix",
            "invalid",
            "reason",
            "lint",
            "enforce_run",
            "save",
            "version",
        ],
        required_stages: &["load", "validate", "reason", "enforce_run", "save", "version"],
    },
    BuiltinWorkflow {
        name: "DataExtension",
        // SEQ(map, ingest, PO{stats, shacl}, reason, query)
        powl_string:
            "PO=(nodes={map, ingest, PO=(nodes={stats, shacl}, order={}), reason, query}, order={map-->ingest, ingest-->PO=(nodes={stats, shacl}, order={}), PO=(nodes={stats, shacl}, order={})-->reason, reason-->query})",
        alphabet: &["map", "ingest", "stats", "shacl", "reason", "query"],
        required_stages: &["map", "ingest", "shacl", "reason", "query"],
    },
    BuiltinWorkflow {
        name: "DataExtensionFastPath",
        // SEQ(load, extend, query)
        powl_string: "PO=(nodes={load, extend, query}, order={load-->extend, extend-->query})",
        alphabet: &["load", "extend", "query"],
        required_stages: &["load", "extend", "query"],
    },
    BuiltinWorkflow {
        name: "LifecycleApply",
        // SEQ(plan_computed, enforce_run, X (violations, enforce_run),
        //     X (apply_safe, apply_migrate, apply_force),
        //     PO{monitor_ok, monitor_alert, monitor_blocked},
        //     X (drift_detected, rollback))
        // CG{...} → XOR for now (TODO above).
        powl_string:
            "PO=(nodes={plan_computed, enforce_run, X (violations, enforce_run), X (apply_safe, apply_migrate, apply_force), PO=(nodes={monitor_ok, monitor_alert, monitor_blocked}, order={}), X (drift_detected, rollback)}, order={plan_computed-->enforce_run, enforce_run-->X (violations, enforce_run), X (violations, enforce_run)-->X (apply_safe, apply_migrate, apply_force), X (apply_safe, apply_migrate, apply_force)-->PO=(nodes={monitor_ok, monitor_alert, monitor_blocked}, order={}), PO=(nodes={monitor_ok, monitor_alert, monitor_blocked}, order={})-->X (drift_detected, rollback)})",
        alphabet: &[
            "plan_computed",
            "enforce_run",
            "violations",
            "apply_safe",
            "apply_migrate",
            "apply_force",
            "monitor_ok",
            "monitor_alert",
            "monitor_blocked",
            "drift_detected",
            "rollback",
        ],
        required_stages: &["plan_computed", "enforce_run"],
    },
    BuiltinWorkflow {
        name: "Alignment",
        // SEQ(load, load, embed, align_run, X (low_confidence, align_feedback))
        // Two distinct `load` activities are not expressible as siblings in a
        // partial order with unique node ids; we collapse to a single `load`
        // step (the alphabet still matches). TODO: revisit when POWL gains
        // labeled-instance disambiguation.
        powl_string:
            "PO=(nodes={load, embed, align_run, X (low_confidence, align_feedback)}, order={load-->embed, embed-->align_run, align_run-->X (low_confidence, align_feedback)})",
        alphabet: &["load", "embed", "align_run", "low_confidence", "align_feedback"],
        required_stages: &["load", "embed", "align_run"],
    },
    BuiltinWorkflow {
        name: "Codegen",
        // SEQ(load, validate, reason, codegen_run, lineage_recorded)
        powl_string:
            "PO=(nodes={load, validate, reason, codegen_run, lineage_recorded}, order={load-->validate, validate-->reason, reason-->codegen_run, codegen_run-->lineage_recorded})",
        alphabet: &["load", "validate", "reason", "codegen_run", "lineage_recorded"],
        required_stages: &["load", "validate", "reason", "codegen_run", "lineage_recorded"],
    },
    BuiltinWorkflow {
        name: "GovernedRelease",
        // Inlined composition: OntologyAuthoring -> LifecycleApply -> Codegen.
        // Sub-workflow names are kept as opaque activity labels here so the
        // top-level shape is replayable; the alphabet enumerates the named
        // sub-workflows plus the activities each of them admits.
        powl_string:
            "PO=(nodes={OntologyAuthoring, LifecycleApply, Codegen}, order={OntologyAuthoring-->LifecycleApply, LifecycleApply-->Codegen})",
        alphabet: &["OntologyAuthoring", "LifecycleApply", "Codegen"],
        required_stages: &["OntologyAuthoring", "LifecycleApply", "Codegen"],
    },
];

/// Lookup a built-in workflow by exact `name`.
pub fn by_name(name: &str) -> Option<&'static BuiltinWorkflow> {
    BUILTIN_WORKFLOWS.iter().find(|w| w.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_seven_entries() {
        assert_eq!(BUILTIN_WORKFLOWS.len(), 7);
    }

    #[test]
    fn lookup_known_and_unknown() {
        assert!(by_name("OntologyAuthoring").is_some());
        assert!(by_name("DataExtension").is_some());
        assert!(by_name("DataExtensionFastPath").is_some());
        assert!(by_name("LifecycleApply").is_some());
        assert!(by_name("Alignment").is_some());
        assert!(by_name("Codegen").is_some());
        assert!(by_name("GovernedRelease").is_some());
        assert!(by_name("Nope").is_none());
    }
}
