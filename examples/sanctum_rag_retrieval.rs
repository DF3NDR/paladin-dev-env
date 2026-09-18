// examples/sanctum_rag_retrieval.rs
//
// Sanctum RAG Retrieval: the Phase 33 Rationed Retrieval Surface
//
// `examples/paladin_with_rag.rs` is a printed conceptual walkthrough of RAG in
// Paladin -- it calls none of the Phase 33 API. This program is its runnable
// sibling: it drives `RagRetrievalService` for real, over an in-memory Sanctum,
// and shows:
//
// 1. Retrieving context and reading the typed `RagRetrievalResult` back (EX-116).
// 2. Reading the `ShedItem` records a rationed retrieval carries when the token
//    budget forces some memories out (EX-117).
// 3. A retrieval that fails, matched against the typed `RagRetrievalError` enum
//    (EX-118).
// 4. The timeout-bounded free function `retrieve_context_with_timeout` (EX-119).
// 5. Injecting an exact token counter via `with_token_counter`, contrasted against
//    the heuristic default's `exact_tally` answer (EX-120).
//
// This example is fully offline: it uses an in-memory Sanctum and a deterministic,
// no-network embedding stand-in, and reads no LLM provider API key from the
// environment -- no external service is needed. See `paladin_with_rag.rs` for the
// conceptual overview this program complements.
//
// To run this example:
// ```bash
// cargo run --example sanctum_rag_retrieval
// ```

use std::sync::Arc;

use async_trait::async_trait;

use paladin::core::platform::container::sanctum::{MemoryBuilder, MemoryType, SanctumEntry};
use paladin_memory::sanctum::InMemorySanctum;
use paladin_memory::services::rag_retrieval_service::{
    RagConfig, RagRetrievalError, RagRetrievalResult, RagRetrievalService, RetrievalTrigger,
    retrieve_context_with_timeout,
};
use paladin_ports::output::embedding_port::{Embedding, EmbeddingError, EmbeddingPort};
use paladin_ports::output::sanctum_port::SanctumPort;
use paladin_ports::output::token_counter_port::TokenCounterPort;

/// A deterministic, no-network embedder: derives a fixed-dimension, always
/// non-zero vector from `text`'s own content hash (`DefaultHasher::new()`
/// uses a fixed key, so this is stable across process runs -- unlike
/// `InMemorySanctum`'s own `HashMap`-backed storage, whose iteration order
/// is randomized per process). Distinct texts get distinct vectors, so
/// similarity scores across the seeded memories are deterministic and never
/// tie, regardless of storage iteration order. (A zero vector would score
/// 0.0 by `InMemorySanctum`'s own "treat zero vectors as orthogonal" rule --
/// this embedder's `1.0 + ...` construction deliberately avoids that.)
struct FixedEmbedder;

impl FixedEmbedder {
    fn vector_for(text: &str) -> Vec<f32> {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        let bytes = hasher.finish().to_le_bytes();
        bytes[..4]
            .iter()
            .map(|b| 1.0 + (f32::from(*b) / 255.0))
            .collect()
    }
}

