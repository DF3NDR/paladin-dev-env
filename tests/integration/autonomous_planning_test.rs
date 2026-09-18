//! Integration tests for autonomous planning functionality
//!
//! These tests verify the complete planning workflow:
//! 1. Plan creation via LLM
//! 2. Subtask execution with dependency tracking
//! 3. Result synthesis into cohesive response
//!
//! Uses mock LLM adapter for deterministic testing.

use async_trait::async_trait;
use chrono::Utc;
use paladin::application::services::paladin::planning_service::PlanningService;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, TokenUsage,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Mock LLM adapter that returns predefined responses based on call count
///
/// This allows simulating the multi-step planning process:
/// - Call 1: Return task decomposition plan (JSON)
/// - Call 2+: Return subtask execution results
/// - Final call: Return synthesized result
struct MultiStepMockLlmPort {
    responses: Arc<Mutex<Vec<String>>>,
    call_count: Arc<Mutex<usize>>,
}

impl MultiStepMockLlmPort {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            call_count: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait]
impl LlmPort for MultiStepMockLlmPort {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let mut count = self.call_count.lock().unwrap();
        let responses = self.responses.lock().unwrap();

        let response_content = if *count < responses.len() {
            responses[*count].clone()
        } else {
            // Return last response if we run out
            responses.last().unwrap().clone()
        };

        *count += 1;

        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "test-model".to_string(),
            content: response_content,
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(10, 20),
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        })
    }

    async fn generate_stream(
        &self,
        _request: LlmRequest,
    ) -> Result<
        Box<
            dyn futures::Stream<
                    Item = Result<paladin_ports::output::llm_port::StreamingResponse, LlmError>,
                > + Send,
        >,
        LlmError,
    > {
        unimplemented!("Streaming not needed for integration tests")
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["test-model".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "mock-multi-step"
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: false,
            supports_function_calling: false,
            supports_tool_calling: false,
            supports_vision: false,
            supports_embeddings: false,
            supports_system_messages: true,
            max_context_tokens: Some(4096),
            temperature_range: None,
        }
    }
}

#[tokio::test]
async fn test_full_planning_workflow_simple_linear() {
    // Given: A multi-step mock LLM with responses for plan creation, execution, and synthesis
    let responses = vec![
        // Response 1: Task decomposition plan
        r#"{
            "task": "Write a blog post about Rust",
            "subtasks": [
                {
                    "id": "1",
                    "description": "Research Rust features and benefits",
                    "dependencies": []
                },
                {
                    "id": "2",
                    "description": "Create outline with key points",
                    "dependencies": ["1"]
                },
                {
                    "id": "3",
                    "description": "Write draft based on outline",
                    "dependencies": ["2"]
                }
            ]
        }"#.to_string(),
        // Response 2: Subtask 1 result
        "Research complete: Rust offers memory safety, zero-cost abstractions, fearless concurrency, and excellent tooling (cargo, clippy). Key benefits: performance, safety, productivity.".to_string(),
        // Response 3: Subtask 2 result
        "Outline created:\nI. Introduction to Rust\nII. Key Features\n  A. Memory Safety\n  B. Performance\n  C. Concurrency\nIII. Benefits for developers\nIV. Conclusion".to_string(),
        // Response 4: Subtask 3 result
        "Draft written: 'Rust: A Modern Systems Programming Language' - comprehensive 1500-word article covering all outlined points with code examples.".to_string(),
        // Response 5: Synthesis
        "Blog post successfully created:\n1. Conducted research on Rust features and benefits\n2. Structured content with comprehensive outline\n3. Completed 1500-word draft covering memory safety, performance, concurrency\n\nThe article is ready for review and publication.".to_string(),
    ];

    let llm_port = Arc::new(MultiStepMockLlmPort::new(responses));
    let service = PlanningService::new(llm_port);

    // When: Executing the full planning workflow
    // Step 1: Create plan
    let plan = service
        .create_plan("Write a blog post about Rust", 10, "gpt-4")
        .await
        .expect("Plan creation should succeed");

    assert_eq!(plan.subtasks.len(), 3);
    assert_eq!(plan.original_task, "Write a blog post about Rust");

    // Step 2: Execute subtasks
    let executed_plan = service
        .execute_subtasks(&plan, "Write a blog post about Rust", "gpt-4")
        .await
        .expect("Subtask execution should succeed");

    assert!(executed_plan.subtasks.iter().all(|st| st.completed));

    // Step 3: Synthesize results
    let final_result = service
        .synthesize_results(&executed_plan, "Write a blog post about Rust", "gpt-4")
        .await
        .expect("Synthesis should succeed");

    // Then: Verify complete workflow execution
    assert!(final_result.contains("Blog post successfully created"));
    assert!(final_result.contains("ready for review"));

    // Verify all subtasks have results
    for subtask in &executed_plan.subtasks {
        assert!(
            subtask.result.is_some(),
            "Subtask {} should have a result",
            subtask.id
        );
    }
}

