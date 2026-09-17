# CLI Test Guide

This document describes the CLI test infrastructure, how tests are organized into tiers, and how to run them.

## Test Tiers

### Tier 1: Core Functionality (No External Dependencies)

Tests that run with `cargo test` and require no external services, API keys, or Docker.

**Location:** `tests/cli/environment_tests.rs`

**What's tested:**
- Config file loading (valid, invalid, missing)
- YAML parsing and validation (syntax errors, duplicate keys, tabs)
- Edge cases (empty fields, large inputs, concurrent loading)
- Non-interactive mode (all commands work via flags, no hanging prompts)
- Environment variation (NO_COLOR, quiet/verbose modes, formatter behavior)
- Full user journey (template generation → config load → output formatting)

**Run:**
```bash
cargo test cli::environment_tests::
```

### Tier 2: Docker-Gated Service Tests

Tests that require Docker services (Redis, MinIO) to be running. Skipped automatically when services are unavailable.

**Location:** `tests/integration/cli_real_services_test.rs`

**What's tested:**
- Redis connectivity and health checks
- MinIO connectivity and health checks
- Service unavailability detection
- Connection error handling

**Prerequisites:**
```bash
make services-up   # Start Redis, MinIO, MySQL via Docker Compose
```

**Run:**
```bash
cargo test --test lib cli_real_services -- --ignored
```

**Skip message:** Tests print a clear message when Docker services are not available.

### Tier 3: API-Key-Gated Provider Tests

Tests that require real LLM API keys. Behind the `integration-tests` feature flag and `#[ignore]`.

**Location:** `tests/integration/cli_real_providers_test.rs`

**What's tested:**
- OpenAI provider connection and streaming
- Anthropic provider connection
- DeepSeek provider connection
- End-to-end agent config with real providers

**Prerequisites:**
```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export DEEPSEEK_API_KEY="sk-..."
```

**Run:**
```bash
cargo test --features integration-tests --test lib cli_real_providers -- --ignored
```

### Tier 4: Live LLM API Integration Tests

Direct adapter-level tests that make real API calls to LLM providers. These tests validate the low-level integration of OpenAI, DeepSeek, and Anthropic adapters with their respective APIs. **These tests incur API costs and should be run sparingly.**

**Location:** `tests/integration/llm_live_api_tests.rs`

**Feature Flag:** `live-api-tests`

**What's tested:**

Each provider (OpenAI, DeepSeek, Anthropic) has 4 dedicated tests:

1. **Basic completion** - Validates `generate()` method with real API
2. **Streaming completion** - Validates `generate_stream()` method with chunked responses
3. **Error handling** - Tests invalid model detection and error mapping
4. **Capabilities** - Validates provider capabilities reporting

**Total:** 13 tests — 12 live-API tests (4 per provider × 3 providers) plus one
`test_suite_documentation` meta-test that documents the suite and always passes without making a
call.

**Test Characteristics:**
- All tests are marked with `#[ignore]` - they don't run by default
- Tests skip gracefully if API keys are not present
- Each test makes a real API call (costs apply)
- Validates response structure, token usage, and finish reasons
- Tests both success and error paths

**Prerequisites:**
```bash
# Set one or more API keys
export OPENAI_API_KEY="sk-..."
export DEEPSEEK_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-..."
```

**Run all live API tests:**
```bash
cargo test --features live-api-tests -- --ignored
```

**Run specific provider tests:**
```bash
# OpenAI only (4 tests)
cargo test --features live-api-tests test_openai -- --ignored

# DeepSeek only (4 tests)
cargo test --features live-api-tests test_deepseek -- --ignored

# Anthropic only (4 tests)
cargo test --features live-api-tests test_anthropic -- --ignored
```

**Example output when API key is missing:**
```
test test_openai_basic_completion ... ok (SKIPPED: OpenAI API key not found. Set OPENAI_API_KEY environment variable to run OpenAI live API tests.)
```

**Example output when test passes:**
```
test test_openai_basic_completion ... ok
✓ OpenAI basic completion: Hello from OpenAI
```

**Cost Considerations:**
- Each test makes 1 API call (except error handling tests, which may fail fast)
- Use small prompts (< 100 tokens) to minimize costs
- Recommended models: `gpt-3.5-turbo`, `deepseek-chat`, `claude-3-5-sonnet-20241022`
- Estimated cost per full test run: < $0.10 USD

