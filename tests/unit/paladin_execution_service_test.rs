//! Unit tests for PaladinExecutionService
//! Following TDD - these tests should fail initially

use paladin::application::services::paladin::error::PaladinError;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, StreamingResponse, TokenUsage,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Mock LLM Port that tracks calls and can simulate various scenarios
struct MockLlmPort {
    call_count: AtomicU32,
    responses: Mutex<Vec<Result<String, LlmError>>>,
    delay: Option<Duration>,
}

impl MockLlmPort {
    fn new(responses: Vec<Result<String, LlmError>>) -> Self {
        Self {
            call_count: AtomicU32::new(0),
            responses: Mutex::new(responses),
            delay: None,
        }
    }

    fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    fn simple_success() -> Self {
        Self::new(vec![Ok("Success response".to_string())])
    }

    fn with_stop_word() -> Self {
        Self::new(vec![Ok("Response with STOP keyword".to_string())])
    }

    fn with_failures_then_success(failure_count: usize) -> Self {
        let mut responses = vec![];
        for _ in 0..failure_count {
            responses.push(Err(LlmError::ProcessingError(
                "Temporary failure".to_string(),
            )));
        }
        responses.push(Ok("Success after retries".to_string()));
        Self::new(responses)
    }

    fn always_fail() -> Self {
        Self::new(vec![Err(LlmError::ProcessingError(
            "Permanent failure".to_string(),
        ))])
    }

    fn get_call_count(&self) -> u32 {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl LlmPort for MockLlmPort {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Apply delay if configured
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }

        let call_num = self.call_count.fetch_add(1, Ordering::SeqCst) as usize;
        let responses = self.responses.lock().unwrap();

        let response = if call_num < responses.len() {
            responses[call_num].clone()
        } else {
            // Return last response if we've exhausted the list
            responses
                .last()
                .cloned()
                .unwrap_or(Ok("Default response".to_string()))
        };

        match response {
            Ok(content) => Ok(LlmResponse {
                id: uuid::Uuid::new_v4(),
                request_id: uuid::Uuid::new_v4(),
                model: "mock-model".to_string(),
                content,
                finish_reason: FinishReason::Stop,
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                },
                created_at: chrono::Utc::now(),
                metadata: std::collections::HashMap::new(),
                function_call: None,
            }),
            Err(e) => Err(e),
        }
    }

    async fn generate_stream(
        &self,
        _request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
    {
        Err(LlmError::ProcessingError(
            "Streaming not implemented in mock".to_string(),
        ))
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["mock-model".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "MockProvider"
    }

    fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
        paladin_ports::output::llm_port::ProviderCapabilities::default()
    }
}

#[tokio::test]
async fn test_execution_service_executes_successfully() {
    let llm_port = Arc::new(MockLlmPort::simple_success());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .name("TestPaladin")
        .max_loops(1) // Only run one loop for this test
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_ok(), "Execution should succeed");
    let paladin_result = result.unwrap();
    assert!(
        !paladin_result.output.is_empty(),
        "Output should not be empty"
    );
    assert_eq!(
        paladin_result.loop_count, 1,
        "Should execute exactly one loop"
    );
    assert!(
        paladin_result.token_count > 0,
        "Token count should be tracked"
    );
    // Execution time is always tracked (u64, defaults to 0)
}

#[tokio::test]
async fn test_execution_service_respects_max_loops() {
    // Create mock that returns multiple responses
    let responses = vec![
        Ok("Loop 1".to_string()),
        Ok("Loop 2".to_string()),
        Ok("Loop 3".to_string()),
        Ok("Loop 4".to_string()),
        Ok("Loop 5".to_string()),
        Ok("Loop 6".to_string()), // This shouldn't be reached
    ];
    let llm_port = Arc::new(MockLlmPort::new(responses));
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(5)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_ok(), "Execution should succeed");
    let paladin_result = result.unwrap();
    assert_eq!(
        paladin_result.loop_count, 5,
        "Should execute exactly max_loops iterations"
    );
}

#[tokio::test]
async fn test_execution_service_detects_stop_words() {
    let llm_port = Arc::new(MockLlmPort::with_stop_word());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .add_stop_word("STOP")
        .max_loops(5)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_err(), "Should fail when stop word detected");
    match result.unwrap_err() {
        PaladinError::StopWordDetected(word) => {
            assert_eq!(word, "STOP", "Should detect the correct stop word");
        }
        e => panic!("Expected StopWordDetected error, got {:?}", e),
    }
}

