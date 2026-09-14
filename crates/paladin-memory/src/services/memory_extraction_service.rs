/// Memory Extraction Service
///
/// Extracts meaningful memories from conversation history and stores them
/// in long-term memory (Sanctum) for later retrieval via RAG.
///
/// This service:
/// - Analyzes conversation history using LLM
/// - Identifies important facts, preferences, and context
/// - Generates embeddings for semantic search
/// - Detects and prevents duplicate memories
/// - Stores memories with proper metadata
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use paladin_core::platform::container::garrison::GarrisonEntry;
use paladin_core::platform::container::prompt::{PromptItem, PromptRole, PromptType, TextPrompt};
use paladin_core::platform::container::sanctum::{MemoryBuilder, MemoryType, SanctumEntry};
use paladin_ports::output::embedding_port::EmbeddingPort;
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use paladin_ports::output::sanctum_port::{SanctumError, SanctumFilter, SanctumPort, SanctumQuery};
use serde_json::Value;

/// Strategy for when to extract memories from conversations.
// MemoryExtractionStrategy moved to crates/paladin-memory/src/config/rag.rs (Task 6.0)
pub use crate::config::rag::MemoryExtractionStrategy;

/// Intermediate representation of extracted memory before storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub content: String,
    pub memory_type: MemoryType,
    pub importance: f32,
    pub metadata: HashMap<String, String>,
}

/// Memory Extraction Service.
///
/// Coordinates LLM-based memory extraction and storage via [`SanctumPort`].
/// Depends only on port traits — contains no concrete adapter references.
///
/// # Examples
///
/// ```
/// use paladin_memory::services::memory_extraction_service::MemoryExtractionService;
/// use paladin_ports::output::embedding_port::EmbeddingPort;
/// use paladin_ports::output::llm_port::LlmPort;
/// use paladin_ports::output::sanctum_port::SanctumPort;
/// use std::sync::Arc;
///
/// fn build(
///     llm: Arc<dyn LlmPort>,
///     embedding: Arc<dyn EmbeddingPort>,
///     sanctum: Arc<dyn SanctumPort>,
/// ) -> MemoryExtractionService {
///     MemoryExtractionService::new(llm, embedding, sanctum)
/// }
/// ```
pub struct MemoryExtractionService {
    llm: Arc<dyn LlmPort>,
    embedding: Arc<dyn EmbeddingPort>,
    sanctum: Arc<dyn SanctumPort>,
}

impl MemoryExtractionService {
    /// Create a new memory extraction service.
    pub fn new(
        llm: Arc<dyn LlmPort>,
        embedding: Arc<dyn EmbeddingPort>,
        sanctum: Arc<dyn SanctumPort>,
    ) -> Self {
        Self {
            llm,
            embedding,
            sanctum,
        }
    }

