---
phase: 51-adjustable-reasoning-effort
plan: 12
subsystem: llm-adapters
tags: [reasoning-effort, anthropic, openai, deepseek, gemini, seeded-registry, effort-probe, conformance, round-trip-invariant]
status: complete

requires:
  - phase: 51-01
    provides: "EffortSupport, EffortLevels, ModelProfile::with_effort / with_effort_verified_on, the unset-effort goldens, CASE_COUNT"
  - phase: 51-03
    provides: "DeepSeek and OpenAI map_reasoning, DEEPSEEK_REASONING_WIRE, OPENAI_REASONING_WIRE"
  - phase: 51-05
    provides: "Anthropic and Gemini map_reasoning, ANTHROPIC_REASONING_WIRE, GEMINI_REASONING_WIRE, the Gemini spelling fn"
  - phase: 51-10
    provides: "51-EFFORT-PROBE.md, the only source effort data may be seeded from"
  - phase: 51-11
    provides: "the owner's wire decisions, the `// Source for the effort data` seeding pattern"
provides:
  - "Probe-evidenced effort data on every probed Anthropic (11), OpenAI (17), DeepSeek (2) and Gemini (12) seeded row, each with a source comment and effort_verified_on 2026-10-22"
  - "The four native caps constants confirmed (not WireCaps::NONE), with the verdict rows recorded beside each"
  - "Gemini's lowercase thinkingLevel confirmed and pinned by a wire-body test (no spelling change)"
  - "Conformance suite 21 -> 24 cases and the required fixture method effort_wire_value; the assumption-delta round-trip invariant over every seeded row of every adapter"
  - "registry::seeded_tests::every_seeded_effort_entry_validates"
affects: [51-13, 51-14]

tech-stack:
  added: []
  patterns:
    - "per-vendor `*_effort_seed_matches_the_transcript` tests type the expected table from the transcript and read it through the adapter's model_profile"
    - "conformance cases discover seeded ids from the registry tables (`seeded_models(provider)`), so a new seeded row joins the round-trip invariant with no test edit"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/registry/seeded.rs
    - crates/paladin-llm/src/registry/mod.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - crates/paladin-llm/src/conformance.rs
    - crates/paladin-llm/CHANGELOG.md
    - CHANGELOG.md

key-decisions:
  - "All four native forms are confirmed in the verdict table, so every caps constant keeps its documented capabilities (owner, 2026-10-22); none is WireCaps::NONE."
  - "No native row carries a budget span: neither the Anthropic nor the Gemini part of the probe file's vendor-documented section names a fixed maximum, and the seed rule needs one."
  - "enable_thinking_with_level is set on exactly claude-opus-4-8, claude-opus-4-7, claude-opus-4-6, claude-sonnet-4-6."
  - "DeepSeek's documented silent coercions (minimal, medium, xhigh) are seeded as `coerced`, so they refuse whatever the fallback."

patterns-established:
  - "Plan 51-13/14 can rely on `ConformanceFixture::effort_wire_value` as the single statement of where each adapter puts a level."

requirements-completed: []
requirements-progress: [REASON-02, REASON-03, REASON-04, REASON-05, REASON-06]

duration: about 3h (cold worktree build, tracer checkpoint round trip)
completed: 2026-10-22
---

# Phase 51 Plan 12: Native adapters' probe-confirmed caps, measured effort and nine-adapter conformance Summary

**The four native adapters now ship only what `51-EFFORT-PROBE.md` evidences: 42 seeded rows with dated per-model effort data, caps recorded as confirmed, and a 24-case conformance suite whose new cases prove every adapter refuses unmeasured effort before HTTP and that every seeded level reaches the wire as exactly that level.**

REASON-02 to REASON-06 advance but are not marked complete: the user guide and ADR (51-13) and the surface register (51-14) remain. `requirements-completed` is empty and REQUIREMENTS.md is untouched. STATE.md and ROADMAP.md are untouched (the orchestrator owns them).

## Checkpoint decisions applied

