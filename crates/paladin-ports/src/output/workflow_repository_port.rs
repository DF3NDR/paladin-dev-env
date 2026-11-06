//! Workflow persistence repository port.
//!
//! Defines the abstract contract that SQL-backed (or any other) workflow-state
//! stores must implement so the orchestrator can persist execution progress and
//! resume incomplete workflows after a restart or crash.
//!
//! The port deliberately exposes only a coarse-grained persistence status
//! ([`WorkflowPersistenceStatus`](crate::output::workflow_repository_port::WorkflowPersistenceStatus)) plus the set of already-completed job ids.
//! The orchestrator's fine-grained internal execution state stays private; this
//! DTO carries just enough to recover without re-running completed work.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use paladin_core::platform::container::workflow::Workflow;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Coarse-grained persisted lifecycle status of a workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowPersistenceStatus {
    /// Created but not yet started.
    Pending,
    /// Currently executing (or interrupted mid-execution).
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with at least one failed job.
    Failed,
}

impl WorkflowPersistenceStatus {
    /// Returns the lowercase string representation used for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkflowPersistenceStatus::Pending => "pending",
            WorkflowPersistenceStatus::Running => "running",
            WorkflowPersistenceStatus::Completed => "completed",
            WorkflowPersistenceStatus::Failed => "failed",
        }
    }

    /// Parses a stored string back into a status.
    pub fn from_str_value(value: &str) -> Result<Self, WorkflowRepositoryError> {
        match value {
            "pending" => Ok(WorkflowPersistenceStatus::Pending),
            "running" => Ok(WorkflowPersistenceStatus::Running),
            "completed" => Ok(WorkflowPersistenceStatus::Completed),
            "failed" => Ok(WorkflowPersistenceStatus::Failed),
            other => Err(WorkflowRepositoryError::DeserializationError(format!(
                "unknown workflow status: {other}"
            ))),
        }
    }

    /// Returns `true` if the status represents a terminal (finished) state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            WorkflowPersistenceStatus::Completed | WorkflowPersistenceStatus::Failed
        )
    }
}

/// A persisted snapshot of a workflow's definition and execution progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedWorkflow {
    /// The workflow's unique identifier.
    pub workflow_id: Uuid,
    /// Current coarse-grained lifecycle status.
    pub status: WorkflowPersistenceStatus,
    /// Ids of jobs that have already completed successfully.
    pub completed_job_ids: Vec<Uuid>,
    /// The full workflow definition, used to resume remaining work.
    pub definition: Workflow,
    /// Timestamp of the most recent persistence update.
    pub updated_at: DateTime<Utc>,
}

impl PersistedWorkflow {
    /// Creates a new pending record for the given workflow definition.
    pub fn pending(definition: Workflow) -> Self {
        Self {
            workflow_id: definition.id,
            status: WorkflowPersistenceStatus::Pending,
            completed_job_ids: Vec::new(),
            definition,
            updated_at: Utc::now(),
        }
    }
}

/// Errors raised by workflow repository adapters.
#[derive(Debug, Error)]
pub enum WorkflowRepositoryError {
    /// The underlying datastore reported a failure.
    #[error("Workflow repository error: {0}")]
    RepositoryError(String),
    /// A stored record could not be deserialized.
    #[error("Workflow deserialization error: {0}")]
    DeserializationError(String),
    /// A record could not be serialized for storage.
    #[error("Workflow serialization error: {0}")]
    SerializationError(String),
}

/// Repository port for persisting and recovering [`Workflow`] execution state.
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::workflow_repository_port::{
///     WorkflowRepositoryPort, PersistedWorkflow, WorkflowRepositoryError,
/// };
/// use paladin_core::platform::container::workflow::{Workflow, WorkflowExecutionOrder};
/// use paladin_core::platform::container::orchestration_context::OrchestrationContext;
/// use uuid::Uuid;
/// use chrono::Utc;
///
/// async fn checkpoint_workflow(
///     repo: &dyn WorkflowRepositoryPort,
/// ) -> Result<(), WorkflowRepositoryError> {
///     let workflow = Workflow {
///         id: Uuid::new_v4(),
///         name: "nightly-ingest".to_string(),
///         description: "Ingest and index new content".to_string(),
///         jobs: vec![],
///         listeners: vec![],
///         queues: vec![],
///         execution_order: WorkflowExecutionOrder::Sequential,
///         context: OrchestrationContext::new("scheduler".to_string(), "prod".to_string()),
///         created_at: Utc::now(),
///         updated_at: Utc::now(),
///     };
///
///     repo.save(&PersistedWorkflow::pending(workflow)).await?;
///
///     let incomplete = repo.list_incomplete().await?;
///     println!("{} workflows still in progress", incomplete.len());
///
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait WorkflowRepositoryPort: Send + Sync {
    /// Insert or overwrite the persisted state for a workflow (upsert).
    async fn save(&self, record: &PersistedWorkflow) -> Result<(), WorkflowRepositoryError>;

    /// Load a single persisted workflow by id, if present.
    async fn load(
        &self,
        workflow_id: Uuid,
    ) -> Result<Option<PersistedWorkflow>, WorkflowRepositoryError>;

    /// Return every workflow that has not reached a terminal state.
    async fn list_incomplete(&self) -> Result<Vec<PersistedWorkflow>, WorkflowRepositoryError>;
}
