//! # Embedding Port - Text-to-Vector Embedding Interface
//!
//! Port trait defining how the application generates vector embeddings from text for semantic search.
//!
//! ## Purpose
//!
//! The Embedding Port provides a unified abstraction for converting text into dense vector
//! representations (embeddings). These embeddings enable semantic similarity search, allowing
//! the system to find conceptually similar content regardless of exact word matches. The port supports:
//!
//! - **Single Text Embedding**: Convert individual text strings to vectors
//! - **Batch Embedding**: Efficiently process multiple texts in one API call
//! - **Model Information**: Query embedding model name and dimension
//! - **Provider Abstraction**: Switch between OpenAI, local models, custom embedders
//!
//! By decoupling embedding generation from specific providers, Embedding Port enables:
//! - Switching between embedding models (OpenAI, Cohere, local Sentence Transformers)
//! - Testing with mock implementations
//! - Optimizing costs by choosing appropriate models per use case
//!
//! ## Hexagonal Architecture
//!
//! This is an **output port** in the application layer. It defines the interface for
//! text-to-vector conversion, allowing Paladin agents to perform semantic operations
//! without depending on specific embedding service implementations.
//!
//! **Adapter Implementations:**
//! - `OpenAIEmbeddingAdapter` - OpenAI embedding models (text-embedding-3-small, text-embedding-3-large)
//! - `LocalEmbeddingAdapter` - Local Sentence Transformers models
//! - `CohereEmbeddingAdapter` - Cohere embedding API
//! - `MockEmbeddingAdapter` - Testing with fixed vectors
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync` to support concurrent async operations.
//! Multiple Paladin agents may request embeddings simultaneously. Implementations should
//! handle concurrent requests efficiently (connection pooling, batching).
//!
//! ## Error Handling
//!
//! Operations return `Result<T, EmbeddingError>` with specific error variants for:
//! - Network failures (timeouts, connection issues)
//! - Rate limiting (API quota exhaustion)
//! - Invalid input (empty text, exceeds token limits)
//! - Provider errors (API key issues, model unavailable)
//!
//! See [`EmbeddingError`] for all error categories and handling strategies.
//!
//! ## Common Use Cases
//!
//! 1. **Semantic Search**: Generate query embeddings for similarity search in Sanctum/Garrison
//! 2. **RAG Systems**: Embed documents and queries for retrieval-augmented generation
//! 3. **Clustering**: Group similar content by embedding proximity
//! 4. **Deduplication**: Detect duplicate/similar content by high cosine similarity
//!
//! ## Examples
//!
//! ### Basic Text Embedding
//!
//! ```rust,no_run
//! use paladin_ports::output::embedding_port::EmbeddingPort;
//!
//! async fn embed_query(embedder: &dyn EmbeddingPort) -> Result<(), Box<dyn std::error::Error>> {
//!     let query = "What is the capital of France?";
//!     let embedding = embedder.embed_text(query).await?;
//!
//!     println!("Generated {}-dimensional embedding", embedding.dimension);
//!     println!("Using model: {}", embedding.model);
//!     println!("Vector preview: {:?}", &embedding.vector[..5]);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Batch Embedding for Efficiency
//!
//! ```rust,no_run
//! use paladin_ports::output::embedding_port::EmbeddingPort;
//!
//! async fn embed_documents(
//!     embedder: &dyn EmbeddingPort,
//!     documents: Vec<String>,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Convert to &str for batch API
//!     let doc_refs: Vec<&str> = documents.iter().map(|s| s.as_str()).collect();
//!
//!     // Batch embedding (10-100x faster than individual calls)
//!     let embeddings = embedder.embed_batch(&doc_refs).await?;
//!
//!     println!("Embedded {} documents in one API call", embeddings.len());
//!
//!     // Process embeddings
//!     for (doc, emb) in documents.iter().zip(embeddings.iter()) {
//!         println!("Doc: {}... -> {}-dim vector",
//!             &doc.chars().take(30).collect::<String>(),
//!             emb.dimension);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Semantic Similarity Calculation
//!
//! ```rust,no_run
//! use paladin_ports::output::embedding_port::EmbeddingPort;
//!
//! async fn calculate_similarity(
//!     embedder: &dyn EmbeddingPort,
//!     text1: &str,
//!     text2: &str,
//! ) -> Result<f32, Box<dyn std::error::Error>> {
//!     // Generate embeddings
//!     let emb1 = embedder.embed_text(text1).await?;
//!     let emb2 = embedder.embed_text(text2).await?;
//!
//!     // Calculate cosine similarity
//!     let dot_product: f32 = emb1.vector.iter()
//!         .zip(emb2.vector.iter())
//!         .map(|(a, b)| a * b)
//!         .sum();
//!
//!     let norm1: f32 = emb1.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
//!     let norm2: f32 = emb2.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
//!
//!     let similarity = dot_product / (norm1 * norm2);
//!     println!("Similarity: {:.3}", similarity);
//!
//!     Ok(similarity)
//! }
//! ```
//!
//! ## Implementation Notes
//!
//! ### Performance Considerations
//! - **Batch Operations**: Always use `embed_batch()` for multiple texts (10-100x faster)
//! - **Token Limits**: Check provider token limits (8K for OpenAI text-embedding-3-small)
//! - **Caching**: Cache embeddings for frequently embedded content
//! - **Connection Pooling**: Reuse HTTP connections for multiple requests
//! - **Rate Limiting**: Implement exponential backoff for rate limit errors
//!
//! ### Best Practices
//! 1. **Consistent Models**: Use same model for all embeddings in a collection
//! 2. **Normalization**: Normalize embeddings if using dot product instead of cosine similarity
//! 3. **Error Handling**: Always handle `RateLimited` errors with retry logic
//! 4. **Dimension Validation**: Verify embedding dimension matches expected value
//! 5. **Cost Optimization**: Choose appropriate model (small vs large) based on use case
//!
//! ### Common Pitfalls
//! - Embedding texts one-by-one instead of using batch API (slow, expensive)
//! - Mixing embeddings from different models (invalid similarity scores)
//! - Not handling rate limits (requests fail unnecessarily)
//! - Embedding very long texts without chunking (exceeds token limits)
//! - Not caching embeddings for repeated content (wasted API calls)
//!
//! ## Related Ports
//!
//! - [`SanctumPort`](crate::output::sanctum_port::SanctumPort) - Vector storage (stores embeddings from this port)
//! - [`LongTermGarrisonPort`](crate::output::garrison_port::LongTermGarrisonPort) - Garrison with embeddings (uses this port)
//! - [`LlmPort`](crate::output::llm_port::LlmPort) - LLM integration (complementary for RAG systems)
//!
//! ## See Also
//!
//! - Application Ports (`crate::application::ports`)
//! - [OpenAI Embeddings Guide](https://platform.openai.com/docs/guides/embeddings)
//! - Infrastructure Adapters (`crate::infrastructure::adapters::embedding`)

