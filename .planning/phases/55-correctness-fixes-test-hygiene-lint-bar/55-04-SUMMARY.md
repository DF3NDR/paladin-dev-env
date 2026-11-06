---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 04
subsystem: paladin-llm
tags: [clippy, cfg-gates, lint, self-hosted-providers, seed-registry, tests-03, tests-04, tests-05, fixes-05]
status: complete

requires:
  - phase: 55-correctness-fixes-test-hygiene-lint-bar
    provides: "plans 55-01..55-03 landed; no code dependency"
provides:
  - "cargo clippy -p paladin-llm --tests exits 0 under -D warnings for default features and every one of the 12 features taken alone"
  - "SELF_HOSTED_PROVIDERS as the single pub(crate) definition of the self-hosted provider pair, read by both the reasoning-wire layering refusal and the effort refusal message"
  - "claude-opus-4-8 seed row (reasoning: false) with a comment and a registry pin citing the 50.1 and 51 probe records"
affects: [55-13]

tech-stack:
  added: []
  patterns:
    - "Test-only helpers are cfg-gated to the features of their users, never silenced with allow/expect (D-10)"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/provider_factory.rs
    - crates/paladin-llm/src/conformance.rs
    - crates/paladin-llm/src/cadence.rs
    - crates/paladin-llm/src/fallback.rs
    - crates/paladin-llm/src/reasoning.rs
    - crates/paladin-llm/src/config/layers.rs
    - crates/paladin-llm/src/registry/seeded.rs
    - crates/paladin-llm/src/registry/mod.rs

key-decisions:
  - "mock alone is in the clippy matrix; the reasoning_defaults module is gated on any provider feature (PD-04-1)"
  - "reasoning.rs owns SELF_HOSTED_PROVIDERS (pub(crate)); its rustdoc absorbs layers.rs's paragraph (PD-04-2)"
  - "claude-opus-4-8 keeps reasoning: false; comment and pin corrected, no flip, no probe, no CHANGELOG entry (PD-04-3)"

patterns-established:
  - "When one gate exposes a newly unused sibling, gate the sibling on the same feature list"

requirements-completed: [TESTS-03, TESTS-04, TESTS-05, FIXES-05]

duration: ~50 min
completed: 2026-11-01
---

# Phase 55 Plan 04: paladin-llm single-feature lint bar, one self-hosted pair, opus-4-8 seed evidence Summary

**`paladin-llm`'s test build is clippy-clean under `-D warnings` for default features and all 12 single features (cfg gates only, no suppression), the self-hosted provider pair is defined once as `pub(crate) SELF_HOSTED_PROVIDERS`, and the `claude-opus-4-8` row keeps `reasoning: false` with a comment and pin that cite the probe records.**

## Plan-start SHA

`dea69385ccf595d97287f58c42d51eadbdf81b89`

## Performance

- **Tasks:** 3 of 3 (Task 1 is the tracer; its gate ran in auto mode and passed, see Deviations)
- **Files modified:** 8 (0 created)
- **Commits:** `255266661` (Task 1), `3bd4a18fd` (Task 2), `2e0466b7a` (Task 3)

## Clippy matrix (before)

Measured at the plan-start SHA with `cargo clippy -p paladin-llm --tests --offline [--no-default-features --features <f>]`, counting `^warning:` lines excluding the `generated N warnings` summary. All 13 counts equal RESEARCH's.

| Combination | Count | Warnings |
|-------------|-------|----------|
| default | 1 | unused import `ReasoningFallback` |
| openai | 5 | `ReasoningFallback`; `PinnedSpread`; `pin_waiter_spread`; `ProfiledStubPort`; `new` and `asked` |
| anthropic | 7 | `ReasoningFallback`; unused variable `chat`; `PinnedSpread`; `pin_waiter_spread`; `chat_effort_wire_value`; `ProfiledStubPort`; `new` and `asked` |
| deepseek | 4 | `PinnedSpread`; `pin_waiter_spread`; `ProfiledStubPort`; `new` and `asked` |
| kimi | 6 | `ReasoningFallback`; `PinnedSpread`; `pin_waiter_spread`; `chat_effort_wire_value`; `ProfiledStubPort`; `new` and `asked` |
| qwen | 8 | `ReasoningFallback`; `PinnedSpread`; `pin_waiter_spread`; `chat_effort_wire_value`; `ProfiledStubPort`; `new` and `asked`; `level_spelling`; `levels` |
| grok | 5 | `ReasoningFallback`; `PinnedSpread`; `pin_waiter_spread`; `ProfiledStubPort`; `new` and `asked` |
| ollama | 5 | same five as grok |
| openai-compatible | 5 | same five as grok |
| gemini | 7 | `ReasoningFallback`; `chat`; `PinnedSpread`; `pin_waiter_spread`; `chat_effort_wire_value`; `ProfiledStubPort`; `new` and `asked` |
| mock | 10 | `ReasoningFallback`; macro `probe`; `chat`; `Case`; `UNPROFILED`; `level_spelling`; `levels`; `send`; `stamped`; `assert_default_applies` |
| vision | 5 | same five as grok |
| openai-embeddings | 5 | same five as grok |

