//! Phase 5 — DSPy-style shaped translator end-to-end.
//!
//! Drives `GroqTranslator::translate_with_signature` against a tokio
//! mock that:
//!   - Attempt 1: returns a malformed response missing required fields
//!   - Attempt 2: returns a well-shaped response
//!
//! Asserts the refine loop catches the validation failure on attempt
//! 1, embeds the typed revision hints into the system prompt for
//! attempt 2, and returns the admitted field map.
//!
//! This is the language-to-contract boundary closure materialized in
//! green test code: the LLM is *molded* before generation and *gauged*
//! after, with retry-on-failure driven by the typed
//! `ValidationFailure` taxonomy.

use open_ontologies::llm_translator::GroqTranslator;
use open_ontologies::signature_shape::{ctq_signature, FieldSpec, SignatureShape};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

const KEY: &str = "test-key-not-real-zk21";

/// Spawn a Groq mock that serves multiple responses in sequence.
/// `responses[i]` is returned for the i-th request; after the list is
/// exhausted the last response is repeated.
async fn spawn_seq_mock(responses: Vec<String>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured_systems = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured_clone = captured_systems.clone();
    let responses = Arc::new(responses);
    tokio::spawn(async move {
        let mut idx = 0usize;
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            let captured = captured_clone.clone();
            let responses = responses.clone();
            let i = idx;
            idx += 1;
            tokio::spawn(async move {
                let (rd, mut wr) = stream.into_split();
                let mut reader = BufReader::new(rd);
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                        break;
                    }
                    if line == "\r\n" || line == "\n" {
                        break;
                    }
                    if let Some(idx) = line.find(':') {
                        let (name, rest) = line.split_at(idx);
                        let value = rest.trim_start_matches(':').trim_start();
                        let value = value.trim_end_matches('\n').trim_end_matches('\r');
                        if name.eq_ignore_ascii_case("content-length") {
                            content_length = value.trim().parse().unwrap_or(0);
                        }
                    }
                }
                let mut req_body = vec![0u8; content_length];
                if content_length > 0 {
                    AsyncReadExt::read_exact(&mut reader, &mut req_body).await.ok();
                }
                let req_text = String::from_utf8_lossy(&req_body).to_string();
                // Capture the system prompt (for hint-leak assertions).
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&req_text) {
                    if let Some(arr) = parsed.get("messages").and_then(|m| m.as_array()) {
                        for m in arr {
                            if m.get("role").and_then(|v| v.as_str()) == Some("system") {
                                if let Some(c) = m.get("content").and_then(|v| v.as_str()) {
                                    captured.lock().await.push(c.to_string());
                                }
                            }
                        }
                    }
                }
                let pick = responses.get(i).or_else(|| responses.last()).cloned()
                    .unwrap_or_else(|| "{}".to_string());
                let resp_body = serde_json::json!({
                    "choices": [{"message": {"content": pick}}]
                })
                .to_string();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    resp_body.len(),
                    resp_body
                );
                wr.write_all(resp.as_bytes()).await.ok();
                wr.shutdown().await.ok();
            });
        }
    });
    (format!("http://{addr}"), captured_systems)
}

#[tokio::test]
async fn shaped_translation_admits_canonical_response_on_first_try() {
    // Mock returns a well-shaped CtqProposal immediately.
    let good = serde_json::json!({
        "ctq_text": "Booking reconciliation must trace through admitted contract chain",
        "measure_text": "completeness rate",
        "verification_text": "nightly reconciliation check",
        "negative_case_text": "refuse classification when no contract present",
        "control_plan_text": "block booking_complete event without chain evidence",
        "defect_class_hint": "ctq_incomplete"
    })
    .to_string();
    let (base, _captured) = spawn_seq_mock(vec![good]).await;
    let translator = GroqTranslator::new(
        &base,
        Some(KEY.to_string()),
        "llama-3.3-70b-versatile",
        Duration::from_secs(5),
    )
    .unwrap();
    let mut inputs = BTreeMap::new();
    inputs.insert("source_voice".into(), "Sales says committed; Finance can't reconcile".into());
    inputs.insert("voice_kind".into(), "operator".into());

    let parsed = translator
        .translate_with_signature(&ctq_signature(), &inputs, 2)
        .await
        .expect("admitted on first attempt");
    assert_eq!(parsed.fields.len(), 6);
    assert!(parsed.fields["ctq_text"].len() >= 20);
}

