# Changelog

All notable changes to the Paladin project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Workspace MSRV floor raised from 1.85 to 1.88 (X-11.2 stop-and-flag resolution).** The
  declared 1.85 floor could not actually be satisfied: the exact `rmcp = "=2.1.0"` pin's
  `transport-child-process` feature reaches `process-wrap ^9.0`, whose every release requires
  rustc >= 1.87, and clearing RUSTSEC-2026-0009 requires `time` >= 0.3.47, which requires rustc
  >= 1.88. 1.88 is the lowest floor that satisfies both without a new security exception. The
  floor now lives in exactly one place — `workspace.package.rust-version = "1.88"` in root
  `Cargo.toml` — inherited by all ten crate manifests via `rust-version.workspace = true`, and
  `[workspace] resolver = "3"` guards against a future `cargo update` silently re-resolving above
  it. See [`MIGRATION.md` §9.3](MIGRATION.md#93-toolchain--dependencies) for the full dependency
  chain and the two rejected alternatives.

- **Custom edge conditions now fail closed instead of always-routing (M-B-01, BUG-01 fix).**
  `EdgeCondition::Custom(name)` previously evaluated to `true` on every run when no evaluator was
  registered — the edge silently always fired regardless of what the source Paladin actually
  produced. Both consumers (`CampaignExecutionService` and the `WarEngine`) now reject an
  unregistered `Custom` name at validation time, before any node executes, naming every offender.
  Register an evaluator via `CampaignExecutionService::with_evaluator` /
  `WarEngine::with_edge_evaluator`, or replace the condition with `Contains`/`Regex`/`Always`. See
  [`MIGRATION.md` §9.1, M-B-01](MIGRATION.md#91-behavioral-changes-user-visible-without-code-changes)
  for the worked before/after example.

- **Graph fingerprint bumped `v3` → `v4` (HITL-01, D-09).** The new `Gate` node's
  routing-relevant properties (`kind`, `output_field`, `choices`, `on_expire`'s discriminant) are
  now part of the hashed graph shape. A thread suspended under a `v3` fingerprint fails closed
  with `EngineError::GraphMismatch` on resume rather than being silently reinterpreted under the
  new hash — resume it against the graph it suspended with, or restart the run under the
  `v4`-fingerprinted graph.

### Added

- **Node-driven `Directive` routing (CF-02).** A `StateNode::run` now returns a `Directive` — its
  `StateDelta` plus a `NextStep` (`Edges`, `Goto`, `Muster`, `End`, or the not-yet-implemented
  `Parley`) — letting a node author its own routing instead of relying solely on static graph
  edges. `NextStep::Edges` is the default and reproduces pre-CF-02 behavior exactly.
  `NodeSpec::Paladin` nodes opt in via the new `DirectiveParser` (`PlainOutput` default,
  `StructuredDirective` for a documented JSON envelope). See the
  [Control Flow guide](docs/src/user-guides/control-flow.md).
- **Muster: dynamic worker fan-out (CF-03).** `NextStep::Muster` dispatches N worker tasks to a
  node registered via the new `WarGraph::add_worker_template`, each with an isolated payload
  (never merged into the Battlefield) addressed through the new `{muster.payload}` /
  `{muster.task_key}` template namespace. Bounded by the new `EngineLimits::max_muster_tasks`
  (default 100, configurable via `EngineConfig`/`APP_ENGINE_MAX_MUSTER_TASKS`).
- **Subgraph composition via `NodeSpec::Battalion` (CF-04).** A child `WarGraph` can be embedded
  as a single parent node, running to completion within one parent superstep and exchanging state
  only through the new `StateMap`'s declared `inputs`/`outputs` field pairs — everything else the
  child touches stays private.
- **LLM-evaluated edge routing and Commander Semantic strategy selection (CF-05).** The new
  `LlmDecisionEvaluator` resolves an `EdgeCondition::Custom` edge from a live model's answer
  against a closed, author-declared choice list, asking once per decision per superstep. Commander
  gained `StrategySelection::Semantic`, which prompts a model to name a Battalion strategy and
  falls back to the existing heuristic deterministically on any error or unrecognized answer. Both
  are off by default and reached only by constructing one in code — no environment variable or
  config field enables either.
- **`EngineConfig` (`src/config/engine.rs`).** Configures the `WarEngine`'s `EngineLimits` and
  `WaypointDurability`: `max_supersteps`, `max_node_visits`, `run_timeout_secs`,
  `waypoint_durability`, and `max_muster_tasks`, each with an `APP_ENGINE_*` environment override.
  `EngineConfig::default()` converts to exactly today's `EngineLimits::default()` and
  `WaypointDurability::Strict`, so a v0.9 configuration file boots v0.10 with identical behavior.

- **Waypoint retention is a public application-layer service.** `WaypointRetentionService`
  (`application::services::waypoint_retention`) owns the single definition of a protected
  Waypoint — a thread's latest plus every `AwaitingInput` entry — and passes the keep-set into
  the storage routine, which prunes on `WaypointPort::prune_thread` instead of the former
  delete-then-resave loop. `WaypointRetentionConfig` (`config::waypoint_retention`) configures
  it, with env overrides and validation. Additive API only (no pre-existing item changed);
  API-surface baseline regenerated to match.

- **Human-in-the-loop pause and resume: the `Gate` node and typed `resume_with` (HITL-01,
  HITL-02).** `NextStep::Parley` now really suspends a run instead of failing it: the emitting
  superstep's deltas merge, a Waypoint with `status: AwaitingInput { parleys, responses }` is
  persisted, every resource for the run is released, and the run survives full process
  termination — resumable from a different process instance over the same `WaypointPort`. A new
  first-class `NodeSpec::Gate` node renders its prompt/payload from the Battlefield and is the
  primary approval-gate building block: one `Gate` plus two conditional edges is a complete
  approval gate. `WarEngine::resume_with(graph, thread, responses)` validates every submitted
  response totally before persisting anything (unknown/already-answered/wrong-shape/expired, each
  a distinct typed `EngineError`), persists a valid-but-partial submission as a same-superstep
  `AwaitingInput` Waypoint chain, and evaluates each request's `on_expire` policy
  (`FailRun` | `ResumeWithDefault`) lazily at resume time — no background timer. A Paladin node can
  also raise a parley through the structured directive envelope's `next.parley` key and read the
  answer back through a new `parley.` `InputMapping` namespace, reserved at graph-validation time
  exactly like the existing `muster.` namespace. See the
  [Parley & Chronicle guide](docs/src/user-guides/parley-and-chronicle.md).
- **Chronicle: inspectable, forkable execution history (HITL-03).** `Waypoint`/`WaypointSummary`
  gain an additive `fork_of: Option<WaypointId>` branch marker; `WarEngine::replay`/
  `WarEngine::fork` re-enter the superstep loop from any past Waypoint, each producing a new
  branch while the original chain stays byte-identical (a hard, tested invariant) — `fork` merges
  a caller-supplied `StateDelta` edit before the first forked superstep, letting a "what-if" edit
  flip a conditional edge's routing. `ChronicleService::{history, inspect, latest_on_branch}`
  exposes this as a thin, port-only read facade with no engine dependency. A branch's
  `NodeSpec::Battalion` children run under `ThreadId::child_on_branch`, so a fork's subgraph
  children never share Waypoints with the mainline's.
- **Graceful shutdown (HITL-04).** A mid-superstep cancellation now races the whole in-flight
  batch of node tasks against one shared `shutdown_grace` deadline (default 30s, tunable via
  `EngineConfig`) instead of a per-node timeout; a node still running past the deadline is aborted
  and recorded `NodeOutcomeKind::Skipped { reason: "shutdown" }`, its id re-listed in the Halted
  Waypoint's vanguard so `resume` re-executes it exactly once. `ShutdownCoordinator`
  (`paladin-battalion::engine::shutdown`) tracks every in-flight run; both `paladin-server`'s
  `shutdown_signal` and `ServiceRunner::wait_for_shutdown` cancel the same coordinator on
  SIGTERM/SIGINT and wait up to `shutdown_grace` for the batch to drain. Two new env vars:
  `APP_ENGINE_SHUTDOWN_GRACE_SECS` (default `30`) and `APP_ENGINE_GRACEFUL_SHUTDOWN` (default
  `true`; set `false` to restore the old immediate-exit behavior). Both shipped Kubernetes
  manifests now declare `terminationGracePeriodSeconds: 60` (2× the default grace). See
  [`MIGRATION.md` §9.1, M-B-02](MIGRATION.md#91-behavioral-changes-user-visible-without-code-changes)
  for the worked before/after example.
- **Threads over HTTP (HITL-05).** `paladin-web` gains three routes behind the same
  authentication middleware `/v1/agents/*` already uses: `GET /v1/threads/{id}/state`,
  `POST /v1/threads/{id}/resume` (returns `202 Accepted { thread_id, state_url }` immediately —
  the engine continuation runs as a background task, never holding the connection open; a client
  polls `.../state` for the outcome), and `GET /v1/threads/{id}/history` (paginated, `limit` ≤
  100, opaque cursor). Backed by a new `ParleyPort` (`paladin-ports`) with zero `paladin-battalion`
  dependency; `paladin-server` wires a real backend via the new `WaypointStoreConfig`
  (`APP_WAYPOINT_STORE_BACKEND=sqlite|postgres`, disabled by default — every thread route answers
  `501 not_implemented` naming the config key until an operator sets it). `openapi.json`
  regenerated with the three new paths; every pre-existing `/v1/agents/*` path is unchanged.

## [0.9.0] - 2026-09-01

**First stable release since 0.5.1 (2026-06-04), and the first cut by the rebuilt release
pipeline.** Versions 0.6.0, 0.7.0 and 0.8.0 were finalized in this changelog but never tagged or
published; 0.8.1 existed only as the rc.1–rc.5 prerelease line used to prove the pipeline work
below (their sections follow this one). From this release forward, release version numbers are
reconciled with the project's planning milestones — 0.9.0 is the "Security Tooling" milestone.

### Security

- **Publishing to crates.io no longer uses a long-lived credential.** The `publish-crates` job
  mints a short-lived token per run from its GitHub OIDC identity via crates.io Trusted
  Publishing, under a `crates-io` GitHub Environment restricted to `v*.*.*` tags. The old
  publish-scoped token was revoked and the `CARGO_REGISTRY_TOKEN` repository secret deleted
  (both 2026-08-27, operator-confirmed at the registry); the branch that let a release finish
  green while silently publishing nothing was removed. All eleven publishable crates —
  `paladin-herald` now included in the publish order — carry a Trust Publisher Configuration.
  See [`docs/src/appendix/release-automation.md`](docs/src/appendix/release-automation.md).
- **CodeQL Rust static analysis runs on every pull request, advisory-only.** The scanner was
  measured against a five-class planted-vulnerability probe before any adoption decision and
  **disqualified as a merge-gating Rust SAST** at CodeQL 2.26.3 / `rust-queries` 0.1.40 (SQL
  injection, path traversal and regex injection never fired, across four independent
  measurements with 385/385-file coverage proven on every run). The scan is retained for its one
  reliably-working class (hardcoded credentials) behind a governed dismissal register
  (`.github/CODEQL-DISMISSALS.md` + offline guard); the manual credential-handling review
  remains the primary control. See `.github/instructions/security.instructions.md`.

### Added

- **A pre-publish consistency gate.** No crate is published until the tag, all eleven manifest
  versions, the root and ten per-crate changelog sections, and the tagged commit's recorded CI
  conclusion agree (`scripts/check-release-consistency.sh`, run as a `release.yml` job that
  `publish-crates` structurally cannot bypass, and locally via `make check-release-consistency`).
  Every mismatch is reported in one pass.
- **Idempotent release re-runs and a recovery runbook.** `create-release` looks the GitHub
  release up by tag and reuses it (no more hard failure on retry); already-published crates are
  detected from crates.io registry state with a bounded index-visibility poll (replacing
  error-prose matching and a fixed `sleep 20`); the publish job emits a per-crate outcome table
  and fails a run in which no crate moved. The stuck-halfway runbook, including the yank policy,
  is at [`docs/src/appendix/release-recovery.md`](docs/src/appendix/release-recovery.md) — and
  was proven by live recovery rehearsals (0.8.1-rc.3/rc.4).
- **Real, verifiable release artifacts.** The release body is extracted from this changelog's
  section for the tagged version (a missing section fails the run — no commit-log fallback);
  `paladin`, `paladin-cli` and `paladin-server` are built with the features their targets
  require, with existence asserts before archiving; the container image is referenced by its
  immutable `sha256:` digest; an aggregated `SHA256SUMS` ships with one-command verification
  instructions; and the attached CycloneDX SBOM is identified as covering the root `paladin-ai`
  package. Proven end-to-end on 0.8.1-rc.5 — the first fully-green release run in this
  project's history.

### Changed

- **The release workflow no longer uses archived actions.** `actions/create-release@v1` and
  `actions/upload-release-asset@v1` (archived upstream since 2021) are replaced by `gh`-CLI
  calls, and the `upload_url` plumbing that served them is removed. Signing/build provenance was
  examined and deferred with recorded reasoning (`actions/attest-build-provenance` is the named
  future candidate).
- **Notable items consolidated from the 0.8.1-rc prerelease line** (full detail in the rc
  sections below): the Qwen adapter's default `base_url` is the Singapore (international)
  DashScope endpoint with `DASHSCOPE_BASE_URL` as the region override, Qwen's declared
  temperature range narrowed to the measured `[0.0, 1.99]`, and the facade's `llm-*` provider
  flags now genuinely gate their adapters (the default build still compiles `openai`,
  `anthropic`, `deepseek` — no consumer action required; ADR-0046).

## [0.8.1-rc.5] - 2026-08-31

## [0.8.1-rc.4] - 2026-08-29

## [0.8.1-rc.3] - 2026-08-28

### Security

- **Publishing to crates.io no longer uses a long-lived repository secret.** The `publish-crates`
  job in `.github/workflows/release.yml` now mints a short-lived (~30 minute) token per run from
  its own GitHub OIDC identity via crates.io Trusted Publishing, under a `crates-io` GitHub
  Environment restricted to `v*.*.*` tags. The old publish-scoped crates.io token ("Paladin") was
  revoked and the `CARGO_REGISTRY_TOKEN` repository secret was deleted, both on 2026-08-27; the
  branch that let a release finish green while silently publishing nothing (an absent secret
  skipping the publish job) was removed rather than reworded — that behavior no longer exists.
  `paladin-herald`, previously absent from the publish order, is now the eleventh crate in it, and
  all eleven crates carry a crates.io Trust Publisher Configuration pointing at this workflow and
  environment. See the credential history in
  [`docs/src/appendix/release-automation.md`](docs/src/appendix/release-automation.md#credential-history)
  for the full record.

## [0.8.1-rc.2] - 2026-08-27

## [0.8.1-rc.1] - 2026-08-26

### Changed

- **The Qwen (Alibaba DashScope) adapter's shipped default `base_url` is the Singapore
  (international) endpoint.** `QWEN_DEFAULT_BASE_URL` resolves to
  `https://dashscope-intl.aliyuncs.com/compatible-mode/v1`. This constant moved twice inside one
  week: Singapore → US (Virginia) on 2026-08-22, then back to Singapore on 2026-08-23, when the
  project's own credential was replaced with a Singapore-scoped key and the Virginia default it
  had just shipped started failing. Neither move settles the question for every operator —
  DashScope API keys are region-scoped and rejected with a well-formed `401` by every region
  except their own, so **whichever region this default names, an operator whose workspace is in
  a different region must set `DASHSCOPE_BASE_URL`** to their own endpoint:
  `https://dashscope-us.aliyuncs.com/compatible-mode/v1` (US/Virginia) or
  `https://dashscope.aliyuncs.com/compatible-mode/v1` (mainland China). See the "Region default"
  and "Reversal record" docs on `QWEN_DEFAULT_BASE_URL` in
  `crates/paladin-llm/src/qwen/adapter.rs` for the full endpoint table and why no single default
  here is ever the whole answer — the override and the diagnostic below are what actually protect
  every operator, not the choice of default.
- **`QWEN_DEFAULT_MODEL` stays `qwen-plus`, now for a stated reason.** Live-verified 2026-08-23
  against the shipped Singapore endpoint alongside the alternative candidate `qwen3.7-plus`: both
  are present in the live catalog and both accept every measured sampling parameter, so
  `qwen-plus` was kept as the rolling, generation-independent alias rather than switched to the
  generation-pinned `qwen3.7-plus`, which a future Qwen generation will eventually retire the way
  `moonshot-v1-8k` and `gemini-2.5-flash` were retired earlier in this phase.
- **Qwen's declared `temperature_range` narrows from `(0.0, 2.0)` to `(0.0, 1.99)`.** Live
  measurement against the shipped endpoint found DashScope's own accepted range is the half-open
  interval `[0.0, 2.0)` — a request carrying `temperature: 2.0` is rejected with
  `HTTP 400 InternalError.Algo.InvalidParameter`. Since `ProviderCapabilities`'s validation gate
  treats both endpoints of a declared range as inclusive, advertising `2.0` verbatim would let a
  legal-looking request through the local gate only to fail on the wire.

### Fixed

- **The facade's `llm-*` provider flags now actually gate their adapters.** Root `Cargo.toml`
  previously declared `llm-openai = []`, `llm-anthropic = []` and `llm-deepseek = []` as empty
  stubs while pulling `paladin-llm` in unconditionally (`Cargo.toml:55`, pre-fix) with
  `features = ["openai", "anthropic", "deepseek", "mock", "vision"]` — every build compiled all
  three provider adapters regardless of which flags were set. Each `llm-<provider>` flag now
  forwards into the matching `paladin-llm` feature (`llm-openai = ["paladin-llm/openai"]`, and the
  same shape for `anthropic`, `deepseek`, and the five providers Phase 17 added — `kimi`, `qwen`,
  `grok`, `ollama`, `gemini`, plus the generic `openai-compatible` provider), and the
  `paladin-llm` dependency line no longer hardcodes any provider feature. **The default build
  compiles the same three providers it did before — `openai`, `anthropic`, `deepseek` — so no
  action is required of any consumer.** See
  [ADR-0046](.planning/decisions/0046-facade-llm-feature-flag-wiring.md).

## [0.8.0] - 2026-08-12

### Changed

- **BREAKING: the agent API's `http.auth` bearer-token config key is renamed, with no
  compatibility alias.** `http.auth.jwt.enabled` is renamed to `http.auth.bearer_token.enabled`;
  the corresponding Rust type `paladin::config::agents::JwtAuthConfig` is renamed to
  `BearerTokenAuthConfig`, and `AuthConfig`'s `jwt` field is renamed to `bearer_token`. No
  `#[serde(alias = ...)]` or other compatibility shim is provided — a `config.yml` naming the old
  `jwt` key (as shipped in v0.6.0/v0.7.0) will fail to deserialize rather than being silently
  accepted. **Remedy:** rename the key from `jwt:` to `bearer_token:` in your `config.yml`. This
  corrects the config surface to match the mechanism it has always run: an opaque, server-issued
  bearer token verified in-process, never a signed or self-describing JWT. See
  [ADR-0040](.planning/decisions/0040-opaque-bearer-token-mechanism.md).

- **Licence: the project is now dual-licensed `MIT OR Apache-2.0`, additive to the existing MIT
  grant.** The root package and all ten library crates (`paladin-ai`, `paladin-ai-core`,
  `paladin-ports`, `paladin-battalion`, `paladin-herald`, `paladin-llm`, `paladin-memory`,
  `paladin-storage`, `paladin-notifications`, `paladin-content`, `paladin-web`) now declare
  `license = "MIT OR Apache-2.0"`. The root `LICENSE` file (MIT text) is renamed to `LICENSE-MIT`
  with its history preserved via `git mv`; a new `LICENSE-APACHE` carries the verbatim Apache
  License, Version 2.0 text. This is an **additional grant to existing consumers, not a
  restriction** — everyone who already depends on the published `0.1.0` crates under MIT keeps
  that permission unchanged, and gains the option to instead rely on the Apache-2.0 grant (which
  includes an explicit patent license). Decided by the repository owner (`DF3NDR`) at a blocking
  checkpoint on 2026-08-08, confirming the licence policy recorded in
  `.project/Milestone_7-Production-Hardening/Epic_4/license-compatibility-decision-checklist.md`.
  See [ADR-0025](.planning/decisions/0025-licence-posture.md).

### Added

- **`GroveConfig.routing_model: Option<String>`** — the operator-configured LLM model used by
  Grove's `RoutingStrategy::LlmRouting` routing decisions, additive to the existing YAML/JSON
  config surface (`#[serde(skip_serializing_if = "Option::is_none")]`).
- **`GroveBuilder::routing_model(..)`** — the fluent setter for the field above.

### Changed

- **Breaking (runtime): Grove LLM routing now requires `routing_model` to be set.** A Grove whose
  `routing_strategy` is `RoutingStrategy::LlmRouting` must set `routing_model` in its
  configuration; until it does, calling `GroveExecutionService::execute()` — the entry point every
  caller uses — returns `BattalionError::RoutingError` naming the missing configuration, instead of
  silently defaulting to OpenAI's `gpt-4` as it did before this change. This configuration error is
  excluded from Grove's routing fallback handling: no `fallback_tree` substitution and no default
  agent selection, regardless of whether either is configured and would otherwise succeed.
  **Migration:** set `routing_model` in the Grove's YAML/JSON configuration (e.g.
  `routing_model: "claude-3-5-sonnet-20241022"` or `routing_model: "deepseek-chat"`), or pass it
  via `GroveBuilder::routing_model(..)` when building the Grove programmatically.
  **Scope:** only Groves that explicitly select `RoutingStrategy::LlmRouting` *and* do not set
  `routing_model` are affected — a default-constructed Grove is unaffected, since
  `RoutingStrategy::default()` is `KeywordMatch`, and every other Grove routing failure (a
  transient LLM call failure, unparseable JSON, a below-threshold confidence, or an absent
  `llm_port`) keeps its existing fallback behaviour unchanged.
  See [ADR-0013](.planning/decisions/0013-grove-routing-model.md).

- **Breaking (build): the `paladin` binary now requires the `cli` feature.** `cargo run` and
  `cargo build --bin paladin` no longer build that binary without `--features cli` — it now
  carries `required-features = ["cli"]`, the same gate its two siblings (`paladin-cli`,
  `paladin-server`) already had, making all three `[[bin]]` targets consistent with
  ADR-0019's three-binary architecture. **Remedy:** run `cargo build --bin paladin --features cli`
  (or `cargo run --features cli`) instead of the bare command. `Dockerfile` and `Dockerfile.chef`
  were updated to pass `--features cli` in their release build stage. Underneath this gate,
  `src/main.rs` was also migrated from `structopt` (removed from the workspace entirely) to
  `clap` v4, with identical flags (`-c` / `--config`, default `config.yml`) and an unchanged
  binary name (`smartcontent-aggregator`) — a caller's invocation and arguments do not change,
  only the feature requirement does. See
  [ADR-0023](.planning/decisions/0023-cli-dependency-isolation.md).
- **Breaking (library): `paladin-herald`'s table and coloured-markdown formatters moved behind
  features.** `paladin-herald` gained its first `[features]` section (`default = []`, `table`,
  `color`). `TableHerald` is now available only under the `table` feature, and `MarkdownHerald`'s
  coloured rendering path (status badges, bold fields, the coloured error heading) is now behind
  the `color` feature — `MarkdownHerald` itself stays constructible and functional without it,
  falling back to plain text. `JsonHerald` and the uncoloured `MarkdownHerald` remain available in
  a default (featureless) build. **Remedy:** add `features = ["table", "color"]` to the
  `paladin-herald` dependency, or depend on the root `paladin` crate's `cli` feature, which enables
  both. Two consequences a downstream consumer can observe: (1)
  `paladin::infrastructure::adapters::herald::TableHerald` is available only when the root `cli`
  feature is enabled; (2) `Settings::create_default_herald()` called with
  `herald.default_formatter = "table"` in a build without `cli` returns the existing
  `Unknown formatter 'table'. Valid options: json, markdown` error instead of constructing a table
  Herald. `paladin-herald` is published on crates.io, so this is a change to its default public
  API. See [ADR-0023](.planning/decisions/0023-cli-dependency-isolation.md).

## [0.7.0] - 2026-08-03

### Phase 12.1 — Complete the Paladin Arsenal MCP client (dogfood)

> **Provenance note (Phase 4, 2026-08-03):** "Phase 12.1" refers to the historical `.project/`-era
> milestone and epic numbering, not a GSD `.planning/phases/` phase. This project's GSD phases are
> numbered 1, 2, 3, 4… with no decimal sub-phases; GSD Phase 12 is unrelated (SUPPLY-01/SUPPLY-02).

Swaps the Arsenal's hand-rolled MCP JSON-RPC engine for the official `rmcp` SDK, adds a
real authenticated remote transport, and un-stubs tool execution — completing the MCP
client this project depends on for its own downstream Arsenal-MCP-client dogfooding.

#### Added

- **`MCPClient::connect_streamable_http`** — a real, authenticated remote MCP transport
  (Streamable-HTTP) with a bounded connect+handshake timeout, distinguishing
  `ArsenalError::AuthFailed` (401/403-shaped rejections) from general transport/protocol
  faults.
- **`MCPStreamableHttpAdapter`** — a thin fluent builder (`new(endpoint).with_bearer_token(..)
  .with_custom_headers(..).connect()`) mirroring `MCPStdioAdapter`'s shape for the remote
  transport. The bearer token is held in a `BearerToken` wrapper that zeroizes on drop and
  never derives `Debug` (redacted `{:?}` only).
- **Config-driven remote servers** — `arsenal.mcp_servers[].type: "streamable_http"` +
  `endpoint` + `auth_token_env` (an env-var NAME, never a literal secret) in `config.yml`/
  `.mcp.json`, and the CLI's `--mcp-streamable-http <url> [--mcp-auth-token-env <VAR>]` flags.
- **`examples/arsenal_streamable_http_tools.rs`** — a runnable example demonstrating the
  authenticated remote transport end-to-end (connect, discover tools, invoke a tool), with
  the bearer token sourced from an environment variable only.
- Hermetic Streamable-HTTP round-trip test against a real in-process `rmcp` server
  (`tests/integration/mcp_streamable_http_test.rs`), plus a `#[ignore]`'d live probe against
  `mcp.etherscan.io` for operator-run verification (`tests/integration/mcp_streamable_http_live_test.rs`).
- `ArsenalRegistry::list()` and a real, un-stubbed `ArsenalExecutionService::invoke` bridge
  routing tool calls through the connected `MCPClient`.

#### Changed

- **MCP engine swap**: the Arsenal's transport now performs the full spec
  `initialize -> notifications/initialized` handshake via `rmcp::ServiceExt::serve()` for
  every transport, closing the #1 correctness gap in the previous hand-rolled engine
  (which never performed this handshake).
- **`PaladinYamlConfig::validate()`** now accepts `arsenal.mcp_servers[].type: "streamable_http"`
  (previously rejected by the schema-validation allowlist before ever reaching the loader —
  a regression discovered while truthing up this changelog/the docs below).
- Documentation truth-up: `docs/src/user-guides/tool-integration.md`, `arsenal-tools.md`,
  `getting-started/configuration.md`, `appendix/cli-usage.md`, `appendix/cli-configuration.md`,
  `api-reference/stable-api.md`, and `appendix/integration-tests.md` now describe the real,
  implemented `streamable_http` + `auth_token_env` + `MCPClient::connect_streamable_http` flow.

#### Removed

- **`MCPSseAdapter`/`mcp_sse_adapter`** (and the `--mcp-sse` CLI flag) — retired entirely.
  This adapter was never real SSE or Streamable-HTTP; it was a mislabeled, unauthenticated
  plain-HTTP-POST adapter. The `"sse"` `server_type` value in `config.yml`/`.mcp.json` now
  fails loud with an actionable migration message ("Use 'streamable_http' instead") rather
  than silently constructing a since-removed adapter.
- The hand-rolled MCP JSON-RPC 2.0 types (`MCPMessage`/`MCPRequest`/`MCPResponse`/
  `MCPNotification`/`MCPCapabilities`/`ServerInfo`/`ToolInfo`/`MCPTransport`) — superseded by
  `rmcp::model::*` and rmcp's own transport abstraction.

## [0.6.0] - 2026-06-10

**Milestone 12 — Web API.** Paladin now ships an HTTP agent API **out of the box**: the
`paladin-server` binary (the `web-server` feature) serves a versioned (`/v1`), authenticated,
OpenAPI-documented surface for executing and managing resident agents — with container, compose,
and Kubernetes artifacts to deploy it. Epics 1–7 below.

### Milestone 12 — Epic 7: Deployment artifacts, examples & documentation

#### Added

- **Container image** — `Dockerfile.server` (multi-stage, `debian:12-slim`, non-root) building and
  running `paladin-server`, plus `docker/docker-compose.server.yml` and a `make docker-build-server`
  target.
- **Kubernetes manifests** — `k8s/server/` Deployment + Service + ConfigMap (+ `secret.yaml.example`)
  with liveness `/health` and readiness `/ready` probes.
- **Runnable example** — `examples/http_service_host.rs` boots the server in-process (hermetic, mock
  LLM) and exercises an agent; the deployment-topology doc-example now embeds the shipped router.
- **End-to-end tests** — `tests/web_server_e2e.rs` covering auth, buffered/streaming/jobs, health,
  errors, and the served spec/UI.

#### Changed

- The **HTTP service-host deployment docs** now describe the shipped `paladin-server` (routes, auth,
  config, Docker/k8s) instead of "compose your own endpoint".
- **Version:** all workspace crates bumped to **0.6.0**.

### Milestone 12 — Epic 6: OpenAPI specification & interactive docs

Publishes a machine-readable contract for the agent API and an interactive explorer, and introduces
the `/v1` version prefix as a stability boundary. Built in `paladin-web` with `utoipa`/`utoipa-axum`.

#### Changed

- **BREAKING (HTTP API):** the agent API now serves under a **`/v1`** prefix — `/v1/agents`,
  `/v1/agents/{id}/execute[/stream]`, `/v1/agents/{id}/jobs[/{job_id}]`. `/health`, `/ready`,
  `/openapi.json`, and `/docs` remain unversioned. (Pre-release; no released consumer is affected.)

#### Added

- **OpenAPI 3.1 spec** at `GET /openapi.json`, derived from the handlers (`#[utoipa::path]`) and DTOs
  (`ToSchema`) via `utoipa-axum`, including the error envelope and the `api_key` (`X-API-Key`) +
  `jwt` (bearer) security schemes.
- **Swagger UI** at `/docs`. Both endpoints are gated by `http.docs.enabled` (default true; unversioned
  and unauthenticated) — set it `false` to omit them in production.
- **Versioning/stability policy** (README): additive-only within `/v1`; breaking changes ship as
  `/v2`.
- **Spec drift guard**: a committed `crates/paladin-web/openapi.json` baseline and a test that fails
  on divergence; regenerate with `make openapi`.

#### Build

- `paladin-web` adds `utoipa`, `utoipa-axum`, and `utoipa-swagger-ui`.

### Milestone 12 — Epic 5: API security & authorization

Secures the agent HTTP API with authentication, per-agent authorization, and an admin gate on
runtime registration. Built in `paladin-web` (the JWT `AuthPort` impl is injected by the binary —
no facade dependency).

#### Added

- **Authentication** (`paladin_web::{AgentAuthConfig, Principal}`): a configured **API-key** list
  (`X-API-Key` header, constant-time match) and the existing **JWT** bearer path (`AuthPort`). Either
  credential authenticates; failures return `401` via the unified `ApiError` envelope.
- **Per-agent authorization**: an agent's optional `allowed_roles` restricts invocation (empty ⇒ any
  authenticated caller); a disallowed role gets `403`.
- **Admin gate**: `POST /agents` and `DELETE /agents/{id}` require an `admin` role (`403` otherwise).
- **Config** (`http.auth`): `enabled` (default true), `api_keys: [{ key, name, role }]`, `jwt`, plus
  per-agent `allowed_roles`. `paladin-server` maps these onto the auth layer and wires
  `InMemoryTokenAuthAdapter` for the JWT path.

#### Security

- **Fail-closed posture**: with auth enabled and no credentials configured, `paladin-server` refuses
  to start with an actionable error. `auth.enabled: false` serves open with a logged warning.
- `GET /health` and `GET /ready` remain unauthenticated.
- Credentials are never logged (the request logger emits no headers/bodies) and never echoed in error
  responses; discovery endpoints never return prompts/keys.

#### Changed

- The user-management routes' `401`/`403` responses now use the unified `ApiError` envelope, matching
  the agent API.

### Milestone 12 — Epic 4: API cross-cutting concerns

Adds production middleware to the agent HTTP API: a unified error model, health/readiness probes,
request logging, and configurable CORS / body-limit / timeout / rate-limit layers.

#### Changed

- **BREAKING (HTTP API):** error responses now use a structured envelope
  `{ "error": { "code", "message", "details" } }` with a stable machine-readable `code`, replacing
  the previous flat `{ "error": "<message>" }` (and the user controller's `ApiResponse` error form).
  Applies to all `paladin-web` controllers (agent, user-management, content-delivery) and SSE
  `error` events. Success bodies are unchanged.

#### Added

- **`ApiError`** (`paladin_web::ApiError`): the unified error type (`IntoResponse`) with
  constructors per status and stable codes.
- **Health/readiness**: `GET /health` (`{ "status": "ok" }`) and `GET /ready`
  (`{ "status": "ready", "agents": N }`), suitable for Kubernetes probes.
- **Request logging**: an `x-request-id` is generated (or echoed) and returned on every response;
  each request is logged with method, path, status, and latency via `log`.
- **Edge layers** (`paladin_web::{HttpLayersConfig, with_http_layers}`): configurable CORS, a
  request body-size limit (`413`), an optional global request timeout (non-streaming routes only —
  SSE is never cut off), and an optional per-IP rate limiter (`tower-governor`, `429`, off by
  default). Configured via the new `http:` section in `config.yml`; `paladin-server` serves with
  `ConnectInfo` so the limiter keys on the peer IP.

#### Build

- `paladin-web` adds `tower`, `tower-http` (cors/limit/timeout), and `tower_governor`.

### Milestone 12 — Epic 3: Streaming & asynchronous execution

Adds token streaming, execution timeouts/cancellation, and in-process async jobs to the agent
HTTP API. All additive — the buffered `execute` path and `PaladinExecutorPort` are unchanged.

#### Added

- **`StreamingExecutorPort`** (`paladin-ports`): a focused streaming counterpart to
  `PaladinExecutorPort` (`execute_stream → PaladinStream`). `PaladinExecutionService` now implements
  it over `LlmPort::generate_stream`.
- **SSE streaming endpoint** `POST /agents/{id}/execute/stream`: real incremental tokens as
  Server-Sent Events (`chunk` events + a terminal `done`/`error`). Agents without a streaming
  backend fall back to a buffered single `chunk` + `done`. The registry entry carries an optional
  streaming handle, wired by the config builder and the runtime provisioner.
- **Execution timeouts & cancellation**: per-request `timeout_seconds`, per-agent
  `timeout_seconds`, and a server-wide `timeouts` policy (`default_seconds`/`max_seconds`), resolved
  request → agent → default and clamped to the max. On expiry the in-flight work is cancelled and
  the call returns `504` (buffered/job) or a terminal `error` SSE event.
- **In-process async jobs**: `POST /agents/{id}/jobs` (returns `202` + `job_id`) and
  `GET /agents/{id}/jobs/{job_id}` (status `running`/`completed`/`failed`/`timed_out` + result).
  Backed by a bounded, in-memory `JobStore` (ephemeral; durable/distributed jobs remain the
  queue/worker topology).
- `config.example.yml` gains a `timeouts` section and a per-agent `timeout_seconds`.

#### Notes

- The new agent routes remain **unauthenticated** in this milestone (auth → Epic 5). `paladin-web`
  adds `futures` + `async-stream` for the SSE deadline race.

### Milestone 12 — Epic 2: Configurable web host & `paladin-server` binary

Makes the HTTP service-host topology runnable with no Rust required: a config schema, a
registry-from-config builder, a runtime provisioner, and a server binary that serves the agent
API from Epic 1.

#### Added

- **`agents` configuration** (`config.yml`): a list of `AgentDefinition`s
  (`id`/`model`/`system_prompt` required; optional `provider`/`temperature`/`max_loops`/
  `stop_words`). The bind address reuses the existing `server` (`host`/`port`) section. API keys
  continue to come from the `llm:` provider env vars, never the agent definitions.
- **Registry-from-config builder** (`paladin::infrastructure::web::agent_host`):
  `build_agent_registry`, `build_agent`, `validate_config` (fail-fast, key-free pre-flight:
  non-empty fields, no duplicate ids, provider available), and `bind_address`. Agents are
  **LLM + prompt only** (no garrison/arsenal this epic).
- **Runtime provisioner** (`paladin::infrastructure::web::facade_provisioner::FacadeProvisioner`):
  the concrete `AgentProvisioner` so `POST /agents` builds and registers agents at runtime, sharing
  the same build path as config load.
- **`paladin-server` binary** (`--features web-server`): loads `config.yml`, builds the agents,
  serves the `/agents/*` API with `axum`, logs the bound address + agent ids, and shuts down
  gracefully on Ctrl-C / SIGTERM. Config path via `PALADIN_CONFIG` or the first CLI argument.

#### Build

- Added an **optional `axum`** dependency to `paladin-ai`, gated by the `web-server` feature (used
  only by the `paladin-server` binary).

### Milestone 12 — Epic 1: Agent registry & execution API (`paladin-web`)

The HTTP service-host topology previously shipped no agent-execution endpoint — consumers had to
hand-write the registry and handlers. This adds that surface to `paladin-web`, so agents can be run
over HTTP out of the box. The web layer depends only on the `PaladinExecutorPort` trait and the
`Paladin` entity (no dependency on the `paladin-ai` facade).

#### Added

- **Agent registry** (`paladin_web::AgentRegistry`): thread-safe, in-memory map of id →
  `(Arc<Paladin>, Arc<dyn PaladinExecutorPort>)` (per-agent executor) with `get`/`list`/`insert`
  (no-overwrite)/`remove`/`contains`; poison-safe, never holds its lock across `.await`.
- **Provisioning seam** (`paladin_web::{AgentProvisioner, AgentSpec, ProvisionError}`): the trait the
  composition root implements to build agents for runtime registration, keeping `paladin-web`
  decoupled from the facade.
- **Agent-execution HTTP API** (`paladin_web::agent_controller`, mounted via
  `paladin_web::agent_router` / `app::create_app_router_with_agents`):
  - `POST /agents/{id}/execute` — run an agent (`200` + output/metadata; `404`; `400`; `502` on
    execution failure).
  - `GET /agents` and `GET /agents/{id}` — discovery with a safe `AgentSummary` (no secrets; system
    prompt reduced to a short description preview).
  - `POST /agents` — runtime registration via the provisioner (`201`; `409`; `422`; `400`; `501`
    when no provisioner is wired).
  - `DELETE /agents/{id}` — deregistration (`204`/`404`).
- Wire types `ExecuteRequest`/`ExecuteResponse` and a `create_app_router_with_agents` composer that
  merges the agent routes alongside the user/auth and delivery routers.

#### Notes

- Agent routes are intentionally **unauthenticated** in this epic; authentication and per-agent
  authorization arrive in Milestone 12, Epic 5. Config-driven hosting and a runnable server binary
  (including the concrete `AgentProvisioner`) arrive in Epic 2.

### Milestone 8 — Epic 7: `paladin-web` single web framework (axum)

`paladin-web` depended on **two** HTTP frameworks (axum + actix-web) but served everything through
axum; the actix code was orphaned (never mounted). This consolidates on axum, revives the
content-delivery endpoints as real served routes, and guards against framework sprawl.

#### Added

- **Content-delivery axum routes** (`paladin_web::delivery_controller`): `POST /api/delivery/deliver`,
  `GET /api/delivery/status/{delivery_id}`, and `GET /api/delivery/stats`, exposed via
  `create_delivery_routes` and merged into `create_app_router` as public routes (parity with the
  previous, never-mounted actix handlers). Backed by the existing `ApiContentDeliverer`.

#### Changed

- `paladin_web::app::create_app_router` now also takes an `Arc<ApiContentDeliverer>` and mounts the
  delivery routes alongside the user-management API.

#### Removed

- **`actix-web` dependency** from `paladin-web` (its only user). The orphaned actix `configure()` +
  handlers in `api_content_deliverer.rs` were deleted; the reqwest-based `ApiContentDeliverer`
  service is unchanged. `Cargo.lock` drops ~450 lines of actix transitive dependencies.

#### Build

- `deny.toml` now bans `actix-web` so a second web framework cannot be reintroduced without a
  deliberate, reviewed change.

### Milestone 11 — Documentation Overhaul & Publish (Epic 6: Deployment Topologies)

#### Documentation

- **New "Deployment Topologies" book section** answering "how do I run a number of different
  agents?" A decision-matrix landing page (comparison table + Mermaid flowchart) plus one page
  per topology: **embedded library**, **Battalion orchestration**, **HTTP service host**,
  **queue / worker (distributed)**, and **sidecar (separate process)**. Each carries a
  compile-verified example pulled from `paladin-doc-examples` via mdBook `{{#include}}`.
- **Honest gap callouts** where the framework provides no first-class mechanism: the HTTP-host
  page documents a *consumer-composed* `axum` endpoint (the shipped `create_app_router` is the
  user/auth API, not an agent runner), and the sidecar page states plainly that no IPC/RPC
  transport ships (the pattern composes the HTTP host + an HTTP client).
- `crates/doc-examples` gained `axum`, `reqwest`, `serde` (derive), and `paladin-storage`
  (`redis-queue`) so the new topology examples compile against the live API. No shipped-library
  crate or public API changed.

### Milestone 8 — Facade Cleanup & Shim Resolution (completion)

Finishes the relocations that Milestone 8 Epic 3 had deferred and removes the dead/duplicate code
left behind by the earlier workspace decomposition. ~10,250 net LOC removed; the facade is reduced
to a true composition root. No external consumers exist, but the facade's public API surface
changed — see **Removed** / **Changed** below.

#### Added

- **New `paladin-herald` crate** — the `Herald` output formatters (`JsonHerald`, `MarkdownHerald`,
  `TableHerald`) now live here with their presentation dependencies (`comfy-table`, `colored`),
  keeping `paladin-core` dependency-light. Still reachable via
  `paladin::infrastructure::adapters::herald::{JsonHerald, MarkdownHerald, TableHerald}` (re-export).
- `paladin-storage` gained `s3` (MinIO/S3) and `redis-queue` (Redis) features, and is now a
  non-optional facade dependency with `sqlite` always enabled.

#### Changed

- **Persistence consolidated in `paladin-storage`.** The MinIO/S3 file-storage adapter and the
  Redis queue adapter moved out of the facade into `paladin-storage` (behind the `s3` and
  `redis-queue` features); the facade re-exports them under its existing `s3-storage` / `redis-queue`
  features, so `crate::infrastructure::adapters::file_storage::minio` and `...::queue::redis`
  paths are unchanged. `paladin-storage` bumped to `edition = "2024"`.
- **`FileCitadel` moved to `paladin-memory`** (`paladin_memory::citadel::file_citadel`); re-exported
  by the facade at the same path.
- **`paladin-storage` is now non-optional** with SQLite always compiled — `sqlx` is now part of the
  default build. The facade's local SQLite repository fallbacks were deleted; the SQLite repos are
  always sourced from `paladin-storage`.
- Facade adapter modules for relocated implementations are now `pub use` re-exports rather than
  local `pub mod` (citadel, herald, sqlite repositories).
- Service/infrastructure status `println!` output converted to `log::*` (CLI output unchanged).

#### Removed

- **`paladin::infrastructure::adapters::paladin_registry`** (and `HashMapPaladinRegistry`) — the
  registry was consolidated into `paladin-battalion`. Use
  `paladin_battalion::in_memory_registry::HashMapPaladinRegistry` instead. **(breaking — facade API)**
- **`paladin::infrastructure::repositories::file_content_repository`** — unused, deleted.
- The half-built `user` CLI command, the placeholder TensorFlow ML adapter, and the now-unused `ml`
  feature flag — removed and documented for future reintroduction in
  `project/Milestone_8-Facade-Cleanup-Shim-Resolution/deferred-features.md`.
- The dead facade-local notification fallback adapters and orphaned/duplicate adapter files left
  behind by earlier extractions.

## [0.5.1] - 2026-06-04

### Fixed

- **Release pipeline: `linux-arm64` binary.** The `Build Binaries` job ran the host x86_64 `strip`
  on the cross-compiled aarch64 binary and failed (`strip: Unable to recognise the format`), so
  v0.5.0 shipped without the `paladin-linux-arm64` asset. The strip step now uses the matching
  aarch64 cross strip (`binutils-aarch64-linux-gnu`) for that target. No library code changed.

## [0.5.0] - 2026-06-03

The **v0.5.0** release completes **Milestone 11 — Documentation Overhaul & Publish**, consolidating
the work of Milestones 8–11 (orchestrator completion, facade cleanup, CI hardening, and the full
documentation rebuild) into the first release with a published, compile-verified documentation site.
No public API changed; this is a documentation, packaging, and release-readiness milestone.

### Added

#### Documentation site (Milestone 11)

- **Published mdBook** at <https://df3ndr.github.io/paladin-dev-env/> — the complete reorganized
  documentation (getting started, user guides, architecture, deployment, operations, API reference,
  contributing) deployed to GitHub Pages via `.github/workflows/docs.yml`.
- **New user guides** — a comprehensive **Orchestration** guide (all Battalion patterns + job
  scheduling + the event/trigger system), a **Content Processing** guide, and a standalone
  **Agent ↔ Orchestrator Bridge** guide with four end-to-end recipes.
- **Crate Map & Feature-Flag reference** (`api-reference/crate-map.md`) — every workspace crate with
  purpose, layer, and key exports; a Mermaid dependency graph; per-crate feature-flag tables; and
  copy-paste `Cargo.toml` consumer profiles.

#### Compile-verified documentation examples (Milestone 11)

- **`paladin-doc-examples` crate** — the substantive code examples in the guides now live in a real
  workspace crate and are pulled into the markdown via mdBook `{{#include}}`, so `cargo check`
  guarantees every documented example compiles against the current API.
- **`scripts/check-doc-examples.sh`** now compiles that crate as the primary gate (and keeps the
  README Quick Example in sync with its compiled source), and **`scripts/check-doc-config.sh`**
  validates every fenced YAML config snippet. Both run in CI (`docs.yml`) and as pre-push hooks.
- **Root `LICENSE`** (MIT) added.

### Changed

- **Root `README.md`** rewritten as a concise landing page (badges, a compile-verified quick
  example, crate-ecosystem table, and links to the published docs) — down from ~1,000 lines.
- **Workspace version → 0.5.0** (lockstep across all crates); documentation version references
  synchronized to 0.5.0.
- The `documentation` metadata now points at the published mdBook site.

### Fixed

- Corrected numerous documentation inaccuracies surfaced by compiling the examples: wrong
  `BattalionResult`/`Phalanx`/`Campaign`/`ChainOfCommand` APIs, a non-existent file fetcher, and
  incorrect queue/aggregation types, among others.
- Replaced stale `yourusername/paladin` placeholder URLs with the real repository, removed dead
  "TODO: Add link" stubs, and fixed mislabeled code-fence languages.

### Documentation

- Every existing documentation file was audited (current / stale / delete), migrated into the mdBook
  chapter structure, and rewritten so that all code examples compile and all internal links resolve
  (`mdbook build` passes with linkcheck `warning-policy = "error"`, zero broken links).

## [0.4.3] - 2026-06-01

## [0.4.2] - 2026-06-01

## [0.4.1] - 2026-05-31

### Added

#### Release Branch Protection — Tag-from-Main Enforcement (Milestone 10, Epic 5)

- **`verify-tag-source` CI guard** — `release.yml` now fails the entire release pipeline before any
  publishing if the tagged commit is not contained in `main` (`git merge-base --is-ancestor`). The
  `test` and `create-release` jobs depend on it, so Docker, binary, SBOM, and crates.io publishing
  are all gated.
- **`make release` main-branch guard** — refuses to bump/tag unless run from an up-to-date `main`;
  documented `RELEASE_ALLOW_ANY_BRANCH=1` override for hotfix branches (CI guard remains
  authoritative).
- **Importable GitHub rulesets** — `.github/rulesets/protect-main-branch.json` (PR + status checks,
  no force-push/deletion) and `.github/rulesets/protect-release-tags.json` (restrict `v*` tag
  creation to admins).
- **`docs/BRANCH_PROTECTION.md`** — policy rationale, the three enforcement layers, and ruleset
  import instructions (GitHub UI and `gh api`).
- Updated `CONTRIBUTING.md` `## Releasing` to document the main-only release policy.

## [0.4.0] - 2026-05-31

### Added

#### CI Hardening: Pre-commit / Pre-push Hook Framework (Milestone 10, Epic 1)

- **`.pre-commit-config.yaml`** — commit-stage hooks: `cargo fmt --check`, `cargo clippy`,
  `gitleaks` secret detection, TOML/YAML/JSON validation, large-file and merge-conflict checks,
  trailing-whitespace and end-of-file normalization.
- **Pre-push stage** — `cargo build --workspace` and `cargo test --workspace --lib` run
  automatically on every `git push`.
- **`make hooks`** Makefile target — installs both the commit and push hook stages in one command.
- Provisioned `pre-commit` (4.6.0) and `gitleaks` in `.devcontainer/Dockerfile.dev`; hooks are
  available immediately when the dev container is (re)built.
- Normalized trailing-whitespace and end-of-file markers across all source files.
- Added pre-commit hook instructions to `CONTRIBUTING.md` (Git Hooks section).

#### Dependency Security and License Compliance (Milestone 10, Epic 2)

- **`.cargo/audit.toml`** — exception list for known advisories; `cargo audit` exits 0 with all
  exceptions documented (rationale and affected crates recorded inline).
- **`deny.toml`** — `cargo-deny` configuration with a license allow-list
  (MIT / Apache-2.0 / BSD-2-Clause / BSD-3-Clause / ISC / Zlib) and per-crate exceptions for
  MPL-2.0, CC0-1.0, CDLA-Permissive-2.0, and 0BSD; advisory ignore list mirrors `audit.toml`.
- **CycloneDX SBOM** — `release.yml` now generates a JSON SBOM artifact via
  `cargo cyclonedx --all --format json` on every tagged release.
- **OSV-Scanner** — annotate-only job in `ci.yml` surfaces new advisories without blocking CI.
- **`make security`** / **`make audit`** / **`make deny`** / **`make sbom`** Makefile targets.
- **`docs/SECURITY_SCANNING.md`** — full tooling overview, license policy, and advisory
  exception process.
- Updated `CONTRIBUTING.md` with Security subsection and cross-references.

#### Release Automation (Milestone 10, Epic 3)

- **`release.toml`** — `cargo-release` workspace config: `shared-version = true`,
  `publish = false`, `push = false` for lockstep workspace versioning.
- **Tag-triggered `publish-crates` CI job** in `release.yml` — publishes in dependency order
  (paladin-ai-core → paladin-ports → leaf crates → paladin); supports dry-run and skip modes;
  20 s gaps between publishes to avoid crates.io index propagation races.
- **`workflow_dispatch` `dry_run` input** added to `release.yml` for manual pipeline exercises.
- **`make release VERSION=`** Makefile target — validates semver, runs the full quality gate
  (`fmt`, `clippy`, tests, audit, release build), bumps all crates lockstep via
  `cargo release version`, finalizes `CHANGELOG.md`, commits, tags, and pushes.
- **`make publish-dry-run`** target — runs `cargo publish --dry-run` for every crate in
  dependency order.
- **`docs/RELEASE_AUTOMATION.md`** — tooling decision document and operator guide (cargo-release
  vs release-plz comparison, install instructions, publish order, required secrets).
- Updated `docs/RELEASE_CHECKLIST.md` with a cross-reference to `RELEASE_AUTOMATION.md`.
- Updated `CONTRIBUTING.md` with `## Releasing` section covering the `make release` workflow.
- Provisioned `cargo-release` (1.1.2), `cargo-deny` (0.19.8), and `cargo-cyclonedx` (0.5.9) in
  `.devcontainer/Dockerfile.dev` and `make setup` so rebuilt dev images ship the tools locally.

#### Finalization (Milestone 10, Epic 4)

- **`CONTRIBUTING.md` — "Adding a New Dependency" section** — step-by-step guide: `cargo add`,
  license check (`make deny`), vulnerability check (`make audit`), exception documentation in
  `deny.toml` / `.cargo/audit.toml`, `CHANGELOG.md` update, CI gate expectations.
- **`CONTRIBUTING.md` Table of Contents** — added missing `Releasing` and
  `Adding a New Dependency` entries.
- **Lockstep version bump** — all workspace crates bumped from `0.3.0` to `0.4.0`.

### Fixed

- **`paladin-content` module rename** (Milestone 8, Epic 6): Renamed `use_cases` → `services`
  inside the `paladin-content` leaf crate to align with the naming convention established by
  Epic 4. The directory `crates/paladin-content/src/use_cases/` is now
  `crates/paladin-content/src/services/`; `lib.rs` now declares `pub mod services;`.
- **Resolved six latent `E0432` unresolved import errors** in the facade re-export bridge
  (`src/application/services/content/mod.rs`). The bridge already referenced
  `paladin_content::services::*` (correct post-Epic-4 path), but the leaf crate had not been
  updated. The errors were previously masked by the `content-processing` feature gate and did
  not surface in default `cargo test` runs.

---

## [0.3.0] - 2026-05-31

Milestone 9 — Classic Orchestrator, Content Pipeline, and Agent-Orchestrator Bridge.
This release makes the time- and event-driven orchestration paths functional end-to-end, bridges
the content pipeline and AI agents into the `Orchestrator`, and completes the user/admin system with
authentication and role-based access control.

### Added

#### Orchestration (Milestone 9, Epic 1)

- **Workflow execution loop**: the `Orchestrator` now executes workflows end-to-end rather than
  simulating them. Jobs are dispatched, their outcomes aggregated into a `WorkflowExecutionResult`,
  and the configured error strategy is honored when a job fails.
- **Workflow state persistence & resume**: in-progress workflow state is persisted so incomplete
  workflows can be resumed after a restart.
- **Real `TaskService` behavior**: simulated task implementations were replaced with real dispatch
  and error-strategy handling, validated by a full-lifecycle integration test.

#### Scheduler & Queue (Milestone 9, Epic 2)

- **Validated scheduler tick loop**: `next_run` computation for `Schedule::Interval`,
  `Schedule::Cron`, and `Schedule::Once` is verified, disabled jobs are skipped, and
  `last_run`/`run_count`/`next_run` advance correctly after each dispatch.
- **Validated `QueuePort` contract**: the in-memory queue and the `RedisQueueAdapter` are exercised
  against the same `QueuePort` contract, including retry and dead-letter behavior.
- **Validated event → trigger → job pipeline**: a matching event produces exactly one trigger per
  matching listener, which is converted to a job and executed via the Epic 1 dispatch path.

#### Content Pipeline (Milestone 9, Epic 3)

- **`PaladinContentProcessor`**: a content → agent bridge that routes ingested content through a
  Paladin agent.
- **`BattalionContentProcessor`**: a content processor backed by a Battalion for multi-agent content
  enrichment.
- **Orchestrator wiring**: the content processors are wired into the `Orchestrator`, with an
  ingestion → enrichment pipeline integration test.

#### Agent–Orchestrator Bridge (Milestone 9, Epic 4)

- **`OrchestratorPort`**: a new bridge interface in `paladin-ports` that lets agents invoke
  orchestrator workflows.
- **`OrchestratorBridgeAdapter`**: an adapter implementing `OrchestratorPort` over the concrete
  `Orchestrator`.
- **`PaladinExecutionService` integration**: agents can now drive orchestrator workflows through the
  port, validated by an agent → orchestrator bridge integration test.

#### User/Admin System & Security (Milestone 9, Epic 5)

- **Role-based access control**: a `UserRole` (`Admin`/`User`) was added to the user domain and is
  persisted by the SQLite user repositories.
- **`AuthPort` authentication abstraction**: a new port in `paladin-ports` defining token issuance,
  verification, and revocation (`AuthToken`, `AuthClaims`, `AuthError`).
- **In-memory opaque-token auth adapter**: a concrete `AuthPort` implementation issuing opaque
  bearer tokens (random token material, only SHA-256 hashes stored, configurable expiry).
- **User CRUD & token-issuing login**: the user service gained `delete_user` and `list_users`, and
  login now issues an authentication token.
- **Axum auth middleware & RBAC guards**: bearer-token authentication middleware (`require_auth`),
  an admin guard (`require_admin`), and self-or-admin authorization, all returning non-revealing
  `401`/`403` responses.
- **Protected routes & app router**: a `create_app_router` composition exposing public routes
  (register, login), self-scoped routes (`GET`/`PUT /users/{id}`), and admin-only routes
  (`GET /users`, `DELETE /users/{id}`), validated by authentication + RBAC integration tests.

### Changed

- Workspace version bumped to `0.3.0` across the root crate and all member crates.

---

## [0.2.0] - 2026-05-30

### Breaking Changes

- **Services Directory Rename** (Milestone 8, Epic 4): `src/application/use_cases/` renamed to
  `src/application/services/`. All module paths under `paladin::application::use_cases` are now at
  `paladin::application::services`. No logic was changed; this is a pure path rename.

  | Old path | New path |
  |----------|----------|
  | `paladin::application::use_cases::paladin::*` | `paladin::application::services::paladin::*` |
  | `paladin::application::use_cases::battalion::*` | `paladin::application::services::battalion::*` |
  | `paladin::application::use_cases::arsenal::*` | `paladin::application::services::arsenal::*` |
  | `paladin::application::use_cases::content::*` | `paladin::application::services::content::*` |
  | `paladin::application::use_cases::herald::*` | `paladin::application::services::herald::*` |
  | `paladin::application::use_cases::orchestration::*` | `paladin::application::services::orchestration::*` |
  | `paladin::application::use_cases::log_orchestrator::*` | `paladin::application::services::log_orchestrator::*` |
  | `paladin::application::use_cases::notification_orchestrator::*` | `paladin::application::services::notification_orchestrator::*` |
  | `paladin::application::use_cases::queue_orchestrator::*` | `paladin::application::services::queue_orchestrator::*` |
  | `paladin::application::use_cases::sanctum::*` | `paladin::application::services::sanctum::*` |
  | `paladin::application::use_cases::analysis::*` | `paladin::application::services::analysis::*` |

### Added
- **CLI Feature Flag** (Milestone 4, Epic 3): Gate the `paladin-cli` binary and `application::cli` module behind the new `cli` feature flag
  - New feature: `cli = ["dep:clap", "dep:dialoguer", "dep:indicatif", "dep:console", "dep:serde_yaml"]`
  - CLI-only dependencies (`clap`, `dialoguer`, `indicatif`, `console`, `serde_yaml`) are now `optional = true`
  - The `application::cli` module is now `#[cfg(feature = "cli")]`-gated in both `src/application/mod.rs` and `src/lib.rs`
  - The `paladin-cli` binary now requires `required-features = ["cli"]` in Cargo.toml
  - The `full` convenience flag includes `cli`
  - New integration test suite: `tests/cli_isolation_test.rs` — 9 regression tests verifying library compiles without CLI deps
  - Dedicated `cli-isolation` CI job verifies library-only and CLI-enabled builds
  - **Benefit**: Library consumers who don't use the CLI avoid compiling `clap` and associated TUI dependencies

### Changed
- **Facade Crate Documentation** (Milestone 8, Epic 5): Documented facade crate role as the
  application assembly point and composition root. Added `src/README.md` with full module layout
  reference. Updated `src/lib.rs` `//!` docs with a new `## Facade Crate Role` section explaining
  what the facade contains (ServiceRunner, services, config, CLI, binaries), what it does not
  contain (business logic, port traits, adapters), and the dependency-flow rule (facade → leaf
  crates; one direction only).

### Removed
- **Storage Re-export Shims** (Milestone 8, Epic 3): Deleted `src/application/storage/` (3 files:
  `sql_store.rs`, `user_store.rs`, `mod.rs`). These files contained only `pub use` re-exports of port
  traits that already live in `paladin_ports`. Six internal consumers were updated to import directly
  from the canonical crate paths.

  | Removed shim path | Replacement canonical path |
  |-------------------|---------------------------|
  | `paladin::application::storage::sql_store::ContentRepository` | `paladin_ports::output::repository_port::ContentRepository` |
  | `paladin::application::storage::sql_store::ContentListRepository` | `paladin_ports::output::repository_port::ContentListRepository` |
  | `paladin::application::storage::sql_store::MigrationManager` | `paladin_ports::output::repository_port::MigrationManager` |
  | `paladin::application::storage::sql_store::RepositoryError` | `paladin_ports::output::repository_port::RepositoryError` |
  | `paladin::application::storage::sql_store::RepositoryStats` | `paladin_ports::output::repository_port::RepositoryStats` |
  | `paladin::application::storage::sql_store::SqlStore` | `paladin_ports::output::repository_port::SqlStore` |
  | `paladin::application::storage::sql_store::TransactionManager` | `paladin_ports::output::repository_port::TransactionManager` |
  | `paladin::application::storage::user_store::UserRepositoryPort` | `paladin_ports::output::user_repository_port::UserRepositoryPort` |

- **Facade Short-path Aliases** (Milestone 8, Epic 2): Removed zero-consumer `pub use` re-export aliases
  from `src/lib.rs`. These aliases had no workspace consumers; the underlying types are unchanged and
  remain accessible via their canonical crate paths.

  The following short-path aliases (`paladin::<Type>`) have been removed. Use the crate-level paths shown instead:

  | Removed short-path alias | Replacement canonical path |
  |--------------------------|---------------------------|
  | `paladin::LlmError` | `paladin_ports::output::llm_port::LlmError` |
  | `paladin::LlmPort` | `paladin_ports::output::llm_port::LlmPort` |
  | `paladin::LlmRequest` | `paladin_ports::output::llm_port::LlmRequest` |
  | `paladin::LlmResponse` | `paladin_ports::output::llm_port::LlmResponse` |
  | `paladin::ProviderCapabilities` | `paladin_ports::output::llm_port::ProviderCapabilities` |
  | `paladin::TokenUsage` | `paladin_ports::output::llm_port::TokenUsage` |
  | `paladin::LlmProviderError` | `paladin_llm::error::LlmProviderError` |
  | `paladin::PromptItem` | `paladin_core::platform::container::prompt::PromptItem` |
  | `paladin::GarrisonError` | `paladin_ports::output::garrison_port::GarrisonError` |
  | `paladin::GarrisonPort` | `paladin_ports::output::garrison_port::GarrisonPort` |
  | `paladin::GarrisonStats` | `paladin_ports::output::garrison_port::GarrisonStats` |
  | `paladin::LongTermGarrisonPort` | `paladin_ports::output::garrison_port::LongTermGarrisonPort` |
  | `paladin::SanctumError` | `paladin_ports::output::sanctum_port::SanctumError` |
  | `paladin::SanctumFilter` | `paladin_ports::output::sanctum_port::SanctumFilter` |
  | `paladin::SanctumPort` | `paladin_ports::output::sanctum_port::SanctumPort` |
  | `paladin::SanctumQuery` | `paladin_ports::output::sanctum_port::SanctumQuery` |
  | `paladin::SanctumSearchResult` | `paladin_ports::output::sanctum_port::SanctumSearchResult` |
  | `paladin::SanctumEntry` | `paladin_core::platform::container::sanctum::SanctumEntry` |
  | `paladin::InMemoryGarrison` | `paladin_memory::garrison::InMemoryGarrison` |
  | `paladin::SqliteGarrison` | `paladin_memory::garrison::SqliteGarrison` |
  | `paladin::InMemorySanctum` | `paladin_memory::sanctum::InMemorySanctum` |
  | `paladin::QdrantSanctumAdapter` | `paladin_memory::sanctum::QdrantSanctumAdapter` |
  | `paladin::ExtractedMemory` | `paladin_memory::services::ExtractedMemory` |
  | `paladin::MemoryExtractionService` | `paladin_memory::services::MemoryExtractionService` |
  | `paladin::MemoryExtractionStrategy` | `paladin_memory::services::MemoryExtractionStrategy` |
  | `paladin::RagConfig` | `paladin_memory::services::RagConfig` |
  | `paladin::RagRetrievalService` | `paladin_memory::services::RagRetrievalService` |
  | `paladin::RetrievalTrigger` | `paladin_memory::services::RetrievalTrigger` |
  | `paladin::Embedding` | `paladin_ports::output::embedding_port::Embedding` |
  | `paladin::EmbeddingError` | `paladin_ports::output::embedding_port::EmbeddingError` |
  | `paladin::EmbeddingPort` | `paladin_ports::output::embedding_port::EmbeddingPort` |
  | `paladin::ArsenalPort` | `paladin_ports::output::arsenal_port::ArsenalPort` |
  | `paladin::ArsenalRegistry` | `paladin_ports::output::arsenal_port::ArsenalRegistry` |
  | `paladin::ArsenalError` | `paladin_core::platform::container::arsenal::ArsenalError` |
  | `paladin::CitadelPort` | `paladin_ports::output::citadel_port::CitadelPort` |
  | `paladin::CitadelError` | `paladin_core::application::errors::citadel_error::CitadelError` |
  | `paladin::CitadelServiceError` | `paladin_core::application::errors::citadel_error::CitadelError` |
  | `paladin::QueuePort` | `paladin_ports::output::queue_port::QueuePort` |
  | `paladin::QueueError` | `paladin_core::application::use_cases::queue_orchestrator::QueueError` |
  | `paladin::NotificationDeliveryPort` | `paladin_ports::output::notification_port::NotificationDeliveryPort` |
  | `paladin::NotificationTemplatePort` | `paladin_ports::output::notification_port::NotificationTemplatePort` |
  | `paladin::Notification` | `paladin_ports::output::notification_port::Notification` |
  | `paladin::NotificationChannel` | `paladin_ports::output::notification_port::NotificationChannel` |
  | `paladin::NotificationPortError` | `paladin_ports::output::notification_port::NotificationPortError` |
  | `paladin::NotificationPriority` | `paladin_ports::output::notification_port::NotificationPriority` |
  | `paladin::NotificationStatus` | `paladin_ports::output::notification_port::NotificationStatus` |
  | `paladin::NotificationTemplate` | `paladin_ports::output::notification_port::NotificationTemplate` |
  | `paladin::FileStorageError` | `paladin_ports::output::file_storage_port::FileStorageError` |
  | `paladin::FileStoragePort` | `paladin_ports::output::file_storage_port::FileStoragePort` |
  | `paladin::PaladinPort` | `paladin_ports::output::paladin_port::PaladinPort` |
  | `paladin::PaladinResult` | `paladin_ports::output::paladin_port::PaladinResult` |
  | `paladin::StopReason` | `paladin_ports::output::paladin_port::StopReason` |
  | `paladin::BattalionPort` | `paladin_ports::output::battalion_port::BattalionPort` |
  | `paladin::BattalionResult` | `paladin_core::platform::container::battalion::BattalionResult` |
  | `paladin::BattalionStatus` | `paladin_core::platform::container::battalion::BattalionStatus` |
  | `paladin::paladin_battalion` | `paladin_battalion` (direct crate dependency) |
  | `paladin::ContentIngestionPort` | `paladin_ports::input::content_input_port::ContentIngestionPort` |
  | `paladin::DocumentPort` | `paladin_ports::input::document_port::DocumentPort` |
  | `paladin::MlPort` | `paladin_ports::input::ml_port::MlPort` |
  | `paladin::Campaign` | `paladin_battalion::campaign::Campaign` |
  | `paladin::ChainOfCommand` | `paladin_battalion::chain_of_command::ChainOfCommand` |
  | `paladin::Formation` | `paladin_battalion::formation::Formation` |
  | `paladin::Phalanx` | `paladin_battalion::phalanx::Phalanx` |
  | `paladin::Armament` | `paladin_core::platform::container::arsenal::Armament` |
  | `paladin::ArmamentCall` | `paladin_core::platform::container::arsenal::ArmamentCall` |
  | `paladin::ArmamentResult` | `paladin_core::platform::container::arsenal::ArmamentResult` |
  | `paladin::CommanderBuilder` | `paladin_battalion::commander::CommanderBuilder` |
  | `paladin::PaladinBuilder` | `paladin_core::application::use_cases::paladin::PaladinBuilder` |
  | `paladin::CouncilBuilder` | `paladin_battalion::council::CouncilBuilder` |
  | `paladin::GroveBuilder` | `paladin_battalion::grove::GroveBuilder` |
  | `paladin::PaladinError` | `paladin_core::application::use_cases::paladin::error::PaladinError` |
  | `paladin::CollectionType` | `paladin_core::base::entity::collection::CollectionType` |
  | `paladin::Field` | `paladin_core::base::entity::field::Field` |
  | `paladin::Message` | `paladin_core::base::entity::message::Message` |
  | `paladin::Node` | `paladin_core::base::entity::node::Node` |

  **Still exported as short-path aliases** (confirmed workspace consumers):
  - `paladin::MockLlmAdapter`, `paladin::MultiStepMockLlmPort` — used in 13+ tests and examples
  - `paladin::OpenAIAdapter`, `paladin::OpenAIConfig` — used in examples and integration tests
  - `paladin::AnthropicAdapter`, `paladin::AnthropicConfig` — used in examples and integration tests
  - `paladin::DeepSeekAdapter`, `paladin::DeepSeekConfig` — used in integration tests
  - `paladin::OpenAIEmbeddingAdapter`, `paladin::OpenAIEmbeddingConfig` — used in embedding tests
  - `paladin::LlmProviderFactory` — used in integration tests
  - `paladin::Paladin`, `paladin::PaladinData`, `paladin::PaladinStatus` — used in 17+ consumers
  - `paladin::PaladinConfig`, `paladin::BattalionConfig`, `paladin::BattalionError` — used in `cli_isolation_test`

  **Also removed** — 26 empty/orphaned source files from `src/` (Milestone 8, Epic 2, Tasks 2.0–4.0):
  - Application layer: `notifications/` directory (3 files), `storage/` stubs (4 files),
    `use_cases/content/` empties (3 files), `use_cases/subject/` directory (5 files + mod.rs)
  - Core layer: `core/platform/manager/admin/` (4 files), `core/platform/manager/user/` (4 files)
  - Infrastructure layer: `adapters/logs/access_log_adapter.rs`, `adapters/notifications/push_notification_adapter.rs`

### Changed - BREAKING
- **Default Feature Flags Revised**: Default features changed from `["redis-queue", "s3-storage", "openai-embeddings"]` to `["llm-openai"]` only
  - **Impact**: Applications relying on Redis queue, S3 storage, or OpenAI embeddings in default builds must now explicitly enable these features
  - **Migration**: Add required features to `Cargo.toml`: `paladin = { version = "0.1", features = ["redis-queue", "s3-storage"] }`
  - **Reason**: Enables minimal builds for pure orchestration use cases, reduces compile times and binary sizes
  - See [docs/MIGRATION.md](docs/MIGRATION.md) for complete migration guide

### Changed
- **Internal Type Visibility**: Applied `#[doc(hidden)]` to ~60 adapter and repository types (Milestone 4, Epic 2, Task 7.0)
  - **Affected Types**: All LLM adapters, Garrison adapters, Sanctum adapters, Arsenal adapters, Herald formatters, Repository implementations, and infrastructure adapters
  - **Impact**: No breaking changes - types remain accessible but hidden from documentation
  - **Strategy**: Used `#[doc(hidden)]` instead of `pub(crate)` to maintain compatibility with examples/tests/benchmarks (separate crates)
  - **User Guidance**: Consumers should use port traits (e.g., `LlmPort`, `GarrisonPort`) instead of concrete adapter types
  - **No import path changes required** - all existing code continues to work unchanged
  - See [project/DEPRECATIONS.md](project/DEPRECATIONS.md) for API transition strategy

### Added
- **Feature Flag System**: Comprehensive feature flags for controlling compiled dependencies
  - LLM Provider Flags: `llm-openai`, `llm-anthropic`, `llm-deepseek`, `llm-all`
  - Subsystem Flags: `vision`, `content-processing`, `web-server`, `notifications`
  - Infrastructure Flags: `redis-queue`, `s3-storage`, `openai-embeddings`, `qdrant`
  - Convenience Flag: `full` (enables all optional features)
  - See [docs/FEATURE_FLAGS.md](docs/FEATURE_FLAGS.md) for complete reference
- **CI Feature Matrix**: GitHub Actions workflow testing 15 feature combinations
  - Tests: no-default, default, all-features, full, individual providers and subsystems
  - Ensures all feature combinations compile and pass tests
  - See [.github/workflows/feature-flags.yml](.github/workflows/feature-flags.yml)

### Fixed
- **Live API Tests**: All OpenAI and Anthropic live API tests now passing (10/10 essential tests)
  - OpenAI: Fixed model assertion to handle versioned models (e.g., "gpt-3.5-turbo-0125")
  - OpenAI: Added graceful streaming error handling for incomplete JSON chunks
  - Anthropic: Fixed struct deserialization by removing underscore-prefixed fields
  - Anthropic: Updated test model to claude-3-haiku-20240307 (wider API tier access)
  - Anthropic: Added graceful streaming error handling
  - All tests verified with real API calls and comprehensive output validation
  - See **Milestone 3: Post-Epic 24 Completion** section below and `project/Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_SUCCESS.md` for complete documentation

### Removed
- **Legacy OpenAI Adapter**: Removed unused `openai_llm_adapter.rs` from `infrastructure/adapters/output/`
  - All functionality migrated to `infrastructure/adapters/llm/openai_adapter.rs`
  - Updated documentation references in `docs/HERALD.md`
  - Updated code examples in `examples/llm_provider_selection.rs`
  - Zero functional impact - adapter had no actual usage in codebase
  - See **Milestone 3: Post-Epic 24 Completion** section below for complete cleanup details
- **Legacy Root Benchmark Files**: Removed obsolete root-level benchmark files that no longer match the workspace benchmark ownership model
  - Removed `benches/battalion_benchmarks.rs` and `benches/garrison_benchmarks.rs` in favor of planned crate-local replacements under `paladin-battalion` and `paladin-memory`
  - Removed `benches/herald_benchmarks.rs` because Herald formatting is outside Epic 3's approved critical-path benchmark scope
  - Removed `benches/paladin_benchmarks.rs.disabled` and `benches/arsenal_benchmarks.rs.disabled` because they target outdated architectural boundaries and would require out-of-scope rewrites
  - Benchmark migration continues under Milestone 7 Epic 3 with crate-owned replacements

---

## Milestone 3: Post-Epic 24 Completion & Test Hardening

**Status**: ✅ Complete
**Branch**: `bugs/epic-24-post-fixes`
**Documentation**: `project/Milestone_3-Completion/Post-Epic_24-cleanup/`

This section documents the comprehensive cleanup, hardening, and bug fixes performed after Epic 24 to finalize Milestone 3. All work focused on ensuring production-readiness through integration test fixes, infrastructure improvements, and code quality enhancements.

### Added - Post-Epic 24 Completion

#### DevContainer Docker Compose Integration
- **Full docker-compose integration** for development services
  - Configured DevContainer to use `docker-compose.yml` for service orchestration
  - Services: Redis (queue), MySQL (storage), MinIO (S3-compatible storage)
  - Automatic service startup on container creation
  - Network: `paladin-network` for inter-service communication
- **DevContainer configurations**:
  - Features: rust, docker-in-docker, git
  - Mounts: cargo cache, target directory, git config
  - Post-create commands: install cargo-nextest, restore dependencies
  - VS Code extensions: rust-analyzer, crates, better-toml, GitLens
- **Service health checks and readiness**:
  - Redis: automatic connection test on startup
  - MySQL: root user with full privileges
  - MinIO: S3-compatible API on port 9000, console on 9001
- **Documentation updates**:
  - Updated `.devcontainer/README.md` with service details
  - Service connection information and credentials
  - Troubleshooting guide for common DevContainer issues

### Fixed - Post-Epic 24 Completion

#### Integration Test Fixes
- **Redis Queue Integration Tests** (all tests now passing):
  - Fixed Redis connection to use external docker-compose service instead of testcontainers
  - Updated connection from localhost to `redis` service hostname
  - Modified tests to support persistent Redis service (clear existing queues before tests)
  - Added proper cleanup: `FLUSHDB` command to reset state between tests
  - Removed testcontainers dependency from Redis queue tests (simplified infrastructure)
  - All 6 Redis queue integration tests passing
  - Tests documented: enqueue/dequeue, priority, batch operations, error handling

- **SQLite Garrison Integration Tests** (all tests now passing):
  - Fixed path resolution for in-memory SQLite databases
  - Changed from `:memory:` to unique file-based paths for test isolation
  - Added proper cleanup: remove test database files after completion
  - Fixed concurrent test execution issues (unique DB per test)
  - All 12 garrison integration tests passing
  - Tests documented: CRUD operations, search, TTL, concurrent access

- **LLM Provider Integration Tests** (modernized API):
  - Updated OpenAI integration test to use current `OpenAIAdapter` API
  - Fixed import paths from legacy `output::openai_llm_adapter` to `llm::openai_adapter`
  - Updated type names: `OpenAILlmAdapter` → `OpenAIAdapter`
  - Fixed configuration API: `OpenAIConfig::new()` now takes single argument (api_key)
  - Corrected provider name assertion: expects lowercase "openai"
  - Removed duplicate `cfg` attributes in live API tests
  - All integration tests compile and run successfully

#### Code Quality & Cleanup
- **Dead Code Warnings Resolved**:
  - Added `#[allow(dead_code)]` for deserialization-only fields in `OpenAIAdapter`
  - Suppressed warnings for: `OpenAIResponse.id`, `OpenAIChoice.index`, `OpenAIStreamChunk.id`, `OpenAIStreamChoice.index`, `OpenAIStreamDelta.role`
  - Added `#[allow(dead_code)]` for `RedisContainer.container` field (required for RAII)
  - All fields necessary for proper struct deserialization or resource management

- **Test Code Cleanup**:
  - Removed superfluous `vec![]` in test assertions (use direct comparison)
  - Fixed formatting inconsistencies in test files
  - Removed unused imports and dead test helper code
  - Cleaned up deprecated test patterns

- **Provider Factory Test Fixes**:
  - Fixed `test_case_insensitive_provider_names` to be environment-agnostic
  - Test now handles both success (API key present) and ConfigurationMissing (API key absent)
  - No longer assumes API keys are missing in test environment
  - Properly validates case-insensitive provider name matching

#### DevContainer Configuration
- **Formatting and Structure**:
  - Reformatted `.devcontainer/devcontainer.json` for consistency
  - Added inline comments explaining each configuration section
  - Standardized indentation and JSON structure
  - Improved readability of mounts and features configuration

- **Settings Corrections**:
  - Fixed rust-analyzer settings for better IDE experience
  - Corrected cargo check settings for faster feedback
  - Updated file associations for better file type recognition
  - Aligned editor settings with project conventions

### Changed - Post-Epic 24 Completion

#### Test Infrastructure
- **Integration Test Strategy**:
  - Redis tests: external service via docker-compose (no testcontainers)
  - SQLite tests: file-based databases with unique paths (better isolation)
  - LLM tests: feature-gated `live-api-tests` with proper `#[ignore]` markers
  - Clear separation: unit tests (always run) vs integration tests (opt-in)

- **Service Architecture**:
  - Redis: persistent service (not ephemeral testcontainer)
  - Requires explicit state cleanup in tests (`FLUSHDB`)
  - Better reflects production environment (persistent service)
  - Faster test execution (no container startup time)

- **Documentation**:
  - Added comprehensive test fix documentation in `Post-Epic_24-cleanup/`
  - `BUILD_TEST_FIXES.md`: Details all test compilation fixes
  - `LEGACY_CLEANUP_SUMMARY.md`: Legacy adapter removal summary
  - `LIVE_API_TESTS_SUCCESS.md`: Comprehensive live API test fixes
  - `QUICK_SUMMARY.md`: Quick reference for test status
  - `SESSION_SUMMARY.md`: Complete session chronicle
  - `verify_live_api_tests.sh`: Automated verification script

### Technical Debt Resolution

#### Resolved Issues
1. ✅ Legacy OpenAI adapter confusion (removed 580+ lines of dead code)
2. ✅ Integration test failures (Redis, SQLite, LLM providers - all fixed)
3. ✅ DevContainer service integration (docker-compose working)
4. ✅ Live API test robustness (graceful streaming error handling)
5. ✅ Code quality warnings (all dead_code warnings properly addressed)
6. ✅ Test environment dependencies (Redis testcontainer removed)

#### Quality Metrics
- **All unit tests passing**: 1606/1606 (100%)
- **All integration tests passing**: Redis 6/6, SQLite 12/12, LLM providers 10/10
- **Live API tests**: OpenAI 6/6, Anthropic 4/4 (100% essential tests)
- **Build status**: Clean compilation with `cargo check`
- **Code quality**: Zero clippy warnings with `cargo clippy -- -D warnings`
- **Formatting**: All code formatted with `cargo fmt`

### Production Readiness

#### Milestone 3 Completion Criteria
✅ **All Epic 24 tests passing** (100% test success rate)
✅ **Live API integration verified** (OpenAI, Anthropic with real API calls)
✅ **DevContainer fully operational** (docker-compose services working)
✅ **Integration tests hardened** (Redis, SQLite, Qdrant, LLM providers)
✅ **Code quality standards met** (zero warnings, all formatting checks pass)
✅ **Documentation complete** (comprehensive cleanup docs in Post-Epic_24-cleanup/)
✅ **Legacy code removed** (580+ lines of unused code eliminated)

#### Coverage Statistics
- **Total tests**: 1,628 (1606 unit + 22 integration)
- **Test execution time**: < 10 seconds for full test suite
- **CI-ready**: All tests deterministic, no flaky tests
- **API independence**: Core tests run without API keys

---

### Added - Epic 23: CLI, Config & Infrastructure Completion

#### Garrison Configuration
- Complete garrison (memory) configuration support from YAML files
- Support for `in_memory` garrison type: fast, temporary memory storage
- Support for `sqlite` garrison type: persistent memory with database backing
- Configuration options: `max_entries`, `ttl_seconds`, `path` for SQLite
- Garrison wiring in CLI agent command (resolved TODO at line 293)
- 9 comprehensive unit tests covering all configuration scenarios
- Example configurations in `examples/cli_configs/paladin_with_garrison.yaml`
- Comprehensive error handling with actionable error messages

#### Arsenal/MCP Configuration
- Complete arsenal (external tools) configuration support from YAML files
- Support for STDIO MCP servers: command-line tools via stdin/stdout
- Support for SSE MCP servers: HTTP-based tools via Server-Sent Events
- Automatic tool discovery and registration from MCP servers
- Support for environment variable substitution in configs (`${VAR_NAME}`)
- Arsenal wiring in CLI agent command (resolved TODO at line 296)
- 8 comprehensive unit tests covering STDIO, SSE, and validation scenarios
- Example configurations in `examples/cli_configs/paladin_with_arsenal.yaml`
- Integration examples: web search, filesystem, GitHub, custom APIs

#### Mock LLM Infrastructure
- **MockLlmAdapter** for CI-ready testing without API keys (`tests/helpers/mock_llm_adapter.rs`)
- Configurable responses: text, tool calls, streaming, and error injection
- Invocation recording for test assertions and verification
- Tool call simulation for arsenal integration testing
- Builder pattern for fluent mock configuration
- Support for sequential response queues
- Zero external dependencies for core test suite

#### Mock Arsenal Infrastructure
- **MockArsenalPort** for in-process tool testing (`tests/helpers/mock_arsenal_adapter.rs`)
- Tool registration with schemas and response configuration
- Success and error response simulation
- Invocation tracking with argument capture
- 9 unit tests for mock infrastructure validation
- Enables comprehensive tool integration testing in CI

#### CLI Integration Tests
- **84 comprehensive CLI integration tests**, all passing
- 6 Paladin execution tests: basic, with garrison, with arsenal, with config
- 4 Formation execution tests: sequential flow, output chaining, error propagation
- 5 Phalanx execution tests: parallel execution, result aggregation, error handling
- 8 Tool integration tests: LLM ↔ Arsenal ↔ result loop (Task 4.6)
  - Core flow: function call → Arsenal execution → result
  - Error handling: no arsenal, unknown tool, invalid arguments, execution errors
  - Advanced: sequential tool chains, garrison+arsenal integration
- 14 Error handling tests: configuration errors, execution failures, validation
- 9 Garrison configuration tests: in-memory, SQLite, validation, errors
- 8 Arsenal configuration tests: STDIO, SSE, tool registration, errors
- All tests use mock infrastructure - **zero API keys required**
- **CI-ready**: complete in < 5 seconds, no external dependencies

#### Scheduler Integration
- Production-ready scheduler using tokio-cron-scheduler v0.13
- **SchedulerPort trait** (`src/application/ports/output/scheduler_port.rs`):
  - Methods: `schedule_job()`, `cancel_job()`, `list_jobs()`, `get_job_info()`
  - Types: JobId, JobSpec, JobInfo, JobStatus, SchedulerError
  - 6 inline tests for trait contract
- **TokioCronSchedulerAdapter** (`src/infrastructure/adapters/scheduling/tokio_cron_adapter.rs`):
  - Full cron expression support for scheduling
  - Job lifecycle management (create, cancel, list, query)
  - Error handling and logging
  - 13 inline tests for adapter implementation
- **APIContentDeliverer integration**:
  - Replaced scheduler stub (resolved TODO at line 297)
  - `schedule_delivery()` creates real scheduled jobs
  - `cancel_delivery()` cancels pending deliveries
  - Returns JobId for job tracking
- **Configuration support**:
  - SchedulerConfig in `src/config/application_settings.rs`
  - Fields: `enabled`, `default_cron`, `channel_size`
  - YAML configuration support
- 21 total scheduler tests (16 unit + 5 integration)

#### Documentation
- **CLI Configuration Guide** (`docs/cli/CONFIGURATION.md`, 500+ lines):
  - Comprehensive guide for garrison, arsenal, and scheduler configuration
  - Complete YAML configuration examples with detailed comments
  - Environment variable usage and substitution
  - Troubleshooting section with common errors and solutions
  - Integration examples for popular MCP servers
- **CLI Testing Guide** (`docs/cli/TESTING.md`) updates:
  - Mock infrastructure documentation (MockLlmAdapter, MockArsenalPort)
  - Test tier strategy (no deps, Docker-gated, API-key-gated)
  - Test coverage statistics and categories
  - Best practices for writing tests with mocks
- **CLI Usage Guide** (`docs/CLI_USAGE.md`) updates:
  - References to new CONFIGURATION.md guide
  - Updated with garrison and arsenal capabilities
  - Example usage patterns

#### Configuration Examples
- `examples/cli_configs/paladin_with_garrison.yaml` - In-memory and SQLite garrison examples
- `examples/cli_configs/paladin_with_arsenal.yaml` - STDIO and SSE MCP server examples
- `examples/cli_configs/paladin_full_config.yaml` - Complete configuration with all features
- All examples include extensive inline comments and usage instructions
- Examples tested and validated for out-of-the-box functionality

### Changed - Epic 23: CLI, Config & Infrastructure Completion

#### Configuration Loading
- Extended `PaladinYamlConfig` with garrison and arsenal configuration structures
- Enhanced ConfigLoader with garrison and arsenal parsing methods
- Added environment variable resolution for sensitive configuration values
- Improved error messages with actionable guidance

#### CLI Command Infrastructure
- Removed TODO at `src/application/cli/commands/agent.rs` line 293 (garrison wiring)
- Removed TODO at `src/application/cli/commands/agent.rs` line 296 (arsenal wiring)
- Garrison adapter instantiation based on YAML config
- Arsenal registry population from MCP server configs
- Integration with PaladinBuilder for full feature wiring

#### Content Delivery Infrastructure
- Removed scheduler stub at `src/infrastructure/adapters/output/api_content_deliverer.rs` line 297
- Integrated SchedulerPort for scheduled content delivery
- Added cancellation support for pending scheduled deliveries
- JobId tracking for scheduled tasks

#### Test Organization
- Implemented three-tier test strategy:
  - **Tier 1**: Core functionality, no dependencies (84 tests, runs in CI)
  - **Tier 2**: Docker-gated service tests (#[ignore], clear skip messages)
  - **Tier 3**: API-key-gated provider tests (feature flag + #[ignore])
- All Tier 1 tests CI-ready with deterministic execution
- Test helper module exports: MockLlmAdapter, MockArsenalPort, MockPaladinPort

### Fixed - Epic 23: CLI, Config & Infrastructure Completion

#### Code Quality
- Resolved all Epic 23 scope TODOs (3 total: agent.rs lines 293, 296; api_content_deliverer.rs line 297)
- All code passes `cargo clippy -- -D warnings` with zero warnings
- All code formatted with `cargo fmt` - zero formatting issues
- Zero compilation warnings in Epic 23 changes

#### Test Coverage
- Closed critical test coverage gap: LLM ↔ Arsenal ↔ result tool call loop (8 tests added)
- Added missing garrison configuration tests (9 tests)
- Added missing arsenal configuration tests (8 tests)
- Added missing error handling tests (14 tests)
- **Test count:** 84 new CLI integration tests, all passing

#### Deferred Task Completion
- **Epic 9, Task 5.8**: Garrison configuration wiring ✅
- **Epic 9, Task 5.9**: Arsenal/MCP configuration wiring ✅
- **Epic 10, Tasks 13.4-13.6**: CLI integration tests for Paladin, Formation, Phalanx ✅
- **Epic 18, Tasks 9.1-9.7**: End-to-end testing and test documentation ✅
- **All Milestone 3 deferred tasks now complete**

### Added - Epic 22: Battalion & Commander Hardening

#### Commander Metadata Export
- Commander now exports detailed execution metadata to JSON files when `metadata_output_dir` is configured
- JSON files use naming convention: `{strategy}_{timestamp}_{uuid_short}.json`
- Metadata includes: battalion_id, battalion_name, strategy_used, timestamps, final_output, paladin_results
- Per-paladin execution metrics: execution times and token usage independently tracked
- Comprehensive metadata structure for audit trails, debugging, and performance analysis
- Automatic directory creation and validation with detailed error messages
- Integration test coverage for end-to-end metadata export validation

#### Enhanced Phalanx Metrics Collection
- Phalanx now tracks per-paladin execution times in `per_paladin_times: HashMap<String, u64>`
- Per-paladin token usage tracked in `per_paladin_tokens: HashMap<String, TokenUsage>`
- Total token aggregation across all parallel executions in `total_tokens: u64`
- Success/failure counts: `paladin_success_count` and `paladin_failure_count`
- Metrics collected concurrently for accurate parallel execution profiling
- Enhanced BattalionResult with comprehensive performance data
- 100% test coverage for metrics collection across all Battalion patterns

#### Test Infrastructure Improvements
- MockLlmAdapter test infrastructure with configurable response queueing
- Call count tracking and state management for repeatable tests
- Helper functions: `create_mock_with_responses()`, `create_test_paladin_with_mock()`
- Strategy-specific mock implementations (MockChainOfCommandPort for delegation testing)
- Comprehensive test coverage for Campaign and ChainOfCommand orchestration patterns
- Error handling tests: FailFast, ContinueOnError, RetryThenContinue, partial failure scenarios
- Integration test for Commander metadata export with JSON validation
- All 1590 lib tests passing, 211 doctests passing, 19 integration tests

#### Paladin Registry Foundation
- PaladinRegistry trait defining standard interface for Paladin lookup and management
- HashMapPaladinRegistry implementation with thread-safe concurrent access via RwLock
- O(1) average case lookup performance for Paladin retrieval by ID
- Methods: `register()`, `unregister()`, `get()`, `contains()`, `list_ids()`, `clear()`, `count()`
- Duplicate ID prevention with detailed error reporting
- Full rustdoc with usage examples and performance characteristics
- Ready for Council and Grove integration (implementation in Epic 22 Sprint 2)

### Changed - Epic 22: Battalion & Commander Hardening

#### BattalionConfig Enhancements
- Added `metadata_output_dir: Option<PathBuf>` for optional metadata export configuration
- New `validate_metadata_dir()` method ensures directory exists and is writable before execution
- Builder pattern method: `with_metadata_dir(dir: PathBuf)` for fluent configuration
- Comprehensive error messages for directory validation failures

#### BattalionResult Structure
- Extended with `TokenUsage` struct containing `prompt_tokens`, `completion_tokens`, `total_tokens`
- Added `per_paladin_times: HashMap<String, u64>` for granular timing data
- Added `per_paladin_tokens: HashMap<String, TokenUsage>` for granular token tracking
- Added `total_tokens: u64` for cross-Battalion token aggregation
- Added `paladin_success_count: usize` and `paladin_failure_count: usize` for execution summaries
- Backward-compatible additions, all existing code continues to work

#### Commander Test Coverage
- Enabled and fixed previously ignored Campaign orchestration tests with DAG validation
- Enabled and fixed previously ignored ChainOfCommand delegation tests with specialist selection
- Added comprehensive error handling test suite covering all ErrorStrategy variants
- MockChainOfCommandPort returns properly formatted "SELECT: name1, name2\nREASON: ..." responses
- Unique paladin naming in tests to avoid graph cycle detection false positives
- 50 Commander unit tests now passing (up from ~40 with ignored tests)

### Fixed - Epic 22: Battalion & Commander Hardening

#### Code Quality
- Resolved clippy warning: unused loop variable in phalanx_service.rs timing validation
- Resolved clippy warning: manual Option::map pattern in paladin_builder.rs MockArsenalRegistry
- Fixed 6 failing doctests in PaladinRegistry with correct trait imports and API usage
- Fixed typo in HandoffConfig doctest (removed `stat` field)
- All code now passes `cargo clippy -- -D warnings` with zero warnings

#### Test Reliability
- Fixed ChainOfCommand test failures by implementing context-aware mock responses
- Fixed Campaign test failures by ensuring unique paladin names in DAG construction
- MockPaladinPort enhanced with configurable response strategies per test scenario
- All integration tests now run reliably without flaky failures

### Added - Epic 20: Vision Pipeline Completion

#### Vision Configuration System
- Complete vision configuration support with retry logic and token limits
- `VisionConfig` struct with configurable retry parameters: `max_retries`, `initial_backoff_ms`, `backoff_multiplier`
- Provider-specific token limits for OpenAI and Anthropic
- Exponential backoff for transient failures (network errors, rate limits, timeouts)
- Configuration loaded from `config.yml` with sensible defaults
- Test configuration support in `config.test.yml`

#### Vision Error Handling
- Comprehensive `VisionError` enum with 10 error variants
- Error types: `InvalidImage`, `UnsupportedFormat`, `AuthenticationError`, `RateLimitExceeded`, `ProviderError`, `NetworkError`, `Timeout`, `UnsupportedProvider`, `MaxRetriesExceeded`, `FileTooLarge`
- Detailed error messages with context for debugging
- Integration with existing error handling patterns via `thiserror`

#### OpenAI Vision Adapter
- Full OpenAI vision API integration with retry logic
- Support for URL-based and base64-encoded images
- Image detail levels: Auto, Low (512x512), High (2048x2048)
- Multiple images per request (up to 10)
- Token estimation: ~85 tokens (low), ~170 tokens per tile (high)
- Models supported: `gpt-4o`, `gpt-4o-mini`, `gpt-4-vision-preview`
- Automatic retry with exponential backoff on transient failures
- Comprehensive unit tests with mock server validation

#### Anthropic Vision Adapter
- Full Anthropic vision API integration with retry logic
- Support for URL-based images (auto-converted to base64) and base64-encoded images
- Image detail levels with automatic conversion
- Multiple images per request (up to 20)
- Models supported: `claude-3-opus`, `claude-3-sonnet`, `claude-3-haiku`
- Automatic base64 encoding for all image types
- Automatic retry with exponential backoff on transient failures
- Comprehensive unit tests with mock server validation

#### Paladin Vision Execution
- `execute_with_vision()` method added to `PaladinExecutionService`
- Seamless integration with existing Paladin execution flow
- Support for vision-capable LLM providers through trait abstraction
- Vision content validation before API calls
- Memory (Garrison) integration for vision analysis history
- Tool (Arsenal) integration for vision-augmented agents
- Circuit breaker support for fault tolerance

#### Vision Integration Tests
- Environment-gated integration tests with real API calls
- Tests controlled by `ENABLE_VISION_TESTS` environment variable
- OpenAI vision integration tests with multiple scenarios
- Anthropic vision integration tests with multiple scenarios
- Multiple images test, image URL test, high detail processing test
- Test fixtures: sample images for integration testing
- Comprehensive documentation for running tests with API keys

#### Examples and Documentation
- Updated `vision_analysis.rs` example with comprehensive demonstrations
- Base64-encoded image processing example
- Multiple images comparison example
- Error handling patterns and best practices
- Added vision retry configuration documentation to `SENTINEL.md`
- Image size limits documentation (OpenAI: 20MB, Anthropic: 5MB)
- Troubleshooting section for common vision issues
- Configuration examples and best practices for different environments

### Changed - Epic 20: Vision Pipeline Completion

#### Configuration Structure
- Enhanced `ApplicationSettings` with `vision: VisionConfig` field
- Vision configuration loaded from YAML with proper deserialization
- Backward-compatible configuration loading (vision section optional)

#### LLM Adapters Enhancement
- OpenAI adapter constructor updated to accept `VisionConfig`
- Anthropic adapter constructor updated to accept `VisionConfig`
- Vision-specific retry logic separated from general LLM retries
- Provider capabilities detection for vision support

### Added - Epic 19: Herald & Domain Type Consolidation

#### StreamChunk Extensible Metadata
- Complete StreamChunk structure with 7 fields including extensible metadata HashMap
- Builder pattern with validation for safe construction
- Fields: `chunk_id`, `sequence_number`, `timestamp`, `content`, `token_count`, `is_final`, `metadata`
- Support for provider-specific and custom metadata without struct changes
- JSON serialization/deserialization with flattened metadata
- Comprehensive rustdoc with multiple usage examples

#### ExecutionMetadata Full Telemetry
- Complete ExecutionMetadata structure with 9 fields for comprehensive observability
- Builder pattern with validation for safe construction
- Fields: `execution_id`, `start_time`, `end_time`, `duration_ms`, `model_used`, `token_usage`, `cost_estimate`, `error_count`, `metadata`
- Duration calculation helper method
- Total cost estimation helper method
- Extensible metadata for custom telemetry
- Re-exported `TokenUsage` from llm_port with consistent field names
- Comprehensive rustdoc with telemetry use cases and examples

#### Auto-Registration of Built-in Formatters
- `HeraldRegistry::default()` automatically registers three built-in formatters
- Zero-config pattern: JSON, Markdown, and Table formatters available immediately
- No manual registration required for built-in formatters
- Custom formatters can still be added via `register()` method
- Built-in formatters can be overridden with custom configurations
- Updated rustdoc with zero-config and extensible patterns

### Changed - Epic 19: Herald & Domain Type Consolidation

#### Domain Type Consolidation
- Removed placeholder `PaladinResult`, `BattalionResult`, and `PaladinError` types from herald.rs
- Herald system now uses actual domain types from paladin.rs and battalion modules
- Added public re-exports for Herald consumers: `PaladinResult`, `BattalionResult`, `PaladinError`, `TokenUsage`
- Updated all Herald adapters (JSON, Markdown, Table) to use actual type structures
- Fixed field access patterns: `paladin_results` instead of `results` for Battalion
- PaladinError now handled as enum with match on variants

#### Documentation Improvements
- Enhanced StreamChunk rustdoc with detailed field descriptions and extensible metadata examples
- Enhanced ExecutionMetadata rustdoc with telemetry use cases and comprehensive examples
- Updated HeraldRegistry rustdoc documenting auto-registered formatters and usage patterns
- Added examples for zero-config pattern (recommended) and manual registration
- Updated all Herald-related documentation for consolidated types

### Added - Epic 18: CLI Enhancement

#### New CLI Commands

**Onboarding Wizard** (`paladin onboarding`)
- Interactive first-time setup wizard for environment configuration
- Provider selection (OpenAI, DeepSeek, Anthropic) with descriptions
- Secure API key input with validation and masking
- Automatic `.env` file generation with comments
- Sample configuration file creation
- Resumable state for interrupted sessions
- Real-time validation of API keys and connectivity

**Setup Check** (`paladin setup-check`)
- Comprehensive environment validation
- System checks: Rust version, cargo, git availability
- Environment checks: Required and optional variables
- Provider validation: API key format and connectivity
- Optional service checks: Redis, Qdrant, MinIO
- Categorized results: System, Environment, Provider, Service
- Multiple output formats: standard, verbose, JSON
- Exit codes: 0 (success), 1 (critical failures), 2 (warnings only)
- CI/CD integration support

**Features Discovery** (`paladin features`)
- Discover all available Paladin capabilities
- Feature categories: Agent, Battalion, Orchestration, Memory, Utilities
- 24 documented features with descriptions and documentation links
- Orchestration patterns: Formation, Phalanx, Campaign, Chain of Command, Conclave, Council, Grove, Maneuver
- Memory systems: Garrison (InMemory, Sqlite), Sanctum (InMemory, Qdrant)
- Category filtering: `--category` flag
- Output formats: table (default), JSON
- Feature availability status indicators

**Muster Command** (`paladin muster`) [STUB]
- AI-powered Battalion configuration generation from natural language
- LLM-based task analysis and pattern suggestion
- Automatic YAML/JSON config generation
- Validates generated configurations
- Supports all orchestration patterns
- Note: Requires LLM integration (currently returns stub configurations)

**Council Command** (`paladin council`) [STUB]
- Quick multi-agent discussions without configuration files
- Multiple discussion modes: parallel, sequential, debate
- Configurable agent roles and perspectives
- Automatic synthesis of diverse viewpoints
- Output formats: markdown, JSON, plain text
- Note: Requires LLM integration (currently returns mock discussions)

#### CLI Infrastructure

**Output Formatters**
- `OutputFormatter`: Unified formatter for CLI output with colored styling
- `TableFormatter`: ASCII table rendering with alignment and borders
- Consistent styling: success (green), error (red), warning (yellow), info (cyan)
- NO_COLOR environment variable support
- Support for both TTY and non-TTY environments

**Progress Indicators**
- `ProgressSpinner`: Async spinner for long-running operations
- `ProgressBar`: Progress tracking with percentage and ETA
- Customizable messages and styling
- Automatic cleanup on completion or error

**Error Handling**
- `CliError` enum with 30+ specific error variants
- Detailed error messages with context
- Error categories: Configuration, IO, Validation, Provider, Service
- Proper error propagation with `CliResult<T>`
- User-friendly error formatting

**Templates**
- `.env` file template generation with provider-specific sections
- Paladin configuration templates (YAML) for all providers
- Battalion configuration templates for all orchestration patterns
- Template merging for incremental updates
- Valid YAML/JSON output with comments

#### Documentation

**CLI Usage Guide** (`docs/CLI_USAGE.md`)
- Comprehensive command reference (405 new lines)
- Getting Started section with onboarding workflow
- Detailed syntax, options, and examples for all commands
- Cross-references to detailed guides

**Detailed Command Guides** (~1,900 lines total)
- `docs/cli/ONBOARDING.md`: Wizard flow, security, troubleshooting (~300 lines)
- `docs/cli/SETUP_CHECK.md`: Check categories, exit codes, CI/CD (~350 lines)
- `docs/cli/MUSTER.md`: AI-powered generation, patterns, examples (~600 lines)
- `docs/cli/COUNCIL.md`: Multi-agent discussions, modes, advanced usage (~650 lines)

**README Updates**
- Added CLI Quick Start section (65 lines)
- Installation instructions
- First-time setup with onboarding wizard
- Quick commands reference
- Links to comprehensive documentation

**Example Configurations**
- Enhanced `examples/cli_configs/paladin_with_rag.yaml`
- Verified existing examples: basic_paladin, formation, phalanx
- All examples include usage instructions

#### Testing

**Test Infrastructure** (29 new tests, 193 CLI tests total)
- Mock test utilities in `src/application/cli/tests/mod.rs`
- `formatter_tests.rs`: 13 tests for output and table formatters
- `command_tests.rs`: 16 tests for command validation and parsing
- Integration test framework in `tests/cli/integration_tests.rs`

**Test Coverage**
- 193 CLI unit tests (100% pass rate, 6 intentionally ignored)
- All tests follow TDD principles
- Zero clippy warnings with `-D warnings` flag
- Code formatted with `cargo fmt`
- 1,487 total project tests passing

#### Architecture & Code Quality

**Hexagonal Architecture**
- All CLI code in application layer (`src/application/cli/`)
- Clear separation: commands, config, formatters, templates, error handling
- Port/adapter pattern for external integrations
- No direct dependencies on infrastructure layer

**Code Quality Metrics**
- Zero clippy warnings (strict mode: `-D warnings`)
- All code formatted with `cargo fmt`
- Comprehensive rustdoc for public APIs
- Consistent error handling patterns
- No debug prints or temporary code in production

**Performance**
- Release build: 2m 48s
- CLI test suite: 0.02s
- Full test suite: 7.82s
- Async spinners (non-blocking UI)
- Efficient table rendering

### Changed - Epic 17.5: CLI Directory Consolidation

#### CLI Module Consolidation
- **Unified CLI Structure**: Consolidated all CLI code into `src/application/cli/`
  - Removed legacy `src/cli/` directory (18 files)
  - All CLI functionality now follows hexagonal architecture in application layer
  - Commands: agent, arsenal, battalion, maneuver, user
  - Config: paladin_config, battalion_config, loader
  - Output: Unified `CliError` type with 25+ variants
  - Templates: paladin_template, battalion_template
  - Interactive: TTY utilities and prompts

- **Error Handling**: Single unified error type
  - Merged `src/cli/output/errors::CliError` and `src/application/cli::error::CliError`
  - All CLI commands now use `CliError` and `CliResult` from `application::cli::error`
  - Removed duplicate error conversion logic
  - Improved error messages with detailed formatting

- **Import Path Changes**: Updated all imports to new structure
  - **Old**: `use paladin::cli::*;` (deprecated and removed)
  - **New**: `use paladin::application::cli::*;`
  - Binary entry point (`paladin-cli.rs`) updated
  - All examples and tests updated

- **Code Quality Improvements**:
  - Fixed 27 clippy warnings (clone_on_copy, field_reassign_with_default, etc.)
  - All tests passing: 1411 unit tests
  - Zero clippy warnings with `-D warnings`
  - Code formatted with `cargo fmt`

#### Migration Guide for Developers

If you have code importing from the old CLI structure, update your imports:

```rust
// OLD (removed)
use paladin::cli::output::errors::CliError;
use paladin::cli::commands::agent;
use paladin::cli::config::loader::load_paladin_config;

// NEW (current)
use paladin::application::cli::error::{CliError, CliResult};
use paladin::application::cli::commands::agent;
use paladin::application::cli::config::loader::load_paladin_config;
```

The `src/cli/` directory has been completely removed. All CLI functionality is now properly organized in the application layer following hexagonal architecture principles.

### Added - Epic 17: Flow DSL & Agent Rearrangement (Maneuver Pattern)

#### Flow DSL Parser
- **FlowParser**: String-based workflow orchestration with intuitive syntax
  - Sequential operator `->` for linear workflows (e.g., "A -> B -> C")
  - Parallel operator `,` for concurrent execution (e.g., "A, B, C")
  - Nested patterns with parentheses for complex workflows
  - Complete lexer, AST, and parser implementation in core layer
  - 57 comprehensive tests covering all syntax patterns
- **Error Handling**: Detailed `FlowParseError` types with position tracking
  - Helpful error messages for common syntax mistakes
  - Support for debugging complex nested expressions
  - Suggestion methods for error recovery

#### Maneuver Domain Model
- **Maneuver**: New Battalion pattern for declarative workflow definition
  - Parse flow expressions into executable agent graphs
  - Support for 10-30 agent workflows with automatic dependency resolution
  - Three error strategies: FailFast, ContinueParallel, IgnoreErrors
  - Two output formats: CombinedText, StructuredJson
  - 21 domain tests validating configuration and behavior
- **ManeuverConfig**: Comprehensive configuration with timeouts and validation
  - Per-agent timeout controls
  - Error strategy selection
  - Output format specification
  - Validation rules for agent count and flow complexity

#### Execution Engine
- **ManeuverExecutionService**: Async execution with dependency resolution
  - Parallel execution of independent agents
  - Sequential execution for dependent agents
  - Result aggregation based on output format
  - Error handling with configurable strategies
  - 3 integration tests verifying execution patterns
- **Flow Visualization**: ASCII and Mermaid diagram generation
  - ASCII art for terminal display and documentation
  - Mermaid diagrams for rich visualizations
  - Support for simple, nested, and complex flows
  - 12 tests covering all visualization scenarios

#### Commander Integration
- **Pattern Detection**: Automatic Maneuver pattern recognition
  - Parse flow expressions from input strings
  - Detect sequential and parallel patterns automatically
  - Seamless integration with existing Formation and Phalanx patterns
  - 16 tests for Commander Maneuver integration
- **CLI Commands**: Complete CLI support for Maneuver operations
  - `paladin maneuver create` - Generate Maneuver configurations
  - `paladin maneuver execute` - Execute flow expressions
  - `paladin maneuver validate` - Validate flow syntax
  - `paladin maneuver visualize` - Generate visualizations
  - 4 CLI command tests

#### Documentation & Examples
- **Comprehensive Documentation**: Complete documentation suite
  - `docs/guides/flow-dsl.md` (800+ lines) - Complete Flow DSL user guide
    - Syntax reference with EBNF grammar
    - Error handling strategies (FailFast, ContinueParallel, IgnoreErrors)
    - Visualization guide (ASCII tree, Mermaid flowcharts)
    - 10+ practical examples and best practices
    - Troubleshooting section with common errors
    - Performance considerations and scalability limits
  - Updated `docs/BATTALION.md` with Maneuver pattern section (lines 500-560)
  - Updated `docs/CLI_USAGE.md` with Maneuver CLI commands
  - Updated main `README.md` - Changed from 5 to 8 orchestration patterns
    - Added Council, Grove, and Maneuver pattern descriptions
    - Added link to Flow DSL guide
- **Production Examples**: 3 complete working examples (958 lines)
  - `maneuver_basic.rs` - Introduction to Flow DSL
  - `maneuver_nested_flow.rs` - Enterprise review pipeline
  - `maneuver_dynamic_flow.rs` - Runtime flow generation
- **Performance Benchmarks**: 7 benchmark suites (32 test cases)
  - Parse time benchmarks (4 complexity levels)
  - Visualization performance (ASCII and Mermaid)
  - Validation overhead measurement
  - Sequential and parallel execution benchmarks
  - Nested flow performance testing
  - Overhead comparison vs Formation/Phalanx patterns

#### Test Coverage
- **113 Total Tests**: Comprehensive coverage across all components
  - Parser: 57 tests (lexer, AST, error handling)
  - Domain: 21 tests (Maneuver, ManeuverConfig)
  - Execution: 3 tests (ManeuverExecutionService)
  - Commander: 16 tests (pattern detection, integration)
  - Visualization: 12 tests (ASCII, Mermaid)
  - CLI: 4 tests (command validation)
- **Benchmark Coverage**: 32 performance test cases
  - Parse performance: < 1ms for complex flows
  - Execution overhead: < 2% vs direct patterns
  - Memory efficiency validation
  - Scalability testing (3-20 agents)

### Added - Epic 14: Autonomous Agent Features

#### Autonomous Planning Mode
- **Auto Loop Detection**: New `MaxLoops::Auto { max_subtasks: u32 }` variant enables intelligent loop optimization
  - Automatic task complexity analysis
  - Dynamic subtask decomposition for complex tasks
  - Optimal loop count determination (simple tasks use fewer loops)
- **Planning Service**: New `PlanningService` with comprehensive task planning
  - Task complexity assessment
  - Structured plan generation with subtasks
  - Subtask execution and synthesis
  - Integration with Paladin execution flow
- **Planning Configuration**: `PlanningConfig` with enabled flag, max_subtasks, and complexity threshold
- **Domain Types**: `TaskPlan`, `Subtask`, `ComplexityLevel` for structured planning representation

#### Auto-Generate System Prompts
- **Prompt Generation Service**: New `PromptGenerationService` for LLM-powered prompt creation
  - Generate system prompts from natural language agent descriptions
  - Optimize prompts for specific agent roles and capabilities
  - Cache generated prompts for reuse
  - Support for prompt regeneration and manual overrides
- **Prompt Configuration**: `PromptConfig` with enabled flag and optional cache control
- **Builder Integration**: `agent_description()` method on PaladinBuilder for seamless prompt generation

#### Dynamic Temperature Adjustment
- **Temperature Service**: New `TemperatureService` with task-based temperature optimization
  - Automatic task type classification (factual, creative, balanced)
  - Temperature bounds configuration (min/max range)
  - Classification heuristics based on task keywords
  - Real-time temperature adjustment per task
- **Temperature Configuration**: `TemperatureConfig` with enabled flag, min/max bounds, and custom keywords
- **Task Types**: `TaskType` enum (Factual, Creative, Balanced) with appropriate temperature ranges

#### Intelligent Agent Handoffs
- **Handoff Service**: New `HandoffService` for delegation between specialist agents
  - Specialist discovery and routing
  - Task complexity assessment for delegation
  - Circuit breaker integration for reliability
  - Handoff depth limiting (prevent infinite delegation)
- **Handoff Configuration**: `HandoffConfig` with enabled flag, strategy, and max delegation depth
- **Handoff Strategies**: `HandoffStrategy` enum (Automatic, ExplicitOnly) for control
- **Domain Types**: `HandoffDecision`, `HandoffMetadata` for structured delegation tracking

#### Handoff Tool Integration
- **Arsenal Integration**: New `HandoffTool` registered in Arsenal for LLM-accessible delegation
  - `delegate_to_specialist` function for explicit handoffs
  - JSON schema for LLM tool use
  - Specialist validation and routing
  - Seamless integration with agent execution loop

#### Configuration & Builder API
- **Autonomous Configuration**: New `AutonomousConfig` aggregating all autonomous features
  - Centralized configuration structure
  - YAML configuration support
  - CLI flag integration
  - Builder pattern support via `PaladinBuilder`
- **Builder Methods**: New autonomous feature methods on PaladinBuilder
  - `enable_planning(bool)` - Toggle autonomous planning
  - `agent_description(String)` - Set description for prompt generation
  - `enable_dynamic_temperature(bool)` - Toggle temperature adjustment
  - `enable_handoffs(bool)` - Toggle delegation capabilities

#### Documentation & Examples
- **Comprehensive Guide**: New `docs/AUTONOMOUS.md` (400+ lines)
  - Introduction and features overview
  - Detailed user story documentation (all 5 features)
  - Configuration guide (YAML, CLI, Builder)
  - Best practices and performance considerations
  - Error handling and troubleshooting
  - Advanced usage patterns
  - Complete API reference
- **Working Examples**: 5 comprehensive example files (~1,400 lines)
  - `autonomous_planning.rs` - Planning mode with task decomposition
  - `autonomous_prompt_generation.rs` - Auto-prompt generation concepts
  - `dynamic_temperature.rs` - Temperature adjustment by task type
  - `agent_handoffs.rs` - Specialist delegation workflow
  - `autonomous_full_config.rs` - All features combined
- **Examples README**: Updated `examples/README.md` with autonomous section

#### Testing & Quality
- **Comprehensive Testing**: 1,280+ tests passing including autonomous features
  - Unit tests for all services and domain logic
  - Integration tests for Paladin with autonomous features
  - MockLlmAdapter integration for deterministic testing
- **Code Quality**: Zero clippy warnings in strict mode
  - All code formatted with rustfmt
  - Comprehensive rustdoc for all public APIs
  - Error handling with thiserror patterns

#### Security Audit Results
- **Vulnerabilities**: 2 transitive dependency vulnerabilities identified (non-critical)
  - `rsa 0.9.10`: Marvin Attack timing sidechannel (RUSTSEC-2023-0071) - Medium severity, no upgrade available (from sqlx-mysql)
  - `tokio-tar 0.3.1`: PAX header parsing issue (RUSTSEC-2025-0111) - No upgrade available (from testcontainers, dev dependency only)
- **Unmaintained Crates**: 9 warnings about unmaintained transitive dependencies
  - All are indirect dependencies from test/dev dependencies
  - No immediate security risk to production code
  - Monitored for future upgrades when upstream updates available

### Added - Epic 13: Sentinel Vision System

#### Vision API & Multi-Modal Processing
- **Vision Content Types**: Support for three image input formats
  - `ImageUrl`: Process images from public web URLs
  - `ImageFile`: Load and analyze local image files with automatic base64 encoding
  - `ImageBase64`: Direct base64-encoded image input
- **Vision-Enabled Paladins**: New `enable_vision()` builder method and `execute_with_vision()` function
- **Image Detail Levels**: Control token usage and analysis depth
  - `Low`: ~85 tokens, fast processing for simple tasks
  - `High`: 170+ tokens, detailed analysis with fine-grained details
  - `Auto`: Automatic balancing based on image complexity
- **Multi-Provider Support**: Vision capabilities across LLM providers
  - OpenAI: GPT-4o, GPT-4o-mini with vision support
  - Anthropic: Claude 3 Opus, Sonnet, Haiku with vision capabilities

#### Document Processing System
- **PDF Extraction**: Comprehensive PDF text extraction via `PdfExtractor`
  - Multi-page document support
  - Metadata extraction (title, author, creation date, page count)
  - Character-accurate text extraction
  - Page-by-page content access
- **Intelligent Document Chunking**: Flexible chunking strategies via `ChunkConfig`
  - Configurable chunk sizes (characters per chunk)
  - Overlap control for context preservation
  - Custom separators (paragraphs, sentences, custom delimiters)
  - Three built-in configurations:
    - RAG-optimized: 500 chars, 100 overlap, paragraph-based
    - Summarization: 2000 chars, 200 overlap, paragraph-based
    - Sentence-based: 300 chars, 50 overlap, sentence-based
- **DocumentPort Interface**: Clean abstraction for document operations
  - Extract metadata and content from PDFs
  - Chunk documents with configurable strategies
  - Extensible to other document formats

#### Security & Data Protection
- **Vision Data Encryption**: ChaCha20-Poly1305 authenticated encryption
  - Secure at-rest encryption for image data
  - Automatic encryption for `ImageFile` and `ImageBase64` types
  - Decryption utilities for secure data access
- **Data Retention Policies**: Configurable retention for sensitive vision data
  - Time-based retention (e.g., 30 days)
  - Automatic cleanup of expired encrypted data
  - Audit logging for compliance
- **Audit Logging**: Comprehensive tracking of vision operations
  - Document processing events (PDF extraction, chunking)
  - Vision API calls (provider, model, image count, tokens)
  - Encryption/decryption operations
  - Security-related events (data retention, cleanup)

#### CLI Integration
- **Vision Analysis Commands**:
  ```bash
  paladin vision analyze --image path/to/image.jpg --prompt "Describe this image"
  paladin vision analyze --url https://example.com/image.jpg --detail high
  paladin vision batch --directory images/ --prompt "Classify image"
  ```
- **Document Processing Commands**:
  ```bash
  paladin document extract --pdf document.pdf --output text
  paladin document chunk --pdf report.pdf --chunk-size 500 --overlap 100
  paladin document analyze --pdf paper.pdf --prompt "Summarize key findings"
  ```
- **Security Commands**:
  ```bash
  paladin vision encrypt --image sensitive.jpg --output encrypted.bin
  paladin vision decrypt --input encrypted.bin --output decrypted.jpg
  paladin security audit --filter vision --since "30 days ago"
  ```

#### YAML Configuration Support
- **Vision Configuration Section**:
  ```yaml
  vision:
    default_detail: "auto"
    max_images_per_request: 10
    supported_formats: ["png", "jpg", "jpeg", "gif", "webp"]
    enable_encryption: true
  ```
- **Document Processing Configuration**:
  ```yaml
  document:
    pdf:
      max_pages: 1000
      chunk_size: 500
      chunk_overlap: 100
      separator: "\n\n"
  ```
- **Security Configuration**:
  ```yaml
  security:
    vision:
      encryption_enabled: true
      data_retention_days: 30
      audit_logging: true
  ```

#### Battalion Integration
- **Formation Pattern**: Sequential vision pipelines
  - Example: Image Analyzer → Detail Extractor → Insight Generator
  - Output of each stage feeds into the next
  - Perfect for multi-stage vision analysis workflows
- **Phalanx Pattern**: Parallel image processing
  - Process multiple images concurrently with ~3x speedup
  - Each image analyzed by a separate vision-enabled Paladin
  - Results aggregated at completion
- **Campaign Pattern**: Graph-based vision workflows
  - Complex vision processing DAGs
  - Conditional branching based on vision analysis results
  - Mix vision and non-vision tasks in same graph
- **Chain of Command Pattern**: Hierarchical vision delegation
  - Commander Paladin delegates vision tasks to specialist Paladins
  - Automatic load balancing across vision-capable Paladins
  - Escalation for complex or ambiguous visual content

#### Documentation
- **Comprehensive Guide**: `docs/SENTINEL.md` (600+ lines)
  - 13 major sections covering entire vision system
  - Getting started tutorials
  - Supported providers and models
  - Paladin Vision API reference
  - Document processing workflows
  - CLI usage with 8+ command examples
  - YAML configuration templates
  - Security best practices
  - Battalion integration patterns
  - Error handling strategies
  - Performance optimization tips
  - Troubleshooting guide (7 common issues)
- **Code Examples**: Three comprehensive working examples
  - `examples/vision_analysis.rs`: Single-image analysis with detail levels (200 lines)
  - `examples/document_processing.rs`: PDF extraction and chunking strategies (280 lines)
  - `examples/vision_battalion.rs`: Formation and Phalanx patterns (320 lines)
- **README Updates**: Vision & Multi-Modal Processing section
  - Key features overview
  - Quick start code samples
  - Supported content types
  - Document processing examples
  - CLI command references
  - Battalion integration notes
  - Links to comprehensive documentation

### Technical Details

#### Architecture
- **Hexagonal Architecture Compliance**: All vision components follow ports/adapters pattern
  - Vision domain entities in `core/platform/container/`
  - Vision port definitions in `application/ports/output/`
  - Provider-specific adapters in `infrastructure/adapters/llm/`
- **Test-Driven Development**: Comprehensive test coverage
  - 1146 library tests passing (including vision tests)
  - Unit tests for all vision content types
  - Integration tests with mocked API responses
  - Error path testing for invalid formats
  - Security tests for encryption/decryption

#### Dependencies
- **New Dependencies**:
  - `pdf-extract`: PDF text extraction
  - `lopdf`: Low-level PDF manipulation
  - Additional cryptographic dependencies for vision encryption

#### Performance
- **Benchmarks Available**: Vision-specific performance tests
  - Image encoding/decoding: ~50ms per 2MB image
  - Batch processing: ~3x speedup with Phalanx pattern
  - PDF extraction: ~200ms per 100-page document
  - Document chunking: ~10ms for 10k character document

### Security

#### Known Vulnerabilities (from `cargo audit`)
- **RUSTSEC-2023-0071**: RSA timing sidechannel in `rsa 0.9.10` (Medium severity)
  - **Impact**: Potential key recovery through timing attacks
  - **Source**: Transitive dependency via `sqlx-mysql`
  - **Status**: No fixed upgrade available
  - **Mitigation**: Affects MySQL TLS certificate validation (optional feature)
  - **Risk Assessment**: Low for Paladin use case (MySQL connections are internal)

- **RUSTSEC-2025-0111**: tokio-tar PAX header parsing vulnerability
  - **Impact**: File smuggling attacks via malformed TAR archives
  - **Source**: Dev dependency via `testcontainers`
  - **Status**: No fixed upgrade available
  - **Mitigation**: Only used in test environment, not production code
  - **Risk Assessment**: Low (development-only dependency)

#### Unmaintained Dependencies (Warnings)
- `ansi_term 0.12.1` (via structopt): Consider migrating to `clap 4.x`
- `atty 0.2.14` (via structopt): Replaced by `is-terminal` in modern Rust
- `dotenv 0.15.0`: Consider migrating to `dotenvy`
- `fxhash 0.2.1` (via scraper): Low risk, internal to scraper crate
- `gcc 0.3.55` (via fasthash-sys): Build-time only dependency
- `number_prefix 0.4.0` (via indicatif): No security impact
- `proc-macro-error 1.0.4` (via structopt): Compile-time only
- `rustls-pemfile 2.2.0` (via testcontainers): Dev dependency only

**Action Plan**: Monitor for updates to `sqlx` and consider migrating from `structopt` to `clap 4.x` in future release.

### Testing

#### Test Coverage
- **Total Tests**: 1146 passing (0 failed)
- **Test Execution**: 7.33s for full library test suite
- **Coverage**: ≥80% for vision and document modules
  - Vision content types: 100% coverage
  - Document extraction: 95% coverage
  - Security (encryption): 90% coverage

#### Test Categories
- **Unit Tests**: 1000+ tests for core functionality
- **Integration Tests**: Mocked API responses for vision providers
- **Security Tests**: Encryption, decryption, audit logging
- **Error Path Tests**: Invalid formats, corrupted data, missing files

### Code Quality

#### Static Analysis
- **Clippy**: PASSED with `-D warnings` (library code)
- **Formatting**: PASSED `cargo fmt --check`
- **Compilation**: CLEAN with `cargo check --all-features`

#### Documentation Quality
- All public APIs have rustdoc comments
- Three comprehensive code examples (800+ lines total)
- User guide with 13 major sections
- Troubleshooting guide with 7 common issues

### Breaking Changes
None. All changes are additive and backward compatible.

### Migration Guide
No migration required. Existing Paladin code works without modification.

To use new vision features:
```rust
// Enable vision on a Paladin
let paladin = PaladinBuilder::new(llm_port)
    .system_prompt("You are a vision-enabled AI assistant")
    .enable_vision(true)
    .build()?;

// Process images
let content = vec![VisionContent::ImageUrl {
    url: "https://example.com/image.jpg".to_string(),
    detail: ImageDetail::Auto,
}];

let result = service.execute_with_vision(&paladin, "Describe this image", content).await?;
```

### Contributors
- John Amatulli (jamatulli) - Epic 13 implementation and documentation

---

## [0.1.0] - Previous Releases

### Added
- Core Paladin platform with Hexagonal Architecture
- Multi-provider LLM support (OpenAI, DeepSeek, Anthropic)
- Battalion orchestration patterns (Formation, Phalanx, Campaign, Chain of Command)
- Arsenal MCP integration for external tools
- Garrison memory and context system
- Citadel state persistence
- Herald output formatting
- Comprehensive CLI (paladin-cli)
- User management system with authentication
- Content processing pipeline
- Redis queue integration
- MinIO file storage integration
- MySQL and SQLite repository support
- Security features (TLS verification, audit logging)
- Docker development environment

### Technical Foundation
- Test-Driven Development (TDD) methodology
- Domain-Driven Design (DDD) principles
- Three-layer hexagonal architecture
- Comprehensive test suite (1146+ tests)
- Continuous integration ready

[Unreleased]: https://github.com/jamatulli/paladin/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jamatulli/paladin/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamatulli/paladin/releases/tag/v0.1.0
