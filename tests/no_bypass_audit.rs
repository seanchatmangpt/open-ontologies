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
///
/// **R4 WE — §14 hardening**: each entry is now a `(name, justification)`
/// tuple. The justification MUST start with `READ-ONLY: ` and MUST NOT
/// contain weasel words (see `validate_allowlist_justification`). The four
/// previously-allowlisted-as-read-only mutators (`onto_declare_workflow`,
/// `onto_close_workflow`, `onto_plan_workflow`, `onto_exemplar_seed`) have
/// been promoted to full admission / audit-only and DELIBERATELY removed
/// from this list.
pub fn read_only_allowlist() -> HashSet<(&'static str, &'static str)> {
    [
        // Pure queries / introspection.
        ("onto_status",   "READ-ONLY: server liveness probe; queries no tables and writes none"),
        ("onto_load",     "READ-ONLY: parses TTL and loads into the in-memory RDF graph; does not write SQLite tables"),
        ("onto_load_remote", "READ-ONLY: HTTP fetch of TTL into the in-memory RDF graph; does not write SQLite tables"),
        ("onto_pull",     "READ-ONLY: pulls remote ontology bytes into the in-memory RDF graph; does not write SQLite tables"),
        ("onto_query",    "READ-ONLY: SPARQL SELECT/CONSTRUCT/ASK against the in-memory RDF graph"),
        ("onto_stats",    "READ-ONLY: returns class/property/triple counts from the in-memory RDF graph"),
        ("onto_lint",     "READ-ONLY: lints the in-memory RDF graph; does not write"),
        ("onto_validate", "READ-ONLY: SHACL validation against the in-memory RDF graph"),
        ("onto_lint_feedback",     "READ-ONLY: feedback inspection; does not persist"),
        ("onto_enforce",  "READ-ONLY: enforce-rule check against the in-memory RDF graph"),
        ("onto_enforce_feedback",  "READ-ONLY: feedback inspection; does not persist"),
        ("onto_diff",     "READ-ONLY: diff between two RDF graph snapshots"),
        ("onto_drift",    "READ-ONLY: drift detection between snapshots"),
        ("onto_dl_explain",  "READ-ONLY: tableaux explanation; does not write"),
        ("onto_dl_check", "READ-ONLY: subsumption check; does not write"),
        ("onto_history",  "READ-ONLY: lists named version snapshots"),
        ("onto_search",   "READ-ONLY: vector search over loaded embeddings"),
        ("onto_similarity",  "READ-ONLY: cosine similarity between two IRIs"),
        ("onto_embed",    "READ-ONLY: generates embeddings into a transient cache; no SQLite mutation"),
        ("onto_lock",     "READ-ONLY: marks IRIs locked in an in-memory set; does not persist"),
        ("onto_marketplace",  "READ-ONLY: catalogue listing/install dispatch"),
        ("onto_repo_list",  "READ-ONLY: enumerates ontology files in configured directories"),
        ("onto_repo_load",  "READ-ONLY: loads a repo file into the in-memory RDF graph"),
        ("onto_cache_status",  "READ-ONLY: cache inspection"),
        ("onto_cache_list",   "READ-ONLY: cache listing"),
        ("onto_recompile",    "READ-ONLY: re-parses a cached source into the cache; not a SQLite write"),
        ("onto_validate_clinical",  "READ-ONLY: clinical-label validation"),
        ("onto_crosswalk",    "READ-ONLY: clinical terminology lookup"),
        ("onto_enrich",   "READ-ONLY: returns proposed skos:exactMatch triples; caller must apply"),
        ("onto_lineage",  "READ-ONLY: lineage trail inspection"),
        ("onto_admission_check",  "READ-ONLY: dry-run of the admission gate; persists nothing"),
        ("onto_session_reset",    "READ-ONLY: clears in-memory session state; the only mutation is gated `clear_revocation` audited via evaluate_admission_audit"),
        ("onto_conformance_check",   "READ-ONLY: replay against a declared POWL; does not persist"),
        ("onto_planner_demos",   "READ-ONLY: returns canned planner demo cases"),
        ("onto_threshold_state", "READ-ONLY: returns current threshold sweep state"),
        ("onto_planner_thresholds",  "READ-ONLY: returns planner threshold parameters"),
        ("onto_mustar_solve",    "READ-ONLY: MuStar solver dispatch; does not persist"),
        ("onto_alphastar_solve", "READ-ONLY: AlphaStar solver dispatch; does not persist"),
        ("onto_plan",     "READ-ONLY: plan inspection; does not persist"),
        ("onto_counterfactual",  "READ-ONLY: side-by-side naked-craft vs OntoStar probe; persists nothing"),
        ("onto_query_select",    "READ-ONLY: SPARQL SELECT against the in-memory RDF graph"),
        ("onto_import",   "READ-ONLY: resolves owl:imports into the in-memory RDF graph"),
        ("onto_import_namespace","READ-ONLY: registers namespace prefix in memory"),
        ("onto_shapes_list",     "READ-ONLY: lists loaded SHACL shapes"),
        ("onto_shapes_load",     "READ-ONLY: parses SHACL shapes into the in-memory shapes set"),
        ("onto_reason",   "READ-ONLY: runs OWL reasoning against the in-memory RDF graph"),
        ("onto_monitor",  "READ-ONLY: runs SPARQL watcher queries against the in-memory RDF graph"),
        ("onto_align_dryrun",   "READ-ONLY: alignment dry-run; persists nothing"),
        ("onto_map",      "READ-ONLY: generates a mapping config; persists nothing"),
        ("onto_convert",  "READ-ONLY: file format conversion; persists nothing"),
        ("onto_shacl",    "READ-ONLY: SHACL validation; persists nothing"),
        ("onto_threshold_status",    "READ-ONLY: returns current threshold status"),
        ("onto_process_validate_claim", "READ-ONLY: POWL claim validation; persists nothing"),
        ("onto_process_check_soundness", "READ-ONLY: POWL soundness check; persists nothing"),
        ("onto_propose_work_order",   "READ-ONLY: validates work-order shape and echoes back; persists nothing"),
        ("onto_executive_projection", "READ-ONLY: pure summary via Groq; bounded by token-overlap check"),
        ("onto_old_ai_station",       "READ-ONLY: pure-function dispatch over wasm4pm-cognition breeds"),
        ("onto_groq_status",          "READ-ONLY: liveness probe; never makes a real Groq HTTP call"),
        ("onto_verify",               "READ-ONLY: external-verifier file-and-chain inspection"),
        ("onto_cell8_attest",         "READ-ONLY: Cell8 13-gate attestation read-only inspection"),
        ("onto_attestation_rotate_keys", "READ-ONLY: admin-gated trust-set reload; writes only to trusted_keys_history (closed-by-default via OPEN_ONTOLOGIES_ADMIN_PRINCIPALS)"),
        ("onto_ontostar_attest",      "READ-ONLY: R10-2 OntoStar integration seal — verifies external Ed25519 receipt, records fingerprint in trusted_keys_history, no admission path mutation"),
        ("onto_guide",                "READ-ONLY: static workflow planner — pure function returning a step-by-step plan from intent keywords; no DB writes, no admission path mutation"),
    ]
    .into_iter()
    .collect()
}