#[tokio::test]
async fn test_full_planning_workflow_with_parallel_tasks() {
    // Given: A plan with parallel execution branches
    let responses = vec![
        // Response 1: Task decomposition with parallel branches
        r#"{
            "task": "Prepare for product launch",
            "subtasks": [
                {
                    "id": "1",
                    "description": "Set launch date",
                    "dependencies": []
                },
                {
                    "id": "2",
                    "description": "Prepare marketing materials",
                    "dependencies": ["1"]
                },
                {
                    "id": "3",
                    "description": "Setup analytics tracking",
                    "dependencies": ["1"]
                },
                {
                    "id": "4",
                    "description": "Launch campaign and monitor",
                    "dependencies": ["2", "3"]
                }
            ]
        }"#.to_string(),
        // Response 2: Subtask 1 result
        "Launch date set: March 15, 2024. All stakeholders notified.".to_string(),
        // Response 3: Subtask 2 result (parallel with 4)
        "Marketing materials ready: press release, social media posts, email campaign.".to_string(),
        // Response 4: Subtask 3 result (parallel with 3)
        "Analytics setup complete: Google Analytics, Mixpanel, custom dashboards configured.".to_string(),
        // Response 5: Subtask 4 result
        "Campaign launched successfully on all channels. Monitoring metrics in real-time.".to_string(),
        // Response 6: Synthesis
        "Product launch preparation complete:\n- Launch date confirmed: March 15, 2024\n- Marketing materials: press release, social media, email campaigns ready\n- Analytics: comprehensive tracking setup\n- Campaign: launched and actively monitored\n\nAll systems go for product launch!".to_string(),
    ];

    let llm_port = Arc::new(MultiStepMockLlmPort::new(responses));
    let service = PlanningService::new(llm_port);

    // When: Executing workflow with parallel subtasks
    let plan = service
        .create_plan("Prepare for product launch", 10, "gpt-4")
        .await
        .expect("Plan creation should succeed");

    let executed_plan = service
        .execute_subtasks(&plan, "Prepare for product launch", "gpt-4")
        .await
        .expect("Subtask execution should succeed");

    let final_result = service
        .synthesize_results(&executed_plan, "Prepare for product launch", "gpt-4")
        .await
        .expect("Synthesis should succeed");

    // Then: Verify parallel execution was respected
    // Subtask 2 and 3 can execute in parallel after 1
    // Subtask 4 must wait for both 2 and 3
    assert_eq!(executed_plan.subtasks.len(), 4);
    assert!(final_result.contains("launch preparation complete"));
    assert!(final_result.contains("March 15, 2024"));
}

#[tokio::test]
async fn test_planning_workflow_with_max_subtasks_enforcement() {
    // Given: LLM returns a plan exceeding max_subtasks limit
    let responses = vec![
        r#"{
            "task": "Large project",
            "subtasks": [
                {"id": "1", "description": "Task 1", "dependencies": []},
                {"id": "2", "description": "Task 2", "dependencies": []},
                {"id": "3", "description": "Task 3", "dependencies": []},
                {"id": "4", "description": "Task 4", "dependencies": []},
                {"id": "5", "description": "Task 5", "dependencies": []},
                {"id": "6", "description": "Task 6", "dependencies": []}
            ]
        }"#
        .to_string(),
    ];

    let llm_port = Arc::new(MultiStepMockLlmPort::new(responses));
    let service = PlanningService::new(llm_port);

    // When: Creating a plan with max_subtasks=3
    let result = service.create_plan("Large project", 3, "gpt-4").await;

    // Then: Should reject the plan
    assert!(result.is_err());
    match result {
        Err(paladin::application::errors::planning_error::PlanningError::MaxSubtasksExceeded {
            max,
            attempted,
        }) => {
            assert_eq!(max, 3);
            assert_eq!(attempted, 6);
        }
        _ => panic!("Expected MaxSubtasksExceeded error"),
    }
}
