//! # Garrison Port - Conversation Memory Operations Interface
//!
//! Port trait defining how the application interacts with conversation memory storage (Garrison).
//!
//! ## Purpose
//!
//! The Garrison Port provides a unified abstraction for storing and retrieving conversation
//! history and context. It enables Paladin agents to maintain memory across interactions,
//! learn from past conversations, and provide contextually aware responses. The port supports:
//!
//! - **Short-Term Memory**: Recent conversation history (CRUD operations)
//! - **Semantic Search**: Text-based search across historical conversations
//! - **Long-Term Memory**: Vector embedding-based semantic similarity search (optional)
//! - **Statistics**: Token counts and storage metrics for memory management
//!
//! By decoupling memory operations from specific storage backends, Garrison Port enables:
//! - Switching between in-memory, database, or distributed storage
//! - Testing with mock implementations
//! - Optimizing for different use cases (speed vs. persistence vs. scale)
//!
//! ## Hexagonal Architecture
//!
//! This is an **output port** in the application layer. It defines the interface for memory
//! operations, allowing Paladin agents to maintain conversation context without depending on
//! specific storage implementations (SQL databases, NoSQL stores, in-memory caches).
//!
//! **Adapter Implementations:**
//! - `InMemoryGarrison` - Fast, volatile memory for development/testing
//! - `SqliteGarrison` - Persistent file-based storage for single-agent deployments
//! - `PostgresGarrison` - Scalable database storage for production multi-agent systems
//! - `RedisGarrison` - Distributed cache for high-performance, multi-instance deployments
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync` to support concurrent async operations.
//! Multiple Paladin agents may access the same Garrison simultaneously. Implementations
//! should handle concurrent reads/writes safely.
//!
//! ## Error Handling
//!
//! Operations return `Result<T, GarrisonError>` with specific error variants for:
//! - Storage failures (transient or permanent)
//! - Serialization issues (data format problems)
//! - Tokenization errors (token counting failures)
//! - Not found errors (missing entries)
//!
//! See [`GarrisonError`] for all error categories and handling strategies.
//!
//! ## Traits
//!
//! - [`GarrisonPort`]: Basic memory operations (CRUD, search, stats)
//! - [`LongTermGarrisonPort`]: Extended operations with vector embeddings for semantic search
//!
//! ## Examples
//!
//! ### Basic Usage
//!
//! ```rust,no_run
//! use paladin_ports::output::garrison_port::GarrisonPort;
//! use paladin_core::platform::container::garrison::{GarrisonEntry, ConversationRole};
//!
//! async fn conversation_memory(garrison: &dyn GarrisonPort) -> Result<(), Box<dyn std::error::Error>> {
//!     // Store user message
//!     let user_msg = GarrisonEntry::new(
//!         ConversationRole::User,
//!         "What is the capital of France?".to_string()
//!     );
//!     garrison.remember(user_msg).await?;
//!
//!     // Store assistant response
//!     let assistant_msg = GarrisonEntry::new(
//!         ConversationRole::Assistant,
//!         "The capital of France is Paris.".to_string()
//!     );
//!     garrison.remember(assistant_msg).await?;
//!
//!     // Recall recent conversation
//!     let recent = garrison.recall_recent(10).await?;
//!     println!("Last {} messages:", recent.len());
//!     for entry in recent {
//!         println!("{:?}: {}", entry.role, entry.content);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Search and Statistics
//!
//! ```rust,no_run
//! use paladin_ports::output::garrison_port::GarrisonPort;
//!
//! async fn search_history(garrison: &dyn GarrisonPort) -> Result<(), Box<dyn std::error::Error>> {
//!     // Search for specific topics
//!     let results = garrison.search("machine learning", 5).await?;
//!     println!("Found {} messages about machine learning", results.len());
//!
//!     // Check memory usage
//!     let stats = garrison.stats().await?;
//!     println!("Garrison stats:");
//!     println!("  Entries: {}", stats.entry_count);
//!     println!("  Tokens: {}", stats.total_tokens);
//!     if let Some(size) = stats.size_bytes {
//!         println!("  Size: {} KB", size / 1024);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Custom Implementation
//!
//! ```rust,no_run
//! use paladin_ports::output::garrison_port::{GarrisonPort, GarrisonError, GarrisonStats};
//! use paladin_core::platform::container::garrison::GarrisonEntry;
//! use async_trait::async_trait;
//! use std::sync::{Arc, Mutex};
//!
//! struct CustomGarrison {
//!     entries: Arc<Mutex<Vec<GarrisonEntry>>>,
//! }
//!
//! #[async_trait]
//! impl GarrisonPort for CustomGarrison {
//!     async fn remember(&self, entry: GarrisonEntry) -> Result<(), GarrisonError> {
//!         self.entries.lock().unwrap().push(entry);
//!         Ok(())
//!     }
//!
//!     async fn recall_recent(&self, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError> {
//!         let entries = self.entries.lock().unwrap();
//!         let start = entries.len().saturating_sub(limit);
//!         Ok(entries[start..].to_vec())
//!     }
//!
//!     async fn search(&self, query: &str, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError> {
//!         let entries = self.entries.lock().unwrap();
//!         let results: Vec<_> = entries
//!             .iter()
//!             .filter(|e| e.content.contains(query))
//!             .take(limit)
//!             .cloned()
//!             .collect();
//!         Ok(results)
//!     }
//!
//!     async fn forget_all(&self) -> Result<(), GarrisonError> {
//!         self.entries.lock().unwrap().clear();
//!         Ok(())
//!     }
//!
//!     async fn stats(&self) -> Result<GarrisonStats, GarrisonError> {
//!         let entries = self.entries.lock().unwrap();
//!         Ok(GarrisonStats {
//!             entry_count: entries.len(),
//!             total_tokens: entries.iter().map(|e| e.content.split_whitespace().count() as u32).sum(),
//!             size_bytes: None,
//!         })
//!     }
//! }
//! ```
//!
//! ## Implementation Notes
//!
//! ### Performance Considerations
//! - **Batch Operations**: Retrieve multiple recent entries in one call rather than individual lookups
//! - **Indexing**: Index content for fast text search (full-text search indexes recommended)
//! - **Pagination**: Use `limit` parameters to avoid loading excessive data
//! - **Caching**: Cache frequently accessed recent entries
//! - **Token Counting**: Pre-calculate and store token counts to avoid re-computation
//!
//! ### Best Practices
//! 1. **Memory Management**: Monitor `stats()` and implement eviction policies (LRU, time-based)
//! 2. **Concurrent Access**: Use appropriate locking/transactions for thread safety
//! 3. **Error Recovery**: Implement retry logic for transient storage errors
//! 4. **Data Retention**: Implement `forget_all()` carefully with confirmation prompts
//! 5. **Search Optimization**: Use specialized search indexes rather than full table scans
//!
//! ### Common Pitfalls
//! - Don't hold locks during async operations (deadlock risk)
//! - Don't store entries without enforcing size limits (memory exhaustion)
//! - Don't use `forget_all()` in production without backup mechanisms
//! - Don't perform token counting synchronously on every insert (performance)
//!
//! ## Related Ports
//!
//! - [`SanctumPort`](crate::output::sanctum_port::SanctumPort) - Long-term persistent memory with vector embeddings (superset of LongTermGarrisonPort)
//! - [`EmbeddingPort`](crate::output::embedding_port::EmbeddingPort) - Generate vector embeddings for semantic search
//! - [`LlmPort`](crate::output::llm_port::LlmPort) - LLM integration (uses Garrison for conversation context)
//! - [`CitadelPort`](crate::output::citadel_port::CitadelPort) - State persistence for entire Paladin agents
//!
//! ## See Also
//!
//! - Application Ports (`crate::application::ports`)
//! - [Garrison Domain](paladin_core::platform::container::garrison)
//! - Infrastructure Adapters (`crate::infrastructure::adapters::garrison`)

