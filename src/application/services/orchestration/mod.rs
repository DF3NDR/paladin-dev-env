/*
Orchestration Use Cases

Application-layer orchestration module. Relocated from
`core/platform/manager/orchestrator.rs`, `listener_service.rs`, and `scheduler.rs`.

This module exposes:
- `Orchestrator` — coordinates jobs, workflows, events, and content processing
- `listener` sub-module — `ListenerOrchestrator` (renamed from `ListenerService`)
- `scheduler` sub-module — `SchedulerOrchestrator` (renamed from `Scheduler`)
- `types` sub-module — coordination error/stats/trait types
- `OrchestrationContext` re-exported from paladin-core for callers
*/

pub mod listener;
pub mod orchestrator_bridge;
pub mod processors;
pub mod scheduler;
pub mod types;

pub use crate::core::platform::container::orchestration_context::OrchestrationContext;
pub use listener::{EventListener, ListenerConfig, ListenerService, ListenerStats};
pub use orchestrator_bridge::OrchestratorBridgeAdapter;
pub use scheduler::{Schedule, Scheduler, SchedulerStats};
pub use types::{
    ContentAnalysisType, ContentProcessingResult, ContentProcessor, DefaultContentProcessor,
    JobExecutionOutcome, ListenerError, OrchestratorError, OrchestratorStats, SchedulerError,
    WorkflowExecutionResult,
};

use crate::application::services::queue_orchestrator::QueueService;
use crate::core::base::component::action::{Action, ActionPriority};
use crate::core::base::component::event::Event;
use crate::core::base::entity::message::{Location, Message, MessagePriority};
use crate::core::platform::container::content::ContentItem;
use crate::core::platform::container::job::Job;
use crate::core::platform::container::queue_item::QueueItem;
use crate::core::platform::container::task::{Task, TaskError, TaskService};
use crate::core::platform::container::trigger::{Trigger, TriggerCondition};
use crate::core::platform::container::workflow::{
    Workflow, WorkflowExecutionOrder, WorkflowListener,
};
use chrono::Utc;
use listener::ListenerOrchestrator;
use paladin_ports::output::workflow_repository_port::{
    PersistedWorkflow, WorkflowPersistenceStatus, WorkflowRepositoryPort,
};
use scheduler::SchedulerOrchestrator;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinSet;
use uuid::Uuid;

/// Main Orchestrator service.
pub struct Orchestrator {
    scheduler: Arc<Mutex<SchedulerOrchestrator>>,
    queue_service: Arc<QueueService>,
    listener_service: Arc<ListenerOrchestrator>,
    task_services: Arc<RwLock<HashMap<String, Box<dyn TaskService>>>>,
    workflows: Arc<RwLock<HashMap<Uuid, Workflow>>>,
    workflow_results: Arc<RwLock<HashMap<Uuid, WorkflowExecutionResult>>>,
    active_sessions: Arc<RwLock<HashMap<Uuid, OrchestrationContext>>>,
    content_processors: Arc<RwLock<HashMap<String, Box<dyn ContentProcessor>>>>,
    workflow_repository: Option<Arc<dyn WorkflowRepositoryPort>>,
}

impl Orchestrator {
    /// Create a new Orchestrator.
    pub fn new() -> Self {
        Self {
            scheduler: Arc::new(Mutex::new(SchedulerOrchestrator::new())),
            queue_service: Arc::new(QueueService::new()),
            listener_service: Arc::new(ListenerOrchestrator::new()),
            task_services: Arc::new(RwLock::new(HashMap::new())),
            workflows: Arc::new(RwLock::new(HashMap::new())),
            workflow_results: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            content_processors: Arc::new(RwLock::new(HashMap::new())),
            workflow_repository: None,
        }
    }

    /// Attach a workflow repository so execution progress is persisted and
    /// incomplete workflows can be resumed after a restart.
    ///
    /// Persistence is entirely opt-in: an Orchestrator created with
    /// [`Orchestrator::new`] keeps all state in memory and behaves identically
    /// to before this method existed.
    pub fn with_workflow_repository(mut self, repository: Arc<dyn WorkflowRepositoryPort>) -> Self {
        self.workflow_repository = Some(repository);
        self
    }

    /// Start the orchestrator and all its services.
    pub async fn start(&self) -> Result<(), OrchestratorError> {
        // Initialize default services
        self.initialize_default_services().await?;

        // Recover any workflows interrupted by a previous shutdown/crash.
        self.resume_incomplete_workflows().await?;

        // Start scheduler in background
        let scheduler_clone = Arc::clone(&self.scheduler);
        tokio::spawn(async move {
            let mut scheduler = scheduler_clone.lock().await;
            scheduler.start().await;
        });

        log::info!("Orchestrator started successfully");
        Ok(())
    }

    /// Register a task service.
    pub async fn register_task_service(
        &self,
        service: Box<dyn TaskService>,
    ) -> Result<(), OrchestratorError> {
        let service_name = service.name().to_string();

        // Register with scheduler using the new clone_service method
        {
            let mut scheduler = self.scheduler.lock().await;
            scheduler.register_service(service.clone_service());
        }

        // Register with orchestrator
        {
            let mut services = self.task_services.write().await;
            services.insert(service_name.clone(), service);
        }

        log::info!("Registered task service: {}", service_name);
        Ok(())
    }

    /// Register an event listener.
    pub async fn register_event_listener(
        &self,
        listener: Box<dyn EventListener>,
    ) -> Result<(), OrchestratorError> {
        self.listener_service
            .register_listener(listener)
            .await
            .map_err(OrchestratorError::ListenerError)?;
        Ok(())
    }

    /// Register a content processor.
    pub async fn register_content_processor(
        &self,
        processor: Box<dyn ContentProcessor>,
    ) -> Result<(), OrchestratorError> {
        let processor_name = processor.name().to_string();
        let mut processors = self.content_processors.write().await;
        processors.insert(processor_name.clone(), processor);
        log::info!("Registered content processor: {}", processor_name);
        Ok(())
    }

    /// Create and execute a simple job immediately.
    pub async fn execute_job(
        &self,
        job: Job,
        context: OrchestrationContext,
    ) -> Result<Uuid, OrchestratorError> {
        let job_id = job.id();

        // Start orchestration session
        self.start_session(context.clone()).await?;

        // Execute job with registered services - collect cloned services
        let services: HashMap<String, Box<dyn TaskService>> = {
            let services_guard = self.task_services.read().await;
            services_guard
                .iter()
                .map(|(k, v)| (k.clone(), v.clone_service()))
                .collect()
        };

        let mut job_clone = job.clone();
        match job_clone.execute(&services).await {
            Ok(_) => {
                log::info!("Job '{}' executed successfully", job_clone.name());
                self.end_session(context.session_id).await?;
                Ok(job_id)
            }
            Err(e) => {
                log::info!("Job '{}' execution failed: {}", job_clone.name(), e);
                self.end_session(context.session_id).await?;
                Err(OrchestratorError::JobError(e))
            }
        }
    }

