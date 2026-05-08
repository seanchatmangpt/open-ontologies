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

pub static BUILTIN_WORKFLOWS: &[BuiltinWorkflow] = &[
    BuiltinWorkflow {
        name: "OntologyAuthoring",
        powl_string:
            "SEQ(load, validate, LOOP(IF(invalid, fix), validate), reason, PO{lint, enforce_run}, save, version)",
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
        powl_string: "SEQ(map, ingest, PO{stats, shacl}, reason, query)",
        alphabet: &["map", "ingest", "stats", "shacl", "reason", "query"],
        required_stages: &["map", "ingest", "shacl", "reason", "query"],
    },
    BuiltinWorkflow {
        name: "DataExtensionFastPath",
        powl_string: "SEQ(load, extend, query)",
        alphabet: &["load", "extend", "query"],
        required_stages: &["load", "extend", "query"],
    },
    BuiltinWorkflow {
        name: "LifecycleApply",
        powl_string:
            "SEQ(plan_computed, enforce_run, IF(violations, enforce_run), CG{apply_safe, apply_migrate, apply_force}, PO{monitor_ok, monitor_alert, monitor_blocked}, IF(drift_detected, rollback))",
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
        powl_string:
            "SEQ(load, load, embed, align_run, IF(low_confidence, align_feedback))",
        alphabet: &["load", "embed", "align_run", "low_confidence", "align_feedback"],
        required_stages: &["load", "embed", "align_run"],
    },
    BuiltinWorkflow {
        name: "Codegen",
        powl_string: "SEQ(load, validate, reason, codegen_run, lineage_recorded)",
        alphabet: &["load", "validate", "reason", "codegen_run", "lineage_recorded"],
        required_stages: &["load", "validate", "reason", "codegen_run", "lineage_recorded"],
    },
    BuiltinWorkflow {
        name: "GovernedRelease",
        powl_string: "SEQ(OntologyAuthoring, LifecycleApply, Codegen)",
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