    /// Extract memories from conversation history.
    ///
    /// Analyzes the conversation, extracts important information, and stores it
    /// in long-term memory (Sanctum) for later retrieval.
    pub async fn extract_memories(
        &self,
        paladin_id: &str,
        conversation: &[GarrisonEntry],
    ) -> Result<Vec<SanctumEntry>, SanctumError> {
        let start = std::time::Instant::now();

        if conversation.is_empty() {
            log::debug!("No conversation history to extract memories from");
            return Ok(Vec::new());
        }

        log::info!(
            "Extracting memories for paladin={}, turns={}",
            paladin_id,
            conversation.len()
        );

        // Build extraction prompt
        let prompt = self.build_extraction_prompt(conversation);

        // Call LLM to extract memories
        let prompt_item = PromptItem::new(PromptType::Text(TextPrompt {
            content: prompt,
            role: PromptRole::User,
        }))
        .map_err(|e| SanctumError::StorageError(format!("Failed to create prompt: {}", e)))?;

        let request = LlmRequest::new("gpt-4", prompt_item);

        let response = match self.llm.generate(request).await {
            Ok(resp) => resp.content,
            Err(e) => {
                log::warn!(
                    "Memory extraction LLM call failed: {}, continuing without extraction",
                    e
                );
                return Ok(Vec::new());
            }
        };

        // Parse LLM response to get extracted memories
        let extracted = match self.parse_extraction_response(&response) {
            Ok(memories) => memories,
            Err(e) => {
                log::warn!("Failed to parse extraction response: {}, continuing", e);
                return Ok(Vec::new());
            }
        };

        if extracted.is_empty() {
            log::debug!("No memories extracted from conversation");
            return Ok(Vec::new());
        }

        let extracted_count = extracted.len();
        log::debug!("Extracted {} potential memories", extracted_count);

        // Convert to Memory objects and generate embeddings
        let mut memories_to_store = Vec::new();
        for ext_mem in extracted {
            // Generate embedding
            let embedding = match self.embedding.embed_text(&ext_mem.content).await {
                Ok(emb) => emb,
                Err(e) => {
                    log::warn!("Failed to generate embedding for memory: {}, skipping", e);
                    continue;
                }
            };

            // Check for duplicates
            if self
                .check_for_duplicates(paladin_id, &embedding.vector)
                .await?
            {
                log::debug!(
                    "Duplicate memory detected, skipping: {:?}",
                    &ext_mem.content[..50.min(ext_mem.content.len())]
                );
                continue;
            }

            // Convert HashMap<String, String> to HashMap<String, Value>
            let metadata_values: HashMap<String, Value> = ext_mem
                .metadata
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect();

            // Create Memory object
            let memory = MemoryBuilder::new(paladin_id.to_string(), ext_mem.content)
                .memory_type(ext_mem.memory_type)
                .importance(ext_mem.importance)
                .metadata(metadata_values)
                .build()
                .map_err(|e| SanctumError::StorageError(e.to_string()))?;

            // Create SanctumEntry
            let entry = SanctumEntry::new(memory, embedding.vector)
                .map_err(|e| SanctumError::StorageError(e.to_string()))?;

            memories_to_store.push(entry);
        }

        // Store memories in Sanctum
        let stored = self.store_memories(paladin_id, &memories_to_store).await?;

        let duration = start.elapsed();
        let avg_importance = if !stored.is_empty() {
            stored.iter().map(|e| e.memory.importance).sum::<f32>() / stored.len() as f32
        } else {
            0.0
        };

        log::info!(
            "Memory extraction complete: paladin={}, extracted={}, stored={}, avg_importance={:.2}, duration_ms={}",
            paladin_id,
            extracted_count,
            stored.len(),
            avg_importance,
            duration.as_millis()
        );

        Ok(stored)
    }

    /// Build extraction prompt from conversation history.
    fn build_extraction_prompt(&self, conversation: &[GarrisonEntry]) -> String {
        let mut prompt = String::from(EXTRACTION_PROMPT);
        prompt.push_str("\n\nConversation:\n");

        for entry in conversation {
            prompt.push_str(&format!("{:?}: {}\n", entry.role, entry.content));
        }

        prompt.push_str("\n\nExtract important memories as JSON array:");
        prompt
    }

    /// Parse LLM extraction response into structured memories.
    fn parse_extraction_response(&self, response: &str) -> Result<Vec<ExtractedMemory>, String> {
        // Try to extract JSON from response (might be wrapped in markdown)
        let json_str = if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        serde_json::from_str::<Vec<ExtractedMemory>>(json_str)
            .map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    /// Check if a similar memory already exists (>0.95 similarity).
    async fn check_for_duplicates(
        &self,
        paladin_id: &str,
        embedding: &[f32],
    ) -> Result<bool, SanctumError> {
        let query = SanctumQuery::new(embedding.to_vec(), 1)
            .with_filter(SanctumFilter::new().paladin_id(paladin_id.to_string()))
            .with_min_score(0.95);

        let results = self.sanctum.search(query).await?;
        Ok(!results.is_empty())
    }

    /// Store memories in batch via Sanctum.
    async fn store_memories(
        &self,
        _paladin_id: &str,
        memories: &[SanctumEntry],
    ) -> Result<Vec<SanctumEntry>, SanctumError> {
        let mut stored: Vec<SanctumEntry> = Vec::new();

        for entry in memories {
            match self.sanctum.store(entry.clone()).await {
                Ok(_) => {
                    stored.push(entry.clone());
                }
                Err(e) => {
                    log::warn!("Failed to store memory: {}, continuing", e);
                }
            }
        }

        Ok(stored)
    }
}

/// LLM prompt template for memory extraction.
const EXTRACTION_PROMPT: &str = r#"You are a memory extraction assistant. Analyze the following conversation and extract important memories.

For each memory, provide:
1. content: The actual information to remember (be specific and complete)
2. memory_type: One of "Episodic", "Semantic", "Procedural"
3. importance: 0.0 to 1.0 score indicating how important this information is
4. metadata: Optional key-value pairs for additional context (as an object)

Memory Types:
- Episodic: Specific events, conversations, or experiences
- Semantic: Facts, knowledge, preferences, and general information
- Procedural: How-to instructions, procedures, and workflows

Rules:
- Extract only genuinely important information worth remembering long-term
- Be specific and include relevant details
- Avoid extracting trivial or transient information
- Combine related information into single memories when appropriate
- Use proper memory types

Return ONLY a JSON array of memories, no additional text."#;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use paladin_core::platform::container::garrison::ConversationRole;
    use paladin_core::platform::container::sanctum::MemoryBuilder;
    use paladin_ports::output::embedding_port::{Embedding, EmbeddingError, EmbeddingPort};
    use paladin_ports::output::llm_port::{
        FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities,
        StreamingResponse, TokenUsage,
    };
    use paladin_ports::output::sanctum_port::{
        SanctumError, SanctumFilter, SanctumPort, SanctumQuery, SanctumSearchResult,
    };
    use std::sync::Arc;

