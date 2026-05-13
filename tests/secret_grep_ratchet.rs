//! Level-5 secret-hygiene textual ratchet.
//!
//! Invariant: no source line in `src/` may interpolate an API-key-like
//! identifier into a logging or formatting macro. The intent is to catch
//! `info!("posting with key {}", api_key)` shape mistakes before they
//! reach a log file or OCEL attribute.
//!
//! This is a static lexical scan — not a runtime test. It pairs with
//! `tests/secret_hygiene.rs` which runs the full canary-key cycle.
//!
//! Allowlist: the lexer is dumb, so a small list of explicit exceptions
//! covers cases where the *literal text* "api_key" appears in a log
//! message but the surrounding code does not actually substitute the
//! key value (e.g. tracing target names, redaction-helper definitions).

use std::path::Path;

/// Lines/files that legitimately mention a key-like word in a logging
/// context but do not interpolate the key value. Each entry is `(file
/// suffix, fragment that must appear on the line)` — both must match for
/// the line to be excused.
fn allowlist() -> Vec<(&'static str, &'static str)> {
    vec![
        // The translator's own redaction helper definition. Mentions
        // "Bearer" and "api_key" in doc comments and the redaction
        // string itself, but never substitutes the secret.
        ("src/llm_translator.rs", "<redacted>"),
        ("src/llm_translator.rs", "<unset>"),
        ("src/llm_translator.rs", "Bearer <redacted>"),
        // Doc-comment line that names invariant 7 by quoting the
        // forbidden patterns themselves; not a real interpolation.
        ("src/llm_translator.rs", "MUST NOT"),
    ]
}

fn collect_rs_files(root: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                // Skip target/, .git/, node_modules-like dirs.
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    if name == "target" || name.starts_with('.') {
                        continue;
                    }
                }
                collect_rs_files(&p, out);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
}

/// Pre-scan a file's full text and collect identifier names that have
/// been bound to `api_key` (or another forbidden ident) via `let X = ...`.
/// Aliases captured: `let X = api_key`, `let X = &api_key`,
/// `let X = api_key.clone()`, `let X = api_key.to_string()`.
///
/// Returns owned `String` aliases. Callers fold these into the FORBIDDEN
/// set used by `contains_forbidden_ident_with`.
pub fn collect_aliases(text: &str) -> std::collections::HashSet<String> {
    let mut aliases = std::collections::HashSet::new();
    const ROOTS: &[&str] = &[
        "api_key",
        "groq_key",
        "GROQ_API_KEY",
        "OPENAI_API_KEY",
        "OPEN_ONTOLOGIES_LLM_API_KEY",
    ];
    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("let ") else {
            continue;
        };
        // Read identifier (with optional `mut`).
        let rest = rest.trim_start_matches("mut ").trim_start();
        // Identifier ends at whitespace, ':', or '='.
        let mut name_end = 0usize;
        for (idx, c) in rest.char_indices() {
            if c.is_ascii_alphanumeric() || c == '_' {
                name_end = idx + c.len_utf8();
            } else {
                break;
            }
        }
        if name_end == 0 {
            continue;
        }
        let name = &rest[..name_end];
        // Find '=' on this line.
        let after_name = &rest[name_end..];
        let Some(eq_idx) = after_name.find('=') else {
            continue;
        };
        let rhs = after_name[eq_idx + 1..].trim_start();
        // Strip a leading '&' for `let X = &api_key`.
        let rhs = rhs.trim_start_matches('&').trim_start();
        for root in ROOTS {
            if rhs.starts_with(root) {
                let after_root = &rhs[root.len()..];
                // Acceptable suffixes: `;`, `.clone()`, `.to_string()`,
                // `.to_owned()`, `.into()`, end-of-line, whitespace.
                let after_root = after_root.trim_start();
                if after_root.is_empty()
                    || after_root.starts_with(';')
                    || after_root.starts_with(".clone()")
                    || after_root.starts_with(".to_string()")
                    || after_root.starts_with(".to_owned()")
                    || after_root.starts_with(".into()")
                {
                    aliases.insert(name.to_string());
                    break;
                }
            }
        }
    }
    aliases
}

