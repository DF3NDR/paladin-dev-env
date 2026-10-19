# Requirements: Paladin — Milestone v0.11.0 "Treasurer Spend Governance"

**Defined:** 2026-09-24
**Core Value:** A Rust developer can compose and run multi-agent workflows against any supported
LLM provider through stable port abstractions — without their own domain code depending on a
provider, transport, or storage implementation.

**Source of truth:** `.project/Milestone_14-Treasurer/` (overview + Epic 1 PRD R1-R6) for the
Treasurer; PROJECT.md *Current Milestone* for the supporting scope; `.planning/research/SUMMARY.md`
for integration points and pitfalls. Operator decisions recorded 2026-09-24: full governance
(per-tenant + per-API-key); prices from operator config only, nothing bundled; rolling-period
allowances + optional lifetime cap; refuse at admission AND halt runs in flight on every run path;
soft warn threshold; tenant mapped from API key; ledger via a new port with in-memory/SQLite/Postgres;
pacing in-process + Redis-shared + stampede lock, degrading to local pacing when Redis is down;
clean-break removal of the legacy Battalion error/retry/timeout surfaces and
`PaladinError::LlmError(String)`.

## v1 Requirements

Requirements for this milestone. Each maps to exactly one roadmap phase.

### Pricing (PRICE)

- [x] **PRICE-01**: Operator can configure a per-model price table (prompt, completion, cache-read,
  cache-write and reasoning unit prices, written as decimal strings); the default table is empty,
  and negative or malformed prices are rejected at config validation

- [x] **PRICE-02**: A pure function maps a `TokenUsage` and the price table to an exact cost in
  fixed-point integer micro-units (no `f64` accumulation), unit-tested per token type including
  cache and reasoning tokens
  *(Amended 2026-09-25, Phase 38 plan 38-01: the cost unit is i64 nano-units (1e-9), a finer scale
  that satisfies "micro-units" — see 38-CONTEXT.md D-02 and ADR-0053.)*

- [x] **PRICE-03**: A completed run reports its currency cost in `ExecutionMetadata.cost_estimate`
  end-to-end; a model with no configured price yields `None` (never `0`), and the field's reserved
  rustdoc note is updated to "produced by the Treasurer"

### Spend ledger (LEDGR)

- [x] **LEDGR-01**: A `TreasuryLedgerPort` with in-memory, SQLite and Postgres adapters passes one
  shared contract-test suite, backed by a `007` migration in both
  `crates/paladin-storage/migrations/{sqlite,postgres}/`

- [x] **LEDGR-02**: Draws reserve then settle atomically, so concurrent draws never overspend —
  when N draws race and only N−1 fit, exactly N−1 succeed, on every adapter

- [x] **LEDGR-03**: Settlement is idempotent, keyed on run, superstep and attempt, so lease
  redelivery, resume, retries and model fallback never charge twice

- [x] **LEDGR-04**: Operator can view spend per tenant, API key, run and model over a time window
  from the CLI, and spend appears in herald output and trace events

### Tenant identity (TENANT)

- [x] **TENANT-01**: Operator config maps each API key to a tenant and the authenticated
  `Principal` carries `tenant_id`; a caller cannot assert its own tenant. The breaking change is
  recorded in `MIGRATION.md` §9.2 and the `cargo semver-checks` allowlist

- [x] **TENANT-02**: Every run records its submitting principal (API key id and tenant), so spend
  and run reads are attributed to the caller

### Allowances (ALLOW)

- [x] **ALLOW-01**: Operator can set an `allowance` per tenant and per API key with a rolling period
  and an optional lifetime cap, distinct from every existing `max_tokens` meaning; window boundaries
  are computed in UTC from the store or server clock, never a worker's local clock

- [x] **ALLOW-02**: Submitting a run while the caller's tenant or API-key allowance is exhausted is
  refused at admission with a typed error, and no run is persisted

- [x] **ALLOW-03**: A run in flight whose next draw would overspend halts cleanly — typed Treasurer
  error, status `Halted`, last checkpoint kept — and can be resumed once the allowance is replenished
  or the period resets; this holds on both engine-driven (`WarEngine`) and agent-loop
  (`PaladinExecutionService`) runs, with the enforcement attachment point recorded in an ADR

- [x] **ALLOW-04**: A configurable warn threshold (for example 80%) emits one trace event plus a
  herald and webhook notice per window, without blocking the run

- [x] **ALLOW-05**: The Treasurer derives the per-run `TokenBudget` from the remaining allowance and
  works alongside `TokenBudget`, `ModelCallLimit`, `ToolCallLimit` and the Commissary without
  replacing any of them; a guard keeps `Treasurer` a framework-only word

### Rate pacing (PACE)

