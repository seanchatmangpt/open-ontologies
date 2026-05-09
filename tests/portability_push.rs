//! Sabotage tests for the Level-5 Portability gate's HTTP push path.
//!
//! These tests prove that `GraphStore::push_sparql_graph` actually transmits
//! the OntoStar receipt headers (`X-Ostar-Receipt-Hash`,
//! `X-Ostar-Production-Law`, `X-Ostar-Scope-Token`) to the SPARQL endpoint,
//! and that the request body honours the optional named-graph form.
//!
//! Strategy (R4 WA, §24): use the shared axum-based capture endpoint from
//! `tests/common/sparql_capture.rs`. Unlike the deleted Groq HTTP mock,
//! this capture endpoint does not simulate a third-party protocol — it
//! records the wire-format of OUR OWN client emitter, so the assertions
//! cross no external boundary and the test honestly verifies serialization
//! shape (which §24 sanctions).

mod common;

use common::sparql_capture::{await_captured, header_lookup, spawn_capture_endpoint};
use open_ontologies::graph::GraphStore;

#[tokio::test]
async fn push_sparql_graph_forwards_ostar_receipt_headers() {
    let (url, captured) = spawn_capture_endpoint().await;

    let content = r#"<http://example.org/a> <http://example.org/p> "v" ."#;
    let headers: &[(&str, &str)] = &[
        ("X-Ostar-Receipt-Hash", "deadbeefcafef00d"),
        ("X-Ostar-Production-Law", "ontostar-1.0.0"),
        ("X-Ostar-Scope-Token", "scope-test-1"),
    ];

    let result = GraphStore::push_sparql_graph(&url, content, None, headers).await;
    assert!(result.is_ok(), "push failed: {:?}", result.err());

    let req = await_captured(&captured)
        .await
        .expect("axum capture endpoint never observed a request");

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
    let (url, captured) = spawn_capture_endpoint().await;

    let content = r#"<http://example.org/a> <http://example.org/p> "v" ."#;
    let result = GraphStore::push_sparql_graph(&url, content, None, &[]).await;
    assert!(result.is_ok(), "push failed: {:?}", result.err());

    let req = await_captured(&captured)
        .await
        .expect("axum capture endpoint never observed a request");

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
    let (url, captured) = spawn_capture_endpoint().await;

    let content = r#"<http://example.org/a> <http://example.org/p> "v" ."#;
    let headers: &[(&str, &str)] = &[
        ("X-Ostar-Receipt-Hash", "11112222"),
        ("X-Ostar-Production-Law", "ontostar-1.0.0"),
        ("X-Ostar-Scope-Token", "scope-named-graph"),
    ];

    let result =
        GraphStore::push_sparql_graph(&url, content, Some("http://example.org/g1"), headers).await;
    assert!(result.is_ok(), "push failed: {:?}", result.err());

    let req = await_captured(&captured)
        .await
        .expect("axum capture endpoint never observed a request");

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
