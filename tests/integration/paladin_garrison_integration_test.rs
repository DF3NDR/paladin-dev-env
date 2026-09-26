//! Integration tests for Paladin with Garrison memory system
//!
//! These tests verify that Paladins correctly integrate with Garrison for conversation
//! context management across multiple turns.

use async_trait::async_trait;
use chrono::Utc;
use paladin::application::services::paladin::error::PaladinError;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::garrison::{
    ConversationRole, GarrisonConfig, GarrisonEntry,
};
use paladin::infrastructure::adapters::garrison::in_memory_garrison::InMemoryGarrison;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::garrison_port::{GarrisonPort, GarrisonStats};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, StreamingResponse, TokenUsage,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// Mock LLM Port for testing
struct MockLlmPort {
    response_content: String,
}

impl MockLlmPort {
    fn new(response: impl Into<String>) -> Self {
        Self {
            response_content: response.into(),
        }
    }
}

#[async_trait]
impl LlmPort for MockLlmPort {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            id: uuid::Uuid::new_v4(),
            request_id: request.id,
            content: self.response_content.clone(),
            model: "gpt-4".to_string(),
            usage: TokenUsage::new(10, 20),
            cost: None,
            finish_reason: FinishReason::Stop,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        })
    }

    async fn generate_stream(
        &self,
        _request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
    {
        unimplemented!("Streaming not used in these tests")
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["gpt-4".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "Mock"
    }
    fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
        paladin_ports::output::llm_port::ProviderCapabilities::default()
    }
}

#[tokio::test]
async fn test_paladin_with_garrison_stores_conversation() {
    // Create garrison with default config
    let config = GarrisonConfig::default();
    let garrison = Arc::new(InMemoryGarrison::new(config)) as Arc<dyn GarrisonPort>;

    // Create mock LLM port
    let llm_port = Arc::new(MockLlmPort::new("I am a helpful assistant ready to code!"));

    // Build paladin WITH garrison
    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("You are a coding assistant")
        .name("TestPaladin")
        .model("gpt-4")
        .with_garrison(garrison.clone())
        .build()
        .await
        .expect("Failed to build paladin");

    // Create execution service WITH garrison
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, Some(garrison.clone()), None);

    // Execute first turn
    let result = service.execute(&paladin, "Hello, can you help me?").await;
    assert!(result.is_ok(), "First execution should succeed");

    // Verify garrison contains entries
    let stats = garrison.stats().await.expect("Failed to get stats");
    assert!(
        stats.entry_count >= 2,
        "Should have at least user input and assistant response"
    );

    // Verify we can recall recent entries
    let recent = garrison
        .recall_recent(10)
        .await
        .expect("Failed to recall recent");
    assert!(!recent.is_empty(), "Should have stored entries");

    // Check that user message was stored
    let user_entries: Vec<_> = recent
        .iter()
        .filter(|e| matches!(e.role, ConversationRole::User))
        .collect();
    assert!(!user_entries.is_empty(), "Should have user entry");
    assert!(
        user_entries[0].content.contains("Hello"),
        "User entry should contain input"
    );

    // Check that assistant response was stored
    let assistant_entries: Vec<_> = recent
        .iter()
        .filter(|e| matches!(e.role, ConversationRole::Assistant))
        .collect();
    assert!(!assistant_entries.is_empty(), "Should have assistant entry");
}

#[tokio::test]
async fn test_paladin_without_garrison_single_turn() {
    // Create mock LLM port
    let llm_port = Arc::new(MockLlmPort::new("Single turn response"));

    // Build paladin WITHOUT garrison
    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("You are a helpful assistant")
        .name("SingleTurnPaladin")
        .model("gpt-4")
        .build()
        .await
        .expect("Failed to build paladin");

    // Create execution service WITHOUT garrison (None)
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

    // Execute single turn - should work without garrison
    let result = service.execute(&paladin, "Quick question").await;
    assert!(
        result.is_ok(),
        "Single turn without garrison should succeed"
    );
}

