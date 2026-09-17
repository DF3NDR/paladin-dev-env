# Sanctum Vector Memory

**Sanctum** is Paladin AI's long-term semantic memory system. It stores memories as vector
embeddings, enabling similarity-based retrieval across sessions — unlike **Garrison** which
stores sequential conversation history, Sanctum finds *conceptually similar* past experiences.

Sanctum is defined in `crates/paladin-ports/src/output/sanctum_port.rs` (the `SanctumPort`
trait) with adapter implementations in `crates/paladin-memory/src/sanctum/`.

---

## Table of Contents

1. [Sanctum vs. Garrison](#sanctum-vs-garrison)
2. [Quick Start](#quick-start)
3. [Sanctum Adapters](#sanctum-adapters)
4. [SanctumPort Trait](#sanctumport-trait)
5. [SanctumEntry and Memory Types](#sanctumentry-and-memory-types)
6. [Searching with SanctumQuery](#searching-with-sanctumquery)
7. [RAG — Retrieval-Augmented Generation](#rag--retrieval-augmented-generation)
8. [Attaching to a Paladin](#attaching-to-a-paladin)
9. [Docker Setup (Qdrant)](#docker-setup-qdrant)
10. [config.yml Reference](#configyml-reference)
11. [Error Handling](#error-handling)
12. [Best Practices](#best-practices)

---

## Sanctum vs. Garrison

| | Garrison | Sanctum |
|-|----------|---------|
| **Storage** | Sequential entries | Vector embeddings |
| **Retrieval** | Most recent N / keyword | Cosine similarity |
| **Scope** | Single conversation | Across all sessions |
| **Use for** | Conversation context | Knowledge base, RAG |
| **Backend** | In-memory / SQLite | In-memory / Qdrant |
| **Requires embeddings** | No (optional) | Yes |

---

## Quick Start

> **Prerequisite**: A running Qdrant instance. Use `make dev` to start the Docker Compose
> stack, or `docker run -p 6334:6334 qdrant/qdrant`.

```rust,ignore
use paladin_memory::sanctum::QdrantSanctumAdapter;
use paladin_memory::services::rag_retrieval_service::RagRetrievalService;
use paladin_core::platform::container::sanctum::{Memory, MemoryType, SanctumEntry};
use paladin_ports::output::sanctum_port::{SanctumPort, SanctumQuery};
use paladin_ports::output::embedding_port::EmbeddingPort;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sanctum = Arc::new(
        QdrantSanctumAdapter::new("http://localhost:6334", "memories", 1536).await?
    );
    let embedder: Arc<dyn EmbeddingPort> = Arc::new(openai_embedder());

    // Store a memory
    let content = "Rust's borrow checker prevents data races at compile time.";
    let embedding = embedder.embed_text(content).await?;

    let memory = Memory::builder("agent-1".to_string(), content.to_string())
        .memory_type(MemoryType::Semantic)
        .importance(0.9)
        .build()?;

    let entry = SanctumEntry {
        memory,
        embedding: embedding.vector.clone(),
        dimension: embedding.vector.len(),
    };

    sanctum.store(entry).await?;

    // Semantic search
    let query_vec = embedder.embed_text("memory safety in Rust").await?.vector;
    let results = sanctum.search(SanctumQuery {
        embedding: query_vec,
        limit: 5,
        filter: None,
    }).await?;

    for r in results {
        println!("[score: {:.3}] {}", r.score, r.entry.memory.content);
    }
    Ok(())
}
```

---

## Sanctum Adapters

Both in `crates/paladin-memory/src/sanctum/`.

### `QdrantSanctumAdapter`

Production-grade vector store with HNSW indexing.

| Property | Value |
|----------|-------|
| Persistence | Qdrant database |
| Scale | Millions of vectors |
| Search | Cosine similarity, HNSW, <500ms at 100K vectors |
| Use case | Production deployments |

```rust,ignore
use paladin_memory::sanctum::QdrantSanctumAdapter;

let sanctum = QdrantSanctumAdapter::new(
    "http://localhost:6334",  // Qdrant URL
    "paladin_memories",       // Collection name
    1536,                     // Vector dimension (match your embedding model)
).await?;
```

The collection is **auto-created** if it does not exist.

### `InMemorySanctum`

Fast, ephemeral vector store for development and testing.

```rust,ignore
use paladin_memory::sanctum::InMemorySanctumAdapter;

let sanctum = InMemorySanctumAdapter::new(1536);
```

---

## SanctumPort Trait

```rust,ignore
#[async_trait]
pub trait SanctumPort: Send + Sync {
    /// Store a single memory with its embedding
    async fn store(&self, entry: SanctumEntry) -> Result<(), SanctumError>;

    /// Store multiple memories in a single batch operation
    async fn store_batch(&self, entries: Vec<SanctumEntry>) -> Result<(), SanctumError>;

    /// Search for semantically similar memories
    async fn search(&self, query: SanctumQuery) -> Result<Vec<SanctumSearchResult>, SanctumError>;

    /// Delete a memory by its ID
    async fn delete(&self, id: &str) -> Result<bool, SanctumError>;
}
```

---

## SanctumEntry and Memory Types

```rust,ignore
use paladin_core::platform::container::sanctum::{Memory, MemoryType, SanctumEntry};

let memory = Memory::builder("paladin-id".to_string(), "content here".to_string())
    .memory_type(MemoryType::Semantic)   // Semantic | Episodic | Procedural
    .importance(0.8)                      // 0.0–1.0
    .add_metadata("topic".to_string(), serde_json::json!("rust"))
    .build()?;

let entry = SanctumEntry {
    memory,
    embedding: vec![0.1_f32; 1536],  // Your embedding vector
    dimension: 1536,
};
```

**`MemoryType` variants:**

| Variant | Description |
|---------|-------------|
| `Semantic` | Factual knowledge (recommended default) |
| `Episodic` | Specific past events or interactions |
| `Procedural` | How-to instructions and processes |

---

## Searching with SanctumQuery

```rust,ignore
use paladin_ports::output::sanctum_port::{SanctumQuery, SanctumFilter};

// Basic similarity search
let results = sanctum.search(SanctumQuery {
    embedding: query_vec,
    limit: 10,
    filter: None,
}).await?;

// With metadata filter
let results = sanctum.search(SanctumQuery {
    embedding: query_vec,
    limit: 5,
    filter: Some(SanctumFilter {
        paladin_id: Some("agent-1".to_string()),
        memory_type: Some(MemoryType::Semantic),
        min_importance: Some(0.7),
        ..Default::default()
    }),
}).await?;

// Each result contains:
// result.entry   → the SanctumEntry
// result.score   → cosine similarity (0.0–1.0, higher = more similar)
```

---

## RAG — Retrieval-Augmented Generation

The `RagRetrievalService` (camelCase `Rag`, not `RAG`) in
`crates/paladin-memory/src/services/rag_retrieval_service.rs` automates memory retrieval and
injection into the Paladin's prompt context:

```rust,ignore
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_ports::output::sanctum_port::SanctumPort;
use paladin_ports::output::embedding_port::EmbeddingPort;
use std::sync::Arc;

let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are a knowledgeable assistant.")
    .with_sanctum(sanctum_port)       // Vector store
    .with_embedding_port(embedder)    // Embedding provider
    .build()
    .await?;
```

When Sanctum and an embedding port are both attached, the Paladin will automatically:
1. Embed the user's input query.
2. Retrieve the top-K most similar memories from Sanctum.
3. Prepend retrieved context to the prompt before the LLM call.
4. Extract and store important information from the response.

The RAG retrieval config is controlled via `config.yml`:

```yaml
rag:
  enabled: true
  top_k: 5
  min_score: 0.7
  inject_into_prompt: true
```

### Calling `RagRetrievalService` Directly

Construct the service over a `SanctumPort` and `EmbeddingPort`, optionally inject an exact
token counter via `with_token_counter` (the default is a heuristic estimator), and call the
timeout-bounded retrieval entry point, `retrieve_context_with_timeout`:

```rust,ignore
{{#include ../../../crates/doc-examples/src/sanctum_vector_memory.rs:rag_retrieve}}
```

`retrieve_context_with_timeout` wraps `RagRetrievalService::retrieve_context`, which rations
the ranked search results through the Commissary and returns a `RagRetrievalResult`:

```rust,ignore
pub struct RagRetrievalResult {
    pub memories: Vec<RagRetainedMemory>,  // retained, descending relevance order
    pub shed: Vec<ShedItem>,               // memories dropped entirely to fit the budget
    pub prompt_tokens: u32,
    pub allotted_tokens: u32,
    pub exact_tally: bool,
}
```

Every memory that does not survive rationing appears in `shed`, labelled by its memory UUID —
nothing is dropped silently. Retrieval failures surface as a typed `RagRetrievalError`
(`Sanctum`, `Commissary`, `BudgetTooLarge`, `UnmatchedDispensedLabel`, `DuplicateMemoryId`).

### Rendering the Result — the Omission Marker

`RagRetrievalService::format_for_prompt` renders a `RagRetrievalResult` into prompt context —
the same renderer the facade's `PaladinExecutionService::format_retrieved_context` mirrors, so
the two can never drift — and appends a trailing omission-marker line whenever `shed` is
non-empty, naming how many lower-relevance memories were dropped entirely and the token budget
they were rationed against:

```rust,ignore
{{#include ../../../crates/doc-examples/src/sanctum_vector_memory.rs:rag_format}}
```

---

## Attaching to a Paladin

```rust,ignore
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_memory::sanctum::QdrantSanctumAdapter;
use std::sync::Arc;

let sanctum = Arc::new(
    QdrantSanctumAdapter::new("http://localhost:6334", "memories", 1536).await?
);
let embedder = Arc::new(openai_embedder());

let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are a knowledge-augmented assistant.")
    .with_sanctum(sanctum)
    .with_embedding_port(embedder)
    .build()
    .await?;
```

---

## Docker Setup (Qdrant)

The development Docker Compose stack includes Qdrant:

```bash
make dev           # Starts Redis, MinIO, MySQL, and Qdrant
# or individually:
docker run -p 6334:6334 -p 6333:6333 qdrant/qdrant
```

Default connection: `http://localhost:6334` (gRPC) / `http://localhost:6333` (REST dashboard).

---

## config.yml Reference

```yaml
sanctum:
  type: qdrant           # "qdrant" or "in_memory"
  url: "http://localhost:6334"
  collection: paladin_memories
  vector_dimension: 1536 # Must match embedding model output dimension

rag:
  enabled: true
  top_k: 5               # Number of similar memories to retrieve
  min_score: 0.7         # Minimum cosine similarity threshold
  inject_into_prompt: true

memory_extraction:
  enabled: true
  strategy: selective    # "all" or "selective"
```

---

## Error Handling

`SanctumError` variants:

| Variant | Cause | Recovery |
|---------|-------|----------|
| `StorageError(String)` | Qdrant unavailable / capacity | Check Qdrant status |
| `SearchError(String)` | Invalid query / timeout | Reduce `top_k`, check query embedding |
| `DimensionMismatch { expected, got }` | Wrong embedding size | Ensure all vectors match `vector_dimension` |
| `NotFound` | Entry ID does not exist | Expected on first access |
| `ConfigError(String)` | Bad adapter configuration | Check URL and collection name |

---

## Best Practices

- **Match dimensions** — set `vector_dimension` to exactly the output size of your embedding model
  (OpenAI `text-embedding-3-small` = 1536, `text-embedding-3-large` = 3072).
- **Use `store_batch()`** when loading a knowledge base — it is significantly faster than
  individual `store()` calls.
- **Set `min_score`** in `SanctumQuery` to filter out low-quality matches; 0.7 is a good starting point.
- **Separate collections per agent** or per use-case to avoid cross-contamination in multi-agent systems.
- **Use `InMemorySanctum` in tests** to avoid requiring a running Qdrant instance.
