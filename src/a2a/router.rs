//! T2-6 A2A HTTP router for agent-to-agent communication.

use super::agent_card::build_agent_info;
use crate::state::StateDb;
use axum::{response::IntoResponse, routing::get, routing::post, Json, Router};
use serde_json::json;
use std::sync::Arc;

/// Build the A2A router with POST / and GET /agent-card endpoints.
///
/// - `POST /` — handles A2A request messages
/// - `GET /agent-card` — returns agent discovery info
pub fn build_a2a_router(
    _db: Arc<StateDb>,
    _name: &str,
    agent_name: &str,
    agent_url: &str,
) -> Router {
    let agent_name_clone = agent_name.to_string();
    let agent_url_clone = agent_url.to_string();

    Router::new()
        .route("/", post(handle_a2a_request))
        .route(
            "/agent-card",
            get(move || {
                let name = agent_name_clone.clone();
                let url = agent_url_clone.clone();
                async move {
                    let info = build_agent_info(&name, &url);
                    Json(info).into_response()
                }
            }),
        )
}

/// Handle incoming A2A message request.
/// Accepts JSON message structure and returns task response.
async fn handle_a2a_request(
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    Json(json!({
        "task_id": "a2a-task-placeholder",
        "state": "pending",
        "result": {
            "status": "A2A router operational"
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn router_builds_without_panic() {
        let tmp = TempDir::new().expect("create temp dir");
        let db = Arc::new(StateDb::open(tmp.path().join("test.db").as_path()).expect("open StateDb"));
        let _router = build_a2a_router(db, "", "test-agent", "http://localhost:8080");
    }
}
