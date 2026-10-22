# ADR-0004: Temperature validation

## Status

Accepted

**Date:** 2026-07-31

**Amended by ADR-0061** (2026-10-15, Phase 50 plan 50-02): parameter support is now modelled per model (`ModelProfile`), an explicit value the model cannot honour is refused by name, and an auto-selected value is omitted with a `warn!` and response metadata instead of refused -- amendment A2 below is superseded on that one point; A1, A3 and A4 stand.

**Amended by ADR-0062** (2026-10-22, Phase 51): the same principle -- a value is never changed behind the caller's back -- now also governs reasoning effort, with a stricter default than ADR-0061's for sampling: an effort the target model cannot honour exactly (including any explicit effort on a model with no measured effort data) is refused before the HTTP call with `LlmError::UnsupportedParameter`, never omitted, clamped or rounded; a nearest-level substitution happens only when the caller opts in with `ReasoningFallback`, level for level. Nothing in this ADR's temperature rules changes.

## Context

Should temperature validation be provider-aware, or a single global clamp? Two REQUIREMENTS.md
variants disagree — `REQ-temperature-range-v1` (Epic 1 FR-2.3 / US-2) requires the builder to
validate `[0.0, 1.0]` and reject anything above 1.0; `REQ-temperature-range-v2` (Epic 6 REQ-5)
requires the DeepSeek adapter to support temperature `0.0-2.0`. A third position,
`REQ-dynamic-temperature` (run 2), classifies tasks into bands (Factual 0.1-0.3, Analytical
0.3-0.5, Conversational 0.5-0.7, Creative 0.7-1.0) with configurable `temperature_bounds`, and an
Epic 14 DOC states bounds of 0.1-1.0.

Two findings reframe the question rather than merely restate it. **First:** `ProviderCapabilities`
at `crates/paladin-ports/src/output/llm_port.rs:754` has **no temperature-range field at all** —
`supports_streaming`, `supports_tool_calling`, `supports_function_calling`, `supports_vision`,
`supports_embeddings`, `max_context_tokens` and `supports_system_messages` are the complete field
set. The provider-aware position (`REQ-temperature-range-v2`) was never implementable as specified
by anyone who tried to build it, because the struct it depends on does not carry the information
it would need. **Second:** the contradiction is live in the tree, not only on paper.
`src/application/services/paladin/paladin_builder.rs:1112-1117` validates and clamps to
`[0.0, 1.0]`, rejecting anything outside that range with a `ConfigurationError`, while
`crates/paladin-llm/src/config/llm.rs:14` documents `default_temperature` as "0.0–2.0" — a range
the builder's own validation would reject before it ever reached a DeepSeek call. A caller
following the documented config comment and the builder's actual behavior gets two different
answers about what temperature is legal.

## Decision

- Validation is **provider-aware**: a temperature range is added to `ProviderCapabilities`, and
  the selected provider's declared range — not a single global constant — is what a requested
  temperature is validated against.
- The new field is `pub temperature_range: Option<(f32, f32)>` on `ProviderCapabilities`,
  optional (`Option`), holding `(min, max)` as a tuple of `f32`. **Both endpoints are inclusive**,
  and the comparison is a plain float comparison at the endpoints — a requested temperature `t` is
  valid for a provider whose range is `(min, max)` exactly when `t >= min && t <= max` (equivalently,
  `(min..=max).contains(&t)`). There is no separate epsilon tolerance and no rounding; an
  implementer does not need to choose between `<` and `<=` because the contract is `<=` at both
  ends.
- **Fallback when a provider declares no range** (`temperature_range` is `None` — the field
  absent on a provider that has not yet been updated to populate it): validation falls back to
  the existing global default, `[0.0, 1.0]` inclusive, exactly as `paladin_builder.rs:1112-1117`
  validates today. This is a named, deliberate default, not silent permit-everything: it means an
  adapter that has not yet been migrated to declare its own range continues to behave exactly as
  it does today, rather than silently accepting any float once the provider-aware path lands.
- **Validation order**: the provider range (or its `[0.0, 1.0]` fallback) is checked **first**,
  against whatever temperature the caller ultimately supplies. The autonomous task-type bands
  (`TaskType::{Creative, Analytical, Standard}` and their associated temperatures in
  `src/application/services/paladin/temperature_service.rs`, with the `dynamic_temperature`
  bounds validated in `crates/paladin-core/src/platform/container/autonomous_config.rs:106-116`)
  stay **a layer above** provider validation: they are a *selection* mechanism used to pick a
  temperature for autonomous task-type guidance, not a second, independent validation gate. Where
  a task-type band and the provider's range would both constrain the same value — for example the
  Creative band's 0.8-0.9 defaults against a hypothetical provider range narrower than that — the
  band's selected value is **intersected with (clamped into) the provider's range** before use, so
  the provider range is always the outer, authoritative bound and the band narrows within it
  rather than overriding it.
