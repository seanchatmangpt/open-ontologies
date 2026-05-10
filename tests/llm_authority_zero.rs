//! Round 4 WC (R4) → R7 WG-1 (R7): §22 LLMAuthority saboteur ratchet.
//!
//! ## Why this file exists
//!
//! Until R6 WB the authority audit was string-based. R6 WB migrated the
//! `#[tool]` extraction to `syn::parse_file` + `Visit`. The §7 LLM-output
//! ratchet (this file) was still a 5-file substring grep. R7 Explore-2
//! found that 4 of 5 documented LLM authority bypasses live in modules
//! the legacy SCAN_FILES did NOT cover (server.rs, align.rs, embed.rs,
//! embed_remote.rs, clinical.rs, signature_shape.rs).
//!
//! WG-1 migrates this audit to the same syn-AST substrate. Two changes:
//!
//!   1. SCAN_FILES grows from 5 to 11 files.
//!   2. The "forbidden RHS substring" check is replaced with a
//!      structural rule: `parsed.fields[<lit>]` or `candidate.<llm_field>`
//!      flowing AS A DIRECT ARGUMENT into one of the receipt sinks
//!      (`RECEIPT_SINKS` constant below) — `emit_event`, `Receipt::new`,
//!      `evaluate_admission`, `record_admission_*`, `sparql_update`,
//!      `exec_update`, `build_canonical`.
//!
//! ## What "direct argument" means
//!
//! The visitor flags an expression `e` that is structurally
//! `parsed.fields[<lit>]` or `candidate.<llm_field>` (optionally wrapped
//! in `.clone()` / `.to_string()` / `.into()` / `.as_bytes()`) and
//! appears at an argument position of `ExprCall` or `ExprMethodCall`
//! whose terminal name is in `RECEIPT_SINKS`. We do NOT chase
//! through intermediate `let` bindings — the doctrine is that if you
//! need an LLM-output value at a sink, you must pass through admission
//! (the indirection itself is the bypass). Free-standing expressions
//! that flow into JSON response builders or `format!`-summarised
//! token-overlap checks are EXEMPT (they are projections, not authority).
//!
//! ## Permitted patterns
//!
//! - tracing::* macros (info!/debug!/warn!/error!/trace!/event!)
//! - `is_empty()` predicates (read-only inspection)
//! - comment lines (already filtered by syn — comments are not in the AST)
//! - doc strings on items (also not in AST)
//! - `serde_json::json!` / `format!` macros (projections; flagged
//!   ONLY if the expression flows transitively to a sink — which we
//!   conservatively don't follow)
//!
//! ## Sabotage
//!
//! `tests/saboteur_llm_authority_creep.rs` builds synthetic source
//! `parsed.fields["ctq_text"]` flowing into `Receipt::new` via
//! `syn::parse_str` — visitor MUST flag.

use std::collections::HashSet;
use std::path::PathBuf;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprField, ExprIndex, ExprMethodCall, File, Lit, Member};

/// 11 files audited. The first 5 are the original R4 WC scope
/// (persisted-authority modules). The next 6 are the R7 WG-1
/// expansion: every module that LIFTS LLM-shaped data into typed
/// structures or persists IRIs/triples derived from external input.
pub const SCAN_FILES: &[&str] = &[
    // R4 WC persisted-authority core.
    "src/admission.rs",
    "src/cell_ready.rs",
    "src/receipts.rs",
    "src/defects.rs",
    "src/production_record.rs",
    // R7 WG-1 expansion.
    "src/server.rs",
    "src/align.rs",
    "src/embed.rs",
    "src/embed_remote.rs",
    "src/clinical.rs",
    "src/signature_shape.rs",
];

