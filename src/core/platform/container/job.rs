/*
Job Container

A Job is a platform-level container that extends the base Action component 
to orchestrate multiple Tasks. Jobs provide higher-level workflow management
and can execute Tasks in sequence or parallel.

Jobs add:
- Task orchestration and dependency management
- Parallel and sequential execution modes
- Job-level error handling and rollback capabilities
- Workflow state management
- Integration with scheduling systems

Jobs are the primary unit of work for the Scheduler and can represent
complex business processes composed of multiple related Tasks.
*/

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use uuid::Uuid;
use futures::future::join_all;

// Import base Action and Task components
use crate::core::base::component::action::{Action, ActionStatus, ActionResult, ActionError, ActionPriority};
use super::task::{Task, TaskService};

/// Job execution strategy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobExecutionMode {
    /// Execute all tasks in sequence, stop on first failure
    Sequential,
    /// Execute all tasks in sequence, continue on failures
    SequentialContinueOnError,
    /// Execute all tasks in parallel
    Parallel,
    /// Execute tasks in parallel with a maximum concurrency limit
    ParallelLimited(usize),
    /// Execute based on task dependencies (DAG)
    Dependency,
}

/// Job rollback strategy on failure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobRollbackStrategy {
    /// No rollback - leave tasks in their current state
    None,
    /// Cancel all running tasks
    CancelRunning,
    /// Rollback completed tasks (if they support it)
    RollbackCompleted,
    /// Full rollback - cancel running and rollback completed
    Full,
}

/// Job container that extends the base Action component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Base action functionality
    pub action: Action,
    /// Tasks that make up this job
    pub tasks: Vec<Task>,
    /// Execution mode for tasks
    pub execution_mode: JobExecutionMode,
    /// Rollback strategy on failure
    pub rollback_strategy: JobRollbackStrategy,
    /// Task execution results
    pub task_results: HashMap<Uuid, ActionResult>,
    /// Job-specific metadata
    pub job_metadata: HashMap<String, serde_json::Value>,
    /// Task dependencies (task_id -> dependent_task_ids)
    pub task_dependencies: HashMap<Uuid, Vec<Uuid>>,
}

impl Job {
    /// Creates a new Job with specified tasks
    pub fn new(name: String, description: String, tasks: Vec<Task>) -> Self {
        let action = Action::new(
            name.clone(),
            description,
            "job_manager".to_string(),
            "job_orchestrator".to_string(),
        );

        Self {
            action,
            tasks,
            execution_mode: JobExecutionMode::Sequential,
            rollback_strategy: JobRollbackStrategy::CancelRunning,
            task_results: HashMap::new(),
            job_metadata: HashMap::new(),
            task_dependencies: HashMap::new(),
        }
    }

    /// Creates a new Job with full configuration
    pub fn new_with_config(
        name: String,
        description: String,
        tasks: Vec<Task>,
        execution_mode: JobExecutionMode,
        priority: ActionPriority,
    ) -> Self {
        let action = Action::new(
            name.clone(),
            description,
            "job_manager".to_string(),
            "job_orchestrator".to_string(),
        ).with_priority(priority);

        Self {
            action,
            tasks,
            execution_mode,
            rollback_strategy: JobRollbackStrategy::CancelRunning,
            task_results: HashMap::new(),
            job_metadata: HashMap::new(),
            task_dependencies: HashMap::new(),
        }
    }

