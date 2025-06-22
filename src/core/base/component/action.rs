/*
Action Component

An Action Component represents an action that can be taken in the system.
It is the base abstraction for all executable entities in the In4me system.

Actions contain:
- Identity and metadata (id, name, description)
- Execution tracking (timestamps, run counts, status)  
- Error handling and results
- Execution context and parameters

Actions are extended by platform-level containers like Tasks and Jobs to provide
specific orchestration capabilities while maintaining a common base interface.

This component enables event-driven architectures where Events trigger Actions
through a decoupled system of Triggers and Action Services.
*/

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Debug;
use uuid::Uuid;
use thiserror::Error;

/// Represents the execution status of an Action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionStatus {
    /// Action has been created but not yet executed
    Pending,
    /// Action is currently executing
    Running,
    /// Action completed successfully
    Completed,
    /// Action failed with an error
    Failed,
    /// Action was cancelled before completion
    Cancelled,
    /// Action execution timed out
    TimedOut,
}

/// Represents the priority level of an Action
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActionPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Execution context for an Action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContext {
    /// User or system that initiated the action
    pub initiator: String,
    /// Correlation ID for tracking related actions
    pub correlation_id: Option<Uuid>,
    /// Environment context (dev, staging, prod)
    pub environment: String,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

/// Execution result for an Action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,
    /// Duration of execution in milliseconds
    pub duration_ms: u64,
    /// Optional result data
    pub data: Option<Value>,
    /// Error message if failed
    pub error: Option<String>,
    /// Additional result metadata
    pub metadata: HashMap<String, Value>,
}

/// Base Action component that all executable entities extend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Unique identifier for the action
    pub id: Uuid,
    /// Human-readable name
    pub name: String,
    /// Detailed description of what this action does
    pub description: String,
    /// Source component/service that defined this action
    pub source: String,
    /// Target service or endpoint this action calls
    pub target_service: String,
    /// Arguments/parameters for execution
    pub arguments: HashMap<String, Value>,
    /// Execution priority
    pub priority: ActionPriority,
    /// Current execution status
    pub status: ActionStatus,
    /// Execution context
    pub context: ActionContext,
    /// When the action was created
    pub created_at: DateTime<Utc>,
    /// When the action was last updated
    pub updated_at: DateTime<Utc>,
    /// When the action was last executed
    pub last_execution: Option<DateTime<Utc>>,
    /// Number of times this action has been executed
    pub execution_count: u32,
    /// Results from the last execution
    pub last_result: Option<ActionResult>,
    /// Maximum execution time allowed (in seconds)
    pub timeout_seconds: Option<u32>,
    /// Whether this action can be retried on failure
    pub retryable: bool,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Current retry attempt count
    pub retry_count: u32,
}

