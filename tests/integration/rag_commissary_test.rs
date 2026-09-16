//! Integration test: RAG retrieval rationed end-to-end through the real Commissary,
//! reached ONLY via the published facade path (Phase 33, COMM-03 -- the F4 "production
//! caller exercised by integration tests" evidence).
//!
//! Ungated -- no service, no feature flag, no `required-features` on the `[[test]]`
//! target. It proves `RagRetrievalService::retrieve_context` rations a real
//! `InMemorySanctum` search result through `Commissary::dispense`, reached over the
//! SAME re-export path (`paladin::application::services::sanctum`) a downstream
//! application would use -- not the crate-internal `paladin_memory::services` path.

use paladin::application::services::sanctum::{
    RagConfig, RagRetrievalResult, RagRetrievalService, ShedItem, rag_omission_marker,
};
use paladin::core::platform::container::sanctum::{
    Memory, MemoryBuilder, MemoryType, SanctumEntry,
};
use paladin::infrastructure::adapters::sanctum::InMemorySanctum;
use paladin_ports::output::embedding_port::{Embedding, EmbeddingError, EmbeddingPort};
use paladin_ports::output::sanctum_port::SanctumPort;
use std::sync::Arc;
use uuid::Uuid;

/// A deterministic mock embedding port (copied in spirit from
/// `rag_integration_tests.rs`'s `qdrant`-gated `MockEmbeddingPort`, but reproducible
/// rather than random): content starting with `A`/`B`/`C` maps to a fixed vector at
/// decreasing cosine similarity to the query vector; every other text (the query
/// itself, or an unmatched fixture) maps to the query vector, giving it the maximum
/// similarity. No randomness, so retrieval order and scores are exact and repeatable.
struct DeterministicEmbeddingPort;

#[async_trait::async_trait]
impl EmbeddingPort for DeterministicEmbeddingPort {
    async fn embed_text(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let vector = if text.starts_with('A') {
            vec![1.0, 0.0, 0.0]
        } else if text.starts_with('B') {
            vec![0.8, 0.6, 0.0]
        } else if text.starts_with('C') {
            vec![0.6, 0.8, 0.0]
        } else {
            // The query text (and any other unmatched fixture) gets the same vector
            // as the highest-scoring memory -- maximum similarity.
            vec![1.0, 0.0, 0.0]
        };
        Ok(Embedding {
            vector,
            model: "deterministic-mock".to_string(),
            dimension: 3,
            token_count: Some(10),
        })
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        let mut out = Vec::with_capacity(texts.len());
        for text in texts {
            out.push(self.embed_text(text).await?);
        }
        Ok(out)
    }

    fn dimension(&self) -> usize {
        3
    }

    fn model_name(&self) -> &str {
        "deterministic-mock"
    }
}

fn build_memory(content: &str) -> Memory {
    MemoryBuilder::new("paladin-1".to_string(), content.to_string())
        .memory_type(MemoryType::Semantic)
        .importance(0.8)
        .build()
        .expect("failed to build test memory")
}

/// Embeds `content` through `embedding` and stores it in `sanctum`.
async fn store_memory(
    sanctum: &Arc<dyn SanctumPort>,
    embedding: &Arc<dyn EmbeddingPort>,
    content: &str,
) {
    let memory = build_memory(content);
    let embedded = embedding
        .embed_text(content)
        .await
        .expect("the deterministic mock embedding never fails");
    let entry = SanctumEntry::new(memory, embedded.vector).expect("failed to build sanctum entry");
    sanctum
        .store(entry)
        .await
        .expect("failed to store test entry");
}

/// Builds the shared three-memory fixture used by cases (a) and (b): distinct,
/// non-round body sizes (987 / 654 / 321 bytes) at distinct, deterministic scores
/// (~1.0 / 0.8 / 0.6 by construction) so a swapped priority could not pass by
/// coincidence (Phase 31 house style).
async fn three_memory_fixture() -> (Arc<dyn SanctumPort>, Arc<dyn EmbeddingPort>) {
    let sanctum: Arc<dyn SanctumPort> = Arc::new(InMemorySanctum::new(100));
    let embedding: Arc<dyn EmbeddingPort> = Arc::new(DeterministicEmbeddingPort);

    store_memory(&sanctum, &embedding, &"A".repeat(987)).await;
    store_memory(&sanctum, &embedding, &"B".repeat(654)).await;
    store_memory(&sanctum, &embedding, &"C".repeat(321)).await;

    (sanctum, embedding)
}