    /// Sets the execution mode
    pub fn with_execution_mode(mut self, mode: JobExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Sets the rollback strategy
    pub fn with_rollback_strategy(mut self, strategy: JobRollbackStrategy) -> Self {
        self.rollback_strategy = strategy;
        self
    }

    /// Sets job priority
    pub fn with_priority(mut self, priority: ActionPriority) -> Self {
        self.action = self.action.with_priority(priority);
        self
    }

    /// Sets timeout for job execution
    pub fn with_timeout(mut self, timeout_seconds: u32) -> Self {
        self.action = self.action.with_timeout(timeout_seconds);
        self
    }

    /// Adds a task to the job
    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    /// Removes a task by ID
    pub fn remove_task(&mut self, task_id: Uuid) -> bool {
        let original_len = self.tasks.len();
        self.tasks.retain(|t| t.id() != task_id);
        self.task_results.remove(&task_id);
        self.task_dependencies.remove(&task_id);
        self.tasks.len() != original_len
    }

    /// Adds a task dependency
    pub fn add_task_dependency(&mut self, task_id: Uuid, depends_on: Vec<Uuid>) {
        self.task_dependencies.insert(task_id, depends_on);
    }

    /// Adds job metadata
    pub fn add_metadata<T: Serialize>(&mut self, key: String, value: T) -> Result<(), JobError> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| JobError::SerializationError(e.to_string()))?;
        self.job_metadata.insert(key, json_value);
        Ok(())
    }

    /// Executes the job with provided task services
    pub async fn execute(&mut self, services: &HashMap<String, Box<dyn TaskService>>) -> Result<(), JobError> {
        if self.tasks.is_empty() {
            return Err(JobError::NoTasks);
        }

        if !self.action.can_execute() {
            return Err(JobError::InvalidState(self.action.status.clone()));
        }

        // Start job execution
        self.action.start_execution();
        let start_time = std::time::Instant::now();

        // Execute based on mode
        let execution_result = match self.execution_mode {
            JobExecutionMode::Sequential => self.execute_sequential(services).await,
            JobExecutionMode::SequentialContinueOnError => self.execute_sequential_continue_on_error(services).await,
            JobExecutionMode::Parallel => self.execute_parallel(services).await,
            JobExecutionMode::ParallelLimited(limit) => self.execute_parallel_limited(services, limit).await,
            JobExecutionMode::Dependency => self.execute_with_dependencies(services).await,
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Handle execution result
        match execution_result {
            Ok(_) => {
                let action_result = ActionResult {
                    success: true,
                    duration_ms,
                    data: Some(self.create_job_result()),
                    error: None,
                    metadata: HashMap::new(),
                };
                self.action.complete_execution(action_result);
                Ok(())
            }
            Err(job_error) => {
                // Handle rollback if needed
                self.handle_rollback().await;

                let error_message = job_error.to_string();
                let can_retry = self.action.fail_execution(error_message, duration_ms);
                
                if can_retry {
                    Err(JobError::RetryableFailure(job_error.to_string()))
                } else {
                    Err(job_error)
                }
            }
        }
    }
    
    /// Executes tasks sequentially, stopping on first failure
    async fn execute_sequential(&mut self, services: &HashMap<String, Box<dyn TaskService>>) -> Result<(), JobError> {
        for task in &mut self.tasks {
            Self::execute_single_task(task, services).await?;
        }
        Ok(())
    }

    /// Executes tasks sequentially, continuing on errors
    async fn execute_sequential_continue_on_error(&mut self, services: &HashMap<String, Box<dyn TaskService>>) -> Result<(), JobError> {
        let mut has_failures = false;
        let mut error_messages = Vec::new();

        for task in &mut self.tasks {
            if let Err(error) = Self::execute_single_task(task, services).await {
                has_failures = true;
                error_messages.push(format!("Task '{}': {}", task.name(), error));
            }
        }

        if has_failures {
            Err(JobError::PartialFailure(error_messages))
        } else {
            Ok(())
        }
    }

    /// Executes tasks based on dependencies (simplified DAG execution)
    async fn execute_with_dependencies(&mut self, services: &HashMap<String, Box<dyn TaskService>>) -> Result<(), JobError> {
        // Pre-validate all services exist
        for task in &self.tasks {
            if !services.contains_key(&task.service_name) {
                return Err(JobError::ServiceNotFound(task.service_name.clone()));
            }
        }

        let mut completed_tasks = std::collections::HashSet::new();
        let mut remaining_task_indices: Vec<_> = (0..self.tasks.len()).collect();

        while !remaining_task_indices.is_empty() {
            let mut ready_task_indices = Vec::new();
            let mut not_ready_task_indices = Vec::new();

            // Find tasks that are ready to execute
            for &task_index in &remaining_task_indices {
                let task_id = self.tasks[task_index].id();
                let dependencies = self.task_dependencies.get(&task_id).cloned().unwrap_or_default();
                
                let ready = dependencies.iter().all(|dep_id| completed_tasks.contains(dep_id));
                
                if ready {
                    ready_task_indices.push(task_index);
                } else {
                    not_ready_task_indices.push(task_index);
                }
            }

            if ready_task_indices.is_empty() {
                return Err(JobError::CircularDependency);
            }

            // Execute ready tasks sequentially using execute_single_task
            for task_index in ready_task_indices {
                let task = &mut self.tasks[task_index];
                let task_id = task.id();
                
                match Self::execute_single_task(task, services).await {
                    Ok(_) => {
                        completed_tasks.insert(task_id);
                    }
                    Err(error) => {
                        return Err(error);
                    }
                }
            }

            remaining_task_indices = not_ready_task_indices;
        }

        Ok(())
    }

    /// Executes tasks in parallel
    async fn execute_parallel(&mut self, services: &HashMap<String, Box<dyn TaskService>>) -> Result<(), JobError> {
        // Collect task execution futures
        let mut futures = Vec::new();
        
        // Pre-validate all services exist
        for task in &self.tasks {
            if !services.contains_key(&task.service_name) {
                return Err(JobError::ServiceNotFound(task.service_name.clone()));
            }
        }
        
        // Split self to avoid multiple mutable borrows
        let tasks = &mut self.tasks;
        
        for (i, task) in tasks.iter_mut().enumerate() {
            let service = services.get(&task.service_name).unwrap(); // Safe due to validation above
            let future = async move {
                let result = task.execute(service.as_ref()).await;
                (i, result)
            };
            futures.push(future);
        }

        let results = join_all(futures).await;
        
        let mut errors = Vec::new();
        for (i, result) in results {
            if let Err(error) = result {
                errors.push(format!("Task '{}': {}", tasks[i].name(), error));
            }
        }

        if !errors.is_empty() {
            Err(JobError::PartialFailure(errors))
        } else {
            Ok(())
        }
    }

    /// Executes tasks in parallel with concurrency limit
    async fn execute_parallel_limited(&mut self, services: &HashMap<String, Box<dyn TaskService>>, limit: usize) -> Result<(), JobError> {
        use futures::stream::{FuturesUnordered, StreamExt};
        use std::pin::Pin;
        use std::future::Future;
        
        // Pre-validate all services exist
        for task in &self.tasks {
            if !services.contains_key(&task.service_name) {
                return Err(JobError::ServiceNotFound(task.service_name.clone()));
            }
        }
        
        let mut futures: FuturesUnordered<Pin<Box<dyn Future<Output = (String, Result<(), crate::core::platform::container::task::TaskError>)> + Send>>> = FuturesUnordered::new();
        let tasks_len = self.tasks.len();
        let mut task_iter = self.tasks.iter_mut().enumerate();
        let mut errors = Vec::new();

        // Start initial batch
        for _ in 0..limit.min(tasks_len) {
            if let Some((_, task)) = task_iter.next() {
                let service = services.get(&task.service_name).unwrap();
                let task_name = task.name().to_string();
                let future = async move {
                    let result = task.execute(service.as_ref()).await;
                    (task_name, result)
                };
                futures.push(Box::pin(future));
            }
        }

        // Process results and add new tasks
        while let Some((task_name, result)) = futures.next().await {
            if let Err(error) = result {
                errors.push(format!("Task '{}': {}", task_name, error));
            }

            // Add next task if available
            if let Some((_, task)) = task_iter.next() {
                let service = services.get(&task.service_name).unwrap();
                let task_name = task.name().to_string();
                let future = async move {
                    let result = task.execute(service.as_ref()).await;
                    (task_name, result)
                };
                futures.push(Box::pin(future));
            }
        }

        if !errors.is_empty() {
            Err(JobError::PartialFailure(errors))
        } else {
            Ok(())
        }
    }

    /// Executes a single task - static helper function
    async fn execute_single_task(task: &mut Task, services: &HashMap<String, Box<dyn TaskService>>) -> Result<(), JobError> {
        let service = services.get(&task.service_name)
            .ok_or_else(|| JobError::ServiceNotFound(task.service_name.clone()))?;

        task.execute(service.as_ref()).await
            .map_err(|e| JobError::TaskExecutionFailed(e.to_string()))
    }

    /// Handles rollback based on strategy
    async fn handle_rollback(&mut self) {
        match self.rollback_strategy {
            JobRollbackStrategy::None => {}
            JobRollbackStrategy::CancelRunning => {
                for task in &mut self.tasks {
                    if matches!(task.status(), ActionStatus::Running) {
                        task.cancel();
                    }
                }
            }
            JobRollbackStrategy::RollbackCompleted => {
                // Rollback completed tasks (implementation depends on task capabilities)
                for task in &mut self.tasks {
                    if matches!(task.status(), ActionStatus::Completed) {
                        task.reset(); // Reset for potential retry
                    }
                }
            }
            JobRollbackStrategy::Full => {
                for task in &mut self.tasks {
                    match task.status() {
                        ActionStatus::Running => task.cancel(),
                        ActionStatus::Completed => task.reset(),
                        _ => {}
                    }
                }
            }
        }
    }

    /// Creates job result summary
    fn create_job_result(&self) -> serde_json::Value {
        let completed_tasks = self.tasks.iter().filter(|t| matches!(t.status(), ActionStatus::Completed)).count();
        let failed_tasks = self.tasks.iter().filter(|t| matches!(t.status(), ActionStatus::Failed)).count();
        let total_tasks = self.tasks.len();

        serde_json::json!({
            "total_tasks": total_tasks,
            "completed_tasks": completed_tasks,
            "failed_tasks": failed_tasks,
            "success_rate": if total_tasks > 0 { (completed_tasks as f64 / total_tasks as f64) * 100.0 } else { 0.0 },
            "execution_mode": self.execution_mode
        })
    }

    /// Gets job statistics
    pub fn job_stats(&self) -> JobStats {
        let total_tasks = self.tasks.len();
        let completed_tasks = self.tasks.iter().filter(|t| matches!(t.status(), ActionStatus::Completed)).count();
        let failed_tasks = self.tasks.iter().filter(|t| matches!(t.status(), ActionStatus::Failed)).count();
        let running_tasks = self.tasks.iter().filter(|t| matches!(t.status(), ActionStatus::Running)).count();

        JobStats {
            total_tasks,
            completed_tasks,
            failed_tasks,
            running_tasks,
            success_rate: if total_tasks > 0 { (completed_tasks as f64 / total_tasks as f64) * 100.0 } else { 0.0 },
            execution_count: self.action.execution_count,
            last_execution: self.action.last_execution,
        }
    }

    /// Gets the job ID
    pub fn id(&self) -> Uuid {
        self.action.id
    }

    /// Gets the job name
    pub fn name(&self) -> &str {
        &self.action.name
    }

    /// Gets the job status
    pub fn status(&self) -> &ActionStatus {
        &self.action.status
    }

    /// Checks if job can be executed
    pub fn can_execute(&self) -> bool {
        self.action.can_execute()
    }

    /// Cancels the job
    pub fn cancel(&mut self) {
        self.action.cancel();
        for task in &mut self.tasks {
            task.cancel();
        }
    }

    /// Resets the job for re-execution
    pub fn reset(&mut self) {
        self.action.reset();
        for task in &mut self.tasks {
            task.reset();
        }
        self.task_results.clear();
    }
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.action.id == other.action.id
    }
}

