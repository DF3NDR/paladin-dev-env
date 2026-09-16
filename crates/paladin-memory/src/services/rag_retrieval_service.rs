/// RAG Retrieval Service
///
/// Handles retrieval of relevant memories from long-term storage (Sanctum)
/// for Retrieval-Augmented Generation (RAG).
///
/// This service:
/// - Generates embeddings for queries
/// - Searches Sanctum for similar memories
/// - Filters by similarity threshold
/// - Deduplicates near-identical memories
/// - Ranks by relevance
/// - Rations to fit the injection budget via `Commissary::dispense` (Phase 33, COMM-01)
/// - Formats memories for prompt injection
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub use paladin_llm::services::commissary::ShedItem;
use paladin_llm::services::commissary::{
    Commissary, CommissaryError, CommissaryPlan, Consignment, ConsignmentItem,
};
use paladin_ports::output::embedding_port::EmbeddingPort;
use paladin_ports::output::llm_port::ProviderCapabilities;
use paladin_ports::output::sanctum_port::{
    SanctumError, SanctumFilter, SanctumPort, SanctumQuery, SanctumSearchResult,
};
use paladin_ports::output::token_counter_port::TokenCounterPort;
use thiserror::Error;

// RagConfig and RetrievalTrigger moved to crates/paladin-memory/src/config/rag.rs (Task 6.0)
pub use crate::config::rag::{RagConfig, RetrievalTrigger};

/// A single memory that survived Commissary rationing, in relevance-rank order.
#[derive(Debug, Clone)]
pub struct RagRetainedMemory {
    /// The original Sanctum search result (entry + score) this item was ranked from.
    pub result: SanctumSearchResult,
    /// The post-dispense body — possibly cut and marker-suffixed by the Commissary.
    /// Renderers must print THIS, never `result.entry.memory.content` (D-12).
    pub body: String,
    /// Whether `body` had to be shortened by the Commissary to fit its allotted share.
    pub truncated: bool,
}

/// The result of a rationed RAG retrieval (D-12): the retained memories in
/// relevance-rank order, the shed record, and the Commissary's Stockpile accounting.
///
/// Nothing is dropped silently — every memory that did not survive rationing appears
/// in [`RagRetrievalResult::shed`], and every retained memory whose body was cut
/// carries `truncated: true`.
#[derive(Debug, Clone, Default)]
pub struct RagRetrievalResult {
    /// Retained memories, in descending relevance-score order (ties keep insertion
    /// order — the rank sort is stable).
    pub memories: Vec<RagRetainedMemory>,
    /// Memories shed to stay within `rag.max_tokens`, in shed order. Never silently
    /// dropped — every shed memory is recorded here labelled by its memory UUID (D-09).
    pub shed: Vec<ShedItem>,
    /// The final measured token tally of the rendered context.
    pub prompt_tokens: u32,
    /// The token allowance this retrieval was dispensed against (`rag.max_tokens`,
    /// converted to `u32`).
    pub allotted_tokens: u32,
    /// `true` if `prompt_tokens` is an exact tally (read live from the injected
    /// [`TokenCounterPort::is_exact`]), `false` if it is a deliberately over-counting
    /// estimate.
    pub exact_tally: bool,
}

impl RagRetrievalResult {
    /// The number of retained memories.
    pub fn len(&self) -> usize {
        self.memories.len()
    }

    /// Whether no memories were retained.
    pub fn is_empty(&self) -> bool {
        self.memories.is_empty()
    }

    /// Whether this retrieval shed at least one memory, or truncated at least one
    /// retained memory's body, to fit the budget.
    pub fn was_rationed(&self) -> bool {
        !self.shed.is_empty() || self.memories.iter().any(|m| m.truncated)
    }
}

