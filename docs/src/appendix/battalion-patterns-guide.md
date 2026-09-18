# Battalion Patterns Guide

Multi-agent orchestration patterns for coordinating Paladins. This guide covers Formation, Phalanx, Campaign, and Chain of Command patterns with practical examples and decision criteria.

## Table of Contents

- [Overview](#overview)
- [Formation (Sequential)](#formation-sequential)
- [Phalanx (Parallel)](#phalanx-parallel)
- [Campaign (Graph/DAG)](#campaign-graphdag)
- [Chain of Command (Hierarchical)](#chain-of-command-hierarchical)
- [Pattern Selection Guide](#pattern-selection-guide)
- [Common Pitfalls](#common-pitfalls)
- [Performance Considerations](#performance-considerations)

## Overview

Battalions coordinate multiple Paladins to solve complex tasks that require:
- Sequential processing of information
- Parallel analysis of different aspects
- Complex multi-step workflows with dependencies
- Hierarchical decision-making

**Key Concept**: Each Paladin in a Battalion is an independent AI agent with its own configuration, but they work together under coordinated execution patterns.

## Formation (Sequential)

**Pattern**: Execute Paladins one after another, passing output from one to the next.

**Use When**:
- Output of one Paladin is input to the next
- Tasks have a natural sequential flow
- Each step builds on previous results

### Example: Research → Analysis → Summary

```rust,ignore
use paladin::core::platform::container::battalion::*;
use paladin::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm_adapter = Arc::new(OpenAIAdapter::new().build()?);

    // Researcher Paladin
    let researcher = PaladinBuilder::new(llm_adapter.clone())
        .name("Researcher")
        .system_prompt("You are a research assistant. Gather relevant information on the given topic. \
                        Output key facts and sources.")
        .temperature(0.5)
        .build()?;

    // Analyst Paladin
    let analyst = PaladinBuilder::new(llm_adapter.clone())
        .name("Analyst")
        .system_prompt("You are a data analyst. Analyze the research provided and identify trends, \
                        insights, and patterns. Output structured analysis.")
        .temperature(0.6)
        .build()?;

    // Writer Paladin
    let writer = PaladinBuilder::new(llm_adapter)
        .name("Writer")
        .system_prompt("You are a technical writer. Take the analysis and create a clear, \
                        concise summary for executives. Output professional report.")
        .temperature(0.7)
        .build()?;

    // Create Formation
    let formation = Formation::new()
        .add_paladin(researcher)
        .add_paladin(analyst)
        .add_paladin(writer)
        .build()?;

    // Execute
    let result = formation.execute("Analyze trends in Rust adoption 2024").await?;
    println!("{}", result.final_output);

    Ok(())
}
```

### Data Flow

```
Input: "Analyze Rust trends 2024"
    ↓
┌─────────────────┐
│   Researcher    │ → "Rust usage increased 45% in 2024..."
└─────────────────┘
    ↓
┌─────────────────┐
│    Analyst      │ → "Key trends: adoption in embedded systems..."
└─────────────────┘
    ↓
┌─────────────────┐
│     Writer      │ → "Executive Summary: Rust shows strong growth..."
└─────────────────┘
    ↓
Output: Professional report
```

### Configuration Options

```rust,ignore
let formation = Formation::new()
    .add_paladin(p1)
    .add_paladin(p2)
    .checkpoint_enabled(true)           // Save state after each step
    .stop_on_error(false)               // Continue even if one Paladin fails
    .output_format(OutputFormat::Json)  // Structured output
    .build()?;
```

## Phalanx (Parallel)

**Pattern**: Execute multiple Paladins concurrently, then aggregate results.

**Use When**:
- Tasks can be processed independently
- Need to analyze same input from different perspectives
- Want to reduce overall execution time
- Generating diverse ideas or solutions

### Example: Multi-Perspective Analysis

```rust,ignore
use paladin::core::platform::container::battalion::*;
use paladin::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm_adapter = Arc::new(OpenAIAdapter::new().build()?);

    // Technical Reviewer
    let technical = PaladinBuilder::new(llm_adapter.clone())
        .name("TechnicalReviewer")
        .system_prompt("Review code from a technical perspective: correctness, efficiency, safety.")
        .build()?;

    // Security Reviewer
    let security = PaladinBuilder::new(llm_adapter.clone())
        .name("SecurityReviewer")
        .system_prompt("Review code from a security perspective: vulnerabilities, unsafe practices.")
        .build()?;

    // UX Reviewer
    let ux = PaladinBuilder::new(llm_adapter.clone())
        .name("UXReviewer")
        .system_prompt("Review code from a UX perspective: usability, error messages, documentation.")
        .build()?;

    // Aggregator
    let aggregator = PaladinBuilder::new(llm_adapter)
        .name("Aggregator")
        .system_prompt("Combine multiple code reviews into a single coherent report. \
                        Prioritize critical issues and provide actionable feedback.")
        .build()?;

    // Create Phalanx
    let phalanx = Phalanx::new()
        .add_paladin(technical)
        .add_paladin(security)
        .add_paladin(ux)
        .aggregator(aggregator)
        .max_concurrency(3)  // Run all 3 in parallel
        .build()?;

    let code = r#"
        pub fn process_user_input(input: String) -> Result<String> {
            // Code to review...
        }
    "#;

    let result = phalanx.execute(code).await?;
    println!("{}", result.aggregated_output);

    Ok(())
}
```

### Data Flow

```
Input: "Code to review"
    ↓
┌──────────────────────────────────────┐
│  ┌─────────┐  ┌─────────┐  ┌───────┐│
│  │Technical│  │Security │  │  UX   ││  (Parallel execution)
│  └─────────┘  └─────────┘  └───────┘│
└──────────────────────────────────────┘
    ↓          ↓         ↓
┌─────────────────────────────────────┐
│          Aggregator                  │
└─────────────────────────────────────┘
    ↓
Output: Combined review report
```

### Performance Tuning

```rust,ignore
let phalanx = Phalanx::new()
    .add_paladin(p1)
    .add_paladin(p2)
    .add_paladin(p3)
    .max_concurrency(2)                    // Limit concurrent executions
    .timeout(Duration::from_secs(60))       // Overall timeout
    .aggregation_strategy(AggregationStrategy::Weighted) // Custom aggregation
    .build()?;
```

## Campaign (Graph/DAG)

**Pattern**: Execute Paladins based on a directed acyclic graph (DAG) with conditional flows and dependencies.

**Use When**:
- Complex workflows with branching logic
- Tasks have multiple dependencies
- Need conditional execution paths
- Implementing state machines or decision trees

### Example: Content Generation Pipeline

```rust,ignore
use paladin::core::platform::container::battalion::*;
use paladin::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm_adapter = Arc::new(OpenAIAdapter::new().build()?);

    // Define Paladins
    let topic_generator = create_paladin("TopicGenerator", "Generate blog post topics", llm_adapter.clone())?;
    let researcher = create_paladin("Researcher", "Research the topic", llm_adapter.clone())?;
    let outline_creator = create_paladin("OutlineCreator", "Create article outline", llm_adapter.clone())?;
    let writer = create_paladin("Writer", "Write the article", llm_adapter.clone())?;
    let fact_checker = create_paladin("FactChecker", "Verify factual accuracy", llm_adapter.clone())?;
    let editor = create_paladin("Editor", "Edit and polish", llm_adapter)?;

    // Build Campaign Graph
    let campaign = Campaign::new()
        // Initial node
        .add_node("generate_topic", topic_generator)

        // Research path
        .add_node("research", researcher)
        .add_edge("generate_topic", "research")

        // Parallel outline and fact-checking
        .add_node("outline", outline_creator)
        .add_node("fact_check", fact_checker)
        .add_edge("research", "outline")
        .add_edge("research", "fact_check")

        // Converge at writing
        .add_node("write", writer)
        .add_edge("outline", "write")
        .add_edge("fact_check", "write")

        // Final editing
        .add_node("edit", editor)
        .add_edge("write", "edit")

        // Conditional re-check if needed
        .add_conditional("edit", "fact_check", |output| {
            output.contains("NEEDS_VERIFICATION")
        })

        .build()?;

    let result = campaign.execute("AI in healthcare").await?;
    println!("{}", result.final_output);

    Ok(())
}
```

### Graph Visualization

```
          ┌──────────────────┐
          │ generate_topic   │
          └──────────────────┘
                   ↓
          ┌──────────────────┐
          │    research      │
          └──────────────────┘
                   ↓
         ┌─────────┴─────────┐
         ↓                   ↓
┌─────────────┐      ┌──────────────┐
│  outline    │      │ fact_check   │
└─────────────┘      └──────────────┘
         ↓                   ↓
         └─────────┬─────────┘
                   ↓
          ┌──────────────────┐
          │     write        │
          └──────────────────┘
                   ↓
          ┌──────────────────┐
          │      edit        │
          └──────────────────┘
                   ↓ (conditional)
          ┌──────────────────┐
          │  fact_check      │  (if needed)
          └──────────────────┘
```

### Advanced Features

```rust,ignore
let campaign = Campaign::new()
    .add_node("start", start_paladin)
    .add_node("process", process_paladin)

    // Conditional edges
    .add_conditional("start", "process", |output| {
        output.score > 0.8
    })

    // Error handling
    .add_error_handler("process", fallback_paladin)

    // Checkpointing
    .enable_checkpoints(true)

    // Max iterations for cycles (with safeguards)
    .max_iterations(10)

    .build()?;
```

## Chain of Command (Hierarchical)

**Pattern**: Hierarchical delegation where a commander Paladin delegates subtasks to subordinate Paladins.

**Use When**:
- Tasks require decomposition into subtasks
- Need dynamic task distribution
- Implementing hierarchical decision-making
- Agent supervision and coordination

### Example: Project Planning

```rust,ignore
use paladin::core::platform::container::battalion::*;
use paladin::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm_adapter = Arc::new(OpenAIAdapter::new().build()?);

    // Commander - Breaks down project into tasks
    let commander = PaladinBuilder::new(llm_adapter.clone())
        .name("ProjectManager")
        .system_prompt("You are a project manager. Break down projects into specific, \
                        actionable tasks. For each task, specify what needs to be done. \
                        Output format: TASK: <description> for each task.")
        .temperature(0.6)
        .build()?;

    // Subordinates - Specialized for different task types
    let developer = PaladinBuilder::new(llm_adapter.clone())
        .name("Developer")
        .system_prompt("You are a senior developer. Implement the given technical task. \
                        Provide code and implementation details.")
        .build()?;

    let designer = PaladinBuilder::new(llm_adapter.clone())
        .name("Designer")
        .system_prompt("You are a UX/UI designer. Design solutions for the given task. \
                        Provide wireframes and design specifications.")
        .build()?;

    let tester = PaladinBuilder::new(llm_adapter)
        .name("Tester")
        .system_prompt("You are a QA engineer. Create test plans for the given task. \
                        Provide test cases and acceptance criteria.")
        .build()?;

    // Create Chain of Command
    let chain = ChainOfCommand::new()
        .commander(commander)
        .add_subordinate("developer", developer)
        .add_subordinate("designer", designer)
        .add_subordinate("tester", tester)
        // Route tasks based on keywords
        .routing_strategy(RoutingStrategy::KeywordBased(HashMap::from([
            ("code", "developer"),
            ("implement", "developer"),
            ("design", "designer"),
            ("UI", "designer"),
            ("test", "tester"),
            ("QA", "tester"),
        ])))
        .build()?;

    let result = chain.execute("Build a user login system with password reset").await?;

    // Commander breaks it down into tasks:
    // - TASK: Design login UI
    // - TASK: Implement authentication code
    // - TASK: Create password reset flow
    // - TASK: Test security and usability
    //
    // Each task is routed to appropriate subordinate

    println!("{}", result.aggregated_output);

    Ok(())
}
```

### Hierarchy Visualization

```
            ┌─────────────────────┐
            │     Commander       │
            │  (Project Manager)  │
            └─────────────────────┘
                       ↓
        ┌──────────────┴───────────────┐
        ↓              ↓                ↓
┌─────────────┐ ┌──────────────┐ ┌──────────────┐
│  Developer  │ │   Designer   │ │    Tester    │
└─────────────┘ └──────────────┘ └──────────────┘
```

### Routing Strategies

```rust,ignore
// 1. Keyword-based routing
.routing_strategy(RoutingStrategy::KeywordBased(keywords_map))

// 2. LLM-based routing (Commander decides)
.routing_strategy(RoutingStrategy::LlmDecision)

// 3. Round-robin
.routing_strategy(RoutingStrategy::RoundRobin)

// 4. Load-balanced
.routing_strategy(RoutingStrategy::LoadBalanced)

// 5. Custom routing
.routing_strategy(RoutingStrategy::Custom(Box::new(|task, subordinates| {
    // Your routing logic
    select_subordinate(task, subordinates)
})))
```

## Pattern Selection Guide

### Decision Matrix

| Factor | Formation | Phalanx | Campaign | Chain of Command |
|--------|-----------|---------|----------|------------------|
| **Sequential dependency** | ✅ High | ❌ Low | ✅ High | ⚠️ Medium |
| **Parallel execution** | ❌ No | ✅ Yes | ⚠️ Partial | ⚠️ Partial |
| **Complex workflow** | ❌ Low | ❌ Low | ✅ High | ⚠️ Medium |
| **Dynamic routing** | ❌ No | ❌ No | ⚠️ Limited | ✅ Yes |
| **Simplicity** | ✅ Simple | ⚠️ Medium | ❌ Complex | ⚠️ Medium |
| **Execution time** | Slow (sequential) | Fast (parallel) | Variable | Variable |
| **Use case** | Pipeline | Multi-view | Workflows | Task delegation |

### When to Use Each Pattern

**Formation** ✅
- Content generation pipeline (research → outline → write → edit)
- Data processing pipeline (extract → transform → load)
- Sequential analysis (collect → analyze → report)
- Any task with clear step-by-step flow

**Phalanx** ✅
- Code review from multiple perspectives
- Multi-language translation
- A/B testing content variations
- Brainstorming diverse ideas
- Parallel data processing

**Campaign** ✅
- Complex approval workflows
- State machines (order processing, incident management)
- Conditional pipelines (if-then-else logic)
- Multi-stage decision processes
- Workflows with feedback loops

**Chain of Command** ✅
- Project decomposition and execution
- Dynamic task assignment
- Hierarchical decision-making
- Supervised multi-agent systems
- Load distribution across specialized agents

## Common Pitfalls

### 1. Wrong Pattern Choice

❌ **Anti-pattern**: Using Formation for independent tasks
```rust,ignore
// Slow: Analyst must wait for researcher to finish
Formation::new()
    .add_paladin(researcher)
    .add_paladin(analyst)  // Could run in parallel!
```

✅ **Better**: Use Phalanx for parallel execution
```rust,ignore
Phalanx::new()
    .add_paladin(researcher)
    .add_paladin(analyst)  // Run simultaneously
```

### 2. Inefficient Aggregation

❌ **Anti-pattern**: Not using an aggregator in Phalanx
```rust,ignore
// Raw outputs are hard to process
let results = phalanx.execute_all(input).await?;
// Now you have to manually combine 5 different outputs
```

✅ **Better**: Define aggregator Paladin
```rust,ignore
let aggregator = PaladinBuilder::new(llm_adapter)
    .system_prompt("Combine reviews into single report...")
    .build()?;

phalanx.aggregator(aggregator)
```

### 3. Missing Error Handling

❌ **Anti-pattern**: Letting one failure stop everything
```rust,ignore
Formation::new()
    .stop_on_error(true)  // One error kills entire pipeline
```

✅ **Better**: Graceful degradation
```rust,ignore
Formation::new()
    .stop_on_error(false)
    .fallback_strategy(FallbackStrategy::UseLastValid)
```

### 4. Circular Dependencies in Campaign

❌ **Anti-pattern**: Creating cycles without limits
```rust,ignore
Campaign::new()
    .add_edge("A", "B")
    .add_edge("B", "A")  // Infinite loop!
```

✅ **Better**: Add cycle detection and limits
```rust,ignore
Campaign::new()
    .add_edge("A", "B")
    .add_conditional("B", "A", condition)
    .max_iterations(10)  // Safety limit
```

## Performance Considerations

### Formation Performance

```rust,ignore
// Sequential execution time: T1 + T2 + T3
// Use when output dependency is required
```

**Optimization tips**:
- Minimize Paladin count
- Use faster models for intermediate steps
- Enable checkpointing for recovery

### Phalanx Performance

```rust,ignore
// Parallel execution time: max(T1, T2, T3) + aggregation
// Best for reducing total execution time
```

**Optimization tips**:
- Set appropriate `max_concurrency` based on rate limits
- Use consistent temperature across Paladins for similar outputs
- Optimize aggregator prompt for efficiency

### Campaign Performance

```rust,ignore
// Variable: depends on graph structure and conditionals
// Can have exponential complexity if not careful
```

**Optimization tips**:
- Minimize graph depth
- Use early termination conditions
- Cache node results where possible
- Set strict `max_iterations` limits

### Chain of Command Performance

```rust,ignore
// Depends on routing efficiency and subordinate parallelization
```

**Optimization tips**:
- Efficient routing strategy
- Parallelize subordinate execution when possible
- Commander should be fast (lower temperature, simpler model)

## Monitoring and Debugging

### Enable Detailed Logging

```rust,ignore
env::set_var("RUST_LOG", "paladin::battalion=debug");

let formation = Formation::new()
    .verbose(true)  // Log each step
    .build()?;
```

### Track Execution Time

```rust,ignore
use std::time::Instant;

let start = Instant::now();
let result = battalion.execute(input).await?;
println!("Execution time: {:?}", start.elapsed());
```

### Checkpoint Recovery

```rust,ignore
let campaign = Campaign::new()
    .enable_checkpoints(true)
    .checkpoint_path("./campaign_state")
    .build()?;

// If execution fails, recover from last checkpoint
if let Some(state) = campaign.load_checkpoint()? {
    campaign.resume_from(state).await?;
}
```

## Next Steps

- **[Tool Integration](../user-guides/tool-integration.md)** - Add Arsenal to Battalions
- **[Memory Management](../user-guides/memory-management.md)** - Use Garrison with Battalions
- **[Examples](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples)** - See Battalions in action
- **[Performance Tuning](../operations/performance-tuning.md)** - Optimize Battalion execution

## Examples

See working examples:
- `examples/formation_sequential.rs` - Sequential pipeline
- `examples/phalanx_parallel.rs` - Parallel execution
- `examples/campaign_workflow.rs` - DAG orchestration
- `examples/chain_of_command_delegation.rs` - Hierarchical delegation
- `examples/commander_auto.rs` - Automatic pattern selection
