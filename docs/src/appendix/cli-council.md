# paladin council - Quick Group Discussions

Execute quick multi-agent discussions without writing configuration files. Get diverse perspectives from multiple AI Paladins on any topic.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Command Syntax](#command-syntax)
- [Agent Roles](#agent-roles)
- [Discussion Rounds](#discussion-rounds)
- [Output Options](#output-options)
- [Best Practices](#best-practices)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Overview

The `council` command enables:
- **Ad-hoc** multi-agent discussions without configuration files
- **Diverse perspectives** from multiple AI personas
- **Multi-round** discussions bounded by `--max-rounds`
- **Transcript saving** to a file with `--save`
- **Quick iterations** for brainstorming and decision-making

### When to Use Council

✅ **Use council when:**
- Need quick input from multiple AI perspectives
- Brainstorming solutions to problems
- Evaluating options from different viewpoints
- Quick analysis without formal configuration
- Prototyping multi-agent workflows

❌ **Don't use council when:**
- Need precise control over agent configuration
- Building production workflows (use `paladin run` instead)
- Require state persistence across sessions
- Need custom tools or memory systems

## Quick Start

### Basic Usage

```bash
# Simple discussion with default agents
paladin council --topic "What are the best practices for API design?"

# Specify number of participants
paladin council --topic "Should we migrate to microservices?" --participants 5

# Bound the number of discussion rounds
paladin council --topic "Analyze this business proposal..." --max-rounds 2

# Save the transcript to a file
paladin council --topic "Security implications of cloud migration" --save results.md
```

## Command Syntax

The `council` subcommand takes no positional argument; the discussion topic is passed with
`--topic`. Build the binary with the `cli` feature before capturing this output yourself:
`cargo build --release --features cli --bin paladin-cli` (the binary carries
`required-features = ["cli"]` and is not produced by a default `cargo build`).

```text
$ paladin-cli council --help
Run a council discussion

Usage: paladin-cli council [OPTIONS]

Options:
      --topic <TOPIC>                Discussion topic
      --participants <PARTICIPANTS>  Number of participants (2-10) [default: 3]
      --roles <ROLES>                Custom roles (comma-separated)
      --max-rounds <MAX_ROUNDS>      Maximum discussion rounds [default: 5]
      --save <SAVE>                  Save transcript to file
      --model <MODEL>                Model to use
      --temperature <TEMPERATURE>    Temperature setting
      --quiet                        Enable quiet mode (minimal output)
      --verbose                      Enable verbose mode (detailed output)
  -h, --help                         Print help
```

`--quiet` and `--verbose` are global flags shared by every subcommand.

## Agent Roles

### Default Roles

When roles aren't specified, council uses diverse default perspectives:

1. **Analyst** - Data-driven, analytical approach
2. **Critic** - Identifies risks, challenges, and weaknesses
3. **Optimist** - Focuses on opportunities and benefits

### Custom Roles

```bash
# Technical perspectives
paladin council --topic "System design question" --roles "architect,security,devops,qa"

# Business perspectives
paladin council --topic "Product launch strategy" --roles "ceo,cfo,cmo,product"

# Creative perspectives
paladin council --topic "Marketing campaign" --roles "creative,pragmatic,critic,synthesizer"

# Domain-specific
paladin council --topic "Data governance policy" --roles "legal,compliance,privacy,security"
```

### Role Examples

| Role | Perspective | Best For |
|------|-------------|----------|
| **technical** | Engineering, architecture, implementation | Technical decisions |
| **business** | ROI, market fit, business value | Business strategy |
| **security** | Threats, vulnerabilities, compliance | Security reviews |
| **ux** | User experience, usability, accessibility | Design decisions |
| **legal** | Compliance, liability, regulations | Legal considerations |
| **creative** | Innovation, alternative approaches | Brainstorming |
| **critic** | Risks, challenges, weaknesses | Risk analysis |
| **pragmatic** | Practical, realistic, achievable | Implementation planning |
| **optimist** | Opportunities, benefits, positives | Opportunity discovery |
| **analyst** | Data, metrics, evidence-based | Data-driven decisions |

## Discussion Rounds

The live `council` subcommand has no discussion-mode flag — there is no parallel, sequential or
debate mode to select. Instead, `--participants` sets how many agents join the discussion and
`--max-rounds` bounds how many rounds the discussion runs for (default: 5 rounds, 3 participants).

```bash
# Fewer rounds for a quick read
paladin council --topic "What are the pros and cons of NoSQL?" --max-rounds 2

# More rounds for a deeper discussion
paladin council --topic "How should we approach this technical debt?" --max-rounds 8

# More participants for broader coverage
paladin council --topic "Should we use serverless architecture?" --participants 6
```

## Output Options

The `council` subcommand has one output flag, `--save <FILE>`, which writes the discussion
transcript to a file. There is no `--format` flag — the live subcommand exposes no JSON or plain
text output option.

```bash
paladin council --topic "Cloud strategy" --save discussion.md
```

```markdown
# Council Discussion: Cloud Strategy

## Question
What cloud strategy should we adopt?

## Participants
- Technical Architect
- Business Analyst  
- Security Specialist

## Responses

### Technical Architect
**Perspective:** Technical Implementation

[Response content...]

**Key Points:**
- Multi-cloud for redundancy
- Containerization strategy
- Migration roadmap

### Business Analyst
**Perspective:** Business Value

[Response content...]

**Key Points:**
- Cost optimization
- Scalability benefits
- Time to market

### Security Specialist
**Perspective:** Security & Compliance

[Response content...]

**Key Points:**
- Data sovereignty
- Encryption standards
- Compliance requirements

## Synthesis

[Synthesized recommendations...]

## Action Items

1. Evaluate cloud providers
2. Conduct security audit
3. Create migration plan
```

## Best Practices

### 1. Frame Topics Clearly

✅ **Good:**
```bash
paladin council --topic "
Should we adopt GraphQL for our public API?

Context:
- RESTful API with 50+ endpoints
- 100k requests/day
- Mobile and web clients
- Team of 5 backend developers
"
```

❌ **Avoid:**
```bash
paladin council --topic "graphql?"
```

### 2. Choose Appropriate Roles

```bash
# For technical decisions
paladin council --topic "Kubernetes vs. ECS" --roles "architect,security,devops"

# For product decisions
paladin council --topic "Feature prioritization" --roles "product,ux,engineering,business"

# For strategic decisions
paladin council --topic "Market expansion strategy" --roles "ceo,cto,cfo,cmo"
```

### 3. Tune Participants and Rounds

```bash
# Quick diverse input → fewer participants, fewer rounds
paladin council --topic "Initial thoughts on blockchain integration" --participants 3 --max-rounds 2

# Building on ideas over more rounds
paladin council --topic "Refine our architecture approach" --max-rounds 8

# Broader coverage for evaluating alternatives
paladin council --topic "Build vs. buy for authentication" --participants 6
```

### 4. Save the Transcript

```bash
# Save the discussion transcript
paladin council --topic "Complex decision" --save results.md
# Then review results.md for the discussion history
```

### 5. Iterate and Refine

```bash
# First pass - broad input
paladin council --topic "App architecture options" --save round1.md

# Review results, then deep dive
paladin council --topic "Microservices concerns from round 1" --save round2.md

# Final decision, more rounds for depth
paladin council --topic "Final architecture decision" --max-rounds 8 --save final.md
```

## Examples

### Example 1: Quick Technical Decision

```bash
paladin council --participants 4 --topic "
Should we use TypeScript or JavaScript for our new service?

Context:
- Team has JavaScript experience
- Large codebase (100k+ LOC)
- Need to maintain velocity
- Some junior developers
"
```

### Example 2: Security Review

```bash
paladin council --roles "security,privacy,compliance,devops" --max-rounds 6 --topic "
Review our authentication approach:

Current:
- JWT tokens
- 1-hour expiration  
- Stored in localStorage
- No refresh tokens

Concerns:
- XSS vulnerability?
- CSRF protection?
- Mobile app considerations?
"
```

### Example 3: Architecture Trade-off Discussion

```bash
paladin council --roles "monolith-advocate,microservices-advocate" --max-rounds 6 --topic "
Should we migrate from monolith to microservices?

Current state:
- Monolithic Rails app
- 5-year-old codebase
- 10 developers
- Deployment issues
- Scaling challenges
"
```

### Example 4: Product Strategy

```bash
paladin council --roles "product,marketing,sales,engineering,support" --save strategy.md --topic "
Should we build a mobile app or focus on responsive web?

Data:
- 60% mobile traffic
- Limited mobile team
- 6-month timeline
- Competitor has native apps
"
```

### Example 5: Incident Post-Mortem

```bash
paladin council --roles "sre,security,engineering,management" --topic "
Post-mortem for database outage:

Incident:
- 2-hour downtime
- Caused by failed migration
- No rollback plan
- Manual recovery

Questions:
- What went wrong?
- How to prevent?
- Process improvements?
"
```

### Example 6: Code Review Perspectives

```bash
paladin council --roles "security,performance,maintainability,testing" --topic "
Review this architecture decision:

Plan to use Redis for:
- Session storage
- Cache layer  
- Message queue
- Rate limiting

Is this appropriate?
"
```

## Troubleshooting

### Common Issues

#### Issue: Responses are too generic

**Solution:**
```bash
# Provide more context
paladin council --topic "Question with detailed context: ..."

# Use more specific roles
paladin council --roles "senior-architect,principal-engineer" --topic "..."

# Increase rounds for depth
paladin council --max-rounds 8 --topic "..."
```

#### Issue: Conflicting perspectives without resolution

**Solution:**
```bash
# Increase participants for more coverage
paladin council --participants 6 --topic "..."

# Do a follow-up round
paladin council --topic "Based on previous discussion, recommend best approach"
```

#### Issue: Discussion runs too long

**Solution:**
```bash
# Reduce the number of discussion rounds
paladin council --max-rounds 2 --topic "complex question"

# Reduce the number of participants
paladin council --participants 2 --topic "..."
```

#### Issue: Not enough detail in responses

**Solution:**
```bash
# Increase the number of discussion rounds
paladin council --max-rounds 8 --topic "detailed analysis needed"

# Ask more specific topics
paladin council --topic "Specific aspect of broader topic"

# Use higher temperature for creativity
paladin council --temperature 1.0 --topic "creative problem-solving"
```

#### Issue: Agent perspectives are too similar

**Solution:**
```bash
# Use more diverse roles
paladin council --roles "conservative,progressive,radical,pragmatic" --topic "..."

# Increase rounds so agents can diverge
paladin council --max-rounds 6 --topic "..."

# Increase temperature
paladin council --temperature 1.2 --topic "diverse viewpoints needed"
```

### Debugging

```bash
# Enable verbose mode to see execution details
paladin council --verbose --topic "..."

# Test with a simpler topic first
paladin council --participants 2 --topic "Hello, how are you?"

# Check provider configuration
paladin setup-check
```

## Advanced Usage

### Combining with Other Commands

```bash
# Generate config, then discuss it
paladin muster --task "workflow" --output workflow.yaml
paladin council --topic "Review this workflow config: $(cat workflow.yaml)"

# Council for planning, then generate and execute the resulting config
paladin council --topic "Best approach for task X" --save plan.md
# Review plan.md, then generate and run the battalion config in one step
paladin muster --task "$(cat plan.md)" --execute
```

### Batch Processing

```bash
# Multiple topics from file
while IFS= read -r topic; do
    paladin council --topic "$topic" --save "output_$(echo "$topic" | md5sum | cut -c1-8).md"
done < topics.txt

# Different role combinations
for roles in "tech,security" "business,legal" "ux,product"; do
    paladin council --roles "$roles" --topic "Same topic" --save "perspective_${roles}.md"
done
```

### Reviewing the Saved Transcript

```bash
# Save the transcript
paladin council --topic "Complex decision" --save transcript.md

# Feed it back for a follow-up discussion
paladin council --topic "Follow up on this discussion: $(cat transcript.md)"
```

## Performance Tips

| Scenario | Recommended Settings |
|----------|---------------------|
| **Quick input** | `--participants 3 --max-rounds 2` |
| **Detailed analysis** | `--participants 5 --max-rounds 8` |
| **Fast iteration** | `--participants 2 --max-rounds 2` |
| **Deep dive** | `--participants 4 --max-rounds 8` |
| **High quality** | `--model claude-3-opus` |

## See Also

- [CLI Usage Guide](cli-usage.md) - Overview of all CLI commands
- [Muster Command](cli-muster.md) - Generate full Battalion configurations
- [Conclave Pattern](council.md) - Detailed council/conclave documentation
- [Battalion Patterns](../user-guides/battalion-patterns.md) - Understanding orchestration patterns
- [Examples Directory](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples) - Sample implementations

## Support

- **Issues**: Report bugs at https://github.com/DF3NDR/paladin-dev-env/issues
- **Discussions**: Ask questions in GitHub Discussions
- **Documentation**: Full docs at https://paladin-ai.dev

---

*Council discussions are ephemeral and don't persist state. For production workflows with state management, use `paladin run` with configuration files.*
