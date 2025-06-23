/*
Workflow container

Workflows are Node entities that encapsulate the orchestration logic and can be triggered by 
various events or conditions.
They allow for advanced control over job execution, including sequential, parallel, and 
event-driven processing.
They can also define custom execution orders and stages, enabling sophisticated 
orchestration patterns.
*/
use crate::core::platform::container::job::{Job, JobExecutionMode};
use crate::core::platform::manager::queue_service::QueueConfig;
use crate::core::platform::manager::orchestrator::OrchestrationContext;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::core::platform::container::trigger::TriggerCondition;

/// Workflow definition for complex orchestrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub jobs: Vec<Job>,
    pub listeners: Vec<WorkflowListener>,
    pub queues: Vec<WorkflowQueue>,
    pub execution_order: WorkflowExecutionOrder,
    pub context: OrchestrationContext,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowListener {
    pub name: String,
    pub conditions: Vec<TriggerCondition>,
    pub target_job_id: Option<Uuid>,
    pub target_queue: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowQueue {
    pub name: String,
    pub config: QueueConfig,
    pub processor_job_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowExecutionOrder {
    Sequential,
    Parallel,
    EventDriven,
    Custom(Vec<WorkflowStage>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStage {
    pub name: String,
    pub job_ids: Vec<Uuid>,
    pub dependencies: Vec<String>, // Stage names this stage depends on
    pub execution_mode: JobExecutionMode,
}