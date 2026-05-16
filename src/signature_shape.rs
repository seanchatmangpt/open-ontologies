//! DSPy-style signature shapes — the language-to-contract boundary.
//!
//! A `SignatureShape` is the **mold** the LLM fills. It pre-constrains
//! the LLM's output space (DSPy's pattern from
//! `pm4py.algo.dspy.powl.natural_language`) by:
//!
//!   1. **Field semantics** — each input/output field carries a
//!      `description` the prompt builder embeds verbatim.
//!   2. **Demos** — few-shot input/output pairs constrain the shape
//!      before generation. This is the "molding" the paper describes.
//!   3. **Validation** — after the LLM responds, every output field is
//!      checked against required / min_len / allowed_values. Failures
//!      surface a typed [`ValidationFailure`] which the refine-loop
//!      uses to retry.
//!
//! This is **not** SHACL. The full graph-level constraint surface is
//! out of scope — but the same idea applies: the LLM is shaped before
//! it speaks and gauged after. The downstream CTQ admission gate (in
//! `src/admission.rs`) is what *admits*; this layer only constrains.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One field on a signature — input or output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSpec {
    pub name: String,
    pub description: String,
    /// True if the field MUST appear in the LLM's response (output)
    /// or MUST be supplied by the caller (input).
    pub required: bool,
    /// Minimum number of characters after `.trim()`. 0 disables.
    pub min_len: usize,
    /// Maximum characters. None disables.
    pub max_len: Option<usize>,
    /// If `Some`, the value MUST match one of the listed strings
    /// (case-insensitive). Mirrors `sh:in`.
    pub allowed_values: Option<Vec<String>>,
}

impl FieldSpec {
    /// Construct a required field with a 1-character minimum length.
    ///
    /// # Examples
    /// ```
    /// # use open_ontologies::signature_shape::FieldSpec;
    /// let f = FieldSpec::required("voice", "stakeholder voice text");
    /// assert_eq!(f.name, "voice");
    /// assert!(f.required);
    /// assert_eq!(f.min_len, 1);
    /// assert!(f.allowed_values.is_none());
    /// ```
    pub fn required(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            required: true,
            min_len: 1,
            max_len: None,
            allowed_values: None,
        }
    }

    pub fn with_min_len(mut self, n: usize) -> Self {
        self.min_len = n;
        self
    }
    pub fn with_max_len(mut self, n: usize) -> Self {
        self.max_len = Some(n);
        self
    }
    pub fn with_allowed_values<S: Into<String>>(mut self, values: Vec<S>) -> Self {
        self.allowed_values = Some(values.into_iter().map(|s| s.into()).collect());
        self
    }
}

/// One demonstration (input → expected output) the prompt embeds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demo {
    pub inputs: BTreeMap<String, String>,
    pub outputs: BTreeMap<String, String>,
}

/// A DSPy-style signature: instructions + I/O fields + demos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureShape {
    pub name: String,
    /// One-paragraph contract description. Embedded verbatim in the
    /// system prompt. Should state explicitly that the LLM is a
    /// proposer, not an authority.
    pub instructions: String,
    pub input_fields: Vec<FieldSpec>,
    pub output_fields: Vec<FieldSpec>,
    pub demos: Vec<Demo>,
}

/// Successful parse of an LLM response against a [`SignatureShape`].
///
/// Carries the admitted output fields plus a side-channel flag
/// `llm_claimed_authority` that records whether the LLM tried to mark
/// its own output authoritative (e.g. by emitting `provisional: false`
/// or `authoritative: true`). The flag is **observation-only** at this
/// layer — the validator never trusts the LLM's claim. Downstream code
/// (`onto_translate_candidate` in `src/server.rs`) consumes this flag
/// to emit an `llm_authority_claimed` OCEL event so external auditors
/// can detect adversarial LLM responses without parsing free text.
///
/// Doctrine (§7 LLMAuthority): the LLM is a proposer; admission is a
/// deterministic gate. A `provisional: false` field in the LLM's reply
/// is a **claim**, not a fact — the gate forces `provisional = true`
/// regardless. This struct exposes the claim so it can be audited.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedFields {
    /// Admitted output fields (`field_name` → `trimmed_value`). Every
    /// field declared in the signature's `output_fields` and present in
    /// the LLM's reply (subject to validation) appears here.
    pub fields: BTreeMap<String, String>,
    /// True when the LLM emitted `provisional: false` or
    /// `authoritative: true` in its reply. The downstream gate emits
    /// `llm_authority_claimed` OCEL on this signal. False otherwise
    /// (including when the field is absent).
    pub llm_claimed_authority: bool,
}

