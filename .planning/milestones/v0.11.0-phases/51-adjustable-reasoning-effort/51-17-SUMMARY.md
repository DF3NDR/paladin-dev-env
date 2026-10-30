---
phase: 51-adjustable-reasoning-effort
plan: 17
subsystem: llm-port
tags: [reasoning-effort, check-reasoning, paladin-builder, provider-defaults, gap-closure, g-2]
status: complete

requires:
  - phase: 51-03
    provides: "reasoning::in_force / admit, EffortSupport::resolve"
  - phase: 51-05
    provides: "SubstitutionLog warn-once record"
  - phase: 51-06
    provides: "provider-level reasoning_effort / reasoning_fallback config knobs"
  - phase: 51-07
    provides: "PaladinBuilder D-09 build-time effort check"
provides:
  - "LlmPort::check_reasoning(model, effort, fallback) -> Result<(), LlmError>: additive provided method; default = the builder's pre-gap rule"
  - "paladin_llm::reasoning::judge (admit minus the log), ::check (in_force + judge, no max_tokens) and ::FALLBACK_HINT, crate-private"
  - "check_reasoning overrides on DeepSeekAdapter, OpenAIAdapter, AnthropicAdapter, GeminiAdapter"
  - "PaladinBuilder::validate delegates its effort check to the port, so a provider-level fallback / default effort is honoured at build"
affects: [51-18, 51-19, 51-20]

tech-stack:
  added: []
  patterns:
    - "a pre-judging gate asks the port for a verdict instead of reading data, so build time and send time share one code path"
    - "admit = judge + record; check = in_force + judge: the log is the only thing the send path adds"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-llm/src/reasoning.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - src/application/services/paladin/paladin_builder.rs

key-decisions:
  - "A verdict method rather than a data accessor (remedy_decision): a chain's hops each have their own defaults, which a data accessor cannot express (plan 51-19 overrides per hop)."
  - "The per-request Budget-versus-max_tokens rule is the one rule not pre-judged (A1-51-17): it needs the request's own cap."
  - "The 'set a reasoning_fallback' hint is appended only when Refuse is in force after precedence and the refusal is LevelUnsupported, at build and at send time alike."
  - "PaladinBuilder now judges a provider-level default effort when the agent sets none (A2-51-17): a behaviour change recorded for plan 51-20's CHANGELOG / MIGRATION DR-21."

requirements-completed: [REASON-01]

duration: 2 sessions (one interrupted by a host crash; edits survived uncommitted)
completed: 2026-10-23
---

# Phase 51 Plan 17: LlmPort::check_reasoning Summary

**An additive `LlmPort::check_reasoning` verdict method, overridden by the four native adapters over the same `in_force` + `judge` path they run at send time, so `PaladinBuilder::build` honours `llm.<provider>.reasoning_fallback` / `reasoning_effort` (G-2 item 2, WR-01).**

## Accomplishments

- `LlmPort::check_reasoning` with a default body that is exactly the builder's pre-gap rule (no effort -> `Ok`; otherwise `model_profile` judged by `EffortSupport::resolve`; no profile -> refused, D-03), a "# Decorators ... must forward" paragraph, `# Errors`, and a compiling `# Examples` doc test.
- `reasoning::judge` extracted from `admit` (admit = judge + `log.record`), `reasoning::check` (= `in_force` + `judge`, never reads the profile when nothing is in force, takes no `SubstitutionLog`), and `FALLBACK_HINT` now reaching send-time refusals too (`refusal_reason` gains the in-force fallback).
- DeepSeek, OpenAI, Anthropic and Gemini override `check_reasoning` through `reasoning::check` with their own `wire_profile` and config defaults.
- `PaladinBuilder::validate` makes one unconditional `check_reasoning` call; `effort_refusal` maps the `LlmError` (provider and model come from the error, so a chain hop can be named once 51-19 lands). The "cannot drift" comment is replaced.

## Task Commits

1. Task 1 (tracer): `fbf1dfa2` -- `feat(51-17): LlmPort::check_reasoning -- build-time effort verdict equals send time (G-2, D-09)`
2. Task 2: `93c41a17` -- `feat(51-17): OpenAI, Anthropic, Gemini check_reasoning (D-09)`