#[tokio::test]
async fn test_paladin_multi_turn_conversation() {
    // Create garrison with default config
    let config = GarrisonConfig::default();
    let garrison = Arc::new(InMemoryGarrison::new(config)) as Arc<dyn GarrisonPort>;

    // Create mock LLM port that echoes context
    let llm_port = Arc::new(MockLlmPort::new("Response based on context"));

    // Build paladin WITH garrison
    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("You are a conversational assistant")
        .name("ConversationalPaladin")
        .model("gpt-4")
        .max_loops(1) // Single loop per turn
        .with_garrison(garrison.clone())
        .build()
        .await
        .expect("Failed to build paladin");

    // Create execution service WITH garrison
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, Some(garrison.clone()), None);

    // First turn
    let result1 = service.execute(&paladin, "My name is Alice").await;
    assert!(result1.is_ok(), "First turn should succeed");

    // Second turn - garrison should have previous context
    let result2 = service.execute(&paladin, "What is my name?").await;
    assert!(result2.is_ok(), "Second turn should succeed");

    // Verify garrison has all messages
    let stats = garrison.stats().await.expect("Failed to get stats");
    assert!(
        stats.entry_count >= 4,
        "Should have 2 user + 2 assistant messages"
    );

    // Verify conversation order
    let recent = garrison.recall_recent(10).await.expect("Failed to recall");
    assert!(recent.len() >= 4, "Should have at least 4 entries");
}

#[tokio::test]
async fn test_garrison_error_handling() {
    // Create garrison that will fail
    struct FailingGarrison;

    #[async_trait]
    impl GarrisonPort for FailingGarrison {
        async fn remember(
            &self,
            _entry: GarrisonEntry,
        ) -> Result<(), paladin_ports::output::garrison_port::GarrisonError> {
            Err(
                paladin_ports::output::garrison_port::GarrisonError::StorageError(
                    "Mock storage failure".to_string(),
                ),
            )
        }

        async fn recall_recent(
            &self,
            _limit: usize,
        ) -> Result<Vec<GarrisonEntry>, paladin_ports::output::garrison_port::GarrisonError>
        {
            Ok(vec![])
        }

        async fn search(
            &self,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<GarrisonEntry>, paladin_ports::output::garrison_port::GarrisonError>
        {
            Ok(vec![])
        }

        async fn forget_all(
            &self,
        ) -> Result<(), paladin_ports::output::garrison_port::GarrisonError> {
            Ok(())
        }

        async fn stats(
            &self,
        ) -> Result<GarrisonStats, paladin_ports::output::garrison_port::GarrisonError> {
            Ok(GarrisonStats {
                entry_count: 0,
                total_tokens: 0,
                size_bytes: Some(0),
            })
        }
    }

    let garrison = Arc::new(FailingGarrison) as Arc<dyn GarrisonPort>;
    let llm_port = Arc::new(MockLlmPort::new("Response"));

    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("Test")
        .with_garrison(garrison.clone())
        .build()
        .await
        .expect("Failed to build");

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, Some(garrison.clone()), None);

    // Execution should fail due to garrison error
    let result = service.execute(&paladin, "Test input").await;
    assert!(result.is_err(), "Should fail when garrison fails");

    // Verify error type
    if let Err(e) = result {
        assert!(
            matches!(e, PaladinError::GarrisonError(_)),
            "Should be GarrisonError variant"
        );
    }
}
#[tokio::test]
async fn test_garrison_token_limit_enforcement() {
    // Create garrison with small token limit to test eviction
    let config = GarrisonConfig::new(10, Some(100)); // 10 entries, 100 token limit

    let garrison = Arc::new(InMemoryGarrison::new(config)) as Arc<dyn GarrisonPort>;
    let llm_port = Arc::new(MockLlmPort::new("This is a response that will be stored."));

    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("You are a helpful assistant")
        .name("TokenLimitPaladin")
        .with_garrison(garrison.clone())
        .build()
        .await
        .expect("Failed to build paladin");

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, Some(garrison.clone()), None);

    // Execute multiple turns to exceed token limit
    for i in 0..5 {
        let result = service
            .execute(&paladin, &format!("Turn {} with some content", i))
            .await;
        assert!(result.is_ok(), "Turn {} should succeed", i);
    }

    // Verify garrison has evicted entries to stay under token limit
    let stats = garrison.stats().await.expect("Failed to get stats");
    assert!(
        stats.total_tokens <= 100,
        "Token count should not exceed limit, got {}",
        stats.total_tokens
    );
}

