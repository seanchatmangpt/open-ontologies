//! R6 WB — sabotage tests for the AST red-team scanner.
//!
//! Each test embeds a synthetic Rust source string and asserts the
//! AST-based detector flags it. Without the migration, a string-find
//! detector would miss B1/B2/B3/B4. With the migration, all four are red
//! → green (the test file passes only because the AST scanner finds the
//! sabotaged shape).
//!
//! These are the four bypass theses R6 Explore-2 confirmed against the
//! pre-migration audit substrate.

use syn::visit::Visit;
use syn::{File, Item};

// ─── B1 — positional bypass on `#[tool(name = …)]` ───────────────────────

#[test]
fn b1_positional_bypass_caught_by_ast() {
    // Hostile source: `description` arg precedes `name`. The legacy
    // string-find extractor (needle = `#[tool(name = "`) MISSES this.
    // The AST extractor MUST find `onto_evil`.
    let src = r#"
        impl OpenOntologiesServer {
            #[tool(description = "leading description bypass", name = "onto_evil_b1")]
            async fn onto_evil_b1(&self) -> String { String::new() }
        }
    "#;
    let file: File = syn::parse_str(src).expect("synthetic parses");
    // Re-implement extract_tool_name inline so this test does not depend
    // on linking to round5_ast_red_team.rs (separate test binary).
    let names = extract_tool_names(&file);
    assert!(
        names.contains(&"onto_evil_b1".to_string()),
        "B1 sabotage: AST scanner failed to extract `name` when it follows \
         `description`; got {:?}",
        names
    );

    // And confirm that the legacy substring-only check would have MISSED
    // this — proving that without the AST migration the bypass was real.
    let legacy_needle = "#[tool(name = \"";
    assert!(
        !src.contains(legacy_needle),
        "B1 setup is wrong: legacy needle should not match this hostile source"
    );
}

// ─── B2 — macro_rules wrapping admission inside handler body ─────────────

#[test]
fn b2_macro_rules_wrapping_admission_flagged() {
    // Hostile source: a handler defines a `gated_call` macro that wraps
    // `evaluate_admission` and invokes it from inside the handler body.
    // The literal substring `evaluate_admission(` appears INSIDE a
    // macro_rules definition, not as a real call inside the handler.
    // The AST visitor must detect the call structurally (inside the
    // macro_rules body or inside the macro invocation expansion).
    let src = r#"
        impl OpenOntologiesServer {
            #[tool(name = "onto_evil_b2", description = "macro-shrouded")]
            async fn onto_evil_b2(&self) -> String {
                macro_rules! gated_call {
                    ($e:expr) => { $e };
                }
                let _decision = gated_call!(self.evaluate_admission(input).await);
                String::new()
            }
        }
    "#;
    let file: File = syn::parse_str(src).expect("synthetic parses");

    // Source-level AST visitor: walk every method-call expression AND
    // every macro invocation. For macros we attempt to parse the token
    // stream as an Expr and recursively walk it — closes the case where
    // a macro invocation wraps an admission call (the call lives in the
    // invocation tokens, not as a real ExprMethodCall node).
    #[derive(Default)]
    struct V {
        found: bool,
    }
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
            if m.method == "evaluate_admission" {
                self.found = true;
            }
            syn::visit::visit_expr_method_call(self, m);
        }
        fn visit_macro(&mut self, m: &'ast syn::Macro) {
            // Macro invocation: try to parse the tokens as an Expr and
            // walk it. If parsing fails (declarative macro_rules! body,
            // attribute macros, etc.), fall back to a substring check.
            let toks = m.tokens.clone();
            if let Ok(inner) = syn::parse2::<syn::Expr>(toks.clone()) {
                self.visit_expr(&inner);
            } else if toks.to_string().contains("evaluate_admission") {
                self.found = true;
            }
            syn::visit::visit_macro(self, m);
        }
    }
    let mut v = V::default();
    v.visit_file(&file);
    assert!(
        v.found,
        "B2 sabotage: AST visitor missed the evaluate_admission call wrapped \
         in a macro_rules body. Without AST detection, a hostile rename of \
         `evaluate_admission` to `gated_call!(real_admission)` would slip past \
         a string-only audit."
    );
}

// ─── B3 — let _ = $p; laundered across crate boundaries ──────────────────

#[test]
fn b3_macro_laundered_dead_param_flagged_post_expansion() {
    // Hostile pattern (post-expansion): the `discard!` macro is defined
    // in `src/lib.rs` and invoked from `src/cmds/foo.rs`. Pre-expansion,
    // `src/cmds/` contains no `let _ =` line — the legacy shell gate is
    // blind. Post-expansion, the laundered `let _ = some_param;` IS in
    // the expanded source.
    //
    // This test simulates a `target/expanded.rs` with the laundered
    // pattern and asserts the syn-based dead-param scanner (the new
    // `tools/dead-param-gate.rs` Rust binary) flags it.
    let expanded = r#"
        fn handler(some_param: u32) -> u32 {
            // (laundered from `discard!(some_param)` macro invocation)
            let _ = some_param;
            42
        }
    "#;
    let file: File = syn::parse_str(expanded).expect("synthetic parses");
    let violations = scan_dead_params(&file);
    assert!(
        !violations.is_empty(),
        "B3 sabotage: AST dead-param scanner missed `let _ = some_param;` \
         in expanded source. Without post-expansion AST scan, macro-laundered \
         dead params slip past the gate."
    );
    assert_eq!(violations[0], "some_param");
}

