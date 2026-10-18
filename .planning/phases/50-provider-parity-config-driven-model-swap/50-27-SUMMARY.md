---
phase: 50-provider-parity-config-driven-model-swap
plan: 27
subsystem: llm
tags: [rust, paladin-llm, llm-error, billed-usage, pricing, treasurer, conformance, gap-closure, g-50-5a, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5a]

requires:
  - phase: 50
    provides: "50-26 BilledUsage, LlmError::with_billed_usage / billed_usage, response_guards::bill, compat buffered billing; 50-21/50-23 folded and reconciled usage mapping per adapter"
provides:
  - "PricingLlmAdapter prices billed errors through one private helper price_billed_error: generate against BilledUsage.model (served model), generate_stream Err items against the request model; unpriced keeps cost None with the existing warn-once"
  - "Every adapter bills its EmptyCompletion / Refusal on the buffered path and on the stream's terminal Err item: compat (kimi, qwen, grok, ollama, openai-compatible), OpenAI, DeepSeek, Anthropic, Gemini; compat no-choices and Gemini no-candidates bill the reported usage"
  - "DeepSeek annotate_with_usage removed; the EmptyCompletion detail no longer embeds token counts (deliberate flip)"
  - "Conformance case billed_failures_carry_their_usage on all nine fixtures; CASE_COUNT 20"
  - "Tests priced_billed_empty_completion_carries_its_cost (tracer), unbilled_and_unpriced_errors_pass_through_generate, priced_billed_stream_error_carries_its_cost, deepseek_empty_completion_carries_its_billed_usage"
affects: [50-28, 50-29, 50-30, 50-31]

tech-stack:
  added: []
  patterns:
    - "Billed stream terminal: Err(error) => items.push(Err(bill(error, &request_model, held_usage.take())))"
    - "One pricing helper for billed errors on both decorator paths (D-09: buffered and streamed pricing cannot diverge)"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/pricing.rs
    - crates/paladin-llm/src/response_guards.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/conformance.rs

key-decisions:
  - "Stream bills name the request model (the only model identity a stream carries; Phase 31 contract), buffered bills the served model, else the request model (A3-50-26)"
  - "price_billed_error prices only a bill whose cost is None; a bill already priced is left as it is"
  - "OpenAI and DeepSeek 'no choices' stay ProcessingError (not an EmptyCompletion site); changing their classification is out of scope"
  - "Anthropic's buffered bill names response.model (parse_response has no request model; Anthropic always names its model)"

patterns-established:
  - "Billing a stream failure: bill the guard's terminal error with the held usage on the request model; deltas already yielded are unchanged"

requirements-completed: [PARITY-05, PARITY-07]

duration: 72min
completed: 2026-10-18
---

# Phase 50 Plan 27: Every adapter bills its empty and refused calls; the pricing decorator prices them (G-50-5a) Summary

**A call that a vendor billed but that ended as `EmptyCompletion` or `Refusal` now carries its folded usage on the
error, on both paths of all nine adapters. Behind `PricingLlmAdapter` it also carries its Treasurer price.**

- Tracer: a Kimi (compat) `finish_reason: length` answer with empty content and usage 1_000 / 62_000 / 63_000,
  through `PricingLlmAdapter::generate` with a 2.5 / 10 USD-per-million row for the served model, returns
  `Err(EmptyCompletion)` whose `billed_usage().cost` is 622_500_000 nanos USD. An unpriced served model gives the
  same bill with `cost: None`.
- Conformance: `billed_failures_carry_their_usage` passes on all nine fixtures (`CASE_COUNT` 20).

## Performance

- **Duration:** about 72 min (2026-10-18 about 19:20Z to 20:31Z), most of it build time with `CARGO_INCREMENTAL=0`.
- **Tasks:** 2 of 2, two commits.
  - Task 1 was the tracer. Gate (autonomous): after the commit, both Task 1 `<verify>` commands were re-run and
    exited 0. "Tracer verified end-to-end - expanding."
  - Caveat: Task 2's adapter edits were already in the working tree, uncommitted, when Task 1 was verified and
    committed. Task 1's commit holds `pricing.rs` only, and its tracer depends only on the compat buffered billing
    that 50-26 had already committed.