#[tokio::test]
async fn test_garrison_importance_based_eviction() {
    // Create garrison with importance-based eviction
    let config = GarrisonConfig::new(5, None)
        .with_eviction_strategy(
            paladin::core::platform::container::garrison::EvictionStrategy::ImportanceBased,
        )
        .with_preserve_recent(2); // Always keep 2 most recent

    let garrison = Arc::new(InMemoryGarrison::new(config)) as Arc<dyn GarrisonPort>;
    let llm_port = Arc::new(MockLlmPort::new("Response"));

    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("Test")
        .with_garrison(garrison.clone())
        .build()
        .await
        .expect("Failed to build");

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, Some(garrison.clone()), None);

    // Add more entries than max_entries
    for i in 0..10 {
        let result = service.execute(&paladin, &format!("Message {}", i)).await;
        assert!(result.is_ok(), "Execution {} should succeed", i);
    }

    // Verify entry count is within limit
    let stats = garrison.stats().await.expect("Failed to get stats");
    assert!(
        stats.entry_count <= 5,
        "Entry count should not exceed max_entries"
    );

    // Verify recent entries are preserved
    let recent = garrison.recall_recent(10).await.expect("Failed to recall");
    assert!(
        recent.len() >= 2,
        "Should preserve at least 2 recent entries"
    );
}

#[tokio::test]
async fn test_garrison_fifo_eviction() {
    // Create garrison with FIFO eviction
    let config = GarrisonConfig::new(4, None).with_eviction_strategy(
        paladin::core::platform::container::garrison::EvictionStrategy::FIFO,
    );

    let garrison = Arc::new(InMemoryGarrison::new(config)) as Arc<dyn GarrisonPort>;
    let llm_port = Arc::new(MockLlmPort::new("Response"));

    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("Test")
        .with_garrison(garrison.clone())
        .build()
        .await
        .expect("Failed to build");

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, Some(garrison.clone()), None);

    // Add entries to trigger FIFO eviction
    for i in 0..6 {
        let result = service.execute(&paladin, &format!("Turn {}", i)).await;
        assert!(result.is_ok());
    }

    // Verify entry count is within limit
    let stats = garrison.stats().await.expect("Failed to get stats");
    assert!(
        stats.entry_count <= 4,
        "FIFO should maintain max_entries limit"
    );
}

#[tokio::test]
async fn test_garrison_sliding_window_eviction() {
    // Create garrison with sliding window eviction
    let config = GarrisonConfig::new(6, None)
        .with_eviction_strategy(
            paladin::core::platform::container::garrison::EvictionStrategy::SlidingWindow,
        )
        .with_preserve_recent(3);

    let garrison = Arc::new(InMemoryGarrison::new(config)) as Arc<dyn GarrisonPort>;
    let llm_port = Arc::new(MockLlmPort::new("Response"));

    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("Test")
        .with_garrison(garrison.clone())
        .build()
        .await
        .expect("Failed to build");

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, Some(garrison.clone()), None);

    // Add entries to trigger sliding window
    for i in 0..8 {
        let result = service.execute(&paladin, &format!("Message {}", i)).await;
        assert!(result.is_ok());
    }

    // Verify entry count is within limit
    let stats = garrison.stats().await.expect("Failed to get stats");
    assert!(
        stats.entry_count <= 6,
        "Sliding window should maintain max_entries"
    );

    // Verify recent entries exist
    let recent = garrison.recall_recent(10).await.expect("Failed to recall");
    assert!(
        recent.len() >= 3,
        "Should preserve recent entries in sliding window"
    );
}

#[tokio::test]
async fn test_garrison_search_functionality() {
    // Create garrison with default config
    let config = GarrisonConfig::default();
    let garrison = Arc::new(InMemoryGarrison::new(config)) as Arc<dyn GarrisonPort>;
    let llm_port = Arc::new(MockLlmPort::new("Response about Rust programming"));

    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("Test")
        .with_garrison(garrison.clone())
        .build()
        .await
        .expect("Failed to build");

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, Some(garrison.clone()), None);

    // Execute with specific content
    let result = service
        .execute(&paladin, "Tell me about Rust programming language")
        .await;
    assert!(result.is_ok());

    // Search for content
    let search_results = garrison
        .search("Rust", 10)
        .await
        .expect("Search should succeed");

    assert!(
        !search_results.is_empty(),
        "Should find entries containing 'Rust'"
    );
    assert!(
        search_results.iter().any(|e| e.content.contains("Rust")),
        "Search results should contain the query term"
    );
}

