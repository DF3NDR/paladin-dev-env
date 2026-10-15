# ADR-0061: Per-model parameter support -- refuse an explicit value the target cannot honour, omit an auto-selected one visibly

## Status

Accepted

**Date:** 2026-10-15

**Phase:** 50 (Provider Parity & Configuration-Driven Model Swap), plan 50-02; Code Conformance measured at the closeout, plan 50-16.

**Amends ADR-0004** (temperature validation); supersedes its amendment A2 (an auto-selected value out of range was refused; it is now omitted and recorded).

## Context

ADR-0004 made temperature validation provider-aware through one adapter-wide `ProviderCapabilities.temperature_range`, and its amendments A1-A4 narrowed the gate to a temperature the caller actually expressed, recorded that omission is not clamping, and required a declared range to be the measured one. Phase 50 found that the adapter-wide shape is not enough, for three reasons.

**First, the originals validate and then discard.** The Kimi preset declares `(1.0, 1.0)` and never sends `temperature`. The Anthropic adapter declares `(0.0, 1.0)` yet never puts `temperature` on the wire. An original adapter therefore accepts a value at build time, throws it away at the wire, and tells nobody. That is the "no undiscoverable substitution" failure ADR-0004 was written to prevent, only moved one layer down: the caller asked for 0.2, the vendor ran at its own default, and nothing in the run says so.

**Second, parameter support varies per model, not per adapter.** One adapter serves models that accept a range of temperatures, models that accept exactly one value, and models that reject the parameter outright; xAI rejects `stop` on some models and not on others. One `Option<(f32, f32)>` per adapter cannot state that.

**Third, `ProviderCapabilities` may not grow.** Phase 26 D-28 pinned the struct with the compile guard `provider_capabilities_gained_no_field`, and it has 92 literal construction sites. A clean break would be an X-03 supersession, which the owner rejected at discuss (D-03). The new fact has to arrive beside the struct, not in it.

The decisions below are the owner's answers at `/gsd-discuss-phase 50` (D-00c, D-00f, D-01, D-02, D-03, D-04, D-04a, D-05, D-07, D-13 in `50-CONTEXT.md`). No decision was re-opened. ADR-0004's principle, that a value is never changed behind the caller's back, is extended, not weakened (D-00f).

## Decision

### (a) The policy (D-01)

An **explicit** sampling parameter (temperature, `top_p`, frequency penalty, presence penalty, stop sequences) that the target provider and model cannot honour is **refused at build time** with an error that names the provider, the model and the parameter. An **auto-selected or framework-default** value that the target cannot honour is **omitted**, with one `warn!` per adapter instance per parameter and a `paladin.params_omitted` entry in the response metadata of every response that omitted something. The omission is never logged at `debug!` and never silent. The policy governs all nine adapters, the three original ones included.

Nothing in this policy substitutes a value. There is no clamping into a range, no snapping an explicit value to a `Fixed` value and no rounding: an inadmissible explicit value is refused by name, an inadmissible non-explicit value is omitted and recorded.

### (b) What explicit means (D-02)

These are explicit: `PaladinBuilder::temperature`, `PaladinBuilder::add_stop_word`, the agent YAML `temperature:` and `stop_words:` keys, and `llm.<provider>.default_temperature`. These are not: the output of the `auto_temperature` heuristic, the dynamic-temperature schedule, and the `0.7` that `PaladinData::default()` fabricates. Configuration an operator wrote is an operator statement even when it is a default for the provider; only a value the framework produced on its own is non-explicit.

### (c) The shape (D-03)

`paladin-ports` gains an additive, `#[non_exhaustive]` `ModelProfile` returned by `LlmPort::model_profile(&self, model: &str) -> Option<ModelProfile>`, whose default body returns `None`. `ProviderCapabilities` and its guard test are untouched. The builder consults the profile first and falls back to the adapter-wide `temperature_range` only when the port returns `None`, so an adapter that does not override the method behaves exactly as it does today. `ModelProfile` is built only through `ModelProfile::new()` plus `with_*` setters, so Phase 51 can add effort levels and budget bounds to the same type without a break (D-00j; ADR-0062).

The per-model fields are `temperature`, `top_p`, `frequency_penalty` and `presence_penalty` (each a `ParamSupport`), `stop_sequences` (a `StopSupport`), `context_window`, `max_output_tokens`, `reasoning`, `json_object`, `json_schema` and `verified_on`. `ModelProfile::new()` is **fail closed**: every sampling parameter `NotSent`, stop `NotSent`, no windows, the booleans `false`. A profile that forgets a field refuses an explicit value loudly instead of sending it; this is the lesson of `CompatRequestParameters`, which has no `Default` for the same reason.