/// (a) A small `rag.max_tokens` budget forces a shed: `result.shed` is non-empty,
/// `was_rationed()` is true, every shed label parses as a UUID, and
/// `format_for_prompt` ends with EXACTLY the shared omission marker for the actual
/// shed count and allotted budget (D-18).
#[tokio::test]
async fn small_budget_sheds_and_marks() {
    let (sanctum, embedding) = three_memory_fixture().await;

    // The same budget wave 1's end-to-end unit test proved forces a shed under these
    // exact body sizes (987/654/321 bytes) at the Commissary's default pessimistic
    // planning ratio -- see
    // `rag_retrieval_service.rs::commissary_rations_rag_retrieval_end_to_end`. A
    // budget of 1,234 tokens (the property-test strategy's sketch figure) would NOT
    // force a shed here -- it yields an allowance comfortably larger than the total
    // 1,962-byte sum of all three bodies.
    let config = RagConfig {
        max_tokens: 233,
        min_similarity: 0.0,
        top_k: 10,
        ..RagConfig::default()
    };
    let service = RagRetrievalService::new(sanctum, embedding, config);

    let result: RagRetrievalResult = service
        .retrieve_context("paladin-1", "find the relevant memories")
        .await
        .expect("retrieval should succeed");

    assert!(
        !result.shed.is_empty(),
        "a tight budget must shed at least the lowest-scoring memory"
    );
    assert!(
        result.was_rationed(),
        "a retrieval that shed a memory must report was_rationed() == true"
    );
    for shed in &result.shed {
        let shed: &ShedItem = shed;
        Uuid::parse_str(&shed.label)
            .unwrap_or_else(|e| panic!("shed label '{}' is not a UUID: {e}", shed.label));
    }

    let formatted = service.format_for_prompt(&result);
    let expected_marker = rag_omission_marker(result.shed.len(), result.allotted_tokens);
    assert!(
        formatted.ends_with(&expected_marker),
        "expected formatted output to end with the shared omission marker {expected_marker:?}, got: {formatted}"
    );
}

/// (b) A budget comfortably above the total sheds nothing: `result.shed` is empty,
/// every retained memory's `truncated` is false, and the rendered prompt contains
/// none of the marker text -- checked by comparing against the shared helper's output
/// for a hypothetical single omission and asserting absence (D-18).
#[tokio::test]
async fn large_budget_sheds_nothing_and_emits_no_marker() {
    let (sanctum, embedding) = three_memory_fixture().await;

    let config = RagConfig {
        max_tokens: 1_000_000,
        min_similarity: 0.0,
        top_k: 10,
        ..RagConfig::default()
    };
    let service = RagRetrievalService::new(sanctum, embedding, config);

    let result = service
        .retrieve_context("paladin-1", "find the relevant memories")
        .await
        .expect("retrieval should succeed");

    assert!(
        result.shed.is_empty(),
        "a large budget must shed nothing, got {} shed",
        result.shed.len()
    );
    assert!(
        result.memories.iter().all(|m| !m.truncated),
        "a large budget must not truncate any retained memory"
    );

    let formatted = service.format_for_prompt(&result);
    let hypothetical_marker = rag_omission_marker(1, result.allotted_tokens);
    assert!(
        !formatted.contains(&hypothetical_marker),
        "no omission marker should be present when nothing was shed, got: {formatted}"
    );
}

/// (c) One memory far larger than the whole budget is RETAINED, TRUNCATED with the
/// Commissary's per-item marker -- never shed (the D-10(a) edge, at integration level
/// this time): `Commissary::dispense` never sheds the sole remaining candidate once
/// only one item is left to consider (`commissary.rs`'s `dispense` step 5: "n == 1:
/// nothing lower-priority remains to shed").
#[tokio::test]
async fn single_oversized_memory_is_retained_truncated_not_shed() {
    let sanctum: Arc<dyn SanctumPort> = Arc::new(InMemorySanctum::new(10));
    let embedding: Arc<dyn EmbeddingPort> = Arc::new(DeterministicEmbeddingPort);

    let oversized_content = "Z".repeat(5_000);
    store_memory(&sanctum, &embedding, &oversized_content).await;

    let config = RagConfig {
        max_tokens: 50,
        min_similarity: 0.0,
        top_k: 10,
        ..RagConfig::default()
    };
    let service = RagRetrievalService::new(sanctum, embedding, config);

    let result = service
        .retrieve_context("paladin-1", "find the relevant memories")
        .await
        .expect("retrieval should succeed");

    assert_eq!(
        result.memories.len(),
        1,
        "the sole oversized memory must be retained, not dropped"
    );
    assert!(
        result.memories[0].truncated,
        "a memory far larger than the budget must be truncated, not silently dropped whole"
    );
    assert!(
        result.memories[0].body.contains("(truncated)"),
        "the retained body must carry the Commissary's per-item truncation marker, got: {}",
        result.memories[0].body
    );
    assert!(
        result.shed.is_empty(),
        "the sole candidate must never be shed -- nothing lower-priority remains"
    );
}
