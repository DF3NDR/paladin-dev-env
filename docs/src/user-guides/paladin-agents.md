# Paladin Agents

A **Paladin** is Paladin AI's core autonomous agent entity — an LLM-powered reasoner that
operates a configurable reasoning loop, maintains conversation memory via Garrison, executes
external tools via Arsenal, and optionally leverages autonomous features like task planning,
auto-generated prompts, and dynamic temperature.

> Ready to *run* a number of agents? See [Deployment Topologies](../deployment-topologies/overview.md)
> for how to choose between embedding, hosting, queue/worker, and sidecar models.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [PaladinBuilder API](#paladinbuilder-api)
3. [Execution Model](#execution-model)
4. [PaladinResult Fields](#paladinresult-fields)
5. [StopReason Variants](#stopreason-variants)
6. [Autonomous Features](#autonomous-features)
7. [Memory — Garrison](#memory--garrison)
8. [Tools — Arsenal](#tools--arsenal)
9. [Output Formatting — Herald](#output-formatting--herald)
10. [Configuration Reference](#configuration-reference)
11. [Error Handling](#error-handling)
12. [Best Practices](#best-practices)

---

## Quick Start

Add the `paladin-ai` crate and enable any desired feature flags:

```toml
[dependencies]
paladin-ai = { version = "0.5.0", features = ["llm-openai"] }
tokio = { version = "1", features = ["full"] }
```

Build and execute a Paladin:

```rust,ignore
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Construct an LLM adapter (e.g., OpenAI)
    let llm_port: Arc<dyn LlmPort> = Arc::new(openai_adapter());

    // Build the Paladin
    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a helpful assistant.")
        .name("Assistant")
        .model("gpt-4o")
        .temperature(0.7)
        .max_loops(3)
        .timeout_seconds(120)
        .build()
        .await?;

    // Execute
    let result = paladin
        .execute("Explain the Rust ownership model in one paragraph.")
        .await?;

    println!("{}", result.output);
    println!("Tokens used: {}", result.usage.total_tokens);
    println!("Stop reason: {:?}", result.stop_reason);
    Ok(())
}
```

---

## PaladinBuilder API

`PaladinBuilder` is located at `src/application/services/paladin/paladin_builder.rs`.
All methods are fluent (return `Self`). Call `.build().await?` at the end.

### Core Configuration

| Method | Type | Default | Description |
|--------|------|---------|-------------|
| `system_prompt(prompt)` | `impl Into<String>` | `""` | Defines agent personality and instructions |
| `name(name)` | `impl Into<String>` | `""` | Display name for the agent |
| `user_name(name)` | `impl Into<String>` | `""` | Name used for the human turn in prompts |
| `model(model)` | `impl Into<String>` | `""` | LLM model identifier (e.g. `"gpt-4o"`) |
| `temperature(t)` | `f32` | `0.7` | Randomness 0.0–1.0; 0.0 = deterministic |
| `max_loops(n)` | `u32` | `3` | Fixed reasoning iterations (1–100) |
| `add_stop_word(word)` | `impl Into<String>` | — | Halt execution when word appears in output |
| `retry_attempts(n)` | `u32` | `3` | Transient-failure retries |
| `timeout_seconds(s)` | `u64` | `300` | Execution wall-clock timeout |
| `enable_planning(b)` | `bool` | `false` | Activate planning phase before execution |
| `enable_vision(b)` | `bool` | `false` | Enable multimodal image input |
| `output_format(f)` | `OutputFormat` | `Text` | `Text` / `Json` / `Structured` |

### Integrations

| Method | Argument | Description |
|--------|----------|-------------|
| `with_garrison(g)` | `Arc<dyn GarrisonPort>` | Attach conversation memory |
| `with_arsenal_registry(r)` | `Arc<dyn ArsenalRegistry>` | Attach tool registry |
| `with_herald(h)` | `Arc<dyn Herald>` | Set output formatter |
| `with_sanctum(s)` | `Arc<dyn SanctumPort>` | Attach vector memory (requires embedding port) |
| `with_embedding_port(e)` | `Arc<dyn EmbeddingPort>` | Embedding provider for RAG |

### Autonomous Features

| Method | Type | Description |
|--------|------|-------------|
| `enable_autonomous_planning(b)` | `bool` | Decompose tasks into subtasks via LLM planning |
| `enable_autonomous_prompts(b)` | `bool` | Auto-generate system prompt from agent description |
| `enable_dynamic_temperature(b)` | `bool` | Increase temperature linearly over reasoning loops |
| `auto_generate_prompt(b)` | `bool` | Alias for `enable_autonomous_prompts` |
| `auto_temperature(b)` | `bool` | Select optimal temperature from agent description |
| `agent_description(d)` | `impl Into<String>` | Role description for auto-prompt and auto-temperature |

---

## Execution Model

A Paladin's inner reasoning loop:

```text
┌─────────────────────────────────────────────────────────────────┐
│  1. Build Prompt                                                 │
│     System prompt + Garrison history + User input               │
├─────────────────────────────────────────────────────────────────┤
│  2. LLM Call (via LlmPort)                                       │
│     Generate response from the configured model                  │
├─────────────────────────────────────────────────────────────────┤
│  3. Check Stop Conditions                                        │
│     • Stop word detected in output?  → StopWord(word)           │
│     • loop_count ≥ max_loops?        → MaxLoops                 │
│     • Elapsed > timeout?             → Timeout                  │
├─────────────────────────────────────────────────────────────────┤
│  4. Tool Execution (if Arsenal attached)                        │
│     Parse tool-call JSON in response → invoke via ArsenalPort   │
│     Append tool result to context                                │
├─────────────────────────────────────────────────────────────────┤
│  5. Update Garrison                                              │
│     Store assistant turn and any tool results                    │
├─────────────────────────────────────────────────────────────────┤
│  6. Loop or Complete                                             │
│     If no stop condition: loop_count++ → back to step 1         │
│     Otherwise: build PaladinResult and return                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## PaladinResult Fields

Returned by `execute()` and streamed by `execute_stream()`.

| Field | Type | Description |
|-------|------|-------------|
| `output` | `String` | Final generated text |
| `usage` | `TokenUsage` | Prompt/completion split, plus cache/reasoning sub-counts when the provider reports them |
| `execution_time_ms` | `u64` | Wall-clock execution time in milliseconds |
| `loop_count` | `u32` | Number of reasoning iterations performed |
| `stop_reason` | `StopReason` | Why execution terminated |
| `plan` | `Option<TaskPlan>` | Subtask plan (only in autonomous planning mode) |
| `handoff_history` | `Vec<HandoffRecord>` | Agent delegation records |

Check completeness:

```rust,ignore
if result.stop_reason.is_successful() {
    println!("Complete output: {}", result.output);
} else {
    println!("Partial output ({}): {}", result.stop_reason, result.output);
}
```

---

## StopReason Variants

| Variant | `is_successful()` | Meaning |
|---------|-------------------|---------|
| `Completed` | `true` | Natural end of generation |
| `StopWord(String)` | `true` | Configured stop word detected |
| `MaxLoops` | `false` | Loop limit reached (output may be partial) |
| `Timeout` | `false` | Wall-clock timeout exceeded |

---

## Autonomous Features

All autonomous features are **opt-in** (disabled by default) to maintain backward compatibility.

### Autonomous Planning (`MaxLoops::Auto`)

When enabled, the Paladin uses an LLM call to decompose the user's task into subtasks before
executing them sequentially.

```rust,ignore
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::core::platform::container::paladin::MaxLoops;

let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are a research assistant.")
    .enable_autonomous_planning(true)
    // max_loops controls the subtask cap when using auto planning:
    .max_loops(10)
    .build()
    .await?;
```

The `PaladinResult.plan` field contains the `TaskPlan` with each subtask's description and result.

### Auto-Generated System Prompts

Instead of writing a system prompt manually, provide an agent description and let the LLM
generate an optimized prompt:

```rust,ignore
let paladin = PaladinBuilder::new(llm_port)
    .agent_description("Expert in Rust async programming and tokio runtime")
    .enable_autonomous_prompts(true)
    .build()
    .await?;
```

> **Tip**: Calling `.system_prompt(...)` on the same builder disables auto-generation for that
> instance — the manual prompt always takes precedence.

### Dynamic Temperature

Temperature increases linearly from the configured base value toward `1.0` over the reasoning
loops. This encourages broader exploration in later iterations when the agent may be stuck:

```rust,ignore
let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are a problem solver.")
    .temperature(0.3)          // Start temperature
    .max_loops(5)
    .enable_dynamic_temperature(true)  // Reaches ~1.0 by loop 5
    .build()
    .await?;
```

### Agent Handoffs

A Paladin can delegate sub-tasks to specialist agents at runtime using the Arsenal handoff tool.
Register specialist agents on the builder:

```rust,ignore
let paladin = PaladinBuilder::new(llm_port.clone())
    .system_prompt("Routing coordinator. Delegate to specialists.")
    .with_specialist(Arc::new(code_reviewer_paladin))
    .with_specialist(Arc::new(security_auditor_paladin))
    .build()
    .await?;
```

Delegation records appear in `PaladinResult.handoff_history`.

---

## Memory — Garrison

Attach a Garrison adapter to give the Paladin persistent conversation memory.

```rust,ignore
use paladin_memory::garrison::in_memory_garrison::InMemoryGarrison;
use paladin_ports::output::garrison_port::GarrisonPort;
use std::sync::Arc;

let garrison: Arc<dyn GarrisonPort> = Arc::new(InMemoryGarrison::new());

let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are a memory-enabled assistant.")
    .with_garrison(garrison)
    .build()
    .await?;
```

Available Garrison adapters (in `crates/paladin-memory/`):

| Adapter | Persistence | Use Case |
|---------|-------------|----------|
| `InMemoryGarrison` | None (process-scoped) | Development, testing |
| `SqliteGarrison` | SQLite file | Single-agent production |

See [Garrison Memory](garrison-memory.md) for full documentation.

---

## Tools — Arsenal

Attach an Arsenal registry backed by MCP (Model Context Protocol) servers:

```rust,ignore
use paladin_ports::output::arsenal_port::ArsenalRegistry;

// Registry pre-loaded from config.yml arsenal.mcp_servers section
let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are a web researcher with tool access.")
    .with_arsenal_registry(arsenal_registry)
    .build()
    .await?;
```

See [Arsenal Tools](arsenal-tools.md) for MCP server configuration and custom tool implementation.

---

## Output Formatting — Herald

Format execution results using a Herald adapter:

```rust,ignore
use paladin::infrastructure::adapters::herald::JsonHerald;
use paladin_core::platform::container::herald::Herald;

let herald: Arc<dyn Herald> = Arc::new(JsonHerald::default());

let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are an API assistant.")
    .with_herald(herald)
    .build()
    .await?;
```

See [Herald Output](herald-output.md) for available formatters.

---

## Configuration Reference

All builder values can also be set through `config.yml`:

```yaml
paladin:
  default_model: "gpt-4o"
  default_temperature: 0.7
  default_max_loops: 3
  timeout_seconds: 300
  retry_attempts: 3

autonomous:
  planning:
    enabled: false
    max_subtasks: 10
  prompt_generation:
    enabled: false
  dynamic_temperature:
    enabled: false
  handoffs:
    enabled: false
    max_depth: 3
```

See [Configuration](../getting-started/configuration.md) for the full schema.

---

## Error Handling

`PaladinError` variants from `paladin_core::platform::container::paladin_error`:

| Variant | Retryable | Recovery |
|---------|-----------|----------|
| `ConfigurationError(String)` | No | Fix builder parameters |
| `ExecutionError(String)` | Maybe | Check message, retry if transient |
| `LlmError(String)` | Yes | Retry with exponential back-off |
| `Timeout(u64)` | Yes | Increase `timeout_seconds` or reduce `max_loops` |
| `StopWordDetected(String)` | N/A | Success — check result output |

---

## Best Practices

- **Always set a system prompt** that clearly defines the agent's role and constraints.
- **Set `timeout_seconds`** appropriate for your task; defaults to 300s.
- **Use `add_stop_word`** for structured output tasks so the agent knows when it is done.
- **Enable Garrison** for any multi-turn conversation to maintain context.
- **Check `stop_reason.is_successful()`** before consuming `result.output` in production.
- **Prefer `execute_stream()`** for tasks > 30s so the caller can render output incrementally.
- **Use autonomous features sparingly** — they add LLM overhead; profile before enabling in loops.