/// Errors that can occur during embedding generation.
///
/// All embedding operations return `Result<T, EmbeddingError>`. These errors cover
/// network failures, rate limiting, invalid input, and provider-specific issues.
///
/// # Error Categories
///
/// ## Transient Errors (Retryable)
/// - [`NetworkError`](Self::NetworkError) - May succeed on retry (connection issues, timeouts)
/// - [`RateLimited`](Self::RateLimited) - Retry after backoff period
///
/// ## Permanent Errors (Non-Retryable)
/// - [`InvalidInput`](Self::InvalidInput) - Text is empty or too long
/// - [`ProviderError`](Self::ProviderError) - API key invalid, model unavailable
///
/// # Examples
///
/// ## Error Handling with Retry
///
/// ```rust,no_run
/// use paladin_ports::output::embedding_port::{EmbeddingPort, EmbeddingError};
///
/// async fn embed_with_retry(
///     embedder: &dyn EmbeddingPort,
///     text: &str,
///     max_retries: u32,
/// ) -> Result<paladin_ports::output::embedding_port::Embedding, EmbeddingError> {
///     let mut attempts = 0;
///     loop {
///         match embedder.embed_text(text).await {
///             Ok(embedding) => return Ok(embedding),
///             Err(EmbeddingError::RateLimited(msg)) if attempts < max_retries => {
///                 attempts += 1;
///                 let wait_ms = 1000 * 2_u64.pow(attempts); // Exponential backoff
///                 eprintln!("Rate limited, waiting {}ms (attempt {})", wait_ms, attempts);
///                 tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;
///             }
///             Err(EmbeddingError::NetworkError(e)) if attempts < max_retries => {
///                 attempts += 1;
///                 eprintln!("Network error, retrying: {}", e);
///                 tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
///             }
///             Err(e) => return Err(e),
///         }
///     }
/// }
/// ```
///
/// ## Input Validation
///
/// ```rust,no_run
/// use paladin_ports::output::embedding_port::{EmbeddingPort, EmbeddingError};
///
/// async fn embed_with_validation(
///     embedder: &dyn EmbeddingPort,
///     text: &str,
/// ) -> Result<paladin_ports::output::embedding_port::Embedding, EmbeddingError> {
///     // Validate before embedding
///     if text.trim().is_empty() {
///         return Err(EmbeddingError::InvalidInput("Text is empty".to_string()));
///     }
///
///     let token_count = text.split_whitespace().count();
///     if token_count > 8000 {
///         return Err(EmbeddingError::InvalidInput(
///             format!("Text too long: {} tokens (max 8000)", token_count)
///         ));
///     }
///
///     embedder.embed_text(text).await
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    /// Network-related errors (timeouts, connection failures).
    ///
    /// **Retryable**: Yes (may be transient network issue)
    ///
    /// **Common Causes**:
    /// - Connection timeout to embedding service
    /// - DNS resolution failure
    /// - Network partition
    /// - TLS handshake failure
    ///
    /// **Recovery**: Retry with exponential backoff (500ms, 1s, 2s, 4s).
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Rate limiting errors from the embedding provider.
    ///
    /// **Retryable**: Yes (retry after backoff period)
    ///
    /// **Common Causes**:
    /// - Exceeded requests per minute (RPM) limit
    /// - Exceeded tokens per minute (TPM) limit
    /// - Exceeded requests per day (RPD) limit
    ///
    /// **Recovery**: Retry with exponential backoff (1s, 2s, 4s, 8s). Check rate limit headers.
    #[error("Rate limited: {0}")]
    RateLimited(String),

    /// Invalid input provided to the embedding service.
    ///
    /// **Retryable**: No (input must be fixed)
    ///
    /// **Common Causes**:
    /// - Empty text string
    /// - Text exceeds token limit (8K for OpenAI text-embedding-3-small)
    /// - Invalid characters or encoding
    ///
    /// **Recovery**: Validate and fix input (trim whitespace, chunk long texts).
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Provider-specific errors (API key issues, model not found, etc.).
    ///
    /// **Retryable**: Usually no (configuration issue)
    ///
    /// **Common Causes**:
    /// - Invalid or expired API key
    /// - Model name doesn't exist
    /// - Insufficient permissions
    /// - Provider service outage
    ///
    /// **Recovery**: Check API key, model name, and provider status page.
    #[error("Provider error: {0}")]
    ProviderError(String),
}