use async_trait::async_trait;
use paladin_core::platform::container::garrison::GarrisonEntry;
use serde::{Deserialize, Serialize};

pub use paladin_core::platform::container::garrison_error::GarrisonError;

/// Statistics about a Garrison's current state.
///
/// Provides metrics for monitoring memory usage, token consumption, and storage capacity.
/// Use these statistics to implement eviction policies, display memory status to users,
/// and optimize Paladin agent performance.
///
/// # Fields
///
/// - `entry_count`: Number of conversation entries currently stored
/// - `total_tokens`: Cumulative token count across all entries (for LLM context management)
/// - `size_bytes`: Approximate storage size in bytes (implementation-dependent)
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::garrison_port::GarrisonStats;
///
/// let stats = GarrisonStats {
///     entry_count: 150,
///     total_tokens: 8000,
///     size_bytes: Some(102400), // ~100 KB
/// };
///
/// // Check if approaching token limit
/// const MAX_TOKENS: u32 = 10000;
/// if stats.total_tokens > MAX_TOKENS * 80 / 100 {
///     println!("Warning: Using {}% of token capacity",
///         stats.total_tokens * 100 / MAX_TOKENS);
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarrisonStats {
    /// Total number of entries currently stored
    pub entry_count: usize,
    /// Total tokens across all entries (if tracked)
    pub total_tokens: u32,
    /// Approximate size in bytes (if available)
    pub size_bytes: Option<usize>,
}