#[tokio::test]
async fn shaped_translation_refines_after_validation_failure() {
    // Attempt 1: missing measure_text + ctq_text too short.
    let bad = serde_json::json!({
        "ctq_text": "short",
        "verification_text": "nightly check",
        "negative_case_text": "refuse missing chain",
        "control_plan_text": "block partial chain",
        "defect_class_hint": "ctq_incomplete"
    })
    .to_string();
    // Attempt 2: corrected.
    let good = serde_json::json!({
        "ctq_text": "Booking must reconcile to admitted contract before classification",
        "measure_text": "completeness rate over admitted chains",
        "verification_text": "nightly reconciliation",
        "negative_case_text": "refuse when no contract",
        "control_plan_text": "block booking_complete missing chain",
        "defect_class_hint": "ctq_incomplete"
    })
    .to_string();
    let (base, captured) = spawn_seq_mock(vec![bad, good]).await;
    let translator = GroqTranslator::new(
        &base,
        Some(KEY.to_string()),
        "llama-3.3-70b-versatile",
        Duration::from_secs(5),
    )
    .unwrap();
    let mut inputs = BTreeMap::new();
    inputs.insert("source_voice".into(), "voice".into());
    inputs.insert("voice_kind".into(), "operator".into());

    let parsed = translator
        .translate_with_signature(&ctq_signature(), &inputs, 3)
        .await
        .expect("admitted on second attempt after refinement");
    assert_eq!(parsed.fields["ctq_text"].len() >= 20, true);

    // The captured system prompts: attempt 2's prompt MUST contain the
    // revision hints derived from attempt 1's failures.
    let systems = captured.lock().await.clone();
    assert_eq!(systems.len(), 2, "expected exactly 2 attempts");
    assert!(
        systems[1].contains("Previous attempt failed"),
        "refine attempt 2 must include the revision block"
    );
    assert!(
        systems[1].contains("measure_text"),
        "refine prompt must name the missing field"
    );
    assert!(
        systems[1].contains("ctq_text") && systems[1].contains("at least 20"),
        "refine prompt must surface the too-short hint with the required min_len"
    );
}

#[tokio::test]
async fn shaped_translation_exhausts_refinements_and_errors() {
    // Always returns a bad response — refinement loop must give up
    // and surface the final failure list.
    let bad = serde_json::json!({"oops": "no fields"}).to_string();
    let (base, _captured) =
        spawn_seq_mock(vec![bad.clone(), bad.clone(), bad.clone(), bad]).await;
    let translator = GroqTranslator::new(
        &base,
        Some(KEY.to_string()),
        "x",
        Duration::from_secs(5),
    )
    .unwrap();
    let mut inputs = BTreeMap::new();
    inputs.insert("source_voice".into(), "voice".into());
    inputs.insert("voice_kind".into(), "operator".into());

    let err = translator
        .translate_with_signature(&ctq_signature(), &inputs, 2)
        .await
        .unwrap_err();
    let s = format!("{err:?}");
    assert!(s.contains("shaped translation failed"));
    assert!(s.contains("required field"));
}

#[tokio::test]
async fn shaped_translation_handles_llm_with_code_fences_and_prose() {
    // Real LLMs sometimes wrap JSON in ```json fences with prose
    // before/after. The shape's parser must extract the first balanced
    // `{...}` block.
    let content = "Sure, here's the answer:\n```json\n{\
        \"ctq_text\": \"Booking reconciliation enforced via chain evidence\",\
        \"measure_text\": \"completeness rate\",\
        \"verification_text\": \"nightly check\",\
        \"negative_case_text\": \"refuse missing contract chain\",\
        \"control_plan_text\": \"block booking_complete missing chain\",\
        \"defect_class_hint\": \"ctq_incomplete\"\
        }\n```\nHope that helps!";
    let (base, _captured) = spawn_seq_mock(vec![content.to_string()]).await;
    let translator = GroqTranslator::new(
        &base,
        Some(KEY.to_string()),
        "x",
        Duration::from_secs(5),
    )
    .unwrap();
    let mut inputs = BTreeMap::new();
    inputs.insert("source_voice".into(), "voice".into());
    inputs.insert("voice_kind".into(), "operator".into());

    let parsed = translator
        .translate_with_signature(&ctq_signature(), &inputs, 1)
        .await
        .expect("fence-wrapped JSON must be extracted and admitted");
    assert!(parsed.fields["ctq_text"].len() >= 20);
}

#[tokio::test]
async fn shaped_translation_disallowed_value_triggers_refine() {
    // Custom shape with an `sh:in`-style allowed_values constraint.
    let shape = SignatureShape {
        name: "VoiceKindCheck".into(),
        instructions: "Pick a voice_kind".into(),
        input_fields: vec![FieldSpec::required("voice", "the voice")],
        output_fields: vec![FieldSpec::required("voice_kind", "the kind")
            .with_allowed_values(vec!["operator", "customer"])],
        demos: vec![],
    };
    let bad = serde_json::json!({"voice_kind": "executive"}).to_string();
    let good = serde_json::json!({"voice_kind": "operator"}).to_string();
    let (base, captured) = spawn_seq_mock(vec![bad, good]).await;
    let translator = GroqTranslator::new(
        &base,
        Some(KEY.to_string()),
        "x",
        Duration::from_secs(5),
    )
    .unwrap();
    let mut inputs = BTreeMap::new();
    inputs.insert("voice".into(), "x".into());
    let parsed = translator
        .translate_with_signature(&shape, &inputs, 2)
        .await
        .expect("admit after refining the disallowed value");
    assert_eq!(parsed.fields["voice_kind"], "operator");

    let systems = captured.lock().await.clone();
    assert!(systems[1].contains("not allowed"));
    assert!(systems[1].contains("operator") && systems[1].contains("customer"));
}