The wire decisions are the owner's, taken 2026-10-22 at plan 51-11's checkpoint (`51-11-SUMMARY.md`, `## Checkpoint decision`) and applied here without re-asking: OpenAI top-level `reasoning_effort`, DeepSeek top-level `reasoning_effort` with its `thinking` off/auto rows, Anthropic `output_config.effort` with the pairing set, Gemini lowercase `thinkingLevel`, D-02 `unmeasured`. The Task 1 tracer gate was resolved by the user ("approved") on 2026-10-22, including the five judgement calls listed under Deviations.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: `deepseek-v4-pro` ships its probe-confirmed caps and measured levels, end to end through `DeepSeekAdapter` | `e21e8e1a` |
| 2 | Anthropic, OpenAI and Gemini rows: confirmed caps, measured levels, the Anthropic pairing, Gemini's spelling | `8d305d92` |
| 3 | Nine-adapter conformance cases, the round-trip invariant, the reasoning-surface audit, the CHANGELOG record | `9fc185ef` |

## What was built

### Seeded effort data (all dated 2026-10-22, commit `32ce9dd0`, source comment on every row)

Seed rules (plan 51-11's, applied mechanically): `levels` = accepted through the confirmed form; `coerced` = accepted levels the vendor-documented section lists as run as another; `off` = the off form accepted with no reasoning tokens reported (a count of 0 or no count); `auto` = the auto form (for a top-level or default-call form, the default call) accepted with more than 0 reasoning tokens; `budget` only with a documented maximum.

- **DeepSeek** (`deepseek-flash`, `deepseek-v4-pro`, identical): levels `low`, `high`, `max`; coerced `minimal`, `medium`, `xhigh` (api-docs.deepseek.com, accessed 2026-10-22: minimal to low, medium to high, xhigh to high); `off`; `auto` (17 and 110 reasoning tokens). No `enable_thinking_with_level` (no paired row; the default call already reasoned).
- **Anthropic** (11 rows):

  | model | levels | off | auto | paired |
  |---|---|---|---|---|
  | claude-haiku-5-5 | low..max | yes | yes (95) | no |
  | claude-opus-4-8 | low..max | yes | no (0) | yes |
  | claude-fable-5-1, claude-fable-5, claude-opus-5-5 | low..max | no (400) | yes (30, 11, 8) | no |
  | claude-opus-5 | low..max | yes | yes (12) | no |
  | claude-sonnet-5-5 | low..max | no (400) | no (0) | no |
  | claude-sonnet-5 | low..max | yes | no (0) | no |
  | claude-opus-4-7 | low..max | yes | no (0) | yes |
  | claude-opus-4-6, claude-sonnet-4-6 | low, medium, high, max (xhigh 400) | yes (no count) | yes (46) | yes |

- **OpenAI** (17 probed rows; `gpt-4o-mini` and `chat-latest` stay `Unmeasured`, no date): levels `low`, `medium`, `high`, `xhigh` on all except `gpt-5.1` (`low`, `medium`, `high`) and `gpt-5`, `gpt-5-mini`, `gpt-5-nano` (`minimal`..`high`). `off` (`none`) true on 11 models; false (the `none` row drew 400) on `gpt-6.1-sol`, `gpt-6-astra`, `gpt-5`, `gpt-5-mini`, `gpt-5-nano`, `o3`. `auto` true where the default call reported reasoning tokens: `gpt-6.1-sol`, `gpt-6-sol`, `gpt-6-luna`, `gpt-6-astra`, `gpt-5.6-luna`, `gpt-5.6-terra`, `gpt-5.5`, `gpt-5`, `gpt-5-nano`; false on `gpt-5.4-nano`, `gpt-5.6-sol`, `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.2`, `gpt-5.1`, `gpt-5-mini`, `o3` (default 0).
- **Gemini** (12 rows): `low`, `medium`, `high` on all; `minimal` additionally on `gemini-flash-lite-latest`, `gemini-3.1-flash-lite`, `gemini-3-flash-preview`, `gemini-3.5-flash-lite`, `gemini-3.6-flash`, `gemini-3.5-flash`. `off` (`thinkingBudget` 0) true on `gemini-4-argon`, `gemini-3.1-flash-lite`, `gemini-3-flash-preview`, `gemini-3.6-flash`, `gemini-3.5-flash`. `auto` true except `gemini-flash-lite-latest`, `gemini-3.1-flash-lite`, `gemini-3.5-flash-lite` (no count on the default call).

### Caps

`DEEPSEEK_REASONING_WIRE`, `OPENAI_REASONING_WIRE`, `ANTHROPIC_REASONING_WIRE` and `GEMINI_REASONING_WIRE` keep their documented capabilities; each doc comment now records the decision, its date and the verdict rows (OpenAI parsed 17 of 17, effect 14 of 17; Anthropic parsed 11 of 11, effect 7 of 9; DeepSeek parsed 2 of 2, effect 2 of 2; Gemini parsed 12 of 12, effect 5 of 5). Gemini's `thinking_level_spelling` doc now records the lowercase confirmation; no spelling change was needed, and `thinking_level_is_sent_in_lowercase_on_every_seeded_level` pins it on all 12 rows.

### Conformance (suite 21 to 24; `CASE_COUNT` and its pinned test updated)

- `ConformanceFixture::effort_wire_value(body) -> Option<String>`: required, no default; all nine fixtures plus the macro's own `TrivialFixture` state it (top-level `reasoning_effort` for OpenAI, DeepSeek and Grok; `thinking.effort` only for Kimi, so a top-level `reasoning_effort` can never read as a level; both for the two self-hosted adapters; `output_config.effort` for Anthropic; lower-cased `generationConfig.thinkingConfig.thinkingLevel` for Gemini; `None` for Qwen and the trivial transport).
- `explicit_reasoning_on_an_unprofiled_model_refuses_before_http`: `with_reasoning(High)` on `conformance-unprofiled-model` is `UnsupportedParameter`, `Permanent`, on buffered and streaming paths, against a mock that expects zero requests. 9 of 9 pass.
- `seeded_effort_levels_reach_the_wire`: the first seeded model whose profile lists a level sends the unset body plus only the effort field(s) and `effort_wire_value` reads the level back. Fixtures with no such model (Qwen: budget-only rows; Ollama and OpenAI-compatible: no table, default model) take the every-id-refuses branch. 9 of 9 pass.
- `every_seeded_effort_level_round_trips_to_its_wire_body`: every seeded row of every adapter, every listed level. 9 of 9 pass. Mutation check: reading the wrong field name in the shared reader made the OpenAI, DeepSeek and Grok instances of both wire cases fail, so the cases are not vacuous.

## web3sec models

- `claude-opus-4-8`: accepts every level the transcript evidences (`low`, `medium`, `high`, `xhigh`, `max`) through the shipped adapter, each sent as `output_config.effort` beside `thinking: {type: adaptive}` as the two trailing body fields (`claude_opus_4_8_takes_every_evidenced_level_through_the_shipped_adapter`). No level failed.
- `deepseek-v4-pro`: `low`, `high`, `max` reach the wire as `reasoning_effort`. The transcript also shows `minimal`, `medium` and `xhigh` accepted by the vendor, but the vendor-documented section runs them as `low`, `high` and `high`, so those three refuse with zero hits under every fallback. This is a deliberate refusal of a silent substitution, not a failed row.
- `kimi-k3` (plan 51-11): `low`, `high`, `max` reach the wire as `thinking.effort`; unchanged here and covered by the new round-trip invariant.

## REASON-05 / D-00g reasoning-surface audit

Finding: no gap on any adapter; nothing was added and no second reasoning surface exists. "Text" is `LlmResponse.reasoning` and `StreamingResponse.reasoning`; "tokens" is `TokenUsage.reasoning_tokens`.

| adapter | receives reasoning text | buffered text | streamed text | reasoning_tokens (both paths) |
|---|---|---|---|---|
| OpenAI | no (Chat Completions never returns it; D-00g forbids asking) | n/a | n/a | filled by `map_usage` (buffered and stream terminal) |
| Anthropic | yes (thinking blocks) | filled (`concat_thinking_blocks`) | filled (thinking deltas) | filled by shared `map_claude_usage` from `output_tokens_details.thinking_tokens` |
| DeepSeek | yes (`reasoning_content`) | filled | filled | filled by `map_usage` |
| Gemini | no (`includeThoughts` is never sent, pinned by a test) | n/a | n/a | filled by shared `map_gemini_usage` from `thoughtsTokenCount` (buffered and terminal frame) |
| Kimi | yes (`reasoning_content`) | filled (shared engine) | filled | filled (shared engine) |
| Qwen | yes | filled | filled | filled |
| Grok | yes | filled | filled | filled |
| Ollama | reads `reasoning_content` only; the provider was SKIPPED in the probe, so what the daemon returns is unmeasured | filled if sent | filled if sent | filled if reported |
| OpenAI-compatible | reads `reasoning_content` only; SKIPPED in the probe | filled if sent | filled if sent | filled if reported |

The conformance case `usage_fields_are_mapped` already proves the token mapping for all nine; text is covered by the existing `deepseek_buffered_reasoning_text_is_surfaced`, `deepseek_stream_reasoning_keeps_wire_order`, the engine's reasoning tests and the Anthropic stream test. I did not add support for any other reasoning-text field name (for example Ollama's `reasoning`): no probe evidence exists, and adding one would be a new surface.

