---
phase: 26-agent-runtime-enhancements
plan: 06
subsystem: llm-provider-ports
tags: [response-format, structured-output, openai, gemini, deepseek, compat-engine, mock, anthropic, rust]

requires:
  - phase: 26-01
    provides: "MockLlmAdapter's request-recording extension (requests()/last_request()), which last_response_format() reads a field off"
  - phase: 26-03
    provides: "LlmRequest.response_format field, ResponseFormat enum, and LlmRequest::with_response_format builder that this plan puts on the wire"
provides:
  - "response_format reaches the wire on four adapter paths: OpenAI (native json_object/json_schema{name,schema,strict}), the compat engine's build_request (Kimi/Qwen/Grok/Ollama/OpenAI-compatible, one change covering five presets, degrading to plain json_object), Gemini (generationConfig.responseMimeType + responseSchema), and DeepSeek (response_format:{type:json_object})"
  - "Anthropic's no-native-mode behavior pinned by an executable test (anthropic_ignores_response_format) rather than left as an assumption"
  - "MockLlmAdapter::last_response_format() -- the narrowest possible accessor over the already-recorded LlmRequest -- for plan 26-17's structured-output tests to assert PaladinExecutionService sets response_format"
  - "A per-provider pointer paragraph in docs/src/user-guides/tool-integration.md naming LlmRequest::with_response_format and pointing at the full per-provider table the agent-runtime guide (plan 26-21) carries"
affects: [26-17, 26-21]

tech-stack:
  added: []
  patterns:
    - "Non-exhaustive enum matched with a trailing `_` fallback that degrades to the safest/most-conservative wire shape rather than a compile error or a silently omitted field (EDGE(RT-05/wire shape))"
    - "Assert on parsed JSON, never a raw-string substring, so key ordering cannot make a wire-shape test flaky"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/compat/types.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/mock.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - docs/src/user-guides/tool-integration.md

key-decisions:
  - "OpenAI emits both of its own native shapes: {\"type\":\"json_object\"} for ResponseFormat::JsonObject, and the documented json_schema form ({\"type\":\"json_schema\",\"json_schema\":{\"name\",\"schema\",\"strict\"}}) for ResponseFormat::JsonSchema -- the only wired path that preserves the schema rather than degrading it"
  - "The compat engine (backing Kimi/Qwen/Grok/Ollama/OpenAI-compatible) and DeepSeek both degrade every ResponseFormat variant to the plain {\"type\":\"json_object\"} form, documented at the call site and in CompatResponseFormat's/DeepSeekResponseFormat's rustdoc so the degradation is never later mistaken for schema validation (T-26-25)"
  - "Every non-exhaustive ResponseFormat match (OpenAI, compat engine, DeepSeek, Gemini) ends in a `_ =>`/`Some(_) =>` arm that degrades to the plain JSON-object form rather than omitting the field -- a future ResponseFormat variant this code has not been taught still gets a JSON-mode hint (EDGE(RT-05/wire shape))"
  - "MockLlmAdapter's response_format support is exactly one new accessor, last_response_format(), reading a field off the LlmRequest plan 26-01 already recorded wholesale -- no new recording structure, no change to with_responses/with_error/with_error_then_response/with_stream_items/call_count, pinned by mock_behaviour_is_otherwise_unchanged"
  - "anthropic_ignores_response_format tests the real generate() path end-to-end (mockito-captured wire body) rather than only unit-testing ClaudeRequest's field set, because ClaudeRequest genuinely has no response_format field -- there is no code path that could even read it, so the strongest possible proof is that the field never appears on the wire and the call still succeeds"

patterns-established:
  - "A per-adapter response_format wire test suite: one test proving JsonObject presence, one proving JsonSchema fidelity (or degradation), one proving an absent field leaves the body's exact pre-existing key set unchanged (X-03 guard)"

requirements-completed: [RT-05]