- **Files modified:** 8 (0 created)

## Accomplishments

- **Red first.**
  - Task 1: `priced_billed_empty_completion_carries_its_cost` failed before the helper existed. Output was
    `left: None, right: Some(Cost { nanos: 622500000, .. })`.
  - Task 2: the conformance case and the DeepSeek test were written with the adapter changes, not before them. The
    red evidence is a mutation check on the green tree instead: `bill` was changed to drop the usage, and 12 tests
    failed:
    - all 9 `billed_failures_carry_their_usage`;
    - `deepseek_empty_completion_carries_its_billed_usage`;
    - `compat_billed_empty_completion_carries_its_usage`;
    - `priced_billed_empty_completion_carries_its_cost`.

    The file was then restored and checked with `cmp`.
- **pricing.rs:**
  - `price_billed_error(table, Option<&str> model, err)` is the one helper for both paths. It prices with
    `price_or_warn`, then `with_billed_usage(billed.with_cost(cost))`.
  - `generate` passes `None`, which means the bill's served model.
  - `generate_stream` passes the request model for `Err` items.
  - Rustdoc was updated on the module, on `generate` ("billed errors are priced like successes; other errors pass
    through untouched") and on `generate_stream`.
- **Adapters.** In each, usage is mapped before classification, and the error goes through `bill`:
  - **compat** (`engine.rs`): the buffered "no choices" `EmptyCompletion` now bills too, because the mapping moved
    above it. The `[DONE]` terminal `Err` bills `held_usage` on the request model.
  - **OpenAI:** the buffered path bills the served model, else the request model. The `[DONE]` terminal is billed.
    `generate_stream` gained rustdoc.
  - **DeepSeek:** the buffered path bills, and the `[DONE]` terminal is billed. `annotate_with_usage` and its three
    tests are deleted. `deepseek_empty_stop_completion_is_empty_completion_with_usage` is flipped: the detail must
    not contain "provider-reported usage", and the bill carries total 2.
  - **Anthropic:** `parse_response` bills `map_claude_usage` on `response.model`, after `with_block_census`. The
    `message_stop` `Err` bills the merged `message_start` / `message_delta` usage on the request model.
  - **Gemini:** `parse_response` bills only when `usageMetadata` is present (never a fabricated zero, D-00c). That
    includes "no candidates". `classify_stream_items` bills the finish frame's `chunk.usage` on the request model.
- `response_guards.rs`: the temporary `#[allow(dead_code)]` on `bill` is removed. Every adapter that compiles the
  module calls it. Clippy is clean on the single-feature builds (`gemini` only, `deepseek` only).
- **conformance.rs:** case `billed_failures_carry_their_usage` added to the behaviour list. It covers the buffered
  empty, the streamed empty (last item), and the typed refusal where the wire has one. It asserts:
  - `billed_usage()` is `Some`;
  - `total == prompt + completion`, and the total is non-zero;
  - the model is named;
  - `cost` is `None`.

  `CASE_COUNT` pin and message changed to 20; the macro and module docs were updated.

## Task Commits

1. **Task 1 (tracer): PricingLlmAdapter prices billed-but-failed calls**: `c3833a14` (feat)
2. **Task 2: every adapter bills its empty and refused calls on both paths**: `e9470be6` (feat)

Both commits carry the session trailer. `c3833a14` uses the original Co-Authored-By line. `e9470be6` uses the
updated line the coordinator asked for mid-plan (`Claude Fable 5.1`). Earlier commits were not amended.

## Deviations from Plan

### Auto-fixed Issues

None.

### Other deviations

- **Red evidence for Task 2 is a mutation check, not a pre-implementation failing run** (see Accomplishments).
- **OpenAI and DeepSeek "no choices" are not billed.** They return `ProcessingError`, not `EmptyCompletion`, and
  `with_billed_usage` leaves other variants unchanged. Reclassifying them would be a behaviour change outside this
  plan. Only compat's and Gemini's no-choices/no-candidates sites are `EmptyCompletion`, and both now bill.
- **`response_guards::StreamCompletion` is unchanged.** Billing happens at each adapter's terminal call site, where
  the held usage lives, rather than inside `StreamCompletion::finish`.
- **Typed-refusal coverage.** The refusal leg of the conformance case runs on the 7 fixtures whose `refusal_shape`
  is `Typed`. Gemini and DeepSeek declare `ContentFilterOnly`, an `Ok` finish with no error to bill.
- **No CHANGELOG entry.** The plan defers documentation and changelogs to 50-31, and `make api-surface` is
  unchanged.

## Verification

All exit 0 unless stated:

- **Tests:**
  - `cargo test -p paladin-llm --all-features --lib`: 1196 passed. That is 1186, plus 9 conformance cases, plus 4
    new tests, minus 3 deleted annotate tests.
  - `cargo test -p paladin-llm --all-features --lib billed_failures_carry_their_usage`: 9 passed.
  - `cargo test --test provider_swap_matrix --features llm-all`: 51 passed.
  - `cargo test -p paladin-llm` (default features): 452 passed.
  - `cargo test --doc -p paladin-llm --all-features`: 23 passed.
- **Lint and format:**
  - `cargo fmt --all --check`: clean.
  - `cargo clippy -p paladin-llm --no-default-features --features gemini -- -D warnings`: clean. The same with
    `--features deepseek`: clean.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
- **Gates:**
  - `make check-gates`: OK.
  - `make api-surface`: unchanged (4298 items), so no `api-surface-update` or semver-checks run was needed.
  - `make check-consumer-surface`: passed.
  - `make clean-code`: exit 0.
- **Acceptance greps:**
  - `fn annotate_with_usage`: 0.
  - `bill(` per file: compat 3, openai 2, deepseek 2, anthropic 2, gemini 3.
  - `CASE_COUNT, 20`: 1.
  - `billed_usage` in `pricing.rs`: 10.
- **`make security`: exit 2, baseline, not caused by this plan.** It reports exactly RUSTSEC-2026-0367/0368/0369
  (`zip 3.0.0`) plus the allowed warnings. `Cargo.lock` is unchanged.
- **Not run:**
  - The two ledgered `build_run_api_persists_*run_traces*` lib tests (WINDOWS row 73). This plan touched no root
    crate code, so the workspace lib suite was not re-run.
  - Docker-bound integration tests.
  - Live vendor calls (no key; wire requests are unchanged).
- **`unwrap(` / `expect(` / `panic!(` in the diff:** all are in `#[cfg(test)]` code. That means `conformance.rs`,
  which is test-only, and the adapters' and `pricing.rs` test modules. None were added in non-test code.
- **Manual credential-handling review:**
  - A bill carries a model id, `TokenUsage` counts and an `Option<Cost>`. No response text and no key reaches it.
  - `Refusal.detail` is still built only through `diagnostic_excerpt`, which redacts and then bounds (D-00a). The
    Anthropic census still redacts.
  - No new log line. The only pricing log is the existing unpriced warn, which carries the model name only
    (T-50-27-04).
  - No HTTP client, redirect policy or header changed.
  - DeepSeek's detail now carries less text than before.

## Known Stubs

None.

## Threat Flags

None. Against the register:
- T-50-27-01 (mitigated): bills on both paths of all nine adapters, proved by the conformance case.
- T-50-27-02 (mitigated): one helper prices both paths, with a test on each.
- T-50-27-03 (mitigated): an unpriced model keeps `cost: None`, tested on both paths.
- T-50-27-04 (mitigated): no new log line.

## Flagged assumptions

- A1-50-27 held. The shared empty-stop bodies, the stream bodies and the typed refusal bodies all carry usage, so
  no new fixture hook was needed.
- A2-50-27 held. Only the terminal `Err` item changes; already-yielded deltas are untouched (asserted in
  `priced_billed_stream_error_carries_its_cost`).

## Self-Check: PASSED

- All 8 modified files exist. Commits `c3833a14` and `e9470be6` are on `claude/laughing-dirac-e0h2ax`.
- `grep -q billed_failures_carry_their_usage crates/paladin-llm/src/conformance.rs` and
  `grep -q billed_usage crates/paladin-llm/src/pricing.rs` both succeed.