`thinking_effort_wire_value` appears in none of the 13 runs.

## Clippy matrix (after)

`cargo clippy -p paladin-llm --tests --offline [flags] -- -D warnings`, run after Task 2 and again after Task 3 (the Task 3 run is the final one):

| Combination | Exit | Warnings |
|-------------|------|----------|
| default | 0 | 0 |
| openai | 0 | 0 |
| anthropic | 0 | 0 |
| deepseek | 0 | 0 |
| kimi | 0 | 0 |
| qwen | 0 | 0 |
| grok | 0 | 0 |
| ollama | 0 | 0 |
| openai-compatible | 0 | 0 |
| gemini | 0 | 0 |
| mock | 0 | 0 |
| vision | 0 | 0 |
| openai-embeddings | 0 | 0 |

`git diff dea69385 -- crates/paladin-llm/src | grep '^+' | grep -cE '#\[(allow|expect)\('` prints `0`. The pre-existing `#[allow(dead_code)]` on `env_of` and `#[allow(unused_mut, clippy::vec_init_then_push)]` on `wire_cases` were not touched.

## TESTS-04 closed sites

Three of the four named sites were fixed in plan 51-14; the fourth (the unused `ReasoningFallback` import) is fixed here (Task 1).

- `cargo doc -p paladin-battalion --no-deps --offline 2>&1 | grep -c '^warning'` printed `0` (the `NODE_CACHE_KEY_VERSION` link).
- `cargo doc -p paladin-llm --no-deps --offline 2>&1 | grep -A3 'redundant explicit link' | grep -c 'reasoning.rs'` printed `0`.
- `thinking_effort_wire_value` appears in none of the 13 before runs (`grep -c` on the saved baseline printed `0`) or the 13 after runs.
- The 51-14 record, `.planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/deferred-items.md` ("Resolved in plan 51-14"): "`NODE_CACHE_KEY_VERSION` is now a path-qualified link in `cache_key.rs`; the two redundant link targets in `reasoning.rs` are plain `[`ReasoningFallback`]` / `[`ReasoningEffort`]` links; `thinking_effort_wire_value` in `conformance.rs` is gated on the three adapter features (`kimi`, `ollama`, `openai-compatible`) whose fixtures call it."

## TESTS-05 (51 IN-01) reproduced and fixed

`grep -rn '\["ollama", "openai-compatible"\]' crates/paladin-llm/src` before the change hit `reasoning.rs:257` (`SELF_HOSTED_PROVIDERS`), `config/layers.rs:342` (`REASONING_WIRE_PROVIDERS`) and a test loop literal at `reasoning.rs:700`. After the change it has exactly one hit, the `pub(crate)` definition. The new layering test was watched failing (`error[E0603]: constant SELF_HOSTED_PROVIDERS is private`) before the widening.

## FIXES-05 verification

D-06 as reconciled: `claude-opus-4-8` keeps `reasoning: false`. No default-call row shows reasoning text.

- 50.1 probe 1 (`50.1-SEED-PROBE.md`, `### Probes: claude-opus-4-8`, row 1, default parameters): last column `reasoning text` = `absent`.
- 51 effort probe (`51-EFFORT-PROBE.md`, `### claude-opus-4-8`): `| 1 | default | - | 200 | end_turn | 0 | 37 |`; `output_config` rows 2-6 all `0`; `output_config+adaptive` rows 7-10 `0`, row 11 `max` `| 41 | 44 |`; `thinking_object` `off` and `auto` `0`.
- `crates/paladin-ports/src/output/model_profile.rs` contract: "Whether the default call to this model **returned reasoning text**."

The 41 is on a non-default `max` + `thinking: adaptive` request, outside the field's contract. No live probe and no key were used. Plan 55-13's `## FIXES-05 verification` can quote these lines.

## Accomplishments

