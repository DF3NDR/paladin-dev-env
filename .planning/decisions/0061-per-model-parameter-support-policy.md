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

### (h) Run metadata (D-01, D-07; owner reading R-1 = option-b)

The "run-metadata entry" of D-01 and D-07 reaches **every caller, buffered and streamed**. The owner decided this at plan 50-18 Task 1 on 2026-10-16 (R-1, `one-way`; see `## Owner readings`); it is not the planner's reading.

- **Per call.** Every buffered `LlmResponse` carries the `paladin.*` keys of this policy in `LlmResponse.metadata`, beside `paladin.served_by` and `paladin.served_model` (`paladin.params_omitted`, `paladin.unprofiled_model`, `paladin.response_format_adjusted`). This was the floor of plans 50-03, 50-05 and 50-14.
- **Per run.** `PaladinResult.run_metadata: BTreeMap<String, String>` carries those keys onto the run result, keyed as stamped (the `paladin.` prefix kept). `PaladinExecutionService` merges every model call's keys into it: `paladin.params_omitted` and `paladin.unprofiled_model` hold the de-duplicated union of their comma-separated items in first-seen order; every other `paladin.*` key holds the last call's value; `paladin.served_by` is not repeated (it has its own field, `PaladinResult.served_by`); a key that does not start with `paladin.` is never copied. A run whose calls stamped nothing has an empty map and serialises byte-identically to before (`#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`).
- **Per stream.** `StreamingResponse.metadata: HashMap<String, String>` (additive on the `#[non_exhaustive]` struct, `StreamingResponse::with_metadata`) carries the same keys the buffered path would stamp **on the stream's terminal chunk only**, for the compat engine, OpenAI, DeepSeek, Anthropic and Gemini; content chunks carry an empty map, and an `Err` terminal item (refusal, empty completion) carries none. A fallback chain's stream names the serving hop on its terminal chunk with `paladin.served_by` and `paladin.served_model`, as its buffered path does. The execution service's streamed run has no `PaladinResult` -- its result is the final `PaladinStreamChunk` -- so it merges the terminal chunk's keys by the same rule into `ChunkMetadata.run_metadata` (additive on the `#[non_exhaustive]` struct, `ChunkMetadata::with_run_metadata`), which also carries `paladin.served_by` because a stream chunk has no dedicated field for it.
- **Not extended.** The `paladin-web` HTTP body (`ExecuteResponse`) does not carry `run_metadata`; exposing it over HTTP is a separate, published-contract decision (A2-50-18).
- **Cost.** One registered break on the constructible `PaladinResult` (a full struct literal downstream breaks; `..Default::default()` keeps compiling), measured by the D-27 run and recorded in MIGRATION.md 9.2 on the existing `paladin-ai-core | PaladinResult` row. The consumer takes it in its single move to v0.11.0.
- **Content.** The values are provider names, model ids and parameter names only -- never request content or credentials -- so carrying them onto a persisted or serialised result adds no secret (A1-50-18, T-50-18-01).

### (i) Registry fields that shape a request, not only sampling

- `json_object` / `json_schema`: a requested `response_format` that the model's profile does not support is **downgraded** (schema to object) or **omitted**, with one `warn!` per adapter instance per (model, format) and the `paladin.response_format_adjusted` response-metadata key, never silently. The caller's schema instruction block keeps correctness (Phase 26 D-27).
- `max_output_tokens`: a request whose effective `max_tokens` exceeds the model's limit is **refused before sending** with `LlmError::InvalidPrompt` naming provider, model, `max_tokens`, the value and the limit. There is no clamping.

## Owner readings

The planner's reading of an ambiguous owner statement is not recorded as fact. Each reading below was written `pending` with its reversibility rating and the checkpoint that decides it; the deciding plan replaced `pending` with the owner's answer and its date. Both are answered (R-2 at plan 50-17, R-1 at plan 50-18, both 2026-10-16); none is pending.

| Id | Reading | Rating | Decided at | Answer |
|----|---------|--------|------------|--------|
| R-1 | What the D-01/D-07 "run-metadata entry" is: per-call `LlmResponse.metadata` only, or also carried onto `PaladinResult` and the streaming terminal chunk | `one-way` (a public field on a constructible struct) | plan 50-18 Task 1 | 2026-10-16: `option-b` -- "option-b: the run-metadata entry reaches every caller, buffered and streamed; PaladinResult.run_metadata and the streaming terminal chunk carry the paladin.* keys. One registered break taken before the consumer's single move to v0.11.0." |
| R-2 | SC-5's usage-cap clause, per compat vendor: live-verified against a captured quota body, or owner-accepted as unverified | `reversible` (an override is a single `error_override` added on the day a quota body is captured) | plan 50-17 Task 1 | 2026-10-16, per vendor: kimi `accept unverified`; qwen `accept unverified`; grok `accept unverified`; ollama `accept unverified`; openai-compatible `accept unverified`. No quota-exhaustion body was captured for any compat preset (see `50-SEED-PROBE.md`); each keeps today's error mapping and is recorded as unverified, with no override encoded. |

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