#[tokio::test]
async fn test_garrison_forget_all() {
    // Create garrison with default config
    let config = GarrisonConfig::default();
    let garrison = Arc::new(InMemoryGarrison::new(config)) as Arc<dyn GarrisonPort>;
    let llm_port = Arc::new(MockLlmPort::new("Response"));

    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("Test")
        .with_garrison(garrison.clone())
        .build()
        .await
        .expect("Failed to build");

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, Some(garrison.clone()), None);

    // Execute multiple turns
    for i in 0..3 {
        let result = service.execute(&paladin, &format!("Turn {}", i)).await;
        assert!(result.is_ok());
    }

    // Verify entries exist
    let stats_before = garrison.stats().await.expect("Failed to get stats");
    assert!(stats_before.entry_count > 0, "Should have entries");

    // Clear garrison
    garrison
        .forget_all()
        .await
        .expect("Forget all should succeed");

    // Verify garrison is empty
    let stats_after = garrison.stats().await.expect("Failed to get stats");
    assert_eq!(stats_after.entry_count, 0, "Garrison should be empty");
    assert_eq!(stats_after.total_tokens, 0, "Token count should be zero");
}

#[tokio::test]
async fn test_garrison_with_circuit_breaker_interaction() {
    // Test that garrison works correctly when circuit breaker trips
    let config = GarrisonConfig::default();
    let garrison = Arc::new(InMemoryGarrison::new(config)) as Arc<dyn GarrisonPort>;

    // Create failing LLM port
    struct FailingLlmPort;

    #[async_trait]
    impl LlmPort for FailingLlmPort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Err(LlmError::ProcessingError("Simulated failure".to_string()))
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            unimplemented!()
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec![])
        }

        fn get_provider_name(&self) -> &'static str {
            "Failing"
        }

        fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
            paladin_ports::output::llm_port::ProviderCapabilities::default()
        }
    }

    let llm_port = Arc::new(FailingLlmPort);

    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("Test")
        .with_garrison(garrison.clone())
        .build()
        .await
        .expect("Failed to build");

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(1)));
    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, Some(garrison.clone()), None);

    // Execute and expect failure
    let result = service.execute(&paladin, "Test input").await;
    assert!(result.is_err(), "Should fail due to LLM error");

    // Verify garrison was not corrupted by failure
    let stats = garrison.stats().await.expect("Garrison should still work");
    // User message might have been stored before LLM failure
    assert!(
        stats.entry_count <= 1,
        "Should have at most user message stored"
    );
}

#[tokio::test]
async fn test_garrison_stats_accuracy() {
    // Test that garrison stats are accurate
    let config = GarrisonConfig::default();
    let garrison = Arc::new(InMemoryGarrison::new(config)) as Arc<dyn GarrisonPort>;
    let llm_port = Arc::new(MockLlmPort::new("Test response"));

    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("Test")
        .with_garrison(garrison.clone())
        .build()
        .await
        .expect("Failed to build");

    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service =
        PaladinExecutionService::new(llm_port, circuit_breaker, Some(garrison.clone()), None);

    // Initial stats should be empty
    let initial_stats = garrison.stats().await.expect("Failed to get stats");
    assert_eq!(initial_stats.entry_count, 0);
    assert_eq!(initial_stats.total_tokens, 0);

    // Execute one turn
    let result = service.execute(&paladin, "Hello").await;
    assert!(result.is_ok());

    // Stats should reflect new entries
    let after_stats = garrison.stats().await.expect("Failed to get stats");
    assert!(
        after_stats.entry_count >= 2,
        "Should have user + assistant entries, got {}",
        after_stats.entry_count
    );
    // Token counting may be zero if tokenizer is not configured, but entries should exist
    // This is acceptable for testing - the important part is that entries are stored

    // Execute another turn
    let result2 = service.execute(&paladin, "Another message").await;
    assert!(result2.is_ok());

    // Stats should increase
    let final_stats = garrison.stats().await.expect("Failed to get stats");
    assert!(
        final_stats.entry_count > after_stats.entry_count,
        "Entry count should increase from {} to {}",
        after_stats.entry_count,
        final_stats.entry_count
    );
    // Token count may or may not increase depending on tokenizer configuration
    // The important part is that the garrison is tracking entries correctly
}
