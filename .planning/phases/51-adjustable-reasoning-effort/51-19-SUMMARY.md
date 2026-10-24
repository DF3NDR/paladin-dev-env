---
phase: 51-adjustable-reasoning-effort
plan: 19
subsystem: llm-fallback
tags: [reasoning-effort, fallback-chain, check-reasoning, preflight, gap-closure, g-2]
status: complete

requires:
  - phase: 51-07
    provides: "FallbackLlmAdapter pre-flight (D-08) and the builder D-09 build-time effort check"
  - phase: 51-17
    provides: "LlmPort::check_reasoning, reasoning::check / in_force, native adapter overrides, builder delegation"
provides:
  - "FallbackLlmAdapter::check_reasoning: the AND over hops of each hop's own check_reasoning, first refusal prefixed `fallback hop <index>: `"
  - "preflight_reasoning reduced to self.check_reasoning on the request's model, effort and fallback"
  - "FallbackLlmAdapter::model_profile reports effort Unmeasured (no effort_verified_on) when any hop reports no profile; sampling unchanged"
affects: [51-20]

tech-stack:
  added: []
  patterns:
    - "the hop (port + step model + that port's own provider defaults) is the noun an effort verdict belongs to; the intersected chain profile is demoted to a sampling detail"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/fallback.rs
    - src/application/services/paladin/paladin_builder.rs

key-decisions:
  - "Promote the per-hop verdict over the intersected chain profile as the effort authority (assumption_delta_decision): build, pre-flight and send read one function, so they cannot drift."
  - "model_profile keeps its lenient sampling intersection (ADR-0061) and fails closed only for effort, so no other reader of the chain profile can mistake an unprofiled chain for one that takes an explicit effort."
  - "The pre-flight's early return on a request with no effort was removed: a hop's provider default can now be in force, and each hop's own check returns early when nothing is."

requirements-completed: [REASON-04, REASON-01]

completed: 2026-10-23
---

# Phase 51 Plan 19: Chain effort verdict is every hop's own verdict Summary

**`FallbackLlmAdapter` overrides `LlmPort::check_reasoning` by asking each hop its own verdict against its own step model and provider defaults; the pre-flight is that call, and the chain profile no longer advertises effort a profile-less hop cannot give (G-2 item 3, WR-01 chain half, WR-03).**

## Accomplishments

- `check_reasoning` override in `impl LlmPort for FallbackLlmAdapter`: hops in chain order, `hop_model` = step model else `model`; `UnsupportedParameter` is re-raised with the hop's own provider and model and the reason prefixed `fallback hop <index>: `; any other error is returned unchanged.
- `preflight_reasoning` is now exactly `self.check_reasoning(&request.model, request.reasoning, request.reasoning_fallback)`; still first in `generate` and `generate_stream`. `PaladinBuilder::build` (plan 51-17 delegation) reaches the same method, so a chain has one build-time and pre-flight verdict.
- `model_profile` collects each hop's profile, intersects the `Some` ones exactly as before, and sets `effort = Unmeasured`, `effort_verified_on = None` when at least one hop answered `None`.
- Rustdoc of the type, `preflight_reasoning`, `check_reasoning` and `model_profile` states the per-hop rule; the A2-51-07 sentence is gone.

## Task Commits

1. Task 1 (tracer): `4de07ef2` -- `feat(51-19): a chain's effort verdict is every hop's own check_reasoning (G-2, D-08, D-09)`
2. Task 2: `62dafa79` -- `fix(51-19): chain profile effort fails closed on a profile-less hop (G-2 item 3)`

Both used `--no-verify` as sanctioned for this wave (`workflow.worktree_skip_hooks`); the plan's own verify commands, scoped clippy and fmt were run before each commit.

## RED runs (recorded per acceptance criteria)

