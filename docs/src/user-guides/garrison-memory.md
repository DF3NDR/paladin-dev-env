# Garrison Memory

The **Garrison** is Paladin AI's conversation memory system. When attached to a Paladin it
stores and retrieves conversation history, giving the agent context across multiple reasoning
loops and between invocations.

Garrison is defined in `crates/paladin-ports/src/output/garrison_port.rs` (the `GarrisonPort`
trait) with adapter implementations in `crates/paladin-memory/src/garrison/`.

---

## Table of Contents

1. [Concepts](#concepts)
2. [Quick Start](#quick-start)
3. [Garrison Adapters](#garrison-adapters)
4. [GarrisonPort Trait](#garrisonport-trait)
5. [GarrisonConfig](#garrisonconfig)
6. [Conversation Roles](#conversation-roles)
7. [Attaching to a Paladin](#attaching-to-a-paladin)
8. [Long-Term Memory with Embeddings](#long-term-memory-with-embeddings)
9. [config.yml Reference](#configyml-reference)
10. [Error Handling](#error-handling)
11. [Best Practices](#best-practices)

---

## Concepts

| Term | Definition |
|------|------------|
| **Garrison** | The memory subsystem; stores conversation entries |
| **GarrisonEntry** | A single message with role, content, timestamp, and optional token count |
| **ConversationRole** | `User`, `Assistant`, `System`, or `Tool` |
| **GarrisonConfig** | Window size, token budget, and eviction strategy |
| **GarrisonStats** | Entry count, total token count, optional storage size |

---

## Quick Start

```rust,ignore
use paladin_memory::garrison::InMemoryGarrison;
use paladin_core::platform::container::garrison::{GarrisonConfig, GarrisonEntry, ConversationRole};
use paladin_ports::output::garrison_port::GarrisonPort;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let garrison = Arc::new(InMemoryGarrison::new(GarrisonConfig::default()));

    // Store a user message
    garrison.remember(GarrisonEntry::new(
        ConversationRole::User,
        "What is Rust's ownership model?".to_string(),
    )).await?;

    // Store the assistant reply
    garrison.remember(GarrisonEntry::new(
        ConversationRole::Assistant,
        "Rust's ownership model ensures memory safety without a GC...".to_string(),
    )).await?;

    // Recall last 10 entries
    let history = garrison.recall_recent(10).await?;
    for entry in &history {
        println!("{:?}: {}", entry.role, entry.content);
    }

    // Search by keyword
    let results = garrison.search("ownership", 5).await?;
    println!("Found {} messages about ownership", results.len());

    Ok(())
}
```

---

## Garrison Adapters

Both adapters are in `crates/paladin-memory/src/garrison/`.

### `InMemoryGarrison`

| Property | Value |
|----------|-------|
| Persistence | None (process-scoped) |
| Performance | O(1) write, O(N) search |
| Use case | Development, testing, short-lived sessions |

```rust,ignore
use paladin_memory::garrison::InMemoryGarrison;
use paladin_core::platform::container::garrison::GarrisonConfig;

let garrison = InMemoryGarrison::new(GarrisonConfig::default());
```

### `SqliteGarrison`

| Property | Value |
|----------|-------|
| Persistence | SQLite file (survives restarts) |
| Performance | O(log N) indexed read, FTS5 full-text search |
| Use case | Single-agent production deployments |

```rust,ignore
use paladin_memory::garrison::SqliteGarrison;
use paladin_core::platform::container::garrison::GarrisonConfig;

let garrison = SqliteGarrison::connect(
    "./garrison.db",
    GarrisonConfig::default(),
    "my-paladin-id",
).await?;
```

`SqliteGarrison::connect()` creates the file and runs migrations automatically.

---

## GarrisonPort Trait

Full interface defined in `crates/paladin-ports/src/output/garrison_port.rs`:

```rust,ignore
#[async_trait]
pub trait GarrisonPort: Send + Sync {
    /// Store a new conversation entry
    async fn remember(&self, entry: GarrisonEntry) -> Result<(), GarrisonError>;

    /// Retrieve the N most recent entries (newest last)
    async fn recall_recent(&self, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError>;

    /// Full-text search across stored entries
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError>;

    /// Clear all entries from this garrison
    async fn forget_all(&self) -> Result<(), GarrisonError>;

    /// Get storage statistics
    async fn stats(&self) -> Result<GarrisonStats, GarrisonError>;
}
```

### `GarrisonStats`

```rust,ignore
pub struct GarrisonStats {
    pub entry_count: usize,   // Total stored entries
    pub total_tokens: u32,    // Cumulative token count
    pub size_bytes: Option<u64>, // Storage size (adapters may not support this)
}
```

---

## GarrisonConfig

```rust,ignore
use paladin_core::platform::container::garrison::GarrisonConfig;

// max_entries: window size; max_tokens: token budget per context window
let config = GarrisonConfig::new(200, Some(8000));
```

| Field | Default | Description |
|-------|---------|-------------|
| `max_entries` | 100 | Maximum entries to retain in the window |
| `max_tokens` | None | Optional token budget; triggers eviction when exceeded |
| `eviction_strategy` | `Oldest` | `Oldest` removes oldest entries when window is full |

---

## Conversation Roles

```rust,ignore
use paladin_core::platform::container::garrison::ConversationRole;

ConversationRole::User       // Human turn
ConversationRole::Assistant  // Paladin / LLM turn
ConversationRole::System     // System instruction
ConversationRole::Tool       // Tool call result
```

---

## Attaching to a Paladin

```rust,ignore
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_memory::garrison::SqliteGarrison;
use paladin_core::platform::container::garrison::GarrisonConfig;
use paladin_ports::output::garrison_port::GarrisonPort;
use std::sync::Arc;

let garrison: Arc<dyn GarrisonPort> = Arc::new(
    SqliteGarrison::connect("./memory.db", GarrisonConfig::default(), "agent-1").await?
);

let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are a persistent memory assistant.")
    .with_garrison(garrison)
    .build()
    .await?;
```

Once attached, the Paladin automatically:
1. Retrieves recent history before each LLM call.
2. Appends the user turn and assistant response after each loop.

---

## Long-Term Memory with Embeddings

The `LongTermGarrisonPort` trait (also in `garrison_port.rs`) extends `GarrisonPort` with
semantic similarity search using vector embeddings:

```rust,ignore
pub trait LongTermGarrisonPort: GarrisonPort {
    async fn remember_with_embedding(
        &self,
        entry: GarrisonEntry,
        embedding: Vec<f32>,
    ) -> Result<(), GarrisonError>;

    async fn search_similar(
        &self,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<GarrisonEntry>, GarrisonError>;
}
```

For full vector-based semantic memory, consider **Sanctum** — see
[Sanctum Vector Memory](sanctum-vector-memory.md).

---

## config.yml Reference

```yaml
garrison:
  type: sqlite        # "in_memory" or "sqlite"
  path: ./garrison.db # SQLite only
  max_entries: 100
  max_tokens: 8000    # Optional token budget
  eviction_strategy: oldest  # "oldest" (default)
```

---

## Error Handling

`GarrisonError` variants:

| Variant | Cause | Recovery |
|---------|-------|----------|
| `StorageError(String)` | Database / IO failure | Check path, permissions, disk space |
| `SerializationError(String)` | Corrupt entry data | Clear and rebuild garrison |
| `TokenizationError(String)` | Token counting failure | Check tokenizer config |
| `NotFound` | Entry missing | Expected after `forget_all()` |

---

## Summaries (`is_summary`)

A `GarrisonEntry` can carry `is_summary: true`, marking it as a compressed stand-in for older
history rather than a raw conversation turn — written by the `SummarizationMiddleware` described
in the [Agent Runtime guide](agent-runtime.md) once a conversation's history exceeds a configured
token or message threshold. Summaries **compound**: the middleware builds each new summary from the
newest existing summary plus the raw entries newer than it, so an unbounded conversation converges
on one summary plus a bounded raw tail rather than growing forever.

`GarrisonPort` has **no delete-by-id method** (`remember`, `recall_recent`, `search`, `forget_all`
and `stats` are the whole trait), so old summary entries are never removed from the store — this is
by design, not an oversight. The effective history a consumer should use is computed by finding the
*newest* entry with `is_summary == true` in a recalled window and taking that entry plus every raw
entry newer than it; older summaries remain in the store as historical artifacts, latest-wins.

## Best Practices

- **Always use `SqliteGarrison` in production** — `InMemoryGarrison` loses all history when
  the process restarts.
- **Set `max_tokens`** to stay within the LLM's context window; large histories degrade performance.
- **Use one Garrison per Paladin** — shared garrisons across multiple agents mix conversation
  contexts and confuse the LLM.
- **Call `forget_all()` between sessions** if context carry-over is undesirable (e.g., fresh
  chat sessions).
- **Use `search()`** to retrieve relevant past entries rather than dumping the full history into
  the prompt.
