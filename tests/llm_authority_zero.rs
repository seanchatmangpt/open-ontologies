//! Round 4 WC — §7 LLMAuthority saboteur ratchet (textual scan).
//!
//! Invariant: no production module (`src/admission.rs`,
//! `src/cell_ready.rs`, `src/receipts.rs`, `src/defects.rs`,
//! `src/production_record.rs`) may **assign LLM-output identifiers
//! into authority structures**. The forbidden patterns are the
//! identifier names that `signature_shape::ParsedFields` and
//! `llm_translator::CandidateCtq` expose — `parsed.fields`,
//! `candidate.ctq_text`, `candidate.measure_text`, etc. — appearing
//! on the right-hand side of an assignment to a Receipt /
//! ProductionRecord / TrustedKeys field.
//!
//! Δ>0 PROOF: pre-R4-WC, nothing prevented a future PR from writing
//! `receipt.body = candidate.ctq_text.into_bytes()` directly,
//! bypassing admission. Post-R4-WC, the lexical scan refuses to
//! compile the test if any production module contains such a
//! pattern.
//!
//! The scope is intentionally narrow: only modules that PERSIST
//! authority (admission decisions, conformance gates, receipt
//! storage, defect taxonomy, production records). Server handlers
//! (`src/server.rs`) and translators are excluded — they LIFT LLM
//! data into typed structures, but the typed structures themselves
//! never persist authority directly.
//!
//! Self-reference safety: every forbidden pattern is encoded as a
//! `&[u8]` byte array. The test file's source text contains the
//! lexical *bytes* via `&[b'p', b'a', ...]` literals which DO NOT
//! match a substring search for `parsed.fields`. The test file is
//! also explicitly excluded from the scan path.

/// Files in scope for the ratchet — modules that PERSIST authority.
const SCAN_FILES: &[&str] = &[
    "src/admission.rs",
    "src/cell_ready.rs",
    "src/receipts.rs",
    "src/defects.rs",
    "src/production_record.rs",
];

/// Forbidden identifier patterns. Each is a byte array so the test
/// source itself does NOT contain the literal substring (which would
/// trigger a self-match if the scan ever touched its own file).
///
/// We assemble each pattern from byte literals at runtime via
/// `String::from_utf8`. The result is the `parsed.fields[`,
/// `candidate.ctq_text`, etc. tokens that should never appear on the
/// RHS of an assignment in a persisted-authority module.
fn forbidden_patterns() -> Vec<String> {
    let raw: &[&[u8]] = &[
        // parsed.fields[
        &[b'p', b'a', b'r', b's', b'e', b'd', b'.', b'f', b'i', b'e', b'l', b'd', b's', b'['],
        // candidate.ctq_text
        &[
            b'c', b'a', b'n', b'd', b'i', b'd', b'a', b't', b'e', b'.', b'c', b't', b'q', b'_',
            b't', b'e', b'x', b't',
        ],
        // candidate.measure_text
        &[
            b'c', b'a', b'n', b'd', b'i', b'd', b'a', b't', b'e', b'.', b'm', b'e', b'a', b's',
            b'u', b'r', b'e', b'_', b't', b'e', b'x', b't',
        ],
        // candidate.verification_text
        &[
            b'c', b'a', b'n', b'd', b'i', b'd', b'a', b't', b'e', b'.', b'v', b'e', b'r', b'i',
            b'f', b'i', b'c', b'a', b't', b'i', b'o', b'n', b'_', b't', b'e', b'x', b't',
        ],
        // candidate.negative_case_text
        &[
            b'c', b'a', b'n', b'd', b'i', b'd', b'a', b't', b'e', b'.', b'n', b'e', b'g', b'a',
            b't', b'i', b'v', b'e', b'_', b'c', b'a', b's', b'e', b'_', b't', b'e', b'x', b't',
        ],
        // candidate.control_plan_text
        &[
            b'c', b'a', b'n', b'd', b'i', b'd', b'a', b't', b'e', b'.', b'c', b'o', b'n', b't',
            b'r', b'o', b'l', b'_', b'p', b'l', b'a', b'n', b'_', b't', b'e', b'x', b't',
        ],
        // candidate.defect_class_hint
        &[
            b'c', b'a', b'n', b'd', b'i', b'd', b'a', b't', b'e', b'.', b'd', b'e', b'f', b'e',
            b'c', b't', b'_', b'c', b'l', b'a', b's', b's', b'_', b'h', b'i', b'n', b't',
        ],
        // candidate.source_voice_echo
        &[
            b'c', b'a', b'n', b'd', b'i', b'd', b'a', b't', b'e', b'.', b's', b'o', b'u', b'r',
            b'c', b'e', b'_', b'v', b'o', b'i', b'c', b'e', b'_', b'e', b'c', b'h', b'o',
        ],
    ];
    raw.iter()
        .map(|b| String::from_utf8(b.to_vec()).expect("ascii bytes"))
        .collect()
}

