---
phase: 51-adjustable-reasoning-effort
plan: 01
subsystem: llm-ports
tags: [reasoning-effort, model-profile, admission-rule, golden-bodies, llm-error]
status: complete

requires: []
provides:
  - "ReasoningEffort / ReasoningFallback in paladin-core (facade module paladin::core::platform::container::reasoning)"
  - "EffortSupport::resolve (the one admission rule) and EffortSupport::intersect (AND semantics)"
  - "LlmRequest.reasoning / reasoning_fallback and LlmError::UnsupportedParameter"
  - "Nine raw-byte golden request bodies for an unset effort and the conformance case that guards them"
affects: [51-02, 51-03, 51-04, 51-05, 51-06, 51-07, 51-08, 51-09, 51-11, 51-12, 51-13, 51-14]

tech-stack:
  added: []
  patterns:
    - "single pure admission function on the profile (ParamSupport::admits precedent)"
    - "struct-variant error + infallible constructor + #[non_exhaustive] (Refusal precedent)"
    - "raw-byte golden body compared in a shared conformance suite, bless switch UPDATE_REASONING_GOLDEN=1"

key-files:
  created:
    - crates/paladin-core/src/platform/container/reasoning.rs
    - crates/paladin-llm/tests/golden/reasoning_unset/anthropic.json
    - crates/paladin-llm/tests/golden/reasoning_unset/deepseek.json
    - crates/paladin-llm/tests/golden/reasoning_unset/gemini.json
    - crates/paladin-llm/tests/golden/reasoning_unset/grok.json
    - crates/paladin-llm/tests/golden/reasoning_unset/kimi.json
    - crates/paladin-llm/tests/golden/reasoning_unset/ollama.json
    - crates/paladin-llm/tests/golden/reasoning_unset/openai-compatible.json
    - crates/paladin-llm/tests/golden/reasoning_unset/openai.json
    - crates/paladin-llm/tests/golden/reasoning_unset/qwen.json
  modified:
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-core/src/platform/container/token_usage.rs
    - src/core/platform/mod.rs
    - crates/paladin-ports/src/output/model_profile.rs
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-battalion/src/llm_failure.rs
    - crates/paladin-battalion/src/llm_decision.rs
    - crates/paladin-llm/src/conformance.rs
    - tests/consumer_surface_muster.rs
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "LlmError::UnsupportedParameter carries a fifth field `reason` (A1-51-01): D-03 needs the error text to name the llm.<provider>.models.<model-id>.effort override, which a thiserror message can only do from a field. The variant is #[non_exhaustive]."
  - "EffortSupport::intersect treats NoReasoning as the 'off only' element and normalises any result with no levels, no auto and no budget to NoReasoning (when off) or an empty Supported (when not). That drops `coerced` and `enable_thinking_with_level` in the collapsed case (nothing is left to refuse by level) and is what makes the operation associative."
  - "enable_thinking_with_level is OR-ed in an intersection; admission never reads it."
  - "Golden files are written with one trailing newline (the repository's end-of-file hook adds one) and the comparison ignores exactly one trailing newline of the golden; every other byte is compared."

patterns-established:
  - "One admission rule: every later caller (adapters, builder, fallback pre-flight, docs drift test) calls EffortSupport::resolve; no adapter may re-derive it."
  - "Unknown future enum variants in resolve / intersect fail closed (refuse / Unmeasured)."

requirements-completed: []
requirements-progress: [REASON-01, REASON-02, REASON-04, REASON-06]

duration: about 2h (including a cold workspace build)
completed: 2026-10-21
---

# Phase 51 Plan 01: Reasoning effort vocabulary, admission rule and pinned wire bodies Summary

**ReasoningEffort / ReasoningFallback in the domain core, one fail-closed `EffortSupport::resolve` admission rule with an associative AND intersection, a typed Permanent `LlmError::UnsupportedParameter`, and nine byte-for-byte golden bodies pinning every adapter's unset-effort request before any adapter changes.**

