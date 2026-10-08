---
phase: 43-rate-pacing
plan: 04
subsystem: llm-pacing
tags: [rate-limit, 429, retry-after, compat-engine, deepseek, gemini, conformance, cadence]

requires:
  - phase: 43-rate-pacing
    provides: the typed RateLimitExceeded { retry_after, hints } carrier (43-02); the shared header parser, its Generic family and map_http_status_with_hints (43-03)
provides:
  - compat engine (Kimi, Qwen, Grok, Ollama and OpenAI-compatible presets), DeepSeek and Gemini surface their first 429 and carry the generic-family Retry-After on both the buffered and streaming non-2xx paths
  - conformance case 10, rate_limit_is_surfaced_once_with_its_retry_delay (CASE_COUNT 10), run by TrivialFixture and eight adapter fixtures
  - DeepSeek guard test flipped to call_api_with_retry_surfaces_rate_limit_exceeded_on_the_first_attempt
  - behaviour registered on MIGRATION 9.1 (M-B-05) and CHANGELOG
affects: [43-06, 43-09, 43-12, 43-13]

tech-stack:
  added: []
  patterns:
    - "A private snapshot_rate_limit_hints(status, headers) per adapter (429 only, generic family, no credential header read), called before response.text() consumes the response"
    - "map_error as a cfg(test) wrapper over map_error_with_hints so existing mapping tests compile unchanged"
    - "A once-only conformance case: one mockito mock with expect(1) asserted after the call, so any hidden retry fails the case"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/conformance.rs
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "D-02 applied to DeepSeek and Gemini as well as the compat engine (Open Question 1): both run private loops that retried a 429 and both sit in the conformance suite; D-02's own rationale covers them. Rated reversible: restoring a 429 to a loop is a one-line change per adapter. To be surfaced to the operator in the phase return and ADR-0058."
  - "Gemini keeps its body-level RetryInfo unparsed (research OQ6); a Gemini 429 carries a delay only when the endpoint also sends a Retry-After header, and a RESOURCE_EXHAUSTED envelope on a non-429 status maps to RateLimitExceeded with no delay"
  - "fetch_live_models in the compat engine and Gemini also snapshot and pass hints, because making map_error cfg(test) removes the plain wrapper from non-test builds (same fix as OpenAI in 43-03)"
  - "The per-adapter snapshot helper is duplicated three times rather than centralised, because rate_limit_headers.rs is not in this plan's files_modified and the helper is five lines"

patterns-established:
  - "Conformance cases measure attempt count separately from classification: transience_by_value keeps expect_at_least(1) for the 429 row, case 10 uses expect(1)"

requirements-completed: [PACE-01, PACE-02]

duration: about 1 h wall clock (two commits, 19:14 and 19:23 UTC, plus verification)
completed: 2026-10-08
status: complete
---

# Phase 43 Plan 04: Compat engine, DeepSeek and Gemini surface their first 429 Summary

**No provider adapter in the crate retries a 429 inside its own loop any more: the compat engine (five presets), DeepSeek and Gemini read Retry-After through the shared generic parser and surface the first 429, and a tenth shared conformance case proves "observed exactly once, carries 7 s, classifies Transient" for every fixture.**

## Performance

- **Tasks:** 2 of 2
- **Commits:** ef1f3543, af00005b
- **Files:** 6 modified, 0 created (about 750 lines added, most of it tests and rustdoc)

## Accomplishments