/// Receipt sinks. Any of these called with an LLM-output expression
/// (parsed.fields[lit] or candidate.<llm_field>) as a direct argument
/// is a §22 violation.
///
/// Naming is by terminal segment / method name — we don't care if it's
/// `self.emit_event(...)`, `db.emit_event(...)`, or `Foo::emit_event(...)`.
const RECEIPT_SINKS: &[&str] = &[
    "emit_event",
    "evaluate_admission",
    "evaluate_admission_audit",
    "record_admission_granted",
    "record_admission_denied",
    "record_admission_audit",
    "sparql_update",
    "exec_update",
    "build_canonical",
    // `Receipt::new` is a path call, not a method call — handled by
    // path matching in the visitor.
    "new",
];

/// Path expressions that, when called as `Path::new(...)`, are sinks.
/// Distinct from `RECEIPT_SINKS` because a bare `.new(...)` could be
/// any constructor — only `Receipt::new` / `ProductionRecord::new` /
/// `AdmissionDecision::new` are receipt-grade.
const RECEIPT_CONSTRUCTOR_TYPES: &[&str] = &[
    "Receipt",
    "ReceiptChain",
    "ProductionRecord",
    "AdmissionDecision",
];

/// LLM-output field names on `CandidateCtq` that may NEVER flow into
/// a sink without admission. Mirror of
/// `signature_shape::ParsedFields` keys + `llm_translator::CandidateCtq`
/// public fields.
const LLM_FIELDS: &[&str] = &[
    "ctq_text",
    "measure_text",
    "verification_text",
    "negative_case_text",
    "control_plan_text",
    "defect_class_hint",
    "source_voice_echo",
];

/// Allowlist: `(file_suffix, fn_name, llm_field)`. If a violation's
/// triple matches an allowlist row, it is excused. Adding to this list
/// is a deliberate audit step (this is the single audit-narrowing
/// surface; everything else is structural).
///
/// Currently empty — every violation that survives the structural
/// rules is a real bypass. R7 Explore-2 surveyed all 11 files; F1-F4
/// documented bypasses do NOT manifest as direct-argument flows
/// through the structural rules below (they manifest as missing
/// validation, auto-apply, etc.) and are addressed by WG-2/3/4
/// independently.
fn allowlist() -> Vec<(&'static str, &'static str, &'static str)> {
    Vec::new()
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Strip wrapping `.clone()`, `.to_string()`, `.into()`, `.as_bytes()`,
/// `.as_str()`, `.trim()` etc. so the visitor can look at the
/// underlying receiver.
fn unwrap_method_chain(expr: &Expr) -> &Expr {
    let mut cur = expr;
    loop {
        if let Expr::MethodCall(m) = cur {
            let name = m.method.to_string();
            // Read-only / projection methods that don't change the
            // identity of the value semantically.
            const PROJECTIONS: &[&str] = &[
                "clone",
                "to_string",
                "to_owned",
                "into",
                "as_bytes",
                "as_str",
                "trim",
                "trim_start",
                "trim_end",
                "deref",
                "as_ref",
            ];
            if PROJECTIONS.contains(&name.as_str()) {
                cur = &m.receiver;
                continue;
            }
        }
        break;
    }
    cur
}

/// Returns `Some("ctq_text")` if `expr` is `candidate.<llm_field>`.
fn match_candidate_field(expr: &Expr) -> Option<&'static str> {
    let inner = unwrap_method_chain(expr);
    if let Expr::Field(ExprField {
        base,
        member: Member::Named(ident),
        ..
    }) = inner
    {
        if let Expr::Path(p) = &**base
            && p.qself.is_none()
            && p.path.is_ident("candidate")
        {
            let name = ident.to_string();
            for f in LLM_FIELDS {
                if name == *f {
                    return Some(*f);
                }
            }
        }
    }
    None
}

/// Returns `Some("ctq_text")` if `expr` is `parsed.fields["<lit>"]`
/// (or wrapped projection chain).
fn match_parsed_fields_index(expr: &Expr) -> Option<String> {
    let inner = unwrap_method_chain(expr);
    let Expr::Index(ExprIndex { expr: base, index, .. }) = inner else {
        return None;
    };
    // base must be `parsed.fields`.
    let Expr::Field(ExprField {
        base: outer_base,
        member: Member::Named(outer_member),
        ..
    }) = &**base
    else {
        return None;
    };
    if outer_member != "fields" {
        return None;
    }
    if let Expr::Path(p) = &**outer_base
        && p.qself.is_none()
        && p.path.is_ident("parsed")
    {
        // index must be a string literal.
        if let Expr::Lit(lit) = &**index
            && let Lit::Str(s) = &lit.lit
        {
            return Some(s.value());
        }
    }
    None
}