// GarrisonError is re-exported from core for API compatibility.

/// Port for basic Garrison memory operations.
///
/// This trait defines the core interface for storing and retrieving conversation
/// history. All Garrison implementations must implement this trait to enable Paladin
/// agents to maintain context across interactions.
///
/// # Capabilities
///
/// - **Storage**: Add new conversation entries with [`remember`](Self::remember)
/// - **Retrieval**: Get recent entries with [`recall_recent`](Self::recall_recent)
/// - **Search**: Find entries by text query with [`search`](Self::search)
/// - **Management**: Clear all entries with [`forget_all`](Self::forget_all)
/// - **Monitoring**: Get storage statistics with [`stats`](Self::stats)
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to support async operations across
/// thread boundaries. Multiple Paladin agents may access the same Garrison concurrently.
///
/// # Implementation Requirements
///
/// Implementations should:
/// 1. Store entries in chronological order
/// 2. Return recent entries from oldest to newest
/// 3. Support concurrent read/write operations safely
/// 4. Handle storage failures gracefully (return appropriate errors)
/// 5. Calculate token counts accurately for context window management
///
/// # Examples
///
/// ## Basic Conversation Storage
///
/// ```rust,no_run
/// use paladin_ports::output::garrison_port::GarrisonPort;
/// use paladin_core::platform::container::garrison::{GarrisonEntry, ConversationRole};
///
/// async fn conversation_flow(garrison: &dyn GarrisonPort) -> Result<(), Box<dyn std::error::Error>> {
///     // Store user input
///     let user_entry = GarrisonEntry::new(
///         ConversationRole::User,
///         "What is Rust?".to_string()
///     );
///     garrison.remember(user_entry).await?;
///
///     // Store assistant response
///     let assistant_entry = GarrisonEntry::new(
///         ConversationRole::Assistant,
///         "Rust is a systems programming language...".to_string()
///     );
///     garrison.remember(assistant_entry).await?;
///
///     // Retrieve for next interaction
///     let history = garrison.recall_recent(10).await?;
///     println!("Context has {} messages", history.len());
///
///     Ok(())
/// }
/// ```
///
/// ## Memory Management
///
/// ```rust,no_run
/// use paladin_ports::output::garrison_port::GarrisonPort;
///
/// async fn manage_memory(garrison: &dyn GarrisonPort) -> Result<(), Box<dyn std::error::Error>> {
///     // Check current usage
///     let stats = garrison.stats().await?;
///     println!("Entries: {}, Tokens: {}", stats.entry_count, stats.total_tokens);
///
///     // Implement eviction policy
///     const MAX_TOKENS: u32 = 8000;
///     if stats.total_tokens > MAX_TOKENS {
///         println!("Approaching token limit, consider clearing old entries");
///         // In production: implement LRU or time-based eviction
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## Search Historical Context
///
/// ```rust,no_run
/// use paladin_ports::output::garrison_port::GarrisonPort;
///
/// async fn search_context(garrison: &dyn GarrisonPort) -> Result<(), Box<dyn std::error::Error>> {
///     // Find past discussions about specific topics
///     let results = garrison.search("deployment", 5).await?;
///
///     if results.is_empty() {
///         println!("No prior discussions about deployment");
///     } else {
///         println!("Found {} relevant messages:", results.len());
///         for entry in results {
///             println!("  - {:?}: {}", entry.role, entry.content.chars().take(50).collect::<String>());
///         }
///     }
///
///     Ok(())
/// }
/// ```
///
/// # Implementation Notes
///
/// ## Performance Optimization
/// - Cache recently accessed entries to reduce storage lookups
/// - Use database indexes on timestamp/role columns for fast `recall_recent()`
/// - Implement full-text search indexes for `search()` queries
/// - Pre-calculate token counts on insert rather than on-demand
///
/// ## Concurrency Patterns
/// ```rust,ignore
/// // Good: Async-safe lock-free design
/// use tokio::sync::RwLock;
/// struct MyGarrison {
///     entries: Arc<RwLock<Vec<GarrisonEntry>>>,
/// }
///
/// // Avoid: Blocking mutex in async context
/// // use std::sync::Mutex; // DON'T do this in async code
/// ```
///
/// ## Error Handling Best Practices
/// - Return `StorageError` for transient failures (database timeout)
/// - Return `SerializationError` for permanent data issues
/// - Log errors before returning for debugging
/// - Implement retry logic in adapter, not in port trait
///
/// # See Also
///
/// - [`LongTermGarrisonPort`] - Extended trait with vector embedding support
/// - [`SanctumPort`](crate::output::sanctum_port::SanctumPort) - Long-term persistent memory (alternative/superset)
/// - [`GarrisonEntry`] - Entry data structure
#[async_trait]
pub trait GarrisonPort: Send + Sync {
    /// Stores a new entry in the Garrison
    ///
    /// # Arguments
    ///
    /// * `entry` - The entry to store
    ///
    /// # Errors
    ///
    /// Returns [`GarrisonError::StorageError`] if the storage operation fails.
    /// Returns [`GarrisonError::SerializationError`] if the entry cannot be serialized.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin_ports::output::garrison_port::GarrisonPort;
    /// # use paladin_core::platform::container::garrison::{GarrisonEntry, ConversationRole};
    /// # async fn example(garrison: &dyn GarrisonPort) {
    /// let entry = GarrisonEntry::new(
    ///     ConversationRole::User,
    ///     "Store this message".to_string()
    /// );
    /// garrison.remember(entry).await.expect("Failed to store entry");
    /// # }
    /// ```
    async fn remember(&self, entry: GarrisonEntry) -> Result<(), GarrisonError>;

