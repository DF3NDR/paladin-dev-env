//! # Sanctum Port - Vector Storage and Semantic Search Interface
//!
//! Port trait defining how the application interacts with vector storage backends for long-term
//! semantic memory with embedding-based similarity search.
//!
//! ## Purpose
//!
//! The Sanctum Port provides a unified abstraction for storing and retrieving vector embeddings
//! with associated memories. It enables Paladin agents to maintain long-term semantic memory,
//! find conceptually similar past experiences, and build knowledge bases with advanced search
//! capabilities. The port supports:
//!
//! - **Vector Storage**: Store memories with vector embeddings for semantic similarity
//! - **Semantic Search**: Find similar memories using cosine similarity or other distance metrics
//! - **Metadata Filtering**: Filter results by Paladin ID, memory type, time range, importance
//! - **Batch Operations**: Efficient bulk storage for multiple memories
//! - **CRUD Operations**: Create, read, update, delete individual memories
//!
//! By decoupling vector storage from specific database implementations, Sanctum Port enables:
//! - Switching between vector databases (Qdrant, Pinecone, Weaviate, pgvector)
//! - Testing with in-memory implementations
//! - Optimizing for different scales (local files, distributed clusters)
//!
//! ## Hexagonal Architecture
//!
//! This is an **output port** in the application layer. It defines the interface for vector
//! storage operations, allowing Paladin agents to maintain semantic memory without depending
//! on specific vector database implementations.
//!
//! **Adapter Implementations:**
//! - `QdrantSanctum` - Production vector database with HNSW indexing
//! - `InMemorySanctum` - Fast, volatile memory for development/testing
//! - `PgVectorSanctum` - PostgreSQL with pgvector extension for SQL integration
//! - `PineconeSanctum` - Managed cloud vector database
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync` to support concurrent async operations.
//! Multiple Paladin agents may access the same Sanctum simultaneously. Implementations
//! should handle concurrent reads/writes safely.
//!
//! ## Error Handling
//!
//! Operations return `Result<T, SanctumError>` with specific error variants for:
//! - Storage failures (connection issues, capacity limits)
//! - Search failures (invalid queries, timeout)
//! - Dimension mismatches (embedding size inconsistencies)
//! - Not found errors (missing entries)
//!
//! See [`SanctumError`](crate::output::sanctum_port::SanctumError) for all error categories and handling strategies.
//!
//! ## Relationship to Other Ports
//!
//! **Sanctum vs Garrison**:
//! - **Sanctum**: Long-term semantic memory with vector embeddings (this port)
//! - **Garrison**: Short-term conversation history (basic CRUD + text search)
//! - **LongTermGarrisonPort**: Bridges the gap (Garrison with embedding support)
//!
//! Use Sanctum for:
//! - Knowledge bases requiring semantic search
//! - Long-term memory across sessions
//! - Finding conceptually similar past experiences
//!
//! Use Garrison for:
//! - Recent conversation context
//! - Sequential message history
//! - Fast text-based lookup
//!
//! ## Examples
//!
//! ### Basic Vector Storage and Search
//!
//! ```rust,no_run
//! use paladin_ports::output::sanctum_port::{SanctumPort, SanctumQuery};
//! use paladin_ports::output::embedding_port::EmbeddingPort;
//! use paladin_core::platform::container::sanctum::{SanctumEntry, MemoryBuilder, MemoryType};
//!
//! async fn semantic_memory(
//!     sanctum: &dyn SanctumPort,
//!     embedder: &dyn EmbeddingPort,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Store a memory with embedding
//!     let content = "Rust is a systems programming language focused on safety and performance.";
//!     let embedding = embedder.embed_text(content).await?;
//!
//!     let memory = MemoryBuilder::new("paladin-123".to_string(), content.to_string())
//!         .memory_type(MemoryType::Semantic)
//!         .importance(0.8)
//!         .add_metadata("topic".to_string(), serde_json::json!("programming"))
//!         .build()?;
//!
//!     let entry = SanctumEntry {
//!         memory,
//!         embedding: embedding.vector.clone(),
//!         dimension: embedding.vector.len(),
//!     };
//!
//!     sanctum.store(entry).await?;
//!
//!     // Search for similar memories
//!     let query_embedding = embedder.embed_text("Tell me about programming languages").await?;
//!     let query = SanctumQuery::new(query_embedding.vector, 5);
//!     let results = sanctum.search(query).await?;
//!
//!     println!("Found {} similar memories", results.len());
//!     for result in results {
//!         println!("Score: {:.3} - {}", result.score, result.entry.memory.content);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Filtered Search with Metadata
//!
//! ```rust,no_run
//! use paladin_ports::output::sanctum_port::{SanctumPort, SanctumQuery, SanctumFilter};
//! use paladin_core::platform::container::sanctum::MemoryType;
//!
//! async fn filtered_search(
//!     sanctum: &dyn SanctumPort,
//!     query_embedding: Vec<f32>,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Build a filter
//!     let filter = SanctumFilter::new()
//!         .paladin_id("paladin-123".to_string())
//!         .memory_type(MemoryType::Episodic)
//!         .min_importance(0.7);
//!
//!     // Search with filter
//!     let query = SanctumQuery::new(query_embedding, 10)
//!         .filter(filter)
//!         .min_score(0.5);
//!
//!     let results = sanctum.search(query).await?;
//!     println!("Found {} high-importance episodic memories", results.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Batch Storage for Efficiency
//!
//! ```rust,no_run
//! use paladin_ports::output::sanctum_port::SanctumPort;
//! use paladin_core::platform::container::sanctum::{SanctumEntry, MemoryBuilder, MemoryType};
//!
//! async fn batch_store(
//!     sanctum: &dyn SanctumPort,
//!     documents: Vec<(String, Vec<f32>)>, // (content, embedding) pairs
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     let mut entries = Vec::new();
//!
//!     for (content, embedding) in documents {
//!         let memory = MemoryBuilder::new("batch-paladin".to_string(), content)
//!             .memory_type(MemoryType::Semantic)
//!             .importance(0.5)
//!             .build()?;
//!
//!         let entry = SanctumEntry {
//!             memory,
//!             embedding: embedding.clone(),
//!             dimension: embedding.len(),
//!         };
//!         entries.push(entry);
//!     }
//!
//!     // Store all at once (much faster than individual stores)
//!     sanctum.store_batch(entries).await?;
//!     println!("Batch storage complete");
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Implementation Notes
//!
//! ### Performance Considerations
//! - **Indexing**: Use HNSW, IVF, or similar ANN (approximate nearest neighbor) indexes
//! - **Batch Operations**: Prefer `store_batch()` over multiple `store()` calls
//! - **Dimension Consistency**: Validate all embeddings have same dimension
//! - **Filtering**: Apply metadata filters before vector similarity to reduce search space
//! - **Caching**: Cache frequently accessed vectors in memory
//!
//! ### Best Practices
//! 1. **Normalize Embeddings**: Store normalized vectors for faster cosine similarity
//! 2. **Index Strategy**: Choose appropriate index type based on dataset size
//! 3. **Pagination**: For large result sets, implement pagination beyond `top_k`
//! 4. **Metadata Schema**: Define consistent metadata schema across entries
//! 5. **Error Handling**: Implement retry logic for transient storage errors
//!
//! ### Common Pitfalls
//! - Mixing embeddings from different models (invalid similarity scores)
//! - Not setting `min_score` threshold (low-quality results)
//! - Using linear scan for large datasets (use ANN indexes)
//! - Storing embeddings without metadata (limited filtering capability)
//!
//! ## Related Ports
//!
//! - [`GarrisonPort`](crate::output::garrison_port::GarrisonPort) - Short-term conversation memory (basic CRUD)
//! - [`LongTermGarrisonPort`](crate::output::garrison_port::LongTermGarrisonPort) - Garrison with embedding support (hybrid)
//! - [`EmbeddingPort`](crate::output::embedding_port::EmbeddingPort) - Generate vector embeddings for storage
//! - [`LlmPort`](crate::output::llm_port::LlmPort) - LLM integration (uses Sanctum for knowledge retrieval)
//!
//! ## See Also
//!
//! - Application Ports (`crate::application::ports`)
//! - [Sanctum Domain](paladin_core::platform::container::sanctum)
//! - Infrastructure Adapters (`crate::infrastructure::adapters::sanctum`)
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use paladin_core::platform::container::sanctum::{MemoryType, SanctumEntry};

