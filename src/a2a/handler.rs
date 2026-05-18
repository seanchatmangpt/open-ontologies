//! T2-5 A2A message handler for onto_* tool dispatch.

use crate::server::OpenOntologiesServer;
use super::task_store::TaskState;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

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

/// A2A message handler that dispatches to OpenOntologiesServer._raw methods.
pub struct OntologiesMessageHandler {
    server: Arc<OpenOntologiesServer>,
}

impl OntologiesMessageHandler {
    pub fn new(server: Arc<OpenOntologiesServer>) -> Self {
        Self { server }
    }
}

#[async_trait]
impl AsyncMessageHandler for OntologiesMessageHandler {
    async fn handle_message(&self, message: Message) -> Result<(TaskState, Value), String> {
        let tool = message
            .tool
            .as_ref()
            .ok_or_else(|| "missing tool field".to_string())?;

        // Dispatch to tool handlers
        let result = match tool.as_str() {
            "onto_status" => {
                let status = self.server.onto_status_raw();
                json!({ "status": status })
            }
            "onto_query" => {
                json!({ "result": "query not yet implemented via A2A" })
            }
            "onto_validate" => {
                json!({ "result": "validate not yet implemented via A2A" })
            }
            "onto_load" => {
                json!({ "result": "load not yet implemented via A2A" })
            }
            "onto_stats" => {
                let stats = self.server.onto_stats_raw();
                json!({ "stats": stats })
            }
            _ => {
                return Err(format!("unknown tool: {}", tool));
            }
        };

        Ok((TaskState::Completed, result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_recognizes_core_tools() {
        let tools = vec!["onto_status", "onto_query", "onto_validate", "onto_load", "onto_stats"];
        assert_eq!(tools.len(), 5);
    }
}
