# Configuration

Paladin is configured via a YAML file (`config.yml` by default) and environment
variables. Environment variables take precedence over file values and use the
`APP_` prefix format shown throughout this guide.

## Loading Configuration

```rust,ignore
// Load from the default config.yml in the current directory
let settings = paladin_ai_core::config::ApplicationSettings::load()?;

// Or specify a path
let settings = paladin_ai_core::config::ApplicationSettings::from_file("config.yml")?;
```

## LLM Provider

Nine providers are supported: `openai`, `anthropic`, `deepseek`, `kimi`, `qwen`, `grok`,
`ollama`, `gemini`, and a generic operator-configured `openai-compatible` provider for any
other OpenAI-compatible endpoint. Only the providers compiled in via the matching
`llm-<provider>` Cargo feature are usable at runtime; the compiled default remains
`openai` + `anthropic` + `deepseek` (see [Feature Flags](../api-reference/feature-flags.md)).

```yaml
llm:
  default_provider: "openai"   # openai | anthropic | deepseek | kimi | qwen | grok | ollama | gemini | openai-compatible

  openai:
    base_url: "https://api.openai.com/v1"
    default_model: "gpt-4"
    default_temperature: 0.7
    timeout_seconds: 300
    max_retries: 3

  deepseek:
    base_url: "https://api.deepseek.com/v1"
    default_model: "deepseek-chat"
    default_temperature: 0.7
    timeout_seconds: 300
    max_retries: 3

  anthropic:
    base_url: "https://api.anthropic.com/v1"
    default_model: "claude-3-5-sonnet-20241022"
    default_temperature: 0.7
    timeout_seconds: 300
    max_retries: 3

  # Six providers added alongside the original three. Each carries its own dated
  # verification status below rather than one blanket disclaimer — see "Live
  # verification status" further down for what was confirmed and when.
  #
  # Kimi (Moonshot AI). Live-verified 2026-08-22 (plan 17-19, closing G-17-4b): GET
  # /models and a generate() round trip both succeeded against api.moonshot.ai.
  kimi:
    base_url: "https://api.moonshot.ai/v1"
    default_model: "kimi-k3"
    timeout_seconds: 60

  # Qwen (Alibaba DashScope). Live-verified 2026-08-23 (plan 17-21 gap closure): GET
  # /models returned a 162-model catalog at the endpoint below, including the default
  # model, and a generate() round trip succeeded.
  #
  # DashScope API keys are scoped to the Model Studio region that issued them and are
  # REJECTED by every other region's endpoint. `base_url` below is Singapore, the
  # shipped default. If your workspace is in the US or on the mainland, you MUST
  # set `DASHSCOPE_BASE_URL` to your own region's endpoint:
  #   - Singapore      (shipped default): https://dashscope-intl.aliyuncs.com/compatible-mode/v1
  #   - US (Virginia):                    https://dashscope-us.aliyuncs.com/compatible-mode/v1
  #   - China (mainland):                 https://dashscope.aliyuncs.com/compatible-mode/v1
  qwen:
    base_url: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1"
    default_model: "qwen-plus"
    timeout_seconds: 60

  # Grok (xAI). Live-verified 2026-08-22 (plan 17-18, closing G-17-4a): GET /models and
  # a generate() round trip both succeeded against api.x.ai.
  grok:
    base_url: "https://api.x.ai/v1"
    default_model: "grok-4.6"
    timeout_seconds: 60

  # Ollama (self-hosted) requires no api_key at all (D-12) — omit the field entirely.
  # Not applicable to live-vendor verification: self-hosted, no vendor endpoint to
  # verify. Its live exercise is the Docker Tier 2 suite (UAT test 3), passed on a
  # GitHub Actions runner 2026-08-19.
  ollama:
    base_url: "http://localhost:11434/v1"
    default_model: "llama3"
    timeout_seconds: 60

  # Gemini uses a bespoke `generateContent` protocol, not OpenAI-compatible.
  # Live-verified: GET /models and a generate() round trip both succeeded against
  # generativelanguage.googleapis.com. default_model was refreshed from
  # gemini-2.5-flash (retired for new users) to gemini-3.6-flash per the live catalog.
  gemini:
    base_url: "https://generativelanguage.googleapis.com/v1beta"
    default_model: "gemini-3.6-flash"
    timeout_seconds: 60

  # Generic adapter for ANY OpenAI-compatible endpoint not named above (self-hosted
  # vLLM/LiteLLM, Groq, Together, Mistral, Fireworks, Bedrock's OpenAI-compat mode, ...).
  # base_url and default_model are REQUIRED here — there is no vendor default.
  openai-compatible:
    base_url: "https://your-endpoint.example.com/v1"
    default_model: "your-model-name"
    timeout_seconds: 60
```