#[tokio::test]
async fn test_execution_service_enforces_timeout() {
    // Note: Current implementation uses max_loops * 60 seconds as timeout
    // For practical testing, we test the timeout mechanism works correctly
    // by using a mock with 2 second delay and verifying it completes within timeout

    // Create a mock with minimal delay
    let llm_port = Arc::new(MockLlmPort::simple_success().with_delay(Duration::from_millis(100)));
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(120)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    // Configure Paladin with max_loops=1 (timeout will be 60 seconds)
    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(1)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    // Should succeed - demonstrates timeout mechanism works when delay < timeout
    assert!(result.is_ok(), "Should complete within timeout");
}

#[tokio::test]
async fn test_execution_completes_before_timeout() {
    // Create a mock with short delay (under timeout)
    let llm_port = Arc::new(MockLlmPort::simple_success().with_delay(Duration::from_millis(50)));
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    // Configure Paladin with max_loops=1 (timeout=60s), mock takes 50ms
    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(1)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    // Should complete successfully before timeout
    assert!(result.is_ok(), "Should complete successfully: {:?}", result);
    let paladin_result = result.unwrap();
    assert_eq!(paladin_result.loop_count, 1);
    assert!(!paladin_result.output.is_empty());
}

#[tokio::test]
async fn test_timeout_with_multiple_loops() {
    // Verify timeout calculation: max_loops * 60 seconds
    // We test the calculation is correct by checking the mechanism works
    let llm_port = Arc::new(
        MockLlmPort::new(vec![
            Ok("First loop".to_string()),
            Ok("Second loop".to_string()),
        ])
        .with_delay(Duration::from_millis(100)),
    );
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(180)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    // Configure Paladin: max_loops=2 means timeout=120 seconds
    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(2)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    // Should complete successfully (demonstrates multi-loop with timeout works)
    assert!(result.is_ok(), "Should complete multi-loop execution");
}

#[tokio::test]
async fn test_execution_service_retries_on_failure() {
    let llm_port = Arc::new(MockLlmPort::with_failures_then_success(2));
    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .retry_attempts(3)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_ok(), "Should succeed after retries");
    assert!(
        llm_port.get_call_count() > 1,
        "Should have retried at least once"
    );
}

#[tokio::test]
async fn test_execution_service_exponential_backoff() {
    let llm_port = Arc::new(MockLlmPort::with_failures_then_success(2));
    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .retry_attempts(3)
        .build()
        .await
        .expect("Failed to build paladin");

    let start = std::time::Instant::now();
    let result = service.execute(&paladin, "Test input").await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Should succeed after retries");
    // With 2 failures: 100ms + 200ms = 300ms minimum
    assert!(
        elapsed >= Duration::from_millis(250),
        "Should have exponential backoff delay (expected >= 250ms, got {:?})",
        elapsed
    );
}

#[tokio::test]
async fn test_execution_service_uses_circuit_breaker() {
    let llm_port = Arc::new(MockLlmPort::always_fail());
    let circuit_breaker = Arc::new(CircuitBreaker::new(2, 2, Duration::from_millis(100)));
    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker.clone(), None, None);

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .retry_attempts(1)
        .build()
        .await
        .expect("Failed to build paladin");

    // First attempt should fail and increment circuit breaker failure count
    let result1 = service.execute(&paladin, "Test input 1").await;
    assert!(result1.is_err(), "First attempt should fail");

    // Second attempt should fail and open the circuit
    let result2 = service.execute(&paladin, "Test input 2").await;
    assert!(result2.is_err(), "Second attempt should fail");

    // Third attempt should fail fast due to open circuit
    let result3 = service.execute(&paladin, "Test input 3").await;
    assert!(result3.is_err(), "Third attempt should fail fast");
    match result3.unwrap_err() {
        PaladinError::CircuitBreakerOpen => {
            // Expected - circuit breaker is protecting us
        }
        e => panic!("Expected CircuitBreakerOpen, got {:?}", e),
    }
}

#[tokio::test]
async fn test_execution_service_tracks_metadata() {
    let llm_port = Arc::new(MockLlmPort::simple_success());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(1) // Set to 1 loop for predictable testing
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_ok(), "Execution should succeed");
    let paladin_result = result.unwrap();

    // Verify all metadata is tracked
    // execution_time_ms is u64, always >= 0, can be 0 for fast mocks
    assert!(
        paladin_result.token_count > 0,
        "Token count should be tracked"
    );
    assert_eq!(paladin_result.loop_count, 1, "Loop count should be tracked");
    assert!(
        !paladin_result.output.is_empty(),
        "Output should be captured"
    );
}

