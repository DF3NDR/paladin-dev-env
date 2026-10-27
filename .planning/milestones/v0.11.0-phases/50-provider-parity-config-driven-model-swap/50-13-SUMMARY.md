---
phase: 50-provider-parity-config-driven-model-swap
plan: 13
subsystem: api
tags: [rust, paladin-llm, anthropic, gemini, refusal, empty-completion, thinking, stop-reason, usage-cap, parity]

requires:
  - phase: 50
    provides: "plan 50-12: LlmError::Refusal, response_guards::classify_completion and StreamCompletion; plan 50-11: LlmResponse.reasoning and StreamingResponse.reasoning with their with_reasoning constructors; plan 50-17: owner reading R-2 (all five compat presets accept unverified)"
provides:
  - "Anthropic on classify_completion, buffered and streamed: stop_reason refusal is checked before emptiness and is LlmError::Refusal built from stop_details through diagnostic_excerpt"
  - "Every documented Anthropic stop reason mapped by name (tool_use -> FunctionCall, pause_turn, model_context_window_exceeded -> Length, unknown: <value>)"
  - "Anthropic thinking text on LlmResponse.reasoning and thinking_delta on StreamingResponse.reasoning, in order"
  - "Anthropic stream holds message_delta stop_reason and stop_details and classifies at message_stop; unparsable stream events are warned (paladin::anthropic, event type only)"
  - "Gemini on classify_completion, buffered and streamed; SAFETY, RECITATION, BLOCKLIST, PROHIBITED_CONTENT and SPII map to ContentFilter and stay Ok with no text; the hand-synced empty check is gone"
  - "provider_swap_matrix empty_stop_completion_is_typed green on all nine providers (9 run, 0 ignored)"
  - "MIGRATION.md 9.1 M-B-17 and a CHANGELOG Changed entry"
  - "Usage-cap status for the five compat presets recorded as unverified (owner reading R-2); no error_override encoded"
affects: [50-14, 50-15, 50-16]

tech-stack:
  added: []
  patterns:
    - "Streams are classified at the terminal chunk from accumulated facts (StreamCompletion); Gemini does it with a pure post-pass over the parsed items so the stateless frame parser is untouched"
    - "Vendor-supplied tokens placed in a finish reason or a log line are cut to a bounded run of identifier characters (sanitize_token)"
    - "Flip a pinned test deliberately: rename it to the new behaviour and keep a comment naming the old pinned behaviour and the decision that flipped it"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - tests/provider_swap_matrix.rs
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "No usage-cap error_override is encoded for any compat preset: owner reading R-2 (plan 50-17) accepted all five as unverified, so no evidence exists to encode (PARITY-05 prohibition, D-11a)"
  - "Gemini RECITATION (and BLOCKLIST, PROHIBITED_CONTENT, SPII) map to ContentFilter, not Error(<value>), because the shared guard would otherwise report a recitation block with no text as an empty completion"
  - "An Anthropic stream that never reports a stop_reason keeps the old terminal Stop; a buffered response with no stop_reason keeps Error(\"unknown\")"
  - "The Anthropic EmptyCompletion payload is the guard's text plus the content-block census, so the 'thinking' diagnostic the old check gave is kept"
  - "total_tokens stays prompt + completion (D-02 invariant, WINDOWS row 74); Anthropic and Gemini usage mapping was not touched"

patterns-established:
  - "A refusal signal is read before emptiness on every adapter that has one; adapters with no refusal signal (Gemini, A2-50-13) pass None to the guard and rely on the ContentFilter exemption"

requirements-completed: [PARITY-05, PARITY-01]