**API keys** are read exclusively from environment variables:

| Variable | Provider |
|----------|----------|
| `OPENAI_API_KEY` | OpenAI |
| `DEEPSEEK_API_KEY` | DeepSeek |
| `ANTHROPIC_API_KEY` | Anthropic |
| `MOONSHOT_API_KEY` | Kimi |
| `DASHSCOPE_API_KEY` | Qwen |
| `XAI_API_KEY` | Grok |
| — (none required) | Ollama — self-hosted, no vendor credential (D-12) |
| `GEMINI_API_KEY` | Gemini |
| `OPENAI_COMPATIBLE_API_KEY` | Generic OpenAI-compatible provider — **not** the same variable as `OPENAI_API_KEY`, a different credential for a different provider; the two names are one word apart, read both character-by-character before exporting either |
| `APP_LLM_DEFAULT_PROVIDER` | Override default provider at runtime |

> **Security:** Never put API keys in `config.yml`. Use environment variables or
> a secrets manager (AWS Secrets Manager, HashiCorp Vault, Kubernetes Secrets).

### Live verification status

Per-vendor, not one blanket disclaimer — each provider's `base_url` and `default_model`
above carry their own dated status:

| Provider | Status |
|---|---|
| Gemini | Live-verified: a model-list fetch and a `generate()` round trip both succeeded. |
| Grok (xAI) | Live-verified 2026-08-22 (plan 17-18): model list + a `generate()` round trip against `api.x.ai`. |
| Kimi (Moonshot) | Live-verified 2026-08-22 (plan 17-19): model list + a `generate()` round trip against `api.moonshot.ai`, including its measured fixed-temperature constraint. |
| Qwen (DashScope) | Live-verified 2026-08-23 (plan 17-21 gap closure): a model-list fetch (162 models at the shipped Singapore endpoint) and a `generate()` round trip both succeeded. See the region-scoping note above `qwen:` for the mandatory override outside the Singapore region. |
| Ollama | Not applicable — self-hosted, no vendor endpoint to verify. Its live exercise is the Docker Tier 2 suite (UAT test 3), passed on a GitHub Actions runner 2026-08-19. |

### Running against a local Ollama server (RT-FR-22)

Ollama is the only provider in the table above that requires no vendor API key (D-12) — it is
also the only one you can run entirely on your own machine. Two commands get a model serving
locally:

```sh
ollama serve                 # starts the local server (default: http://localhost:11434)
ollama pull llama3           # pulls the model named in the Ollama config block above
```