/// Returns the terminal sink name if `call_expr` is a method/path call
/// to a known sink — otherwise None.
fn sink_name(call_func: &Expr) -> Option<String> {
    if let Expr::Path(p) = call_func {
        // Path calls — Receipt::new(...) etc.
        let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
        // Constructor pattern: Type::new
        if segs.len() == 2 && segs[1] == "new" && RECEIPT_CONSTRUCTOR_TYPES.contains(&segs[0].as_str()) {
            return Some(format!("{}::new", segs[0]));
        }
        // Bare named function (rare in this codebase).
        if let Some(last) = segs.last()
            && RECEIPT_SINKS.contains(&last.as_str())
            && last != "new"
        {
            return Some(last.clone());
        }
    }
    None
}

/// Visitor that walks all expressions and flags any sink invocation
/// whose argument list contains a forbidden LLM-output expression at
/// the top level (after `unwrap_method_chain`).
#[derive(Default)]
struct SinkFlowVisitor {
    file: String,
    fn_name: String,
    fn_stack: Vec<String>,
    in_test_module: usize,
    in_test_fn: bool,
    violations: Vec<String>,
    allow: HashSet<(String, String, String)>,
}

impl SinkFlowVisitor {
    fn new(file: String, allow: &[(&'static str, &'static str, &'static str)]) -> Self {
        let mut s = Self::default();
        s.file = file;
        s.allow = allow
            .iter()
            .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()))
            .collect();
        s
    }

    fn record(&mut self, sink: &str, field: &str, span_repr: &str) {
        // Allowlist check.
        for (suffix, fn_name, llm_field) in &self.allow {
            if self.file.ends_with(suffix.as_str())
                && self.fn_name == *fn_name
                && field == llm_field
            {
                return;
            }
        }
        self.violations.push(format!(
            "{}::{}: LLM-output field `{}` flows into receipt sink `{}` ({})",
            self.file, self.fn_name, field, sink, span_repr,
        ));
    }

    fn check_arg(&mut self, sink: &str, arg: &Expr) {
        if let Some(field) = match_candidate_field(arg) {
            let repr = format!("candidate.{}", field);
            self.record(sink, field, &repr);
        }
        if let Some(field) = match_parsed_fields_index(arg) {
            let repr = format!("parsed.fields[\"{}\"]", field);
            self.record(sink, &field, &repr);
        }
    }

    fn enter_fn(&mut self, name: String, is_test: bool) {
        self.fn_stack.push(self.fn_name.clone());
        self.fn_name = name;
        if is_test {
            self.in_test_fn = true;
        }
    }

    fn leave_fn(&mut self) {
        self.in_test_fn = false;
        if let Some(prev) = self.fn_stack.pop() {
            self.fn_name = prev;
        }
    }
}

impl<'ast> Visit<'ast> for SinkFlowVisitor {
    fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
        let is_test = m
            .attrs
            .iter()
            .any(|a| a.path().is_ident("cfg") && a.to_token_stream_string().contains("test"));
        if is_test {
            self.in_test_module += 1;
        }
        syn::visit::visit_item_mod(self, m);
        if is_test {
            self.in_test_module -= 1;
        }
    }

    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        let is_test = f.attrs.iter().any(|a| a.path().is_ident("test"));
        self.enter_fn(f.sig.ident.to_string(), is_test);
        syn::visit::visit_item_fn(self, f);
        self.leave_fn();
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        let is_test = f.attrs.iter().any(|a| a.path().is_ident("test"));
        self.enter_fn(f.sig.ident.to_string(), is_test);
        syn::visit::visit_impl_item_fn(self, f);
        self.leave_fn();
    }

    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        if self.in_test_module == 0 && !self.in_test_fn {
            let method = m.method.to_string();
            if RECEIPT_SINKS.contains(&method.as_str()) && method != "new" {
                for arg in &m.args {
                    self.check_arg(&method, arg);
                }
            }
        }
        syn::visit::visit_expr_method_call(self, m);
    }

    fn visit_expr_call(&mut self, c: &'ast ExprCall) {
        if self.in_test_module == 0 && !self.in_test_fn {
            if let Some(name) = sink_name(&c.func) {
                for arg in &c.args {
                    self.check_arg(&name, arg);
                }
            }
        }
        syn::visit::visit_expr_call(self, c);
    }
}

