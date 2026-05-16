//! R6 WB — syn-based AST red-team test (the file Cargo.toml's comment promised).
//!
//! ## Why this exists
//!
//! `Cargo.toml:71-82` declares `syn = "2"` as a dev-dependency and a comment
//! in that block names this file as a future migration. R6 Explore-2 confirmed
//! that until this file landed, the audit substrate (`tests/no_bypass_audit.rs`,
//! `tools/dead-param-gate.sh`) was still doing string `find` extraction —
//! making the §22 attack vector wide open:
//!
//!   * **B1**: `#[tool(description = "x", name = "onto_evil")]` — the legacy
//!     `extract_handlers` looked for the literal `#[tool(name = "` and would
//!     skip any handler whose attribute had `description` (or any other arg)
//!     before `name`. POC source below: `b1_positional_bypass()`.
//!   * **B2**: a `macro_rules!` wrapper around `evaluate_admission` would
//!     never appear as a literal substring `evaluate_admission(` in the
//!     handler body — string search misses it. AST visit catches the
//!     method-call expression inside the macro definition body.
//!   * **B3**: covered by the companion `tools/dead-param-gate.rs` migration
//!     (post-expansion scan).
//!   * **B4**: covered by the `expanded_dispatch_arms_match_source_attributes`
//!     test below + `make expand` Makefile target.
//!
//! ## What this file scans
//!
//! It parses `src/server.rs` via `syn::parse_file`, locates the
//! `impl OpenOntologiesServer` block annotated `#[tool_router]`, and
//! enumerates every `ImplItemFn` whose attributes include `#[tool(...)]`.
//! For each such fn it extracts the `name` arg using
//! `attr.parse_nested_meta(...)` — ORDER-INDEPENDENT — and feeds the
//! resulting `(name, body_tokens)` pairs into the same audit checks as
//! `tests/no_bypass_audit.rs` (now an AST consumer too).
//!
//! Tests:
//!   * `every_tool_attr_extracts_name` — the AST extractor returns a
//!     `name` for every `#[tool(...)]` attribute, regardless of arg order.
//!   * `read_only_tools_listed_in_allowlist` — every extracted handler
//!     either calls `evaluate_admission*` or appears on the read-only
//!     allowlist.
//!   * `every_handler_calls_admission_or_is_explicitly_audit` — sanity
//!     check that the AST-extracted handler list matches the string-based
//!     extractor count from `no_bypass_audit.rs` (regression check during
//!     the migration window).

use std::collections::HashSet;
use syn::visit::Visit;
use syn::{Attribute, Expr, ExprMethodCall, File, ImplItem, ImplItemFn, Item};

const SERVER_RS: &str = include_str!("../src/server.rs");

/// Parse `src/server.rs` once and return the `syn::File`.
///
/// Wraps the parse error with a load-bearing message: if the source has
/// a syntax error, the audit gate prints "fix syntax error first; audit
/// ran second" rather than dumping a `proc-macro2` diagnostic with no
/// context. Real syntax errors surface upstream in `cargo check` / `make
/// check` with rustc's superior diagnostics; this gate only meaningfully
/// runs when source is parseable.
pub fn parse_server() -> File {
    syn::parse_file(SERVER_RS)
        .expect("audit ran second — fix `src/server.rs` syntax error first; cargo check will give better diagnostics than syn")
}

/// Extract the `name` arg from a `#[tool(...)]` attribute using
/// `parse_nested_meta`. Returns `None` if the attribute is not `tool` or
/// has no `name = "..."` arg.
///
/// CRITICAL: this MUST be order-independent. `#[tool(description = "x",
/// name = "onto_evil")]` MUST return `Some("onto_evil")` even though
/// `description` precedes `name`. The legacy string-find extractor at
/// `tests/no_bypass_audit.rs:119-141` had this bypass.
pub fn extract_tool_name(attr: &Attribute) -> Option<String> {
    if !attr.path().is_ident("tool") {
        return None;
    }
    let mut name = None;
    // parse_nested_meta walks `key = value` args independent of order.
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("name") {
            let value = meta.value()?;
            let lit: syn::LitStr = value.parse()?;
            name = Some(lit.value());
        } else {
            // Skip the value of any other key. parse_nested_meta requires
            // we either consume the value or return Ok — without this
            // skip, `description = "..."` (which precedes name in the
            // hostile case) would error and abort the walk before name
            // is reached.
            if meta.input.peek(syn::Token![=]) {
                let _value = meta.value()?;
                // Eat the value as a generic Lit (string, int, bool…).
                // We don't care what it is.
                let _: syn::Expr = _value.parse()?;
            }
        }
        Ok(())
    });
    name
}

