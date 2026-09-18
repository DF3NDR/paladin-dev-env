# Paladin Examples Gallery

This directory contains comprehensive examples demonstrating Paladin's capabilities. Each example is a fully-functional Rust program that you can run and modify.

## Table of Contents

- [Getting Started](#getting-started)
- [Basic Paladin Examples](#basic-paladin-examples)
- [Autonomous Agent Examples](#autonomous-agent-examples) 🆕
- [Memory & Garrison Examples](#memory--garrison-examples)
- [Token Economy Examples](#token-economy-examples)
- [Sanctum Long-term Memory Examples](#sanctum-long-term-memory-examples)
- [Tool Integration Examples](#tool-integration-examples)
- [Battalion Orchestration Examples](#battalion-orchestration-examples)
- [Output Formatting Examples](#output-formatting-examples)
- [State Management Examples](#state-management-examples)
- [Vision](#vision)
- [Document Processing](#document-processing)
- [HTTP Service Host](#http-service-host)
- [RAG & Retrieval](#rag--retrieval)
- [Commander Strategies (Council / Grove / Conclave)](#commander-strategies-council--grove--conclave)
- [Performance Benchmarking Examples](#performance-benchmarking-examples)
- [WarEngine Configuration & Checkpoints](#warengine-configuration--checkpoints)
- [Control Flow & Dynamic Routing](#control-flow--dynamic-routing)
- [Human-in-the-Loop](#human-in-the-loop)
- [Graceful Shutdown](#graceful-shutdown)
- [Agent Runtime & Middleware](#agent-runtime--middleware)
- [Structured Output](#structured-output)
- [Platform API](#platform-api)
- [Node-Result Cache](#node-result-cache)
- [Observability & Tracing](#observability--tracing)
- [Evaluation](#evaluation)
- [Configuration Examples](#configuration-examples)
- [Advanced Examples](#advanced-examples)
- [Running Examples](#running-examples)

## Getting Started

All examples require:
- Rust 1.88 or later
- API keys for LLM providers (OpenAI, DeepSeek, or Anthropic)
- Docker (for examples using Redis/MinIO)

Set API keys (recommended: keep them in `/workspace/.env`):
```bash
export OPENAI_API_KEY="your-api-key-here"
# or
export DEEPSEEK_API_KEY="your-api-key-here"
# or
export ANTHROPIC_API_KEY="your-api-key-here"
```

Or use a persistent file in this DevContainer:

```bash
cp .env.example .env
# edit .env and set OPENAI_API_KEY / DEEPSEEK_API_KEY / ANTHROPIC_API_KEY

# load for current terminal
set -a
. /workspace/.env
set +a
```

## Basic Paladin Examples

### [basic_paladin.rs](basic_paladin.rs)
**Demonstrates:** Creating and executing a simple Paladin agent

The most basic Paladin usage - create an agent with a system prompt and execute a query.

```bash
cargo run --example basic_paladin
```

**Key concepts:**
- PaladinBuilder fluent API
- System prompt configuration
- Simple execution

**Code snippet:**
```rust
let paladin = PaladinBuilder::new(llm_adapter)
    .name("Assistant")
    .system_prompt("You are a helpful AI assistant.")
    .build()?;

let response = paladin.execute("Hello!").await?;
println!("{}", response.output);
```

### [paladin_with_config.rs](paladin_with_config.rs)
**Demonstrates:** Advanced Paladin configuration

Shows how to configure temperature, max loops, stop words, and other parameters.

```bash
cargo run --example paladin_with_config
```

**Key concepts:**
- Temperature tuning
- Maximum loop control
- Stop word configuration
- Timeout settings
- Retry logic

**Code snippet:**
```rust
let paladin = PaladinBuilder::new(llm_adapter)
    .name("ConfiguredAgent")
    .system_prompt("You are a precise assistant.")
    .temperature(0.3)
    .max_loops(5)
    .stop_words(vec!["DONE", "END"])
    .timeout(Duration::from_secs(60))
    .build()?;
```

### [llm_provider_selection.rs](llm_provider_selection.rs)
**Demonstrates:** Using different LLM providers

Shows how to switch between OpenAI, DeepSeek, and Anthropic providers.

```bash
cargo run --example llm_provider_selection
```

**Key concepts:**
- Provider adapters
- Model selection
- API configuration
- Multi-provider support

## Autonomous Agent Examples

### [autonomous_planning.rs](autonomous_planning.rs) 🆕
**Demonstrates:** Autonomous planning with MaxLoops::Auto

Shows how agents automatically decompose complex tasks into structured plans and execute them sequentially.

```bash
cargo run --example autonomous_planning
```

**Key concepts:**
- MaxLoops::Auto for autonomous planning
- Automatic task complexity analysis
- Dynamic subtask generation
- Intelligent loop optimization
- Complex vs. simple task handling

**Code snippet:**
```rust
let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are an expert analyst...")
    .max_loops(MaxLoops::Auto) // Enables autonomous planning
    .build()
    .await?;

// Agent will automatically create and execute a plan
let result = service.execute(&paladin, complex_task).await?;
```

**Learn more:** See [docs/AUTONOMOUS.md](../docs/AUTONOMOUS.md) §1

### [autonomous_prompt_generation.rs](autonomous_prompt_generation.rs) 🆕
**Demonstrates:** Automatic system prompt generation

Shows how agents generate optimized system prompts from agent descriptions, eliminating manual prompt engineering.

```bash
cargo run --example autonomous_prompt_generation
```

**Key concepts:**
- Prompt generation from agent description
- Automatic persona optimization
- Role-specific prompt customization
- Reduced configuration overhead
- Examples: Code reviewer, technical writer

**Code snippet:**
```rust
let config = PaladinConfig::builder()
    .autonomous(AutonomousConfig {
        prompt_generation: Some(PromptGenerationConfig {
            enabled: true,
            description: Some("A code review specialist...".to_string()),
        }),
        ..Default::default()
    })
    .build()?;

let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("Default") // Will be auto-generated
    .with_config(config)
    .build()
    .await?;
```

**Learn more:** See [docs/AUTONOMOUS.md](../docs/AUTONOMOUS.md) §2

### [dynamic_temperature.rs](dynamic_temperature.rs) 🆕
**Demonstrates:** Dynamic temperature adjustment by task type

Shows how temperature automatically adjusts based on whether tasks are factual, creative, or balanced.

```bash
cargo run --example dynamic_temperature
```

**Key concepts:**
- Automatic temperature adjustment
- Task type detection (factual/creative/balanced)
- Low temp (0.1-0.3) for precision
- High temp (0.7-0.9) for creativity
- Medium temp (0.4-0.6) for explanations
- Configurable min/max bounds

**Code snippet:**
```rust
let config = PaladinConfig::builder()
    .autonomous(AutonomousConfig {
        dynamic_temperature: Some(DynamicTemperatureConfig {
            enabled: true,
            min_temperature: 0.1,
            max_temperature: 0.9,
            step_size: 0.1,
        }),
        ..Default::default()
    })
    .build()?;

// Temperature adjusts automatically per task
```

**Learn more:** See [docs/AUTONOMOUS.md](../docs/AUTONOMOUS.md) §3

### [agent_handoffs.rs](agent_handoffs.rs) 🆕
**Demonstrates:** Intelligent task delegation to specialist agents

Shows how coordinator agents automatically delegate subtasks to specialized experts based on task requirements.

```bash
cargo run --example agent_handoffs
```

**Key concepts:**
- Automatic delegation detection
- Specialist agent pool
- Hierarchical task distribution
- Result synthesis from multiple specialists
- Handoff strategies (automatic/manual/hybrid)
- Max depth control

**Code snippet:**
```rust
let config = PaladinConfig::builder()
    .autonomous(AutonomousConfig {
        handoffs: Some(HandoffConfig {
            enabled: true,
            strategy: HandoffStrategy::Automatic,
            max_depth: 3,
            specialist_pool: vec![
                "DatabaseArchitect".to_string(),
                "SecuritySpecialist".to_string(),
                "ApiDesigner".to_string(),
            ],
        }),
        ..Default::default()
    })
    .build()?;

// Coordinator automatically delegates to specialists
```

**Learn more:** See [docs/AUTONOMOUS.md](../docs/AUTONOMOUS.md) §4

### [autonomous_full_config.rs](autonomous_full_config.rs) 🆕
**Demonstrates:** All autonomous features working together

Shows the full power of autonomous agents with all features enabled: planning, prompt generation, dynamic temperature, and handoffs.

```bash
cargo run --example autonomous_full_config
```

**Key concepts:**
- Complete autonomous configuration
- Feature synergy and interaction
- End-to-end complex task execution
- Real-world system design scenario
- Multi-specialist coordination
- Comprehensive configuration examples

**Code snippet:**
```rust
let autonomous_config = AutonomousConfig {
    planning: Some(PlanningConfig {
        enabled: true,
        max_subtasks: 8,
    }),
    prompt_generation: Some(PromptGenerationConfig {
        enabled: true,
        description: Some("Senior software architect...".to_string()),
    }),
    dynamic_temperature: Some(DynamicTemperatureConfig {
        enabled: true,
        min_temperature: 0.1,
        max_temperature: 0.8,
        step_size: 0.1,
    }),
    handoffs: Some(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 3,
        specialist_pool: vec![/* ... */],
    }),
};

// All features work together seamlessly
```

**Learn more:** See [docs/AUTONOMOUS.md](../docs/AUTONOMOUS.md) for complete documentation

## Memory & Garrison Examples

### [garrison_in_memory.rs](garrison_in_memory.rs)
**Demonstrates:** In-memory conversation history

Shows basic memory management with the in-memory Garrison.

```bash
cargo run --example garrison_in_memory
```

**Key concepts:**
- Conversation context
- Memory windowing
- Token management
- Multi-turn dialogue

**Code snippet:**
```rust
let garrison = Arc::new(InMemoryGarrison::new(
    GarrisonConfig::default()
        .with_max_entries(100)
        .with_max_tokens(4000)
));

let paladin = PaladinBuilder::new(llm_adapter)
    .with_garrison(garrison)
    .build()?;

// First turn
paladin.execute("My name is Alice").await?;

// Second turn - remembers context
paladin.execute("What's my name?").await?;
```

### [garrison_persistent.rs](garrison_persistent.rs)
**Demonstrates:** SQLite-backed persistent memory

Shows how to save and restore conversation history across sessions.

```bash
cargo run --example garrison_persistent
```

**Key concepts:**
- SQLite storage
- Session management
- Cross-session persistence
- Database migrations

**Code snippet:**
```rust
let garrison = Arc::new(
    SqliteGarrison::new("garrison.db")
        .await?
        .with_session_id(session_id)
);

// History persists across restarts
```

### [garrison_semantic_search.rs](garrison_semantic_search.rs)
**Demonstrates:** Vector embeddings and semantic search

Shows long-term memory with semantic retrieval.

```bash
cargo run --example garrison_semantic_search
```

**Key concepts:**
- Embedding generation
- Vector similarity search
- RAG (Retrieval-Augmented Generation)
- Long-term knowledge

## Token Economy Examples

### [token_economy_commissary.rs](token_economy_commissary.rs)
**Demonstrates:** Commissary prompt-budgeting and context-window resolution

Shows the Commissary -- the input-side, per-call window-rationing officer -- measuring
an assembled prompt against a provider's own declared context window, dispensing a
bounded, priority-ordered stockpile of material, and resolving that window through the
shared `resolve_context_window` precedence chain. Runs fully offline against a
`MockLlmAdapter`; needs no provider API key.

```bash
cargo run --example token_economy_commissary
```

**Key concepts:**
- `Commissary::new` / `Commissary::dispense`
- `TokenCounterPort::is_exact` (approximate vs. exact counters)
- `resolve_context_window`, `WindowSource`, `WindowFallbackPolicy`
- `TokenUsage`'s cache-read/cache-write sub-counts (Anthropic-shaped usage)

## Sanctum Long-term Memory Examples

### [sanctum_basic_inmemory.rs](sanctum_basic_inmemory.rs)
**Demonstrates:** Basic Sanctum usage with InMemory adapter

Shows fundamental Sanctum operations: storing, searching, filtering, updating memories.

```bash
cargo run --example sanctum_basic_inmemory
```

**Key concepts:**
- InMemory adapter (development)
- Memory types (Episodic, Semantic, Procedural)
- Importance scoring (0.0-1.0)
- Semantic search with scoring
- Metadata filtering
- Batch operations

**Code snippet:**
```rust
let sanctum = InMemorySanctum::new();

let memory = MemoryBuilder::new(
    "paladin-123".to_string(),
    "User asked about Rust programming".to_string(),
)
.memory_type(MemoryType::Episodic)
.importance(0.8)
.with_metadata("topic", json!("programming"))
.build()?;

let entry = SanctumEntry::new(memory, embedding)?;
sanctum.store(entry).await?;

// Semantic search
let query = SanctumQuery::new(query_embedding, 5).min_score(0.7);
let results = sanctum.search(query).await?;
```

### [sanctum_qdrant_production.rs](sanctum_qdrant_production.rs)
**Demonstrates:** Production-ready Qdrant adapter with real embeddings

Shows how to use Sanctum in production with Qdrant vector database and OpenAI embeddings.

**Prerequisites:**
```bash
# Start Qdrant
docker run -p 6334:6334 qdrant/qdrant:latest

# Set API key
export OPENAI_API_KEY=sk-your-key
```

```bash
cargo run --example sanctum_qdrant_production
```

**Key concepts:**
- Qdrant adapter (production)
- Real vector embeddings (OpenAI)
- Persistent storage
- Performance benchmarking
- Collection statistics
- Production error handling

**Code snippet:**
```rust
let sanctum = QdrantSanctumAdapter::new(
    "http://localhost:6334",
    "paladin_memories",
    1536,  // OpenAI text-embedding-3-small dimension
).await?;

let embedding_service = OpenAIEmbeddingAdapter::new(api_key, model)?;

// Generate real embeddings
let embedding = embedding_service.embed("content").await?;
let entry = SanctumEntry::new(memory, embedding)?;
sanctum.store(entry).await?;
```

### [sanctum_adapter_migration.rs](sanctum_adapter_migration.rs)
**Demonstrates:** Migrating memories between adapters

Shows complete migration process from InMemory to Qdrant adapter.

**Prerequisites:**
```bash
docker run -p 6334:6334 qdrant/qdrant:latest
```

```bash
cargo run --example sanctum_adapter_migration
```

**Key concepts:**
- Export to JSON format
- Adapter-agnostic migration
- Import in batches
- Validation (counts, search)
- Error handling
- Cleanup procedures

**Migration phases:**
1. Export from source adapter
2. Prepare target adapter
3. Import in batches
4. Validate migration
5. Cleanup

### [sanctum_configuration.rs](sanctum_configuration.rs)
**Demonstrates:** Sanctum configuration patterns

Shows different configuration approaches for development, staging, and production.

```bash
cargo run --example sanctum_configuration
```

**Key concepts:**
- Development config (InMemory)
- Production config (Qdrant)
- Environment variable overrides
- Runtime adapter switching
- Configuration validation
- Vector dimension selection

**Configuration examples:**
```yaml
# Development
sanctum:
  enabled: true
  adapter_type: "in_memory"

# Production
sanctum:
  enabled: true
  adapter_type: "qdrant"
  qdrant:
    url: "http://qdrant:6334"
    collection_name: "paladin_production"
    vector_dimension: 1536
```

**Environment overrides:**
```bash
export APP_SANCTUM_ADAPTER_TYPE=qdrant
export APP_SANCTUM_QDRANT_URL=http://prod-qdrant:6334
export APP_SANCTUM_QDRANT_COLLECTION_NAME=memories_v2
```

### [paladin_with_sanctum.rs](paladin_with_sanctum.rs)
**Demonstrates:** Integrating Sanctum with Paladin agents

Shows how to use Sanctum for long-term memory alongside Garrison for short-term context.

```bash
cargo run --example paladin_with_sanctum
```

**Key concepts:**
- Garrison vs Sanctum (short-term vs long-term)
- Storing conversation history
- Retrieving relevant memories
- Building agent knowledge base
- Memory importance updates
- Memory analytics

**Use cases:**
- Teaching assistant building knowledge
- Customer support with history
- Personalized agent responses
- Cross-session continuity

**Code snippet:**
```rust
// Store important interaction
let memory = MemoryBuilder::new(paladin_id, content)
    .memory_type(MemoryType::Semantic)
    .importance(0.9)
    .build()?;
sanctum.store(entry).await?;

// Retrieve relevant context before responding
let query = SanctumQuery::new(query_embedding, 5).min_score(0.7);
let relevant_memories = sanctum.search(query).await?;

// Use memories to enrich agent response
let context = relevant_memories.iter()
    .map(|m| m.entry.memory.content.clone())
    .collect::<Vec<_>>()
    .join("\n");
```

**Garrison vs Sanctum:**
| Aspect | Garrison (Short-term) | Sanctum (Long-term) |
|--------|----------------------|---------------------|
| Purpose | Recent conversation | Knowledge base |
| Duration | Session-scoped | Persistent |
| Retrieval | Sequential/windowed | Semantic search |
| Size | Limited (e.g., 20 messages) | Unlimited |
| Storage | In-memory/SQLite | Vector database |

## Tool Integration Examples

### [arsenal_stdio_tools.rs](arsenal_stdio_tools.rs)
**Demonstrates:** MCP STDIO tool servers

Shows how to connect command-line tool servers via MCP protocol.

```bash
# Install MCP server first
uvx mcp-server-fetch

cargo run --example arsenal_stdio_tools
```

**Key concepts:**
- STDIO communication
- MCP protocol
- Tool discovery
- Function calling

**Code snippet:**
```rust
// MCPStdioAdapter::new(command, args) is a thin builder; connect() spawns
// the subprocess and performs the full MCP handshake, returning an MCPClient.
let client = MCPStdioAdapter::new("uvx", vec!["mcp-server-fetch"])
    .connect()
    .await?;

let tools = client.discover_tools().await?;
for tool in tools {
    registry.register(tool).await;
}

// Paladin automatically uses registered tools when needed
```

### [arsenal_streamable_http_tools.rs](arsenal_streamable_http_tools.rs)
**Demonstrates:** MCP Streamable-HTTP tool servers (authenticated remote transport)

Shows how to connect remote, optionally authenticated tool servers over the
Streamable-HTTP transport (Phase 12.1 D-02/D-03) -- the real replacement for
the retired, never-real `MCPSseAdapter` fluent auth API.

```bash
export MCP_STREAMABLE_HTTP_ENDPOINT="https://mcp.etherscan.io/mcp"
export ETHERSCAN_API_KEY="..."
export MCP_STREAMABLE_HTTP_AUTH_TOKEN_ENV="ETHERSCAN_API_KEY"
cargo run --example arsenal_streamable_http_tools
```

**Key concepts:**
- Streamable-HTTP communication (rmcp-backed, D-01/D-04)
- Bearer-token authentication sourced from an env var, never hardcoded
- Tool discovery and invocation
- Remote tools

**Code snippet:**
```rust
let client = MCPClient::connect_streamable_http(
    &endpoint,
    bearer_token.as_deref(),
    None,
)
.await?;

let tools = client.discover_tools().await?;
let result = client.invoke_tool(&tools[0].name, HashMap::new()).await?;
```

## Battalion Orchestration Examples

### [formation_sequential.rs](formation_sequential.rs)
**Demonstrates:** Sequential multi-agent execution

Shows Formation pattern where Paladins execute in sequence, passing output.

```bash
cargo run --example formation_sequential
```

**Key concepts:**
- Sequential execution
- Output passing
- Pipeline patterns
- Multi-stage processing

**Code snippet:**
```rust
let researcher = PaladinBuilder::new(llm_adapter.clone())
    .name("Researcher")
    .system_prompt("Research the topic and provide facts.")
    .build()?;

let analyst = PaladinBuilder::new(llm_adapter.clone())
    .name("Analyst")
    .system_prompt("Analyze the research and identify key insights.")
    .build()?;

let writer = PaladinBuilder::new(llm_adapter)
    .name("Writer")
    .system_prompt("Write a clear summary based on the analysis.")
    .build()?;

let formation = Formation::new("ResearchPipeline", vec![
    researcher,
    analyst,
    writer,
]);

let result = formation.execute("Explain quantum computing").await?;
```

### [phalanx_parallel.rs](phalanx_parallel.rs)
**Demonstrates:** Concurrent multi-agent execution

Shows Phalanx pattern where multiple Paladins process the same input in parallel.

```bash
cargo run --example phalanx_parallel
```

**Key concepts:**
- Parallel execution
- Result aggregation
- Concurrent processing
- Multiple perspectives

**Code snippet:**
```rust
let technical = PaladinBuilder::new(llm_adapter.clone())
    .name("TechnicalReviewer")
    .system_prompt("Review for technical accuracy.")
    .build()?;

let security = PaladinBuilder::new(llm_adapter.clone())
    .name("SecurityReviewer")
    .system_prompt("Review for security issues.")
    .build()?;

let ux = PaladinBuilder::new(llm_adapter)
    .name("UXReviewer")
    .system_prompt("Review for user experience.")
    .build()?;

let phalanx = Phalanx::new("CodeReview", vec![
    technical,
    security,
    ux,
]).with_aggregation(AggregationStrategy::Concatenate);

let result = phalanx.execute("Review this code: ...").await?;
```

### [campaign_workflow.rs](campaign_workflow.rs)
**Demonstrates:** Graph-based agent orchestration

Shows Campaign pattern with conditional routing and DAG execution.

```bash
cargo run --example campaign_workflow
```

**Key concepts:**
- Directed Acyclic Graph (DAG)
- Conditional edges
- Branching logic
- Complex workflows

**Code snippet:**
```rust
let mut campaign = Campaign::new("DataProcessing");

// Add nodes
let classifier_idx = campaign.add_paladin(classifier);
let technical_idx = campaign.add_paladin(technical_analyst);
let business_idx = campaign.add_paladin(business_analyst);

// Add conditional edges
campaign.add_edge(
    classifier_idx,
    technical_idx,
    CampaignEdge::new()
        .with_condition(EdgeCondition::OutputContains("technical"))
);

campaign.add_edge(
    classifier_idx,
    business_idx,
    CampaignEdge::new()
        .with_condition(EdgeCondition::OutputContains("business"))
);

let result = campaign.execute("Classify and analyze this document").await?;
```

### [chain_of_command_delegation.rs](chain_of_command_delegation.rs)
**Demonstrates:** Hierarchical agent delegation

Shows Chain of Command pattern with leader and specialist Paladins.

```bash
cargo run --example chain_of_command_delegation
```

**Key concepts:**
- Hierarchical structure
- Dynamic delegation
- Specialist routing
- Result synthesis

**Code snippet:**
```rust
let commander = PaladinBuilder::new(llm_adapter.clone())
    .name("Commander")
    .system_prompt("You coordinate specialists. Analyze tasks and delegate.")
    .build()?;

let specialists = vec![
    database_specialist,
    api_specialist,
    frontend_specialist,
];

let chain = ChainOfCommand::new(
    "DevelopmentTeam",
    commander,
    specialists,
).with_delegation_strategy(DelegationStrategy::CommanderChoice);

let result = chain.execute("Implement user authentication").await?;
```

### [commander_basic.rs](commander_basic.rs)
**Demonstrates:** Basic Commander usage

Shows dynamic Battalion strategy selection based on task.

```bash
cargo run --example commander_basic
```

**Key concepts:**
- Strategy routing
- Task analysis
- Dynamic selection
- Battalion types

### [commander_auto.rs](commander_auto.rs)
**Demonstrates:** Automatic strategy selection

Shows Commander with LLM-based strategy decision.

```bash
cargo run --example commander_auto
```

**Key concepts:**
- Intelligent routing
- LLM-based decisions
- Auto-optimization
- Adaptive orchestration

### [commander_full_config.rs](commander_full_config.rs)
**Demonstrates:** Complete Commander configuration

Shows all Commander features with full customization.

```bash
cargo run --example commander_full_config
```

**Key concepts:**
- Custom routing logic
- Multiple strategies
- Fallback handling
- Advanced configuration

### [commander_with_metadata_export.rs](commander_with_metadata_export.rs) 🆕
**Demonstrates:** Battalion execution metadata export

Shows how to enable and use comprehensive JSON metadata export for audit trails, performance analysis, and cost tracking.

```bash
cargo run --example commander_with_metadata_export
```

**Key concepts:**
- Metadata export configuration
- JSON file structure and naming
- Per-Paladin metrics collection
- Performance profiling
- Cost tracking
- Audit trail generation

**Code snippet:**
```rust
// Enable metadata export
let config = BattalionConfig::new("audited_battalion")
    .with_metadata_dir(PathBuf::from("./battalion_metadata"));

let commander = CommanderBuilder::new(paladin_port)
    .strategy(BattalionStrategy::Phalanx)
    .paladins(paladins)
    .config(config)
    .build()?;

// Execute and get detailed metrics
let result = commander.execute("Analyze quarterly sales data").await?;

// Access per-Paladin metrics
for (name, time_ms) in &result.per_paladin_times {
    let tokens = result.per_paladin_tokens.get(name).unwrap();
    println!("{}: {}ms, {} tokens", name, time_ms, tokens.total_tokens);
}

// Metadata automatically written to:
// ./battalion_metadata/{strategy}_{timestamp}_{uuid}.json
```

### [maneuver_basic.rs](maneuver_basic.rs) 🆕
**Demonstrates:** Flow DSL orchestration basics

Shows Maneuver pattern with string-based workflow expressions combining sequential and parallel execution.

```bash
cargo run --example maneuver_basic
```

**Key concepts:**
- Flow DSL syntax (`->` sequential, `,` parallel)
- Declarative workflows
- Mixed patterns in one expression
- Visual flow feedback

**Code snippet:**
```rust
// Define workflow with Flow DSL
let flow = "intake -> (analyzer, summarizer) -> reviewer";

// Parse flow expression
let parsed_flow = FlowParser::parse(flow)?;

// Create Maneuver with agents
let mut agents = HashMap::new();
agents.insert("intake".to_string(), intake_paladin);
agents.insert("analyzer".to_string(), analyzer_paladin);
agents.insert("summarizer".to_string(), summarizer_paladin);
agents.insert("reviewer".to_string(), reviewer_paladin);

let maneuver = Maneuver::new("DocPipeline", agents, parsed_flow, ManeuverConfig::default());

// Visualize flow
let visualizer = FlowVisualizer::new();
let ascii = visualizer.visualize(&parsed_flow, VisualizationFormat::AsciiTree)?;
println!("{}", ascii);

// Execute
let result = service.execute(&maneuver, "Analyze this document...").await?;
```

### [maneuver_nested_flow.rs](maneuver_nested_flow.rs) 🆕
**Demonstrates:** Complex nested Flow DSL patterns

Shows advanced Maneuver workflows with nested groupings, fan-out/fan-in patterns, and multi-stage processing.

```bash
cargo run --example maneuver_nested_flow
```

**Key concepts:**
- Nested flow patterns with parentheses
- Fan-out and fan-in orchestration
- Complex enterprise pipelines
- Error handling strategies

**Code snippet:**
```rust
// Complex nested flow: intake -> (analysis branch, validation branch) -> merger
let flow = "intake -> (analyzer -> (technical, security), validator -> compliance) -> merger -> formatter";

let config = ManeuverConfig {
    error_strategy: ErrorStrategy::ContinueParallel,
    output_format: OutputFormat::StructuredJson,
    collect_timing_metrics: true,
    ..Default::default()
};

let maneuver = Maneuver::new("EnterpriseReview", agents, parsed_flow, config);

// Execute with timing metrics
let result = service.execute(&maneuver, input).await?;

// Inspect timing
if let Some(metrics) = &result.timing_metrics {
    for (agent, duration) in metrics {
        println!("{}: {:?}", agent, duration);
    }
}
```

### [maneuver_dynamic_flow.rs](maneuver_dynamic_flow.rs) 🆕
**Demonstrates:** Dynamic workflow generation

Shows runtime flow creation based on task requirements with visualization and validation.

```bash
cargo run --example maneuver_dynamic_flow
```

**Key concepts:**
- Runtime flow generation
- Task-based workflow selection
- Flow validation before execution
- Adaptive orchestration

**Code snippet:**
```rust
// Generate flow based on task type
let flow = match task_type {
    TaskType::Simple => "analyzer -> formatter",
    TaskType::Review => "intake -> (technical, business) -> reviewer",
    TaskType::Complex => "intake -> (a -> b, c -> d) -> merger -> formatter",
};

// Validate flow before execution
let parsed = FlowParser::parse(flow)?;
let validation = maneuver.validate()?;
assert!(validation.is_ok(), "Flow validation failed");

// Visualize with Mermaid for documentation
let mermaid = visualizer.visualize(&parsed, VisualizationFormat::Mermaid)?;
println!("{}", mermaid);
```

### [cli_configs/maneuver.yaml](cli_configs/maneuver.yaml) 🆕

Complete Maneuver YAML configuration template for Maneuver Battalion with all options.

**Key concepts:**
- YAML-based flow definition
- Error strategy configuration (FailFast, ContinueParallel, IgnoreErrors)
- Output format options (CombinedText, StructuredJson)
- Timing metrics collection
- Multi-agent pipeline setup

**Example configuration:**
```yaml
type: maneuver
name: "DocumentAnalysisPipeline"

flow: "intake -> (analyzer, classifier) -> reviewer -> formatter"

config:
  error_strategy: FailFast
  output_format: CombinedText
  pass_output_as_input: true
  timeout_seconds: 300
  collect_timing_metrics: true
  output_separator: "\n\n---\n\n"

paladins:
  - inline:
      name: "intake"
      system_prompt: "Validate and prepare input..."
      # ... agent configuration
```

**Usage:**
```bash
# Run with configuration
paladin battalion run -c examples/cli_configs/maneuver.yaml -i "Input text"

# Visualize flow structure
paladin maneuver visualize --flow "intake -> (analyzer, classifier) -> reviewer"

# Validate configuration
paladin battalion validate -c examples/cli_configs/maneuver.yaml
```

## Output Formatting Examples

### [herald_markdown_output.rs](herald_markdown_output.rs)
**Demonstrates:** Markdown formatting

Shows how to format Paladin output as Markdown.

```bash
cargo run --example herald_markdown_output
```

**Key concepts:**
- Markdown herald
- Code blocks
- Headers and structure
- Documentation generation

**Code snippet:**
```rust
let herald = Arc::new(MarkdownHerald::new()
    .with_code_highlighting(true)
    .with_table_of_contents(true)
);

let paladin = PaladinBuilder::new(llm_adapter)
    .with_herald(herald)
    .build()?;

let response = paladin.execute("Explain Rust ownership").await?;
// Output is formatted Markdown
```

### [herald_json_output.rs](herald_json_output.rs)
**Demonstrates:** Structured JSON output

Shows JSON formatting with schema validation.

```bash
cargo run --example herald_json_output
```

**Key concepts:**
- JSON schema
- Validation
- Structured data
- API responses

**Code snippet:**
```rust
let herald = Arc::new(JsonHerald::new()
    .with_schema(schema)
    .validate_output(true)
);

let paladin = PaladinBuilder::new(llm_adapter)
    .system_prompt("Respond only in JSON: {result: string, confidence: number}")
    .with_herald(herald)
    .build()?;
```

### [herald_streaming.rs](herald_streaming.rs)
**Demonstrates:** Real-time streaming output

Shows streaming with live formatting.

```bash
cargo run --example herald_streaming
```

**Key concepts:**
- Streaming responses
- Real-time formatting
- Progress indicators
- User experience

**Code snippet:**
```rust
let mut stream = paladin.execute_stream("Write a story").await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    let formatted = herald.format_chunk(&chunk.content).await?;
    print!("{}", formatted);
    std::io::stdout().flush()?;
}
```

### [herald_custom_formatter.rs](herald_custom_formatter.rs)
**Demonstrates:** Custom Herald implementation

Shows how to create custom output formatters.

```bash
cargo run --example herald_custom_formatter
```

**Key concepts:**
- Herald trait
- Custom formatting
- Format validation
- Extensibility

## State Management Examples

### [citadel_autosave.rs](citadel_autosave.rs)
**Demonstrates:** Automatic state persistence

Shows Citadel autosave functionality for recovery.

```bash
cargo run --example citadel_autosave
```

**Key concepts:**
- State snapshots
- Automatic saving
- Checkpoint creation
- Failure recovery

**Code snippet:**
```rust
let citadel = Arc::new(
    FileCitadel::new("checkpoints/")
        .with_autosave(true)
        .with_interval(Duration::from_secs(30))
);

let paladin = PaladinBuilder::new(llm_adapter)
    .with_citadel(citadel)
    .build()?;
```

### [citadel_restore.rs](citadel_restore.rs)
**Demonstrates:** State restoration after failure

Shows how to recover Paladin state from checkpoints.

```bash
cargo run --example citadel_restore
```

**Key concepts:**
- Checkpoint restoration
- State recovery
- Resume execution
- Fault tolerance

### [battalion_checkpoint_recovery.rs](battalion_checkpoint_recovery.rs)
**Demonstrates:** Battalion state management

Shows checkpoint/recovery for multi-agent orchestration.

```bash
cargo run --example battalion_checkpoint_recovery
```

**Key concepts:**
- Battalion state
- Multi-agent recovery
- Partial execution
- Fault handling

## Vision

### [vision_analysis.rs](vision_analysis.rs)
**Demonstrates:** Basic single-image analysis using the Sentinel Vision System

Creates a vision-enabled Paladin, analyzes an image from a file, and processes the
analysis result. Needs an OpenAI API key (`OPENAI_API_KEY`) and the `vision`/`llm-openai`
features.

```bash
cargo run --example vision_analysis --features "vision,llm-openai"
```

**Key concepts:**
- Vision-enabled Paladin construction
- `VisionContent` and `ImageDetail`
- Analyzing an image from a file path
- `VisionError` handling

### [vision_battalion.rs](vision_battalion.rs)
**Demonstrates:** Multi-agent vision processing using Battalion orchestration patterns

Shows Formation (sequential vision analysis pipeline) and Phalanx (parallel vision
processing across multiple images) applied to Sentinel vision content. Needs an OpenAI
API key and the `vision`/`llm-openai` features.

```bash
cargo run --example vision_battalion --features "vision,llm-openai"
```

**Key concepts:**
- Formation: sequential vision analysis pipeline
- Phalanx: parallel vision processing across images
- Vision-enabled Battalion orchestration

## Document Processing

### [document_processing.rs](document_processing.rs)
**Demonstrates:** PDF text extraction and intelligent document chunking

Extracts text from PDF documents, reads document metadata, chunks documents for RAG or
analysis, and processes documents with Paladins. Needs the `content-processing` feature.

```bash
cargo run --example document_processing --features content-processing
```

**Key concepts:**
- `PdfExtractor` / `DocumentAdapter`
- `DocumentPort` and `DocumentSource`
- `ChunkConfig` document chunking
- Document metadata access

## HTTP Service Host

### [http_service_host.rs](http_service_host.rs)
**Demonstrates:** Booting the Paladin HTTP API in-process and calling an agent, with full router parity against the shipped server

Assembles the app exactly as the `paladin-server` binary does -- the agent router under
`/v1`, merged with the thread router and the run router (in that order), and only then
the OpenAPI docs router -- serves it on an ephemeral port, and drives it over real HTTP:
lists agents, runs one buffered and one streamed execution, calls one thread route and
one run route (both report `501 not_implemented`, matching the shipped server's own
off-by-default behavior with no waypoint/run store wired), and reads the OpenAPI title.
Hermetic -- backed by `MockLlmAdapter`, needs no network or provider keys, but does need
the `web-server` feature.

```bash
cargo run --example http_service_host --features web-server
```

**Key concepts:**
- `agent_router` + `thread_router` + `run_router` merge order (server parity)
- In-process HTTP serving on an ephemeral port
- Buffered and streamed agent execution over HTTP
- Off-by-default thread/run store wiring (`501 not_implemented`)
- OpenAPI docs router

## RAG & Retrieval

### [paladin_with_rag.rs](paladin_with_rag.rs)
**Demonstrates:** RAG (Retrieval-Augmented Generation) configuration and conceptual workflow

A conceptual, printed walkthrough of `RagRetrievalService` (automatic context retrieval
from Sanctum) and `MemoryExtractionService` (automatic memory storage) wired into
`PaladinExecutionService`'s execution flow. Calls none of the Phase 33 rationed-retrieval
API directly -- see `sanctum_rag_retrieval.rs` below for a runnable sibling that drives
the real service.

```bash
cargo run --example paladin_with_rag
```

**Key concepts:**
- `RagRetrievalService` automatic context retrieval
- `MemoryExtractionService` automatic memory storage
- RAG configuration via `config.yml` / `examples/cli_configs/paladin_rag.yaml`
- Building agent knowledge over sessions

### [sanctum_rag_retrieval.rs](sanctum_rag_retrieval.rs)
**Demonstrates:** `RagRetrievalResult` typed retrieval, `ShedItem` shed records, a typed `RagRetrievalError`, the timeout-bounded `retrieve_context_with_timeout` free function, and exact `TokenCounterPort` injection via `with_token_counter`

A runnable sibling to `paladin_with_rag.rs`'s conceptual walkthrough: drives
`RagRetrievalService` for real over an in-memory Sanctum, reads back the typed
`RagRetrievalResult` and its `ShedItem` records when a token budget forces memories out,
matches a failing retrieval against the typed `RagRetrievalError` enum, calls the
timeout-bounded `retrieve_context_with_timeout` free function, and contrasts an exact
`TokenCounterPort` against the heuristic default. Fully offline -- in-memory Sanctum,
deterministic embedding stand-in.

```bash
cargo run --example sanctum_rag_retrieval
```

**Key concepts:**
- `RagRetrievalResult` typed retrieval
- `ShedItem` shed records under a rationed budget
- Typed `RagRetrievalError` (e.g. `BudgetTooLarge`)
- `retrieve_context_with_timeout` free function
- `with_token_counter` exact-counter injection

## Commander Strategies (Council / Grove / Conclave)

### [commander_council.rs](commander_council.rs)
**Demonstrates:** Commander orchestrating Council discussions with different turn-taking strategies and termination conditions

Shows Commander automatically selecting the Council strategy, contrasting RoundRobin and
ModeratorDirected turn-taking, and MaxRounds/Consensus/Keyword termination conditions,
with formatted discussion output.

```bash
cargo run --example commander_council
```

**Key concepts:**
- Commander Council strategy selection
- Turn-taking strategies (RoundRobin, ModeratorDirected)
- Termination conditions (MaxRounds, Consensus, Keyword)
- Formatted discussion output

### [commander_grove.rs](commander_grove.rs)
**Demonstrates:** Commander orchestrating Grove routing with all three routing strategies

Shows Commander automatically selecting the Grove strategy across KeywordMatch (fast,
deterministic), SemanticSimilarity (contextual, embedding-based) and LlmRouting
(intelligent, LLM-powered), plus fallback behavior and confidence scoring.

```bash
cargo run --example commander_grove
```

**Key concepts:**
- Commander Grove strategy selection
- KeywordMatch / SemanticSimilarity / LlmRouting
- Fallback behavior
- Confidence scoring

### [conclave_expert_panel.rs](conclave_expert_panel.rs)
**Demonstrates:** The Conclave Mixture-of-Agents pattern -- parallel expert analysis synthesized by an aggregator

Multiple specialized Paladins (Technical, Business, Security) analyze a task in
parallel, and an aggregator synthesizes their diverse perspectives into one response,
with retry logic and partial-success handling.

```bash
cargo run --example conclave_expert_panel
```

**Key concepts:**
- Expert parallel execution
- Diverse perspectives (Technical, Business, Security)
- Synthesis aggregation
- Retry logic with exponential backoff
- Partial success handling
- Observability levels (Minimal/Standard/Verbose)

### [council_discussion.rs](council_discussion.rs)
**Demonstrates:** The Council pattern with multiple expert Paladins engaged in a structured discussion

Several expert Paladins discuss implementing two-factor authentication using a
round-robin turn-taking strategy and a maximum-rounds termination condition, with a
formatted discussion transcript.

```bash
cargo run --example council_discussion
```

**Key concepts:**
- Council with multiple expert participants
- Round-robin turn-taking strategy
- Maximum rounds termination condition
- Formatted discussion transcript

### [grove_routing.rs](grove_routing.rs)
**Demonstrates:** The Grove pattern for routing tasks to specialized agent trees based on keyword matching

Creates a Grove with multiple expert trees, routes tasks by keyword match to specialized
agents, and prints the routing decision and confidence score.

```bash
cargo run --example grove_routing
```

**Key concepts:**
- Grove with multiple expert trees
- Keyword-based routing strategy
- Specialized agents with specific expertise
- Routing decision visibility and confidence scoring

## Performance Benchmarking Examples

### [muster_baseline.rs](muster_baseline.rs)
**Demonstrates:** Performance-baseline measurement harness

The recorded harness behind `docs/src/appendix/performance-baseline.md`'s memory-per-Paladin
and startup-time figures — the two metric families `criterion` benchmarks do not produce.

```bash
APP_ENV=test cargo run --offline --release --example muster_baseline
```

**Key concepts:**
- Process RSS delta measurement via `/proc/self/status`
- In-process startup timing
- Measurement harnesses vs. `criterion` benchmarks
- Host-specific baseline figures (not portable performance claims)

### [war_engine_memory_baseline.rs](war_engine_memory_baseline.rs)
**Demonstrates:** WarEngine memory-per-superstep measurement harness (ENG-NFR-02)

A recorded measurement harness for the memory half of ENG-NFR-02 ("one Battlefield clone
per superstep maximum, plus one per concurrently executing node view"). Reads the
process's resident set size (RSS) from `/proc/self/status` before and after a fixed
`WarGraph` workload and reports the delta, following `muster_baseline.rs`'s exact method.

```bash
cargo run --release --example war_engine_memory_baseline
```

**Key concepts:**
- Process RSS delta measurement via `/proc/self/status`
- `Arc<Battlefield>` clone-count measurement through the public `WarEngine` API
- Fixed-width, fixed-depth `WarGraph` workload
- Host-specific baseline figures (not portable performance claims)

## WarEngine Configuration & Checkpoints

### [war_engine_configuration.rs](war_engine_configuration.rs)
**Demonstrates:** WaypointPort injection, EngineConfig, the superstep-cap environment override, checkpoint history read-back, WaypointRetentionService pruning, and the graph fingerprint version

Injects an explicit `InMemoryWaypointStore` as the `WaypointPort` implementor,
configures a `WarEngine` via `EngineConfig` naming every bounded-iteration/durability
field, overrides `APP_ENGINE_MAX_SUPERSTEPS` in-process, runs a small cyclic `WarGraph`
and reads its checkpoint history back through the port, prunes that history with
`WaypointRetentionService`, and prints `GRAPH_FINGERPRINT_VERSION`. Fully offline --
reads no LLM provider API key.

```bash
cargo run --example war_engine_configuration
```

**Key concepts:**
- `WaypointPort` injection (`InMemoryWaypointStore`)
- `EngineConfig` (max_supersteps, max_node_visits, run_timeout_secs, waypoint_durability, max_muster_tasks)
- `APP_ENGINE_MAX_SUPERSTEPS` environment override
- Checkpoint history read-back
- `WaypointRetentionService` pruning
- `GRAPH_FINGERPRINT_VERSION`

## Control Flow & Dynamic Routing

### [control_flow_dynamic_routing.rs](control_flow_dynamic_routing.rs)
**Demonstrates:** Custom edge conditions (fail-closed vs. registered), a nested Battalion subgraph, LLM-driven routing, and the Muster fan-out cap

Registers an `EdgeCondition::Custom` evaluator and contrasts the fail-closed
`EngineError::UnregisteredEdgeCondition` outcome against the edge taken once
registered; embeds a child `WarGraph` as a `NodeSpec::Battalion` node; drives an edge
decision from `MockLlmAdapter` through `LlmDecisionEvaluator`; and overrides
`APP_ENGINE_MAX_MUSTER_TASKS`, enforced against a running engine. Fully offline --
every LLM call goes through `MockLlmAdapter`.

```bash
cargo run --example control_flow_dynamic_routing
```

**Key concepts:**
- `EdgeCondition::Custom` fail-closed vs. registered contrast
- `NodeSpec::Battalion` nested subgraph
- `LlmDecisionEvaluator` LLM-driven routing
- `APP_ENGINE_MAX_MUSTER_TASKS` fan-out cap enforcement

## Human-in-the-Loop

### [human_in_the_loop_gate.rs](human_in_the_loop_gate.rs)
**Demonstrates:** Pausing at a Gate node, resuming with typed responses (total validation), and replaying the thread onto a new branch

Runs a graph whose entry point is a `NodeSpec::Gate` to `RunOutcome::AwaitingInput`,
calls `WarEngine::resume_with` and shows a total-validation rejection of an unrelated
parley id before the correct response completes the run, reads the history back
through `ChronicleService`, then replays the run onto a new branch resumed with the
opposite decision to a divergent result. Fully offline -- no Paladin node, no LLM call.

```bash
cargo run --example human_in_the_loop_gate
```

**Key concepts:**
- `NodeSpec::Gate` pause
- `WarEngine::resume_with` typed total validation (`EngineError::UnknownParleyId`)
- `ChronicleService` history read-back
- `WarEngine::replay` onto a new branch

## Graceful Shutdown

### [graceful_shutdown.rs](graceful_shutdown.rs)
**Demonstrates:** Draining in-flight work, configuring the grace period from the environment, and toggling graceful shutdown off and on

Registers a run with a `ShutdownCoordinator`, fans out a fast and a slow node, and
drains in-flight work in one `Halted` checkpoint showing both `Succeeded` and
`Skipped { reason: "shutdown" }` outcomes; overrides `APP_ENGINE_SHUTDOWN_GRACE_SECS`
and re-runs the drain bounded at the new value; and toggles
`APP_ENGINE_GRACEFUL_SHUTDOWN` to contrast exit-immediately against wait-and-drain.
Fully offline -- no Paladin node, no LLM call, no real signal handler.

```bash
cargo run --example graceful_shutdown
```

**Key concepts:**
- `ShutdownCoordinator::register` / `cancel_and_wait`
- Draining a fan-out of in-flight work into one `Halted` checkpoint
- `APP_ENGINE_SHUTDOWN_GRACE_SECS` grace-period override
- `APP_ENGINE_GRACEFUL_SHUTDOWN` toggle

## Agent Runtime & Middleware

### [agent_runtime_middleware.rs](agent_runtime_middleware.rs)
**Demonstrates:** Custom ExecutionMiddleware hooks, AgentRuntimeConfig-resolved built-in middleware, custom token-counter injection, context-window management, ConfinedVault memory namespacing, and the fail-run tool error mode

A custom `ExecutionMiddleware`'s `before_model`/`after_model`/`around_tool` hooks fire
against a real `PaladinExecutionService::execute` run; `AgentRuntimeConfig::build_chain`
resolves middleware from configuration; a custom `TokenCounterPort` is contrasted with
the defaulted heuristic counter; `HistoryTrimmer` and `SummarizationMiddleware` reduce
a long history; two `ConfinedVault` handles namespace two agents' memories apart with a
denied cross-namespace read; and `ToolErrorMode::FailRun` fails a run with a structured
`PaladinError::ArmamentFailed`. Fully offline -- uses `MockLlmAdapter` and in-memory
ports.

```bash
cargo run --example agent_runtime_middleware
```

**Key concepts:**
- Custom `ExecutionMiddleware` hook chain
- `AgentRuntimeConfig::build_chain`
- Custom `TokenCounterPort` injection
- `HistoryTrimmer` + `SummarizationMiddleware`
- `ConfinedVault` memory namespacing
- `ToolErrorMode::FailRun` -> `PaladinError::ArmamentFailed`

## Structured Output

### [structured_output_schema.rs](structured_output_schema.rs)
**Demonstrates:** JSON Schema derivation via schemars and schema-validated structured output (accept and reject)

Derives a JSON Schema from a Rust type via `schemars::schema_for!`, executes a Paladin
through the typed structured-execution path (`StructuredExecutorExt::execute_structured`)
against a conforming response and prints the typed value's own fields, then drives a
schema-violating response to a typed `PaladinError::StructuredOutputInvalid` rejection
rather than silent acceptance. Fully offline -- uses `MockLlmAdapter`.

```bash
cargo run --example structured_output_schema
```

**Key concepts:**
- `schemars::schema_for!` JSON Schema derivation
- `StructuredExecutorExt::execute_structured`
- Typed value returned (not a raw string)
- `PaladinError::StructuredOutputInvalid` typed rejection

## Platform API

### [platform_api_client.rs](platform_api_client.rs)
**Demonstrates:** The full Platform API surface driven in-process: run submit/stream/cancel, assistants, schedules, thread state/resume/history, the dev-ui inspector route, token usage, and queue/store backend selection

An in-process, fully in-memory Platform API client (every durable store is the
`paladin-storage` in-memory adapter for its port) that submits, streams and cancels
runs; creates and publishes assistant versions; creates and lists a cron schedule;
calls thread state/resume/history and the admin-and-feature-gated dev-ui inspector
route (against a Waypoint-less thread -- see the program's own header for what it
cannot demonstrate offline); prints the prompt/completion token split; and prints
which run-queue/run-store implementation is in force. Hermetic -- backed by
`MockLlmAdapter`, reads no provider key, needs the `web-server` and `dev-ui` features.

```bash
cargo run --example platform_api_client --features "web-server,dev-ui"
```

**Key concepts:**
- Run submission, SSE streaming, cancellation
- Assistants (create/publish/list versions)
- Schedules (create/list)
- Thread state/resume/history routes
- The dev-ui inspector route
- Token usage (prompt/completion split)
- Run queue / run store backend selection

### [webhook_receiver.rs](webhook_receiver.rs)
**Demonstrates:** Webhook signature verification and the private-address SSRF override, from the receiving end

Stands up a receiver on an ephemeral loopback port that captures the raw request body
before deserialization and recomputes the digest with `sign_webhook_body` -- the same
function and `hmac`/`sha2` crates the shipped `WebhookDeliveryService` signs with --
verifying in constant time via `hmac::Mac::verify_slice`; verifies a genuine delivery,
rejects a tampered body under the same signature, and names the
`APP_WEBHOOKS_ALLOW_PRIVATE` override against the receiver's own loopback address and
the always-rejected cloud metadata address. Hermetic -- no provider key, no external
service, needs the `web-server` feature.

```bash
cargo run --example webhook_receiver --features "web-server"
```

**Key concepts:**
- Raw-byte signature verification (`sign_webhook_body`, reused verbatim from the sender)
- `X-Paladin-Signature: sha256=<hex>` header verification
- Constant-time comparison (`hmac::Mac::verify_slice`)
- `APP_WEBHOOKS_ALLOW_PRIVATE` private-address SSRF override
- The always-rejected cloud metadata address (169.254.169.254)

## Node-Result Cache

### [node_result_cache.rs](node_result_cache.rs)
**Demonstrates:** The Redis-backed node-result cache adapter and the node-cache enable/disable toggle

Constructs the `redis-cache`-gated `RedisNodeCache` adapter and wires it onto a
`WarEngine` via `with_node_cache`; runs a graph twice under different threads to show
a cache miss then a hit (no re-execution, proven from the persisted Waypoint's own
`cache_hit` field); then toggles `APP_NODE_CACHE_ENABLED` off and runs a graph with no
`CachePolicy` attached on an engine with no cache backend wired to show the cache
bypassed entirely. Needs a running Redis server (`make services-up`); build-only in CI
(no Redis server available there).

```bash
cargo build --example node_result_cache --features "redis-cache"
```

**Key concepts:**
- `RedisNodeCache` (`NodeCachePort` adapter)
- `WarEngine::with_node_cache`
- Cache miss vs. hit via `NodeExecutionRecord.cache_hit`
- `APP_NODE_CACHE_ENABLED` toggle
- `EngineError::CachePolicyWithoutCacheBackend`

## Observability & Tracing

### [observability_tracing.rs](observability_tracing.rs)
**Demonstrates:** The trace record envelope and real event variant names, TraceConfig-driven sink composition, the OTLP environment toggle, and persisted trace history read-back

Runs a small graph with a custom in-process `TraceSink` and prints every captured
`TraceRecord`'s envelope plus its real `TraceEvent` variant name; constructs a
`TraceConfig` and calls the facade's own
`paladin::infrastructure::telemetry::build_run_sink` to show a single changed field's
None-vs-Some effect; sets `PALADIN_TRACE_OTEL_ENABLED` and shows export also requires
the `otel` Cargo feature; and reads the same run's persisted trace rows back via
`RunTracePort::read`. Fully offline -- default features, no LLM provider key needed.

```bash
cargo run --example observability_tracing
```

**Key concepts:**
- `TraceEvent` / `TraceRecord` envelope
- `paladin::infrastructure::telemetry::build_run_sink`
- `PALADIN_TRACE_OTEL_ENABLED` environment toggle
- `RunTracePort::read` persisted trace history

### [observability_otel_export.rs](observability_otel_export.rs)
**Demonstrates:** The OTLP trace export sink (OtelTraceSink)

Wires the `otel`-gated `OtelTraceSink` onto a `WarEngine` run and prints the endpoint
configuration it exports through. Needs a reachable OTLP/HTTP collector (an
OpenTelemetry Collector, Jaeger, or similar) at the configured endpoint; build-only in
CI (no collector available there).

```bash
cargo build --example observability_otel_export --features "otel"
```

**Key concepts:**
- `OtelTraceSink` OTLP export
- `otel` Cargo feature (pulls in `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`)
- Configured collector endpoint (`http://localhost:4318/v1/traces` by default)

## Evaluation

### [eval_scenarios_demo.rs](eval_scenarios_demo.rs)
**Demonstrates:** Scenario declaration and ScenarioRunner::run_case, the PALADIN_EVAL_LIVE live-mode toggle, and the CLI eval-run command-line form

Declares two scenarios in Rust against a Paladin built with `MockLlmAdapter` and runs
them through `ScenarioRunner::run_case`; names `PALADIN_EVAL_LIVE`'s effect via
`check_live_mode(false)`'s typed `Err(LiveModeError::FlagNotSet)` refusal; and writes a
real `.eval.yaml` scenario file, prints the equivalent `paladin eval run` CLI command,
and drives the same glob through `ScenarioRunner::trials` and `Scenario::from_path` +
`run_case` in-process. Fully offline -- needs no `required-features` (`paladin-eval` is
an unconditional dev-dependency).

```bash
cargo run --example eval_scenarios_demo
```

**Key concepts:**
- `Scenario` / `Case` / `ScenarioRunner::run_case`
- `PALADIN_EVAL_LIVE` live-mode toggle (`check_live_mode`)
- `.eval.yaml` glob discovery (`ScenarioRunner::trials`)
- The CLI `paladin eval run "<glob>"` equivalent

## Configuration Examples

The `cli_configs/` directory contains YAML configuration examples:

### [basic_paladin.yaml](cli_configs/basic_paladin.yaml)
Basic Paladin configuration for CLI usage.

### [advanced_paladin.yaml](cli_configs/advanced_paladin.yaml)
Advanced Paladin with all options configured.

### [formation.yaml](cli_configs/formation.yaml)
Formation Battalion configuration.

### [phalanx.yaml](cli_configs/phalanx.yaml)
Phalanx Battalion configuration.

### [campaign.yaml](cli_configs/campaign.yaml)
Campaign Battalion with graph structure.

### [chain_of_command.yaml](cli_configs/chain_of_command.yaml)
Chain of Command configuration.

## Advanced Examples

> **Note:** the snippets below are illustrative pseudocode meant to convey a
> pattern, not compiled or verified against the current API the way every
> `### [name.rs](name.rs)` example above is (those all build and run in CI).
> `load_config()`, `create_llm_adapter()` and `create_fallback_adapter()`
> below are placeholders for your own code, not real Paladin functions.

### Error Handling Patterns

Most examples include robust error handling:

```rust
use paladin::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = load_config()?;

    // Initialize adapters with fallback
    let llm_adapter = create_llm_adapter(&config)
        .or_else(|_| create_fallback_adapter())?;

    // Create Paladin with retries (PaladinBuilder::retry_attempts, not
    // max_retries/retry_delay -- there is no per-Paladin retry-delay setting)
    let paladin = PaladinBuilder::new(llm_adapter)
        .retry_attempts(3)
        .build()
        .await?;

    // Execute with timeout
    let result = tokio::time::timeout(
        Duration::from_secs(60),
        paladin.execute(input)
    ).await??;

    Ok(())
}
```

### Logging and Observability

Examples include tracing integration:

```rust
use tracing::{info, warn, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Paladin execution");

    let response = paladin.execute(input).await?;

    info!(
        tokens = response.usage.total_tokens,
        duration = ?response.execution_time_ms,
        "Execution completed"
    );

    Ok(())
}
```

## Running Examples

### Run a Specific Example

```bash
cargo run --example basic_paladin
```

### Run with Specific Features

```bash
# With Redis queue support
cargo run --example basic_paladin --features redis-queue

# With S3 storage
cargo run --example basic_paladin --features s3-storage

# All features
cargo run --example basic_paladin --all-features
```

### Run in Release Mode

```bash
cargo run --release --example basic_paladin
```

### Set Log Level

```bash
RUST_LOG=debug cargo run --example basic_paladin
```

### With Custom Config

```bash
cargo run --example basic_paladin -- --config my-config.yml
```

## Example Dependencies

Some examples require external services:

### Start Docker Services

```bash
# All services (Redis, MinIO)
make dev

# Or individually
docker-compose -f docker/docker-compose.yml up redis -d
docker-compose -f docker/docker-compose.yml up minio -d
```

### Install MCP Servers

```bash
# Web search
uvx mcp-server-fetch

# File system
uvx mcp-server-filesystem

# Calculator
uvx mcp-server-calculator
```

## Building a Custom Example

Create a new example in `examples/my_example.rs`:

```rust
use paladin::prelude::*;
use paladin_llm::openai::adapter::{OpenAIAdapter, OpenAIConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load API key
    let api_key = std::env::var("OPENAI_API_KEY")?;

    // Create LLM adapter (OpenAIAdapter::new takes an OpenAIConfig, not a
    // builder chain -- the model is set on PaladinBuilder below, per-request,
    // not on the adapter itself)
    let llm_adapter: Arc<dyn LlmPort> = Arc::new(
        OpenAIAdapter::new(OpenAIConfig::new(api_key))?
    );

    // Create Paladin
    let paladin = PaladinBuilder::new(llm_adapter)
        .name("MyPaladin")
        .system_prompt("You are a helpful assistant.")
        .model("gpt-4")
        .build()
        .await?;

    // Execute
    let response = paladin.execute("Hello!").await?;
    println!("{}", response.output);

    Ok(())
}
```

Run it:
```bash
cargo run --example my_example
```

## Troubleshooting

### Example Won't Compile

```bash
# Clean and rebuild
cargo clean
cargo build --examples

# Check specific example
cargo check --example basic_paladin
```

### Missing API Key

```bash
# Preferred: use .env in workspace
cp .env.example .env
# set OPENAI_API_KEY=sk-... in .env

# Load into current terminal
set -a
. /workspace/.env
set +a

# Then run example
cargo run --example basic_paladin
```

### Service Connection Errors

```bash
# Check services are running
make health

# Restart services
make dev-restart

# View logs
docker-compose -f docker/docker-compose.yml logs
```

## Learning Path

Recommended order for learning:

1. **Start Simple**
   - `basic_paladin.rs` - Understand core concepts
   - `paladin_with_config.rs` - Learn configuration

2. **Add Memory**
   - `garrison_in_memory.rs` - Basic memory
   - `garrison_persistent.rs` - Persistence

3. **Add Tools**
   - `arsenal_stdio_tools.rs` - Tool integration
   - Custom tool creation

4. **Multi-Agent**
   - `formation_sequential.rs` - Sequential
   - `phalanx_parallel.rs` - Parallel
   - `campaign_workflow.rs` - Advanced

5. **Production Features**
   - `herald_json_output.rs` - Structured output
   - `citadel_autosave.rs` - State management
   - `commander_auto.rs` - Dynamic routing

## Contributing Examples

Want to add an example? See [CONTRIBUTING.md](../docs/contributing/CONTRIBUTING.md) for guidelines.

Good example characteristics:
- Self-contained (single file)
- Well-commented
- Demonstrates one clear concept
- Includes error handling
- Has descriptive output

## Next Steps

- **[Quickstart Guide](../docs/QUICKSTART.md)** - Get started with Paladin
- **[User Guides](../docs/guides/)** - In-depth documentation
- **[API Reference](https://docs.rs/paladin)** - Complete API docs
- **[Architecture](../docs/architecture/)** - System design details

## Questions?

- Check [Documentation](../docs/)
- Open an [Issue](https://github.com/DF3NDR/paladin-dev-env/issues)