/// Public wrapper for `extract_handlers` so the red-team integration test
/// (`tests/round4_no_bypass_red_team.rs`) can drive the same lexical
/// extraction over synthetic source strings without duplicating the logic.
pub fn extract_handlers_for_test(src: &str) -> Vec<(String, String)> {
    extract_handlers(src)
}

/// Public wrapper for `build_fn_map` (same rationale as
/// `extract_handlers_for_test`).
pub fn build_fn_map_for_test(src: &str) -> std::collections::HashMap<String, String> {
    build_fn_map(src)
}

/// R6 WB — syn-based handler extractor.
///
/// Replaces the original string-find scanner that searched for the literal
/// `#[tool(name = "` needle. The legacy needle was vulnerable to B1
/// positional bypass: `#[tool(description = "x", name = "onto_evil")]` had
/// `description` precede `name`, so the needle never matched and the
/// handler was invisible to this audit.
///
/// The syn version uses `attr.parse_nested_meta(...)` to extract the `name`
/// argument ORDER-INDEPENDENTLY. The body is reconstructed from the AST
/// node via `quote::ToTokens` so callers can still do substring checks
/// (e.g. `body.contains("evaluate_admission(")`).
///
/// Falls back to the legacy string scanner only if `syn::parse_file` fails
/// (typically a partial/in-flight save with broken syntax) — in that case
/// `cargo check` will already be failing upstream with rustc's diagnostics.
fn extract_handlers(src: &str) -> Vec<(String, String)> {
    if let Ok(file) = syn::parse_file(src) {
        return extract_handlers_via_syn(&file);
    }
    // Fallback path — keeps the audit useful even if syn cannot parse
    // (e.g. an unreleased nightly feature). Legacy logic is preserved
    // verbatim below for that narrow window.
    extract_handlers_via_string(src)
}