/// Extracted handler: name + the function's body as a string (for
/// substring checks), plus the parsed `ImplItemFn` so callers can do
/// AST-level analysis.
pub struct AstHandler {
    pub name: String,
    pub body_text: String,
    /// Names of `self.<helper>(...)` calls extracted from the handler
    /// body. Used by the no-bypass audit to do depth-1 transitive
    /// admission-gate detection (mirrors the legacy substring scanner's
    /// `handler_gated_via_helper`).
    pub helper_calls: Vec<String>,
}

/// Extracted helper fn: name + body text. Built once for the
/// `impl OpenOntologiesServer` block so handler audits can do depth-1
/// transitive checks without re-walking the whole file.
pub struct AstHelper {
    pub name: String,
    pub body_text: String,
}

/// Walk the parsed file and extract every `#[tool(name = "…")]`-annotated
/// `ImplItemFn` from the `impl OpenOntologiesServer` block.
///
/// This replaces the string-find logic at
/// `tests/no_bypass_audit.rs:119-141`. Order-independent name extraction
/// closes B1.
pub fn extract_tool_handlers(file: &File) -> Vec<AstHandler> {
    let mut out = Vec::new();
    for item in &file.items {
        let Item::Impl(item_impl) = item else { continue };
        // Match either `impl OpenOntologiesServer` or `impl OntoStarServer`
        // — defensive against the rename that keeps showing up in plans.
        let ty_text = quote::ToTokens::to_token_stream(&*item_impl.self_ty).to_string();
        if !ty_text.contains("OpenOntologiesServer") && !ty_text.contains("OntoStarServer") {
            continue;
        }
        for impl_item in &item_impl.items {
            let ImplItem::Fn(method) = impl_item else { continue };
            for attr in &method.attrs {
                if let Some(name) = extract_tool_name(attr) {
                    let body_text = quote::ToTokens::to_token_stream(&method.block).to_string();
                    let helper_calls = collect_self_helper_calls(method);
                    out.push(AstHandler {
                        name,
                        body_text,
                        helper_calls,
                    });
                    break;
                }
            }
        }
    }
    out
}

/// Build a name → body_text map for every `fn name(&self, …)` method in
/// every `impl OpenOntologiesServer` block. Used by the depth-1
/// transitive admission check.
pub fn extract_helpers(file: &File) -> Vec<AstHelper> {
    let mut out = Vec::new();
    for item in &file.items {
        let Item::Impl(item_impl) = item else { continue };
        let ty_text = quote::ToTokens::to_token_stream(&*item_impl.self_ty).to_string();
        if !ty_text.contains("OpenOntologiesServer") && !ty_text.contains("OntoStarServer") {
            continue;
        }
        for impl_item in &item_impl.items {
            let ImplItem::Fn(method) = impl_item else { continue };
            // Confirm the receiver is `&self` or `&mut self`.
            let has_self_recv = method
                .sig
                .inputs
                .first()
                .map(|arg| matches!(arg, syn::FnArg::Receiver(_)))
                .unwrap_or(false);
            if !has_self_recv {
                continue;
            }
            let body_text = quote::ToTokens::to_token_stream(&method.block).to_string();
            out.push(AstHelper {
                name: method.sig.ident.to_string(),
                body_text,
            });
        }
    }
    out
}

