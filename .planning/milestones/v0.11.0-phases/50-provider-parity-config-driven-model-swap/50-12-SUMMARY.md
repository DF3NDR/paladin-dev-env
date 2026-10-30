---
phase: 50-provider-parity-config-driven-model-swap
plan: 12
subsystem: api
tags: [rust, paladin-ports, paladin-llm, refusal, empty-completion, reasoning, cache-write, parity]

requires:
  - phase: 50
    provides: "plan 50-11: LlmResponse.reasoning, StreamingResponse.reasoning, LlmResponse::new and the two with_reasoning constructors; plans 50-06..50-10: seeded tables, option-a configs, create_from_config"
provides:
  - "LlmError::Refusal { provider, model, detail } (Permanent) with an explicit arm in all four classification tables: transience, typed_origin, llm_error_class, classify_fetch_failure (Misconfiguration)"
  - "EmptyCompletion Display is now `Empty completion: {0}`; every payload names its finish reason and carries the advice that fits it"
  - "crates/paladin-llm/src/response_guards.rs: pub(crate) classify_completion (refusal > content_filter > function_call > empty on any finish reason) and a crate-private StreamCompletion accumulator for the terminal chunk of a stream"
  - "CompatEngine, OpenAI and DeepSeek on the guard, buffered and streamed; compat cache_write_tokens and reasoning_content (buffered and per streamed delta); compat refusal on message and delta"
  - "OpenAI response message type split from the request type (optional content and refusal); tool_calls maps to FinishReason::FunctionCall; DeepSeek reasoning_content surfaced"
  - "provider_swap_matrix empty_stop_completion_is_typed un-ignored for kimi, qwen, grok, ollama, openai-compatible, openai, deepseek; MIGRATION.md 9.1 M-B-16; CHANGELOG Added and Changed entries"
affects: [50-13, 50-14, 50-15, 50-16]

tech-stack:
  added: []
  patterns:
    - "One crate-private decision function for a response rule, called by every adapter, so three hand-synced copies cannot drift (Don't Hand-Roll)"
    - "Streams are classified at the terminal chunk from accumulated facts: deltas already yielded stay yielded, only the terminal item becomes the typed error"
    - "Vendor text embedded in an error goes through diagnostic_excerpt only (redact, then bound by characters)"

key-files:
  created:
    - crates/paladin-llm/src/response_guards.rs
  modified:
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-battalion/src/llm_failure.rs
    - crates/paladin-battalion/src/llm_decision.rs
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/compat/types.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - tests/provider_swap_matrix.rs
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "TokenUsage.total_tokens stays prompt + completion (D-02, documented invariant on the type); the plan's 'vendor total, never recomputed' truth was not implemented. Sub-counts (cache read, cache write, reasoning) are copied exactly. Flagged for the owner below."
  - "StreamCompletion, a small accumulator beside classify_completion, carries the streaming half of the rule so the three streaming paths share it instead of repeating the bookkeeping"
  - "A blank refusal string counts as no refusal; a refusal with no provider-visible text never becomes Err(Refusal)"
  - "response_guards is compiled under the seven features whose adapters call it (openai, deepseek, kimi, qwen, grok, ollama, openai-compatible); plan 50-13 widens the list for anthropic and gemini"
  - "A refusal on a models fetch is Misconfiguration-class in classify_fetch_failure (the endpoint answered a prompt-less request with a policy refusal), per the plan"

patterns-established:
  - "Flip a pinned test deliberately: rename it to the new behaviour, keep a comment naming the old pinned behaviour and the decision that flipped it"

requirements-completed: [PARITY-05, PARITY-01]

