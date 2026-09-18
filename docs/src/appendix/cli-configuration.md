# Paladin CLI Configuration Guide

Comprehensive guide to configuring Paladin agents through YAML configuration files.

## Table of Contents

- [Overview](#overview)
- [Configuration File Structure](#configuration-file-structure)
- [Garrison Configuration (Memory)](#garrison-configuration-memory)
- [Arsenal Configuration (Tools)](#arsenal-configuration-tools)
- [Scheduler Configuration](#scheduler-configuration)
- [Complete Configuration Examples](#complete-configuration-examples)
- [Environment Variables](#environment-variables)
- [Troubleshooting](#troubleshooting)

## Overview

Paladin agents can be configured entirely through YAML files, enabling:

- **Reproducible deployments**: Version-control your agent configurations
- **Complex orchestration**: Configure multi-agent battalions with memory and tools
- **Environment-specific settings**: Use environment variables for sensitive data
- **Testing and CI/CD**: Run agents with mock providers and predictable configurations

## Configuration File Structure

Basic Paladin YAML configuration:

```yaml
name: "my-agent"
system_prompt: "You are a helpful AI assistant."
llm:
  provider: "openai"
  model: "gpt-4"
  temperature: 0.7
max_loops: 3
user_name: "User"
stop_words:
  - "TERMINATE"
  - "DONE"
```

## Garrison Configuration (Memory)

Garrison provides memory capabilities to Paladins, enabling context retention across interactions.

### In-Memory Garrison

Fast, non-persistent memory suitable for single-session use:

```yaml
garrison:
  type: "in_memory"
  max_entries: 1000
```

**Configuration Options:**
- `type`: Must be `"in_memory"`
- `max_entries`: Maximum number of memory entries (default: 1000)

**Use cases:**
- Development and testing
- Short-lived agent sessions
- When persistence is not required

### SQLite Garrison

Persistent memory backed by SQLite database:

```yaml
garrison:
  type: "sqlite"
  path: "./data/agent_memory.db"
  max_entries: 10000
  ttl_seconds: 86400  # 24 hours
```

**Configuration Options:**
- `type`: Must be `"sqlite"`
- `path`: Database file path (will be created if it doesn't exist)
- `max_entries`: Maximum number of entries before cleanup (default: 10000)
- `ttl_seconds`: Entry time-to-live in seconds (optional, default: no expiration)

**Use cases:**
- Production deployments
- Long-running agents with conversation history
- Multi-session context retention

### Memory Operations

When garrison is configured, Paladins automatically:

1. **Store interactions**: Each LLM call and response is recorded
2. **Retrieve context**: Recent interactions are included in prompts
3. **Semantic search**: Find relevant past interactions (future enhancement)

## Arsenal Configuration (Tools)

Arsenal enables Paladins to access external tools via the Model Context Protocol (MCP).

### MCP STDIO Servers

Connect to command-line MCP servers:

```yaml
arsenal:
  mcp_servers:
    - name: "web_search"
      type: "stdio"
      command: "uvx"
      args:
        - "mcp-web-search"

    - name: "filesystem"
      type: "stdio"
      command: "node"
      args:
        - "/path/to/mcp-server-filesystem"
        - "--root"
        - "/workspace"
```

**Configuration Options:**
- `name`: Unique identifier for the tool server
- `type`: Must be `"stdio"`
- `command`: Executable command (e.g., `uvx`, `node`, `python`)
- `args`: Command-line arguments as a list

### MCP Streamable-HTTP Servers

Connect to remote, optionally authenticated MCP servers over HTTP(S) (the real,
currently-implemented remote transport, D-02/D-03 — supersedes the retired `"sse"` type,
which was never actually SSE and now fails loud with a migration message):

```yaml
arsenal:
  mcp_servers:
    - name: "api_tools"
      type: "streamable_http"
      endpoint: "https://api.example.com/mcp"
      auth_token_env: "MCP_API_TOKEN"
```

**Configuration Options:**
- `name`: Unique identifier for the tool server
- `type`: Must be `"streamable_http"`
- `endpoint`: HTTP(S) endpoint for the MCP server
- `auth_token_env`: NAMES the environment variable holding the bearer token — an env-var
  REFERENCE, never a literal secret in this file. Resolved host-side at connect time and
  never serialized back out. Omit entirely for an unauthenticated server.

### Tool Discovery and Registration

When arsenal is configured:

1. **Auto-discovery**: All MCP servers are queried for available tools
2. **Registration**: Tools are registered in the arsenal registry
3. **LLM integration**: Tool schemas are included in LLM system prompts
4. **Invocation**: Paladins can call tools by name with JSON arguments

### Available MCP Servers

Popular MCP servers you can integrate:

- **mcp-web-search**: Web search capabilities (Brave, Google)
- **mcp-server-filesystem**: File system operations
- **mcp-server-git**: Git repository operations
- **mcp-server-brave-search**: Brave search API
- **mcp-server-slack**: Slack workspace integration
- **mcp-server-github**: GitHub API access

See [MCP Server Directory](https://github.com/modelcontextprotocol/servers) for more.

## Scheduler Configuration

Configure scheduled task execution for async operations:

```yaml
scheduler:
  enabled: true
  default_cron: "0 0 * * *"  # Daily at midnight
  channel_size: 100
```

**Configuration Options:**
- `enabled`: Enable/disable scheduler (default: `false`)
- `default_cron`: Default cron expression for scheduled tasks
- `channel_size`: Task queue channel size (default: 100)

**Cron Expression Examples:**
```
"0 * * * *"      # Every hour
"0 0 * * *"      # Daily at midnight
"0 0 * * 1"      # Weekly on Monday
"*/15 * * * *"   # Every 15 minutes
"0 9-17 * * *"   # Hourly between 9 AM and 5 PM
```

**Use cases:**
- Scheduled content delivery
- Periodic agent execution
- Batch processing workflows

## Complete Configuration Examples

### Example 1: Basic Paladin with Memory

```yaml
name: "research-assistant"
system_prompt: |
  You are a research assistant that helps users find and analyze information.
  You have access to web search tools and maintain conversation context.

llm:
  provider: "openai"
  model: "gpt-4"
  temperature: 0.7

max_loops: 5
user_name: "Researcher"

garrison:
  type: "sqlite"
  path: "./data/research_memory.db"
  max_entries: 5000
  ttl_seconds: 604800  # 7 days
```

### Example 2: Paladin with Tools and Memory

```yaml
name: "developer-assistant"
system_prompt: |
  You are a software development assistant with access to code search,
  file system operations, and Git commands. Use tools to help users
  with coding tasks.

llm:
  provider: "openai"
  model: "gpt-4"
  temperature: 0.5

max_loops: 10
user_name: "Developer"

garrison:
  type: "sqlite"
  path: "./data/dev_memory.db"
  max_entries: 10000

arsenal:
  mcp_servers:
    - name: "filesystem"
      type: "stdio"
      command: "node"
      args:
        - "/usr/local/lib/mcp-server-filesystem"
        - "--root"
        - "${WORKSPACE_DIR}"

    - name: "git"
      type: "stdio"
      command: "node"
      args:
        - "/usr/local/lib/mcp-server-git"

    - name: "web_search"
      type: "stdio"
      command: "uvx"
      args:
        - "mcp-web-search"
        - "--brave-api-key"
        - "${BRAVE_API_KEY}"
```

### Example 3: Full-Featured Configuration

```yaml
name: "production-agent"
system_prompt: |
  You are a production AI agent with full capabilities:
  - Persistent memory for conversation context
  - Tool access for external operations
  - Scheduled task execution

  Always maintain context across sessions and use tools when appropriate.

llm:
  provider: "openai"
  model: "gpt-4"
  temperature: 0.7

max_loops: 5
user_name: "User"
stop_words:
  - "TERMINATE"
  - "TASK_COMPLETE"

garrison:
  type: "sqlite"
  path: "/var/lib/paladin/memory/agent.db"
  max_entries: 50000
  ttl_seconds: 2592000  # 30 days

arsenal:
  mcp_servers:
    - name: "web_search"
      type: "stdio"
      command: "uvx"
      args:
        - "mcp-web-search"

    - name: "slack"
      type: "stdio"
      command: "node"
      args:
        - "/opt/mcp-server-slack"
        - "--workspace"
        - "${SLACK_WORKSPACE_ID}"
        - "--token"
        - "${SLACK_BOT_TOKEN}"

    - name: "api_tools"
      type: "streamable_http"
      endpoint: "https://api.company.com/mcp"
      auth_token_env: "COMPANY_API_TOKEN"

scheduler:
  enabled: true
  default_cron: "0 */6 * * *"  # Every 6 hours
  channel_size: 200
```

## Environment Variables

### LLM Provider Keys

```bash
# OpenAI
export OPENAI_API_KEY="sk-..."

# DeepSeek
export DEEPSEEK_API_KEY="..."

# Anthropic
export ANTHROPIC_API_KEY="..."
```

### Tool Authentication

```bash
# Brave Search
export BRAVE_API_KEY="..."

# Slack
export SLACK_BOT_TOKEN="xoxb-..."
export SLACK_WORKSPACE_ID="T..."

# Custom APIs
export COMPANY_API_TOKEN="..."
```

### File Paths

```bash
# Use environment variables in configuration
export WORKSPACE_DIR="/home/user/workspace"
export GARRISON_DB_PATH="/var/lib/paladin/memory"
```

### Using Environment Variables in YAML

```yaml
garrison:
  path: "${GARRISON_DB_PATH}/agent.db"

arsenal:
  mcp_servers:
    - name: "api"
      type: "streamable_http"
      endpoint: "${API_SERVER_URL}"
      # auth_token_env is the NAME of an env var, not a "${...}"-interpolated
      # literal -- the bearer token is resolved from the API_TOKEN env var
      # host-side at connect time, never written back to this file.
      auth_token_env: "API_TOKEN"
```

## Troubleshooting

### Garrison Issues

#### SQLite Database Locked

**Symptom:** `SqliteError: database is locked`

**Solutions:**
- Ensure only one Paladin instance accesses the database
- Check file permissions on the database file
- Use WAL mode for concurrent reads (automatic in SQLite garrison)

#### Memory Not Persisting

**Symptom:** Agent doesn't remember previous interactions

**Solutions:**
- Verify garrison type is `"sqlite"`, not `"in_memory"`
- Check database file path is correct and writable
- Verify `ttl_seconds` hasn't expired old entries
- Check that the `agent` command actually built a garrison: it reads `garrison.type` from the
  paladin config and passes it to `instantiate_garrison`
  (`src/application/cli/config/loader.rs`), which constructs the `GarrisonPort` implementation
  (`InMemoryGarrison` or the SQLite-backed adapter) and hands it to
  `PaladinExecutionService::new`. If `garrison.type` is unset, `instantiate_garrison` returns
  `None` and the Paladin runs without memory — set `garrison.type` in the config to fix that.

### Arsenal Issues

#### Tool Not Found

**Symptom:** `ArsenalError: Tool 'tool_name' not registered`

**Solutions:**
- Verify MCP server configuration is correct
- Check MCP server command is executable: `which <command>`
- Test MCP server independently: run command with `--list-tools` (if supported)
- Check arsenal registry logs for tool discovery errors
- Check that the `agent` command actually built an arsenal: it reads `arsenal.mcp_servers` from
  the paladin config and passes it to `instantiate_arsenal`
  (`src/application/cli/config/loader.rs`), which builds an `ArsenalExecutionService` registered
  against each configured server and hands it to `PaladinExecutionService::new`. An empty or
  missing `arsenal.mcp_servers` list produces an arsenal with no tools registered — verify the
  server's `name` entry appears under `arsenal.mcp_servers` in the config.

#### MCP Server Connection Failed

**Symptom:** `ArsenalError: Failed to connect to MCP server`

**Solutions:**
- For STDIO: Verify command and args are correct
- For STDIO: Check executable is in PATH
- For Streamable-HTTP: Verify the endpoint is reachable: `curl <endpoint>`
- For Streamable-HTTP: Check the `auth_token_env`-named environment variable is set and the
  token is valid (a missing/invalid token surfaces as `ArsenalError::AuthFailed`)
- Review MCP server logs for startup errors

#### Tool Invocation Timeout

**Symptom:** Tool call hangs or times out

**Solutions:**
- Increase timeout in PaladinConfig
- Check MCP server is responding (may be slow external API)
- Verify tool arguments are valid JSON
- Check MCP server logs for errors

### Scheduler Issues

#### Scheduled Tasks Not Executing

**Symptom:** Jobs scheduled but never run

**Solutions:**
- Verify `scheduler.enabled: true` in config
- Check cron expression is valid: use [crontab.guru](https://crontab.guru/)
- Check the schedule's status through the Platform API's `/v1/schedules*` route family (Phase 27)
  — `GET /v1/schedules/{schedule_id}` reports `last_tick`/`next_tick`/`skipped_ticks`, which shows
  whether the tick is firing and whether runs are being skipped because a `fixed_thread` is busy;
  see [Platform API — Schedules](../api-reference/platform-api.md#schedules)
- Review scheduler logs for errors
- Verify `APP_SCHEDULES_ENABLED` and `APP_SCHEDULES_TICK_INTERVAL_MS` are set as expected

#### Invalid Cron Expression

**Symptom:** `SchedulerError: Invalid cron expression`

**Solutions:**
- Use standard cron format: `minute hour day month weekday`
- Test expression at [crontab.guru](https://crontab.guru/)
- Use quotes around cron expressions in YAML
- Common format: `"0 0 * * *"` (daily), `"*/15 * * * *"` (every 15 min)

### Configuration File Errors

#### YAML Parsing Failed

**Symptom:** `ConfigError: Failed to parse YAML`

**Solutions:**
- Validate YAML syntax: `yamllint config.yaml`
- Check indentation (use spaces, not tabs)
- Ensure strings with special characters are quoted
- Verify list syntax uses `- ` prefix

#### Required Field Missing

**Symptom:** `ConfigError: Missing required field 'name'`

**Solutions:**
- Review configuration file structure above
- Ensure all required fields are present:
  - `name`
  - `system_prompt`
  - `llm.provider`
  - `llm.model`

#### Environment Variable Not Resolved

**Symptom:** Configuration contains literal `"${VAR_NAME}"`

**Solutions:**
- Export environment variable before running: `export VAR_NAME=value`
- Check variable name matches exactly (case-sensitive)
- Use quotes in YAML: `api_key: "${OPENAI_API_KEY}"`
- Verify environment variable is set: `echo $VAR_NAME`
- Note: `arsenal.mcp_servers[].auth_token_env` is a different pattern -- it takes the env
  var's NAME as a plain string (e.g. `auth_token_env: "OPENAI_API_KEY"`), not a
  `"${VAR_NAME}"`-interpolated value

### Common Error Messages

| Error | Cause | Solution |
|-------|-------|----------|
| `GarrisonConfigError: Unknown type 'postgres'` | Invalid garrison type | Use `"in_memory"` or `"sqlite"` |
| `ArsenalConfigError: Missing required field 'command'` | STDIO config incomplete | Add `command` and `args` fields |
| `ArsenalConfigError: Missing required field 'endpoint'` | Streamable-HTTP config incomplete | Add `endpoint` field for `streamable_http` type |
| `ArsenalConfigError: ... 'type' 'sse' is deprecated` | Config still uses the retired `sse` type | Change `type` to `"streamable_http"` and rename `url`/`auth_token` to `endpoint`/`auth_token_env` |
| `SchedulerError: Job not found` | Attempting to cancel non-existent job | Check JobId is valid before cancellation |
| `LlmError: API key not found` | Missing environment variable | Set provider API key: `export OPENAI_API_KEY=...` |

### Getting Help

Still having issues? Check:

1. **Logs**: Run with `-v` flag for verbose output
   ```bash
   paladin agent run -c config.yaml -i "test" -v
   ```

2. **Test Configuration**: Use `paladin setup-check` to verify environment

3. **GitHub Issues**: [github.com/DF3NDR/paladin-dev-env/issues](https://github.com/DF3NDR/paladin-dev-env/issues)

4. **Documentation**:
   - [CLI Usage Guide](cli-usage.md)
   - [Testing Guide](../contributing/testing-guide.md)
   - [Architecture Documentation](../architecture/overview.md)

---

**Last updated:** February 14, 2026  
**Epic:** 23 - CLI, Config & Infrastructure Completion