impl Eq for Job {}

/// Job execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStats {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub running_tasks: usize,
    pub success_rate: f64,
    pub execution_count: u32,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

/// Job-specific errors
#[derive(Debug, Error)]
pub enum JobError {
    #[error("Job has no tasks to execute")]
    NoTasks,
    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    #[error("Partial failure - some tasks failed: {0:?}")]
    PartialFailure(Vec<String>),
    #[error("Circular dependency detected in task dependencies")]
    CircularDependency,
    #[error("Job failed but can be retried: {0}")]
    RetryableFailure(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Job is not in a valid state for execution: {0:?}")]
    InvalidState(ActionStatus),
    #[error("Action error: {0}")]
    ActionError(#[from] ActionError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::task::{DataBackupService, ContentIndexingService, EmailNotificationService};
   
    #[tokio::test]
    async fn test_job_creation() {
        let tasks = vec![
            Task::new("Task 1".to_string(), "First task".to_string(), "TestService".to_string()),
            Task::new("Task 2".to_string(), "Second task".to_string(), "TestService".to_string()),
        ];

        let job = Job::new(
            "Test Job".to_string(),
            "A test job with tasks".to_string(),
            tasks,
        );

        assert_eq!(job.name(), "Test Job");
        assert_eq!(job.tasks.len(), 2);
        assert_eq!(job.execution_mode, JobExecutionMode::Sequential);
        assert!(job.can_execute());
    }

    #[tokio::test]
    async fn test_job_sequential_execution() {
        let tasks = vec![
            Task::new("Backup Task".to_string(), "Backup data".to_string(), "DataBackupService".to_string()),
            Task::new("Index Task".to_string(), "Index content".to_string(), "ContentIndexingService".to_string()),
        ];

        let mut job = Job::new(
            "Sequential Job".to_string(),
            "A sequential job".to_string(),
            tasks,
        );

        let mut services: HashMap<String, Box<dyn TaskService>> = HashMap::new();
        services.insert("DataBackupService".to_string(), Box::new(DataBackupService { backup_path: "/tmp/backup".to_string() }));
        services.insert("ContentIndexingService".to_string(), Box::new(ContentIndexingService { index_name: "test_index".to_string() }));

        let result = job.execute(&services).await;
        assert!(result.is_ok());
        assert_eq!(job.status(), &ActionStatus::Completed);

        let stats = job.job_stats();
        assert_eq!(stats.total_tasks, 2);
        assert_eq!(stats.completed_tasks, 2);
        assert_eq!(stats.success_rate, 100.0);
    }

    #[tokio::test]
    async fn test_job_parallel_execution() {
        let tasks = vec![
            Task::new("Task 1".to_string(), "First parallel task".to_string(), "DataBackupService".to_string()),
            Task::new("Task 2".to_string(), "Second parallel task".to_string(), "ContentIndexingService".to_string()),
            Task::new("Task 3".to_string(), "Third parallel task".to_string(), "EmailNotificationService".to_string()),
        ];

        let mut job = Job::new_with_config(
            "Parallel Job".to_string(),
            "A parallel job".to_string(),
            tasks,
            JobExecutionMode::Parallel,
            ActionPriority::High,
        );

        // Add email arguments to the email task
        job.tasks[2].action.add_argument("to_email".to_string(), "test@example.com").unwrap();

        let mut services: HashMap<String, Box<dyn TaskService>> = HashMap::new();
        services.insert("DataBackupService".to_string(), Box::new(DataBackupService { backup_path: "/tmp/backup".to_string() }));
        services.insert("ContentIndexingService".to_string(), Box::new(ContentIndexingService { index_name: "test_index".to_string() }));
        services.insert("EmailNotificationService".to_string(), Box::new(EmailNotificationService { smtp_server: "smtp.test.com".to_string() }));

        let result = job.execute(&services).await;
        assert!(result.is_ok());
        assert_eq!(job.status(), &ActionStatus::Completed);
    }

    #[tokio::test]
    async fn test_job_with_dependencies() {
        let task1 = Task::new("Task 1".to_string(), "First task".to_string(), "DataBackupService".to_string());
        let task2 = Task::new("Task 2".to_string(), "Second task".to_string(), "ContentIndexingService".to_string());
        let task3 = Task::new("Task 3".to_string(), "Third task".to_string(), "EmailNotificationService".to_string());

        let task1_id = task1.id();
        let task2_id = task2.id();

        let mut job = Job::new_with_config(
            "Dependency Job".to_string(),
            "Job with task dependencies".to_string(),
            vec![task1, task2, task3],
            JobExecutionMode::Dependency,
            ActionPriority::Normal,
        );

        // Task 2 depends on Task 1, Task 3 depends on both Task 1 and Task 2
        job.add_task_dependency(task2_id, vec![task1_id]);
        job.add_task_dependency(job.tasks[2].id(), vec![task1_id, task2_id]);

        // Add email arguments
        job.tasks[2].action.add_argument("to_email".to_string(), "test@example.com").unwrap();

        let mut services: HashMap<String, Box<dyn TaskService>> = HashMap::new();
        services.insert("DataBackupService".to_string(), Box::new(DataBackupService { backup_path: "/tmp/backup".to_string() }));
        services.insert("ContentIndexingService".to_string(), Box::new(ContentIndexingService { index_name: "test_index".to_string() }));
        services.insert("EmailNotificationService".to_string(), Box::new(EmailNotificationService { smtp_server: "smtp.test.com".to_string() }));

        let result = job.execute(&services).await;
        assert!(result.is_ok());
        assert_eq!(job.status(), &ActionStatus::Completed);
    }

    #[tokio::test]
    async fn test_job_metadata() {
        let job = Job::new(
            "Metadata Job".to_string(),
            "Job with metadata".to_string(),
            vec![],
        );

        let mut job = job;
        job.add_metadata("workflow_id".to_string(), "workflow_123").unwrap();
        job.add_metadata("priority_level".to_string(), 5).unwrap();

        assert_eq!(job.job_metadata.get("workflow_id"), Some(&serde_json::json!("workflow_123")));
        assert_eq!(job.job_metadata.get("priority_level"), Some(&serde_json::json!(5)));
    }
}