## RED runs (recorded per acceptance criteria)

- `deepseek_check_reasoning_agrees_with_generate_on_every_combination` failed with the override disabled (trait default in force): first divergence `default None/None, request Some(Medium)/None` -- generate's reason carried `this model accepts: levels [low, high]`, the default's did not (and every provider-default row diverges in verdict). Passes with the override (full 3 x 3 x 6 x 4 grid).
- `a_provider_level_fallback_lets_an_agent_effort_substitute_at_build` failed before the delegation: `ConfigurationError(... set a reasoning_fallback to substitute the nearest level)` for a port already configured `NearestLower`.
- `a_provider_level_default_effort_the_model_cannot_honour_is_refused_at_build` failed before the delegation: the builder built (`Ok`).
- `the_builder_verdict_equals_check_reasoning_for_every_combination` failed before the delegation (`default None/Some(NearestLower), agent Some(Medium)/None`: builder refused, `check_reasoning` `Ok`).
- `an_agents_refuse_beats_a_provider_nearest_lower_at_build` passed before the delegation (the old rule already used the agent's fallback); it is a regression guard for T-51-49.
- Task 2: `openai_`, `anthropic_`, `gemini_check_reasoning_consults_the_provider_defaults` each failed on `substituting.check_reasoning(PROBE, Some(Medium), None).is_ok()` before their override and pass after.

## Verification

- `cargo test -p paladin-ports --lib check_reasoning` (2) and `--doc check_reasoning` (1): pass.
- `cargo test -p paladin-llm --lib --all-features -- reasoning:: deepseek_check_reasoning_agrees... unset_reasoning_leaves_the_wire_body_unchanged`: 30 pass; Task 2 verify (`check_reasoning_consults_the_provider_defaults`, unset golden, unprofiled-refusal conformance, all nine adapters): 21 pass.
- `cargo test -p paladin-ai --lib --all-features -- paladin_builder`: 53 pass, including every 51-07 effort test.
- `cargo clippy --offline -p paladin-ports -p paladin-llm -p paladin-ai --all-targets --all-features -- -D warnings`: clean. `cargo check --offline --workspace --all-targets --all-features`: compiles (every `LlmPort` implementor unchanged). `cargo fmt --all`: applied.
- `make api-surface`: "API surface unchanged" (4323 items; the facade baseline lists no `LlmPort` methods, so 51-20 owns the register entry).
- All cargo runs used `CARGO_BUILD_JOBS=4`, one at a time, scoped to the plan's crates (thermal cap after the host crash). The workspace-wide clippy / `test --no-run` of the plan's `<verification>` was replaced by scoped clippy plus a workspace `cargo check --all-targets`; the orchestrator's post-merge hook chain runs the full set.

## Deviations from Plan

None in behaviour. Notes:

- [Process] Task 1 tests were written after the trait method and `reasoning::check` scaffold (the plan's SCAFFOLD step), so the RED evidence for DeepSeek was captured by temporarily compiling the override out (`#[cfg(any())]`, reverted before commit), not by a missing symbol.
- [Scope] `src/infrastructure/web/agent_host.rs:213` still carries its own copy of the fallback hint text; that gate is out of this plan's `files_modified` (host parity is plan 51-20's).
- No `deferred-items.md` entries; the fallback chain, compat presets and decorators keep the trait default until plans 51-18 / 51-19 (by plan design).

## Threat Flags

None. Threat register: T-51-48 mitigated (builder delegates; full-grid parity tests); T-51-49 mitigated (`in_force` is request-first field by field; `an_agents_refuse_beats_a_provider_nearest_lower_at_build`); T-51-51 mitigated (`check` / `judge` take no `SubstitutionLog`). T-51-50 -- manual credential-handling review: the only strings `check` produces are provider name, model, effort spellings, fixed refusal wording, and the YAML key names in `(set by ...)` / the unlock hints; no key, header, prompt or response body is reachable from `check` or `judge`, and no log statement was added.

## Known Stubs

None.

## Self-Check: PASSED

- Files: all seven modified files exist; commits `fbf1dfa2` and `93c41a17` exist on `worktree-agent-a2e8b3a7b104d52ab`.
