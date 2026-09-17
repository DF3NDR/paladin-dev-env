# Arsenal Tools

The **Arsenal** system (`crates/paladin-ports/src/output/arsenal_port.rs`) gives Paladins access
to external tools and services through the **Model Context Protocol** (MCP). Tools are called
*Armaments*; the registry that holds them is the *Arsenal*.

---

## Table of Contents

1. [Concepts](#concepts)
2. [Quick Start — STDIO Server](#quick-start--stdio-server)
3. [Streamable-HTTP Server Configuration](#streamable-http-server-configuration)
4. [config.yml Reference](#configyml-reference)
5. [ArsenalPort Trait](#arsenalport-trait)
6. [ArsenalRegistry Trait](#arsenalregistry-trait)
7. [Attaching Arsenal to a Paladin](#attaching-arsenal-to-a-paladin)
8. [Custom Armaments (Direct Rust Tools)](#custom-armaments-direct-rust-tools)
9. [Handoff Tool](#handoff-tool)
10. [Error Handling](#error-handling)
11. [Best Practices](#best-practices)

---

## Concepts

| Term | Definition |
|------|------------|
| **Armament** | A single callable tool (name, description, JSON schema) |
| **ArmamentCall** | A runtime invocation (tool name + argument map) |
| **ArmamentResult** | Return value (`success: bool`, `output: Option<Value>`, `error: Option<String>`) |
| **ArsenalPort** | Trait for discovering and invoking armaments |
| **ArsenalRegistry** | Trait for managing the registry lifecycle (register, remove) |
| **MCPStdioAdapter** | Communicates with command-line MCP servers via stdin/stdout |
| **MCPStreamableHttpAdapter** | Communicates with remote, optionally authenticated MCP servers over Streamable-HTTP (replaces the retired, never-actually-SSE `MCPSseAdapter`) |

---

## Quick Start — STDIO Server

STDIO servers are the most common MCP transport. The process is spawned and communicated with
via newline-delimited JSON on stdin/stdout.

### 1. Configure in `config.yml`

```yaml
arsenal:
  mcp_servers:
    - name: web_search
      type: stdio
      command: uvx
      args: ["mcp-server-brave-search"]
      env:
        BRAVE_API_KEY: "${BRAVE_API_KEY}"
    - name: filesystem
      type: stdio
      command: npx
      args: ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"]
```

### 2. Build a Paladin with the Arsenal

```rust,ignore
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::arsenal_port::ArsenalRegistry;
use std::sync::Arc;

// Arsenal registry is built from config.yml automatically when using
// PaladinBuilder::from_config() or can be constructed manually.
let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are a research assistant with web search access.")
    .with_arsenal_registry(arsenal_registry)
    .build()
    .await?;

let result = paladin.execute("Find the latest Rust release notes").await?;
println!("{}", result.output);
```

The Paladin will automatically detect tool-call JSON in LLM responses, invoke the tool via
the Arsenal, and feed results back into the reasoning loop.

---

## Streamable-HTTP Server Configuration

Remote MCP servers are reached over HTTP(S) using the Streamable-HTTP transport (D-02/D-03).
This is the real, currently-implemented remote transport, replacing the retired
`MCPSseAdapter` (which was never actually SSE — just a mislabeled, unauthenticated
plain-HTTP-POST adapter):

```yaml
arsenal:
  mcp_servers:
    - name: my_api_server
      type: streamable_http
      endpoint: "http://localhost:8080/mcp"
      # NAMES the env var holding the bearer token -- never a literal secret
      # in this file. Omit entirely for an unauthenticated server.
      auth_token_env: "MY_API_SERVER_TOKEN"
```

`MCPStreamableHttpAdapter` builds the connection and delegates to
`MCPClient::connect_streamable_http`, which performs the full
`initialize -> notifications/initialized` handshake:

```rust,ignore
use paladin::infrastructure::adapters::arsenal::mcp_streamable_http_adapter::MCPStreamableHttpAdapter;

let adapter = MCPStreamableHttpAdapter::new("http://localhost:8080/mcp")
    .with_bearer_token(std::env::var("MY_API_SERVER_TOKEN")?); // never hardcode the token
let client = adapter.connect().await?;
let tools = client.discover_tools().await?;
```

---

## config.yml Reference

```yaml
arsenal:
  mcp_servers:
    - name: <identifier>              # Unique name used in logs and errors
      type: stdio | streamable_http   # Transport type ("sse" is retired --
                                       # fails loud with a migration message)
      # STDIO fields:
      command: <executable>           # e.g. python3, npx, uvx
      args: [<arg>, ...]              # Command-line arguments
      # Streamable-HTTP fields:
      endpoint: <url>                 # Full URL of the remote MCP endpoint
      auth_token_env: <ENV_VAR_NAME>  # NAMES the env var holding the bearer
                                       # token -- never a literal secret here
```

---

## ArsenalPort Trait

Defined in `crates/paladin-ports/src/output/arsenal_port.rs`:

```rust,ignore
#[async_trait]
pub trait ArsenalPort: Send + Sync {
    /// List all available armaments from this MCP server
    async fn list_armaments(&self) -> Vec<Armament>;

    /// Invoke an armament with the given arguments
    async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError>;

    /// Validate call arguments against the armament's JSON schema
    fn validate_call(&self, call: &ArmamentCall) -> Result<(), ArsenalError>;
}
```

Direct usage:

```rust,ignore
use paladin_core::platform::container::arsenal::ArmamentCall;
use serde_json::json;
use std::collections::HashMap;

let mut args = HashMap::new();
args.insert("query".to_string(), json!("Rust 2024 edition features"));

let call = ArmamentCall::new("web_search", args);
arsenal_port.validate_call(&call)?;

let result = arsenal_port.invoke(call).await?;
if result.success {
    println!("{}", result.output.unwrap());
}
```

---

## ArsenalRegistry Trait

Defined alongside `ArsenalPort`:

```rust,ignore
#[async_trait]
pub trait ArsenalRegistry: Send + Sync {
    /// Register a new armament in the registry
    async fn register(&self, armament: Armament);

    /// Remove an armament by name
    async fn remove(&self, name: &str);

    /// Get all registered armament descriptors
    async fn list(&self) -> Vec<Armament>;

    /// Look up a specific armament by name
    async fn get(&self, name: &str) -> Option<Armament>;
}
```

---

## Attaching Arsenal to a Paladin

```rust,ignore
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_ports::output::arsenal_port::ArsenalRegistry;
use std::sync::Arc;

let paladin = PaladinBuilder::new(llm_port)
    .system_prompt(
        "You are a coding assistant. Use the filesystem tool to read files when needed."
    )
    .with_arsenal_registry(Arc::new(my_registry))
    .build()
    .await?;
```

---

## Custom Armaments (Direct Rust Tools)

Implement `ArsenalPort` to expose any Rust function as a tool. `Armament` carries a
`parameters` JSON Schema (not `input_schema`) plus a `required_params` list; the live
`ArmamentResult` has five fields — `call_id`, `success`, `output`, `error` and
`execution_time_ms` — and a call's arguments are read from the `arguments` map on
`ArmamentCall`, not an `args` field:

```rust,ignore
{{#include ../../../crates/doc-examples/src/arsenal_tools.rs:custom_armament}}
```

---

## Handoff Tool

The `handoff_tool` in `crates/paladin-core/src/platform/container/arsenal/handoff_tool.rs`
is a built-in Armament that allows a Paladin to delegate sub-tasks to specialist agents
at runtime. Register specialist agents on the builder via `with_handoffs`, which takes
the whole specialist list at once — there is no per-call chainable registration method:

```rust,ignore
{{#include ../../../crates/doc-examples/src/arsenal_tools.rs:handoffs}}
```

The LLM will emit a tool-call for `handoff` when it determines a specialist is more
appropriate. Delegation records appear in `PaladinResult.handoff_history`.

---

## Error Handling

`ArsenalError` variants (from `paladin_core::platform::container::arsenal`):

| Variant | Cause | Recovery |
|---------|-------|----------|
| `ToolNotFound(String)` | Armament name not in registry | Check `list_armaments()` |
| `InvalidArguments(String)` | Schema validation failed | Fix argument map |
| `Timeout` | Tool took too long | Increase `timeout_seconds` in config |
| `ProtocolError(String)` | Malformed MCP message | Check MCP server logs |
| `TransportError(String)` | Process/network failure | Verify server is running |

---

## Best Practices

- **Validate before invoking** — call `validate_call()` to catch argument errors early.
- **Set timeouts** — all MCP servers should have `timeout_seconds` to avoid blocking the
  reasoning loop indefinitely.
- **Describe tools well** — the Armament `description` is what the LLM reads to decide
  whether to call the tool; make it precise.
- **Namespace tool names** — use `server_name.tool_name` convention to avoid collisions
  when registering multiple servers.
- **Test with mock** — implement a `MockArsenalPort` in tests to avoid spawning real subprocesses.