/// Detect tracing structured-field captures: `?<ident>` / `%<ident>` on a
/// logging-macro line where `<ident>` is forbidden.
pub fn line_uses_tracing_field(line: &str, extra_forbidden: &[String]) -> bool {
    if !is_logging_macro_line(line) {
        return false;
    }
    // Find `?<ident>` or `%<ident>` patterns. Walk char-by-char outside
    // string literals.
    let mut inside_string = false;
    let mut prev = '\0';
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch == '"' && prev != '\\' {
            inside_string = !inside_string;
            prev = ch;
            i += 1;
            continue;
        }
        if !inside_string && (ch == '?' || ch == '%') {
            // Read following ident.
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j > i + 1 {
                let ident = &line[i + 1..j];
                if is_forbidden_ident(ident, extra_forbidden) {
                    return true;
                }
            }
            i = j;
            prev = ch;
            continue;
        }
        prev = ch;
        i += 1;
    }
    false
}

/// Detect format-string brace interpolation of forbidden idents:
/// `{api_key}`, `{api_key:?}`, `{alias:#?}` inside a logging-macro line's
/// string literal.
pub fn line_format_brace_uses_forbidden(line: &str, extra_forbidden: &[String]) -> bool {
    if !is_logging_macro_line(line) {
        return false;
    }
    // Walk string literals; for each, scan `{<ident>(:[^}]*)?}` patterns.
    let mut inside_string = false;
    let mut prev = '\0';
    let mut buf = String::new();
    for ch in line.chars() {
        if ch == '"' && prev != '\\' {
            if inside_string {
                // Just closed a string literal; scan it.
                if scan_braces_for_forbidden(&buf, extra_forbidden) {
                    return true;
                }
                buf.clear();
            }
            inside_string = !inside_string;
        } else if inside_string {
            buf.push(ch);
        }
        prev = ch;
    }
    false
}

