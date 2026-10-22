# ADR-0062: Reasoning effort -- one provider-neutral setting, honoured exactly or refused before HTTP

## Status

Accepted

**Date:** 2026-10-22

**Phase:** 51 (Adjustable Reasoning Effort), plan 51-13; Code Conformance measured on the tree plans 51-01 to 51-13 produced.

**Amends ADR-0004** (temperature validation) and extends ADR-0061 (per-model parameter support): the same principle -- a value is never changed behind the caller's back -- applied to a parameter whose silent failure is expensive rather than merely wrong, with a stricter default than ADR-0061's.

## Context

Reasoning models spend output tokens thinking before they answer, and how much they spend is a vendor-side setting. Left unset it is the vendor's default, and that default can be extreme: the motivating measurement (handoff section 4.1, `kimi-k3`, 2026-10-13) is that an unconfigured request is a `max`-effort request that spent its whole 16,000-token cap in about eight minutes and returned no answer, while `low` answered in 56 seconds and `thinking: {type: disabled}` in 2. Effort is a cost, latency and answer-quality lever, and a request that silently ran at a different effort than the caller wrote is the "undiscoverable substitution" ADR-0004 exists to prevent.

The vendors make that failure easy. At least four return HTTP 200 while the requested effort is not what ran: Kimi ignores a top-level `reasoning_effort` (measured, handoff section 5), DeepSeek coerces `minimal`, `medium` and `xhigh`, xAI treats `xhigh` as `high` on `grok-4.5`, and Gemini's OpenAI-compatible endpoint maps `minimal` to `low` on 3.1 Pro. A design that sent an OpenAI-style `reasoning_effort` from the shared compat engine for every preset would have been a silent no-op on Kimi, the provider that most needs the setting. The vendors' documentation also disagreed with the measurements (Moonshot's pages say K3 takes the top-level field; the probe shows it is ignored), so desk research cannot be the source of what is encoded.

ADR-0061 already gave every model a `ModelProfile` and put sampling parameters under a build-time-refuse, wire-time-omit policy. Effort cannot reuse the omit half: omitting an effort the caller asked for is exactly the silent substitution at issue, and an unmeasured model cannot be given the benefit of the doubt, because a wrong effort silently changes cost, latency and answers.

The decisions below are the owner's answers at `/gsd-discuss-phase 51` (D-00c to D-00i and D-01 to D-11 in `51-CONTEXT.md`), plus the dated checkpoint answers under `## Owner checkpoints`. No decision was re-opened. The motivating hazards, the vendor survey and the recommended shape are in `51-HANDOFF-web3sec-reasoning-effort.md`; the evidence every encoded fact comes from is `51-EFFORT-PROBE.md`.

## Decision

### (a) One neutral setting, nine values (D-00d)

`ReasoningEffort` is a `#[non_exhaustive]` enum: `Off`, `Auto`, `Minimal`, `Low`, `Medium`, `High`, `XHigh`, `Max` and `Budget(u32)`, in the domain core (`paladin-core`, re-exported by `paladin-ports`) so `PaladinData` can hold it and dependencies keep flowing inward. The six levels are ranked `Minimal < Low < Medium < High < XHigh < Max`; `Off`, `Auto` and `Budget` are not levels and are never ranked or substituted for one another. `Auto` ("on, at the vendor's default depth") is a value, distinct from unset. The serde spelling of `XHigh` is `xhigh` (pinned; the derive's default would be `x_high`) and a budget is `{budget: n}`.

### (b) Where it is set, and in what order (D-00e)

`LlmRequest.reasoning` (`with_reasoning`), then `PaladinData.reasoning_effort` (`PaladinBuilder::reasoning_effort`, YAML `agents[].reasoning_effort`), then a `reasoning_effort` knob on the nine `*Config` types through `config_knob_builders!` (YAML `llm.<provider>.reasoning_effort`), then the vendor's own default. Never `PromptParameters`, never `ProviderCapabilities` (its `provider_capabilities_gained_no_field` guard stays green). Each level is resolved field by field, per request, against the request's own model, never once at construction. Only a provider-wide default exists; a per-model default is not offered.

### (c) Fail loudly, before HTTP (D-00f, D-03)

An effort the model cannot honour exactly is refused **before the HTTP call** with a new additive `LlmError::UnsupportedParameter { param, value, provider, model, reason }`, classified `Permanent` (never retried, never failed over) and given a `llm_failure` label row. The five-field shape is the roadmap's four plus `reason`, so the text can name the key that fixes it (A1-51-01). One function, `EffortSupport::resolve`, is the single admission rule the adapters, `PaladinBuilder` and the chain pre-flight all call, so they cannot drift. It refuses a level the model lacks or gates off, `Off` where reasoning cannot be switched off, `Auto` where there is no vendor default depth, a `Budget` where only levels exist or outside the accepted span, a level the vendor is known to coerce silently (`coerced`), and **any explicit effort on a model with no measured effort data** (`Unmeasured`), with the error naming the `llm.<provider>.models.<model-id>.effort` override that unlocks it. It is deliberately stricter than ADR-0061's warn-and-fall-back for sampling. It never uses the compat engine's omit-and-log path. A `Budget` must also stay below the request's effective `max_tokens`.

### (d) Effort data comes only from a dated live probe (D-01, D-02, D-04)

Per-model effort support is a new additive field on `ModelProfile` (`effort: EffortSupport`, `effort_verified_on`), written **only where measured**, each entry with its probe date. The source is `crates/paladin-llm/examples/live_vendor_smoke.rs --effort`, run on the operator's keys; its scrubbed transcript is `51-EFFORT-PROBE.md` (2026-10-22, commit `32ce9dd0`, 62 models over seven providers). ADR-0061's seed rule is unchanged, word for word: a value with no evidence in a transcript is not written. A model the probe did not measure stays `Unmeasured`; a model that does not reason is `NoReasoning` only if measured (otherwise `Unmeasured`). **A wire form that cannot be confirmed live ships `Unsupported`**: `ReasoningWire::Unsupported` refuses every explicit effort, and the page's mapping table says `unverified:` with the reason. A form is confirmed when its invalid value draws a 4xx (the field is parsed) and a higher level reports more reasoning tokens (the field has effect).

### (e) The wire is the probe's, not the vendor documentation's (D-00h, D-04)

Each vendor's field is the form the probe confirmed, decided by the owner on 2026-10-22: OpenAI, DeepSeek and Grok top-level `reasoning_effort`; **Kimi `thinking.effort`**, and the top-level field is never emitted (Kimi answers 200 to an invalid value there: the silent no-op, pinned by `kimi_preset_never_emits_top_level_reasoning_effort`); Qwen `enable_thinking` plus `thinking_budget` (off, auto and a budget, no named levels, which refuse); Anthropic `output_config.effort`, paired with `thinking: {type: adaptive}` on exactly the models whose effort alone measured no thinking tokens (`claude-opus-4-8`, `claude-opus-4-7`, `claude-opus-4-6`, `claude-sonnet-4-6`), and the deprecated `budget_tokens` form never sent to a model that rejects it; Gemini lower-case `generationConfig.thinkingConfig.thinkingLevel`. Ollama and OpenAI-compatible declare `Unsupported` unless the operator supplies a wire (decision (i)). Every compat preset must state its reasoning position explicitly: `CompatRequestParameters` has no `Default`, and the convenience constructor `all()` now takes the wire as an argument. No native model seeds a `Budget` bound, because Anthropic and Gemini document no fixed maximum and the seed rule needs one.

### (f) Unset is byte-identical; no compiled-in default (D-00i)

`reasoning: None` at every level produces today's request body on every adapter, pinned by a golden wire-body case per adapter and preset written before any adapter edit (`unset_reasoning_leaves_the_wire_body_unchanged`, nine fixtures, `UPDATE_REASONING_GOLDEN` to re-bless). Consequently **no provider has a compiled-in effort, Kimi included**: the requirement that `kimi-k3` at documented defaults answers is met by the Reasoning Effort page's measured configuration (`reasoning_effort`, `max_tokens` and `timeout_seconds` together), not by a baked-in value. Reasoning-capable models already default to a 300 s timeout (Phase 50 D-13); `EmptyCompletion` with finish `length` is the runtime backstop for an output cap that is too small, not a validation.

### (g) The opt-in nearest level (D-05, D-06, D-07)

A plain `ReasoningFallback` setting beside the effort, with the same three places and precedence (request, agent, provider): `Refuse` (the default), `NearestLower` (cost- and latency-safe) and `NearestHigher` (quality-safe). "Nearest" is measured along the ranked scale against the model's **measured** levels; when no level exists in the chosen direction the request refuses (the direction is never flipped). A substitution logs one `warn!` per (provider, model, requested, effective) and records both efforts in the metadata. It substitutes **level for level only**: `Off` on an always-thinking model, any `Budget` <-> level conversion, a vendor-coerced level and any unmeasured model **still refuse** with the opt-in set. A coerced level is also never a substitution target.

### (h) Chains and build time (D-08, D-09)

`FallbackLlmAdapter` resolves the request's effort and fallback against **every hop's own model** (`FallbackStep.model`, through each port's `model_profile`) before the first HTTP call; if any hop can neither honour nor substitute, the whole request refuses with `fallback hop <index>: <reason>`, so a misconfigured chain fails on its first call, not during an outage of hop 1. Each hop reached sends its own resolved effort. `PaladinBuilder::build` and the agent host refuse an agent- or provider-level effort the agent's model cannot honour (for a chain, against the intersected profile, which gains an effort rule consistent with its AND semantics), naming `agents[<id>].reasoning_effort` or `llm.<provider>.reasoning_effort`. The adapter's pre-HTTP check stays the authority for per-request values.

### (i) Self-hosted endpoints (D-10)

`OllamaConfig` and `OpenAiCompatibleConfig` gain `with_reasoning_wire(..)` and YAML `llm.<provider>.reasoning_wire`; the levels come from `llm.<provider>.models.<model-id>.effort` overrides. **Both** are required before any explicit effort is sent; the compiled default stays `Unsupported` for both. The operator owns verification, supported by the effort probe accepting an `ollama` or `openai-compatible` target: its invalid-value-rejected row is the proof the field is parsed. No network call at construction. **Self-hosted only:** the named vendors take no wire override (a `reasoning_wire` on one is a configuration error), so the shipped table stays the truth for them.

### (j) Observability (D-11, REASON-05)

The requested and effective effort go into `LlmResponse.metadata` as `paladin.reasoning_requested` and `paladin.reasoning_effective` (only when an effort was in force; on a stream, on the terminal chunk), into `PaladinResult.run_metadata` by Phase 50's merge rule, and onto the trace stream as `TraceEvent::NodeFinished.reasoning_requested` / `reasoning_effective` (`#[serde(default, skip_serializing_if = "Option::is_none")]`). Every adapter fills `TokenUsage.reasoning_tokens` when the vendor reports it. Reasoning **text** is not requested (D-00g): `LlmResponse.reasoning` and `StreamingResponse.reasoning` are filled where the vendor sends it; no second surface was added. The node-cache fingerprint includes an agent's effort and fallback only when set, so unset keys are unchanged; a provider-level default is not in the key.

### (k) The record is kept honest by tests (D-00c, REASON-07)

The mdBook page `docs/src/user-guides/reasoning-effort.md` carries a mapping table of every provider's declared wire and every seeded model's cells (`exact`, `refused`, `refused (coerced)`, `budget <min>-<max>`, the verified-on date or `unmeasured`), and `registry::reasoning_page_tests` derives each expected cell from the built adapters (`model_profile` after wire masking, through `EffortSupport::resolve`), so a row cannot show a refused effort as working. This ADR is recorded as **amending ADR-0004**, which gains a dated `**Amended by ADR-0062**` note; `PROMOTION.md` advances to 0063.

## Owner checkpoints

Each checkpoint was a blocking decision resolved by the owner before the plan that applied it ran; the words are recorded as handed to the executor.

| Plan | Date | Decision | Owner's answer |
|------|------|----------|----------------|
| 51-10 | 2026-10-22 | Probe option, candidate set, D-02 treatment of an unprobed seeded model | Option `run-here` ("run here."); candidate set confirmed, no ids struck or added ("that should be the ones that have been previously probed ... the models that are the currently active latest models available"); D-02: "D02 unmeasured" (`EffortSupport::Unmeasured`, not `NoReasoning`). |
| 51-11 | 2026-10-22 | Wire for Kimi, Grok and Qwen; the other four vendors' wires for plan 51-12 | "apply-verdicts; Qwen: ship EnableThinking with off/auto/budget seeded from the probe". Kimi `thinking.effort` (top-level never shipped); Grok top-level `reasoning_effort`; Qwen keeps `enable_thinking` by an explicit exception to the mechanical D-04 rule (it has no level rows to confirm a budget-only wire); OpenAI, DeepSeek, Anthropic (with the four-model pairing set) and Gemini (lower case) confirmed for plan 51-12. |
| 51-12 | 2026-10-22 | Tracer approval with five judgement calls | "approved": no Anthropic or Gemini budget bound is seeded (no documented maximum); Gemini `minimal` is seeded per row; `off` and `auto` are read literally from the probe's token counts; `enable_thinking_with_level` is set on exactly the four pairing models; unprobed seeded rows stay `Unmeasured`. |

## Considered Options

- **Desk-research seeding with a "documented" tier** (D-01): rejected. It breaks ADR-0061's seed rule, and the vendors' documentation contradicted the measurements (Kimi). Overrides only was also rejected: every model would refuse out of the box.
- **Probe every seeded model, or web3sec's models plus one per vendor** (D-02): rejected in favour of every seeded entry, with the owner striking or adding ids before the run; the result was 62 ids, all probed. Treating a non-reasoning seeded entry as "does not reason" was replaced, at the owner's checkpoint, by `Unmeasured`, because the seeded `reasoning` flag means "returned reasoning text", not "can reason" (research Pitfall 1).
- **Adapter-wide fallback with `warn!` and `paladin.unprofiled_model` for an unmeasured model** (D-03): rejected. It is ADR-0061's behaviour for sampling, but a wrong effort silently changes cost, latency and answers.
- **Ship the desk-research wire form where a probe cannot confirm it** (D-04): rejected. The wire is `Unsupported` and the row reads `unverified` until a probe confirms it.
- **Nearest-level fallback configured at provider level only, or on the request only** (D-05): rejected. Request-level reach is required because web3sec builds adapters directly, and provider-level reach is what operators need.
- **Always round down, or always round up** (D-06): rejected. The direction is the caller's choice.
- **Also substitute `Off` with the lowest level, or map a `Budget` to a level heuristically** (D-07): rejected. Those change the kind of request, not its depth; they stay refusals.
- **Validate each fallback hop lazily and stop the chain, or skip incompatible hops** (D-08): rejected. The first hides a misconfiguration until failover; the second loses hops silently.
- **Send-time checks only** (D-09): rejected. `PaladinBuilder` and the agent host also refuse at build, as Phase 50 does for explicit sampling values.
- **Per-model levels only with fixed wire forms for self-hosted endpoints; nothing this phase; overridable wire forms on every compat preset** (D-10): rejected. Only the two self-hosted adapters take a declared wire; the named presets change through a probe and a release.
- **Send an OpenAI-style `reasoning_effort` from the shared compat engine for every preset** (handoff section 5): rejected. It is a silent no-op on Kimi.
- **A compiled-in effort default for Kimi** (D-00i): rejected. It would change today's request body and break the unset-is-identical guarantee; the page documents the configuration instead.
- **Reuse ADR-0061's omit-and-log path for effort**: rejected. Omitting an effort the caller asked for is the silent substitution this ADR forbids.
- **A new `LlmError` variant carrying the roadmap's four fields only**: rejected in favour of a fifth `reason` field, so the error can name the fixing key.

## Consequences

- An explicit effort on a model nobody measured is refused until an operator declares it. Loosening this later is a behaviour change consumers may already depend on (D-03: **costly** to reverse).
- `ReasoningFallback` is a public type and fields on `LlmRequest`, `PaladinData` and nine `*Config` types (D-05: **costly**). `ReasoningWire` is reachable from two `*Config` types and YAML (D-10: **costly**). The probe data, the per-hop pre-flight and the `ModelProfile` fields are **reversible** (data rows with their own dates, behaviour inside `FallbackLlmAdapter`).
- A provider-wide default refuses on every model that cannot honour it, naming the key that set it: set it only where every served model supports it.
- Some surfaces are registered breaks, taken in this phase and recorded in `MIGRATION.md` 9.2 with allowlist entries: `PaladinData`, `AgentDefinition`, `TraceEvent` and `CompatRequestParameters`. Every other touched type is `#[non_exhaustive]` and the change is additive.
- `EffortSupport::intersect` makes a chain's build-time check conservative (a level must be usable on every hop) while the send-time check lets each hop substitute differently.
- The OpenAI Responses API and the command-line tool's own agent YAML are not covered; levels are not discovered from vendor model listings.
- A new model, or a vendor's changed wire, needs a probe and a one-line data or wire change before it takes an explicit effort; the page's drift test fails until the table and the code agree.

## Code Locations

- `crates/paladin-core/src/platform/container/reasoning.rs` -- `ReasoningEffort`, `ReasoningFallback`, the two metadata-key constants
- `crates/paladin-ports/src/output/model_profile.rs` -- `EffortSupport`, `EffortLevels`, `BudgetBounds`, `EffortRefusal`, `ResolvedEffort`, `EffortSupport::resolve` / `intersect`, `ModelProfile.effort` / `effort_verified_on`
- `crates/paladin-ports/src/output/llm_port.rs` -- `LlmRequest.reasoning` / `reasoning_fallback`, `LlmError::UnsupportedParameter`, its `Transience` row and the `llm_failure` label row
- `crates/paladin-llm/src/reasoning.rs` -- `ReasoningWire`, `admit`, `in_force`, `WireCaps`, `mask_effort`, `SubstitutionLog`
- `crates/paladin-llm/src/registry/mod.rs` and `registry/seeded.rs` -- `ModelOverride.effort`, the dated effort rows traced to `51-EFFORT-PROBE.md`, `reasoning_page_tests`
- `crates/paladin-llm/src/{openai,anthropic,deepseek,gemini}/adapter.rs` and `compat/engine.rs` with the `kimi`, `qwen`, `grok`, `ollama`, `openai_compatible` presets -- each adapter's `map_reasoning` and wire caps
- `crates/paladin-llm/src/fallback.rs` -- the per-hop `preflight_reasoning`; `config/{llm,layers,bridge}.rs` and `provider_factory.rs` -- `llm.<provider>.reasoning_effort`, `reasoning_fallback`, `reasoning_wire`
- `crates/paladin-core/src/platform/container/paladin.rs`, `src/application/services/paladin/paladin_builder.rs`, `paladin_execution_service.rs`, `src/config/agents.rs`, `src/infrastructure/web/agent_host.rs` -- the agent level and the build-time refusals
- `crates/paladin-core/src/platform/container/trace.rs` and `crates/paladin-battalion/src/engine/cache_key.rs` -- the trace fields and the node-cache fingerprint
- `crates/paladin-llm/src/conformance.rs`, `crates/paladin-llm/examples/live_vendor_smoke.rs` -- the offline wire cases and the live `--effort` / `--effort-smoke` probes
- `.planning/phases/51-adjustable-reasoning-effort/51-HANDOFF-web3sec-reasoning-effort.md` (the source document) and `51-EFFORT-PROBE.md` (the evidence) -- traced for the decisions above
- `docs/src/user-guides/reasoning-effort.md` -- the published guide and the drift-checked mapping table

## Code Conformance

conforms

Measured on the tree plans 51-01 to 51-13 produced. Each decision group is proved by named tests:

- **(a) The type.** `effort_spellings_round_trip_through_serde` pins all nine spellings, including `xhigh`; `ReasoningFallback` has its own round trip.
- **(b) Placement and precedence.** The `reasoning::in_force` tests, the provider-factory `reasoning_defaults` cases (request wins, same level, neither set, a refused default naming the key), the `paladin_execution_service` request-site tests, and the agent YAML tests in `config::agents` and `agent_host`.
- **(c) Fail loudly.** The `EffortSupport::resolve` table tests in `model_profile.rs` (including `a_coerced_level_is_never_a_substitution_target`), the `reasoning::admit` tests, and the conformance case `explicit_reasoning_on_an_unprofiled_model_refuses_before_http` on all nine adapters (buffered and streaming, zero requests).
- **(d) Dated evidence.** `registry::seeded_tests::every_seeded_effort_entry_validates`, the per-vendor `*_effort_seed_matches_the_transcript` tests, and the `// Source for the effort data` comment on every `.with_effort(..)` row.
- **(e) The wire.** `kimi_preset_never_emits_top_level_reasoning_effort` (all nine values under all three fallbacks), the per-adapter wire-body tests, and the conformance cases `seeded_effort_levels_reach_the_wire` and `every_seeded_effort_level_round_trips_to_its_wire_body` over every seeded row and level of every adapter.
- **(f) Unset is identical.** `unset_reasoning_leaves_the_wire_body_unchanged`: the nine golden bodies under `crates/paladin-llm/tests/golden/reasoning_unset/`, written before any adapter edit and green after every task.
- **(g) The opt-in.** The fallback tests in `model_profile.rs` (direction never flips, nothing substituted for `Off`, `Auto` or `Budget`) and each adapter's substitution cases.
- **(h) Chains and build time.** The `fallback::` pre-flight tests (`a_later_hop_that_cannot_honour_the_effort_refuses_even_when_the_first_would_serve`, `a_hop_with_no_profile_at_all_refuses_an_explicit_effort`), the `paladin_builder` effort tests (`an_effort_the_model_does_not_list_is_refused_at_build_by_name`, `a_chain_builds_an_effort_every_hop_lists_and_refuses_one_a_hop_lacks`) and `agent_host` `a_blocks_default_effort_the_model_cannot_honour_refuses_the_agent_naming_the_block_key`.
- **(i) Self-hosted.** The `ollama_declares_no_reasoning_wire_by_default` and `openai_compatible` wire tests, the `config::layers` test that a named vendor refuses `reasoning_wire`, and the harness's own plan tests for the `ollama` / `openai_compatible` targets.
- **(j) Observability.** The `NodeFinished` emission tests in `crates/paladin-battalion/src/engine/mod.rs`, the node-cache key tests (`unset_effort_key_is_the_pinned_literal`, `key_changes_when_the_agents_effort_changes`), the metadata stamping tests on buffered and terminal-chunk paths, and `usage_fields_are_mapped` on all nine adapters.
- **(k) The record.** `registry::reasoning_page_tests` (14 cases): the page is compared with the nine built adapters, a deleted row for any measured seeded model fails naming it, an edited cell fails naming the row and the column, an unmeasured row cannot read `exact`, a provider without a wire cannot name one, and a tree without `docs/` is a recorded skip while a tree with `docs/` but no page fails.

**Stated limits.** (1) Seven of nine providers were probed on 2026-10-22; Ollama and OpenAI-compatible were skipped and ship `Unsupported`. (2) The probe's samples are single calls, not averages; the "higher level reports more tokens" test therefore held on 14 of 17 OpenAI, 7 of 9 Anthropic and 1 of 2 Kimi models and is a signal, not a cost model. (3) `qwen-max` and `qwen-plus-latest` carry `off` only because the probe's `auto` and `budget` rows did not meet the seed rule; 32,768 is the lowest of several Qwen budget ceilings. (4) The wire-field text on the page for the seven named vendors is pinned by the adapters' wire-body tests rather than read from the adapters by the drift test. (5) `cargo-semver-checks` was absent while plans 51-07 to 51-09 registered their breaks; plan 51-14 measures them. (6) A provider-level default effort is not part of the node-cache key.

## Downstream Consumers

- **The web3sec consumer** -- sets effort at request level on directly built adapters for `claude-opus-4-8`, `deepseek-v4-pro` and `kimi-k3`; all three were probed end to end and accept `low`, `high` and `max` (Anthropic also `medium` and `xhigh`). DeepSeek's `minimal`, `medium` and `xhigh` are refused as coerced; set a `reasoning_fallback` if that is not wanted.
- **Operators of mixed-model providers** -- a provider-wide `reasoning_effort` refuses on any served model that cannot honour it; set it on the agent instead.
- **Plan 51-14** -- re-runs the page's `llm.kimi` example live and measures the registered breaks.
- **Future phases** -- an OpenAI Responses API wire plugs its `reasoning: {effort}` form into the same `map_reasoning` and profile data.
