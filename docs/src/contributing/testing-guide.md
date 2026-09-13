# Testing Guide

Comprehensive testing guide for Paladin development with TDD practices, coverage requirements, and testing patterns.

## Quick Reference: Test Commands

```bash
# Unit tests (all workspace crates)
cargo test --workspace --lib

# All tests (unit + integration)
make test-all

# Integration tests with Docker services (Redis, MinIO, MySQL)
make test-integration-docker

# Doc tests only
cargo test --doc

# Specific integration test file
cargo test --test paladin_tests

# Run with feature flags
cargo test --features "integration-tests"
cargo test --features "live-api-tests"   # requires real API keys
```

## Table of Contents

- [Quick Reference: Test Commands](#quick-reference-test-commands)
- [Testing Philosophy](#testing-philosophy)
- [Test Organization](#test-organization)
- [Unit Testing](#unit-testing)
- [Integration Testing](#integration-testing)
- [Functional Testing](#functional-testing)
- [Test Coverage](#test-coverage)
- [Mocking and Fixtures](#mocking-and-fixtures)
- [CI Integration](#ci-integration)
- [Testing Best Practices](#testing-best-practices)
- [Next Steps](#next-steps)

## Testing Philosophy

Paladin follows **Test-Driven Development (TDD)** with the Red-Green-Refactor cycle:

```
┌─────────────┐
│  1. RED     │  Write failing test first
│  ✗ Failing  │
└─────────────┘
       │
       ▼
┌─────────────┐
│  2. GREEN   │  Write minimal code to pass
│  ✓ Passing  │
└─────────────┘
       │
       ▼
┌─────────────┐
│ 3. REFACTOR │  Improve while keeping tests green
│  ✓ Passing  │
└─────────────┘
```

### Coverage Requirements

There is a single binding coverage floor, recorded in ADR-0006 (`.planning/decisions/0006-coverage-gate.md`):
**82% workspace line coverage**, gated by `cargo llvm-cov --fail-under-lines` in CI's `coverage`
job and mirrored locally by `make coverage`. There is no separate unit-test target and no separate
integration-test target — see [Test Coverage](#test-coverage) below for the full procedure,
scope, and threshold policy. Public APIs still require doc tests (100%), which coverage
tooling counts separately from the line-coverage gate.

## Test Organization

### Directory Structure

```
tests/
├── lib.rs                    # Test utilities and common setup
├── unit/                     # Unit tests (parallel execution)
│   ├── mod.rs
│   ├── paladin_tests.rs
│   ├── garrison_tests.rs
│   └── arsenal_tests.rs
├── integration/              # Integration tests (serial execution)
│   ├── mod.rs
│   ├── redis_queue_test.rs
│   ├── minio_storage_test.rs
│   └── llm_provider_test.rs
├── functional/               # End-to-end functional tests
│   ├── mod.rs
│   ├── content_lifecycle_test.rs
│   └── battalion_execution_test.rs
└── fixtures/                 # Test data and fixtures
    ├── config.test.yml
    └── sample_data.json
```

### Test Module Naming

```rust,ignore
// Unit tests inline with code
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paladin_builder_validation() {
        // Test implementation
    }
}

// Integration tests in tests/ directory
// tests/integration/redis_queue_test.rs
#[tokio::test]
async fn test_redis_queue_operations() {
    // Test implementation
}
```

## Unit Testing

### Basic Unit Test Pattern

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paladin_builder_creates_valid_paladin() {
        // Arrange
        let llm_port = Arc::new(MockLlmPort::new());
        let builder = PaladinBuilder::new(llm_port);

        // Act
        let result = builder
            .name("test-paladin")
            .system_prompt("You are a helpful assistant")
            .build();

        // Assert
        assert!(result.is_ok());
        let paladin = result.unwrap();
        assert_eq!(paladin.name(), "test-paladin");
    }

    #[test]
    fn test_paladin_builder_validates_empty_prompt() {
        // Arrange
        let llm_port = Arc::new(MockLlmPort::new());
        let builder = PaladinBuilder::new(llm_port);

        // Act
        let result = builder
            .name("test-paladin")
            .system_prompt("")  // Invalid: empty prompt
            .build();

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PaladinError::ConfigurationError(_)
        ));
    }
}
```

### Testing Async Code

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_paladin_execution() {
        // Arrange
        let mock_llm = Arc::new(MockLlmPort::with_response("Test response"));
        let paladin = create_test_paladin(mock_llm);

        // Act
        let result = paladin.execute("Test input").await;

        // Assert
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content, "Test response");
    }
}
```

### Property-Based Testing

```rust,ignore
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_garrison_always_respects_max_entries(
        entries in prop::collection::vec(any::<String>(), 0..1000)
    ) {
        let max_entries = 100;
        let garrison = InMemoryGarrison::new(max_entries);
        let session_id = Uuid::new_v4();

        // Add all entries
        for entry in entries {
            let _ = garrison.add_entry(session_id, entry);
        }

        // Verify max entries constraint
        let stored = garrison.get_entries(session_id, None).unwrap();
        prop_assert!(stored.len() <= max_entries);
    }
}
```

## Integration Testing

### Redis Integration Test

```rust,ignore
// tests/integration/redis_queue_test.rs

use paladin::infrastructure::adapters::queue::RedisQueueAdapter;
use testcontainers::{clients, images};

#[tokio::test]
#[serial]  // Run serially to avoid port conflicts
async fn test_redis_queue_enqueue_dequeue() {
    // Arrange: Start Redis container
    let docker = clients::Cli::default();
    let redis = docker.run(images::redis::Redis::default());
    let port = redis.get_host_port_ipv4(6379);

    let adapter = RedisQueueAdapter::new(&format!("redis://localhost:{}", port))
        .await
        .unwrap();

    // Act: Enqueue task
    let task = Task::new("test-task", serde_json::json!({"input": "test"}));
    adapter.enqueue(task.clone()).await.unwrap();

    // Assert: Dequeue task
    let dequeued = adapter.dequeue().await.unwrap();
    assert!(dequeued.is_some());
    assert_eq!(dequeued.unwrap().id, task.id);
}
```

### MinIO Integration Test

```rust,ignore
// tests/integration/minio_storage_test.rs

use paladin::infrastructure::adapters::file_storage::MinioAdapter;
use testcontainers::{clients, GenericImage};

#[tokio::test]
#[serial]
async fn test_minio_upload_download() {
    // Arrange: Start MinIO container
    let docker = clients::Cli::default();
    let minio = docker.run(
        GenericImage::new("quay.io/minio/minio", "RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772")
            .with_env_var("MINIO_ROOT_USER", "minioadmin")
            .with_env_var("MINIO_ROOT_PASSWORD", "minioadmin")
            .with_wait_for(WaitFor::message_on_stdout("API:"))
    );

    let adapter = MinioAdapter::new(
        "localhost:9000",
        "minioadmin",
        "minioadmin",
        "test-bucket",
    ).await.unwrap();

    // Act: Upload file
    let content = b"Test content";
    adapter.upload("test.txt", content).await.unwrap();

    // Assert: Download file
    let downloaded = adapter.download("test.txt").await.unwrap();
    assert_eq!(downloaded, content);
}
```

### LLM Provider Mock Test

```rust,ignore
// tests/integration/llm_provider_test.rs

use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_openai_adapter_with_mock_server() {
    // Arrange: Start mock server
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "Mock response"
                    }
                }],
                "usage": {
                    "total_tokens": 10
                }
            })
        ))
        .mount(&mock_server)
        .await;

    // Act: Create adapter with mock URL
    let adapter = OpenAiAdapter::new(
        "test-key",
        &mock_server.uri(),
    );

    let messages = vec![Message::user("Test")];
    let response = adapter.generate(&messages, &LlmConfig::default()).await.unwrap();

    // Assert
    assert_eq!(response.content, "Mock response");
}
```

## Functional Testing

### End-to-End Content Lifecycle

```rust,ignore
// tests/functional/content_lifecycle_test.rs

#[tokio::test]
async fn test_complete_content_processing_flow() {
    // Arrange: Set up full application stack
    let config = ApplicationSettings::test_config();
    let app = Application::build(&config).await.unwrap();

    // Act: Submit content for processing
    let content = ContentItem::new("Test article", "https://example.com");
    let result = app.ingest_content(content).await.unwrap();

    // Assert: Verify content processed through all stages
    assert_eq!(result.status, ContentStatus::Completed);

    // Verify analysis results exist
    let analysis = app.get_analysis(result.id).await.unwrap();
    assert!(analysis.is_some());

    // Verify stored in database
    let stored = app.get_content(result.id).await.unwrap();
    assert!(stored.is_some());
}
```

### Battalion Execution Flow

```rust,ignore
// tests/functional/battalion_execution_test.rs

#[tokio::test]
async fn test_formation_sequential_execution() {
    // Arrange
    let llm_port = Arc::new(MockLlmPort::sequential_responses(vec![
        "Response 1",
        "Response 2",
        "Response 3",
    ]));

    let paladin1 = create_test_paladin(llm_port.clone(), "paladin-1");
    let paladin2 = create_test_paladin(llm_port.clone(), "paladin-2");
    let paladin3 = create_test_paladin(llm_port.clone(), "paladin-3");

    let formation = Formation::new(vec![paladin1, paladin2, paladin3]);

    // Act
    let result = formation.execute("Initial input").await.unwrap();

    // Assert
    assert_eq!(result.steps.len(), 3);
    assert_eq!(result.steps[0].output, "Response 1");
    assert_eq!(result.steps[1].output, "Response 2");
    assert_eq!(result.steps[2].output, "Response 3");
}
```

## Test Coverage

This section is the single documented procedure for reproducing the coverage number CI's
`coverage` job reports. Follow it top to bottom; every command here is the same command CI runs,
not an approximation of it.

### Prerequisites

Coverage uses [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov), the LLVM
source-based instrumentation tool and the tool of record per
ADR-0006 (`.planning/decisions/0006-coverage-gate.md`). Install it:

```bash
# Required: the LLVM tools component cargo-llvm-cov instruments with.
# Without it, `cargo llvm-cov` fails immediately with a missing-component error —
# it cannot instrument the build at all.
rustup component add llvm-tools-preview

# Install cargo-llvm-cov itself
cargo install cargo-llvm-cov --locked

# Faster alternative to `cargo install`: cargo binstall downloads a prebuilt
# binary instead of compiling from source.
cargo binstall cargo-llvm-cov
```

The gated measurement runs against `--features integration-tests`, which needs live Redis and
MinIO. **Start them first** — `make services-up` — or your local figure will not match CI's.

### Local generation

Two-step sequence — measuring does not implicitly start services (a Make dependency that spins up
containers as a side effect of reading a number would be surprising):

```bash
# 1. Start Redis and MinIO (once per session)
make services-up

# 2. Measure coverage — LCOV report plus the fail-under-lines threshold check
make coverage

# 3. Optional: browsable HTML report at target/coverage
make coverage-html
```

`make coverage` is a thin wrapper — this is the full underlying invocation, so you can see it is
not a different measurement from what CI runs:

```bash
cargo llvm-cov --workspace --features integration-tests --lcov --output-path lcov.info \
  --fail-under-lines 82 -- --test-threads=1
```

If `make coverage` fails with a Redis/MinIO connection error, that is `make coverage` itself
telling you to run `make services-up` first — it fails loudly with a pointer rather than starting
containers for you.

### The scope, and why it is that scope

The command above measures `--workspace --features integration-tests`, deliberately **not**
`--all-features`. `qdrant` requires a live Qdrant service and the vision/embedding suites require
real provider API keys — under `--all-features` that code would enter the denominator with
nothing in CI able to exercise it, depressing the number for no signal.

The three `[[bin]]` targets (`paladin`, `paladin-cli`, `paladin-server`) are feature-gated behind
`cli` and `web-server` respectively. `paladin` and `paladin-cli` sit outside the denominator by
construction under this feature set, matching `.codecov.yml`'s `src/bin/**` ignore entry (a
reporting-only exclusion for Codecov, not what the CI gate itself measures).

`#[ignore]`-gated tests are **outside both the numerator and the denominator** — the measurement
does not pass `--include-ignored`, which is `cargo test`'s default behavior, per ADR-0006's Phase
15 amendment.

### The threshold policy

**The floor: 82%**, from ADR-0006 (`.planning/decisions/0006-coverage-gate.md`)'s Phase
15 amendment. This is the single binding number — there is no separate unit-test target and no
separate integration-test target.

The derivation rule: the measured percentage is **truncated toward zero** to a whole percent —
explicitly neither round-half-up nor round-half-even — and the comparison is **at-or-above**. A
run measuring exactly 82% passes; a run measuring 81.99% fails. Because the floor is the measured
figure truncated downward at the time it was set, the gate cannot be red on the run that sets it.

The floor only moves up. ADR-0006's ratchet clause raises it at a qualifying milestone close — by
amending the ADR in place with the new figure, command, and date — and it never falls.

### Reading the output

`make coverage` prints an LCOV summary; `make coverage-html` writes a browsable report to
`target/coverage/html/index.html`. The report breaks down by region, function, and line:

- **Region** — sub-expression-level coverage (e.g., both branches of an `if`).
- **Function** — whether a function was called at all.
- **Line** — whether a source line executed.

**Only the line figure is what the gate compares.** `--fail-under-lines` reads the line-coverage
percentage exclusively; region and function percentages are informational context, not gated.

### Codecov behaviour

Codecov posts a PR comment with a diff view, but it **does not gate** — `.codecov.yml` sets both
the `project` and `patch` status blocks to `informational: true`. This is deliberate: without
`CODECOV_TOKEN` set, an upload can fail silently, especially on fork PRs, and a gate that silently
does not run is worse than no gate at all. The actual threshold gate is `cargo llvm-cov
--fail-under-lines` inside the `coverage` job — the same flag `make coverage` runs.

### Troubleshooting

**`error: llvm-tools-preview component not found`** — `rustup component add llvm-tools-preview`
was skipped or targeted the wrong toolchain. Re-run it against the active toolchain
(`rustup show`).

**Local figure lower than CI's** — the services were not running. `--features integration-tests`
exercises Redis- and MinIO-backed code paths; if `make services-up` was not run first, those tests
skip or fail, and the lines they would have covered count as missed. Run `make services-up`, then
re-run `make coverage`.

**Low patch coverage on a PR, overall coverage unaffected** — Codecov's patch view (informational
only) can flag newly added lines with no covering test even when the workspace-wide `--fail-under-lines`
gate still passes. Add a test for the flagged lines; it is not a CI failure, but it is a real gap.

**Codecov upload fails or is silently skipped** — `CODECOV_TOKEN` is unset or invalid, most
commonly on a fork PR where secrets are not available to the workflow. This is **not a build
failure**: `.codecov.yml`'s informational status blocks mean the PR still passes. The actual gate
(`--fail-under-lines`) is unaffected by a Codecov upload failure.

## Mocking and Fixtures

### Mock LLM Port

```rust,ignore
// tests/lib.rs

pub struct MockLlmPort {
    responses: Vec<String>,
    call_count: Arc<Mutex<usize>>,
}

impl MockLlmPort {
    pub fn new() -> Self {
        Self {
            responses: vec!["Mock response".into()],
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_response(response: impl Into<String>) -> Self {
        Self {
            responses: vec![response.into()],
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn sequential_responses(responses: Vec<impl Into<String>>) -> Self {
        Self {
            responses: responses.into_iter().map(Into::into).collect(),
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

#[async_trait]
impl LlmPort for MockLlmPort {
    async fn generate(
        &self,
        _messages: &[Message],
        _config: &LlmConfig,
    ) -> Result<LlmResponse, PaladinError> {
        let mut count = self.call_count.lock().unwrap();
        let index = *count % self.responses.len();
        *count += 1;

        Ok(LlmResponse {
            content: self.responses[index].clone(),
            model: "mock".into(),
            usage: Usage::default(),
            tool_calls: vec![],
        })
    }

    async fn generate_stream(
        &self,
        _messages: &[Message],
        _config: &LlmConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmChunk>>>>, PaladinError> {
        unimplemented!("Stream not implemented in mock")
    }

    fn validate_model(&self, _model: &str) -> Result<(), PaladinError> {
        Ok(())
    }
}
```

### Test Fixtures

```rust,ignore
// tests/lib.rs

pub fn create_test_paladin(llm_port: Arc<dyn LlmPort>, name: &str) -> Paladin {
    PaladinBuilder::new(llm_port)
        .name(name)
        .system_prompt("Test system prompt")
        .model("test-model")
        .temperature(0.7)
        .max_loops(3)
        .build()
        .unwrap()
}

pub fn test_config() -> ApplicationSettings {
    ApplicationSettings {
        llm: LlmConfig {
            provider: "mock".into(),
            ..Default::default()
        },
        garrison: GarrisonConfig {
            r#type: "in_memory".into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
```

## CI Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    strategy:
      matrix:
        rust: [stable, beta]

    services:
      redis:
        image: redis:7
        ports:
          - 6379:6379

      minio:
        image: quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772
        env:
          MINIO_ROOT_USER: minioadmin
          MINIO_ROOT_PASSWORD: minioadmin
        ports:
          - 9000:9000

    steps:
      - uses: actions/checkout@v3

      - uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}
          override: true

      - name: Run unit tests
        run: cargo test --lib

      - name: Run integration tests
        run: cargo test --test '*' -- --test-threads=1

      - name: Run doc tests
        run: cargo test --doc

      - name: Generate coverage
        run: |
          cargo install cargo-llvm-cov
          cargo llvm-cov --lcov --output-path lcov.info

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info
```

### Pre-commit Hooks

```bash
# .git/hooks/pre-commit
#!/bin/bash

echo "Running tests..."
cargo test --quiet || exit 1

echo "Checking formatting..."
cargo fmt --check || exit 1

echo "Running clippy..."
cargo clippy -- -D warnings || exit 1

echo "All checks passed!"
```

## Testing Best Practices

### Do's ✅

- Write tests first (TDD)
- Use descriptive test names
- Test one thing per test
- Use arrange-act-assert pattern
- Mock external dependencies
- Test error cases
- Use property-based testing for algorithms
- Maintain high coverage

### Don'ts ❌

- Don't test implementation details
- Don't ignore failing tests
- Don't skip integration tests
- Don't hardcode test data
- Don't make tests dependent on order
- Don't test framework code
- Don't ignore performance tests

## Next Steps

- **[Adapter Development](contributing-providers.md)** - Create custom adapters
- **[Contributing Guide](development-setup.md)** - Contribution workflow
- **[CI/CD](../deployment/cicd.md)** - Continuous integration setup
