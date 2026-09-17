# Integration Tests

This document describes the integration test suite for the Paladin workspace: test
ownership, service requirements, how to run tests locally, and how services are
provisioned in CI.

---

## 1. Test Ownership and Service Requirements

All integration tests live at `tests/integration/` (workspace root). Every file
imports from at least the `paladin` facade crate, and most also import
`paladin-ports` traits directly. No file is a candidate for relocation into a
per-crate `tests/` directory because all tests exercise cross-crate behaviour
through the public API surface.

The `tests/integration/battalion/` sub-module contains battalion-specific tests
and is declared from `tests/integration/mod.rs`.

### Main test files

| Test File | Crate Scope | Services Required | Feature Gate |
|-----------|-------------|-------------------|--------------|
| `aegis_retry_stress_test.rs` | `paladin`, `paladin-ports` | SQLite (temp file) | — |
| `anthropic_provider_test.rs` | `paladin` | live-api (Anthropic key) | `llm-anthropic` |
| `arsenal_bridge_regression_test.rs` | `paladin`, `paladin-ports` | none | — |
| `arsenal_execution_integration_test.rs` | `paladin`, `paladin-ports` | none | — |
| `arsenal_registry_integration_test.rs` | `paladin`, `paladin-ports` | none | — |
| `autonomous_planning_test.rs` | `paladin`, `paladin-ports` | none | — |
| `battalion_campaign_integration_test.rs` | `paladin`, `paladin-ports` | none | — |
| `battalion_chain_of_command_herald_test.rs` | `paladin`, `paladin-ports` | none | — |
| `battalion_chain_of_command_integration_test.rs` | `paladin`, `paladin-ports` | none | — |
| `battalion_herald_end_to_end_test.rs` | `paladin`, `paladin-ports` | none | `cli` |
| `citadel_integration_test.rs` | `paladin`, `paladin-ports` | none | — |
| `cli_integration_test.rs` | `paladin` | live-api | `cli` |
| `cli_real_providers_test.rs` | `paladin` | live-api | `cli` |
| `cli_real_services_test.rs` | `paladin` | Redis, MinIO | `cli` |
| `commander_error_paths_test.rs` | `paladin`, `paladin-ports` | none | — |
| `commander_integration_tests.rs` | `paladin`, `paladin-ports` | none | — |
| `context_injection_test.rs` | `paladin`, `paladin-ports` | none | — |
| `deepseek_provider_test.rs` | `paladin` | live-api (DeepSeek key) | `llm-deepseek` |
| `e2e_approval_gate_test.rs` | `paladin`, `paladin-ports` | SQLite (temp file) | — |
| `e2e_compensation_chain_test.rs` | `paladin`, `paladin-ports` | SQLite (temp file) | — |
| `e2e_crash_resume_test.rs` | `paladin`, `paladin-ports` | SQLite (temp file) | — |
| `e2e_muster_defer_order_test.rs` | `paladin`, `paladin-ports` | SQLite (temp file) | — |
| `e2e_platform_api_test.rs` | `paladin`, `paladin-ports` | none (mockito) | `web-server` |
| `file_storage_integration_tests.rs` | `paladin`, `paladin-ports` | MinIO | `s3-storage` |
| `golden_bridge_equivalence_test.rs` | `paladin` | none | — |
| `herald_integration_test.rs` | `paladin`, `paladin-ports` | none | — |
| `in_memory_sanctum_tests.rs` | `paladin`, `paladin-ports` | none | — |
| `llm_live_api_tests.rs` | `paladin`, `paladin-ports` | live-api | `live-api-tests` |
| `mcp_stdio_test.rs` | `paladin` | none | — |
| `mcp_streamable_http_test.rs` | `paladin` | none (hermetic, in-process rmcp server) | — |
| `mcp_streamable_http_live_test.rs` | `paladin` | live-api (`ETHERSCAN_API_KEY`), `#[ignore]`'d | — |
| `middleware_under_engine_test.rs` | `paladin`, `paladin-ports` | none | — |
| `multi_parley_suspension_test.rs` | `paladin`, `paladin-ports` | SQLite (temp file) | — |
| `notification_system_integration_test.rs` | `paladin`, `paladin-ports` | none | — |
| `ollama_docker_test.rs` | `paladin`, `paladin-ports` | Ollama (Docker) | `integration-tests`+`llm-ollama` |
| `openai_content_analysis_integration_test.rs` | `paladin`, `paladin-ports` | none (mock) | `llm-openai` |
| `openai_embedding_tests.rs` | `paladin`, `paladin-ports` | none (mock) | `openai-embeddings` |
| `openai_provider_test.rs` | `paladin` | live-api (OpenAI key) | `llm-openai` |
| `orchestrator_workflow_lifecycle_test.rs` | `paladin` | none | — |
| `otel_transport_test.rs` | `paladin`, `paladin-ports` | none (hermetic OTLP/HTTP) | `otel` |
| `paladin_garrison_integration_test.rs` | `paladin`, `paladin-ports` | none | — |
| `paladin_integration_test.rs` | `paladin`, `paladin-ports` | none | — |
| `parley_resume_stress_test.rs` | `paladin`, `paladin-ports` | SQLite (temp file) | — |
| `provider_switching_test.rs` | `paladin`, `paladin-ports` | none (mockito) | — |
| `qdrant_sanctum_tests.rs` | `paladin`, `paladin-ports` | Qdrant | `qdrant` |
| `rag_commissary_test.rs` | `paladin`, `paladin-ports` | none (mock embedding) | — |
| `rag_integration_tests.rs` | `paladin` | Qdrant | `qdrant` |
| `reasoning_agent_test.rs` | `paladin`, `paladin-ports` | none | — |
| `redis_queue_integration_test.rs` | `paladin` | Redis | `redis-queue` |
| `scheduler_integration_test.rs` | `paladin`, `paladin-ports` | none | — |
| `sqlite_garrison_integration_test.rs` | `paladin`, `paladin-ports` | SQLite (temp file) | — |
| `structured_engine_node_test.rs` | `paladin`, `paladin-ports` | none | — |
| `subgraph_formation_in_campaign_test.rs` | `paladin`, `paladin-ports` | none | — |
| `system_log_integration_test.rs` | `paladin`, `paladin-ports` | none | — |
| `v0_9_config_boot_test.rs` | `paladin` | none | `web-server` |
| `vault_confinement_test.rs` | `paladin`, `paladin-ports` | none | — |
| `vision_integration_test.rs` | `paladin`, `paladin-ports` | live-api | `vision`+`llm-openai`+`llm-anthropic` |
| `war_engine_tracer_test.rs` | `paladin`, `paladin-ports` | none | — |
| `waypoint_retention_fault_injection_test.rs` | `paladin`, `paladin-ports` | none | — |

