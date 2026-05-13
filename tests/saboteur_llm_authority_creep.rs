//! R7 WG-1 saboteur — synthesise an adversarial source string where
//! `parsed.fields["ctq_text"]` flows directly into `Receipt::new`. The
//! audit visitor (rebuilt for this test) MUST flag it.
//!
//! ## Why this exists
//!
//! `tests/llm_authority_zero.rs` walks the production tree and asserts
//! "no flow." This file proves the visitor would actually catch a
//! REAL bypass — closes the §17 fake-gauge nightmare where an "always
//! green" audit is asserted against an empty set of inputs.
//!
//! The visitor logic lives in `tests/llm_authority_zero.rs` and we
//! re-implement the structural matching here against a synthetic
//! `syn::File`. (Cross-test-binary symbol sharing isn't worth the
//! `pub mod` surface — this is one self-contained scan.)

use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprField, ExprIndex, ExprMethodCall, File, Lit, Member};

const LLM_FIELDS: &[&str] = &[
    "ctq_text",
    "measure_text",
    "verification_text",
    "negative_case_text",
    "control_plan_text",
    "defect_class_hint",
    "source_voice_echo",
];

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
];

const RECEIPT_CONSTRUCTOR_TYPES: &[&str] = &[
    "Receipt",
    "ReceiptChain",
    "ProductionRecord",
    "AdmissionDecision",
];

fn unwrap_method_chain(expr: &Expr) -> &Expr {
    let mut cur = expr;
    loop {
        if let Expr::MethodCall(m) = cur {
            let name = m.method.to_string();
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

fn match_candidate_field(expr: &Expr) -> Option<String> {
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
                    return Some(name);
                }
            }
        }
    }
    None
}

fn match_parsed_fields_index(expr: &Expr) -> Option<String> {
    let inner = unwrap_method_chain(expr);
    let Expr::Index(ExprIndex { expr: base, index, .. }) = inner else {
        return None;
    };
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
        if let Expr::Lit(lit) = &**index
            && let Lit::Str(s) = &lit.lit
        {
            return Some(s.value());
        }
    }
    None
}

fn sink_path_name(call_func: &Expr) -> Option<String> {
    if let Expr::Path(p) = call_func {
        let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
        if segs.len() == 2 && segs[1] == "new" && RECEIPT_CONSTRUCTOR_TYPES.contains(&segs[0].as_str()) {
            return Some(format!("{}::new", segs[0]));
        }
        if let Some(last) = segs.last()
            && RECEIPT_SINKS.contains(&last.as_str())
        {
            return Some(last.clone());
        }
    }
    None
}

#[derive(Default)]
struct V {
    flagged: Vec<String>,
}

impl<'ast> Visit<'ast> for V {
    fn visit_expr_call(&mut self, c: &'ast ExprCall) {
        if let Some(sink) = sink_path_name(&c.func) {
            for arg in &c.args {
                if let Some(field) = match_candidate_field(arg) {
                    self.flagged.push(format!("{}: candidate.{}", sink, field));
                }
                if let Some(field) = match_parsed_fields_index(arg) {
                    self.flagged.push(format!("{}: parsed.fields[\"{}\"]", sink, field));
                }
            }
        }
        syn::visit::visit_expr_call(self, c);
    }

    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        let name = m.method.to_string();
        if RECEIPT_SINKS.contains(&name.as_str()) {
            for arg in &m.args {
                if let Some(field) = match_candidate_field(arg) {
                    self.flagged.push(format!("{}: candidate.{}", name, field));
                }
                if let Some(field) = match_parsed_fields_index(arg) {
                    self.flagged.push(format!("{}: parsed.fields[\"{}\"]", name, field));
                }
            }
        }
        syn::visit::visit_expr_method_call(self, m);
    }
}

#[test]
fn b1_parsed_fields_index_into_receipt_new_is_flagged() {
    // Adversarial source: `parsed.fields["ctq_text"]` is passed
    // directly to `Receipt::new(...)`. This is the exact authority
    // bypass §22 forbids.
    let synthetic = r#"
        fn poison() {
            let parsed = some_translator();
            let _r = Receipt::new(parsed.fields["ctq_text"]);
        }
    "#;
    let file: File = syn::parse_str(synthetic).expect("synthetic parses");
    let mut v = V::default();
    v.visit_file(&file);
    assert!(
        v.flagged.iter().any(|s| s.contains("Receipt::new") && s.contains("ctq_text")),
        "saboteur missed: parsed.fields[\"ctq_text\"] flows into Receipt::new \
         but visitor did not flag.\nflagged={:?}",
        v.flagged
    );
}

#[test]
fn b2_candidate_field_clone_into_evaluate_admission_is_flagged() {
    // `candidate.measure_text.clone()` — the .clone() projection MUST
    // not hide the LLM-output identity from the visitor.
    let synthetic = r#"
        fn poison(this: &Server, candidate: &CandidateCtq) {
            this.evaluate_admission(
                AdmissionOp::CtqAdmitted,
                None,
                "ctq",
                candidate.measure_text.clone(),
                false,
                None,
            );
        }
    "#;
    let file: File = syn::parse_str(synthetic).expect("synthetic parses");
    let mut v = V::default();
    v.visit_file(&file);
    assert!(
        v.flagged.iter().any(|s| s.contains("evaluate_admission") && s.contains("measure_text")),
        "saboteur missed: candidate.measure_text.clone() flows into \
         evaluate_admission but visitor did not flag.\nflagged={:?}",
        v.flagged
    );
}

#[test]
fn b3_parsed_fields_into_emit_event_is_flagged() {
    let synthetic = r#"
        fn poison(this: &Server, parsed: &ParsedFields) {
            this.emit_event("ctq_admitted", parsed.fields["verification_text"]);
        }
    "#;
    let file: File = syn::parse_str(synthetic).expect("synthetic parses");
    let mut v = V::default();
    v.visit_file(&file);
    assert!(
        v.flagged.iter().any(|s| s.contains("emit_event") && s.contains("verification_text")),
        "saboteur missed: parsed.fields[\"verification_text\"] flows into \
         emit_event but visitor did not flag.\nflagged={:?}",
        v.flagged
    );
}

#[test]
fn n1_legitimate_admission_input_is_not_flagged() {
    // Negative control: passing a sanitized `input.scope_token` (which
    // is NOT an LLM-output identifier — it's a session token from
    // admission) MUST NOT trip the visitor. Also confirms that
    // tracing-style logging is not in scope.
    let synthetic = r#"
        fn ok(this: &Server, input: &Inputs) {
            this.emit_event("scope", input.scope_token.as_str());
            tracing::info!(?input.scope_token, "scope read");
        }
    "#;
    let file: File = syn::parse_str(synthetic).expect("synthetic parses");
    let mut v = V::default();
    v.visit_file(&file);
    assert!(
        v.flagged.is_empty(),
        "false positive — `input.scope_token` is not an LLM identifier \
         and must not flag.\nflagged={:?}",
        v.flagged
    );
}