coverage:
  - id: D1
    description: "D-09 Anthropic buffered: refusal (content: []) is Err(Refusal) naming anthropic and the response model, detail from stop_details category and explanation (either null), never EmptyCompletion; refusal wins over partial text"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "anthropic::adapter::tests::response_parity::{anthropic_refusal_is_typed_not_an_empty_completion, anthropic_refusal_with_null_details_names_the_stop_reason, anthropic_refusal_wins_over_partial_text}"
        status: pass
    human_judgment: false
  - id: D2
    description: "Refusal detail is redacted before bounding (configured key straddling the 512-character boundary)"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "anthropic::adapter::tests::response_parity::anthropic_refusal_detail_redacts_the_configured_key_before_bounding"
        status: pass
    human_judgment: false
  - id: D3
    description: "Anthropic stop reasons map by name on the buffered path; unknown values are named and bounded"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "response_parity::{anthropic_stop_reasons_map_by_name, anthropic_unknown_stop_reason_is_bounded_to_identifier_characters, anthropic_tool_use_without_text_stays_ok, anthropic_empty_content_on_end_turn_is_an_empty_completion, anthropic_context_window_stop_with_no_text_names_the_cause}"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-11 Anthropic thinking text reaches LlmResponse.reasoning in block order (redacted_thinking skipped, blank is None) and thinking_delta reaches StreamingResponse.reasoning in wire order"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "response_parity::{anthropic_thinking_text_reaches_reasoning_in_block_order, anthropic_blank_thinking_is_no_reasoning, anthropic_stream_thinking_deltas_reach_reasoning_in_wire_order}"
        status: pass
    human_judgment: false
  - id: D5
    description: "Pitfall 2 Anthropic stream: real stop_reason on the terminal chunk, refusal after partial text ends the stream with Err(Refusal) with the partial text still yielded, malformed event skipped with a warn naming only a bounded type"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "response_parity::{anthropic_stream_maps_the_real_stop_reason_on_the_terminal_chunk, anthropic_stream_refusal_after_partial_text_ends_in_refusal, anthropic_stream_skips_a_malformed_event_and_keeps_going, unparsable_event_type_names_only_a_bounded_type}; existing streaming_usage_wiring tests unchanged and green"
        status: pass
    human_judgment: false
  - id: D6
    description: "D-11a on the Anthropic and Gemini streams: no non-blank text ends in Err(EmptyCompletion) at the terminal chunk; reasoning already yielded stays yielded; a Gemini blocked finish stays an Ok ContentFilter terminal chunk"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "anthropic_empty_stream_is_empty_completion, gemini_empty_stream_is_empty_completion, gemini_empty_stream_on_max_tokens_advises_a_larger_budget, gemini_blocked_stream_stays_an_ok_content_filter_terminal_chunk, gemini_stream_with_text_before_the_terminal_frame_is_fine"
        status: pass
    human_judgment: false
  - id: D7
    description: "Gemini buffered on the shared guard: the pinned SAFETY-with-no-text test unchanged and green, every blocked finish stays Ok ContentFilter, STOP with empty text is EmptyCompletion, MAX_TOKENS with no text keeps its max_tokens advice"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "gemini::adapter::tests::{parse_response_does_not_blame_the_token_budget_for_a_refusal, parse_response_keeps_every_blocked_content_finish_as_content_filter, parse_response_maps_an_empty_stop_to_empty_completion, parse_response_maps_max_tokens_with_no_parts_to_empty_completion, map_finish_reason_covers_every_documented_value}"
        status: pass
    human_judgment: false
  - id: D8
    description: "Matrix: empty_stop_completion_is_typed runs and passes for all nine providers with none ignored; PARITY-01 edge (an agent that sets no temperature builds on all nine) still green via prompt_reaches_the_wire"
    requirement: "PARITY-01"
    verification:
      - kind: integration
        ref: "cargo test --test provider_swap_matrix --features llm-all empty_stop_completion_is_typed: 9 passed, 0 ignored; full file: 48 passed, 1 ignored (the 50-14 fallback case)"
        status: pass
    human_judgment: false
  - id: D9
    description: "D-11a usage caps are encoded only where evidenced; none is evidenced, so none is encoded and each of the five is recorded unverified"
    requirement: "PARITY-05"
    verification:
      - kind: other
        ref: "Usage-cap status table below matches the R-2 list in 50-SEED-PROBE.md line for line; git diff of this plan adds no 'error_override: Some(' line"
        status: pass
    human_judgment: false

duration: ~30min
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 13: Anthropic and Gemini response parity Summary

**Anthropic refusals are typed (checked before emptiness) with every stop reason named, thinking text surfaced and the stream's real stop reason held to `message_stop`; Gemini joins the shared `classify_completion` guard with blocked-content finishes kept as `Ok(ContentFilter)`; all nine providers now pass the empty-stop matrix case; no usage-cap override is encoded because owner reading R-2 left every compat preset unverified.**

## Performance

- **Duration:** about 30 min
- **Tasks:** 3 of 3, one commit each
- **Files:** 11 modified, none created (plus this SUMMARY)

## Accomplishments