REASON-01, REASON-02, REASON-04 and REASON-06 are advanced (types and rule) but not marked complete: the adapters, builder, config and docs that make them user-visible land in plans 51-02 to 51-14, so `requirements-completed` is left empty and REQUIREMENTS.md is untouched.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: request-level effort reaches the admission rule; refusal is typed and Permanent through the battalion classification | `434f7a1b` |
| 2 | Full admission table, AND intersection and spelling round-trip (D-03, D-06, D-07, D-09) | `ae1c5932` |
| 3 | Pin today's nine wire bodies, the request-level consumer surface, and record the additive surface (D-00i, D-00e) | `fe16c9cd` |

The Task 1 tracer was verified by the user at the interactive checkpoint ("approved") before Tasks 2 and 3 began.

## What was built

- **paladin-core** `platform::container::reasoning`: `ReasoningEffort` (`#[non_exhaustive]`, `Copy + Eq + Hash`; serde `off auto minimal low medium high xhigh max`, `XHigh` renamed explicitly, `Budget(n)` as `{"budget":n}`; `level_rank`, `is_level`, `LEVELS`, `Display`), `ReasoningFallback` (`Refuse` default), and the two metadata-key constants. Re-exported from the `paladin` facade.
- **paladin-ports** `model_profile`: `BudgetBounds`, `EffortLevels` (infallible `new`, `validate`, `with_*`), `EffortSupport` (`Unmeasured` default), `ResolvedEffort`, `EffortRefusal`, `EffortSupport::resolve` / `intersect`, and `ModelProfile.effort` / `effort_verified_on`. `ModelProfile.reasoning` rustdoc corrected (it records returned reasoning text, not effort support).
- **paladin-ports** `llm_port`: `LlmRequest.reasoning` / `reasoning_fallback` (absent from an unset serialised request), `with_reasoning` / `with_reasoning_fallback`, re-exports, `LlmError::UnsupportedParameter` + `unsupported_parameter` (Permanent). Rustdoc on the three `with_reasoning` methods (`LlmRequest`, `LlmResponse`, `TokenUsage`) says which sets an effort.
- **paladin-battalion**: `typed_origin` names the provider for the new variant; `llm_error_class` labels it `"unsupported parameter"`.
- **paladin-llm** conformance: `send_capturing_raw`, the case `unset_reasoning_leaves_the_wire_body_unchanged` added to the behaviour list (suite is now 21 cases; the pinned `CASE_COUNT` assertion and docs updated), and nine golden files.
- **Consumer surface**: `phase_51_request_surface` in `tests/consumer_surface_muster.rs`; CHANGELOG entries (root, core, ports); `.project/current-exports.txt` refreshed (one new line, the facade `reasoning` module).

## TDD record

- **Task 1 RED:** a test-only `reasoning.rs` failed to compile (34 errors in the `paladin-ai-core` lib test build). GREEN: the 9 core tests pass, plus the ports and battalion tests listed in the plan.
- **Task 2 RED** (run against the Task 1 fail-closed `resolve` / `intersect`), 9 tests failed:
  `a_coerced_level_is_never_a_substitution_target`, `budget_precision_survives_resolve`, `admission_table_for_a_supported_row`, `budget_table_is_inclusive_and_never_substituted`, `an_invalid_budget_span_fails_closed`, `effort_intersect_follows_the_documented_cases`, `empty_levels_validate_and_refuse_every_level`, `model_profile_intersect_covers_effort`, `substitution_follows_the_chosen_direction_and_keeps_both_values`.
  These 9 plus 4 that already held on the fail-closed stub (`off_is_never_substituted`, `effort_levels_normalise_and_validate`, `effort_intersect_is_commutative_and_associative`, `effort_support_serde_round_trips`) all pass after GREEN (27 `model_profile` tests). The commutative/associative test passing on the stub is expected (a constant-ish result is trivially associative); it is the real guard once the AND rules landed, and still passes.
- **Task 3** is guard work: the golden case was blessed once from the unmodified adapters (`UPDATE_REASONING_GOLDEN=1`), then re-run without the switch (nine passes, twice, so the bodies are deterministic).

## Verification

