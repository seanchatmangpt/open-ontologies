//! T2-5 A2A message handler for receipt-preserving onto_* tool dispatch.

use super::task_store::TaskState;
use crate::inputs::{OntoLoadInput, OntoQueryInput, OntoValidateInput};
use crate::server::OpenOntologiesServer;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

const REFUSAL_MISSING_TOOL: &str = "A2A_MISSING_TOOL";
const REFUSAL_UNKNOWN_TOOL: &str = "A2A_UNKNOWN_TOOL";
const REFUSAL_INVALID_PARAMS: &str = "A2A_INVALID_PARAMS";

/// A2A message from another agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub tool: Option<String>,
    pub params: Option<Value>,
}

/// A2A message handler trait.
#[async_trait]
pub trait AsyncMessageHandler: Send + Sync {
    async fn handle_message(&self, message: Message) -> Result<(TaskState, Value), String>;
}

/// A2A message handler that delegates to the same raw wrappers used by the
/// MCP server. It does not reimplement query, validation, or loading logic.
pub struct OntologiesMessageHandler {
    server: Arc<OpenOntologiesServer>,
}

impl OntologiesMessageHandler {
    pub fn new(server: Arc<OpenOntologiesServer>) -> Self {
        Self { server }
    }
}

fn deserialize_params<T: DeserializeOwned>(message: &Message, tool: &str) -> Result<T, String> {
    let params = message.params.clone().unwrap_or(Value::Null);
    serde_json::from_value(params)
        .map_err(|error| format!("{REFUSAL_INVALID_PARAMS}:{tool}:{error}"))
}

fn decode_tool_output(tool: &str, field: &str, raw: String) -> (TaskState, Value) {
    let output = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| Value::String(raw));
    let failed = output
        .get("ok")
        .and_then(Value::as_bool)
        .is_some_and(|ok| !ok)
        || output.get("error").is_some();
    let state = if failed {
        TaskState::Failed
    } else {
        TaskState::Completed
    };
    (state, json!({ "tool": tool, field: output }))
}

#[async_trait]
impl AsyncMessageHandler for OntologiesMessageHandler {
    async fn handle_message(&self, message: Message) -> Result<(TaskState, Value), String> {
        let tool = message
            .tool
            .as_deref()
            .filter(|tool| !tool.trim().is_empty())
            .ok_or_else(|| REFUSAL_MISSING_TOOL.to_string())?;

        let outcome = match tool {
            "onto_status" => decode_tool_output(tool, "status", self.server.onto_status_raw()),
            "onto_query" => {
                let input: OntoQueryInput = deserialize_params(&message, tool)?;
                decode_tool_output(tool, "result", self.server.onto_query_raw(input).await)
            }
            "onto_validate" => {
                let input: OntoValidateInput = deserialize_params(&message, tool)?;
                decode_tool_output(tool, "result", self.server.onto_validate_raw(input).await)
            }
            "onto_load" => {
                let input: OntoLoadInput = deserialize_params(&message, tool)?;
                decode_tool_output(tool, "result", self.server.onto_load_raw(input).await)
            }
            "onto_stats" => decode_tool_output(tool, "stats", self.server.onto_stats_raw()),
            _ => return Err(format!("{REFUSAL_UNKNOWN_TOOL}:{tool}")),
        };

        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::StateDb;
    use tempfile::TempDir;

    fn handler() -> (TempDir, OntologiesMessageHandler) {
        let temp = TempDir::new().expect("create temp directory");
        let db = StateDb::open(&temp.path().join("a2a.db")).expect("open state database");
        let server = Arc::new(OpenOntologiesServer::new(db));
        (temp, OntologiesMessageHandler::new(server))
    }

    #[tokio::test]
    async fn delegates_load_query_and_validate_to_real_server_boundaries() {
        let (_temp, handler) = handler();
        let turtle = "@prefix ex: <https://example.test/> . ex:subject ex:predicate ex:object .";

        let (load_state, load) = handler
            .handle_message(Message {
                tool: Some("onto_load".to_string()),
                params: Some(json!({
                    "path": null,
                    "turtle": turtle,
                    "name": "a2a-witness",
                    "auto_refresh": false,
                    "force_recompile": true
                })),
            })
            .await
            .expect("dispatch load");
        assert_eq!(load_state, TaskState::Completed, "load response: {load}");

        let (query_state, query) = handler
            .handle_message(Message {
                tool: Some("onto_query".to_string()),
                params: Some(json!({
                    "query": "SELECT ?s ?p ?o WHERE { ?s ?p ?o }"
                })),
            })
            .await
            .expect("dispatch query");
        assert_eq!(query_state, TaskState::Completed, "query response: {query}");
        assert!(query.to_string().contains("example.test"));

        let (validate_state, validate) = handler
            .handle_message(Message {
                tool: Some("onto_validate".to_string()),
                params: Some(json!({ "input": turtle, "inline": true })),
            })
            .await
            .expect("dispatch validation");
        assert_eq!(
            validate_state,
            TaskState::Completed,
            "validation response: {validate}"
        );
    }

    #[tokio::test]
    async fn malformed_params_and_unknown_tools_are_typed_refusals() {
        let (_temp, handler) = handler();

        let malformed = handler
            .handle_message(Message {
                tool: Some("onto_query".to_string()),
                params: Some(json!({ "wrong": "field" })),
            })
            .await
            .expect_err("malformed query parameters must refuse");
        assert!(malformed.starts_with("A2A_INVALID_PARAMS:onto_query:"));

        let unknown = handler
            .handle_message(Message {
                tool: Some("onto_delete_everything".to_string()),
                params: None,
            })
            .await
            .expect_err("unknown tool must refuse");
        assert_eq!(unknown, "A2A_UNKNOWN_TOOL:onto_delete_everything");

        let missing = handler
            .handle_message(Message {
                tool: None,
                params: None,
            })
            .await
            .expect_err("missing tool must refuse");
        assert_eq!(missing, "A2A_MISSING_TOOL");
    }

    #[test]
    fn tool_error_payloads_produce_failed_task_state() {
        let (state, envelope) = decode_tool_output(
            "onto_validate",
            "result",
            r#"{"ok":false,"error":"invalid RDF"}"#.to_string(),
        );
        assert_eq!(state, TaskState::Failed);
        assert_eq!(envelope["tool"], "onto_validate");
        assert_eq!(envelope["result"]["error"], "invalid RDF");
    }
}
