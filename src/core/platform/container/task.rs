/*
Task Container

A Task is a platform-level container that extends the base Action component 
to provide specific execution capabilities. Tasks represent individual units 
of work that can be executed independently or as part of a larger Job.

Tasks add:
- Service-based execution through TaskService trait
- Task-specific error handling and states
- Integration with the Action lifecycle
- Platform-level orchestration capabilities

Tasks are the building blocks for Jobs and can be scheduled, queued, 
and executed through various platform services.
*/

use std::fmt::Debug;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use uuid::Uuid;

// Import the base Action component
use crate::core::base::component::action::{Action, ActionStatus, ActionResult, ActionError, ActionPriority};

/// Task-specific execution modes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskExecutionMode {
    /// Execute once and complete
    OneTime,
    /// Can be retried on failure
    Retryable,
    /// Idempotent - safe to run multiple times
    Idempotent,
}

/// Task container that extends the base Action component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Base action functionality
    pub action: Action,
    /// Task-specific execution mode
    pub execution_mode: TaskExecutionMode,
    /// Service name that will execute this task
    pub service_name: String,
    /// Task-specific metadata
    pub task_metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl Task {
    /// Creates a new Task with the specified service
    pub fn new(name: String, description: String, service_name: String) -> Self {
        let action = Action::new(
            name.clone(),
            description,
            "task_manager".to_string(),
            service_name.clone(),
        );

        Self {
            action,
            execution_mode: TaskExecutionMode::Retryable,
            service_name,
            task_metadata: std::collections::HashMap::new(),
        }
    }

    /// Creates a new Task with full configuration
    pub fn new_with_config(
        name: String,
        description: String,
        service_name: String,
        execution_mode: TaskExecutionMode,
        priority: ActionPriority,
    ) -> Self {
        let action = Action::new(
            name.clone(),
            description,
            "task_manager".to_string(),
            service_name.clone(),
        ).with_priority(priority);

        Self {
            action,
            execution_mode,
            service_name,
            task_metadata: std::collections::HashMap::new(),
        }
    }

    /// Sets the execution mode
    pub fn with_execution_mode(mut self, mode: TaskExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Sets task priority
    pub fn with_priority(mut self, priority: ActionPriority) -> Self {
        self.action = self.action.with_priority(priority);
        self
    }

    /// Sets timeout for task execution
    pub fn with_timeout(mut self, timeout_seconds: u32) -> Self {
        self.action = self.action.with_timeout(timeout_seconds);
        self
    }

    /// Adds task-specific metadata
    pub fn add_metadata<T: Serialize>(&mut self, key: String, value: T) -> Result<(), TaskError> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| TaskError::SerializationError(e.to_string()))?;
        self.task_metadata.insert(key, json_value);
        Ok(())
    }

    /// Gets task metadata
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.task_metadata.get(key)
    }

    /// Executes the task using the provided service
    pub async fn execute<T: TaskService + ?Sized>(&mut self, service: &T) -> Result<(), TaskError> {
        // Check if task can be executed
        if !self.action.can_execute() {
            return Err(TaskError::InvalidState(self.action.status.clone()));
        }

        // Start execution
        self.action.start_execution();

        // Execute the task
        let start_time = std::time::Instant::now();
        let execution_result = service.execute(&self.action).await;
        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Handle result
        match execution_result {
            Ok(result_data) => {
                let action_result = ActionResult {
                    success: true,
                    duration_ms,
                    data: result_data,
                    error: None,
                    metadata: std::collections::HashMap::new(),
                };
                self.action.complete_execution(action_result);
                Ok(())
            }
            Err(task_error) => {
                let error_message = task_error.to_string();
                let can_retry = self.action.fail_execution(error_message, duration_ms);
                
                if can_retry && matches!(self.execution_mode, TaskExecutionMode::Retryable) {
                    Err(TaskError::RetryableFailure(task_error.to_string()))
                } else {
                    Err(task_error)
                }
            }
        }
    }

    /// Executes the task without a service (for testing or simple tasks)
    pub async fn execute_simple(&mut self) -> Result<(), TaskError> {
        if !self.action.can_execute() {
            return Err(TaskError::InvalidState(self.action.status.clone()));
        }

        self.action.start_execution();

        // Simulate simple execution
        let start_time = std::time::Instant::now();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let duration_ms = start_time.elapsed().as_millis() as u64;

        let action_result = ActionResult {
            success: true,
            duration_ms,
            data: Some(serde_json::json!({"message": "Task completed successfully"})),
            error: None,
            metadata: std::collections::HashMap::new(),
        };

        self.action.complete_execution(action_result);
        Ok(())
    }

    /// Cancels the task
    pub fn cancel(&mut self) {
        self.action.cancel();
    }

    /// Resets the task for re-execution
    pub fn reset(&mut self) {
        self.action.reset();
    }

    /// Checks if the task can be executed
    pub fn can_execute(&self) -> bool {
        self.action.can_execute()
    }

    /// Checks if the task is in a terminal state
    pub fn is_complete(&self) -> bool {
        self.action.is_terminal()
    }

    /// Gets the task's current status
    pub fn status(&self) -> &ActionStatus {
        &self.action.status
    }

    /// Gets the task ID
    pub fn id(&self) -> Uuid {
        self.action.id
    }

    /// Gets the task name
    pub fn name(&self) -> &str {
        &self.action.name
    }

    /// Gets execution statistics
    pub fn execution_stats(&self) -> TaskStats {
        TaskStats {
            execution_count: self.action.execution_count,
            success_rate: self.action.success_rate(),
            average_duration_ms: self.action.average_duration_ms(),
            last_execution: self.action.last_execution,
        }
    }

    /// Creates a new instance for re-execution (useful for recurring tasks)
    pub fn clone_for_new_execution(&self) -> Self {
        let mut cloned = self.clone();
        cloned.action = self.action.clone_for_new_execution();
        cloned
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.action.id == other.action.id
    }
}

