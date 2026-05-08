//! Sabotage tests for the Level-5 Portability gate's HTTP push path.
//!
//! These tests prove that `GraphStore::push_sparql_graph` actually transmits
//! the OntoStar receipt headers (`X-Ostar-Receipt-Hash`,
//! `X-Ostar-Production-Law`, `X-Ostar-Scope-Token`) to the SPARQL endpoint,
//! and that the request body honours the optional named-graph form.
//!
//! Strategy: stand up a tiny in-process HTTP/1.1 server on a random port using
//! tokio TCP primitives only. The handler parses the request, records the
//! headers and body into a shared `Mutex`, and replies `200 OK`. This avoids
//! adding any new dev-dependencies (no httpmock, wiremock, or mockito) and
//! keeps the test deterministic and offline.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use open_ontologies::graph::GraphStore;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Default, Clone)]
struct CapturedRequest {
    headers: HashMap<String, String>,
    body: String,
}

/// Spawn a one-shot mock SPARQL endpoint. Returns the bound URL and a handle
/// from which the captured request can be retrieved after the call completes.
async fn spawn_mock_endpoint() -> (String, Arc<Mutex<Option<CapturedRequest>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}/sparql", addr);
    let captured: Arc<Mutex<Option<CapturedRequest>>> = Arc::new(Mutex::new(None));
    let captured_for_task = Arc::clone(&captured);

    tokio::spawn(async move {
        // Accept exactly one connection.
        let (mut socket, _) = match listener.accept().await {
            Ok(v) => v,
            Err(_) => return,
        };

        // Read until we have headers + full body. We rely on Content-Length.
        let mut buf = Vec::with_capacity(4096);
        let mut tmp = [0u8; 2048];
        let mut header_end: Option<usize> = None;
        let mut content_length: usize = 0;

        loop {
            let n = match socket.read(&mut tmp).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => return,
            };
            buf.extend_from_slice(&tmp[..n]);

            if header_end.is_none() {
                if let Some(idx) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    header_end = Some(idx + 4);
                    let header_text = std::str::from_utf8(&buf[..idx]).unwrap_or("");
                    for line in header_text.split("\r\n").skip(1) {
                        if let Some((k, v)) = line.split_once(':') {
                            if k.eq_ignore_ascii_case("content-length") {
                                content_length = v.trim().parse().unwrap_or(0);
                            }
                        }
                    }
                }
            }

            if let Some(he) = header_end {
                if buf.len() >= he + content_length {
                    break;
                }
            }
        }

        let he = header_end.unwrap_or(buf.len());
        let header_text = std::str::from_utf8(&buf[..he.saturating_sub(4)]).unwrap_or("");
        let mut headers: HashMap<String, String> = HashMap::new();
        for line in header_text.split("\r\n").skip(1) {
            if let Some((k, v)) = line.split_once(':') {
                headers.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        let body = std::str::from_utf8(&buf[he..he + content_length])
            .unwrap_or("")
            .to_string();

        *captured_for_task.lock().unwrap() = Some(CapturedRequest { headers, body });

        let _ = socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK")
            .await;
        let _ = socket.shutdown().await;
    });

    (url, captured)
}

fn header_lookup<'a>(req: &'a CapturedRequest, name: &str) -> Option<&'a String> {
    req.headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v)
}

#[tokio::test]
async fn push_sparql_graph_forwards_ostar_receipt_headers() {
    let (url, captured) = spawn_mock_endpoint().await;

    let content = r#"<http://example.org/a> <http://example.org/p> "v" ."#;
    let headers: &[(&str, &str)] = &[
        ("X-Ostar-Receipt-Hash", "deadbeefcafef00d"),
        ("X-Ostar-Production-Law", "ontostar-1.0.0"),
        ("X-Ostar-Scope-Token", "scope-test-1"),
    ];

    let result = GraphStore::push_sparql_graph(&url, content, None, headers).await;
    assert!(result.is_ok(), "push failed: {:?}", result.err());

    // Give the mock task a tick to record state.
    for _ in 0..20 {
        if captured.lock().unwrap().is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let req = captured
        .lock()
        .unwrap()
        .clone()
        .expect("mock endpoint never observed a request");

    assert_eq!(
        header_lookup(&req, "X-Ostar-Receipt-Hash").map(String::as_str),
        Some("deadbeefcafef00d"),
        "receipt hash header missing or wrong: {:?}",
        req.headers
    );
    assert_eq!(
        header_lookup(&req, "X-Ostar-Production-Law").map(String::as_str),
        Some("ontostar-1.0.0")
    );
    assert_eq!(
        header_lookup(&req, "X-Ostar-Scope-Token").map(String::as_str),
        Some("scope-test-1")
    );
    assert_eq!(
        header_lookup(&req, "Content-Type").map(String::as_str),
        Some("application/sparql-update")
    );
    // Default-graph form: no GRAPH <iri> wrapper.
    assert!(
        req.body.starts_with("INSERT DATA {"),
        "unexpected body: {}",
        req.body
    );
    assert!(!req.body.contains("GRAPH <"), "unexpected named graph in body: {}", req.body);
}

#[tokio::test]
async fn push_sparql_graph_omits_ostar_headers_when_none_provided() {
    let (url, captured) = spawn_mock_endpoint().await;

    let content = r#"<http://example.org/a> <http://example.org/p> "v" ."#;
    let result = GraphStore::push_sparql_graph(&url, content, None, &[]).await;
    assert!(result.is_ok(), "push failed: {:?}", result.err());

    for _ in 0..20 {
        if captured.lock().unwrap().is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let req = captured
        .lock()
        .unwrap()
        .clone()
        .expect("mock endpoint never observed a request");

    let ostar_headers: Vec<_> = req
        .headers
        .keys()
        .filter(|k| k.to_ascii_lowercase().starts_with("x-ostar-"))
        .collect();
    assert!(
        ostar_headers.is_empty(),
        "no X-Ostar-* headers should be sent, found: {:?}",
        ostar_headers
    );
}

#[tokio::test]
async fn push_sparql_graph_named_graph_includes_graph_clause_and_headers() {
    let (url, captured) = spawn_mock_endpoint().await;

    let content = r#"<http://example.org/a> <http://example.org/p> "v" ."#;
    let headers: &[(&str, &str)] = &[
        ("X-Ostar-Receipt-Hash", "11112222"),
        ("X-Ostar-Production-Law", "ontostar-1.0.0"),
        ("X-Ostar-Scope-Token", "scope-named-graph"),
    ];

    let result =
        GraphStore::push_sparql_graph(&url, content, Some("http://example.org/g1"), headers).await;
    assert!(result.is_ok(), "push failed: {:?}", result.err());

    for _ in 0..20 {
        if captured.lock().unwrap().is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let req = captured
        .lock()
        .unwrap()
        .clone()
        .expect("mock endpoint never observed a request");

    assert!(
        req.body
            .starts_with("INSERT DATA { GRAPH <http://example.org/g1>"),
        "named-graph body shape wrong: {}",
        req.body
    );
    assert_eq!(
        header_lookup(&req, "X-Ostar-Receipt-Hash").map(String::as_str),
        Some("11112222")
    );
    assert_eq!(
        header_lookup(&req, "X-Ostar-Scope-Token").map(String::as_str),
        Some("scope-named-graph")
    );
}
