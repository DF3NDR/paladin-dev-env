# Paladin CLI Usage Guide

Complete guide to using the Paladin command-line interface for running AI agents and multi-agent battalions.

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Environment Setup](#environment-setup)
- [Getting Started](#getting-started)
  - [paladin onboarding](#paladin-onboarding)
  - [paladin setup-check](#paladin-setup-check)
  - [paladin features](#paladin-features)
- [Commands Reference](#commands-reference)
  - [paladin agent](#paladin-agent)
  - [paladin battalion](#paladin-battalion)
  - [paladin muster](#paladin-muster)
  - [paladin council](#paladin-council)
  - [paladin maneuver](#paladin-maneuver)
  - [paladin arsenal](#paladin-arsenal)
- [Configuration Files](#configuration-files)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

> 📖 **For comprehensive configuration documentation**, see the [CLI Configuration Guide](cli-configuration.md) - covers garrison (memory), arsenal (tools), and scheduler configuration with complete examples.

## Quick Start

```bash
# 1. Run the interactive onboarding wizard
paladin onboarding

# 2. Verify your setup
paladin setup-check

# 3. Discover available features
paladin features

# 4. Generate a battalion configuration using AI
paladin muster --task "Analyze market trends and generate a report"

# 5. Start a quick group discussion
paladin council --topic "Best practices for AI agent design"
```

## Quick Start (Manual Setup)

```bash
# 1. Set your API key
export OPENAI_API_KEY="sk-..."

# 2. Generate a Paladin template
paladin agent new -n my-agent -o my-agent.yaml

# 3. Edit the template (customize system_prompt, etc.)
vim my-agent.yaml

# 4. Run your Paladin
paladin agent run -c my-agent.yaml -i "Hello, Paladin!"
```

## Installation

The `paladin-cli` binary carries `required-features = ["cli"]` and is not produced by a default
`cargo build`; the `cli` feature must be passed explicitly.

```bash
# Build from source
cargo build --release --features cli --bin paladin-cli

# Binary will be at: target/release/paladin-cli

# Add to PATH (optional)
sudo ln -s $(pwd)/target/release/paladin-cli /usr/local/bin/paladin
```

### Command Overview

The top-level `--help` output lists every subcommand and the two global flags:

```text
$ paladin-cli --help
Paladin Multi-Agent Orchestration CLI

Usage: paladin-cli [OPTIONS] <COMMAND>

Commands:
  agent        Paladin agent operations (create, run)
  battalion    Battalion multi-agent operations (create, run)
  arsenal      Arsenal tool management (list, test)
  maneuver     Maneuver flow DSL operations (visualize, validate, execute)
  onboarding   Interactive onboarding wizard for initial setup
  setup-check  Check environment setup and configuration
  features     Discover available features and commands
  muster       Generate battalion configuration from task description
  eval         Evaluation harness operations (run scripted scenarios)
  graph        Graph document operations (export to Mermaid/DOT)
  run          Run/thread execution overlay operations (export to Mermaid)
  council      Run a council discussion
  help         Print this message or the help of the given subcommand(s)

Options:
      --quiet    Enable quiet mode (minimal output)
      --verbose  Enable verbose mode (detailed output)
  -h, --help     Print help
  -V, --version  Print version
```

## Environment Setup

### Required: API Keys

Set the appropriate environment variable for your chosen LLM provider:

```bash
# OpenAI
export OPENAI_API_KEY="sk-..."

# DeepSeek
export DEEPSEEK_API_KEY="sk-..."

# Anthropic
export ANTHROPIC_API_KEY="sk-..."
```

### Optional: MCP Servers

For external tool access (Arsenal), install MCP servers:

```bash
# Web search capability
pip install mcp-web-search

# Or use npx for Node-based servers
npx -y @modelcontextprotocol/server-filesystem /path/to/dir
```

---

## Getting Started

New to Paladin? Start here with these helpful commands.

### paladin onboarding

Interactive wizard to set up your Paladin environment.

**Syntax:**
```bash
paladin onboarding
```

**What it does:**
1. Welcomes you and explains Paladin capabilities
2. Guides you through provider selection (OpenAI, Anthropic, DeepSeek)
3. Validates your API keys with real connectivity tests
4. Creates/updates your `.env` file with secure configuration
5. Generates sample configuration files for quick start
6. Provides next steps and resources

**Examples:**
```bash
# Run the interactive onboarding wizard
paladin onboarding

# The wizard will guide you through:
# ✓ Provider selection
# ✓ API key input (with secure masking)
# ✓ Connectivity validation
# ✓ Environment file creation
# ✓ Sample config generation
```

**Features:**
- ✅ Secure API key input with masking
- ✅ Real-time validation with actual API calls
- ✅ Intelligent `.env` file merging (no duplicates)
- ✅ Resumable state (interruption-safe)
- ✅ Sample configuration generation

**See also:** [Onboarding Guide](cli-onboarding.md)

---

### paladin setup-check

Validate your Paladin installation and environment configuration.

**Syntax:**
```bash
paladin setup-check [OPTIONS]
```

**Options:**
- `--verbose` - Show detailed version strings and response times
- `--quiet` - Minimal output, only show failures

**What it checks:**
1. **System**: Paladin CLI version, Rust toolchain version
2. **Environment**: .env file existence, API key configuration
3. **Providers**: OpenAI, Anthropic, DeepSeek connectivity
4. **Services** (optional): Redis, Qdrant availability

**Examples:**
```bash
# Basic check with summary
paladin setup-check

# Detailed check with timing information
paladin setup-check --verbose

# Quiet mode (CI-friendly)
paladin setup-check --quiet
```

**Exit codes:**
- `0` - All checks passed
- `1` - Critical failures detected
- `2` - Warnings present (non-critical)

**Sample output:**
```
=== Paladin Setup Check ===

System:
  ✓ Paladin CLI: v0.1.0
  ✓ Rust Toolchain: 1.88.0

Environment:
  ✓ .env file: Found
  ⚠ OPENAI_API_KEY: Configured but not validated

Providers:
  ✓ OpenAI: Connected (gpt-4, gpt-3.5-turbo) [342ms]
  ✗ Anthropic: API key not configured
  ⚠ DeepSeek: Connection timeout

Services (Optional):
  ✓ Redis: Connected
  - Qdrant: Not configured

=== Summary ===
✓ 5 passed
⚠ 2 warnings
✗ 1 failed

Next Steps:
  • Configure ANTHROPIC_API_KEY in .env
  • Check DeepSeek API endpoint connectivity
```

**See also:** [Setup Check Guide](cli-setup-check.md)

---

### paladin features

Discover available Paladin features and capabilities.

**Syntax:**
```bash
paladin features [OPTIONS]
```

**Options:**
- `--category <CATEGORY>` - Filter by category
  - Valid values: `agent`, `battalion`, `orchestration`, `memory`, `utilities`
- `--format <FORMAT>` - Output format (default: table)
  - Valid values: `table`, `json`

**Examples:**
```bash
# List all features
paladin features

# Show only battalion patterns
paladin features --category battalion

# Show orchestration patterns
paladin features --category orchestration

# JSON output for scripting
paladin features --format json
```

**Sample output:**
```
=== Paladin Features ===

Agent:
  • Basic Paladin         - Single autonomous AI agent
  • Autonomous Planning   - Self-directed task planning
  • Tool Integration      - External tool access via Arsenal

Battalion:
  • Formation            - Sequential agent execution
  • Phalanx              - Parallel agent execution
  • Campaign             - DAG-based workflow orchestration
  • Chain of Command     - Hierarchical delegation

Orchestration:
  • Conclave             - Expert panel discussions
  • Council              - Quick group discussions
  • Grove                - Dynamic routing patterns
  • Maneuver             - Flow-based orchestration

Memory:
  • In-Memory Garrison   - Fast, non-persistent memory
  • Persistent Garrison  - SQLite-backed memory
  • Sanctum (RAG)        - Vector-based retrieval

[24 features total]
```

**See also:** [Architecture Documentation](../architecture/overview.md)

---

## Commands Reference

### paladin agent

Manage and run individual Paladin agents.

#### `paladin agent new`

Generate a new Paladin configuration template.

**Syntax:**
```bash
paladin agent new -n <name> -o <output> [-p <provider>]
```

**Options:**
- `-n, --name <NAME>` - Paladin name (required)
- `-o, --output <PATH>` - Output file path (required)
- `-p, --provider <PROVIDER>` - LLM provider (optional, default: openai)
  - Valid values: `openai`, `deepseek`, `anthropic`

**Examples:**
```bash
# Basic template with OpenAI
paladin agent new -n MyAgent -o agent.yaml

# DeepSeek template
paladin agent new -n DeepAgent -o deepseek-agent.yaml -p deepseek

# Anthropic template
paladin agent new -n ClaudeAgent -o claude-agent.yaml -p anthropic
```

#### `paladin agent run`

Execute a Paladin from a configuration file.

**Syntax:**
```bash
paladin agent run -c <config> [-i <input>] [-o <output>] [-v]
```

**Options:**
- `-c, --config <PATH>` - Configuration file path (required)
- `-i, --input <TEXT>` - Input text (optional, prompts if omitted)
- `-o, --output <PATH>` - Save JSON output to file (optional)
- `-v, --verbose` - Show detailed execution logs (optional)

**Examples:**
```bash
# Run with command-line input
paladin agent run -c agent.yaml -i "What is Rust?"

# Interactive mode (prompts for input)
paladin agent run -c agent.yaml

# With verbose output
paladin agent run -c agent.yaml -i "Query" --verbose

# Save results to file
paladin agent run -c agent.yaml -i "Query" -o result.json
```

---

### paladin battalion

Manage and run multi-agent battalions.

#### `paladin battalion new`

Generate a new Battalion configuration template.

**Syntax:**
```bash
paladin battalion new -n <name> -t <type> -o <output>
```

**Options:**
- `-n, --name <NAME>` - Battalion name (required)
- `-t, --type <TYPE>` - Battalion type (required)
  - `formation` - Sequential execution (pipeline)
  - `phalanx` - Parallel execution (concurrent)
  - `campaign` - DAG workflow (complex dependencies)
  - `chain-of-command` - Hierarchical delegation
- `-o, --output <PATH>` - Output file path (required)

**Examples:**
```bash
# Formation (sequential)
paladin battalion new -n MyFormation -t formation -o formation.yaml

# Phalanx (parallel)
paladin battalion new -n MyPhalanx -t phalanx -o phalanx.yaml

# Campaign (DAG)
paladin battalion new -n MyCampaign -t campaign -o campaign.yaml

# Chain of Command (hierarchical)
paladin battalion new -n MyTeam -t chain-of-command -o team.yaml
```

#### `paladin battalion run`

Execute a Battalion from a configuration file.

**Syntax:**
```bash
paladin battalion run -c <config> -t <type> [-o <output>] [-v]
```

**Options:**
- `-c, --config <PATH>` - Configuration file path (required)
- `-t, --type <TYPE>` - Battalion type; must match the type in the config file (required)
- `-o, --output <PATH>` - Save JSON output to file (optional)
- `-v, --verbose` - Show detailed execution logs (optional)

**Examples:**
```bash
# Run formation
paladin battalion run -c formation.yaml -t formation

# Run phalanx with verbose output
paladin battalion run -c phalanx.yaml -t phalanx --verbose

# Run campaign and save results
paladin battalion run -c campaign.yaml -t campaign -o results.json
```

---

### paladin muster

Generate battalion configurations using AI-powered task analysis.

**Syntax:**
```bash
paladin muster [OPTIONS]
```

**Options:**
- `--task <DESCRIPTION>` - Task description (prompts if omitted)
- `-o, --output <PATH>` - Output file path (default: muster_<name>_<timestamp>.yaml)
- `--provider <PROVIDER>` - LLM provider for analysis (default: openai)
  - Valid values: `openai`, `deepseek`, `anthropic`
- `--model <MODEL>` - Specific model to use (optional)
- `--no-review` - Skip interactive review (non-interactive mode)
- `--execute` - Run the generated battalion immediately (experimental)

**What it does:**
1. Analyzes your task description using LLM
2. Recommends appropriate battalion pattern (Formation, Phalanx, Campaign, etc.)
3. Generates agent roles and system prompts
4. Creates complete YAML configuration
5. Allows interactive review and editing
6. Saves configuration to file

**Examples:**
```bash
# Interactive mode (wizard)
paladin muster

# With task description
paladin muster --task "Analyze market trends and generate investment report"

# Custom output path
paladin muster --task "Code review workflow" -o code-review.yaml

# Non-interactive mode (for scripting)
paladin muster --task "Data pipeline" --no-review -o pipeline.yaml

# Use specific provider and model
paladin muster --task "Research summary" --provider anthropic --model claude-3-opus
```

**Task Examples:**
```
"Research competitive landscape and create comparison report"
→ Recommends: Formation (researcher -> analyzer -> writer)

"Review pull request from multiple perspectives"
→ Recommends: Phalanx (code_quality, security, performance in parallel)

"Complex data processing with conditional steps"
→ Recommends: Campaign (DAG with dependencies)

"Multi-step decision making with oversight"
→ Recommends: Chain of Command (analysts -> supervisor)
```

**Fallback Mode:**
If LLM is unavailable, muster uses template-based fallback with keyword matching:
- Sequential keywords (then, after, next) → Formation
- Parallel keywords (multiple, compare, simultaneously) → Phalanx
- Discussion keywords (discuss, consensus, perspectives) → Council
- Default → Formation (safe fallback)

**See also:** [Muster Guide](cli-muster.md)

---

### paladin council

Start a quick multi-agent discussion on a topic.

**Syntax:**
```bash
paladin council [OPTIONS]
```

**Options:**
- `--topic <TOPIC>` - Discussion topic (prompts if omitted)
- `--participants <COUNT>` - Number of participants (default: 3, min: 2, max: 10)
- `--roles <ROLES>` - Custom roles (comma-separated, overrides default assignment)
- `--max-rounds <COUNT>` - Maximum discussion rounds (default: 5)
- `--save <PATH>` - Save transcript to file (markdown format)
- `--model <MODEL>` - LLM model to use (optional)
- `--temperature <TEMP>` - LLM temperature (optional)

**Default Role Assignment:**
- 2 participants: Advocate, Critic
- 3 participants: + Moderator
- 4 participants: + Synthesizer
- 5 participants: + Subject Matter Expert
- 6+ participants: + Expert 2, Expert 3, etc.

**Examples:**
```bash
# Interactive mode (wizard)
paladin council

# With topic
paladin council --topic "Best practices for microservices architecture"

# Custom participant count
paladin council --topic "AI ethics" --participants 5

# Custom roles
paladin council --topic "Product roadmap" --roles "PM,Engineer,Designer,Customer"

# Save transcript
paladin council --topic "Security review" --save security-discussion.md

# Full configuration
paladin council \
  --topic "System design review" \
  --participants 4 \
  --max-rounds 3 \
  --model gpt-4 \
  --temperature 0.8 \
  --save design-review.md
```

**Sample Output:**
```
=== Council Discussion: Best Practices for Microservices ===

Participants: 3
Roles: Advocate, Critic, Moderator

──────────────────────────────────────────
Round 1
──────────────────────────────────────────

[Advocate] (Proponent):
Microservices offer excellent scalability and independent deployment...

[Critic] (Skeptic):
However, the operational complexity increases significantly...

[Moderator] (Facilitator):
Both perspectives raise valid points. Let's explore the trade-offs...

──────────────────────────────────────────
Round 2
──────────────────────────────────────────

[... discussion continues ...]

=== Summary ===

Rounds: 5
Total Contributions: 15

Key Points:
• Scalability benefits clear for large teams
• Operational overhead requires investment
• Event-driven patterns recommended

Consensus:
Start with monolith, extract services as needed

Conclusion:
The council recommends a pragmatic approach: begin with a well-structured
monolith and extract microservices only when clear boundaries emerge.
```

**Transcript Format** (when using --save):
```markdown
# Council Discussion: [Topic]

**Started:** 2026-02-09 10:30:00  
**Ended:** 2026-02-09 10:45:00  
**Participants:** 3

## Participants

- **Alice** - Advocate (Proponent)
- **Bob** - Critic (Skeptic)
- **Carol** - Moderator (Facilitator)

## Discussion

### Round 1

**Alice** (Advocate): [message]
**Bob** (Critic): [message]
**Carol** (Moderator): [message]

### Round 2

[... continues ...]

## Summary

[Summary content]
```

**See also:** [Council Guide](cli-council.md), [Conclave Documentation](council.md)

---

### paladin maneuver

Visualize and validate Flow DSL orchestration patterns.

#### `paladin maneuver visualize`

Generate visual representation of a Maneuver flow expression.

**Syntax:**
```bash
paladin maneuver visualize -c <config> [-f <format>] [-o <output>]
```

**Options:**
- `-c, --config <PATH>` - Path to Maneuver YAML configuration (required)
- `-f, --format <FORMAT>` - Output format (optional, default: ascii)
  - `ascii` - ASCII tree visualization for terminal
  - `mermaid` - Mermaid.js flowchart for documentation
- `-o, --output <PATH>` - Save output to file instead of stdout (optional)

**Examples:**
```bash
# ASCII tree visualization (terminal-friendly)
paladin maneuver visualize -c workflow.yaml

# Output example:
# └─> intake
#     ├─> [PARALLEL]
#     │   ├─> technical
#     │   ├─> business
#     │   └─> security
#     └─> synthesis

# Mermaid flowchart (for documentation)
paladin maneuver visualize -c workflow.yaml --format mermaid

# Save to file
paladin maneuver visualize -c workflow.yaml -f ascii -o flow.txt
```

#### `paladin maneuver validate`

Validate a Maneuver configuration for syntax and structure errors.

**Syntax:**
```bash
paladin maneuver validate -c <config> [-v]
```

**Options:**
- `-c, --config <PATH>` - Path to Maneuver YAML configuration (required)
- `-v, --verbose` - Show detailed validation output (optional)

**Validation Checks:**
- Flow expression syntax correctness
- All agents referenced in flow exist in configuration
- Agent configuration structure validity
- Provider settings correctness

**Examples:**
```bash
# Basic validation
paladin maneuver validate -c workflow.yaml

# Verbose validation with detailed output
paladin maneuver validate -c workflow.yaml --verbose
```

**Output (Success):**
```
✅ Flow syntax valid: intake -> (technical, business, security) -> synthesis
✅ All agents referenced in flow are configured
✅ Configuration structure valid
✅ 5 agents configured: intake, technical, business, security, synthesis
```

**Output (Error):**
```
❌ Flow syntax error at position 23: unexpected character '|'
   Expected: '->' or ',' for flow operators

❌ Agent 'reviewer' referenced in flow but not found in configuration
   Flow agents: [intake, technical, business, reviewer]
   Configured: [intake, technical, business]
```

---

### paladin arsenal

Manage and test external tools (MCP servers).

#### `paladin arsenal list`

List all configured MCP servers and their tools.

**Syntax:**
```bash
paladin arsenal list
```

**Example:**
```bash
paladin arsenal list

# Output:
# Tool Name       | Description          | Type   | Status
# ────────────────┼──────────────────────┼────────┼─────────
# web_search      | Search the web       | stdio  | ✓ Connected
# filesystem      | File operations      | stdio  | ✓ Connected
```

#### `paladin arsenal test`

Test connection to an MCP server.

**Syntax:**
```bash
paladin arsenal test --mcp-stdio <command>
paladin arsenal test --mcp-streamable-http <url> [--mcp-auth-token-env <ENV_VAR_NAME>]
```

**Options:**
- `--mcp-stdio <COMMAND>` - Test STDIO MCP server (mutually exclusive with `--mcp-streamable-http`)
- `--mcp-streamable-http <URL>` - Test a Streamable-HTTP MCP server (mutually exclusive with
  `--mcp-stdio`; renamed from the retired `--mcp-sse` flag, which pointed at a mislabeled
  plain-HTTP-POST adapter that was never real SSE or Streamable-HTTP)
- `--mcp-auth-token-env <ENV_VAR_NAME>` - NAMES the environment variable holding the bearer
  token for `--mcp-streamable-http` (optional; requires `--mcp-streamable-http`). The token
  itself is never accepted as a CLI argument and never logged.

**Examples:**
```bash
# Test STDIO server
paladin arsenal test --mcp-stdio "uvx mcp-web-search"

# Test an unauthenticated Streamable-HTTP server
paladin arsenal test --mcp-streamable-http "http://localhost:3000/mcp"

# Test an authenticated Streamable-HTTP server (token sourced from the named env var)
export ETHERSCAN_API_KEY="..."
paladin arsenal test --mcp-streamable-http "https://mcp.etherscan.io/mcp" \
    --mcp-auth-token-env ETHERSCAN_API_KEY

# With full command and args
paladin arsenal test --mcp-stdio "npx -y @modelcontextprotocol/server-filesystem /tmp"
```

---

## Configuration Files

### Paladin Configuration Schema

```yaml
# Identity
name: "PaladinName"
user_name: "UserName"

# System prompt (most important!)
system_prompt: |
  Define the Paladin's role, capabilities, and behavior here.

# LLM settings
model: "gpt-4"
temperature: 0.7
max_loops: 3
timeout_seconds: 300
stop_words: ["STOP"]

# Provider
provider:
  type: openai  # or deepseek, anthropic

# Optional: Memory
garrison:
  type: sqlite
  path: ./garrison.db
  max_entries: 1000

# Optional: Tools
arsenal:
  mcp_servers:
    - name: web_search
      type: stdio
      command: uvx
      args: [mcp-web-search]
```

### Battalion Configuration Schema

**Formation (Sequential):**
```yaml
type: formation
name: "FormationName"
pass_output_to_next: true
paladins:
  - inline: { ... paladin config ... }
  - inline: { ... paladin config ... }
```

**Phalanx (Parallel):**
```yaml
type: phalanx
name: "PhalanxName"
paladins:
  - inline: { ... paladin config ... }
  - inline: { ... paladin config ... }
inputs: []  # Optional: different input for each
```

**Campaign (DAG):**
```yaml
type: campaign
name: "CampaignName"
nodes:
  - id: node1
    paladin: { inline: { ... } }
  - id: node2
    paladin: { inline: { ... } }
edges:
  - from: node1
    to: node2
start_node: node1
```

**Chain of Command (Hierarchical):**
```yaml
type: chain_of_command
name: "TeamName"
commander:
  inline: { ... paladin config ... }
delegates:
  - inline: { ... paladin config ... }
  - inline: { ... paladin config ... }
```

---

## Examples

### Example 1: Simple Q&A Agent

```bash
# 1. Create config
cat > qa-agent.yaml << 'EOF'
name: "QAAgent"
system_prompt: "You are a helpful Q&A assistant."
model: "gpt-4"
temperature: 0.7
max_loops: 1
provider: { type: openai }
EOF

# 2. Run
export OPENAI_API_KEY="sk-..."
paladin agent run -c qa-agent.yaml -i "What is Rust?"
```

### Example 2: Multi-Stage Analysis

```bash
# 1. Generate formation template
paladin battalion new -n Analysis -t formation -o analysis.yaml

# 2. Edit to add analyzer → summarizer → validator stages

# 3. Run
paladin battalion run -c analysis.yaml -t formation
```

### Example 3: Agent with Web Search

```bash
# 1. Install MCP web search
pip install mcp-web-search

# 2. Create config with arsenal
cat > web-agent.yaml << 'EOF'
name: "WebAgent"
system_prompt: "You can search the web for current information."
model: "gpt-4"
temperature: 0.7
max_loops: 3
provider: { type: openai }
arsenal:
  mcp_servers:
    - name: web_search
      type: stdio
      command: uvx
      args: [mcp-web-search]
EOF

# 3. Run
paladin agent run -c web-agent.yaml -i "Latest AI news"
```

---

## Troubleshooting

### Common Errors

#### Error: "Missing API key"

**Problem:** Required environment variable not set.

**Solution:**
```bash
export OPENAI_API_KEY="sk-..."
# Or for other providers:
export DEEPSEEK_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-..."
```

#### Error: "Config file not found"

**Problem:** Path to configuration file is incorrect.

**Solution:**
- Use absolute paths: `/full/path/to/config.yaml`
- Or relative from current directory: `./config.yaml`
- Check file exists: `ls -l config.yaml`

#### Error: "Invalid YAML"

**Problem:** Syntax error in configuration file.

**Solution:**
- Validate YAML online: https://www.yamllint.com/
- Check indentation (use spaces, not tabs)
- Ensure all strings with special characters are quoted
- Use `yamllint config.yaml` if available

#### Error: "Invalid provider"

**Problem:** Provider type not recognized.

**Solution:**
- Valid providers: `openai`, `deepseek`, `anthropic`
- Check spelling in config file
- Use `paladin agent new -p <provider>` to generate correct template

#### Error: "MCP server connection failed"

**Problem:** Cannot connect to MCP server.

**Solution:**
- Verify server is installed: `which uvx`, `which npx`
- Test server manually: `uvx mcp-web-search`
- Check command and args in config
- Ensure server supports MCP protocol
- Review server logs in stderr

#### Error: "Timeout"

**Problem:** Execution exceeded configured timeout.

**Solution:**
- Increase `timeout_seconds` in config
- Reduce `max_loops` for simpler tasks
- Check if LLM API is responding slowly
- Verify network connectivity

#### Error: "Rate limit exceeded"

**Problem:** Too many API requests to LLM provider.

**Solution:**
- Wait and retry
- Use `--verbose` to see which call failed
- Consider using cheaper model for testing
- Check provider's rate limits
- Add delays between requests

### Getting Help

- **Documentation:** See `examples/cli_configs/` for working examples
- **Issues:** Report bugs at https://github.com/DF3NDR/paladin-dev-env/issues
- **Verbose Mode:** Use `--verbose` flag to see detailed execution logs
- **Logs:** Check stderr output for detailed error messages

### Performance Tips

1. **Model Selection:**
   - Use `gpt-3.5-turbo` for simple tasks (faster, cheaper)
   - Use `gpt-4` for complex reasoning
   - Use `deepseek-chat` for cost-effective alternative

2. **Temperature:**
   - Lower (0.0-0.3) for factual, consistent outputs
   - Medium (0.4-0.7) for balanced responses
   - Higher (0.8-1.0) for creative, varied outputs

3. **Max Loops:**
   - 1-2: Simple single-response tasks
   - 3-5: Default for most tasks
   - 6+: Complex multi-step reasoning

4. **Timeouts:**
   - 60s: Simple queries
   - 180-300s: Standard tasks
   - 600s+: Complex multi-step operations

5. **Battalions:**
   - Use Phalanx for parallel speedup
   - Use Formation for sequential pipelines
   - Monitor costs with `--verbose`

---

## Advanced Topics

### External Configuration References

Instead of inline Paladin configs, reference external files:

```yaml
paladins:
  - file: ./agents/analyzer.yaml
  - file: ./agents/summarizer.yaml
```

### Environment Variable Substitution

Use environment variables in configs:

```yaml
provider:
  api_key_env: "${CUSTOM_API_KEY_VAR}"
```

### Custom MCP Servers

Create your own tools:
- Implement MCP protocol
- Register in arsenal configuration
- See MCP documentation: https://modelcontextprotocol.io/

### Streaming Responses

For real-time output (coming soon):
```bash
paladin agent run -c config.yaml -i "Query" --stream
```

---

## See Also

### Documentation
- [CLI Configuration Guide](cli-configuration.md) - Complete reference for garrison, arsenal, and scheduler configuration
- [CLI Testing Guide](../contributing/testing-guide.md) - Guide for testing CLI commands
- [Main README](../introduction.md)

### Configuration Examples
- [Basic Paladin Example](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples/cli_configs)
- [Advanced Paladin Example](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples/cli_configs)
- [Formation Example](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples/cli_configs)
- [Phalanx Example](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples/cli_configs)
- [Campaign Example](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples/cli_configs)
- [Chain of Command Example](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples/cli_configs)
