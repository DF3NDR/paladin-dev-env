//! Shared mock adapters and constructors used by the documentation examples.
//!
//! These mirror the in-crate test doubles (e.g. `MockPaladinPort` in
//! `paladin-battalion`) so that book examples can be *real, compiled code*
//! without standing up live LLM providers or services.
#![allow(dead_code)]

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_ports::output::orchestrator_port::{
    EventDispatchResult, FireEventRequest, OrchestratorBridgeError, OrchestratorPort,
    QueueItemRequest, ScheduleJobRequest, SendNotificationRequest,
};
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_ports::output::paladin_port::{
    PaladinPort, PaladinResult, PaladinStreamChunk, StopReason,
};
use paladin_ports::output::scheduler_port::{
    JobId, JobInfo, JobSpec, JobStatus, SchedulerError, SchedulerPort,
};

/// A no-network `PaladinPort` that echoes its input — enough to drive the
/// orchestration examples without a real LLM.
pub struct MockPaladinPort;

#[async_trait]
impl PaladinPort for MockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        Ok(PaladinResult {
            output: format!("Processed: {} by {}", input, paladin.node.name),
            usage: paladin_ports::output::llm_port::TokenUsage::new(100, 0),
            execution_time_ms: 10,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<PaladinStreamChunk, PaladinError>>, PaladinError>
    {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Build a `MockPaladinPort` wrapped in `Arc<dyn PaladinPort>`, ready to hand to
/// any Battalion execution service.
pub fn mock_paladin_port() -> Arc<dyn PaladinPort> {
    Arc::new(MockPaladinPort)
}

/// Construct a test `Paladin` with the given display name.
pub fn create_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("You are {name}"),
        name: name.to_string(),
        user_name: "User".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(3),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

/// A `PaladinExecutorPort` that echoes input — for workflow→agent examples.
pub struct MockExecutor;

#[async_trait]
impl PaladinExecutorPort for MockExecutor {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        Ok(PaladinResult {
            output: format!("{} handled: {input}", paladin.node.name),
            usage: paladin_ports::output::llm_port::TokenUsage::new(42, 0),
            execution_time_ms: 5,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }
}

/// Build a `MockExecutor` as `Arc<dyn PaladinExecutorPort>`.
pub fn mock_executor() -> Arc<dyn PaladinExecutorPort> {
    Arc::new(MockExecutor)
}

/// A `SchedulerPort` that accepts any job and reports it as `Scheduled`.
pub struct MockScheduler;

#[async_trait]
impl SchedulerPort for MockScheduler {
    async fn start(&self) -> Result<(), SchedulerError> {
        Ok(())
    }
    async fn shutdown(&self) -> Result<(), SchedulerError> {
        Ok(())
    }
    async fn schedule_job(&self, _spec: JobSpec) -> Result<JobId, SchedulerError> {
        Ok(JobId::new())
    }
    async fn cancel_job(&self, _job_id: &JobId) -> Result<(), SchedulerError> {
        Ok(())
    }
    async fn get_job_status(&self, _job_id: &JobId) -> Result<JobStatus, SchedulerError> {
        Ok(JobStatus::Scheduled)
    }
    async fn get_job_info(&self, job_id: &JobId) -> Result<JobInfo, SchedulerError> {
        Ok(JobInfo {
            id: JobId::from_uuid(*job_id.as_uuid()),
            spec: JobSpec::new("mock", "0 0 9 * * *"),
            status: JobStatus::Scheduled,
            created_at: Utc::now(),
            last_run: None,
            next_run: None,
            run_count: 0,
            failure_count: 0,
        })
    }
    async fn list_jobs(&self) -> Result<Vec<JobInfo>, SchedulerError> {
        Ok(vec![])
    }
    fn is_running(&self) -> bool {
        true
    }
}

/// Build a `MockScheduler` as `Arc<dyn SchedulerPort>`.
pub fn mock_scheduler() -> Arc<dyn SchedulerPort> {
    Arc::new(MockScheduler)
}

/// An `OrchestratorPort` that accepts every bridge action — for bridge examples.
pub struct MockOrchestrator;

#[async_trait]
impl OrchestratorPort for MockOrchestrator {
    async fn schedule_job(
        &self,
        _request: ScheduleJobRequest,
    ) -> Result<Uuid, OrchestratorBridgeError> {
        Ok(Uuid::new_v4())
    }
    async fn queue_item(
        &self,
        _request: QueueItemRequest,
    ) -> Result<Uuid, OrchestratorBridgeError> {
        Ok(Uuid::new_v4())
    }
    async fn fire_event(
        &self,
        _request: FireEventRequest,
    ) -> Result<EventDispatchResult, OrchestratorBridgeError> {
        Ok(EventDispatchResult::default())
    }
    async fn send_notification(
        &self,
        _request: SendNotificationRequest,
    ) -> Result<Uuid, OrchestratorBridgeError> {
        Ok(Uuid::new_v4())
    }
}

/// Build a `MockOrchestrator` as `Arc<dyn OrchestratorPort>`.
pub fn mock_orchestrator() -> Arc<dyn OrchestratorPort> {
    Arc::new(MockOrchestrator)
}

// --- Content helpers ---------------------------------------------------------

use paladin_core::platform::container::content::{ContentItem, ContentType, TextContent};
use paladin_core::platform::container::prompt::{PromptItem, PromptRole, PromptType, TextPrompt};
use paladin_ports::output::content_delivery_port::{
    ContentDeliveryError, ContentDeliveryService, DeliveryMethod, DeliveryRequest,
    DeliveryResponse, DeliveryStats, DeliveryStatus,
};

/// Build a text `ContentItem` from a string (no file I/O).
pub fn text_content_item(text: &str) -> ContentItem {
    let text_content = TextContent::new(None, Some(text.to_string())).expect("valid text content");
    ContentItem::new(ContentType::Text(text_content)).expect("valid content item")
}

/// Build a text `PromptItem` from a string.
pub fn text_prompt_item(text: &str) -> PromptItem {
    let text_prompt = TextPrompt {
        content: text.to_string(),
        role: PromptRole::User,
    };
    PromptItem::new(PromptType::Text(text_prompt)).expect("valid prompt item")
}

/// A `ContentDeliveryService` that reports every delivery as `Delivered`.
pub struct MockDeliveryAdapter;

impl ContentDeliveryService for MockDeliveryAdapter {
    fn deliver_content(
        &self,
        _request: DeliveryRequest,
    ) -> Result<DeliveryResponse, ContentDeliveryError> {
        Ok(DeliveryResponse {
            delivery_id: Uuid::new_v4(),
            status: DeliveryStatus::Delivered,
            delivered_at: Some(Utc::now()),
            attempt_count: 1,
            error_message: None,
            metadata: None,
        })
    }
    fn schedule_delivery(
        &self,
        request: DeliveryRequest,
    ) -> Result<DeliveryResponse, ContentDeliveryError> {
        self.deliver_content(request)
    }
    fn cancel_delivery(&self, _delivery_id: Uuid) -> Result<(), ContentDeliveryError> {
        Ok(())
    }
    fn get_delivery_status(
        &self,
        _delivery_id: Uuid,
    ) -> Result<DeliveryResponse, ContentDeliveryError> {
        Err(ContentDeliveryError::DeliveryFailed(
            "not tracked".to_string(),
        ))
    }
    fn list_deliveries(
        &self,
        _recipient_id: &str,
        _limit: Option<u32>,
    ) -> Result<Vec<DeliveryResponse>, ContentDeliveryError> {
        Ok(vec![])
    }
    fn get_delivery_stats(
        &self,
        _recipient_id: Option<&str>,
    ) -> Result<DeliveryStats, ContentDeliveryError> {
        Ok(DeliveryStats {
            total_deliveries: 0,
            successful_deliveries: 0,
            failed_deliveries: 0,
            pending_deliveries: 0,
            average_delivery_time_ms: None,
        })
    }
    fn validate_delivery_method(
        &self,
        _method: &DeliveryMethod,
    ) -> Result<(), ContentDeliveryError> {
        Ok(())
    }
}

/// A `ContentListService` whose `aggregate_content` merges JSON values into an array.
pub struct MockListService;

impl paladin_content::services::content_aggregator_service::ContentListService for MockListService {
    fn fetch_content_list(
        &self,
        _url: &str,
    ) -> Result<paladin_core::platform::container::content_list::ContentList, String> {
        Err("not used in this example".to_string())
    }
    fn aggregate_content(&self, data: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::Value::Array(data)
    }
}