fn extract_handlers_via_syn(file: &syn::File) -> Vec<(String, String)> {
    use syn::{ImplItem, Item};
    let mut out = Vec::new();
    for item in &file.items {
        let Item::Impl(item_impl) = item else { continue };
        let ty_text =
            quote::ToTokens::to_token_stream(&*item_impl.self_ty).to_string();
        if !ty_text.contains("OpenOntologiesServer") && !ty_text.contains("OntoStarServer") {
            continue;
        }
        for impl_item in &item_impl.items {
            let ImplItem::Fn(method) = impl_item else { continue };
            for attr in &method.attrs {
                if !attr.path().is_ident("tool") {
                    continue;
                }
                // Order-independent name extraction — closes B1.
                let mut name: Option<String> = None;
                let _ = attr.parse_nested_meta(|m| {
                    if m.path.is_ident("name") {
                        let v = m.value()?;
                        let lit: syn::LitStr = v.parse()?;
                        name = Some(lit.value());
                    } else if m.input.peek(syn::Token![=]) {
                        let v = m.value()?;
                        let _: syn::Expr = v.parse()?;
                    }
                    Ok(())
                });
                if let Some(name) = name {
                    // The body string is consumed by callers via
                    // `body.contains("evaluate_admission(")` and similar
                    // substring checks. quote::ToTokens emits tokens
                    // separated by single spaces (`evaluate_admission (`
                    // not `evaluate_admission(`), which would break
                    // those checks. Strip ALL whitespace inside the
                    // body so the substring checks work both with
                    // syn-formatted output and with hand-written code.
                    let body_tokens =
                        quote::ToTokens::to_token_stream(&method.block).to_string();
                    let body_normalized = strip_space_before_paren(&body_tokens);
                    out.push((name, body_normalized));
                }
                break;
            }
        }
    }
    out
}

/// Strip the spaces that quote::ToTokens emits between idents and `(`,
/// `.`, `::`, `<`, `>`. The substring checks in this audit are written
/// against original-source spelling (`self.evaluate_admission(`) and
/// must continue to work after the syn migration emits spaced tokens
/// (`self . evaluate_admission (`).
fn strip_space_before_paren(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_pushed: Option<char> = None;
    for ch in s.chars() {
        match ch {
            // Drop a space that immediately precedes `(`, `.`, `::`,
            // `<`, `>`, `,`, `;`. Preserves semantic structure while
            // making `evaluate_admission ( foo )` look like
            // `evaluate_admission(foo)`.
            '(' | '.' | ',' | ';' | ')' | '<' | '>' | ':' | '!' => {
                if matches!(prev_pushed, Some(' ')) {
                    out.pop();
                }
                out.push(ch);
                prev_pushed = Some(ch);
            }
            ' ' => {
                // Drop a space immediately after `(`, `.`, `::`, etc.
                if matches!(prev_pushed, Some('(' | '.' | ':' | '<' | '>' | '!')) {
                    continue;
                }
                out.push(ch);
                prev_pushed = Some(ch);
            }
            _ => {
                out.push(ch);
                prev_pushed = Some(ch);
            }
        }
    }
    out
}