- **Task 1 (Anthropic).** `parse_response` maps the stop reason by name (`map_stop_reason`), builds the refusal text from `stop_details` (`refusal_text`: `category=<c>; <explanation>`, or `stop_reason=refusal` when both are null), and hands the joined text, finish reason and refusal to `classify_completion`, so a refusal can no longer surface as an empty completion. Thinking-block text becomes `reasoning` through `with_reasoning`. The stream holds `stop_reason` and `stop_details` from `message_delta` beside `held_usage`, accumulates text in a `StreamCompletion`, and at `message_stop` either emits the terminal chunk with the mapped finish reason or the typed error as the last item; `thinking_delta` events become empty-delta chunks carrying `reasoning`. An unparsable event line is logged with `log::warn!(target: "paladin::anthropic")` naming only a bounded event type. `detect_no_text_content` is removed; its block census is kept by appending it to the guard's `EmptyCompletion` payload.
- **Task 2 (Gemini).** `parse_response` calls `classify_completion("gemini", ...)` (no refusal signal, A2-50-13) and the hand-synced empty check and its "keep in step by hand" comment are gone. `map_finish_reason` maps `SAFETY`, `RECITATION`, `BLOCKLIST`, `PROHIBITED_CONTENT` and `SPII` to `ContentFilter`. The stream runs a pure post-pass (`classify_stream_items`) over each parsed chunk's items, accumulating deltas and classifying at the item that carries `finishReason`; `parse_sse_chunk` itself is unchanged. `response_guards` is now compiled under the `anthropic` and `gemini` features. The matrix row `gemini_empty_stop_completion_is_typed` is un-ignored and passes without an assertion change.
- **Task 3 (record).** MIGRATION.md 9.1 M-B-17 and a CHANGELOG `### Changed` entry. Each of the five compat presets' `error_override: None` line now carries a comment saying why there is no usage-cap mapping (owner reading R-2, 2026-10-16, plan 50-17).

## Task Commits

1. **Task 1** - `38ab93e9` (feat: Anthropic refusal before empty, named stop reasons, thinking text, held stream stop reason)
2. **Task 2** - `eed4a9ee` (feat: Gemini on the shared response guard; empty-stop matrix green on all nine)
3. **Task 3** - `a07b4105` (feat: record Anthropic and Gemini response parity; usage caps left unverified for compat presets)

**Plan metadata:** the docs commit that carries this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Usage-cap status

Carried from owner reading R-2 (`50-SEED-PROBE.md`, `### Usage-cap evidence (owner reading R-2)` and `## Owner reading R-2`), not re-decided. No quota-exhaustion body was captured for any compat preset, so no `error_override` was encoded and each keeps today's status-based mapping. Plan 50-16's docs page should copy these statuses per vendor.

| Preset | Status |
|--------|--------|
| kimi | unverified -- owner accepted at plan 50-17 Task 1 (2026-10-16) |
| qwen | unverified -- owner accepted at plan 50-17 Task 1 (2026-10-16) |
| grok | unverified -- owner accepted at plan 50-17 Task 1 (2026-10-16) |
| ollama | unverified -- owner accepted at plan 50-17 Task 1 (2026-10-16) |
| openai-compatible | unverified -- owner accepted at plan 50-17 Task 1 (2026-10-16) |

Checks run for this table:

- `grep -n "R-2 " 50-SEED-PROBE.md` reads `accept unverified` for all five, and the evidence list reads `not captured -- owner accepted unverified (2026-10-16)` for all five: five rows, five matches, zero `captured`.
- `git diff 3bb02804..HEAD -- 'crates/*.rs' | grep -c '^+.*error_override: Some'` prints 0: no override was added, so there is no override-next-to-a-date line to show. The only `error_override` edits are the five explanatory comments.
- Anthropic (400-body signature) and DeepSeek (402) keep their existing quota mappings (A2-50-17). The DashScope `limit_requests` and Gemini free-tier bodies in the transcript are rate-limit bodies, recorded there for context only.

## Verification

- `cargo fmt --check`: clean. `cargo clippy --workspace --all-features --all-targets -- -D warnings`: clean. `make clean-code` not run end to end (shellcheck absent, as in the baseline).
- `cargo test -p paladin-llm --all-features --lib`: all green (the anthropic:: group is 90 tests, the gemini:: group 110).
- `cargo test --workspace --lib --bins --no-fail-fast`: 14 binaries; the only failures are the two ledgered `run_api_wiring` tests (WINDOWS.md row 73). No new failure.
- `cargo test --test provider_swap_matrix --features llm-all`: 48 passed, 1 ignored (the 50-14 fallback case); `empty_stop_completion_is_typed`: 9 passed, none ignored.
- `make api-surface` (unchanged), `make check-gates`, `make check-consumer-surface`, `make check-api-examples`, `scripts/check-changelogs.sh`: exit 0. No public item changed; `AnthropicStopDetails` is private.
- **Manual credential-handling review (D-00a):** every vendor text embedded in an error or log goes through `diagnostic_excerpt` or is omitted. `Refusal.detail` is built only by `classify_completion` (redact, then bound by characters) from the Anthropic `category` and `explanation`; the stream accumulates the whole refusal before redacting. The block-type census appended to `EmptyCompletion` goes through `diagnostic_excerpt`. The unknown stop-reason value placed in `Error("unknown: <value>")` and the event type placed in the malformed-event `warn!` are cut to at most 64 identifier characters (`sanitize_token`); the `warn!` never includes the event body or serde's error text (which can quote the value). Gemini passes no refusal text. No HTTP client, config type or `Debug` impl was touched.
- **Threat model:** T-50-13-01 mitigated (redaction test with the configured key straddling the boundary; bounded tokens in logs). T-50-13-02 mitigated (refusal before empty, tested buffered and streamed). T-50-13-03 mitigated by not encoding any unevidenced override.

