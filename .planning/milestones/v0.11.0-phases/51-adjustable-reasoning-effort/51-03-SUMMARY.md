---
phase: 51-adjustable-reasoning-effort
plan: 03
subsystem: llm-adapters
tags: [reasoning-effort, deepseek, openai, admission, wire-masking, model-override, metadata]
status: complete

requires:
  - phase: 51-01
    provides: "ReasoningEffort/ReasoningFallback, EffortSupport::resolve, LlmRequest.reasoning, LlmError::UnsupportedParameter, unset-effort golden bodies"
provides:
  - "paladin_llm::reasoning: ReasoningWire, crate-private admit / mask_effort / WireCaps / SubstitutionLog / level_spelling, re-exported metadata keys"
  - "RequestNotes.reasoning_requested / reasoning_effective stamped on buffered responses and stream terminal chunks"
  - "ModelOverride.effort (YAML llm.<provider>.models.<model-id>.effort), validated and laid over the profile"
  - "DeepSeek and OpenAI honour LlmRequest.reasoning exactly or refuse before HTTP; model_profile reports only what the wire can carry"
affects: [51-04, 51-05, 51-06, 51-07, 51-11, 51-12, 51-13, 51-14]

tech-stack:
  added: []
  patterns:
    - "one crate-private admit() called from each adapter's build_request (before HTTP and before the retry loop)"
    - "wire folded into the reported profile with mask_effort(support, WireCaps)"
    - "warn-once SubstitutionLog copied from OmissionLog, with an explicit cap"

key-files:
  created:
    - crates/paladin-llm/src/reasoning.rs
  modified:
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/src/sampling.rs
    - crates/paladin-llm/src/registry/mod.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/CHANGELOG.md
    - CHANGELOG.md

key-decisions:
  - "WireCaps::NONE masks every support (including Unmeasured and NoReasoning) to WireUnsupported: a wire with no effort form cannot be unlocked by an override, so the refusal must not point at one."
  - "Budget-versus-output rule lives in admit, after resolve: a profile refusal is reported before a max_tokens complaint."
  - "map_reasoning has a defensive final arm returning UnsupportedParameter; masking and admit already stop those efforts, so it can never put an unrequested value on the wire."

patterns-established:
  - "Every later adapter (51-04, 51-05) calls reasoning::admit with its own WireCaps and masks model_profile the same way."
  - "Effort metadata values are the ReasoningEffort Display spellings (xhigh, budget:2048)."

requirements-completed: []
requirements-progress: [REASON-02, REASON-03, REASON-04, REASON-05, REASON-06]

duration: about 3h (including a cold worktree build and the tracer checkpoint)
completed: 2026-10-21
---

# Phase 51 Plan 03: DeepSeek and OpenAI reasoning effort Summary

**Shared `paladin_llm::reasoning` admission (refuse before HTTP naming the unlocking override key, warn-once substitution, requested/effective metadata), `ModelOverride.effort`, and exact wire mappings for DeepSeek (`reasoning_effort` plus the `thinking` switch) and OpenAI (top-level `reasoning_effort`, `none` for off), with every unset-effort wire body still byte-identical.**

REASON-02 to REASON-06 are advanced for two of nine adapters but not marked complete: the other adapters, builder, configuration and docs land in plans 51-04 to 51-14, and no model is seeded with effort data until the dated probe (51-10 to 51-12). `requirements-completed` is left empty and REQUIREMENTS.md is untouched.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: a DeepSeek request with an effort reaches the wire exactly, stamped, or is refused before HTTP naming the override | `e58976e1` |
| 2 | DeepSeek complete: off, auto, coerced levels, substitution, the budget-versus-output rule and the streaming path | `b2952781` |
| 3 | OpenAI's top-level `reasoning_effort` and the additive surface record | `34498bb0` |

The Task 1 tracer was verified by the user at the interactive checkpoint ("approved") before Tasks 2 and 3 began; the orchestrator had re-run the evidence on `e58976e1`.

## What was built