**When to run these tests:**
- Before releasing a new version
- After modifying adapter implementations
- When troubleshooting provider-specific issues
- For validating API key configuration during setup
- **Not recommended in CI/CD pipelines** (use mocks instead)

## Running Tests

### Quick Check (Tier 1 only — no dependencies)
```bash
cargo test cli::environment_tests::
```

### All CLI Tests (Tier 1)
```bash
cargo test --test lib cli::
```

### With Docker Services (Tier 1 + 2)
```bash
make services-up
cargo test --test lib cli:: -- --include-ignored
```

### Full Suite (Tier 1 + 2 + 3)
```bash
make services-up
export OPENAI_API_KEY="sk-..."
cargo test --features integration-tests --test lib -- --include-ignored
```

## Test Counts

| Tier | Count | Gate |
|------|-------|------|
| Tier 1 (Core) | 45 | None |
| Tier 2 (Docker) | 6 | `#[ignore]` + service check |
| Tier 3 (API keys) | 5 | `integration-tests` feature + `#[ignore]` + env var |
| Tier 4 (Live API) | 13 | `live-api-tests` feature + `#[ignore]` + env var |

## CI/CD Notes

- **Tier 1** tests run in every CI pipeline with no setup required
- **Non-interactive safety:** All Tier 1 tests verify that CLI operations never block on stdin. The `ensure_tty()` guard detects non-TTY environments (CI runners) and returns a clear `ValidationError` instead of hanging
- **NO_COLOR:** Formatters respect the `NO_COLOR` environment variable. Set `NO_COLOR=1` in CI to suppress ANSI escape codes
- **Line buffering:** All output uses `println!`/`eprintln!` which flush per-line — safe for CI log capture

## Mock Infrastructure for Testing

### MockLlmAdapter

The `MockLlmAdapter` provides a test double for LLM providers, enabling Tier 1 tests without API keys.

**Location:** `tests/helpers/mock_llm_adapter.rs`

**Features:**
- **Configurable responses**: Queue pre-defined text, tool calls, streaming, or errors
- **Invocation recording**: Capture all LLM calls for test assertions
- **Tool call simulation**: Return function calls to test arsenal integration
- **Error injection**: Simulate API failures, timeouts, rate limits

**Example usage:**
```rust
use tests::helpers::mock_llm_adapter::MockLlmAdapter;

let mock = MockLlmAdapter::new()
    .add_response("First response")
    .add_tool_call("web_search", json!({"query": "test"}))
    .add_response("Final answer");

// Use mock in PaladinExecutionService
let service = PaladinExecutionService::new(
    Arc::new(mock.clone()) as Arc<dyn LlmPort>,
    None,
    Arc::new(ArsenalRegistry::new()),
);

// Execute and assert
let result = service.execute(&paladin, "test input").await?;
assert_eq!(mock.invocations().len(), 3);
```

### MockArsenalPort

The `MockArsenalPort` provides in-process tool mocking for testing arsenal integration.

**Location:** `tests/helpers/mock_arsenal_adapter.rs`

**Features:**
- **Tool registration**: Add mock tools with schemas
- **Response configuration**: Set success responses or errors
- **Invocation tracking**: Verify tool calls with arguments
- **Error simulation**: Test tool failure scenarios

**Example usage:**
```rust
use tests::helpers::mock_arsenal_adapter::MockArsenalPort;

let mock = MockArsenalPort::new()
    .add_tool("calculator", "Perform calculations", json!({
        "type": "object",
        "properties": {
            "expression": {"type": "string"}
        }
    }))
    .set_response("calculator", Ok(json!({"result": 42})));

// Use in PaladinExecutionService via ArsenalRegistry
let mut registry = ArsenalRegistry::new();
registry.register("mock_server", Arc::new(mock.clone()))?;

// Execute and assert
assert_eq!(mock.call_count("calculator"), 1);
```

### MockPaladinPort

The `MockPaladinPort` enables Battalion testing without full Paladin execution.

**Location:** `tests/helpers/mock_paladin_port.rs`

**Features:**
- **Result configuration**: Set expected Paladin outputs
- **Error simulation**: Test error propagation in Battalions
- **Execution tracking**: Verify execution order and count

## Test Coverage

### Current Test Statistics (as of Epic 23 completion)

