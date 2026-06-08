# Tasks: Configurable Web Host & Server Binary (Milestone 12, Epic 2)

**PRD:** [prd-configurable-web-host-server-binary.md](prd-configurable-web-host-server-binary.md)
**Crate:** `paladin-ai` (facade / composition root) — reuses `paladin-web` (Epic 1)
**Status:** Phase 2 — sub-tasks expanded, ready for implementation

---

## Relevant Files

- `src/config/agents.rs` - **New.** `AgentDefinition` config struct (id, model, system_prompt, optional provider/temperature/max_loops/stop_words) + unit tests. Wired into `Settings`.
- `src/config/settings.rs` - **Modify.** Add an `agents: Vec<AgentDefinition>` (or `Option<…>`) field to `Settings`; the bind address reuses the existing `server` (`host`/`port`) section from `src/config/web_server.rs`.
- `src/infrastructure/web/agent_host.rs` - **New.** The registry-from-config builder (`build_agent_registry`) and the per-agent build helper (`build_agent`) shared with the provisioner. Unit tests in-file.
- `src/infrastructure/web/facade_provisioner.rs` - **New.** Concrete `AgentProvisioner` impl (`provision(&AgentSpec)`) reusing `build_agent`. Unit tests in-file.
- `src/infrastructure/web/mod.rs` - **New/Modify.** Module wiring for the two files above.
- `src/bin/paladin-server.rs` - **New.** The `paladin-server` binary: load config → build registry + `AgentApiState` (with provisioner) → `agent_router` → `axum::serve` with graceful shutdown + startup diagnostics.
- `Cargo.toml` - **Modify.** Add `[[bin]] name = "paladin-server"`, `required-features = ["web-server"]`; add any binary-only deps behind that feature if needed (e.g. `tokio` signal — already present).
- `config.example.yml` (or extend `config.yml`) - **Modify/Add.** A documented `host` + `agents` example for the server.
- `tests/paladin_server_smoke.rs` - **New.** Integration smoke test: boot the server on `127.0.0.1:0` with a hermetic mock provider; assert `GET /agents` and `POST /agents/{id}/execute`.
- `CHANGELOG.md` - **Modify.** `[Unreleased]` entry for the configurable host + `paladin-server` binary.

### Notes

- **TDD (Red-Green-Refactor):** write the failing test first for each behavior-bearing sub-task.
- Rust unit tests live in-file under `#[cfg(test)] mod tests { ... }`; the boot smoke test is an integration test in `tests/`.
- Run with `cargo test` (or `cargo test --features web-server` for the server paths). Before
  committing a parent task: `cargo test` → `cargo fmt --check` → `cargo clippy -- -D warnings` → `make deny`.
- **Composition-root rule:** all new code lives in the **facade crate** (`paladin-ai`). It may
  depend on both `paladin-web` and `PaladinExecutionService`/LLM adapters. `paladin-web` gains **no**
  new dependency (Epic 1 is reused unchanged) — verify the dependency direction is preserved.
- **Reused (verified) building blocks:** `Settings` (`src/config/settings.rs`, `ServerConfig` has
  `host`+`port`); the `paladin-llm` provider factory (`create(provider) -> Arc<dyn LlmPort>`,
  `get_default_provider`, `list_available_providers`); `PaladinBuilder`
  (`name/system_prompt/model/temperature/max_loops`); `PaladinExecutionService::new(llm, breaker,
  None, None)`; `CircuitBreaker`; `paladin_web::{AgentRegistry, AgentApiState, AgentProvisioner,
  AgentSpec, ProvisionError, agent_router}`.
- **Out of scope** (later epics): user/auth/delivery routes, auth, garrison/arsenal, streaming,
  health/CORS/error-model, OpenAPI, Docker/k8s/TLS, config hot-reload.

## Tasks

- [x] 0.0 Create feature branch
  - [x] 0.1 Created and checked out `feature/m12-epic2-configurable-web-host-server-binary`, branched from the Epic 1 branch (not yet merged to `main`). Epic 2 PRD/tasks committed on it.
  - [x] 0.2 Clean baseline confirmed: `cargo build --features web-server` succeeds; `cargo test -p paladin-web` → 62 + 5 pass.