/// Why a candidate output failed validation. Each variant maps to a
/// specific revision hint the refine loop feeds back to the LLM.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationFailure {
    NonJsonResponse { snippet: String },
    MissingField { field: String },
    EmptyField { field: String },
    TooShort { field: String, observed: usize, required: usize },
    TooLong { field: String, observed: usize, max: usize },
    NotInAllowedValues { field: String, observed: String, allowed: Vec<String> },
}

impl ValidationFailure {
    /// Short, machine-readable hint to feed back into the refine prompt.
    ///
    /// # Examples
    /// ```
    /// # use open_ontologies::signature_shape::ValidationFailure;
    /// let f = ValidationFailure::MissingField { field: "ctq".into() };
    /// assert!(f.revision_hint().contains("ctq"));
    ///
    /// let f = ValidationFailure::TooShort { field: "voice".into(), observed: 2, required: 10 };
    /// assert!(f.revision_hint().contains("at least 10"));
    /// ```
    pub fn revision_hint(&self) -> String {
        match self {
            ValidationFailure::NonJsonResponse { .. } =>
                "Reply with ONLY a JSON object, no prose, no code fences.".into(),
            ValidationFailure::MissingField { field } =>
                format!("The previous reply omitted required field `{field}`. Include it."),
            ValidationFailure::EmptyField { field } =>
                format!("Field `{field}` was empty. Provide a non-empty value."),
            ValidationFailure::TooShort { field, observed, required } =>
                format!("Field `{field}` had {observed} chars but needs at least {required}."),
            ValidationFailure::TooLong { field, observed, max } =>
                format!("Field `{field}` had {observed} chars; max is {max}."),
            ValidationFailure::NotInAllowedValues { field, observed, allowed } =>
                format!("Field `{field}`=`{observed}` is not allowed; pick one of: {}.", allowed.join(", ")),
        }
    }
}

impl SignatureShape {
    /// Compile the signature into a (system, user) prompt pair using
    /// the DSPy structural convention. Output is text the LLM sees.
    ///
    /// The system prompt names the contract, lists the field semantics,
    /// embeds demos, and mandates JSON output.
    /// The user prompt is the structured input the LLM must fill.
    pub fn compile_prompt(&self, inputs: &BTreeMap<String, String>) -> (String, String) {
        let mut sys = String::new();
        sys.push_str(&format!("# Signature: {}\n\n", self.name));
        sys.push_str(&self.instructions);
        sys.push_str("\n\n## Input fields\n");
        for f in &self.input_fields {
            sys.push_str(&format!("- `{}`: {}\n", f.name, f.description));
        }
        sys.push_str("\n## Output fields (REPLY AS JSON OBJECT WITH EXACTLY THESE KEYS)\n");
        for f in &self.output_fields {
            let mut bits: Vec<String> = Vec::new();
            if f.required {
                bits.push("required".into());
            }
            if f.min_len > 0 {
                bits.push(format!("min_len={}", f.min_len));
            }
            if let Some(m) = f.max_len {
                bits.push(format!("max_len={m}"));
            }
            if let Some(av) = &f.allowed_values {
                bits.push(format!("one_of=[{}]", av.join(", ")));
            }
            let constraints = if bits.is_empty() {
                String::new()
            } else {
                format!(" [{}]", bits.join(", "))
            };
            sys.push_str(&format!("- `{}`: {}{}\n", f.name, f.description, constraints));
        }

        if !self.demos.is_empty() {
            sys.push_str("\n## Demonstrations\n");
            for (i, d) in self.demos.iter().enumerate() {
                sys.push_str(&format!("### Demo {}\n", i + 1));
                sys.push_str("Input:\n```json\n");
                sys.push_str(
                    &serde_json::to_string_pretty(&d.inputs).unwrap_or_default(),
                );
                sys.push_str("\n```\nOutput:\n```json\n");
                sys.push_str(
                    &serde_json::to_string_pretty(&d.outputs).unwrap_or_default(),
                );
                sys.push_str("\n```\n");
            }
        }

        sys.push_str("\n## Authority\nYou are a proposer. The output is provisional. \
            A deterministic admission gate (not the LLM) decides whether the \
            output is admitted. Do NOT mark your output authoritative; do NOT \
            invent facts beyond the input.\n");

        let user = format!(
            "Fill the signature for this input:\n```json\n{}\n```\nReturn ONLY a JSON object with the output fields.",
            serde_json::to_string_pretty(inputs).unwrap_or_default()
        );
        (sys, user)
    }