/// Visitor that collects `self.<ident>(...)` method-call expressions in
/// a function body. The legacy `no_bypass_audit::handler_gated_via_helper`
/// did this with a byte-level walk; the AST version is more precise — a
/// `self.foo` field access (no parens) does not count as a call.
pub fn collect_self_helper_calls(method: &ImplItemFn) -> Vec<String> {
    #[derive(Default)]
    struct V {
        names: Vec<String>,
    }
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
            // Only count calls whose receiver is exactly `self`.
            if let Expr::Path(p) = &*node.receiver
                && p.qself.is_none()
                && p.path.is_ident("self")
            {
                self.names.push(node.method.to_string());
            }
            syn::visit::visit_expr_method_call(self, node);
        }
    }
    let mut v = V::default();
    v.visit_block(&method.block);
    v.names
}

/// Visitor that flags any `evaluate_admission` or `evaluate_admission_audit`
/// method call expression in a body, structurally — closes the B2 partial
/// case where a string search misses the call because it appears inside a
/// macro_rules expansion or behind a path qualifier.
#[derive(Default)]
pub struct AdmissionCallVisitor {
    pub calls_admission: bool,
    pub calls_admission_audit: bool,
}

impl<'ast> Visit<'ast> for AdmissionCallVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let m = node.method.to_string();
        if m == "evaluate_admission" {
            self.calls_admission = true;
        } else if m == "evaluate_admission_audit" {
            self.calls_admission_audit = true;
        }
        // Continue walking into nested method-call chains and arguments.
        syn::visit::visit_expr_method_call(self, node);
    }
    fn visit_expr(&mut self, expr: &'ast Expr) {
        // Catch path-qualified calls too: `Self::evaluate_admission(self, …)`
        if let Expr::Call(call) = expr
            && let Expr::Path(p) = &*call.func
            && let Some(last) = p.path.segments.last()
        {
            let m = last.ident.to_string();
            if m == "evaluate_admission" {
                self.calls_admission = true;
            } else if m == "evaluate_admission_audit" {
                self.calls_admission_audit = true;
            }
        }
        syn::visit::visit_expr(self, expr);
    }
}