- [x] 1.0 Add the `agents` configuration schema and wire it into `Settings`
  - [x] 1.1 Created `src/config/agents.rs` with `AgentDefinition` (`id`/`model`/`system_prompt` required; `provider`/`temperature`/`max_loops`/`stop_words` optional via `#[serde(default)]`), fully documented.
  - [x] 1.2 **(Test first)** Unit tests: full deserialize, minimal deserialize (asserts defaults: `provider`/`temperature`/`max_loops` `None`, `stop_words` empty), and missing-required-field → error. (Lenient parsing; no `deny_unknown_fields`.) Bind-address derivation from `server` (`host`+`port`) is covered in task 4 (server binary) where it is used.
  - [x] 1.3 Added `#[serde(default)] pub agents: Vec<AgentDefinition>` to `Settings` (empty when `agents:` absent — non-server configs unaffected) and re-exported `AgentDefinition` from `config::mod`. Existing config loading unchanged.
  - [x] 1.4 Updated the `Settings` `Default` impl and the `user_config.rs` test fixture (`agents: Vec::new()`); workspace config tests pass (48); rustdoc on the new field. `fmt`/`clippy -D warnings` clean.

- [x] 2.0 Build the registry-from-config builder (facade `agent_host`)
  - [x] 2.1 Added `src/infrastructure/web/agent_host.rs`, declared `pub mod agent_host;` in `src/infrastructure/web/mod.rs` (behind `#[cfg(feature = "web-server")]`).
  - [x] 2.2 **(Test first)** Split for hermetic testing: `resolve_provider` (precedence test), `build_agent_with_llm` (builds a `Paladin` with prompt/model/temperature/max_loops via `MockLlmAdapter` — no keys), and `build_agent` (provider resolution; unknown-provider → `HostBuildError::Provider`).
  - [x] 2.3 Implemented `build_agent_with_llm`/`build_agent` using `PaladinBuilder` + the `paladin-llm` `LlmProviderFactory` + `PaladinExecutionService::new(llm, breaker, None, None)`. `HostBuildError` (`thiserror`: `Provider`/`Build`/`DuplicateId`).
  - [x] 2.4 **(Test first)** `register_built` duplicate-id test (`MockLlmAdapter`, hermetic) → `HostBuildError::DuplicateId`; `build_agent_registry` empty-config test. (Full multi-agent config-load needs real provider keys, so it is covered via component tests + the Epic-6/Task-6 smoke path rather than a key-dependent unit test.)
  - [x] 2.5 Implemented `build_agent_registry(&Settings)`: resolves the default provider (`settings.llm.default_provider` → factory default → `"openai"`), iterates `settings.agents`, builds each via `build_agent`, inserts via `register_built` (duplicate → error).
  - [x] 2.6 Rustdoc on all public items; `fmt`/`clippy --all-targets -D warnings` clean; 5 tests pass.

- [x] 3.0 Implement the concrete `AgentProvisioner` (facade) for runtime registration
  - [x] 3.1 Created `src/infrastructure/web/facade_provisioner.rs` with `FacadeProvisioner` (holds `LlmProviderFactory` + default provider + `Arc<CircuitBreaker>`); `new(..)` and `from_settings(&Settings)` constructors.
  - [x] 3.2 **(Test first)** Unit tests: `spec_to_definition` mapping (id/model/system_prompt/temperature/stop_words carried; provider/max_loops default), and `provision` with an unknown provider → `ProvisionError` (hermetic, no keys). (Success path is covered by the shared `build_agent_with_llm` tests in 2.0 — it routes through the real factory here.)
  - [x] 3.3 Implemented `#[async_trait] impl AgentProvisioner for FacadeProvisioner`: maps `AgentSpec` → `AgentDefinition` → `build_agent`, with `HostBuildError::Build → ProvisionError::InvalidSpec` and others → `ProvisionError::Failed`.
  - [x] 3.4 Rustdoc; `build_agent` confirmed as the single shared build path (config-load and runtime provisioning); `fmt`/`clippy --all-targets -D warnings` clean; 2 tests pass.

