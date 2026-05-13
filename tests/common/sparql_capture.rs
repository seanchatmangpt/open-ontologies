//! Shared axum-based SPARQL endpoint capture (R4 WA, §24 Chicago TDD).
//!
//! Replaces the per-test raw-TCP `spawn_mock_endpoint` previously inlined
//! in `tests/portability_push.rs`. Unlike the deleted Groq HTTP mock, this
//! capture endpoint is honest about what it is: a request-recorder, not a
//! protocol simulator. The asserts on the captured request live entirely
//! inside the test (they cross no service boundary), so this is testing
//! `GraphStore::push_sparql_graph`'s wire-format directly — exactly the
//! kind of testing §24 sanctions for serialization-shape verification.
//!
//! Behaviour: bind a random `127.0.0.1` port, accept exactly one request,
//! parse headers + body via axum, store both into a shared
//! `Arc<Mutex<Option<CapturedRequest>>>`, reply `200 OK`.
//!
//! This module is consumed by `tests/portability_push.rs` (and any future
//! consumer) via the `mod common; use common::sparql_capture::*;` pattern,
//! mirroring the `tests/cell_ready_fixtures/mod.rs` precedent established
//! by R4 WB. Sub-modules in `tests/` are per-binary, so each consumer
//! declares its own `mod common;` linkage.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::post;
use axum::Router;
use tokio::net::TcpListener;

#[derive(Default, Clone)]
pub struct CapturedRequest {
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub type Captured = Arc<Mutex<Option<CapturedRequest>>>;

/// Spawn a one-shot mock SPARQL endpoint backed by axum.
///
/// Returns `(url, captured)` where `url` is `http://127.0.0.1:<port>/sparql`
/// (a stable shape so callers may swap this in for the deleted raw-TCP
/// listener without touching call sites) and `captured` is a shared handle
/// into which the first observed request will be written. Subsequent
/// requests are still served `200 OK` but only the first is recorded.
pub async fn spawn_capture_endpoint() -> (String, Captured) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}/sparql");
    let captured: Captured = Arc::new(Mutex::new(None));
    let captured_for_handler = Arc::clone(&captured);

    let app = Router::new()
        .route("/sparql", post(handle_capture))
        .with_state(captured_for_handler);

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (url, captured)
}

async fn handle_capture(
    State(captured): State<Captured>,
    headers: HeaderMap,
    body: Bytes,
) -> &'static str {
    let mut h: HashMap<String, String> = HashMap::new();
    for (k, v) in headers.iter() {
        h.insert(k.as_str().to_string(), v.to_str().unwrap_or("").to_string());
    }
    let body_str = String::from_utf8_lossy(&body).into_owned();
    let mut slot = captured.lock().unwrap();
    if slot.is_none() {
        *slot = Some(CapturedRequest {
            headers: h,
            body: body_str,
        });
    }
    "OK"
}

/// Case-insensitive header lookup.
pub fn header_lookup<'a>(req: &'a CapturedRequest, name: &str) -> Option<&'a String> {
    req.headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v)
}

/// Poll the captured slot for up to ~200ms (20 × 10ms ticks) waiting for
/// the handler to record a request. Returns `Some(req)` once the slot is
/// populated, or `None` if the timeout expires.
pub async fn await_captured(captured: &Captured) -> Option<CapturedRequest> {
    for _ in 0..20 {
        if captured.lock().unwrap().is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    captured.lock().unwrap().clone()
}
