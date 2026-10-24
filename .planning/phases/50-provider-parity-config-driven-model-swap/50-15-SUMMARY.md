---
phase: 50-provider-parity-config-driven-model-swap
plan: 15
subsystem: testing
tags: [rust, paladin-llm, conformance, anthropic, gemini, capability, refusal, usage, parity]

requires:
  - phase: 50
    provides: "50-05/50-06 model_profile and seeded tables on every adapter; 50-08/50-09 config builders; 50-11 LlmResponse.reasoning; 50-12/50-13 LlmError::Refusal, classify_completion and the guard on every adapter; 50-14 decorators forward model_profile"
provides:
  - "Wire::Anthropic and an AnthropicFixture: all nine adapters answer one shared case list"
  - "Seven new cases for all nine fixtures (153 generated tests): capability_declarations_match_the_wire, response_format_reaches_the_wire, empty_stop_completion_is_empty_completion, empty_stop_stream_is_empty_completion, refusal_is_typed_and_permanent, usage_fields_are_mapped, unsupported_parameters_are_omitted_and_recorded"
  - "CASE_COUNT is 17, derived from the macro's own lists and pinned by the macro self-test"
  - "ConformanceFixture methods default_model, provider_name, api_key, malformed_frame, empty_stop_body, empty_stop_stream_body, refusal_shape, usage_body, expected_usage; types RefusalShape, ExpectedUsage, MalformedFrame; wire_defaults module"
  - "Redirect case repaired (307, GET and POST mocks on the target) so a followed redirect is detected"
affects: [50-16, 50-18]

tech-stack:
  added: []
  patterns:
    - "A vendor that lacks a feature states its documented alternative shape through the fixture (RefusalShape::ContentFilterOnly, MalformedFrame::SkippedWithWarning); no case returns early"
    - "Wire-default bodies keyed by Wire, overridden per fixture only where the vendor genuinely differs"
    - "The case that measures the secret-redaction path plants the adapter's own key in the vendor's refusal text, so a fixture whose key differs fails rather than passes vacuously"

key-files:
  created:
    - .planning/phases/50-provider-parity-config-driven-model-swap/deferred-items.md
  modified:
    - crates/paladin-llm/src/conformance.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs

key-decisions:
  - "Anthropic's stream skips an unparsable event with a warn (pre-existing behaviour, pinned by 50-13), so stream_error_after_the_first_chunk gained a MalformedFrame fixture knob: Error for eight adapters, SkippedWithWarning for Anthropic (stream finishes, terminal chunk still arrives, assembled text intact)"
  - "DeepSeek's wire has no refusal field (A3-50-12), so its refusal_shape is ContentFilterOnly: finish_reason content_filter with no text stays Ok(ContentFilter)"
  - "OpenAI's fixture default_model is the unseeded gpt-4o, not the seeded gpt-5.4-nano: OpenAI has no adapter default model and its adapter-wide window (128,000) legitimately differs from gpt-5.4-nano's seeded 400,000 (50-06), so the window-equality assertion measures the adapter-wide declaration the way it applies to an unseeded id"
  - "usage case asserts total_tokens == prompt + completion (the TokenUsage invariant, WINDOWS row 74), not a vendor total; the plan's truth text said 'the vendor's figure' and the dispatch instruction overrode it"
  - "The macro's own self-test fixture (a bare transport) uses llm_conformance_suite!(Fixture, transport_only): it runs the ten transport cases, and CASE_COUNT is still the full 17"
  - "The Kimi adapter-wide fallback profile (Fixed(1.0) temperature, Fixed(0.95) top_p, temperature_range (1.0, 1.0)) is NOT reconciled here: the capability case measures the default model (seeded NotSent), which it does not touch"

patterns-established:
  - "Mutation-check a conformance suite once by hand: delete a case id (self-test fails 16 vs 17), stop a decorator forwarding model_profile (capability case fails)"

requirements-completed: [PARITY-07]