- [x] 4.0 Add the `paladin-server` binary (load → build → serve → graceful shutdown)
  - [x] 4.1 Added `[[bin]] name = "paladin-server"` (`required-features = ["web-server"]`) to `Cargo.toml`; added an **optional `axum`** dep gated by the `web-server` feature (`web-server = ["dep:paladin-web", "dep:axum"]`) so the binary can call `axum::serve` without pulling axum into default builds.
  - [x] 4.2 Implemented `main`/`run`: `env_logger` init + `dotenv` (debug) like `main.rs`; load `Settings`; `build_agent_registry`; `AgentApiState::new(..).with_provisioner(FacadeProvisioner::from_settings(..))`; `agent_router(state)`.
  - [x] 4.3 Binds `TcpListener` to `server.host:port` and `axum::serve(..).with_graceful_shutdown(shutdown_signal())`. Verified end-to-end: boots, `GET /agents` → `[]`, unknown `POST …/execute` → `404`, logs bound address + routes.
  - [x] 4.4 `shutdown_signal()` selects on `tokio::signal::ctrl_c()` and, on Unix, a `SIGTERM` stream.
  - [x] 4.5 No secrets logged; `run()` returns `Result`, and `main` logs + `process::exit(1)` on startup failure (verified: missing config/provider key fails fast with a clear message).

- [x] 5.0 Add startup validation and diagnostics (fail-fast + route/address logging)
  - [x] 5.1 Added `validate_config(&Settings)` (key-free pre-flight) called at the top of `build_agent_registry`: non-empty `id`/`model`/`system_prompt`, no duplicate ids, and resolved provider ∈ `list_available_providers()` (catches typos/unavailable providers without needing keys). New `HostBuildError::{UnknownProvider, InvalidAgent}` variants; provider-key failures still surface in `build_agent`. Added `bind_address(&Settings)` helper.
  - [x] 5.2 Binary now logs the bound address and a summary including the **sorted agent ids** plus the route paths.
  - [x] 5.3 **(Test first)** Function-level unit tests (no live bind): `bind_address` formatting, validate passes (empty agents), and rejects empty required field / duplicate ids / unknown provider. 10 `agent_host` tests pass; binary builds; `fmt`/`clippy -D warnings` (lib + bin) clean.

- [x] 6.0 Tests: config parsing, builder, provisioner, and a boot smoke integration test
  - [x] 6.1 Unit coverage in place: config parsing (1.0, 3 tests), `build_agent`/`build_agent_with_llm`/`resolve_provider`/`register_built`/validate (2.0+5.0, 10 tests), `provision`/spec-mapping (3.0, 2 tests).
  - [x] 6.2 **(Test first)** Added `tests/paladin_server_smoke.rs` (gated `#![cfg(feature = "web-server")]`): builds a hermetic `MockLlmAdapter`-backed agent via public API, serves on `127.0.0.1:0` with `axum::serve`, and asserts over real `reqwest` HTTP: `GET /agents` → `200` (1 agent), `POST /agents/researcher/execute` → `200` with an `output` string, unknown id → `404`. No network/keys.
  - [x] 6.3 Graceful-shutdown asserted: a `oneshot` triggers `with_graceful_shutdown`; the spawned server task is awaited and must join cleanly after the signal.

- [x] 7.0 Finalize: sample config, docs, CHANGELOG, and quality gates
  - [x] 7.1 Added a tracked, runnable **`config.example.yml`** (host + `agents:` shape; keys via env, not the file). `config.yml` is gitignored, so the example lives in a committed file; verified it boots `paladin-server` serving 2 agents. (`README` links to it.)
  - [x] 7.2 Added a "Running agents behind an HTTP API" subsection to `README.md` with the `cargo run --bin paladin-server --features web-server` command, env-var note, route list, and the unauthenticated caveat.
  - [x] 7.3 Full gate green: `cargo test --features web-server` (all crates pass; facade lib 650, smoke 1), `cargo fmt --check`, `cargo clippy --workspace --all-targets --features web-server -- -D warnings`, `make deny` (advisories/bans/licenses/sources ok). No debug prints.
  - [x] 7.4 API-surface check: the `agents` config schema is **not** `web-server`-gated, so `paladin::config::AgentDefinition` + `Settings::agents` are legitimate **additive** default-surface items. Regenerated `project/current-exports.txt` (additive-only, zero removals); check now passes (1827 items).
  - [x] 7.5 Added a `CHANGELOG.md [Unreleased]` entry (Milestone 12 — Epic 2): config schema, builder, provisioner, `paladin-server` binary, optional axum dep.
  - [x] 7.6 Committed; parent tasks complete; **stop for go-ahead**.