/// Errors that can occur during Sanctum vector storage operations.
///
/// All Sanctum operations return `Result<T, SanctumError>`. These errors cover
/// storage failures, search issues, dimension mismatches, and configuration problems.
///
/// # Error Categories
///
/// ## Transient Errors (Retryable)
/// - [`StorageError`](Self::StorageError) - May succeed on retry (network issues, timeouts)
/// - [`SearchError`](Self::SearchError) - May succeed on retry (temporary service issues)
///
/// ## Permanent Errors (Non-Retryable)
/// - [`InvalidDimension`](Self::InvalidDimension) - Embedding size mismatch
/// - [`NotFound`](Self::NotFound) - Entry doesn't exist
/// - [`ConfigError`](Self::ConfigError) - Invalid configuration
///
/// # Examples
///
/// ## Error Handling with Retry
///
/// ```rust,no_run
/// use paladin_ports::output::sanctum_port::{SanctumPort, SanctumError};
/// use paladin_core::platform::container::sanctum::SanctumEntry;
///
/// async fn store_with_retry(
///     sanctum: &dyn SanctumPort,
///     entry: SanctumEntry,
///     max_retries: u32,
/// ) -> Result<(), SanctumError> {
///     let mut attempts = 0;
///     loop {
///         match sanctum.store(entry.clone()).await {
///             Ok(_) => return Ok(()),
///             Err(SanctumError::StorageError(e)) if attempts < max_retries => {
///                 attempts += 1;
///                 eprintln!("Retry {}/{}: {}", attempts, max_retries, e);
///                 tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
///             }
///             Err(e) => return Err(e),
///         }
///     }
/// }
/// ```
///
/// ## Dimension Validation
///
/// ```rust,no_run
/// use paladin_ports::output::sanctum_port::{SanctumPort, SanctumError};
/// use paladin_core::platform::container::sanctum::SanctumEntry;
///
/// async fn store_with_validation(
///     sanctum: &dyn SanctumPort,
///     entry: SanctumEntry,
///     expected_dim: usize,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     if entry.embedding.len() != expected_dim {
///         return Err(Box::new(SanctumError::InvalidDimension(
///             format!("Expected {} dimensions, got {}", expected_dim, entry.embedding.len())
///         )));
///     }
///
///     sanctum.store(entry).await?;
///     Ok(())
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum SanctumError {
    /// Error occurred in underlying vector storage (database connection, disk I/O, network).
    ///
    /// **Retryable**: Yes (may be transient network/database issue)
    ///
    /// **Common Causes**:
    /// - Vector database connection timeout
    /// - Disk space exhausted
    /// - Network partition
    /// - Resource exhaustion (memory, connections)
    ///
    /// **Recovery**: Retry with exponential backoff. If persistent, check vector DB health.
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Search operation failed.
    ///
    /// **Retryable**: Yes (may be transient service issue)
    ///
    /// **Common Causes**:
    /// - Vector database query timeout
    /// - Invalid query parameters
    /// - Index not built/available
    /// - Service temporarily unavailable
    ///
    /// **Recovery**: Retry with adjusted parameters (lower top_k, simpler filter).
    #[error("Search error: {0}")]
    SearchError(String),

    /// Embedding dimension doesn't match expected size.
    ///
    /// **Retryable**: No (data format issue)
    ///
    /// **Common Causes**:
    /// - Mixing embeddings from different models
    /// - Incorrect embedding model configuration
    /// - Corrupted embedding data
    ///
    /// **Recovery**: Regenerate embeddings with correct model, or migrate collection to new dimension.
    #[error("Invalid dimension: {0}")]
    InvalidDimension(String),

    /// Requested entry was not found.
    ///
    /// **Retryable**: No (entry doesn't exist)
    ///
    /// **Common Causes**:
    /// - Entry was deleted
    /// - Incorrect entry ID
    /// - Collection was cleared
    ///
    /// **Recovery**: Handle missing entry case (e.g., skip update, return empty result).
    #[error("Not found: {0}")]
    NotFound(String),

    /// Configuration is invalid.
    ///
    /// **Retryable**: No (requires configuration fix)
    ///
    /// **Common Causes**:
    /// - Missing connection URL
    /// - Invalid collection name
    /// - Unsupported embedding dimensions
    /// - Missing API key
    ///
    /// **Recovery**: Fix configuration and reinitialize Sanctum.
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Filter criteria for searching memories.
///
/// Allows filtering by metadata fields before semantic similarity search.
/// Filters are applied before vector similarity calculation to reduce search space
/// and improve performance.
///
/// # Use Cases
///
/// - Filter memories by specific Paladin agent
/// - Find memories of a particular type (Episodic, Semantic, Procedural)
/// - Search within a time range
/// - Filter by importance threshold
/// - Apply custom metadata filters
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::sanctum_port::SanctumFilter;
/// use paladin_core::platform::container::sanctum::MemoryType;
/// use chrono::{Utc, Duration};
///
/// // Filter recent important semantic memories
/// let filter = SanctumFilter::new()
///     .paladin_id("agent-007".to_string())
///     .memory_type(MemoryType::Semantic)
///     .min_importance(0.8);
///
/// // Filter by time range
/// let yesterday = Utc::now() - Duration::days(1);
/// let filter_recent = SanctumFilter::default();
/// // Note: created_after field is set directly, no builder method
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SanctumFilter {
    /// Filter by Paladin ID
    pub paladin_id: Option<String>,

    /// Filter by memory type
    pub memory_type: Option<MemoryType>,

    /// Filter by creation date range
    pub created_after: Option<DateTime<Utc>>,
    /// Upper bound of the creation date range filter.
    pub created_before: Option<DateTime<Utc>>,

    /// Filter by importance threshold
    pub min_importance: Option<f32>,

    /// Additional custom metadata filters
    pub metadata_filters: HashMap<String, Value>,
}

impl SanctumFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by Paladin ID
    pub fn paladin_id(mut self, paladin_id: String) -> Self {
        self.paladin_id = Some(paladin_id);
        self
    }

    /// Filter by memory type
    pub fn memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = Some(memory_type);
        self
    }

    /// Filter by minimum importance
    pub fn min_importance(mut self, min_importance: f32) -> Self {
        self.min_importance = Some(min_importance);
        self
    }

    /// Add a custom metadata filter
    pub fn add_metadata_filter(mut self, key: String, value: Value) -> Self {
        self.metadata_filters.insert(key, value);
        self
    }
}