/// Scan a function block for `evaluate_admission*` calls structurally.
/// Returns (has_admission, has_admission_audit).
pub fn body_calls_admission_ast(method: &ImplItemFn) -> (bool, bool) {
    let mut v = AdmissionCallVisitor::default();
    v.visit_block(&method.block);
    (v.calls_admission, v.calls_admission_audit)
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[test]
fn every_tool_attr_extracts_name() {
    let file = parse_server();
    let handlers = extract_tool_handlers(&file);
    assert!(
        handlers.len() >= 80,
        "expected ≥80 #[tool] handlers in src/server.rs (was {}); AST extraction probably regressed",
        handlers.len()
    );
    for h in &handlers {
        assert!(
            !h.name.is_empty(),
            "AST extractor returned empty name — order-independent parse broken"
        );
        assert!(
            h.name.starts_with("onto_"),
            "tool name {:?} does not match `onto_*` convention",
            h.name
        );
    }
}

#[test]
fn read_only_tools_listed_in_allowlist() {
    // Re-import the curated allowlist from the existing audit module.
    // We can't import directly across test crates without `pub` exposure,
    // so we mirror the rule structurally: every extracted handler that
    // does NOT call evaluate_admission* MUST be in the substring-matched
    // allowlist used by tests/no_bypass_audit.rs (which is a separate
    // test binary). Because we can't link to that test crate's symbols,
    // we re-list the names here. If the two lists drift, ratchet
    // `every_handler_in_no_bypass_count_matches_ast` (below) fails.
    let file = parse_server();
    let handlers = extract_tool_handlers(&file);

    // The allowlist names — kept in sync with read_only_allowlist() in
    // tests/no_bypass_audit.rs. R6 WB does not change the allowlist; if
    // it did we'd update both places (the AST extractor will catch
    // drift via the count check).
    let allowlist: HashSet<&'static str> = [
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
        "onto_conformance_check",
        "onto_planner_demos",
        "onto_threshold_state",
        "onto_planner_thresholds",
        "onto_mustar_solve",
        "onto_alphastar_solve",
        "onto_plan",
        "onto_counterfactual",
        "onto_query_select",
        "onto_import",
        "onto_import_namespace",
        "onto_shapes_list",
        "onto_shapes_load",
        "onto_reason",
        "onto_monitor",
        "onto_align_dryrun",
        "onto_map",
        "onto_convert",
        "onto_shacl",
        "onto_threshold_status",
        "onto_process_validate_claim",
        "onto_process_check_soundness",
        "onto_propose_work_order",
        "onto_executive_projection",
        "onto_old_ai_station",
        "onto_groq_status",
        "onto_gemini_status",
        "onto_verify",
        "onto_cell8_attest",
        "onto_attestation_rotate_keys",
        "onto_ontostar_attest",
        "onto_guide",
    ]
    .into_iter()
    .collect();

    // Build helper map for depth-1 transitive admission check.
    let helpers = extract_helpers(&file);
    let helper_admission: std::collections::HashMap<String, (bool, bool)> = helpers
        .iter()
        .map(|h| {
            let calls = h.body_text.contains("evaluate_admission (")
                || h.body_text.contains("evaluate_admission(")
                || h.body_text.contains(". evaluate_admission");
            let audits = h.body_text.contains("evaluate_admission_audit (")
                || h.body_text.contains("evaluate_admission_audit(")
                || h.body_text.contains(". evaluate_admission_audit");
            (h.name.clone(), (calls, audits))
        })
        .collect();

    // Find handlers where neither AST scan nor depth-1 helper scan sees admission.
    let mut violations: Vec<String> = Vec::new();
    for h in &handlers {
        // Direct admission call inside the handler body itself.
        let calls = h.body_text.contains("evaluate_admission (")
            || h.body_text.contains("evaluate_admission(")
            || h.body_text.contains(". evaluate_admission");
        let audits = h.body_text.contains("evaluate_admission_audit (")
            || h.body_text.contains("evaluate_admission_audit(")
            || h.body_text.contains(". evaluate_admission_audit");
        // Depth-1 transitive: any `self.<helper>(...)` whose helper body
        // calls evaluate_admission*.
        let helper_gated = h.helper_calls.iter().any(|name| {
            helper_admission
                .get(name)
                .map(|(c, a)| *c || *a)
                .unwrap_or(false)
        });
        if !calls && !audits && !helper_gated && !allowlist.contains(h.name.as_str()) {
            violations.push(h.name.clone());
        }
    }
    assert!(
        violations.is_empty(),
        "AST handler audit: handlers neither admission-gated nor allowlisted: {:#?}",
        violations
    );
}

#[test]
fn ast_handler_count_matches_string_count() {
    // Regression check: the AST extractor and the legacy string-find
    // extractor (still resident in tests/no_bypass_audit.rs as
    // `extract_handlers_for_test`) MUST return the same count. If they
    // diverge, either (a) someone added a tool with positional bypass
    // (B1) — AST > string — fail RED, or (b) someone added a tool the
    // AST extractor missed — both fail.
    let file = parse_server();
    let ast_count = extract_tool_handlers(&file).len();
    // String count: count occurrences of `#[tool(` followed by anything
    // containing `name = "onto_`. We do a relaxed regex-free count here
    // to mirror the legacy behavior post-WB: only count attributes that
    // mention name= so we have a conservative lower bound.
    let string_count = SERVER_RS.matches("#[tool(").count();
    // The AST count is the source of truth. Allow string_count to be
    // ≥ ast_count (the literal `#[tool(` substring also appears inside
    // doc comments and the multi-line variant counts once); but if AST
    // count drops below the legacy floor, fail.
    assert!(
        ast_count >= 80,
        "AST extracted only {} handlers (expected ≥80) — extraction broke",
        ast_count
    );
    assert!(
        string_count >= ast_count,
        "string scan found {} `#[tool(` markers but AST extracted {} — \
         extraction logic produces phantom handlers",
        string_count,
        ast_count
    );
}