- Gated the `reasoning_defaults` module on any provider feature, and `ReasoningFallback`, `level_spelling`, `levels`, the `wire_cases()` `chat` closure, `chat_response` and the `json` import to the features of their users (`provider_factory.rs`).
- Gated `chat_effort_wire_value` on `openai`, `deepseek`, `grok`, `ollama`, `openai-compatible` (its five callers), copying `thinking_effort_wire_value`'s shape.
- Gated `PinnedSpread`, its `Drop`, `pin_waiter_spread` and the whole `fallback::test_support` module on `all(test, feature = "mock")`.
- Made `SELF_HOSTED_PROVIDERS` `pub(crate)` in `reasoning.rs` with merged rustdoc; `layers.rs` imports it at the refusal and in the test loop and no longer defines a pair.
- Added `reasoning_wire_providers_are_the_self_hosted_pair` (layering site); the existing `wire_unsupported_names_reasoning_wire_only_on_self_hosted_providers` test is renamed `wire_refusal_points_self_hosted_providers_at_reasoning_wire` and now iterates the constant.
- Corrected the `claude_opus_4_8` comment and the `anthropic_opus_4_8_seed_matches_the_transcript` pin comment to cite the probe rows.

## Verification

- Default lib tests: 546 passed (545 before the one new test). `--features deepseek` lib tests: 429 passed (Task 1). `anthropic_opus_4_8_seed_matches_the_transcript` passes under `--features anthropic`.
- `cargo fmt --check` exits 0. `make api-surface` reports "API surface unchanged" (4325 items). `git diff dea69385 -- crates/paladin-llm/CHANGELOG.md` is empty.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Gating exposed three more single-feature warnings**
- **Found during:** Task 1, step 4 re-measure
- **Issue:** `chat_response` became unused once `chat` was gated (`anthropic`, `gemini`, `mock`); the `json` import became unused under `mock`; and the `probe` macro in `registry/mod.rs` (`reasoning_default_builders_work_on_every_config`) is not in `reasoning_defaults` as the plan's class 6 stated, so the module gate did not reach it.
- **Fix:** gated `chat_response` and the `json` import on their users' features, and gated the whole `reasoning_default_builders_work_on_every_config` test on any provider feature (it has nothing to probe under `mock` alone).
- **Files modified:** `crates/paladin-llm/src/provider_factory.rs`, `crates/paladin-llm/src/registry/mod.rs`
- **Commit:** `255266661`

**2. [Plan wording] Pair assertion written without the array literal**
- The plan's `<behavior>` asserts `SELF_HOSTED_PROVIDERS == ["ollama", "openai-compatible"]`, but Task 3's verify requires the literal to appear exactly once. The test asserts `len() == 2` and `contains` for each name instead; the check is equivalent.

**3. [Plan wording] Existing test renamed instead of a second one added**
- `<behavior>` allows extending an existing test that already pins the refusal-message site. `wire_unsupported_names_reasoning_wire_only_on_self_hosted_providers` did exactly that over a literal, so it was renamed to the artifact name and made to iterate the constant.

**4. [Tracer gate] Task 1's literal verify cannot pass until Task 2**
- Task 1's `<verify>` includes `cargo clippy ... --features deepseek -- -D warnings`, but class 3 (the `cadence` / `fallback` helpers) is assigned to Task 2. The tracer gate ran in auto mode: default-feature clippy exited 0 and the lib tests passed; under `deepseek` the only diagnostics were the four class 3 items. That was the planned interim state, not a broken foundation, so execution continued.

## Deferred Issues

- `cargo test -p paladin-llm --offline --lib --no-default-features --features mock` has 3 failures (375 pass): `every_config_recognised_and_compiled_provider_name_resolves_through_create`, `from_config::concurrent_create_from_config_builds_independent_adapters`, `from_config::create_from_config_builds_each_provider_from_its_block`. They are deliberate "at least one provider is compiled" guards (`provider_factory.rs` lines 828, 1359, 1759), unchanged by this plan, and fail in any provider-less build. Task 2's step 4 asked for that run to pass; it cannot, and the plan's own matrix (clippy, which compiles the tests) is unaffected. Out of scope (pre-existing); not fixed.

## Known Stubs

None.

## Threat Flags

None. The only visibility change is `pub(crate)`; no endpoint, auth path or trust-boundary surface was added.

## Notes

- No disk-space intervention was needed (`target/debug/incremental` was not deleted; `CARGO_INCREMENTAL=0` throughout, about 6.5 GB free at the end).
- No model identifier appears in code, comments, docs or planning artifacts.

## Self-Check: PASSED

- All 8 modified files exist; commits `255266661`, `3bd4a18fd` and `2e0466b7a` are present in `git log`.
- `grep -n 'pub(crate) const SELF_HOSTED_PROVIDERS'` matches in `reasoning.rs`; `layers.rs` holds 4 references; the pair literal occurs once; `reasoning text: absent` is in `seeded.rs`.