coverage:
  - id: D1
    description: "PARITY-07 / H-1: Anthropic is in the shared suite; all ten existing cases pass for it and for the other eight"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib conformance_suite -- --list: 153 tests (17 cases x 9 fixtures); anthropic::adapter::tests::conformance_suite::* all pass"
        status: pass
    human_judgment: false
  - id: D2
    description: "CASE_COUNT is 17 and a dropped case fails the self-test"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "conformance::tests::suite_generates_the_full_case_list_for_a_fixture; hand mutation: removing usage_fields_are_mapped from the macro list gave left 16, right 17 (restored)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Capability truthfulness for all nine: default model profiled, window equal, admitted parameters (and supported stop sequences) reach the wire, NotSent never does, Pricing(Cadence(adapter)) returns the same profile"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "conformance_suite::capability_declarations_match_the_wire x 9; hand mutation: CadenceLlmAdapter::model_profile returning None made it fail on gemini and deepseek (restored)"
        status: pass
    human_judgment: false
  - id: D4
    description: "response_format JSON-object wire shape per vendor and the paladin.response_format_adjusted record"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "conformance_suite::response_format_reaches_the_wire x 9"
        status: pass
    human_judgment: false
  - id: D5
    description: "D-11a empty stop completion, buffered and streamed, is EmptyCompletion and Permanent for all nine"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "conformance_suite::empty_stop_completion_is_empty_completion x 9; conformance_suite::empty_stop_stream_is_empty_completion x 9 (last stream item is the error)"
        status: pass
    human_judgment: false
  - id: D6
    description: "D-09 refusal: typed Refusal naming the provider, Permanent, adapter key redacted from detail (seven adapters); documented ContentFilter alternative for Gemini and DeepSeek"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "conformance_suite::refusal_is_typed_and_permanent x 9"
        status: pass
    human_judgment: false
  - id: D7
    description: "Usage fields mapped exactly per vendor (None where unreported) and total_tokens == prompt + completion"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "conformance_suite::usage_fields_are_mapped x 9"
        status: pass
    human_judgment: false
  - id: D8
    description: "Parameter-drop policy: temperature 99.0 absent on every wire and recorded under paladin.params_omitted"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "conformance_suite::unsupported_parameters_are_omitted_and_recorded x 9"
        status: pass
    human_judgment: false
  - id: D9
    description: "Redirect case detection power: a client that follows redirects fails it"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "hand check: the trivial fixture's client switched to Policy::limited(5) made redirect_is_not_followed_with_a_credential_header fail (restored); all nine fixtures pass with Policy::none()"
        status: pass
    human_judgment: false

duration: ~60min
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 15: Conformance suite for all nine adapters Summary

**Anthropic joins the shared conformance suite and seven new cases (capability truthfulness, JSON-object wire shape, empty completion buffered and streamed, refusal, usage fields, parameter-drop policy) run for all nine adapters: 17 cases x 9 fixtures = 153 generated tests, with `CASE_COUNT` 17 pinned by the macro self-test.**

## Performance

- **Duration:** about 60 min
- **Tasks:** 2 of 2, one commit each
- **Files:** 10 modified (conformance.rs plus the nine adapter fixtures), 1 created (`deferred-items.md`)

## Accomplishments

