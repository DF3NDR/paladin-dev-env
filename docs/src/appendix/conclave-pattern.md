# Conclave Pattern Guide

Multi-expert synthesis orchestration implementing the Mixture-of-Agents approach. Multiple specialized Paladins analyze a task in parallel, then an aggregator synthesizes their diverse perspectives into a comprehensive response.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Programmatic API](#programmatic-api)
- [YAML Configuration](#yaml-configuration)
- [CLI Usage](#cli-usage)
- [Use Cases](#use-cases)
- [Error Handling](#error-handling)
- [Observability](#observability)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

The Conclave pattern solves problems requiring **multiple expert perspectives** that must be **intelligently synthesized**. Unlike simple parallel execution (Phalanx), Conclave specifically focuses on combining diverse viewpoints through an aggregator agent.

### When to Use Conclave

**✅ Use Conclave When:**
- Decisions benefit from multiple perspectives (technical, business, security, etc.)
- You need diverse expert opinions synthesized into actionable recommendations
- Different stakeholders have unique concerns that must all be addressed
- Quality improves through deliberate multi-perspective analysis

**❌ Don't Use Conclave When:**
- Single perspective is sufficient
- All agents would provide identical analysis
- Simple parallel processing without synthesis is adequate (use Phalanx instead)
- Real-time response is critical (Conclave adds synthesis overhead)

### Architecture

```text
                    ┌──────────────┐
                    │   Input      │
                    │   Query      │
                    └──────┬───────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
  │  Expert 1   │   │  Expert 2   │   │  Expert 3   │
  │ (Technical) │   │ (Business)  │   │ (Security)  │
  └──────┬──────┘   └──────┬──────┘   └──────┬──────┘
         │                 │                 │
         └─────────────────┼─────────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │ Aggregator  │
                    │  Synthesis  │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │   Final     │
                    │  Response   │
                    └─────────────┘
```

### Key Benefits

1. **Higher Quality Outputs**: Multiple perspectives catch blind spots
2. **Comprehensive Analysis**: Technical, business, security, etc. all considered
3. **Balanced Decisions**: Aggregator weighs competing priorities
4. **Resilience**: Continues even if some experts fail
5. **Traceable Reasoning**: See each expert's input to final decision

## Quick Start

### Minimal Example

```rust
use paladin::prelude::*;
use paladin::battalion::conclave::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm_adapter = Arc::new(OpenAIAdapter::new().build()?);

    // Create 3 experts with different perspectives
    let technical = create_paladin(llm_adapter.clone(),
        "TechnicalExpert",
        "You are a technical architect. Analyze from a technical perspective."
    )?;

    let business = create_paladin(llm_adapter.clone(),
        "BusinessExpert",
        "You are a business strategist. Analyze from a business perspective."
    )?;

    let security = create_paladin(llm_adapter.clone(),
        "SecurityExpert",
        "You are a security expert. Analyze from a security perspective."
    )?;

    // Create aggregator to synthesize expert outputs
    let aggregator = create_paladin(llm_adapter.clone(),
        "Aggregator",
        "Synthesize the expert analyses into a comprehensive recommendation."
    )?;

    // Configure Conclave
    let config = ConclaveConfig::new("expert-panel", BattalionConfig::default())
        .with_timeout(300)
        .with_retry_attempts(2);

    // Build Conclave
    let conclave = Conclave::new(
        vec![technical, business, security],
        aggregator,
        config
    )?;

    // Execute
    let service = ConclaveExecutionService::new(paladin_port);
    let result = service.execute(&conclave,
        "Should we migrate to microservices?"
    ).await?;

    println!("Final Recommendation:\n{}", result.aggregated_output.output);
    Ok(())
}

fn create_paladin(
    llm: Arc<dyn LlmPort>,
    name: &str,
    prompt: &str
) -> Result<Paladin, Box<dyn std::error::Error>> {
    PaladinBuilder::new(llm)
        .name(name)
        .system_prompt(prompt)
        .temperature(0.7)
        .build()
}
```

## Configuration

### ConclaveConfig Options

```rust
pub struct ConclaveConfig {
    /// Conclave name (required)
    name: String,

    /// Battalion base configuration
    battalion_config: BattalionConfig,

    /// Maximum execution time (seconds)
    timeout_seconds: u64,

    /// Retry attempts for failed experts (default: 2)
    max_retry_attempts: u32,

    /// Custom synthesis prompt (optional)
    synthesis_prompt: Option<String>,

    /// Include expert names in aggregator input (default: true)
    include_expert_names: bool,

    /// Max tokens per expert before truncation (optional)
    max_expert_tokens: Option<usize>,

    /// Observability level (default: Standard)
    observability: ObservabilityLevel,
}
```

### Builder Pattern

```rust
let config = ConclaveConfig::new("my-conclave", battalion_config)
    .with_timeout(600)                    // 10 minutes
    .with_retry_attempts(3)               // Retry up to 3 times
    .with_observability(ObservabilityLevel::Verbose)
    .with_expert_names(true)              // Show expert attribution
    .with_max_expert_tokens(2000)         // Truncate long outputs
    .with_synthesis_prompt(               // Override aggregator prompt
        "Focus only on technical feasibility. YES/NO answer required."
    );
```

### Retry Configuration

Conclave uses **exponential backoff with jitter** for retries:

```
Attempt 1: 1 second  ± 20% jitter
Attempt 2: 2 seconds ± 20% jitter
Attempt 3: 4 seconds ± 20% jitter
Attempt 4: 8 seconds ± 20% jitter
Attempt 5: 16 seconds ± 20% jitter
```

Example configuration:

```rust
let config = ConclaveConfig::new("resilient", battalion_config)
    .with_retry_attempts(3)  // Total 4 attempts (1 initial + 3 retries)
    .with_timeout(300);      // Overall timeout for all attempts
```

### Observability Levels

```rust
pub enum ObservabilityLevel {
    Minimal,   // Errors and final result only
    Standard,  // Progress updates + timing (default)
    Verbose,   // Detailed logs, individual outputs, retries
}
```

**Minimal**: Production systems with log aggregation
```rust
.with_observability(ObservabilityLevel::Minimal)
```

**Standard**: Development and staging (recommended)
```rust
.with_observability(ObservabilityLevel::Standard)
```

**Verbose**: Debugging and troubleshooting
```rust
.with_observability(ObservabilityLevel::Verbose)
```

## Programmatic API

### Expert Creation

Create diverse experts with specialized roles:

```rust
// Technical Expert - Focus on implementation details
let technical_expert = PaladinBuilder::new(llm_port.clone())
    .name("TechnicalArchitect")
    .system_prompt(
        "You are a senior technical architect with 15+ years experience \
         in distributed systems. Analyze the proposal focusing on:\n\
         - System architecture and design patterns\n\
         - Scalability and performance\n\
         - Technology stack recommendations\n\
         - Implementation risks and complexity"
    )
    .temperature(0.7)
    .max_loops(3)
    .build()?;

// Business Expert - Focus on ROI and strategy
let business_expert = PaladinBuilder::new(llm_port.clone())
    .name("BusinessStrategist")
    .system_prompt(
        "You are a business strategist and product manager. Analyze focusing on:\n\
         - Market opportunity and competitive positioning\n\
         - Cost-benefit analysis and ROI projections\n\
         - Resource requirements (team, budget, timeline)\n\
         - Stakeholder impact across departments"
    )
    .temperature(0.7)
    .max_loops(3)
    .build()?;

// Security Expert - Focus on risks and compliance
let security_expert = PaladinBuilder::new(llm_port.clone())
    .name("SecurityExpert")
    .system_prompt(
        "You are a security expert specializing in application security. Analyze focusing on:\n\
         - Threat modeling and attack surface\n\
         - Required security controls (auth, encryption, etc.)\n\
         - Compliance requirements (GDPR, SOC 2, HIPAA)\n\
         - Security testing requirements"
    )
    .temperature(0.7)
    .max_loops(3)
    .build()?;
```

### Aggregator Creation

The aggregator synthesizes expert outputs:

```rust
let aggregator = PaladinBuilder::new(llm_port.clone())
    .name("SynthesisAggregator")
    .system_prompt(
        "You are a synthesis expert combining multiple perspectives. \
         You receive technical, business, and security analyses. \
         Your synthesis should:\n\
         1. Create an executive summary with clear recommendation\n\
         2. Identify common themes across experts\n\
         3. Highlight unique insights from each perspective\n\
         4. Resolve contradictions by weighing evidence\n\
         5. Provide prioritized action items\n\
         6. Outline critical success factors and risks\n\n\
         Structure with clear sections. Integrate thoughtfully, don't just concatenate."
    )
    .temperature(0.5)  // Lower temperature for consistent synthesis
    .max_loops(2)
    .build()?;
```

### Building and Executing

```rust
// Create Conclave
let experts = vec![technical_expert, business_expert, security_expert];

let config = ConclaveConfig::new("expert-panel", BattalionConfig::default())
    .with_timeout(300)
    .with_retry_attempts(2)
    .with_observability(ObservabilityLevel::Standard);

let conclave = Conclave::new(experts, aggregator, config)?;

// Execute
let service = ConclaveExecutionService::new(paladin_port);
let result = service.execute(&conclave,
    "Should we implement real-time WebSocket notifications?"
).await?;

// Access results
println!("Status: {:?}", result.status);
println!("Execution time: {}ms", result.execution_time_ms);
println!("Expert success rate: {}/{}",
    result.successful_expert_count(),
    conclave.expert_count()
);

// Individual expert outputs
for (name, output) in result.expert_outputs.iter() {
    println!("\n{}: {}", name, output.output);
}

// Final synthesized output
println!("\nFinal Recommendation:\n{}", result.aggregated_output.output);
```

### Error Handling with Partial Success

```rust
match service.execute(&conclave, input).await {
    Ok(result) => {
        if result.successful_expert_count() < conclave.expert_count() {
            eprintln!("Warning: {} experts failed",
                conclave.expert_count() - result.successful_expert_count());
        }

        // Check aggregation success
        if result.status == ConclaveStatus::Completed {
            println!("Success: {}", result.aggregated_output.output);
        } else {
            eprintln!("Aggregation failed but partial results available");
            for (name, output) in result.expert_outputs.iter() {
                println!("{}: {}", name, output.output);
            }
        }
    }
    Err(ConclaveError::AllExpertsFailed) => {
        eprintln!("Critical: All experts failed");
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## YAML Configuration

### Basic YAML Structure

Create `conclave.yaml`:

```yaml
type: conclave
name: "expert-panel"

experts:
  - inline:
      name: "TechnicalExpert"
      system_prompt: |
        You are a technical architect...
      model: "gpt-4o"
      temperature: 0.7
      max_loops: 3
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai

  - inline:
      name: "BusinessExpert"
      system_prompt: |
        You are a business strategist...
      model: "gpt-4o"
      temperature: 0.7
      max_loops: 3
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai

aggregator:
  inline:
    name: "Aggregator"
    system_prompt: |
      Synthesize expert analyses...
    model: "gpt-4o"
    temperature: 0.5
    max_loops: 2
    timeout_seconds: 300
    stop_words: []
    provider:
      type: openai

timeout_seconds: 300
retry_attempts: 2
include_expert_names: true
observability_level: "standard"
```

### External Paladin References

Reference pre-defined Paladin configs:

```yaml
type: conclave
name: "expert-panel"

experts:
  - file: "configs/technical_expert.yaml"
  - file: "configs/business_expert.yaml"
  - file: "configs/security_expert.yaml"

aggregator:
  file: "configs/synthesis_aggregator.yaml"

timeout_seconds: 300
retry_attempts: 2
```

### Advanced Options

```yaml
type: conclave
name: "custom-conclave"

experts:
  - inline:
      # ... expert configs ...

aggregator:
  inline:
    # ... aggregator config ...

# Custom synthesis prompt (overrides aggregator's system_prompt)
synthesis_prompt: |
  Focus ONLY on technical feasibility.
  Provide YES/NO recommendation with brief justification.
  Ignore business and security concerns for this analysis.

# Include expert names in aggregator input
include_expert_names: true

# Truncate expert outputs to 2000 tokens before aggregation
max_expert_output_tokens: 2000

# Verbose logging for debugging
observability_level: "verbose"

# Aggressive retry policy
timeout_seconds: 600
retry_attempts: 3
```

## CLI Usage

### Generate Template

Create a new Conclave configuration:

```bash
paladin battalion new my-experts --type conclave --output conclave.yaml
```

This generates a template with 3 experts (Technical, Business, Security) and an aggregator with helpful comments.

### Run Conclave

Execute a Conclave configuration:

```bash
paladin battalion run --config conclave.yaml --type conclave
```

You'll be prompted for input:

```
? Enter task for expert analysis: Should we migrate to microservices?
```

### Output to JSON

Save structured output:

```bash
paladin battalion run -c conclave.yaml -t conclave -o result.json
```

### Verbose Mode

See detailed execution logs:

```bash
paladin battalion run -c conclave.yaml -t conclave --verbose
```

Output includes:
- Expert execution progress
- Individual expert outputs (truncated)
- Execution timing
- Success/failure rates
- Final aggregated output

## Use Cases

### 1. Technical Decision Making

**Scenario**: Evaluate architectural changes

**Experts**:
- Technical Architect (implementation feasibility)
- DevOps Engineer (operational impact)
- Security Engineer (security implications)

**Input**: "Should we adopt Kubernetes for our infrastructure?"

**Value**: Comprehensive evaluation covering development, operations, and security perspectives.

### 2. Product Feature Evaluation

**Scenario**: Prioritize product features

**Experts**:
- Product Manager (market fit, user value)
- Engineering Lead (implementation complexity)
- Data Scientist (data requirements, ML feasibility)

**Input**: "Should we build an in-house recommendation engine?"

**Value**: Balanced view of business value vs. technical effort.

### 3. Code Review

**Scenario**: Comprehensive code quality analysis

**Experts**:
- Security Reviewer (vulnerability detection)
- Performance Reviewer (optimization opportunities)
- Maintainability Reviewer (code quality, patterns)

**Input**: Code snippet or PR description

**Value**: Multi-dimensional review catching issues from different angles.

### 4. Compliance Assessment

**Scenario**: Evaluate regulatory compliance

**Experts**:
- GDPR Expert (data protection requirements)
- SOC 2 Expert (security controls)
- Industry Expert (sector-specific regulations)

**Input**: "Assess compliance requirements for storing health data"

**Value**: Comprehensive compliance coverage across multiple frameworks.

### 5. Strategic Planning

**Scenario**: Long-term strategic decisions

**Experts**:
- Market Analyst (competitive landscape, trends)
- Financial Advisor (budget, ROI projections)
- Risk Manager (strategic risks, mitigation)

**Input**: "Should we expand to European markets in 2025?"

**Value**: Well-rounded strategic recommendation considering multiple stakeholder concerns.

## Error Handling

### Partial Success Scenarios

Conclave continues even if some experts fail:

```rust
let result = service.execute(&conclave, input).await?;

// Check success rate
let success_rate = result.successful_expert_count() as f64 /
                  conclave.expert_count() as f64;

if success_rate < 0.5 {
    eprintln!("Warning: Less than 50% experts succeeded");
}

// Aggregation proceeds with available expert outputs
if result.status == ConclaveStatus::PartialSuccess {
    println!("Aggregation completed with partial expert data");
}
```

### Retry Behavior

Failed experts are automatically retried:

```rust
let config = ConclaveConfig::new("resilient", battalion_config)
    .with_retry_attempts(3)  // Retry up to 3 times
    .with_timeout(300);      // Overall timeout includes retries
```

**Retry triggers**:
- Network timeouts
- API rate limits (429 errors)
- Temporary service unavailability (503 errors)

**No retry for**:
- Authentication failures (401, 403)
- Invalid requests (400)
- Not found (404)
- Exceeded overall timeout

### Error Recovery

```rust
match service.execute(&conclave, input).await {
    Ok(result) => {
        match result.status {
            ConclaveStatus::Completed => {
                // All experts succeeded, aggregation successful
                println!("Success: {}", result.aggregated_output.output);
            }
            ConclaveStatus::PartialSuccess => {
                // Some experts failed, but aggregation succeeded
                println!("Partial success: {}", result.aggregated_output.output);
                log::warn!("Failed experts: {}",
                    conclave.expert_count() - result.successful_expert_count());
            }
            ConclaveStatus::Failed => {
                // Aggregation failed
                log::error!("Aggregation failed");
                // Access individual expert outputs if available
                for (name, output) in result.expert_outputs.iter() {
                    println!("{}: {}", name, output.output);
                }
            }
        }
    }
    Err(ConclaveError::AllExpertsFailed) => {
        log::error!("All experts failed - cannot proceed with aggregation");
    }
    Err(ConclaveError::Timeout(secs)) => {
        log::error!("Execution exceeded {} second timeout", secs);
    }
    Err(e) => {
        log::error!("Unexpected error: {}", e);
    }
}
```

## Observability

### Logging Levels

Configure observability to match your environment:

**Minimal** (Production):
```rust
.with_observability(ObservabilityLevel::Minimal)
```

Logs only:
- Critical errors
- Final execution status
- Total execution time

**Standard** (Staging/Development):
```rust
.with_observability(ObservabilityLevel::Standard)
```

Logs:
- Expert execution start/completion
- Retry attempts
- Partial failure warnings
- Aggregation timing
- Success/failure counts

**Verbose** (Debugging):
```rust
.with_observability(ObservabilityLevel::Verbose)
```

Logs:
- All Standard logs PLUS:
- Individual expert outputs (truncated)
- Detailed retry information
- Token counts per expert
- Timing breakdown by phase

### Execution Metrics

Access detailed metrics from results. Each expert's and the aggregator's `usage` field is a
full `TokenUsage` (prompt/completion split, plus cache/reasoning sub-counts when the provider
reports them) -- `.total_tokens` below is its derived sum:

```rust
let result = service.execute(&conclave, input).await?;

// Overall metrics
println!("Total time: {}ms", result.execution_time_ms);
println!("Status: {:?}", result.status);

// Expert-level metrics
for (name, expert_result) in result.expert_outputs.iter() {
    println!("{}: {}ms, {} tokens, {} loops",
        name,
        expert_result.execution_time_ms,
        expert_result.usage.total_tokens,
        expert_result.loop_count
    );
}

// Aggregation metrics
println!("Aggregator: {}ms, {} tokens",
    result.aggregated_output.execution_time_ms,
    result.aggregated_output.usage.total_tokens
);

// Success rate
println!("Success rate: {}/{}",
    result.successful_expert_count(),
    conclave.expert_count()
);
```

### Structured Logging

Integrate with structured logging frameworks:

```rust
use log::{info, warn, error};

let result = service.execute(&conclave, input).await?;

info!(
    "Conclave execution completed";
    "conclave_name" => &conclave.name(),
    "status" => format!("{:?}", result.status),
    "execution_ms" => result.execution_time_ms,
    "expert_count" => conclave.expert_count(),
    "successful_experts" => result.successful_expert_count(),
);

if result.successful_expert_count() < conclave.expert_count() {
    warn!(
        "Partial expert failure";
        "failed_count" => conclave.expert_count() - result.successful_expert_count(),
    );
}
```

## Best Practices

### Expert Configuration

**1. Recommended Number of Experts: 3-5**

- **Minimum 2**: Required for diversity
- **Optimal 3-4**: Balanced quality vs. cost/latency
- **Maximum 5-6**: Diminishing returns beyond this

**2. Ensure Expert Diversity**

❌ Don't create redundant experts:
```rust
let expert1 = create_expert("Expert1", "You are a technical expert");
let expert2 = create_expert("Expert2", "You are a technical expert");
// Same perspective - wasteful!
```

✅ Create distinct perspectives:
```rust
let technical = create_expert("Technical", "Architecture and implementation");
let business = create_expert("Business", "ROI and strategy");
let security = create_expert("Security", "Risks and compliance");
// Different perspectives - valuable diversity
```

**3. Use Lower Temperature for Aggregator**

Experts can be creative (temperature 0.6-0.8), but aggregator should be consistent:

```rust
// Experts: Creative analysis
let expert = PaladinBuilder::new(llm)
    .temperature(0.7)
    .build()?;

// Aggregator: Consistent synthesis
let aggregator = PaladinBuilder::new(llm)
    .temperature(0.5)  // Lower for consistency
    .build()?;
```

### Prompt Engineering

**1. Structure Expert Prompts**

Use clear sections in system prompts:

```rust
let expert = create_expert(
    "TechnicalExpert",
    "You are a senior technical architect.\n\
     \n\
     Analyze the input focusing on:\n\
     - System architecture and design patterns\n\
     - Scalability and performance considerations\n\
     - Technology stack recommendations\n\
     - Implementation risks and complexity\n\
     \n\
     Provide specific technical details.\n\
     Cite proven patterns and best practices."
);
```

**2. Aggregator Synthesis Instructions**

Be explicit about synthesis requirements:

```rust
let aggregator = create_expert(
    "Aggregator",
    "Synthesize expert analyses following these steps:\n\
     1. Create executive summary with clear recommendation\n\
     2. Identify common themes across all experts\n\
     3. Highlight unique insights from each perspective\n\
     4. Resolve contradictions by weighing evidence\n\
     5. Provide prioritized action items\n\
     6. Outline critical success factors and risks\n\
     \n\
     DO NOT simply concatenate expert outputs.\n\
     Integrate thoughtfully into coherent narrative."
);
```

**3. Use synthesis_prompt for Task-Specific Focus**

Override aggregator behavior for specific tasks:

```rust
let config = ConclaveConfig::new("focused", battalion_config)
    .with_synthesis_prompt(
        "Focus ONLY on technical feasibility. \
         Ignore business and security concerns. \
         Provide YES/NO recommendation with 2-3 sentence justification."
    );
```

### Performance Optimization

**1. Set Appropriate Timeouts**

```rust
// Quick analysis
let config = ConclaveConfig::new("quick", battalion_config)
    .with_timeout(60);  // 1 minute

// Thorough analysis
let config = ConclaveConfig::new("thorough", battalion_config)
    .with_timeout(600);  // 10 minutes
```

**2. Truncate Verbose Expert Outputs**

Prevent token limit issues:

```rust
let config = ConclaveConfig::new("optimized", battalion_config)
    .with_max_expert_tokens(2000);  // Limit per expert
```

**3. Parallel Execution is Automatic**

Experts execute concurrently - no additional configuration needed.

### Cost Management

**1. Choose Appropriate Models**

```rust
// Experts: Use fast, cost-effective models
let expert = PaladinBuilder::new(llm)
    .model("gpt-4o-mini")  // Cheaper model
    .temperature(0.7)
    .build()?;

// Aggregator: Use more capable model for synthesis
let aggregator = PaladinBuilder::new(llm)
    .model("gpt-4o")  // Better model for complex synthesis
    .temperature(0.5)
    .build()?;
```

**2. Limit max_loops**

Prevent excessive LLM calls:

```rust
let expert = PaladinBuilder::new(llm)
    .max_loops(2)  // Reasonable limit
    .build()?;
```

**3. Monitor Token Usage**

```rust
let result = service.execute(&conclave, input).await?;

let total_tokens: usize = result.expert_outputs.values()
    .map(|r| r.usage.total_tokens as usize)
    .sum::<usize>() + result.aggregated_output.usage.total_tokens as usize;

println!("Total tokens used: {}", total_tokens);
```

## Troubleshooting

### Problem: All Experts Fail

**Symptoms**:
- Error: `ConclaveError::AllExpertsFailed`
- No expert outputs in result

**Possible Causes**:
1. API key issues
2. Network connectivity problems
3. Rate limiting
4. Invalid model names

**Solutions**:
```rust
// 1. Verify API keys
std::env::var("OPENAI_API_KEY").expect("API key not set");

// 2. Increase timeout
let config = ConclaveConfig::new("patient", battalion_config)
    .with_timeout(600);  // Longer timeout

// 3. Add more retry attempts
let config = ConclaveConfig::new("persistent", battalion_config)
    .with_retry_attempts(5);

// 4. Enable verbose logging
let config = ConclaveConfig::new("debug", battalion_config)
    .with_observability(ObservabilityLevel::Verbose);
```

### Problem: Aggregation Fails Despite Successful Experts

**Symptoms**:
- Expert outputs are present
- `result.status == ConclaveStatus::Failed`
- Aggregation error in logs

**Possible Causes**:
1. Aggregator timeout (processing combined expert outputs)
2. Token limit exceeded (too much expert output)
3. Aggregator model capacity issues

**Solutions**:
```rust
// 1. Increase aggregator-specific timeout
let aggregator = PaladinBuilder::new(llm)
    .timeout_seconds(600)  // Longer timeout for synthesis
    .build()?;

// 2. Truncate expert outputs
let config = ConclaveConfig::new("limited", battalion_config)
    .with_max_expert_tokens(1500);

// 3. Use more capable aggregator model
let aggregator = PaladinBuilder::new(llm)
    .model("gpt-4o")  // Upgrade from mini
    .build()?;
```

### Problem: Poor Quality Synthesis

**Symptoms**:
- Aggregator simply concatenates expert outputs
- Missing integration of perspectives
- No actionable recommendations

**Solutions**:
```rust
// 1. Improve aggregator prompt
let aggregator = create_expert(
    "Aggregator",
    "You are a synthesis expert. Your role is to INTEGRATE (not concatenate) \
     the expert analyses. Create a coherent narrative that:\n\
     - Identifies patterns and common themes\n\
     - Highlights contradictions and resolves them\n\
     - Provides clear, actionable recommendations\n\
     - Structures output with sections and bullet points"
);

// 2. Use synthesis_prompt for task-specific guidance
let config = ConclaveConfig::new("guided", battalion_config)
    .with_synthesis_prompt(
        "Combine expert analyses into a single recommendation. \
         Format as: Executive Summary, Key Findings, Recommendation, Next Steps."
    );

// 3. Lower aggregator temperature for consistency
let aggregator = PaladinBuilder::new(llm)
    .temperature(0.3)  // Very consistent
    .build()?;
```

### Problem: Slow Execution

**Symptoms**:
- Execution takes longer than expected
- Timeout errors

**Possible Causes**:
1. Sequential expert execution (shouldn't happen - experts are parallel)
2. Slow individual experts
3. Excessive retries

**Solutions**:
```rust
// 1. Verify parallel execution (automatic, but check logs)
let config = ConclaveConfig::new("fast", battalion_config)
    .with_observability(ObservabilityLevel::Verbose);

// 2. Reduce expert max_loops
let expert = PaladinBuilder::new(llm)
    .max_loops(1)  // Single pass
    .build()?;

// 3. Limit retry attempts
let config = ConclaveConfig::new("quick", battalion_config)
    .with_retry_attempts(1);  // One retry only

// 4. Use faster models
let expert = PaladinBuilder::new(llm)
    .model("gpt-4o-mini")
    .build()?;
```

### Problem: Inconsistent Expert Names in Output

**Symptoms**:
- Expert outputs lack attribution
- Can't tell which expert said what

**Solution**:
```rust
let config = ConclaveConfig::new("attributed", battalion_config)
    .with_expert_names(true);  // Ensure this is set
```

## See Also

- [Battalion Patterns Guide](battalion-patterns-guide.md) - Other orchestration patterns
- [Paladin Configuration](../user-guides/paladin-configuration.md) - Expert setup
- [Examples](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples) - Complete working examples
- [CLI Configs](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples) - YAML templates