    /// Retrieves the N most recent entries
    ///
    /// Returns entries in chronological order (oldest first).
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of entries to retrieve
    ///
    /// # Errors
    ///
    /// Returns [`GarrisonError::StorageError`] if the retrieval fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin_ports::output::garrison_port::GarrisonPort;
    /// # async fn example(garrison: &dyn GarrisonPort) {
    /// let recent = garrison.recall_recent(5).await.expect("Failed to recall");
    /// for entry in recent {
    ///     println!("{:?}: {}", entry.role, entry.content);
    /// }
    /// # }
    /// ```
    async fn recall_recent(&self, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError>;

    /// Searches for entries matching a text query
    ///
    /// The exact search behavior (substring, full-text, etc.) is implementation-specific.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string
    /// * `limit` - Maximum number of results to return
    ///
    /// # Errors
    ///
    /// Returns [`GarrisonError::StorageError`] if the search fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin_ports::output::garrison_port::GarrisonPort;
    /// # async fn example(garrison: &dyn GarrisonPort) {
    /// let results = garrison.search("error", 10).await.expect("Search failed");
    /// println!("Found {} entries containing 'error'", results.len());
    /// # }
    /// ```
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError>;

    /// Clears all entries from the Garrison
    ///
    /// # Errors
    ///
    /// Returns [`GarrisonError::StorageError`] if the clear operation fails.
    ///
    /// # Warning
    ///
    /// This operation is irreversible. All conversation history will be lost.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin_ports::output::garrison_port::GarrisonPort;
    /// # async fn example(garrison: &dyn GarrisonPort) {
    /// garrison.forget_all().await.expect("Failed to clear garrison");
    /// # }
    /// ```
    async fn forget_all(&self) -> Result<(), GarrisonError>;

    /// Returns statistics about the current state of the Garrison
    ///
    /// # Errors
    ///
    /// Returns [`GarrisonError::StorageError`] if statistics cannot be calculated.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin_ports::output::garrison_port::GarrisonPort;
    /// # async fn example(garrison: &dyn GarrisonPort) {
    /// let stats = garrison.stats().await.expect("Failed to get stats");
    /// println!("Entries: {}, Tokens: {}", stats.entry_count, stats.total_tokens);
    /// # }
    /// ```
    async fn stats(&self) -> Result<GarrisonStats, GarrisonError>;
}

/// Extended port for long-term memory with semantic search capabilities.
///
/// This trait extends [`GarrisonPort`] with vector embedding support for semantic
/// similarity search. Use this when you need to find conceptually similar past
/// conversations, not just exact text matches.
///
/// # Capabilities
///
/// Beyond basic [`GarrisonPort`] operations:
/// - **Semantic Storage**: Store entries with vector embeddings
/// - **Similarity Search**: Find entries by semantic similarity (cosine distance)
///
/// # Use Cases
///
/// - **Knowledge Base**: Find related past solutions when user asks new questions
/// - **Context Retrieval**: Pull in relevant historical context based on current topic
/// - **Deduplication**: Detect similar/duplicate queries before processing
/// - **Recommendation**: Suggest related past conversations to users
///
/// # Embedding Models
///
/// This trait is agnostic to embedding model choice. Common options:
/// - OpenAI `text-embedding-3-small` (1536 dimensions)
/// - OpenAI `text-embedding-3-large` (3072 dimensions)
/// - Sentence Transformers (768 dimensions)
/// - Custom fine-tuned models
///
/// **Important**: All entries in a Garrison must use the same embedding model
/// and dimension for accurate similarity comparisons.
///
/// # Thread Safety
///
/// Same requirements as [`GarrisonPort`]: implementations must be `Send + Sync`.
///
/// # Examples
///
/// ## Semantic Context Retrieval
///
/// ```rust,no_run
/// use paladin_ports::output::garrison_port::LongTermGarrisonPort;
/// use paladin_ports::output::embedding_port::EmbeddingPort;
/// use paladin_core::platform::container::garrison::{GarrisonEntry, ConversationRole};
///
/// async fn semantic_context(
///     garrison: &dyn LongTermGarrisonPort,
///     embedder: &dyn EmbeddingPort,
///     query: &str,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     // Generate query embedding
///     let embedding = embedder.embed_text(query).await?;
///
///     // Find semantically similar past conversations
///     let similar = garrison.search_similar(embedding.vector, 5).await?;
///
///     println!("Found {} related discussions:", similar.len());
///     for entry in similar {
///         println!("  {:?}: {}...", entry.role,
///             entry.content.chars().take(60).collect::<String>());
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## Store with Embeddings
///
/// ```rust,no_run
/// use paladin_ports::output::garrison_port::LongTermGarrisonPort;
/// use paladin_ports::output::embedding_port::EmbeddingPort;
/// use paladin_core::platform::container::garrison::{GarrisonEntry, ConversationRole};
///
/// async fn store_with_embedding(
///     garrison: &dyn LongTermGarrisonPort,
///     embedder: &dyn EmbeddingPort,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     let content = "How do I deploy a Rust application to production?";
///
///     // Create entry
///     let entry = GarrisonEntry::new(
///         ConversationRole::User,
///         content.to_string()
///     );
///
///     // Generate embedding
///     let embedding = embedder.embed_text(content).await?;
///
///     // Store with semantic searchability
///     garrison.remember_with_embedding(entry, embedding.vector.clone()).await?;
///
///     println!("Stored entry with {}-dimensional embedding", embedding.vector.len());
///     Ok(())
/// }
/// ```
///
/// ## Hybrid Search (Text + Semantic)
///
/// ```rust,no_run
/// use paladin_ports::output::garrison_port::{GarrisonPort, LongTermGarrisonPort};
/// use paladin_ports::output::embedding_port::EmbeddingPort;
/// use paladin_core::platform::container::garrison::GarrisonEntry;
///
/// async fn hybrid_search(
///     garrison: &dyn LongTermGarrisonPort,
///     embedder: &dyn EmbeddingPort,
///     query: &str,
/// ) -> Result<Vec<GarrisonEntry>, Box<dyn std::error::Error>> {
///     // Text-based search (fast, exact matches)
///     let text_results = garrison.search(query, 10).await?;
///
///     // Semantic search (slower, conceptual matches)
///     let embedding = embedder.embed_text(query).await?;
///     let semantic_results = garrison.search_similar(embedding.vector, 10).await?;
///
///     // Combine results (deduplicate by ID if applicable)
///     let mut combined = text_results;
///     combined.extend(semantic_results);
///
///     Ok(combined)
/// }
/// ```
///
/// # Implementation Notes
///
/// ## Vector Storage
/// - Use specialized vector databases (Qdrant, Pinecone, Weaviate)
/// - Or vector extensions for SQL (pgvector for PostgreSQL)
/// - Store embeddings as BLOB/BYTEA with indexes
///
/// ## Similarity Calculation
/// - Use cosine similarity: `dot(a, b) / (norm(a) * norm(b))`
/// - Normalize embeddings before storage for faster cosine similarity
/// - Consider approximate nearest neighbor (ANN) indexes for large datasets
///
/// ## Performance Optimization
/// ```rust,ignore
/// // Pre-normalize embeddings for faster similarity search
/// fn normalize(embedding: &[f32]) -> Vec<f32> {
///     let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
///     embedding.iter().map(|x| x / norm).collect()
/// }
/// ```
///
/// ## Best Practices
/// 1. **Embedding Consistency**: Always use same model and dimension
/// 2. **Batch Embedding**: Generate embeddings in batches for efficiency
/// 3. **Cache Embeddings**: Don't regenerate for same content
/// 4. **Dimension Validation**: Validate embedding dimensions on insert
/// 5. **Index Strategy**: Use HNSW or IVF indexes for large-scale similarity search
///
/// ## Common Pitfalls
/// - Mixing embeddings from different models (invalid similarity scores)
/// - Not normalizing embeddings (inconsistent cosine similarity)
/// - Linear scan for similarity (use ANN indexes)
/// - Storing embeddings as JSON (inefficient, use binary format)
///
/// # See Also
///
/// - [`GarrisonPort`] - Base trait (this extends it)
/// - [`SanctumPort`](crate::output::sanctum_port::SanctumPort) - Production-grade long-term memory with embeddings
/// - [`EmbeddingPort`](crate::output::embedding_port::EmbeddingPort) - Generate vector embeddings
/// - [Vector Embeddings Guide](https://platform.openai.com/docs/guides/embeddings)
#[async_trait]
pub trait LongTermGarrisonPort: GarrisonPort {
    /// Stores an entry with its vector embedding for semantic search
    ///
    /// # Arguments
    ///
    /// * `entry` - The entry to store
    /// * `embedding` - Vector representation of the entry content
    ///
    /// # Errors
    ///
    /// Returns [`GarrisonError::StorageError`] if storage fails.
    /// Returns [`GarrisonError::SerializationError`] if data cannot be serialized.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin_ports::output::garrison_port::LongTermGarrisonPort;
    /// # use paladin_core::platform::container::garrison::{GarrisonEntry, ConversationRole};
    /// # async fn example(garrison: &dyn LongTermGarrisonPort) {
    /// let entry = GarrisonEntry::new(
    ///     ConversationRole::User,
    ///     "Important information".to_string()
    /// );
    /// let embedding = vec![0.1, 0.2, 0.3]; // From embedding model
    /// garrison.remember_with_embedding(entry, embedding).await
    ///     .expect("Failed to store with embedding");
    /// # }
    /// ```
    async fn remember_with_embedding(
        &self,
        entry: GarrisonEntry,
        embedding: Vec<f32>,
    ) -> Result<(), GarrisonError>;

