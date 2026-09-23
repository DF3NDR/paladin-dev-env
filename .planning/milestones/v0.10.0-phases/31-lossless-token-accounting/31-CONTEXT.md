# Phase 31: Lossless Token Accounting - Context

**Gathered:** 2026-09-14
**Status:** Ready for planning
**Mode:** `--auto` (all gray areas auto-selected; every question resolved to the recommended option and logged in `31-DISCUSSION-LOG.md`)

<domain>
## Phase Boundary

Carry the provider's `TokenUsage` prompt/completion split, plus new optional cache-read /
cache-write / reasoning counts, unchanged from `LlmPort` up through `PaladinResult`,
`BattalionResult.per_paladin_tokens`, the Waypoint `NodeExecutionRecord`,
`TraceEvent::NodeFinished` and `TraceEvent::RunFinished`, and out to a herald in both JSON and
Markdown. Remove the `TokenUsage::from_total` zeroing from the battalion aggregation path. Make
every LLM adapter's streaming path report the same `TokenUsage` as its non-streaming path, or
document the adapter as an explicit exception. Register every touched public type in
`MIGRATION.md` §9.2 with a row-level-matched `cargo semver-checks` allowlist entry and a
`CHANGELOG.md` `[0.10.0]` entry.

**Breaking, clean break, no shims** — governed by ADR-0051 (X-03 superseded for Phases 31-33
only). No `#[deprecated]` bare-count accessor, no `token_count` compatibility field, no
legacy-shape deserializer. This is the keystone phase: Phase 32 and Milestone 14 (Treasurer)
build on the shipped shape.

**Not in this phase:** currency pricing, `cost_estimate` production, allowances, rate pacing
(Milestone 14); renaming any token type (`TokenUsage`, `TokenBudget`, `TokenCounterPort`,
`max_tokens` all keep their names — Phase 30 D-17); the per-chunk `tokens`/`token_count` hints
on `ChunkMetadata`/`StreamChunk` (approximate chunk sizes, not accounting — untouched);
`TokenCounterPort`/`Commissary` changes (Phase 32); RAG truncation (Phase 33).

</domain>

<decisions>
## Implementation Decisions

### `TokenUsage` shape and the meaning of `total_tokens` (ACCT-01)
- **D-01:** `TokenUsage` (`crates/paladin-core/src/platform/container/token_usage.rs`, the single
  definition, re-exported by `paladin_ports::output::llm_port` and
  `paladin_core::platform::container::battalion`) gains exactly three fields, each
  `Option<u32>` with `#[serde(default)]`: `cache_read_tokens`, `cache_write_tokens`,
  `reasoning_tokens`. Field order after the three existing counters. No other field is added
  or removed. — **Reversibility:** one-way — it is the serialized wire/persistence shape every
  carrier, herald, SSE event and the Milestone 14 pricing function will consume.
- **D-02:** **`total_tokens` INCLUDES cache and reasoning tokens.** The invariant is
  `total_tokens == prompt_tokens + completion_tokens`; `prompt_tokens` is every input token
  the provider billed for the call (cache reads and cache writes included), `completion_tokens`
  is every output token (reasoning/thinking included). The three optionals are **"of which"
  sub-counts** that are already inside those two numbers — never additive on top of them —
  so `cache_read_tokens + cache_write_tokens <= prompt_tokens` and
  `reasoning_tokens <= completion_tokens` always hold. The rustdoc on the struct states this in
  one sentence and states the two inequalities. Rationale: it keeps `TokenUsage::new(p, c)`,
  `PartialEq` and every existing total-based consumer (`TokenBudget`, `total_tokens_max` eval
  assertion, `BattalionResult.total_tokens`) semantically unchanged, and a pricing function can
  still compute discounted cache pricing as `(prompt − cache_read) × p_in + cache_read × p_cache`.
  — **Reversibility:** one-way — a documented arithmetic contract downstream will encode.
- **D-03:** `None` means "the provider did not report this figure"; `Some(0)` means "the provider
  reported zero". Adapters never fabricate `Some(0)` for a field the provider's usage object
  does not carry.
- **D-04:** `TokenUsage` stays a **constructible struct with `Default`** (X-10.3 option (b), the
  `PaladinResult` / `Settings` precedent in `MIGRATION.md` §9.2) — it is NOT marked
  `#[non_exhaustive]`, because it has `Default` and functional-update syntax is disallowed
  cross-crate on a `#[non_exhaustive]` struct. `TokenUsage::new(prompt, completion)` keeps its
  signature and leaves the optionals `None`; three chainable builders are added
  (`with_cache_read(u32)`, `with_cache_write(u32)`, `with_reasoning(u32)`). Every in-tree full
  struct literal (`openai/adapter.rs`, `anthropic/adapter.rs`, `deepseek/adapter.rs`,
  `gemini/adapter.rs`, `compat/engine.rs`, both vision adapters, `mock.rs`, herald and core
  tests) migrates to `TokenUsage::new(..)` + builders or `..Default::default()` in the same
  commit as the field addition. Suppression: the existing
  `constructible_struct_adds_field = "allow"` in `crates/paladin-core/Cargo.toml`, mirrored
  by a new `.cargo/semver-checks-allowlist.toml` entry for `paladin-ai-core | TokenUsage`.
- **D-05:** `TokenUsage::from_total` is **deleted outright** (not just removed from the battalion
  path). A constructor whose only purpose is to discard the split is the bug this phase exists
  to fix; the only in-tree callers are the two battalion services and test fixtures, all of
  which move to `TokenUsage::new`. The §9.2 row tells a downstream caller that a total-only
  usage is an upstream data-loss bug to fix at the producer, not something to reconstruct.
  — **Reversibility:** reversible (a method can be re-added), but deliberately not offered.