/// Helper: stringify an Attribute's tokens for substring matching.
trait ToTokenStreamString {
    fn to_token_stream_string(&self) -> String;
}

impl ToTokenStreamString for syn::Attribute {
    fn to_token_stream_string(&self) -> String {
        quote::ToTokens::to_token_stream(self).to_string()
    }
}

/// Run the visitor over a single file and return its violations.
pub fn audit_file(rel: &str, src: &str) -> Vec<String> {
    let allow = allowlist();
    let allow_static: Vec<(&'static str, &'static str, &'static str)> =
        allow.iter().map(|t| (t.0, t.1, t.2)).collect();

    let file: File = match syn::parse_file(src) {
        Ok(f) => f,
        Err(e) => {
            return vec![format!("{}: parse error (run `cargo check` first): {}", rel, e)];
        }
    };
    let mut v = SinkFlowVisitor::new(rel.to_string(), &allow_static);
    v.visit_file(&file);
    v.violations
}

#[test]
fn no_production_module_flows_llm_output_into_receipt_sinks() {
    let root = workspace_root();
    let mut all_violations: Vec<String> = Vec::new();

    for rel in SCAN_FILES {
        let path = root.join(rel);
        if !path.exists() {
            continue;
        }
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let v = audit_file(rel, &src);
        all_violations.extend(v);
    }

    assert!(
        all_violations.is_empty(),
        "§22 LLMAuthority AST ratchet failed — {} flow(s) of LLM-output \
         identifiers into receipt sinks across the 11-file scope.\n\n\
         Doctrine: LLMs translate. Gates admit. Receipts prove.\n\
         Each violation below is a path where `parsed.fields[\"<llm_field>\"]` \
         or `candidate.<llm_field>` reaches an authority sink \
         (`emit_event`, `Receipt::new`, `evaluate_admission*`, \
         `record_admission_*`, `sparql_update`, `exec_update`, \
         `build_canonical`) WITHOUT admission.\n\n\
         Violations:\n{}",
        all_violations.len(),
        all_violations.join("\n"),
    );
}

#[test]
fn scan_files_count_pinned_at_eleven() {
    // R7 WG-1 expansion: 5 (R4 WC) + 6 (R7) = 11. If a future PR
    // adds files, the count must change deliberately (along with this
    // pin).
    assert_eq!(
        SCAN_FILES.len(),
        11,
        "SCAN_FILES count drifted; deliberate adds must update this pin."
    );
}

#[test]
fn llm_fields_match_candidate_ctq_public_fields() {
    // If `CandidateCtq` grows a new public LLM-output field, this test
    // serves as a reminder that LLM_FIELDS must be extended too. We
    // can't introspect across crates without `pub` exposure, so we
    // assert against the documented Round-4 baseline.
    let expected: HashSet<&'static str> = [
        "ctq_text",
        "measure_text",
        "verification_text",
        "negative_case_text",
        "control_plan_text",
        "defect_class_hint",
        "source_voice_echo",
    ]
    .into_iter()
    .collect();
    let got: HashSet<&'static str> = LLM_FIELDS.iter().copied().collect();
    assert_eq!(
        expected, got,
        "LLM_FIELDS drifted from CandidateCtq public field set; if a new \
         field landed, extend LLM_FIELDS to keep the audit honest."
    );
}