    // ── Strategy / serialization tests ────────────────────────────────────────

    #[test]
    fn test_extraction_strategy_default() {
        assert_eq!(
            MemoryExtractionStrategy::default(),
            MemoryExtractionStrategy::OnCompletion
        );
    }

    #[test]
    fn test_extraction_strategy_equality() {
        assert_eq!(
            MemoryExtractionStrategy::EveryTurn,
            MemoryExtractionStrategy::EveryTurn
        );
        assert_ne!(
            MemoryExtractionStrategy::EveryTurn,
            MemoryExtractionStrategy::OnCompletion
        );
    }

    #[test]
    fn test_extraction_strategy_threshold() {
        let strategy = MemoryExtractionStrategy::Threshold { importance: 7 };
        if let MemoryExtractionStrategy::Threshold { importance } = strategy {
            assert_eq!(importance, 7);
        } else {
            panic!("Expected Threshold variant");
        }
    }

    #[test]
    fn test_extracted_memory_serialization() {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "conversation".to_string());

        let memory = ExtractedMemory {
            content: "User prefers dark mode".to_string(),
            memory_type: MemoryType::Semantic,
            importance: 0.8,
            metadata,
        };

        let json = serde_json::to_string(&memory).unwrap();
        let deserialized: ExtractedMemory = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.content, memory.content);
        assert_eq!(deserialized.importance, memory.importance);
    }

    #[test]
    fn test_extraction_strategy_serialization() {
        let strategy = MemoryExtractionStrategy::Threshold { importance: 8 };
        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: MemoryExtractionStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, deserialized);
    }

    // ── Mock helpers ──────────────────────────────────────────────────────────

    struct MockLlmPort {
        response: String,
        should_fail: bool,
    }

    #[async_trait]
    impl LlmPort for MockLlmPort {
        async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
            if self.should_fail {
                Err(LlmError::ProcessingError("Mock LLM failure".to_string()))
            } else {
                Ok(LlmResponse {
                    id: uuid::Uuid::new_v4(),
                    request_id: request.id,
                    model: "mock-model".to_string(),
                    content: self.response.clone(),
                    finish_reason: FinishReason::Stop,
                    usage: TokenUsage {
                        prompt_tokens: 10,
                        completion_tokens: 20,
                        total_tokens: 30,
                    },
                    created_at: chrono::Utc::now(),
                    metadata: HashMap::new(),
                    function_call: None,
                })
            }
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            Err(LlmError::ProcessingError(
                "Streaming not supported in mock".to_string(),
            ))
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec!["mock-model".to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            "mock-provider"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    struct MockEmbeddingPort {
        dimension: usize,
        should_fail: bool,
    }

    #[async_trait]
    impl EmbeddingPort for MockEmbeddingPort {
        async fn embed_text(&self, _text: &str) -> Result<Embedding, EmbeddingError> {
            if self.should_fail {
                Err(EmbeddingError::NetworkError(
                    "Mock embedding failure".to_string(),
                ))
            } else {
                Ok(Embedding {
                    vector: vec![0.1; self.dimension],
                    model: "mock-embedding-model".to_string(),
                    dimension: self.dimension,
                    token_count: None,
                })
            }
        }

        async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
            if self.should_fail {
                Err(EmbeddingError::NetworkError(
                    "Mock embedding failure".to_string(),
                ))
            } else {
                Ok(texts
                    .iter()
                    .map(|_| Embedding {
                        vector: vec![0.1; self.dimension],
                        model: "mock-embedding-model".to_string(),
                        dimension: self.dimension,
                        token_count: None,
                    })
                    .collect())
            }
        }

        fn dimension(&self) -> usize {
            self.dimension
        }

        fn model_name(&self) -> &str {
            "mock-embedding-model"
        }
    }

    struct MockSanctumPort {
        stored_entries: std::sync::Mutex<Vec<SanctumEntry>>,
        should_fail_store: bool,
        duplicate_threshold: f32,
    }

    impl MockSanctumPort {
        fn new() -> Self {
            Self {
                stored_entries: std::sync::Mutex::new(Vec::new()),
                should_fail_store: false,
                // Default: threshold above 1.0 → duplicates never triggered
                duplicate_threshold: 1.1,
            }
        }

        fn with_duplicate_threshold(mut self, threshold: f32) -> Self {
            self.duplicate_threshold = threshold;
            self
        }
    }

    #[async_trait]
    impl SanctumPort for MockSanctumPort {
        async fn store(&self, entry: SanctumEntry) -> Result<(), SanctumError> {
            if self.should_fail_store {
                Err(SanctumError::StorageError(
                    "Mock storage failure".to_string(),
                ))
            } else {
                self.stored_entries.lock().unwrap().push(entry);
                Ok(())
            }
        }

        async fn store_batch(&self, entries: Vec<SanctumEntry>) -> Result<(), SanctumError> {
            for entry in entries {
                self.store(entry).await?;
            }
            Ok(())
        }

        async fn search(
            &self,
            query: SanctumQuery,
        ) -> Result<Vec<SanctumSearchResult>, SanctumError> {
            if query.min_score.unwrap_or(0.0) >= self.duplicate_threshold {
                let mock_memory = MemoryBuilder::new(
                    "test-paladin".to_string(),
                    "Existing duplicate memory".to_string(),
                )
                .memory_type(MemoryType::Semantic)
                .build()
                .unwrap();
                let mock_entry = SanctumEntry::new(mock_memory, vec![0.1; 1536]).unwrap();
                Ok(vec![SanctumSearchResult::new(mock_entry, 0.96)])
            } else {
                Ok(Vec::new())
            }
        }

        async fn delete(&self, _id: &str) -> Result<bool, SanctumError> {
            Ok(true)
        }

        async fn update(&self, _entry: SanctumEntry) -> Result<(), SanctumError> {
            Ok(())
        }

        async fn count(&self, _filter: Option<SanctumFilter>) -> Result<usize, SanctumError> {
            Ok(self.stored_entries.lock().unwrap().len())
        }
    }

    // ── Async service tests ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_successful_extraction_with_multiple_memory_types() {
        let llm_response = r#"[
            {
                "content": "User prefers dark mode in all applications",
                "memory_type": "Semantic",
                "importance": 0.8,
                "metadata": {"category": "ui"}
            },
            {
                "content": "User is learning Rust programming language",
                "memory_type": "Semantic",
                "importance": 0.9,
                "metadata": {"topic": "programming"}
            },
            {
                "content": "User wants to build a web application",
                "memory_type": "Episodic",
                "importance": 0.85,
                "metadata": {}
            }
        ]"#;

        let llm = Arc::new(MockLlmPort {
            response: llm_response.to_string(),
            should_fail: false,
        });
        let embedding = Arc::new(MockEmbeddingPort {
            dimension: 1536,
            should_fail: false,
        });
        let sanctum = Arc::new(MockSanctumPort::new());

        let service = MemoryExtractionService::new(llm, embedding, sanctum.clone());

        let conversation = vec![
            GarrisonEntry::new(ConversationRole::User, "I prefer dark mode".to_string()),
            GarrisonEntry::new(
                ConversationRole::Assistant,
                "Noted! I'll remember your preference".to_string(),
            ),
        ];

        let result = service
            .extract_memories("test-paladin", &conversation)
            .await
            .unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(sanctum.stored_entries.lock().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn test_importance_scoring_correctly_assigned() {
        let llm_response = r#"[
            {
                "content": "High importance memory",
                "memory_type": "Semantic",
                "importance": 0.95,
                "metadata": {}
            },
            {
                "content": "Low importance memory",
                "memory_type": "Episodic",
                "importance": 0.3,
                "metadata": {}
            }
        ]"#;

        let llm = Arc::new(MockLlmPort {
            response: llm_response.to_string(),
            should_fail: false,
        });
        let embedding = Arc::new(MockEmbeddingPort {
            dimension: 1536,
            should_fail: false,
        });
        let sanctum = Arc::new(MockSanctumPort::new());

        let service = MemoryExtractionService::new(llm, embedding, sanctum.clone());

        let conversation = vec![GarrisonEntry::new(
            ConversationRole::User,
            "Test content".to_string(),
        )];

        let result = service
            .extract_memories("test-paladin", &conversation)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].memory.importance, 0.95);
        assert_eq!(result[1].memory.importance, 0.3);
    }

    #[tokio::test]
    async fn test_duplicate_detection_prevents_restorage() {
        let llm_response = r#"[
            {
                "content": "Duplicate memory content",
                "memory_type": "Semantic",
                "importance": 0.8,
                "metadata": {}
            }
        ]"#;

        let llm = Arc::new(MockLlmPort {
            response: llm_response.to_string(),
            should_fail: false,
        });
        let embedding = Arc::new(MockEmbeddingPort {
            dimension: 1536,
            should_fail: false,
        });
        let sanctum = Arc::new(MockSanctumPort::new().with_duplicate_threshold(0.95));

        let service = MemoryExtractionService::new(llm, embedding, sanctum.clone());

        let conversation = vec![GarrisonEntry::new(
            ConversationRole::User,
            "Duplicate content".to_string(),
        )];

        let result = service
            .extract_memories("test-paladin", &conversation)
            .await
            .unwrap();

        assert_eq!(result.len(), 0);
        assert_eq!(sanctum.stored_entries.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_llm_failure_handled_gracefully() {
        let llm = Arc::new(MockLlmPort {
            response: String::new(),
            should_fail: true,
        });
        let embedding = Arc::new(MockEmbeddingPort {
            dimension: 1536,
            should_fail: false,
        });
        let sanctum = Arc::new(MockSanctumPort::new());

        let service = MemoryExtractionService::new(llm, embedding, sanctum);

        let conversation = vec![GarrisonEntry::new(
            ConversationRole::User,
            "Test".to_string(),
        )];

        let result = service
            .extract_memories("test-paladin", &conversation)
            .await
            .unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_empty_conversation_returns_empty() {
        let llm = Arc::new(MockLlmPort {
            response: String::new(),
            should_fail: false,
        });
        let embedding = Arc::new(MockEmbeddingPort {
            dimension: 1536,
            should_fail: false,
        });
        let sanctum = Arc::new(MockSanctumPort::new());

        let service = MemoryExtractionService::new(llm, embedding, sanctum);

        let result = service.extract_memories("test-paladin", &[]).await.unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_malformed_json_response_handled() {
        let llm = Arc::new(MockLlmPort {
            response: "This is not valid JSON".to_string(),
            should_fail: false,
        });
        let embedding = Arc::new(MockEmbeddingPort {
            dimension: 1536,
            should_fail: false,
        });
        let sanctum = Arc::new(MockSanctumPort::new());

        let service = MemoryExtractionService::new(llm, embedding, sanctum);

        let conversation = vec![GarrisonEntry::new(
            ConversationRole::User,
            "Test".to_string(),
        )];

        let result = service
            .extract_memories("test-paladin", &conversation)
            .await
            .unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_embedding_failure_skips_memory() {
        let llm_response = r#"[
            {
                "content": "Test memory",
                "memory_type": "Semantic",
                "importance": 0.8,
                "metadata": {}
            }
        ]"#;

        let llm = Arc::new(MockLlmPort {
            response: llm_response.to_string(),
            should_fail: false,
        });
        let embedding = Arc::new(MockEmbeddingPort {
            dimension: 1536,
            should_fail: true,
        });
        let sanctum = Arc::new(MockSanctumPort::new());

        let service = MemoryExtractionService::new(llm, embedding, sanctum);

        let conversation = vec![GarrisonEntry::new(
            ConversationRole::User,
            "Test".to_string(),
        )];

        let result = service
            .extract_memories("test-paladin", &conversation)
            .await
            .unwrap();

        assert_eq!(result.len(), 0);
    }
}
