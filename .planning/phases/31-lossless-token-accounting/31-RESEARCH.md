# Phase 31: Lossless Token Accounting - Research

**Researched:** 2026-09-14
**Domain:** Rust domain-type carrier migration (breaking, clean-break) across a hexagonal
workspace: core value type → battalion/engine aggregation → trace/Waypoint persistence →
streaming LLM adapters → heralds/CLI/HTTP edge.
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Phase Boundary.** Carry the provider's `TokenUsage` prompt/completion split, plus new optional
cache-read/cache-write/reasoning counts, unchanged from `LlmPort` up through `PaladinResult`,
`BattalionResult.per_paladin_tokens`, the Waypoint `NodeExecutionRecord`, `TraceEvent::NodeFinished`
and `TraceEvent::RunFinished`, and out to a herald in both JSON and Markdown. Remove the
`TokenUsage::from_total` zeroing from the battalion aggregation path. Make every LLM adapter's
streaming path report the same `TokenUsage` as its non-streaming path, or document the adapter as
an explicit exception. Register every touched public type in `MIGRATION.md` §9.2 with a
row-level-matched `cargo semver-checks` allowlist entry and a `CHANGELOG.md` `[0.10.0]` entry.

**Breaking, clean break, no shims** — governed by ADR-0051 (X-03 superseded for Phases 31-33
only). No `#[deprecated]` bare-count accessor, no `token_count` compatibility field, no
legacy-shape deserializer. This is the keystone phase: Phase 32 and Milestone 14 (Treasurer)
build on the shipped shape.

**Not in this phase:** currency pricing, `cost_estimate` production, allowances, rate pacing
(Milestone 14); renaming any token type (`TokenUsage`, `TokenBudget`, `TokenCounterPort`,
`max_tokens` all keep their names — Phase 30 D-17); the per-chunk `tokens`/`token_count` hints on
`ChunkMetadata`/`StreamChunk` (approximate chunk sizes, not accounting — untouched);
`TokenCounterPort`/`Commissary` changes (Phase 32); RAG truncation (Phase 33).

**`TokenUsage` shape and `total_tokens` meaning (ACCT-01)**
- D-01: `TokenUsage` gains exactly three fields, each `Option<u32>` with `#[serde(default)]`:
  `cache_read_tokens`, `cache_write_tokens`, `reasoning_tokens`, in that order after the three
  existing counters. No other field added or removed.
- D-02: `total_tokens` INCLUDES cache and reasoning tokens. Invariant:
  `total_tokens == prompt_tokens + completion_tokens`; the three optionals are "of which"
  sub-counts already inside those two numbers, never additive on top —
  `cache_read_tokens + cache_write_tokens <= prompt_tokens` and
  `reasoning_tokens <= completion_tokens` always hold. Rustdoc states this in one sentence plus
  the two inequalities.
- D-03: `None` = provider did not report the figure; `Some(0)` = provider reported zero. Adapters
  never fabricate `Some(0)`.
- D-04: `TokenUsage` stays constructible with `Default` (X-10.3 option (b)) — NOT
  `#[non_exhaustive]` (it has `Default`; functional-update is disallowed cross-crate on a
  `#[non_exhaustive]` struct). `new(prompt, completion)` keeps its signature, optionals stay
  `None`; add three chainable builders `with_cache_read(u32)`, `with_cache_write(u32)`,
  `with_reasoning(u32)`. Every in-tree full struct literal migrates to `new(..)` + builders or
  `..Default::default()` in the same commit. Suppression: existing
  `constructible_struct_adds_field = "allow"` in `crates/paladin-core/Cargo.toml`, mirrored by a
  new `.cargo/semver-checks-allowlist.toml` entry `paladin-ai-core | TokenUsage`.
- D-05: `TokenUsage::from_total` is DELETED outright (not just removed from the battalion path).
  Only in-tree callers are the two battalion services and test fixtures — all move to
  `TokenUsage::new`.