| Category | Tests | Coverage |
|----------|-------|----------|
| **Garrison Configuration** | 9 | In-memory, SQLite, validation |
| **Arsenal Configuration** | 8 | STDIO, SSE, tool registration |
| **Error Handling** | 14 | Config errors, execution errors |
| **Paladin Execution** | 6 | Basic, with garrison, with arsenal |
| **Formation Execution** | 4 | Sequential flow, error propagation |
| **Phalanx Execution** | 5 | Parallel execution, aggregation |
| **Tool Integration** | 8 | LLM → Arsenal → result loop |
| **Mock Infrastructure** | 9 | MockArsenalPort unit tests |
| **Scheduler** | 21 | Unit + integration tests |
| **Total CLI Tests** | 84 | All CI-ready with mocks |

### Tool Integration Tests

**Location:** `tests/cli/tool_integration_test.rs`

Tests the complete LLM ↔ Arsenal ↔ Paladin tool call loop:

1. **Core flow tests** (2):
   - `test_tool_call_basic_flow`: LLM function call → Arsenal execution → result
   - `test_tool_call_result_fed_back_to_llm`: Tool result returned to LLM for synthesis

2. **Error handling tests** (4):
   - `test_tool_call_no_arsenal_available`: Graceful handling when Arsenal not configured
   - `test_tool_call_unknown_tool`: Tool not in registry
   - `test_tool_call_invalid_arguments`: Malformed JSON arguments
   - `test_tool_call_execution_error`: Tool invocation failure

3. **Advanced tests** (2):
   - `test_multiple_sequential_tool_calls`: Chain of tool calls
   - `test_tool_call_with_garrison`: Tools + memory integration

## Adding New Tests