/// Query for searching the vector store.
///
/// Contains the search vector, result limits, and filtering criteria.
/// Queries use cosine similarity (or other distance metrics) to find
/// the most similar vectors in the database.
///
/// # Query Parameters
///
/// - `embedding`: The query vector (must match collection dimension)
/// - `top_k`: Maximum number of results to return
/// - `filter`: Optional metadata filter (applied before similarity)
/// - `min_score`: Minimum similarity threshold (0.0-1.0, higher = more similar)
///
/// # Examples
///
/// ## Basic Search
///
/// ```rust
/// use paladin_ports::output::sanctum_port::SanctumQuery;
///
/// let query_vector = vec![0.1, 0.2, 0.3]; // From embedding model
/// let query = SanctumQuery::new(query_vector, 10); // Top 10 results
/// ```
///
/// ## Filtered Search with Score Threshold
///
/// ```rust
/// use paladin_ports::output::sanctum_port::{SanctumQuery, SanctumFilter};
/// use paladin_core::platform::container::sanctum::MemoryType;
///
/// let query_vector = vec![0.1, 0.2, 0.3];
/// let filter = SanctumFilter::new()
///     .paladin_id("agent-123".to_string())
///     .memory_type(MemoryType::Episodic);
///
/// let query = SanctumQuery::new(query_vector, 5)
///     .filter(filter)
///     .min_score(0.7); // Only results with 70%+ similarity
/// ```
#[derive(Debug, Clone)]
pub struct SanctumQuery {
    /// The query embedding vector
    pub embedding: Vec<f32>,

