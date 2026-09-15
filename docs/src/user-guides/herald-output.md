# Herald Output Formatting

The **Herald** system provides pluggable output formatters for Paladin and Battalion execution
results. A Herald transforms a `PaladinResult` or `BattalionResult` into a human-readable or
machine-readable string — JSON, Markdown, or ASCII table.

The `Herald` trait is defined in `crates/paladin-core/src/platform/container/herald.rs`.
Adapters are in `src/infrastructure/adapters/herald/`.

---

## Table of Contents

1. [Overview](#overview)
2. [Available Heralds](#available-heralds)
3. [Herald Trait](#herald-trait)
4. [Attaching to a Paladin](#attaching-to-a-paladin)
5. [Attaching to a Battalion Service](#attaching-to-a-battalion-service)
6. [Custom Herald Implementation](#custom-herald-implementation)
7. [Streaming Output](#streaming-output)
8. [Error Handling](#error-handling)

---

## Overview

| Herald | Import | Best For |
|--------|--------|----------|
| `JsonHerald` | `paladin::infrastructure::adapters::herald::JsonHerald` | APIs, logging, programmatic consumption |
| `MarkdownHerald` | `paladin::infrastructure::adapters::herald::MarkdownHerald` | Terminal display, reports, documentation |
| `TableHerald` | `paladin::infrastructure::adapters::herald::TableHerald` | Tabular terminal output, log files |

---

## Available Heralds

### `JsonHerald`

Serialises `PaladinResult` and `BattalionResult` to JSON.

```rust,ignore
use paladin::infrastructure::adapters::herald::JsonHerald;
use paladin::infrastructure::adapters::herald::json_herald::JsonHeraldConfig;

// Default: pretty = true, include_metadata = true
let herald = JsonHerald::new();

// Compact JSON without metadata
let herald = JsonHerald::with_config(JsonHeraldConfig {
    pretty: false,
    include_metadata: false,
});

let json_str = herald.format_paladin_result(&result)?;
// "usage" is the full TokenUsage split, serialized as a stable six-key object:
// {"output": "...", "usage": {"prompt_tokens": 100, "completion_tokens": 50,
//   "total_tokens": 150, "cache_read_tokens": null, "cache_write_tokens": null,
//   "reasoning_tokens": null}, "execution_time_ms": 1230, ...}
```

### `MarkdownHerald`

Formats results with Markdown headings, status badges, and code blocks.
Supports ANSI colour codes for terminal output.

```rust,ignore
use paladin::infrastructure::adapters::herald::MarkdownHerald;
use paladin::infrastructure::adapters::herald::markdown_herald::MarkdownHeraldConfig;

// Default: auto-detects terminal colour support
let herald = MarkdownHerald::new();

// Custom: force no colours, H1 headings
let herald = MarkdownHerald::with_config(MarkdownHeraldConfig {
    include_colors: false,
    heading_level: 1,
});
```

### `TableHerald`

Renders results as ASCII tables using `comfy-table`.

```rust,ignore
use paladin::infrastructure::adapters::herald::TableHerald;
use paladin::infrastructure::adapters::herald::table_herald::TableHeraldConfig;

// Default configuration
let herald = TableHerald::default();

// Custom: 80-char column width, rounded borders
let herald = TableHerald::new(TableHeraldConfig {
    max_column_width: 80,
    border_style: "rounded".to_string(),
});
```

---

## Herald Trait

```rust,ignore
pub trait Herald: Send + Sync {
    /// Format a completed Paladin result
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError>;

    /// Format a completed Battalion result
    fn format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError>;

    /// Format a streaming chunk — returns Some(String) when output is ready to emit,
    /// or None if the Herald is buffering
    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<Option<String>, HeraldError>;
}
```

---

## Attaching to a Paladin

```rust,ignore
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::infrastructure::adapters::herald::JsonHerald;
use paladin_core::platform::container::herald::Herald;
use std::sync::Arc;

let herald: Arc<dyn Herald> = Arc::new(JsonHerald::new());

let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are an API assistant.")
    .with_herald(herald)
    .build()
    .await?;

let result = paladin.execute("List all Rust 2024 edition features").await?;
// result.output is already formatted as JSON
println!("{}", result.output);
```

---

## Attaching to a Battalion Service

Formation, Phalanx, and other services accept a Herald via `.with_herald()`:

```rust,ignore
use paladin_battalion::phalanx_service::PhalanxExecutionService;
use paladin::infrastructure::adapters::herald::MarkdownHerald;
use std::sync::Arc;

let service = PhalanxExecutionService::new(paladin_port)
    .with_herald(Arc::new(MarkdownHerald::new()));

let result = service.execute(&phalanx, "Analyse this dataset").await?;
println!("{}", result.output);
```

---

## Custom Herald Implementation

Implement the `Herald` trait to create a bespoke formatter:

```rust,ignore
use paladin_core::platform::container::herald::{
    Herald, HeraldError, PaladinResult, BattalionResult, StreamChunk,
};

pub struct CsvHerald;

impl Herald for CsvHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        Ok(format!(
            "{},{},{},{:?}\n",
            result.output.replace(',', ";"),
            result.usage.total_tokens,
            result.execution_time_ms,
            result.stop_reason,
        ))
    }

    fn format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError> {
        Ok(result.output.clone())
    }

    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<Option<String>, HeraldError> {
        Ok(Some(chunk.text.clone()))
    }
}
```

---

## Streaming Output

Use `format_stream_chunk()` during `execute_stream()`:

```rust,ignore
use paladin_ports::output::paladin_port::PaladinStreamChunk;
use paladin_core::platform::container::herald::Herald;

let mut stream = paladin.execute_stream("Generate a long report").await?;
let herald = MarkdownHerald::new();

while let Some(chunk_result) = stream.recv().await {
    match chunk_result {
        Ok(chunk) => {
            // chunk.text is raw text; wrap in a StreamChunk for the Herald
            if let Some(formatted) = herald.format_stream_chunk(&chunk.into())? {
                print!("{}", formatted);
            }
            if chunk.is_final { break; }
        }
        Err(e) => eprintln!("Stream error: {}", e),
    }
}
```

---

## Error Handling

`HeraldError` variants from `paladin_core::platform::container::herald_error`:

| Variant | Cause | Recovery |
|---------|-------|----------|
| `SerializationError(String)` | JSON serialisation failure | Check result data for non-serialisable fields |
| `FormatError(String)` | Internal formatter error | Report as bug; fallback to `to_string()` |
| `InvalidInput(String)` | Unexpected input shape | Validate result before formatting |
