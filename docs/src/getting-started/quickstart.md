# Quickstart

Get a Paladin agent running in under 15 minutes.

## Prerequisites

Complete [Installation](installation.md) first and set your LLM API key:

```bash
export OPENAI_API_KEY="sk-..."
```

## Create a New Project

```bash
cargo new my-paladin-agent
cd my-paladin-agent
```

Add Paladin to `Cargo.toml`:

```toml
[dependencies]
paladin-ai    = "0.7.0"
paladin-ports = "0.7.0"
paladin-llm   = { version = "0.7.0", features = ["openai"] }
tokio         = { version = "1", features = ["full"] }
```

## Your First Paladin Agent

Replace `src/main.rs` with the following:

```rust,ignore
// src/main.rs -- Hello, Paladin!
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::LlmPort;
use paladin_llm::openai::OpenAIAdapter;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create an LLM adapter (reads OPENAI_API_KEY from env)
    let llm_port: Arc<dyn LlmPort> = Arc::new(OpenAIAdapter::from_env()?);

    // 2. Build the Paladin using the fluent builder
    let paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("You are a concise and helpful assistant.")
        .name("HelloPaladin")
        .model("gpt-4")
        .temperature(0.7)
        .max_loops(1)
        .build()
        .await?;

    // 3. Create an execution service (the circuit breaker guards against cascading failures)
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

    // 4. Execute with a prompt
    let result = service.execute(&paladin, "Say hello in one sentence.").await?;

    println!("Output : {}", result.output);
    println!("Tokens : {}", result.usage.total_tokens);
    println!("Time   : {}ms", result.execution_time_ms);

    Ok(())
}
```

Run it:

```bash
cargo run
```

Expected output (exact wording varies):

```
Output : Hello! I am your AI assistant, ready to help.
Tokens : 18
Time   : 342ms
```

## Running the Built-in Examples

The Paladin workspace ships with ready-to-run examples:

```bash
# Clone the workspace if you haven't already
git clone https://github.com/DF3NDR/paladin-dev-env.git
cd paladin-dev-env

# Start backing services (Redis, MinIO) -- optional for basic examples
make services-up

# Run the basic Paladin example
cargo run --example basic_paladin

# Sequential multi-agent pipeline
cargo run --example formation_sequential

# Concurrent multi-agent execution
cargo run --example phalanx_parallel
```

## Understanding the Output

`PaladinExecutionService::execute` returns a `PaladinResult` with these fields:

| Field | Type | Description |
|-------|------|-------------|
| `output` | `String` | Final LLM response text |
| `loop_count` | `u32` | Number of reasoning loops performed |
| `usage` | `TokenUsage` | Prompt/completion split, plus cache/reasoning sub-counts when the provider reports them |
| `execution_time_ms` | `u64` | Wall-clock time in milliseconds |
| `stop_reason` | `StopReason` | Why execution stopped (`MaxLoops`, `StopWord`, `Done`) |

## What's Next?

| Topic | Guide |
|-------|-------|
| Detailed configuration | [Configuration](configuration.md) |
| Memory between turns | [Garrison Memory](../user-guides/garrison-memory.md) |
| Tool use / MCP | [Arsenal & Tools](../user-guides/arsenal-tools.md) |
| Multi-agent patterns | [Battalion Patterns](../user-guides/battalion-patterns.md) |
| Output formatting | [Herald Output](../user-guides/herald-output.md) |