    /// Maximum number of results to return
    pub top_k: usize,

    /// Optional filter criteria
    pub filter: Option<SanctumFilter>,

    /// Minimum similarity score (0.0 - 1.0)
    pub min_score: Option<f32>,
}

impl SanctumQuery {
    /// Create a new query with an embedding vector
    ///
    /// # Arguments
    ///
    /// * `embedding` - The query vector
    /// * `top_k` - Maximum number of results to return
    pub fn new(embedding: Vec<f32>, top_k: usize) -> Self {
        Self {
            embedding,
            top_k,
            filter: None,
            min_score: None,
        }
    }

    /// Add a filter to the query
    pub fn with_filter(mut self, filter: SanctumFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Add a filter to the query (short alias for with_filter)
    pub fn filter(self, filter: SanctumFilter) -> Self {
        self.with_filter(filter)
    }

    /// Set minimum similarity score threshold
    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = Some(min_score);
        self
    }

    /// Set minimum similarity score threshold (short alias for with_min_score)
    pub fn min_score(self, min_score: f32) -> Self {
        self.with_min_score(min_score)
    }
}

/// Search result containing an entry and its similarity score.
///
/// Results are typically sorted by score in descending order (most similar first).
/// The score represents similarity (cosine similarity by default), where:
/// - 1.0 = identical vectors
/// - 0.0 = orthogonal (no similarity)
/// - -1.0 = opposite direction
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::sanctum_port::SanctumSearchResult;
/// use paladin_core::platform::container::sanctum::{SanctumEntry, Memory, MemoryType};
/// use uuid::Uuid;
/// use chrono::Utc;
///
/// let memory = Memory {
///     id: Uuid::new_v4(),
///     paladin_id: "test".to_string(),
///     content: "Example memory".to_string(),
///     memory_type: MemoryType::Semantic,
///     importance: 0.5,
///     access_count: 0,
///     last_accessed: Utc::now(),
///     created_at: Utc::now(),
///     metadata: std::collections::HashMap::new(),
/// };
///
/// let entry = SanctumEntry {
///     memory,
///     embedding: vec![0.1, 0.2, 0.3],
///     dimension: 3,
/// };
///
/// let result = SanctumSearchResult::new(entry, 0.95);
/// assert!(result.score > 0.9); // High similarity
/// ```
#[derive(Debug, Clone)]
pub struct SanctumSearchResult {
    /// The matching entry
    pub entry: SanctumEntry,