coverage:
  - id: D1
    description: "D-09: LlmError::Refusal exists, Display names provider and model, Permanent, and has an explicit arm in all four tables"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "paladin-ports llm_port::tests::llm_error_transience_table, refusal_display_names_provider_model_and_detail; doc test LlmError::Refusal; paladin-battalion llm_failure::tests::refusal_carries_its_provider, llm_decision::tests::llm_error_class_labels_a_refusal; paladin-llm compat::engine::tests::classify_fetch_failure_refusal_is_misconfiguration"
        status: pass
    human_judgment: false
  - id: D2
    description: "D-09 / D-11a precedence in one shared function: refusal wins, content_filter and function_call stay Ok, otherwise empty-after-trim is EmptyCompletion on any finish reason"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "response_guards::tests::classify_completion_precedence, refusal_names_provider_and_model"
        status: pass
    human_judgment: false
  - id: D3
    description: "Refusal.detail is redacted before bounding (512-char boundary, straddling key) and bounded by characters (multibyte)"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "response_guards::tests::refusal_detail_keeps_exactly_the_budget_whole_and_bounds_one_more, refusal_detail_redacts_before_bounding, refusal_detail_bounds_multibyte_text_by_chars; compat_buffered_refusal_redacts_the_configured_key; openai_refusal_detail_is_redacted_before_bounding"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-11a on the streaming path: compat, OpenAI and DeepSeek streams with no non-blank content end in Err(EmptyCompletion); content_filter stays Ok; a refusal is the last item; yielded reasoning stays yielded"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "compat_empty_stream_is_empty_completion, compat_content_filter_stream_with_no_text_stays_an_ok_terminal_chunk, compat_stream_refusal_is_the_last_item_and_wins_over_partial_text, openai_empty_stream_is_empty_completion, openai_stream_refusal_is_the_last_item, deepseek_empty_stream_is_empty_completion, response_guards::tests::stream_completion_*"
        status: pass
    human_judgment: false
  - id: D5
    description: "D-11 compat and DeepSeek: reasoning_content on LlmResponse.reasoning and each streamed delta in wire order (blank is None), cache_write_tokens mapped, sub-counts copied as reported"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "compat_buffered_reasoning_text_is_surfaced_and_empty_is_none, compat_buffered_cache_write_tokens_are_mapped, compat_usage_fields_are_copied_exactly, compat_stream_reasoning_keeps_wire_order, deepseek_buffered_reasoning_text_is_surfaced, deepseek_stream_reasoning_keeps_wire_order"
        status: pass
    human_judgment: false
  - id: D6
    description: "OpenAI: refusal (content null + refusal) is Err(Refusal) instead of a parse failure; tool_calls maps to FunctionCall; empty stop is EmptyCompletion"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "openai::adapter::tests::response_guard_wiring::{openai_refusal_is_typed_not_a_parse_failure, openai_tool_calls_finish_maps_to_function_call, openai_empty_stop_completion_is_empty_completion, openai_content_filter_with_empty_text_stays_ok}"
        status: pass
    human_judgment: false
  - id: D7
    description: "Matrix: empty_stop_completion_is_typed green for kimi, qwen, grok, ollama, openai-compatible, openai, deepseek (anthropic already green; gemini stays ignored until 50-13)"
    requirement: "PARITY-05"
    verification:
      - kind: integration
        ref: "cargo test --test provider_swap_matrix --features llm-all empty_stop_completion_is_typed: 8 passed, 1 ignored (gemini); full file: 47 passed, 2 ignored"
        status: pass
    human_judgment: false
  - id: D8
    description: "Record: MIGRATION.md 9.1 M-B-16 and CHANGELOG entries; no new semver register pair"
    requirement: "PARITY-05"
    verification:
      - kind: integration
        ref: "cargo semver-checks check-release --package paladin-ports --baseline-version 0.10.1 --release-type minor --default-features: 191 pass, no update required; make check-gates; make check-consumer-surface; make api-surface (unchanged, 4278 items); make check-api-examples"
        status: pass
    human_judgment: false

duration: ~35min
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 12: Typed refusals and one empty-output rule Summary

**`LlmError::Refusal` (Permanent, named in all four classification tables), one crate-private `classify_completion` rule (refusal, then content filter, then function call, then empty on any finish reason), and the five compat presets, OpenAI and DeepSeek on it, buffered and streamed, with compat cache-write tokens and reasoning text surfaced.**

## Performance

- **Duration:** about 35 min (start about 15:55Z, end 16:30Z)
- **Tasks:** 3 of 3, one commit each
- **Files:** 13 modified, 1 created (plus this SUMMARY)

## Accomplishments

- **Task 1.** `Refusal { provider, model, detail }` after `UsageLimitExceeded`, with rustdoc and a doc test; `Permanent` in `transience()` and a row in `llm_error_transience_table`; explicit arms in `typed_origin` (returns the provider), `llm_error_class` (`"refusal"`) and `classify_fetch_failure` (`Misconfiguration`). `EmptyCompletion`'s `#[error]` is `Empty completion: {0}` and its rustdoc and example say the payload carries the advice. The Anthropic payload, which relied on the old suffix, now appends advice that fits its stop reason.
- **Task 2.** `response_guards.rs` with `classify_completion` and `StreamCompletion`. The CompatEngine's private `detect_empty_completion` is gone. `generate` classifies every response, sets `reasoning` through `with_reasoning`, and builds the response with `LlmResponse::new`; `generate_stream` attaches each delta's `reasoning_content`, accumulates `refusal`, and at `[DONE]` lets the guard decide whether the terminal item is the `Ok` chunk or the typed error. `map_compat_usage` maps `cache_write_tokens`. Wire types gain `refusal`, `cache_write_tokens` and the stream delta's `reasoning_content`.
- **Task 3.** OpenAI's response message is its own type (optional `content`, optional `refusal`), `tool_calls` maps to `FunctionCall` on both paths through one `convert_finish_reason`, and its stream ends on the guard. DeepSeek drops its private copy for the guard (keeping `annotate_with_usage` on the buffered error), surfaces `reasoning_content` on both paths, and its stream ends on the guard. Seven matrix rows un-ignored and green without touching an assertion. MIGRATION 9.1 M-B-16, CHANGELOG Added and Changed.

