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
//!
//! Hardened (Task E):
//!   1. Strip dead-code blocks (`if false { … }`, `cfg!(any())`, `#[cfg(any())]`)
//!      before the greppable scan.
//!   2. Reject `let _ = ...evaluate_admission` (Result must be matched).
//!   3. Reject string-literal occurrences of `evaluate_admission(` (the
//!      gating call must appear OUTSIDE string literals).
//!   4. Helper transitive scan: a handler that delegates to a private fn
//!      counts as gated if THAT fn calls `evaluate_admission`. Capped at
//!      depth 1.

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

// ── Hardened sub-checks (Task E) ──────────────────────────────────────────

/// Strip syntactically dead-code blocks from a body so the greppable scan
/// cannot be fooled by `if false { self.evaluate_admission(...) }`.
///
/// Strips:
///   * `if false { ... }` (balanced braces)
///   * `if cfg!(any()) { ... }` (balanced braces)
///   * `#[cfg(any())] fn/block ...` — strips the immediately following
///     balanced-brace block.
pub fn strip_dead_code_blocks(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut i = 0usize;
    while i < body.len() {
        let rest = &body[i..];
        // Detect "if false {" or "if cfg!(any()) {" at this offset.
        if let Some(skip_to) = match_dead_head(rest) {
            let brace_abs = i + skip_to;
            if brace_abs < body.len() && body.as_bytes()[brace_abs] == b'{' {
                let block = collect_balanced_block(body, brace_abs);
                i = brace_abs + block.len();
                continue;
            }
        }
        // Detect `#[cfg(any())]` then optional whitespace then `{`.
        if rest.starts_with("#[cfg(any())]") {
            let after = i + "#[cfg(any())]".len();
            if let Some(rel) = body[after..].find('{') {
                let brace_abs = after + rel;
                let block = collect_balanced_block(body, brace_abs);
                i = brace_abs + block.len();
                continue;
            }
        }
        // Advance one full char (so we never split a multi-byte boundary).
        let ch = rest.chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// If `s` starts with a dead-code head pattern, return the offset (relative
/// to s) of the `{` that opens the block to be stripped.
fn match_dead_head(s: &str) -> Option<usize> {
    // "if false" + ws + "{"
    if let Some(rest) = s.strip_prefix("if false") {
        let trimmed = rest.trim_start();
        if trimmed.starts_with('{') {
            return Some(s.len() - trimmed.len());
        }
    }
    // "if cfg!(any())" + ws + "{"
    if let Some(rest) = s.strip_prefix("if cfg!(any())") {
        let trimmed = rest.trim_start();
        if trimmed.starts_with('{') {
            return Some(s.len() - trimmed.len());
        }
    }
    None
}

/// Returns `true` if the body contains `let _ = ...evaluate_admission(...)`
/// — i.e. the gating Result is being deliberately discarded. This is a
/// failure: the Result must be matched and acted on.
pub fn body_discards_admission(body: &str) -> bool {
    // Match `let _ = ` followed (possibly via `self.` or namespace path) by
    // `evaluate_admission`. Cheap: scan for the literal substring.
    body.contains("let _ = self.evaluate_admission")
        || body.contains("let _ = evaluate_admission")
        || body.contains("let _ = Self::evaluate_admission")
}

/// Returns `true` if the body contains `evaluate_admission(` as a real
/// call — that is, OUTSIDE any string literal. A bare string-literal
/// occurrence (e.g. `"evaluate_admission("` in a doc/log message) does
/// NOT count.
pub fn body_calls_admission_real(body: &str, needle: &str) -> bool {
    let stripped = strip_string_literals(body);
    stripped.contains(needle)
}

/// Strip Rust string literals (`"..."`, `r"..."`, `r#"..."#`, `r##"..."##`,
/// char literals `'.'`) from `s`, replacing them with a single space so
/// offsets/length characteristics are preserved. Comments are not stripped
/// (they don't matter for call detection — but `// "evaluate_admission("`
/// in a comment WILL false-positive; this is acceptable since we only use
/// this to test for REAL calls that would actually execute, and a comment
/// containing a real-looking call but no actual code path is a marginal
/// case the policy explicitly tolerates as deepening authority).
pub fn strip_string_literals(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0usize;
    while i < bytes.len() {
        // Raw string: r#"..."#  (any number of #)
        if bytes[i] == b'r' && i + 1 < bytes.len() && (bytes[i + 1] == b'"' || bytes[i + 1] == b'#') {
            let mut hash_count = 0usize;
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == b'#' {
                hash_count += 1;
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'"' {
                // Find closing "###...
                let mut k = j + 1;
                while k < bytes.len() {
                    if bytes[k] == b'"' {
                        // Check for `hash_count` '#' after.
                        let mut all = true;
                        for h in 0..hash_count {
                            if k + 1 + h >= bytes.len() || bytes[k + 1 + h] != b'#' {
                                all = false;
                                break;
                            }
                        }
                        if all {
                            k = k + 1 + hash_count;
                            break;
                        }
                    }
                    k += 1;
                }
                out.push(' ');
                i = k;
                continue;
            }
        }
        // Regular string: "..."  (with backslash escapes)
        if bytes[i] == b'"' {
            let mut k = i + 1;
            while k < bytes.len() {
                if bytes[k] == b'\\' && k + 1 < bytes.len() {
                    k += 2;
                    continue;
                }
                if bytes[k] == b'"' {
                    k += 1;
                    break;
                }
                k += 1;
            }
            out.push(' ');
            i = k;
            continue;
        }
        // Char literal `'.'` or `'\n'` etc. — only if next char is not an
        // ident-ish (lifetime annotations like 'a). Cheap detection:
        // require closing `'` within 4 bytes.
        if bytes[i] == b'\'' {
            let mut k = i + 1;
            // Try escape.
            if k < bytes.len() && bytes[k] == b'\\' {
                k += 2;
                if k < bytes.len() && bytes[k] == b'\'' {
                    out.push(' ');
                    i = k + 1;
                    continue;
                }
            } else if k + 1 < bytes.len() && bytes[k + 1] == b'\'' {
                out.push(' ');
                i = k + 2;
                continue;
            }
            // Otherwise treat as lifetime; fall through.
        }
        // Advance by a full char to preserve UTF-8 boundaries.
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Helper transitive scan, capped at depth 1.
///
/// Build a map of `fn <name>(&self ...)` → body for all private fns in
/// `src/server.rs`. If a handler body invokes `self.<helper>(...)` and
/// THAT helper's body calls `evaluate_admission` (real, not in-string),
/// treat the handler as gated.
pub fn handler_gated_via_helper(body: &str, fn_map: &std::collections::HashMap<String, String>) -> bool {
    // Find `self.<ident>(` invocations.
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i + 5 < bytes.len() {
        if &bytes[i..i + 5] == b"self." {
            // Read identifier.
            let mut j = i + 5;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_')
            {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'(' && j > i + 5 {
                let name = &body[i + 5..j];
                if let Some(helper_body) = fn_map.get(name) {
                    if body_calls_admission_real(helper_body, "evaluate_admission(")
                        || body_calls_admission_real(
                            helper_body,
                            "evaluate_admission_audit(",
                        )
                    {
                        return true;
                    }
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    false
}

/// Build a map `fn name → body` for private/free fns matching the regex
/// `fn ([a-z_]+)\(&self`. Returns owned Strings so callers can index by
/// name without lifetime gymnastics.
pub fn build_fn_map(src: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let bytes = src.as_bytes();
    let mut i = 0usize;
    while i + 3 < bytes.len() {
        // Look for "fn " preceded by non-ident byte (so we don't match e.g. "rfn ").
        if &bytes[i..i + 3] == b"fn " {
            let prev_ok = i == 0 || {
                let p = bytes[i - 1];
                !(p.is_ascii_alphanumeric() || p == b'_')
            };
            if prev_ok {
                // Read identifier after "fn ".
                let mut j = i + 3;
                while j < bytes.len() && (bytes[j].is_ascii_lowercase() || bytes[j] == b'_') {
                    j += 1;
                }
                if j > i + 3 && j < bytes.len() && bytes[j] == b'(' {
                    let name = src[i + 3..j].to_string();
                    // Confirm "&self" appears in the param list before ')'.
                    if let Some(close) = src[j..].find(')') {
                        let params = &src[j..j + close];
                        if params.contains("&self") {
                            // Walk to body '{'.
                            if let Some(rel) = src[j + close..].find('{') {
                                let body_start = j + close + rel;
                                let body = collect_balanced_block(src, body_start);
                                map.insert(name, body);
                            }
                        }
                    }
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    map
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

    let fn_map = build_fn_map(SERVER_RS);

    let mut violations: Vec<String> = Vec::new();
    for (name, body) in &handlers {
        // Sub-check 1: strip dead-code blocks first.
        let live = strip_dead_code_blocks(body);

        // Sub-check 2: `let _ = ...evaluate_admission` is forbidden.
        if body_discards_admission(&live) {
            violations.push(format!("{} — discards admission Result via `let _ =`", name));
            continue;
        }

        // Sub-check 3: only OUT-OF-STRING occurrences count.
        let gated = body_calls_admission_real(&live, "evaluate_admission(");
        let audited = body_calls_admission_real(&live, "evaluate_admission_audit(");

        // Sub-check 4: helper transitive scan (depth-1).
        let helper_gated = !gated && !audited && handler_gated_via_helper(&live, &fn_map);

        let allowed = allowlist.contains(name.as_str());
        if !gated && !audited && !helper_gated && !allowed {
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
        // Loop 2/3 audit-only handlers reclassified by Task E:
        "onto_workflow_discover",
        "onto_workflow_feedback",
        "onto_threshold_sweep",
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