### Battalion sub-module (`tests/integration/battalion/`)

| Test File | Services Required |
|-----------|-------------------|
| `campaign_integration_test.rs` | none |
| `chain_of_command_integration_test.rs` | none |
| `council_integration_test.rs` | none |
| `formation_integration_test.rs` | none |
| `grove_integration_test.rs` | none |
| `load_test.rs` | none |
| `phalanx_integration_test.rs` | none |

### Service legend

| Symbol | Meaning |
|--------|---------|
| none | In-memory / mock only; no external process needed |
| Redis | Requires a Redis 7 instance |
| MinIO | Requires MinIO (S3-compatible object storage) |
| SQLite | Uses a `tempfile::NamedTempFile`; no external service needed |
| Qdrant | Requires a Qdrant vector-database instance |
| live-api | Requires real provider API keys (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, or `DEEPSEEK_API_KEY`); skipped in normal CI |

---

## 2. Running Integration Tests Locally

### Prerequisites

- Rust stable toolchain
- Docker (for Redis / MinIO when running service-dependent tests)
- `docker compose` v2 plugin (`docker compose version` must succeed)

### Option A — All integration tests (mock/in-process only)

```bash
cargo test --workspace --features integration-tests -- --test-threads=1
```

This runs every test that does not require an external service. Tests gated
behind `live-api-tests`, `qdrant`, etc. are excluded unless the corresponding
feature is enabled.

### Option B — With Redis and MinIO (docker-compose)