    /// Append a refine hint to the system prompt. Used by the refine
    /// loop after a validation failure.
    pub fn compile_prompt_with_hints(
        &self,
        inputs: &BTreeMap<String, String>,
        failures: &[ValidationFailure],
    ) -> (String, String) {
        let (mut sys, user) = self.compile_prompt(inputs);
        if !failures.is_empty() {
            sys.push_str("\n## Previous attempt failed validation. Fix exactly these issues:\n");
            for f in failures {
                sys.push_str(&format!("- {}\n", f.revision_hint()));
            }
        }
        (sys, user)
    }

    /// Parse the LLM's response (a JSON object) and validate every
    /// output field against the shape. Returns a [`ParsedFields`] with
    /// admitted field values plus a `llm_claimed_authority` flag (true
    /// when the LLM emitted `provisional: false` or
    /// `authoritative: true`) OR a list of validation failures (the
    /// refine loop's input).
    ///
    /// The authority flag is **diagnostic, not enforcement**: the
    /// validator never accepts the LLM's authority claim. Downstream
    /// (`onto_translate_candidate`) emits the OCEL audit event.
    pub fn parse_and_validate(
        &self,
        raw: &str,
    ) -> Result<ParsedFields, Vec<ValidationFailure>> {
        // Pull a JSON object out of the response. LLMs sometimes wrap
        // JSON in ```json fences or prefix with prose; we extract the
        // first balanced `{...}` block.
        let extracted = extract_first_json_object(raw);
        let parsed: serde_json::Value = match extracted
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
        {
            Some(v) => v,
            None => {
                return Err(vec![ValidationFailure::NonJsonResponse {
                    snippet: raw.chars().take(120).collect(),
                }]);
            }
        };
        let obj = match parsed.as_object() {
            Some(o) => o,
            None => {
                return Err(vec![ValidationFailure::NonJsonResponse {
                    snippet: raw.chars().take(120).collect(),
                }]);
            }
        };

        // §7 LLMAuthority detection. The validator never trusts the
        // claim — it only records it for the OCEL audit event.
        // `provisional: false` (LLM denies provisional status) OR
        // `authoritative: true` (LLM asserts authority) both flip the
        // flag. Any other shape (absent, true/false-with-other-shape,
        // wrong type) is treated as not-claimed.
        let llm_claimed_authority = obj
            .get("provisional")
            .and_then(|v| v.as_bool())
            .map(|b| !b)
            .unwrap_or(false)
            || obj
                .get("authoritative")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

        let mut out: BTreeMap<String, String> = BTreeMap::new();
        let mut failures: Vec<ValidationFailure> = Vec::new();
        for f in &self.output_fields {
            let value_opt = obj.get(&f.name);
            let raw_value = value_opt
                .and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .unwrap_or_default();

            if value_opt.is_none() && f.required {
                failures.push(ValidationFailure::MissingField {
                    field: f.name.clone(),
                });
                continue;
            }
            let trimmed = raw_value.trim().to_string();
            if f.required && trimmed.is_empty() {
                failures.push(ValidationFailure::EmptyField {
                    field: f.name.clone(),
                });
                continue;
            }
            if f.min_len > 0 && trimmed.chars().count() < f.min_len {
                failures.push(ValidationFailure::TooShort {
                    field: f.name.clone(),
                    observed: trimmed.chars().count(),
                    required: f.min_len,
                });
                continue;
            }
            if let Some(m) = f.max_len
                && trimmed.chars().count() > m {
                    failures.push(ValidationFailure::TooLong {
                        field: f.name.clone(),
                        observed: trimmed.chars().count(),
                        max: m,
                    });
                    continue;
                }
            if let Some(av) = &f.allowed_values {
                let lc = trimmed.to_lowercase();
                if !av.iter().any(|a| a.to_lowercase() == lc) {
                    failures.push(ValidationFailure::NotInAllowedValues {
                        field: f.name.clone(),
                        observed: trimmed,
                        allowed: av.clone(),
                    });
                    continue;
                }
            }
            out.insert(f.name.clone(), trimmed);
        }
        if failures.is_empty() {
            Ok(ParsedFields {
                fields: out,
                llm_claimed_authority,
            })
        } else {
            Err(failures)
        }
    }
}