1. **Pure logic / config tests** → Add to `tests/cli/environment_tests.rs` (Tier 1)
2. **Requires Docker services** → Add to `tests/integration/cli_real_services_test.rs` with `#[ignore]`
3. **Requires API keys** → Add to `tests/integration/cli_real_providers_test.rs` with feature gate + `#[ignore]`
4. **Tool integration** → Add to `tests/cli/tool_integration_test.rs` using MockLlmAdapter + MockArsenalPort
5. **Battalion orchestration** → Use MockPaladinPort in Formation/Phalanx/Campaign tests
6. **CLI output formatting** → Add snapshot tests to `tests/cli/` (see [CLI Snapshot Testing](#cli-snapshot-testing))
7. **Live LLM adapter tests** → Add to `tests/integration/llm_live_api_tests.rs` with `#[cfg(feature = "live-api-tests")]` and `#[ignore]`
8. Always run `cargo test cli::environment_tests::` after changes to verify Tier 1 passes

## CLI Snapshot Testing

CLI snapshot testing ensures output consistency across code changes using the [`insta`](https://insta.rs/) library.

### Overview

**Location:** `tests/cli/`

**Test Files:**
- `table_output_test.rs` - Table formatting with comfy-table
- `progress_output_test.rs` - Progress indicators and bars
- `error_output_test.rs` - Error messages and styled output
- `help_output_test.rs` - Help text and documentation

**Snapshot Location:** `tests/cli/snapshots/`

### Running Snapshot Tests

```bash
# Run all CLI snapshot tests
cargo test --test cli

# Review new/changed snapshots
cargo insta review

# Accept all new snapshots
cargo insta accept

# Reject all pending snapshots
cargo insta reject
```

### Writing Snapshot Tests

Snapshot tests capture CLI output and compare against saved baselines:

```rust
use paladin::application::cli::formatters::table::TableFormatter;

#[test]
fn test_execution_summary() {
    let mut table = TableFormatter::new();
    table
        .set_header(vec!["Agent", "Status", "Time"])
        .add_row(vec!["DataAnalyzer", "Success", "1.2s"]);

    let output = table.render();

    // Compare against saved snapshot
    insta::assert_snapshot!("execution_summary", output);
}
```

**First Run:** Creates `tests/cli/snapshots/cli__table_output_test__execution_summary.snap`

**Subsequent Runs:** Compares output against snapshot, fails if different

### Best Practices

1. **Disable colors in tests:**
   ```bash
   NO_COLOR=1 cargo test --test cli
   ```

2. **Use descriptive snapshot names:**
   ```rust
   insta::assert_snapshot!("table_with_styled_cells", output);  // Good
   insta::assert_snapshot!("test1", output);                     // Bad
   ```

3. **Test edge cases:**
   - Empty tables
   - Long content requiring truncation
   - Unicode/special characters
   - Multi-line output

4. **Review snapshots carefully:**
   - Verify output is correct before accepting
   - Use `cargo insta review` for interactive approval
   - Inspect snapshot files in `tests/cli/snapshots/`

5. **Group related tests:**
   - Table tests → `table_output_test.rs`
   - Error tests → `error_output_test.rs`
   - Keep test files focused and organized

### Snapshot File Format

Snapshots are stored as `.snap` files:

```snap
---
source: tests/cli/table_output_test.rs
expression: output
---
┌────────┬─────────┬──────┐
│ Agent  ┆ Status  ┆ Time │
╞════════╪═════════╪══════╡
│ DataA… ┆ Success ┆ 1.2s │
└────────┴─────────┴──────┘
```

**Fields:**
- `source`: Test file location
- `expression`: Rust expression being tested
- Content: Actual snapshot data

### CI/CD Integration

Snapshot tests run automatically in CI as the dedicated `cli-tests` job in `ci.yml` (see the job
table in [CI/CD Guide](../deployment/cicd.md#ci-pipeline) — required, not advisory):

```yaml
# excerpt: .github/workflows/ci.yml — job: cli-tests
- name: Run CLI snapshot tests
  shell: bash
  run: |
    set -o pipefail
    cargo test -p paladin-ai --features cli --test cli 2>&1 | tee cli-test-output.log
    executed=$(grep -oE '[0-9]+ passed' cli-test-output.log | tail -1 | grep -oE '[0-9]+')
    if [ -z "$executed" ] || [ "$executed" -eq 0 ]; then
      echo "::error::cli-tests reported zero executed tests. This usually means --features cli was dropped, causing Cargo to silently skip the required-features-gated cli test target."
      exit 1
    fi
    echo "cli-tests executed $executed test(s)"
```

**Note:** the job asserts at least one test actually ran (guarding against `--features cli` being
silently dropped) rather than checking for pending `insta` snapshots. Run `cargo insta review`
locally before committing a snapshot change so CI sees an already-accepted `.snap` file.

### Example Test Categories

#### Table Output Tests (8 tests)
- Simple tables
- Long content
- Styled cells (success/error/warning/info)
- Empty tables
- Single column
- Numeric data
- Special characters
- Battalion results

#### Progress Output Tests (8 tests)
- Default progress bar template
- Custom template
- Different totals
- Message variations
- Progress states (0%, 25%, 50%, 75%, 100%)
- Builder pattern
- Batch operations
- File size formatting

#### Error Output Tests (15 tests)
- Error message styles
- Warning message styles
- Info message styles
- Success message styles
- Link styles
- Header rendering
- Section rendering
- Box message rendering
- Key-value formatting
- Emoji fallback
- Separator lines
- Quiet/verbose mode flags
- Combined error scenarios
- Multi-line error formatting

#### Help Output Tests (12 tests)
- Basic command help
- Command help with examples
- Subcommand lists
- Option groups
- Help header
- Usage examples section
- Error help messages
- Feature flags help
- Environment variables help
- Configuration help
- Troubleshooting help
- Version output

### Total Snapshot Tests: 43

## Writing Tests with Mocks

### Best Practices

1. **Use MockLlmAdapter for LLM tests**:
   - Queue expected responses in order
   - Verify invocations after execution
   - Test both success and error paths

2. **Use MockArsenalPort for tool tests**:
   - Register tools with realistic schemas
   - Configure responses for each tool
   - Verify tool call arguments

3. **Keep tests deterministic**:
   - No random values in mocks
   - Use fixed response sequences
   - Assert exact invocation counts

4. **Test error scenarios**:
   - LLM errors: rate limits, timeouts, invalid responses
   - Tool errors: execution failures, timeouts, unknown tools
   - Config errors: invalid YAML, missing fields, type mismatches

5. **Verify integration points**:
   - Garrison is queried for context
   - Arsenal is called with correct arguments
   - CircuitBreaker tracks failures
   - Results are formatted correctly

---

**Last updated:** February 14, 2026  
**Epic:** 23 - CLI, Config & Infrastructure Completion