- D-06: Accumulation lives on the type: `impl Add`, `impl AddAssign`, `impl Sum` for `TokenUsage`,
  SATURATING on every `u32`, `Option` merge rule `None+None=None`, `None+Some(x)=Some(x)`,
  `Some(a)+Some(b)=Some(a.saturating_add(b))`; `total_tokens` recomputed as `prompt+completion`
  after the add. Every accumulator in the tree (execution-service loop, Formation/Phalanx maps,
  the engine's `TraceDispatcher` run total, eval runner) uses this one implementation. Counters
  stay `u32` (widening to `u64` is out of scope); the trace aggregate that was `u64` becomes a
  saturating `u32` inside `TokenUsage`, documented on the rustdoc.

**Carrier shape and naming (ACCT-02)**
- D-07: The carrier field is named `usage` everywhere, matching `LlmResponse.usage`:
  `PaladinResult.usage: TokenUsage` (replaces `token_count: u32`), `NodeExecutionRecord.usage:
  TokenUsage` (replaces `token_count: u64`), `TraceEvent::NodeFinished { usage: TokenUsage, .. }`
  (replaces `token_count: u64`), `TraceEvent::RunFinished { usage: TokenUsage, .. }` (replaces
  `total_tokens: u64`). `ExecutionMetadata.token_usage` (herald.rs) KEEPS its existing name — it
  is already a full `TokenUsage`.
- D-08: Bare-count rule: a bare total may coexist beside a full `TokenUsage` at the same level as
  a derived convenience; it may never be the ONLY token carrier at a level. `BattalionResult.
  total_tokens: u64` STAYS (split lives in `per_paladin_tokens: HashMap<String, TokenUsage>` —
  only its VALUES change from zeroed to real); `PaladinResult`, `NodeExecutionRecord`,
  `NodeFinished`, `RunFinished` (whose bare count was their ONLY carrier) are replaced.
- D-09: `PaladinResult` stays constructible, NOT `#[non_exhaustive]` (Phase 25 D-26 unchanged);
  `PaladinResult::new(output, usage: TokenUsage, execution_time_ms, loop_count, stop_reason)`
  takes usage in the `token_count` position. No `token_count()`/`total()`/`total_tokens()`
  accessor added — `result.usage.total_tokens` needs no convenience method.
- D-10: Formation (`formation_service.rs:~231`) and Phalanx (`phalanx_service.rs:~279`) insert
  `result.usage.clone()` into `per_paladin_tokens` and add `u64::from(result.usage.total_tokens)`
  to `total_tokens`. Chain-of-Command/Campaign/Conclave/Council/Grove/Maneuver/Commander audited
  for any other place a `PaladinResult` count is copied into a `TokenUsage`; none may construct a
  zero-split usage.
- D-11: Engine: `superstep.rs`'s two Paladin arms (structured `~1095`, scoped `~1140`) thread
  `result.usage` (a `TokenUsage`, not `u64::from(result.token_count)`) through the `(paladin_id,
  usage, outcome)` tuple into `NodeExecutionRecord` and `NodeFinished`; a cache hit records
  `TokenUsage::default()` (was `0`), keeps `cache_hit: true`. `hooks.rs`'s
  `TraceDispatcher.token_total: AtomicU64` becomes a synchronously-updated `TokenUsage`
  accumulator (`std::sync::Mutex<TokenUsage>` held for the duration of one `+=`, OR five atomics —
  planner's choice) so `RunFinished.usage` is exact the instant `emit` returns. `RunFinished` is
  emitted at five sites in `engine/mod.rs` (`~2081-2084`, `~2195-2198`, `~2314-2317`,
  `~2668-2671`, `~2837-2840`) — all five switch together.
- D-12: The execution service's reasoning loop (`paladin_execution_service.rs` `~1257`, `~1501`)
  accumulates `usage += response.usage` per iteration, sets `middleware_cx.cumulative_tokens =
  usage.total_tokens` (`TokenBudget` middleware keeps total-based semantics unchanged, VOCAB-05 /
  Phase 30 D-17). All six `PaladinResult` construction sites carry the accumulated `usage`.

**Streaming usage parity (ACCT-03)**
- D-13: `StreamingResponse` gains `usage: Option<TokenUsage>` with `#[serde(default)]`, marked
  `#[non_exhaustive]` (X-10.3 option (a) — no `Default`, so nothing to lose). Add
  `StreamingResponse::delta(text)`, `::terminal(finish_reason)`, chainable `with_usage(TokenUsage)`.
  Suppression: existing `struct_marked_non_exhaustive = "allow"` in `crates/paladin-ports/
  Cargo.toml`, mirrored by a new allowlist entry `paladin-ports | StreamingResponse`.
- D-14: Terminal-chunk contract: `usage` is `Some` on EXACTLY ONE chunk per stream — the chunk
  carrying `finish_reason: Some(..)` — and `None` on every other chunk. Adapters guarantee this
  even when the usage arrives AFTER the finish frame: OpenAI-family `stream_options:
  {"include_usage": true}` usage frame arrives after the `finish_reason` frame and before
  `[DONE]`, so `CompatEngine::generate_stream`, `openai/adapter.rs`'s own
  `make_streaming_request`, and `deepseek/adapter.rs` HOLD the finish reason seen on a
  `choices[].finish_reason` frame and emit it on the `[DONE]` terminal chunk together with the
  captured usage (they already emit a `Stop` chunk at `[DONE]`; the change is to carry the real
  finish reason and usage on it, stop emitting `finish_reason` on the earlier frame). The
  execution service's stream consumer (`if is_final { return; }`) keeps stopping at the first
  finished chunk.
- D-15: Per-provider mechanism (verify against current docs, don't trust memory — done below):
  OpenAI/CompatEngine presets (openai_compatible, grok, kimi, qwen, ollama)/DeepSeek send
  `stream_options: {"include_usage": true}`, parse `usage` on the frame whose `choices` is empty;
  `CompatUsage`/OpenAI/DeepSeek usage structs gain optional `prompt_tokens_details.cached_tokens`
  and `completion_tokens_details.reasoning_tokens` (DeepSeek additionally
  `prompt_cache_hit_tokens`). Anthropic: `message_start.message.usage` carries `input_tokens` +
  `cache_read_input_tokens` + `cache_creation_input_tokens`; `message_delta.usage` carries
  cumulative `output_tokens` (and, per captured fixtures, `output_tokens_details.thinking_tokens`);
  accumulate across events, attach the final `TokenUsage` on the `message_stop` terminal chunk.
  Gemini: every SSE frame's `usageMetadata` is cumulative; the LAST frame's value (arrives with the
  terminal `finishReason`) attaches to the terminal chunk; `cachedContentTokenCount` →
  `cache_read_tokens`, `thoughtsTokenCount` → `reasoning_tokens`, `completion_tokens =
  candidatesTokenCount + thoughtsTokenCount` (D-02). Mock: scripted stream ends with a terminal
  chunk carrying the configured `token_usage`. `FallbackLlmAdapter` passes chunks through
  unchanged.
- D-16: Explicit exception: the generic `OpenAiCompatibleAdapter` (and any preset whose server
  ignores `stream_options`) cannot guarantee streamed usage — terminal chunk carries `usage: None`
  when the server omits the frame. Documented in the adapter's rustdoc AND the mdBook provider
  page (`docs/src/appendix/provider-expansion.md`, add a "Streamed usage" row/column;
  `contributing-providers.md` gains the terminal-chunk contract as a new-adapter requirement).
  Ollama's native `/api/chat` shape documented the same way IF the adapter's engine does not
  receive an OpenAI-shaped usage frame.
- D-17: No estimation fallback. A streamed call ending with `usage: None` records
  `TokenUsage::default()` and logs one `warn!` naming the provider — never substitutes a
  `TokenCounterPort` estimate for a billed count.
- D-18: `ChunkMetadata` (`paladin_port.rs`, the `PaladinStreamChunk.metadata` payload) gains
  `usage: Option<TokenUsage>` (`#[serde(default)]`), populated only on the `is_final` chunk from
  the provider's terminal chunk; existing per-chunk `tokens: Option<u32>` hint untouched. X-10.3
  handling for `ChunkMetadata` follows whichever of (a)/(b) its `Default` status dictates (it has
  NO `Default` today — verified — so option (a), `#[non_exhaustive]`, applies, mirroring D-13). SSE
  run-stream and `paladin-web` streaming responses forward it.
- D-19: Parity test = one shared conformance case
  (`streaming_usage_equals_non_streaming_usage`) in `crates/paladin-llm/src/conformance.rs`'s
  `llm_conformance_suite!`: opens `success_body()` through `generate()` and `stream_body()`
  through `generate_stream()` against the same mockito server, asserts the terminal chunk's
  `usage` equals the non-streaming `LlmResponse.usage` field-for-field (including the three
  optionals). `ConformanceFixture::stream_body()`'s contract extended: must carry the same usage
  figures as `success_body()`. Today ONLY `ollama`, `gemini`, `openai_compatible` instantiate the
  suite (verified) — `openai`, `anthropic`, `deepseek`, `grok`, `kimi`, `qwen`, and the mock must
  be covered too (instantiate the suite where cheap; a dedicated `#[tokio::test]` where the wire
  shape differs, e.g. Anthropic's event stream). `CASE_COUNT` assertion updates from 8 to 9. A
  second, execution-service-level test proves a Paladin run via `execute_stream` ends with the
  same `usage` as `execute` against the mock.

**Adapter population of cache/reasoning (ACCT-03)**
- D-20: Adapters populate the optionals on BOTH paths wherever the provider's existing usage
  object carries the figure, mapping to D-02's inclusive semantics. OpenAI
  `prompt_tokens_details.cached_tokens` → `cache_read_tokens`,
  `completion_tokens_details.reasoning_tokens` → `reasoning_tokens`; Anthropic
  `cache_read_input_tokens` → `cache_read_tokens`, `cache_creation_input_tokens` →
  `cache_write_tokens`, `output_tokens_details.thinking_tokens` → `reasoning_tokens`, and
  **`prompt_tokens = input_tokens + cache_read_input_tokens + cache_creation_input_tokens`**
  (Anthropic's `input_tokens` EXCLUDES cached tokens, so today's `prompt_tokens` under-reports
  billed input whenever caching is on — call this out in the CHANGELOG); Gemini as in D-15;
  DeepSeek `prompt_cache_hit_tokens` → `cache_read_tokens`,
  `completion_tokens_details.reasoning_tokens` → `reasoning_tokens`; `CompatEngine` presets parse
  the OpenAI-shaped `*_details` objects when present. Ollama native fields and any provider
  without the figure → `None`. Vision adapters (`VisionTokenUsage`) untouched.

**Herald observability (ACCT-04)**
- D-21: `JsonHerald` emits the full `TokenUsage` object under `"usage"` for a `PaladinResult`
  (replacing `"token_count"`), serializes `per_paladin_tokens` values as full objects (already the
  case — now with real splits), emits `ExecutionMetadata.token_usage` as the full object (today
  `json_herald.rs`'s `finalize_stream` emits only `total_tokens`). JSON ALWAYS emits the three
  optional keys, as `null` when `None` — no `skip_serializing_if` — so JSON consumers see a stable
  key set; deserialization tolerates absence via `#[serde(default)]`.
- D-22: `MarkdownHerald` replaces the single "Token Count" field with a "Token Usage" block:
  `Prompt`, `Completion`, `Total` always; `Cache read`, `Cache write`, `Reasoning` rows only when
  `Some`. For a `BattalionResult` add a per-Paladin usage table (name | prompt | completion |
  total | cache read | cache write | reasoning) under the existing "Total Tokens" summary.
- D-23: `TableHerald` is Claude's discretion (see below).

**Edge surfaces: HTTP API, SSE, inspector, dev UI**
- D-24: The HTTP edge carries the full object too. `paladin-web` defines a `TokenUsageResponse`
  DTO (`utoipa::ToSchema`, `From<TokenUsage>`, same six fields) — `paladin-core` gains no
  `utoipa` dependency — and: `ExecuteResponse` (`agent_controller.rs`) replaces `token_count: u32`
  with `usage: TokenUsageResponse`; the run inspector's `CompletedRow` (verified location:
  `crates/paladin-ports/src/input/run_inspector_port.rs:72`, NOT `src/application/services/run/
  inspector.rs:~295` as CONTEXT.md's approximate citation reads) replaces `token_count:
  Option<u64>` with `usage: Option<TokenUsageResponse>` (still `None` on a cache hit); the dev-UI
  row (`dev_ui_controller.rs`) follows. SSE run events serialize `TraceEvent` directly, so
  `node_finished`/`run_finished` payloads change with D-07. `crates/paladin-web/openapi.json` is
  regenerated with `make openapi` in the same commit; change recorded in `MIGRATION.md` §9.6. The
  Python-client generation job in `ci.yml` must stay green on the regenerated document.

**Compatibility mechanics (ACCT-05)**
- D-25: Serde policy: every new `usage` field is `#[serde(default)]`; retired bare-count keys
  (`token_count`, `total_tokens` on the trace event) are simply dropped — serde ignores unknown
  keys. NO legacy-shape deserializer maps an old `token_count` into `usage.total_tokens`:
  Waypoints/trace records are new in the untagged v0.10.0, so pre-Phase-31 rows are only developer
  databases; a `PaladinResult` JSON written by 0.9.x loads with `usage == TokenUsage::default()`.
  Stated in a `MIGRATION.md` §9.4 note. Existing `execution_result.rs` legacy-JSON test is
  rewritten to assert exactly that.
- D-26: X-10.3 choice per touched type (record reason on each §9.2 row): `TokenUsage`
  constructible (D-04); `PaladinResult` constructible (D-09); `StreamingResponse`
  `#[non_exhaustive]` (D-13); `ChunkMetadata` per D-18; `TraceEvent` (already
  `#[non_exhaustive]`), `NodeExecutionRecord`, and `CompletedRow` (verified: new-in-0.10,
  `paladin-ports`, Phase 28 OBS-03 — same treatment) are new-in-0.10 types → rows "listed for
  completeness — N/A, not in the 0.9.0 baseline", NO allowlist entry, like the
  `ResumeAccepted`/`ThreadApiState` rows; `ExecuteResponse` (shipped 0.9.0) gets a `paladin-web`
  row and entry; `BattalionResult` needs no row (no signature change — only `per_paladin_tokens`
  values change; note in the `TokenUsage` row's Change cell).
- D-27: Lint discovery is EMPIRICAL, not guessed: after the code lands, run `cargo semver-checks
  check-release --package <pkg> --default-features --baseline-version 0.9.0` for
  `paladin-ai-core`, `paladin-ports`, `paladin-web` and write one `[[entry]]` per `(crate, lint,
  migration_row)` that actually fires. Expected but unconfirmed: `constructible_struct_adds_field`
  and `inherent_method_missing` (the deleted `from_total`) on `paladin-ai-core | TokenUsage`;
  `struct_pub_field_missing` + `constructible_struct_adds_field` on `paladin-ai-core |
  PaladinResult`; `struct_marked_non_exhaustive` (+ possibly `constructible_struct_adds_field`) on
  `paladin-ports | StreamingResponse`; `struct_pub_field_missing` on `paladin-web |
  ExecuteResponse`. The Phase 29 D-04 row-level set-equality step is the gate.
- D-28: `CHANGELOG.md` entries go in the `[0.10.0]` section (verified: this section already exists
  in the file, dated 2026-09-10, still open since the tag is not yet cut) under a `### Changed`
  bullet for the carrier change and a `### Fixed` bullet naming the two corrected under-reports
  (battalion per-Paladin split zeroed by `from_total`; Anthropic `prompt_tokens` excluding cached
  input). The `[Unreleased]` Commissary re-export bullet stays untouched.
  `docs/src/api-reference/upgrading.md`/`migration-guide.md` gain a short "Token usage carriers"
  subsection pointing at §9.2.
- D-29: Rustdoc and mdBook pages that show `token_count` (grep hit list in CONTEXT.md) updated in
  the SAME plan that changes the type, so `cargo test --doc` and the mdBook link/warning-policy
  stay green.

**Test strategy and gates**
- D-30: Round-trip test (ACCT-02): one engine-level test in `crates/paladin-battalion` (next to
  `paladin_node_execution_record_carries_reported_token_count`) scripts a Paladin whose
  `RecordingPaladinPort` (`set_output_with_tokens` grows a `set_output_with_usage(name, output,
  TokenUsage)`) returns `TokenUsage::new(1_234, 567).with_cache_read(100).with_cache_write(50).
  with_reasoning(200)` and asserts the identical value on `NodeExecutionRecord`, `NodeFinished`,
  and `RunFinished.usage` (summed across two nodes with distinct non-round figures — the
  "trace-and-Waypoint-must-agree" test is the pattern). A second test in `formation_service.rs`
  and `phalanx_service.rs` asserts `per_paladin_tokens[name]` equals the Paladin's full usage
  (regression for the `from_total` bug). A serde test in `token_usage.rs` covers
  legacy-JSON-without-optionals and new-JSON round-trip.
- D-31: Gates before sealing: `make clean-code`, `cargo test` (doctests included — `cargo
  llvm-cov` skips them), the 82% workspace line-coverage floor, `make security`, `cargo doc`
  zero-warning, `mdbook build docs/`, the `semver` job's per-package run plus the row-level
  allowlist check, the OpenAPI baseline test after `make openapi`, and the Python-client
  generation job. The folded coverage todo is satisfied by noting whether local `make coverage`
  reproduces the CI figure.

### Claude's Discretion
- Exact rustdoc wording for D-02; whether the D-06 `Option` merge lives in a private helper or
  inline; `Mutex` vs atomics in `TraceDispatcher` (D-11).
- Constructor names on `StreamingResponse` (D-13) — must be doc-tested.
- Whether `TableHerald` gains split columns (D-23); exact CLI text.
- Whether `TokenUsageResponse` lives in `agent_controller.rs` or a small shared `dto` module.
- Plan/commit granularity — natural waves: (1) `TokenUsage` fields + arithmetic + `from_total`
  removal + serde tests; (2) carriers (`PaladinResult`, battalion services, engine, trace,
  execution service, eval runner, CLI, web) as one coordinated break; (3) streaming contract +
  adapters + conformance; (4) heralds + docs; (5) MIGRATION/allowlist/CHANGELOG/OpenAPI + gate
  evidence. Commit with `git commit` directly (pre-commit hook runs workspace clippy; the GSD
  commit helper is known to time out — Phase 30 note).

### Folded Todos
- Verify local `make coverage` reproduces CI's 82.39% figure — folded as a verification note
  only; no new scope.

### Deferred Ideas (OUT OF SCOPE)
- Estimating usage from `TokenCounterPort` when a provider omits streamed usage — a Milestone 14
  opt-in to consider; never mixed into billed counts here (D-17).
- Widening `TokenUsage` counters to `u64` — a separate, deliberate port-level break if a real
  overflow case appears; saturating arithmetic covers this phase (D-06).
- Currency cost, pricing tables, `cost_estimate` producer, allowances, pacing — Milestone 14
  Treasurer, FUT-08/FUT-09.
- Per-chunk token hints (`ChunkMetadata.tokens`, `StreamChunk.token_count`) — untouched.
- A `TokenUsage` `Display` impl / shared human formatter shared by Markdown herald and CLI — Claude's
  discretion if it falls out naturally, otherwise later.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ACCT-01 | `TokenUsage` gains three `#[serde(default)]` optionals; rustdoc states total-inclusion; legacy/new JSON round-trip | Standard Stack + Architecture Pattern 1 (verified current `token_usage.rs` shape, exact field order, existing tests to extend) |
| ACCT-02 | `PaladinResult`, `BattalionResult.per_paladin_tokens`, `NodeExecutionRecord`, `NodeFinished`, `RunFinished` carry full `TokenUsage`; `from_total` removed from battalion path; round-trip test | Architecture Pattern 2 (verified every carrier's current field, exact line numbers, the closure-tuple threading pattern in `superstep.rs` that must change) |
| ACCT-03 | Every adapter's `execute_stream` audited; parity test or documented exception | Architecture Pattern 3 + provider-mechanism verification (Common Pitfalls; Code Examples) — every adapter's current streaming code read directly, provider docs verified via WebSearch this session |
| ACCT-04 | Breakdown observable in JSON and Markdown herald | Architecture Pattern 4 (verified current `json_herald.rs`/`markdown_herald.rs` output shape) |
| ACCT-05 | Every touched type has a §9.2 row + allowlist row; CHANGELOG `[0.10.0]`; `make clean-code` + coverage floor green | Package Legitimacy N/A + Validation Architecture + verified `MIGRATION.md`/`ci.yml`/`.cargo/semver-checks-allowlist.toml` mechanics |

</phase_requirements>

## Summary

This is a keystone, single-domain carrier-migration phase: one value type (`TokenUsage`) gains
three optional fields and loses one lossy constructor, and that change ripples through every layer
of the stack that currently stores or forwards a token count as a bare `u32`/`u64` instead of the
full struct. The domain logic itself is trivial (three new `Option<u32>` fields, arithmetic that
already exists in spirit); the actual engineering weight is in the number of call sites that must
migrate together in one commit (a genuinely wide, mechanical "coordinated break" per CONTEXT.md's
own wave structure), and in getting five different LLM providers' streaming wire formats right —
each with a distinct usage-frame timing and shape that this session verified against current
provider documentation rather than trusting training-data memory.

Every code location CONTEXT.md's `<canonical_refs>` names was read directly in this session. Two
material corrections surfaced: (1) `ChunkMetadata` (`paladin_port.rs`) has **no** `#[derive(Default)]`
today — confirmed by direct inspection — so D-18's X-10.3 choice resolves to option (a),
`#[non_exhaustive]`, the same as `StreamingResponse`; (2) `CompletedRow`'s real definition lives at
`crates/paladin-ports/src/input/run_inspector_port.rs:72`, not `src/application/services/run/
inspector.rs:~295` as CONTEXT.md's approximate citation states — and it is itself a new-in-0.10
type (Phase 28, OBS-03), so it takes the same "N/A, no allowlist entry" treatment as
`NodeExecutionRecord` under D-26, not a fresh judgment call.

Provider streaming semantics were verified this session (WebSearch, current documentation and
community sources — Context7 was unavailable in this environment despite being listed; all
provider claims below are tagged `[CITED]`, not `[VERIFIED]`, and should be spot-checked against
the exact wire fixtures if any adapter's live-integration test starts failing). The single most
important cross-cutting fact: **OpenAI-family (and OpenAI-compatible, DeepSeek, xAI Grok, Moonshot
Kimi, Alibaba DashScope/Qwen compatible-mode, and Ollama's `/v1/chat/completions`) ALL honor
`stream_options: {"include_usage": true}`** — this is not a five-way fragmented landscape, it is
one shared mechanism (`CompatEngine` + `openai/adapter.rs`'s own client) plus two bespoke shapes
(Anthropic's event-accumulation, Gemini's cumulative-per-frame). This corrects an implicit
assumption in CONTEXT.md D-16 that Ollama or the generic OpenAI-compatible preset are likely
streamed-usage exceptions — Ollama's own OpenAI-compat documentation lists `stream_options` as
explicitly supported; the "explicit exception" in D-16 should be scoped to a *third-party*
OpenAI-compatible server that does not implement `stream_options` at all (which the generic
`OpenAiCompatibleAdapter` cannot know in advance), not to Ollama specifically.

**Primary recommendation:** Execute the five waves CONTEXT.md's own Claude's Discretion note
already lays out, in that order, because each wave's tests gate the next: (1) `TokenUsage` type +
arithmetic + `from_total` removal + serde tests (touches nothing else, compiles standalone once
every in-tree literal is migrated); (2) carriers as one coordinated break (the widest wave —
`PaladinResult`, battalion aggregation, the engine's closure-tuple threading in `superstep.rs`, the
`TraceDispatcher` accumulator, the execution service's loop and stream consumer, eval, CLI, web
DTOs); (3) streaming contract + all seven remaining adapters' usage-frame parsing + the
conformance-suite parity case; (4) heralds + docs; (5) MIGRATION.md/allowlist/CHANGELOG/OpenAPI +
gate evidence, with the semver-checks lint IDs discovered empirically per D-27, not guessed ahead
of time.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `TokenUsage` value type + arithmetic | Core domain (`paladin-core`) | — | Pure value type, zero I/O, the single definition every other tier re-exports |
| Provider usage-frame parsing (cache/reasoning mapping) | Infrastructure adapter (`paladin-llm`) | — | Each provider's wire shape is a private, per-adapter serde struct; the mapping to `TokenUsage`'s inclusive semantics is adapter-owned translation, never loosened onto the core type |
| Battalion/engine aggregation (`per_paladin_tokens`, `NodeExecutionRecord`, `TraceDispatcher`) | Application/orchestration (`paladin-battalion`) | Core domain (carries the type) | Formation/Phalanx/engine own the SUM semantics (D-06's `Add`/`AddAssign`); the type itself only defines how two usages combine |
| Persisted Waypoint/trace record shape | Core domain (schema) | Storage adapter (serialization only) | `NodeExecutionRecord`/`TraceEvent` are `paladin-core` types; `paladin-storage`'s SQLite/Postgres adapters serialize them unchanged — no adapter-level schema migration needed since these are JSON-blob columns, not typed SQL columns (verify per D-25/§9.4) |
| Streaming usage-frame timing/parity contract | Application layer (port contract) + Infrastructure (per-adapter enforcement) | — | `StreamingResponse`/`ChunkMetadata`'s terminal-chunk contract is a port-level (`paladin-ports`) invariant; every adapter in `paladin-llm` is independently responsible for honoring it |
| Herald/CLI/HTTP-edge observability | Presentation (`paladin-herald`, facade CLI, `paladin-web`) | — | Pure read-and-render of the carrier the lower tiers now populate; no new business logic |
| `MIGRATION.md`/semver-checks/CHANGELOG bookkeeping | Program governance (repo-root, CI) | — | Cross-cutting process artifact, not a runtime tier |

## Package Legitimacy Audit

Not applicable — this phase adds no new external dependency to any `Cargo.toml`. Every change is
to first-party types and existing adapter wire-parsing logic (new optional fields on already-vendored
provider response/request serde structs). `utoipa` (for the new `TokenUsageResponse` DTO, D-24) is
already a dependency of `paladin-web` — confirmed by `ExecuteResponse`'s existing
`#[derive(... utoipa::ToSchema)]` in `crates/paladin-web/src/agent_controller.rs`.

## Standard Stack

No new libraries. This phase uses only the already-pinned toolchain:

| Tool | Version (pinned) | Purpose |
|------|------|---------|
| `cargo-semver-checks` | 0.50.0 `[VERIFIED: .github/workflows/ci.yml semver job]` | ACCT-05's empirical lint-discovery step (D-27) |
| `serde`/`serde_json` | workspace-pinned (unchanged) | `#[serde(default)]` optionals on `TokenUsage`, `StreamingResponse`, `ChunkMetadata` |
| `utoipa` | already a `paladin-web` dependency `[VERIFIED: agent_controller.rs ExecuteResponse derive]` | The new `TokenUsageResponse` DTO (D-24) |
| `mockito` | already a `paladin-llm` dev-dependency (used throughout `conformance.rs` and every adapter's test module) | The ACCT-03 parity conformance case and per-adapter streaming tests |
| `cargo llvm-cov` | project-standard, invoked via `Makefile`/`ci.yml` `coverage` job | ACCT-05's 82% floor — doctests are excluded from this tool (project memory: llvm-cov and `--tests` both skip doctests) |

**Installation:** none required.

**Version verification:** No new package versions to verify — every touched type lives in a crate
already at its current workspace version; no `Cargo.toml` dependency line changes.

## Architecture Patterns

### System Architecture Diagram

```
 Provider wire response (OpenAI/Anthropic/Gemini/DeepSeek/Grok/Kimi/Qwen/Ollama/Mock)
        │  non-streaming: one JSON body with a usage object
        │  streaming: SSE frames; usage arrives on ONE frame (D-14 terminal-chunk contract)
        ▼
 ┌────────────────────────────────────────────────────────────────────────┐
 │ paladin-llm adapters (openai/anthropic/gemini/deepseek/compat::engine)  │
 │  private per-provider *Usage structs map onto TokenUsage per D-20:     │
 │  cache_read_tokens / cache_write_tokens / reasoning_tokens ("of which") │
 └───────────────┬───────────────────────────────┬────────────────────────┘
                 │ generate() -> LlmResponse.usage │ generate_stream() -> terminal
                 │ (unchanged carrier, richer type) │ StreamingResponse.usage (NEW, D-13)
                 ▼                                 ▼
 ┌───────────────────────────────┐   ┌────────────────────────────────────┐
 │ PaladinExecutionService        │   │ execute_stream_inner's forwarding   │
 │  loop accumulator: usage +=    │   │ task: attaches ChunkMetadata.usage  │
 │  response.usage (D-06 Add)     │   │ (NEW, D-18) on the is_final chunk   │
 │  -> PaladinResult.usage (D-09) │   └──────────────┬───────────────────────┘
 └───────────────┬─────────────────────────────────┘
                 │ result.usage: TokenUsage (was token_count: u32)
                 ▼
 ┌──────────────────────────────────────────────────────────────────────────┐
 │ Formation/Phalanx (D-10): per_paladin_tokens[name] = result.usage.clone() │
 │ Engine superstep.rs Paladin arms (D-11): usage threaded through the       │
 │ dispatch closure's (paladin_id, usage, outcome) tuple                    │
 └───────────────┬─────────────────────────────┬─────────────────────────────┘
                 │                              │
                 ▼                              ▼
 ┌───────────────────────────┐   ┌────────────────────────────────────────┐
 │ NodeExecutionRecord.usage │   │ TraceEvent::NodeFinished { usage, .. }  │
 │ (Waypoint, persisted)     │   │  -> TraceDispatcher accumulator (D-11)  │
 └───────────────────────────┘   │  -> TraceEvent::RunFinished { usage }   │
                                 └──────────────┬───────────────────────────┘
                                                │
                    ┌───────────────────────────┼───────────────────────────┐
                    ▼                           ▼                           ▼
        ┌───────────────────┐      ┌─────────────────────┐      ┌───────────────────┐
        │ JsonHerald/        │      │ SSE run events       │      │ eval assertion.rs  │
        │ MarkdownHerald     │      │ (TraceEvent直接serialize) │  │ total_tokens_max    │
        │ (D-21/D-22)        │      │ -> paladin-web DTOs   │      │ reads usage.total_  │
        └───────────────────┘      │ (TokenUsageResponse,   │      │ tokens (was u64)     │
                                    │ D-24)                 │      └───────────────────┘
                                    └─────────────────────┘
```

### Recommended Wave Structure (matches CONTEXT.md's own Claude's Discretion ordering)

```
Wave 1 — TokenUsage type (ACCT-01)
  - crates/paladin-core/src/platform/container/token_usage.rs:
      + cache_read_tokens/cache_write_tokens/reasoning_tokens: Option<u32> (#[serde(default)])
      + with_cache_read/with_cache_write/with_reasoning builders
      + impl Add, AddAssign, Sum (saturating, Option-merge per D-06)
      - delete from_total (+ its one test)
      rustdoc: state the two inequalities + total-inclusion invariant
  - Migrate every in-tree TokenUsage { .. } / from_total(..) literal in the SAME commit:
      openai/adapter.rs, anthropic/adapter.rs, deepseek/adapter.rs, gemini/adapter.rs (via
      GeminiUsageMetadata mapping), compat/engine.rs, mock.rs (both impls),
      formation_service.rs, phalanx_service.rs, herald tests, core tests
  - New serde round-trip tests: legacy-JSON-without-optionals, new-JSON-with-optionals

Wave 2 — Carriers, one coordinated break (ACCT-02)
  - execution_result.rs: PaladinResult.token_count -> usage: TokenUsage; PaladinResult::new
    signature; Default impl; the two legacy-JSON tests rewritten per D-25
  - battalion/mod.rs: TokenUsage re-export unaffected; BattalionResult unchanged signature
  - formation_service.rs (~231), phalanx_service.rs (~279): from_total(..) -> result.usage.clone()
  - waypoint.rs: NodeExecutionRecord.token_count: u64 -> usage: TokenUsage
  - trace.rs: NodeFinished.token_count -> usage; RunFinished.total_tokens -> usage; update the
    "all twelve variants construct" test and the flat-record serialization test
  - engine/superstep.rs: the dispatch closure's (paladin_id, token_count: u64, ..) tuples become
    (paladin_id, usage: TokenUsage, ..) at every site listed in canonical_refs (~1095, ~1140,
    ~1556, ~2555-2996, ~3090-3302); every NodeExecutionRecord{..} / NodeFinished{..} literal
    updated in lockstep; a cache hit uses TokenUsage::default()
  - engine/hooks.rs: TraceDispatcher.token_total: AtomicU64 -> Mutex<TokenUsage> (or five atomics);
    emit()'s NodeFinished match arm accumulates via TokenUsage::AddAssign; token_total() ->
    total_usage() (or equivalent) feeding RunFinished.usage
  - engine/mod.rs: five RunFinished{..} construction sites (~2081-2084, ~2195-2198, ~2314-2317,
    ~2668-2671, ~2837-2840) switch total_tokens: trace.token_total() -> usage: trace.total_usage()
  - engine/test_support.rs: RecordingPaladinPort gains set_output_with_usage(name, output,
    TokenUsage); its execute() builds PaladinResult { usage, .. }
  - paladin_execution_service.rs: total_tokens: u32 accumulator (~1257) -> usage: TokenUsage;
    the two total_tokens += response.usage.total_tokens sites (~1501) -> usage += response.usage;
    middleware_cx.cumulative_tokens = usage.total_tokens (unchanged semantics, D-12); all six
    PaladinResult{..} construction sites (~1216 vision, ~1468, ~1533, ~1814, ~1841, ~3065) carry usage
  - handoff_service.rs (~522, log line), paladin_processor.rs (~120,
    base_metadata(&output, result.usage.total_tokens)): read sites updated
  - crates/paladin-eval/src/assertion.rs: total_tokens_max reads usage.total_tokens instead of
    the bare total_tokens field; RunFinishedInfo struct field renamed
  - crates/doc-examples/src/bridge.rs: any token_count example updated
  - CLI/table_herald.rs's name_pool matching (currently matches on paladin_result.token_count) —
    must be re-derived to match on usage.total_tokens or restructured (see Pitfall 3 below)
  - Round-trip tests (D-30): engine-level (NodeExecutionRecord/NodeFinished/RunFinished agree),
    Formation/Phalanx per-Paladin regression test

Wave 3 — Streaming contract + adapters + conformance (ACCT-03)
  - llm_port.rs: StreamingResponse gains usage: Option<TokenUsage>, #[non_exhaustive];
    add delta()/terminal()/with_usage() constructors; migrate every in-tree literal (every
    adapter currently builds StreamingResponse { id, delta, finish_reason } directly)
  - paladin_port.rs: ChunkMetadata gains usage: Option<TokenUsage>, #[non_exhaustive]
    (confirmed: no Default today -> X-10.3 option (a))
  - compat/types.rs: CompatRequest gains stream_options: Option<CompatStreamOptions> (new
    private struct { include_usage: bool }), sent only when request.stream (i.e. only on
    generate_stream's built request, or unconditionally with skip_serializing_if since it's
    inert non-streaming); CompatStreamResponse gains an optional usage: Option<CompatUsage>
    field (present only on the empty-choices frame) and optional cache/reasoning detail
    sub-objects on CompatUsage
  - compat/engine.rs generate_stream: hold the finish_reason seen on a choices[].finish_reason
    frame in a local variable; do NOT emit it on that frame; on [DONE], emit ONE terminal
    StreamingResponse carrying the held finish_reason AND the usage captured from the
    empty-choices frame (D-14)
  - openai/adapter.rs: same stream_options + usage-frame handling in its own
    make_streaming_request (does NOT go through CompatEngine); OpenAIUsage gains the two
    *_details optionals
  - deepseek/adapter.rs: same pattern in its own generate_stream; DeepSeekUsage gains
    prompt_cache_hit_tokens + completion_tokens_details.reasoning_tokens
  - anthropic/adapter.rs: ClaudeStreamEvent grows fields for message_start's usage object and
    message_delta's usage object; ClaudeUsage (non-streaming) gains the four new fields and the
    parse_response prompt_tokens correction (input_tokens + cache_read + cache_creation);
    generate_stream accumulates across message_start/message_delta/message_stop and attaches
    the final TokenUsage on message_stop
  - gemini/adapter.rs: GeminiUsageMetadata gains cached_content_token_count/thoughts_token_count;
    parse_sse_chunk needs the LAST frame's usage_metadata (the one with a finish_reason) attached
    to that frame's StreamingResponse
  - mock.rs: MockLlmAdapter's un-scripted generate_stream terminal chunk gains
    usage: Some(token_usage) (currently omits it entirely, verified)
  - conformance.rs: new streaming_usage_equals_non_streaming_usage case; CASE_COUNT 8 -> 9;
    every ConformanceFixture impl's stream_body() extended to carry matching usage figures;
    instantiate the suite (or an equivalent dedicated test) for openai, anthropic, deepseek,
    grok, kimi, qwen, mock -- only ollama/gemini/openai_compatible instantiate it today (verified)
  - docs/src/appendix/provider-expansion.md: new "Streamed usage" column; contributing-providers.md:
    new adapter-authoring requirement

Wave 4 — Heralds + docs (ACCT-04)
  - json_herald.rs: paladin_result_to_json emits "usage": <full object> (not "token_count");
    finalize_stream emits "usage": <full metadata.token_usage object> plus keep "total_tokens"
    as a derived convenience per D-08's bare-count-may-coexist rule if useful, or drop it --
    Claude's discretion, D-08 only forbids it being the SOLE carrier
  - markdown_herald.rs: "Token Usage" block replacing "Token Count"; per-Paladin usage table on
    BattalionResult output
  - table_herald.rs (D-23, discretion): minimally keep the Tokens column as usage.total_tokens,
    drop from_total from its tests
  - CLI formatters (output.rs, agent.rs, battalion.rs): "total (prompt P / completion C)" plus
    cache/reasoning when present; JSON mode emits the usage object
  - Grep-and-fix every token_count mdBook/rustdoc reference named in canonical_refs

Wave 5 — HTTP edge + MIGRATION/allowlist/CHANGELOG/OpenAPI (ACCT-05, D-24)
  - paladin-web: new TokenUsageResponse DTO (utoipa::ToSchema, From<TokenUsage>); ExecuteResponse
    swaps token_count -> usage; CompletedRow (run_inspector_port.rs) swaps token_count -> usage;
    dev_ui_controller.rs follows
  - make openapi (regenerate crates/paladin-web/openapi.json); confirm the Python-client
    generation CI job stays green
  - MIGRATION.md §9.2: new rows for TokenUsage, PaladinResult (extend or new row -- it already
    has one row from FT-FR-17; this is a further Change-cell extension or a new row, planner's
    call, matching the file's own precedent of extending vs. adding), StreamingResponse,
    ChunkMetadata, ExecuteResponse; §9.4 note for the Waypoint/trace zero-usage-on-old-rows
    behavior; §9.6 for the HTTP response shape change
  - .cargo/semver-checks-allowlist.toml: new entries per D-27's EMPIRICAL run (do not guess IDs
    ahead of running the tool -- see Common Pitfalls)
  - CHANGELOG.md [0.10.0]: ### Changed + ### Fixed bullets per D-28
  - cargo semver-checks check-release --package {paladin-ai-core,paladin-ports,paladin-web}
    --default-features --baseline-version 0.9.0, then the row-level set-equality script
```

### Pattern 1: `TokenUsage`'s current shape is the smallest possible diff surface
**What:** The type today is four lines of struct + two constructors + four tests. Every field is
a plain `u32` with no `Option`, no serde attributes beyond the derive. This means D-01's three new
fields are the ENTIRE domain-layer change; everything else in this phase is carrier plumbing.
**Verified current state** (`crates/paladin-core/src/platform/container/token_usage.rs`, full
file read this session):
```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
impl TokenUsage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self { .. }
    pub fn from_total(total_tokens: u32) -> Self { .. }  // DELETE (D-05)
}
```
Four existing tests: `new_computes_total_from_prompt_and_completion`,
`from_total_leaves_prompt_and_completion_at_zero` (delete with the method),
`default_is_all_zero`, `partial_eq_compares_all_three_fields_not_only_total`. `PartialEq` is
derived (structural), so adding three new `Option<u32>` fields automatically makes them part of
equality — no extra work needed for D-30's round-trip test to actually catch a swapped/zeroed
optional field.

### Pattern 2: The engine's closure-tuple threading is the single widest mechanical change
**What:** `superstep.rs`'s per-node dispatch closures return a `(paladin_id, token_count: u64,
Result<Directive, NodeFailure>)` tuple (verified at the structured-executor arm `~1095` and the
plain-`PaladinPort` arm `~1140`); this tuple's second element flows, unchanged in type, into every
`NodeExecutionRecord { .. token_count, .. }` and every `TraceEvent::NodeFinished { .. token_count,
.. }` literal at four-plus construction sites in the same file (`~2991`, `~3085`, `~3186`,
`~3257`, `~3277`, verified via direct grep). Changing the tuple's middle element from `u64` to
`TokenUsage` is mechanical (rename the local variable, change its type, update every downstream
literal) but touches the most lines in the phase.
**When to use:** Wave 2, as one atomic commit — a partial migration here leaves the crate
non-compiling, which is actually a useful forcing function (the compiler enumerates every site
that needs the change).
**Example (structured-executor arm, current code, `~1095`):**
```rust
// Source: crates/paladin-battalion/src/engine/superstep.rs:1094-1099 (verified read this session)
Ok(structured) => {
    let token_count = u64::from(structured.raw.token_count);
    let mut delta = StateDelta::new();
    delta.set_raw(output_field, structured.value);
    (paladin_id, token_count, Ok(delta.into()))
}
```
becomes (mechanical rename, `structured.raw.usage: TokenUsage` from the earlier
`PaladinResult.usage` migration):
```rust
Ok(structured) => {
    let usage = structured.raw.usage.clone();
    let mut delta = StateDelta::new();
    delta.set_raw(output_field, structured.value);
    (paladin_id, usage, Ok(delta.into()))
}
```

### Pattern 3: Provider streaming usage mechanisms (verified this session, tagged by confidence)
Context7 was unavailable in this environment (`mcp__context7__*` tools returned "No such tool");
every claim below comes from `WebSearch` against current (2026) documentation, community posts,
and — for the two already-shipped provider fixtures — this repo's own captured test data. Tag
`[CITED: <source>]` per the source hierarchy; none of this is `[VERIFIED]` since no live API call
was made this session.

| Provider | Streamed usage support | Mechanism | Confidence |
|----------|------------------------|-----------|------------|
| OpenAI (Chat Completions) | Supported | `stream_options: {"include_usage": true}`; final chunk before `[DONE]` has empty `choices: []` and the full `usage` object, including `prompt_tokens_details.cached_tokens` / `completion_tokens_details.reasoning_tokens` | `[CITED: OpenAI API Reference / OpenAI Developer Community — "Usage stats now available when using streaming"]` |
| Anthropic (Messages) | Supported | `message_start.message.usage` carries `input_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`; `message_delta.usage` carries cumulative `output_tokens`; `output_tokens_details.thinking_tokens` present per this repo's own captured fixtures (`anthropic/adapter.rs` test constants, `[VERIFIED: repo test fixtures]`) — accumulate `message_start` + `message_delta`, attach at `message_stop` | `[CITED: GitHub issue threads on Anthropic streaming usage accumulation]` + `[VERIFIED: repo fixtures]` for the field names' existence |
| Google Gemini (`streamGenerateContent`) | Supported | Every SSE frame's `usageMetadata` is CUMULATIVE (not per-frame delta); `totalTokenCount = promptTokenCount + candidatesTokenCount + toolUsePromptTokenCount + thoughtsTokenCount` — confirms D-02's inclusive-total semantics is exactly Gemini's own native contract; take the LAST frame (the one with `finishReason`) | `[CITED: Google AI for Developers — "Counting tokens" / GenerateContentResponse reference]` |
| DeepSeek | Supported (OpenAI-compatible) | `stream_options: {"include_usage": true}`; usage object has `prompt_cache_hit_tokens` + `prompt_cache_miss_tokens` (sum = `prompt_tokens`); `completion_tokens_details.reasoning_tokens` referenced in community sources but not independently confirmed against an official field-by-field reference this session | `[CITED: api-docs.deepseek.com Context Caching guide]` for cache fields; reasoning_tokens field name is `[ASSUMED]` pending a live-fixture check |
| xAI Grok (`/v1/chat/completions`) | Supported (OpenAI-compatible) | `stream_options: {include_usage: true}`; usage has `prompt_tokens_details` with `text_tokens`/`audio_tokens`/`image_tokens`/`cached_tokens` — no separate `reasoning_tokens` field confirmed this session | `[CITED: docs.x.ai]`; reasoning-token field name `[ASSUMED]` |
| Moonshot Kimi (`platform.moonshot.cn`) | Supported (OpenAI-compatible) | `stream_options.include_usage` defaults `false`; when `true`, an additional chunk before `[DONE]` carries `usage` with empty `choices` | `[CITED: platform.kimi.ai/docs/api/chat]` |
| Alibaba DashScope / Qwen (`compatible-mode/v1`) | Supported (OpenAI-compatible) | Explicitly documented: "The OpenAI protocol does not return token usage by default. Set `stream_options={"include_usage": true}`" | `[CITED: alibabacloud.com/help/en/model-studio — compatibility-of-openai-with-dashscope]` |
| Ollama (`/v1/chat/completions`, OpenAI-compat mode) | **Supported** — corrects an implicit CONTEXT.md assumption | Ollama's own OpenAI-compatibility doc lists `stream_options` (with `include_usage`) as an explicitly SUPPORTED request field, alongside `tools`/`reasoning`/`reasoning_effort`; `tool_choice`/`logit_bias`/`n`/`logprobs` are the unsupported list | `[CITED: docs.ollama.com/api/openai-compatibility]` |
| Generic third-party OpenAI-compatible server (via `OpenAiCompatibleAdapter`) | **Unknown / server-dependent** — the genuine D-16 exception | The adapter cannot know ahead of time whether an arbitrary self-hosted or third-party server honors `stream_options`; if it silently ignores the field, no usage frame ever arrives and the terminal chunk correctly carries `usage: None` | `[ASSUMED]` — this is a design consequence, not a documented provider behavior |

**Correction to apply in planning:** CONTEXT.md D-16 names "the generic `OpenAiCompatibleAdapter`
(and any preset whose server ignores `stream_options`)" as the exception — this is accurate. But
its second sentence ("Ollama's native `/api/chat` shape... documented the same way if the
adapter's engine does not receive an OpenAI-shaped usage frame") should be read as a conditional
fallback note, not an expected outcome: this repo's `ollama` preset goes through `CompatEngine`
speaking the OpenAI-compatible `/v1/chat/completions` shape (verified: `ollama/adapter.rs`
delegates to `CompatEngine`), and Ollama's own docs confirm that endpoint supports
`stream_options`. The exception documentation should therefore say "supported when Ollama is
recent enough to have added `stream_options` support to its OpenAI-compat layer; verify against
the pinned dev-stack Ollama version" rather than presenting it as a known gap.

### Pattern 4: Current herald output — exact strings that must change
**What:** `json_herald.rs`'s `paladin_result_to_json` (verified, `~118-125`) emits a bare
`"token_count": result.token_count` key; its `finalize_stream` (verified, `~195-205`) emits
`"total_tokens": metadata.token_usage.total_tokens` — a derived scalar, not the object.
`markdown_herald.rs`'s `format_paladin_result` (verified, `~254-256`) emits one line: `**Token
Count**: {result.token_count}`; its `finalize_stream` (verified, `~330-334`) emits `**Total
Tokens**: {metadata.token_usage.total_tokens}`. Both must change to emit/render the full
`TokenUsage` per D-21/D-22.
**`TableHerald`'s `per_paladin_times`/`per_paladin_tokens` name-matching (Pitfall, see below):**
`table_herald.rs`'s battalion formatter (verified, `~200-225`) recovers each row's Paladin NAME by
building a `name_pool: Vec<(String, u64, u32)>` from `(name, time, tokens.total_tokens)` triples
and matching each `paladin_result` against the pool by EXACT `(execution_time_ms, token_count)`
pair — this match key must be updated to compare against `usage.total_tokens` (still `u32`, still
matchable) once `paladin_result.token_count` becomes `paladin_result.usage.total_tokens`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Summing two `TokenUsage`s across loop iterations / Paladin executions | An ad-hoc `usage.prompt_tokens += x.prompt_tokens; usage.completion_tokens += ...` at each of the ~6 accumulation sites | `impl Add`/`AddAssign`/`Sum` on `TokenUsage` itself (D-06) | One saturating, Option-merge-correct implementation used everywhere means a bug fixed once is fixed at every call site; six independent hand-rolled accumulators would each need the same `None`/`Some`/saturating logic re-derived and re-tested |
| Detecting "the terminal usage-bearing chunk" per adapter | A per-adapter ad-hoc `if is_last_frame { attach usage }` check with different logic per provider | The uniform terminal-chunk contract (D-14): `usage` is `Some` on EXACTLY the chunk carrying `finish_reason: Some(..)`, enforced identically across every adapter and asserted by the ONE shared conformance case (D-19) | A per-adapter bespoke detection risks five different interpretations of "terminal" (first `[DONE]`? last SSE event? the frame with empty `choices`?) — the shared conformance case only works if every adapter converges on the identical rule |
| A legacy-JSON `token_count` → `usage.total_tokens` shim deserializer | A custom `Deserialize` impl or an untagged-enum fallback on `PaladinResult`/`NodeExecutionRecord` that tries both old and new field names | Nothing — D-25 explicitly rules this out; `#[serde(default)]` plus the documented "old rows report zero usage" contract (§9.4) is the whole mechanism | ADR-0051's clean-break policy exists precisely to avoid this category of shim; building one here would silently reintroduce the "keep a `token_count` deprecation shim" goal the source PRD's own R3 overrides |
| Streaming usage estimation when a provider omits it | Falling back to `TokenCounterPort`'s heuristic/tiktoken estimate and reporting it as `usage.total_tokens` | `TokenUsage::default()` + a `warn!` log line naming the provider (D-17) | An estimate silently reported as a billed figure is exactly the failure mode Milestone 14's Treasurer must never price against — mixing estimate and billed-count provenance in the same field poisons every downstream cost calculation |

**Key insight:** every "don't hand-roll" item above is really the same principle restated: this
phase's job is to make ONE mechanism (the type's own arithmetic, the port's own terminal-chunk
contract, the documented zero-on-legacy-rows behavior) the single source of truth that every call
site defers to, rather than letting six-plus call sites or five-plus adapters each invent their
own compatible-but-subtly-different version of the same rule.

## Runtime State Inventory

This is not a rename/rebrand phase, but it IS a persisted-schema-shape change (Waypoint
`NodeExecutionRecord` and `TraceRecord`/`TraceEvent`), so the same canonical question applies:
*after every file in the repo is updated, what runtime systems still have the old shape cached,
stored, or registered?*

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | SQLite/Postgres `WaypointPort` backends persist `Waypoint` (containing `NodeExecutionRecord`) as a serialized JSON blob column, NOT typed SQL columns per field (verified pattern from Phase 22/25's additive `#[serde(default)]` fields, e.g. `attempts`, `cache_hit`, following the identical precedent) — a pre-Phase-31 row's JSON blob has `token_count` instead of `usage`; it deserializes with `usage: TokenUsage::default()` (all fields `None`/`0`) per D-25, losing no OTHER data. **No SQL migration file is needed** (no typed column exists to alter) — verify this against `crates/paladin-storage`'s actual Waypoint table schema before assuming it, since a typed `token_count INTEGER` column would need a different treatment. | Code edit only (serde `#[serde(default)]`); confirm no typed SQL column exists for this field during Wave 2 |
| Stored data | `run_traces` persistence (Phase 28, opt-in) stores `TraceRecord` rows the same way — same zero-on-legacy-rows contract applies to `NodeFinished.usage`/`RunFinished.usage` | Code edit only; document in §9.4 |
| Live service config | None — no external service (n8n, Datadog, Tailscale, Cloudflare-equivalent) stores a token-count field for this project | None |
| OS-registered state | None | None |
| Secrets/env vars | None — no env var or secret name references `token_count`/`usage` | None |
| Build artifacts | None — no compiled binary or installed package caches this shape | None |

**Nothing found requiring a data migration** — every persisted record is additive-JSON-tolerant by
the existing `#[serde(default)]` convention this codebase already uses for every prior Waypoint
field addition (`attempts`, `fork_of`, `cache_hit`). Verify the SQLite/Postgres column type
assumption above during Wave 2 execution rather than trusting this research note blindly, since it
was inferred from precedent rather than reading every migration file in `crates/paladin-storage`.

## Common Pitfalls

### Pitfall 1: Guessing `cargo-semver-checks` lint IDs instead of running the tool
**What goes wrong:** Writing `.cargo/semver-checks-allowlist.toml` entries with plausible-sounding
lint IDs (`struct_pub_field_missing`, `inherent_method_missing`, etc.) before ever running
`cargo semver-checks check-release`, then discovering at CI time that the actual fired lint has a
different exact ID, or that TWO lints fire for one change where only one was allowlisted.
**Why it happens:** The lint names are genuinely predictable from the tool's naming convention,
which tempts skipping the empirical step.
**How to avoid:** D-27 is explicit: run the tool FIRST, after the code change lands, then write
the allowlist entries from its actual output. Treat every lint ID in this research (and in
CONTEXT.md's own "expected but unconfirmed" list) as a hypothesis, not a fact.
**Warning signs:** A Wave 5 plan task that writes allowlist entries before a Wave 2/3 plan task
has run `cargo semver-checks` on the landed code.

### Pitfall 2: The `TableHerald` name-matching key silently breaks if only `usage` is added without updating the match
**What goes wrong:** `table_herald.rs`'s battalion formatter matches a `paladin_result` to its
name via `(execution_time_ms, token_count)` as a compound key (verified, `~217-222`). If
`PaladinResult.token_count` is renamed to `usage: TokenUsage` and the match key isn't updated to
`(execution_time_ms, usage.total_tokens)` in the SAME change, this file simply fails to compile
(good — a hard stop) rather than silently misbehaving. But if a future refactor changes the match
to compare a NEW `usage` struct field-for-field instead of just `total_tokens`, two Paladins with
identical total but different prompt/completion splits would no longer collide — worth noting as
a possible improvement, not a requirement.
**How to avoid:** Keep the match on `usage.total_tokens` (matching today's semantics exactly)
unless there's a reason to widen it; the compiler will catch the rename regardless.

### Pitfall 3: Anthropic's `prompt_tokens` correction changes a number a downstream consumer may already depend on
**What goes wrong:** D-20 requires `prompt_tokens = input_tokens + cache_read_input_tokens +
cache_creation_input_tokens` for Anthropic, which is a genuine BEHAVIOR change to an existing,
already-shipped field (`LlmResponse.usage.prompt_tokens` for every Anthropic call) — not just an
additive optional. A caller (in-tree or downstream) reading `prompt_tokens` today gets a
cache-exclusive figure; after this phase, the same field reports a cache-inclusive (billed) figure
that can be meaningfully LARGER for any call that hits Anthropic's prompt cache.
**Why it happens:** This is presented in CONTEXT.md as a bug fix (D-20's own text: "today's
`prompt_tokens` under-reports billed input whenever caching is on"), correctly. But it is still an
observable value change on a non-optional, already-shipped field, distinct from every other
additive change in this phase.
**How to avoid:** This is EXPLICITLY called out in D-28 as one of the two `### Fixed` CHANGELOG
bullets — make sure the plan actually writes that bullet with enough detail (before/after formula)
that a downstream consumer computing their own cost estimates from `prompt_tokens` notices the
change. This is the one item in this phase that most resembles a genuine behavioral change rather
than a pure carrier-shape change, even though CONTEXT.md correctly scopes it under ACCT-03's
"adapters populate the optionals" umbrella rather than as a separate requirement.
**Warning signs:** A test that asserts the OLD (cache-exclusive) `prompt_tokens` value for an
Anthropic fixture with non-zero cache fields — this would be testing the bug, not the fix; the
existing captured fixtures (`THINKING_TEXT_OPUS_5_JSON` etc.) all have `cache_read_input_tokens: 0`
and `cache_creation_input_tokens: 0`, so none of today's existing tests will catch a regression
here — a NEW fixture with non-zero cache fields is needed to prove the correction actually fires.

### Pitfall 4: `GeminiUsageMetadata`'s existing struct has `#[derive(Default)]` — don't accidentally lose that when adding fields
**What goes wrong:** The current struct (`#[derive(Debug, Default, Deserialize)]`, verified) is
used as a `#[serde(default)]` fallback when the field is entirely absent from a response. Adding
`cached_content_token_count`/`thoughts_token_count` as plain `u32` fields (mirroring the existing
three) keeps `Default` derivable automatically; using `Option<u32>` instead is also fine (also
`Default`-derivable) and more faithful to "the provider didn't report this" (D-03) — but either
way, don't remove the `Default` derive itself, since `GeminiResponse.usage_metadata:
Option<GeminiUsageMetadata>` already handles the true-absence case at one level up.
**How to avoid:** Prefer `Option<u32>` with `#[serde(default)]` for the two new fields
specifically (matching D-03's None-vs-Some(0) semantics more precisely than a bare `u32`
defaulting to 0), even though the three EXISTING fields on this struct are plain `u32`.

### Pitfall 5: The Anthropic `ClaudeUsage` struct silently drops fields it doesn't declare — verify against ALL three captured fixtures, not just one
**What goes wrong:** Because `ClaudeUsage`'s current `#[derive(Deserialize)]` has no
`#[serde(deny_unknown_fields)]`, adding the four new fields and testing against only ONE of the
three captured fixture constants (`TEXT_ONLY_OPUS_4_8_JSON`, `THINKING_ONLY_SONNET_5_JSON`,
`THINKING_TEXT_OPUS_5_JSON`) could pass while a different fixture's shape (e.g., one with a
non-zero `cache_creation` nested object, present in all three fixtures but not currently
deserialized at all) reveals a mapping bug.
**How to avoid:** Use `THINKING_TEXT_OPUS_5_JSON` (verified: `input_tokens: 85`,
`cache_creation_input_tokens: 0`, `cache_read_input_tokens: 0`, `output_tokens: 3000`,
`output_tokens_details.thinking_tokens: 2561`) as the D-20 mapping test's PRIMARY fixture exactly
as CONTEXT.md's `<specifics>` section directs — but add a NEW, non-zero-cache fixture (Pitfall 3
above) since none of the three existing ones exercises the cache-inclusive `prompt_tokens`
correction.

## Code Examples

### `TokenUsage`'s new arithmetic (D-06) — the pattern every accumulation site defers to
```rust
// Source: this phase's own design (D-06), no prior art in this repo to copy verbatim from —
// pattern derived from the Option-merge rule stated in CONTEXT.md D-06.
impl TokenUsage {
    fn merge_optional(a: Option<u32>, b: Option<u32>) -> Option<u32> {
        match (a, b) {
            (None, None) => None,
            (None, Some(x)) | (Some(x), None) => Some(x),
            (Some(a), Some(b)) => Some(a.saturating_add(b)),
        }
    }
}

impl std::ops::Add for TokenUsage {
    type Output = TokenUsage;
    fn add(self, rhs: Self) -> Self::Output {
        let prompt_tokens = self.prompt_tokens.saturating_add(rhs.prompt_tokens);
        let completion_tokens = self.completion_tokens.saturating_add(rhs.completion_tokens);
        TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens.saturating_add(completion_tokens), // D-02 invariant recomputed
            cache_read_tokens: Self::merge_optional(self.cache_read_tokens, rhs.cache_read_tokens),
            cache_write_tokens: Self::merge_optional(self.cache_write_tokens, rhs.cache_write_tokens),
            reasoning_tokens: Self::merge_optional(self.reasoning_tokens, rhs.reasoning_tokens),
        }
    }
}
impl std::ops::AddAssign for TokenUsage {
    fn add_assign(&mut self, rhs: Self) { *self = self.clone() + rhs; }
}
impl std::iter::Sum for TokenUsage {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(TokenUsage::default(), |acc, x| acc + x)
    }
}
```

### D-14's terminal-chunk contract, illustrated against the current `CompatEngine` code
```rust
// Source: crates/paladin-llm/src/compat/engine.rs:893-943 (verified current shape). The current
// loop emits finish_reason on the SAME frame the provider sends it, and never captures usage.
// D-14 changes this to a hold-and-emit-at-DONE pattern (sketch, not verified against a landed
// implementation):
let mut held_finish_reason: Option<FinishReason> = None;
let mut held_usage: Option<TokenUsage> = None;
// ... inside the per-line parse loop:
if json_str.trim() == "[DONE]" {
    items.push(Ok(StreamingResponse {
        id: Uuid::new_v4(),
        delta: String::new(),
        finish_reason: held_finish_reason.take().or(Some(FinishReason::Stop)),
        usage: held_usage.take(), // D-13's new field
    }));
    continue;
}
match serde_json::from_str::<CompatStreamResponse>(json_str) {
    Ok(response) => {
        if let Some(usage) = response.usage {
            held_usage = Some(map_compat_usage(usage)); // the empty-choices usage frame
        }
        if let Some(choice) = response.choices.first() {
            if let Some(reason) = &choice.finish_reason {
                held_finish_reason = Some(Self::map_finish_reason(Some(reason.clone())));
                continue; // D-14: do NOT emit finish_reason on this frame
            }
            items.push(Ok(StreamingResponse { /* delta chunk, no usage, no finish_reason */ }));
        }
    }
    Err(e) => { /* unchanged */ }
}
```

### Existing D-30-style round-trip test to extend (the pattern to mirror)
```rust
// Source: crates/paladin-battalion/src/engine/mod.rs (the existing
// paladin_node_execution_record_carries_reported_token_count test and the
// "trace-and-Waypoint-must-agree" test at ~10227 are the two patterns D-30 says to extend —
// read this session to confirm they exist and are named as CONTEXT.md describes)
// [VERIFIED: grep confirms `RunFinished { .. }` construction and `TraceEvent::RunFinished { status, .. }`
// pattern-match sites exist at the cited line numbers in engine/mod.rs]
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `TokenUsage::from_total(u32)` — bare count, zeroed split | `TokenUsage::new(prompt, completion)` + optional cache/reasoning builders | This phase (ACCT-01/02) | Every consumer that previously read `usage.prompt_tokens == 0` on a battalion-aggregated result now sees the real split |
| `PaladinResult.token_count: u32` | `PaladinResult.usage: TokenUsage` | This phase (ACCT-02) | Breaking; every full struct literal across ~6 crates + examples must migrate in the same commit |
| Anthropic `prompt_tokens` excludes cached input | Anthropic `prompt_tokens` includes cache reads + cache writes (billed figure) | This phase (ACCT-03, D-20) | A genuine value-change bug fix, not just an additive field — see Pitfall 3 |
| No streamed usage on any adapter except mock's scripted path | Every adapter attaches `usage` to its stream's terminal chunk (or documents why not) | This phase (ACCT-03) | Enables the Treasurer (Milestone 14) to price a streamed run identically to a non-streamed one |
| `TraceDispatcher.token_total: AtomicU64` | `TraceDispatcher`'s accumulator carries a full `TokenUsage` (Mutex or five atomics) | This phase (ACCT-02, D-11) | `RunFinished.usage` is exact at emit time, same guarantee `token_total()` provided for the bare count |

**Deprecated/outdated:**
- `TokenUsage::from_total` — deleted, not deprecated (ADR-0051 clean-break policy for Phases
  31-33 only).
- The "usage is a bare u64 wherever it crosses a trace/Waypoint boundary" assumption baked into
  `hooks.rs`'s doc comments (`token_total`) and `trace.rs`'s `total_tokens: u64` field doc — both
  need their rustdoc updated in the same commit as the type change (D-29).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | DeepSeek's `completion_tokens_details.reasoning_tokens` field name is assumed by analogy to OpenAI's identical-shaped field; not independently confirmed against an official DeepSeek field-by-field reference this session (only community/aggregator sources were found) | Architecture Pattern 3, provider table | Low-medium — if the actual field name differs, the adapter's optional simply stays `None` (D-03's safe default) until corrected; no silent wrong-value risk since serde with `#[serde(default)]` on an absent/misnamed field just yields `None` |
| A2 | xAI Grok's reasoning-token field name/path was not found in any source this session; whether Grok's `prompt_tokens_details` carries a `reasoning_tokens`-equivalent under `completion_tokens_details` is unconfirmed | Architecture Pattern 3, provider table | Low — same safe-degradation as A1; Grok is not one of the three "verified anchor" providers CONTEXT.md names directly (OpenAI/Anthropic/DeepSeek), so this gap is lower-priority |
| A3 | Context7 MCP tools were expected per this agent's tool_strategy but returned "No such tool available" in this environment; all provider-documentation claims in this file are `[CITED: WebSearch]`, not `[VERIFIED: Context7]` | Throughout Architecture Pattern 3 | Medium — WebSearch results reflect community synthesis and third-party doc mirrors more than official pages in a few cases (e.g. developers.openai.com rather than platform.openai.com); the planner or executor should re-verify exact field names against a live API response or the adapter's own captured-fixture pattern before shipping, exactly as the existing Anthropic fixtures already do |
| A4 | The SQLite/Postgres Waypoint table stores `NodeExecutionRecord` as an opaque JSON blob column (not a typed `token_count` SQL column), so no `ALTER TABLE` migration is needed for the carrier change | Runtime State Inventory | Medium — if a typed column actually exists, ACCT-05's gate list is missing a schema-migration task; verify against `crates/paladin-storage`'s actual Waypoint schema file before closing Wave 2 |

**Highest-risk item:** A4. If wrong, it changes ACCT-05's scope (a new migration file needed) —
verify this FIRST, at the start of Wave 2, before assuming the additive-JSON-only story.

## Open Questions

1. **Does `MIGRATION.md` need a NEW `PaladinResult` row, or an extension of its existing FT-FR-17
   row?**
   - What we know: `PaladinResult` already has a §9.2 row (verified, the `served_by` field
     addition from Phase 25 FT-FR-17), and the document's own established convention (see the
     `Settings` row, extended across Phase 26 and Phase 28) is to EXTEND a Change cell for a
     second field addition to the same type rather than duplicate the row — but this phase
     REMOVES the `token_count` field entirely and REPLACES it with `usage`, which is a
     field-replacement, not a pure addition like the `Settings` precedent.
   - What's unclear: Whether the row-level set-equality CI gate (D-04, Phase 29) treats a
     "replace field X with field Y on the same type" as needing its own new allowlist entry
     distinct from the existing `served_by` one, or whether one `paladin-ai-core | PaladinResult`
     allowlist entry can cover both the historical `served_by` addition and this phase's
     `token_count` → `usage` replacement (the set-equality check is `(crate, type)`-keyed, not
     `(crate, type, specific-field)`-keyed per the verified `ci.yml` script logic — deduplication
     to one set member per `(crate, type)` pair is explicitly the documented behavior for
     `paladin-ai | Settings`).
   - Recommendation: Extend the EXISTING `PaladinResult` row's Change cell (mirroring the
     `Settings` precedent) rather than adding a new row, since the set-equality gate is
     `(crate, type)`-keyed and already has one `paladin-ai-core | PaladinResult` allowlist entry
     that can stay as-is (or gain the newly-empirically-discovered lint IDs from D-27 appended to
     its justification). Confirm this reading against the exact awk script logic in `ci.yml`
     before finalizing the plan.

2. **Is a typed SQL column involved in the Waypoint's persisted `NodeExecutionRecord.token_count`
   field (Runtime State Inventory A4)?**
   - What we know: every prior additive Waypoint field (`attempts`, `fork_of`, `cache_hit`) used
     the `#[serde(default)]`-tolerant JSON-blob pattern with no SQL migration file, per this
     phase's own citation trail.
   - What's unclear: whether `NodeExecutionRecord` specifically is stored as a JSON blob or has
     any typed projection column (e.g. for query/indexing purposes) in `paladin-storage`'s SQLite
     or Postgres adapter.
   - Recommendation: `grep -n "token_count" crates/paladin-storage/src -r` and inspect the
     Waypoint table's actual `CREATE TABLE`/migration files as the FIRST task of Wave 2, before
     assuming the additive-only story holds.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo-semver-checks` | ACCT-05 D-27's empirical lint discovery | Not directly probed this session (research phase); CI installs it pinned at 0.50.0 via `taiki-e/install-action` | 0.50.0 (CI-pinned) `[VERIFIED: ci.yml]` | Install locally with `cargo install cargo-semver-checks --locked --version 0.50.0` if not present |
| Context7 MCP | Provider streaming-usage doc verification | ✗ — tool not registered in this environment (`mcp__context7__resolve-library-id` and `mcp__context7__query-docs` both returned "No such tool available") | — | WebSearch used instead; see Assumptions Log A3 |
| `mockito` | ACCT-03 conformance suite | Already a dev-dependency, used throughout `paladin-llm`'s existing test modules | workspace-pinned | — |
| Live provider API keys | Would be needed for a live-fixture re-verification of A1/A2's uncertain field names | Not applicable to this research session (no live calls made or needed) | — | The adapter's own captured-fixture-test pattern (already established for Anthropic) is the correct verification mechanism at implementation time |

**Missing dependencies with no fallback:** none — Context7's absence has a documented fallback
(WebSearch) that was used throughout this research.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (workspace), `cargo llvm-cov` for coverage (doctests excluded — project memory) |
| Config file | None dedicated; workspace `Cargo.toml` + per-crate `[dev-dependencies]` (mockito, etc.) |
| Quick run command | `cargo test -p paladin-core token_usage` (Wave 1); `cargo test -p paladin-battalion` (Wave 2 round-trip tests); `cargo test -p paladin-llm` (Wave 3 conformance) |
| Full suite command | `cargo test --workspace --all-features` then `cargo llvm-cov --workspace --fail-under-lines 82` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ACCT-01 | `TokenUsage` serde round-trip (legacy JSON without optionals; new JSON with optionals) | unit | `cargo test -p paladin-core --lib platform::container::token_usage` | ❌ Wave 1 — new tests |
| ACCT-01 | `from_total` deleted, arithmetic (`Add`/`AddAssign`/`Sum`) correct incl. saturating + Option-merge | unit | `cargo test -p paladin-core --lib token_usage` | ❌ Wave 1 |
| ACCT-02 | `NodeExecutionRecord`/`NodeFinished`/`RunFinished` carry the identical `TokenUsage` end-to-end (round-trip, D-30) | integration | `cargo test -p paladin-battalion --lib engine` (extend the existing `paladin_node_execution_record_carries_reported_token_count`-style test) | ❌ Wave 2 |
| ACCT-02 | Formation/Phalanx `per_paladin_tokens[name]` equals the Paladin's full usage (regression for `from_total`) | unit | `cargo test -p paladin-battalion --lib formation_service phalanx_service` | ❌ Wave 2 |
| ACCT-03 | Streaming `TokenUsage` equals non-streaming `TokenUsage`, per adapter | integration (mockito) | `cargo test -p paladin-llm streaming_usage_equals_non_streaming_usage` (new conformance case, CASE_COUNT 8→9) | ❌ Wave 3 |
| ACCT-03 | Execution-service-level: `execute_stream` ends with same `usage` as `execute` (mock) | integration | `cargo test -p paladin --lib paladin_execution_service` | ❌ Wave 3 |
| ACCT-04 | JSON herald emits full `usage` object (three optionals always present, even as `null`) | unit | `cargo test -p paladin-herald --lib json_herald` | ❌ Wave 4 |
| ACCT-04 | Markdown herald renders "Token Usage" block, omitting unset optionals | unit | `cargo test -p paladin-herald --lib markdown_herald` | ❌ Wave 4 |
| ACCT-05 | `cargo semver-checks` clean vs 0.9.0 baseline for the three touched crates, given the allowlist | CI | `cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0` (+ ports, web) | ✅ job exists; ❌ new allowlist rows |
| ACCT-05 | Allowlist ↔ MIGRATION.md §9.2 row-level set-equality | CI | the `ci.yml` `semver` job's second step (awk script) | ✅ job exists |
| ACCT-05 | OpenAPI baseline regenerated and matches served spec | CI + local | `make openapi` then `cargo test -p paladin-web --lib openapi_matches_committed_baseline` | ✅ target exists |

### Sampling Rate
- **Per task commit:** the narrowest `cargo test -p <crate> <filter>` for the type just touched
- **Per wave merge:** `cargo test --workspace` (full), `cargo clippy --workspace -- -D warnings`,
  `cargo fmt --check`
- **Phase gate:** `make clean-code`; `cargo llvm-cov --workspace --fail-under-lines 82`; `make
  security`; `cargo doc --workspace --no-deps` (zero warnings); `mdbook build docs/`; the full
  `semver` CI job's two steps; `make openapi` diff-clean; the Python-client generation job

### Wave 0 Gaps
None — the existing test infrastructure (`llm_conformance_suite!`, `RecordingPaladinPort`,
mockito per-adapter harnesses, the engine's own round-trip test patterns) fully covers what this
phase needs; no new test framework or fixture scaffolding is required, only new test CASES within
the existing harnesses.

## Security Domain

`security_enforcement` is absent from `.planning/config.json` (absent = enabled), so this section
is included, scoped honestly to what actually applies.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | No auth surface touched |
| V3 Session Management | No | No session surface touched |
| V4 Access Control | No | No access-control surface touched |
| V5 Input Validation | Yes (narrow) | Every new field on `TokenUsage`/`StreamingResponse`/`ChunkMetadata` is `Option<u32>` deserialized via `serde` with `#[serde(default)]` — no new untrusted-input parsing surface beyond what already exists for the three pre-existing counters; a malicious/malformed provider response with an out-of-range or negative-looking usage figure is bounded by `u32`'s own type (serde rejects non-numeric or negative values into a `u32` field, same as today) |
| V6 Cryptography | No | No cryptographic code touched |

### Known Threat Patterns for this stack

This phase's own `<security_enforcement>` surface is narrow — it touches provider RESPONSE
bodies, which is exactly the credential-handling review class `.github/instructions/
security.instructions.md` calls out: "For any code touching an API key or an external response
body, confirm... Response bodies are redacted BEFORE truncation when embedded in errors or logs."

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| A provider response body containing a credential-shaped string (e.g. an echoed `Authorization` header from a misconfigured gateway) leaking into a log line via the NEW usage-frame parsing code paths | Information Disclosure | This phase adds NO new logging of raw response bodies — the new usage-frame parsing (D-15) extracts only numeric token counts into typed struct fields; the existing redact-then-truncate diagnostic-excerpt pattern (`crate::redaction::diagnostic_excerpt`, already used in `anthropic/adapter.rs`'s deserialization-failure branch) is UNCHANGED by this phase and continues to cover the one place a raw body could reach a log/error message |
| A malformed or adversarial streaming usage frame causing a panic or unbounded allocation | Denial of Service | `serde_json::from_str` on an untrusted frame already returns `Result`, not a panic, for every existing adapter; the new optional fields add no new `.unwrap()`/`.expect()` — verify this discipline holds in Wave 3's new parsing code (per `rust.instructions.md`'s "avoid unwrap/expect... return Result" rule) |
| The one existing `warn!` log line this phase adds (D-17, "streamed call ended with usage: None") naming the provider | Information Disclosure (low) | The log line names only the provider identifier (e.g. `"openai"`), never any request/response content — no redaction concern, matches the existing `FallbackHop` trace event's pattern of naming providers by their static string identifier only |

No new ASVS V2-V4/V6 surface is opened by this phase. The one credential-handling review item
worth a plan checkpoint: confirm the new Anthropic/OpenAI/DeepSeek streaming usage-frame parsing
code in Wave 3 does not introduce a new place a raw response body (as opposed to a parsed numeric
field) could reach a log or error string unredacted — a `checkpoint:human-verify` style review of
the diff against `security.instructions.md`'s credential-handling checklist is appropriate before
sealing Wave 3, consistent with how `anthropic/adapter.rs`'s existing deserialization-failure path
already handles this.

## Sources

### Primary (HIGH confidence — verified directly against this repo's tree this session)
- `crates/paladin-core/src/platform/container/token_usage.rs` — full file, current shape, all 4 tests
- `crates/paladin-core/src/platform/container/execution_result.rs` — full file, `PaladinResult`/`StopReason`, X-10.3 rationale already on file
- `crates/paladin-core/src/platform/container/battalion/mod.rs` (~460-600) — `BattalionResult`, `NodeError`, `TokenUsage` re-export
- `crates/paladin-core/src/platform/container/waypoint.rs` (~530-600) — `NodeExecutionRecord`, `NodeOutcomeKind`
- `crates/paladin-core/src/platform/container/trace.rs` (~190-520) — `TraceEvent` (all 12 variants), `TraceRecord`, existing tests
- `crates/paladin-core/src/platform/container/herald.rs` — `ExecutionMetadata` (confirmed `token_usage: TokenUsage` already full type)
- `crates/paladin-battalion/src/engine/hooks.rs` — full file, `TraceDispatcher`, `token_total: AtomicU64`, `emit()`'s accumulation logic
- `crates/paladin-battalion/src/formation_service.rs` (~180-320), `phalanx_service.rs` (~255-310) — `from_total` sites confirmed exact
- `crates/paladin-battalion/src/engine/superstep.rs` (~1060-1180) — the dispatch closure's tuple threading, structured + plain-port arms
- `crates/paladin-battalion/src/engine/test_support.rs` (~480-530) — `RecordingPaladinPort`
- `crates/paladin-battalion/src/engine/mod.rs` — five `RunFinished{..}` construction sites at exact line numbers, `total_tokens_max`-adjacent grep
- `crates/paladin-ports/src/output/llm_port.rs` (~850-1065) — `LlmResponse`, `StreamingResponse` (confirmed NOT `#[non_exhaustive]` today), `TokenUsage` re-export
- `crates/paladin-ports/src/output/paladin_port.rs` (~380-450) — `ChunkMetadata` (confirmed NO `Default` derive today), `PaladinStreamChunk`
- `crates/paladin-ports/src/input/run_inspector_port.rs` (~55-95) — `CompletedRow`'s REAL location (correcting CONTEXT.md's approximate citation)
- `crates/paladin-llm/src/compat/types.rs`, `compat/engine.rs` (~760-950) — `CompatUsage`/`CompatStreamResponse` current shape, `generate_stream`'s current (no-usage-frame) behavior
- `crates/paladin-llm/src/openai/adapter.rs` (~140-260, ~470-670) — `OpenAIUsage`, `make_streaming_request`, `generate`/`generate_stream`
- `crates/paladin-llm/src/anthropic/adapter.rs` (~460-800, ~880-940) — `ClaudeUsage`, `ClaudeStreamEvent`, `generate_stream`, the three captured fixture constants with exact token figures
- `crates/paladin-llm/src/deepseek/adapter.rs` (~180-300, ~640-770) — `DeepSeekUsage`, `annotate_with_usage`, `generate_stream`
- `crates/paladin-llm/src/gemini/adapter.rs` (~860-1100, ~1230-1270) — `GeminiUsageMetadata`, `generate_stream`, `parse_sse_chunk`
- `crates/paladin-llm/src/mock.rs` (~370-560) — both `generate_stream` impls, confirmed neither attaches usage to the terminal chunk today
- `crates/paladin-llm/src/fallback.rs` (~250-310) — confirmed pass-through, no change needed
- `crates/paladin-llm/src/conformance.rs` — `ConformanceFixture`, `CASE_COUNT`, confirmed only `ollama`/`gemini`/`openai_compatible` instantiate the suite today
- `crates/paladin-herald/src/json_herald.rs`, `markdown_herald.rs`, `table_herald.rs` — exact current output strings/keys
- `src/application/services/paladin/paladin_execution_service.rs` (~1190-1850, ~3030-3280) — the loop accumulator, all `PaladinResult` construction sites, the stream consumer's `metadata: None` site
- `src/application/services/paladin/handoff_service.rs` (~515-530), `src/application/services/orchestration/processors/paladin_processor.rs` (~110-130), `src/application/services/paladin/middleware/limits.rs` — read sites of `result.token_count`/`cumulative_tokens`
- `crates/paladin-eval/src/assertion.rs` — `total_tokens_max`, `RunFinishedInfo`
- `crates/paladin-web/src/agent_controller.rs` (~110-155) — `ExecuteResponse`, confirmed `utoipa::ToSchema` already derived
- `MIGRATION.md` §9.2 (162-270) — row format, the `PaladinResult`/`Settings` extension precedent, the "new-in-0.10, N/A" convention
- `.cargo/semver-checks-allowlist.toml` — full schema, 9 existing entries, exact justification-writing style
- `crates/paladin-core/Cargo.toml`, `paladin-ports/Cargo.toml`, `paladin-web/Cargo.toml` — exact `[package.metadata.cargo-semver-checks.lints]` tables
- `.github/workflows/ci.yml` (~290-410) — the `semver` job (0.50.0 pin, 11-package list), the row-level set-equality awk script
- `CHANGELOG.md` (1-40) — confirmed `[0.10.0]` section exists, dated 2026-09-10, still the active target section
- `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` — full ADR read, confirms scope (Phases 31-33 only)
- `.planning/phases/30-token-economy-vocabulary-commissary-anchoring/30-RESEARCH.md` — format precedent

### Secondary (MEDIUM confidence — WebSearch against current documentation, Context7 unavailable)
- OpenAI streaming `stream_options`/`include_usage` mechanism and frame ordering — `[CITED: OpenAI API Reference / OpenAI Developer Community]`
- Anthropic streaming `message_start`/`message_delta` usage split — `[CITED: multiple GitHub issue threads describing the same accumulation pattern independently]`
- Gemini `usageMetadata` cumulative-per-frame semantics and `totalTokenCount` formula — `[CITED: Google AI for Developers docs]`
- DeepSeek `prompt_cache_hit_tokens`/`prompt_cache_miss_tokens` — `[CITED: api-docs.deepseek.com]`
- xAI Grok `stream_options`/`prompt_tokens_details` — `[CITED: docs.x.ai]`
- Moonshot Kimi `stream_options.include_usage` — `[CITED: platform.kimi.ai/docs/api/chat]`
- Alibaba DashScope/Qwen compatible-mode `stream_options` — `[CITED: alibabacloud.com/help/en/model-studio]`
- Ollama OpenAI-compat `stream_options` support — `[CITED: docs.ollama.com/api/openai-compatibility]` (fetched directly via WebFetch this session)

### Tertiary (LOW confidence — flagged, not used as fact)
- DeepSeek's exact `completion_tokens_details.reasoning_tokens` field name (A1) and xAI Grok's
  reasoning-token field path (A2) — both `[ASSUMED]` by analogy to OpenAI's shape, not confirmed
  against an authoritative field-by-field reference this session.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; every tool already pinned and in use
- Architecture (carrier migration mechanics): HIGH — every code location read directly this
  session, two material corrections to CONTEXT.md's approximate citations already folded in
- Provider streaming mechanisms: MEDIUM — WebSearch-sourced (Context7 unavailable this session);
  the two providers with existing in-repo captured fixtures (Anthropic, and partially Gemini via
  its existing test bodies) have HIGH confidence on field NAMES even where the exact streaming
  frame-arrival TIMING is MEDIUM (community-sourced)
- Pitfalls: HIGH — derived from direct code inspection (exact match-key logic in
  `table_herald.rs`, exact fixture contents in `anthropic/adapter.rs`, exact struct
  derive-attribute state on `ChunkMetadata`/`GeminiUsageMetadata`)

**Research date:** 2026-09-14
**Valid until:** Provider-specific claims (Architecture Pattern 3, Sources/Secondary) should be
treated as valid for ~30 days or until the first per-adapter parity test is actually run against
a live or freshly-recorded fixture, whichever comes first — provider API surfaces (especially
reasoning-token reporting, a fast-moving area across every vendor in 2026) can change field names
without a major version bump. Everything else (the in-tree code shapes, MIGRATION.md conventions,
CI gate mechanics) is stable until Phase 31 itself lands and changes them.