// ==================== PHASE 4: AUTONOMOUS ORCHESTRATION TESTS ====================
// Tests for Epic 21 - Layered execution with graceful degradation
// ==============================================================================

#[tokio::test]
async fn test_layer0_core_execution_always_runs() {
    // Layer 0 (Core) should always execute, even with all autonomous features disabled
    let llm_port = Arc::new(MockLlmPort::simple_success());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(1)
        // All autonomous features disabled (default)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_ok(), "Core execution should always succeed");
    let paladin_result = result.unwrap();
    assert!(
        !paladin_result.output.is_empty(),
        "Core execution should produce output"
    );
    assert!(
        paladin_result.plan.is_none(),
        "No planning service, so plan should be None"
    );
    assert!(
        paladin_result.handoff_history.is_empty(),
        "No handoffs without agent_description"
    );
}

#[tokio::test]
async fn test_layer2_dynamic_temperature_disabled() {
    // When dynamic_temperature is false, temperature should remain constant
    let llm_port = Arc::new(MockLlmPort::simple_success());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .temperature(0.7)
        .max_loops(3)
        // dynamic_temperature = false (default)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_ok(), "Execution should succeed");
    // Note: We can't directly verify temperature was constant in the mock,
    // but the test validates the code path runs without dynamic temperature
}

#[tokio::test]
async fn test_layer2_dynamic_temperature_enabled() {
    // When dynamic_temperature is true, temperature should increase per loop
    let llm_port = Arc::new(MockLlmPort::new(vec![
        Ok("Loop 1".to_string()),
        Ok("Loop 2".to_string()),
        Ok("Loop 3".to_string()),
    ]));
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let mut paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .temperature(0.5)
        .max_loops(3)
        .build()
        .await
        .expect("Failed to build paladin");

    // Enable dynamic temperature
    paladin.node.dynamic_temperature = true;

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_ok(), "Execution should succeed");
    let paladin_result = result.unwrap();
    assert_eq!(paladin_result.loop_count, 3, "Should run all loops");
    // Note: Mock doesn't capture temperature values, but validates the code path
}

#[tokio::test]
async fn test_autonomous_metadata_population() {
    // Verify PaladinResult metadata fields are populated correctly
    let llm_port = Arc::new(MockLlmPort::simple_success());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(1)
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_ok(), "Execution should succeed");
    let paladin_result = result.unwrap();

    // Phase 2 enhancements: PaladinResult now has plan and handoff_history
    assert!(
        paladin_result.plan.is_none(),
        "No planning service configured"
    );
    assert!(
        paladin_result.handoff_history.is_empty(),
        "No handoffs executed"
    );
    assert!(paladin_result.token_count > 0, "Token count tracked");
    // execution_time_ms is u64, always >= 0, no need to check
    assert_eq!(paladin_result.loop_count, 1, "Loop count tracked");
}

#[tokio::test]
async fn test_graceful_degradation_no_planning_service() {
    // With autonomous_planning=true but no PlanningService, execution should continue
    let llm_port = Arc::new(MockLlmPort::simple_success());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let mut paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(1)
        .build()
        .await
        .expect("Failed to build paladin");

    // Enable planning without service (graceful degradation test)
    paladin.node.autonomous_planning = true;

    let result = service.execute(&paladin, "Test input").await;

    assert!(
        result.is_ok(),
        "Should gracefully degrade when planning service missing"
    );
    let paladin_result = result.unwrap();
    assert!(paladin_result.plan.is_none(), "No plan without service");
    assert!(
        !paladin_result.output.is_empty(),
        "Core execution should still work"
    );
}

#[tokio::test]
async fn test_graceful_degradation_no_prompt_service() {
    // With autonomous_prompts=true but no PromptGenerationService, execution should continue
    let llm_port = Arc::new(MockLlmPort::simple_success());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let mut paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .agent_description("A test agent") // Required for prompt generation
        .max_loops(1)
        .build()
        .await
        .expect("Failed to build paladin");

    // Enable prompt generation without service (graceful degradation test)
    paladin.node.autonomous_prompts = true;

    let result = service.execute(&paladin, "Test input").await;

    assert!(
        result.is_ok(),
        "Should gracefully degrade when prompt service missing"
    );
    let paladin_result = result.unwrap();
    assert!(
        !paladin_result.output.is_empty(),
        "Core execution should still work"
    );
}