## Seed trace (manual review)

Every native effort value traces to a quoted `51-EFFORT-PROBE.md` row: the source comment above each `.with_effort(..)` names the section (`### <model>` under `## <Vendor>`, run number, date, commit), the rows used and the reasoning-token figure behind each `off` and `auto` call. The only non-transcript inputs are the DeepSeek coercions, from the probe file's `## Vendor-documented bounds and coercions` (URL and access date in the row comment). Nothing is seeded from memory or outside that file.

## A2-51-12 (sampling together with effort)

Recorded only; no seeded sampling row changed. The `high+temperature=0.5` row drew 400 on all 17 OpenAI models, on 9 of 11 Anthropic models (all except `claude-opus-4-6` and `claude-sonnet-4-6`), on `kimi-k3` and `kimi-k2.6`, and on `grok-4.20-reasoning`; it drew 200 on both DeepSeek models, all 12 Gemini models and the other four Grok models. The existing seeded sampling rows (for example Anthropic temperature fixed at 1.0, OpenAI fixed group) already express the vendor's own restrictions.

## Verification

- `cargo test --offline -p paladin-llm --all-features`: lib 1480 passed, 0 failed (was 1432 before this plan); `--doc` 35 passed. Task verify filters (`deepseek` 126, `registry::` 74, `anthropic`/`openai`/`gemini` plus `registry::` 592, the three conformance cases 9 each) all green; `suite_generates_the_full_case_list_for_a_fixture` pins 24; all nine unset-effort goldens pass (the guard stayed green after every task).
- `cargo clippy --offline -p paladin-llm --all-targets --all-features -- -D warnings` clean; `cargo fmt --check -p paladin-llm` clean; `make api-surface` "API surface unchanged" (4323 items; the trait is `#[cfg(test)]`-only, no public item added); `cargo test --offline --workspace --all-features --no-run` compiles.
- Not run: the full `make clean-code` chain, `make security` (no dependency added or changed) and the workspace test run; the orchestrator runs the hook chain over the merged wave. Commits used `--no-verify` as the wave allows.
- Manual credential-handling review (security.instructions.md): no client, header, redirect-policy, `Debug` or error-body path was touched. Refusal text is built from provider, model, effort and override-key names only. No vendor call was made; no key value was read, printed or committed.