impl Action {
    /// Creates a new Action with required parameters
    pub fn new(
        name: String,
        description: String,
        source: String,
        target_service: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            source,
            target_service,
            arguments: HashMap::new(),
            priority: ActionPriority::Normal,
            status: ActionStatus::Pending,
            context: ActionContext {
                initiator: "system".to_string(),
                correlation_id: None,
                environment: "default".to_string(),
                metadata: HashMap::new(),
            },
            created_at: now,
            updated_at: now,
            last_execution: None,
            execution_count: 0,
            last_result: None,
            timeout_seconds: None,
            retryable: true,
            max_retries: 3,
            retry_count: 0,
        }
    }

    /// Creates a new Action with full context
    pub fn new_with_context(
        name: String,
        description: String,
        source: String,
        target_service: String,
        context: ActionContext,
    ) -> Self {
        let mut action = Self::new(name, description, source, target_service);
        action.context = context;
        action
    }

    /// Sets the execution priority
    pub fn with_priority(mut self, priority: ActionPriority) -> Self {
        self.priority = priority;
        self.updated_at = Utc::now();
        self
    }

    /// Sets execution timeout
    pub fn with_timeout(mut self, timeout_seconds: u32) -> Self {
        self.timeout_seconds = Some(timeout_seconds);
        self.updated_at = Utc::now();
        self
    }

    /// Sets retry configuration
    pub fn with_retry_config(mut self, retryable: bool, max_retries: u32) -> Self {
        self.retryable = retryable;
        self.max_retries = max_retries;
        self.updated_at = Utc::now();
        self
    }

    /// Adds an argument for execution
    pub fn add_argument<T: Serialize>(&mut self, key: String, value: T) -> Result<(), ActionError> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| ActionError::SerializationError(e.to_string()))?;
        self.arguments.insert(key, json_value);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Gets an argument value
    pub fn get_argument(&self, key: &str) -> Option<&Value> {
        self.arguments.get(key)
    }

    /// Sets multiple arguments at once
    pub fn set_arguments(&mut self, arguments: HashMap<String, Value>) {
        self.arguments = arguments;
        self.updated_at = Utc::now();
    }

    /// Updates the action status
    pub fn set_status(&mut self, status: ActionStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Records the start of execution
    pub fn start_execution(&mut self) {
        self.status = ActionStatus::Running;
        self.last_execution = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Records the completion of execution
    pub fn complete_execution(&mut self, result: ActionResult) {
        self.status = if result.success {
            ActionStatus::Completed
        } else {
            ActionStatus::Failed
        };
        self.last_result = Some(result);
        self.execution_count += 1;
        self.retry_count = 0; // Reset retry count on completion
        self.updated_at = Utc::now();
    }

    /// Records a failed execution and determines if retry is possible
    pub fn fail_execution(&mut self, error: String, duration_ms: u64) -> bool {
        let result = ActionResult {
            success: false,
            duration_ms,
            data: None,
            error: Some(error),
            metadata: HashMap::new(),
        };

        self.last_result = Some(result);
        self.execution_count += 1;
        self.retry_count += 1;
        self.updated_at = Utc::now();

        // Determine if we can retry - allow retries up to and including max_retries
        let can_retry = self.retryable && self.retry_count <= self.max_retries;
        
        if can_retry {
            self.status = ActionStatus::Pending; // Reset to pending for retry
        } else {
            self.status = ActionStatus::Failed;
        }

        can_retry
    }

    /// Cancels the action
    pub fn cancel(&mut self) {
        self.status = ActionStatus::Cancelled;
        self.updated_at = Utc::now();
    }

    /// Marks the action as timed out
    pub fn timeout(&mut self) {
        self.status = ActionStatus::TimedOut;
        self.updated_at = Utc::now();
    }

    /// Checks if the action can be executed
    pub fn can_execute(&self) -> bool {
        matches!(self.status, ActionStatus::Pending)
    }

    /// Checks if the action is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            ActionStatus::Completed | ActionStatus::Failed | ActionStatus::Cancelled | ActionStatus::TimedOut
        )
    }

    /// Gets the success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.execution_count == 0 {
            return 0.0;
        }

        let successful_executions = if let Some(result) = &self.last_result {
            if result.success { 1.0 } else { 0.0 }
        } else {
            0.0
        };

        (successful_executions / self.execution_count as f64) * 100.0
    }

    /// Gets the average execution duration
    pub fn average_duration_ms(&self) -> Option<u64> {
        self.last_result.as_ref().map(|r| r.duration_ms)
    }

    /// Resets the action to pending state for re-execution
    pub fn reset(&mut self) {
        self.status = ActionStatus::Pending;
        self.retry_count = 0;
        self.updated_at = Utc::now();
    }

    /// Creates a clone of this action with a new ID (useful for recurring actions)
    pub fn clone_for_new_execution(&self) -> Self {
        let mut cloned = self.clone();
        cloned.id = Uuid::new_v4();
        cloned.status = ActionStatus::Pending;
        cloned.last_execution = None;
        cloned.execution_count = 0;
        cloned.last_result = None;
        cloned.retry_count = 0;
        cloned.created_at = Utc::now();
        cloned.updated_at = Utc::now();
        cloned
    }
}

impl Default for Action {
    fn default() -> Self {
        Self::new(
            "Default Action".to_string(),
            "A default action".to_string(),
            "system".to_string(),
            "default_service".to_string(),
        )
    }
}

impl PartialEq for Action {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Action {}

/// Errors that can occur when working with Actions
#[derive(Debug, Error)]
pub enum ActionError {
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("Timeout error: execution exceeded {0} seconds")]
    TimeoutError(u32),
    #[error("Action is not in a valid state for this operation: {0:?}")]
    InvalidState(ActionStatus),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_action_creation() {
        let action = Action::new(
            "Test Action".to_string(),
            "A test action".to_string(),
            "test_source".to_string(),
            "test_service".to_string(),
        );

        assert_eq!(action.name, "Test Action");
        assert_eq!(action.status, ActionStatus::Pending);
        assert_eq!(action.execution_count, 0);
        assert!(action.can_execute());
        assert!(!action.is_terminal());
    }

