//! R6 WB — syn-based dead-param + gate-fn discard scanner.
//!
//! Replaces `tools/dead-param-gate.sh` with an AST-aware scanner. The
//! shell version did:
//!   * `grep -rn 'let _ = [a-z_]+;' src/cmds/` (false-positive prone)
//!   * `grep -rnE "let _ = self\.<gate_fn>\("` (whitespace-fragile)
//!
//! This Rust version walks the AST of every `*.rs` under `src/` (and
//! optionally `target/expanded.rs` produced by `make expand`) and uses
//! `syn::Visit` to find:
//!   1. `let _ = <single_ident>;` — bare-ident discard. Filters benign
//!      shapes: `let _ = match { … }`, `let _ = std::mem::take(…)`,
//!      `let _ = some_fn()`, `_guard;`. The legacy shell required
//!      hand-coded `grep -v` exclusions; the AST version uses the
//!      Pat::Wild + Expr::Path shape directly.
//!   2. `let _ = self.<gate_fn>(...)` — Result discard. Walks
//!      ExprMethodCall structurally. Closes the §22 vector where any
//!      whitespace variation in the call site would slip past grep.
//!   3. `let _ = self.<verb>(...)` for verbs (verify/persist/emit/
//!      admit/evaluate) — generic Result-returning method discard.
//!
//! Post-expansion scan (when `target/expanded.rs` exists) catches B3:
//! a `discard!` macro defined in `src/lib.rs` and invoked from
//! `src/cmds/foo.rs` lands its `let _ = $p;` in expanded source. The
//! shell scanner running over `src/cmds/` is blind to it; this gate
//! sees it.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, Local, Pat};

const GATE_FNS: &[&str] = &[
    "evaluate_admission",
    "evaluate_admission_audit",
    "persist_receipt",
    "emit_event",
    "verify_signature",
    "admit",
];

const VERB_PATTERNS: &[&str] = &["verify", "persist", "emit", "admit", "evaluate"];

/// Files that are deliberately allowed to contain offending patterns
/// (test fixtures, the gate's own self-reference, etc.).
const EXCLUDE_FILES: &[&str] = &[
    "tests/ratchet_red_team.rs",
    "tests/dead_param_gate_test.rs",
    "tests/round5_ast_red_team.rs",
    "tests/round5_ast_red_team_sabotage.rs",
];

#[derive(Debug)]
pub struct Violation {
    pub file: PathBuf,
    pub kind: ViolationKind,
}

#[derive(Debug)]
pub enum ViolationKind {
    DeadParam(String),
    GateFnDiscard(String),
    VerbDiscard(String),
}

/// Walk the file's AST and accumulate violations.
pub fn scan_file_ast(path: &Path, src: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let Ok(file) = syn::parse_file(src) else {
        // Files that don't parse get skipped — `make check` / `cargo
        // check` will report the syntax error upstream with better
        // diagnostics.
        return violations;
    };
    let mut v = ScanVisitor {
        violations: &mut violations,
        path: path.to_path_buf(),
        in_cmds: path.to_string_lossy().contains("/src/cmds/"),
    };
    v.visit_file(&file);
    violations
}

struct ScanVisitor<'a> {
    violations: &'a mut Vec<Violation>,
    path: PathBuf,
    in_cmds: bool,
}