## TDD record

- **Task 1:** the seeded rows were written before the tests (order inverted, recorded under Deviations). RED was then shown by reverting `seeded.rs` and re-running: 3 of the 5 new DeepSeek tests failed (seed matches, level wire, off/auto wire); the coerced and budget cases are refusal guards that pass either way. GREEN restored.
- **Task 2:** same order and same RED check: 11 of the new tests across the three adapters failed with the seed reverted. GREEN: 592 tests across the four filters.
- **Task 3:** RED first: adding the required trait method and the three cases made the build fail for all ten fixtures (nine adapters plus `TrivialFixture`, 10 `E0046` errors). GREEN: implemented `effort_wire_value` everywhere; all 27 new tests pass; mutation check above.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Existing tests used newly seeded ids as their "unmeasured model"**
- **Found during:** Task 1 (DeepSeek: `deepseek-flash`), Task 2 (Anthropic: `claude-opus-4-8`; Gemini: `gemini-4-argon`).
- **Fix:** moved those four refusal/profile tests to unprofiled ids (`deepseek-unprofiled-model`, `claude-unprofiled-test-model`, `gemini-unprofiled-test-model`). Their assertions are unchanged.
- **Commits:** `e21e8e1a`, `8d305d92`.

### Judgement calls (approved by the user at the tracer checkpoint) and plan-wording notes