- Compat engine, DeepSeek and Gemini: on every non-2xx path (buffered `generate`, streaming open, and the model-list fetch where one exists) a 429's generic-family hints are snapshotted before `response.text()` consumes the response. `map_error` became a `cfg(test)` wrapper over a private `map_error_with_hints`, and the generic arms go through `map_http_status_with_hints`.
- D-02: `RateLimitExceeded { .. }` joined the non-retryable early-return set in `CompatEngine::call_api_with_retry`, `DeepSeekAdapter::call_api_with_retry` and `GeminiAdapter::execute_with_retry`, each with a comment citing D-02. Network, timeout and 5xx retry rules are untouched. The DeepSeek and Gemini retry rustdoc was rewritten (DeepSeek's paragraph used to state 429 was retried).
- Gemini's own 429 and `RESOURCE_EXHAUSTED` arms build `rate_limited_with_hints(h)` when hints exist, else `rate_limited(None)`; every existing Gemini mapping test passes unchanged.
- Conformance case 10 `rate_limit_is_surfaced_once_with_its_retry_delay`: one mockito mock for any POST returning 429 with `retry-after: 7`, `.expect(1)` asserted after the call; asserts `RateLimitExceeded`, `retry_after() == Some(7s)` and `Transient`. Added to the macro list, module docs and macro rustdoc updated, `CASE_COUNT` pin moved 9 to 10. `TrivialAdapter` snapshots headers and maps through `map_http_status_with_hints`.
- Registered on the MIGRATION 9.1 M-B-05 row and a CHANGELOG `Changed` bullet. No public API change.

## Red / green record

- **Compat engine:** four tests written first (`call_api_with_retry_surfaces_rate_limit_exceeded_on_the_first_attempt`, `compat_429_is_surfaced_on_the_first_attempt_with_its_retry_delay`, `compat_stream_open_429_is_surfaced_once_with_its_retry_delay`, `compat_429_without_retry_after_carries_no_delay`): 4 failed against the unchanged engine. After the change: 96 compat tests pass.
- **DeepSeek:** the "still retries" guard was replaced and three header tests added first: 3 failed (the unit-level once-only test, the buffered and the streaming header tests); `deepseek_429_without_retry_after_carries_no_delay` passes before and after by design (it pins that a bare 429 carries no hints). After: 52 deepseek tests pass.
- **Gemini:** five tests written first; the red state was a compile failure (`map_error_with_hints` did not exist), so the individual behavioural failures were not observed separately. After: 84 gemini tests pass.
- **Conformance:** with the case and the pin added but `TrivialAdapter` unchanged, 8 of 9 instantiations passed (the real adapters had already been fixed by Task 1) and `conformance::tests::rate_limit_is_surfaced_once_with_its_retry_delay` failed (no delay carried). After updating `TrivialAdapter`: 9 passed.

## Task Commits

1. **Task 1: compat engine, DeepSeek and Gemini surface the first 429 with its Retry-After** - `ef1f3543`
2. **Task 2: conformance case 10 and the register** - `af00005b`

## Verification

- `cargo test -p paladin-llm --all-features --lib`: 627 passed, 0 failed (618 after Task 1, plus the nine case-10 instantiations).
- `cargo test -p paladin-llm --all-features --lib rate_limit_is_surfaced_once_with_its_retry_delay`: 9 passed (TrivialFixture plus qwen, grok, ollama, openai_compatible, gemini, kimi, deepseek, openai). `suite_generates_the_full_case_list_for_a_fixture`: pass with `CASE_COUNT, 10`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `cargo clippy -p paladin-llm --no-default-features -- -D warnings`: clean. `cargo fmt --check`: clean. `RUSTDOCFLAGS='-D warnings' cargo doc -p paladin-llm --no-deps --all-features`: clean.
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: exit 0, "API surface unchanged". `./scripts/check-migration-allowlist.sh`: set-equal.
- Regression: `cargo test --test unit -- deepseek rate_limit` (12 passed, including `test_deepseek_rate_limit_429` and `test_anthropic_rate_limit_429`).
- Acceptance greps: `hints_from_headers(` appears once in each of the three adapters; `LlmError::RateLimitExceeded { .. }` is in each retry function's non-retryable set; `fn call_api_with_retry_still_retries_rate_limit_exceeded` outside comments: 0; the 9.1 section mentions `DeepSeek`: 1 line.
- Manual credential-handling review: each snapshot reads only rate-limit and `Date` names via the generic family, never `Authorization`, `x-goog-api-key` or any credential header; no new log line or error text embeds a header value or body text; `RateLimitHints` has no string field. All three HTTP clients keep their no-redirect policy and the 3xx arms are untouched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `fetch_live_models` in the compat engine and Gemini also called `map_error`**
- **Found during:** Task 1
- **Issue:** making `map_error` a `cfg(test)` wrapper (to keep `-D warnings` free of dead code) would have broken the non-test build, because the model-list fetch in both files still called it.
- **Fix:** gave those paths the same header snapshot and `map_error_with_hints`. No behaviour change for non-429 statuses. Same fix as OpenAI `get_available_models` in 43-03.
- **Commit:** ef1f3543

**2. [Plan wording] Anthropic test name in the case-10 rustdoc**
- **Issue:** the plan's task text names the Anthropic once-only proof loosely as "the hand-written test from 43-03"; the real name is `anthropic_429_is_surfaced_once_with_its_retry_delay_and_four_dimensions`, so the rustdoc cites that.
- **Commit:** af00005b

**3. [Extra beyond the plan] Doc comments and tests**
- Extra tests beyond the plan's behaviour list: streaming-open 429 for each of the three adapters, "429 without Retry-After carries no delay" for each, and a unit-level once-only test for the compat engine and Gemini loops. The 9.1 register text also names the shared conformance case.

**Total deviations:** 3 (1 Rule 3, 1 wording, 1 additive). **Impact:** none on the planned behaviour.

### Open Question 1 (flagged, as the plan required)

D-02 names the OpenAI adapter, the compat engine and the Anthropic adapter. This plan extends it to DeepSeek and Gemini because both ran private loops that retried a 429 and both are in the conformance suite. Reversible (one line per adapter). Not instantiated with `llm_conformance_suite!`: Anthropic (its once-only proof is the 43-03 hand-written test). The OpenAI fixture runs with `max_retries: 0`, so case 10's "exactly once" half is vacuous for it; its once-only proof with `max_retries: 3` is `openai_429_is_surfaced_on_the_first_attempt` (43-01) and its header carriage is `openai_429_carries_retry_after_and_ratelimit_dimensions` (43-03). The vision adapters' `VisionError` loops are a different port and out of scope. To be recorded in ADR-0058 by 43-13.

## Notes

- `.project/current-exports.txt` was not consulted or changed: the plan's acceptance for this plan does not name a baseline symbol, and the baseline covers only the `paladin` facade crate. `make api-surface` is the gate and passes.
- `crates/paladin-llm/src/mock.rs` still carries a historical comment that says "nine cases" (Plan 31-04 era). It is outside this plan's files and describes why `MockLlmAdapter` does not instantiate the macro, so it was left alone.
- The compat engine's `error_override` hook still runs first and receives no hints. No shipped preset sets one (all five pass `None`), so no 429 loses its delay today; a future override for 429 would need to carry hints itself.
- The plan file's tail contained a stray embedded tool-call fragment; it was ignored as instructed.
- Free disk stayed above 7 GB; no ENOSPC and no cleanup was needed. `CARGO_INCREMENTAL=0` was used for every cargo run.

## Deferred Issues

- `docs/src/appendix/provider-expansion.md` and `docs/src/contributing/contributing-providers.md` (flagged in 43-02) are untouched here; 43-13 owns the documentation pass.
- A full `cargo test --workspace --all-features` was not run (disk headroom). Run per-crate: `paladin-llm` all-features lib (627), the root `unit` target's DeepSeek and rate-limit cases (12). Workspace `clippy --all-targets --all-features -D warnings`, `fmt --check` and `make api-surface` are clean. `make security` was not re-run: this plan changed no dependency.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. Mitigations implemented: T-43-13 (first 429 surfaced in all three loops; conformance case 10 asserts one provider hit per fixture), T-43-14 (the shared parser keeps numbers only; no new rendering of header or body text), T-43-15 (the Cadence decorator owns waiting; `RetryPolicy` is untouched and still classifies 429 retryable).

## Next Phase Readiness

Every provider adapter in the crate now surfaces its first 429 with the provider's `Retry-After`, so the Cadence decorator sees each 429 once and 43-06 can pace fallback hops on the same carrier. 43-12 still owns the operator verification of the OpenAI header names; 43-13 owns ADR-0058, the term-table row and the crate CHANGELOG entries.

## Self-Check: PASSED

All six modified files exist, and commits ef1f3543 and af00005b are present in `git log` on `claude/laughing-dirac-e0h2ax`.