- `a_chain_with_a_profileless_hop_refuses_an_explicit_effort_at_build` failed: expected a `ConfigurationError`, got `Ok(Entity ...)` (the profile-less hop was dropped from the intersection).
- `a_chain_whose_hop_levels_only_touch_builds_under_nearest_lower` failed: `ConfigurationError("model 'm' on provider 'fallback' does not accept parameter 'reasoning_effort' = medium: no offered effort level lies in the chosen fallback direction")` (intersection `{high}` has no lower level).
- `a_chain_build_refusal_names_the_same_hop_as_the_pre_flight` failed: build named provider `fallback` with no hop index while the pre-flight named `MockWithProfile` and `fallback hop 1: ...`.
- `a_hops_own_provider_fallback_is_honoured_by_the_pre_flight` failed: `UnsupportedParameter { provider: "deepseek", reason: "fallback hop 1: this model does not offer that effort level" }` (the hop's own `NearestLower` was ignored).
- `a_hops_provider_default_effort_is_pre_flighted_even_when_the_request_sets_none` failed: the chain returned `Ok` served by `hop-a` (the early return skipped the pre-flight).
- `a_chain_check_reasoning_is_the_and_of_every_hops_check` failed on the first grid row: the chain inherited the trait default, naming provider `fallback` instead of `hop-a`.
- `a_chain_profile_is_effort_unmeasured_when_any_hop_reports_none` failed: effort was `Supported([Low, High])`, expected `Unmeasured`. `a_chain_profile_keeps_sampling_from_the_hops_that_report_one` passed before and after (regression guard).

## Verification

- `cargo test --offline -p paladin-llm --lib --all-features -- reasoning_preflight_tests`: 11 pass. Full `paladin-llm --lib --all-features`: 1506 pass. `-- fallback:: pricing_over_cadence_forwards_model_profile`: 66 pass (every existing fallback test, including the sampling-intersection test, unchanged).
- `cargo test --offline -p paladin-ai --lib --all-features -- a_chain_ paladin_builder`: 56 pass, including `a_chain_builds_an_effort_every_hop_lists_and_refuses_one_a_hop_lacks` and every 51-17 builder test.
- `cargo clippy --offline -p paladin-llm -p paladin-ai --all-targets --all-features -- -D warnings`: clean. `cargo fmt --all --check`: clean. `make api-surface`: unchanged (4323 items).
- Acceptance greps: one `fn check_reasoning(` in `fallback.rs`; `preflight_reasoning` body contains one `self.check_reasoning(`; `A2-51-07` count 0.
- Thermal cap honoured: every cargo run used `CARGO_BUILD_JOBS=4`, one at a time. The plan's workspace-wide clippy was replaced by clippy scoped to `paladin-llm` and `paladin-ai` (the only crates this plan touches); the orchestrator's post-merge chain runs the full set.

## Deviations from Plan

None in behaviour. Notes:

- [Process] Test-helper edits were applied with small Python scripts kept in the scratchpad (a Bash heredoc form was blocked by the sandbox); only the two files in `files_modified` were touched.
- [Scope] The rustdoc of `check_reasoning` also says the chain profile over-refuses level sets that only touch, which is the reason the per-hop verdict is the authority; no behaviour change beyond the plan.
- Sibling plan 51-18's files (compat, kimi, qwen, grok, ollama, openai_compatible, pricing.rs, cadence.rs, test_support.rs) were not touched. The Pricing and Cadence decorators do not yet forward `check_reasoning` in this base; that is plan 51-18's, so a chain whose hops are decorator-wrapped will see the trait default per hop until both plans are merged.

## Threat Flags

None new. T-51-55 mitigated (per-hop `check_reasoning` with hop defaults before the first call; `a_hops_provider_default_effort_is_pre_flighted_even_when_the_request_sets_none`). T-51-56 mitigated (profile-less hop refused at build and pre-flight; `model_profile` effort `Unmeasured`). T-51-57 -- manual credential-handling review: the chain adds only `fallback hop <index>: ` to the hop's own reason text (provider name, model, effort spellings, fixed wording, YAML key names); no hop credential, base URL, header, prompt or response body is read or interpolated, and no log statement was added.

## Known Stubs

None.

## Self-Check: PASSED

- Files: `crates/paladin-llm/src/fallback.rs` and `src/application/services/paladin/paladin_builder.rs` modified and present; this SUMMARY present.
- Commits `4de07ef2` and `62dafa79` exist on `worktree-agent-ad014754d7b450962`.