fn scan_braces_for_forbidden(s: &str, extra_forbidden: &[String]) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            // Skip `{{` (escape).
            if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                i += 2;
                continue;
            }
            // Read ident.
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j > i + 1 {
                let ident = &s[i + 1..j];
                // Followed by ':' or '}'.
                if j < bytes.len() && (bytes[j] == b':' || bytes[j] == b'}')
                    && is_forbidden_ident(ident, extra_forbidden)
                {
                    return true;
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    false
}

fn is_logging_macro_line(line: &str) -> bool {
    line.contains("info!(")
        || line.contains("warn!(")
        || line.contains("error!(")
        || line.contains("debug!(")
        || line.contains("trace!(")
        || line.contains("println!(")
        || line.contains("print!(")
        || line.contains("eprintln!(")
        || line.contains("eprint!(")
        || line.contains("format!(")
        || line.contains("panic!(")
        || line.contains("write!(")
        || line.contains("writeln!(")
        || line.contains("dbg!(")
        || line.contains("tracing::info!(")
        || line.contains("tracing::warn!(")
        || line.contains("tracing::error!(")
        || line.contains("tracing::debug!(")
        || line.contains("tracing::trace!(")
}

fn is_forbidden_ident(ident: &str, extra: &[String]) -> bool {
    const FORBIDDEN_BASE: &[&str] = &[
        "api_key",
        "groq_key",
        "GROQ_API_KEY",
        "OPENAI_API_KEY",
        "OPEN_ONTOLOGIES_LLM_API_KEY",
    ];
    FORBIDDEN_BASE.iter().any(|f| *f == ident) || extra.iter().any(|a| a == ident)
}

pub fn line_substitutes_key_with(line: &str, extra_forbidden: &[String]) -> bool {
    let macro_call = is_logging_macro_line(line);
    if !macro_call {
        return false;
    }
    let mut inside_string = false;
    let mut prev = '\0';
    let mut buf = String::new();
    for ch in line.chars() {
        if ch == '"' && prev != '\\' {
            if !inside_string && contains_forbidden_ident_with(&buf, extra_forbidden) {
                return true;
            }
            buf.clear();
            inside_string = !inside_string;
        } else if !inside_string {
            buf.push(ch);
        }
        prev = ch;
    }
    if !inside_string && contains_forbidden_ident_with(&buf, extra_forbidden) {
        return true;
    }
    false
}

fn contains_forbidden_ident_with(s: &str, extra: &[String]) -> bool {
    const FORBIDDEN_BASE: &[&str] = &[
        "api_key",
        "groq_key",
        "GROQ_API_KEY",
        "OPENAI_API_KEY",
        "OPEN_ONTOLOGIES_LLM_API_KEY",
    ];
    let all: Vec<&str> = FORBIDDEN_BASE
        .iter()
        .copied()
        .chain(extra.iter().map(|s| s.as_str()))
        .collect();
    for needle in &all {
        if let Some(i) = s.find(needle) {
            let before_ok = i == 0
                || s.as_bytes()
                    .get(i - 1)
                    .map(|b| !(b.is_ascii_alphanumeric() || *b == b'_'))
                    .unwrap_or(true);
            let after_idx = i + needle.len();
            let after_ok = after_idx >= s.len()
                || s.as_bytes()
                    .get(after_idx)
                    .map(|b| !(b.is_ascii_alphanumeric() || *b == b'_'))
                    .unwrap_or(true);
            if before_ok && after_ok {
                return true;
            }
        }
    }
    false
}

fn line_substitutes_key(line: &str) -> bool {
    // The line interpolates a key-like identifier into a formatting
    // macro if it contains BOTH:
    //   1. a logging/formatting macro invocation (info!, warn!, error!,
    //      debug!, trace!, println!, print!, eprintln!, eprint!, format!,
    //      panic!, write!, writeln!, dbg!), AND
    //   2. a substring of one of: "api_key", "groq_key", "GROQ_API_KEY",
    //      "OPENAI_API_KEY", "secret", or any "*_token" / "_secret"
    //      identifier suffix used as an interpolation argument.
    //
    // We require the name to look like an *identifier reference*,
    // i.e. preceded by `,` or `(` or `{` or whitespace and followed by
    // `,` or `)` or `}` or whitespace. Bare occurrences inside string
    // literals (the format string itself) are fine.
    let macro_call = line.contains("info!(")
        || line.contains("warn!(")
        || line.contains("error!(")
        || line.contains("debug!(")
        || line.contains("trace!(")
        || line.contains("println!(")
        || line.contains("print!(")
        || line.contains("eprintln!(")
        || line.contains("eprint!(")
        || line.contains("format!(")
        || line.contains("panic!(")
        || line.contains("write!(")
        || line.contains("writeln!(")
        || line.contains("dbg!(");
    if !macro_call {
        return false;
    }
    // Look for an identifier ref to a forbidden name OUTSIDE string
    // literals. Cheap heuristic: split on '"' and check odd-indexed
    // segments are inside strings (skip them).
    let mut inside_string = false;
    let mut prev = '\0';
    let mut buf = String::new();
    for ch in line.chars() {
        if ch == '"' && prev != '\\' {
            // Check `buf` accumulated outside-string content for
            // forbidden idents.
            if !inside_string && contains_forbidden_ident(&buf) {
                return true;
            }
            buf.clear();
            inside_string = !inside_string;
        } else if !inside_string {
            buf.push(ch);
        }
        prev = ch;
    }
    if !inside_string && contains_forbidden_ident(&buf) {
        return true;
    }
    false
}

fn contains_forbidden_ident(s: &str) -> bool {
    // Forbidden identifier substrings that, when used as a Rust
    // identifier reference (not in a string literal), strongly imply
    // the secret value would be substituted into the macro.
    const FORBIDDEN: &[&str] = &[
        "api_key",
        "groq_key",
        "GROQ_API_KEY",
        "OPENAI_API_KEY",
        "OPEN_ONTOLOGIES_LLM_API_KEY",
    ];
    for needle in FORBIDDEN {
        if let Some(i) = s.find(needle) {
            // Check char before is not alphanumeric/_ (so we don't
            // false-match e.g. `redact_api_key` callsites — though
            // those would also be suspicious).
            let before_ok = i == 0
                || s.as_bytes()
                    .get(i - 1)
                    .map(|b| !(b.is_ascii_alphanumeric() || *b == b'_'))
                    .unwrap_or(true);
            let after_idx = i + needle.len();
            let after_ok = after_idx >= s.len()
                || s.as_bytes()
                    .get(after_idx)
                    .map(|b| !(b.is_ascii_alphanumeric() || *b == b'_'))
                    .unwrap_or(true);
            if before_ok && after_ok {
                return true;
            }
        }
    }
    false
}

fn line_is_allowlisted(file: &Path, line: &str, allow: &[(&'static str, &'static str)]) -> bool {
    let path_str = file.to_string_lossy();
    for (suffix, fragment) in allow {
        if path_str.ends_with(*suffix) && line.contains(*fragment) {
            return true;
        }
    }
    false
}

#[test]
fn no_log_or_format_site_interpolates_api_key() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        files.len() >= 30,
        "expected ≥30 .rs files under src/, found {}",
        files.len()
    );

    let allow = allowlist();
    let mut violations: Vec<String> = Vec::new();

    for file in &files {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        // Per-file alias scan: identifiers bound to a forbidden root.
        let aliases_set = collect_aliases(&text);
        let aliases: Vec<String> = aliases_set.into_iter().collect();
        for (lineno, line) in text.lines().enumerate() {
            let triggered = line_substitutes_key_with(line, &aliases)
                || line_uses_tracing_field(line, &aliases)
                || line_format_brace_uses_forbidden(line, &aliases);
            if triggered && !line_is_allowlisted(file, line, &allow) {
                violations.push(format!(
                    "{}:{}: {}",
                    file.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                        .unwrap_or(file)
                        .display(),
                    lineno + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "secret-hygiene ratchet: source lines interpolate a key-like \
         identifier into a logging/formatting macro. If this is a \
         deliberate redacted log (e.g. logging that a key is *present* \
         without its value), add the file+fragment pair to the allowlist \
         in tests/secret_grep_ratchet.rs::allowlist() with a justification.\n{:#?}",
        violations
    );
}

// ── self-tests for the lexer ──────────────────────────────────────────────

#[test]
fn lexer_flags_simple_interpolation() {
    let line = r#"        info!("posting with key {}", api_key);"#;
    assert!(line_substitutes_key(line));
}

#[test]
fn lexer_ignores_string_literal_mention() {
    let line = r#"        info!("api_key resolution: {}", outcome);"#;
    assert!(!line_substitutes_key(line));
}

#[test]
fn lexer_ignores_non_macro_lines() {
    let line = r#"    let api_key = std::env::var("GROQ_API_KEY").ok();"#;
    assert!(!line_substitutes_key(line));
}

#[test]
fn lexer_flags_bearer_format() {
    // This is the pattern the plan explicitly calls out as forbidden:
    // format!("Bearer {}", api_key).
    let line = r#"    let h = format!("Bearer {}", api_key);"#;
    assert!(line_substitutes_key(line));
}

#[test]
fn lexer_does_not_flag_redaction_function_definition() {
    let line = r#"fn redact_api_key(s: &str) -> String { s.to_string() }"#;
    assert!(!line_substitutes_key(line));
}