    /// Similarity score (0.0 - 1.0, higher is more similar)
    pub score: f32,
}

impl SanctumSearchResult {
    /// Create a new search result
    pub fn new(entry: SanctumEntry, score: f32) -> Self {
        Self { entry, score }
    }
}

/// Port trait for vector storage and semantic search.\n///\n/// This trait provides a standardized interface for storing and retrieving\n/// vector embeddings with associated memories. Implementations can use\n/// different vector databases (Qdrant, Pinecone, Weaviate, in-memory, etc.).\n///\n/// # Capabilities\n///\n/// - **Storage**: Add single or batch entries with [`store`](Self::store) / [`store_batch`](Self::store_batch)\n/// - **Search**: Find similar vectors with [`search`](Self::search)\n/// - **Management**: Update or delete entries with [`update`](Self::update) / [`delete`](Self::delete)\n/// - **Monitoring**: Count entries with [`count`](Self::count)\n///\n/// # Thread Safety\n///\n/// All implementations must be `Send + Sync` to support async operations across\n/// thread boundaries. Multiple Paladin agents may access the same Sanctum concurrently.\n///\n/// # Implementation Requirements\n///\n/// Implementations should:\n/// 1. Use ANN (approximate nearest neighbor) indexes for fast similarity search\n/// 2. Validate embedding dimensions on insert (must be consistent)\n/// 3. Support concurrent read/write operations safely\n/// 4. Return search results sorted by similarity score (descending)\n/// 5. Apply filters before similarity calculation for efficiency\n///\n/// # Examples\n///\n/// ## Knowledge Base RAG Pattern\n///\n/// ```rust,no_run\n/// use paladin_ports::output::sanctum_port::{SanctumPort, SanctumQuery};\n/// use paladin_ports::output::embedding_port::EmbeddingPort;\n///\n/// async fn rag_retrieval(\n///     sanctum: &dyn SanctumPort,\n///     embedder: &dyn EmbeddingPort,\n///     user_query: &str,\n/// ) -> Result<Vec<String>, Box<dyn std::error::Error>> {\n///     // Generate query embedding\n///     let query_embedding = embedder.embed_text(user_query).await?;\n///     \n///     // Search for relevant knowledge\n///     let query = SanctumQuery::new(query_embedding.vector, 3)\n///         .min_score(0.7); // Only high-quality matches\n///     \n///     let results = sanctum.search(query).await?;\n///     \n///     // Extract content for LLM context\n///     let context: Vec<String> = results\n///         .into_iter()\n///         .map(|r| r.entry.memory.content)\n///         .collect();\n///     \n///     Ok(context)\n/// }\n/// ```\n///\n/// ## Multi-Agent Memory Isolation\n///\n/// ```rust,no_run\n/// use paladin_ports::output::sanctum_port::{SanctumPort, SanctumQuery, SanctumFilter};\n///\n/// async fn agent_specific_search(\n///     sanctum: &dyn SanctumPort,\n///     paladin_id: &str,\n///     query_embedding: Vec<f32>,\n/// ) -> Result<(), Box<dyn std::error::Error>> {\n///     // Filter to only this agent's memories\n///     let filter = SanctumFilter::new()\n///         .paladin_id(paladin_id.to_string());\n///     \n///     let query = SanctumQuery::new(query_embedding, 10)\n///         .filter(filter);\n///     \n///     let results = sanctum.search(query).await?;\n///     println!(\"Agent {} has {} relevant memories\", paladin_id, results.len());\n///     \n///     Ok(())\n/// }\n/// ```\n///\n/// ## Memory Lifecycle Management\n///\n/// ```rust,no_run\n/// use paladin_ports::output::sanctum_port::SanctumPort;\n/// use paladin_core::platform::container::sanctum::SanctumEntry;\n///\n/// async fn update_memory_importance(\n///     sanctum: &dyn SanctumPort,\n///     memory_id: &str,\n///     mut entry: SanctumEntry,\n///     new_importance: f32,\n/// ) -> Result<(), Box<dyn std::error::Error>> {\n///     // Update importance score\n///     entry.memory.importance = new_importance;\n///     \n///     // Persist update\n///     sanctum.update(entry).await?;\n///     println!(\"Updated memory {} importance to {}\", memory_id, new_importance);\n///     \n///     Ok(())\n/// }\n/// ```\n///\n/// # Implementation Notes\n///\n/// ## Vector Database Selection\n///\n/// Choose based on scale and requirements:\n/// - **Qdrant**: Self-hosted, excellent performance, HNSW indexes\n/// - **Pinecone**: Managed service, easy scaling, pay-per-use\n/// - **Weaviate**: GraphQL API, hybrid search, schema validation\n/// - **pgvector**: PostgreSQL extension, SQL integration, ACID guarantees\n/// - **In-Memory**: Development/testing only, no persistence\n///\n/// ## Performance Optimization\n///\n/// ```rust,ignore\n/// // Good: Batch storage (10-100x faster)\n/// sanctum.store_batch(entries).await?;\n///\n/// // Avoid: Individual stores in loop\n/// for entry in entries {\n///     sanctum.store(entry).await?; // Slow!\n/// }\n///\n/// // Good: Filter before search\n/// let query = SanctumQuery::new(embedding, 10)\n///     .filter(SanctumFilter::new().paladin_id(id));\n///\n/// // Good: Set min_score threshold\n/// let query = SanctumQuery::new(embedding, 10)\n///     .min_score(0.5); // Filter low-quality matches\n/// ```\n///\n/// ## Index Configuration\n///\n/// For optimal performance:\n/// - Use HNSW index for < 1M vectors (high recall, fast queries)\n/// - Use IVF index for > 1M vectors (good recall, scalable)\n/// - Tune index parameters: `m`, `ef_construction`, `nprobe`\n/// - Rebuild indexes periodically for best performance\n///\n/// ## Embedding Best Practices\n///\n/// 1. **Consistency**: Always use same model and dimension\n/// 2. **Normalization**: Store normalized vectors for cosine similarity\n/// 3. **Validation**: Check dimension on every insert\n/// 4. **Metadata**: Include rich metadata for filtering\n/// 5. **Batch Operations**: Process embeddings in batches of 10-100\n///\n/// # Common Pitfalls\n///\n/// - Using different embedding models in same collection (invalid similarity)\n/// - Not setting `min_score` threshold (low-quality results)\n/// - Linear scan without indexes (slow for > 10k vectors)\n/// - Storing unnormalized embeddings (inconsistent cosine similarity)\n/// - Not using batch operations (poor performance)\n///\n/// # See Also\n///\n/// - [`SanctumQuery`] - Query builder with fluent API\n/// - [`SanctumFilter`] - Metadata filtering options\n/// - [`SanctumSearchResult`] - Search result with similarity score\n/// - [`GarrisonPort`](crate::output::garrison_port::GarrisonPort) - Short-term memory alternative
#[async_trait]
pub trait SanctumPort: Send + Sync {
    /// Store a single entry in the vector database
    ///
    /// # Arguments
    ///
    /// * `entry` - The entry to store
    ///
    /// # Errors
    ///
    /// Returns `SanctumError::StorageError` if the operation fails
    async fn store(&self, entry: SanctumEntry) -> Result<(), SanctumError>;