/// Errors from a rationed RAG retrieval (D-13).
#[derive(Debug, Error)]
pub enum RagRetrievalError {
    /// The underlying Sanctum search or embedding step failed.
    #[error("Sanctum error: {0}")]
    Sanctum(#[from] SanctumError),
    /// The Commissary failed to dispense the retrieved memories.
    #[error("Commissary error: {0}")]
    Commissary(#[from] CommissaryError),
    /// `rag.max_tokens` (a `usize`) does not fit in the `u32` the Commissary takes.
    /// Never clamped — a typed error, per ADR-0004 / D-05.
    #[error("rag.max_tokens ({max_tokens}) exceeds u32::MAX and cannot be rationed")]
    BudgetTooLarge {
        /// The configured `max_tokens` value that failed conversion.
        max_tokens: usize,
    },
    /// A dispensed item's label did not map back to any retrieved memory — an internal
    /// invariant violation. Fails loud rather than silently skipping the item.
    #[error("dispensed item label '{label}' did not match any retrieved memory")]
    UnmatchedDispensedLabel {
        /// The label that failed to resolve.
        label: String,
    },
}

/// Renders the RAG-level omission marker (D-15): a single trailing line naming how
/// many lower-relevance memories were dropped ENTIRELY and the token budget they were
/// rationed against — see the example in `rag_omission_marker_uses_plural_noun_for_multiple_omissions`
/// for the exact rendered shape. Chooses the singular noun (`memory`) when
/// `omitted == 1`, the plural (`memories`) otherwise. The returned string carries a
/// leading and trailing newline so it appends cleanly to either renderer's output
/// without the caller needing to reason about existing trailing whitespace.
///
/// This is the RAG-level marker — which memories were omitted entirely from the
/// result. It is distinct from
/// [`CommissaryPlan::truncation_marker`](paladin_llm::services::commissary::CommissaryPlan::truncation_marker),
/// the PER-ITEM marker the Commissary appends inside a retained body that was cut to
/// its share (D-10). Two markers, two meanings, both visible.
///
/// This is the ONLY place this string is constructed (D-15) — both
/// [`RagRetrievalService::format_for_prompt`] and the facade's
/// `format_retrieved_context` call this function rather than re-typing the literal, so
/// the two renderers can never drift.
pub fn rag_omission_marker(omitted: usize, budget_tokens: u32) -> String {
    let noun = if omitted == 1 { "memory" } else { "memories" };
    format!(
        "\n[{omitted} lower-relevance {noun} omitted to fit the {budget_tokens}-token RAG budget]\n"
    )
}

/// Service for retrieving relevant memories using RAG.
///
/// Depends only on port traits — contains no concrete adapter references.
///
/// # Examples
///
/// ```
/// use paladin_memory::services::rag_retrieval_service::{RagConfig, RagRetrievalService};
/// use paladin_ports::output::embedding_port::EmbeddingPort;
/// use paladin_ports::output::sanctum_port::SanctumPort;
/// use std::sync::Arc;
///
/// fn build(
///     sanctum: Arc<dyn SanctumPort>,
///     embedding: Arc<dyn EmbeddingPort>,
/// ) -> RagRetrievalService {
///     RagRetrievalService::new(sanctum, embedding, RagConfig::default())
/// }
/// ```
pub struct RagRetrievalService {
    sanctum: Arc<dyn SanctumPort>,
    embedding: Arc<dyn EmbeddingPort>,
    config: RagConfig,
    token_counter: Arc<dyn TokenCounterPort>,
}

impl RagRetrievalService {
    /// Create a new RAG retrieval service.
    ///
    /// # Arguments
    ///
    /// * `sanctum` - Vector storage port for memory retrieval
    /// * `embedding` - Embedding generation port for query vectorization
    /// * `config` - RAG configuration parameters
    ///
    /// Defaults the token counter to [`crate::token_counter::HeuristicTokenCounter`]
    /// (D-07) — use [`Self::with_token_counter`] to inject an exact counter such as
    /// `TiktokenCounter`.
    pub fn new(
        sanctum: Arc<dyn SanctumPort>,
        embedding: Arc<dyn EmbeddingPort>,
        config: RagConfig,
    ) -> Self {
        Self {
            sanctum,
            embedding,
            config,
            token_counter: Arc::new(crate::token_counter::HeuristicTokenCounter),
        }
    }

    /// Sets the token-counting port the Commissary rations against (D-07). Mirrors
    /// `PaladinExecutionService::with_token_counter` exactly, so a caller who injects
    /// an exact counter (e.g. `TiktokenCounter`) there can inject it here too.
    ///
    /// # Arguments
    ///
    /// * `counter` - The token-counting adapter to use
    ///
    /// # Returns
    ///
    /// Returns self for method chaining.
    pub fn with_token_counter(mut self, counter: Arc<dyn TokenCounterPort>) -> Self {
        self.token_counter = counter;
        self
    }

    /// Constructs a per-call [`Commissary`] over synthetic capabilities for `budget`
    /// (D-04, D-06). RAG has no provider window, only an injection cap, so the
    /// capabilities are synthesized: provider label `"rag"`,
    /// `max_context_tokens: Some(budget)`, and a [`CommissaryPlan`] with
    /// `reserved_completion_tokens: 0` (the whole budget is for memories) and
    /// `fallback_context_tokens: None`.
    fn commissary(&self, budget: u32) -> Result<Commissary, RagRetrievalError> {
        let capabilities = ProviderCapabilities {
            max_context_tokens: Some(budget),
            ..ProviderCapabilities::default()
        };
        let plan = CommissaryPlan {
            reserved_completion_tokens: 0,
            fallback_context_tokens: None,
            ..CommissaryPlan::default()
        };
        let commissary =
            Commissary::new("rag", capabilities, Arc::clone(&self.token_counter), plan)?;
        Ok(commissary)
    }

    /// Rations `ranked` (already sorted by score descending, stable) through
    /// [`Commissary::dispense`], building a [`Consignment`] whose priority mirrors
    /// rank order: item `i` gets `priority = u8::try_from(i).unwrap_or(u8::MAX)` (D-08)
    /// — lower number == higher priority == shed last, so "the highest-scoring
    /// memories are the ones retained" is a structural guarantee, not a rounding
    /// property. Equal scores keep insertion order — the rank sort is stable.
    ///
    /// Consequence (D-10(a)): a single memory larger than the whole budget is
    /// RETAINED, TRUNCATED with the Commissary's per-item marker, never silently
    /// dropped — the opposite of the old byte-length helper's behaviour.
    fn ration(
        &self,
        ranked: Vec<SanctumSearchResult>,
    ) -> Result<RagRetrievalResult, RagRetrievalError> {
        if ranked.is_empty() {
            return Ok(RagRetrievalResult::default());
        }

        let budget = u32::try_from(self.config.max_tokens).map_err(|_| {
            RagRetrievalError::BudgetTooLarge {
                max_tokens: self.config.max_tokens,
            }
        })?;
        let commissary = self.commissary(budget)?;

        let mut consignment = Consignment::new();
        let mut index_by_label: HashMap<String, usize> = HashMap::with_capacity(ranked.len());
        for (rank, result) in ranked.iter().enumerate() {
            let label = result.entry.memory.id.to_string();
            index_by_label.insert(label.clone(), rank);
            consignment.push(ConsignmentItem {
                label,
                body: result.entry.memory.content.clone(),
                priority: u8::try_from(rank).unwrap_or(u8::MAX),
            });
        }

        let stockpile = commissary.dispense("", &consignment)?;

        let mut memories = Vec::with_capacity(stockpile.dispensed.len());
        for item in stockpile.dispensed {
            let idx = *index_by_label.get(&item.label).ok_or_else(|| {
                RagRetrievalError::UnmatchedDispensedLabel {
                    label: item.label.clone(),
                }
            })?;
            memories.push(RagRetainedMemory {
                result: ranked[idx].clone(),
                body: item.body,
                truncated: item.truncated,
            });
        }

        Ok(RagRetrievalResult {
            memories,
            shed: stockpile.shed,
            prompt_tokens: stockpile.prompt_tokens,
            allotted_tokens: stockpile.allotted_tokens,
            exact_tally: stockpile.exact_tally,
        })
    }

    /// Retrieve relevant memories for a given query.
    ///
    /// # Arguments
    ///
    /// * `paladin_id` - ID of the Paladin requesting memories
    /// * `query` - The query text to find relevant memories for
    ///
    /// # Returns
    ///
    /// A [`RagRetrievalResult`] carrying the retained memories (descending relevance
    /// order), the shed record, and the Commissary's Stockpile accounting.
    ///
    /// # Errors
    ///
    /// Returns [`RagRetrievalError`] if embedding generation, search, or Commissary
    /// rationing fails.
    pub async fn retrieve_context(
        &self,
        paladin_id: &str,
        query: &str,
    ) -> Result<RagRetrievalResult, RagRetrievalError> {
        // Generate query embedding
        let embedding_result = self.embedding.embed_text(query).await.map_err(|e| {
            SanctumError::SearchError(format!("Embedding generation failed: {}", e))
        })?;

        // Build filter for this Paladin
        let filter = SanctumFilter::new().paladin_id(paladin_id.to_string());

        // Build search query
        let sanctum_query = SanctumQuery::new(embedding_result.vector, self.config.top_k)
            .with_filter(filter)
            .with_min_score(self.config.min_similarity);

        // Execute search
        let mut results = self.sanctum.search(sanctum_query).await?;

        log::debug!(
            "Retrieved {} memories for paladin {} with query: {}",
            results.len(),
            paladin_id,
            query
        );

        // Apply post-processing
        results = self.filter_by_similarity(results);
        results = self.deduplicate_memories(results);
        results = self.rank_by_relevance(results);

        let rationed = self.ration(results)?;

        log::info!(
            "RAG rationing: retained={}, shed={}, prompt_tokens={}, allotted_tokens={}, exact_tally={}",
            rationed.memories.len(),
            rationed.shed.len(),
            rationed.prompt_tokens,
            rationed.allotted_tokens,
            rationed.exact_tally
        );

        Ok(rationed)
    }

    /// Filter results by minimum similarity threshold.
    fn filter_by_similarity(&self, results: Vec<SanctumSearchResult>) -> Vec<SanctumSearchResult> {
        results
            .into_iter()
            .filter(|r| r.score >= self.config.min_similarity)
            .collect()
    }

    /// Deduplicate near-identical memories (>0.95 similarity).
    ///
    /// Removes memories that are very similar to each other, keeping only
    /// the highest-scoring instance.
    fn deduplicate_memories(
        &self,
        mut results: Vec<SanctumSearchResult>,
    ) -> Vec<SanctumSearchResult> {
        if results.len() <= 1 {
            return results;
        }

        // Sort by score descending to keep highest-scoring duplicates
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        let original_count = results.len();
        let mut deduplicated = Vec::new();
        let mut seen_contents = HashSet::new();

        for result in results {
            // Use content as deduplication key
            let content_key = result.entry.memory.content.trim().to_lowercase();

            // Check if we've seen very similar content
            let is_duplicate = seen_contents.iter().any(|seen: &String| {
                // Simple similarity: check if content is substring or vice versa
                content_key.contains(seen) || seen.contains(&content_key)
            });

            if !is_duplicate {
                seen_contents.insert(content_key);
                deduplicated.push(result);
            }
        }

        log::debug!(
            "Deduplication: {} -> {} memories",
            original_count,
            deduplicated.len()
        );

        deduplicated
    }

    /// Rank memories by relevance score (descending).
    fn rank_by_relevance(&self, mut results: Vec<SanctumSearchResult>) -> Vec<SanctumSearchResult> {
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Format retrieved memories for prompt injection.
    ///
    /// Creates a structured text block suitable for including in the system
    /// prompt or user message. Prints [`RagRetainedMemory::body`] (the post-dispense
    /// body) rather than the raw memory content, so a truncated excerpt is rendered
    /// exactly as the Commissary retained it.
    ///
    /// # Arguments
    ///
    /// * `result` - The rationed retrieval result to format
    ///
    /// # Returns
    ///
    /// A formatted string containing the relevant context section.
    pub fn format_for_prompt(&self, result: &RagRetrievalResult) -> String {
        if result.memories.is_empty() {
            return String::new();
        }

        let mut formatted = String::from("## Relevant Context\n\n");
        formatted.push_str("The following memories may be relevant to your current task:\n\n");

        for (idx, retained) in result.memories.iter().enumerate() {
            let memory = &retained.result.entry.memory;

            formatted.push_str(&format!(
                "**Memory {}** (Similarity: {:.2})\n",
                idx + 1,
                retained.result.score
            ));
            formatted.push_str(&format!("Type: {:?}\n", memory.memory_type));
            formatted.push_str(&format!("Content: {}\n", retained.body));
            formatted.push_str(&format!(
                "Source: Conversation on {}\n\n",
                memory.created_at.format("%Y-%m-%d")
            ));
        }

        formatted.push_str("---\n\n");

        if !result.shed.is_empty() {
            formatted.push_str(&rag_omission_marker(
                result.shed.len(),
                result.allotted_tokens,
            ));
        }

        formatted
    }
}

/// Async wrapper for [`RagRetrievalService::retrieve_context`] with a timeout.
///
/// Returns an empty [`RagRetrievalResult`] on timeout to enable graceful degradation.
pub async fn retrieve_context_with_timeout(
    service: &RagRetrievalService,
    paladin_id: &str,
    query: &str,
    timeout_secs: u64,
) -> Result<RagRetrievalResult, RagRetrievalError> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        service.retrieve_context(paladin_id, query),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            log::warn!(
                "Memory retrieval timed out after {} seconds, continuing with empty context",
                timeout_secs
            );
            Ok(RagRetrievalResult::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use paladin_core::platform::container::sanctum::{MemoryBuilder, MemoryType, SanctumEntry};
    use paladin_ports::output::embedding_port::{Embedding, EmbeddingError, EmbeddingPort};
    use paladin_ports::output::sanctum_port::{
        SanctumError, SanctumFilter, SanctumPort, SanctumQuery, SanctumSearchResult,
    };
    use proptest::prelude::*;
    use std::sync::Arc;
    use uuid::Uuid;

    // ── Mock helpers ──────────────────────────────────────────────────────────

    struct MockEmbeddingPort;

    #[async_trait]
    impl EmbeddingPort for MockEmbeddingPort {
        async fn embed_text(&self, _text: &str) -> Result<Embedding, EmbeddingError> {
            Ok(Embedding {
                vector: vec![0.1, 0.2, 0.3, 0.4, 0.5],
                model: "mock-model".to_string(),
                dimension: 5,
                token_count: Some(10),
            })
        }

        async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
            Ok(texts
                .iter()
                .map(|_| Embedding {
                    vector: vec![0.1, 0.2, 0.3, 0.4, 0.5],
                    model: "mock-model".to_string(),
                    dimension: 5,
                    token_count: Some(10),
                })
                .collect())
        }

        fn dimension(&self) -> usize {
            5
        }

        fn model_name(&self) -> &str {
            "mock-model"
        }
    }

    struct MockSanctumPort {
        results: Vec<SanctumSearchResult>,
    }

    #[async_trait]
    impl SanctumPort for MockSanctumPort {
        async fn store(&self, _entry: SanctumEntry) -> Result<(), SanctumError> {
            Ok(())
        }

        async fn store_batch(&self, _entries: Vec<SanctumEntry>) -> Result<(), SanctumError> {
            Ok(())
        }

        async fn search(
            &self,
            _query: SanctumQuery,
        ) -> Result<Vec<SanctumSearchResult>, SanctumError> {
            Ok(self.results.clone())
        }

        async fn delete(&self, _id: &str) -> Result<bool, SanctumError> {
            Ok(true)
        }

        async fn update(&self, _entry: SanctumEntry) -> Result<(), SanctumError> {
            Ok(())
        }

        async fn count(&self, _filter: Option<SanctumFilter>) -> Result<usize, SanctumError> {
            Ok(self.results.len())
        }
    }

    fn create_test_entry(
        paladin_id: &str,
        content: &str,
        importance: f32,
        score: f32,
    ) -> SanctumSearchResult {
        let memory = MemoryBuilder::new(paladin_id.to_string(), content.to_string())
            .importance(importance)
            .memory_type(MemoryType::Semantic)
            .build()
            .unwrap();
        let entry = SanctumEntry::new(memory, vec![0.1, 0.2, 0.3, 0.4, 0.5]).unwrap();
        SanctumSearchResult::new(entry, score)
    }

    /// Wraps already-retrieved [`SanctumSearchResult`]s directly into a
    /// [`RagRetrievalResult`], with each body sourced verbatim from the memory's
    /// content and `truncated: false` — for tests that exercise the renderer without
    /// going through a real Commissary dispense.
    fn build_result(memories: Vec<SanctumSearchResult>) -> RagRetrievalResult {
        let retained = memories
            .into_iter()
            .map(|result| {
                let body = result.entry.memory.content.clone();
                RagRetainedMemory {
                    result,
                    body,
                    truncated: false,
                }
            })
            .collect();
        RagRetrievalResult {
            memories: retained,
            ..RagRetrievalResult::default()
        }
    }

    // ── Config / trigger tests ────────────────────────────────────────────────

    #[test]
    fn test_rag_config_builder() {
        let config = RagConfig {
            top_k: 10,
            min_similarity: 0.8,
            max_tokens: 3000,
            timeout_seconds: 5,
            retrieval_trigger: RetrievalTrigger::KeywordBased,
        };

        assert_eq!(config.top_k, 10);
        assert_eq!(config.min_similarity, 0.8);
        assert_eq!(config.max_tokens, 3000);
    }

    #[test]
    fn test_retrieval_trigger_equality() {
        assert_eq!(RetrievalTrigger::Always, RetrievalTrigger::Always);
        assert_ne!(RetrievalTrigger::Always, RetrievalTrigger::KeywordBased);
    }

    #[test]
    fn test_rag_config_default() {
        let config = RagConfig::default();
        assert_eq!(config.top_k, 5);
        assert_eq!(config.min_similarity, 0.7);
        assert_eq!(config.max_tokens, 2000);
    }

    #[test]
    fn test_retrieval_trigger_variants() {
        let trigger = RetrievalTrigger::Always;
        assert!(matches!(trigger, RetrievalTrigger::Always));
    }

    // ── Async retrieval tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_successful_retrieval_with_multiple_memories() {
        let mock_results = vec![
            create_test_entry("paladin-1", "Memory 1", 0.9, 0.95),
            create_test_entry("paladin-1", "Memory 2", 0.8, 0.85),
            create_test_entry("paladin-1", "Memory 3", 0.7, 0.75),
        ];

        let sanctum = Arc::new(MockSanctumPort {
            results: mock_results,
        });
        let embedding = Arc::new(MockEmbeddingPort);
        let config = RagConfig::default();

        let service = RagRetrievalService::new(sanctum, embedding, config);
        let results = service
            .retrieve_context("paladin-1", "test query")
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_filtering_by_min_similarity() {
        let mock_results = vec![
            create_test_entry("paladin-1", "High score", 0.9, 0.95),
            create_test_entry("paladin-1", "Medium score", 0.8, 0.75),
            create_test_entry("paladin-1", "Low score", 0.7, 0.50),
        ];

        let sanctum = Arc::new(MockSanctumPort {
            results: mock_results,
        });
        let embedding = Arc::new(MockEmbeddingPort);
        let config = RagConfig {
            min_similarity: 0.7,
            ..Default::default()
        };

        let service = RagRetrievalService::new(sanctum, embedding, config);
        let results = service
            .retrieve_context("paladin-1", "test query")
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.memories.iter().all(|m| m.result.score >= 0.7));
    }

    #[test]
    fn test_format_for_prompt() {
        let memories = vec![
            create_test_entry("paladin-1", "First memory", 0.9, 0.95),
            create_test_entry("paladin-1", "Second memory", 0.8, 0.85),
        ];

        let sanctum = Arc::new(MockSanctumPort { results: vec![] });
        let embedding = Arc::new(MockEmbeddingPort);
        let service = RagRetrievalService::new(sanctum, embedding, RagConfig::default());

        let result = build_result(memories);
        let formatted = service.format_for_prompt(&result);

        assert!(formatted.contains("## Relevant Context"));
        assert!(formatted.contains("First memory"));
        assert!(formatted.contains("Second memory"));
        assert!(formatted.contains("0.95"));
        assert!(formatted.contains("0.85"));
    }

    #[tokio::test]
    async fn test_empty_results_graceful_handling() {
        let sanctum = Arc::new(MockSanctumPort { results: vec![] });
        let embedding = Arc::new(MockEmbeddingPort);
        let service = RagRetrievalService::new(sanctum, embedding, RagConfig::default());

        let results = service
            .retrieve_context("paladin-1", "test query")
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    // ── Commissary rationing tracer (Phase 33, COMM-01) ─────────────────────────

    /// End-to-end proof that a real retrieval is rationed through
    /// `Commissary::dispense`: a budget too small for all three memories sheds the
    /// lowest-scoring ones (recorded, never silently dropped), keeps the retained
    /// memory in descending-score order, and never loses or duplicates an id across
    /// `memories` and `shed`. Distinct, non-round body sizes (987 / 654 / 321 bytes)
    /// so a swapped priority could not pass by coincidence (Phase 31 house style).
    ///
    /// The budget (233 tokens) is chosen small enough to force at least one shed
    /// under the Commissary's default pessimistic byte-planning ratio — a budget of
    /// 1,234 tokens (the figure sketched for the property test's random-input
    /// strategy) yields an allowance of ~3,446 bytes, comfortably fitting the
    /// 1,962-byte sum of all three bodies here and so would shed nothing.
    #[tokio::test]
    async fn commissary_rations_rag_retrieval_end_to_end() {
        let entry_a = create_test_entry("paladin-1", &"A".repeat(987), 0.9, 0.95);
        let entry_b = create_test_entry("paladin-1", &"B".repeat(654), 0.8, 0.85);
        let entry_c = create_test_entry("paladin-1", &"C".repeat(321), 0.7, 0.75);

        let input_ids: HashSet<String> = [&entry_a, &entry_b, &entry_c]
            .into_iter()
            .map(|r| r.entry.memory.id.to_string())
            .collect();

        let sanctum = Arc::new(MockSanctumPort {
            results: vec![entry_a, entry_b, entry_c],
        });
        let embedding = Arc::new(MockEmbeddingPort);
        let config = RagConfig {
            max_tokens: 233,
            ..RagConfig::default()
        };

        let service = RagRetrievalService::new(sanctum, embedding, config);
        let result = service
            .retrieve_context("paladin-1", "test query")
            .await
            .expect("retrieval should succeed");

        assert!(
            !result.shed.is_empty(),
            "a tight budget must shed at least the lowest-scoring memory"
        );
        for shed in &result.shed {
            Uuid::parse_str(&shed.label)
                .unwrap_or_else(|e| panic!("shed label '{}' is not a UUID: {e}", shed.label));
        }
        assert!(
            result.prompt_tokens <= result.allotted_tokens,
            "prompt_tokens ({}) must not exceed allotted_tokens ({})",
            result.prompt_tokens,
            result.allotted_tokens
        );

        let scores: Vec<f32> = result.memories.iter().map(|m| m.result.score).collect();
        let mut sorted_desc = scores.clone();
        sorted_desc.sort_by(|a, b| b.partial_cmp(a).unwrap());
        assert_eq!(
            scores, sorted_desc,
            "retained memories must come back in descending-score order"
        );

        let mut seen_ids: HashSet<String> = result
            .memories
            .iter()
            .map(|m| m.result.entry.memory.id.to_string())
            .collect();
        for shed in &result.shed {
            assert!(
                seen_ids.insert(shed.label.clone()),
                "id {} appeared more than once across memories and shed",
                shed.label
            );
        }
        assert_eq!(
            seen_ids, input_ids,
            "every input memory id must appear exactly once across memories and shed"
        );
    }

    // ── RAG omission marker tests (Phase 33, COMM-02, D-15) ─────────────────────

    #[test]
    fn rag_omission_marker_uses_plural_noun_for_multiple_omissions() {
        let marker = rag_omission_marker(3, 2_000);
        assert_eq!(
            marker,
            "\n[3 lower-relevance memories omitted to fit the 2000-token RAG budget]\n"
        );
    }

    #[test]
    fn rag_omission_marker_uses_singular_noun_for_one_omission() {
        let marker = rag_omission_marker(1, 1_234);
        assert_eq!(
            marker,
            "\n[1 lower-relevance memory omitted to fit the 1234-token RAG budget]\n"
        );
    }

    #[test]
    fn format_for_prompt_ends_with_marker_when_shed_nonempty() {
        let memories = vec![create_test_entry("paladin-1", "Kept memory", 0.9, 0.95)];
        let sanctum = Arc::new(MockSanctumPort { results: vec![] });
        let embedding = Arc::new(MockEmbeddingPort);
        let service = RagRetrievalService::new(sanctum, embedding, RagConfig::default());

        let mut result = build_result(memories);
        result.shed = vec![ShedItem {
            label: Uuid::new_v4().to_string(),
            priority: 1,
            original_bytes: 512,
        }];
        result.allotted_tokens = 1_234;

        let formatted = service.format_for_prompt(&result);
        assert!(
            formatted.ends_with(&rag_omission_marker(1, 1_234)),
            "expected formatted output to end with the shared omission marker, got: {formatted}"
        );
    }

    #[test]
    fn format_for_prompt_contains_no_marker_when_shed_empty() {
        let memories = vec![create_test_entry("paladin-1", "Kept memory", 0.9, 0.95)];
        let sanctum = Arc::new(MockSanctumPort { results: vec![] });
        let embedding = Arc::new(MockEmbeddingPort);
        let service = RagRetrievalService::new(sanctum, embedding, RagConfig::default());

        let result = build_result(memories);
        assert!(result.shed.is_empty());

        let formatted = service.format_for_prompt(&result);
        assert!(!formatted.contains("omitted to fit"));
    }

    /// Named edge test (COMM-02): an empty retrieval renders the empty string with no
    /// shed record and no omission marker.
    #[test]
    fn format_for_prompt_empty_retrieval_has_no_shed_and_no_marker() {
        let sanctum = Arc::new(MockSanctumPort { results: vec![] });
        let embedding = Arc::new(MockEmbeddingPort);
        let service = RagRetrievalService::new(sanctum, embedding, RagConfig::default());

        let result = RagRetrievalResult::default();
        assert!(result.shed.is_empty());

        let formatted = service.format_for_prompt(&result);
        assert_eq!(formatted, "");
        assert!(!formatted.contains("omitted to fit"));
    }

    /// Named edge test (COMM-02): a single memory that fits the budget is retained
    /// whole (`truncated == false`), `shed` is empty, and no marker is emitted.
    #[test]
    fn format_for_prompt_single_fitting_memory_has_no_marker() {
        let memory = create_test_entry("paladin-1", "Small memory", 0.9, 0.95);
        let sanctum = Arc::new(MockSanctumPort { results: vec![] });
        let embedding = Arc::new(MockEmbeddingPort);
        let service = RagRetrievalService::new(sanctum, embedding, RagConfig::default());

        let result = build_result(vec![memory]);
        assert_eq!(result.memories.len(), 1);
        assert!(!result.memories[0].truncated);
        assert!(result.shed.is_empty());

        let formatted = service.format_for_prompt(&result);
        assert!(!formatted.contains("omitted to fit"));
    }

    // ── Property test: the rationing seam (Phase 33, COMM-01, D-17) ─────────────

    proptest! {
        /// For random `(content, score)` pairs and random budgets, driving the SYNC
        /// `ration` seam directly (D-14) — never `retrieve_context`, so no async runtime
        /// is needed inside this proptest closure — this proves the three COMM-01
        /// invariants hold over generated input, not just the single fixed example in
        /// `commissary_rations_rag_retrieval_end_to_end`: (i) the retained total never
        /// exceeds the budget; (ii) no shed memory outscores a retained one; (iii)
        /// retained union shed equals the input by id — nothing is lost.
        ///
        /// Property (i) is scoped around the D-10(a) edge: when exactly one memory
        /// survives AND had to be truncated, `Commissary::dispense`'s own documented
        /// contract ("n == 1: nothing lower-priority remains to shed — the single
        /// retained item proceeds with whatever share it was clamped to") does not
        /// re-check the byte budget before returning, so the final marker-suffixed body
        /// can measure over budget for a sufficiently tiny budget or sufficiently
        /// multi-byte content (a `Commissary` behaviour this phase's `<domain>` boundary
        /// excludes from change). That single named edge already has its own dedicated
        /// test (`single_memory_larger_than_budget_is_retained_truncated_not_dropped`);
        /// this property instead proves the general case, where `Commissary::dispense`
        /// verifies byte-budget compliance before returning (`n >= 2`, or `n == 1` with
        /// no truncation needed) — and byte compliance always implies token compliance
        /// here, because the heuristic counter's `chars / 4` never exceeds the planning
        /// ratio's `bytes * 358 / 1000` (UTF-8 never has fewer bytes than chars).
        #[test]
        fn ration_respects_budget_and_rank_order(
            items in prop::collection::vec((".{1,600}", 0.0f32..=1.0f32), 0..=20),
            budget in 1u32..=2_000u32,
        ) {
            let sanctum = Arc::new(MockSanctumPort { results: vec![] });
            let embedding = Arc::new(MockEmbeddingPort);
            let config = RagConfig {
                max_tokens: budget as usize,
                ..RagConfig::default()
            };
            let service = RagRetrievalService::new(sanctum, embedding, config);

            // Deliberately unsorted input — `rank_by_relevance` does the sorting, and
            // the rank-order property below is exercised against this unsorted start.
            let mut score_by_id: HashMap<String, f32> = HashMap::with_capacity(items.len());
            let mut unranked = Vec::with_capacity(items.len());
            for (content, score) in &items {
                let entry = create_test_entry("paladin-1", content, 0.5, *score);
                let id = entry.entry.memory.id.to_string();
                score_by_id.insert(id, *score);
                unranked.push(entry);
            }
            let input_ids: HashSet<String> = score_by_id.keys().cloned().collect();

            let ranked = service.rank_by_relevance(unranked);
            let result = service
                .ration(ranked)
                .expect("a budget in 1..=2_000 over an empty fixed section never fails to dispense");

            // (i) the retained total never exceeds the budget -- outside the D-10(a)
            // single-truncated-survivor edge, which `Commissary::dispense` itself does
            // not budget-check before returning (see the doc comment above).
            let is_single_truncated_survivor =
                result.memories.len() == 1 && result.memories[0].truncated;
            if !is_single_truncated_survivor {
                prop_assert!(
                    result.prompt_tokens <= budget,
                    "prompt_tokens ({}) exceeded budget ({budget})",
                    result.prompt_tokens
                );
            }

            // (ii) no shed memory outscores a retained one.
            for shed in &result.shed {
                let shed_score = *score_by_id
                    .get(&shed.label)
                    .expect("every shed label must resolve back to a generated input score");
                for retained in &result.memories {
                    prop_assert!(
                        shed_score <= retained.result.score,
                        "shed memory (score {shed_score}) outscored a retained memory (score {})",
                        retained.result.score
                    );
                }
            }

            // (iii) retained ∪ shed equals the input by id — nothing lost.
            let mut output_ids: HashSet<String> = result
                .memories
                .iter()
                .map(|m| m.result.entry.memory.id.to_string())
                .collect();
            for shed in &result.shed {
                output_ids.insert(shed.label.clone());
            }
            prop_assert_eq!(output_ids, input_ids);
        }
    }

    // ── Named edge tests: decisions D-05, D-08, D-10 (Phase 33) ──────────────────

    /// D-10(a) edge: one memory whose body is far larger than the whole allowance is
    /// RETAINED, TRUNCATED with the Commissary's per-item marker — never shed, never
    /// dropped. This is the exact opposite of the pre-phase `truncate_to_token_budget`
    /// behaviour, which silently dropped it and returned nothing at all.
    #[test]
    fn single_memory_larger_than_budget_is_retained_truncated_not_dropped() {
        let sanctum = Arc::new(MockSanctumPort { results: vec![] });
        let embedding = Arc::new(MockEmbeddingPort);
        let config = RagConfig {
            max_tokens: 1_234,
            ..RagConfig::default()
        };
        let service = RagRetrievalService::new(sanctum, embedding, config);

        let oversized_body = "Z".repeat(12_345);
        let entry = create_test_entry("paladin-1", &oversized_body, 0.9, 0.95);

        let result = service
            .ration(vec![entry])
            .expect("a single oversized memory is retained truncated, never an error");

        assert_eq!(result.memories.len(), 1);
        assert!(
            result.memories[0].truncated,
            "an oversized single memory must be marked truncated"
        );
        assert!(
            result.memories[0]
                .body
                .ends_with(&CommissaryPlan::default().truncation_marker),
            "the retained body must end with the Commissary's per-item truncation marker"
        );
        assert!(
            result.shed.is_empty(),
            "an oversized single memory is retained truncated, never shed"
        );
        assert!(
            result.memories[0].body.len() < oversized_body.len(),
            "the retained body must be shorter than the original content"
        );
    }

    /// D-05 edge: `rag.max_tokens` beyond `u32::MAX` returns the typed budget-conversion
    /// error and never a wrapped, clamped or zero budget.
    #[test]
    fn budget_beyond_u32_returns_typed_error_and_never_clamps() {
        let sanctum = Arc::new(MockSanctumPort { results: vec![] });
        let embedding = Arc::new(MockEmbeddingPort);
        let config = RagConfig {
            max_tokens: usize::MAX,
            ..RagConfig::default()
        };
        let service = RagRetrievalService::new(sanctum, embedding, config);

        let entry = create_test_entry("paladin-1", "some memory content", 0.5, 0.5);

        match service.ration(vec![entry]) {
            Err(RagRetrievalError::BudgetTooLarge { max_tokens }) => {
                assert_eq!(max_tokens, usize::MAX);
                let message = RagRetrievalError::BudgetTooLarge { max_tokens }.to_string();
                assert!(
                    message.contains(&usize::MAX.to_string()),
                    "the error message must name the configured value, got: {message}"
                );
            }
            other => panic!(
                "expected Err(RagRetrievalError::BudgetTooLarge {{ .. }}), got {other:?} -- \
                 a budget beyond u32::MAX must never produce a reduced-budget Ok result"
            ),
        }
    }

    /// D-08 edge: memories whose scores are exactly equal keep insertion order through
    /// the rank sort and receive adjacent, distinct rank priorities — equal scores never
    /// merge into one item and never reorder.
    #[test]
    fn equal_scores_keep_insertion_order_and_distinct_priorities() {
        let sanctum = Arc::new(MockSanctumPort { results: vec![] });
        let embedding = Arc::new(MockEmbeddingPort);
        let config = RagConfig {
            max_tokens: 5_678,
            ..RagConfig::default()
        };
        let service = RagRetrievalService::new(sanctum, embedding, config);

        // `tied_first` and `tied_second` carry byte-identical scores; `highest` outranks
        // both.
        let tied_first = create_test_entry("paladin-1", &"A".repeat(145), 0.5, 0.6);
        let tied_second = create_test_entry("paladin-1", &"B".repeat(267), 0.5, 0.6);
        let highest = create_test_entry("paladin-1", &"C".repeat(389), 0.5, 0.9);

        let tied_first_id = tied_first.entry.memory.id.to_string();
        let tied_second_id = tied_second.entry.memory.id.to_string();
        let highest_id = highest.entry.memory.id.to_string();

        let ranked = service.rank_by_relevance(vec![tied_first, tied_second, highest]);
        let result = service
            .ration(ranked)
            .expect("a generous budget over three small memories never fails to dispense");

        assert!(
            result.shed.is_empty(),
            "a generous budget must not shed any memory"
        );
        assert_eq!(
            result.memories.len() + result.shed.len(),
            3,
            "nothing must be merged -- three inputs must yield three outputs total"
        );

        let ids: Vec<String> = result
            .memories
            .iter()
            .map(|m| m.result.entry.memory.id.to_string())
            .collect();
        let unique_ids: HashSet<&String> = ids.iter().collect();
        assert_eq!(unique_ids.len(), 3, "all three ids must remain distinct");

        let tied_first_pos = ids
            .iter()
            .position(|id| id == &tied_first_id)
            .expect("tied_first must be retained");
        let tied_second_pos = ids
            .iter()
            .position(|id| id == &tied_second_id)
            .expect("tied_second must be retained");
        assert!(
            tied_first_pos < tied_second_pos,
            "equal-scoring memories must keep their pre-sort insertion order"
        );
        assert!(ids.contains(&highest_id));
    }

    /// The budget boundary (COMM-01): a set that fits is retained whole with no shed and
    /// no cut; one memory past the allowance sheds the lowest-priority item rather than
    /// cutting every item.
    #[test]
    fn at_the_budget_boundary_nothing_is_shed_and_one_past_it_sheds_the_lowest() {
        let highest = create_test_entry("paladin-1", &"A".repeat(101), 0.9, 0.97);
        let middle = create_test_entry("paladin-1", &"B".repeat(103), 0.8, 0.83);
        let lowest = create_test_entry("paladin-1", &"C".repeat(107), 0.7, 0.61);

        let highest_id = highest.entry.memory.id.to_string();
        let middle_id = middle.entry.memory.id.to_string();
        let lowest_id = lowest.entry.memory.id.to_string();

        // Run 1: a generous budget -- the whole set fits, nothing shed, nothing cut.
        {
            let sanctum = Arc::new(MockSanctumPort { results: vec![] });
            let embedding = Arc::new(MockEmbeddingPort);
            let config = RagConfig {
                max_tokens: 4_321,
                ..RagConfig::default()
            };
            let service = RagRetrievalService::new(sanctum, embedding, config);
            let ranked =
                service.rank_by_relevance(vec![highest.clone(), middle.clone(), lowest.clone()]);
            let result = service
                .ration(ranked)
                .expect("a generous budget over three small memories never fails to dispense");

            assert!(
                result.shed.is_empty(),
                "a set that fits must be retained whole with no shed"
            );
            assert!(
                result.memories.iter().all(|m| !m.truncated),
                "a set that fits must have no cut memories"
            );
            assert!(result.prompt_tokens <= result.allotted_tokens);
        }

        // Run 2: a tight budget -- one past the allowance sheds the lowest-priority item.
        {
            let sanctum = Arc::new(MockSanctumPort { results: vec![] });
            let embedding = Arc::new(MockEmbeddingPort);
            let config = RagConfig {
                max_tokens: 75,
                ..RagConfig::default()
            };
            let service = RagRetrievalService::new(sanctum, embedding, config);
            let ranked =
                service.rank_by_relevance(vec![highest.clone(), middle.clone(), lowest.clone()]);
            let result = service
                .ration(ranked)
                .expect("a tight budget still dispenses -- it sheds, it does not error");

            assert_eq!(
                result.shed.len(),
                1,
                "exactly one memory must be shed at this budget"
            );
            assert_eq!(
                result.shed[0].label, lowest_id,
                "the shed memory must be the lowest-scoring one"
            );

            let retained_ids: HashSet<String> = result
                .memories
                .iter()
                .map(|m| m.result.entry.memory.id.to_string())
                .collect();
            assert!(
                retained_ids.contains(&highest_id),
                "the two highest-scoring memories must still be present"
            );
            assert!(
                retained_ids.contains(&middle_id),
                "the two highest-scoring memories must still be present"
            );
            assert!(result.prompt_tokens <= result.allotted_tokens);
        }
    }
}