    #[test]
    fn test_action_execution_lifecycle() {
        let mut action = Action::new(
            "Test Action".to_string(),
            "A test action".to_string(),
            "test_source".to_string(),
            "test_service".to_string(),
        );

        // Start execution
        action.start_execution();
        assert_eq!(action.status, ActionStatus::Running);
        assert!(action.last_execution.is_some());

        // Complete execution
        let result = ActionResult {
            success: true,
            duration_ms: 1000,
            data: Some(json!({"result": "success"})),
            error: None,
            metadata: HashMap::new(),
        };
        action.complete_execution(result);

        assert_eq!(action.status, ActionStatus::Completed);
        assert_eq!(action.execution_count, 1);
        assert_eq!(action.retry_count, 0);
        assert!(action.is_terminal());
    }

    #[test]
    fn test_action_retry_logic() {
        let mut action = Action::new(
            "Test Action".to_string(),
            "A test action".to_string(),
            "test_source".to_string(),
            "test_service".to_string(),
        ).with_retry_config(true, 2);

        // First failure - should allow retry
        let can_retry = action.fail_execution("Test error".to_string(), 500);
        assert!(can_retry);
        assert_eq!(action.status, ActionStatus::Pending);
        assert_eq!(action.retry_count, 1);

        // Second failure - should allow retry
        let can_retry = action.fail_execution("Test error".to_string(), 500);
        assert!(can_retry);
        assert_eq!(action.status, ActionStatus::Pending);
        assert_eq!(action.retry_count, 2);

        // Third failure - should not allow retry (exceeded max_retries)
        let can_retry = action.fail_execution("Test error".to_string(), 500);
        assert!(!can_retry);
        assert_eq!(action.status, ActionStatus::Failed);
        assert_eq!(action.retry_count, 3);
    }

    #[test]
    fn test_action_arguments() {
        let mut action = Action::default();

        // Add arguments
        action.add_argument("string_arg".to_string(), "test_value").unwrap();
        action.add_argument("number_arg".to_string(), 42).unwrap();
        action.add_argument("object_arg".to_string(), json!({"key": "value"})).unwrap();

        // Verify arguments
        assert_eq!(action.get_argument("string_arg"), Some(&json!("test_value")));
        assert_eq!(action.get_argument("number_arg"), Some(&json!(42)));
        assert_eq!(action.get_argument("object_arg"), Some(&json!({"key": "value"})));
        assert_eq!(action.get_argument("missing_arg"), None);
    }

    #[test]
    fn test_action_priority() {
        let action = Action::default()
            .with_priority(ActionPriority::Critical)
            .with_timeout(30);

        assert_eq!(action.priority, ActionPriority::Critical);
        assert_eq!(action.timeout_seconds, Some(30));
    }

    #[test]
    fn test_action_context() {
        let context = ActionContext {
            initiator: "user123".to_string(),
            correlation_id: Some(Uuid::new_v4()),
            environment: "production".to_string(),
            metadata: HashMap::new(),
        };

        let action = Action::new_with_context(
            "Contextual Action".to_string(),
            "An action with context".to_string(),
            "test_source".to_string(),
            "test_service".to_string(),
            context.clone(),
        );

        assert_eq!(action.context.initiator, "user123");
        assert_eq!(action.context.environment, "production");
        assert!(action.context.correlation_id.is_some());
    }

    #[test]
    fn test_action_clone_for_new_execution() {
        let mut original = Action::default();
        original.execution_count = 5;
        original.retry_count = 2;
        original.status = ActionStatus::Failed;

        let cloned = original.clone_for_new_execution();

        assert_ne!(original.id, cloned.id);
        assert_eq!(cloned.status, ActionStatus::Pending);
        assert_eq!(cloned.execution_count, 0);
        assert_eq!(cloned.retry_count, 0);
        assert!(cloned.last_result.is_none());
    }

    #[test]
    fn test_action_state_management() {
        let mut action = Action::default();

        // Test cancellation
        action.cancel();
        assert_eq!(action.status, ActionStatus::Cancelled);
        assert!(action.is_terminal());

        // Reset and test timeout
        action.reset();
        assert_eq!(action.status, ActionStatus::Pending);
        assert!(!action.is_terminal());

        action.timeout();
        assert_eq!(action.status, ActionStatus::TimedOut);
        assert!(action.is_terminal());
    }
}