    /// Store multiple entries in a batch operation
    ///
    /// This is more efficient than calling `store()` multiple times.
    ///
    /// # Arguments
    ///
    /// * `entries` - The entries to store
    ///
    /// # Errors
    ///
    /// Returns `SanctumError::StorageError` if the operation fails
    async fn store_batch(&self, entries: Vec<SanctumEntry>) -> Result<(), SanctumError>;

    /// Search for similar entries using semantic similarity
    ///
    /// # Arguments
    ///
    /// * `query` - The search query with embedding and filters
    ///
    /// # Returns
    ///
    /// A vector of search results sorted by similarity score (descending)
    ///
    /// # Errors
    ///
    /// Returns `SanctumError::SearchError` if the operation fails
    async fn search(&self, query: SanctumQuery) -> Result<Vec<SanctumSearchResult>, SanctumError>;

    /// Delete an entry by its memory ID
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID of the memory to delete (as string)
    ///
    /// # Returns
    ///
    /// `true` if the entry was found and deleted, `false` if not found
    ///
    /// # Errors
    ///
    /// Returns `SanctumError::StorageError` if the operation fails
    async fn delete(&self, id: &str) -> Result<bool, SanctumError>;

    /// Update an existing entry
    ///
    /// # Arguments
    ///
    /// * `entry` - The updated entry (must have existing ID)
    ///
    /// # Errors
    ///
    /// Returns `SanctumError::StorageError` if the operation fails
    /// Returns `SanctumError::NotFound` if the entry doesn't exist
    async fn update(&self, entry: SanctumEntry) -> Result<(), SanctumError>;

