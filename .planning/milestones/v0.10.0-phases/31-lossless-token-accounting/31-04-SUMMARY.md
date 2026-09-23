---
phase: 31-lossless-token-accounting
plan: 04
subsystem: llm-adapters
tags: [rust, streaming, sse, token-usage, anthropic, gemini, conformance-testing, tdd]

# Dependency graph
requires:
  - phase: 31-lossless-token-accounting
    provides: "StreamingResponse.usage/ChunkMetadata.usage terminal-chunk contract, CompatEngine's D-14 hold-and-emit pattern, openai/deepseek's own streaming usage parsing, and the execution service's stream consumer (plan 31-03) -- this plan builds on that contract for the two remaining bespoke wire shapes"
provides:
  - "Anthropic's ClaudeUsage gains cache_read_input_tokens/cache_creation_input_tokens/output_tokens_details.thinking_tokens, a shared map_claude_usage function applying the D-20 cache-inclusive prompt_tokens correction, and generate_stream's message_start/message_delta event accumulation attached to the message_stop terminal chunk"
  - "Gemini's GeminiUsageMetadata gains cached_content_token_count/thoughts_token_count, a shared map_gemini_usage function (completion_tokens = candidates + thoughts per D-02), and parse_sse_chunk attaching usage on the finish-reason-bearing frame only"
  - "conformance.rs's ninth shared case streaming_usage_equals_non_streaming_usage (CASE_COUNT 8->9), instantiated for openai/deepseek/grok/kimi/qwen (new) plus gemini/ollama/openai_compatible (extended stream_body() fixtures); Anthropic and the mock adapter get dedicated stand-in tests instead"
  - "D-16 exception documented in OpenAiCompatibleAdapter's own rustdoc plus a new 'Streamed Usage Support' table in docs/src/appendix/provider-expansion.md covering every adapter, and the terminal-chunk contract as a new-adapter testing requirement in docs/src/contributing/contributing-providers.md"
