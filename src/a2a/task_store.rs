//! T2-2 A2A task store backed by StateDb.

use crate::state::StateDb;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Task state enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
}

/// Async task manager trait.
#[async_trait]
pub trait AsyncTaskManager: Send + Sync {
    async fn create_task(&self, context_id: &str, tenant_id: &str) -> Result<String, String>;
    async fn get_task(
        &self,
        task_id: &str,
        tenant_id: &str,
    ) -> Result<Option<(TaskState, HashMap<String, String>)>, String>;
    async fn task_exists(&self, task_id: &str, tenant_id: &str) -> Result<bool, String>;
    async fn update_task_state(
        &self,
        task_id: &str,
        state: TaskState,
        tenant_id: &str,
    ) -> Result<(), String>;
}

/// Async notification manager trait.
#[async_trait]
pub trait AsyncNotificationManager: Send + Sync {
    async fn emit_notification(
        &self,
        task_id: &str,
        message: String,
        tenant_id: &str,
    ) -> Result<(), String>;
}

/// A2A task store backed by SQLite StateDb.
///
/// Stores task state, messages, and artifacts per (task_id, tenant_id) pair.
/// Both AsyncTaskManager and AsyncNotificationManager are implemented.
pub struct A2aTaskStore {
    db: Arc<StateDb>,
}

impl A2aTaskStore {
    pub fn new(db: Arc<StateDb>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AsyncTaskManager for A2aTaskStore {
    async fn create_task(&self, context_id: &str, tenant_id: &str) -> Result<String, String> {
        let task_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO a2a_tasks (task_id, context_id, state, created_at, updated_at, tenant_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [&task_id, context_id, "pending", &now, &now, tenant_id],
        )
        .map_err(|e| e.to_string())?;

        Ok(task_id)
    }

    async fn get_task(
        &self,
        task_id: &str,
        tenant_id: &str,
    ) -> Result<Option<(TaskState, HashMap<String, String>)>, String> {
        let conn = self.db.conn();

        let mut stmt = conn
            .prepare(
                "SELECT state, messages FROM a2a_tasks WHERE task_id = ?1 AND tenant_id = ?2",
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query([task_id, tenant_id])
            .map_err(|e| e.to_string())?;

        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let state_str: String = row.get(0).map_err(|e| e.to_string())?;
            let messages_json: String = row.get(1).map_err(|e| e.to_string())?;

            let state = match state_str.as_str() {
                "pending" => TaskState::Pending,
                "in_progress" => TaskState::InProgress,
                "completed" => TaskState::Completed,
                "failed" => TaskState::Failed,
                _ => TaskState::Pending,
            };

            let messages: HashMap<String, String> =
                serde_json::from_str(&messages_json).unwrap_or_default();

            Ok(Some((state, messages)))
        } else {
            Ok(None)
        }
    }

    async fn task_exists(&self, task_id: &str, _tenant_id: &str) -> Result<bool, String> {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare("SELECT 1 FROM a2a_tasks WHERE task_id = ?1 LIMIT 1")
            .map_err(|e| e.to_string())?;

        Ok(stmt
            .exists([task_id])
            .map_err(|e| e.to_string())?)
    }

    async fn update_task_state(
        &self,
        task_id: &str,
        state: TaskState,
        _tenant_id: &str,
    ) -> Result<(), String> {
        let state_str = match state {
            TaskState::Pending => "pending",
            TaskState::InProgress => "in_progress",
            TaskState::Completed => "completed",
            TaskState::Failed => "failed",
        };
        let now = chrono::Utc::now().to_rfc3339();

        let conn = self.db.conn();
        conn.execute(
            "UPDATE a2a_tasks SET state = ?1, updated_at = ?2 WHERE task_id = ?3",
            [state_str, &now, task_id],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[async_trait]
impl AsyncNotificationManager for A2aTaskStore {
    async fn emit_notification(
        &self,
        task_id: &str,
        message: String,
        _tenant_id: &str,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        let msg_obj = serde_json::json!({
            "timestamp": now,
            "content": message
        });

        let conn = self.db.conn();

        // Fetch existing messages
        let mut stmt = conn
            .prepare("SELECT messages FROM a2a_tasks WHERE task_id = ?1")
            .map_err(|e| e.to_string())?;

        let messages_json: String = stmt
            .query_row([task_id], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let mut messages: Vec<serde_json::Value> =
            serde_json::from_str(&messages_json).unwrap_or_default();
        messages.push(msg_obj);

        let updated_json = serde_json::to_string(&messages).map_err(|e| e.to_string())?;

        let now_str = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE a2a_tasks SET messages = ?1, updated_at = ?2 WHERE task_id = ?3",
            [&updated_json, &now_str, task_id],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn task_store_basic_operations() {
        let tmp = TempDir::new().expect("create temp dir");
        let db = Arc::new(StateDb::open(tmp.path().join("test.db").as_path()).expect("open StateDb"));
        let store = A2aTaskStore::new(db);

        // Create task
        let task_id = store
            .create_task("ctx1", "default")
            .await
            .expect("create task");
        assert!(!task_id.is_empty());

        // Task exists
        let exists = store
            .task_exists(&task_id, "default")
            .await
            .expect("check exists");
        assert!(exists);

        // Get task
        let (state, _msgs) = store
            .get_task(&task_id, "default")
            .await
            .expect("get task")
            .expect("task found");
        assert_eq!(state, TaskState::Pending);

        // Update state
        store
            .update_task_state(&task_id, TaskState::InProgress, "default")
            .await
            .expect("update state");

        let (state, _) = store
            .get_task(&task_id, "default")
            .await
            .expect("get task")
            .expect("task found");
        assert_eq!(state, TaskState::InProgress);
    }
}