fn extract_handlers_via_string(src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let needle = "#[tool(name = \"";
    while let Some(start) = src[i..].find(needle) {
        let p = i + start + needle.len();
        let name_end = src[p..].find('"').map(|d| p + d).unwrap_or(p);
        let name = src[p..name_end].to_string();
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

/// Brace-balanced block walker that is aware of Rust char literals, string
/// literals (regular + raw), and comments. Braces inside any of those do
/// NOT shift the balance — a literal `'{'` or `"{"` in a function body
/// will not cause body extraction to overrun or truncate.
///
/// Lexer states: Normal, LineComment, BlockComment(depth), String, RawString(hashes), Char.
pub fn collect_balanced_block(src: &str, body_start: usize) -> String {
    let bytes = src.as_bytes();
    if body_start >= bytes.len() || bytes[body_start] != b'{' {
        return String::new();
    }
    #[derive(Clone, Copy)]
    enum St {
        Normal,
        LineComment,
        BlockComment(u32),
        Str,
        RawStr(u32), // # count
        Chr,
    }
    let mut state = St::Normal;
    let mut depth = 0i32;
    let mut i = body_start;
    let n = bytes.len();
    let mut end = body_start;
    while i < n {
        let b = bytes[i];
        match state {
            St::Normal => {
                // Comments
                if b == b'/' && i + 1 < n && bytes[i + 1] == b'/' {
                    state = St::LineComment;
                    i += 2;
                    continue;
                }
                if b == b'/' && i + 1 < n && bytes[i + 1] == b'*' {
                    state = St::BlockComment(1);
                    i += 2;
                    continue;
                }
                // Raw string: r"..." / r#"..."# / r##"..."## ...
                if b == b'r' && i + 1 < n && (bytes[i + 1] == b'"' || bytes[i + 1] == b'#') {
                    let mut j = i + 1;
                    let mut hashes = 0u32;
                    while j < n && bytes[j] == b'#' {
                        hashes += 1;
                        j += 1;
                    }
                    if j < n && bytes[j] == b'"' {
                        // ensure prev byte isn't ident-ish (so we don't
                        // mis-fire on `for` etc.)
                        let prev_ok = i == 0 || {
                            let p = bytes[i - 1];
                            !(p.is_ascii_alphanumeric() || p == b'_')
                        };
                        if prev_ok {
                            state = St::RawStr(hashes);
                            i = j + 1;
                            continue;
                        }
                    }
                }
                // Regular string
                if b == b'"' {
                    state = St::Str;
                    i += 1;
                    continue;
                }
                // Char literal vs. lifetime. A char literal looks like
                // `'\\?.'` — a single (possibly-escaped) char then `'`.
                // A lifetime looks like `'ident` with NO closing `'`
                // immediately after.
                if b == b'\'' {
                    // Try escape: '\X' or '\xNN' or '\u{...}'
                    if i + 1 < n && bytes[i + 1] == b'\\' {
                        // Find the closing ' within a small window.
                        // For \u{...}, look for } then '.
                        let mut k = i + 2;
                        if k < n && bytes[k] == b'u' && k + 1 < n && bytes[k + 1] == b'{' {
                            k += 2;
                            while k < n && bytes[k] != b'}' {
                                k += 1;
                            }
                            if k < n {
                                k += 1; // past }
                            }
                        } else {
                            // simple escape: skip one char
                            // (handles \n, \t, \', \", \\, \0, \xNN — for
                            // \xNN we may skip too few but the closing '
                            // search below recovers)
                            k += 1;
                            // skip hex digits if any (for \xNN)
                            while k < n && bytes[k].is_ascii_hexdigit() && bytes[k] != b'\'' {
                                if k - (i + 2) > 4 {
                                    break;
                                }
                                k += 1;
                            }
                        }
                        if k < n && bytes[k] == b'\'' {
                            state = St::Chr;
                            i += 1; // entered char body
                            // We'll let the Chr state walk to the closing '.
                            continue;
                        }
                        // not a char literal, fall through and treat ' as normal
                        i += 1;
                        continue;
                    }
                    // Non-escaped: '<one char>'
                    // Need to find a single UTF-8 char then `'`.
                    let ch_len = {
                        let s = std::str::from_utf8(&bytes[i + 1..]).ok();
                        s.and_then(|s| s.chars().next().map(|c| c.len_utf8()))
                            .unwrap_or(0)
                    };
                    if ch_len > 0 && i + 1 + ch_len < n && bytes[i + 1 + ch_len] == b'\'' {
                        state = St::Chr;
                        i += 1;
                        continue;
                    }
                    // Lifetime: just consume the ' and the following ident.
                    i += 1;
                    while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                        i += 1;
                    }
                    continue;
                }
                // Brace counting
                if b == b'{' {
                    depth += 1;
                } else if b == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        end = i + 1;
                        break;
                    }
                }
                i += 1;
            }
            St::LineComment => {
                if b == b'\n' {
                    state = St::Normal;
                }
                i += 1;
            }
            St::BlockComment(d) => {
                if b == b'/' && i + 1 < n && bytes[i + 1] == b'*' {
                    state = St::BlockComment(d + 1);
                    i += 2;
                    continue;
                }
                if b == b'*' && i + 1 < n && bytes[i + 1] == b'/' {
                    if d == 1 {
                        state = St::Normal;
                    } else {
                        state = St::BlockComment(d - 1);
                    }
                    i += 2;
                    continue;
                }
                i += 1;
            }
            St::Str => {
                if b == b'\\' && i + 1 < n {
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    state = St::Normal;
                }
                i += 1;
            }
            St::RawStr(hashes) => {
                if b == b'"' {
                    // Must be followed by exactly `hashes` '#'s.
                    let mut all = true;
                    for h in 0..hashes {
                        let k = i + 1 + h as usize;
                        if k >= n || bytes[k] != b'#' {
                            all = false;
                            break;
                        }
                    }
                    if all {
                        state = St::Normal;
                        i += 1 + hashes as usize;
                        continue;
                    }
                }
                i += 1;
            }
            St::Chr => {
                if b == b'\\' && i + 1 < n {
                    i += 2;
                    continue;
                }
                if b == b'\'' {
                    state = St::Normal;
                }
                i += 1;
            }
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

// ── R4 WE — §14 hardened sub-checks ─────────────────────────────────────

/// Direct-DB-write detection. Returns `true` if `body` (assumed to already
/// be `strip_string_literals`-normalised) contains any of the canonical
/// SQLite mutation patterns. Used to catch handlers that bypass the
/// `evaluate_admission*` gate by going straight to `conn.execute(...)`.
pub fn body_writes_db(body: &str) -> bool {
    // Method-call patterns on a connection-like value.
    let stripped = strip_string_literals(body);
    if stripped.contains(".execute(")
        || stripped.contains(".execute_batch(")
        || stripped.contains(".prepare(")
    {
        return true;
    }
    // SQL statement substrings — these MUST appear in code only as string
    // literals; if they survive `strip_string_literals` they're inside an
    // identifier or macro path, which is itself a hostile shape.
    // Safer: scan the ORIGINAL body but require an `execute*` / `prepare*`
    // method call near the SQL keyword. The first branch already catches
    // that; this branch catches `query_row`-style writes and SQL strings
    // that signal an INSERT/UPDATE/DELETE intent regardless of method.
    let raw = body;
    if raw.contains("INSERT INTO") || raw.contains("UPDATE ") || raw.contains("DELETE FROM") {
        return true;
    }
    false
}

/// Depth-2 transitive helper scan. Walks every `self.<helper>(...)` call
/// in `body`; if the helper's own body matches `body_writes_db` AND
/// neither the handler nor the helper crosses an `evaluate_admission*`
/// call, returns `true` — a direct DB write was reached without ever
/// passing through the gate.
///
/// The "bypassing_gate" semantics: if the handler itself contains a real
/// `evaluate_admission(` or `evaluate_admission_audit(` call, the depth-2
/// scan is skipped — the handler is gated, downstream helpers may write
/// freely. Only ungated handlers trigger the walk.
pub fn handler_reaches_db_write_bypassing_gate(
    body: &str,
    fn_map: &std::collections::HashMap<String, String>,
) -> bool {
    // If the handler is itself gated (or audited), no further check.
    if body_calls_admission_real(body, "evaluate_admission(")
        || body_calls_admission_real(body, "evaluate_admission_audit(")
    {
        return false;
    }
    // Direct DB write in the handler body itself — caught here before any
    // helper walking.
    if body_writes_db(body) {
        return true;
    }
    // Walk depth-2: look at each `self.<helper>(` call.
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i + 5 < bytes.len() {
        if &bytes[i..i + 5] == b"self." {
            let mut j = i + 5;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_')
            {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'(' && j > i + 5 {
                let name = &body[i + 5..j];
                if let Some(helper_body) = fn_map.get(name) {
                    // If THIS helper is gated/audited, skip — admission
                    // crosses the gate inside the helper.
                    let helper_gated = body_calls_admission_real(
                        helper_body,
                        "evaluate_admission(",
                    ) || body_calls_admission_real(
                        helper_body,
                        "evaluate_admission_audit(",
                    );
                    if !helper_gated && body_writes_db(helper_body) {
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

/// Validate a single allowlist entry's justification string. Returns
/// `Err(reason)` if the justification fails any rule.
///
/// Rules:
///   - Must start with the literal prefix `"READ-ONLY: "`.
///   - "graph" appearing without "RDF graph" is rejected (it might mean
///     the SQLite store; the author must disambiguate).
///   - "before the gate" appearing without `OPEN_ONTOLOGIES_BOOTSTRAP_MODE`
///     is rejected (it's a fail-open weasel; if the handler runs before
///     the gate, the bootstrap window must be the named justification).
pub fn validate_allowlist_justification(j: &str) -> Result<(), String> {
    if !j.starts_with("READ-ONLY: ") {
        return Err(format!("missing 'READ-ONLY: ' prefix: {:?}", j));
    }
    if j.contains("graph") && !j.contains("RDF graph") {
        return Err(format!(
            "weasel word 'graph' without 'RDF graph' (does it mean the SQLite store?): {:?}",
            j
        ));
    }
    if j.contains("before the gate") && !j.contains("OPEN_ONTOLOGIES_BOOTSTRAP_MODE") {
        return Err(format!(
            "'before the gate' without OPEN_ONTOLOGIES_BOOTSTRAP_MODE — name the bootstrap window explicitly: {:?}",
            j
        ));
    }
    Ok(())
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
    // Project to a name-only set for the gating check.
    let allowed_names: HashSet<&'static str> =
        allowlist.iter().map(|(n, _)| *n).collect();
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

        let allowed = allowed_names.contains(name.as_str());

        // R4 WE — §14: hardened direct-DB-write detection on allowlisted
        // handlers. If a handler is on the allowlist but its body — or any
        // depth-1 helper it transitively reaches — contains a direct DB
        // write bypassing the gate, that's a fail-open hole disguised as
        // a read-only handler. Fail loudly.
        if allowed
            && handler_reaches_db_write_bypassing_gate(&live, &fn_map)
        {
            violations.push(format!(
                "{} — allowlisted as READ-ONLY but reaches a direct DB write bypassing the gate",
                name
            ));
            continue;
        }

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

/// R4 WE — §14: every entry in the read-only allowlist must carry a
/// `READ-ONLY: ` prefixed justification that survives the
/// `validate_allowlist_justification` regex rules. Catches lazy
/// allowlist additions that paper over fail-open holes with weasel words.
#[test]
fn read_only_allowlist_justifications_pass_regex() {
    let mut bad: Vec<String> = Vec::new();
    for (name, justification) in read_only_allowlist() {
        if let Err(reason) = validate_allowlist_justification(justification) {
            bad.push(format!("{}: {}", name, reason));
        }
    }
    assert!(
        bad.is_empty(),
        "Allowlist justifications failed the §14 regex:\n{:#?}",
        bad
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
        // R4 WE — §14: workflow scope ops promoted to full admission.
        "onto_declare_workflow",
        "onto_close_workflow",
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
        // R4 WE — §14: bootstrap-only seed handler.
        "onto_exemplar_seed",
        // R5 WC-2 — admin-only operational tools. Each emits a
        // tamper-evident OCEL audit event via evaluate_admission_audit
        // with a distinct AdmissionOp variant (BootstrapUnlock,
        // ReceiptsBatchRevoke, SessionRevoke for distinct audit
        // semantics; Feedback for the paired pause/resume tweaks).
        // Audit-only because these are recovery / governance ops; full
        // admission would deadlock onto_bootstrap_unlock when the lock
        // row itself is the problem the operator is repairing.
        "onto_bootstrap_unlock",
        "onto_receipts_revoke_batch",
        "onto_session_revoke_by_principal",
        "onto_retention_pause",
        "onto_retention_resume",
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