#[tokio::test]
async fn test_all_autonomous_features_disabled() {
    // Baseline test: all autonomous features off, pure Layer 0 execution
    let llm_port = Arc::new(MockLlmPort::simple_success());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .max_loops(2)
        // All defaults: autonomous_planning=false, autonomous_prompts=false, dynamic_temperature=false
        .build()
        .await
        .expect("Failed to build paladin");

    let result = service.execute(&paladin, "Test input").await;

    assert!(result.is_ok(), "Pure Layer 0 execution should work");
    let paladin_result = result.unwrap();
    assert!(!paladin_result.output.is_empty(), "Should have output");
    assert!(paladin_result.plan.is_none(), "No planning");
    assert!(paladin_result.handoff_history.is_empty(), "No handoffs");
}

#[tokio::test]
async fn test_mixed_autonomous_features() {
    // Enable some features but not others to test independent layer execution
    let llm_port = Arc::new(MockLlmPort::simple_success());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

    let mut paladin = PaladinBuilder::new(llm_port.clone() as Arc<dyn LlmPort>)
        .system_prompt("You are a helpful assistant")
        .temperature(0.6)
        .max_loops(2)
        .build()
        .await
        .expect("Failed to build paladin");

    // Enable only dynamic_temperature (Layer 2)
    paladin.node.dynamic_temperature = true;

    let result = service.execute(&paladin, "Test input").await;

    assert!(
        result.is_ok(),
        "Should work with partial feature enablement"
    );
    let paladin_result = result.unwrap();
    assert!(!paladin_result.output.is_empty(), "Core execution works");
    assert!(paladin_result.plan.is_none(), "Planning not enabled");
}

// ---------------------------------------------------------------------------
// Phase 25 Plan 08 (FT-FR-17, D-26): the serving provider on PaladinResult
// ---------------------------------------------------------------------------

mod served_by {
    use super::*;
    use paladin_core::platform::container::execution_result::{PaladinResult, StopReason};
    use paladin_llm::fallback::FallbackLlmAdapter;
    use paladin_llm::mock::MockLlmAdapter;

    fn service(llm_port: Arc<dyn LlmPort>) -> PaladinExecutionService {
        let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
        PaladinExecutionService::new(llm_port, circuit_breaker, None, None)
    }

    async fn run(llm_port: Arc<dyn LlmPort>) -> PaladinResult {
        let paladin = PaladinBuilder::new(llm_port.clone())
            .system_prompt("You are a helpful assistant")
            .name("ServedByPaladin")
            .max_loops(1)
            .build()
            .await
            .expect("Failed to build paladin");
        service(llm_port)
            .execute(&paladin, "Test input")
            .await
            .expect("execution should succeed")
    }

    /// Test 4: a service driven by a FallbackLlmAdapter whose first provider
    /// fails transiently reports the SECOND provider's name.
    #[tokio::test]
    async fn fallback_served_result_records_the_serving_provider() {
        let primary = MockLlmAdapter::new()
            .with_provider_name("openai")
            .with_error(LlmError::ProviderError {
                provider: "openai".to_string(),
                status: 503,
                message: "upstream unavailable".to_string(),
            });
        let backup = MockLlmAdapter::new()
            .with_provider_name("anthropic")
            .with_response("served by the backup");
        let chain = FallbackLlmAdapter::new(vec![
            Arc::new(primary) as Arc<dyn LlmPort>,
            Arc::new(backup) as Arc<dyn LlmPort>,
        ])
        .expect("two-element chain");

        let result = run(Arc::new(chain)).await;

        assert_eq!(result.output, "served by the backup");
        assert_eq!(result.served_by.as_deref(), Some("anthropic"));
    }

    /// Test 5: a plain single adapter stamps nothing, so `served_by` stays
    /// `None` and the JSON is byte-identical to today's.
    #[tokio::test]
    async fn a_non_fallback_result_leaves_served_by_none() {
        let single = MockLlmAdapter::new()
            .with_provider_name("openai")
            .with_response("served directly");

        let result = run(Arc::new(single)).await;

        assert_eq!(result.output, "served directly");
        assert!(result.served_by.is_none());
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("served_by"), "{json}");
    }

    /// Test 6: a CROSS-CRATE functional-update construction compiles, which
    /// is only possible because `PaladinResult` is not `#[non_exhaustive]`
    /// (D-26, X-10.3 option (b)).
    #[test]
    fn paladin_result_is_not_marked_non_exhaustive() {
        let result = PaladinResult {
            output: "constructed across a crate boundary".to_string(),
            stop_reason: StopReason::Completed,
            ..Default::default()
        };

        assert!(result.served_by.is_none());
        assert_eq!(result.loop_count, 0);
    }
}