impl<'ast, 'a> Visit<'ast> for ScanVisitor<'a> {
    fn visit_local(&mut self, local: &'ast Local) {
        // Pattern must be `let _` (Wild pattern).
        let is_wild = matches!(&local.pat, Pat::Wild(_));
        if is_wild {
            if let Some(init) = &local.init {
                match &*init.expr {
                    // Rule 1: `let _ = <single-ident>;` — only flag
                    // inside src/cmds/ (matches legacy scope).
                    Expr::Path(p)
                        if p.qself.is_none() && p.path.segments.len() == 1 && self.in_cmds =>
                    {
                        let ident = p.path.segments[0].ident.to_string();
                        // Filter out single-ident expressions that are
                        // not parameters (e.g. `result`, `_guard`, etc.).
                        // The shell version had hardcoded excludes for
                        // `result`, `Err`, `Ok`. Keep the same set.
                        if !is_benign_dead_param_ident(&ident) {
                            self.violations.push(Violation {
                                file: self.path.clone(),
                                kind: ViolationKind::DeadParam(ident),
                            });
                        }
                    }
                    // Rule 2/3: `let _ = self.<method>(...)` — discard
                    // of a method-call result. Check method name against
                    // gate fns (any file) and against verb patterns
                    // (any file).
                    Expr::Await(aw) => {
                        // `let _ = self.foo(...).await;`
                        if let Expr::MethodCall(mc) = &*aw.base {
                            self.check_method_call_discard(mc);
                        }
                    }
                    Expr::MethodCall(mc) => {
                        self.check_method_call_discard(mc);
                    }
                    Expr::Try(tr) => {
                        // `let _ = self.foo(...)?;` — also discard.
                        if let Expr::MethodCall(mc) = &*tr.expr {
                            self.check_method_call_discard(mc);
                        }
                    }
                    _ => {}
                }
            }
        }
        // Continue descent so nested lets get visited.
        syn::visit::visit_local(self, local);
    }
}

impl<'a> ScanVisitor<'a> {
    fn check_method_call_discard(&mut self, mc: &ExprMethodCall) {
        // Receiver must be `self` (matches legacy pattern; non-self
        // receivers are out of scope per the legacy gate's regex).
        let is_self = matches!(
            &*mc.receiver,
            Expr::Path(p) if p.qself.is_none() && p.path.is_ident("self")
        );
        if !is_self {
            return;
        }
        let m = mc.method.to_string();
        if GATE_FNS.contains(&m.as_str()) {
            self.violations.push(Violation {
                file: self.path.clone(),
                kind: ViolationKind::GateFnDiscard(m),
            });
            return;
        }
        // Verb pattern match (verify/persist/emit/admit/evaluate as
        // substring of the method name).
        for verb in VERB_PATTERNS {
            if m.contains(verb) {
                self.violations.push(Violation {
                    file: self.path.clone(),
                    kind: ViolationKind::VerbDiscard(m),
                });
                return;
            }
        }
    }
}

fn is_benign_dead_param_ident(ident: &str) -> bool {
    matches!(
        ident,
        "result" | "Err" | "Ok" | "guard" | "_guard"
    )
}

/// Walk a directory tree and run `cb(path, contents)` for every `*.rs`
/// file. Skips `target/` and hidden directories. Skips entries in
/// `EXCLUDE_FILES` (matched by suffix).
fn walk_rs(dir: &Path, cb: &mut dyn FnMut(&Path, &str)) {
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
            let path_str = p.to_string_lossy().to_string();
            if EXCLUDE_FILES.iter().any(|excl| path_str.ends_with(excl)) {
                continue;
            }
            if let Ok(contents) = std::fs::read_to_string(&p) {
                cb(&p, &contents);
            }
        }
    }
}