#[async_trait]
impl EmbeddingPort for FixedEmbedder {
    async fn embed_text(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        Ok(Embedding {
            vector: Self::vector_for(text),
            model: "fixed-demo-embedder".to_string(),
            dimension: 4,
            token_count: Some(1),
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
        4
    }

    fn model_name(&self) -> &str {
        "fixed-demo-embedder"
    }
}

/// A stand-in for an exact, BPE-based counter: overrides `is_exact` to `true`
/// so its shed decision can be contrasted against the heuristic default
/// (EX-120).
#[derive(Debug, Default, Clone, Copy)]
struct ExactWordCounter;

impl TokenCounterPort for ExactWordCounter {
    fn count(&self, text: &str, _model: &str) -> u32 {
        u32::try_from(text.split_whitespace().count()).unwrap_or(u32::MAX)
    }

    fn name(&self) -> &str {
        "exact-word-rag-demo"
    }

    fn is_exact(&self) -> bool {
        true
    }
}

const MEMORIES: [&str; 5] = [
    "Rust's ownership system ensures memory safety without a garbage collector by enforcing \
     a single owner for each value at compile time.",
    "Borrowing lets code temporarily reference a value without taking ownership, subject to \
     the compiler's aliasing rules: many shared references, or exactly one exclusive one.",
    "Lifetimes are the compiler's way of tracking how long a reference remains valid relative \
     to the data it points to, catching dangling references before the program ever runs.",
    "The Send and Sync auto traits let the compiler reason about which types are safe to move \
     or share across threads, ruling out data races at compile time rather than at runtime.",
    "Pattern matching over enums such as Option and Result is exhaustive by default, catching \
     every unhandled case at compile time instead of leaving a gap for a runtime panic.",
];

async fn seed(
    sanctum: &dyn SanctumPort,
    paladin_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for content in MEMORIES {
        let memory = MemoryBuilder::new(paladin_id.to_string(), content.to_string())
            .memory_type(MemoryType::Semantic)
            .importance(0.8)
            .build()?;
        let entry = SanctumEntry::new(memory, FixedEmbedder::vector_for(content))?;
        sanctum.store(entry).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Sanctum RAG Retrieval: the Phase 33 Rationed Retrieval Surface\n");

    // ------------------------------------------------------------------------------
    // Part 0 -- re-check the D-02 drift before doing anything else. Phase 35 added
    // crates/doc-examples/src/sanctum_vector_memory.rs, which already calls
    // retrieve_context_with_timeout -- the audit's own zero-hit grep for that token
    // is now stale. Every hit count below is recorded verbatim in this plan's
    // evidence file, not silently skipped.
    // ------------------------------------------------------------------------------
    println!(
        "0. Post-audit drift re-check (D-02) -- see 36-evidence/36-08-examples.txt for the\n   \
         verbatim grep output this program's own capability tokens were re-checked against.\n"
    );

    let paladin_id = "rag-demo-agent";
    let query = "How does Rust manage memory safety and prevent data races?";

    // ------------------------------------------------------------------------------
    // Part 1 -- retrieve context and read the typed result back (EX-116), forcing
    // truncation with a small token budget so shed records exist too (EX-117).
    // ------------------------------------------------------------------------------
    println!("1. RagRetrievalResult -- retrieve context and read the typed result (EX-116)\n");
    println!("2. ShedItem -- a rationed retrieval's shed record (EX-117)\n");

    let sanctum: Arc<dyn SanctumPort> = Arc::new(InMemorySanctum::new(1_000));
    seed(sanctum.as_ref(), paladin_id).await?;
    let embedding: Arc<dyn paladin_ports::output::embedding_port::EmbeddingPort> =
        Arc::new(FixedEmbedder);

    let small_budget_config = RagConfig {
        top_k: 5,
        min_similarity: 0.5,
        max_tokens: 40,
        timeout_seconds: 5,
        retrieval_trigger: RetrievalTrigger::Always,
    };
    let service = RagRetrievalService::new(sanctum.clone(), embedding.clone(), small_budget_config);

    let result: RagRetrievalResult = service.retrieve_context(paladin_id, query).await?;
    println!(
        "   retained={} shed={} prompt_tokens={} allotted_tokens={} exact_tally={}",
        result.memories.len(),
        result.shed.len(),
        result.prompt_tokens,
        result.allotted_tokens,
        result.exact_tally
    );
    for (idx, retained) in result.memories.iter().enumerate() {
        println!(
            "   retained[{idx}] score={:.2} truncated={} body={:?}",
            retained.result.score, retained.truncated, retained.body
        );
    }
    println!("\n   rendered context (RagRetrievalService::format_for_prompt):");
    println!("{}", service.format_for_prompt(&result));

    for (idx, shed) in result.shed.iter().enumerate() {
        println!(
            "   shed[{idx}] label={} priority={} original_bytes={} (did not fit the {}-token budget)",
            shed.label, shed.priority, shed.original_bytes, result.allotted_tokens
        );
    }

    // ------------------------------------------------------------------------------
    // Part 3 -- a retrieval that fails, matched on the typed error enum (EX-118).
    // ------------------------------------------------------------------------------
    println!("\n3. RagRetrievalError -- a typed, matched retrieval failure (EX-118)\n");

    // `rag.max_tokens` beyond u32::MAX cannot be rationed -- a typed error, never a
    // silent clamp (Phase 33 D-10(a)/D-05).
    let impossible_budget_config = RagConfig {
        max_tokens: usize::MAX,
        ..RagConfig::default()
    };
    let impossible_service =
        RagRetrievalService::new(sanctum.clone(), embedding.clone(), impossible_budget_config);

    match impossible_service.retrieve_context(paladin_id, query).await {
        Ok(_) => println!("   UNEXPECTED: retrieval succeeded against an impossible budget"),
        Err(RagRetrievalError::BudgetTooLarge { max_tokens }) => {
            println!("   RagRetrievalError::BudgetTooLarge {{ max_tokens: {max_tokens} }}");
        }
        Err(other) => println!("   UNEXPECTED error variant: {other}"),
    }

    // ------------------------------------------------------------------------------
    // Part 4 -- the timeout-bounded free function (EX-119).
    // ------------------------------------------------------------------------------
    println!(
        "\n4. retrieve_context_with_timeout -- the free-function form with a deadline (EX-119)\n"
    );

    let timeout_result = retrieve_context_with_timeout(&service, paladin_id, query, 5).await?;
    println!(
        "   retrieve_context_with_timeout(..., timeout_secs=5) -- retained={} shed={}",
        timeout_result.memories.len(),
        timeout_result.shed.len()
    );

    // ------------------------------------------------------------------------------
    // Part 5 -- an exact token counter, injected via with_token_counter (EX-120).
    // ------------------------------------------------------------------------------
    println!(
        "\n5. with_token_counter -- an exact counter injected into the retrieval service (EX-120)\n"
    );

    // The heuristic counter (chars/4, rounded up) counts noticeably more tokens per
    // memory than the exact word-count counter does for this prose. Rather than a
    // hand-picked "magic" budget, compute both counters' total over every seeded
    // memory and use the midpoint -- a principled budget rather than an arbitrary
    // one. Whether the retained/shed *counts* end up differing depends on exactly
    // where the ration boundary falls for this budget; what always differs, and is
    // what `with_token_counter` actually injects, is `exact_tally` itself.
    let heuristic_counter: Arc<dyn TokenCounterPort> =
        Arc::new(paladin_memory::token_counter::HeuristicTokenCounter);
    let exact_counter: Arc<dyn TokenCounterPort> = Arc::new(ExactWordCounter);
    let heuristic_total: u32 = MEMORIES
        .iter()
        .map(|m| heuristic_counter.count(m, "demo-model"))
        .sum();
    let exact_total: u32 = MEMORIES
        .iter()
        .map(|m| exact_counter.count(m, "demo-model"))
        .sum();
    let contrast_budget = usize::try_from((heuristic_total + exact_total) / 2).unwrap_or(150);
    println!(
        "   all 5 memories: heuristic total={heuristic_total} tokens, exact total={exact_total} \
         words -- contrast budget = {contrast_budget}"
    );

    let contrast_config = RagConfig {
        top_k: 5,
        min_similarity: 0.5,
        max_tokens: contrast_budget,
        timeout_seconds: 5,
        retrieval_trigger: RetrievalTrigger::Always,
    };
    let heuristic_contrast_service =
        RagRetrievalService::new(sanctum.clone(), embedding.clone(), contrast_config.clone());
    let heuristic_contrast_result = heuristic_contrast_service
        .retrieve_context(paladin_id, query)
        .await?;

    let exact_service =
        RagRetrievalService::new(sanctum.clone(), embedding.clone(), contrast_config)
            .with_token_counter(Arc::new(ExactWordCounter));
    let exact_result = exact_service.retrieve_context(paladin_id, query).await?;

    println!(
        "   heuristic run (chars/4 counter) -- retained={} shed={} exact_tally={}",
        heuristic_contrast_result.memories.len(),
        heuristic_contrast_result.shed.len(),
        heuristic_contrast_result.exact_tally
    );
    println!(
        "   exact run (word-count counter)  -- retained={} shed={} exact_tally={}",
        exact_result.memories.len(),
        exact_result.shed.len(),
        exact_result.exact_tally
    );

    println!("\nDone -- fully offline, no provider API key was read.");
    Ok(())
}