- **D-06:** Accumulation arithmetic lives on the type: `impl Add`, `impl AddAssign` and
  `impl Sum` for `TokenUsage` in `token_usage.rs`, **saturating** on every `u32`, with the
  `Option` merge rule `None + None = None`, `None + Some(x) = Some(x)`,
  `Some(a) + Some(b) = Some(a.saturating_add(b))`; `total_tokens` is recomputed as
  `prompt + completion` after the add so the D-02 invariant survives summation. Every
  accumulator in the tree (execution-service reasoning loop, Formation/Phalanx per-Paladin
  maps, the engine's `TraceDispatcher` run total, eval runner) uses this one implementation.
  Counters stay `u32` — widening to `u64` is out of scope (it would retype the port and every
  adapter for a per-run overflow that needs 4 billion tokens); the trace aggregate that was
  `u64` becomes a saturating `u32` inside `TokenUsage` and the rustdoc says so.

### Carrier shape and naming (ACCT-02)
- **D-07:** The carrier field is named **`usage`** on every carrier, matching
  `LlmResponse.usage` — `PaladinResult.usage: TokenUsage` (replaces `token_count: u32`),
  `NodeExecutionRecord.usage: TokenUsage` (replaces `token_count: u64`),
  `TraceEvent::NodeFinished { usage: TokenUsage, .. }` (replaces `token_count: u64`),
  `TraceEvent::RunFinished { usage: TokenUsage, .. }` (replaces `total_tokens: u64`).
  `ExecutionMetadata.token_usage` (herald.rs) keeps its existing name — it is already a full
  `TokenUsage` and is not a bare count. — **Reversibility:** one-way — public field names on
  serialized types consumed by the downstream app, SSE clients and persisted Waypoints/traces.
- **D-08:** **The bare-count rule:** a bare total (`u64`/`u32`) may coexist beside a full
  `TokenUsage` at the same level as a derived convenience; it may never be the ONLY token
  carrier at any level. Therefore `BattalionResult.total_tokens: u64` **stays** (the split lives
  in `per_paladin_tokens`, which already has type `HashMap<String, TokenUsage>` — only its
  values change from zeroed to real), while `PaladinResult`, `NodeExecutionRecord`,
  `NodeFinished` and `RunFinished`, whose bare count was their only carrier, are replaced.
- **D-09:** `PaladinResult` stays **constructible and not `#[non_exhaustive]`** (Phase 25 D-26
  reasoning is unchanged); `PaladinResult::new(output, usage: TokenUsage, execution_time_ms,
  loop_count, stop_reason)` takes the usage in the `token_count` position. No `token_count()`,
  `total()` or `total_tokens()` accessor is added on `PaladinResult` — `result.usage.total_tokens`
  is one field access and needs no convenience method (PRD R3: only permanent good API earns a
  place). Every in-tree literal and `..Default::default()` site migrates in the same commit.
- **D-10:** Formation (`formation_service.rs:~231`) and Phalanx (`phalanx_service.rs:~279`)
  insert `result.usage.clone()` into `per_paladin_tokens` and add
  `u64::from(result.usage.total_tokens)` to `total_tokens`. Chain-of-Command, Campaign,
  Conclave, Council, Grove, Maneuver and `Commander` are audited for any other place a
  `PaladinResult` count is copied into a `TokenUsage`; none may construct a zero-split usage.
- **D-11:** Engine: `superstep.rs`'s two Paladin arms (structured `~1095`, scoped `~1140`)
  thread `result.usage` (a `TokenUsage`, not `u64::from(result.token_count)`) through the
  `(paladin_id, usage, outcome)` tuple into the `NodeExecutionRecord` and the `NodeFinished`
  event; a cache hit records `TokenUsage::default()` (was `0`) and keeps `cache_hit: true`.
  `hooks.rs`'s `TraceDispatcher.token_total: AtomicU64` becomes a synchronously-updated
  `TokenUsage` accumulator (a `std::sync::Mutex<TokenUsage>` held for the duration of one
  `+=` is acceptable; five atomics is also acceptable — planner's choice) so
  `RunFinished.usage` is exact the instant `emit` returns, preserving the existing D-02/D-04
  observability guarantee in `hooks.rs`'s own rustdoc. `RunFinished` is emitted at five sites
  in `engine/mod.rs` (`~2084`, `~2198`, `~2317`, `~2671`, `~2840`) — all five switch together.
- **D-12:** The execution service's reasoning loop (`paladin_execution_service.rs` `~1257`,
  `~1501`) accumulates `usage += response.usage` per iteration and sets
  `middleware_cx.cumulative_tokens = usage.total_tokens` (the `TokenBudget` middleware keeps
  its total-based semantics unchanged — VOCAB-05 / Phase 30 D-17). All six `PaladinResult`
  construction sites in the service (`~1216` vision: `TokenUsage::new(vt.prompt_tokens,
  vt.completion_tokens)` from `VisionTokenUsage`; `~1468`, `~1533`, `~1814`, `~1841`, `~3065`)
  carry the accumulated `usage`.

### Streaming usage parity (ACCT-03)
- **D-13:** `StreamingResponse` (`crates/paladin-ports/src/output/llm_port.rs:~1054`) gains
  `usage: Option<TokenUsage>` with `#[serde(default)]` and is marked **`#[non_exhaustive]`**
  (X-10.3 option (a), the `LlmRequest` / `GarrisonEntry` precedent: it has no `Default`, so
  there is no functional-update site to preserve and marking it makes the *next* field free).
  Constructors are added so every adapter, the mock, `fallback.rs` and the conformance suite
  stop writing literals: `StreamingResponse::delta(text)`, `StreamingResponse::terminal(finish_reason)`
  and a chainable `with_usage(TokenUsage)`. Suppression: the existing
  `struct_marked_non_exhaustive = "allow"` in `crates/paladin-ports/Cargo.toml`, mirrored by a new
  allowlist entry `paladin-ports | StreamingResponse`. — **Reversibility:** one-way — a port
  trait's item type used by every provider adapter, in-tree and downstream.
- **D-14:** **Terminal-chunk contract:** `usage` is `Some` on exactly one chunk per stream —
  the chunk that carries `finish_reason: Some(..)` — and `None` on every other chunk. Adapters
  guarantee this even when the provider delivers usage *after* the finish frame: the
  OpenAI-family `stream_options: {"include_usage": true}` usage frame arrives after the
  `finish_reason` frame and before `[DONE]`, so `CompatEngine::generate_stream`
  (`compat/engine.rs:~836-935`), `openai/adapter.rs`'s own `make_streaming_request` and
  `deepseek/adapter.rs:~674` **hold the finish reason** seen on a `choices[].finish_reason` frame
  and emit it on the `[DONE]` terminal chunk together with the captured usage (they already emit
  a `Stop` chunk at `[DONE]`; the change is to carry the real finish reason and usage on it and
  stop emitting `finish_reason` on the earlier frame). The execution service's stream consumer
  (`paladin_execution_service.rs:~3217`, `if is_final { return; }`) therefore keeps stopping
  at the first finished chunk and reads the usage from it. Rationale: one rule, no buffering of
  text, and the existing conformance case "streaming assembles in wire order with a terminal
  stop" stays valid.
- **D-15:** Per-provider mechanism (the researcher verifies each against current provider docs
  via Context7 before planning locks field names; do not trust memory):
  - **OpenAI / `CompatEngine` presets (openai_compatible, grok, kimi, qwen, ollama) / DeepSeek:**
    send `stream_options: {"include_usage": true}` on every streaming request; parse the
    `usage` object on the frame whose `choices` is empty; `CompatUsage`
    (`compat/types.rs:~90`) and the OpenAI/DeepSeek usage structs gain optional
    `prompt_tokens_details.cached_tokens` and `completion_tokens_details.reasoning_tokens`
    (DeepSeek additionally `prompt_cache_hit_tokens`) so both paths map cache/reasoning.
  - **Anthropic** (`anthropic/adapter.rs:~483`): `message_start.message.usage` carries
    `input_tokens` + `cache_read_input_tokens` + `cache_creation_input_tokens`;
    `message_delta.usage` carries the cumulative `output_tokens` (and, per the captured
    fixtures already in the adapter's tests, `output_tokens_details.thinking_tokens`); the
    adapter accumulates across events and attaches the final `TokenUsage` on the
    `message_stop` terminal chunk. `ClaudeStreamEvent` grows the fields it needs.
  - **Gemini** (`gemini/adapter.rs:~872`): every SSE frame's `usageMetadata` is cumulative; the
    last frame's value (which arrives together with the terminal `finishReason`) is attached to
    the terminal chunk; `cachedContentTokenCount` → `cache_read_tokens`, `thoughtsTokenCount` →
    `reasoning_tokens`, `completion_tokens = candidatesTokenCount + thoughtsTokenCount` (D-02).
  - **Mock** (`mock.rs`, both `generate_stream` impls): the scripted stream ends with a terminal
    chunk carrying the configured `token_usage`, so the execution-service parity test runs
    offline. **`FallbackLlmAdapter`** (`fallback.rs:~278`) passes chunks through unchanged.
- **D-16:** **Explicit exception:** the generic `OpenAiCompatibleAdapter` (and any preset whose
  server ignores `stream_options`) cannot guarantee streamed usage — when the server omits the
  usage frame the terminal chunk carries `usage: None`. This is documented in the adapter's
  rustdoc AND on the mdBook provider page (`docs/src/appendix/provider-expansion.md`, the
  per-provider feature matrix — add a "Streamed usage" row/column; `contributing-providers.md`
  gains the terminal-chunk contract as a requirement for new adapters). Ollama's native
  `/api/chat` shape (`prompt_eval_count`/`eval_count`) is documented the same way if the
  adapter's engine does not receive an OpenAI-shaped usage frame from the Ollama version pinned
  in the dev stack.
- **D-17:** **No estimation fallback.** When a streamed call ends with `usage: None`, the
  execution service records `TokenUsage::default()` for that call and logs one `warn!` naming
  the provider — it never substitutes a `TokenCounterPort` estimate for a billed count
  (estimates are a Commissary concern; the Treasurer must never price an estimate as a
  provider figure). Deferred as a possible Milestone 14 opt-in.
- **D-18:** The execution service's streaming surface carries the usage to its consumer:
  `ChunkMetadata` (`crates/paladin-ports/src/output/paladin_port.rs:~409`, the
  `PaladinStreamChunk.metadata` payload) gains `usage: Option<TokenUsage>` (`#[serde(default)]`),
  populated only on the `is_final` chunk from the provider's terminal chunk; its existing
  per-chunk `tokens: Option<u32>` hint is untouched. X-10.3 handling for `ChunkMetadata` follows
  whichever of options (a)/(b) its `Default` status dictates (planner checks; same reasoning as
  D-04/D-13). The SSE run-stream and `paladin-web` streaming responses forward it.
- **D-19:** **Parity test = one shared conformance case.** A new case in
  `crates/paladin-llm/src/conformance.rs`'s `llm_conformance_suite!`
  (`streaming_usage_equals_non_streaming_usage`) opens `success_body()` through `generate()`
  and `stream_body()` through `generate_stream()` against the same mockito server and asserts
  the terminal chunk's `usage` equals the non-streaming `LlmResponse.usage` field-for-field
  (including the three optionals). `ConformanceFixture::stream_body()`'s contract is extended:
  it must carry the same usage figures as `success_body()`. Every adapter with a real streaming
  parser instantiates the suite or an equivalent per-adapter test: today only `ollama`,
  `gemini` and `openai_compatible` instantiate it — **`openai`, `anthropic`, `deepseek`, `grok`,
  `kimi`, `qwen` and the mock must be covered too** (instantiate the suite where the fixture is
  cheap; a dedicated `#[tokio::test]` where the wire shape differs, e.g. Anthropic's event
  stream). The `CASE_COUNT` assertion in `conformance.rs:~649` is updated from 8 to 9. A second,
  execution-service-level test proves a Paladin run via `execute_stream` ends with the same
  `usage` as `execute` against the mock.

### Adapter population of cache and reasoning counts
- **D-20:** Adapters populate the optionals on **both** paths wherever the provider's existing
  usage object carries the figure, mapping to D-02's inclusive semantics; nothing else changes
  in adapter behaviour. Known mappings (researcher confirms names): OpenAI
  `prompt_tokens_details.cached_tokens` → `cache_read_tokens`,
  `completion_tokens_details.reasoning_tokens` → `reasoning_tokens`; Anthropic
  `cache_read_input_tokens` → `cache_read_tokens`, `cache_creation_input_tokens` →
  `cache_write_tokens`, `output_tokens_details.thinking_tokens` → `reasoning_tokens`, and
  **`prompt_tokens = input_tokens + cache_read_input_tokens + cache_creation_input_tokens`**
  (Anthropic's `input_tokens` excludes cached tokens, so today's `prompt_tokens` under-reports
  billed input whenever caching is on — this correction is called out in the CHANGELOG entry);
  Gemini as in D-15; DeepSeek `prompt_cache_hit_tokens` → `cache_read_tokens`,
  `completion_tokens_details.reasoning_tokens` → `reasoning_tokens`; `CompatEngine` presets
  parse the OpenAI-shaped `*_details` objects when present. Ollama native fields and any
  provider without the figure → `None` (D-03). Vision adapters (`VisionTokenUsage`) are
  untouched.

### Herald observability (ACCT-04)
- **D-21:** `JsonHerald` emits the full `TokenUsage` object under `"usage"` for a
  `PaladinResult` (replacing `"token_count"`), serializes `per_paladin_tokens` values as full
  objects (already the case — now with real splits), and emits `ExecutionMetadata.token_usage`
  as the full object (today `json_herald.rs:~207` emits only `total_tokens`). **JSON always emits
  the three optional keys**, as `null` when `None` — no `skip_serializing_if` — so JSON
  consumers see a stable key set; deserialization tolerates their absence via
  `#[serde(default)]`.
- **D-22:** `MarkdownHerald` replaces the single "Token Count" field with a "Token Usage" block:
  `Prompt`, `Completion`, `Total` always; `Cache read`, `Cache write`, `Reasoning` rows only
  when `Some` (Markdown is for humans — unreported figures are omitted, not rendered as
  "n/a"). For a `BattalionResult` it adds a per-Paladin usage table (name | prompt | completion
  | total | cache read | cache write | reasoning) under the existing "Total Tokens" summary.
- **D-23:** `TableHerald` is Claude's discretion: minimally its "Tokens" column keeps the total
  and its tests drop `from_total`; adding prompt/completion columns is optional. The CLI
  formatters (`src/application/cli/formatters/output.rs`, `commands/agent.rs`,
  `commands/battalion.rs`) print `total (prompt P / completion C)` and append cache/reasoning
  figures only when reported; their JSON mode emits the `usage` object.

### Edge surfaces: HTTP API, SSE, inspector, dev UI
- **D-24:** The HTTP edge carries the full object too — leaving a bare `token_count` on the
  wire would re-create the collapse one layer up and force a second break after 1.0.
  `paladin-web` defines a `TokenUsageResponse` DTO (`utoipa::ToSchema`, `From<TokenUsage>`,
  same six fields) — `paladin-core` gains no `utoipa` dependency — and: `ExecuteResponse`
  (`agent_controller.rs:~122`) replaces `token_count: u32` with `usage: TokenUsageResponse`;
  the run inspector's `CompletedRow` (`src/application/services/run/inspector.rs:~295`)
  replaces `token_count: Option<u64>` with `usage: Option<TokenUsageResponse>` (still `None` on
  a cache hit); the dev-UI row (`dev_ui_controller.rs`) follows. SSE run events serialize
  `TraceEvent` directly, so `node_finished`/`run_finished` payloads change with D-07.
  `crates/paladin-web/openapi.json` is regenerated with `make openapi` in the same commit and
  the change is recorded in `MIGRATION.md` §9.6 (a response-shape change, not a new route).
  The Python-client generation job in `ci.yml` (`~1156-1223`) must stay green on the
  regenerated document. — **Reversibility:** one-way — a published HTTP response contract.

### Compatibility mechanics (ACCT-05)
- **D-25:** **Serde policy:** every new `usage` field is `#[serde(default)]`; the retired
  bare-count keys (`token_count`, `total_tokens` on the trace event) are simply dropped —
  serde ignores unknown keys (no `deny_unknown_fields` exists on these types). **No
  legacy-shape deserializer** maps an old `token_count` into `usage.total_tokens`: Waypoints
  and persisted trace records are new in the untagged v0.10.0 (ADR-0051: no `0.10.0` tag
  exists until Phase 33), so the only pre-Phase-31 rows are developer databases, and a
  `PaladinResult` JSON written by 0.9.x loads with `usage == TokenUsage::default()`. This is
  stated in a `MIGRATION.md` §9.4 note (Waypoint/trace rows written before this phase report
  zero usage for their history; resume is unaffected because token history is not
  resume-critical). The existing `execution_result.rs` legacy-JSON test is rewritten to assert
  exactly that.
- **D-26:** **X-10.3 choice per touched type** (record the reason on each §9.2 row, as the
  existing rows do): `TokenUsage` constructible (D-04); `PaladinResult` constructible (D-09);
  `StreamingResponse` `#[non_exhaustive]` (D-13); `ChunkMetadata` per D-18; `TraceEvent`
  (already `#[non_exhaustive]`) and `NodeExecutionRecord` are new-in-0.10 types, so their rows
  are "listed for completeness — N/A, not in the 0.9.0 baseline" and get **no** allowlist
  entry, exactly like the `ResumeAccepted`/`ThreadApiState` rows; `ExecuteResponse` (shipped
  0.9.0) gets a `paladin-web` row and entry; `BattalionResult` needs no row (no signature
  change — only the values in `per_paladin_tokens` change; note it in the `TokenUsage` row's
  Change cell).
- **D-27:** **Lint discovery is empirical, not guessed:** after the code lands, the executor runs
  `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0`
  for `paladin-ai-core`, `paladin-ports` and `paladin-web` (the `semver` CI job's own command,
  `ci.yml:~357`) and writes one `[[entry]]` per `(crate, lint, migration_row)` that actually
  fires, adding any new lint id to that crate's `[package.metadata.cargo-semver-checks.lints]`
  table. Expected but unconfirmed: `constructible_struct_adds_field` and
  `inherent_method_missing` (the deleted `from_total`) on `paladin-ai-core | TokenUsage`;
  `struct_pub_field_missing` + `constructible_struct_adds_field` on
  `paladin-ai-core | PaladinResult`; `struct_marked_non_exhaustive` (+ possibly
  `constructible_struct_adds_field`) on `paladin-ports | StreamingResponse`;
  `struct_pub_field_missing` on `paladin-web | ExecuteResponse`. The Phase 29 D-04 row-level
  set-equality step (`ci.yml:~359-400`) is the gate: `migration_row` values must equal the
  §9.2 row's first two cells (`crate | Type`), type cell reduced to its first backtick
  identifier.
- **D-28:** `CHANGELOG.md`: the entries go in the **`[0.10.0]`** section (ACCT-05 says so; the
  dated section already exists and the release is untagged), under a `### Changed` bullet for
  the carrier change (`PaladinResult.usage`, `NodeFinished`/`RunFinished`/`NodeExecutionRecord`
  `usage`, `from_total` removed, `StreamingResponse.usage` + `#[non_exhaustive]`,
  `ExecuteResponse.usage`) and a `### Fixed` bullet naming the two corrected under-reports
  (battalion per-Paladin split zeroed by `from_total`; Anthropic `prompt_tokens` excluding
  cached input). The `[Unreleased]` Commissary re-export bullet already there is left alone.
  `docs/src/api-reference/upgrading.md` / `migration-guide.md` gain a short "Token usage
  carriers" subsection pointing at §9.2.
- **D-29:** Rustdoc and mdBook pages that show `token_count` (grep hit list:
  `docs/src/getting-started/quickstart.md`, `operations/observability.md`,
  `user-guides/battalion-patterns.md`, `output-formatting.md`, `agent-orchestrator-bridge.md`,
  `appendix/conclave-pattern.md`, `architecture/domain-model.md`, `user-guides/herald-output.md`,
  `memory-management.md`, `paladin-agents.md`; the `paladin_port.rs` / `llm_port.rs` module
  docs and `PaladinResult`'s own doc example) are updated in the same plan that changes the
  type, so `cargo test --doc` and the mdBook link/`warning-policy = "error"` build stay green.

### Test strategy and gates
- **D-30:** **Round-trip test (ACCT-02):** one engine-level test in `crates/paladin-battalion`
  (next to `paladin_node_execution_record_carries_reported_token_count`, `engine/mod.rs:~3352`)
  scripts a Paladin whose `RecordingPaladinPort` (`engine/test_support.rs`, whose
  `set_output_with_tokens` grows a `set_output_with_usage(name, output, TokenUsage)`) returns
  `TokenUsage::new(1_234, 567).with_cache_read(100).with_cache_write(50).with_reasoning(200)`
  and asserts the identical value on the `NodeExecutionRecord`, the `NodeFinished` event and
  `RunFinished.usage` (summed across two nodes with distinct non-round figures — the
  `trace-and-Waypoint-must-agree` test at `~10227` is the pattern). A second test in
  `formation_service.rs` and `phalanx_service.rs` asserts `per_paladin_tokens[name]` equals the
  Paladin's full usage (regression for the `from_total` bug). A serde test in `token_usage.rs`
  covers legacy-JSON-without-optionals and new-JSON round-trip (ACCT-01).
- **D-31:** Gates before the phase is sealed: `make clean-code`, `cargo test` (doctests
  included — `cargo llvm-cov` skips them, see the project memory note), the 82 % workspace
  line-coverage floor, `make security`, `cargo doc` zero-warning, `mdbook build docs/` (or the
  docs CI job), the `semver` job's per-package run plus the row-level allowlist check, the
  OpenAPI baseline test after `make openapi`, and the Python-client generation job. The
  folded coverage todo (below) is satisfied by noting in the coverage evidence whether local
  `make coverage` reproduces the CI figure.

### Claude's Discretion
- Exact rustdoc wording for D-02; whether the D-06 `Option` merge lives in a private helper or
  inline; `Mutex` vs atomics in `TraceDispatcher` (D-11).
- Constructor names on `StreamingResponse` (D-13) — must be doc-tested.
- Whether `TableHerald` gains split columns (D-23); exact CLI text.
- Whether `TokenUsageResponse` lives in `agent_controller.rs` or a small shared `dto` module.
- Plan/commit granularity — natural waves: (1) `TokenUsage` fields + arithmetic + `from_total`
  removal + serde tests; (2) carriers (`PaladinResult`, battalion services, engine, trace,
  execution service, eval runner, CLI, web) as one coordinated break; (3) streaming contract
  + adapters + conformance; (4) heralds + docs; (5) MIGRATION/allowlist/CHANGELOG/OpenAPI +
  gate evidence. Commit with `git commit` directly (the pre-commit hook runs workspace clippy;
  the GSD commit helper is known to time out — Phase 30 `<specifics>`).

### Folded Todos
- **Verify local `make coverage` reproduces CI's 82.39 % figure**
  (`.planning/todos/2026-08-13-verify-local-coverage-reproduction.md`, score 0.6) — folded as
  a verification note only: ACCT-05 already requires the coverage floor to be green, so the
  plan that records coverage evidence also records whether the local run reproduces the CI
  number. No new scope.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone corpus and governing decisions
- `.project/Milestone_13-Token-Economy/Epic_2/prd-lossless-token-accounting.md` — the source
  PRD: R1-R6, §5 tests, §6 exit criteria, §7 downstream impact. Its §2 "keep a `token_count`
  deprecation shim" bullet is overridden by its own R3 and by ACCT-02 (no shim).
- `.project/Milestone_13-Token-Economy/overview/Milestone-13_Token-Economy.md` — §0 locked
  terms (no renames), §3 verified anchors, §4 findings F1/F8 and decision D-4, §5 clean-break
  policy and downstream-pointer coordination, §7 out of scope.
- `.project/Milestone_14-Treasurer/Epic_1/prd-treasurer-spend-governance.md` — R1 "a function
  mapping a `TokenUsage` (Epic 2 shape) to a currency cost" — the consumer D-02's semantics
  must serve; do not build any of it.
- `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` — clean break for
  Phases 31-33 only; every break still gets a §9.2 row + allowlist row as documentation;
  `0.10.0` is untagged until Phase 33.
- `.planning/decisions/0050-treasurer-reservation.md` — what is reserved for Milestone 14
  (pricing, `cost_estimate`, allowances, pacing) and therefore out of this phase.
- `.project/v0.10.0/00-program-overview.md` §3 X-10 (semver hygiene, still governing) and
  X-03 (superseded here).

### Planning record
- `.planning/ROADMAP.md` — Phase 31 entry (goal, depends-on, success criteria 1-5) and the
  Phase 32/33 entries that depend on this shape (Phase 33 COMM-04 re-seals the release gates).
- `.planning/REQUIREMENTS.md` — ACCT-01…05 and the 2026-09-14 extension record (scope-time
  conflict resolutions a/b/c).
- `.planning/phases/30-token-economy-vocabulary-commissary-anchoring/30-CONTEXT.md` — D-17
  (no renames), D-18 (write "v0.10.0"), `<specifics>` on commit mechanics.
- `.planning/phases/29-program-gates-release/29-CONTEXT.md` D-04 — the row-level allowlist ↔
  §9.2 set-equality gate this phase must satisfy.

### Migration register and semver tooling
- `MIGRATION.md` §9.2 (lines ~162-195) — row format, the `PaladinResult` row (Phase 25 D-26
  constructible reasoning), the `LlmRequest`/`GarrisonEntry` rows (`#[non_exhaustive]`
  reasoning), the "new in 0.10 — listed for completeness, N/A" convention; §9.4
  (persistence notes) and §9.6 (HTTP API) for D-25/D-24.
- `.cargo/semver-checks-allowlist.toml` — `[[entry]]` schema (`crate`, `lint`,
  `migration_row`, `requirement_id`, `justification`); 11 entries today.
- `crates/paladin-core/Cargo.toml:64-67`, `crates/paladin-ports/Cargo.toml:45-47`,
  `crates/paladin-web/Cargo.toml:79-81` — the per-crate `cargo-semver-checks` lint tables.
- `.github/workflows/ci.yml` — `semver` job (`~303-357`, baseline `0.9.0`, eleven packages),
  allowlist set-equality step (`~359-400`), Python client generation from `openapi.json`
  (`~1156-1223`); `Makefile` `openapi` target (`~368`) regenerates
  `crates/paladin-web/openapi.json`.
- `CHANGELOG.md` — `[Unreleased]` and `[0.10.0] - 2026-09-10` sections (D-28).

### Code: the type and every carrier
- `crates/paladin-core/src/platform/container/token_usage.rs` — `TokenUsage` (three `u32`
  fields, `new`, `from_total`, four tests).
- `crates/paladin-core/src/platform/container/execution_result.rs` — `PaladinResult`
  (`token_count: u32` at `~53`, `new` at `~215`, constructible rationale at `~40`, legacy-JSON
  test at `~286`).
- `crates/paladin-core/src/platform/container/battalion/mod.rs` — `BattalionResult`
  (`per_paladin_tokens` `~567`, `total_tokens` `~569`), `TokenUsage` re-export `~494`, tests
  using `from_total` (`~1140`, `~1209`).
- `crates/paladin-core/src/platform/container/waypoint.rs:~562` — `NodeExecutionRecord`
  (`token_count: u64`, `cache_hit`), additive-field precedent (`attempts`, `fork_of`).
- `crates/paladin-core/src/platform/container/trace.rs` — `TraceEvent::NodeFinished`
  (`~220`, `token_count: u64`), `RunFinished` (`~282`, `total_tokens: u64`), the
  every-variant test (`~383-435`), flat-record serialization test (`~460`).
- `crates/paladin-core/src/platform/container/herald.rs` — `ExecutionMetadata.token_usage`
  (`~519`), `StreamChunk.token_count` (per-chunk hint, untouched).
- `crates/paladin-battalion/src/formation_service.rs:~197-308`, `phalanx_service.rs:~279` —
  the `from_total` sites; `chain_of_command_service.rs:~156`, `commander.rs:~669/~740`,
  `campaign_service.rs:~354` — other `BattalionResult`/`token_count` touch points.
- `crates/paladin-battalion/src/engine/superstep.rs` (`~1095`, `~1140`, `~1556`,
  `~2555-2996`, `~3090-3302`), `engine/hooks.rs` (`~84-95`, `~298-308`, `~371`),
  `engine/mod.rs` (five `RunFinished` sites; tests `~3352`, `~9978`, `~10227`),
  `engine/test_support.rs:~500-555` (`RecordingPaladinPort`), `engine/export/overlay.rs:~154`.
- `src/application/services/paladin/paladin_execution_service.rs` — loop accumulator
  (`~1257`, `~1501-1502`), six `PaladinResult` sites, stream consumer (`~3121-3262`,
  `if is_final { return; }` at `~3217`).
- `src/application/services/paladin/handoff_service.rs:~524`,
  `src/application/services/orchestration/processors/paladin_processor.rs:~120`,
  `src/application/services/paladin/middleware/limits.rs` (`cumulative_tokens` consumer),
  `crates/paladin-eval/src/runner.rs:~771`, `crates/paladin-eval/src/assertion.rs:~157-184`
  (`total_tokens_max` reads `RunFinished`), `crates/doc-examples/src/bridge.rs:~64`.

### Code: ports, adapters, streaming
- `crates/paladin-ports/src/output/llm_port.rs` — `LlmResponse.usage` (`~876`),
  `StreamingResponse` (`~1054`), `generate_stream` signature (`~1411`), `TokenUsage` re-export
  (`~1004`), module-doc examples that print `token_count`.
- `crates/paladin-ports/src/output/paladin_port.rs` — `PaladinStreamChunk` (`~396`),
  `ChunkMetadata` (`~409`), doc examples and tests referencing `token_count`.
- `crates/paladin-llm/src/compat/engine.rs` (`generate` usage build `~783-800`,
  `generate_stream` `~836-935`), `compat/types.rs:~90` (`CompatUsage`, `CompatStreamResponse`).
- `crates/paladin-llm/src/openai/adapter.rs` (`OpenAIUsage` `~201`, `generate_stream` `~632`,
  own `make_streaming_request`), `deepseek/adapter.rs` (`DeepSeekUsage` `~197`,
  `annotate_with_usage` `~274`, stream `~674-760`), `anthropic/adapter.rs` (`ClaudeUsage`
  `~663`, stream `~483-560`, captured usage fixtures `~881-909` showing
  `cache_read_input_tokens`/`cache_creation_input_tokens`/`output_tokens_details.thinking_tokens`),
  `gemini/adapter.rs` (`GeminiUsageMetadata` `~1058`, stream `~872`), `grok`/`kimi`/`qwen`/
  `ollama` adapters (delegate `generate_stream` to `CompatEngine`), `mock.rs` (`~383`, `~534`,
  `with_token_usage*` `~149-165`), `fallback.rs:~278`.
- `crates/paladin-llm/src/conformance.rs` — `ConformanceFixture`, the 8 cases, the
  `CASE_COUNT` assertion (`~649`); instantiated by `ollama` (`~603`), `gemini` (`~3106`),
  `openai_compatible` (`~1325`) only.
- `crates/paladin-ports/src/output/vision_port.rs` — `VisionTokenUsage` (untouched; mapped
  into `TokenUsage::new` at the execution service's vision site).

### Code: heralds, CLI, HTTP edge
- `crates/paladin-herald/src/json_herald.rs` (`~122`, `~151-152`, `~207`, tests `~387-441`),
  `markdown_herald.rs` (`~257`, `~294`, `~340`, tests `~548-607`), `table_herald.rs`
  (`~183-230`, `~403`).
- `src/application/cli/formatters/output.rs` (`~271`, `~350`, `~506`, `~567`),
  `src/application/cli/commands/agent.rs:~510`, `commands/battalion.rs` (`~386`, `~539`, `~729`).
- `crates/paladin-web/src/agent_controller.rs:~118-145` (`ExecuteResponse`),
  `src/application/services/run/inspector.rs:~295` and `~455-475` (`CompletedRow`),
  `crates/paladin-web/src/dev_ui_controller.rs` (`~232`, `~641`, `~752-760`),
  `src/application/services/run/events.rs` (SSE run events over `TraceEvent`),
  `src/infrastructure/telemetry/persisting_sink.rs:~176`, `crates/paladin-web/openapi.json`
  (`token_count` at `~3353/~3379`).

### Docs to update
- `docs/src/appendix/provider-expansion.md` (the provider feature matrix — D-16's home),
  `docs/src/contributing/contributing-providers.md` (adapter requirements),
  `docs/src/user-guides/herald-output.md`, `output-formatting.md`, `battalion-patterns.md`,
  `paladin-agents.md`, `agent-orchestrator-bridge.md`, `memory-management.md`,
  `getting-started/quickstart.md`, `operations/observability.md`,
  `appendix/conclave-pattern.md`, `architecture/domain-model.md`,
  `api-reference/upgrading.md`, `api-reference/migration-guide.md`; `docs/book.toml`
  (`warning-policy = "error"`), `.github/workflows/docs.yml` (pinned mdbook versions).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `TokenUsage::new(prompt, completion)` already computes the total — keep it as the one
  constructor and add builders; the `Default` derive makes `..Default::default()` the
  migration path for literals (D-04).
- `llm_conformance_suite!` + `ConformanceFixture` is the ready-made per-adapter harness for
  the ACCT-03 parity case (D-19); mockito servers are already wired per adapter.
- `RecordingPaladinPort::set_output_with_tokens` in `engine/test_support.rs` is the seam for
  the D-30 round-trip test; the "trace and Waypoint token_count must agree" test
  (`engine/mod.rs:~10227`) is the assertion pattern to extend.
- `MockLlmAdapter::with_token_usage_struct` already configures a full `TokenUsage` for the
  non-streaming path; the streaming impls just need to attach it to the terminal chunk.
- `MIGRATION.md` §9.2 rows for `PaladinResult` (constructible), `LlmRequest`
  (`#[non_exhaustive]`) and `ResumeAccepted` (new-in-0.10, N/A) are the three templates every
  new row copies.

### Established Patterns
- Additive persisted fields use `#[serde(default)]` (`NodeExecutionRecord.attempts`,
  `Waypoint.fork_of`, `PaladinResult.served_by`); this phase follows it for every `usage` field.
- Every `match` over the `#[non_exhaustive]` `TraceEvent` in the workspace carries a wildcard
  arm (enforced by the `trace.rs:~437` test) — changing variant fields is a compile-visible
  change only at the sites that destructure `token_count`/`total_tokens` (`hooks.rs`,
  `assertion.rs`, `inspector.rs`, `events.rs`, `engine/mod.rs` tests, `overlay.rs`).
- The `TraceDispatcher` counts run totals synchronously inside `emit` so `RunFinished` is
  exact when it is enqueued — D-11 must preserve that.
- `CompatEngine` already emits a synthetic terminal `Stop` chunk at `[DONE]`; D-14 rides that
  chunk. Anthropic and Gemini streams have no `[DONE]`; their terminal frames
  (`message_stop`, last `candidates[]` frame) are the D-14 terminal chunk.
- Provider usage structs are per-adapter private serde types (`OpenAIUsage`, `ClaudeUsage`,
  `DeepSeekUsage`, `GeminiUsageMetadata`, `CompatUsage`) — the optional cache/reasoning
  sub-objects are added there, never by loosening `TokenUsage`.

### Integration Points
- `LlmResponse.usage` → execution-service loop accumulator → `PaladinResult.usage` →
  Formation/Phalanx `per_paladin_tokens` and the engine's `superstep.rs` Paladin arms →
  `NodeExecutionRecord.usage` (Waypoint, persisted) and `NodeFinished.usage` (trace) →
  `TraceDispatcher` accumulator → `RunFinished.usage` → SSE run events, eval
  `total_tokens_max`, persisting sink, heralds, CLI, web DTOs.
- `StreamingResponse.usage` (terminal chunk) → execution-service stream consumer →
  `ChunkMetadata.usage` on the final `PaladinStreamChunk` → SSE/web streaming responses.
- `TokenBudget` middleware keeps reading `cumulative_tokens: u32` (a total) — the split does
  not change budget enforcement.

</code_context>

<specifics>
## Specific Ideas

- Test figures must be distinct and non-round (`1_234` / `567` / `100` / `50` / `200`, second
  node different) so a swapped or zeroed field cannot pass by coincidence — the house style
  in `table_herald.rs:~381` and `json_herald.rs:~437`.
- The Anthropic captured fixtures already in `anthropic/adapter.rs` tests
  (`THINKING_TEXT_OPUS_5_JSON`: `input_tokens: 85`, `output_tokens: 3000`,
  `thinking_tokens: 2561`) are real data for the D-20 mapping test: expect
  `reasoning_tokens == Some(2561)` and `completion_tokens == 3000`.
- Write "v0.10.0" in every new doc and row, never "v0.11.0" (Phase 30 D-18).
- The provider page's "Streamed usage" cell for each adapter says one of: "yes (usage frame)",
  "yes (event accumulation)", "server-dependent — `None` when omitted" — never "partial".

</specifics>

<deferred>
## Deferred Ideas

- **Estimating usage from `TokenCounterPort` when a provider omits streamed usage** — an
  opt-in for Milestone 14 to consider; never mixed into billed counts here (D-17).
- **Widening `TokenUsage` counters to `u64`** — a separate, deliberate port-level break if a
  real overflow case ever appears; saturating arithmetic covers this phase (D-06).
- **Currency cost, pricing tables, `cost_estimate` producer, allowances, pacing** —
  Milestone 14 Treasurer (`.project/Milestone_14-Treasurer/`), FUT-08/FUT-09.
- **Per-chunk token hints (`ChunkMetadata.tokens`, `StreamChunk.token_count`)** — untouched;
  reconsider only if a consumer needs mid-stream accounting.
- **A `TokenUsage` `Display` impl / shared human formatter** shared by Markdown herald and CLI —
  nice-to-have; Claude's discretion in this phase if it falls out naturally, otherwise later.

### Reviewed Todos (not folded)
- **Evaluate replacing MinIO with RustFS in the dev/test stack**
  (`.planning/todos/2026-09-13-evaluate-rustfs-replacement-for-minio.md`, score 0.6) — a
  keyword-only match (`test`, `crates`, `paladin`); unrelated infrastructure work with no bearing
  on token accounting. Left in the backlog.

</deferred>

---

*Phase: 31-lossless-token-accounting*
*Context gathered: 2026-09-14 via `/gsd-discuss-phase 31 --auto`*