## Decisions Made

See `key-decisions`. Flagged assumptions: A1-50-13 held (`model_context_window_exceeded` is `Length`; with no text it is `EmptyCompletion` naming `finish_reason=length`, tested). A2-50-13 held (Gemini passes no refusal; blocked finishes stay `Ok(ContentFilter)`, tested buffered and streamed). A3-50-13 resolved by owner reading R-2 as above.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Pinned behaviour flipped] Three Gemini tests pinned the old behaviour**
- **Found during:** Task 2 (`cargo test -p paladin-llm --all-features --lib gemini::`)
- **Issue:** `map_finish_reason_covers_every_documented_value` and `generate_recitation_finish_reason_produces_error_not_stop` pinned `RECITATION` as `Error("RECITATION")`; `parse_response_keeps_an_empty_response_that_finished_normally` pinned `STOP` with empty text as `Ok`. The plan's truth lists `RECITATION` among the finishes that "stay `Ok(ContentFilter)`", but it was `Error(...)` before this plan, and `STOP` with empty text becoming `EmptyCompletion` is exactly D-11a.
- **Fix:** the first two now assert `ContentFilter` for all five blocked finishes (comment names the old pin and the decision), the third is renamed `parse_response_maps_an_empty_stop_to_empty_completion`. The `SAFETY`-with-no-text test the plan names as pinned is unchanged and green. Mapping `RECITATION` to `ContentFilter` was required by the guard: leaving it `Error` would make a recitation block with no text an `EmptyCompletion`, the misreport D-09 forbids.
- **Files modified:** `crates/paladin-llm/src/gemini/adapter.rs`
- **Commit:** `eed4a9ee`
- Recorded for callers in MIGRATION.md 9.1 M-B-17 (`RECITATION`, `BLOCKLIST`, `PROHIBITED_CONTENT` and `SPII` were `Error(<value>)`).

**2. [Rule 2 - Missing critical functionality] Anthropic empty-content diagnostic would have lost its block census**
- **Found during:** Task 1. The existing test `test_thinking_only_response_yields_empty_completion_not_empty_success` requires the message to name the `thinking` block type, which the shared guard's payload does not carry. Rather than weaken the test, the census is appended to the guard's payload (`with_block_census`, through `diagnostic_excerpt`). No assertion changed.
- **Commit:** `38ab93e9`

### Scope and process notes

- **No `error_override` was added** (owner reading R-2). The plan's Task 3 unit test "feeding the evidenced body" has nothing to feed; no placeholder test was written. The five touched preset files carry comment-only changes.
- **Anthropic stream with no reported stop reason** keeps the old terminal `Stop` (the existing `message_stop_carries_usage_none_when_no_usage_payload_ever_arrives` test sends none); a buffered response with no stop reason stays `Error("unknown")`.
- **A stream that ends without `message_stop`** (Anthropic) or without a `finishReason` frame (Gemini) is not classified, as before: there is no terminal chunk to attach the verdict to.
- **Matrix ledger row** (WINDOWS.md row 71) still lists "50-13: 1" among the by-design ignored cases; that case is now un-ignored. The ledger was not edited.
- **TDD gate:** the plan is `type: execute`; each task's tests and implementation landed in the task's single commit rather than as separate RED and GREEN commits.

## Known Stubs

None.

## Threat Flags

None. No endpoint, auth path, file access or schema at a trust boundary was added; the vendor-text sinks (`Refusal.detail`, the block census, the stop-reason label, the `warn!`) are the ones the threat register covers.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None.

## Next Phase Readiness

- Plan 50-14's fallback adapter can match `LlmError::Refusal` from Anthropic as well as the compat, OpenAI and DeepSeek paths; it is `Permanent`, so the chain already stops on it by default.
- Plan 50-15's conformance fixture for Anthropic can assert `refusal_is_typed_and_permanent` and `empty_stop_stream_is_empty_completion`; for Gemini the refusal case asserts the `ContentFilterOnly` shape (A2-50-13).
- Plan 50-16's docs page should copy the Usage-cap status table above per vendor and mention the Gemini finish-reason remapping.
- Open owner question carried from 50-12: the `total_tokens` decision (WINDOWS row 74). Not reopened here.

## Self-Check: PASSED

- Files: every entry in `key-files.modified` exists and is committed; this SUMMARY present.
- Commits `38ab93e9`, `eed4a9ee`, `a07b4105` present in `git log`; no file deletions in any of them.