    /// Get the total count of stored entries
    ///
    /// # Returns
    ///
    /// The number of entries in the database
    /// Count total entries, optionally filtered by criteria
    ///
    /// # Arguments
    /// * `filter` - Optional filter to apply
    ///
    /// # Returns
    ///
    /// Total count of entries matching the filter (or all entries if no filter)
    ///
    /// # Errors
    ///
    /// Returns `SanctumError::StorageError` if the operation fails
    async fn count(&self, filter: Option<SanctumFilter>) -> Result<usize, SanctumError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanctum_filter_builder() {
        let filter = SanctumFilter::new()
            .paladin_id("test-123".to_string())
            .memory_type(MemoryType::Semantic)
            .min_importance(0.7);

        assert_eq!(filter.paladin_id, Some("test-123".to_string()));
        assert_eq!(filter.memory_type, Some(MemoryType::Semantic));
        assert_eq!(filter.min_importance, Some(0.7));
    }

    #[test]
    fn test_sanctum_query_builder() {
        let embedding = vec![0.1, 0.2, 0.3];
        let query = SanctumQuery::new(embedding.clone(), 10).with_min_score(0.8);

        assert_eq!(query.embedding, embedding);
        assert_eq!(query.top_k, 10);
        assert_eq!(query.min_score, Some(0.8));
    }

    #[test]
    fn test_sanctum_filter_default() {
        let filter = SanctumFilter::default();
        assert!(filter.paladin_id.is_none());
        assert!(filter.memory_type.is_none());
        assert!(filter.metadata_filters.is_empty());
    }
}