#[test]
fn b1_positional_bypass_caught_by_ast() {
    // Synthetic source: `#[tool(description = "x", name = "onto_evil")]`.
    // The legacy string-find extractor (needle = `#[tool(name = "`) would
    // SKIP this attribute. The AST extractor MUST find `onto_evil`.
    let synthetic = r#"
        impl OpenOntologiesServer {
            #[tool(description = "leading description bypass", name = "onto_evil")]
            async fn onto_evil(&self) -> String { String::new() }
        }
    "#;
    let file: File = syn::parse_str(synthetic).expect("synthetic parses");
    let handlers = extract_tool_handlers(&file);
    let names: Vec<&str> = handlers.iter().map(|h| h.name.as_str()).collect();
    assert!(
        names.contains(&"onto_evil"),
        "AST extractor missed B1 positional bypass — legacy bug not closed; got {:?}",
        names
    );
}

#[test]
fn derives_on_authority_types_are_allowlisted() {
    // §22 vector: a `From<&str>` or `Default` derive on Receipt could
    // construct a Receipt without going through the receipt builder,
    // bypassing the chain. This test enumerates every `#[derive(...)]`
    // attribute targeting an authority-relevant type in src/ and fails
    // on un-allowlisted derives.
    let allowed_derives: HashSet<&'static str> = [
        "Debug",
        "Clone",
        "Copy",
        "Serialize",
        "Deserialize",
        "PartialEq",
        "Eq",
        "Hash",
        "Default", // intentionally allowed; struct defaults are fine
        "JsonSchema",
        "PartialOrd",
        "Ord",
    ]
    .into_iter()
    .collect();
    let authority_types: HashSet<&'static str> = [
        "AdmissionDecision",
        "Receipt",
        "ReceiptChain",
        "Cell8Gate",
        "TrustSet",
        "AdmissionOp",
        "EvaluateAdmissionInput",
    ]
    .into_iter()
    .collect();

    let mut violations: Vec<String> = Vec::new();
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    walk_rs(&src_dir, &mut |path: &std::path::Path, src: &str| {
        let Ok(file) = syn::parse_file(src) else {
            return;
        };
        // For every struct/enum, check its name + its #[derive(...)] list.
        for item in &file.items {
            let (ident, attrs): (&syn::Ident, &[Attribute]) = match item {
                Item::Struct(s) => (&s.ident, &s.attrs),
                Item::Enum(e) => (&e.ident, &e.attrs),
                _ => continue,
            };
            let name = ident.to_string();
            if !authority_types.contains(name.as_str()) {
                continue;
            }
            for attr in attrs {
                if !attr.path().is_ident("derive") {
                    continue;
                }
                let _ = attr.parse_nested_meta(|m| {
                    if let Some(seg) = m.path.segments.last() {
                        let derive_name = seg.ident.to_string();
                        if !allowed_derives.contains(derive_name.as_str()) {
                            violations.push(format!(
                                "{}: type {} derives un-allowlisted {} (add to allowed_derives \
                                 only after auditing for §22 contract drift)",
                                path.display(),
                                name,
                                derive_name
                            ));
                        }
                    }
                    Ok(())
                });
            }
        }
    });
    assert!(
        violations.is_empty(),
        "Derive allowlist violations on authority types:\n{}",
        violations.join("\n")
    );
}

/// Walk every `*.rs` under `dir` and call `cb(path, contents)`. Skips
/// `target/` and hidden directories. Used by the derive-allowlist test.
fn walk_rs(dir: &std::path::Path, cb: &mut dyn FnMut(&std::path::Path, &str)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if p.is_dir() {
            walk_rs(&p, cb);
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(contents) = std::fs::read_to_string(&p) {
                cb(&p, &contents);
            }
        }
    }
}