/// A vector embedding with associated metadata.
///
/// Represents the result of converting text into a dense vector representation.
/// The vector enables semantic similarity calculations (cosine similarity, dot product).
///
/// # Fields
///
/// - `vector`: The dense vector representation (e.g., 1536 dimensions for OpenAI text-embedding-3-small)
/// - `model`: Model identifier used to generate this embedding
/// - `dimension`: Number of dimensions in the vector
/// - `token_count`: Optional token count (useful for billing estimation)
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::embedding_port::Embedding;
///
/// let embedding = Embedding {
///     vector: vec![0.1, 0.2, 0.3],
///     model: "text-embedding-3-small".to_string(),
///     dimension: 3,
///     token_count: Some(5),
/// };
///
/// assert_eq!(embedding.dimension, embedding.vector.len());
/// assert_eq!(embedding.model, "text-embedding-3-small");
/// ```
///
/// ## Similarity Calculation
///
/// ```rust
/// use paladin_ports::output::embedding_port::Embedding;
///
/// fn cosine_similarity(emb1: &Embedding, emb2: &Embedding) -> f32 {
///     let dot: f32 = emb1.vector.iter().zip(emb2.vector.iter()).map(|(a, b)| a * b).sum();
///     let norm1: f32 = emb1.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
///     let norm2: f32 = emb2.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
///     dot / (norm1 * norm2)
/// }
///
/// let emb1 = Embedding {
///     vector: vec![1.0, 0.0, 0.0],
///     model: "test".to_string(),
///     dimension: 3,
///     token_count: None,
/// };
///
/// let emb2 = Embedding {
///     vector: vec![0.8, 0.6, 0.0],
///     model: "test".to_string(),
///     dimension: 3,
///     token_count: None,
/// };
///
/// let similarity = cosine_similarity(&emb1, &emb2);
/// assert!((similarity - 0.8).abs() < 0.01);
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Embedding {
    /// The embedding vector
    pub vector: Vec<f32>,

    /// The model used to generate this embedding
    pub model: String,

    /// The dimension of the embedding vector
    pub dimension: usize,

    /// Optional token count for the embedded text
    pub token_count: Option<u32>,
}