## Task Commits

1. **Task 1** - `4bced6e3` (feat: Refusal in every classification table; EmptyCompletion Display neutralised)
2. **Task 2** - `9964a47f` (feat: one response guard; compat refusal, empty-on-any-stop, reasoning and cache-write)
3. **Task 3** - `849bf872` (feat: typed refusals and one empty-output rule on the compat, OpenAI and DeepSeek paths)

**Plan metadata:** the docs commit that carries this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Verification

- `cargo fmt --check`: clean. `cargo clippy --workspace --all-features --all-targets -- -D warnings`: clean. `make clean-code` not run end to end (shellcheck absent).
- `cargo test --workspace --lib --bins --no-fail-fast`: 14 binaries, 4974 passed, 2 failed (baseline was 4947 passed, 2 failed: the two ledgered `run_api_wiring` tests, WINDOWS.md row 73). 27 new tests, no new failure.
- `cargo test --workspace --all-features --no-fail-fast`: 8547 passed, 18 failed, exactly the 50-11 set (the same two plus 16 Docker-bound `redis_queue_integration_test` cases).
- `cargo test --test provider_swap_matrix --features llm-all`: 47 passed, 2 ignored (gemini empty-stop for 50-13 and the 50-14 fallback case). `cargo test -p paladin-ports --doc`: 194 passed.
- `make check-gates`, `make check-consumer-surface`, `make api-surface` (unchanged, 4278 items), `make check-api-examples` (105 entry points), `scripts/check-changelogs.sh`: exit 0.
- `cargo semver-checks check-release --package paladin-ports --baseline-version 0.10.1 --release-type minor --default-features`: 191 pass, "no semver update required". Adding a variant to the `#[non_exhaustive]` enum fires no lint, so the `paladin-ports | LlmError` row and allowlist entry are untouched. `target/semver-checks` deleted afterwards.
- **Manual credential-handling review (D-00a):** the only vendor text embedded in an error by this plan is `Refusal.detail`, built solely through `redaction::diagnostic_excerpt` (redact, then bound by characters) in `classify_completion`; the stream path accumulates the whole refusal before redacting (no pre-cap that could slice a key). The `EmptyCompletion` payload carries a fixed finish-reason label and a numeric length, never vendor text (an `Error(_)` finish reason's wrapped string is deliberately left out). No log statement, `Debug` impl, HTTP client or config type was touched.
- **Threat model:** T-50-12-01 mitigated (`refusal_detail_redacts_before_bounding`, multibyte test, per-adapter key-redaction tests). T-50-12-02 mitigated (one precedence, table-tested; PARITY-05 prohibition holds: a refusal is never `ContentFilter`, `EmptyCompletion` or `ProcessingError`). T-50-12-03 mitigated (`Permanent`; fallback stop-by-default is unchanged until 50-14). T-50-12-04 accepted, recorded as M-B-16.

## Decisions Made

See `key-decisions`. Flagged assumptions: A1-50-12 held (`content_filter` with empty text stays `Ok(ContentFilter)`, tested buffered and streamed on compat, OpenAI and DeepSeek). A2-50-12 held (the guard runs at every stream's terminal chunk on the three adapters here). A3-50-12 held (the probe shows `prompt_tokens_details.cache_write_tokens` only on Kimi and `cached_tokens` elsewhere on the compat presets, so only those two fields are mapped). A4-50-12 held (`paladin-eval` untouched).

## Deviations from Plan

### Owner attention

**1. [Plan truth not implemented] `total_tokens` is not copied from the vendor**
- **Plan said:** "`total_tokens` is the vendor's total, never recomputed (`compat_usage_fields_are_copied_exactly`)."
- **Conflict:** `TokenUsage` (paladin-core) documents `total_tokens == prompt_tokens + completion_tokens` as an invariant that always holds (D-02), every accumulation recomputes it, and the compat, OpenAI and DeepSeek mappers all comment that the vendor total is "deliberately unread". Overwriting the field on one path would make the same struct mean two things.
- **What was done:** `compat_usage_fields_are_copied_exactly` asserts prompt, completion, cache-read, cache-write and reasoning counts are copied exactly and that an absent sub-count stays `None`; it also asserts `total_tokens` is prompt + completion when the vendor reports a different total (9999 against 1801 in the test). The `map_compat_usage` rustdoc states the rule.
- **Needs a decision:** if the owner wants the vendor total preserved, that is a `TokenUsage` contract change in `paladin-core` (a field or an invariant relaxation), not a compat-engine edit, and belongs in its own plan.

### Auto-fixed Issues

**2. [Rule 2 - Missing critical functionality] Anthropic `EmptyCompletion` payload relied on the removed Display suffix**
- **Found during:** Task 1
- **Issue:** `detect_no_text_content` deliberately omitted advice because `Display` appended it; with the suffix gone its message had none.
- **Fix:** the payload now ends with advice that fits the stop reason (`max_tokens` stop: larger `max_tokens`; otherwise "the model returned no text"). The existing thinking-only test still passes. `anthropic/adapter.rs` is not in the plan's file list; plan 50-13 owns the rest of the Anthropic refusal work.
- **Commit:** `4bced6e3`

**3. [Rule 1 - Pinned behaviour flipped] Two Kimi tests pinned the old empty-stop behaviour**
- **Found during:** Task 2 (`cargo test -p paladin-llm --all-features --lib`)
- **Issue:** `null_message_content_deserializes_to_empty_string_without_panicking` pinned `Ok("")` and `generate_stream_with_only_done_terminates_with_stop_and_no_error` pinned an `Ok` Stop chunk for a `[DONE]`-only stream. Both are exactly what D-11a changes.
- **Fix:** renamed and flipped (`..._is_an_empty_completion_not_a_parse_failure`, `..._ends_in_an_empty_completion`), each with a comment naming what it pinned and why it flipped. `kimi/adapter.rs` is not in the plan's file list.
- **Commit:** `9964a47f`

**4. [Rule 1 - Test bug] The redaction test asserted the whole placeholder survives truncation**
- **Found during:** Task 2. With the key starting 8 characters before the 512-character boundary, the 10-character `[REDACTED]` itself straddles the cut, so only `[REDACT` survives. The assertion was loosened to the surviving prefix; the key-fragment check (every 3-or-more-character window of the key absent) is unchanged. No production change.
- **Commit:** `9964a47f`

### Scope and process notes

- **`StreamCompletion` was added** beside `classify_completion` (not named in the plan's artifacts) so the three streaming paths share the accumulate-then-classify bookkeeping instead of repeating it. It keeps only the content up to the first non-blank delta and the joined refusal.
- **`.project/current-exports.txt` unchanged.** The baseline tracks root-crate re-exports only, so `grep -c Refusal .project/current-exports.txt` prints 0 where the plan's acceptance line expects at least 1; `make api-surface` exits 0 and reports the surface unchanged. Nothing to commit.
- **DeepSeek passes no refusal** to the guard: its wire has no dedicated refusal field and the probe shows none (A3-50-12).
- **The compat engine's `map_finish_reason` does not map `tool_calls`** (the plan scopes that to OpenAI) and the engine never sends tools, so no compat response carries one today.
- **TDD gate:** the plan is `type: execute`; each task's tests and implementation landed in the task's single commit rather than as separate RED and GREEN commits.
- Disk was never the limiting factor (5.5 GB free at the lowest); `target/debug/incremental` and `target/semver-checks` were deleted after the full runs.

## Known Stubs

None.

## Threat Flags

None. No endpoint, auth path, file access or schema at a trust boundary was added; the new vendor-text sink (`Refusal.detail`) is the one the threat register already covers.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None.

## Next Phase Readiness

- Plan 50-13 can put Anthropic and Gemini on `classify_completion` (widen the `#[cfg(any(feature = ...))]` list on `mod response_guards` in `lib.rs` to include `anthropic` and `gemini`), un-ignore `gemini_empty_stop_completion_is_typed`, and reuse `StreamCompletion` for their streams. The usage-cap `error_override`s stay unencoded for the compat vendors per owner reading R-2.
- Plan 50-14's fallback adapter can match `LlmError::Refusal` for `hop_on_refusal`; it is `Permanent`, so the chain already stops on it by default.
- Open question for the owner: the `total_tokens` decision above.

## Self-Check: PASSED

- Files: this SUMMARY and `crates/paladin-llm/src/response_guards.rs` present; every modified file in `key-files` present.
- Commits `4bced6e3`, `9964a47f`, `849bf872` present in `git log`; no file deletions in any of them.