- [x] **PACE-01**: `LlmError::RateLimitExceeded` carries a retry delay parsed from `Retry-After`
  (delta-seconds or HTTP-date) and from provider rate-limit headers, with the header names verified
  against official OpenAI and Anthropic documentation

- [x] **PACE-02**: An `LlmPort` pacing decorator paces each provider and model in-process and backs
  off with jitter on a 429, treating the retry delay as a minimum; it wraps every
  `FallbackLlmAdapter` hop, and a mocked-429 test proves back-off rather than thrash

- [x] **PACE-03**: With Redis configured, pacing state is shared across worker instances through an
  atomic Lua script using the Redis server clock, so a 429 slows the whole fleet

- [x] **PACE-04**: A distributed cache-stampede lock (set-if-absent with expiry, fencing token,
  delete-only-if-owner) stops concurrent workers from issuing the same cached request twice

- [x] **PACE-05**: When Redis is unavailable, pacing degrades to conservative per-process pacing
  with a trace warning and never runs unpaced; this is covered by a test

### Legacy clean break (LEGACY)

- [x] **LEGACY-01**: Legacy `battalion::RetryPolicy`, `battalion::ErrorStrategy` and
  `battalion::NodeError` are removed, while the same-named Aegis and Flow types stay untouched

- [x] **LEGACY-02**: Legacy Formation, Phalanx and Campaign timeout handling is removed in favour of
  Aegis timeouts

- [x] **LEGACY-03**: `PaladinError::LlmError(String)` is removed, and the message-matching retry
  check in `conclave_execution_service.rs` moves to the typed `LlmFailure` / `Transience` taxonomy

- [x] **LEGACY-04**: The X-03 supersession is recorded in an ADR, and every removal has a
  `MIGRATION.md` §9.2 row and a `cargo semver-checks` allowlist entry, with examples and docs updated

### Base entity rename (ENTITY)

*Minted 2026-10-12 from the Phase 48 code survey (`48-CONTEXT.md`). The base-layer wrapper
`paladin_core::base::entity::node::Node<T>` predates the v0.10.0 superstep engine, whose graph
vocabulary (`NodeId`, `NodeSpec`, `StateNode`, …) now outnumbers it ~5:1 and lives in public wire
formats; the two senses of "node" already collide inside the engine. The entity side is renamed,
the graph side is kept.*

*Corrected 2026-10-14 (plan 48-09): ENTITY-01's storage list, ENTITY-04's type name and outcome, and ENTITY-05's baseline file were wrong or open in the survey; the shipped tree and 48-CONTEXT.md D-05/D-15 settle them. No requirement ID changed.*

- [x] **ENTITY-01**: `paladin_core::base::entity::Entity<T>` replaces `Node<T>` with the payload
  field renamed `node` → `data`; `Paladin`, `Council`, `Grove`, `User`, `Log`, `ContentItem` and
  `PromptItem` are built on it; `#[serde(alias = "node")]` keeps every pre-rename persisted JSON
  (Citadel snapshot files, stored Paladins, and any caller-owned store of entity JSON -- no
  in-tree SQLite/MySQL table stores entity JSON; those repositories store columns and rebuild the
  entity) deserializing, proven by a fixture test

- [x] **ENTITY-02**: The old name's fate is decided before planning and recorded: by default
  `Node<T>` and the `base::entity::node` module are removed outright under a Phase-48-only X-03
  supersession recorded in an ADR (the ADR-0059 shape; `MIGRATION.md` §9.7 stays empty), or — if
  the owner chooses X-03 as written — a `#[deprecated(since = "0.11.0")]` `Node<T>` alias and
  re-exporting module keep compiling with no in-tree use and §9.7 schedules their removal for
  v0.12.0

- [x] **ENTITY-03**: Phase 44 left exactly one `NodeError` (the structured
  `node_error::NodeError`); after Phase 48 that still holds and `tests/legacy_removal_guard.rs`
  is still green, there is exactly one `Paladin` alias (the `citadel.rs:48` duplicate is
  removed), and `NodeVersion`, `NodeVersionRepository`, `NodeVersionService` and
  `InMemoryNodeVersionRepository` become `EntityVersion*`

- [x] **ENTITY-04**: `Field`, `Collection`, `Element`, `FieldVersionService` and
  `CollectionVersionService` (module `collection_versioning_service`) (zero consumers outside
  `base/`) are each removed or kept with a written reason, and `Entity<T>::version`'s contract is
  documented in rustdoc, including that only `ContentItem` currently honours it through the version
  service -- Phase 48 outcome: all five retained pending review, the reason written in `base/mod.rs`
  rustdoc (D-05, ADR-0060)

