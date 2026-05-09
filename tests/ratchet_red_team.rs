//! Red-team test pack for the hardened ratchets.
//!
//! Each test constructs a synthetic source-line/body string carrying a
//! known bypass pattern and asserts that the ratchet's lexer flags it (or
//! refuses to count it as gated, where applicable). These tests are the
//! ratchets-of-the-ratchets — they prove the hardened lexers reject
//! exactly the bypasses Task E was hardened against.
//!
//! Patterns covered:
//!   1. String-constant occurrence of `evaluate_admission(` (must NOT count as a call).
//!   2. `if false { ... evaluate_admission ... }` (dead code; must NOT count).
//!   3. `let _ = self.evaluate_admission(...)` (Result discarded; must FAIL).
//!   4. `let X = api_key.clone()` aliasing leaked through `info!("{}", X)`.
//!   5. Format-string brace ident `info!("v={api_key}")`.
//!   6. `format!("Bearer {}", api_key)` concatenation.
//!   7. `tracing::info!(?api_key, "...")` structured field capture.
//!   8. Helper indirection: a handler delegating to a private fn whose body
//!      is hardcoded `Ok(())` (i.e. NOT actually gated) must remain
//!      un-gated according to the transitive scan.

#[path = "no_bypass_audit.rs"]
mod no_bypass_audit;

#[path = "secret_grep_ratchet.rs"]
mod secret_grep_ratchet;

use no_bypass_audit::{
    body_calls_admission_real, body_discards_admission, build_fn_map,
    handler_gated_via_helper, strip_dead_code_blocks,
};
use secret_grep_ratchet::{
    collect_aliases, line_format_brace_uses_forbidden, line_substitutes_key_with,
    line_uses_tracing_field,
};

// ── 1. String constant must NOT count as a real call ─────────────────────

#[test]
fn red_team_string_constant_does_not_count_as_call() {
    // A handler body whose ONLY occurrence of `evaluate_admission(` is
    // inside a string literal must not be considered gated.
    let body = r#"{
        let msg = "evaluate_admission(stub) called";
        log::info!("{}", msg);
    }"#;
    assert!(
        !body_calls_admission_real(body, "evaluate_admission("),
        "string-literal occurrence of evaluate_admission( must not count \
         as a real gating call"
    );
}

// ── 2. `if false { … }` block stripped before scan ───────────────────────

#[test]
fn red_team_if_false_block_does_not_count_as_call() {
    let body = r#"{
        if false { self.evaluate_admission(op, scope, kind, bytes); }
        do_the_mutation();
    }"#;
    let live = strip_dead_code_blocks(body);
    assert!(
        !body_calls_admission_real(&live, "evaluate_admission("),
        "evaluate_admission inside `if false {{ … }}` must be stripped \
         before the gating-call scan"
    );
}

// ── 3. `let _ = self.evaluate_admission(...)` discards Result ────────────

#[test]
fn red_team_let_underscore_discards_admission() {
    let body = r#"{
        let _ = self.evaluate_admission(op, scope, kind, bytes);
        proceed_with_mutation();
    }"#;
    assert!(
        body_discards_admission(body),
        "`let _ = self.evaluate_admission(...)` must be flagged — the \
         gating Result must be matched, not discarded"
    );
}

// ── 4. `let X = api_key.clone()` alias caught by per-file alias scan ─────

#[test]
fn red_team_secret_alias_clone_is_flagged() {
    let file_text = r#"
fn dispatch(api_key: &str) {
    let leaked = api_key.clone();
    info!("posting with {}", leaked);
}
"#;
    let aliases: Vec<String> = collect_aliases(file_text).into_iter().collect();
    assert!(
        aliases.iter().any(|a| a == "leaked"),
        "alias scanner must detect `let leaked = api_key.clone()`; \
         got {:?}",
        aliases
    );
    let line = r#"    info!("posting with {}", leaked);"#;
    assert!(
        line_substitutes_key_with(line, &aliases),
        "interpolation of an aliased secret identifier into info!(...) \
         must be flagged"
    );
}