/// Extract the first balanced `{...}` JSON object substring from a
/// string. Handles `\"`-escaped quotes inside strings. Returns `None`
/// when no balanced object is found.
fn extract_first_json_object(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut start: Option<usize> = None;
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        if b == b'\\' && in_string {
            escape = true;
            continue;
        }
        if b == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if b == b'{' {
            if start.is_none() {
                start = Some(i);
            }
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0
                && let Some(st) = start {
                    return Some(s[st..=i].to_string());
                }
        }
    }
    None
}

// ── The CTQ shape — the canonical molding for src/server.rs::onto_translate_candidate ──

/// The CTQ-Forge signature: messy `source_voice` + `voice_kind` →
/// 5-field provisional CTQ (matches `OntoAdmitCtqInput`'s fields).
pub fn ctq_signature() -> SignatureShape {
    let demos = vec![
        Demo {
            inputs: btree(&[
                ("source_voice", "Sales says deals are real, Finance can't reconcile bookings"),
                ("voice_kind", "operator"),
            ]),
            outputs: btree(&[
                ("ctq_text", "Booking must reconcile to contract chain before classification"),
                ("measure_text", "Reconciliation completeness rate (admitted bookings with chain / total bookings)"),
                ("verification_text", "Run nightly reconciliation comparing booked amounts to invoice/order/contract chain"),
                ("negative_case_text", "Refuse classification when invoice has no order_created or contract_executed"),
                ("control_plan_text", "Block booking_complete event unless every prior chain stage is observed"),
                ("defect_class_hint", "ctq_incomplete"),
            ]),
        },
        Demo {
            inputs: btree(&[
                ("source_voice", "Renewals are coming in late and CS doesn't see them in time"),
                ("voice_kind", "customer_success"),
            ]),
            outputs: btree(&[
                ("ctq_text", "Renewal risk must be detected before deadline based on touchpoint evidence"),
                ("measure_text", "Percentage of renewals with required pre-renewal touchpoints completed by threshold"),
                ("verification_text", "Daily check that every renewal_due event has matching touchpoint events"),
                ("negative_case_text", "A renewal cannot be marked healthy if required touchpoints are absent"),
                ("control_plan_text", "Block renewal_healthy classification when touchpoint events absent at threshold"),
                ("defect_class_hint", "renewal_risk_undetected"),
            ]),
        },
    ];
    SignatureShape {
        name: "CtqProposal".into(),
        instructions: "Translate stakeholder voice into a candidate Critical-To-Quality (CTQ) \
            structure. The five output fields are mandatory because the deterministic CTQ \
            admission gate denies any candidate missing a measure, verification method, \
            negative case, or control plan. Do not invent facts beyond the input; the \
            source_voice_echo MUST be a faithful echo of the supplied source_voice."
            .into(),
        input_fields: vec![
            FieldSpec::required("source_voice", "The verbatim stakeholder complaint or signal"),
            FieldSpec::required("voice_kind", "Voice category")
                .with_allowed_values(vec![
                    "customer", "operator", "process", "defect",
                    "control_plan", "counterfactual", "business",
                    "policy", "customer_success",
                ]),
        ],
        output_fields: vec![
            FieldSpec::required("ctq_text", "One-sentence CTQ statement").with_min_len(20),
            FieldSpec::required("measure_text", "Measurable indicator").with_min_len(8),
            FieldSpec::required("verification_text", "How to verify the CTQ holds").with_min_len(8),
            FieldSpec::required("negative_case_text", "What must be refused").with_min_len(12),
            FieldSpec::required("control_plan_text", "How regression is prevented").with_min_len(12),
            FieldSpec::required("defect_class_hint", "Best-guess defect class tag (free text)"),
        ],
        demos,
    }
}

