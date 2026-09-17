# paladin muster - AI-Powered Battalion Generation

Generate production-ready Battalion configurations from natural language descriptions using LLM intelligence.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Command Syntax](#command-syntax)
- [Generation Workflow](#generation-workflow)
- [Configuration Options](#configuration-options)
- [Best Practices](#best-practices)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Overview

The `muster` command leverages LLM intelligence to:
- **Translate** natural language descriptions into Battalion configurations
- **Recommend** an orchestration pattern (Formation, Phalanx, Campaign, Chain of Command,
  Conclave, Maneuver) based on the task description
- **Generate** a complete YAML configuration
- **Review** the generated configuration before saving (accept, edit, or cancel), unless
  `--no-review` skips the step
- **Execute** the battalion immediately after generation, when `--execute` is given

### When to Use Muster

✅ **Use muster when:**
- Creating complex multi-agent workflows from scratch
- Prototyping new orchestration patterns
- Need AI suggestions for optimal agent coordination
- Want a ready-to-edit configuration quickly

❌ **Don't use muster when:**
- You have existing configurations (use `paladin battalion run` instead)
- Need precise manual control over every parameter
- Working with sensitive/proprietary orchestration logic

## Quick Start

### Basic Usage

```bash
# Generate a simple sequential workflow
paladin muster --task "Create a data analysis pipeline: fetch data, clean it, analyze patterns, generate report"

# Generate a parallel processing workflow
paladin muster --task "Process customer reviews in parallel: sentiment analysis, topic extraction, summary generation"

# Generate and save to a specific path, skipping the interactive review
paladin muster --task "Code review workflow" --output code_review.yaml --no-review
```

## Command Syntax

The `muster` subcommand takes no positional argument; the task description is passed with
`--task` (or supplied interactively if omitted). Build the binary with the `cli` feature before
capturing this output yourself: `cargo build --release --features cli --bin paladin-cli` (the
binary carries `required-features = ["cli"]` and is not produced by a default `cargo build`).

```text
$ paladin-cli muster --help
Generate battalion configuration from task description

Usage: paladin-cli muster [OPTIONS]

Options:
      --task <TASK>          Task description
  -o, --output <OUTPUT>      Output file path
      --execute              Execute immediately after generation
      --provider <PROVIDER>  LLM provider to use
      --model <MODEL>        Model to use
      --no-review            Skip review step
      --quiet                Enable quiet mode (minimal output)
      --verbose              Enable verbose mode (detailed output)
  -h, --help                 Print help
```

`--quiet` and `--verbose` are global flags shared by every subcommand. The orchestration pattern
is chosen by the LLM analysis rather than a flag, output is always YAML, and the only interactive
step is the accept/edit/cancel review prompt (skipped with `--no-review`); there is no separate
flag to force a pattern, pick an output format, auto-confirm, tune generation temperature, toggle
schema validation, or enter a conversational refinement mode.

## Generation Workflow

### 1. Analysis Phase

```bash
paladin muster --task "Build a content moderation system"
```

```
🏰 Muster - Battalion Configuration Generator

Analyzing task requirements with AI...

📋 Analysis Results
Pattern: campaign
Battalion: content_moderation_system
Reasoning: ...
Agents: 4 recommended
```

### 2. Configuration Generation

```
Generating battalion configuration...
```

### 3. Review Phase (skipped with `--no-review`)

```
📄 Generated Configuration

name: content_moderation_system
description: Automated content moderation with classification and review

battalion:
  type: campaign
  graph:
    nodes:
      - id: content_classifier
        paladin: classifier
      - id: toxicity_detector
        paladin: toxicity
      - id: human_review
        paladin: reviewer
        condition: "{{toxicity_detector.score}} > 0.7"
      - id: final_decision
        paladin: decision_maker

    edges:
      - from: content_classifier
        to: toxicity_detector
      - from: toxicity_detector
        to: human_review
      - from: toxicity_detector
        to: final_decision
      - from: human_review
        to: final_decision

paladins:
  classifier:
    system_prompt: "Classify content into categories..."
    model: gpt-4
    temperature: 0.3
  # ... additional paladins

Accept this configuration? [Y/n]:
```

### 4. Save (and optional Execute)

The configuration is written to `--output` (or a timestamped default,
`muster_<battalion_name>_<timestamp>.yaml`, if `--output` is omitted). If `--execute` was given,
the battalion runs immediately after saving; otherwise the command prints the follow-up
`paladin battalion run -c <path>` invocation.

## Configuration Options

### Orchestration Patterns

The LLM analysis recommends one of six patterns based on the task description — there is no flag
to force a pattern:

| Pattern | Best for |
|---------|----------|
| **Formation** (Sequential) | Linear workflows, step-by-step processing (extract → transform → load) |
| **Phalanx** (Parallel) | Independent parallel tasks — multiple perspectives on the same input |
| **Campaign** (Graph/DAG) | Complex workflows with branching or conditional logic |
| **Chain of Command** (Hierarchical) | Manager-worker patterns, dynamic task distribution |
| **Conclave** | Expert-panel discussion with voting, for consensus decisions |
| **Maneuver** | Dynamic workflow adaptation at runtime |

Describe the dependency shape you want in `--task` to steer the recommendation, for example:

```bash
paladin muster --task "Data processing pipeline: extract, then transform, then load"

paladin muster --task "Analyze documents from multiple independent perspectives in parallel"

paladin muster --task "Complex workflow with conditional branches based on review outcome"
```

### Provider Selection

```bash
# Use a specific provider
paladin muster --task "Customer support workflow" --provider openai

# Use a specific model
paladin muster --task "Research synthesis" --provider anthropic --model claude-3-opus
```

There is no `--temperature` flag on `muster` — generation temperature is not user-configurable
for this subcommand.

## Best Practices

### 1. Write Clear Descriptions

✅ **Good:**
```bash
paladin muster --task "Create a 3-stage content pipeline:
1. Extract key information from articles
2. Summarize findings into bullet points
3. Generate social media posts from summaries"
```

❌ **Avoid:**
```bash
paladin muster --task "do content stuff"
```

### 2. Specify Requirements

```bash
paladin muster --task "
Research workflow that:
- Searches multiple sources in parallel
- Synthesizes findings sequentially
- Requires 4-5 specialized agents
- Should complete within 2 minutes
"
```

### 3. Use the Review Step to Confirm or Cancel

The review prompt (skipped with `--no-review`) lets you accept the generated configuration or
cancel the run before anything is saved:

```bash
paladin muster --task "Customer onboarding workflow"
# Review the printed YAML, then answer the "Accept this configuration?" prompt
```

### 4. Review Before Production

```bash
# Generate, then review the saved file
paladin muster --task "Workflow" --output config.yaml

# Test before relying on it
paladin battalion run -c config.yaml
```

### 5. Use Version Control

```bash
# Save with descriptive names
paladin muster --task "v2 with retry logic" --output workflow_v2.yaml

# Track changes
git add workflow_v2.yaml
git commit -m "feat: add retry logic to workflow"
```

## Examples

### Example 1: Data Analysis Pipeline

```bash
paladin muster --output data_pipeline.yaml --task "
Sequential data analysis:
1. Fetch data from API
2. Clean and validate data
3. Perform statistical analysis
4. Generate visualization recommendations
5. Create final report
"
```

### Example 2: Parallel Content Processing

```bash
paladin muster --output content_processor.yaml --task "
Process a blog post in parallel:
- Generate SEO keywords
- Create social media summaries
- Extract key quotes
- Suggest related topics
- Analyze sentiment
"
```

### Example 3: Approval Workflow

```bash
paladin muster --output approval_workflow.yaml --task "
Document approval workflow with conditional branching:
1. Initial review checks format and completeness
2. If incomplete, request revisions
3. If complete, route to appropriate reviewer based on category
4. Technical docs go to tech reviewer
5. Business docs go to business reviewer
6. Final approval from manager
"
```

### Example 4: Customer Support Routing

```bash
paladin muster --output support_routing.yaml --task "
Hierarchical customer support ticket routing:
- Manager paladin receives all tickets
- Routes technical questions to tech support team
- Routes billing questions to billing team
- Routes general inquiries to customer service
- Escalates complex issues to senior support
"
```

### Example 5: Research & Synthesis

```bash
paladin muster --output research_workflow.yaml --task "
Research workflow:
1. Parallel search across academic papers, news, and blogs
2. Collect and filter relevant information
3. Synthesize findings into coherent summary
4. Generate citation list
"
```

## Troubleshooting

### Common Issues

#### Issue: Generated config is too simple

**Solution:**
```bash
# Provide a more detailed description
paladin muster --task "Detailed workflow with specific steps: ..." --verbose

# Describe the dependency shape you want more explicitly
paladin muster --task "..."
```

#### Issue: Wrong orchestration pattern recommended

**Solution:**
```bash
# Describe the dependency shape explicitly — the LLM analysis picks the
# pattern, there is no flag to force one
paladin muster --task "Workflow where step B depends on step A, and step C depends on step B"
```

#### Issue: Configuration doesn't match expectations

**Solution:**
```bash
# Use the review step to reject and regenerate
paladin muster --task "..."
# Answer "n" at the "Accept this configuration?" prompt, then retry with a clearer task

# Or iterate manually
paladin muster --output v1.yaml --task "..."
# Edit v1.yaml as needed
paladin battalion run -c v1.yaml  # Test
paladin muster --output v2.yaml --task "improved description"
```

#### Issue: LLM provider errors

**Solution:**
```bash
# Check API keys and provider configuration
paladin setup-check

# Try a different provider
paladin muster --task "..." --provider deepseek

# Simplify the task description
paladin muster --task "simplified version of workflow"
```

### Getting Help

```bash
# View all muster options
paladin-cli muster --help

# Check provider status
paladin setup-check

# Enable verbose output for debugging
paladin muster --task "..." --verbose
```

## Advanced Usage

### Custom System Prompts

While `muster` generates system prompts, you can provide hints in the task description:

```bash
paladin muster --task "
Code review workflow:
- Use technical, professional tone
- Focus on security and performance
- Provide actionable feedback
"
```

### Resource Requirements

Specify computational constraints in the task description:

```bash
paladin muster --task "
Fast processing workflow:
- Each step should complete in under 5 seconds
- Use lighter models (gpt-3.5-turbo)
- Minimize agent loops
"
```

### Integration with Existing Configs

```bash
# Generate a new component
paladin muster --output retry_component.yaml --task "Add retry logic component"

# Manually integrate into existing config
# Or use as reference for manual updates
```

## See Also

- [CLI Usage Guide](cli-usage.md) - Overview of all CLI commands
- [Battalion Documentation](../user-guides/battalion-patterns.md) - Understanding orchestration patterns
- [Paladin Configuration](../getting-started/quickstart.md) - Manual configuration guide
- [Council Command](cli-council.md) - Quick group discussions
- [Examples Directory](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples) - Sample configurations

## Support

- **Issues**: Report bugs at https://github.com/DF3NDR/paladin-dev-env/issues
- **Discussions**: Ask questions in GitHub Discussions
- **Documentation**: Full docs at https://paladin-ai.dev

---

*Generated configurations should be reviewed before production use. Always test with sample inputs first.*