Measured at the closeout (plan 50-16, 2026-10-16), by `git diff 865a3968..HEAD` (the parent of the first Phase 50 code commit).

- `crates/paladin-ports/src/output/model_profile.rs` -- `ModelProfile`, `ParamSupport`, `StopSupport`, `SamplingParam`
- `crates/paladin-ports/src/output/llm_port.rs` -- `LlmPort::model_profile`, the re-export of the model-profile types, `LlmError::Refusal`, `LlmResponse::new` / `reasoning`, `StreamingResponse.reasoning` / `metadata`
- `crates/paladin-ports/src/output/paladin_port.rs` and `crates/paladin-core/src/platform/container/execution_result.rs` -- `ChunkMetadata.run_metadata` and `PaladinResult.run_metadata` (decision (h), owner reading R-1)
- `crates/paladin-llm/src/sampling.rs` -- the shared wire filter, the omission log, the response-format and `max_tokens` checks, `RequestNotes`
- `crates/paladin-llm/src/registry/mod.rs` and `registry/seeded.rs` -- the per-model registry, `ModelOverride`, the dated per-vendor tables (every value traced to `50-SEED-PROBE.md`)
- `crates/paladin-llm/src/compat/engine.rs` and the five compat presets (`kimi`, `qwen`, `grok`, `ollama`, `openai_compatible` `adapter.rs`) -- the compat request builder through the filter
- `crates/paladin-llm/src/{openai,anthropic,deepseek,gemini}/adapter.rs` -- the four original adapters' request builders and `model_profile`
- `crates/paladin-llm/src/{pricing,cadence,fallback}.rs` -- the three decorators forwarding `model_profile`; the fallback chain's intersection and per-step models
- `crates/paladin-llm/src/config/{llm,layers,bridge}.rs` and `provider_factory.rs` -- the `llm.<provider>` block, the D-13 layering and `create_from_config`
- `crates/paladin-llm/src/{window,response_guards,turn}.rs` -- the per-model window, the one empty-output and refusal rule, the shared user-turn text
- `src/application/services/paladin/paladin_builder.rs` -- the build-time refusal; `paladin_execution_service.rs` -- the run-metadata merge
- `crates/paladin-llm/src/conformance.rs`, `tests/provider_swap_matrix.rs`, `docs/src/user-guides/switching-providers.md` -- the 17-case suite, the nine-provider matrix and the published behaviour table with its drift test

## Code Conformance

conforms

Measured at the Phase 50 closeout (plan 50-16, 2026-10-16) on the finished tree, against the phase base `865a3968`. Each decision group is proved by named tests or recorded gate results:

- **(a) The policy (D-01).** `provider_swap_matrix` (`tests/provider_swap_matrix.rs`): 50 passed, 0 failed, **0 ignored** (`-- --ignored --list` lists nothing; the 19 cases ignored at plan 50-01 were each un-ignored by the plan that fixed them, assertions untouched, and `WINDOWS.md` row 71 is closed). It runs one agent definition against all nine providers, built from an `llm.<provider>` block alone, and requires equivalent behaviour or a refusal naming provider, model and parameter. The 21 `sampling::` tests prove admission, omission and the no-clamp rule; the `paladin_builder` profile tests (16 temperature, profile and refusal cases) prove the build-time refusal.
- **(b) Explicit versus auto (D-02).** The builder tests above distinguish `PaladinBuilder::temperature` and `add_stop_word` (refused) from the `auto_temperature` heuristic and the fabricated `0.7` default (omitted and recorded); `config.example.yml`'s Anthropic block carries no `default_temperature`, and a pinned test asserts it.
- **(c) The shape (D-03).** `provider_capabilities_gained_no_field` is green; `LlmPort::model_profile` defaults to `None` and the consumer-surface muster compiles an implementor that provides only the required methods; `ModelProfile` is `#[non_exhaustive]` and built through `new()` plus `with_*`.
- **(d) `Fixed`, `NotSent` and the admission rule (D-04a).** The edge-value table in `model_profile.rs` (both endpoints inclusive, exact `Fixed` equality, `NaN` admitted by none), 10 unit tests and 12 doc tests in `paladin-ports`; the suite case `unsupported_parameters_are_omitted_and_recorded` runs on all nine adapters.
- **(e) Refuse at build, filter at the wire.** `sampling::apply` is the single filter, called by the compat engine and the four original adapters; the suite cases `capability_declarations_match_the_wire` and `response_format_reaches_the_wire` run on all nine. **Stated limit, unchanged:** a direct `LlmRequest` caller gets omit, `warn!` and metadata, not a refusal.
- **(f) Precedence (D-13).** 54 `registry::` tests and 31 `config::layers::` tests (per-model override over adapter-wide `context_window` over seeded entry over fallback; the 300 s reasoning default; `${VAR}` placeholders), plus `config::bridge::` (29) and `provider_factory::` (31).
- **(g) Decorators and chains (Pitfall 4).** The three forwarding tests including the production `Pricing(Cadence(provider))` composition, and 47 `fallback::` tests including per-step models, `hop_on_refusal` and the intersection's order independence.
- **(h) Run metadata (owner reading R-1 = option-b).** `PaladinResult.run_metadata`, `ChunkMetadata.run_metadata` and the terminal-chunk `StreamingResponse.metadata` are asserted for all nine adapters; a merge test proves a foreign key is never copied; the fallback stream names the serving hop. The `paladin-web` HTTP body is deliberately not extended.
- **(i) Request-shaping registry fields.** `sampling::apply_response_format` and `check_max_tokens` tests; the suite's `response_format_reaches_the_wire` and a refused `max_tokens` above a seeded maximum output.
- **The 17-case conformance suite on nine adapters.** `conformance_suite` is 153 tests (17 cases, ten transport and seven behaviour, over OpenAI, Anthropic, DeepSeek, Kimi, Qwen, Grok, Gemini, Ollama and OpenAI-compatible), all green, including the credential-redaction and redirect cases (the redirect case answers 307 with a `GET` and a `POST` mock at `expect(0)`, so it fails a client that follows) and `refusal_is_typed_and_permanent`, which plants the adapter's own key in the vendor's refusal text and asserts the detail never contains it.
- **The published page.** `switching_providers_page_lists_every_seeded_model` (and 12 controls) compares every seeded row, the fixed provider order, the nine-provider coverage and the response-mapping table against the registry; it was failed by hand (a deleted row, a changed verified-on date) and restored.
- **Gates.** On the final tree: `cargo test --workspace --features integration-tests,llm-all --no-fail-fast` passes 7,840 tests across 58 test binaries and 6 doctest runs and fails exactly the two ledgered sandbox tests (`run_api_wiring` pair, `WINDOWS.md` row 73, identical on the untouched tree); `make clean-code` (fmt, clippy `-D warnings`, shellcheck, check, the rustdoc zero-warning bar, the `# Examples` gate), `make check-gates`, `make api-surface` (4,298 items, unchanged), `make security` (advisories, bans, licenses, sources ok), `make check-consumer-surface`, `make check-migration-allowlist` and `make check-api-examples` all exit 0. The closeout D-27 (cargo-semver-checks 0.50.0 against the published 0.10.1, allows disabled and restored byte for byte) found no Phase 50 lint without a `MIGRATION.md` 9.2 row and allowlist entry in `paladin-ports`, `paladin-llm`, `paladin-battalion` or `paladin-ai`. The phase diff adds no `unwrap(`, `expect(` or `panic!(` outside `#[cfg(test)]` code, and the manual credential-handling review (redact before truncate, no key in logs, `Debug`, metadata or errors, redirects refused on every adapter including factory-built ones, `${VAR}` placeholders never sent literally) is recorded in the plan 50-16 SUMMARY. The 82 percent workspace line-coverage floor (ADR-0006) is gated by CI's `coverage` job; `cargo-llvm-cov` is not run locally and no percentage is quoted here.