affects: [31-05, 31-06, 31-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Event-accumulation streaming usage (Anthropic): usage split across two events (message_start carries input+cache figures, message_delta carries cumulative output+thinking figures) is merged via a field-level merge function (merge_claude_stream_usage) into one held accumulator, mapped through the SAME map_claude_usage function the non-streaming path uses, and attached only to the message_stop terminal chunk (no [DONE] sentinel exists on this wire)"
    - "Cumulative-per-frame streaming usage (Gemini): unlike Anthropic, no cross-chunk state is needed -- every SSE frame's usageMetadata is already cumulative and arrives on the SAME frame as finishReason, so attaching usage exactly when finish_reason.is_some() IS 'take the last frame's value'"
    - "A shared per-adapter map_X_usage function (map_claude_usage, map_gemini_usage) is the ONLY place D-20's cache/reasoning mapping and D-02's inclusive-total arithmetic live, called from both generate() and generate_stream()'s terminal-chunk attachment so the two paths cannot drift"
    - "The conformance suite's ninth case opens success_body() through generate() and stream_body() through generate_stream() against SEPARATE mockito servers, extending ConformanceFixture::stream_body()'s doc contract to require the SAME usage figures success_body() carries"
    - "Where an adapter's wire shape diverges too far from ConformanceFixture's single-string-per-body assumption (Anthropic's event stream) or its core design forbids real network calls (the mock adapter), a dedicated #[tokio::test] proves the identical three properties by hand, with a rustdoc comment stating it stands in for the shared case"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/conformance.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/mock.rs
    - docs/src/appendix/provider-expansion.md
    - docs/src/contributing/contributing-providers.md

key-decisions:
  - "Anthropic's generate_stream switched from Stream::map to Stream::flat_map -- the identical Rule 1 auto-fix plan 31-03 already applied to CompatEngine/openai/deepseek. The prior .map() returned on the FIRST matching data: line per network chunk, silently dropping every subsequent SSE event (including message_stop itself) whenever mockito's with_body() delivered a whole multi-frame body as one chunk -- which it does by default, making the D-14 contract untestable and, on a real transport with the same framing behavior, unimplementable without this fix."
  - "MockLlmAdapter does NOT instantiate crate::llm_conformance_suite! despite the plan's action text naming it alongside openai/deepseek/grok/kimi/qwen. Five of the shared macro's nine cases (stream_error_before_the_first_chunk, dedicated_status_mappings, transience_by_value, credential_never_appears_in_a_rendered_error, redirect_is_not_followed_with_a_credential_header) require the adapter to issue a real HTTP request a mockito server answers with a specific status -- exactly the network path MockLlmAdapter's own module doc says it exists to avoid ('an in-process LLM adapter without real API calls'). Forcing a fake HTTP path onto it to satisfy the macro would be test theater, not an audit. D-19's own CONTEXT.md text anticipates this ('the execution-service parity test runs offline') -- mock.rs instead gets a dedicated #[tokio::test] asserting the identical three properties directly against the default (non-scripted) generate_stream path. See Deviations for the numeric acceptance-criterion consequence."
  - "The generic OpenAiCompatibleFixture's, ollama's, and every new CompatEngine-family fixture's stream_body() now carries a trailing empty-choices usage frame matching success_body() -- required by D-19's extended fixture contract for the shared parity case to assert equality at all. This does not contradict the D-16 exception (a real third-party server that ignores stream_options): the fixture proves the code path correctly attaches usage WHEN a server sends the frame, documented explicitly in both the fixture's own comment and OpenAiCompatibleAdapter's rustdoc."
  - "Anthropic's ClaudeUsage struct's input_tokens/output_tokens fields gained #[serde(default)] (defaulting to 0) so the SAME struct can deserialize both the non-streaming ClaudeResponse.usage (always carries both) and the streaming message_delta.usage payload (carries only output_tokens, no input_tokens at all) -- reusing one type for both wire shapes rather than declaring a second near-identical struct."

patterns-established:
  - "Every provider's 'Streamed usage' capability is now documented in exactly one of three permitted prose values across two locations (the adapter's own rustdoc and docs/src/appendix/provider-expansion.md's dedicated table) -- never a fourth 'partial' value, and the table covers all nine adapters this crate ships, not just the original three-provider comparison table."

requirements-completed: [ACCT-03]

coverage:
  - id: D1
    description: "Anthropic reports the same cache-inclusive TokenUsage on both generate() and generate_stream(), attaching it to the message_stop terminal chunk; a new non-zero-cache fixture proves the D-20 prompt_tokens correction actually fires (none of the three pre-existing fixtures could catch it, since all report explicit zeros)"
    requirement: "ACCT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/anthropic/adapter.rs#tests::test_cache_inclusive_prompt_tokens_correction_fires_on_non_zero_cache_fixture"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/anthropic/adapter.rs#tests::test_thinking_text_maps_reasoning_and_reports_explicit_zero_cache_figures"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/anthropic/adapter.rs#tests::streaming_usage_wiring::message_stop_is_the_only_usage_bearing_chunk_and_equals_the_non_streaming_usage"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/anthropic/adapter.rs#tests::streaming_usage_wiring::message_stop_carries_usage_none_when_no_usage_payload_ever_arrives"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/anthropic/adapter.rs#tests -- all three pre-existing captured-fixture tests still pass with their original expected values"
        status: pass
    human_judgment: false
  - id: D2
    description: "Gemini reports the same TokenUsage on both paths including cached-content and thoughts sub-counts (completion_tokens = candidates + thoughts, D-02), attached to the last (finish-reason-bearing) streaming frame, with the Default derive intact so a response with no usageMetadata key still parses"
    requirement: "ACCT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/gemini/adapter.rs#tests::parse_response_maps_cached_content_and_thoughts_into_completion_and_optionals"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/gemini/adapter.rs#tests::parse_response_leaves_cache_and_reasoning_none_when_the_payload_omits_them"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/gemini/adapter.rs#tests::parse_response_with_no_usage_metadata_key_still_parses_via_default"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/gemini/adapter.rs#tests::generate_stream_last_frame_usage_equals_the_non_streaming_usage"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/gemini/adapter.rs#tests::generate_stream_without_usage_metadata_yields_terminal_chunk_with_usage_none"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/gemini/adapter.rs#tests::conformance_suite::streaming_assembles_in_wire_order_with_a_terminal_stop -- pre-existing case still passes unchanged"
        status: pass
    human_judgment: false
  - id: D3
    description: "One shared conformance case (streaming_usage_equals_non_streaming_usage, CASE_COUNT 8->9) proves streaming usage equals non-streaming usage for every adapter with a real streaming parser; instantiated for openai/deepseek/grok/kimi/qwen (new) plus gemini/ollama/openai_compatible (extended); Anthropic and the mock adapter each get a dedicated stand-in test with a rustdoc explaining why"
    requirement: "ACCT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/conformance.rs#tests::suite_generates_the_full_case_list_for_a_fixture (pins CASE_COUNT == 9)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --lib conformance (82 tests, includes the new case for every instantiated fixture)"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/mock.rs#tests::streaming_usage_equals_non_streaming_usage_for_the_default_stream_path"
        status: pass
    human_judgment: false
  - id: D4
    description: "The D-16 exception (generic OpenAiCompatibleAdapter's server-dependent streamed usage) is documented as an explicit exception in the adapter's own rustdoc AND in the mdBook provider feature matrix, using exactly one of three permitted values across every listed provider, never a fourth 'partial' value"
    requirement: "ACCT-03"
    verification:
      - kind: other
        ref: "grep -c 'server-dependent' docs/src/appendix/provider-expansion.md == 1; grep -ci 'partial' docs/src/appendix/provider-expansion.md == 0; grep -c 'Streamed usage' docs/src/appendix/provider-expansion.md == 1"
        status: pass
      - kind: other
        ref: "mdbook build docs/ exits 0 with the book's error-level warning policy unchanged"
        status: pass
    human_judgment: false
  - id: D5
    description: "Full workspace compiles, lints, formats and tests clean on the completed streaming usage contract"
    requirement: "ACCT-03"
    verification:
      - kind: other
        ref: "cargo test --workspace --all-features --no-fail-fast (all green except the pre-existing, unrelated cli_isolation/--all-features conflict already logged by plan 31-01 in deferred-items.md)"
        status: pass
      - kind: other
        ref: "cargo fmt --all -- --check (exit 0)"
        status: pass
      - kind: other
        ref: "cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
    human_judgment: false

duration: ~2.5h (estimated; PLAN_START_TIME was not captured at session start)
completed: 2026-09-15
status: complete
---

# Phase 31 Plan 04: Anthropic/Gemini Streaming Usage + Cross-Adapter Parity Conformance Summary

**Anthropic's event-accumulation stream and Gemini's cumulative-per-frame stream both gain the same terminal-chunk usage contract the OpenAI family already has, Anthropic's cache-exclusive `prompt_tokens` is corrected to the billed (cache-inclusive) figure, and one shared conformance case now proves streaming/non-streaming usage parity across nine of ten adapters, with the two structural exceptions (Anthropic's event-accumulation wire shape, the mock adapter's no-real-network design) each getting a dedicated stand-in test instead.**

## Performance

- **Duration:** ~2.5h (estimated)
- **Completed:** 2026-09-15
- **Tasks:** 3 (one `tracer` task with TDD, two `auto` tasks, both `tdd="true"`)
- **Files modified:** 13 across 3 commits

## Accomplishments

- **Task 1 (tracer, TDD) — Anthropic streaming usage accumulation and the cache-inclusive `prompt_tokens` correction.** `ClaudeUsage` gains `cache_read_input_tokens`/`cache_creation_input_tokens`/`output_tokens_details.thinking_tokens` (all `Option<u32>`, `#[serde(default)]`, keeping `None` = "not reported" distinct from `Some(0)` = "reported zero", D-03). A new `map_claude_usage` function builds `TokenUsage` on both paths, applying the D-20 correction: `prompt_tokens = input_tokens + cache_read_input_tokens + cache_creation_input_tokens` (saturating) — Anthropic's `input_tokens` alone excludes cached tokens, so the pre-existing code under-reported billed input on every cache-hitting call. `generate_stream` now accumulates `message_start.message.usage` (input + cache figures) and `message_delta.usage` (cumulative output + thinking figures) via a new `merge_claude_stream_usage` field-level merge, attaching the combined `TokenUsage` to the `message_stop` terminal chunk — this wire has no `[DONE]` sentinel, so `message_stop` IS the D-14 terminal chunk. The adapter's `.map()` streaming loop was switched to `.flat_map()` (a Rule 1 auto-fix identical to 31-03's `CompatEngine`/`openai`/`deepseek` fix): a single network chunk can carry more than one SSE line, and the prior return-on-first-match silently dropped every subsequent event — including `message_stop` itself — whenever a test transport (or a real one with the same framing) delivered more than one frame per chunk. A new `CACHED_PROMPT_SONNET_5_JSON` fixture with non-zero `cache_read_input_tokens: 512`/`cache_creation_input_tokens: 128` proves the correction actually fires (none of the three pre-existing fixtures — all reporting explicit zeros — could catch a regression here).
- **Task 2 — Gemini terminal-frame usage with cached-content and thoughts mapping.** `GeminiUsageMetadata` gains `cached_content_token_count`/`thoughts_token_count` (`Option<u32>`, keeping the struct's pre-existing `Default` derive intact per RESEARCH.md's Pitfall 4). A new `map_gemini_usage` function applies the D-02 inclusive-total contract: `completion_tokens = candidatesTokenCount + thoughtsTokenCount` (saturating), `cachedContentTokenCount → cache_read_tokens`, `cache_write_tokens` always `None` (Gemini reports no cache-write figure). Unlike Anthropic, no cross-chunk accumulator is needed: every Gemini SSE frame's `usageMetadata` is already cumulative and arrives on the SAME frame as `finishReason`, so `parse_sse_chunk` simply attaches usage exactly when `finish_reason.is_some()` — "take the last frame's value" falls out of that condition for free.
- **Task 3 — one shared parity conformance case across the adapter set, plus the provider-page exception.** `conformance.rs` gains a ninth case, `streaming_usage_equals_non_streaming_usage`, opening `success_body()` through `generate()` and `stream_body()` through `generate_stream()` against separate `mockito` servers and asserting the finish-reason chunk and the usage chunk are the SAME chunk, equal field-for-field to the non-streaming `LlmResponse.usage` (`CASE_COUNT` bumped 8 → 9). `ConformanceFixture::stream_body()`'s doc contract now requires carrying the same usage figures `success_body()` does — every existing and new fixture's `stream_body()` was extended with a trailing usage frame accordingly (`gemini`, `ollama`, `openai_compatible`, plus new fixtures for `openai`, `deepseek`, `grok`, `kimi`, `qwen`). Anthropic's event-accumulation wire shape has no single `stream_body()` string to instantiate the macro against, so it gets a dedicated `#[tokio::test]` with a rustdoc stating it stands in for the shared case. The mock adapter (`mock.rs`) similarly gets a dedicated offline test rather than a macro instantiation — see Deviations. `OpenAiCompatibleAdapter`'s own rustdoc now documents the D-16 exception in full, and `docs/src/appendix/provider-expansion.md` gains a new "Streamed Usage Support" table covering every adapter this crate ships (the original three-provider comparison table only ever covered OpenAI/DeepSeek/Anthropic); `docs/src/contributing/contributing-providers.md` gains the terminal-chunk contract as a stated new-adapter testing requirement.
- Full verification: `cargo test -p paladin-llm --lib --all-features` (516 passing, includes `CASE_COUNT == 9` and the parity case green for every instantiated fixture); `mdbook build docs/` exits 0; `cargo fmt --all` clean; `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0; `cargo test --workspace --all-features --no-fail-fast` green except the pre-existing, unrelated `cli_isolation`/`--all-features` conflict already logged in `deferred-items.md` by plan 31-01.

## Task Commits

Each task was committed atomically:

1. **Task 1 (tracer, TDD): Anthropic streaming usage accumulation + cache-inclusive `prompt_tokens`** - `230abad6` (feat)
2. **Task 2: Gemini terminal-frame usage with cached-content and thoughts mapping** - `d9ef5b6c` (feat)
3. **Task 3: shared streaming-usage parity conformance case across every adapter** - `e4e32425` (feat)

**Plan metadata:** this SUMMARY.md commit (docs, worktree mode — orchestrator handles the final metadata commit after merge)

## Files Created/Modified

**Task 1 (1 file):**
- `crates/paladin-llm/src/anthropic/adapter.rs` — `ClaudeUsage`'s four new fields, `map_claude_usage`, `merge_claude_stream_usage`, `.flat_map()`-based `generate_stream`, new `CACHED_PROMPT_SONNET_5_JSON` fixture, mapping + streaming tests

**Task 2 (1 file):**
- `crates/paladin-llm/src/gemini/adapter.rs` — `GeminiUsageMetadata`'s two new optionals, `map_gemini_usage`, `parse_sse_chunk`'s finish-reason-gated usage attachment, extended `GeminiFixture::stream_body()`, mapping + streaming tests

**Task 3 (11 files):**
- `crates/paladin-llm/src/conformance.rs` — the new case, macro list, `CASE_COUNT` pin, extended `ConformanceFixture::stream_body()` doc contract, `TrivialFixture` extended
- `crates/paladin-llm/src/{openai,deepseek,grok,kimi,qwen}/adapter.rs` — new zero-sized `ConformanceFixture` markers instantiating the suite
- `crates/paladin-llm/src/{ollama,openai_compatible}/adapter.rs` — extended existing fixtures' `stream_body()` with a trailing usage frame; `OpenAiCompatibleAdapter` gains the D-16 rustdoc
- `crates/paladin-llm/src/mock.rs` — dedicated offline streaming-usage-parity test
- `docs/src/appendix/provider-expansion.md` — new "Streamed Usage Support" table
- `docs/src/contributing/contributing-providers.md` — new-adapter terminal-chunk contract requirement

## Decisions Made

- **Anthropic's `generate_stream` switched from `Stream::map` to `Stream::flat_map`** (Rule 1 auto-fix, identical in kind to 31-03's `CompatEngine`/`openai`/`deepseek` fix): the prior loop returned on the first matching `data:` line per network chunk, silently dropping every subsequent SSE event — including `message_stop` — whenever more than one frame arrived in a single chunk (which `mockito`'s `with_body()` does by default). This made the D-14 contract both untestable and, on a real transport with equivalent framing, unimplementable without the fix.
- **`ClaudeUsage`'s `input_tokens`/`output_tokens` fields gained `#[serde(default)]`** so the same struct can deserialize both the non-streaming `ClaudeResponse.usage` (always carries both) and the streaming `message_delta.usage` payload (carries only `output_tokens`) — one type for both wire shapes rather than a near-duplicate second struct.
- **The mock adapter (`mock.rs`) does NOT instantiate `crate::llm_conformance_suite!`**, despite the plan's action text listing it alongside `openai`/`deepseek`/`grok`/`kimi`/`qwen`. Five of the shared macro's nine cases require the adapter to make a real HTTP request that a `mockito` server answers with a specific status code, routed through `crate::http_status::map_http_status` — exactly the network path `MockLlmAdapter`'s own module doc says it exists to avoid ("an in-process LLM adapter without real API calls"). Forcing a fake HTTP path onto it purely to satisfy the macro would be test theater, not an audit, and CONTEXT.md's own D-19 text anticipates this outcome ("the execution-service parity test runs offline"). `mock.rs` instead gets a dedicated `#[tokio::test]` asserting the identical three properties by hand. See Deviations for the specific numeric acceptance-criterion consequence.
- **Every `stream_body()` fixture instantiating the shared suite now carries a trailing usage frame matching its `success_body()`** — required by D-19's extended `ConformanceFixture` contract. For `openai_compatible` specifically this does not weaken the D-16 exception: the fixture proves the code path correctly attaches usage WHEN a server sends the frame; a real third-party server that ignores `stream_options` entirely is documented separately, in prose, in both the adapter's rustdoc and the mdBook table — not something this fixture is meant to exercise.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed the same latent multi-frame-per-network-chunk drop in Anthropic's own streaming parser that 31-03 already fixed in `CompatEngine`/`openai`/`deepseek`**
- **Found during:** Task 1
- **Issue:** Anthropic's pre-existing `generate_stream` used `Stream::map` with a `for line in text.lines() { ... return Ok(...) }` loop that returned on the FIRST matching `data:` line, silently dropping every subsequent SSE event in the same network chunk — including `message_stop` itself, which would make the D-14 terminal-chunk contract untestable against a `mockito` fixture that (realistically) delivers a multi-frame body as one chunk.
- **Fix:** Restructured to `Stream::flat_map`, iterating every `data:` line per chunk and emitting one item per event, mirroring `CompatEngine`'s Task-1/31-03 pattern.
- **Files modified:** `crates/paladin-llm/src/anthropic/adapter.rs`
- **Verification:** New multi-event mockito streaming tests pass; all pre-existing non-streaming tests unaffected.
- **Committed in:** `230abad6` (Task 1 commit)

**2. [Rule 4-adjacent — reasoned deviation from a downstream acceptance criterion, not from CONTEXT.md] Mock adapter does not instantiate the shared conformance macro**
- **Found during:** Task 3
- **Issue:** The plan's Task 3 action text and one acceptance criterion ("`grep -rc 'llm_conformance_suite!(' ... totals at least 9 instantiation sites`") both read as if `mock.rs` should instantiate `crate::llm_conformance_suite!` alongside `openai`/`deepseek`/`grok`/`kimi`/`qwen`. `MockLlmAdapter` structurally cannot: five of the shared macro's nine cases require the adapter to issue a real HTTP request a `mockito` server answers with a specific status — `MockLlmAdapter` never performs a network call at all (its own module doc: "an in-process LLM adapter without real API calls"), so those five cases cannot pass without adding a fake network path that contradicts the adapter's entire design purpose. CONTEXT.md's own D-19 text anticipates exactly this ("Mock: ... the execution-service parity test runs offline"), which reads as the authoritative resolution of this specific tension.
- **Resolution:** Gave `mock.rs` a dedicated `#[tokio::test]` (`streaming_usage_equals_non_streaming_usage_for_the_default_stream_path`) asserting the identical three properties the shared case asserts, directly against the default (non-scripted) `generate_stream` path — the same treatment Anthropic's event-accumulation wire shape gets, and the same pattern the plan's own text explicitly sanctions ("a dedicated `#[tokio::test]` where the wire shape differs").
- **Consequence:** `grep -rc 'llm_conformance_suite!(' crates/paladin-llm/src --include=adapter.rs --include=mock.rs` totals **8**, not the plan's literal "at least 9" (`gemini`, `ollama`, `openai_compatible` pre-existing + `openai`, `deepseek`, `grok`, `kimi`, `qwen` new). Total ADAPTER coverage for the D-19 parity property is nonetheless 10 of 10 (8 via the shared macro, Anthropic and the mock each via a dedicated stand-in test) — the phase's actual success criterion ("every LLM adapter's streaming path is audited... or documented as an explicit exception") is fully met; only the plan's specific numeric grep target for macro-instantiation-site count is not.
- **Files modified:** `crates/paladin-llm/src/mock.rs`
- **Verification:** `cargo test -p paladin-llm --lib mock` passes, including the new dedicated test.
- **Committed in:** `e4e32425` (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 1 — a pre-existing bug the task's own fixtures exposed, identical in kind to a 31-03 finding; 1 reasoned deviation from a downstream acceptance-criterion's literal numeric target, made in favor of the phase's own authoritative CONTEXT.md decision and the adapter's documented design purpose).
**Impact on plan:** Both were necessary to make the D-14/D-19 contract actually work and be honestly tested. No scope creep — every adapter's streaming path is now either proven equal to its non-streaming path or documented as an explicit, reasoned exception, which is the phase's real success criterion.

## Issues Encountered

**Pre-existing, unrelated test/feature-flag conflict (not an issue with this plan's changes), already logged by plan 31-01 and reconfirmed by plans 31-02/31-03:**
`tests/cli_isolation_test.rs::test_cli_feature_is_not_default` fails under `cargo test --workspace --all-features` because the test asserts the `cli` feature is NOT active, while `--all-features` (this plan's own verify command) necessarily activates it. Already documented in `.planning/phases/31-lossless-token-accounting/deferred-items.md`; left unfixed as out of scope for this plan too. This was the ONLY failure across the entire `cargo test --workspace --all-features --no-fail-fast` run.

## Known Stubs

None.

## Threat Flags

None beyond the plan's own pre-declared `T-31-13`..`T-31-16` register, all of which this plan's changes satisfy as scoped:
- **T-31-13** (new usage parsing in `anthropic/adapter.rs`/`gemini/adapter.rs`, information disclosure): no new interpolation of a raw response body into an error/log string was added; `diagnostic_excerpt` call count in `anthropic/adapter.rs` is unchanged (2, before and after this plan).
- **T-31-14** (denial of service via an adversarial Anthropic event stream): the accumulator is a fixed-size `ClaudeUsage`/`TokenUsage` updated in place, no per-event allocation grows with event count; the new code adds no `unwrap`/`expect`/`panic!`.
- **T-31-15** (a provider over-reporting a cached-token figure larger than its own input count): accepted per the threat register — the inequality contract is documented, not enforced at runtime, matching the plan's own disposition.
- **T-31-16** (credential-shaped literal in the new Anthropic fixture): `grep -rEn '(sk-ant-|api[_-]?key"?\s*[:=]\s*")' crates/paladin-llm/src/anthropic/adapter.rs` returns matches only in the pre-existing `sk-ant-test123` test-config literal, none newly added by this plan's `CACHED_PROMPT_SONNET_5_JSON` fixture.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

ACCT-03 is now fully satisfied: every LLM adapter's streaming path is either proven to report the same `TokenUsage` as its non-streaming path (OpenAI, DeepSeek, Grok, Kimi, Qwen, Ollama via the shared conformance case; Anthropic and Gemini via their own bespoke event-accumulation/cumulative-frame mechanisms) or documented as an explicit, reasoned exception in both rustdoc and the mdBook provider page (the generic `OpenAiCompatibleAdapter`'s D-16 server-dependent case; the mock adapter's offline-by-design stand-in). Plan 31-05 can proceed to heralds and docs (ACCT-04) with the full streaming usage contract now landed across the entire adapter set. No blockers.

## Self-Check: PASSED

- FOUND: `crates/paladin-llm/src/anthropic/adapter.rs`
- FOUND: `crates/paladin-llm/src/gemini/adapter.rs`
- FOUND: `crates/paladin-llm/src/conformance.rs`
- FOUND: `crates/paladin-llm/src/openai/adapter.rs`
- FOUND: `crates/paladin-llm/src/deepseek/adapter.rs`
- FOUND: `crates/paladin-llm/src/mock.rs`
- FOUND: `docs/src/appendix/provider-expansion.md`
- FOUND: `docs/src/contributing/contributing-providers.md`
- FOUND commit `230abad6` (feat: Task 1 Anthropic streaming usage)
- FOUND commit `d9ef5b6c` (feat: Task 2 Gemini terminal-frame usage)
- FOUND commit `e4e32425` (feat: Task 3 shared conformance case)

---
*Phase: 31-lossless-token-accounting*
*Completed: 2026-09-15*