- `cargo test --offline -p paladin-ai-core --lib reasoning` (9), `-p paladin-ports --lib` (297 incl. new), `-p paladin-ports --doc` (229), `-p paladin-battalion --lib -- llm_failure llm_decision` (23), `-p paladin-llm --all-features --lib` (1263, includes the nine golden cases and the `CASE_COUNT == 21` pin), `--features llm-anthropic,llm-deepseek --test consumer_surface_muster` (1): all pass.
- `cargo test --offline --workspace --all-features --no-run` compiles. `cargo fmt --check` clean. `cargo clippy -D warnings` clean on `paladin-ai-core`, `paladin-ports`, `paladin-battalion` and `paladin-llm` (`--all-targets --all-features` for the last). `RUSTDOCFLAGS=-D warnings cargo doc` clean on core, ports, battalion.
- `make api-surface` exits 0 after `make api-surface-update`; `make check-gates` and `make check-api-examples` exit 0.
- Not run: the full `make clean-code` chain and `make security` (cargo-audit / cargo-deny); no dependency was added. `cargo-llvm-cov` is absent locally; the proxy is that every new public item has a doc test and every `resolve` branch is named by a test. The orchestrator's post-wave hook chain covers the rest.
- Manual credential-handling review (security.instructions.md): `UnsupportedParameter` is built from five strings only; a test plants a prompt string and asserts it is absent from `Display` and `Debug`. No `Debug` of a key-bearing type was added, and the golden bodies were grepped: none contains `authorization`, `bearer` or an API key (keys travel in headers, not bodies).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `ResolvedEffort::substitute` introduced in Task 2, not Task 1**
- **Found during:** Task 1 clippy. A private constructor with no caller is a dead-code error under `-D warnings`, and Task 1's `resolve` never substitutes.
- **Fix:** added the helper with the substituting arms in Task 2; the `substituted()` doctest used an exact example in the Task 1 commit and was restored to the `Max` / `NearestLower` example in Task 2.
- **Commits:** `434f7a1b`, `ae1c5932`

**2. [Rule 1 - Bug] clippy `collapsible_if` in `EffortLevels::validate`**
- **Fix:** rewritten with `Option::filter` (let-chains avoided).
- **Commit:** `434f7a1b`

### Notes against the plan's wording

- **Gemini golden has no `model` key.** The acceptance criterion says each golden's `model` key equals the fixture's default model; Gemini carries the model in the URL path, not the body, so its golden (`contents` + `generationConfig`) has none. The other eight match (`grok-4.3`, `some-model`, `deepseek-flash`, `llama3`, `kimi-k2.6`, `claude-haiku-5-5`, `qwen3.8-flash`, `gpt-4o`).
- **`crates/paladin-core/src/platform/container/token_usage.rs`** is touched (rustdoc only) although not in `files_modified`: the plan requires rustdoc on all three `with_reasoning` methods and `TokenUsage` lives in core.
- **Intersection corner (documented in `EffortSupport::intersect`):** when an intersection leaves no level, no `auto` and no budget, `coerced` and `enable_thinking_with_level` are dropped. A coerced level asked of such a result is refused as `LevelUnsupported` rather than `LevelCoerced`; it is refused either way. This is the cost of the stated `NoReasoning ∧ Supported(l)` rule staying associative.
- **A2-51-01 (per-request random value):** none found. No body carries a random value, no key was masked, and the second un-blessed run compared green.
- **Commit trailer:** the system attribution reminder (`Claude Sonnet 5.5`) was used rather than the `Claude Fable 5.1` line in the project notes.

## Authentication Gates

None.

## Known Stubs

None. The only deliberate limitation is that no adapter sends an effort yet; that is the scope of plans 51-03 to 51-05 and 51-12.

## Threat Flags

None. No new network endpoint, auth path or file access beyond reading the golden files in a test. T-51-01 through T-51-05 are covered by Task 2's table tests, the Task 1 privacy test, the Permanent classification and battalion arms, and the Task 3 precondition (no adapter file was modified; the commit's diff touches only `conformance.rs` in `paladin-llm`).

## Open items for the owner

- A1-51-01 stays on the list: the fifth `reason` field on `UnsupportedParameter`.

## Self-Check: PASSED

- Files: `reasoning.rs`, nine golden files, this SUMMARY, all present.
- Commits `434f7a1b`, `ae1c5932`, `fe16c9cd` present in `git log` on `worktree-agent-a16a3827c107a97d4`.
