//! Shared in-process Groq mock — Phase 3.
//!
//! Listens on a random port, accepts one request per spawn, captures
//! the Authorization header (case-insensitive name match, original-case
//! value preserved), and returns a fixed CandidateCtq JSON. Pattern
//! lifted from tests/secret_hygiene.rs and tests/portability_push.rs.

#![allow(dead_code)]

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

/// Spawn a single-shot Groq mock that returns the supplied candidate
/// JSON. Returns `(base_url, captured_authorization_value)`. The base
/// URL has the shape `http://127.0.0.1:<port>` and is suitable as the
/// translator's `api_base`.
pub async fn spawn_with_response(candidate_json: String) -> (String, Arc<Mutex<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured_auth = Arc::new(Mutex::new(String::new()));
    let captured_auth_clone = captured_auth.clone();
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let (rd, mut wr) = stream.into_split();
            let mut reader = BufReader::new(rd);
            let mut content_length: usize = 0;
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
                    let lname = name.to_ascii_lowercase();
                    if lname == "authorization" {
                        *captured_auth_clone.lock().await = value.to_string();
                    } else if lname == "content-length" {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }
            }
            let mut body = vec![0u8; content_length];
            if content_length > 0 {
                AsyncReadExt::read_exact(&mut reader, &mut body).await.ok();
            }
            let resp_body = serde_json::json!({
                "choices": [{"message": {"content": candidate_json}}]
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

/// Default fixed `CandidateCtq` JSON for tests that don't care about
/// the specific content. Provisional=false in the mock; the translator
/// must override to true.
pub fn default_candidate_json() -> String {
    serde_json::json!({
        "source_voice_echo": "Sales committed; Finance not booked",
        "defect_class_hint": "ctq_incomplete",
        "ctq_text": "Booking must reconcile to contract chain",
        "measure_text": "Reconciliation completeness rate",
        "verification_text": "Run reconciliation report nightly",
        "negative_case_text": "Refuse classification when contract chain missing",
        "control_plan_text": "Block booking event without contract+order",
        "provisional": false,
    })
    .to_string()
}
