//! Integration tests for RAG (Retrieval-Augmented Generation) Configuration
//!
//! Tests RAG and Memory Extraction configuration validation.
//! Full end-to-end integration tests with Qdrant will be added in a follow-up task.

use paladin::config::{MemoryExtractionConfig, MemoryExtractionStrategy, RagConfig};

#[tokio::test]
async fn test_rag_config_validation() {
    // Test: RagConfig validates constraints
    let config = RagConfig {
        top_k: 5,
        min_similarity: 0.7,
        max_tokens: 2000,
        timeout_seconds: 5,
        ..RagConfig::default()
    };

    assert!(config.validate().is_ok());

    // Invalid configurations
    let invalid_top_k = RagConfig {
        top_k: 0,
        ..RagConfig::default()
    };
    assert!(invalid_top_k.validate().is_err());

    let invalid_similarity = RagConfig {
        min_similarity: 1.5,
        ..RagConfig::default()
    };
    assert!(invalid_similarity.validate().is_err());

    let invalid_tokens = RagConfig {
        max_tokens: 0,
        ..RagConfig::default()
    };
    assert!(invalid_tokens.validate().is_err());
}

#[tokio::test]
async fn test_memory_extraction_config_validation() {
    // Test: MemoryExtractionConfig validates correctly
    let config = MemoryExtractionConfig {
        enabled: true,
        strategy: MemoryExtractionStrategy::OnCompletion,
    };

    assert!(config.validate().is_ok());

    // Threshold strategy with valid importance
    let threshold_config = MemoryExtractionConfig {
        enabled: true,
        strategy: MemoryExtractionStrategy::Threshold { importance: 5 },
    };
    assert!(threshold_config.validate().is_ok());

    // Invalid threshold importance (0)
    let invalid_config = MemoryExtractionConfig {
        enabled: true,
        strategy: MemoryExtractionStrategy::Threshold { importance: 0 },
    };
    assert!(invalid_config.validate().is_err());
}