- [x] **ENTITY-05**: The graph vocabulary (`NodeId`, `NodeSpec`, `StateNode`, `NodeContext`,
  `NodeCache*`, `TraceEvent::Node*`, `wargraph-doc.schema.json`, the `node_cache` config section) is
  untouched; an ADR reserves "node" for the graph sense and "Entity" for the base wrapper; the
  CLAUDE.md/copilot naming table and the mdBook domain-model, design-patterns and crate-map pages
  agree; a repository-scan test in the `tests/legacy_removal_guard.rs` shape (positive controls
  included) pins the freeze and the absence of the old entity paths; every rename has a
  `MIGRATION.md` §9.2 row with a set-equal
  `.cargo/semver-checks-allowlist.toml` entry, the gated public-API baseline
  `.project/current-exports.txt` is refreshed via `make api-surface-update` with a CHANGELOG entry,
  and `cargo doc`, `make check-api-examples`, `make clean-code` and the 82 %
  coverage floor stay green

### Downstream adoption (ADOPT)

*Added 2026-10-14 (Phase 49) from the web3sec handoff of 2026-10-08 (`49-HANDOFF-web3sec.md`).
Paladin's only consumer is 631 commits behind `v0.10.1` and has frozen its own work until it adopts
the release; the trial adoption found no Paladin defect. ADOPT-01..03 are consumer-side (executed
in web3sec, closed here by the handoff's §9 return report); ADOPT-04..05 are Paladin-side.*

*Scope cut 2026-10-14 (owner decision): ADOPT-01..03 are superseded. The consumer does not move to `v0.10.1`; it moves once, straight to `v0.11.0`, after Phases 50 and 51, with `MIGRATION.md` §9.8 "Library consumers: v0.10.1 → v0.11.0" as the spec (todo `2026-10-14-web3sec-single-bump-to-v0-11-0.md`). ADOPT-04 and ADOPT-05 are complete.*

- [ ] **ADOPT-01** *(superseded 2026-10-14, see note above)*: The consumer pins `paladin` at exactly `v0.10.1` (`f7dae267`), detached, in
  one commit that also carries the adaptation and `Cargo.lock`; the local-only submodule commit
  `4b7bed87` is recorded as superseded (its seven planning files are byte-identical in `v0.10.1`)

- [ ] **ADOPT-02** *(superseded 2026-10-14, see note above)*: Consumer call sites are adapted for B-1 (`Commissary::new`/`from_port` without
  `is_exact_counter`), B-2 (`PaladinResult.usage: TokenUsage`) and B-3 (`TokenUsage`'s three
  `Option<u32>` fields), with the D-2 token-usage mapping decided and documented — lossless
  `TokenUsage` carried through `RoutineReport` preferred; the `total_tokens`-only fallback only
  with a site comment and a tech-debt entry

- [ ] **ADOPT-03** *(superseded 2026-10-14, see note above)*: The consumer workspace is green on the new pin: `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo check --workspace` and
  `cargo test --workspace` with ≥ 2,592 passed / 0 failed / 2 ignored, no `unwrap`/`expect`/
  `panic!` added to library code, and no stale `is_exact_counter` / `token_count: report` call
  site

- [x] **ADOPT-04**: Paladin carries a compile-only consumer-surface muster — a test that imports
  every path and takes every signature the consumer uses (handoff Appendix A, re-verified against
  the consumer's actual `use` lines and the current tree) — green on `cargo test`, wired into the
  existing gates, and proven red against a deliberately broken signature (e.g. a `bool` re-added
  to `Commissary::new`)

- [x] **ADOPT-05**: `MIGRATION.md` gains a consumer-facing upgrade note — the "no total-only
  `TokenUsage` constructor" pitfall and the `v0.10.1` → `v0.11.0` break list the muster exposes
  (`battalion::ErrorStrategy`, the legacy `battalion::NodeError`, `Node<T>` → `Entity<T>`,
  `PaladinResult.cost`) — while no Paladin library API is changed, shimmed or deprecated for the
  consumer: zero new §9.2 rows and zero `semver-checks` allowlist changes from this phase

### Provider parity (PARITY)

*Added 2026-10-14 (Phase 50) from the second web3sec handoff of 2026-10-13
(`50-HANDOFF-web3sec-parity.md`). The consumer's config-only swap of its Anthropic agents onto
Moonshot `kimi-k3` fails or silently degrades in five blocking and eleven degrading ways; most are
cross-cutting, and several are defects in the three original adapters. `PROV-*` is spent (Phase
17), so a new prefix. Decisions D-1..D-4 of the handoff are settled at discuss.*

- [x] **PARITY-01**: A provider-swap matrix test with mock transports passes for all nine
  providers — the same agent definition switched only through config builds and runs with
  equivalent behaviour, or fails at build time with an error naming the provider, model and
  parameter — covering manual and auto temperature on a `(1,1)` provider, the user prompt on the
  wire, the per-step model in a fallback chain, and an empty `stop` completion

- [x] **PARITY-02**: Parameter support is modelled per provider and per model (`Range` / `Fixed`
  / `NotSent` for temperature, top_p, penalties and stop sequences), additive to
  `ProviderCapabilities` unless an explicit X-03 supersession is recorded, and one documented
  policy for a configured value the target does not accept — an ADR amending ADR-0004 — governs
  every adapter, the originals included, with nothing substituted undiscoverably

- [x] **PARITY-03**: Provider construction honours `llm.<provider>` config blocks (model,
  base_url, timeout, max_retries, default max_tokens, context-window override) through a public
  library API routed through `config/bridge.rs`; per-agent `provider` + `model` resolve through
  the same path in `agent_host`; existing YAML loads unchanged; and the facade exports all nine
  adapter/config pairs behind ADR-0046's features, with the public-API baseline refreshed and a
  CHANGELOG entry

- [x] **PARITY-04**: Per-model limits (context window, max output, reasoning support, fixed
  sampling values, JSON-mode support) come from a dated registry with config overrides that
  Commissary and `resolve_context_window` read, so `kimi-k3` is rationed against its real
  1,048,576-token window and the stale Anthropic/DeepSeek defaults and model lists are replaced

- [x] **PARITY-05**: Every adapter maps empty output (`EmptyCompletion` on any stop reason,
  OpenAI included), refusal (a typed `LlmError::Refusal`, classified `Permanent`), usage cap
  (live-verified per compat vendor), cache read/write and reasoning tokens the same way,
  surfaces reasoning text on the normal and streaming paths, and sends stop sequences where the
  vendor accepts them

- [x] **PARITY-06**: Fallback chains send each hop its own model, key Cadence pacing on it, and
  record the served model beside `SERVED_BY_METADATA_KEY`

- [x] **PARITY-07**: All nine adapters are in the shared conformance suite (extended with
  capability truthfulness, `response_format` wire shape, empty completion, refusal, usage fields
  and the parameter-drop policy) and the credential-gated live smoke, and the mdBook has a
  "Switching providers" page with the per-provider/per-model behaviour table plus
  MIGRATION/CHANGELOG entries for every intentional break

- [x] **PARITY-08**: The OpenAI adapter sends the prompt text (`query`, with `context` appended
  when present) on the execution-service path, with a mock-transport regression test that is red
  on the current tree and green after the fix

### Reasoning effort (REASON)

*Added 2026-10-14 (Phase 51) from the third web3sec handoff of 2026-10-13
(`51-HANDOFF-web3sec-reasoning-effort.md`). Paladin carries no effort, thinking or budget parameter
on the request path of any of the nine adapters, so the vendor default always applies — `max` on
Moonshot `kimi-k3`, measured live to burn a 16,000-token output budget on reasoning and return no
answer. Several vendors accept a wrong or unsupported effort field with HTTP 200 and ignore or
coerce it, so the design is built around refusing before the HTTP call. Depends on Phase 50's
parameter-support policy (PARITY-02), config-driven construction (PARITY-03), model-profile registry
(PARITY-04), reasoning surfaces (PARITY-05) and per-hop fallback models (PARITY-06). The handoff
read `v0.10.1`; the feature ships in v0.11.0. A new prefix, the fortieth. Decisions D-1..D-5 of the
handoff are settled at discuss.*

- [ ] **REASON-01**: A `#[non_exhaustive]` `ReasoningEffort` enum (`Off`, `Auto`, `Minimal`,
  `Low`, `Medium`, `High`, `XHigh`, `Max`, `Budget(u32)`) is settable per request
  (`LlmRequest.reasoning` + `with_reasoning`, the `response_format` precedent), per agent (a
  `#[serde(default)]` `PaladinData` field, a `PaladinBuilder` setter and a YAML `AgentDefinition`
  field) and per provider (`llm.<provider>.reasoning_effort` through Phase 50's config-driven
  construction), resolved request → agent → provider config → vendor default by
  `PaladinExecutionService` on every request it builds — and an unset value produces a
  byte-identical request body on every adapter

- [ ] **REASON-02**: Every adapter and preset declares its reasoning wire form — a non-`bool`
  `ReasoningWire` field on `CompatRequestParameters` (no `Default`, so each preset author must
  state a position; generic OpenAI-compatible defaults to `Unsupported`) and a private
  model-aware `map_reasoning` in each native adapter — and maps each level exactly or returns a
  new additive `LlmError::UnsupportedParameter { param, value, provider, model }`, classified
  `Permanent`, before any HTTP call; a level the vendor lacks, a level gated off for that model,
  `Off` on an always-thinking model, `Budget` where only levels exist and a level the vendor is
  known to coerce silently all refuse, and nearest-level substitution is an explicit opt-in, off
  by default, that logs `warn!` and records both requested and effective effort

- [ ] **REASON-03**: No preset emits a field its vendor ignores — a silent-no-op regression test
  asserts the Kimi preset never emits top-level `reasoning_effort`, an offline mockito wire-body
  case per adapter/preset pins the exact body sent for each level, and the credential-gated live
  smoke proves per vendor, where a key exists, that an invalid effort is rejected (the field is
  parsed) and that a higher level reports more `reasoning_tokens`

- [ ] **REASON-04**: Supported levels and budget bounds are per model, read from Phase 50's
  dated model-profile registry (PARITY-04) with config overrides rather than from adapter-wide
  constants, and each hop of a fallback chain resolves effort against its own model
  (PARITY-06)

- [ ] **REASON-05**: Requested and effective effort are recorded in `LlmResponse.metadata` beside
  `SERVED_BY_METADATA_KEY` and emitted on the Phase 28 trace stream; every adapter fills
  `TokenUsage.reasoning_tokens` when the vendor reports it; and reasoning text reaches callers
  through the typed shape settled at discuss (D-4) on the normal and streaming paths, building
  on PARITY-05's surface rather than duplicating it

- [ ] **REASON-06**: `Budget(n)` is validated against the vendor's bounds (Anthropic
  `budget_tokens` ≥ 1024 and < `max_tokens`; Qwen `thinking_budget` 1–32768; Gemini 2.5
  `thinkingBudget` ranges, re-verified live) and the request's `max_tokens` before send;
  `EmptyCompletion` stays the runtime backstop; and the effort/output-budget/timeout guidance is
  documented so an agent on `kimi-k3` at the documented defaults answers instead of exhausting
  its budget

- [ ] **REASON-07**: An ADR amending ADR-0004 records the fail-loud policy and its opt-in
  fallback; the mdBook has a "Reasoning effort" page with the per-provider/per-model mapping
  table; MIGRATION §9.2 and CHANGELOG carry the additive type, field, variant and config key
  with the public-API baseline refreshed; and the 82 % coverage floor (ADR-0006) and the doc
  tests are met

### Consumer-surface readiness (SURFACE)

*Added 2026-10-19 (Phase 50.1, inserted after Phase 50 and before Phase 51) from the fourth web3sec
handoff of 2026-10-18 (`50.1-HANDOFF-web3sec-consumer-surface.md`). web3sec pre-measured the single
v0.11.0 bump against the Phase 50 tip `21426037`: 112 compile errors in 3 of 12 crates, every one on a
registered row, fixed mechanically, suite back at 2,592 / 0 / 2. The exercise found six gaps in the
consumer surface and the consumer guide, each cheapest before the tag. SURFACE-02 resolves the
harmonisation todo of 2026-10-16 by owner decision (2026-10-18) and must land before Phase 51 extends
the same nine types. Depends on PARITY-03/04 and ADOPT-04. A new prefix, the forty-first. Decisions
D-A..D-D of the handoff plus D-E (the seed-rule conflict found at filing) are settled at discuss.*

- [ ] **SURFACE-01**: The Aegis family — `Aegis`, `ErrorHandlerSpec`, `TimeoutPolicy`, `StateDelta`,
  `NodeError`, `NodeErrorSource`, `TimeoutKind`, `NodeId`, `Transience` — is nameable through the
  `paladin` facade (whole `aegis` / `node_error` modules or selected items, D-A), so the MIGRATION
  §9.8 DR-1, DR-2, DR-8 and DR-9 recipes compile with `paladin::` paths alone and no direct
  `paladin-ai-core` dependency; the muster pins a `with_aegis(Aegis { on_error, timeout, .. })`
  construction and a `node_errors[i].{node_id, attempt, transience, source}` read through the
  facade; the four rows name the import path; additive, with `make api-surface-update` and a
  CHANGELOG entry

- [ ] **SURFACE-02**: The nine `*Config` types (`OpenAIConfig`, `AnthropicConfig`, `DeepSeekConfig`,
  `GeminiConfig`, `KimiConfig`, `QwenConfig`, `GrokConfig`, `OllamaConfig`,
  `OpenAiCompatibleConfig`) construct with one `new(..)` shape (D-B; endpoint-only providers keep
  their endpoint argument), every knob including `timeout_seconds` (and `base_url` where `new`
  drops it) is settable through a `with_*` builder generated by the shared `config_knob_builders!`
  macro, `from_env()` is unchanged, and the break is registered as a clean break (no deprecated
  alias, ADR-0060 pattern) in `.cargo/semver-checks-allowlist.toml`, MIGRATION §9.2 rows and a
  §9.8 DR row (DR-12's "not part of v0.11.0" sentence amended), with `make api-surface-update`
  refreshed, every in-tree call site (examples, `config::bridge`, the provider factory, tests)
  migrated in the same commits and the `paladin-llm` doctests showing the new shape

- [ ] **SURFACE-03**: `tests/consumer_surface_muster.rs` pins every item in handoff Appendix B —
  `with_aegis` and the `node_errors` element shape; `LlmError::rate_limited` and the
  `RateLimitExceeded { .. }`, `UsageLimitExceeded { .. }` and `ProcessingError(..)` arms;
  `DeepSeekConfig::new` / `AnthropicConfig::new` in the SURFACE-02 shape with
  `with_timeout_seconds`; `Commissary::for_model` / `dispense` / `verify_fits` and the `Stockpile`
  fields; `MockLlmAdapter::with_response` / `with_responses` / `with_error` / `with_delay`;
  `PaladinBuilder::new(llm).name(..).system_prompt(..).build().await`;
  `MCPClient::connect_streamable_http` / `invoke_tool`; `PromptItem::parameters` /
  `set_parameters`; the four-field `Armament` literal; and exhaustive matches on `FinishReason`
  and `ArsenalError` (or both marked `#[non_exhaustive]` and registered in §9.2) — with at least
  one new pin proven red first (`50.1-MUSTER-RED-EVIDENCE.md`), the muster compiling at the seal,
  and D-C (a dedicated gate or CI step for `make check-consumer-surface`) recorded

- [ ] **SURFACE-04**: MIGRATION §9.8 "Library consumers" carries four additions a direct-adapter
  consumer needs: a pointer list to the pre-`v0.10.0` §9.2 rows (P31 `TokenUsage` fields and
  `from_total`, `PaladinResult.token_count` → `usage`, `StreamingResponse` `#[non_exhaustive]`;
  P32 `Commissary::new` / `from_port` without `is_exact_counter`, `TokenCounter` /
  `TokenCounterFactory` removal; P33 `RagRetrievalService` return type); the 429 behaviour row
  (M-B-05: adapters no longer retry a 429, a directly built adapter has no pacing unless Cadence
  is composed, with the `build_cadence` / `compose_llm` or SURFACE-05 recipe, and a spend-cap
  429 is `UsageLimitExceeded`); one line that an `LlmPort` decorator must forward
  `model_profile`; and the adapter-wide `max_context_tokens` consequence (Anthropic 200,000 →
  1,000,000, DeepSeek 64,000 → 1,048,576, so `Commissary::from_port` rations 5× / 16× more) with
  its two remedies (`context_window`, or `Commissary::for_model`) — so Phase 46's migration guide
  inherits them

- [ ] **SURFACE-05**: A facade-reachable pacing-only composition path (a re-export of
  `paladin_llm::cadence::with_cadence`, or a `pace_llm(llm, wiring)`) lets a direct-adapter
  consumer wrap an `Arc<dyn LlmPort>` in Cadence through `paladin::` with no `PriceTable`,
  proven by one doc test (or D-D records the deferral)

- [ ] **SURFACE-06**: `claude-opus-4-8` and `deepseek-v4-pro` resolve to seeded profiles with no
  `paladin.unprofiled_model` for them, by the D-E option the operator chooses — a probe re-run on
  the operator's keys for the two models, a documented listing-only seed tier, or a declined seed
  with a §9.8 `model_overrides` recipe — without making the seed rule in `registry/seeded.rs` or
  ADR-0061 untrue of the tree; `deepseek-v4-flash` is not seeded (absent from DeepSeek's own
  listing)

### Object storage (STORE)

- [x] **STORE-01**: The dev/test compose stack and the Coverage, Integration Tests, Docker
  Integration Tests and Kubernetes Smoke Test CI jobs run against RustFS pinned to an exact tag,
  with bucket bootstrap replacing `mc`; no MinIO image remains in any live configuration

- [x] **STORE-02**: The `FileStoragePort` contract suite (including presigned URLs, multipart
  uploads and ETags) passes against RustFS; the existing S3 adapter is reused, and a new adapter
  behind a feature flag is added only if that suite fails

- [x] **STORE-03**: Storage docs are updated, and the decision on whether the production k8s
  manifest also moves to RustFS is recorded

### Platform (PLAT, continued)

- [x] **PLAT-07**: `GET /runs` and every `/runs/{id}*` read route return only runs the calling
  principal may see, enforced by one shared authorization function; another caller's run returns
  404 (`WINDOWS.md` row 32). The admin-role override is decided in the phase

- [x] **PLAT-08**: Legacy `Runnable::Agent` runs emit SSE live events and webhook deliveries like
  graph runs (row 31)

- [x] **PLAT-09**: The SSE `done` event matches the persisted status — `Cancelled` for a
  caller-cancelled run and `Halted` with the Treasurer reason for a spend halt (D-14)

### Observability (OBS, continued)

- [x] **OBS-05**: `LogTraceSink` / `TraceDispatcher::emit` skips serialisation when logging is
  disabled and reuses its buffers; the tracing-overhead benchmark is re-measured and either meets
  PRD 07's ≤ 3 % bar or records a new accepted figure on the record (D-16, row 35)

### Docs and hygiene (CURR, continued)

- [ ] **CURR-22**: The docs fixes assigned by the v0.10.0 acceptance audit are closed: "twelve
  publishable crates" in `development-setup.md`, `release-automation.md` and `release-recovery.md`;
  `paladin-eval` Trusted Publishing and Credential History rows; the CHANGELOG `[0.10.0]` heading
  date

- [ ] **CURR-23**: A Treasurer mdBook page, a configuration reference for pricing, `allowance` and
  the warn threshold, and a v0.10 → v0.11 `MIGRATION.md` guide are published

- [ ] **CURR-24**: Nyquist validation for Phases 22, 24, 29, 30, 34, 36 and 36.1 moves out of
  `draft`, and Phase 28 is made `nyquist_compliant` or its gaps are recorded

- [ ] **CURR-25**: The three v2 debt lines (oversized service files, clone/lock contention,
  dependency-allowlist drift) are each fixed or waived with a written reason

### Release (SHIP, continued)

- [ ] **SHIP-07**: v0.11.0 is released: all 12 publishable crates on crates.io at `0.11.0`, the tag
  cut on a `main` merge commit, CHANGELOG and MIGRATION complete, and the publish-order gate green

## v2 Requirements

Deferred. Tracked but not in this roadmap. FUT-08 (currency cost), FUT-09 (rate pacing and
stampede locks) and FUT-10 (RustFS) are promoted into this milestone as PRICE-*, PACE-* and STORE-*.

### Platform & Tooling

- **FUT-01**: Hand-polished Python/TypeScript SDKs
- **FUT-02**: Full graphical IDE / live-editing studio
- **FUT-03**: Multi-region/HA storage replication
- **FUT-04**: Billing / usage metering (payment integration)
- **FUT-05**: Multi-tenant orgs / RBAC beyond existing scopes and the API-key → tenant mapping
- **FUT-13**: DNS-rebinding address pinning for the webhook SSRF guard

### Runtime

- **FUT-06**: Automatic memory extraction/writing policies for the Vault
- **FUT-07**: LLM-as-judge eval scoring
- **FUT-11**: Long-context tier pricing (distinct unit prices above a context threshold, e.g. ~200K
  tokens)

- **FUT-12**: Multi-currency pricing and FX conversion (v0.11.0 prices in one operator currency)

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Exact per-call cost prediction before the response | Completion, cache and reasoning counts are unknowable pre-flight; every comparable system settles after the fact |
| Bundled / default price table | Operator decision: prices come from config only; a bundled table goes stale in a published library |
| Billing and payment integration | FUT-04; the Treasurer governs spend, it does not charge anyone |
| Hosted spend dashboard | Spend is surfaced through heralds, CLI and traces; a UI is FUT-02 territory |
| Retroactive allowance enforcement on historical runs | Allowances apply from configuration forward only |
| Reworking `TokenBudget` / `ModelCallLimit` / `ToolCallLimit` or any Commissary (input-side) work | PRD §4; the Treasurer composes these, it does not replace them (ADR-0049, ADR-0050) |
| Renaming `TokenBudget`, `TokenCounterPort`, `TokenUsage` or `max_tokens` | Locked by the two-officer model decisions |
| Using `Treasurer` outside the framework vocabulary | Milestone 13 §5.3 guardrail: it must never mix with the downstream `GarrisonTreasury` term |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PRICE-01 | Phase 38 | Complete |
| PRICE-02 | Phase 38 | Complete |
| PRICE-03 | Phase 38 | Complete |
| LEDGR-01 | Phase 39 | Complete |
| LEDGR-02 | Phase 39 | Complete |
| LEDGR-03 | Phase 39 | Complete |
| LEDGR-04 | Phase 39 | Complete |
| TENANT-01 | Phase 40 | Complete |
| TENANT-02 | Phase 40 | Complete |
| ALLOW-01 | Phase 41 | Complete |
| ALLOW-02 | Phase 41 | Complete |
| ALLOW-03 | Phase 42 | Complete |
| ALLOW-04 | Phase 41 | Complete |
| ALLOW-05 | Phase 42 | Complete |
| PACE-01 | Phase 43 | Complete |
| PACE-02 | Phase 43 | Complete |
| PACE-03 | Phase 43 | Complete |
| PACE-04 | Phase 43 | Complete |
| PACE-05 | Phase 43 | Complete |
| LEGACY-01 | Phase 44 | Complete |
| LEGACY-02 | Phase 44 | Complete |
| LEGACY-03 | Phase 44 | Complete |
| LEGACY-04 | Phase 44 | Complete |
| STORE-01 | Phase 45 | Complete |
| STORE-02 | Phase 45 | Complete |
| STORE-03 | Phase 45 | Complete |
| PLAT-07 | Phase 40 | Complete |
| PLAT-08 | Phase 45 | Complete |
| PLAT-09 | Phase 42 | Complete |
| OBS-05 | Phase 45 | Complete |
| CURR-22 | Phase 46 | Pending |
| CURR-23 | Phase 46 | Pending |
| CURR-24 | Phase 46 | Pending |
| CURR-25 | Phase 46 | Pending |
| SHIP-07 | Phase 47 | Pending |
| ENTITY-01 | Phase 48 | Complete |
| ENTITY-02 | Phase 48 | Complete |
| ENTITY-03 | Phase 48 | Complete |
| ENTITY-04 | Phase 48 | Complete |
| ENTITY-05 | Phase 48 | Complete |
| ADOPT-01 | Phase 49 | Superseded (scope cut 2026-10-14) |
| ADOPT-02 | Phase 49 | Superseded (scope cut 2026-10-14) |
| ADOPT-03 | Phase 49 | Superseded (scope cut 2026-10-14) |
| ADOPT-04 | Phase 49 | Complete |
| ADOPT-05 | Phase 49 | Complete |
| PARITY-01 | Phase 50 | Complete |
| PARITY-02 | Phase 50 | Complete |
| PARITY-03 | Phase 50 | Complete |
| PARITY-04 | Phase 50 | Complete |
| PARITY-05 | Phase 50 | Complete |
| PARITY-06 | Phase 50 | Complete |
| PARITY-07 | Phase 50 | Complete |
| PARITY-08 | Phase 50 | Complete |
| REASON-01 | Phase 51 | Pending |
| REASON-02 | Phase 51 | Pending |
| REASON-03 | Phase 51 | Pending |
| REASON-04 | Phase 51 | Pending |
| REASON-05 | Phase 51 | Pending |
| REASON-06 | Phase 51 | Pending |
| REASON-07 | Phase 51 | Pending |
| SURFACE-01 | Phase 50.1 | Pending |
| SURFACE-02 | Phase 50.1 | Pending |
| SURFACE-03 | Phase 50.1 | Pending |
| SURFACE-04 | Phase 50.1 | Pending |
| SURFACE-05 | Phase 50.1 | Pending |
| SURFACE-06 | Phase 50.1 | Pending |

**Coverage:**

- v1 requirements: 66 total
- Mapped to phases: 66
- Unmapped: 0 ✓

---
*Requirements defined: 2026-09-24*
*Last updated: 2026-10-19 — Phase 50.1 inserted (Consumer-Surface Readiness for v0.11.0, after Phase 50 and before Phase 51), SURFACE-01..06 minted from the fourth web3sec handoff (`50.1-HANDOFF-web3sec-consumer-surface.md`); Phases 38-51 plus 50.1, 66/66 requirements mapped, 100% coverage. Previous: 2026-10-14 — Phase 49 scope cut: ADOPT-01..03 superseded by one consumer move to `v0.11.0` after Phases 50-51; ADOPT-04..05 complete. Previous: 2026-10-14 — Phase 51 added (Adjustable Reasoning Effort), REASON-01..07 minted from the
third web3sec handoff (`51-HANDOFF-web3sec-reasoning-effort.md`); Phases 38-51, 60/60 requirements
mapped, 100% coverage. Previous: 2026-10-14 — Phase 50 added (Provider Parity & Configuration-Driven Model Swap),
PARITY-01..08 minted from the second web3sec handoff (`50-HANDOFF-web3sec-parity.md`); Phases 38-50,
53/53 requirements mapped, 100% coverage. Previous: 2026-10-14 — Phase 49 added (Downstream Adoption of v0.10.1), ADOPT-01..05 minted
from the web3sec handoff (`49-HANDOFF-web3sec.md`); Phases 38-49, 45/45 requirements mapped, 100%
coverage. Previous: 2026-10-12 — Phase 48 added (Base Entity Rename), ENTITY-01..05 minted from the
code survey in `48-CONTEXT.md`; Phases 38-48, 40/40 requirements mapped, 100% coverage. Previous:
2026-09-24 — roadmap created (`/gsd-new-project` roadmapper), Phases 38-47, 35/35 requirements
mapped*
*Corrected 2026-10-14 (plan 48-09): ENTITY-01's storage list, ENTITY-04's type name and outcome, and ENTITY-05's baseline file were wrong or open in the survey; the shipped tree and 48-CONTEXT.md D-05/D-15 settle them. No requirement ID changed.*