impl Eq for Task {}

/// Task execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStats {
    pub execution_count: u32,
    pub success_rate: f64,
    pub average_duration_ms: Option<u64>,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

/// Task-specific errors
#[derive(Debug, Error)]
pub enum TaskError {
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("Task execution timed out")]
    Timeout,
    #[error("Task failed but can be retried: {0}")]
    RetryableFailure(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Task is not in a valid state for execution: {0:?}")]
    InvalidState(ActionStatus),
    #[error("Action error: {0}")]
    ActionError(#[from] ActionError),
}

/// Trait for services that can execute tasks
#[async_trait]
pub trait TaskService: Debug + Send + Sync {
    /// Executes the task and returns optional result data
    async fn execute(&self, action: &Action) -> Result<Option<serde_json::Value>, TaskError>;
    
    /// Returns the service name
    fn name(&self) -> &str;
    
    /// Checks if the service can handle the given task
    fn can_handle(&self, task: &Task) -> bool {
        task.service_name == self.name()
    }
}

// Example task services
#[derive(Debug, Clone)]
pub struct DataBackupService {
    pub backup_path: String,
}

#[async_trait]
impl TaskService for DataBackupService {
    async fn execute(&self, action: &Action) -> Result<Option<serde_json::Value>, TaskError> {
        println!("Running data backup to: {} for task: {}", self.backup_path, action.name);
        
        // Simulate backup work
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        
        let result = serde_json::json!({
            "backup_path": self.backup_path,
            "files_backed_up": 150,
            "size_mb": 1024
        });
        
        println!("Data backup completed for task: {}", action.name);
        Ok(Some(result))
    }

    fn name(&self) -> &str {
        "DataBackupService"
    }
}

#[derive(Debug, Clone)]
pub struct ContentIndexingService {
    pub index_name: String,
}

#[async_trait]
impl TaskService for ContentIndexingService {
    async fn execute(&self, action: &Action) -> Result<Option<serde_json::Value>, TaskError> {
        println!("Running content indexing for index: {} for task: {}", self.index_name, action.name);
        
        // Simulate indexing work
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        
        let result = serde_json::json!({
            "index_name": self.index_name,
            "documents_indexed": 500,
            "index_size_mb": 256
        });
        
        println!("Content indexing completed for task: {}", action.name);
        Ok(Some(result))
    }

    fn name(&self) -> &str {
        "ContentIndexingService"
    }
}

#[derive(Debug, Clone)]
pub struct EmailNotificationService {
    pub smtp_server: String,
}

#[async_trait]
impl TaskService for EmailNotificationService {
    async fn execute(&self, action: &Action) -> Result<Option<serde_json::Value>, TaskError> {
        println!("Sending email notification via: {} for task: {}", self.smtp_server, action.name);
        
        // Get email details from action arguments
        let to_email = action.get_argument("to_email")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown@example.com");
        
        // Simulate email sending
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        
        let result = serde_json::json!({
            "smtp_server": self.smtp_server,
            "to_email": to_email,
            "status": "sent"
        });
        
        println!("Email notification sent to: {} for task: {}", to_email, action.name);
        Ok(Some(result))
    }

    fn name(&self) -> &str {
        "EmailNotificationService"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_task_creation() {
        let task = Task::new(
            "Test Task".to_string(),
            "A test task".to_string(),
            "TestService".to_string(),
        );

        assert_eq!(task.name(), "Test Task");
        assert_eq!(task.service_name, "TestService");
        assert_eq!(task.execution_mode, TaskExecutionMode::Retryable);
        assert!(task.can_execute());
    }

    #[tokio::test]
    async fn test_task_execution_with_service() {
        let mut task = Task::new(
            "Backup Task".to_string(),
            "Backup data task".to_string(),
            "DataBackupService".to_string(),
        );

        let service = DataBackupService {
            backup_path: "/tmp/backup".to_string(),
        };

        let result = task.execute(&service).await;
        assert!(result.is_ok());
        assert_eq!(task.status(), &ActionStatus::Completed);
        assert_eq!(task.action.execution_count, 1);
    }

    #[tokio::test]
    async fn test_task_simple_execution() {
        let mut task = Task::new(
            "Simple Task".to_string(),
            "A simple task".to_string(),
            "SimpleService".to_string(),
        );

        let result = task.execute_simple().await;
        assert!(result.is_ok());
        assert_eq!(task.status(), &ActionStatus::Completed);
    }

    #[tokio::test]
    async fn test_task_with_metadata() {
        let mut task = Task::new(
            "Metadata Task".to_string(),
            "Task with metadata".to_string(),
            "TestService".to_string(),
        );

        task.add_metadata("custom_field".to_string(), "custom_value").unwrap();
        task.add_metadata("priority_level".to_string(), 5).unwrap();

        assert_eq!(task.get_metadata("custom_field"), Some(&json!("custom_value")));
        assert_eq!(task.get_metadata("priority_level"), Some(&json!(5)));
    }

    #[tokio::test]
    async fn test_task_configuration() {
        let task = Task::new_with_config(
            "Config Task".to_string(),
            "Configured task".to_string(),
            "ConfigService".to_string(),
            TaskExecutionMode::Idempotent,
            ActionPriority::High,
        ).with_timeout(30);

        assert_eq!(task.execution_mode, TaskExecutionMode::Idempotent);
        assert_eq!(task.action.priority, ActionPriority::High);
        assert_eq!(task.action.timeout_seconds, Some(30));
    }

    #[tokio::test]
    async fn test_task_statistics() {
        let mut task = Task::new(
            "Stats Task".to_string(),
            "Task for statistics".to_string(),
            "StatsService".to_string(),
        );

        // Execute the task
        task.execute_simple().await.unwrap();

        let stats = task.execution_stats();
        assert_eq!(stats.execution_count, 1);
        assert_eq!(stats.success_rate, 100.0);
        assert!(stats.average_duration_ms.is_some());
        assert!(stats.last_execution.is_some());
    }

    #[tokio::test]
    async fn test_task_clone_for_new_execution() {
        let mut original = Task::new(
            "Original Task".to_string(),
            "Original task".to_string(),
            "TestService".to_string(),
        );

        // Execute the original
        original.execute_simple().await.unwrap();

        // Clone for new execution
        let cloned = original.clone_for_new_execution();

        assert_ne!(original.id(), cloned.id());
        assert_eq!(cloned.status(), &ActionStatus::Pending);
        assert_eq!(cloned.action.execution_count, 0);
    }

    #[tokio::test]
    async fn test_email_notification_service() {
        let mut task = Task::new(
            "Email Task".to_string(),
            "Send email notification".to_string(),
            "EmailNotificationService".to_string(),
        );

        // Add email arguments
        task.action.add_argument("to_email".to_string(), "user@example.com").unwrap();
        task.action.add_argument("subject".to_string(), "Test Subject").unwrap();

        let service = EmailNotificationService {
            smtp_server: "smtp.example.com".to_string(),
        };

        let result = task.execute(&service).await;
        assert!(result.is_ok());
        assert_eq!(task.status(), &ActionStatus::Completed);
    }
}