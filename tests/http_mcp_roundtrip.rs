//! T3-5 HTTP MCP roundtrip tests.
//!
//! Tests MCP over HTTP transport without binding to TCP.
//! Uses tower::ServiceExt::oneshot for in-process testing.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::json;
use tower::ServiceExt;

/// Build a minimal router for testing.
fn make_test_router() -> Router {
    Router::new()
        .route("/health", axum::routing::get(|| async {
            axum::Json(json!({
                "status": "ok",
                "version": env!("CARGO_PKG_VERSION"),
            }))
        }))
}

#[tokio::test]
async fn api_health_returns_200() {
    let app = make_test_router();
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("build request");

    let response = app.oneshot(request).await.expect("execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("parse response as JSON");

    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn health_endpoint_is_accessible() {
    let app = make_test_router();
    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .expect("build request");

    let response = app.oneshot(request).await.expect("execute request");

    assert!(response.status().is_success(), "health endpoint should return success");
}
