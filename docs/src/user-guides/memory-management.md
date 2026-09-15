# Memory Management Guide

This guide covers how to use the Garrison memory system to give your Paladins conversation context, long-term knowledge, and semantic search capabilities.

## Table of Contents

- [Overview](#overview)
- [Garrison Architecture](#garrison-architecture)
- [In-Memory Garrison](#in-memory-garrison)
- [Persistent Garrison](#persistent-garrison)
- [Memory Windowing](#memory-windowing)
- [Semantic Search](#semantic-search)
- [Memory Types](#memory-types)
- [Best Practices](#best-practices)
- [Advanced Patterns](#advanced-patterns)
- [Troubleshooting](#troubleshooting)

## Overview

The Garrison system provides Paladins with:
- **Conversation Context**: Maintain multi-turn dialogue history
- **Memory Windowing**: Manage token limits intelligently
- **Persistence**: Save and restore sessions across restarts
- **Semantic Search**: Retrieve relevant memories by meaning, not just keywords
- **Embeddings**: Vector-based similarity for long-term memory

**Key Concepts:**
- **Garrison**: Memory storage system for a Paladin
- **GarrisonEntry**: Single memory record (message, observation, fact)
- **ConversationHistory**: Ordered sequence of interactions
- **Memory Window**: Limited context size respecting token limits
- **Long-Term Memory**: Persistent storage with semantic retrieval

## Garrison Architecture

### Core Components

```rust,ignore
// Single memory entry
pub struct GarrisonEntry {
    pub id: Uuid,
    pub role: ConversationRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub token_count: Option<u32>,
}

// Conversation roles
pub enum ConversationRole {
    System,    // System prompts
    User,      // User messages
    Assistant, // Paladin responses
    Tool,      // Tool execution results
}

// Memory interface
#[async_trait]
pub trait GarrisonPort: Send + Sync {
    async fn remember(&self, entry: GarrisonEntry) -> Result<(), GarrisonError>;
    async fn recall_recent(&self, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError>;
    async fn forget_all(&self) -> Result<(), GarrisonError>;
    async fn stats(&self) -> Result<GarrisonStats, GarrisonError>;
}

// Extended port for long-term memory
#[async_trait]
pub trait LongTermGarrisonPort: GarrisonPort {
    async fn remember_with_embedding(
        &self,
        entry: GarrisonEntry,
        embedding: Vec<f32>
    ) -> Result<(), GarrisonError>;

    async fn search_similar(
        &self,
        query_embedding: Vec<f32>,
        limit: usize
    ) -> Result<Vec<(GarrisonEntry, f32)>, GarrisonError>;
}
```

### Memory Flow

```
User Input → Garrison adds User entry
    ↓
Paladin retrieves relevant history (window or search)
    ↓
LLM generates response with full context
    ↓
Garrison adds Assistant entry
    ↓
(Optional) Tool calls → Garrison adds Tool entries
    ↓
Repeat for next interaction
```

## In-Memory Garrison

Fastest option for short-lived sessions where persistence isn't needed.

### Basic Usage

```rust,ignore
use paladin_memory::garrison::InMemoryGarrison;
use paladin_core::platform::container::garrison::{GarrisonEntry, ConversationRole, GarrisonConfig};
use paladin::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm_adapter = Arc::new(OpenAiAdapter::new().build()?);

    // Create in-memory garrison — max_entries and max_tokens are GarrisonConfig::new
    // constructor arguments, not with_max_entries()/with_max_tokens() builder calls.
    let garrison = Arc::new(InMemoryGarrison::new(
        GarrisonConfig::new(100, Some(4000))
    ));

    // Build Paladin with memory
    let paladin = PaladinBuilder::new(llm_adapter)
        .name("ChatBot")
        .system_prompt("You are a helpful assistant with memory of our conversation.")
        .with_garrison(garrison.clone())
        .build()?;

    // First interaction
    let response1 = paladin.execute("My name is Alice").await?;
    println!("Bot: {}", response1.content);

    // Second interaction - Paladin remembers
    let response2 = paladin.execute("What's my name?").await?;
    println!("Bot: {}", response2.content);  // Should say "Alice"

    // Check garrison statistics
    let stats = garrison.stats().await?;
    println!("Total memories: {}", stats.entry_count);
    println!("Total tokens: {}", stats.total_tokens);

    Ok(())
}
```

### Configuration Options

```rust,ignore
// GarrisonConfig::new(max_entries, max_tokens) — there is no default-then-with_max_*
// builder chain; entry/token limits are constructor arguments, not builder methods.
let garrison = InMemoryGarrison::new(
    GarrisonConfig::new(100, Some(4000))
        // Eviction strategy when limits reached
        .with_eviction_strategy(EvictionStrategy::FIFO)  // First-in-first-out
        // Entries always kept regardless of eviction strategy
        .with_preserve_recent(10)
);

// Token counting is a separate concern from GarrisonConfig — GarrisonEntry.token_count
// is populated by a `TokenCounterPort` implementation (the exact `TiktokenCounter::new("gpt-4")`,
// or the ungated default `HeuristicTokenCounter`), there is no `GarrisonConfig::with_token_counter`
// method.
```

### Eviction Strategies

The real type is `EvictionStrategy` (`crates/paladin-core/src/platform/container/garrison.rs`),
not `EvictionPolicy`, and it has three variants — there is no `Lru` or `Custom(..)` variant:

```rust,ignore
pub enum EvictionStrategy {
    // Remove oldest entries first
    FIFO,

    // Preserve system prompts and recent messages, evict middle entries (the default)
    ImportanceBased,

    // Always keep only the most recent N entries
    SlidingWindow,
}
```

`ImportanceBased` (the default) already protects system-role entries and the
`preserve_recent_count` most recent entries before evicting anything else — the effect the
former `Custom` closure example was reaching for is the built-in behavior, not something you
need to hand-write:

```rust,ignore
let garrison = InMemoryGarrison::new(
    GarrisonConfig::new(100, Some(4000))
        .with_eviction_strategy(EvictionStrategy::ImportanceBased)
        .with_preserve_recent(10)
);
```

## Persistent Garrison

SQLite-backed storage for sessions that need to survive restarts.

### Setup

```rust,ignore
use paladin_memory::garrison::SqliteGarrison;
use paladin_core::platform::container::garrison::GarrisonConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create persistent garrison — the constructor is `connect`, not `new`, and it
    // takes the config and a paladin_id (the scoping key) directly; there is no
    // separate `.with_config(...)` builder step.
    let garrison = Arc::new(
        SqliteGarrison::connect("garrison.db", GarrisonConfig::default(), "paladin-001")
            .await?
    );

    let paladin = PaladinBuilder::new(llm_adapter)
        .with_garrison(garrison)
        .build()?;

    // All interactions are automatically persisted
    paladin.execute("Remember this important fact!").await?;

    Ok(())
}
```

### Scoping by Paladin

`SqliteGarrison` has no separate "session" concept or `.with_session_id(...)` method — the
scoping key is the `paladin_id` argument passed directly to `connect()`:

```rust,ignore
let paladin_id = "paladin-001";

let garrison = Arc::new(
    SqliteGarrison::connect("garrison.db", GarrisonConfig::default(), paladin_id).await?
);

// Later, reconnect scoped to the same Paladin
let garrison_restored = Arc::new(
    SqliteGarrison::connect("garrison.db", GarrisonConfig::default(), paladin_id).await?
);

// History is preserved
let history = garrison_restored.recall_recent(100).await?;
println!("Restored {} memories", history.len());
```

### Multiple Users

```rust,ignore
pub struct UserGarrison {
    db: SqliteGarrison,
    user_id: String,
}

impl UserGarrison {
    pub async fn new(db_path: &str, user_id: String) -> Result<Self> {
        let db = SqliteGarrison::connect(db_path, GarrisonConfig::default(), &user_id).await?;
        Ok(Self { db, user_id })
    }
}

#[async_trait]
impl GarrisonPort for UserGarrison {
    async fn remember(&self, mut entry: GarrisonEntry) -> Result<()> {
        // Tag entries with user_id
        entry.metadata.insert("user_id".to_string(), self.user_id.clone());
        self.db.remember(entry).await
    }

    async fn recall_recent(&self, limit: usize) -> Result<Vec<GarrisonEntry>> {
        // Filter by user_id
        let all_entries = self.db.recall_recent(limit * 2).await?;
        Ok(all_entries.into_iter()
            .filter(|e| e.metadata.get("user_id") == Some(&self.user_id))
            .take(limit)
            .collect())
    }

    // Implement other methods...
}

// Usage
let alice_garrison = Arc::new(UserGarrison::new("garrison.db", "alice".to_string()).await?);
let bob_garrison = Arc::new(UserGarrison::new("garrison.db", "bob".to_string()).await?);

let alice_paladin = PaladinBuilder::new(llm_adapter.clone())
    .with_garrison(alice_garrison)
    .build()?;

let bob_paladin = PaladinBuilder::new(llm_adapter)
    .with_garrison(bob_garrison)
    .build()?;
```

### Database Schema

The scoping column is `paladin_id`, not `session_id` (there is no session concept — see
[Scoping by Paladin](#scoping-by-paladin) above), embeddings live in a separate table, and
SQLite indexes are declared with standalone `CREATE INDEX` statements, not inline in
`CREATE TABLE` (SQLite does not support the inline `INDEX (...)` syntax at all — this excerpt is
the real `migrations/001_create_garrison_tables.sql`, trimmed to the entries/metadata tables):

```sql
-- migrations/001_create_garrison_tables.sql
CREATE TABLE IF NOT EXISTS garrison_entries (
    id TEXT PRIMARY KEY NOT NULL,
    paladin_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('system', 'user', 'assistant', 'tool')),
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    token_count INTEGER,
    metadata TEXT, -- JSON blob for flexible metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_paladin_timestamp
ON garrison_entries(paladin_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_paladin_role
ON garrison_entries(paladin_id, role);

-- Vector embeddings live in a separate table, not inline in garrison_entries
CREATE TABLE IF NOT EXISTS garrison_embeddings (
    entry_id TEXT PRIMARY KEY NOT NULL,
    embedding BLOB,
    embedding_model TEXT NOT NULL,
    dimension INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (entry_id) REFERENCES garrison_entries(id) ON DELETE CASCADE
);

-- Per-Paladin config and running totals — there is no garrison_sessions table
CREATE TABLE IF NOT EXISTS garrison_metadata (
    paladin_id TEXT PRIMARY KEY NOT NULL,
    max_entries INTEGER NOT NULL DEFAULT 100,
    max_tokens INTEGER,
    eviction_strategy TEXT NOT NULL DEFAULT 'importance_based',
    preserve_recent_count INTEGER NOT NULL DEFAULT 10,
    total_entries INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0
);
```

## Memory Windowing

Intelligently manage context size to respect LLM token limits.

### Token-Based Windowing

```rust,ignore
// Get most recent entries that fit within token limit
let window = garrison.recall_recent(4000).await?;

println!("Window contains {} entries", window.len());
println!("Total tokens: {}",
    window.iter().map(|e| e.token_count.unwrap_or(0)).sum::<u32>());
```

### Sliding Window

```rust,ignore
pub struct SlidingWindowGarrison {
    garrison: Arc<dyn GarrisonPort>,
    window_size: u32,
}

impl SlidingWindowGarrison {
    pub fn new(garrison: Arc<dyn GarrisonPort>, window_size: u32) -> Self {
        Self { garrison, window_size }
    }
}

#[async_trait]
impl GarrisonPort for SlidingWindowGarrison {
    async fn recall_recent(&self, _limit: usize) -> Result<Vec<GarrisonEntry>> {
        // Always return windowed history
        self.garrison.recall_recent(self.window_size).await
    }

    // Forward other methods to inner garrison
    async fn remember(&self, entry: GarrisonEntry) -> Result<()> {
        self.garrison.remember(entry).await
    }

    // ... other methods
}

// Usage - Paladin always sees only recent context
let windowed = Arc::new(SlidingWindowGarrison::new(garrison, 4000));

let paladin = PaladinBuilder::new(llm_adapter)
    .with_garrison(windowed)
    .build()?;
```

### Smart Windowing with Priorities

```rust,ignore
pub struct PriorityWindowGarrison {
    garrison: Arc<dyn GarrisonPort>,
    window_size: u32,
}

impl PriorityWindowGarrison {
    async fn get_prioritized_window(&self) -> Result<Vec<GarrisonEntry>> {
        let all_entries = self.garrison.recall_recent(1000).await?;

        // Always include system prompts
        let system_entries: Vec<_> = all_entries.iter()
            .filter(|e| e.role == ConversationRole::System)
            .cloned()
            .collect();

        // Calculate remaining token budget
        let system_tokens: u32 = system_entries.iter()
            .map(|e| e.token_count.unwrap_or(0))
            .sum();

        let remaining_budget = self.window_size.saturating_sub(system_tokens);

        // Fill with most recent non-system entries
        let mut recent_entries: Vec<_> = all_entries.iter()
            .filter(|e| e.role != ConversationRole::System)
            .rev()
            .cloned()
            .collect();

        let mut token_sum = 0u32;
        let mut windowed_recent = Vec::new();

        for entry in recent_entries {
            let entry_tokens = entry.token_count.unwrap_or(0);
            if token_sum + entry_tokens <= remaining_budget {
                token_sum += entry_tokens;
                windowed_recent.push(entry);
            } else {
                break;
            }
        }

        // Combine: system + recent (chronological order)
        windowed_recent.reverse();
        let mut result = system_entries;
        result.extend(windowed_recent);

        Ok(result)
    }
}
```

### Summarization for Compression

```rust,ignore
pub struct SummarizingGarrison {
    garrison: Arc<dyn GarrisonPort>,
    summarizer: Arc<dyn LlmPort>,
    window_size: u32,
    summary_threshold: usize,
}

impl SummarizingGarrison {
    async fn maybe_summarize(&self) -> Result<()> {
        let entries = self.garrison.recall_recent(self.summary_threshold).await?;

        if entries.len() >= self.summary_threshold {
            // Create summary of old entries
            let old_entries: Vec<_> = entries.iter()
                .take(self.summary_threshold / 2)
                .collect();

            let conversation_text = old_entries.iter()
                .map(|e| format!("{:?}: {}", e.role, e.content))
                .collect::<Vec<_>>()
                .join("\n");

            let prompt = format!(
                "Summarize this conversation in 2-3 paragraphs, preserving key facts:\n\n{}",
                conversation_text
            );

            let summary = self.summarizer.generate(&prompt).await?;

            // GarrisonPort has no remove_entry()/selective-delete method — old entries
            // are not explicitly deleted here; they age out via the configured
            // EvictionStrategy once the summary entry below pushes past the limit.
            self.garrison.remember(GarrisonEntry {
                id: Uuid::new_v4(),
                role: ConversationRole::System,
                content: format!("Previous conversation summary: {}", summary),
                timestamp: Utc::now(),
                metadata: HashMap::from([
                    ("type".to_string(), "summary".to_string()),
                ]),
                token_count: None,
            }).await?;
        }

        Ok(())
    }
}
```

## Semantic Search

Retrieve relevant memories by meaning using embeddings.

`LongTermGarrisonPort::search_similar` (above) is a real trait method, but the tree's own
`examples/garrison_semantic_search.rs` documents it as "planned for future implementation" with
no concrete Garrison-side adapter — the vector-search path that is actually implemented today
is the separate Sanctum subsystem used below, not a Garrison extension.

### Setup with Embeddings

There is no `VectorGarrison` type and no `paladin_memory::embedding` module — semantic search
is a separate subsystem (Sanctum, `crates/paladin-memory/src/sanctum/`) built on the real
`EmbeddingPort` trait and `OpenAIEmbeddingAdapter` (`paladin_llm::openai::embedding`), not a
Garrison variant, and it is not wired into `PaladinBuilder::with_garrison` automatically:

```rust,ignore
use paladin_llm::openai::embedding::{OpenAIEmbeddingAdapter, OpenAIEmbeddingConfig};
use paladin_memory::sanctum::qdrant_adapter::QdrantSanctumAdapter;
use paladin_ports::output::embedding_port::EmbeddingPort;
use paladin_ports::output::sanctum_port::{SanctumPort, SanctumQuery};
use paladin_core::platform::container::sanctum::{MemoryBuilder, SanctumEntry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Embedding generation and vector storage are two separate ports
    let embedder = OpenAIEmbeddingAdapter::new(OpenAIEmbeddingConfig {
        api_key,
        ..Default::default()
    });
    let sanctum = QdrantSanctumAdapter::new("http://localhost:6334", "paladin_memories", 1536).await?;

    // Store entries with embeddings generated explicitly (not automatic on paladin.execute())
    for text in [
        "I love hiking in the mountains",
        "My favorite color is blue",
        "I work as a software engineer",
    ] {
        let embedding = embedder.embed_text(text).await?;
        let memory = MemoryBuilder::new("paladin-001".to_string(), text.to_string()).build()?;
        sanctum.store(SanctumEntry::new(memory, embedding.vector)?).await?;
    }

    // Semantic search — SanctumSearchResult carries the similarity score, unlike
    // LongTermGarrisonPort::search_similar, which returns plain Vec<GarrisonEntry>
    let query_embedding = embedder.embed_text("outdoor activities").await?;
    let query = SanctumQuery::new(query_embedding.vector, 5);
    let results = sanctum.search(query).await?;

    for result in results {
        println!("Similarity: {:.2} - {}", result.score, result.entry.memory.content);
    }
    // Output: High similarity for "hiking in the mountains"

    Ok(())
}
```

### Hybrid Search (Keyword + Semantic)

```rust,ignore
pub struct HybridGarrison {
    garrison: Arc<dyn LongTermGarrisonPort>,
}

impl HybridGarrison {
    pub async fn hybrid_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<GarrisonEntry>> {
        // Get keyword matches
        let keyword_results = self.garrison.search(query, limit * 2).await?;

        // Get semantic matches
        let embedding = self.embedder.embed_text(query).await?;
        let semantic_results = self.garrison
            .semantic_search(embedding, limit * 2)
            .await?;

        // Merge and deduplicate
        let mut combined: HashMap<Uuid, (GarrisonEntry, f32)> = HashMap::new();

        // Add keyword results with base score
        for entry in keyword_results {
            combined.insert(entry.id, (entry, 0.5));
        }

        // Add semantic results, boosting score if already present
        for (entry, similarity) in semantic_results {
            combined.entry(entry.id)
                .and_modify(|(_, score)| *score += similarity * 0.5)
                .or_insert((entry, similarity * 0.5));
        }

        // Sort by combined score
        let mut sorted: Vec<_> = combined.into_values().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        Ok(sorted.into_iter()
            .take(limit)
            .map(|(entry, _)| entry)
            .collect())
    }
}
```

### RAG (Retrieval-Augmented Generation)

```rust,ignore
pub struct RAGPaladin {
    paladin: Paladin,
    garrison: Arc<dyn LongTermGarrisonPort>,
}

impl RAGPaladin {
    pub async fn execute_with_rag(&self, query: &str) -> Result<PaladinResult> {
        // Retrieve relevant context from long-term memory
        let embedding = self.embedder.embed_text(query).await?;
        let relevant_memories = self.garrison
            .semantic_search(embedding, 5)
            .await?;

        // Build augmented prompt
        let context = relevant_memories.iter()
            .map(|(entry, _)| entry.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        let augmented_query = format!(
            "Context from previous conversations:\n{}\n\n\
             Current question: {}",
            context, query
        );

        // Execute with retrieved context
        self.paladin.execute(&augmented_query).await
    }
}

// Usage
let rag_paladin = RAGPaladin {
    paladin,
    garrison: vector_garrison,
};

let response = rag_paladin.execute_with_rag(
    "What programming languages do I know?"
).await?;
```

## Memory Types

### Episodic Memory

Memory of specific events and experiences.

```rust,ignore
// Add episodic memory
garrison.remember(GarrisonEntry {
    id: Uuid::new_v4(),
    role: ConversationRole::User,
    content: "I visited Paris last summer".to_string(),
    timestamp: Utc::now(),
    metadata: HashMap::from([
        ("memory_type".to_string(), "episodic".to_string()),
        ("event_type".to_string(), "travel".to_string()),
        ("location".to_string(), "Paris, France".to_string()),
        ("timeframe".to_string(), "summer 2023".to_string()),
    ]),
    token_count: Some(10),
}).await?;
```

### Semantic Memory

General knowledge and facts.

```rust,ignore
// Add semantic memory (facts)
garrison.remember(GarrisonEntry {
    id: Uuid::new_v4(),
    role: ConversationRole::System,
    content: "User prefers Python over JavaScript for backend development".to_string(),
    timestamp: Utc::now(),
    metadata: HashMap::from([
        ("memory_type".to_string(), "semantic".to_string()),
        ("category".to_string(), "preferences".to_string()),
        ("topic".to_string(), "programming".to_string()),
    ]),
    token_count: Some(15),
}).await?;
```

### Procedural Memory

Knowledge about how to do things.

```rust,ignore
// Add procedural memory
garrison.remember(GarrisonEntry {
    id: Uuid::new_v4(),
    role: ConversationRole::System,
    content: "To deploy this project: cargo build --release && docker build -t app .".to_string(),
    timestamp: Utc::now(),
    metadata: HashMap::from([
        ("memory_type".to_string(), "procedural".to_string()),
        ("task".to_string(), "deployment".to_string()),
    ]),
    token_count: Some(20),
}).await?;
```

## Best Practices

### 1. Choose the Right Garrison Type

```rust,ignore
// ✅ Use InMemoryGarrison for:
// - Temporary chatbots
// - Stateless services
// - Testing and development

let garrison = Arc::new(InMemoryGarrison::new(
    GarrisonConfig::new(100, Some(4000))
));

// ✅ Use SqliteGarrison for:
// - Multi-Paladin applications
// - Per-Paladin contexts
// - Production services needing persistence

let garrison = Arc::new(
    SqliteGarrison::connect("garrison.db", GarrisonConfig::default(), "paladin-001").await?
);

// ✅ Use Sanctum for:
// - Long-term knowledge bases
// - RAG applications
// - Semantic retrieval needs
//
// Sanctum (crates/paladin-memory/src/sanctum/) is a separate long-term-memory subsystem
// from Garrison, not a Garrison variant — there is no "VectorGarrison" type.

let sanctum = Arc::new(
    QdrantSanctumAdapter::new("http://localhost:6334", "paladin_memories", 1536).await?
);
```

### 2. Set Appropriate Token Limits

```rust,ignore
// Model context windows
const GPT_4_TURBO: u32 = 128_000;
const GPT_4: u32 = 8_192;
const GPT_3_5: u32 = 16_385;
const CLAUDE_3: u32 = 200_000;

// Reserve tokens for: system prompt + response + buffer
let response_tokens = 1000;
let system_prompt_tokens = 500;
let buffer = 500;

let available_for_history = GPT_4 - response_tokens - system_prompt_tokens - buffer;

let garrison = InMemoryGarrison::new(
    GarrisonConfig::new(100, Some(available_for_history))  // ~6000 tokens
);
```

### 3. Add Metadata for Better Organization

```rust,ignore
garrison.remember(GarrisonEntry {
    id: Uuid::new_v4(),
    role: ConversationRole::User,
    content: message.clone(),
    timestamp: Utc::now(),
    metadata: HashMap::from([
        ("user_id".to_string(), user_id.clone()),
        ("session_id".to_string(), session_id.to_string()),
        ("channel".to_string(), "web".to_string()),
        ("language".to_string(), "en".to_string()),
        ("importance".to_string(), "high".to_string()),
    ]),
    token_count: Some(estimate_tokens(&message)),
}).await?;
```

### 4. Clean Up Old Memories

`GarrisonPort` has no age-based or per-entry removal method — `forget_all()` is the only
removal operation the trait exposes, and it clears everything. Age-based cleanup is handled
automatically by `GarrisonConfig`'s eviction strategy (`with_eviction_strategy`,
`with_preserve_recent`) rather than an application-level `remove_before(cutoff)` call:

```rust,ignore
// Automatic: entries beyond max_entries/max_tokens are evicted per the configured
// EvictionStrategy on every `remember()` call — no scheduled task is required.
let garrison = InMemoryGarrison::new(
    GarrisonConfig::new(1000, Some(50_000))
        .with_eviction_strategy(EvictionStrategy::ImportanceBased)
);

// Manual, full reset (the only removal GarrisonPort exposes):
garrison.forget_all().await?;
```

### 5. Implement Conversation Branching

```rust,ignore
pub struct BranchingGarrison {
    garrison: Arc<dyn GarrisonPort>,
    current_branch: RwLock<Uuid>,
}

impl BranchingGarrison {
    pub async fn create_branch(&self, from_entry: Uuid) -> Result<Uuid> {
        let branch_id = Uuid::new_v4();

        // Copy history up to branch point
        let history = self.garrison.recall_recent(1000).await?;
        let branch_history: Vec<_> = history.into_iter()
            .take_while(|e| e.id != from_entry)
            .collect();

        // Store branch metadata
        self.garrison.remember(GarrisonEntry {
            id: Uuid::new_v4(),
            role: ConversationRole::System,
            content: format!("Branch created from entry {}", from_entry),
            timestamp: Utc::now(),
            metadata: HashMap::from([
                ("type".to_string(), "branch".to_string()),
                ("branch_id".to_string(), branch_id.to_string()),
                ("parent_entry".to_string(), from_entry.to_string()),
            ]),
            token_count: None,
        }).await?;

        *self.current_branch.write().await = branch_id;
        Ok(branch_id)
    }
}
```

## Advanced Patterns

### Memory Consolidation

```rust,ignore
pub struct ConsolidatingGarrison {
    garrison: Arc<dyn GarrisonPort>,
    llm: Arc<dyn LlmPort>,
}

impl ConsolidatingGarrison {
    pub async fn consolidate_memories(&self) -> Result<()> {
        let entries = self.garrison.recall_recent(100).await?;

        // Group by topic using LLM
        let topics = self.extract_topics(&entries).await?;

        // Create consolidated memory for each topic
        for (topic, topic_entries) in topics {
            let facts = self.extract_facts(&topic_entries).await?;

            self.garrison.remember(GarrisonEntry {
                id: Uuid::new_v4(),
                role: ConversationRole::System,
                content: format!("Consolidated facts about {}: {}", topic, facts),
                timestamp: Utc::now(),
                metadata: HashMap::from([
                    ("type".to_string(), "consolidated".to_string()),
                    ("topic".to_string(), topic),
                    ("source_count".to_string(), topic_entries.len().to_string()),
                ]),
                token_count: None,
            }).await?;
        }

        Ok(())
    }

    async fn extract_topics(&self, entries: &[GarrisonEntry]) -> Result<HashMap<String, Vec<GarrisonEntry>>> {
        // Use LLM to categorize entries by topic
        // Implementation details...
        Ok(HashMap::new())
    }

    async fn extract_facts(&self, entries: &[GarrisonEntry]) -> Result<String> {
        let conversation = entries.iter()
            .map(|e| &e.content)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Extract key facts from this conversation:\n\n{}",
            conversation
        );

        self.llm.generate(&prompt).await
    }
}
```

### Attention Mechanism

```rust,ignore
pub struct AttentionGarrison {
    garrison: Arc<dyn LongTermGarrisonPort>,
}

impl AttentionGarrison {
    pub async fn get_attended_context(
        &self,
        query: &str,
        context_size: u32,
    ) -> Result<Vec<GarrisonEntry>> {
        // Get semantic matches
        let query_embedding = self.embedder.embed_text(query).await?.vector;
        let candidates = self.garrison
            .semantic_search(query_embedding, 50)
            .await?;

        // Score each candidate using attention mechanism
        let mut scored: Vec<_> = candidates.into_iter()
            .map(|(entry, similarity)| {
                let recency_score = self.recency_score(&entry);
                let importance_score = self.importance_score(&entry);

                // Weighted combination
                let attention = similarity * 0.5 + recency_score * 0.3 + importance_score * 0.2;

                (entry, attention)
            })
            .collect();

        // Sort by attention score
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Select top entries within token budget
        let mut selected = Vec::new();
        let mut token_sum = 0u32;

        for (entry, _) in scored {
            let entry_tokens = entry.token_count.unwrap_or(0);
            if token_sum + entry_tokens <= context_size {
                token_sum += entry_tokens;
                selected.push(entry);
            }
        }

        Ok(selected)
    }

    fn recency_score(&self, entry: &GarrisonEntry) -> f32 {
        let age = (Utc::now() - entry.timestamp).num_seconds() as f32;
        let decay_rate = 0.0001;  // Adjust for desired decay speed
        (-decay_rate * age).exp()
    }

    fn importance_score(&self, entry: &GarrisonEntry) -> f32 {
        // Extract importance from metadata or content
        entry.metadata.get("importance")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.5)
    }
}
```

### Memory Reflection

```rust,ignore
pub struct ReflectiveGarrison {
    garrison: Arc<dyn GarrisonPort>,
    llm: Arc<dyn LlmPort>,
}

impl ReflectiveGarrison {
    pub async fn generate_reflections(&self) -> Result<()> {
        let recent_entries = self.garrison.recall_recent(50).await?;

        // Prompt LLM to reflect on conversation
        let conversation = recent_entries.iter()
            .map(|e| format!("{:?}: {}", e.role, e.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Reflect on this conversation and extract:\n\
             1. Key insights about the user\n\
             2. Patterns in the discussion\n\
             3. Important facts to remember\n\n\
             Conversation:\n{}",
            conversation
        );

        let reflection = self.llm.generate(&prompt).await?;

        // Store reflection as high-importance memory
        self.garrison.remember(GarrisonEntry {
            id: Uuid::new_v4(),
            role: ConversationRole::System,
            content: format!("Reflection: {}", reflection),
            timestamp: Utc::now(),
            metadata: HashMap::from([
                ("type".to_string(), "reflection".to_string()),
                ("importance".to_string(), "high".to_string()),
            ]),
            token_count: None,
        }).await?;

        Ok(())
    }
}
```

## Troubleshooting

### Memory Not Persisting

**Problem**: Garrison entries disappear after restart.

**Solutions**:
1. Verify using `SqliteGarrison`, not `InMemoryGarrison`
2. Check database file path is correct and writable
3. Ensure proper async handling (`.await` on all operations)

```rust,ignore
// ❌ Won't persist
let garrison = Arc::new(InMemoryGarrison::new(config));

// ✅ Will persist
let garrison = Arc::new(SqliteGarrison::new("garrison.db").await?);
```

### Context Window Overflow

**Problem**: Errors about exceeding maximum context length.

**Solutions**:
1. Reduce `max_tokens` in `GarrisonConfig`
2. Use `get_window()` instead of `get_history()`
3. Implement summarization for old memories

```rust,ignore
// Calculate safe token limit
let model_limit = 8192;  // GPT-4
let response_budget = 1000;
let system_prompt_tokens = 500;
let safety_buffer = 500;

let garrison_limit = model_limit - response_budget - system_prompt_tokens - safety_buffer;

let garrison = InMemoryGarrison::new(
    GarrisonConfig::new(100, Some(garrison_limit))
);
```

### Slow Semantic Search

**Problem**: Embedding-based search is taking too long.

**Solutions**:
1. Add database indexes on embedding columns
2. Use approximate nearest neighbor (ANN) algorithms
3. Cache embeddings for frequent queries
4. Limit search scope with filters

```sql
-- Embeddings live in garrison_embeddings, not inline on garrison_entries
-- (see Database Schema above); index by model for faster lookups:
CREATE INDEX IF NOT EXISTS idx_embedding_model ON garrison_embeddings(embedding_model);

-- The workspace already ships a Qdrant adapter for vector search at scale
-- (crates/paladin-memory/src/sanctum/qdrant_adapter.rs, behind the `qdrant` feature)
```

### Memory Leaks in Long Sessions

**Problem**: Memory usage grows unbounded.

**Solutions**:
1. Set `max_entries` in config
2. Implement periodic cleanup
3. Use eviction policies
4. Monitor with `garrison.stats()`

```rust,ignore
// Periodic monitoring — GarrisonPort has no compact()/partial-cleanup method;
// eviction already runs automatically per the configured EvictionStrategy on
// every remember() call, so this loop only needs to watch for problems.
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;

        let stats = garrison.stats().await.unwrap();

        if stats.entry_count > 1000 {
            log::warn!("Garrison entry_count exceeds expected bound: {}", stats.entry_count);
        }
    }
});
```

## Testing

### Unit Testing

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_garrison_add_and_retrieve() {
        let garrison = InMemoryGarrison::new(GarrisonConfig::default());

        let entry = GarrisonEntry {
            id: Uuid::new_v4(),
            role: ConversationRole::User,
            content: "Test message".to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            token_count: Some(2),
        };

        garrison.remember(entry.clone()).await.unwrap();

        let history = garrison.recall_recent(10).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "Test message");
    }

    #[tokio::test]
    async fn test_token_window() {
        let garrison = InMemoryGarrison::new(
            GarrisonConfig::new(100, Some(100))
        );

        // Add entries totaling 150 tokens
        for i in 0..15 {
            garrison.remember(GarrisonEntry {
                id: Uuid::new_v4(),
                role: ConversationRole::User,
                content: format!("Message {}", i),
                timestamp: Utc::now(),
                metadata: HashMap::new(),
                token_count: Some(10),
            }).await.unwrap();
        }

        // Window should respect token limit
        let window = garrison.recall_recent(100).await.unwrap();
        let total_tokens: u32 = window.iter()
            .map(|e| e.token_count.unwrap_or(0))
            .sum();

        assert!(total_tokens <= 100);
    }
}
```

## Examples

See working examples:
- `examples/garrison_in_memory.rs` - Basic in-memory usage
- `examples/garrison_persistent.rs` - SQLite persistence
- `examples/garrison_semantic_search.rs` - Placeholder text search; semantic search is not yet
  implemented for Garrison (see [Semantic Search](#semantic-search) above)

## Next Steps

- **[Tool Integration](tool-integration.md)** - Combine memory with tools
- **[Battalion Patterns](battalion-patterns.md)** - Shared memory in multi-agent systems
- **[API Reference](https://docs.rs/paladin)** - Garrison API documentation

## Related Resources

- [Token Counting Strategies](../architecture/overview.md)
- [Vector Database Integration](../user-guides/sanctum-vector-memory.md)
- [Production Deployment](../deployment/production.md)