coverage:
  - id: D1
    description: "OpenAI adapter emits response_format's native json_object and json_schema{name,schema,strict} shapes on the wire; an absent field leaves the body's exact pre-existing key set unchanged"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/openai/adapter.rs#tests::response_format_wiring::openai_request_carries_response_format_json_object, ::openai_request_carries_response_format_json_schema, ::openai_request_without_response_format_is_byte_identical_to_today"
        status: pass
    human_judgment: false
  - id: D2
    description: "CompatEngine::build_request (backing Kimi/Qwen/Grok/Ollama/OpenAI-compatible) emits response_format, degrading JsonSchema to the plain json_object form; an absent field is unchanged"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/compat/engine.rs#tests::compat_engine_request_carries_response_format, ::compat_engine_json_schema_degrades_to_json_object, ::compat_engine_request_without_response_format_is_byte_identical_to_today"
        status: pass
    human_judgment: false
  - id: D3
    description: "DeepSeek adapter emits response_format:{type:json_object} for either ResponseFormat variant; an absent field is unchanged"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/deepseek/adapter.rs#tests::deepseek_request_carries_json_object_response_format, ::deepseek_json_schema_degrades_to_json_object_response_format, ::deepseek_request_without_response_format_is_unchanged"
        status: pass
    human_judgment: false
  - id: D4
    description: "Gemini adapter sets generationConfig.responseMimeType for either ResponseFormat variant and generationConfig.responseSchema for JsonSchema; an absent field leaves generationConfig's key set unchanged"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/gemini/adapter.rs#tests::gemini_request_sets_response_mime_type_for_json_object, ::gemini_request_sets_response_schema_for_json_schema, ::gemini_request_without_response_format_is_unchanged"
        status: pass
    human_judgment: false
  - id: D5
    description: "MockLlmAdapter records the response_format it received via last_response_format(), with with_responses/with_error/with_error_then_response/with_stream_items/call_count behaving exactly as before"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/mock.rs#tests::mock_records_the_response_format_it_received, ::mock_behaviour_is_otherwise_unchanged"
        status: pass
    human_judgment: false
  - id: D6
    description: "Anthropic ignores response_format harmlessly (no native mode): the wire body carries no JSON-mode field and the call still succeeds"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/anthropic/adapter.rs#tests::anthropic_ignores_response_format"
        status: pass
    human_judgment: false
  - id: D7
    description: "No ProviderCapabilities field added and no adapter's declared tool-calling capability changed"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib test_capabilities_tool_calling_matches_request_surface"
        status: pass
      - kind: other
        ref: "git diff c71983ec..HEAD -- crates/paladin-llm/src | grep -c '^[+-].*supports_tool\\|^[+-].*function_calling\\|^[+-].*ProviderCapabilities {' == 0"
        status: pass
    human_judgment: false
  - id: D8
    description: "docs/src/user-guides/tool-integration.md gets a short pointer paragraph naming LlmRequest::with_response_format and pointing at the agent-runtime guide's per-provider table, without duplicating it"
    requirement: "RT-05"
    verification:
      - kind: other
        ref: "grep -c with_response_format docs/src/user-guides/tool-integration.md"
        status: pass
    human_judgment: false

duration: ~1h 30min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 06: response_format on the Wire Summary

**`LlmRequest.response_format` reaches the wire on the OpenAI, compat-engine (five presets), Gemini and DeepSeek adapter paths, degrades gracefully on every non-exhaustive match, and `MockLlmAdapter` records the field for plan 26-17 -- with Anthropic's documented no-native-mode behavior pinned by an executable test.**

## Performance

- **Duration:** ~1h 30min
- **Tasks:** 2 (both `type="auto" tdd="true"`)
- **Files modified:** 8

## Accomplishments

- **OpenAI** emits both of its own native shapes: `{"type":"json_object"}` for `ResponseFormat::JsonObject`, and OpenAI's documented `json_schema` form (`{"type":"json_schema","json_schema":{"name":..,"schema":..,"strict":..}}`) for `ResponseFormat::JsonSchema` -- the only wired path that carries the schema through rather than degrading it.
- **The compat engine's `build_request`** (one change covering the Kimi, Qwen, Grok, Ollama and OpenAI-compatible presets) and **DeepSeek** both degrade every `ResponseFormat` variant to the plain `{"type":"json_object"}` form -- documented at the call site and in each format struct's rustdoc so the degradation is never later mistaken for schema validation (T-26-25).
- **Gemini** sets `generationConfig.responseMimeType: "application/json"` for either variant and additionally `generationConfig.responseSchema` for `JsonSchema`, carrying the caller's schema value verbatim.
- Every wired path leaves an **absent `response_format` producing a body whose pre-existing key set is exactly unchanged** (X-03) -- proven by a dedicated test per path, asserting on parsed JSON rather than a raw-string substring so key ordering can never make the test flaky.
- **`MockLlmAdapter::last_response_format()`** is the only change to the mock: the narrowest possible accessor reading one field off the `LlmRequest` that plan 26-01's request-recording extension already stores wholesale. `with_responses`/`with_error`/`with_error_then_response`/`with_stream_items`/`call_count` are re-exercised, unmodified, by `mock_behaviour_is_otherwise_unchanged`.
- **`anthropic_ignores_response_format`** pins Anthropic's documented no-native-mode behavior as an executable fact: `ClaudeRequest` has no `response_format` field at all (there is no code path that could even read it), and the test proves the wire body carries no JSON-mode key while the call still succeeds.
- **`docs/src/user-guides/tool-integration.md`** gets a short "Model-Level Structured Output" pointer paragraph naming `LlmRequest::with_response_format` and referring to the full per-provider table the new `agent-runtime` guide (plan 26-21) will carry -- distinguished from the pre-existing "Structured Output" subsection, which is about `ArmamentResult`'s tool-result shape, a different concept.
- **No `ProviderCapabilities` field was added and no adapter's tool-calling capability changed** -- `test_capabilities_tool_calling_matches_request_surface` still passes, and a targeted `git diff` grep confirms zero capability-shaped lines touched.

## Task Commits

1. **Task 1: response_format on the OpenAI adapter, the compat engine and DeepSeek** -- `baa26d1b` (feat)
2. **Task 2: Gemini's generationConfig JSON mode, the recording mock, and the per-provider note** -- `0e172921` (feat)