- **Consequence recorded**: this makes Epic 6 REQ-5's DeepSeek `0.0-2.0` range reachable through
  the normal Paladin path, closing the gap where the builder's global `[0.0, 1.0]` clamp made it
  unreachable by construction.
- **Reversibility: costly.** `ProviderCapabilities` is a published type on the framework's
  primary integration contract (`paladin-ports`, re-exported through the prelude). Adding the
  field is additive and non-breaking on its own, but every LLM adapter (OpenAI, Anthropic,
  DeepSeek) must populate it with a correct value for the provider-aware validation to mean
  anything, and any downstream consumer that branches on `ProviderCapabilities` now has one more
  field it may need to read. Reverting this decision after adapters have been updated would mean
  removing a field consumers may already depend on.

## Considered Options

- `REQ-temperature-range-v1` (Epic 1 FR-2.3 / US-2, builder MUST validate `[0.0, 1.0]` globally)
  — rejected as the sole answer; it is what the builder validates today, but applied literally it
  makes `REQ-temperature-range-v2`'s DeepSeek `0.0-2.0` range permanently unreachable through the
  normal Paladin path, which this ADR records as an explicit capability withdrawal it is not
  willing to make silently.
- `REQ-temperature-range-v2` (Epic 6 REQ-5, DeepSeek adapter MUST support temperature `0.0-2.0`)
  — accepted in substance as the motivating case for provider-aware validation, but on its own it
  says nothing about *how* a provider's range should be represented or checked, which is what this
  ADR's `temperature_range` field and its endpoint contract supply.
- A global `[0.0, 1.0]` clamp, kept exactly as shipped today — rejected. Per D-15's framing,
  keeping it would require recording Epic 6 REQ-5 as withdrawn: DeepSeek's documented `0.0-2.0`
  support would never be reachable, which is an explicit capability reduction, not a neutral
  status quo.
- Adapter-level clamping (each LLM adapter silently clamps an out-of-range temperature to its own
  legal range before sending the request) — rejected. Clamping silently alters the caller's value
  past the point where the caller can be told they were wrong; a caller who requested 1.8 and got
  1.0 silently substituted has no way to discover the substitution happened. Validating at the
  ports boundary and rejecting (or explicitly reporting) an out-of-range value keeps the caller
  informed instead.
- `REQ-dynamic-temperature` (task-type bands with configurable `temperature_bounds`) — not
  rejected; it **interacts with, rather than competes with**, this decision. The bands remain a
  selection mechanism for autonomous task-type guidance, now explicitly subordinate to (narrowing
  within) the provider's declared range rather than an independent, unrelated validation surface.

## Code Locations

- `crates/paladin-ports/src/output/llm_port.rs:754` — `struct ProviderCapabilities`, currently with no temperature-range field; this ADR adds `temperature_range: Option<(f32, f32)>` here
- `src/application/services/paladin/paladin_builder.rs:1112-1117` — the shipped global `[0.0, 1.0]` clamp, which becomes this ADR's no-provider-range fallback rather than the sole validation path
- `crates/paladin-llm/src/config/llm.rs:14` — the documented `0.0–2.0` default temperature range that the shipped clamp above currently contradicts
- `src/application/services/paladin/temperature_service.rs:12-47` — `TaskType` and `TemperatureConfig`, the task-type band logic that stays a layer above provider validation
- `crates/paladin-core/src/platform/container/autonomous_config.rs:106-116` — `dynamic_temperature` bounds validation (min/max within `[0.0, 1.0]`, min < max), the autonomous-config side of the band logic

## Code Conformance

must change

`ProviderCapabilities` at `llm_port.rs:754` has no `temperature_range` field today. **GAP-07** in
Phase 2 is the requirement that lands the ports-layer change (the new field, its documentation,
and each adapter populating it). Nothing in this phase edits Rust source — this ADR records the
decision only.

## Downstream Consumers

- Phase 2 GAP-07 — implements the `temperature_range` field on `ProviderCapabilities` and the
  validation-order contract this ADR specifies.