- **Task 1.** `Wire::Anthropic`, the new fixture methods with wire defaults (`wire_defaults`), `RefusalShape`, `ExpectedUsage`, `MalformedFrame`, and `AnthropicFixture` (real Messages API success, SSE stream whose merged terminal usage equals the success body's, and error bodies; `max_retries` 0 like the OpenAI fixture). All ten existing cases pass for nine adapters.
- **Task 2.** The seven cases as `pub async fn <case><F: ConformanceFixture>()`, appended to the macro's lists. The macro now holds the lists as two groups (ten transport, seven behaviour); `CASE_COUNT` counts both from the same tokens. Fixtures declare `default_model` and `provider_name`; DeepSeek, Kimi, Gemini and Ollama override only what their vendor differs on.

## Task Commits

1. **Task 1** - `e8924f90` (test: Anthropic in the shared conformance suite, wire defaults, redirect repair)
2. **Task 2** - `2ccbd608` (test: seven Phase 50 conformance cases for all nine adapters)

**Plan metadata:** the docs commit carrying this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Per-vendor shapes the cases assert

| Adapter | Refusal shape | Usage overrides | Malformed frame |
|---|---|---|---|
| openai, qwen, grok, ollama, openai-compatible | Typed (`message.refusal`) | none (wire default: cached_tokens, reasoning_tokens; no cache write) | Error |
| kimi | Typed | adds `prompt_tokens_details.cache_write_tokens` (Moonshot) -> cache write Some(20) | Error |
| deepseek | ContentFilterOnly (`content_filter`, no text) | `prompt_cache_hit_tokens` -> cache read Some(30); no cache write | Error |
| anthropic | Typed (`stop_reason: refusal` + `stop_details`) | wire default: input 50 + cache read 30 + cache write 20 = prompt 100; thinking_tokens 15 | SkippedWithWarning |
| gemini | ContentFilterOnly (`SAFETY`, no parts) | wire default: candidates 25 + thoughts 15 = completion 40; cachedContentTokenCount 30; no cache write | Error |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The shared redirect case could not detect a followed redirect**
- **Found during:** Task 1 (the dispatch note carried 50-09's finding forward).
- **Issue:** the case answered `302` and mocked the redirect target for `POST` only. A client that follows a `302` re-sends as `GET`, which that mock never matched, so a followed redirect passed.
- **Fix:** answer `307` (method preserved) and mock the target for both `POST` and `GET` with `expect(0)`, asserting both. Checked by hand: a trivial-fixture client switched to `Policy::limited(5)` fails the case; reverted. All nine fixtures pass.
- **Files modified:** `crates/paladin-llm/src/conformance.rs`
- **Commit:** `e8924f90`

**2. [Rule 1 - Case premise did not hold] `stream_error_after_the_first_chunk` fails for Anthropic**
- **Found during:** Task 1, first run of `AnthropicFixture`.
- **Issue:** the case required an unparsable frame after valid ones to surface as an error. Anthropic's stream parser skips an unparsable event with a `warn!` (pre-existing; 50-13 added the warn and pinned `anthropic_stream_skips_a_malformed_event_and_keeps_going`). Changing the adapter was out of scope and would contradict a pinned decision; skipping the case for Anthropic is forbidden by the PARITY-07 prohibition.
- **Fix:** `ConformanceFixture::malformed_frame()` (`MalformedFrame::{Error, SkippedWithWarning}`, default `Error`). The case asserts content survival for every fixture, an error for `Error`, and no error plus a terminal chunk for `SkippedWithWarning`. Only Anthropic overrides.
- **Commit:** `e8924f90`

**3. [Rule 1 - Case premise did not hold] DeepSeek has no typed refusal; OpenAI's window differs from a seeded default**
- **Found during:** Task 2, first run (3 of 153 failed: `deepseek refusal`, `deepseek usage`, `openai capability`).
- **Issue:** (a) DeepSeek's wire has no `refusal` field (A3-50-12), so the OpenAI-chat default `Typed` body is an `EmptyCompletion` there; (b) DeepSeek reports the cache hit as `prompt_cache_hit_tokens`, not `prompt_tokens_details`; (c) OpenAI has no adapter default model, its adapter-wide `get_capabilities` window is 128,000 while seeded `gpt-5.4-nano` is 400,000 (documented in `openai_default_model_is_seeded`).
- **Fix:** DeepSeek overrides `refusal_shape` (`ContentFilterOnly`, `finish_reason: content_filter`) and `usage_body`; Kimi overrides `usage_body` and `expected_usage` for Moonshot's cache-write field; OpenAI's `default_model` is the unseeded `gpt-4o`, which resolves to the adapter-wide fallback profile, whose window is exactly the declared 128,000. No adapter code changed.
- **Commit:** `2ccbd608`

**4. [Rule 3 - Plan listed fewer files] Fixture edits reached the four compat adapters the plan did not list**
- **Why:** assumption A1-50-15 said new methods default from `Self::WIRE`; `default_model` and `provider_name` have no sensible default (an adapter-derived provider name would make the refusal case compare a name with itself). Required methods meant a two-line edit in `qwen`, `grok`, `ollama` and `openai_compatible` fixtures as well. Ollama also overrides `api_key` (its fixed placeholder `ollama`, not `test-key`) and Gemini overrides it to its fixture key.
- **Files modified:** `crates/paladin-llm/src/{qwen,grok,ollama,openai_compatible}/adapter.rs`
- **Commit:** `2ccbd608`

### Scope and process notes

- **The Anthropic hand-written case was kept.** The plan said to remove it "if the suite now covers it". `anthropic_429_is_surfaced_once_with_its_retry_delay_and_four_dimensions` also pins the four rate-limit dimensions and a no-sleep bound the suite's case does not, so it stays; the suite case's rustdoc coverage note was updated. The stale comment on the Anthropic streaming-usage test (it said Anthropic could not instantiate the suite) was rewritten.
- **`usage_fields_are_mapped` asserts `total_tokens == prompt + completion`**, not a vendor total, per the dispatch instruction and the 50-12 decision (WINDOWS row 74, owner decision pending). The vendor body totals happen to equal that sum, so the case does not pre-empt the owner decision.
- **Macro self-test fixture.** The `TrivialFixture` is a bare transport with no sampling, profile or refusal surface, so it uses `llm_conformance_suite!(TrivialFixture, transport_only)`: ten transport tests, `CASE_COUNT` still 17. All nine adapter fixtures use the full form.
- **Kimi adapter-wide fallback profile not reconciled** (`Fixed(1.0)` temperature, `Fixed(0.95)` top_p, `temperature_range (1.0, 1.0)`, conflicting with the 2026-10-15 probe). The capability case measures each fixture's default model, which for Kimi is the seeded `kimi-k2.6` entry (every sampling parameter `NotSent`), so the fallback is never reached and no case needed it. It still applies to unseeded Kimi ids; recorded here for plan 50-16 / the owner, not changed.
- **TDD gate:** the plan is `type: execute` with `tdd="true"` tasks; tests and the (test-only) fixture code landed in each task's single commit. There is no production change in this plan, so no separate RED/GREEN commits exist. Each new case was exercised against real adapters on first run and three genuine divergences surfaced (deviations 2 and 3).
- **Single-feature clippy** is not clean at baseline for `paladin-llm` (an unused `observe_refusal` with only `gemini` or `deepseek` on, among others). Logged in `deferred-items.md`; this plan adds no warning in the seven single-feature builds checked.

## Verification

- `cargo test -p paladin-llm --all-features --lib`: 1100 passed, 0 failed. `... conformance_suite -- --list | grep -c ': test$'`: 153.
- `cargo fmt --check`: clean. `cargo clippy --workspace --all-features --all-targets -- -D warnings`: clean. `make clean-code` not run end to end (no shellcheck, as at baseline).
- `cargo test --workspace --lib --bins --no-fail-fast`: 14 binaries, 5051 passed, 2 failed (the two ledgered `run_api_wiring` tests, WINDOWS row 73). No new failure.
- `make api-surface`: "API surface unchanged" (4298 items). The suite is `#[cfg(test)]`-only (A2-50-15); no baseline, CHANGELOG or MIGRATION edit. `cargo-semver-checks` not run (no public item changed), so no `target/semver-checks` to delete.
- **Hand mutation checks** (each restored, tree clean afterwards): macro list minus `usage_fields_are_mapped` fails the self-test (16 vs 17); `CadenceLlmAdapter::model_profile` returning `None` fails the capability case; the trivial client following redirects fails the redirect case.
- **Manual credential-handling review (D-00a):** fixtures use literal test keys only (`test-key`, Gemini's `test-key-abc123`, Ollama's public placeholder `ollama`). The refusal case plants the adapter's own key in the vendor's refusal text and asserts `detail` never contains it, plus the existing credential-redaction and redirect cases still run for all nine. No production code, HTTP client, config or `Debug` impl changed.
- **Threat model:** T-50-15-01 mitigated (`RefusalShape` and `MalformedFrame` force an alternative assertion; `CASE_COUNT` self-test mutation-checked; no case returns early). T-50-15-02 mitigated (key planted and asserted absent from `detail`).

## Known Stubs

None.

## Threat Flags

None. Test code only; no endpoint, auth path, file access or schema at a trust boundary was added.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None.

## Next Phase Readiness

- Plan 50-16's docs page can state that all nine adapters are measured by one 17-case suite; its closeout can reconcile WINDOWS row 71 (carried from 50-14) and mention Anthropic's warn-and-skip stream behaviour and DeepSeek's lack of a typed refusal (both now asserted by name in the suite).
- Open owner questions carried, unchanged: the `total_tokens` decision (WINDOWS row 74) and the Kimi adapter-wide fallback profile for unseeded ids.

## Self-Check: PASSED

- Files: every entry in `key-files` exists and is committed (`conformance.rs`, the nine adapter files) or created (`deferred-items.md`, uncommitted until the docs commit); this SUMMARY present.
- Commits `e8924f90` and `2ccbd608` present in `git log`; no file deletions in either.
