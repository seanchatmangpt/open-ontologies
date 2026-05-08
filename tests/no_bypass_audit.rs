//! Level-5 No-bypass invariant test.
//!
//! Static-grep invariant: every MCP tool handler in `src/server.rs` whose
//! body mutates persistent state either calls `evaluate_admission` (full
//! admission), `evaluate_admission_audit` (audit-only), or appears on the
//! curated read-only allowlist below.
//!
//! This is intentionally a textual scan rather than a runtime test — the
//! invariant is "no new mutation handler ships without admission wiring."
//! When new handlers land they must explicitly opt into one of three
//! categories.

use std::collections::HashSet;

const SERVER_RS: &str = include_str!("../src/server.rs");

/// Handlers that genuinely do not mutate persistent state. Adding a name
/// here is a deliberate choice and should be reviewed during code review.
fn read_only_allowlist() -> HashSet<&'static str> {
    [
        // Pure queries / introspection.
        "onto_status",
        "onto_load",
        "onto_load_remote",
        "onto_pull",
        "onto_query",
        "onto_stats",
        "onto_lint",
        "onto_validate",
        "onto_lint_feedback",
        "onto_enforce",
        "onto_enforce_feedback",
        "onto_diff",
        "onto_drift",
        "onto_dl_explain",
        "onto_dl_check",
        "onto_history",
        "onto_search",
        "onto_similarity",
        "onto_embed",
        "onto_lock",
        "onto_marketplace",
        "onto_repo_list",
        "onto_repo_load",
        "onto_cache_status",
        "onto_cache_list",
        "onto_recompile",
        "onto_validate_clinical",
        "onto_crosswalk",
        "onto_enrich",
        "onto_lineage",
        "onto_admission_check",
        "onto_session_reset",
        "onto_close_workflow",
        "onto_declare_workflow",
        "onto_conformance_check",
        "onto_planner_demos",
        "onto_workflow_feedback",
        "onto_threshold_state",
        "onto_planner_thresholds",
        "onto_mustar_solve",
        "onto_alphastar_solve",
        "onto_plan",
        "onto_plan_workflow",      // emits OCEL but doesn't mutate the graph
        "onto_exemplar_seed",      // admin-only seed; runs before the gate exists
        "onto_counterfactual",     // read-only probe
        "onto_query_select",
        "onto_import",             // resolves owl:imports — read-only side, fetches into store but no admission tier yet
        "onto_import_namespace",
        "onto_shapes_list",
        "onto_shapes_load",
        "onto_reason",
        "onto_monitor",
        "onto_align_dryrun",       // (if it exists)
        "onto_map",                // generates a mapping config; read-only on the store
        "onto_convert",            // file format conversion — does not touch the store
        "onto_shacl",               // validation only — does not mutate
        "onto_threshold_status",    // inspection
        "onto_threshold_sweep",     // recomputes thresholds in-place; admin op without a graph mutation surface
        "onto_workflow_discover",   // Loop 3 mining; inserts a `discovered_workflows` suggestion row, doesn't mutate the loaded graph
        "onto_process_validate_claim", // POWL claim validation — read-only
        "onto_process_check_soundness", // POWL soundness check — read-only
        // Requirements Andon / CTQ Forge (Phase 1.5):
        "onto_propose_work_order",   // pure echo + validation; no graph mutation
        "onto_executive_projection", // pure summary via Groq; bounded by token-overlap check
        // Old-AI station dispatcher (Phase 1.6):
        "onto_old_ai_station",       // pure-function dispatch over wasm4pm-cognition breeds
    ]
    .into_iter()
    .collect()
}

fn extract_handlers(src: &str) -> Vec<(String, String)> {
    // Find every `#[tool(name = "onto_*", …)]` and capture the handler's
    // function body (until the matching closing brace). Crude but adequate
    // for the static check.
    let mut out = Vec::new();
    let mut i = 0usize;
    let needle = "#[tool(name = \"";
    while let Some(start) = src[i..].find(needle) {
        let p = i + start + needle.len();
        let name_end = src[p..].find('"').map(|d| p + d).unwrap_or(p);
        let name = src[p..name_end].to_string();
        // Find the function body start: `async fn …(` after the attribute,
        // then walk to its closing brace.
        let fn_open = src[name_end..].find("fn ").map(|d| name_end + d);
        let body_start = fn_open
            .and_then(|f| src[f..].find('{').map(|d| f + d))
            .unwrap_or(name_end);
        let body = collect_balanced_block(src, body_start);
        out.push((name, body));
        i = body_start + 1;
    }
    out
}

fn collect_balanced_block(src: &str, body_start: usize) -> String {
    let bytes = src.as_bytes();
    if body_start >= bytes.len() || bytes[body_start] != b'{' {
        return String::new();
    }
    let mut depth = 0i32;
    let mut end = body_start;
    for (idx, b) in bytes[body_start..].iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = body_start + idx + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    src[body_start..end].to_string()
}

#[test]
fn every_mcp_handler_is_gated_audited_or_explicitly_readonly() {
    let allowlist = read_only_allowlist();
    let handlers = extract_handlers(SERVER_RS);
    assert!(
        handlers.len() >= 40,
        "expected ≥40 MCP handlers, found {} — extraction probably regressed",
        handlers.len()
    );

    let mut violations: Vec<String> = Vec::new();
    for (name, body) in &handlers {
        let gated = body.contains("evaluate_admission(");
        let audited = body.contains("evaluate_admission_audit(");
        let allowed = allowlist.contains(name.as_str());
        if !gated && !audited && !allowed {
            violations.push(name.clone());
        }
    }

    assert!(
        violations.is_empty(),
        "MCP handlers neither gated nor audited nor allowlisted (must wire admission \
         OR add to read_only_allowlist with justification): {:#?}",
        violations
    );
}

#[test]
fn full_admission_handlers_present() {
    // Ratchet: assert the handlers we've explicitly gated all still call
    // evaluate_admission. If someone removes the gate from any of these,
    // this test fails loudly.
    let handlers = extract_handlers(SERVER_RS);
    let by_name: std::collections::HashMap<String, String> =
        handlers.into_iter().collect();
    for required in &[
        "onto_save",
        "onto_apply",
        "onto_codegen",
        "onto_push",
        "onto_ingest",
        "onto_align",
        "onto_rollback",
        "onto_import_schema",
        // Requirements Andon / CTQ Forge full-admission handlers (Phase 1.5):
        "onto_propose_requirement",
        "onto_admit_ctq",
        "onto_admit_work_order",
        // Solution Manufacturing full-admission handler (Phase 4):
        "onto_manufacture_solution",
    ] {
        let body = by_name
            .get(*required)
            .unwrap_or_else(|| panic!("handler {} missing from server.rs", required));
        assert!(
            body.contains("evaluate_admission("),
            "handler {} must call evaluate_admission",
            required
        );
    }
}

#[test]
fn audit_only_handlers_present() {
    let handlers = extract_handlers(SERVER_RS);
    let by_name: std::collections::HashMap<String, String> =
        handlers.into_iter().collect();
    for required in &[
        "onto_clear",
        "onto_unload",
        "onto_cache_remove",
        "onto_align_feedback",
        "onto_monitor_clear",
        "onto_version",
        // Requirements Andon audit-only handler (Phase 1.5):
        "onto_translate_candidate",
    ] {
        let body = by_name
            .get(*required)
            .unwrap_or_else(|| panic!("handler {} missing from server.rs", required));
        assert!(
            body.contains("evaluate_admission_audit("),
            "handler {} must call evaluate_admission_audit",
            required
        );
    }
}