    /// Schedule a job for recurring execution.
    pub async fn schedule_job(
        &self,
        job: Job,
        schedule: Schedule,
        context: OrchestrationContext,
    ) -> Result<Uuid, OrchestratorError> {
        let job_id = job.id();

        // Start orchestration session
        self.start_session(context).await?;

        // Add to scheduler
        {
            let mut scheduler = self.scheduler.lock().await;
            scheduler
                .add_job(job, schedule)
                .map_err(OrchestratorError::SchedulerError)?;
        }

        log::info!("Job scheduled with ID: {}", job_id);
        Ok(job_id)
    }

    /// Ensure a named queue exists, creating it with the default configuration
    /// if it is not already present.
    ///
    /// This is idempotent: calling it for an existing queue leaves that queue
    /// (and its contents) untouched. It exists so that callers such as the
    /// agent → orchestrator bridge can enqueue items without first having to
    /// register the target queue out of band.
    pub async fn ensure_queue(&self, queue_name: &str) -> Result<(), OrchestratorError> {
        let existing = self.queue_service.get_all_stats().await;
        if !existing.contains_key(queue_name) {
            self.queue_service
                .create_queue(queue_name.to_string(), None)
                .await
                .map_err(OrchestratorError::QueueError)?;
        }
        Ok(())
    }

    /// Queue a job for asynchronous execution.
    pub async fn queue_job(
        &self,
        job: Job,
        queue_name: &str,
        context: OrchestrationContext,
    ) -> Result<Uuid, OrchestratorError> {
        // Convert job to queue item
        let message = Message::with_priority(
            Location::system("orchestrator"),
            Location::service("job_processor"),
            job.clone(),
            match job.action.priority {
                ActionPriority::Low => MessagePriority::Low,
                ActionPriority::Normal => MessagePriority::Normal,
                ActionPriority::High => MessagePriority::High,
                ActionPriority::Critical => MessagePriority::Critical,
            },
        );

        let queue_item = QueueItem::new(queue_name.to_string(), message, None);
        let item_id = self
            .queue_service
            .enqueue(queue_name, queue_item)
            .await
            .map_err(OrchestratorError::QueueError)?;

        // Start orchestration session
        self.start_session(context).await?;

        log::info!("Job queued with item ID: {}", item_id);
        Ok(item_id)
    }

    /// Process content through a content analysis workflow.
    pub async fn process_content(
        &self,
        content: ContentItem,
        processor_name: &str,
        context: OrchestrationContext,
    ) -> Result<ContentProcessingResult, OrchestratorError> {
        // Get content processor
        let processor = {
            let processors = self.content_processors.read().await;
            processors
                .get(processor_name)
                .ok_or_else(|| OrchestratorError::ProcessorNotFound(processor_name.to_string()))?
                .clone_box()?
        };

        // Start orchestration session
        self.start_session(context.clone()).await?;

        // Process content
        let result = processor.process_content(content, context.clone()).await?;

        // End session
        self.end_session(context.session_id).await?;

        Ok(result)
    }

    /// Create and register a complete workflow.
    pub async fn create_workflow(&self, mut workflow: Workflow) -> Result<Uuid, OrchestratorError> {
        let workflow_id = workflow.id;

        // Create queues for the workflow (skip if empty or if creation fails)
        for workflow_queue in &workflow.queues {
            match self
                .queue_service
                .create_queue(
                    workflow_queue.name.clone(),
                    Some(workflow_queue.config.clone()),
                )
                .await
            {
                Ok(_) => log::info!("Created queue: {}", workflow_queue.name),
                Err(e) => {
                    log::info!(
                        "Warning: Failed to create queue {}: {:?}",
                        workflow_queue.name,
                        e
                    );
                    // Continue with workflow creation even if queue creation fails
                }
            }
        }

        // Register listeners for the workflow
        for workflow_listener in &workflow.listeners {
            let listener = self
                .create_workflow_listener(workflow_listener.clone(), workflow_id)
                .await?;
            if let Err(e) = self.listener_service.register_listener(listener).await {
                log::info!(
                    "Warning: Failed to register listener {}: {:?}",
                    workflow_listener.name,
                    e
                );
                // Continue with workflow creation even if listener registration fails
            }
        }

        // Validate the configured execution order. Jobs are executed on demand
        // via `execute_workflow` (Sequential/Parallel/Custom) or via event
        // triggers (EventDriven); no scheduling side effects happen at creation.
        match &workflow.execution_order {
            WorkflowExecutionOrder::Sequential
            | WorkflowExecutionOrder::Parallel
            | WorkflowExecutionOrder::EventDriven => {}
            WorkflowExecutionOrder::Custom(stages) => {
                if stages.is_empty() {
                    return Err(OrchestratorError::ConfigurationError(
                        "Custom workflow execution order requires at least one stage".to_string(),
                    ));
                }
            }
        }

        // Store workflow
        workflow.updated_at = Utc::now();
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow_id, workflow);
        drop(workflows);