Point Paladin at it with the Ollama configuration block already shown above under
[LLM Provider](#llm-provider) — `base_url` and `default_model` there are exactly what
`ollama serve`/`ollama pull` produce by default. To override the base URL without editing
`config.yml` (a different port, a remote host, a container), set `OLLAMA_BASE_URL`; it follows
the same environment-variable-overrides-file precedence as every other provider on this page.

A minimal `reasoning_agent` preset call against a local Ollama server, once it is running (the
exact signature per `26-CONTEXT.md` D-35 — `paladin::presets` and `ReasoningAgent` do not exist
in the tree yet as of this plan; the runnable, doc-tested version of this snippet arrives with
the `agent-runtime` user guide, plan 26-21):

```rust,ignore
use paladin::presets::{reasoning_agent, ReasoningAgentOptions};

// `llm` is any Arc<dyn LlmPort> -- for example the adapter built from the Ollama
// configuration block above (base_url "http://localhost:11434/v1", model "llama3").
let agent = reasoning_agent(llm, arsenal, ReasoningAgentOptions::default())?;
let result = agent.run("What is the capital of France?").await?;
```

**Verifying the adapter against a real Ollama server is an existing suite, not a new one
(D-32).** `tests/integration/ollama_docker_test.rs` already is the "ignored-by-default
integration test gated on an env var" RT-FR-22 asks for: it is `required-features`-gated on
`integration-tests` + `llm-ollama`, every test in it independently probes `OLLAMA_TEST_URL`
before doing anything else, and it prints a named `SKIP:` reason and returns early — never
panics or hangs — when the service is unreachable. Run it with:

```sh
cargo test --test ollama_docker --features integration-tests,llm-ollama
```

Locally this will almost always print `SKIP: ...` and pass without exercising anything, because
`OLLAMA_TEST_URL` is unset by default — that is the intended, documented behavior for a plain
`cargo test`, not a failure. The suite is exercised for real only in CI's `ollama-integration`
job (`.github/workflows/ci.yml`, the `ollama-integration` job near line 748), which brings up a
Docker Compose `ollama-test` service, waits for it to report healthy, pulls `qwen2.5:0.5b`, runs
this exact suite against it, and separately fails the job if any test took the `SKIP:` path — so
a green run there is proof the live server was actually exercised, not proof-by-absence. **Do
not add a second Ollama integration test file** — this is the one, and none should be added.

### A rejected credential now announces itself

Every provider above except Ollama shares one underlying protocol engine
(`CompatEngine`), so this applies uniformly to all of them — an operator debugging a
self-hosted OpenAI-compatible endpoint gets the same signal as one debugging DashScope,
Moonshot or xAI (2026-08-22, plan 17-22, closing G-17-4d).

Before this change, a rejected credential and an offline vendor looked identical: the
model-list fetch silently fell back to a curated list with nothing above a `debug` log
line, in either case. This is what let a genuine credential/region mismatch go
undiagnosed for five days during this phase's own live verification (`.planning/WINDOWS.md`
gap history). Now, when the configured endpoint rejects the request (an authentication
failure), a `warn`-level line is emitted naming the endpoint and stating that the
returned list is the curated fallback, not the vendor's own catalog — for example:

```text
[WARN] configured endpoint https://dashscope-intl.aliyuncs.com/compatible-mode/v1 rejected
the request while listing models (Authentication failed: ...); the returned model list is
the curated fallback, not this vendor's own catalog — a credential scoped to a different
account or region is the usual cause
```

An endpoint that is simply unreachable — a self-hosted Ollama that has not started yet,
a network blip, a slow response — stays at `debug`, exactly as before: being offline is a
supported state (D-13/D-14), not a misconfiguration, and this diagnostic does not fire
for it. Seeing the warning at all means the fix is the same one described throughout this
page: check the configured `base_url`/`*_BASE_URL` override against the credential you
are using.

## Environment variables

The full LLM environment-variable surface — every credential, base-URL, model, timeout
and (for the generic `openai-compatible` provider) capability/temperature override the
adapters read — is documented as a first-class configuration path, alongside the YAML
above, in `.env.example` at the repository root. Copy it to
`.env` and fill in the credentials you need; unset variables fall back to the defaults
shown in this guide.

