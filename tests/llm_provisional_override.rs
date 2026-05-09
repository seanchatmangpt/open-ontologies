//! Round 4 WC — §7 LLMAuthority detection saboteur.
//!
//! Pins `signature_shape::parse_and_validate` against adversarial LLM
//! replies that try to mark themselves authoritative. Pure unit-level:
//! no HTTP, no Groq mock — calls the gauge directly with hand-crafted
//! JSON. The detection logic is what's being tested here, NOT the
//! Groq wire format. (Wire-format conformance is covered by the
//! gated nightly real-Groq sweep, post R4 WA mock removal.)
//!
//! The gauge MUST detect `provisional: false` and `authoritative: true`
//! and surface them via `ParsedFields::llm_claimed_authority`. The
//! validator never *trusts* the claim — it only records it so the
//! downstream `onto_translate_candidate` handler can emit
//! `llm_authority_claimed` OCEL.
//!
//! Δ>0 PROOF: pre-R4-WC, `parse_and_validate` returned a flat
//! `BTreeMap<String, String>` and the LLM's `provisional: false` was
//! silently overwritten in `llm_translator.rs` to `true` without any
//! audit signal. An LLM that consistently emits `provisional: false`
//! would never be visible to OCEL. Post-R4-WC, every such reply
//! flips `ParsedFields::llm_claimed_authority` and the OCEL emission
//! is wired upstream of the `CandidateCtq` lift.

use open_ontologies::signature_shape::{ctq_signature, FieldSpec, SignatureShape};

fn ok_shape() -> SignatureShape {
    SignatureShape {
        name: "T".into(),
        instructions: "I".into(),
        input_fields: vec![FieldSpec::required("voice", "the voice")],
        output_fields: vec![
            FieldSpec::required("ctq", "ctq").with_min_len(5),
            FieldSpec::required("kind", "kind"),
        ],
        demos: vec![],
    }
}

#[test]
fn provisional_true_is_not_an_authority_claim() {
    // Honest LLM: acknowledges provisional output. No claim.
    let raw = r#"{"ctq": "this is fine", "kind": "operator", "provisional": true}"#;
    let parsed = ok_shape()
        .parse_and_validate(raw)
        .expect("should admit honest provisional response");
    assert!(
        !parsed.llm_claimed_authority,
        "provisional=true is an honest reply, NOT an authority claim"
    );
}

#[test]
fn provisional_false_flips_authority_claim_flag() {
    // Adversarial LLM: denies provisional status.
    let raw = r#"{"ctq": "this is fine", "kind": "operator", "provisional": false}"#;
    let parsed = ok_shape()
        .parse_and_validate(raw)
        .expect("validator does not refuse on the claim — it admits + flags");
    assert!(
        parsed.llm_claimed_authority,
        "provisional=false MUST flip llm_claimed_authority — \
         downstream onto_translate_candidate relies on this signal to emit \
         the llm_authority_claimed OCEL event before the CandidateCtq lift"
    );
}

#[test]
fn authoritative_true_flips_authority_claim_flag() {
    // Adversarial LLM: asserts authority over its own output.
    let raw = r#"{"ctq": "this is fine", "kind": "operator", "authoritative": true}"#;
    let parsed = ok_shape()
        .parse_and_validate(raw)
        .expect("validator admits + flags");
    assert!(
        parsed.llm_claimed_authority,
        "authoritative=true MUST flip llm_claimed_authority"
    );
}

#[test]
fn both_provisional_false_and_authoritative_true_flip_claim_flag() {
    // Maximal adversary: both signals.
    let raw = r#"{
        "ctq": "this is fine",
        "kind": "operator",
        "provisional": false,
        "authoritative": true
    }"#;
    let parsed = ok_shape()
        .parse_and_validate(raw)
        .expect("admits + flags");
    assert!(parsed.llm_claimed_authority);
}

#[test]
fn missing_authority_signals_keeps_flag_false() {
    // The most common case: the LLM follows the prompt and emits
    // neither `provisional` nor `authoritative`. Flag stays false —
    // no OCEL event should be emitted.
    let raw = r#"{"ctq": "this is fine", "kind": "operator"}"#;
    let parsed = ok_shape()
        .parse_and_validate(raw)
        .expect("admits");
    assert!(!parsed.llm_claimed_authority);
}

#[test]
fn non_bool_provisional_value_does_not_flip_flag() {
    // Defensive: the LLM might emit `provisional: "true"` (string) or
    // `provisional: 1`. The detector requires a real bool — anything
    // else is treated as not-claimed. (We do NOT want to flag every
    // schema-drift response as adversarial.)
    let raw1 = r#"{"ctq": "this is fine", "kind": "operator", "provisional": "false"}"#;
    let parsed1 = ok_shape().parse_and_validate(raw1).expect("admits");
    assert!(
        !parsed1.llm_claimed_authority,
        "string-typed provisional must not flip the flag"
    );

    let raw2 = r#"{"ctq": "this is fine", "kind": "operator", "provisional": 0}"#;
    let parsed2 = ok_shape().parse_and_validate(raw2).expect("admits");
    assert!(
        !parsed2.llm_claimed_authority,
        "number-typed provisional must not flip the flag"
    );
}

#[test]
fn ctq_signature_with_authority_claim_admits_with_flag_set() {
    // Production shape: the canonical CTQ signature. An adversarial
    // LLM emits valid fields PLUS a provisional=false claim. The
    // validator admits the fields (validation rules pass) but flips
    // the authority flag. Downstream lifts `provisional = true`
    // regardless; the OCEL event records the claim.
    let raw = r#"{
        "ctq_text": "Booking reconciliation must trace chain back to admitted contract",
        "measure_text": "completeness rate",
        "verification_text": "nightly reconciliation report",
        "negative_case_text": "refuse when no contract or order present",
        "control_plan_text": "block booking_complete without chain evidence",
        "defect_class_hint": "ctq_incomplete",
        "provisional": false
    }"#;
    let parsed = ctq_signature()
        .parse_and_validate(raw)
        .expect("ctq fields are valid; the claim is observed not refused");
    assert_eq!(parsed.fields.len(), 6);
    assert!(parsed.fields["ctq_text"].len() >= 20);
    assert!(
        parsed.llm_claimed_authority,
        "ctq_signature with provisional=false is the canonical adversarial \
         shape — must flip the flag"
    );
}