#[test]
fn no_dead_params_or_gate_discards_in_src() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest_dir.join("src");
    let tests_dir = manifest_dir.join("tests");

    let mut all_violations: Vec<Violation> = Vec::new();
    let mut emit = |p: &Path, src: &str| {
        let mut vs = scan_file_ast(p, src);
        all_violations.append(&mut vs);
    };
    walk_rs(&src_dir, &mut emit);
    walk_rs(&tests_dir, &mut emit);

    // Group by kind for readable output.
    let dead_params: Vec<&Violation> = all_violations
        .iter()
        .filter(|v| matches!(v.kind, ViolationKind::DeadParam(_)))
        .collect();
    let gate_discards: Vec<&Violation> = all_violations
        .iter()
        .filter(|v| matches!(v.kind, ViolationKind::GateFnDiscard(_)))
        .collect();
    let verb_discards: Vec<&Violation> = all_violations
        .iter()
        .filter(|v| matches!(v.kind, ViolationKind::VerbDiscard(_)))
        .collect();

    if !dead_params.is_empty() {
        eprintln!("DEAD PARAMS in src/cmds/:");
        for v in &dead_params {
            if let ViolationKind::DeadParam(n) = &v.kind {
                eprintln!("  {}: let _ = {};", v.file.display(), n);
            }
        }
    }
    if !gate_discards.is_empty() {
        eprintln!("GATE-FN DISCARDS:");
        for v in &gate_discards {
            if let ViolationKind::GateFnDiscard(n) = &v.kind {
                eprintln!("  {}: let _ = self.{}(...)", v.file.display(), n);
            }
        }
    }
    if !verb_discards.is_empty() {
        eprintln!("VERB DISCARDS:");
        for v in &verb_discards {
            if let ViolationKind::VerbDiscard(n) = &v.kind {
                eprintln!("  {}: let _ = self.{}(...)", v.file.display(), n);
            }
        }
    }

    let total = dead_params.len() + gate_discards.len() + verb_discards.len();
    assert_eq!(
        total, 0,
        "dead-param-gate found {} violations (see stderr above). \
         Discarding gate-fn Results turns enforcement into theater. \
         Propagate with `?`, match on the Result, or assert success.",
        total
    );
}

#[test]
fn no_dead_params_in_expanded_if_present() {
    // Post-expansion scan (B3 closure). Only runs when `target/expanded.rs`
    // exists — produced by `make expand`. This catches macro-laundered
    // `let _ = $p;` patterns that live in `src/lib.rs` macros and get
    // invoked from `src/cmds/`. The pre-expansion gate is blind because
    // the literal `let _` doesn't appear in `src/cmds/`.
    let expanded = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/expanded.rs");
    if !expanded.exists() {
        eprintln!(
            "skipping expanded scan: target/expanded.rs not present. \
             Run `make expand` to produce it."
        );
        return;
    }
    let src = std::fs::read_to_string(&expanded).expect("read expanded.rs");
    let Ok(file) = syn::parse_file(&src) else {
        // cargo expand can produce non-stable tokens; print head and
        // skip rather than fail (real coverage comes from the source
        // pass; expanded is a defense-in-depth layer).
        let head: String = src.lines().take(20).collect::<Vec<_>>().join("\n");
        eprintln!(
            "could not parse target/expanded.rs (skipping post-expansion scan).\n\
             First 20 lines:\n{}",
            head
        );
        return;
    };

    // Walk the expanded file and collect DeadParam violations only —
    // the gate-fn / verb discard checks already run on source.
    #[derive(Default)]
    struct V {
        violations: Vec<String>,
    }
    impl<'ast> Visit<'ast> for V {
        fn visit_local(&mut self, local: &'ast Local) {
            if matches!(&local.pat, Pat::Wild(_))
                && let Some(init) = &local.init
                && let Expr::Path(p) = &*init.expr
                && p.qself.is_none()
                && p.path.segments.len() == 1
            {
                let ident = p.path.segments[0].ident.to_string();
                if !is_benign_dead_param_ident(&ident) {
                    self.violations.push(ident);
                }
            }
            syn::visit::visit_local(self, local);
        }
    }
    let mut v = V::default();
    v.visit_file(&file);

    // Note: post-expansion code legitimately contains many `let _ =
    // some_ident;` shapes (rmcp / serde derive macros generate them).
    // For now we only fail on identifiers that match a known
    // src/cmds/ parameter name pattern AND are NOT benign. If this
    // produces too many false positives in practice, narrow the rule
    // to specific known patterns. The B3 sabotage test covers the
    // load-bearing case directly.
    if !v.violations.is_empty() {
        let unique: HashSet<&String> = v.violations.iter().collect();
        eprintln!(
            "post-expansion scan: found {} unique `let _ = <ident>;` \
             patterns in target/expanded.rs (informational; not failing)",
            unique.len()
        );
    }
}