1. **No Anthropic or Gemini budget bounds.** The seed rule needs the maximum in the probe file's vendor-documented section. Anthropic's entry gives a 1,024 minimum and "less than max_tokens" but no fixed upper bound; Gemini has no documented bound (no 2.5 id was probed). So no native row carries a `budget` span and an explicit `Budget` refuses on every native model until an operator override states one. The plan's truth ("with the documented max") assumed a documented maximum that the file does not contain. The facts for a future widening: `claude-opus-4-6` and `claude-sonnet-4-6` accepted `budget_tokens` 1024 and rejected 1023 (the owner text said the 4.6 rows "drew 400"; only 4.7 and later did); Gemini accepted `thinkingBudget` 128 and 1 and rejected -5 with a body naming the range `[-1, 65535]`. Test: `budget_refuses` assertions in each adapter's seed tests (`a_level_the_probe_..._and_every_budget_refuse_before_http`, `a_budget_refuses_on_the_seeded_rows`).
2. **Gemini `minimal` is seeded per row.** The owner text said `minimal` was rejected on 11 of 12 models; the file shows 400 on 6 and 200 (no count) on 6. Seeded from the rows. The lowercase spelling is kept (uppercase `HIGH` was also accepted).
3. **`off` and `auto` read literally.** `off` counts as "no reasoning tokens reported" when the row carried 0 or no count (applies to DeepSeek, `claude-opus-4-6`, `claude-sonnet-4-6` and several Gemini models, as in plan 51-11). `auto` counts only where the default call (top-level and default-call forms) or the adaptive row reported more than 0 reasoning tokens. Consequence: eight OpenAI models, four Anthropic models and three Gemini models carry no `auto` although they accept the default call; a caller on those models cannot request `Auto` explicitly.
4. **`enable_thinking_with_level`** is set on exactly `claude-opus-4-8`, `claude-opus-4-7`, `claude-opus-4-6`, `claude-sonnet-4-6`, the owner's pairing set; it also matches the paired-versus-alone table (alone 0 or no count, paired 41, 50, 48, 48).
5. **Unprobed seeded rows stay `Unmeasured` with no date:** OpenAI `gpt-4o-mini` and `chat-latest` (D-02, the owner's answer "unmeasured").
6. **Task 1 and Task 2 RED order inverted.** The seed data (GREEN) was written before the seed-matches and wire tests; RED was demonstrated afterwards by reverting `seeded.rs` and confirming the tests fail (3 of 5 and 11), then restoring it.
7. **Files beyond `files_modified`:** `registry/mod.rs` (the `every_seeded_effort_entry_validates` test the plan asked for "if plan 51-11 did not" add it; it did not) and `kimi`, `qwen`, `grok`, `ollama`, `openai_compatible` adapter test modules (their conformance fixtures need `effort_wire_value`).
8. **Test-only helper naming:** the invariant reads seeded ids through `registry::seeded` (no new public item). For a fixture with no seeded table the wire case measures its default model on the every-id-refuses branch, as the plan states.
9. **Owner-text counts.** Other owner-text counts differ slightly from the file but did not change a seed: Anthropic "7 of 9 effect" and Gemini "5 of 5 effect" are the verdict-table figures quoted in the caps comments.

## Authentication Gates

None. No live vendor call was made.

## Known Stubs

None. Deliberate limits, all in the CHANGELOG: `gpt-4o-mini`, `chat-latest` and any unseeded id refuse an explicit effort until probed; no native model accepts an explicit `Budget`.

## Threat Flags

None. No endpoint, auth path or file access was added. T-51-36 (an unconfirmed native form or desk-research level shipped) is mitigated by the recorded caps decisions, the seed rules, the per-row source comments and the seed-matches-transcript tests; T-51-37 (a seeded level the adapter does not send) by `every_seeded_effort_level_round_trips_to_its_wire_body` over every seeded row and level.

## Self-Check: PASSED

- Files: this SUMMARY and every file in `key-files.modified` present; `git status` clean before this file.
- Commits `e21e8e1a`, `8d305d92` and `9fc185ef` present on `worktree-agent-a42bd9a73ef508303`.
- `grep -c '\.with_effort(' crates/paladin-llm/src/registry/seeded.rs` is 62 and each carries a `Source for the effort data` comment; `fn effort_wire_value(` in `conformance.rs` has no default body.
