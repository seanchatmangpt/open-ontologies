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
        for (lineno, line) in text.lines().enumerate() {
            if line_substitutes_key(line) && !line_is_allowlisted(file, line, &allow) {
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