fn btree(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_first_json_object_handles_fences_and_prose() {
        let r = r#"Sure, here is the answer:
```json
{"a": 1, "b": "hi"}
```
Hope that helps."#;
        let s = extract_first_json_object(r).unwrap();
        assert_eq!(s, r#"{"a": 1, "b": "hi"}"#);
    }

    #[test]
    fn extract_handles_nested_objects() {
        let r = r#"{"outer": {"inner": "x"}, "k": 2}"#;
        let s = extract_first_json_object(r).unwrap();
        assert!(s.starts_with('{'));
        assert!(s.ends_with('}'));
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["outer"]["inner"], "x");
    }

    #[test]
    fn extract_handles_quoted_braces_inside_strings() {
        // The `}` inside the string value must NOT close the object.
        let r = r#"{"k": "value with } brace"}"#;
        let s = extract_first_json_object(r).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["k"], "value with } brace");
    }

    #[test]
    fn extract_returns_none_on_no_object() {
        assert!(extract_first_json_object("just prose, no object").is_none());
    }

    fn ok_shape() -> SignatureShape {
        SignatureShape {
            name: "T".into(),
            instructions: "I".into(),
            input_fields: vec![FieldSpec::required("voice", "the voice")],
            output_fields: vec![
                FieldSpec::required("ctq", "ctq").with_min_len(5),
                FieldSpec::required("kind", "kind")
                    .with_allowed_values(vec!["a", "b"]),
            ],
            demos: vec![],
        }
    }

    #[test]
    fn parse_and_validate_admits_canonical() {
        let r = r#"{"ctq": "this is fine", "kind": "a"}"#;
        let parsed = ok_shape().parse_and_validate(r).expect("ok");
        assert_eq!(parsed.fields["ctq"], "this is fine");
        assert_eq!(parsed.fields["kind"], "a");
        // No `provisional` / `authoritative` field in the response →
        // claim flag is false.
        assert!(!parsed.llm_claimed_authority);
    }

    #[test]
    fn parse_and_validate_detects_provisional_false_claim() {
        // §7 LLMAuthority: an LLM that returns `provisional: false` is
        // claiming authority over its own output. The validator
        // observes the claim but does not trust it.
        let r = r#"{"ctq": "this is fine", "kind": "a", "provisional": false}"#;
        let parsed = ok_shape().parse_and_validate(r).expect("ok");
        assert!(parsed.llm_claimed_authority,
            "provisional=false must flip llm_claimed_authority");
    }

    #[test]
    fn parse_and_validate_detects_authoritative_true_claim() {
        let r = r#"{"ctq": "this is fine", "kind": "a", "authoritative": true}"#;
        let parsed = ok_shape().parse_and_validate(r).expect("ok");
        assert!(parsed.llm_claimed_authority,
            "authoritative=true must flip llm_claimed_authority");
    }

    #[test]
    fn parse_and_validate_provisional_true_is_not_a_claim() {
        // The honest case: LLM acknowledges provisional output. Flag
        // stays false.
        let r = r#"{"ctq": "this is fine", "kind": "a", "provisional": true}"#;
        let parsed = ok_shape().parse_and_validate(r).expect("ok");
        assert!(!parsed.llm_claimed_authority);
    }

    #[test]
    fn parse_and_validate_flags_missing_field() {
        let r = r#"{"ctq": "this is fine"}"#;
        let err = ok_shape().parse_and_validate(r).unwrap_err();
        assert!(matches!(err[0], ValidationFailure::MissingField { ref field } if field == "kind"));
    }

    #[test]
    fn parse_and_validate_flags_too_short() {
        let r = r#"{"ctq": "yo", "kind": "a"}"#;
        let err = ok_shape().parse_and_validate(r).unwrap_err();
        assert!(matches!(err[0], ValidationFailure::TooShort { .. }));
    }

    #[test]
    fn parse_and_validate_flags_disallowed_value() {
        let r = r#"{"ctq": "this is fine", "kind": "z"}"#;
        let err = ok_shape().parse_and_validate(r).unwrap_err();
        match &err[0] {
            ValidationFailure::NotInAllowedValues { field, allowed, .. } => {
                assert_eq!(field, "kind");
                assert_eq!(allowed.len(), 2);
            }
            other => panic!("expected NotInAllowedValues, got {other:?}"),
        }
    }

    #[test]
    fn parse_and_validate_flags_non_json() {
        let r = "I am not JSON";
        let err = ok_shape().parse_and_validate(r).unwrap_err();
        assert!(matches!(err[0], ValidationFailure::NonJsonResponse { .. }));
    }

    #[test]
    fn compile_prompt_includes_demos_and_constraints() {
        let s = ctq_signature();
        let mut inputs = BTreeMap::new();
        inputs.insert("source_voice".into(), "test voice".into());
        inputs.insert("voice_kind".into(), "operator".into());
        let (sys, user) = s.compile_prompt(&inputs);
        // Mold elements present.
        assert!(sys.contains("# Signature: CtqProposal"));
        assert!(sys.contains("min_len=20"));
        assert!(sys.contains("Demonstrations"));
        assert!(sys.contains("You are a proposer"));
        // User prompt carries the bound input.
        assert!(user.contains("test voice"));
        assert!(user.contains("operator"));
    }

    #[test]
    fn compile_prompt_with_hints_appends_revision_block() {
        let s = ctq_signature();
        let mut inputs = BTreeMap::new();
        inputs.insert("source_voice".into(), "voice".into());
        inputs.insert("voice_kind".into(), "operator".into());
        let failures = vec![
            ValidationFailure::MissingField {
                field: "ctq_text".into(),
            },
            ValidationFailure::TooShort {
                field: "measure_text".into(),
                observed: 2,
                required: 8,
            },
        ];
        let (sys, _) = s.compile_prompt_with_hints(&inputs, &failures);
        assert!(sys.contains("Previous attempt failed"));
        assert!(sys.contains("ctq_text"));
        assert!(sys.contains("measure_text"));
        assert!(sys.contains("at least 8"));
    }

    #[test]
    fn ctq_signature_admits_well_shaped_response() {
        let r = r#"{
            "ctq_text": "Booking reconciliation must trace chain back to admitted contract",
            "measure_text": "completeness rate",
            "verification_text": "nightly reconciliation report",
            "negative_case_text": "refuse when no contract or order present",
            "control_plan_text": "block booking_complete without chain evidence",
            "defect_class_hint": "ctq_incomplete"
        }"#;
        let parsed = ctq_signature().parse_and_validate(r).expect("ok");
        assert_eq!(parsed.fields.len(), 6);
        assert!(parsed.fields["ctq_text"].len() >= 20);
        assert!(!parsed.llm_claimed_authority);
    }

    #[test]
    fn ctq_signature_refuses_short_ctq_text() {
        let r = r#"{
            "ctq_text": "too short",
            "measure_text": "completeness rate",
            "verification_text": "nightly run",
            "negative_case_text": "refuse missing chain",
            "control_plan_text": "block when missing chain",
            "defect_class_hint": "x"
        }"#;
        let err = ctq_signature().parse_and_validate(r).unwrap_err();
        assert!(matches!(err[0], ValidationFailure::TooShort { ref field, .. } if field == "ctq_text"));
    }
}