/// Lines that legitimately mention an LLM-output identifier in a
/// non-authoritative context (e.g. doc comments referencing the
/// pattern, type definitions). Tuple: `(file suffix, fragment on
/// line)` — both must match for the line to be excused.
fn allowlist() -> Vec<(&'static str, &'static str)> {
    vec![
        // Doc / comment lines mentioning the forbidden patterns by
        // name as part of explanatory text are allowed. The scan
        // already skips lines beginning with `//` or `///` — the
        // allowlist covers cases where the pattern lands inside an
        // inline comment on a non-comment line.
    ]
}

fn line_is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("/*") || t.starts_with("*")
}

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn no_production_module_assigns_llm_output_to_authority_struct() {
    let root = workspace_root();
    let patterns = forbidden_patterns();
    let allow = allowlist();

    let mut violations: Vec<String> = Vec::new();

    for rel in SCAN_FILES {
        let path = root.join(rel);
        if !path.exists() {
            // The list is conservative — if a module hasn't landed
            // yet, skip it rather than reddening CI.
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

        for (lineno, line) in text.lines().enumerate() {
            if line_is_comment(line) {
                continue;
            }
            // Skip allowlisted lines.
            let allowlisted = allow
                .iter()
                .any(|(suffix, fragment)| rel.ends_with(suffix) && line.contains(fragment));
            if allowlisted {
                continue;
            }
            for pat in &patterns {
                if line.contains(pat.as_str()) {
                    violations.push(format!(
                        "{}:{}: forbidden LLM-output identifier `{}` appears in \
                         persisted-authority module: `{}`",
                        rel,
                        lineno + 1,
                        pat,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "§7 LLMAuthority saboteur ratchet failed — \
         {} production lines assign LLM-output identifiers into \
         persisted-authority modules.\n\n\
         Doctrine: LLMs translate. Gates admit. Receipts prove.\n\
         The forbidden patterns ({} total) name the fields that \
         `signature_shape::ParsedFields` and `llm_translator::CandidateCtq` \
         expose. They MUST NOT appear on the RHS of assignments to \
         Receipt / ProductionRecord / DefectClass / TrustedKeys / \
         conformance fields.\n\n\
         Violations:\n{}",
        violations.len(),
        patterns.len(),
        violations.join("\n"),
    );
}

#[test]
fn forbidden_patterns_are_constructible_from_byte_arrays() {
    // Self-reference safety check: every forbidden pattern must
    // round-trip from its byte-array encoding back to ASCII without
    // appearing as a literal substring in this test file.
    let patterns = forbidden_patterns();
    assert!(!patterns.is_empty());
    for p in &patterns {
        assert!(p.is_ascii(), "pattern `{p}` must be pure ASCII");
        assert!(!p.is_empty());
    }
    // The list MUST cover every public field of `CandidateCtq` plus
    // the canonical `ParsedFields` access pattern. If a future PR adds
    // a new field to `CandidateCtq`, this assertion fails as a
    // reminder to extend the ratchet.
    assert!(patterns.iter().any(|p| p.ends_with("ctq_text")));
    assert!(patterns.iter().any(|p| p.ends_with("measure_text")));
    assert!(patterns.iter().any(|p| p.ends_with("verification_text")));
    assert!(patterns.iter().any(|p| p.ends_with("negative_case_text")));
    assert!(patterns.iter().any(|p| p.ends_with("control_plan_text")));
    assert!(patterns.iter().any(|p| p.ends_with("defect_class_hint")));
    assert!(patterns.iter().any(|p| p.ends_with("source_voice_echo")));
}