    /// Searches for entries similar to the given embedding
    ///
    /// Returns entries ranked by cosine similarity (most similar first).
    ///
    /// # Arguments
    ///
    /// * `embedding` - Query vector to find similar entries
    /// * `limit` - Maximum number of results to return
    ///
    /// # Errors
    ///
    /// Returns [`GarrisonError::StorageError`] if search fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin_ports::output::garrison_port::LongTermGarrisonPort;
    /// # async fn example(garrison: &dyn LongTermGarrisonPort) {
    /// let query_embedding = vec![0.15, 0.25, 0.35]; // From embedding model
    /// let similar = garrison.search_similar(query_embedding, 10).await
    ///     .expect("Failed to search");
    ///
    /// for entry in similar {
    ///     println!("Similar: {}", entry.content);
    /// }
    /// # }
    /// ```
    async fn search_similar(
        &self,
        embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<GarrisonEntry>, GarrisonError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_garrison_stats_creation() {
        let stats = GarrisonStats {
            entry_count: 42,
            total_tokens: 1000,
            size_bytes: Some(8192),
        };

        assert_eq!(stats.entry_count, 42);
        assert_eq!(stats.total_tokens, 1000);
        assert_eq!(stats.size_bytes, Some(8192));
    }

    #[test]
    fn test_garrison_error_display() {
        let error = GarrisonError::StorageError("Database connection failed".to_string());
        assert_eq!(
            error.to_string(),
            "Storage error: Database connection failed"
        );

        let error = GarrisonError::NotFound("entry-123".to_string());
        assert_eq!(error.to_string(), "Entry not found: entry-123");
    }

    #[test]
    fn test_garrison_stats_serialization() {
        let stats = GarrisonStats {
            entry_count: 10,
            total_tokens: 500,
            size_bytes: None,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: GarrisonStats = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.entry_count, 10);
        assert_eq!(deserialized.total_tokens, 500);
        assert_eq!(deserialized.size_bytes, None);
    }
}