Start the test infrastructure, then run:

```bash
# Start services
docker compose -f docker/docker-compose.test.yml up -d redis-test minio-test minio-test-init

# Wait for minio-test-init to finish creating buckets
until docker inspect paladin-minio-test-init --format="{{.State.Status}}" 2>/dev/null | grep -q exited; do sleep 2; done

# Run tests (all features that need services are enabled by default)
USE_EXTERNAL_TEST_SERVICES=true \
TEST_REDIS_HOST=localhost TEST_REDIS_PORT=6380 \
TEST_MINIO_ENDPOINT=localhost:9010 \
TEST_MINIO_ACCESS_KEY=testuser TEST_MINIO_SECRET_KEY=testpass123 \
cargo test --workspace --features integration-tests -- --test-threads=1

# Tear down
docker compose -f docker/docker-compose.test.yml down -v
```

Or use the helper script which handles all of the above:

```bash
./scripts/run_integration_tests.sh -m docker -v
```

### Option C — Specific test files or patterns

```bash
# Run only SQLite garrison tests
cargo test --workspace --features integration-tests sqlite_garrison -- --test-threads=1

# Run only Redis queue tests
cargo test --workspace --features integration-tests,redis-queue redis_queue -- --test-threads=1

# Run only MinIO file storage tests
cargo test --workspace --features integration-tests,s3-storage file_storage -- --test-threads=1
```

### Option D — Per-crate test targets (Makefile)

```bash
make test-core          # paladin-core unit + integration tests
make test-ports         # paladin-ports
make test-battalion     # paladin-battalion
make test-llm           # paladin-llm
make test-memory        # paladin-memory
make test-storage       # paladin-storage
make test-notifications # paladin-notifications
make test-content       # paladin-content
make test-web           # paladin-web
make test-facade        # paladin (root crate / facade)
```

### Makefile convenience targets

```bash
make test-integration          # local mode (uses testcontainers)
make test-integration-docker   # docker-compose mode (starts services automatically)
make test-integration-redis    # Redis tests only
make test-integration-minio    # MinIO tests only
```

---

## 3. CI Service Provisioning

### Integration Tests job (`.github/workflows/integration-tests.yml`)

The `integration-tests` job uses GitHub-native **service containers**:

| Service | Image | Port |
|---------|-------|------|
| Redis | `redis:7-alpine` | `localhost:6379` |
| MinIO | `quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772` | `localhost:9000` |

The job runs:

```bash
cargo test --workspace --features integration-tests --verbose -- --test-threads=1
```

Environment variables passed to the test binary:

| Variable | Value |
|----------|-------|
| `REDIS_URL` | `redis://localhost:6379` |
| `MINIO_ENDPOINT` | `localhost:9000` |
| `MINIO_ACCESS_KEY` | `minioadmin` |
| `MINIO_SECRET_KEY` | `minioadmin` |
| `MINIO_USE_SSL` | `false` |

### Docker Integration Tests job

The `docker-integration` job builds the test image from `docker/testserver/Dockerfile`
(`test` stage) and runs tests inside the container using `docker/docker-compose.test.yml`.

Services started:

| Service | Container Name | Purpose |
|---------|---------------|---------|
| `redis-test` | `paladin-redis-test` | Redis 7 on port 6380 (host) |
| `minio-test` | `paladin-minio-test` | MinIO on port 9010 (host) |
| `minio-test-init` | `paladin-minio-test-init` | Creates test buckets, then exits |

The test container (`paladin-integration-tests`) runs:

```bash
cargo test --features integration-tests -- --test-threads=1 --nocapture
```

The test image includes:
- `Cargo.toml` / `Cargo.lock`
- `src/`, `crates/`, `tests/`
- `migrations/` (required by `SqliteGarrison` at runtime via `sqlx::migrate`)
- `config.test.yml` (required by `test_load_from_file_regression`)

### Live-API tests

Tests guarded by `live-api-tests`, `llm-openai`, `llm-anthropic`, `llm-deepseek`,
or `qdrant` features are **not run in CI** (API keys are not available in the
public workflow). They are intended for manual verification or a separate
secrets-aware workflow.