- **`crates/paladin-llm/src/reasoning.rs`** (`pub mod reasoning`): `ReasoningWire` (`unsupported`, `top_level_effort`, `thinking_object`, `enable_thinking`; serde snake_case, `Display`, `#[non_exhaustive]`, doc test), the two re-exported metadata keys, and crate-private `WireCaps` (`NONE`), `mask_effort`, `SubstitutionLog`, `level_spelling` and `admit`. `admit` returns `Ok(None)` for an unset effort without reading the profile; otherwise it runs `EffortSupport::resolve`, turns a refusal into `LlmError::unsupported_parameter("reasoning_effort", <Display>, provider, model, reason)`, applies the budget-below-`max_tokens` rule, and records a substitution. Reasons: `Unmeasured` names `llm.<provider>.models.<model>.effort`; `WireUnsupported` names `llm.<provider>.reasoning_wire` only on `ollama` and `openai-compatible` and otherwise says no configuration unlocks it; level refusals list what the model does admit.
- **`sampling.rs`**: `RequestNotes.reasoning_requested` / `reasoning_effective`, inserted under the two keys by `into_metadata` only when set.
- **`registry/mod.rs`**: `ModelOverride.effort: Option<EffortSupport>`, `validate_effort` (through `EffortLevels::validate`, error prefixed `effort:`), and `apply_to` calling `with_effort` (leaves `effort_verified_on` alone). `deny_unknown_fields` still holds.
- **DeepSeek**: `reasoning_effort` and `thinking` (`DeepSeekThinking`) are the last two request fields so the existing order is untouched; `DEEPSEEK_REASONING_WIRE` (levels, off, auto; no budget); `wire_profile()` used by both `build_request` and `model_profile()`; private `map_reasoning` (level, plus `thinking: enabled` when `enable_thinking_with_level`; `Off` gives `thinking: disabled`; `Auto` gives `thinking: enabled` and no `reasoning_effort`); per-instance `SubstitutionLog`.
- **OpenAI**: `reasoning_effort` as the last request field; `OPENAI_REASONING_WIRE` (levels, off, auto; no budget); private `map_reasoning` (level spelling, `none` for an on-wire `Off`, no field for `Auto`, still stamped); masked `model_profile()`; per-instance `SubstitutionLog`.
- CHANGELOG entries in the root and `paladin-llm` `[Unreleased]` sections.

## TDD record

- **Task 1 RED:** the test-only `reasoning.rs`, the registry tests and the DeepSeek tests failed to compile (40 errors for the missing `admit`, `mask_effort`, `WireCaps`, `SubstitutionLog`, `ReasoningWire`, the metadata-key re-exports and `ModelOverride.effort`). GREEN: 112 DeepSeek, 63 `reasoning`, 72 `registry::` tests pass, plus the nine golden cases.
- **Task 2 RED:** 3 failures against the Task 1 code: `off_sends_thinking_disabled_and_no_effort`, `auto_sends_thinking_enabled_and_no_effort`, `a_budget_must_stay_below_max_tokens_by_at_least_one`. The other new Task 2 tests (coerced level refused under a fallback, warn-once substitution, masked budget, stream terminal-chunk stamping, refused stream before HTTP) passed immediately: they exercise machinery Task 1 already built (`resolve`, masking, `build_request` shared by both paths) and are regression guards rather than RED tests. GREEN: 120 DeepSeek and 74 `reasoning` tests pass.
- **Task 3:** the OpenAI mapping was written before its tests (order inversion, noted honestly). Test sensitivity was checked by mutation: changing the `none` spelling to `off` made `off_sends_none_when_the_model_can_turn_reasoning_off` fail; reverted. One test-fixture bug (the scripted OpenAI stream chunks lacked `index`) was fixed in the test, not the adapter.

## Verification