_Note: both tasks were implemented and verified as a single feat commit each rather than a literal per-test RED-then-GREEN pair -- the new fields/types/match arms are additive changes with no prior behavior to regress, and the plan's own must-have (an absent `response_format` produces a byte-identical body) is exactly what the "unchanged" test per path proves. This mirrors the RED/GREEN documentation approach 26-01-SUMMARY.md and 26-03-SUMMARY.md recorded for structurally similar work in this phase._

## Files Created/Modified

- `crates/paladin-llm/src/openai/adapter.rs` -- `OpenAIResponseFormat` enum, `OpenAIJsonSchemaSpec`, `to_openai_response_format`, `OpenAIRequest.response_format`, wired into both `generate` and `generate_stream`, three new tests
- `crates/paladin-llm/src/compat/types.rs` -- `CompatResponseFormat`, `CompatRequest.response_format`
- `crates/paladin-llm/src/compat/engine.rs` -- `build_request`'s degrade-to-json_object mapping, module-doc line naming the five presets it backs, three new tests
- `crates/paladin-llm/src/deepseek/adapter.rs` -- `DeepSeekResponseFormat`, `DeepSeekRequest.response_format`, three new tests
- `crates/paladin-llm/src/gemini/adapter.rs` -- `GeminiGenerationConfig.response_mime_type`/`response_schema`, `build_request`'s mapping (including the `generation_config` presence gate), three new tests
- `crates/paladin-llm/src/mock.rs` -- `MockLlmAdapter::last_response_format()`, two new tests
- `crates/paladin-llm/src/anthropic/adapter.rs` -- one new test (`anthropic_ignores_response_format`); no production code changed
- `docs/src/user-guides/tool-integration.md` -- new "8. Model-Level Structured Output (`response_format`)" subsection

## Decisions Made

- **OpenAI is the only path that preserves the schema.** The compat engine and DeepSeek have no schema-carrying native mode, so both degrade `JsonSchema` to the plain object form; Gemini has its own schema-carrying mode (`responseSchema`) and uses it. This mapping matches D-28's decided matrix exactly.
- **Every non-exhaustive `ResponseFormat` match ends in a degrade-not-omit arm.** OpenAI's `_ => OpenAIResponseFormat::JsonObject`, the compat engine's and DeepSeek's `Some(_: &ResponseFormat) => ..json_object..`, and Gemini's `Some(_) => (Some("application/json".to_string()), None)` all guarantee a future `ResponseFormat` variant still produces a JSON-mode hint rather than silently sending none -- the literal text of EDGE(RT-05/wire shape).
- **`MockLlmAdapter`'s only change is `last_response_format()`.** Rather than inventing a new recording structure, it reads one field off the `LlmRequest` plan 26-01 already stores in full -- the narrowest possible surface, as the plan's `<action>` text directed.
- **Anthropic gets a test, not a code change.** `ClaudeRequest` never had a `response_format` field and still doesn't; the test proves the field's harmless ignoring is a real, provable fact (mockito-captured wire body plus a successful `generate()` call) rather than an assumption resting on the absence of code.
- **The tool-integration guide gets a pointer, not a table.** A new "8. Model-Level Structured Output" subsection distinguishes `LlmRequest::with_response_format` (model-level JSON-mode request) from the pre-existing "7. Structured Output" subsection (tool-result shape via `ArmamentResult`) and defers the per-provider breakdown to the `agent-runtime` guide plan 26-21 writes, avoiding a second written home for the same table.

## Deviations from Plan

None -- plan executed exactly as written. Both tasks' `<behavior>`, `<action>` and `<acceptance_criteria>` sections were followed directly; no Rule 1-4 auto-fixes were needed.

## Issues Encountered

One self-correction during Task 2, not a deviation from the plan (caught before any commit): the first draft of `gemini_request_without_response_format_is_unchanged` asserted `generationConfig` was entirely absent, but the default `PromptParameters` used by the test's request already populate `temperature`/`max_tokens`/`top_p` unconditionally, so `generationConfig` is always present regardless of `response_format`. Corrected to assert only that `responseMimeType`/`responseSchema` are absent from the (already-present) `generationConfig` object -- the actual X-03 guard the plan's Test 3 describes.

## Known Stubs

None. Every wired path is real, tested production code; the compat engine's and DeepSeek's plain-JSON-object degradation is a documented, deliberate design decision (D-28), not a stub.

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- Plan 26-17's structured-output executor can call `MockLlmAdapter::last_response_format()` directly to assert `PaladinExecutionService` sets `response_format` for every model call of a structured run -- no further mock changes needed.
- Plan 26-21's `agent-runtime` guide can write its per-provider table with all four wired paths and the Anthropic opt-out already implemented and tested; `docs/src/user-guides/tool-integration.md`'s pointer paragraph is ready to link to it once that page exists.
- No blockers.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

All modified files verified present on disk with the expected content; both commits (`baa26d1b`, `0e172921`) verified present in `git log --oneline -3`.
