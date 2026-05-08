//! Level-5 secret-hygiene canary test (Phase 1 baseline).
//!
//! Sets a unique canary value as `GROQ_API_KEY`, drives the
//! [`GroqTranslator`] against an in-process tokio mock that returns a
//! fixed `CandidateCtq`, and asserts the canary appears in NONE of the
//! evidence surfaces the translator can write to:
//!
//!   - the `CandidateCtq` JSON (response surface)
//!   - the translator's `Debug` representation
//!   - any error string when the mock rejects the request
//!
//! This is the Phase-1 baseline — it covers the translator-only surface
//! that exists today. After Phase 1.5 lands the full MCP handler chain,
//! this test will be extended to also dump every OCEL row, every
//! receipt, and the executive projection text. The expansion is intended
//! to grow with the surface; the rule does not change.
//!
//! Negative-control: the canary string MUST appear in the in-memory
//! `GroqTranslator`'s configured key (proven via a sentinel Debug frame
//! check) — otherwise the test would pass even if the translator never
//! held the key, defeating the purpose.

use open_ontologies::llm_translator::{CandidateCtq, GroqTranslator};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

const CANARY: &str = "groq-canary-NEVERLEAKMEXYZ-7df3";

/// Mock Groq endpoint. Listens on a random port, accepts one HTTP/1.1
/// request, returns a fixed `CandidateCtq` JSON, and records the
/// captured Authorization header for the test to inspect.
async fn spawn_mock() -> (String, Arc<Mutex<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured_auth = Arc::new(Mutex::new(String::new()));
    let captured_auth_clone = captured_auth.clone();

    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let (rd, mut wr) = stream.into_split();
            let mut reader = BufReader::new(rd);
            let mut header_lines = Vec::new();
            let mut content_length: usize = 0;
            // Read headers
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                    break;
                }
                if line == "\r\n" || line == "\n" {
                    break;
                }
                // Match header *name* case-insensitively but preserve
                // the original-case value (the canary has uppercase
                // letters that would be lost by a blanket lowercase).
                if let Some(idx) = line.find(':') {
                    let (name, rest) = line.split_at(idx);
                    let value = rest.trim_start_matches(':').trim_start();
                    let value = value.trim_end_matches('\n').trim_end_matches('\r');
                    let lname = name.to_ascii_lowercase();
                    if lname == "authorization" {
                        *captured_auth_clone.lock().await = value.to_string();
                    } else if lname == "content-length" {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }
                header_lines.push(line);
            }
            // Drain the body
            let mut body = vec![0u8; content_length];
            if content_length > 0 {
                tokio::io::AsyncReadExt::read_exact(&mut reader, &mut body)
                    .await
                    .ok();
            }
            // Send a fixed JSON response that wraps a CandidateCtq.
            let candidate_json = serde_json::json!({
                "source_voice_echo": "Sales-Finance reconciliation gap",
                "defect_class_hint": "ctq_incomplete",
                "ctq_text": "Booking must reconcile to contract chain",
                "measure_text": "Reconciliation completeness rate",
                "verification_text": "Run reconciliation report nightly",
                "negative_case_text": "Refuse classification when contract chain missing",
                "control_plan_text": "Block booking event without contract+order",
                "provisional": false  // The translator must override this to true.
            })
            .to_string();
            let resp_body = serde_json::json!({
                "choices": [{
                    "message": {"content": candidate_json}
                }]
            })
            .to_string();
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.len(),
                resp_body
            );
            wr.write_all(resp.as_bytes()).await.ok();
            wr.shutdown().await.ok();
        }
    });

    (format!("http://{addr}"), captured_auth)
}