**Stated limits.** (1) A direct `LlmRequest` caller is not refused (decision (e)). (2) Seven of nine providers were probed on 2026-10-15; Ollama and OpenAI-compatible were skipped and have no seeded models. (3) Penalty and `top_p` spans are contract-derived from a single 0.5 probe, the OpenAI and Grok temperature upper bounds are stated rather than measured edges, and the Switching providers page labels each. (4) Usage-cap mapping is unverified for the five compat presets (R-2, owner-accepted 2026-10-16). (5) The OpenAI half is closed by plan 50-19 (owner option (a), 2026-10-17; `WINDOWS.md` row 75 fixed): the OpenAI adapter sends its output limit as `max_completion_tokens` for every model, with no per-model switch, and only `gpt-5.4-nano` and `gpt-4o-mini` were measured accepting that key (2026-10-15), so older OpenAI ids and non-OpenAI servers behind `OPENAI_BASE_URL` are unmeasured (the `openai-compatible` preset keeps `max_tokens`). The Kimi half is closed by plan 50-20 (owner option (a), 2026-10-17; `WINDOWS.md` row 76 fixed): the adapter-wide fallback declares `temperature`, `top_p` and both penalties `NotSent`, equal to the seeded `kimi-k2.6` and `kimi-k3` rows, because the vendor's single legal value depends on the thinking mode ("only 1" on 2026-08-22, "only 0.6" with thinking disabled on 2026-10-15) and no thinking-on temperature probe has run. (6) `TokenUsage.total_tokens` stays `prompt + completion` (D-02, kept by the owner). Plans 50-21 to 50-31 fold every billed category into those two counts, reconcile the vendor total against them (`paladin.usage_unreconciled`), and carry, price and settle billed failures, hopped-past fallback bills and failed or timed-out engine attempts (owner decisions of 2026-10-17, which brought remainders (a) and (b) into Phase 50; plan 50-31's `provider_swap_matrix` case `billed_failures_and_hopped_refusals_reach_the_ledger` proves the chain end to end). `WINDOWS.md` row 74 is fixed. The one open item is remainder (c): Grok's vendor-computed `cost_in_usd_ticks` and `num_sources_used` are not parsed or compared with the Treasurer price. That is the owner's accepted deferral of 2026-10-17, ledgered open as `WINDOWS.md` row 77 with its closing condition (parse both fields on the Grok wire and compare `cost_in_usd_ticks` with the Treasurer price under this ADR's visibility rule, with fixture tests). (7) The Docker-bound suites and the credential-gated `live_vendor_smoke` run are not exercised in this environment; CI's `test-integration-docker` job and the operator's manual smoke run are their proof.

**Update, 2026-10-20.** Phase 50.1 seeded further models from a second operator probe, `.planning/phases/50.1-consumer-surface-readiness/50.1-SEED-PROBE.md` (limit (2) above describes the first, 2026-10-15 probe); the seed rule is unchanged: a value with no evidence in the transcript an entry's `verified_on` date names is not written, and a model is listed only when it was measured end to end. Limit (5) is narrowed by the same probe: besides `gpt-5.4-nano` and `gpt-4o-mini`, the 17 OpenAI ids it seeded (`gpt-6.1-sol`, `gpt-6-sol`, `gpt-6-luna`, `gpt-6-astra`, `gpt-5.6-sol`, `gpt-5.6-luna`, `gpt-5.6-terra`, `gpt-5.5`, `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.2`, `gpt-5.1`, `gpt-5`, `gpt-5-mini`, `gpt-5-nano`, `chat-latest`, `o3`) were measured on 2026-10-20 answering `max_tokens=64` with "Use 'max_completion_tokens' instead" and not refusing `max_completion_tokens=64` (`gpt-5-mini` answered 400 "model output limit was reached", a spent budget rather than a refused key); no other OpenAI id has been measured. One recorded exception to the seed rule: `grok-4.7` is seeded although its `temperature=2.00` probe never produced a response -- it failed at the vendor (`UNREACHABLE: error sending request`) identically in both 2026-10-20 runs, as it does for `grok-4.3`, `grok-4.5` and `grok-4.6`. The owner ruled on 2026-10-20 that a reproducible vendor-side failure at the top endpoint, observed identically in two independent runs, counts as a measured outcome for that one row; the row's temperature range stops at 1.99 so a caller cannot send 2.00, and its maximum output is unset. It applies to `grok-4.7` only and changes no existing row.

**Update, 2026-10-22.** Phase 51 adds effort fields to `ModelProfile` (`effort`, `effort_verified_on`) under this ADR's seed rule, unchanged: their only source is the dated effort-probe transcript `.planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.md` (2026-10-22), and an unprobed model stays unmeasured. See ADR-0062.

## Downstream Consumers

- **Phase 51** -- ADR-0062 extends this policy with stricter defaults for effort levels and budget bounds, on the same `#[non_exhaustive]` `ModelProfile`.
- **The web3sec consumer** -- its Anthropic agents that set `temperature` must drop it, because the current Anthropic models are `NotSent` and an explicit value is now refused.