- The three shipped LLM adapters (OpenAI, Anthropic, DeepSeek) — each must populate
  `temperature_range` with its provider's actual supported range once GAP-07 lands the field, or
  validation silently falls back to `[0.0, 1.0]` for that provider.
- **Sequencing note:** Phase 14's **WEB-03** ("Make `ProviderCapabilities` report the tool-calling
  support the adapters actually have") modifies the same `ProviderCapabilities` struct — a
  different field (`supports_tool_calling`), but the same struct in the same file. These two
  changes must **not be scheduled independently**: landing them separately risks each change
  being written against a stale version of the struct and produces avoidable merge conflicts on
  `llm_port.rs:754` and on every adapter's `ProviderCapabilities` construction site. GAP-07 and
  WEB-03 should land in the same change, or be explicitly sequenced with the second rebasing onto
  the first.

---

## Amendment — 2026-08-23 (Phase 17, plans 17-19 and 17-21)

**Status of this amendment:** Accepted. The original decision above stands; this narrows *when*
the gate fires and records two boundary cases live measurement forced. Flagged as a required
phase-close act by `17-19-SUMMARY.md`.

### A1 — The gate fires only on a temperature the caller actually expressed

`PaladinData::default()` fabricates `temperature: 0.7`. The original gate could not tell that
fabrication apart from a deliberate request, so it judged both against the provider's declared
range. That became a denial of service the moment a provider declared a *narrow, truthful* range:
Kimi's live-measured constraint is the degenerate `(1.0, 1.0)`, which meant **every** Paladin
built against Kimi was rejected — including callers who never mentioned temperature.

The gate in `paladin_builder.rs` now fires only when `manual_temperature_override` is set. A
caller who said nothing is not judged; a caller who asked for an illegal value is still refused by
name, with the legal endpoints in the message, exactly as before.

This is a narrowing of *what is judged*, not a weakening of the judgement.

### A2 — Auto-selected temperatures are judged too (closing CR-01)

The narrowing in A1 opened a hole that code review caught (`17-REVIEW-gaps.md`, CR-01). The
auto-temperature branch in `PaladinBuilder::build` assigns an LLM-chosen temperature but never
sets `manual_temperature_override` — and `validate()` runs *before* that branch regardless. An
auto-selected value therefore reached `PaladinData` with no provider check at all.

The value is now validated at the point of assignment. Consistent with this ADR's core position,
an out-of-range auto selection is **refused by name, never clamped**: a caller who received a
substituted value has no way to discover the substitution happened.

Note the gap predates the A1 narrowing — it only became *reachable* once a provider declared a
range an auto-selected value could fall outside.

### A3 — Omission is not clamping

Some vendors reject a sampling parameter's *presence*, not merely its value. Kimi accepts only
`temperature=1`; xAI rejects `presence_penalty` outright on every current model.

The mechanism adopted (`CompatRequestParameters`, plan 17-18) lets a preset declare which
parameters its request path carries; undeclared ones are **omitted from the request body entirely**
— the JSON key is absent, never `null` — so the vendor's own default applies.

This is explicitly **not** the adapter-level clamping the Considered Options above rejected. No
legal value is substituted for another; the parameter is simply not sent. The distinction matters:
clamping silently rewrites a caller's stated intent, whereas omission declines to state an intent
the caller never had.

### A4 — A declared range must be the measured one, including half-open bounds

`temperature_range` states what the vendor was *measured* to enforce, not a comfortable figure
copied from another provider. Two live measurements on 2026-08-22/23 illustrate the cost of
guessing:

- **Kimi** enforces a single legal value; declared `(1.0, 1.0)`, which drove A1.
- **Qwen** accepts the *half-open* `[0.0, 2.0)` — `2.0` itself is refused. Since
  `temperature_range` is an inclusive pair, it is declared `(0.0, 1.99)`. An inclusive `(0.0, 2.0)`
  would have advertised a value the vendor rejects.

A declared range that is wrong in the permissive direction is worse than no declaration: it moves
the failure from a local build error to a vendor round trip.

### Code locations added by this amendment

- `src/application/services/paladin/paladin_builder.rs` — the `manual_temperature_override` gate (A1) and the auto-branch validation (A2)
- `crates/paladin-llm/src/compat/engine.rs` — `CompatRequestParameters` and its application in `build_request` (A3)
- `crates/paladin-llm/src/kimi/adapter.rs` — the `(1.0, 1.0)` declaration and temperature/top_p omission (A1, A3, A4)
- `crates/paladin-llm/src/qwen/adapter.rs` — the `(0.0, 1.99)` half-open declaration (A4)