#[tokio::test]
async fn canary_key_never_appears_in_any_translator_evidence_surface() {
    // 1. Configure the translator with the canary key directly (no .env
    //    read — that's covered by config tests).
    let (base, captured_auth) = spawn_mock().await;
    let translator = GroqTranslator::new(
        &base,
        Some(CANARY.to_string()),
        "llama-3.3-70b-versatile",
        Duration::from_secs(5),
    )
    .unwrap();

    // 2. Drive a translation. The mock returns a fixed candidate.
    let candidate: CandidateCtq = translator
        .translate_candidate_ctq("Sales says committed; Finance says unbooked")
        .await
        .expect("translation should succeed against the mock");

    // 3. The translator MUST force provisional=true regardless of mock.
    assert!(candidate.provisional, "translator must mark output provisional");

    // 4. Scan every translator-side evidence surface for the canary.
    let surfaces: Vec<(&str, String)> = vec![
        ("CandidateCtq.source_voice_echo", candidate.source_voice_echo.clone()),
        ("CandidateCtq.defect_class_hint", candidate.defect_class_hint.clone()),
        ("CandidateCtq.ctq_text", candidate.ctq_text.clone()),
        ("CandidateCtq.measure_text", candidate.measure_text.clone()),
        ("CandidateCtq.verification_text", candidate.verification_text.clone()),
        ("CandidateCtq.negative_case_text", candidate.negative_case_text.clone()),
        ("CandidateCtq.control_plan_text", candidate.control_plan_text.clone()),
        (
            "serde_json::to_string(&candidate)",
            serde_json::to_string(&candidate).unwrap(),
        ),
        ("Debug(GroqTranslator)", format!("{translator:?}")),
        ("translator.endpoint()", translator.endpoint().to_string()),
        ("translator.model()", translator.model().to_string()),
    ];

    for (name, content) in &surfaces {
        assert!(
            !content.contains(CANARY),
            "secret-hygiene FAILED — canary key leaked into surface `{name}`:\n{content}"
        );
    }

    // 5. Authorization header MUST have been received by the mock with
    //    the canary as the bearer token (proves the test would catch a
    //    leak — without this, the test could pass even if the key was
    //    never sent at all).
    let auth_seen = captured_auth.lock().await.clone();
    assert!(
        auth_seen.to_ascii_lowercase().contains("bearer "),
        "mock never received an Authorization: Bearer header (got `{auth_seen}`)"
    );
    assert!(
        auth_seen.contains(CANARY),
        "mock did not receive the canary as the bearer token (got `{auth_seen}`) — \
         either the translator is not sending bearer auth, or the test harness is broken"
    );
}

#[tokio::test]
async fn translator_debug_frame_redacts_canary() {
    // Negative control: even if the test above passes, we also assert
    // directly that Debug never echoes the key. This catches a future
    // refactor that accidentally derives Debug instead of the manual
    // redacting impl.
    let translator = GroqTranslator::new(
        "https://api.groq.com/openai/v1",
        Some(CANARY.to_string()),
        "llama-3.3-70b-versatile",
        Duration::from_secs(1),
    )
    .unwrap();
    let dbg = format!("{translator:?}");
    assert!(!dbg.contains(CANARY), "Debug leaked canary: {dbg}");
    assert!(dbg.contains("<redacted>"), "Debug should mark redacted: {dbg}");
}

#[tokio::test]
async fn error_path_does_not_echo_bearer_token() {
    // Spin up a mock that returns 401 with a body that *includes* a
    // Bearer-shaped echo, simulating a misbehaving gateway. The
    // translator's redact_bearer_patterns must strip the token.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let canary = "leaky-token-DO-NOT-LEAK-9zk3";

    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let (rd, mut wr) = stream.into_split();
            let mut reader = BufReader::new(rd);
            // Drain headers + (best-effort) body; we don't need them.
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                    break;
                }
                if line == "\r\n" || line == "\n" {
                    break;
                }
            }
            let body = format!(
                "{{\"error\":\"unauthorized — the Bearer {canary} you sent is invalid\"}}"
            );
            let resp = format!(
                "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            wr.write_all(resp.as_bytes()).await.ok();
            wr.shutdown().await.ok();
        }
    });

    let translator = GroqTranslator::new(
        &format!("http://{addr}"),
        Some(canary.to_string()),
        "llama-3.3-70b-versatile",
        Duration::from_secs(5),
    )
    .unwrap();
    let err = translator
        .translate_candidate_ctq("voice")
        .await
        .unwrap_err();
    let err_str = format!("{err:?}");
    assert!(
        !err_str.contains(canary),
        "error path leaked the bearer token: {err_str}"
    );
    assert!(
        err_str.contains("Bearer <redacted>") || err_str.contains("<redacted>"),
        "error string should show redaction marker: {err_str}"
    );
}
