//! T2-7 A2A protocol handler and task store tests.

use open_ontologies::state::StateDb;
use open_ontologies::a2a::task_store::{A2aTaskStore, AsyncTaskManager, TaskState};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn a2a_task_store_create_and_get() {
    let tmp = TempDir::new().expect("create temp dir");
    let db = Arc::new(StateDb::open(tmp.path().join("test.db").as_path()).expect("open StateDb"));
    let store = A2aTaskStore::new(db);

    // Create task
    let task_id = store
        .create_task("ctx1", "default")
        .await
        .expect("create task");
    assert!(!task_id.is_empty());

    // Get task - should exist
    let task = store
        .get_task(&task_id, "default")
        .await
        .expect("get task")
        .expect("task should exist");

    let (state, _msgs) = task;
    assert_eq!(state, TaskState::Pending);
}

#[tokio::test]
async fn a2a_task_store_update_state() {
    let tmp = TempDir::new().expect("create temp dir");
    let db = Arc::new(StateDb::open(tmp.path().join("test.db").as_path()).expect("open StateDb"));
    let store = A2aTaskStore::new(db);

    // Create task
    let task_id = store
        .create_task("ctx2", "default")
        .await
        .expect("create task");

    // Update to in-progress
    store
        .update_task_state(&task_id, TaskState::InProgress, "default")
        .await
        .expect("update state");

    // Verify state changed
    let (state, _) = store
        .get_task(&task_id, "default")
        .await
        .expect("get task")
        .expect("task found");
    assert_eq!(state, TaskState::InProgress);

    // Update to completed
    store
        .update_task_state(&task_id, TaskState::Completed, "default")
        .await
        .expect("update to completed");

    let (state, _) = store
        .get_task(&task_id, "default")
        .await
        .expect("get task")
        .expect("task found");
    assert_eq!(state, TaskState::Completed);
}

#[tokio::test]
async fn a2a_task_exists() {
    let tmp = TempDir::new().expect("create temp dir");
    let db = Arc::new(StateDb::open(tmp.path().join("test.db").as_path()).expect("open StateDb"));
    let store = A2aTaskStore::new(db);

    // Create task
    let task_id = store
        .create_task("ctx3", "default")
        .await
        .expect("create task");

    // Verify it exists
    let exists = store
        .task_exists(&task_id, "default")
        .await
        .expect("check exists");
    assert!(exists);

    // Verify non-existent task returns false
    let exists = store
        .task_exists("nonexistent-task-id", "default")
        .await
        .expect("check nonexistent");
    assert!(!exists);
}