// ── 5. Format-string brace ident `{api_key}` ─────────────────────────────

#[test]
fn red_team_format_string_brace_ident_is_flagged() {
    let line = r#"    info!("payload v={api_key}");"#;
    assert!(
        line_format_brace_uses_forbidden(line, &[]),
        "format-string brace interpolation `{{api_key}}` inside info!(…) \
         must be flagged"
    );
    // `{api_key:?}` debug form.
    let line2 = r#"    info!("debug={api_key:?}");"#;
    assert!(
        line_format_brace_uses_forbidden(line2, &[]),
        "`{{api_key:?}}` debug-form interpolation must also be flagged"
    );
}

// ── 6. Bearer concatenation ──────────────────────────────────────────────

#[test]
fn red_team_bearer_concatenation_is_flagged() {
    let line = r#"        let h = format!("Bearer {}", api_key);"#;
    assert!(
        line_substitutes_key_with(line, &[]),
        "`format!(\"Bearer {{}}\", api_key)` must be flagged by the \
         hardened lexer"
    );
}

// ── 7. `tracing::info!(?api_key, ...)` structured field ──────────────────

#[test]
fn red_team_tracing_structured_field_is_flagged() {
    let line = r#"    tracing::info!(?api_key, "calling provider");"#;
    assert!(
        line_uses_tracing_field(line, &[]),
        "`tracing::info!(?api_key, …)` structured-field capture must be \
         flagged — `?ident` records the Debug repr of the secret"
    );
    // `%api_key` Display form.
    let line2 = r#"    tracing::warn!(%api_key, "retrying");"#;
    assert!(
        line_uses_tracing_field(line2, &[]),
        "`tracing::warn!(%api_key, …)` Display-form capture must also be \
         flagged"
    );
}

// ── 8. Helper indirection with hardcoded Ok must NOT count as gated ──────

#[test]
fn red_team_helper_indirection_with_hardcoded_ok_fails() {
    // Synthetic file containing:
    //   - a handler body that calls `self.gate_helper(...)`,
    //   - a `gate_helper` whose body is just `Ok(())` (does NOT call
    //     evaluate_admission).
    // The transitive helper scan must NOT mark the handler as gated.
    let synthetic = r#"
impl Server {
    fn gate_helper(&self, _op: u32) -> Result<(), ()> {
        Ok(())
    }

    async fn onto_fake(&self) -> String {
        self.gate_helper(0);
        do_mutation();
        "ok".into()
    }
}
"#;
    let fn_map = build_fn_map(synthetic);
    assert!(
        fn_map.contains_key("gate_helper"),
        "build_fn_map must discover `gate_helper`; got keys: {:?}",
        fn_map.keys().collect::<Vec<_>>()
    );
    let handler_body = r#"{
        self.gate_helper(0);
        do_mutation();
        "ok".into()
    }"#;
    assert!(
        !handler_gated_via_helper(handler_body, &fn_map),
        "helper indirection where the helper body is hardcoded Ok(()) \
         must NOT register as gated via the transitive scan"
    );

    // Sanity check: a helper that DOES call evaluate_admission should
    // flip the verdict — proving the scan isn't broken open.
    let synthetic_real = r#"
impl Server {
    fn gate_real(&self, _op: u32) -> Result<(), ()> {
        self.evaluate_admission(op, scope, kind, bytes)
    }
}
"#;
    let fn_map_real = build_fn_map(synthetic_real);
    let handler_body_real = r#"{
        self.gate_real(0);
        do_mutation();
        "ok".into()
    }"#;
    assert!(
        handler_gated_via_helper(handler_body_real, &fn_map_real),
        "helper that DOES call evaluate_admission must register as gated \
         via the transitive scan (sanity check)"
    );
}