/// Port trait for generating vector embeddings from text.\n///\n/// This trait provides a standardized interface for embedding providers,\n/// allowing the system to plug in different embedding services (OpenAI, Cohere,\n/// local models, etc.) without changing core business logic.\n///\n/// # Capabilities\n///\n/// - **Single Embedding**: Convert one text string to vector with [`embed_text`](Self::embed_text)\n/// - **Batch Embedding**: Process multiple texts efficiently with [`embed_batch`](Self::embed_batch)\n/// - **Model Info**: Query dimension and model name with [`dimension`](Self::dimension) / [`model_name`](Self::model_name)\n///\n/// # Thread Safety\n///\n/// All implementations must be `Send + Sync` to support concurrent async operations.\n/// Multiple Paladin agents may request embeddings simultaneously.\n///\n/// # Implementation Requirements\n///\n/// Implementations should:\n/// 1. Return normalized vectors if using dot product similarity (optional for cosine similarity)\n/// 2. Handle rate limiting with appropriate backoff\n/// 3. Support text chunking for inputs exceeding token limits\n/// 4. Reuse HTTP connections for efficiency (connection pooling)\n/// 5. Cache embeddings for frequently embedded content (optional but recommended)\n///\n/// # Examples\n///\n/// ## Embedding for Semantic Search\n///\n/// ```rust,no_run\n/// use paladin_ports::output::embedding_port::EmbeddingPort;\n/// use paladin_ports::output::sanctum_port::{SanctumPort, SanctumQuery};\n///\n/// async fn semantic_search(\n///     embedder: &dyn EmbeddingPort,\n///     sanctum: &dyn SanctumPort,\n///     query: &str,\n/// ) -> Result<(), Box<dyn std::error::Error>> {\n///     // Embed the query\n///     let embedding = embedder.embed_text(query).await?;\n///     \n///     // Search vector database\n///     let search_query = SanctumQuery::new(embedding.vector, 5);\n///     let results = sanctum.search(search_query).await?;\n///     \n///     println!(\"Found {} similar documents\", results.len());\n///     Ok(())\n/// }\n/// ```\n///\n/// ## Batch Processing for Performance\n///\n/// ```rust,no_run\n/// use paladin_ports::output::embedding_port::EmbeddingPort;\n///\n/// async fn process_documents(\n///     embedder: &dyn EmbeddingPort,\n///     documents: Vec<String>,\n/// ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {\n///     // Process in batches of 100 for optimal performance\n///     let batch_size = 100;\n///     let mut all_embeddings = Vec::new();\n///     \n///     for chunk in documents.chunks(batch_size) {\n///         let refs: Vec<&str> = chunk.iter().map(|s| s.as_str()).collect();\n///         let embeddings = embedder.embed_batch(&refs).await?;\n///         all_embeddings.extend(embeddings.into_iter().map(|e| e.vector));\n///     }\n///     \n///     println!(\"Processed {} documents\", all_embeddings.len());\n///     Ok(all_embeddings)\n/// }\n/// ```\n///\n/// ## Custom Implementation Example\n///\n/// ```rust\n/// use paladin_ports::output::embedding_port::{EmbeddingPort, Embedding, EmbeddingError};\n/// use async_trait::async_trait;\n///\n/// struct MockEmbedder {\n///     dimension: usize,\n/// }\n///\n/// #[async_trait]\n/// impl EmbeddingPort for MockEmbedder {\n///     async fn embed_text(&self, text: &str) -> Result<Embedding, EmbeddingError> {\n///         if text.is_empty() {\n///             return Err(EmbeddingError::InvalidInput(\"Empty text\".to_string()));\n///         }\n///         \n///         // Generate deterministic mock vector\n///         let hash = text.len() as f32;\n///         let vector = vec![hash / 100.0; self.dimension];\n///         \n///         Ok(Embedding {\n///             vector,\n///             model: \"mock-model\".to_string(),\n///             dimension: self.dimension,\n///             token_count: Some(text.split_whitespace().count() as u32),\n///         })\n///     }\n///     \n///     async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {\n///         let mut embeddings = Vec::new();\n///         for text in texts {\n///             embeddings.push(self.embed_text(text).await?);\n///         }\n///         Ok(embeddings)\n///     }\n///     \n///     fn dimension(&self) -> usize {\n///         self.dimension\n///     }\n///     \n///     fn model_name(&self) -> &str {\n///         \"mock-model\"\n///     }\n/// }\n/// ```\n///\n/// # Implementation Notes\n///\n/// ## Provider Selection\n///\n/// Choose based on requirements:\n/// - **OpenAI text-embedding-3-small**: Best cost/performance (1536 dim, $0.02/1M tokens)\n/// - **OpenAI text-embedding-3-large**: Highest quality (3072 dim, $0.13/1M tokens)\n/// - **Cohere embed-english-v3.0**: Good for English (1024 dim)\n/// - **Local Sentence Transformers**: Free, private, offline (384-768 dim)\n///\n/// ## Performance Optimization\n///\n/// ```rust,ignore\n/// // Good: Batch embedding (10-100x faster)\n/// let texts = vec![\"text1\", \"text2\", \"text3\"];\n/// let embeddings = embedder.embed_batch(&texts).await?;\n///\n/// // Avoid: Individual calls in loop\n/// for text in texts {\n///     let embedding = embedder.embed_text(text).await?; // Slow!\n/// }\n///\n/// // Good: Chunk long documents\n/// fn chunk_text(text: &str, max_tokens: usize) -> Vec<String> {\n///     text.split_whitespace()\n///         .collect::<Vec<_>>()\n///         .chunks(max_tokens)\n///         .map(|chunk| chunk.join(\" \"))\n///         .collect()\n/// }\n///\n/// // Good: Cache frequently embedded content\n/// use std::collections::HashMap;\n/// let mut cache: HashMap<String, Embedding> = HashMap::new();\n/// if let Some(cached) = cache.get(text) {\n///     return Ok(cached.clone());\n/// }\n/// ```\n///\n/// ## Error Handling Best Practices\n///\n/// 1. **Rate Limits**: Implement exponential backoff (1s, 2s, 4s, 8s)\n/// 2. **Network Errors**: Retry up to 3 times with 500ms delay\n/// 3. **Invalid Input**: Validate before calling (empty check, length check)\n/// 4. **Provider Errors**: Log and alert (indicates configuration issue)\n///\n/// ## Token Limits by Provider\n///\n/// - OpenAI text-embedding-3-small: 8,191 tokens\n/// - OpenAI text-embedding-3-large: 8,191 tokens\n/// - Cohere embed-english-v3.0: 512 tokens\n/// - Local models: Varies (typically 256-512)\n///\n/// ## Cost Estimation\n///\n/// ```rust,ignore\n/// // OpenAI pricing (as of 2024)\n/// fn estimate_cost(token_count: u32, model: &str) -> f64 {\n///     let cost_per_1m = match model {\n///         \"text-embedding-3-small\" => 0.02,\n///         \"text-embedding-3-large\" => 0.13,\n///         _ => 0.0,\n///     };\n///     (token_count as f64 / 1_000_000.0) * cost_per_1m\n/// }\n/// ```\n///\n/// # Common Pitfalls\n///\n/// - Not using batch API for multiple texts (10-100x slower)\n/// - Mixing embeddings from different models (invalid similarity)\n/// - Not handling rate limits (requests fail unnecessarily)\n/// - Embedding without input validation (wasted API calls)\n/// - Not caching embeddings (repeated costs)\n///\n/// # See Also\n///\n/// - [`Embedding`] - Vector embedding data structure\n/// - [`EmbeddingError`] - Error types and recovery strategies\n/// - [`SanctumPort`](crate::output::sanctum_port::SanctumPort) - Vector storage (uses embeddings)\n/// - [`LongTermGarrisonPort`](crate::output::garrison_port::LongTermGarrisonPort) - Memory with embeddings
#[async_trait::async_trait]
pub trait EmbeddingPort: Send + Sync {
    /// Generate an embedding for a single text string
    ///
    /// # Arguments
    ///
    /// * `text` - The text to embed
    ///
    /// # Returns
    ///
    /// Returns an `Embedding` containing the vector and metadata, or an `EmbeddingError`
    ///
    /// # Errors
    ///
    /// * `NetworkError` - If there's a network issue connecting to the provider
    /// * `RateLimited` - If the rate limit has been exceeded
    /// * `InvalidInput` - If the text is empty or exceeds size limits
    /// * `ProviderError` - If there's an API key issue or other provider-specific error
    async fn embed_text(&self, text: &str) -> Result<Embedding, EmbeddingError>;

    /// Generate embeddings for multiple text strings in a batch
    ///
    /// This is more efficient than calling `embed_text` multiple times,
    /// as it can leverage the provider's batch API.
    ///
    /// # Arguments
    ///
    /// * `texts` - A slice of text strings to embed
    ///
    /// # Returns
    ///
    /// Returns a vector of `Embedding` objects in the same order as the input,
    /// or an `EmbeddingError` if the entire batch fails
    ///
    /// # Errors
    ///
    /// Same error types as `embed_text`, plus provider-specific batch size limits
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError>;

    /// Get the dimension of embeddings produced by this provider
    ///
    /// # Returns
    ///
    /// The number of dimensions in the embedding vector (e.g., 1536, 3072)
    fn dimension(&self) -> usize;

    /// Get the name of the model used by this provider
    ///
    /// # Returns
    ///
    /// The model identifier (e.g., "text-embedding-3-small")
    fn model_name(&self) -> &str;
}