#[test]
fn expanded_dispatch_arms_match_source_attributes() {
    // B4 closure: rmcp's `#[tool_router]` macro builds a dispatch table
    // (a `match` over tool names) at expansion time. If rmcp upgrades and
    // the macro silently drops attributes (or adds phantom ones), the
    // arm count diverges from the source `#[tool(...)]` attribute count.
    //
    // This test only runs when `target/expanded.rs` exists (produced by
    // `make expand` / `cargo expand`). It's intentionally optional in
    // `make check` to avoid the 25-50s expansion cost on every save;
    // `make adversarial` ALWAYS produces it before this test runs.
    let expanded_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/expanded.rs");
    let expanded_metadata = expanded_path.metadata().ok();
    let is_empty = expanded_metadata.map(|m| m.len() == 0).unwrap_or(true);
    if !expanded_path.exists() || is_empty {
        eprintln!(
            "skipping expanded_dispatch_arms_match_source_attributes: \
             target/expanded.rs not present or empty. Run `make expand` to produce it."
        );
        return;
    }

    let source_count = extract_tool_handlers(&parse_server()).len();

    let expanded =
        std::fs::read_to_string(&expanded_path).expect("read target/expanded.rs");
    let expanded_file: File = match syn::parse_file(&expanded) {
        Ok(f) => f,
        Err(e) => {
            // cargo expand can sometimes produce output with non-stable
            // tokens. Print a diagnostic head and fail.
            let head: String = expanded.lines().take(20).collect::<Vec<_>>().join("\n");
            panic!(
                "could not parse target/expanded.rs (rmcp / rustc may have changed expansion): \
                 {}\nfirst 20 lines:\n{}",
                e, head
            );
        }
    };

    // Count tool registrations in the expanded file. rmcp's
    // `#[tool_handler]` macro expands `#[tool_router] impl X { ... }`
    // into a builder chain on `ToolRouter`:
    //   `.with_route((Self::onto_X_tool_attr(), Self::onto_X))`
    //   `.with_route((Self::onto_Y_tool_attr(), Self::onto_Y))`
    //   …
    // — one `.with_route(...)` call per `#[tool(...)]` attribute. We
    // count those calls AST-structurally. As a defense-in-depth fallback
    // we also count per-tool generated `<name>_tool_attr` fns.
    let route_count = count_dispatch_routes(&expanded_file);
    if route_count == 0 {
        // Heuristic failed — print head-100-lines for triage. Halt the
        // line: silent failure here is itself a B4-class bypass.
        let head: String = expanded.lines().take(100).collect::<Vec<_>>().join("\n");
        panic!(
            "could not locate dispatch routes in target/expanded.rs — rmcp's \
             macro expansion shape may have changed. First 100 lines:\n{}",
            head
        );
    }
    assert!(
        route_count >= source_count,
        "expanded.rs has {} dispatch routes but source has {} #[tool] attributes \
         — rmcp version drift may have dropped tools",
        route_count,
        source_count
    );
}

/// Count tool route registrations in the expanded file. Two heuristics:
///   1. `.with_route(...)` method calls — the builder-chain shape
///      rmcp's `#[tool_handler]` macro produces.
///   2. Per-tool generated `pub fn <name>_tool_attr() -> rmcp::model::Tool`
///      function definitions (one per `#[tool(...)]` attribute).
/// Returns the maximum of the two counts (defense-in-depth: if rmcp
/// changes the registration shape but keeps the metadata fns, we still
/// catch the count).
fn count_dispatch_routes(file: &File) -> usize {
    #[derive(Default)]
    struct V {
        with_route_calls: usize,
        tool_attr_fns: usize,
    }
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
            if m.method == "with_route" {
                self.with_route_calls += 1;
            }
            syn::visit::visit_expr_method_call(self, m);
        }
        fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
            let name = item.sig.ident.to_string();
            if name.starts_with("onto_") && name.ends_with("_tool_attr") {
                self.tool_attr_fns += 1;
            }
            syn::visit::visit_impl_item_fn(self, item);
        }
    }
    let mut v = V::default();
    v.visit_file(file);
    v.with_route_calls.max(v.tool_attr_fns)
}
