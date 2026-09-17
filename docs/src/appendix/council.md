# Council Pattern

**Multi-agent deliberation framework for collaborative decision-making**

---

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Turn-Taking Strategies](#turn-taking-strategies)
4. [Termination Conditions](#termination-conditions)
5. [Garrison Integration](#garrison-integration)
6. [Configuration](#configuration)
7. [Examples](#examples)
8. [Best Practices](#best-practices)
9. [API Reference](#api-reference)

---

## Overview

The Council pattern enables multiple Paladin agents to engage in structured deliberation and collaborative decision-making. Unlike parallel execution (Phalanx) or sequential processing (Formation), Council creates a **conversational dynamic** where agents take turns, build on each other's contributions, and work toward consensus or comprehensive analysis.

### Key Concepts

**Council**: A group of Paladin agents (participants) engaging in structured discussion around a topic.

**Moderator**: Optional specialized agent controlling discussion flow and termination decisions.

**Turn-Taking**: Strategy determining which participant speaks next (RoundRobin, ModeratorDirected).

**Termination Condition**: Rule determining when deliberation concludes (MaxRounds, Consensus, ModeratorDecision, Keyword).

**Conversation History**: Accumulated context allowing agents to reference and build on previous contributions.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      Council                             │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  Topic: "Should we implement feature X?"                 │
│                                                          │
│  Round 1:                                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ TechnicalExp │→ │ BusinessExp  │→ │ SecurityExp  │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│                                                          │
│  Round 2:                                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ TechnicalExp │→ │ BusinessExp  │→ │ SecurityExp  │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│                                                          │
│  [Continues until termination condition met]            │
│                                                          │
│  Final Output: Synthesized recommendations              │
└─────────────────────────────────────────────────────────┘
```

### When to Use Council

✅ **Ideal Use Cases**:
- **Expert panel discussions**: Gather diverse perspectives on complex decisions
- **Consensus building**: Work toward agreement among stakeholders
- **Comprehensive analysis**: Ensure all angles considered through dialogue
- **Deliberative decision-making**: Structured debate with turn-taking
- **Collaborative problem-solving**: Build on each other's ideas iteratively

❌ **Not Ideal For**:
- Simple sequential processing → Use **Formation**
- Independent parallel analysis → Use **Phalanx**
- Quick routing decisions → Use **Grove**
- Complex conditional workflows → Use **Campaign**

---

## Quick Start

### Basic Council Example

```rust,ignore
use paladin::core::platform::container::battalion::council::{
    CouncilBuilder, CouncilConfig, TurnStrategy, TerminationCondition
};
use paladin::application::services::battalion::council_service::CouncilExecutionService;
use paladin_battalion::in_memory_registry::HashMapPaladinRegistry;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create participants
    let technical_expert = create_paladin(
        "TechnicalExpert",
        "You are a technical expert focusing on implementation feasibility."
    );

    let business_expert = create_paladin(
        "BusinessExpert",
        "You are a business strategist focusing on ROI and market impact."
    );

    let security_expert = create_paladin(
        "SecurityExpert",
        "You are a security expert focusing on risks and compliance."
    );

    // CouncilBuilder::add_participant takes a `paladin_id` string, not a Paladin -- register
    // each Paladin under that same ID so the registry can resolve it at execution time.
    let mut paladins = HashMap::new();
    paladins.insert("technical_expert".to_string(), Arc::new(technical_expert));
    paladins.insert("business_expert".to_string(), Arc::new(business_expert));
    paladins.insert("security_expert".to_string(), Arc::new(security_expert));
    let registry = Arc::new(HashMapPaladinRegistry::from_map(paladins));

    // Build council
    let council = CouncilBuilder::new()
        .name("Expert Panel Council")
        .add_participant("technical_expert")
        .add_participant("business_expert")
        .add_participant("security_expert")
        .turn_strategy(TurnStrategy::RoundRobin)
        .max_rounds(3)
        .termination_condition(TerminationCondition::MaxRounds)
        .build()?;

    // Execute council discussion
    let service = CouncilExecutionService::new(
        Arc::new(paladin_port),
        Some(Arc::new(garrison_port)), // Optional: store conversation history
        registry,
    );

    let topic = "Should we implement two-factor authentication for all users?";
    let result = service.convene(&council, topic).await?;

    println!("Rounds completed: {}", result.rounds_completed);
    println!("Termination reason: {:?}", result.termination_reason);
    if let Some(conclusion) = &result.conclusion {
        println!("\nConclusion:\n{}", conclusion);
    }
    for message in &result.transcript {
        println!("{:?}", message);
    }

    Ok(())
}
```

### Output Example

```
Round 1:
--------
TechnicalExpert: Implementing 2FA is technically feasible. We can use TOTP
with existing libraries like `authenticator`. Main effort is UI/UX for enrollment
and recovery flows. Estimate: 2 sprint cycles.

BusinessExpert: From a business perspective, 2FA adds friction but increases trust.
Our enterprise customers require it per SOC 2 compliance. Churn risk for consumer
users is moderate, can be mitigated with optional rollout. ROI positive within 6 months.

SecurityExpert: 2FA significantly reduces account takeover risk (98% reduction per
Microsoft data). Essential for PII protection. Recommend mandatory for admin accounts,
optional for users. Need backup codes and recovery process for support.

Round 2:
--------
TechnicalExpert: Agreed on phased rollout. Suggest SMS fallback for users without
smartphones, though less secure. Need to handle edge cases like lost devices.

BusinessExpert: Phased rollout aligns with Q3 enterprise push. Can market as security
upgrade. Estimate $50K implementation, $200K annual revenue uplift from enterprise.

SecurityExpert: SMS is vulnerable to SIM swapping. Recommend authenticator app as
primary, with backup codes. Must document recovery procedures for customer support.

Round 3:
--------
[All participants refine recommendations based on discussion...]

Final Recommendation:
--------------------
Implement 2FA with phased rollout: (1) Admin accounts mandatory Q2, (2) Enterprise
customers Q3, (3) All users optional Q4. Use authenticator apps with backup codes.
Skip SMS due to security concerns. Budget approved: $50K dev + $30K support training.
Expected impact: 98% reduction in account takeovers, $200K annual revenue increase.
```

---

## Turn-Taking Strategies

Turn-taking strategies determine **who speaks next** in the council discussion.

### 1. RoundRobin

**Description**: Participants speak in order, cycling through the list repeatedly.

**Behavior**:
- Fair: Each participant gets equal speaking opportunities
- Predictable: Order known in advance
- Balanced: No participant dominates discussion

**Use When**:
- Equal expertise importance
- Balanced participation desired
- Simple discussion structure

**Example**:
```rust,ignore
let council = CouncilBuilder::new()
    .add_participant(expert1)
    .add_participant(expert2)
    .add_participant(expert3)
    .turn_strategy(TurnStrategy::RoundRobin)
    .build()?;

// Turn order: Expert1 → Expert2 → Expert3 → Expert1 → Expert2 → ...
```

**Diagram**:
```
Round 1:  [Expert1] → [Expert2] → [Expert3]
Round 2:  [Expert1] → [Expert2] → [Expert3]
Round 3:  [Expert1] → [Expert2] → [Expert3]
```

---

### 2. ModeratorDirected

**Description**: A moderator agent controls the discussion flow, selecting who speaks next.

**Behavior**:
- Strategic: Moderator calls on relevant experts based on context
- Flexible: Can skip participants if not relevant
- Guided: Moderator ensures productive discussion

**Use When**:
- Complex topics requiring expert guidance
- Some experts more relevant than others
- Need to avoid tangents
- Senior oversight required

**Example**:
```rust,ignore
let moderator = create_paladin(
    "Moderator",
    "You moderate the council. Call on experts strategically and decide when to conclude."
);

let council = CouncilBuilder::new()
    .moderator(moderator)
    .add_participant(frontend_expert)
    .add_participant(backend_expert)
    .add_participant(devops_expert)
    .turn_strategy(TurnStrategy::ModeratorDirected)
    .build()?;
```

**Moderator System Prompt Example**:
```rust,ignore
let moderator_prompt = r#"
You are the Chief Architect moderating a technical council.

Your responsibilities:
1. FACILITATE: Call on relevant experts based on topic
2. MANAGE: Ensure focused, productive discussion
3. SYNTHESIZE: Identify key themes and consensus points
4. DECIDE: Determine when sufficient deliberation achieved

Example commands:
- "I call on [ExpertName] to address [topic]"
- "Let's hear from [ExpertName] on [aspect]"
- "We have consensus - discussion complete"

Keep discussion focused and drive toward actionable recommendations.
"#;
```

**Diagram**:
```
         ┌──────────────┐
         │  Moderator   │
         └──────┬───────┘
                │ (calls on)
    ┌───────────┼───────────┐
    ▼           ▼           ▼
[Expert1]   [Expert2]   [Expert3]
    │           │           │
    └───────────┴───────────┘
                │
         (responds to)
         ┌──────▼───────┐
         │  Moderator   │
         └──────────────┘
```

---

## Termination Conditions

Termination conditions determine **when the council discussion concludes**.

### 1. MaxRounds

**Description**: Discussion ends after a fixed number of rounds.

**Use When**:
- Time-boxed discussions
- Budget constraints (LLM API costs)
- Simple topics not requiring extended debate

**Configuration**:
```rust,ignore
.max_rounds(5)
    .termination_condition(TerminationCondition::MaxRounds)
```

**Behavior**:
- Deterministic: Always stops after N rounds
- Predictable cost: Known number of LLM calls
- May end prematurely if consensus not reached

**Example**:
```rust,ignore
let council = CouncilBuilder::new()
    .add_participant(expert1)
    .add_participant(expert2)
    .add_participant(expert3)
    .turn_strategy(TurnStrategy::RoundRobin)
    .max_rounds(3)
    .termination_condition(TerminationCondition::MaxRounds) // 3 rounds
    .build()?;

// 3 participants × 3 rounds = 9 total turns
```

---

### 2. Consensus

**Description**: Discussion continues until participants reach consensus (detected via keyword or sentiment analysis).

**Use When**:
- Consensus critical to outcome
- Quality more important than speed
- Sufficient budget for extended discussion

**Configuration**:
```rust,ignore
.termination_condition(TerminationCondition::Consensus {
    required_agreement_keywords: vec![
        "I agree".to_string(),
        "consensus reached".to_string(),
        "we all support".to_string(),
    ],
    min_participants: 2, // At least 2 participants must express agreement
})
```

**Detection Logic**:
1. Check if recent participant outputs contain agreement keywords
2. Count how many participants expressed agreement
3. If `min_participants` threshold met → terminate

**Example**:
```rust,ignore
let council = CouncilBuilder::new()
    .add_participant(expert1)
    .add_participant(expert2)
    .add_participant(expert3)
    .turn_strategy(TurnStrategy::RoundRobin)
    .termination_condition(TerminationCondition::Consensus {
        required_agreement_keywords: vec!["I agree".into(), "consensus".into()],
        min_participants: 2,
    })
    .max_rounds(10) // Safety limit
    .build()?;
```

**Behavior**:
- Dynamic: Stops when agreement detected
- Quality-focused: Ensures alignment
- Risk: May run to max_rounds if no consensus

---

### 3. ModeratorDecision

**Description**: Moderator decides when sufficient deliberation has occurred.

**Use When**:
- ModeratorDirected turn strategy
- Need expert judgment on completeness
- Complex topics requiring flexible stopping point

**Configuration**:
```rust,ignore
.termination_condition(TerminationCondition::ModeratorDecision)
```

**Moderator Signal**:
The moderator indicates completion by including a termination phrase:
```
"The discussion is complete."
"We have sufficient input to proceed."
"I conclude this council session."
```

**Detection Keywords** (configurable):
```rust,ignore
pub const DEFAULT_MODERATOR_TERMINATION_KEYWORDS: &[&str] = &[
    "discussion complete",
    "conclude",
    "sufficient input",
    "end discussion",
];
```

**Example**:
```rust,ignore
let moderator = create_paladin("ChiefArchitect", moderator_prompt);

let council = CouncilBuilder::new()
    .moderator(moderator)
    .add_participant(expert1)
    .add_participant(expert2)
    .turn_strategy(TurnStrategy::ModeratorDirected)
    .termination_condition(TerminationCondition::ModeratorDecision)
    .max_rounds(20) // Safety limit
    .build()?;
```

---

### 4. Keyword

**Description**: Discussion ends when any participant uses a specific keyword.

**Use When**:
- Explicit approval workflows (e.g., "APPROVED")
- Go/no-go decisions
- Trigger-based termination

**Configuration**:
```rust,ignore
.termination_condition(TerminationCondition::Keyword("APPROVED".to_string()))
```

**Example - Code Review Approval**:
```rust,ignore
let council = CouncilBuilder::new()
    .add_participant(senior_dev)
    .add_participant(security_reviewer)
    .add_participant(qa_lead)
    .turn_strategy(TurnStrategy::RoundRobin)
    .termination_condition(TerminationCondition::Keyword("APPROVED".into()))
    .build()?;

// Discussion continues until any participant says "APPROVED"
```

**Use Case - Budget Approval**:
```
CFO: "After reviewing the proposal, I approve the $500K budget. APPROVED."
→ Discussion terminates immediately
```

---

## Garrison Integration

Council supports **conversation history storage** via Garrison (memory system), enabling:

✅ **Context Persistence**: Store full discussion transcript
✅ **Retrieval**: Reference past council decisions
✅ **Analysis**: Track consensus patterns over time
✅ **Auditing**: Complete audit trail of deliberations

### Enabling Garrison

```rust,ignore
use paladin::infrastructure::adapters::garrison::in_memory_garrison::InMemoryGarrison;

// Create Garrison
let garrison = Arc::new(InMemoryGarrison::new());

// Create Council service with Garrison
let service = CouncilExecutionService::new(
    Arc::new(paladin_port),
    Some(garrison.clone()), // Enable history storage
    registry,
);

// Execute council
let result = service.convene(&council, topic).await?;

// Access stored conversation
let history = garrison.retrieve(&council.id()).await?;
println!("Full transcript: {}", history);
```

### Storage Format

```json
{
  "council_id": "council-uuid-123",
  "topic": "Should we implement feature X?",
  "participants": ["TechnicalExpert", "BusinessExpert", "SecurityExpert"],
  "rounds": [
    {
      "round": 1,
      "turns": [
        {
          "speaker": "TechnicalExpert",
          "content": "Technical perspective: ...",
          "timestamp": "2026-02-04T10:30:00Z"
        },
        ...
      ]
    }
  ],
  "termination_reason": "MaxRounds",
  "conclusion": "Synthesized recommendation: ..."
}
```

---

## Configuration

### CouncilConfig

```rust,ignore
pub struct CouncilConfig {
    /// Maximum number of rounds before forced termination
    pub max_rounds: u32,

    /// Turn-taking strategy (RoundRobin or ModeratorDirected)
    pub turn_strategy: TurnStrategy,

    /// Termination condition
    pub termination_condition: TerminationCondition,

    /// Whether to include conversation history in each Paladin's context
    pub include_history: bool,
}

impl Default for CouncilConfig {
    fn default() -> Self {
        Self {
            max_rounds: 10,
            turn_strategy: TurnStrategy::default(),
            termination_condition: TerminationCondition::default(),
            include_history: true,
        }
    }
}
```

### Builder Pattern

```rust,ignore
let council = CouncilBuilder::new()
    .name("Expert Panel")
    .add_participant(expert1)
    .add_participant(expert2)
    .add_participant(expert3)
    .moderator(moderator) // Optional
    .turn_strategy(TurnStrategy::RoundRobin)
    .termination_condition(TerminationCondition::MaxRounds)
    .max_rounds(10)
    .include_history(true)
    .build()?;
```

---

## Examples

### Example 1: Security Review Panel

```rust,ignore
let security_expert = create_paladin("SecurityExpert",
    "Focus on security risks and controls");
let legal_expert = create_paladin("LegalExpert",
    "Focus on compliance and legal requirements");
let technical_expert = create_paladin("TechnicalExpert",
    "Focus on implementation feasibility");

let council = CouncilBuilder::new()
    .name("Security Review Council")
    .add_participant(security_expert)
    .add_participant(legal_expert)
    .add_participant(technical_expert)
    .turn_strategy(TurnStrategy::RoundRobin)
    .max_rounds(3)
    .termination_condition(TerminationCondition::MaxRounds)
    .build()?;

let topic = "Evaluate the security implications of storing customer payment data";
let result = service.convene(&council, topic).await?;
```

### Example 2: Moderated Architecture Review

```rust,ignore
let moderator = create_paladin("ChiefArchitect", MODERATOR_PROMPT);

let council = CouncilBuilder::new()
    .name("Architecture Review")
    .moderator(moderator)
    .add_participant(frontend_lead)
    .add_participant(backend_lead)
    .add_participant(devops_lead)
    .turn_strategy(TurnStrategy::ModeratorDirected)
    .termination_condition(TerminationCondition::ModeratorDecision)
    .max_rounds(15)
    .build()?;

let topic = "Should we adopt GraphQL or stick with REST?";
let result = service.convene(&council, topic).await?;
```

### Example 3: Consensus-Based Decision

```rust,ignore
let council = CouncilBuilder::new()
    .name("Product Launch Council")
    .add_participant(product_manager)
    .add_participant(engineering_lead)
    .add_participant(marketing_lead)
    .turn_strategy(TurnStrategy::RoundRobin)
    .termination_condition(TerminationCondition::Consensus {
        required_agreement_keywords: vec!["I agree".into(), "consensus".into()],
        min_participants: 2,
    })
    .max_rounds(8)
    .build()?;

let topic = "Are we ready to launch the new feature to production?";
let result = service.convene(&council, topic).await?;
```

---

## Best Practices

### 1. Participant Selection

✅ **Do**:
- Choose 3-7 participants (optimal for discussion)
- Ensure diverse perspectives
- Define clear expertise areas in system prompts
- Use descriptive names (TechnicalExpert vs Expert1)

❌ **Don't**:
- Use too many participants (>10 = chaotic)
- Include redundant perspectives
- Use generic system prompts
- Forget to specify participant roles

### 2. System Prompts

✅ **Do**:
```rust,ignore
let prompt = r#"
You are a security expert in a council discussion.

Your role:
- Identify security risks and vulnerabilities
- Recommend security controls
- Build on points made by other council members
- Keep responses concise (2-3 paragraphs)

Discussion format:
1. Acknowledge relevant points from previous speakers
2. Contribute your security perspective
3. Ask clarifying questions if needed
"#;
```

❌ **Don't**:
```rust,ignore
let prompt = "You are an expert."; // Too vague
```

### 3. Turn Strategy Selection

| Scenario | Recommended Strategy | Reason |
|----------|---------------------|--------|
| Equal expertise importance | RoundRobin | Fair, balanced |
| Complex topics | ModeratorDirected | Expert guidance |
| Time-sensitive | RoundRobin + MaxRounds | Predictable |
| Critical decisions | ModeratorDirected + ModeratorDecision | Quality focus |

### 4. Termination Condition Selection

| Goal | Recommended Condition | Configuration |
|------|----------------------|---------------|
| Time-boxed | MaxRounds | 3-5 rounds typical |
| Consensus required | Consensus | min_participants = ⌈N/2⌉ |
| Expert-guided | ModeratorDecision | With moderator |
| Approval workflow | Keyword | "APPROVED" or "GO" |

### 5. Cost Optimization

Council discussions can be expensive (multiple LLM calls per round).

**Cost Calculation**:
```
Total Calls = Participants × Rounds
Cost = Total Calls × LLM_Cost_Per_Call

Example: 3 participants × 5 rounds = 15 calls
With GPT-4: 15 × $0.03 = $0.45 per council
With GPT-4o-mini: 15 × $0.005 = $0.075 per council
```

**Optimization Strategies**:
1. Use MaxRounds termination for cost ceiling
2. Choose lower-cost models for non-critical discussions
3. Limit participants to essential perspectives
4. Cache common participant responses
5. Consider Phalanx for independent analysis

### 6. Conversation Quality

**Improve discussion quality**:
1. **Clear topics**: "Should we implement X?" not "Tell me about X"
2. **Specific context**: Provide background information in topic
3. **Response length**: Guide participants to 2-3 paragraphs
4. **Build-on prompts**: Encourage referencing previous speakers
5. **Summarization**: Have final turn synthesize discussion

**Example high-quality topic**:
```rust,ignore
let topic = r#"
Should we implement two-factor authentication for all users?

Context:
- 100K active users (70% consumer, 30% enterprise)
- Recent industry trend toward mandatory 2FA
- Enterprise customers requesting this feature
- Current: Email/password only

Consider:
- Technical implementation complexity
- User experience and friction
- Security improvement quantification
- Cost vs benefit analysis
"#;
```

---

## API Reference

### Core Types

```rust,ignore
// Council configuration
pub struct Council {
    pub id: String,
    pub name: String,
    pub participants: Vec<Paladin>,
    pub moderator: Option<Paladin>,
    pub config: CouncilConfig,
}

// Turn-taking strategies
pub enum TurnStrategy {
    RoundRobin,
    ModeratorDirected,
}

// Termination conditions -- the round count itself lives on CouncilConfig::max_rounds /
// CouncilBuilder::max_rounds(), not on this enum; this enum only selects the strategy.
pub enum TerminationCondition {
    /// Stop after reaching maximum number of rounds
    MaxRounds,
    /// Detect consensus through keyword matching (e.g., "I agree", "consensus reached")
    Consensus,
    /// Moderator decides when to end (e.g., says "discussion concluded")
    ModeratorDecision,
    /// Custom keyword triggers termination
    Keyword(String),
}

// Council result (crates/paladin-battalion/src/council_service.rs)
pub struct CouncilResult {
    pub transcript: Vec<CouncilMessage>,
    pub conclusion: Option<String>,
    pub rounds_completed: u32,
    pub termination_reason: TerminationCondition,
}
```

### Services

```rust,ignore
// Council execution service
pub struct CouncilExecutionService {
    paladin_port: Arc<dyn PaladinPort>,
    garrison_port: Option<Arc<dyn GarrisonPort>>,
}

impl CouncilExecutionService {
    pub fn new(
        paladin_port: Arc<dyn PaladinPort>,
        garrison_port: Option<Arc<dyn GarrisonPort>>,
    ) -> Self;

    pub async fn convene(
        &self,
        council: &Council,
        topic: &str,
    ) -> Result<CouncilResult, CouncilError>;
}
```

### Builder

```rust,ignore
pub struct CouncilBuilder {
    // ...
}

impl CouncilBuilder {
    pub fn new() -> Self;
    pub fn name(self, name: impl Into<String>) -> Self;
    pub fn add_participant(self, paladin: Paladin) -> Self;
    pub fn moderator(self, paladin: Paladin) -> Self;
    pub fn turn_strategy(self, strategy: TurnStrategy) -> Self;
    pub fn termination_condition(self, condition: TerminationCondition) -> Self;
    pub fn max_rounds(self, rounds: u32) -> Self;
    pub fn store_history(self, store: bool) -> Self;
    pub fn build(self) -> Result<Council, CouncilError>;
}
```

---

## See Also

- [Battalion Overview](battalion-patterns-guide.md) - All orchestration patterns
- [Grove Pattern](grove.md) - Intelligent agent routing
- [Commander](../user-guides/orchestration.md) - Strategy selection
- [Configuration Examples](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples/cli_configs) - YAML configs
- [Code Examples](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples) - Rust examples

---

**Next Steps**:
- Try the [Quick Start](#quick-start) example
- Explore [YAML configurations](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples/cli_configs)
- See [practical examples](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples)
- Review [API documentation](https://docs.rs/paladin)
