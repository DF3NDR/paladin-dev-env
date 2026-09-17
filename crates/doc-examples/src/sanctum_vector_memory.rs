//! Examples for `docs/src/user-guides/sanctum-vector-memory.md` (Phase 35, MB-28).
//!
//! Every `// ANCHOR:` region below is pulled into the Sanctum Vector Memory user
//! guide via mdBook `{{#include}}`, so a sample in the guide cannot drift from the
//! landed API: `cargo check -p paladin-doc-examples` compiles all of them.
#![allow(unused_variables, unused_imports, dead_code)]

use std::sync::Arc;

use async_trait::async_trait;

// ANCHOR: rag_retrieve
use paladin_memory::sanctum::InMemorySanctum;
use paladin_memory::services::rag_retrieval_service::{
    RagConfig, RagRetrievalService, retrieve_context_with_timeout,
};
use paladin_memory::token_counter::HeuristicTokenCounter;
use paladin_ports::output::embedding_port::{Embedding, EmbeddingError, EmbeddingPort};
use paladin_ports::output::sanctum_port::SanctumPort;

/// A deterministic, no-network embedder — enough to drive the RAG example without a
/// real embedding provider.
struct MockEmbedder;

#[async_trait]
impl EmbeddingPort for MockEmbedder {
    async fn embed_text(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        Ok(Embedding {
            vector: vec![0.0_f32; 8],
            model: "mock-embedder".to_string(),
            dimension: 8,
            token_count: Some(text.split_whitespace().count() as u32),
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
        8
    }

    fn model_name(&self) -> &str {
        "mock-embedder"
    }
}

/// Build a `RagRetrievalService` (camelCase `Rag`, not `RAG`) over an in-memory
/// Sanctum, inject an exact token counter via `with_token_counter`, and call the
/// timeout-bounded retrieval entry point. Returns a `RagRetrievalResult` — the
/// retained memories plus the Commissary's `shed: Vec<ShedItem>` accounting for
/// anything dropped to fit the token budget.
pub async fn retrieve_with_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let sanctum: Arc<dyn SanctumPort> = Arc::new(InMemorySanctum::new(1_000));
    let embedding: Arc<dyn EmbeddingPort> = Arc::new(MockEmbedder);

    let service = RagRetrievalService::new(sanctum, embedding, RagConfig::default())
        .with_token_counter(Arc::new(HeuristicTokenCounter));

    let result =
        retrieve_context_with_timeout(&service, "agent-1", "memory safety in Rust", 5).await?;

    println!(
        "retained={} shed={}",
        result.memories.len(),
        result.shed.len()
    );
    Ok(())
}
// ANCHOR_END: rag_retrieve

// ANCHOR: rag_format
use paladin_memory::services::rag_retrieval_service::{
    RagRetrievalResult, RagRetrievalService as Service,
};

/// Render a `RagRetrievalResult` into prompt context exactly as
/// `RagRetrievalService::format_for_prompt` does — the same renderer the facade's
/// `PaladinExecutionService::format_retrieved_context` mirrors — appending the shared
/// RAG omission marker whenever memories were shed to stay within the token budget.
pub fn format_result(service: &Service, result: &RagRetrievalResult) -> String {
    service.format_for_prompt(result)
}
// ANCHOR_END: rag_format