        Ok(workflow_id)
    }

    /// Execute a previously-created workflow end-to-end.
    ///
    /// Runs the workflow's jobs according to its
    /// [`WorkflowExecutionOrder`]:
    ///
    /// - `Sequential`: jobs run in order; output of job `N` is threaded into the
    ///   context consumed by job `N+1`. Execution stops at the first failure.
    /// - `Parallel`: all jobs run concurrently; every result is collected (no
    ///   sibling cancellation on failure).
    /// - `Custom(stages)`: stages run in order with a barrier between them; jobs
    ///   within a stage run concurrently. Execution stops after a failed stage.
    /// - `EventDriven`: jobs are driven by event triggers, so this call performs
    ///   no synchronous job execution and returns a completed (empty) result.
    ///
    /// Returns the aggregated [`WorkflowExecutionResult`], which is also retained
    /// internally and retrievable via [`Orchestrator::workflow_execution_result`].
    pub async fn execute_workflow(
        &self,
        workflow_id: Uuid,
    ) -> Result<WorkflowExecutionResult, OrchestratorError> {
        let workflow = {
            let workflows = self.workflows.read().await;
            workflows
                .get(&workflow_id)
                .cloned()
                .ok_or(OrchestratorError::WorkflowNotFound(workflow_id))?
        };

        self.execute_workflow_inner(workflow, Vec::new()).await
    }

    /// Execute (or resume) a workflow definition, skipping any jobs whose ids
    /// appear in `completed_job_ids`.
    ///
    /// When a workflow repository is attached, progress is persisted on start,
    /// after each completed job, and on terminal state so a crash can be
    /// recovered without re-running already-completed work.
    async fn execute_workflow_inner(
        &self,
        workflow: Workflow,
        completed_job_ids: Vec<Uuid>,
    ) -> Result<WorkflowExecutionResult, OrchestratorError> {
        let workflow_id = workflow.id;
        let mut completed: Vec<Uuid> = completed_job_ids;

        let mut result = WorkflowExecutionResult::new(workflow_id);
        result.start();
        self.store_workflow_result(result.clone()).await;
        self.persist_state(&workflow, WorkflowPersistenceStatus::Running, &completed)
            .await?;

        let services = self.snapshot_services().await;

        match &workflow.execution_order {
            WorkflowExecutionOrder::Sequential => {
                let mut context = workflow.context.clone();
                for job in &workflow.jobs {
                    // Skip jobs already completed in a previous (interrupted) run.
                    if completed.contains(&job.id()) {
                        continue;
                    }
                    let outcome = Self::run_single_job(job.clone(), &services).await;
                    let succeeded = outcome.succeeded();
                    // Thread this job's output into the context for the next job.
                    if let Some(output) = &outcome.output {
                        let _ = context
                            .add_metadata(format!("job_{}_output", outcome.job_id), output.clone());
                    }
                    result.record_outcome(outcome);
                    self.store_workflow_result(result.clone()).await;
                    if !succeeded {
                        result.mark_failed();
                        self.store_workflow_result(result.clone()).await;
                        self.persist_state(
                            &workflow,
                            WorkflowPersistenceStatus::Failed,
                            &completed,
                        )
                        .await?;
                        return Ok(result);
                    }
                    completed.push(job.id());
                    self.persist_state(&workflow, WorkflowPersistenceStatus::Running, &completed)
                        .await?;
                }
                result.mark_completed();
            }
            WorkflowExecutionOrder::Parallel => {
                let outcomes = Self::run_jobs_concurrently(&workflow.jobs, &services).await;
                Self::record_ordered(&mut result, outcomes, &workflow.jobs);
                if result.job_outcomes.iter().all(|o| o.succeeded()) {
                    result.mark_completed();
                } else {
                    result.mark_failed();
                }
            }
            WorkflowExecutionOrder::Custom(stages) => {
                let mut workflow_failed = false;
                for stage in stages {
                    let stage_jobs: Vec<Job> = stage
                        .job_ids
                        .iter()
                        .filter_map(|id| workflow.jobs.iter().find(|j| j.id() == *id).cloned())
                        .collect();

                    let outcomes = Self::run_jobs_concurrently(&stage_jobs, &services).await;
                    let stage_failed = outcomes.iter().any(|o| !o.succeeded());
                    Self::record_ordered(&mut result, outcomes, &stage_jobs);
                    self.store_workflow_result(result.clone()).await;

                    if stage_failed {
                        workflow_failed = true;
                        break;
                    }
                }
                if workflow_failed {
                    result.mark_failed();
                } else {
                    result.mark_completed();
                }
            }
            WorkflowExecutionOrder::EventDriven => {
                // Jobs are executed via event triggers, not synchronously here.
                result.mark_completed();
            }
        }

        self.store_workflow_result(result.clone()).await;
        let final_status = if result.completed() {
            WorkflowPersistenceStatus::Completed
        } else {
            WorkflowPersistenceStatus::Failed
        };
        self.persist_state(&workflow, final_status, &completed)
            .await?;
        Ok(result)
    }

    /// Persist the current workflow state when a repository is configured.
    ///
    /// This is a no-op when persistence is not enabled.
    async fn persist_state(
        &self,
        workflow: &Workflow,
        status: WorkflowPersistenceStatus,
        completed_job_ids: &[Uuid],
    ) -> Result<(), OrchestratorError> {
        if let Some(repository) = &self.workflow_repository {
            let record = PersistedWorkflow {
                workflow_id: workflow.id,
                status,
                completed_job_ids: completed_job_ids.to_vec(),
                definition: workflow.clone(),
                updated_at: Utc::now(),
            };
            repository
                .save(&record)
                .await
                .map_err(|e| OrchestratorError::PersistenceError(e.to_string()))?;
        }
        Ok(())
    }

    /// Resume every incomplete workflow found in the attached repository.
    ///
    /// Each recovered workflow is re-registered and execution continues from the
    /// last persisted completed job. Returns the ids of the workflows resumed.
    /// Without a repository this is a no-op returning an empty list.
    pub async fn resume_incomplete_workflows(&self) -> Result<Vec<Uuid>, OrchestratorError> {
        let Some(repository) = &self.workflow_repository else {
            return Ok(Vec::new());
        };

        let incomplete = repository
            .list_incomplete()
            .await
            .map_err(|e| OrchestratorError::PersistenceError(e.to_string()))?;

        let mut resumed = Vec::with_capacity(incomplete.len());
        for record in incomplete {
            {
                let mut workflows = self.workflows.write().await;
                workflows.insert(record.workflow_id, record.definition.clone());
            }
            self.execute_workflow_inner(record.definition, record.completed_job_ids)
                .await?;
            resumed.push(record.workflow_id);
        }

        Ok(resumed)
    }

    /// Retrieve the most recent execution result for a workflow, if any.
    pub async fn workflow_execution_result(
        &self,
        workflow_id: Uuid,
    ) -> Option<WorkflowExecutionResult> {
        let results = self.workflow_results.read().await;
        results.get(&workflow_id).cloned()
    }

    /// Snapshot the registered task services into an owned map.
    async fn snapshot_services(&self) -> HashMap<String, Box<dyn TaskService>> {
        let services_guard = self.task_services.read().await;
        services_guard
            .iter()
            .map(|(k, v)| (k.clone(), v.clone_service()))
            .collect()
    }

    /// Store or overwrite the execution result for a workflow.
    async fn store_workflow_result(&self, result: WorkflowExecutionResult) {
        let mut results = self.workflow_results.write().await;
        results.insert(result.workflow_id, result);
    }

    /// Execute a single job against the provided services and capture its outcome.
    async fn run_single_job(
        mut job: Job,
        services: &HashMap<String, Box<dyn TaskService>>,
    ) -> JobExecutionOutcome {
        let job_id = job.id();
        let job_name = job.name().to_string();
        match job.execute(services).await {
            Ok(_) => {
                let stats = job.job_stats();
                let output = serde_json::json!({
                    "total_tasks": stats.total_tasks,
                    "completed_tasks": stats.completed_tasks,
                    "failed_tasks": stats.failed_tasks,
                    "success_rate": stats.success_rate,
                });
                JobExecutionOutcome::success(job_id, job_name, Some(output))
            }
            Err(e) => JobExecutionOutcome::failure(job_id, job_name, e.to_string()),
        }
    }

    /// Run a set of jobs concurrently, returning their outcomes (unordered).
    async fn run_jobs_concurrently(
        jobs: &[Job],
        services: &HashMap<String, Box<dyn TaskService>>,
    ) -> Vec<JobExecutionOutcome> {
        let mut set: JoinSet<JobExecutionOutcome> = JoinSet::new();
        for job in jobs {
            let job = job.clone();
            let job_services = Self::clone_service_map(services);
            set.spawn(async move { Self::run_single_job(job, &job_services).await });
        }

        let mut outcomes = Vec::with_capacity(jobs.len());
        while let Some(joined) = set.join_next().await {
            match joined {
                Ok(outcome) => outcomes.push(outcome),
                Err(join_err) => outcomes.push(JobExecutionOutcome::failure(
                    Uuid::nil(),
                    "<join-error>".to_string(),
                    join_err.to_string(),
                )),
            }
        }
        outcomes
    }

    /// Record `outcomes` onto `result`, ordered to match the order of `jobs`.
    fn record_ordered(
        result: &mut WorkflowExecutionResult,
        mut outcomes: Vec<JobExecutionOutcome>,
        jobs: &[Job],
    ) {
        outcomes.sort_by_key(|o| {
            jobs.iter()
                .position(|j| j.id() == o.job_id)
                .unwrap_or(usize::MAX)
        });
        for outcome in outcomes {
            result.record_outcome(outcome);
        }
    }

    /// Deep-clone a service map so each concurrent job gets its own instances.
    fn clone_service_map(
        services: &HashMap<String, Box<dyn TaskService>>,
    ) -> HashMap<String, Box<dyn TaskService>> {
        services
            .iter()
            .map(|(k, v)| (k.clone(), v.clone_service()))
            .collect()
    }

    /// Process an event through the listener system.
    pub async fn process_event(&self, event: Event) -> Result<Vec<Uuid>, OrchestratorError> {
        let trigger_ids = self
            .listener_service
            .process_event(event)
            .await
            .map_err(OrchestratorError::ListenerError)?;

        // Process created triggers
        for trigger_id in &trigger_ids {
            if let Some(trigger) = self.listener_service.get_next_trigger().await {
                log::info!(
                    "Processing trigger {} for trigger ID {}",
                    trigger.name,
                    trigger_id
                );
                self.execute_trigger(trigger).await?;
            }
        }

        Ok(trigger_ids)
    }

    /// Execute a trigger (convert to job and execute).
    async fn execute_trigger(&self, trigger: Trigger) -> Result<(), OrchestratorError> {
        // Check if this is a workflow trigger with a specific target job
        let workflows = self.workflows.read().await;
        let mut target_job: Option<Job> = None;

        // Find the workflow and target job if specified
        for workflow in workflows.values() {
            for listener in &workflow.listeners {
                if listener.name == trigger.source
                    && let Some(target_job_id) = listener.target_job_id
                {
                    // Find the specific job in the workflow
                    if let Some(job) = workflow.jobs.iter().find(|j| j.id() == target_job_id) {
                        target_job = Some(job.clone());
                        break;
                    }
                }
            }
            if target_job.is_some() {
                break;
            }
        }

        // Use the target job if found, otherwise create a generic job from the trigger
        let job = if let Some(job) = target_job {
            job
        } else {
            // Create a job from the trigger's action (existing behavior)
            let task = Task::new(
                trigger.action.name.clone(),
                trigger.action.description.clone(),
                trigger.target.clone(),
            );

            Job::new(
                format!("Triggered Job: {}", trigger.name),
                format!("Job created from trigger: {}", trigger.description),
                vec![task],
            )
        };

        let context = OrchestrationContext::new(
            trigger.source.clone(),
            "production".to_string(), // Could be configurable
        );

        // Execute the job
        self.execute_job(job, context).await?;

        Ok(())
    }

    /// Create a content analysis workflow.
    pub async fn create_content_analysis_workflow(
        &self,
        content_items: Vec<ContentItem>,
        analysis_type: ContentAnalysisType,
        context: OrchestrationContext,
    ) -> Result<Uuid, OrchestratorError> {
        if content_items.is_empty() {
            return Err(OrchestratorError::ConfigurationError(
                "No content items provided".to_string(),
            ));
        }

        let workflow_id = Uuid::new_v4();

        // Create tasks for content analysis
        let mut jobs = Vec::new();

        for (i, content_item) in content_items.iter().enumerate() {
            let content_task = self
                .create_content_analysis_task(content_item.clone(), analysis_type.clone(), i)
                .await?;

            let job = Job::new(
                format!("Content Analysis Job {}", i + 1),
                format!("Analyze content item: {}", content_item.uuid()),
                vec![content_task],
            )
            .with_priority(ActionPriority::Normal);

            jobs.push(job);
        }

        // Create workflow without queues to avoid FileNotFound errors
        let workflow = Workflow {
            id: workflow_id,
            name: format!("Content Analysis Workflow - {}", analysis_type.name()),
            description: format!(
                "Analyze {} content items using {}",
                content_items.len(),
                analysis_type.name()
            ),
            jobs,
            listeners: vec![],
            queues: vec![], // Remove queue creation for now
            execution_order: WorkflowExecutionOrder::Parallel,
            context: context.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Store workflow directly instead of calling create_workflow
        // to avoid queue creation issues
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow_id, workflow);

        log::info!("Content analysis workflow created with ID: {}", workflow_id);

        Ok(workflow_id)
    }

    /// Start an orchestration session.
    async fn start_session(&self, context: OrchestrationContext) -> Result<(), OrchestratorError> {
        let mut sessions = self.active_sessions.write().await;
        sessions.insert(context.session_id, context.clone());
        log::info!("Started orchestration session: {}", context.session_id);
        Ok(())
    }

    /// End an orchestration session.
    async fn end_session(&self, session_id: Uuid) -> Result<(), OrchestratorError> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(context) = sessions.remove(&session_id) {
            let duration = Utc::now().timestamp() - context.started_at.timestamp();
            log::info!(
                "Ended orchestration session: {} (duration: {}s)",
                session_id,
                duration
            );
        }
        Ok(())
    }

    /// Get orchestrator statistics.
    pub async fn get_stats(&self) -> OrchestratorStats {
        let scheduler_stats = {
            let scheduler = self.scheduler.lock().await;
            scheduler.stats()
        };

        let queue_stats = self.queue_service.get_all_stats().await;
        let listener_stats = self.listener_service.get_all_stats().await;

        let workflows_count = {
            let workflows = self.workflows.read().await;
            workflows.len()
        };

        let active_sessions_count = {
            let sessions = self.active_sessions.read().await;
            sessions.len()
        };

        let services_count = {
            let services = self.task_services.read().await;
            services.len()
        };

        let processors_count = {
            let processors = self.content_processors.read().await;
            processors.len()
        };

        OrchestratorStats {
            active_sessions: active_sessions_count,
            total_workflows: workflows_count,
            total_services: services_count,
            total_processors: processors_count,
            scheduler_stats,
            queue_stats,
            listener_stats,
        }
    }

    /// Initialize default services.
    async fn initialize_default_services(&self) -> Result<(), OrchestratorError> {
        // Register default task services
        self.register_task_service(Box::new(
            crate::core::platform::container::task::DataBackupService::new(
                std::env::temp_dir()
                    .join("paladin_backups")
                    .to_string_lossy()
                    .to_string(),
            ),
        ))
        .await?;

        self.register_task_service(Box::new(
            crate::core::platform::container::task::ContentIndexingService::new("main_index"),
        ))
        .await?;

        self.register_task_service(Box::new(
            crate::core::platform::container::task::EmailNotificationService::new("localhost:587"),
        ))
        .await?;

        // Register default content processors
        self.register_content_processor(Box::new(DefaultContentProcessor))
            .await?;

        Ok(())
    }

    /// Create a workflow listener.
    async fn create_workflow_listener(
        &self,
        workflow_listener: WorkflowListener,
        workflow_id: Uuid,
    ) -> Result<Box<dyn EventListener>, OrchestratorError> {
        Ok(Box::new(WorkflowEventListener {
            name: workflow_listener.name,
            conditions: workflow_listener.conditions,
            target_job_id: workflow_listener.target_job_id,
            target_queue: workflow_listener.target_queue,
            workflow_id,
            config: ListenerConfig::default(),
        }))
    }

    /// Create a content analysis task.
    async fn create_content_analysis_task(
        &self,
        content_item: ContentItem,
        analysis_type: ContentAnalysisType,
        index: usize,
    ) -> Result<Task, OrchestratorError> {
        let mut task = Task::new(
            format!("Content Analysis Task {}", index + 1),
            format!("Analyze content using {}", analysis_type.name()),
            "ContentAnalysisService".to_string(),
        );

        // Add content item and analysis type as arguments
        task.action
            .add_argument("content_item".to_string(), content_item)
            .map_err(|e| OrchestratorError::TaskError(TaskError::ActionError(e)))?;

        task.action
            .add_argument("analysis_type".to_string(), analysis_type)
            .map_err(|e| OrchestratorError::TaskError(TaskError::ActionError(e)))?;

        Ok(task)
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Workflow event listener implementation.
struct WorkflowEventListener {
    name: String,
    conditions: Vec<TriggerCondition>,
    target_job_id: Option<Uuid>,
    target_queue: Option<String>,
    workflow_id: Uuid,
    config: ListenerConfig,
}

use async_trait::async_trait;

#[async_trait]
impl EventListener for WorkflowEventListener {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Workflow event listener"
    }

    fn conditions(&self) -> &[TriggerCondition] {
        &self.conditions
    }

    async fn should_process(&self, event: &Event) -> bool {
        // Simple pattern matching for demo
        self.conditions.iter().any(|condition| {
            condition.event_type_pattern == "*"
                || condition.event_type_pattern == event.event_type
                || event
                    .event_type
                    .contains(&condition.event_type_pattern.replace('*', ""))
        })
    }

    async fn create_trigger(&self, event: Event) -> Result<Trigger, ListenerError> {
        let mut action = Action::new(
            format!("Workflow Action: {}", self.name),
            format!(
                "Action triggered by workflow listener for event: {}",
                event.event_type
            ),
            self.name.clone(),
            self.target_queue
                .clone()
                .unwrap_or_else(|| "default_queue".to_string()),
        );

        // Add target job ID to action metadata if specified
        if let Some(job_id) = self.target_job_id {
            action
                .add_argument("target_job_id".to_string(), job_id)
                .map_err(|e| ListenerError::TriggerCreationFailed(e.to_string()))?;
        }

        let condition = self
            .conditions
            .first()
            .ok_or_else(|| {
                ListenerError::InvalidConfiguration("No conditions defined".to_string())
            })?
            .clone();

        Ok(Trigger::new(
            format!("Workflow Trigger: {}", self.name),
            format!(
                "Trigger for workflow {} from event {}",
                self.workflow_id, event.event_type
            ),
            self.name.clone(),
            self.target_queue
                .clone()
                .unwrap_or_else(|| "default_service".to_string()),
            event,
            action,
            condition,
        ))
    }

    fn config(&self) -> &ListenerConfig {
        &self.config
    }

    fn update_config(&mut self, config: ListenerConfig) {
        self.config = config;
    }

    async fn health_check(&self) -> Result<bool, ListenerError> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::queue_orchestrator::QueueError;
    use crate::core::platform::container::content::{ContentType, TextContent};

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let orchestrator = Orchestrator::new();

        let stats = orchestrator.get_stats().await;
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_workflows, 0);
    }

    #[tokio::test]
    async fn test_orchestrator_default() {
        let orchestrator = Orchestrator::default();
        let stats = orchestrator.get_stats().await;
        assert_eq!(stats.active_sessions, 0);
    }

    #[tokio::test]
    async fn test_orchestration_context_creation() {
        let context = OrchestrationContext::new("test_user".to_string(), "dev".to_string());

        assert_eq!(context.initiator, "test_user");
        assert_eq!(context.environment, "dev");
        assert!(context.correlation_id.is_none());
        assert!(context.metadata.is_empty());
        assert!(context.started_at <= Utc::now());
    }

    #[tokio::test]
    async fn test_orchestration_context_with_correlation() {
        let correlation_id = Uuid::new_v4();
        let context = OrchestrationContext::new("test_user".to_string(), "staging".to_string())
            .with_correlation(correlation_id);

        assert_eq!(context.correlation_id, Some(correlation_id));
    }

    #[tokio::test]
    async fn test_orchestration_context_add_metadata() {
        let mut context = OrchestrationContext::new("test_user".to_string(), "prod".to_string());

        let result = context.add_metadata("key1".to_string(), "value1");
        assert!(result.is_ok());

        assert_eq!(context.metadata.len(), 1);
        assert!(context.metadata.contains_key("key1"));
    }

    #[tokio::test]
    async fn test_orchestration_context_add_complex_metadata() {
        let mut context = OrchestrationContext::new("test_user".to_string(), "test".to_string());

        let complex_data = serde_json::json!({
            "nested": {
                "field": 123,
                "array": [1, 2, 3]
            }
        });

        let result = context.add_metadata("complex".to_string(), complex_data);
        assert!(result.is_ok());
        assert!(context.metadata.contains_key("complex"));
    }

    #[tokio::test]
    async fn test_error_variants_display() {
        let scheduler_err =
            OrchestratorError::SchedulerError(SchedulerError::JobNotFound(Uuid::new_v4()));
        assert!(scheduler_err.to_string().contains("Scheduler error"));

        let queue_err =
            OrchestratorError::QueueError(QueueError::QueueNotFound("test_queue".to_string()));
        assert!(queue_err.to_string().contains("Queue error"));

        let listener_err = OrchestratorError::ListenerError(ListenerError::ListenerNotFound(
            "test_listener".to_string(),
        ));
        assert!(listener_err.to_string().contains("Listener error"));

        let processor_err = OrchestratorError::ProcessorNotFound("test_processor".to_string());
        assert_eq!(
            processor_err.to_string(),
            "Processor not found: test_processor"
        );

        let workflow_err = OrchestratorError::WorkflowNotFound(Uuid::new_v4());
        assert!(workflow_err.to_string().contains("Workflow not found"));

        let session_err = OrchestratorError::SessionNotFound(Uuid::new_v4());
        assert!(session_err.to_string().contains("Session not found"));

        let serialization_err = OrchestratorError::SerializationError("invalid json".to_string());
        assert_eq!(
            serialization_err.to_string(),
            "Serialization error: invalid json"
        );

        let config_err = OrchestratorError::ConfigurationError("bad config".to_string());
        assert_eq!(config_err.to_string(), "Configuration error: bad config");

        let service_err = OrchestratorError::ServiceError("service down".to_string());
        assert_eq!(service_err.to_string(), "Service error: service down");
    }

    #[tokio::test]
    async fn test_orchestrator_service_registration() {
        let orchestrator = Orchestrator::new();

        let service = crate::core::platform::container::task::DataBackupService {
            backup_path: "/test/backup".to_string(),
        };

        let result = orchestrator.register_task_service(Box::new(service)).await;
        assert!(result.is_ok());

        let stats = orchestrator.get_stats().await;
        assert_eq!(stats.total_services, 1);
    }

    #[tokio::test]
    async fn test_start_and_end_session() {
        let orchestrator = Orchestrator::new();

        let context = OrchestrationContext::new("test_user".to_string(), "test".to_string());
        let session_id = context.session_id;

        // Start session
        let start_result = orchestrator.start_session(context).await;
        assert!(start_result.is_ok());

        let stats = orchestrator.get_stats().await;
        assert_eq!(stats.active_sessions, 1);

        // End session
        let end_result = orchestrator.end_session(session_id).await;
        assert!(end_result.is_ok());

        let stats = orchestrator.get_stats().await;
        assert_eq!(stats.active_sessions, 0);
    }

    #[tokio::test]
    async fn test_end_nonexistent_session() {
        let orchestrator = Orchestrator::new();

        let fake_session_id = Uuid::new_v4();
        let result = orchestrator.end_session(fake_session_id).await;

        // end_session returns Ok even if session doesn't exist (idempotent)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_content_processing() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        // Create test content
        let text_content = TextContent::new(None, Some("Test content".to_string())).unwrap();
        let content_item = ContentItem::new(ContentType::Text(text_content)).unwrap();

        let context = OrchestrationContext::new("test_user".to_string(), "test".to_string());

        let result = orchestrator
            .process_content(content_item, "DefaultContentProcessor", context)
            .await;

        assert!(result.is_ok());
        let processing_result = result.unwrap();
        assert!(processing_result.success);
        assert!(processing_result.result_data.is_some());
        assert!(processing_result.error.is_none());
        assert_eq!(processing_result.processor_name, "DefaultContentProcessor");
    }

    #[tokio::test]
    async fn test_content_processing_with_nonexistent_processor() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        let text_content = TextContent::new(None, Some("Test".to_string())).unwrap();
        let content_item = ContentItem::new(ContentType::Text(text_content)).unwrap();
        let context = OrchestrationContext::new("test_user".to_string(), "test".to_string());

        let result = orchestrator
            .process_content(content_item, "NonExistentProcessor", context)
            .await;

        assert!(result.is_err());
        match result {
            Err(OrchestratorError::ProcessorNotFound(name)) => {
                assert_eq!(name, "NonExistentProcessor");
            }
            _ => panic!("Expected ProcessorNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_job_execution() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        // Create a simple job
        let task = Task::new(
            "Test Task".to_string(),
            "A test task".to_string(),
            "DataBackupService".to_string(),
        );

        let job = Job::new("Test Job".to_string(), "A test job".to_string(), vec![task]);

        let context = OrchestrationContext::new("test_user".to_string(), "test".to_string());

        let result = orchestrator.execute_job(job, context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_schedule_job() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        let task = Task::new(
            "Scheduled Task".to_string(),
            "A scheduled task".to_string(),
            "DataBackupService".to_string(),
        );

        let job = Job::new(
            "Scheduled Job".to_string(),
            "A scheduled job".to_string(),
            vec![task],
        );

        // Use Interval schedule instead of cron
        let schedule = Schedule::Interval(std::time::Duration::from_secs(3600));
        let context = OrchestrationContext::new("test_user".to_string(), "test".to_string());

        let result = orchestrator.schedule_job(job, schedule, context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_queue_job() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        let task = Task::new(
            "Queued Task".to_string(),
            "A queued task".to_string(),
            "DataBackupService".to_string(),
        );

        let job = Job::new(
            "Queued Job".to_string(),
            "A queued job".to_string(),
            vec![task],
        );

        let context = OrchestrationContext::new("test_user".to_string(), "test".to_string());

        // Create the queue first
        let _ = orchestrator
            .queue_service
            .create_queue("test_queue".to_string(), None)
            .await;

        let result = orchestrator.queue_job(job, "test_queue", context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_content_analysis_workflow() {
        let orchestrator = Arc::new(Orchestrator::new());
        orchestrator.start().await.unwrap();

        // Create test content without URL to avoid file system access
        let text_content = TextContent::new(
            None, // Remove URL to avoid file system operations
            Some("This is test content for analysis".to_string()),
        )
        .unwrap();
        let content_item = ContentItem::new(ContentType::Text(text_content)).unwrap();

        let context = OrchestrationContext::new("test_service".to_string(), "test".to_string());

        let result = orchestrator
            .create_content_analysis_workflow(
                vec![content_item],
                ContentAnalysisType::LanguageDetection,
                context,
            )
            .await;

        // Better error handling with detailed error information
        match result {
            Ok(workflow_id) => {
                assert!(workflow_id != Uuid::nil());

                // Verify workflow was created
                let workflows = orchestrator.workflows.read().await;
                assert!(workflows.contains_key(&workflow_id));

                let workflow = workflows.get(&workflow_id).unwrap();
                assert!(!workflow.jobs.is_empty());
                assert!(workflow.name.contains("Content Analysis Workflow"));
            }
            Err(e) => {
                panic!("Workflow creation failed with error: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_event_processing() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        let event = Event::new(
            "test_event".to_string(),
            serde_json::json!({"test": "data"}),
            "test_source".to_string(),
        );

        let result = orchestrator.process_event(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_sessions() {
        let orchestrator = Orchestrator::new();

        // Start multiple sessions
        let context1 = OrchestrationContext::new("user1".to_string(), "dev".to_string());
        let context2 = OrchestrationContext::new("user2".to_string(), "prod".to_string());
        let context3 = OrchestrationContext::new("user3".to_string(), "staging".to_string());

        orchestrator.start_session(context1.clone()).await.unwrap();
        orchestrator.start_session(context2.clone()).await.unwrap();
        orchestrator.start_session(context3.clone()).await.unwrap();

        let stats = orchestrator.get_stats().await;
        assert_eq!(stats.active_sessions, 3);

        // End one session
        orchestrator.end_session(context1.session_id).await.unwrap();

        let stats = orchestrator.get_stats().await;
        assert_eq!(stats.active_sessions, 2);
    }

    #[tokio::test]
    async fn test_workflow_creation() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        // Create workflow manually since there's no Workflow::new
        let context = OrchestrationContext::new("test_user".to_string(), "test".to_string());
        let workflow = Workflow {
            id: Uuid::new_v4(),
            name: "Test Workflow".to_string(),
            description: "A test workflow".to_string(),
            jobs: Vec::new(),
            listeners: Vec::new(),
            queues: Vec::new(),
            execution_order: WorkflowExecutionOrder::Sequential,
            context,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = orchestrator.create_workflow(workflow).await;
        assert!(result.is_ok());

        let stats = orchestrator.get_stats().await;
        assert_eq!(stats.total_workflows, 1);
    }

    #[tokio::test]
    async fn test_default_content_processor() {
        let processor = DefaultContentProcessor;
        assert_eq!(processor.name(), "DefaultContentProcessor");

        let text_content = TextContent::new(None, Some("Test content".to_string())).unwrap();
        let content_item = ContentItem::new(ContentType::Text(text_content)).unwrap();
        let context = OrchestrationContext::new("test_user".to_string(), "test".to_string());

        let result = processor.process_content(content_item, context).await;
        assert!(result.is_ok());

        let processing_result = result.unwrap();
        assert!(processing_result.success);
        assert_eq!(processing_result.processor_name, "DefaultContentProcessor");
    }

    #[tokio::test]
    async fn test_content_processor_clone() {
        let processor = DefaultContentProcessor;
        let cloned = processor.clone_box();
        assert!(cloned.is_ok());

        let cloned_processor = cloned.unwrap();
        assert_eq!(cloned_processor.name(), "DefaultContentProcessor");
    }

    #[tokio::test]
    async fn test_orchestrator_stats_structure() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap(); // Start to register default processor and services

        let stats = orchestrator.get_stats().await;

        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_workflows, 0);
        assert_eq!(stats.total_services, 3); // DataBackupService, ContentIndexingService, EmailNotificationService
        assert_eq!(stats.total_processors, 1); // DefaultContentProcessor registered on start
        assert!(stats.queue_stats.is_empty());
        assert!(stats.listener_stats.is_empty());
    }

    #[tokio::test]
    async fn test_content_processing_result_structure() {
        let result = ContentProcessingResult {
            content_id: Uuid::new_v4(),
            processor_name: "TestProcessor".to_string(),
            processing_time_ms: 150,
            success: true,
            result_data: Some(serde_json::json!({"test": "data"})),
            error: None,
            metadata: HashMap::new(),
        };

        assert_eq!(result.processor_name, "TestProcessor");
        assert_eq!(result.processing_time_ms, 150);
        assert!(result.success);
        assert!(result.result_data.is_some());
        assert!(result.error.is_none());
        assert!(result.metadata.is_empty());
    }

    // --- Workflow execution loop tests (Milestone 9, Epic 1, Task 1.0) ---

    /// Build a single-task job bound to `service_name`.
    fn job_for_service(job_name: &str, service_name: &str) -> Job {
        let task = Task::new(
            format!("{job_name} task"),
            "workflow execution test task".to_string(),
            service_name.to_string(),
        );
        Job::new(
            job_name.to_string(),
            "workflow test job".to_string(),
            vec![task],
        )
    }

    /// Build a workflow from jobs and an execution order.
    fn workflow_with(jobs: Vec<Job>, order: WorkflowExecutionOrder) -> Workflow {
        let context = OrchestrationContext::new("test_user".to_string(), "test".to_string());
        Workflow {
            id: Uuid::new_v4(),
            name: "Execution Test Workflow".to_string(),
            description: "Workflow execution loop test".to_string(),
            jobs,
            listeners: Vec::new(),
            queues: Vec::new(),
            execution_order: order,
            context,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_execute_workflow_not_found() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        let missing = Uuid::new_v4();
        let result = orchestrator.execute_workflow(missing).await;

        match result {
            Err(OrchestratorError::WorkflowNotFound(id)) => assert_eq!(id, missing),
            other => panic!("expected WorkflowNotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_execute_sequential_workflow_orders_and_threads_output() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        let jobs = vec![
            job_for_service("job-1", "DataBackupService"),
            job_for_service("job-2", "DataBackupService"),
            job_for_service("job-3", "DataBackupService"),
        ];
        let expected_order: Vec<Uuid> = jobs.iter().map(|j| j.id()).collect();
        let workflow = workflow_with(jobs, WorkflowExecutionOrder::Sequential);
        let workflow_id = orchestrator.create_workflow(workflow).await.unwrap();

        let result = orchestrator.execute_workflow(workflow_id).await.unwrap();

        assert!(result.completed());
        assert!(!result.failed());
        let actual_order: Vec<Uuid> = result.job_outcomes.iter().map(|o| o.job_id).collect();
        assert_eq!(
            actual_order, expected_order,
            "jobs must run in workflow order"
        );
        for outcome in &result.job_outcomes {
            assert!(outcome.succeeded());
            assert!(outcome.output.is_some(), "output must be threaded forward");
        }

        // Result is retained and retrievable.
        let stored = orchestrator
            .workflow_execution_result(workflow_id)
            .await
            .unwrap();
        assert!(stored.completed());
    }

    #[tokio::test]
    async fn test_execute_sequential_workflow_fail_fast_stops() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        let jobs = vec![
            job_for_service("ok-1", "DataBackupService"),
            job_for_service("boom", "UnregisteredService"),
            job_for_service("never", "DataBackupService"),
        ];
        let workflow = workflow_with(jobs, WorkflowExecutionOrder::Sequential);
        let workflow_id = orchestrator.create_workflow(workflow).await.unwrap();

        let result = orchestrator.execute_workflow(workflow_id).await.unwrap();

        assert!(result.failed());
        assert!(!result.completed());
        // Stops at the failing job: third job must not have run.
        assert_eq!(result.job_outcomes.len(), 2);
        assert!(result.job_outcomes[0].succeeded());
        assert!(!result.job_outcomes[1].succeeded());
        assert!(result.job_outcomes[1].error.is_some());
    }

    #[tokio::test]
    async fn test_execute_parallel_workflow_aggregates_all() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        let jobs = vec![
            job_for_service("ok", "DataBackupService"),
            job_for_service("fail", "UnregisteredService"),
        ];
        let expected_order: Vec<Uuid> = jobs.iter().map(|j| j.id()).collect();
        let workflow = workflow_with(jobs, WorkflowExecutionOrder::Parallel);
        let workflow_id = orchestrator.create_workflow(workflow).await.unwrap();

        let result = orchestrator.execute_workflow(workflow_id).await.unwrap();

        // Every job result is collected even though one failed.
        assert_eq!(result.job_outcomes.len(), 2);
        assert!(result.failed());
        assert!(!result.completed());
        let actual_order: Vec<Uuid> = result.job_outcomes.iter().map(|o| o.job_id).collect();
        assert_eq!(
            actual_order, expected_order,
            "parallel results must be deterministically ordered"
        );
        assert!(result.job_outcomes[0].succeeded());
        assert!(!result.job_outcomes[1].succeeded());
    }

    #[tokio::test]
    async fn test_execute_staged_workflow_orders_stages() {
        use crate::core::platform::container::job::JobExecutionMode;
        use crate::core::platform::container::workflow::WorkflowStage;

        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        let job_a = job_for_service("stage1-a", "DataBackupService");
        let job_b = job_for_service("stage1-b", "ContentIndexingService");
        let job_c = job_for_service("stage2-c", "DataBackupService");
        let (id_a, id_b, id_c) = (job_a.id(), job_b.id(), job_c.id());

        let stages = vec![
            WorkflowStage {
                name: "stage-1".to_string(),
                job_ids: vec![id_a, id_b],
                dependencies: Vec::new(),
                execution_mode: JobExecutionMode::Parallel,
            },
            WorkflowStage {
                name: "stage-2".to_string(),
                job_ids: vec![id_c],
                dependencies: vec!["stage-1".to_string()],
                execution_mode: JobExecutionMode::Sequential,
            },
        ];

        let workflow = workflow_with(
            vec![job_a, job_b, job_c],
            WorkflowExecutionOrder::Custom(stages),
        );
        let workflow_id = orchestrator.create_workflow(workflow).await.unwrap();

        let result = orchestrator.execute_workflow(workflow_id).await.unwrap();

        assert!(result.completed());
        assert_eq!(result.job_outcomes.len(), 3);
        // Stage 2's job must come after both stage 1 jobs (barrier).
        let positions: HashMap<Uuid, usize> = result
            .job_outcomes
            .iter()
            .enumerate()
            .map(|(i, o)| (o.job_id, i))
            .collect();
        assert!(positions[&id_c] > positions[&id_a]);
        assert!(positions[&id_c] > positions[&id_b]);
    }

    #[tokio::test]
    async fn test_unregistered_service_surfaces_typed_error() {
        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        let jobs = vec![job_for_service("lonely", "NoSuchService")];
        let workflow = workflow_with(jobs, WorkflowExecutionOrder::Sequential);
        let workflow_id = orchestrator.create_workflow(workflow).await.unwrap();

        let result = orchestrator.execute_workflow(workflow_id).await.unwrap();

        assert!(result.failed());
        assert_eq!(result.job_outcomes.len(), 1);
        let outcome = &result.job_outcomes[0];
        assert!(!outcome.succeeded());
        let error = outcome
            .error
            .as_ref()
            .expect("failed job must record an error");
        // The typed JobError::ServiceNotFound message names the missing service,
        // proving service resolution failed without panicking/unwrapping.
        assert!(
            error.contains("NoSuchService"),
            "error should name the unregistered service, got: {error}"
        );
    }

    #[tokio::test]
    async fn test_continue_on_error_job_runs_all_tasks() {
        use crate::core::platform::container::job::JobExecutionMode;

        let orchestrator = Orchestrator::new();
        orchestrator.start().await.unwrap();

        // A single job whose first task targets a missing service and whose
        // second task targets a registered one. With continue-on-error the job
        // attempts both tasks before reporting the partial failure.
        let bad_task = Task::new(
            "bad task".to_string(),
            "targets a missing service".to_string(),
            "MissingService".to_string(),
        );
        let good_task = Task::new(
            "good task".to_string(),
            "targets a registered service".to_string(),
            "DataBackupService".to_string(),
        );
        let job = Job::new(
            "resilient-job".to_string(),
            "continue-on-error job".to_string(),
            vec![bad_task, good_task],
        )
        .with_execution_mode(JobExecutionMode::SequentialContinueOnError);

        let workflow = workflow_with(vec![job], WorkflowExecutionOrder::Sequential);
        let workflow_id = orchestrator.create_workflow(workflow).await.unwrap();

        let result = orchestrator.execute_workflow(workflow_id).await.unwrap();

        // The job partially failed (one task could not resolve its service), so
        // the workflow is marked failed, but the registered task still ran.
        assert!(result.failed());
        assert_eq!(result.job_outcomes.len(), 1);
        let outcome = &result.job_outcomes[0];
        assert!(!outcome.succeeded());
        // Failure outcomes carry the error rather than output stats.
        assert!(outcome.output.is_none());
        let error = outcome.error.as_ref().expect("partial failure error");
        assert!(error.contains("MissingService"));
    }

    // --- Workflow state persistence tests (Milestone 9, Epic 1, Task 4.0) ---

    /// In-memory fake [`WorkflowRepositoryPort`] for exercising persistence and
    /// crash-recovery without a real datastore.
    #[derive(Default)]
    struct FakeWorkflowRepository {
        records: std::sync::Mutex<HashMap<Uuid, PersistedWorkflow>>,
    }

    #[async_trait]
    impl WorkflowRepositoryPort for FakeWorkflowRepository {
        async fn save(
            &self,
            record: &PersistedWorkflow,
        ) -> Result<(), paladin_ports::output::workflow_repository_port::WorkflowRepositoryError>
        {
            self.records
                .lock()
                .unwrap()
                .insert(record.workflow_id, record.clone());
            Ok(())
        }

        async fn load(
            &self,
            workflow_id: Uuid,
        ) -> Result<
            Option<PersistedWorkflow>,
            paladin_ports::output::workflow_repository_port::WorkflowRepositoryError,
        > {
            Ok(self.records.lock().unwrap().get(&workflow_id).cloned())
        }

        async fn list_incomplete(
            &self,
        ) -> Result<
            Vec<PersistedWorkflow>,
            paladin_ports::output::workflow_repository_port::WorkflowRepositoryError,
        > {
            Ok(self
                .records
                .lock()
                .unwrap()
                .values()
                .filter(|r| !r.status.is_terminal())
                .cloned()
                .collect())
        }
    }

    #[tokio::test]
    async fn test_execute_workflow_persists_completed_state() {
        let repo = Arc::new(FakeWorkflowRepository::default());
        let orchestrator = Orchestrator::new().with_workflow_repository(repo.clone());
        orchestrator.start().await.unwrap();

        let jobs = vec![
            job_for_service("p-1", "DataBackupService"),
            job_for_service("p-2", "DataBackupService"),
        ];
        let workflow = workflow_with(jobs, WorkflowExecutionOrder::Sequential);
        let workflow_id = orchestrator.create_workflow(workflow).await.unwrap();

        let result = orchestrator.execute_workflow(workflow_id).await.unwrap();
        assert!(result.completed());

        let persisted = repo.load(workflow_id).await.unwrap().unwrap();
        assert_eq!(persisted.status, WorkflowPersistenceStatus::Completed);
        assert_eq!(persisted.completed_job_ids.len(), 2);
    }

    #[tokio::test]
    async fn test_crash_recovery_resumes_remaining_jobs_to_completion() {
        let repo = Arc::new(FakeWorkflowRepository::default());

        // Build a three-job sequential workflow and simulate a crash after the
        // first two jobs already completed in a previous run.
        let job1 = job_for_service("recover-1", "DataBackupService");
        let job2 = job_for_service("recover-2", "DataBackupService");
        let job3 = job_for_service("recover-3", "DataBackupService");
        let (id1, id2, id3) = (job1.id(), job2.id(), job3.id());
        let workflow = workflow_with(vec![job1, job2, job3], WorkflowExecutionOrder::Sequential);
        let workflow_id = workflow.id;

        repo.save(&PersistedWorkflow {
            workflow_id,
            status: WorkflowPersistenceStatus::Running,
            completed_job_ids: vec![id1, id2],
            definition: workflow,
            updated_at: Utc::now(),
        })
        .await
        .unwrap();

        // A fresh orchestrator recovers on start(): default services are
        // registered first, then incomplete workflows resume automatically.
        let orchestrator = Orchestrator::new().with_workflow_repository(repo.clone());
        orchestrator.start().await.unwrap();

        let result = orchestrator
            .workflow_execution_result(workflow_id)
            .await
            .expect("recovered workflow must have a result");
        assert!(result.completed(), "resumed workflow should complete");
        // Only the outstanding third job runs; the first two are skipped.
        assert_eq!(result.job_outcomes.len(), 1);
        assert_eq!(result.job_outcomes[0].job_id, id3);

        let persisted = repo.load(workflow_id).await.unwrap().unwrap();
        assert_eq!(persisted.status, WorkflowPersistenceStatus::Completed);
        assert_eq!(persisted.completed_job_ids.len(), 3);
    }
}
