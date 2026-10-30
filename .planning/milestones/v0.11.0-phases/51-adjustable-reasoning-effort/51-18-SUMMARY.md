---
phase: 51-adjustable-reasoning-effort
plan: 18
subsystem: llm-compat-presets
tags: [reasoning-effort, check-reasoning, compat-engine, decorators, gap-closure, g-2]
status: complete

requires:
  - phase: 51-04
    provides: "compat engine reasoning wire, masked model_profile"
  - phase: 51-06
    provides: "provider-level reasoning_effort / reasoning_fallback config knobs"
  - phase: 51-17
    provides: "LlmPort::check_reasoning, reasoning::check / judge"
provides:
  - "crate-private CompatEngine::check_reasoning over the masked model_profile and the preset's default_reasoning"
  - "check_reasoning overrides on KimiAdapter, QwenAdapter, GrokAdapter, OllamaAdapter, OpenAiCompatibleAdapter"
  - "check_reasoning forwarded by PricingLlmAdapter and CadenceLlmAdapter"
  - "test_support::CheckVerdictPort (refusing, admitting, asked) and test_support::agreed_verdict"
affects: [51-19, 51-20]

tech-stack:
  added: []
  patterns:
    - "a preset's pre-judge is a one-line delegation to the engine, which runs the same profile, defaults and judge as build_request"
    - "a decorator forwards provided trait methods that carry adapter state; inheriting the default would drop it"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - crates/paladin-llm/src/pricing.rs
    - crates/paladin-llm/src/cadence.rs
    - crates/paladin-llm/src/test_support.rs

key-decisions:
  - "CompatEngine::check_reasoning calls reasoning::check with self.model_profile (already masked by the declared wire) and self.default_reasoning, so a preset cannot read a different profile, mask or default at build time than at send time (T-51-54)."
  - "Qwen's provider-default test cannot show a fallback substitution: the enable_thinking wire carries no level, so nothing is listed to substitute. It shows the defaults through a refusal naming the key, a default the wire carries (off) being admitted, and a request-level effort beating a refused default."
  - "agreed_verdict (generate and check_reasoning asked the same question, reasons compared) lives in the crate-level test_support so Ollama and OpenAI-compatible share it; Qwen and Grok kept a local copy written during the tracer."

requirements-completed: [REASON-01]

duration: one session
completed: 2026-10-23
---

# Phase 51 Plan 18: Compat presets and decorators carry check_reasoning Summary

**The five compat presets answer `LlmPort::check_reasoning` through the compat engine's own send-time profile and provider defaults, and `PricingLlmAdapter` / `CadenceLlmAdapter` forward it, so `Pricing(Cadence(adapter))` gives the pre-judging gates the adapter's own verdict (G-2 item 2).**

## Accomplishments

- `CompatEngine::check_reasoning` = `reasoning::check(self.provider_name, model, || self.model_profile(model), effort, fallback, self.default_reasoning.0, self.default_reasoning.1)`, documented as the send-time twin of `build_request`.
- Kimi, Qwen, Grok, Ollama and OpenAI-compatible override `check_reasoning` with `self.engine.check_reasoning(..)`, beside `model_profile`.
- `PricingLlmAdapter` and `CadenceLlmAdapter` forward `check_reasoning` to `inner` (rustdoc in the `model_profile` style).
- `CheckVerdictPort` (scripted verdict plus a record of what it was asked) and `agreed_verdict` in `crates/paladin-llm/src/test_support.rs`.
- All nine adapters, alone or wrapped, now answer with the verdict their own send path gives.

## Task Commits

1. Task 1 (tracer): `835a5edd` -- `feat(51-18): compat engine check_reasoning; Kimi, Qwen, Grok (G-2, D-09)`
2. Task 2: `66c6380a` -- `feat(51-18): self-hosted presets and decorators carry check_reasoning (G-2)`

Tracer gate (auto mode): the Task 1 verify command (Kimi parity grid, Qwen and Grok provider-default tests, `kimi_preset_never_emits_top_level_reasoning_effort`, all nine `unset_reasoning_leaves_the_wire_body_unchanged` goldens) passed on the committed tree. Tracer verified end to end, expansion continued.

## RED runs (recorded per acceptance criteria)

Task 1 (trait default in force, overrides absent):

- `kimi_check_reasoning_agrees_with_generate_on_every_combination` failed at the first grid cell, `default None/None, request Some(Medium)/None`: generate's reason read `this model does not offer that effort level; this model accepts: levels [low, high]; set a reasoning_fallback ...`, the default's omitted the accepted-levels clause.
- `qwen_check_reasoning_consults_the_provider_defaults` failed: generate refused a provider-default `medium` (reason ending ``(set by `llm.qwen.reasoning_effort`)``) while `check_reasoning` returned `Ok(())`.
- `grok_check_reasoning_consults_the_provider_defaults` failed: with provider `nearest_lower`, generate substituted `medium` -> `low` while `check_reasoning` refused.