- `cargo test --offline -p paladin-llm --all-features --lib` filters `deepseek` (120), `reasoning` (74), `registry::` (72), `openai` (193), `unset_reasoning_leaves_the_wire_body_unchanged` (9), `usage_fields_are_mapped` (9): all pass. `--doc`: 31 pass.
- `cargo clippy --offline -p paladin-llm --all-targets --all-features -- -D warnings` clean; `cargo fmt --check -p paladin-llm` clean.
- `cargo test --offline --workspace --all-features --no-run` compiles. `make check-gates`, `make check-api-examples` and `make api-surface` exit 0.
- Not run: full `make clean-code` chain and `make security` (no dependency added); `cargo-llvm-cov` is absent locally. Coverage proxy: every branch of `admit`, `mask_effort`, `describe_admitted`/`refusal_reason` and both `map_reasoning` fns is named by a test (budget unsupported, out-of-bounds, off/auto both wires, coerced, substitution, no-fallback refusal, unmeasured, wire-unsupported on self-hosted and native).
- Manual credential-handling review (security.instructions.md): both adapters' clients still use `redirect::Policy::none()` (untouched); no new `Debug` prints a key (`DeepSeekConfig`/`OpenAIConfig` Debug untouched); vendor 400s still go through the existing mappers and `diagnostic_excerpt`. The refusal and `warn!` text is built from provider, model and effort values only, and a DeepSeek test plants a prompt and a context string and asserts neither them nor the API key appear in the error's `Display` or `Debug`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `RequestNotes` struct literals in three adapters outside `files_modified`**
- **Found during:** Task 1. Adding two fields broke the literal in `compat/engine.rs`, `gemini/adapter.rs` and `anthropic/adapter.rs`.
- **Fix:** added `..RequestNotes::default()` to each (plans 51-04 and 51-05 will fill the fields in). The OpenAI literal got the same placeholder in Task 1 and its real fields in Task 3.
- **Commit:** `e58976e1`

**2. [Rule 1 - Bug avoided] `SubstitutionLog` capacity**
- The plan says to copy `OmissionLog`'s "capacity bound", but `OmissionLog` has none (its keys are model x parameter, with model caller-chosen). `SubstitutionLog` has an explicit 256-triple cap; past it a substitution warns every time rather than being remembered, so memory stays bounded and the signal is never lost. Accepted by the user at the checkpoint.
- **Commit:** `e58976e1`

**3. Budget-versus-output rule landed in Task 2, not Task 1**
- `admit` takes `max_tokens` from Task 1 (named `_max_tokens` while unused) and applies the rule from Task 2, as the plan sequences it. Accepted at the checkpoint.

### Notes against the plan's wording

- **`.project/current-exports.txt` is unchanged.** `make api-surface-update` changed only its generated-at timestamp line: the `paladin` facade does not re-export `paladin_llm::reasoning`, so the baseline tracks no new item. I reverted the timestamp-only change; `make api-surface` exits 0. The new `paladin-llm` surface is recorded in both CHANGELOGs.
- **Acceptance grep `must be below max_tokens`** prints one line in `reasoning.rs` (the implementation); the test asserts a shorter substring.
- **REASON-05 regression assertions:** the Phase 50 surface was already covered (`deepseek_buffered_reasoning_text_is_surfaced`, `deepseek_stream_reasoning_keeps_wire_order`, `map_usage_maps_cache_hit_and_reasoning_when_the_payload_carries_them`, `deepseek_measured_usage_is_read_inside_completion`, and the conformance `usage_fields_are_mapped`); all pass, so no new assertion was needed.
- **Budget-versus-output ordering through an adapter is covered by unit tests of `admit`** and by code position (after `check_max_tokens`), not by an adapter test, because both adapters mask budgets out of their wire; a budget can only reach `admit` on an adapter that carries one (Kimi, Qwen, Anthropic, Gemini, plans 51-04 and 51-05).
- **Commit trailer:** the system attribution reminder (`Claude Sonnet 5.5`) was used.

## Authentication Gates

None.

## Known Stubs

None. The deliberate limitation is that no model carries seeded effort data until plans 51-10 to 51-12, so only a `ModelOverride.effort` unlocks an explicit effort on DeepSeek or OpenAI for now. A1-51-03 stays open: the DeepSeek and OpenAI wire forms are the documented candidates until the probe confirms them.

## Threat Flags

None. No new network endpoint, auth path or file access; the only new outward text is the refusal and the `warn!`, both limited to provider, model and effort values (T-51-12). T-51-09 (effort silently dropped or swapped), T-51-10 (refusal retried / no output room) and T-51-11 (malformed override) are mitigated and tested.

## Self-Check: PASSED

- Files: `crates/paladin-llm/src/reasoning.rs` and this SUMMARY present.
- Commits `e58976e1`, `b2952781`, `34498bb0` present in `git log` on `worktree-agent-a24bbba56fea4e3c4`.
