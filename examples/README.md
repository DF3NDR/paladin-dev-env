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
- [Performance Benchmarking Examples](#performance-benchmarking-examples)
- [Configuration Examples](#configuration-examples)
- [Advanced Examples](#advanced-examples)
- [Running Examples](#running-examples)

## Getting Started

All examples require:
- Rust 1.70 or later
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
println!("{}", response.content);
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
**Demonstrates:** Complete Maneuver YAML configuration

Shows full configuration template for Maneuver Battalion with all options.

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

    // Create Paladin with retries
    let paladin = PaladinBuilder::new(llm_adapter)
        .max_retries(3)
        .retry_delay(Duration::from_secs(1))
        .build()?;

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
        tokens = response.token_usage.total_tokens,
        duration = ?response.execution_time,
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
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load API key
    let api_key = std::env::var("OPENAI_API_KEY")?;

    // Create LLM adapter
    let llm_adapter = Arc::new(
        OpenAiAdapter::new()
            .api_key(&api_key)
            .model("gpt-4")
            .build()?
    );

    // Create Paladin
    let paladin = PaladinBuilder::new(llm_adapter)
        .name("MyPaladin")
        .system_prompt("You are a helpful assistant.")
        .build()?;

    // Execute
    let response = paladin.execute("Hello!").await?;
    println!("{}", response.content);

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
- Open an [Issue](https://github.com/your-org/paladin/issues)
- Join [Discord](https://discord.gg/paladin) (if available)