**In the devcontainer**, these credentials arrive from `~/.config/paladin/` (one file per
secret, filename = the lowercased variable name — e.g. `~/.config/paladin/xai_api_key` →
`XAI_API_KEY`) via `.devcontainer/paladin-env.sh`, sourced automatically into interactive
shells by `~/.bashrc`. A genuinely-exported non-empty value always wins over the file. A
**non-interactive** shell (a script, a CI step, an agent's Bash tool) does not run
`~/.bashrc` and therefore does not source `paladin-env.sh` automatically — it must be
sourced explicitly: `set -a; . .devcontainer/paladin-env.sh; set +a`.

## Garrison (Short-term Memory)

The Garrison stores conversation context between Paladin turns.

```yaml
garrison:
  garrison_type: "in_memory"       # in_memory | sqlite
  # path: "./garrison.db"          # Required when garrison_type = "sqlite"
  max_entries: 100                  # Max conversation turns to retain
  max_tokens: 4000                  # Context-window token budget
  tokenizer: "gpt-4"               # Model name for token counting
  eviction_strategy: "importance_based"  # importance_based | fifo | sliding_window
  preserve_recent_count: 10        # Always keep at least N recent entries
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `garrison_type` | string | `in_memory` | Storage backend |
| `path` | string | - | SQLite file path (sqlite only) |
| `max_entries` | int | 100 | Maximum entries before eviction |
| `max_tokens` | int | 4000 | Token budget for context window |
| `eviction_strategy` | string | `importance_based` | Eviction algorithm |
| `preserve_recent_count` | int | 10 | Minimum recent entries to keep |

**Env vars:** `APP_GARRISON_TYPE`, `APP_GARRISON_PATH`, `APP_GARRISON_MAX_ENTRIES`,
`APP_GARRISON_MAX_TOKENS`, `APP_GARRISON_EVICTION_STRATEGY`, `APP_GARRISON_PRESERVE_RECENT_COUNT`

## Sanctum (Long-term Vector Memory)

Sanctum stores semantic memories in a vector database for RAG.

```yaml
sanctum:
  enabled: false
  adapter_type: "in_memory"        # in_memory | qdrant

  qdrant:                          # Required when adapter_type = "qdrant"
    url: "http://localhost:6334"
    collection_name: "paladin_memories"
    vector_dimension: 1536         # Must match your embedding model

rag:
  top_k: 5                         # Results to retrieve
  min_similarity: 0.7              # Score threshold (0.0-1.0)
  max_tokens: 2000                 # Max tokens to inject from RAG
  timeout_seconds: 5

memory_extraction:
  enabled: true
  strategy: "on_completion"        # every_turn | on_completion | manual
```

**Env vars:** `APP_SANCTUM_ENABLED`, `APP_SANCTUM_ADAPTER_TYPE`,
`APP_SANCTUM_QDRANT_URL`, `APP_SANCTUM_QDRANT_COLLECTION_NAME`,
`APP_SANCTUM_QDRANT_VECTOR_DIMENSION`

See [Sanctum Vector Memory](../user-guides/sanctum-vector-memory.md) for detail.

## Arsenal (Tool System / MCP)

The Arsenal connects Paladins to external tools via the Model Context Protocol.

```yaml
arsenal:
  default_timeout_seconds: 30
  max_concurrent_tools: 5
  mcp_servers:
    # STDIO server (command-line process)
    - name: "web_search"
      server_type: "stdio"
      command: "uvx"
      args: ["mcp-web-search"]

    # Streamable-HTTP server (remote, optionally authenticated)
    - name: "code_analyzer"
      server_type: "streamable_http"
      endpoint: "http://localhost:8080/mcp"
      # NAMES the env var holding the bearer token -- never a literal
      # secret in this file. Omit entirely for an unauthenticated server.
      auth_token_env: "CODE_ANALYZER_TOKEN"
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_timeout_seconds` | int | 30 | Per-tool execution timeout |
| `max_concurrent_tools` | int | 5 | Parallel tool invocations |
| `mcp_servers[].name` | string | - | Unique server identifier |
| `mcp_servers[].server_type` | string | - | `stdio` or `streamable_http` (`sse` is retired -- fails loud with a migration message) |
| `mcp_servers[].command` | string | - | Executable (stdio only) |
| `mcp_servers[].endpoint` | string | - | URL (streamable_http only) |
| `mcp_servers[].auth_token_env` | string | - | Env var NAME holding the bearer token (streamable_http only, optional) |

**Env vars:** `APP_ARSENAL_DEFAULT_TIMEOUT_SECONDS`, `APP_ARSENAL_MAX_CONCURRENT_TOOLS`

See [Arsenal & Tools](../user-guides/arsenal-tools.md) for full integration guide.

## Citadel (State Persistence)

Citadel saves Paladin state to disk for crash recovery and resumption.

```yaml
citadel:
  enabled: false
  state_dir: "./paladin-states"
  autosave_enabled: false          # Save state after each execution
  cleanup_enabled: false           # Delete old state files automatically
  max_state_age_days: 30
```

**Env vars:** `APP_CITADEL_ENABLED`, `APP_CITADEL_STATE_DIR`,
`APP_CITADEL_AUTOSAVE_ENABLED`, `APP_CITADEL_CLEANUP_ENABLED`,
`APP_CITADEL_MAX_STATE_AGE_DAYS`

## Battalion (Multi-agent Orchestration)

```yaml
battalion:
  default_timeout_seconds: 300     # Per-battalion execution timeout
  error_strategy: "fail_fast"      # fail_fast | continue_on_error | retry_then_continue
  max_concurrent_paladins: 10      # Phalanx concurrency limit
  metadata_output_enabled: false   # Write execution metadata to files

  retry:                           # Used when error_strategy = retry_then_continue
    max_attempts: 3
    exponential_backoff: true
    jitter: true
    base_delay_ms: 100
    max_delay_seconds: 10

  maneuver:                        # Flow DSL (Maneuver pattern)
    error_strategy: "fail_fast"    # fail_fast | continue_parallel | ignore_errors
    output_format: "combined_text" # combined_text | structured_json
    pass_output_as_input: true
    timeout_seconds: 300
    collect_timing_metrics: true
    max_agents: 30
    max_depth: 5
```

**Env vars:** `APP_BATTALION_DEFAULT_TIMEOUT_SECONDS`, `APP_BATTALION_ERROR_STRATEGY`,
`APP_BATTALION_MAX_CONCURRENT_PALADINS`, `APP_BATTALION_RETRY_MAX_ATTEMPTS`, etc.

See [Battalion Patterns](../user-guides/battalion-patterns.md) for Formation,
Phalanx, Campaign, and Chain of Command details.

## Herald (Output Formatting)

```yaml
herald:
  default_formatter: "json"        # json | markdown | table

  json:
    pretty: true
    include_metadata: true

  markdown:
    include_colors: true
    heading_level: 2

  table:
    max_column_width: 60
    border_style: "rounded"        # ascii | rounded | modern | sharp | none
```

**Env vars:** `APP_HERALD_DEFAULT_FORMATTER`, `APP_HERALD_JSON_PRETTY`,
`APP_HERALD_MARKDOWN_INCLUDE_COLORS`, `APP_HERALD_TABLE_BORDER_STYLE`

## Autonomous Features

All autonomous features are opt-in (disabled by default). Uncomment sections in
`config.yml` to enable:

```yaml
autonomous:
  planning:
    enabled: false          # Decompose complex tasks into subtasks
    max_subtasks: 10

  prompt_generation:
    enabled: false          # Auto-generate system prompts from description
    description: null       # e.g. "Expert data analyst"

  dynamic_temperature:
    enabled: false          # Adjust temperature per task type
    min: 0.1
    max: 0.9

  handoffs:
    enabled: false          # Delegate to specialist Paladins
    strategy: "automatic"   # automatic | explicit | {threshold: 0.8}
    max_depth: 5
```

**Env vars:** `APP_AUTONOMOUS_PLANNING_ENABLED`, `APP_AUTONOMOUS_PLANNING_MAX_SUBTASKS`,
`APP_AUTONOMOUS_PROMPT_GENERATION_ENABLED`, `APP_AUTONOMOUS_DYNAMIC_TEMPERATURE_ENABLED`,
`APP_AUTONOMOUS_HANDOFFS_ENABLED`, `APP_AUTONOMOUS_HANDOFFS_STRATEGY`

## Multi-Environment Pattern

Keep a `config.yml` for defaults and override per environment:

```bash
# Development
export APP_LLM_DEFAULT_PROVIDER=openai
export APP_GARRISON_TYPE=in_memory

# Staging
export APP_GARRISON_TYPE=sqlite
export APP_GARRISON_PATH=/data/garrison.db
export APP_SANCTUM_ENABLED=true

# Production
export APP_GARRISON_TYPE=sqlite
export APP_SANCTUM_ENABLED=true
export APP_SANCTUM_ADAPTER_TYPE=qdrant
export APP_CITADEL_ENABLED=true
export APP_CITADEL_AUTOSAVE_ENABLED=true
```

## Complete Example (`config.yml`)

```yaml
llm:
  default_provider: "openai"
  openai:
    default_model: "gpt-4"
    default_temperature: 0.7

garrison:
  garrison_type: "sqlite"
  path: "./garrison.db"
  max_entries: 200
  max_tokens: 8000

arsenal:
  default_timeout_seconds: 30
  max_concurrent_tools: 5
  mcp_servers:
    - name: "web_search"
      server_type: "stdio"
      command: "uvx"
      args: ["mcp-web-search"]

battalion:
  error_strategy: "retry_then_continue"
  max_concurrent_paladins: 10
  retry:
    max_attempts: 3
    exponential_backoff: true

herald:
  default_formatter: "markdown"
```

## See Also

- [Installation](installation.md)
- [Quickstart](quickstart.md)
- [Garrison Memory](../user-guides/garrison-memory.md)
- [Arsenal & Tools](../user-guides/arsenal-tools.md)
- [Sanctum Vector Memory](../user-guides/sanctum-vector-memory.md)
- [Battalion Patterns](../user-guides/battalion-patterns.md)
