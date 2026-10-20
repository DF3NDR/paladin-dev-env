completed: 2026-10-20
---
created: 2026-10-16T01:40:00Z
title: Harmonise the drifted `new(..)` signatures across the nine paladin-llm *Config types
area: api-surface
resolves_phase: 50.1
severity: minor
files:
  - crates/paladin-llm/src/openai/adapter.rs:141
  - crates/paladin-llm/src/anthropic/adapter.rs:156
  - crates/paladin-llm/src/deepseek/adapter.rs:122
  - crates/paladin-llm/src/gemini/adapter.rs:270
  - crates/paladin-llm/src/kimi/adapter.rs:138
  - crates/paladin-llm/src/qwen/adapter.rs:229
  - crates/paladin-llm/src/grok/adapter.rs:128
  - crates/paladin-llm/src/ollama/adapter.rs:136
  - crates/paladin-llm/src/openai_compatible/adapter.rs:451
  - tests/consumer_surface_muster.rs:318-324
owner: repo maintainer
raised_by: Phase 50 plan 50-08 Task 1 checkpoint (owner, 2026-10-16)
---

## Problem

Every one of the nine publicly constructible `paladin-llm` config types exposes both
`from_env() -> Result<Self, String>` and an infallible `new(..)`, but the `new(..)`
signatures drifted as adapters were added at different times:

| Type | `new(..)` arguments |
|------|---------------------|
| `OpenAIConfig` | `api_key` |
| `AnthropicConfig` | `api_key, base_url, model, max_tokens` |
| `DeepSeekConfig`, `GeminiConfig`, `KimiConfig`, `QwenConfig`, `GrokConfig` | `api_key, base_url, model` |
| `OllamaConfig` | `base_url, model` |
| `OpenAiCompatibleConfig` | a longer list |

The same functionality is therefore presented with a different shape per provider. This is
historical rather than principled, and a consumer moving an agent between providers
(PARITY-01's whole point) meets a different constructor each time.

## Why it is not folded into Phase 50

Plan 50-08 (owner decision 2026-10-16: option-a) marks all nine types `#[non_exhaustive]` and
adds `with_max_tokens`, `with_max_retries`, `with_context_window`, `with_model_overrides`
while keeping every existing `new`/`from_env` signature unchanged, so that Phase 50 and
Phase 51 cost the consumer exactly one registered break. Changing the `new` signatures is a
second break on all nine published types and must be registered and migrated on its own.

## Proposed resolution

- Settle one shape, for example `new(api_key, model)` with `base_url` and the rest reached
  through the option-a `with_*` builders (Ollama and OpenAI-compatible keep their
  endpoint argument where a key is not meaningful), or a shared `ProviderConfig` builder.
- Register the break in `.cargo/semver-checks-allowlist.toml` and `MIGRATION.md` 9.2, refresh
  `make api-surface-update`, and pin the new shapes in `tests/consumer_surface_muster.rs`.
- Natural home: the v0.11 migration-guide pass (Phase 46) if it can ride the same consumer
  bump, otherwise a later API-hygiene phase before v1.0.

## Update 2026-10-19 (owner decision 2026-10-18, filed as Phase 50.1)

The owner chose to ride this break on the single v0.11.0 consumer bump rather than defer it:
it is **SURFACE-02** of the inserted **Phase 50.1 — Consumer-Surface Readiness for v0.11.0**
(`.planning/phases/50.1-consumer-surface-readiness/50.1-HANDOFF-web3sec-consumer-surface.md`
§3.2), sequenced after Phase 50 and before Phase 51 so that Phase 51 extends a settled
constructor shape. The phase also adds `with_timeout_seconds` (and `with_base_url` where the chosen
shape drops `base_url` from `new`) to `config_knob_builders!`, and amends MIGRATION §9.8 DR-12's
"not part of v0.11.0" sentence. The shape itself is decision D-B at `/gsd-discuss-phase 50.1`;
web3sec's and this todo's shared preference is `new(api_key, model)` + builders. The todo stays
pending until SURFACE-02 seals, then moves to `completed/` with `resolves_phase: 50.1`.

## Resolution (Phase 50.1)

Resolved by SURFACE-02 (plans 50.1-02, -04, -05, -06): seven of the nine config types now construct
with `new(api_key, model)` (`DeepSeekConfig`, `GeminiConfig`, `QwenConfig`, `GrokConfig`,
`KimiConfig`, `OpenAIConfig`, `AnthropicConfig`); `OllamaConfig::new(base_url, model)` and
`OpenAiCompatibleConfig::new(api_key, base_url, model, capabilities)` keep their endpoint argument
because a key is not their first input (D-08). Every other knob, including `with_base_url` and
`with_timeout_seconds`, is a `with_*` builder from `config_knob_builders!`. The break is registered
as one SURFACE-02 row and one allowlist entry per type and as MIGRATION.md section 9.8 DR-19 (DR-12
amended), and the new shapes are pinned in `tests/consumer_surface_muster.rs`. See
`.planning/phases/50.1-consumer-surface-readiness/50.1-06-SUMMARY.md`.
