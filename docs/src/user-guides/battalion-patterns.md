# Battalion Orchestration Patterns

The **Battalion** system in `crates/paladin-battalion/` coordinates multiple Paladin agents
through eight distinct execution patterns, plus the **Commander** strategy router that can
select a pattern automatically.

---

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [The Eight Patterns](#the-eight-patterns)
   - [Formation — Sequential](#formation--sequential)
   - [Phalanx — Concurrent](#phalanx--concurrent)
   - [Campaign — Graph/DAG](#campaign--graphdag)
   - [Chain of Command — Hierarchical](#chain-of-command--hierarchical)
   - [Conclave — Mixture of Experts](#conclave--mixture-of-experts)
   - [Council — Collaborative Discussion](#council--collaborative-discussion)
   - [Grove — Semantic Routing](#grove--semantic-routing)
   - [Maneuver — Flow DSL](#maneuver--flow-dsl)
4. [Commander — Strategy Router](#commander--strategy-router)
5. [Error Handling](#error-handling)
6. [Performance Notes](#performance-notes)
7. [Best Practices](#best-practices)

---

## Overview

| Pattern | Module | Execution | Best For |
|---------|--------|-----------|----------|
| **Formation** | `formation_service` | Sequential (N→N+1) | Multi-step pipelines |
| **Phalanx** | `phalanx_service` | Concurrent | Parallel analysis |
| **Campaign** | `campaign_service` | DAG / topological | Branching workflows |
| **Chain of Command** | `chain_of_command_service` | Hierarchical delegation | Task routing |
| **Conclave** | `conclave_execution_service` | Parallel experts + aggregator | Expert synthesis |
| **Council** | `council_service` | Turn-taking dialogue | Collaborative consensus |
| **Grove** | `grove_service` | Semantic routing | Specialist selection |
| **Maneuver** | `maneuver` | Flow DSL declarative | Dynamic mixed patterns |

All services require only `Arc<dyn PaladinPort>` (from `paladin-ports`) — they never import
LLM provider libraries directly.

---

## Quick Start

```toml
[dependencies]
paladin-ai = { version = "0.10.0", features = ["llm-openai"] }
tokio = { version = "1", features = ["full"] }
```

```rust,ignore
use paladin_battalion::formation_service::FormationExecutionService;
use paladin_core::platform::container::battalion::formation::Formation;
use paladin_core::platform::container::battalion::BattalionConfig;
use std::sync::Arc;

// Each Paladin is built with PaladinBuilder (see paladin-agents.md)
let paladins = vec![analyzer, processor, summarizer];
let config = BattalionConfig::default();
let formation = Formation::new(paladins, config)?;

let service = FormationExecutionService::new(paladin_port);
let result = service.execute(&formation, "Analyze the Q3 earnings report").await?;
println!("{}", result.output);
```

---

## The Eight Patterns

### Formation — Sequential

**Source**: `crates/paladin-battalion/src/formation_service.rs`

Output from each Paladin feeds the input of the next. Ideal for multi-step data
transformation pipelines.

```rust,ignore
use paladin_battalion::formation_service::FormationExecutionService;
use paladin_core::platform::container::battalion::formation::Formation;

let formation = Formation::new(vec![extractor, analyzer, writer], config)?;
let service = FormationExecutionService::new(paladin_port);
let result = service.execute(&formation, "Raw data...").await?;
```

Configuration keys: sequential timeout, error strategy.

---

### Phalanx — Concurrent

**Source**: `crates/paladin-battalion/src/phalanx_service.rs`

All Paladins receive the same input and execute concurrently via `tokio` tasks.
Results are aggregated according to the `AggregationStrategy`.

```rust,ignore
use paladin_battalion::phalanx_service::PhalanxExecutionService;
use paladin_core::platform::container::battalion::phalanx::{AggregationStrategy, Phalanx};

let phalanx = Phalanx::new(
    vec![security_auditor, performance_analyst, style_checker],
    AggregationStrategy::Concatenate,
    config,
)?;

let service = PhalanxExecutionService::new(paladin_port);
let result = service.execute(&phalanx, "Review this Rust code...").await?;
```

`AggregationStrategy` variants: `Concatenate`, `FirstSuccess`, `Majority`, `Custom`.

Concurrency is bounded by a `tokio::sync::Semaphore` (configurable via `max_concurrency` in
`BattalionConfig`).

---

### Campaign — Graph/DAG

**Source**: `crates/paladin-battalion/src/campaign_service.rs`

Paladins are arranged in a directed acyclic graph. Execution is topologically sorted so
upstream agents complete before downstream agents begin.

```rust,ignore
use paladin_battalion::campaign_service::CampaignExecutionService;
use paladin_core::platform::container::battalion::campaign::Campaign;

let campaign = Campaign::builder()
    .add_node("ingest", ingest_paladin)
    .add_node("analyze", analyze_paladin)
    .add_node("report", report_paladin)
    .add_edge("ingest", "analyze")
    .add_edge("analyze", "report")
    .config(config)
    .build()?;

let service = CampaignExecutionService::new(paladin_port);
let result = service.execute(&campaign, "Start").await?;
```

Independent branches execute concurrently; the service enforces dependency order.

---

### Chain of Command — Hierarchical

**Source**: `crates/paladin-battalion/src/chain_of_command_service.rs`

A commander Paladin decomposes the task and routes sub-tasks to specialist Paladins, then
synthesizes their outputs.

```rust,ignore
use paladin_battalion::chain_of_command_service::ChainOfCommandExecutionService;
use paladin_core::platform::container::battalion::chain_of_command::ChainOfCommand;

let chain = ChainOfCommand::new(
    commander_paladin,
    vec![backend_dev, frontend_dev, qa_engineer],
    config,
)?;

let service = ChainOfCommandExecutionService::new(paladin_port);
let result = service.execute(&chain, "Build a login feature").await?;
```

---

### Conclave — Mixture of Experts

**Source**: `crates/paladin-battalion/src/conclave_execution_service.rs`

Multiple expert Paladins process the same task in parallel; an aggregator Paladin synthesizes
their outputs into a final response.

```rust,ignore
use paladin_battalion::conclave_execution_service::ConclaveExecutionService;
use paladin_core::platform::container::battalion::conclave::Conclave;

let conclave = Conclave::new(
    vec![legal_expert, technical_expert, business_expert],
    synthesis_paladin,
    config,
)?;

let service = ConclaveExecutionService::new(paladin_port);
let result = service.execute(&conclave, "Should we adopt microservices?").await?;
// result.aggregated_output contains the synthesized response
// result.successful_expert_count() shows how many experts contributed
```

---

### Council — Collaborative Discussion

**Source**: `crates/paladin-battalion/src/council_service.rs`

Paladins take turns responding to each other in a structured discussion, building toward a
shared conclusion or consensus.

```rust,ignore
use paladin_battalion::council_service::CouncilService;
use paladin_core::platform::container::battalion::council::Council;

let council = Council::new(
    vec![optimist_paladin, skeptic_paladin, moderator_paladin],
    config,  // includes discussion_rounds
)?;

let service = CouncilService::new(paladin_port);
let result = service.execute(&council, "Evaluate adopting async Rust").await?;
```

---

### Grove — Semantic Routing

**Source**: `crates/paladin-battalion/src/grove_service.rs`

The Grove routes the input to the most semantically appropriate Paladin from the registered
specialists, using LLM-based capability matching.

```rust,ignore
use paladin_battalion::grove_service::GroveExecutionService;
use paladin_core::platform::container::battalion::grove::Grove;

let grove = Grove::new(
    vec![python_expert, rust_expert, go_expert],
    config,
)?;

let service = GroveExecutionService::new(paladin_port);
let result = service.execute(&grove, "Help me with Rust lifetimes").await?;
// Routes to rust_expert automatically
```

---

### Maneuver — Flow DSL

**Source**: `crates/paladin-battalion/src/maneuver/`

Maneuver is a declarative flow DSL that lets you compose multiple Battalion patterns in a
single workflow definition. See [Maneuver Flow DSL](maneuver-flow-dsl.md) for full syntax
and examples.

---

## Commander — Strategy Router

**Source**: `crates/paladin-battalion/src/commander.rs`

The Commander provides a single entry-point that automatically selects the optimal pattern
based on input analysis and the number/capabilities of Paladins provided. Build it through
`CommanderBuilder` (the same builder [Orchestration](orchestration.md) uses) and run it with
the live single-argument `execute` method — the direct `Commander::new` constructor takes
five positional arguments (strategy, paladins, config, aggregator, paladin_port), and
`execute` takes only the input string, never a separate strategy/config pair per call:

```rust,ignore
{{#include ../../../crates/doc-examples/src/battalion_patterns.rs:commander}}
```

Force a specific strategy by passing a different `BattalionStrategy` variant to `.strategy()`
on the builder instead of `BattalionStrategy::Auto`.

### Auto Mode Heuristics

| Priority | Pattern | Triggers |
|----------|---------|---------|
| 1 | Conclave | ≥3 paladins + keywords: synthesize, compare, perspectives |
| 2 | Council | ≥2 paladins + keywords: discuss, debate, consensus, brainstorm |
| 3 | Grove | ≥2 paladins + keywords: route, expertise, most qualified |
| 4 | Formation | 1–3 paladins by default / keywords: sequential, pipeline |
| 5 | Phalanx | Multiple paladins for parallel analysis |
| 6 | Campaign | Complex multi-step with branching |

---

## Error Handling

All services use `ErrorStrategy` from `paladin_core::platform::container::battalion`:

| Strategy | Behaviour |
|---------|-----------|
| `FailFast` | First failure aborts the entire Battalion (default) |
| `ContinueOnError` | Failed agents are skipped; others continue |
| `RetryThenContinue` | Retry failed agents up to N times, then continue |

```rust,ignore
use paladin_core::platform::container::battalion::{BattalionConfig, ErrorStrategy};

let config = BattalionConfig {
    error_strategy: ErrorStrategy::ContinueOnError,
    max_concurrency: Some(4),
    timeout_seconds: 120,
    ..Default::default()
};
```

`BattalionResult` fields: `final_output: String`, `paladin_results: Vec<PaladinResult>` (each
entry carries its own `usage: TokenUsage`, D-07), `status: BattalionStatus`,
`per_paladin_tokens: HashMap<String, TokenUsage>` (the per-Paladin split), and
`total_tokens: u64` (the derived aggregate, D-08). There is no `execution_time_ms` or
`token_usage` field on `BattalionResult` itself.

---

## Performance Notes

- **Phalanx concurrency** is capped by `BattalionConfig::max_concurrency` (default: unbounded).
  Set this to avoid overloading upstream LLM rate limits.
- **Formation** adds one LLM call per Paladin sequentially — keep chains short (<6) for
  latency-sensitive workloads.
- **Campaign** parallelises independent branches automatically; no manual coordination needed.
- The **Commander auto-router** adds a small analysis overhead (~50ms); negligible for most tasks.

---

## Best Practices

- **Formation**: Keep the chain ≤5 agents and design each stage to produce clean hand-off text.
- **Phalanx**: Set `max_concurrency` to stay within LLM provider rate limits.
- **Campaign**: Validate your DAG has no cycles before deployment (`Campaign::build()` checks this).
- **Conclave**: Ensure expert agents have distinct, non-overlapping system prompts for better synthesis.
- **Council**: Include a moderator Paladin to keep discussions on track.
- **Grove**: Write precise capability descriptions in each Paladin's `agent_description` field.
- **Commander Auto**: Test your routing decisions with representative inputs before production.