Task 2:

- `ollama_check_reasoning_consults_the_provider_defaults` and `openai_compatible_check_reasoning_consults_the_provider_defaults` failed the same way as Grok (generate substituted, check refused).
- `ollama_check_reasoning_refuses_on_the_undeclared_wire` and `openai_compatible_check_reasoning_refuses_on_the_undeclared_wire` failed: the default's reason lacked the ``set `llm.<provider>.reasoning_wire` ...`` unlock text.
- `pricing_forwards_check_reasoning`, `pricing_over_cadence_forwards_check_reasoning`, `cadence_forwards_check_reasoning` failed: the decorators returned the default's `it reports no model profile, so an explicit reasoning effort cannot be checked` instead of the inner port's scripted verdict.

## Verification

- Task 1 verify (5 filters, 13 tests): pass. Task 2 verify (`check_reasoning_consults_the_provider_defaults`, `forwards_check_reasoning`, `forwards_model_profile`, `ollama_declares_no_reasoning_wire_by_default`, plus the two undeclared-wire tests): 16 pass.
- `cargo test --offline -p paladin-llm --lib --all-features`: 1511 passed, 0 failed.
- `cargo fmt --all --check`: clean. `cargo clippy --offline -p paladin-llm --all-targets --all-features -- -D warnings`: clean. `cargo check --offline --workspace --all-targets --all-features`: compiles.
- `make api-surface`: "API surface unchanged" (4323 items; every addition is crate-private, an override of an existing trait method, or `cfg(test)`).
- Every cargo run used `CARGO_BUILD_JOBS=4`, one at a time. The plan's workspace-wide clippy was replaced by the scoped clippy plus the workspace check above; the orchestrator's post-merge hook chain runs the full set. Commits used `--no-verify` as sanctioned for this wave.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Plan assumption] Qwen cannot show a provider `nearest_lower` substitution**
- **Found during:** Task 1 (writing the Qwen test)
- **Issue:** the plan's behaviour for Qwen and Grok expects a `[low, high]` model on which provider `nearest_lower` lets `medium` substitute. Qwen's `enable_thinking` wire masks every level (`levels: false`), so no Qwen model can list a level, and the plan's "seeded id whose row lists levels" escape hatch does not exist for Qwen.
- **Fix:** the Qwen test proves provider-defaults consultation another way, always with `generate` agreeing: a provider `reasoning_effort: medium` refuses a no-effort check with ``(set by `llm.qwen.reasoning_effort`)`` (the trait default would admit it); a provider default `off` is admitted; a request-level `off` beats the refused provider default; a provider `nearest_lower` still refuses (no level to substitute). Grok keeps the plan's `[low, high]` override test.
- **Files modified:** `crates/paladin-llm/src/qwen/adapter.rs`
- **Commit:** `835a5edd`

**2. [Scope] Extra tests**
- Added `ollama_check_reasoning_refuses_on_the_undeclared_wire` and `openai_compatible_check_reasoning_refuses_on_the_undeclared_wire` to cover the must-have "a self-hosted preset left at `ReasoningWire::Unsupported` refuses an explicit effort exactly as at send time" (D-10, T-51-54). Commit `66c6380a`.

## Threat Flags

None new. T-51-52 mitigated (explicit forwarding in `pricing.rs` / `cadence.rs`, `pricing_over_cadence_forwards_check_reasoning` pins the production composition). T-51-54 mitigated (engine `model_profile` is wire-masked; the two undeclared-wire tests). T-51-53 manual credential-handling review: `reasoning::check` renders only provider name, model, effort spellings, fixed refusal wording and YAML key names; the engine method takes no base URL, API key, header or body, and no log statement was added.

## Known Stubs

None. `CheckVerdictPort::generate` / `generate_stream` return a fixed `ProcessingError` by design (a `cfg(test)` double that only answers `check_reasoning`).

## Self-Check: PASSED

- Files: all nine modified files exist; `grep -c 'engine.check_reasoning('` reports 1 for each of the five presets; `inner.check_reasoning(` appears once in each of `pricing.rs` and `cadence.rs`; `struct CheckVerdictPort` is in `test_support.rs`; `reasoning::check(` appears once in `compat/engine.rs`.
- Commits `835a5edd` and `66c6380a` exist on `worktree-agent-a590697850dc43995`.