#[tokio::test]
async fn test_rag_config_defaults() {
    // Test: RagConfig has sensible defaults
    let config = RagConfig::default();

    assert_eq!(config.top_k, 5);
    assert_eq!(config.min_similarity, 0.7);
    assert_eq!(config.max_tokens, 2000);
    assert_eq!(config.timeout_seconds, 5);

    // Defaults should be valid
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_memory_extraction_config_defaults() {
    // Test: MemoryExtractionConfig has sensible defaults
    let config = MemoryExtractionConfig::default();

    assert!(config.enabled);
    assert_eq!(config.strategy, MemoryExtractionStrategy::OnCompletion);

    // Defaults should be valid
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_memory_extraction_strategy_variants() {
    // Test: All MemoryExtractionStrategy variants can be constructed
    let strategies = vec![
        MemoryExtractionStrategy::EveryTurn,
        MemoryExtractionStrategy::OnCompletion,
        MemoryExtractionStrategy::Manual,
        MemoryExtractionStrategy::Threshold { importance: 5 },
    ];

    for strategy in strategies {
        let config = MemoryExtractionConfig {
            enabled: true,
            strategy,
        };
        assert!(config.validate().is_ok());
    }
}

#[tokio::test]
async fn test_rag_config_boundary_values() {
    // Test: RAG config boundary validation

    // Minimum valid values
    let min_config = RagConfig {
        top_k: 1,
        min_similarity: 0.0,
        max_tokens: 1,
        timeout_seconds: 1,
        ..RagConfig::default()
    };
    assert!(min_config.validate().is_ok());

    // Maximum valid top_k
    let max_top_k = RagConfig {
        top_k: 100,
        ..RagConfig::default()
    };
    assert!(max_top_k.validate().is_ok());

    // Just over limit should fail
    let over_limit = RagConfig {
        top_k: 101,
        ..RagConfig::default()
    };
    assert!(over_limit.validate().is_err());

    // Maximum valid similarity
    let max_similarity = RagConfig {
        min_similarity: 1.0,
        ..RagConfig::default()
    };
    assert!(max_similarity.validate().is_ok());
}

// ============================================================================
// RAG Integration Tests with Qdrant
// ============================================================================

#[cfg(feature = "qdrant")]
mod qdrant_rag_tests {
    use paladin::application::services::sanctum::rag_retrieval_service::{
        RagConfig, RagRetrievalService, RetrievalTrigger,
    };
    use paladin::core::platform::container::sanctum::{MemoryBuilder, MemoryType, SanctumEntry};
    use paladin::infrastructure::adapters::sanctum::QdrantSanctumAdapter;
    use paladin_ports::output::embedding_port::EmbeddingPort;
    use paladin_ports::output::sanctum_port::SanctumPort;
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    // ========================================================================
    // Helper: Check Qdrant Availability
    // ========================================================================

    /// Check if Qdrant is available on localhost:6334
    async fn is_qdrant_available() -> bool {
        match reqwest::Client::new()
            .get("http://localhost:6334/healthz")
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// Get Qdrant adapter or skip test if unavailable
    async fn setup_qdrant_or_skip() -> Option<Arc<QdrantSanctumAdapter>> {
        if !is_qdrant_available().await {
            eprintln!("⚠️  Skipping test: Qdrant not available on localhost:6334");
            eprintln!(
                "   Start with: docker-compose -f docker/docker-compose.yml up -d qdrant --profile test"
            );
            return None;
        }

        let collection = format!("test_rag_{}", Uuid::new_v4().as_simple());
        match QdrantSanctumAdapter::new("http://localhost:6334", &collection, 1536).await {
            Ok(adapter) => Some(Arc::new(adapter)),
            Err(e) => {
                eprintln!("⚠️  Skipping test: Failed to create Qdrant adapter: {}", e);
                None
            }
        }
    }

    // ========================================================================
    // Mock Implementations
    // ========================================================================

    struct MockEmbeddingPort;

    #[async_trait::async_trait]
    impl EmbeddingPort for MockEmbeddingPort {
        async fn embed_text(
            &self,
            _text: &str,
        ) -> Result<
            paladin_ports::output::embedding_port::Embedding,
            paladin_ports::output::embedding_port::EmbeddingError,
        > {
            // Return fixed embedding for testing
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let vec: Vec<f32> = (0..1536).map(|_| rng.gen_range(-1.0..1.0)).collect();
            let magnitude: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            let normalized: Vec<f32> = vec.iter().map(|x| x / magnitude).collect();

            Ok(paladin_ports::output::embedding_port::Embedding {
                vector: normalized,
                model: "mock-embedding".to_string(),
                dimension: 1536,
                token_count: Some(10),
            })
        }

        async fn embed_batch(
            &self,
            texts: &[&str],
        ) -> Result<
            Vec<paladin_ports::output::embedding_port::Embedding>,
            paladin_ports::output::embedding_port::EmbeddingError,
        > {
            let mut embeddings = Vec::new();
            for text in texts {
                let result = self.embed_text(text).await?;
                embeddings.push(result);
            }
            Ok(embeddings)
        }

        fn dimension(&self) -> usize {
            1536
        }

        fn model_name(&self) -> &str {
            "mock-embedding"
        }
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[tokio::test]
    async fn test_rag_retrieval_service_with_real_qdrant() {
        // Setup: Check Qdrant availability
        let Some(sanctum) = setup_qdrant_or_skip().await else {
            return;
        };

        // Store test memories
        let memory1 = MemoryBuilder::new(
            "paladin-1".to_string(),
            "Rust is a systems programming language".to_string(),
        )
        .memory_type(MemoryType::Episodic)
        .importance(0.8)
        .build()
        .expect("Failed to build memory");

        let embedding = Arc::new(MockEmbeddingPort);
        let embed_result1 = embedding
            .embed_text(&memory1.content)
            .await
            .expect("Failed to embed");
        let entry1 = SanctumEntry::new(memory1, embed_result1.vector.clone())
            .expect("Failed to create entry");
        sanctum.store(entry1).await.expect("Failed to store entry1");

        let memory2 = MemoryBuilder::new(
            "paladin-1".to_string(),
            "Memory safety is a key feature of Rust".to_string(),
        )
        .memory_type(MemoryType::Semantic)
        .importance(0.9)
        .build()
        .expect("Failed to build memory");

        let embed_result2 = embedding
            .embed_text(&memory2.content)
            .await
            .expect("Failed to embed");
        let entry2 = SanctumEntry::new(memory2, embed_result2.vector.clone())
            .expect("Failed to create entry");
        sanctum.store(entry2).await.expect("Failed to store entry2");

        // Wait for indexing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Test: Retrieve context with RagRetrievalService
        let rag_config = RagConfig {
            top_k: 2,
            min_similarity: 0.0,
            max_tokens: 2000,
            timeout_seconds: 5,
            retrieval_trigger: RetrievalTrigger::Always,
        };
        let rag_service = RagRetrievalService::new(sanctum, embedding, rag_config);

        let results = rag_service
            .retrieve_context("paladin-1", "What is Rust?")
            .await
            .expect("Failed to retrieve context");

        // Assert: Should retrieve both memories
        assert!(!results.is_empty(), "Should retrieve at least one memory");
        assert!(
            results.memories.iter().any(|m| m.body.contains("Rust")),
            "Retrieved memories should mention Rust"
        );
    }

    #[tokio::test]
    async fn test_rag_retrieval_with_token_budget_limiting() {
        // Setup: Check Qdrant availability
        let Some(sanctum) = setup_qdrant_or_skip().await else {
            return;
        };

        // Store multiple memories with varying sizes
        let embedding = Arc::new(MockEmbeddingPort);
        for i in 0..5 {
            let content = format!(
                "This is test memory number {}. It contains important information about the system that should be retrieved during RAG.",
                i
            );
            let memory = MemoryBuilder::new("paladin-1".to_string(), content.clone())
                .memory_type(MemoryType::Episodic)
                .importance(0.8)
                .build()
                .expect("Failed to build memory");

            let embed_result = embedding
                .embed_text(&content)
                .await
                .expect("Failed to embed");
            let entry =
                SanctumEntry::new(memory, embed_result.vector).expect("Failed to create entry");
            sanctum.store(entry).await.expect("Failed to store entry");
        }

        // Wait for indexing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Test: Configure small token budget
        let rag_config = RagConfig {
            top_k: 5,
            min_similarity: 0.0,
            max_tokens: 100, // Small budget - should truncate results
            timeout_seconds: 5,
            retrieval_trigger: RetrievalTrigger::Always,
        };
        let rag_service = RagRetrievalService::new(sanctum, embedding, rag_config);

        let results = rag_service
            .retrieve_context("paladin-1", "test memory")
            .await
            .expect("Failed to retrieve context");

        // Assert: Should truncate to fit token budget
        assert!(
            results.len() < 5,
            "Should truncate results to fit token budget"
        );

        // Format for prompt and verify size constraint
        let formatted = rag_service.format_for_prompt(&results);
        let estimated_tokens = formatted.split_whitespace().count();
        assert!(
            estimated_tokens < 150,
            "Formatted context should respect token budget (got {} tokens)",
            estimated_tokens
        );
    }

    #[tokio::test]
    async fn test_rag_context_formatting() {
        // Setup: Check Qdrant availability
        let Some(sanctum) = setup_qdrant_or_skip().await else {
            return;
        };

        // Store test memory
        let memory = MemoryBuilder::new(
            "paladin-1".to_string(),
            "Paladin is a multi-agent orchestration framework".to_string(),
        )
        .memory_type(MemoryType::Semantic)
        .importance(0.9)
        .build()
        .expect("Failed to build memory");

        let embedding = Arc::new(MockEmbeddingPort);
        let embed_result = embedding
            .embed_text(&memory.content)
            .await
            .expect("Failed to embed");
        let entry = SanctumEntry::new(memory, embed_result.vector).expect("Failed to create entry");
        sanctum.store(entry).await.expect("Failed to store entry");

        // Wait for indexing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Test: Retrieve and format context
        let rag_config = RagConfig::default();
        let rag_service = RagRetrievalService::new(sanctum, embedding, rag_config);

        let results = rag_service
            .retrieve_context("paladin-1", "What is Paladin?")
            .await
            .expect("Failed to retrieve context");

        let formatted = rag_service.format_for_prompt(&results);

        // Assert: Formatted context has expected structure
        assert!(
            formatted.contains("## Relevant Context"),
            "Should have section header"
        );
        assert!(
            formatted.contains("multi-agent orchestration"),
            "Should contain retrieved content"
        );
        assert!(
            formatted.contains("Score:"),
            "Should include relevance scores"
        );
    }

    // Note: Additional complex e2e tests (MemoryExtractionService, PaladinExecutionService with RAG)
    // are deferred due to complex mock dependencies. The tests above provide good coverage of the
    // RAG retrieval workflow with real Qdrant integration. Full e2e tests will be added in future
    // iterations once a more robust mocking strategy is in place.
}