### (d) `Fixed` and `NotSent` semantics, and the admission rule (D-04a)

`ParamSupport` has three variants: `Range(lo, hi)` (a vendor that accepts a span), `Fixed(v)` (a vendor that accepts exactly one value, such as Kimi's temperature `1.0`) and `NotSent` (a vendor or model that must not receive the parameter at all). One function, `ParamSupport::admits(value)`, is the admission rule that both the build-time check and the wire filter use:

- `Range(lo, hi)` admits `lo <= v <= hi`, **both ends inclusive**, plain float comparison, no epsilon (the ADR-0004 endpoint contract carried over);
- `Fixed(x)` admits only `v == x`, exact `f32` equality;
- `NotSent` admits nothing;
- `NaN` is admitted by none.

`Range(x, x)` therefore admits exactly what `Fixed(x)` admits. `Fixed` and `NotSent` are different statements about the wire: under `Fixed` the parameter may appear on the request carrying that one value, under `NotSent` it is absent from the request body entirely (the JSON key is absent, never `null`), so the vendor's own default applies. The **profile, not the adapter,** decides per model: xAI `stop` is `NotSent` on the models that reject it and supported on the others.

### (e) Mechanism: refuse at build, filter at the wire (RESEARCH Pattern 2)

D-01 is enforced in two places with one rule. `PaladinBuilder` knows which values are explicit (it receives them through the explicit entry points of (b)), so it refuses at build time using `admits`. Each adapter cannot tell an explicit value from an auto one by the time the value reaches an `LlmRequest`, so each adapter filters at the wire with one shared filter (`crates/paladin-llm/src/sampling.rs`), omitting what the profile does not admit, with the `warn!` and the metadata entry of (a).

**Stated limit.** A caller that builds an `LlmRequest` directly, bypassing `PaladinBuilder`, gets omit-plus-warn-plus-metadata, not a refusal, because a bare request carries no provenance. If the owner wants refusal for direct callers too, a provenance carrier on the request is a later, separate change (A1-50-02).

### (f) Profile precedence (D-13: the most specific operator statement wins)

For one (provider, model) pair the profile is resolved in this order, first match wins:

1. the per-model config override (`llm.<provider>.models.<id>` in YAML, `*Config.model_overrides` in code; D-06);
2. the adapter-wide operator `context_window` knob (`llm.<provider>.context_window`, `*Config.context_window`; plan 50-08) -- for the **window field only**;
3. the seeded, dated registry entry (D-05, D-08);
4. the adapter-wide fallback derived from what the adapter sends today.

A model with no registry entry uses the adapter-wide fallback with a `warn!` and the `paladin.unprofiled_model` response-metadata key (D-07). An unknown model is never a refusal: a new vendor model works the day it ships, visibly unprofiled.

### (g) Decorators forward `model_profile`; a chain's profile is an intersection (Pitfall 4)

`PricingLlmAdapter` and `CadenceLlmAdapter` forward `model_profile` to their inner port. A decorator that kept the default `None` would silently disable this whole policy in the composed `Pricing(Cadence(provider))` stack every server-built agent uses. `FallbackLlmAdapter` returns the `ParamSupport`/`StopSupport`/`ModelProfile` **intersection** of every hop's `Some` profile (`None` when no hop declares one), so the builder refuses what any hop could not honour. The intersection is commutative and associative, so a chain's combined profile does not depend on the order its hops are listed. A hop that returns `None` is skipped and judged only by its own request builder; plan 50-14 re-points the intersection at each step's own model (A3-50-02).

### (h) Run metadata (D-07)

Every buffered `LlmResponse` carries the `paladin.*` keys of this policy in `LlmResponse.metadata`, beside `paladin.served_by` (`paladin.params_omitted`, `paladin.unprofiled_model`, `paladin.response_format_adjusted`). That is the floor under any reading. Whether the keys **also** reach the run result (`PaladinResult`) and the streaming terminal chunk is owner reading R-1, pending (see `## Owner readings`; decided at plan 50-18 Task 1).

### (i) Registry fields that shape a request, not only sampling

- `json_object` / `json_schema`: a requested `response_format` that the model's profile does not support is **downgraded** (schema to object) or **omitted**, with one `warn!` per adapter instance per (model, format) and the `paladin.response_format_adjusted` response-metadata key, never silently. The caller's schema instruction block keeps correctness (Phase 26 D-27).
- `max_output_tokens`: a request whose effective `max_tokens` exceeds the model's limit is **refused before sending** with `LlmError::InvalidPrompt` naming provider, model, `max_tokens`, the value and the limit. There is no clamping.

## Owner readings

The planner's reading of an ambiguous owner statement is not recorded as fact. Each reading below is written `pending` with its reversibility rating and the checkpoint that decides it; the deciding plan replaces `pending` with the owner's answer and its date.

| Id | Reading | Rating | Decided at | Answer |
|----|---------|--------|------------|--------|
| R-1 | What the D-01/D-07 "run-metadata entry" is: per-call `LlmResponse.metadata` only, or also carried onto `PaladinResult` and the streaming terminal chunk | `one-way` (a public field on a constructible struct) | plan 50-18 Task 1 | `pending` |
| R-2 | SC-5's usage-cap clause, per compat vendor: live-verified against a captured quota body, or owner-accepted as unverified | `reversible` (an override is a single `error_override` added on the day a quota body is captured) | plan 50-17 Task 1 | `pending` |

## Considered Options

- **Refuse always** (handoff option a): rejected (D-01). `auto_temperature` and the framework's own `0.7` default would then refuse to build on a model such as Kimi, so a caller who never mentioned temperature is denied. ADR-0004 A1 already rejected that denial of service.
- **Omit always** (handoff option c): rejected (D-01). An operator who wrote `temperature: 0.2` against a model that cannot honour it would see a clean build and a run at the vendor default; the explicit statement is dropped without the operator being able to discover it.
- **A clean break of `ProviderCapabilities`** (add the per-model fields to it): rejected (D-03). It has 92 literal construction sites and a Phase 26 compile guard, and removing `temperature_range` would be an X-03 supersession for a v1.0 cleanup. The additive method keeps every implementor compiling.
- **An explicit/auto provenance field on `PaladinData`**: rejected. `PaladinData` has 197 literal sites and the consumer muster builds it with a thirteen-field literal; a field there is a registered break for a distinction only the builder needs.
- **Treating `llm.<provider>.default_temperature` as auto**: rejected (D-02). It is configuration the operator wrote; judging it as the framework's own would let a typo in a config file run at the vendor default unnoticed.
- **Keep ADR-0004 A2** (refuse an out-of-range auto-selected value): superseded. A2 closed CR-01 by refusing, but an auto-selected value is by definition one the caller did not state, so refusing builds that the caller cannot fix is the wrong cost; omission with a `warn!` and metadata is discoverable and substitutes nothing.

## Consequences

- An explicit temperature on a model whose profile says `NotSent` (the current Anthropic models) is now refused at build time, by name.
- `auto_temperature` on Kimi builds: the auto-selected value is omitted and recorded, not refused.
- OpenAI stops sending fabricated `temperature` and `top_p` values it was never given; the vendor default applies and the omission is recorded.
- Stop sequences reach every wire whose profile accepts them (`stop`, `stop_sequences`, `generationConfig.stopSequences`).
- Every omission is visible three ways: a `warn!`, a `paladin.params_omitted` metadata entry, and (for a refused explicit value) the build error.
- `ModelProfile` is `#[non_exhaustive]` and built through `new()` plus `with_*`; Phase 51 adds effort levels and budget bounds without a break.
- A caller that builds an `LlmRequest` directly gets omission, not refusal (the limit stated in (e)).

## Code Locations

Planned locations; measured at the closeout (plan 50-16).

- `crates/paladin-ports/src/output/model_profile.rs` -- `ModelProfile`, `ParamSupport`, `StopSupport`, `SamplingParam`
- `crates/paladin-ports/src/output/llm_port.rs` -- `LlmPort::model_profile` and the re-export of the model-profile types
- `crates/paladin-llm/src/sampling.rs` -- the shared wire filter, the omission log and the response-format and `max_tokens` checks
- `crates/paladin-llm/src/registry/` -- the per-model registry, the seeded dated tables and the per-model config override
- the five compat request builders (Kimi, Qwen, Grok, Ollama, the generic OpenAI-compatible adapter) and the four original adapters' request builders
- `crates/paladin-llm/src/{pricing,cadence,fallback}.rs` -- the three decorators forwarding `model_profile`
- `src/application/services/paladin/paladin_builder.rs` -- the build-time refusal

## Code Conformance

pending -- measured by plan 50-16

## Downstream Consumers

- **Phase 51** -- ADR-0062 extends this policy with stricter defaults for effort levels and budget bounds, on the same `#[non_exhaustive]` `ModelProfile`.
- **The web3sec consumer** -- its Anthropic agents that set `temperature` must drop it, because the current Anthropic models are `NotSent` and an explicit value is now refused.