#[test]
fn b3_false_positives_filtered() {
    // The legitimate patterns must NOT be flagged: `let _ = match`,
    // `let _ = std::mem::take(...)`, `_guard;`, `let _ = some_call();`
    // (call expression, not bare identifier).
    let benign = r#"
        fn handler(x: u32, y: Result<u32, ()>) -> u32 {
            let _ = match x { 0 => "z", _ => "a" };
            let _ = std::mem::take(&mut Some(1));
            let _guard = MutexGuard::new();
            let _ = compute();
            42
        }
    "#;
    let file: File = syn::parse_str(benign).expect("synthetic parses");
    let violations = scan_dead_params(&file);
    assert!(
        violations.is_empty(),
        "B3 false positives: scanner flagged benign patterns: {:?}",
        violations
    );
}

// ─── B4 — rmcp expansion arm count drift ─────────────────────────────────

#[test]
fn b4_arm_count_drift_caught() {
    // Source declares 3 `#[tool(...)]` attributes; the hostile expanded
    // file simulates rmcp dropping one (2 arms). The B4 detector must
    // flag the diff.
    let source = r#"
        impl OpenOntologiesServer {
            #[tool(name = "onto_a", description = "")] async fn a(&self) -> String { String::new() }
            #[tool(name = "onto_b", description = "")] async fn b(&self) -> String { String::new() }
            #[tool(name = "onto_c", description = "")] async fn c(&self) -> String { String::new() }
        }
    "#;
    let source_file: File = syn::parse_str(source).expect("source parses");
    let source_count = extract_tool_names(&source_file).len();
    assert_eq!(source_count, 3);

    // Hostile "expanded" file has only 2 arms.
    let hostile_expanded = r#"
        fn dispatch(name: String) -> String {
            match name.as_str() {
                "onto_a" => String::from("a"),
                "onto_b" => String::from("b"),
                _ => String::new(),
            }
        }
    "#;
    let expanded_file: File = syn::parse_str(hostile_expanded).expect("expanded parses");
    let arm_count = count_onto_arms(&expanded_file);
    assert_eq!(arm_count, 2);

    // The actual gate would assert arm_count >= source_count and fail.
    // Here we assert the failure mode is detectable.
    assert!(
        arm_count < source_count,
        "B4 detector: arm count ({}) must be less than source count ({}) \
         for the gate to fire",
        arm_count,
        source_count
    );
}

// ─── Helpers (shared with round5_ast_red_team.rs but inlined for test isolation) ──

fn extract_tool_names(file: &File) -> Vec<String> {
    let mut names = Vec::new();
    for item in &file.items {
        let Item::Impl(item_impl) = item else { continue };
        for impl_item in &item_impl.items {
            let syn::ImplItem::Fn(method) = impl_item else { continue };
            for attr in &method.attrs {
                if !attr.path().is_ident("tool") {
                    continue;
                }
                let _ = attr.parse_nested_meta(|m| {
                    if m.path.is_ident("name") {
                        let v = m.value()?;
                        let lit: syn::LitStr = v.parse()?;
                        names.push(lit.value());
                    } else if m.input.peek(syn::Token![=]) {
                        let v = m.value()?;
                        let _: syn::Expr = v.parse()?;
                    }
                    Ok(())
                });
            }
        }
    }
    names
}

/// Count match arms whose pattern is a string literal beginning with `onto_`.
fn count_onto_arms(file: &File) -> usize {
    #[derive(Default)]
    struct V {
        best: usize,
    }
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_match(&mut self, m: &'ast syn::ExprMatch) {
            let onto = m
                .arms
                .iter()
                .filter(|a| {
                    let p = quote::ToTokens::to_token_stream(&a.pat).to_string();
                    p.contains("\"onto_")
                })
                .count();
            if onto > self.best {
                self.best = onto;
            }
            syn::visit::visit_expr_match(self, m);
        }
    }
    let mut v = V::default();
    v.visit_file(file);
    v.best
}

/// AST scanner for `let _ = <single-ident>;` patterns. Returns the
/// flagged identifier names. Filters benign cases via AST shape.
fn scan_dead_params(file: &File) -> Vec<String> {
    #[derive(Default)]
    struct V {
        violations: Vec<String>,
    }
    impl<'ast> Visit<'ast> for V {
        fn visit_local(&mut self, local: &'ast syn::Local) {
            // Pattern must be `let _` (Wild pattern).
            let is_wild = matches!(&local.pat, syn::Pat::Wild(_));
            if !is_wild {
                syn::visit::visit_local(self, local);
                return;
            }
            // Init must be `Some(LocalInit { expr, .. })` and the expr a
            // bare path with a single ident.
            let Some(init) = &local.init else {
                syn::visit::visit_local(self, local);
                return;
            };
            if let syn::Expr::Path(p) = &*init.expr
                && p.qself.is_none()
                && p.path.segments.len() == 1
            {
                let ident = p.path.segments[0].ident.to_string();
                self.violations.push(ident);
            }
            syn::visit::visit_local(self, local);
        }
    }
    let mut v = V::default();
    v.visit_file(file);
    v.violations
}
