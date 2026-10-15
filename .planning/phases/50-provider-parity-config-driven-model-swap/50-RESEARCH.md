# Phase 50: Provider Parity & Configuration-Driven Model Swap - Research

**Researched:** 2026-10-15
**Domain:** Rust hexagonal LLM-adapter layer (`paladin-ports` trait surface, `paladin-llm` nine adapters, `PaladinBuilder` validation, config-driven construction, public-API/semver governance)
**Confidence:** HIGH on codebase facts (every claim grep-verified at HEAD `5a6454d3`); MEDIUM on Anthropic vendor facts (fetched from the official docs); LOW on every non-Anthropic vendor fact (official doc hosts were unreachable from this environment, see Environment Availability)

Verified against HEAD `5a6454d3` (docs-only commits on top of the `83a16bfa` tree the ROADMAP re-checked). Every ROADMAP `path:line` was re-grepped; all hold (table in "Verified Ground Truth").

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried forward (locked by earlier phases — not re-asked)

- **D-00a:** Shipped tree outranks any document. 82 % workspace line-coverage floor (ADR-0006).
  `make clean-code`, `make check-gates`, `make api-surface` (+ `make api-surface-update` and a
  CHANGELOG entry for an intentional public-surface change; the gated baseline is
  `.project/current-exports.txt`), `make security`, `make check-api-examples`,
  `make check-migration-allowlist` and the manual credential-handling review in
  `.github/instructions/security.instructions.md` gate every commit. No `unwrap`/`expect`/`panic!`
  added to library code. Response bodies are redacted **before** truncation when embedded in
  errors (this binds the `Refusal.detail` text below).
- **D-00b:** X-03 governs public API: every break needs a `MIGRATION.md` §9.2 row and, when marked
  `Y`, a `.cargo/semver-checks-allowlist.toml` entry naming one published crate name and one lint,
  kept set-equal by `make check-migration-allowlist`. Lints are measured, not guessed (the Phase 44
  D-27 diagnostic: drop the crate-wide `allow` lines, run `cargo semver-checks check-release
  --package <crate> --baseline-version 0.10.1 --release-type minor`, restore). Every supersession
  of X-03 is its own record (ADR-0051, 0059, 0060 precedent), never inherited. This phase is
  designed to be **additive** on the Rust surface (D-03, D-09, D-11, D-15); if the planner finds a
  break is unavoidable, it is recorded in the same commit as the code.
- **D-00c:** ADRs take the next free number from `.planning/decisions/PROMOTION.md` (currently
  **0061**) and advance that line in the same commit.
- **D-00d:** Vocabulary (Phase 30 D-01, Milestone 13 §5): plain names for new config/descriptor
  structs and units (`ModelProfile`, `FallbackStep`, `create_from_config`), medieval names only
  for role-bearing services. No new officer is minted in this phase; Commissary and Cadence keep
  their roles.
- **D-00e:** Phase 26 D-28: `ProviderCapabilities` is compile-guarded against gaining a field
  (`provider_capabilities_gained_no_field`, `crates/paladin-ports/src/output/llm_port.rs:1977`).
  PARITY-02 requires the parameter model to be additive unless an explicit X-03 supersession is
  recorded; D-03 below keeps it additive.
- **D-00f:** ADR-0046: facade default features are `openai + anthropic + deepseek`; the other six
  providers are opt-in features. ADR-0004: temperature validation is provider-aware at the ports
  boundary with no undiscoverable substitution — this phase extends that principle, it does not
  weaken it.
- **D-00g:** Phase 44 D-00f/D-09: the Phase 25 `Transience` taxonomy
  (`crates/paladin-core/src/platform/container/transience.rs`, table-driven
  `LlmError::transience()`) is the classification source, and
  `crates/paladin-battalion/src/llm_failure.rs` carries the label table. A new `LlmError`
  variant must be classified in both. `FallbackLlmAdapter` hops on `Transient`/`Unknown` and
  short-circuits on `Permanent` (`crates/paladin-llm/src/fallback.rs`).
- **D-00h:** Phase 43 (ADR-0058): Cadence pacing is keyed on `(provider, model)`; a fallback hop's
  own model must feed that key (PARITY-06).
- **D-00i:** Phase 49: `tests/consumer_surface_muster.rs` (required-features `llm-anthropic`,
  `llm-deepseek`; `make check-consumer-surface`) compiles the surface web3sec imports. This phase
  changes that surface; the muster is updated as part of each registered break, never
  silenced. The consumer moves once, straight to `v0.11.0`, after Phases 50 and 51 (folded todo
  below).
- **D-00j:** Phase 51 (Adjustable Reasoning Effort) depends on PARITY-02..06 and its roadmap entry
  already assumes reasoning text is **typed** on `LlmResponse` and the streaming delta (its D-4),
  the per-model registry holds supported effort levels and budget bounds, and the D-1 policy below
  is the one it extends with stricter defaults. Nothing here may pre-empt or contradict those
  assumptions.

#### Parameter-drop policy and support model (PARITY-02, handoff D-1)

- **D-01:** **Policy (handoff option b):** a configured sampling parameter (temperature, top_p,
  presence/frequency penalties, stop sequences) that the target provider/model cannot honour is
  **refused at build time when the operator set it explicitly**, with an error naming the
  provider, the model and the parameter. An **auto-selected** value (the `auto_temperature`
  heuristic's output) is **omitted** for a target that does not take it, made visible by a
  `warn!` and a run-metadata entry — never a `debug!`, never a silent drop. This governs every
  adapter, the three originals included, and fixes Anthropic's current validate-then-discard.
  — **Reversibility:** costly — the rule is recorded in an ADR, enforced in `PaladinBuilder`
  validation and the conformance suite, and Phase 51 builds its stricter effort policy on it.
- **D-02:** **"Explicit" means any operator-written value:** `PaladinBuilder::temperature()` (and
  the other sampling setters), the agent YAML `temperature:` field, and
  `llm.<provider>.default_temperature` are all explicit. Only the `auto_temperature` heuristic's
  result is "auto". A provider block's own default that its provider cannot send is a
  configuration error worth refusing. Rejected: treating `llm.<provider>.default_temperature` as
  auto.
- **D-03:** **Shape: an additive `ModelProfile`** — a plainly named per-model descriptor stating
  each sampling parameter as `Range(lo, hi)` / `Fixed(v)` / `NotSent` — returned by a **new
  `LlmPort` method with a default body**, `fn model_profile(&self, model: &str) ->
  Option<ModelProfile>` returning `None`. Non-breaking; adapters opt in; `ProviderCapabilities` is
  untouched and its guard stays. `PaladinBuilder` consults the profile first and falls back to
  `temperature_range` when the profile is `None`. The type lives in `paladin-ports` beside
  `LlmPort`; seeded data lives in `paladin-llm` (D-05). Rejected: a clean break of
  `ProviderCapabilities` (92 struct-literal sites, X-03 supersession, §9.2 rows).
  — **Reversibility:** costly — a public type on the `LlmPort` trait in `paladin-ai-ports`;
  Phase 51 adds effort levels to the same type.
- **D-04:** **ADR form:** a **new ADR-0061** holds the D-01/D-02 policy and the D-03 shape and is
  recorded as *amending ADR-0004*; ADR-0004 gains a dated `**Amended by ADR-0061**` note in the
  ADR-0006/0046 in-place-note style. PROMOTION.md's next-free line advances to 0062 in the same
  commit. Phase 51's second amendment becomes ADR-0062 the same way. Rejected: amending 0004 in
  place only.
- **D-04a:** `Fixed(v)` semantics: an explicit value equal to `v` is accepted and sent; any other
  explicit value is refused; an auto value is omitted per D-01. `NotSent` never puts the parameter
  on the wire regardless of source. Where a vendor rejects a parameter for some models and not
  others (xAI rejects `stop`), the profile, not the adapter, decides.

#### Per-model profile registry (PARITY-04, handoff D-3)

- **D-05:** **Source of truth (handoff option b):** a **dated static table** of profiles per vendor
  in `paladin-llm`, each entry carrying its own verified-on date, **plus config overrides**. Live
  `/models` discovery (option c) is **deferred** — it adds a network dependency to construction
  and only Moonshot exposes the needed fields today. The registry carries, per model: context
  window, max output tokens, reasoning support, fixed/range sampling values (the `ModelProfile`
  data), JSON-mode (`response_format`) support, and leaves room for Phase 51's effort levels and
  budget bounds. Commissary and `resolve_context_window` read it so `kimi-k3` is rationed against
  1,048,576 tokens.
- **D-06:** **Override shape:** nested under the provider block that PARITY-03 makes live —
  `llm.<provider>.models.<model-id>.{context_window, max_output_tokens, ...}`. The key is
  optional, so every existing YAML file loads unchanged (PROV-03). Rejected: a top-level
  `llm.model_profiles` list.
- **D-07:** **Unknown model** (no seeded entry, no override): **fall back to the adapter-wide
  `ProviderCapabilities` values**, made visible by a `warn!` and a run-metadata entry — the same
  visibility rule as D-01. New vendor models keep working without a Paladin release. Commissary's
  `WindowFallbackPolicy::Strict` still refuses only when the adapter declares no window at all.
  Rejected: refusing the build on an unknown model id.
- **D-08:** **Seeding the dated entries:** every seeded model is **live-probed at seed time** on
  the operator's keys against the vendor's `/models` endpoint (vendor docs where no endpoint
  exists), carries its own verified-on date, and the vendor's current general-purpose model
  becomes that provider's default. Anything that cannot be verified is **left out rather than
  guessed**. The stale `claude-3-5-sonnet-20241022` default (`config/bridge.rs`,
  `config.example.yml`), the claude-3 model list, the `deepseek-chat` default and DeepSeek's
  64,000 window are replaced this way. The handoff's Moonshot facts (2026-10-07/13) are inputs to
  re-verify, not data to copy.

#### Typed refusal and reasoning text (PARITY-05, handoff D-2)

- **D-09:** **Typed refusal:** a new additive `LlmError::Refusal { provider, model, detail }`
  variant on the `#[non_exhaustive]` enum, classified **`Permanent`** in the `Transience` table and
  given a row in the `llm_failure` label table. It is **distinct from `ContentFilter`**: Anthropic
  `stop_reason: "refusal"` and OpenAI's `refusal` message field map to `Refusal`; vendor
  `content_filter` finish reasons stay `ContentFilter`. `detail` carries the vendor's refusal text
  **redacted then truncated** (D-00a). Rejected: mapping refusals onto `ContentFilter`.
  — **Reversibility:** costly — a public error variant consumers will match on; the additive
  variant needs no §9.2 row, but removing it later would.
- **D-10:** **Chain behaviour on refusal:** **stop by default** — `Permanent` short-circuits
  exactly as today, so the typed error reaches the consumer, who decides deliberately whether to
  try another provider (the handoff's stated use). A **plainly named chain setting, default off**,
  on `FallbackLlmAdapter` opts in to hopping on `Refusal`, mirroring Phase 51's
  fail-loud-default / explicit-opt-in pattern. Rejected: always hop (hides the refusal, spends
  budget on every hop); always stop with no opt-in.
- **D-11:** **Reasoning text** surfaces as a **typed `Option<String>` on `LlmResponse`** and a
  **typed field on the streaming delta** — both structs are `#[non_exhaustive]`, so this is
  additive — on the normal and streaming paths of every adapter that receives it (compat presets'
  `reasoning_content`, DeepSeek, Anthropic thinking blocks). This is the shape Phase 51's roadmap
  entry already assumes. Rejected: `metadata["reasoning_content"]`.
- **D-11a:** The rest of PARITY-05 is settled by the requirement text and not re-opened: empty
  text is `EmptyCompletion` on **any** stop reason in every adapter (OpenAI included); compat
  presets map `prompt_tokens_details.cache_write_tokens` via `with_cache_write`; usage-cap
  overrides per compat vendor are **live-verified** under the credential gate before being
  encoded; stop sequences go on the wire wherever the profile says the vendor accepts them.

#### Config-driven construction and fallback hops (PARITY-03, PARITY-06, handoff D-4)

- **D-12:** **Public library API:** `LlmProviderFactory::create_from_config(&LlmConfig, provider,
  model)` beside the env-only `create(name)`, returning `Arc<dyn LlmPort>` and **routed through
  the existing `From<&LlmProviderConfig>` bridges in `crates/paladin-llm/src/config/bridge.rs`**,
  which gain the new fields (default `max_tokens`, context-window override, the D-06 model
  overrides). `agent_host` resolves per-agent `provider` + `model` (already in
  `src/config/agents.rs`) through the same call. The `*Config` types remain the direct-construction
  path web3sec uses and gain the same fields. Rejected: a separate builder type.
- **D-13:** **Precedence:** per-agent setting **>** `llm.<provider>` YAML block **>** legacy
  provider env var (`OPENAI_BASE_URL`, `ANTHROPIC_MAX_TOKENS`, `*_API_KEY`, …) **>** adapter
  compiled default. The most specific operator statement wins; env vars fill gaps (the API key
  especially) and never override an explicit block value. Existing env-only deployments keep
  working unchanged. Reasoning-capable models default to a **300 s** timeout, not 60 s, when no
  block or env value is set. Rejected: env var beats YAML.
- **D-14:** **Facade defaults (handoff D-4): keep `openai + anthropic + deepseek`** per ADR-0046.
  The Kimi, Qwen, Grok, Gemini, Ollama and OpenAI-compatible adapter/config pairs are re-exported
  from `src/lib.rs` behind their existing opt-in features; `make api-surface-update` plus a
  CHANGELOG entry; the consumer muster's `required-features` keep compiling. Rejected: enabling
  all nine by default.
- **D-15:** **Fallback hop model:** a plain **`FallbackStep { port, model }`** with a
  **`FallbackLlmAdapter::from_steps`** constructor. Before each hop, `request.model` is rewritten
  to the step's model, that model feeds the Cadence `(provider, model)` key, and the served model
  is recorded beside `SERVED_BY_METADATA_KEY`. `new(Vec<Arc<dyn LlmPort>>)` stays as the
  single-model chain. Rejected: implicitly rewriting to each hop adapter's configured default
  model.
  — **Reversibility:** reversible — additive constructor and struct; the single-model `new` is
  unchanged.

### Claude's Discretion

- Plan count and ordering, starting from the handoff's six-plan shape (red tests + OpenAI defect
  → parameter modelling → config-driven construction → profile registry → response/error parity
  → conformance/smoke/docs). Plans 2 and 4 are prerequisites for Phase 51.
- Layout of the provider-swap matrix test (mock transports, one agent definition, nine
  providers, the four cases SC-1 names) and where it lives.
- Exact run-metadata key names for the D-01 dropped-parameter and D-07 unknown-model entries
  (follow the `paladin.served_by` key style), `warn!` log targets, and the `ModelProfile` field
  and enum names.
- The name of the D-10 opt-in chain setting and the D-11 field names.
- The per-agent resolution helper that `agent_host` calls, and how `agent_host` reports an agent
  whose provider block is missing (must still name provider and model).
- Conformance-suite extension order (capability truthfulness, `response_format` wire shape, empty
  completion, refusal, usage fields, parameter-drop policy) and how Anthropic joins the shared
  suite.
- Live-smoke extension to all nine providers behind the existing credential gate; which live
  checks (quota-error shapes, `json_object` under thinking, cache-read field names) run where.
- The mdBook "Switching providers" page location (beside
  `docs/src/appendix/provider-expansion.md` and `docs/src/contributing/contributing-providers.md`)
  and the shape of its per-provider/per-model behaviour table.
- Whether DeepSeek's current defaults still resolve (handoff G-11 marks this unverified).

### Deferred Ideas (OUT OF SCOPE)

- **Live `/models` discovery** of per-model limits (handoff D-3 option c) — optional later work once
  the dated table plus overrides is in; adds a network dependency to construction.
- **Native tool/function calling, compat vision, prompt caching, request-id propagation** — out of
  scope here (handoff §9, ADR-0042).
- **Reasoning effort** — Phase 51.

(Folded todo: *web3sec moves once, straight to v0.11.0, after Phases 50 and 51* — every intentional
break or additive surface change goes into `MIGRATION.md` §9.2/§9.8 and the CHANGELOG before the
tag, and `tests/consumer_surface_muster.rs` is kept green or updated as part of the registered
break.)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PARITY-01 | Provider-swap matrix test, nine providers, mock transports, config-only switch; manual + auto temperature on a `(1,1)` provider, user prompt on the wire, per-step model in a chain, empty `stop` completion | Matrix-test layout and the **red-test commit strategy** (Recommended Plan Shape, Plan 1); five request-building code paths each need a wire assertion (Architecture Patterns); gating `required-features = ["llm-all"]` |
| PARITY-02 | Per-provider/per-model parameter support (`Range`/`Fixed`/`NotSent`), additive to `ProviderCapabilities`, one policy amending ADR-0004 | `ModelProfile`/`ParamSupport` sketch; build-time refusal + wire-time filter design (Open Question 3); ADR-0061 mechanics; existing `CompatRequestParameters` is the precedent being generalised |
| PARITY-03 | `llm.<provider>` blocks honoured via public API through `bridge.rs`; per-agent `provider`+`model` in `agent_host`; YAML loads unchanged; 300 s reasoning default; facade exports nine pairs | Config findings: no `${VAR}` expansion, `max_retries`/`max_tokens` absent from most `*Config`s, `*Config` are constructible (semver), four `factory.create` call sites, `validate_config` availability check is env-var based |
| PARITY-04 | Dated per-model registry + overrides read by Commissary and `resolve_context_window` | `effective_capabilities` approach (no `WindowSource` variant), HistoryTrimmer is a second reader, seeding procedure and live-probe checkpoint, vendor model facts |
| PARITY-05 | Empty output on any stop reason, typed `Refusal`, usage cap, cache read/write, reasoning tokens + text, stop sequences, every adapter | Five independent code paths (not one engine); refusal-before-empty ordering; `LlmResponse` is **not** `#[non_exhaustive]` (premise correction); `ContentFilter` is a `FinishReason`, not an `LlmError`; Anthropic streaming drops `stop_reason`; per-vendor wire facts |
| PARITY-06 | Per-hop model, Cadence keyed on it, served model recorded | `FallbackStep` design; `pace_hop` uses `request.model`; `CadenceLlmAdapter` keys on `request.model` so a rewrite feeds it for free; `agent_runtime.model_fallback.providers` config path is names-only (Open Question 7) |
| PARITY-07 | Nine adapters in the conformance suite and live smoke; "Switching providers" page; MIGRATION/CHANGELOG | Conformance: Anthropic is the only adapter missing; macro asserts `CASE_COUNT == 10`; smoke harness is copy-pasted per vendor (refactor to a table); docs page location; surface-change register list |
| PARITY-08 | OpenAI sends `query` (+ `context`) on the execution-service path | Root cause is a per-adapter convention split (only OpenAI reads `context`); shared `user_turn_text` helper; OpenAI has four more defects on the same path (fabricated defaults, non-null `content` deserialisation) |
</phase_requirements>

## Summary

The phase is large but well-bounded, and the CONTEXT decisions hold up against the tree with **four premise corrections the planner must absorb** (Verified Ground Truth, "Premise corrections"). The most consequential: (1) `LlmResponse` is deliberately **not** `#[non_exhaustive]` (MIGRATION §9.2, X-10.3 option (b)), so D-11's "additive" `reasoning` field is a registered break with ~83 in-tree struct-literal sites, not an additive change; (2) every `*Config` type is a plain constructible struct and `paladin-llm` has **no** semver-lint allow table, so D-12's "`*Config` types gain the same fields" is also a registered break in a crate that has never had one; (3) PARITY-05's fixes do not "land once in the compat engine" — there are **five independent request/response code paths** (CompatEngine for Kimi/Qwen/Grok/Ollama/openai-compatible; bespoke OpenAI, Anthropic, DeepSeek, Gemini), each with its own empty-completion guard, usage mapping and error mapping; (4) `ContentFilter` is a `FinishReason` variant, not an `LlmError`, so "distinct from ContentFilter" means the adapter must choose between returning `Ok` with `FinishReason::ContentFilter` and returning `Err(Refusal)`.

The technical heart is a single shared **sampling-policy filter** in `paladin-llm` that all five request builders call. Explicit-vs-auto provenance does not need to travel past `build`: `PaladinBuilder` refuses an explicit unsupported value (it has the model and `model_profile`), and at wire time any value the profile cannot take is by construction non-explicit, so the adapter omits it, emits one `warn!` per adapter instance, and stamps a `paladin.*` key on `LlmResponse.metadata`. This avoids adding a field to `PaladinData` (197 literal sites, including the consumer muster's thirteen-field literal). The existing `CompatRequestParameters` static gate is the in-tree precedent being generalised.

Two operational blockers exist outside code: **no vendor API key is set in this environment** and **the egress proxy blocks every vendor documentation host except Anthropic's and GitHub raw**. D-08 (live-probed seeding) and D-11a (live-verified usage-cap shapes) therefore need an explicit operator-keys checkpoint, and every non-Anthropic vendor fact below is `[ASSUMED]`/third-party until that probe runs. Several third-party sources already contradict the handoff's live Moonshot measurements (reasoning-effort field name; quota error HTTP status), which is itself the strongest argument for the probe.

**Primary recommendation:** Land the phase as seven plans (the handoff's six with fallback-hop work split out of response parity): red matrix + OpenAI fix → `ModelProfile` + sampling filter + ADR-0061 → config-driven construction + facade → registry + live-probe checkpoint → ports surface (`Refusal`, `reasoning`) + shared response helpers + compat/OpenAI/DeepSeek → Anthropic/Gemini parity + `FallbackStep` → conformance/smoke/docs; commit the matrix test with not-yet-fixed cases `#[ignore = "red until plan 50-0N"]` so `cargo test` stays green at every commit, and record the red run as evidence in the plan summary.

## Verified Ground Truth

HEAD `5a6454d3`. All ROADMAP claims re-checked by grep.

| Claim | Status | Evidence |
|---|---|---|
| B-4 OpenAI user turn reads `context` | HOLDS | `crates/paladin-llm/src/openai/adapter.rs:435` `user_prompt.context.clone().unwrap_or_default()`; exec service sets `context: None` at `paladin_execution_service.rs:2786`, `:2801`, `:2975` (ROADMAP said `:2974`; off by one) |
| Only OpenAI reads `context` | NEW | every other adapter reads `query`; no production code sets `UserPrompt.context = Some` (grep) |
| B-1 Kimi `(1.0,1.0)` + never sends temperature | HOLDS | `kimi/adapter.rs:210`; `request_parameters.temperature: false`; Kimi window `131_072` at `:196` |
| B-1 Anthropic range `(0,1)`, never sends | HOLDS | `anthropic/adapter.rs:711`; `temperature: None` at `:259`, `:303` |
| Builder validates manual and auto against range | HOLDS | `paladin_builder.rs:1137-1153` (manual), `:1323-1344` (auto) |
| B-2 factory env-only, takes no model | HOLDS | `provider_factory.rs:356`; `agent_host.rs:111` `default_provider_name`; **four** `factory.create` call sites: `agent_host.rs:210`, `facade_provisioner.rs:348` (run-engine LLM), `agent_runtime.rs:1247` (`ModelFallbackConfig::resolve_chain`), plus the provisioner via `build_agent` |
| bridges have no production caller | HOLDS | only `src/infrastructure/adapters/llm/config_bridge.rs` (tests) |
| B-3 fallback resends `request.clone()` | HOLDS | `fallback.rs:425` (generate), `:467` (stream); `pace_hop` is passed `&request.model` at `:436`, `:492` |
| B-5 facade exports 3 of 9 | HOLDS | `src/lib.rs:178-190` |
| G-2 empty guard only on `Length` | HOLDS | `compat/engine.rs:651`; DeepSeek has its **own copy**; Gemini mirrors by hand (`gemini/adapter.rs:~500`) |
| G-1/G-11 stale defaults | HOLDS | Anthropic claude-3 list `anthropic/adapter.rs:669-687`, default `:119`; DeepSeek `deepseek-chat` `:90`, window `64000` `:928`; `bridge.rs:96` |
| G-7 no `Refusal` | HOLDS | Anthropic unknown `stop_reason` -> `FinishReason::Error("unknown")` `anthropic/adapter.rs:~335` |
| H-1 Anthropic not in conformance suite | HOLDS | `llm_conformance_suite!` instantiated for openai, deepseek, kimi, qwen, grok, gemini, ollama, openai_compatible (+ a trivial fixture); not Anthropic |
| H-2 live smoke | HOLDS | `crates/paladin-llm/examples/live_vendor_smoke.rs` covers kimi/qwen/grok/gemini only |
| `ProviderCapabilities` guard | HOLDS | `llm_port.rs:1977` |
| Next ADR number | HOLDS | `PROMOTION.md:82` "Next free ADR number: 0061" |

### Premise corrections (planner must absorb)

1. **`LlmResponse` is NOT `#[non_exhaustive]`.** CONTEXT D-11 says both reasoning-text carriers are `#[non_exhaustive]`. `StreamingResponse` is (`llm_port.rs:1183`, with `delta()/terminal()/with_usage()` constructors); `LlmResponse` (`llm_port.rs:~981`) is not, by an explicit recorded choice (`MIGRATION.md:223`: "deliberately **not** marked `#[non_exhaustive]` (X-10.3 option (b))"). Phase 38 already broke it once for `cost`. A new `reasoning: Option<String>` field is another `constructible_struct_adds_field` break across ~83 `LlmResponse {` occurrences in ~40 files (adapters, mocks, services, tests, benches, examples, `tests/consumer_surface_muster.rs`). `paladin-ports/Cargo.toml` already allows that lint crate-wide and the allowlist already has a `paladin-ports | LlmResponse` entry, so **extending that existing §9.2 row** (rather than adding a new one) keeps `check-migration-allowlist` set-equal with no allowlist edit. MIGRATION `DR-6` and its "insulating the next bump" paragraph tell the consumer to route mocks through one helper — this is the "next bump" it warned about.
2. **`*Config` types are constructible and `paladin-llm` has no semver-lint table.** `KimiConfig { api_key, base_url, model, timeout_seconds }` has no `max_retries` or `max_tokens`; `AnthropicConfig` has `max_tokens` but no `max_retries`; `DeepSeekConfig` neither. Delivering "model, base_url, timeout, max_retries, default max_tokens, context-window override" (PARITY-03) means adding fields to publicly constructible structs. `crates/paladin-llm/Cargo.toml` has **no** `[package.metadata.cargo-semver-checks.lints]` table (paladin-core, -ports, -battalion, -web do). The muster pins `AnthropicConfig::new(4 args)` (`tests/consumer_surface_muster.rs:319`), so keep every `new()` signature; a literal-constructing consumer breaks. Follow D-00b: run the D-27 diagnostic, add a lints table to `paladin-llm`, a §9.2 row and an allowlist entry.
3. **Five code paths, not one engine.** CONTEXT's "most PARITY-05 fixes land once here (compat engine)" holds for five presets only. OpenAI, Anthropic, DeepSeek and Gemini are bespoke and each independently implements request building, empty-completion, finish-reason mapping, usage mapping and streaming. Budget the work as five, and extract shared helpers (see Don't Hand-Roll).
4. **`ContentFilter` is a `FinishReason`, not an `LlmError`** (`llm_port.rs:1081-1087`; there is no `LlmError::ContentFilter`). D-09's "vendor `content_filter` finish reasons stay `ContentFilter`" therefore means they stay `Ok(LlmResponse{finish_reason: ContentFilter})`. The Gemini adapter has an existing test that **pins** a `SAFETY` finish with no text as *not* `EmptyCompletion` (`gemini/adapter.rs:~1910`) — D-11a's "any stop reason" must carve out `ContentFilter`/refusal (Open Question 4).

## Architectural Responsibility Map

Hexagonal tiers (this is a Rust library workspace, not a web stack).

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `ModelProfile`, `ParamSupport`, `LlmPort::model_profile`, `LlmError::Refusal` | Ports (`paladin-ports`) | — | Port vocabulary; D-03 places the type beside `LlmPort`; ports depend on core only |
| Explicit-value refusal at build | Application (`PaladinBuilder`, root crate) | Ports (profile lookup) | ADR-0004 validates "at the ports boundary"; builder has `data.model` and the `Arc<dyn LlmPort>` |
| Wire-time sampling filter, stop sequences, user-turn text, empty/refusal/usage mapping | Infrastructure adapters (`paladin-llm`) | — | Only the adapter knows its wire shape; shared helper module avoids five copies |
| Seeded per-model registry + config overrides | Infrastructure (`paladin-llm`) | Config types (`config/llm.rs`) | D-05; data is vendor-specific and dated |
| `create_from_config`, env/YAML layering, 300 s default | Infrastructure (`provider_factory`, `config/bridge.rs`) | Composition (`agent_host`, `facade_provisioner`, `agent_runtime`) | D-12; composition roots call it, never re-implement |
| Window resolution from profile | Infrastructure (`window.rs`, `Commissary`) | Application (`HistoryTrimmer`, root crate) | Two readers today |
| Fallback hop model, Cadence key, served model | Infrastructure (`fallback.rs`, `cadence.rs`) | Config (`ModelFallbackConfig`) | `CadenceLlmAdapter` keys on `request.model`; chain owns the rewrite |
| `Transience`/label rows for `Refusal` | Core (`transience.rs` types) + Ports (`transience()` table) | Battalion (`llm_failure.rs`, `llm_decision.rs`) | Two label tables, see Pitfall 9 |
| Matrix test, conformance, live smoke, docs page | Test/Docs | — | Root `tests/` (needs `PaladinBuilder` + all nine adapters) |

## Standard Stack

### Core

No new external crates are required. The phase is internal Rust over the existing workspace.

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `thiserror` | workspace | `LlmError::Refusal` variant | Already the project error pattern [VERIFIED: `crates/paladin-ports/src/output/llm_port.rs`] |
| `serde` / `serde_json` | workspace | `ModelProfile` overrides, config structs | Existing |
| `mockito` | 1.7.0 | per-adapter mock transports (conformance, adapter unit tests) | Already a dev-dependency in root and `paladin-llm` [VERIFIED: `Cargo.toml:248`, `crates/paladin-llm/Cargo.toml:59`] |
| `wiremock` | 0.6 | alternative mock transport at the root | [VERIFIED: `Cargo.toml:258`] |
| `log` | workspace | `warn!` visibility (D-01/D-07) | Existing; `tracing` is not used in these crates |
| `cargo-semver-checks` | 0.50.0 | D-27 lint measurement | Installed locally [VERIFIED: `cargo-semver-checks --version`] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `reqwest` (optional dep) | workspace | adapters | unchanged; redirects already disabled everywhere |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| A shared `sampling.rs` filter in `paladin-llm` | Extend `CompatRequestParameters` only | `CompatRequestParameters` covers 5 presets only and is a static `bool` per preset, not per model; OpenAI/Anthropic/DeepSeek/Gemini would stay on divergent logic |
| Extending the existing `LlmResponse` §9.2 row | A new row | New row is also valid; extending the Phase 38 row avoids an allowlist edit but must say "second field" explicitly |
| `effective_capabilities(port, model)` helper for windows | New `WindowSource::ModelProfile` variant | `WindowSource` is a non-`non_exhaustive` public enum with `ALL: [WindowSource; 4]` and substring-pinned labels (`window.rs:~71`); a new variant is a break for exhaustive matchers |

**Installation:** none.

**Version verification:** not applicable (no external package added). Toolchain: Rust `1.97.1` (`rust-toolchain.toml`), `rust-version = "1.88"`, edition 2024 [VERIFIED: local `rustc --version`].

## Package Legitimacy Audit

Not triggered: this phase installs no external packages. If the planner adds one (none is recommended), run `gsd-tools query package-legitimacy check --ecosystem crates <pkg>` first and add the table.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none) | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
 operator YAML (llm.<provider>{,.models.<id>})  +  env vars  +  per-agent {provider, model, temperature}
                         |                              |
                         v                              v
              LlmProviderFactory::create_from_config(&LlmConfig, provider, model)   [PARITY-03]
                 precedence: agent > YAML block > env > compiled default (D-13)
                 api_key: YAML value, else `${VAR}` placeholder -> env, else provider env var
                         |  routes through config/bridge.rs (From<&LlmProviderConfig>)
                         v
                 <Vendor>Config  --(new)-->  <Vendor>Adapter  --holds-->  ModelRegistry (seeded table + YAML overrides)
                         |                                                   |
                         |                          LlmPort::model_profile(model) -> Option<ModelProfile>   [D-03]
                         v                                                   |
   PaladinBuilder::build  --- explicit temperature/top_p/... ---> profile says NotSent / Fixed!=v / out of Range
        |                                                          => ConfigurationError(provider, model, parameter)
        |  auto_temperature result / framework default                [refuse explicit only, D-01]
        v
   PaladinData (unchanged shape) -> PaladinExecutionService -> PromptParameters{temperature: Some(t), stop_sequences,...}
                                                                      |
                                                           LlmRequest{model, prompt, ...}
                                                                      |
     Pricing( Cadence( provider ) ) or FallbackLlmAdapter::from_steps([FallbackStep{port, model}])   [PARITY-06]
        hop i: request.model := step_i.model  ->  Cadence key (provider_i, model_i)  ->  adapter_i.generate
                                                                      |
                     adapter wire builder:  sampling::apply(profile_i, params)  ->  {sent params, omissions}
                          omissions => one warn! per adapter + LlmResponse.metadata["paladin.<key>"]   [D-01/D-07]
                          user turn := query (+ "\n\n" + context)  [shared helper, PARITY-08]
                                                                      |
                                                          vendor HTTP (redirects refused, key redacted)
                                                                      |
   response mapping (shared helpers):  refusal? -> Err(Refusal)  -> content_filter? -> Ok(ContentFilter)
        -> empty text on any other stop -> Err(EmptyCompletion) -> usage{cache_read, cache_write, reasoning}
        -> reasoning text -> LlmResponse.reasoning / StreamingResponse.reasoning
                                                                      |
   Transience: Refusal=Permanent (fallback stops unless opt-in chain flag, D-10); EmptyCompletion=Permanent
```

### Recommended Project Structure

```
crates/paladin-ports/src/output/
├── llm_port.rs            # + LlmError::Refusal, LlmPort::model_profile (default None), LlmResponse.reasoning, StreamingResponse.reasoning
└── model_profile.rs       # NEW: ModelProfile, ParamSupport, StopSupport (pub, #[non_exhaustive], serde)
crates/paladin-llm/src/
├── sampling.rs            # NEW: shared wire-time filter (profile + params -> sent + omissions)
├── turn.rs (or prompt.rs) # NEW: user_turn_text(query, context) shared by all nine adapters
├── response_guards.rs     # NEW: classify_completion(content, finish, refusal, reasoning) shared by 5 paths
├── registry/              # NEW: mod.rs + one dated table per vendor (anthropic.rs, openai.rs, ...)
├── window.rs              # + effective_capabilities(port, model)
├── provider_factory.rs    # + create_from_config; ProviderRegistration gains construct_from_config
├── config/{llm.rs,bridge.rs}  # LlmProviderConfig += max_tokens, context_window, models; manual redacting Debug
├── fallback.rs            # + FallbackStep, from_steps, refusal-hop flag, model rewrite, served-model key
└── conformance.rs         # + Wire::Anthropic, new cases, CASE_COUNT 10 -> N
tests/provider_swap_matrix.rs       # NEW, [[test]] required-features = ["llm-all"]
docs/src/user-guides/switching-providers.md   # NEW (see Open Question 10), linked from SUMMARY.md
.planning/decisions/0061-*.md       # NEW ADR; ADR-0004 gets an "Amended by ADR-0061" note; PROMOTION.md -> 0062
```

### Pattern 1: Additive port type with a defaulted trait method

**What:** `ModelProfile` is a `#[non_exhaustive]` serde struct in `paladin-ports`; `LlmPort::model_profile(&self, model: &str) -> Option<ModelProfile>` has a default body returning `None` (the Phase 26 `response_format` precedent). Decorators (`PricingLlmAdapter`, `CadenceLlmAdapter`, `FallbackLlmAdapter`) **must override and delegate**, or the default `None` silently disables the policy behind them.
**When to use:** all new per-model knowledge; Phase 51 adds effort levels to the same type, so construct it only through `ModelProfile::new()` + `with_*` so a later field is free.
**Example (design sketch, not copied from docs):**
```rust
// crates/paladin-ports/src/output/model_profile.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamSupport { Range(f32, f32), Fixed(f32), NotSent }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ModelProfile {
    pub temperature: ParamSupport,
    pub top_p: ParamSupport,
    pub frequency_penalty: ParamSupport,
    pub presence_penalty: ParamSupport,
    pub stop_sequences: StopSupport,        // NotSent | Supported { max: Option<u32> }
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub reasoning: bool,                    // default-timeout selector (D-13)
    pub json_object: bool,
    pub json_schema: bool,
    pub verified_on: Option<String>,        // "2026-10-15", dated entry (D-05/D-08)
}
```

### Pattern 2: Refuse explicit at build, filter at the wire (provenance without a new `PaladinData` field)

**What:** `PaladinBuilder::validate`/`build` looks up `llm_port.model_profile(&self.data.model)` (falling back to `temperature_range` when `None`):
- `manual_temperature_override` and the value is unacceptable -> `PaladinError::ConfigurationError` naming provider, model, parameter.
- auto-selected value unacceptable -> `log::warn!`, do not refuse (replaces `paladin_builder.rs:1323-1344`'s refusal).

At wire time the adapter's builder calls the shared filter: any value the profile cannot take is omitted, one `warn!` per adapter instance (an `AtomicBool`/`Once` per instance, because `PromptParameters::default()` carries `top_p: 1.0` and `0.0` penalties and `PaladinData::default()` carries `0.7`), and `LlmResponse.metadata["paladin.<omitted-params-key>"]` is stamped on **every** response.
**Why it works:** an explicit unsupported value never reaches the wire (refused at build), so anything the filter must drop is auto or framework-default. `Fixed(v)`: explicit `== v` passes; the build refuses `!= v`; at wire a non-`v` value is dropped.
**Limit:** a caller that bypasses `PaladinBuilder` and builds an `LlmRequest` directly gets filter-plus-warn, not refusal (there is no build step). Document this in ADR-0061.
**Why not a `PaladinData` field:** 197 `PaladinData {` literals, and `tests/consumer_surface_muster.rs:193` constructs it with all thirteen fields — a new field breaks the consumer (Pitfall 3).

### Pattern 3: Layered config with injected environment

**What:** extend the existing pure-function pattern (`KimiConfig::from_parts(api_key, base_url, model, timeout)`, `kimi/adapter.rs:~108`) so each adapter's config has a `from_layers(block: Option<&LlmProviderConfig>, per_agent_model: Option<&str>, env: &dyn Fn(&str) -> Option<String>)`. Tests pass a map-backed closure. This is required: `std::env::set_var` is `unsafe` under edition 2024 and `paladin-llm` denies `unsafe_code`. `create_from_config` calls it; `From<&LlmProviderConfig>` remains the YAML-only conversion the bridges already provide.

### Pattern 4: Per-hop model rewrite

**What:** `FallbackLlmAdapter` keeps `Vec<Arc<dyn LlmPort>>` for `new` and stores `Vec<Option<String>>` (hop model override) internally; `from_steps(Vec<FallbackStep>)` fills it. Before `provider.generate(request.clone())` (`fallback.rs:425`, `:467`) set `request.model` from the step; pass the **step model** (not `request.model` of the original) to `pace_hop` (`:436`, `:492`). `CadenceLlmAdapter` already keys on `request.model` (`cadence.rs:399`, `:420`), so wrapped hops get the right key automatically once the rewrite happens. Record the served model beside `SERVED_BY_METADATA_KEY` (a second constant, same style).

### Anti-Patterns to Avoid

- **Provenance on `PaladinData`/`PaladinConfig`:** breaks every literal (Pitfall 3). Use build-time refusal + wire-time filter.
- **Adding `WindowSource::ModelProfile`:** public non-`non_exhaustive` enum; use `effective_capabilities` instead.
- **Fabricating wire defaults.** The OpenAI adapter sends `temperature: Some(unwrap_or(0.7))`, `top_p: Some(1.0)`, `max_tokens: Some(unwrap_or(4096))` unconditionally (`openai/adapter.rs:~760-790`). That contradicts `NotSent` and ADR-0004's "no undiscoverable substitution". Route through the filter; make `max_tokens` a config knob (Open Question 5).
- **A new officer name.** D-00d: plain names only (`ModelProfile`, `FallbackStep`, `create_from_config`).
- **Testing log output with a second `log::set_logger`.** It can be installed once per process (`paladin-server.rs:745` does it in a bin test). Assert primarily on the metadata key; if a log assertion is needed use one shared `OnceLock` capturing logger per test binary.

## Recommended Plan Shape

Starting point: the handoff's six plans, with fallback hops split out (response parity is the largest plan: 5 code paths x {empty, refusal, usage, cache, reasoning, stop}).

| Plan | Scope | Notes |
|---|---|---|
| 50-01 | `tests/provider_swap_matrix.rs` + OpenAI B-4 fix + shared `user_turn_text` | **Red-test strategy:** `cargo test` must pass at every commit (CLAUDE.md). Commit the matrix with each not-yet-fixed case `#[ignore = "red until plan 50-0N"]`, run once with `--ignored` before fixing and paste the failing output into the SUMMARY as the SC-1 "was red" evidence; un-ignore each case in the plan that fixes it. The PARITY-08 mock-transport regression test is **not** ignored: write it, run it red, fix, commit green. |
| 50-02 | `ModelProfile`/`ParamSupport`/`LlmPort::model_profile`; `sampling.rs`; `PaladinBuilder` refusal/warn; minimal seeded profiles for the already-known cases (Kimi `Fixed`, Anthropic Opus 4.7+ `NotSent`); ADR-0061 + ADR-0004 note + PROMOTION.md; decorators delegate | Prerequisite for Phase 51. Wire the filter into the five request builders here (it replaces `CompatRequestParameters` use and the OpenAI fabricated defaults). |
| 50-03 | `create_from_config`, `from_layers` per adapter, new `*Config` fields (+ D-27 diagnostic, lints table, §9.2 row, allowlist entry), `LlmProviderConfig` fields + redacting `Debug`, `agent_host`/`facade_provisioner`/`agent_runtime` call sites, availability check, facade re-exports of six pairs, `config.example.yml`, `make api-surface-update`, CHANGELOG | Resolve Open Questions 2, 6, 8 first. |
| 50-04 | Registry tables + overrides, `effective_capabilities`, Commissary + HistoryTrimmer readers, 300 s reasoning default, stale default replacement. **Checkpoint:** operator runs the seed probe with real keys. | Prerequisite for Phase 51. Human-action checkpoint; do not seed from this document's vendor facts. |
| 50-05 | Ports surface: `LlmError::Refusal` (+ `transience()` arm and test row, `typed_origin`, `llm_error_class`, `classify_fetch_failure`, optional `paladin-eval` `LlmErrorKind`), `LlmResponse.reasoning` (registered break, ~83 sites), `StreamingResponse.reasoning`; `response_guards.rs`; compat engine + OpenAI + DeepSeek mapping (empty any-stop, cache-write, reasoning text, stop on wire, usage-cap overrides) | Split the `LlmResponse` literal migration into its own task/commit so the diff is reviewable. |
| 50-06 | Anthropic (refusal-before-empty, streaming `stop_reason`/`stop_details`, `stop_sequences`, thinking text, model list) + Gemini (`SAFETY` precedence, stop, refusal) + `FallbackStep`/`from_steps`/refusal-hop flag/served-model key + `ModelFallbackConfig` steps (Open Question 7) | |
| 50-07 | Conformance extensions + Anthropic fixture, table-driven live smoke for nine providers, "Switching providers" page, MIGRATION §9.1/§9.2/§9.5/§9.8 + CHANGELOG, muster update | `llm_conformance_suite!` asserts `CASE_COUNT == 10` (`conformance.rs:~827`); update that number and its doc. |

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Redact-then-truncate vendor text in `Refusal.detail` | A new truncation helper | `crate::redaction::redact_and_bound(body, &api_key)` (`redaction.rs`) | The ordering is load-bearing: bounding first can slice a secret across the boundary (`security.instructions.md`) |
| HTTP status -> `LlmError` | Per-adapter `match status` | `crate::http_status::map_http_status_with_hints` | Already shared; vendor quirks go through `CompatEngineConfig::error_override` (`fn(u16,&str)->Option<LlmError>`, receives the redacted/bounded excerpt) |
| Sampling-parameter dropping | Five hand-written `if let` blocks | one `sampling::apply` | The existing five (`CompatRequestParameters` + OpenAI + Anthropic `None`s + DeepSeek + Gemini) already diverged (Kimi/Anthropic discard silently; OpenAI fabricates) |
| User-turn text | Per-adapter `query`/`context` logic | one `user_turn_text(&UserPrompt)` | The B-4 defect *is* the per-adapter split |
| Empty/refusal classification | Per-adapter copies of `detect_empty_completion` | one `classify_completion` | DeepSeek, compat, Gemini each carry a copy that "must stay in step by hand" (`gemini/adapter.rs:~495`) |
| Env-dependent config tests | `std::env::set_var` | injected `env` closure (Pattern 3) | `unsafe` in edition 2024; crate denies `unsafe_code` |
| Chain-wide pacing | A new sleep in `FallbackLlmAdapter` | existing `pace_hop` + `CadenceLlmAdapter` | PACE-02 owns waiting |
| Ctx-window precedence | A new resolver | `resolve_context_window` fed by `effective_capabilities` | PRIM-04 shared walk (`window.rs`) |
| `${VAR}` placeholder resolution | A general templating engine | a ~10-line `resolve_secret_placeholder` (exact `^\$\{[A-Z0-9_]+\}$` -> env) | The loader performs **no** expansion (`treasurer.rs` tests assert the literal placeholder arrives); keep it exact so a key that merely contains `$` is not rewritten |

**Key insight:** the phase's defects are all "five implementations drifted". The fix that holds is fewer implementations (four small shared helpers), not five parallel patches plus a longer conformance suite.

## Common Pitfalls

### Pitfall 1: Anthropic refusal is swallowed as `EmptyCompletion`
**What goes wrong:** `parse_response` calls `detect_no_text_content` *first* (`anthropic/adapter.rs:~322`). A refusal is HTTP 200 with `"content": []` and `stop_reason: "refusal"` [CITED: platform.claude.com/docs/en/build-with-claude/refusals-and-fallback, fetched 2026-10-15], so it returns `EmptyCompletion` ("retry with a larger max_tokens"), the exact wrong advice.
**How to avoid:** check `stop_reason == "refusal"` before the empty check; carry `stop_details.category` and `explanation` (redacted then bounded) into `Refusal.detail`. `category`/`explanation` may be `null`.
**Warning signs:** a refusal test whose `content` is empty passing as `EmptyCompletion`.

### Pitfall 2: Anthropic streaming never sees the stop reason
**What goes wrong:** the stream loop emits `FinishReason::Stop` on `message_stop` unconditionally and ignores `message_delta.delta.stop_reason`; thinking deltas are dropped; a refusal "can arrive mid-stream after partial output" [CITED: same page].
**How to avoid:** hold `stop_reason`/`stop_details` from `message_delta` (like `held_usage`), map `refusal` to `Err(Refusal)` on the terminal item, and discard partial output per the vendor's guidance. Also note `serde_json::from_str(..) else continue` hides malformed events.

### Pitfall 3: Adding a field to a struct the consumer literal-constructs
**What goes wrong:** `PaladinData` (13 fields, full literal in the muster), `LlmResponse` (no `Default`, no constructor), `*Config`, `LlmProviderConfig` all break consumer literals when extended.
**How to avoid:** prefer designs that add no field (Pattern 2); where a field is unavoidable follow D-00b and update `tests/consumer_surface_muster.rs` in the same commit; consider adding `LlmResponse::new(..)`/`Default`-free constructor helpers so the *next* field is free (MIGRATION's own advice).

### Pitfall 4: A defaulted trait method silently disabled by decorators
**What goes wrong:** `model_profile` defaults to `None`; `PricingLlmAdapter` (`pricing.rs:154`), `CadenceLlmAdapter` (`cadence.rs:392`) and `FallbackLlmAdapter` (`fallback.rs:414`) wrap every agent the server builds (`Pricing(Cadence(provider))`). If they do not forward, the builder falls back to `temperature_range` and the whole policy is bypassed in production while unit tests on bare adapters pass.
**How to avoid:** override in all three; add a conformance/decorator test "decorated port returns the inner profile". A chain returns the **intersection** (strictest) over its steps' profiles so the build refuses what any hop cannot honour (Open Question 7).

### Pitfall 5: "Empty on any stop reason" collides with three existing behaviours
**What goes wrong:** (a) `detect_empty_completion_empty_but_stop_is_not_truncation` (`compat/engine.rs:1242`) and the DeepSeek/Gemini equivalents pin the opposite; (b) Gemini's `SAFETY` test pins non-`EmptyCompletion`; (c) `FinishReason::FunctionCall` legitimately carries empty text; (d) `LlmError::EmptyCompletion`'s `#[error]` text always says "retry with a larger max_tokens budget (reasoning likely consumed the whole budget)" — false for an empty `stop`.
**How to avoid:** precedence **refusal -> content_filter/SAFETY (Ok, unchanged) -> function_call (exempt) -> any other empty = EmptyCompletion**; flip the three pinned tests deliberately and record the behaviour change in MIGRATION §9.1; make the message branch on the finish reason (the `String` payload already varies, but the Display suffix is fixed in the attribute — change it to a neutral suffix or drop the suffix and put advice in the payload). Changing a `Display` is itself a §9.1 note (DR-style "Display unchanged" precedent).

### Pitfall 6: OpenAI response cannot represent a refusal today
**What goes wrong:** `OpenAIMessage { role, content: String }` is used for both request and response (`openai/adapter.rs:233`). OpenAI's `message.content` and `message.refusal` are `string|null` [CITED: github.com/openai/openai-openapi `ChatCompletionResponseMessage`, spec v2.3.0, fetched 2026-10-15]; a refusal (`content: null`) fails deserialisation and surfaces as `ProcessingError("Failed to parse response")`. `finish_reason` also includes `tool_calls`, which `convert_finish_reason` maps to `Error("Unknown: tool_calls")`.
**How to avoid:** separate response-message type with `content: Option<String>`, `refusal: Option<String>`; map `refusal.is_some()` to `Err(Refusal)`; stream path too (`delta.refusal`).

### Pitfall 7: Shipped YAML turns "explicit" and refuses itself
**What goes wrong:** `config.example.yml` sets `default_temperature: 0.7` in the openai, anthropic and deepseek blocks. D-02 makes `llm.<provider>.default_temperature` explicit; once D-08 replaces the Anthropic default with a current model whose profile is `NotSent`, an operator copying the example gets a build refusal at startup. Likewise today those fields are parsed and ignored, so "existing YAML loads unchanged" (PROV-03) is true for *parsing* but not for *behaviour*.
**How to avoid:** drop `default_temperature` from the example's Anthropic block (and any block whose seeded default model is `NotSent`/`Fixed`), add a MIGRATION §9.5 row stating that a block-level `default_temperature` is now applied and validated, and make the refusal message name the YAML key (`llm.anthropic.default_temperature`).

### Pitfall 8: `${VAR}` placeholders arrive literally and the provider check is env-based
**What goes wrong:** the YAML loader does not expand `${OPENAI_API_KEY}` (tests in `src/config/treasurer.rs:~1492` assert the literal arrives). A naive `create_from_config` that honours `api_key` would send the literal `"${OPENAI_API_KEY}"` as the credential. Separately `validate_config` (`agent_host.rs:~312`) decides availability with `LlmProviderFactory::list_available_providers()`, which tests only the provider env var, so a provider configured purely in YAML is reported unavailable; and `LlmConfig::validate` requires the `default_provider` block to be present.
**How to avoid:** treat a missing/empty/exact-placeholder `api_key` as "fall through to env" (Open Question 6); make availability = block with a usable key OR env var; update both checks and test the YAML-only path.

### Pitfall 9: Two label tables, one `transience()` match with no wildcard
**What goes wrong:** `LlmError::transience()` (`llm_port.rs:~625`) is exhaustive inside `paladin-ports` (no wildcard): the new variant is a compile error there — good. The out-of-crate tables silently fall to wildcards: `typed_origin` (`llm_failure.rs:~110`, so `Refusal` would lose its typed `provider`), `llm_error_class` (`llm_decision.rs:~302`, "unknown error"), `classify_fetch_failure` (`compat/engine.rs:~268`). CONTEXT says "a row in the `llm_failure` label table"; the real label table is `llm_decision.rs::llm_error_class`, and `llm_failure.rs` carries `typed_origin`.
**How to avoid:** add an arm to all four plus a `llm_error_transience_table` row (`llm_port.rs:~2083`), and `paladin-eval`'s `LlmErrorKind` if scripted refusals are wanted.

### Pitfall 10: Window readers other than Commissary
**What goes wrong:** `HistoryTrimmer::resolve_limit` (`middleware/history.rs:~96`) also resolves through `get_capabilities()`. Feeding only Commissary leaves history trimming at the adapter-wide value. `Commissary::new` resolves **once at construction** from `config.model_hint`; a per-agent model change needs a new Commissary.
**How to avoid:** `effective_capabilities(&dyn LlmPort, model) -> ProviderCapabilities` (overrides `max_context_tokens` from the profile) used by both `Commissary::from_port` and `HistoryTrimmer`; `Commissary::new` keeps its signature (the muster pins it).

### Pitfall 11: `max_retries` / `max_tokens` knobs that no adapter reads
**What goes wrong:** `CompatEngineConfig.max_retries` is hard-coded `3` in every preset (`kimi/adapter.rs:~189`); DeepSeek/Anthropic have no retry setting at all; compat has no default `max_tokens` (the exec service sends `None`). Plumbing the YAML value into `KimiConfig` is not enough.
**How to avoid:** add the field to each `*Config`, thread it into the engine config, and add `default_max_tokens` application inside the engine/adapter when `params.max_tokens` is `None` (and use `max_completion_tokens` where a profile says so, see Vendor facts).

### Pitfall 12: Live facts conflict, so seeding from documents will be wrong
**What goes wrong:** see State of the Art (Moonshot effort field, quota HTTP status, DeepSeek retired IDs).
**How to avoid:** the D-08 probe is mandatory, dated, and its output committed as a transcript like Phase 17-19's; entries that cannot be probed are omitted (unknown-model fallback, D-07).

### Pitfall 13: `agent.timeout_seconds` is not an HTTP timeout
**What goes wrong:** `AgentDefinition.timeout_seconds` (`src/config/agents.rs:~346`) is the per-agent *execution* timeout stored in the registry (`register_built`), not the provider client timeout. D-13's "per-agent setting > block" applies to `provider`, `model`, `temperature` only.
**How to avoid:** do not map it onto the client timeout.

## Code Examples

Design sketches grounded in in-tree patterns (no vendor source; cited paths are the patterns they extend).

### Shared user turn (PARITY-08)
```rust
// Source pattern: crates/paladin-llm/src/openai/adapter.rs:432-436 (the defect) and every other adapter's `query` read.
pub(crate) fn user_turn_text(user: &UserPrompt) -> String {
    match user.context.as_deref().filter(|c| !c.is_empty()) {
        Some(ctx) => format!("{}\n\n{}", user.query, ctx),
        None => user.query.clone(),
    }
}
```

### Wire-time filter contract
```rust
// crates/paladin-llm/src/sampling.rs  (all five request builders call this)
pub(crate) struct Applied { pub temperature: Option<f32>, pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>, pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>, pub omitted: Vec<&'static str> }
pub(crate) fn apply(profile: Option<&ModelProfile>, caps: &ProviderCapabilities,
                    p: &PromptParameters) -> Applied { /* Range: in-range sends; Fixed(v): ==v sends;
                    NotSent / mismatch: omit + record the parameter name; stop > max truncates? NO: omit + record */ }
```
Do **not** truncate stop sequences silently (OpenAI allows 4 [CITED: openai-openapi `StopConfiguration`, `maxItems: 4`]); record the omission.

### Refusal-before-empty (Anthropic)
```rust
// Source pattern: anthropic/adapter.rs parse_response; ordering is the point.
if response.stop_reason.as_deref() == Some("refusal") {
    return Err(LlmError::Refusal { provider: ANTHROPIC_PROVIDER.into(), model: response.model.clone(),
        detail: crate::redaction::redact_and_bound(&refusal_text(&response), &self.config.api_key) });
}
if let Some(err) = detect_no_text_content(&response.content, response.stop_reason.as_deref()) { return Err(err); }
```

### Fallback hop rewrite
```rust
// Source pattern: fallback.rs:425 / :436
let mut hop_request = request.clone();
if let Some(model) = step_model { hop_request.model = model.clone(); }
match provider.generate(hop_request).await { Ok(mut r) => { r.metadata.insert(SERVED_BY_METADATA_KEY.into(), name.into());
    r.metadata.insert(SERVED_MODEL_METADATA_KEY.into(), hop_model.clone()); return Ok(r) } ... }
// pace_hop(provider, &hop_model, ...)   // was &request.model
```

### Layered config
```rust
// Source pattern: kimi/adapter.rs from_parts. `env` is injected, never std::env in tests.
fn from_layers(block: Option<&LlmProviderConfig>, agent_model: Option<&str>,
               env: &dyn Fn(&str) -> Option<String>) -> Result<Self, String>
```

## State of the Art

Vendor facts as of 2026-10-15. **Only the Anthropic rows were read from official documentation.** Everything else is a third-party/web-search summary or training knowledge and is `[ASSUMED]` until the D-08 probe; the doc hosts for those vendors are blocked here.

| Vendor | Fact | Tag |
|---|---|---|
| Anthropic | Current IDs `claude-fable-5-1`, `claude-opus-5-5`, `claude-sonnet-5-5`, `claude-haiku-5-5`: 1M context, 128K max output; legacy still available: Fable 5, Opus 5, Opus 4.8/4.7/4.6/4.5, Sonnet 5, Sonnet 4.6, Haiku 4.5. `claude-3-*` and `claude-3-5-sonnet-20241022` appear nowhere in the lineup | [CITED: platform.claude.com/docs/en/models/overview, 2026-10-15] |
| Anthropic | Models API returns `max_input_tokens`, `max_tokens` and a `capabilities` object per model — the seeding source for Anthropic | [CITED: same page] |
| Anthropic | On Opus 4.7 and later (incl. Opus 5.5) any non-default `temperature`/`top_p`/`top_k` returns HTTP 400; omit them | [CITED: platform.claude.com/docs/en/models/opus-5-5/migration-guide] |
| Anthropic | Whether Haiku 4.5 / Sonnet 4.6 / older still accept `temperature` is not stated in what was read | [ASSUMED] |
| Anthropic | `stop_reason` values: `end_turn`, `max_tokens`, `stop_sequence`, `tool_use`, `pause_turn`, `refusal`, `model_context_window_exceeded`. The adapter maps only the first three; the rest become `Error("unknown")` | [CITED: .../handling-stop-reasons] |
| Anthropic | Refusal = HTTP 200, `stop_reason:"refusal"`, `content: []`, `stop_details:{type,category,explanation}`; categories `cyber`, `bio`, `frontier_llm`, `general_harms`, null; may arrive mid-stream after partial output; some categories are billed | [CITED: .../refusals-and-fallback] |
| Anthropic | `stop_sequences` still supported on the 5.x line; thinking cannot be disabled on Opus 5.5 | `stop_sequences`: [ASSUMED] (not mentioned in the migration guide, no removal found); thinking: [CITED: migration guide] |
| OpenAI | `stop`: up to 4 sequences, "Not supported with latest reasoning models o3 and o4-mini"; `message.content` and `message.refusal` are `string|null`; `finish_reason` enum `stop,length,tool_calls,content_filter,function_call`; `max_completion_tokens` exists | [CITED: github.com/openai/openai-openapi master/openapi.yaml v2.3.0] |
| OpenAI | gpt-5.x chat-completions reject `stop` and `temperature` on reasoning models; reasoning models want `max_completion_tokens`; model IDs/windows | [ASSUMED] (forum/NVIDIA docs summaries only) |
| Moonshot | `kimi-k3`: 1,048,576 context; `max_completion_tokens` default 131072, ceiling 1048576; thinking always on, `reasoning_content` returned; fixed `temperature` 1.0 / `top_p` 0.95 to be omitted | [CITED: github.com/MoonshotAI/Kimi-K3 README for context/thinking]; sampling/`max_completion_tokens` from search summary of platform.kimi.ai quickstart [ASSUMED] |
| Moonshot | **Conflict:** vendor docs say K3 takes top-level `reasoning_effort` (low/high/max, default max) and does **not** accept `thinking`; the handoff measured the opposite on 2026-10-13 (top-level ignored with HTTP 200, `thinking.effort` validated). Phase 51's concern, but the profile must not hard-code either | [CITED docs] vs [handoff live measurement] |
| Moonshot | **Conflict:** `exceeded_current_quota_error` listed as HTTP 429 on platform.kimi.ai troubleshooting but as 403 on the kimi.com error-code page; `engine_overloaded_error` is 429 (transient). An override must key on `error.type`, not the status | [ASSUMED] (search summaries) |
| DeepSeek | `deepseek-v4-flash` / `deepseek-v4-pro`, 1M context, 384K max output; legacy `deepseek-chat`/`deepseek-reasoner` reportedly retire 2026-07-24 15:59 UTC — **already past**, so the current `deepseek-chat` default (`deepseek/adapter.rs:90`) and bridge default may not resolve | [ASSUMED] (third-party blogs; resolves handoff G-11 only after a probe) |
| xAI Grok | Grok-4-class reasoning models reject `stop`, `presence_penalty`, `frequency_penalty` (and `reasoning_effort` on Grok 4); prefer `max_completion_tokens`; several `grok-4*`/`grok-3` strings reportedly retired 2026-05-15 (default is `grok-4.6`) | [ASSUMED] (third-party wrappers) |
| Gemini | `generationConfig` carries `temperature, topP, topK, candidateCount, maxOutputTokens, stopSequences, seed, thinkingConfig`; model listing exposes input/output token limits | field list [ASSUMED] (search summary); listing fields [ASSUMED] |
| Qwen/DashScope | `stop` accepts `str | list[str] | list[int] | list[list[int]]`; `enable_thinking` per request for hybrid models, default is model-specific; regional base URLs; temperature range not officially confirmed | [ASSUMED] (search summaries) |
| Ollama | OpenAI-compat endpoint supports `stop`, `temperature`, penalties, `response_format`, `seed`, `max_tokens`, `reasoning_effort`; per-model thinking support from `/api/show`; context size is a Modelfile `num_ctx`, not request-settable | [CITED: raw.githubusercontent.com/ollama/ollama docs/api/openai-compatibility.mdx, 2026-10-15] |

**Deprecated/outdated in this tree:**
- Anthropic `validate_model`/`get_available_models` are hard-coded claude-3/2 lists (`anthropic/adapter.rs:~667-690`); none of those IDs is in the current lineup.
- `claude-3-5-sonnet-20241022` is the default in `AnthropicConfig::from_env` (`:119`), `bridge.rs:96`, `config.example.yml:33`, and ~30 test fixtures (the fixtures can stay; they are model-agnostic strings).
- `CompatRequestParameters` (static per-preset bool, no `stop`) is superseded by the per-model profile; keep it as the vendor-wide fallback when a model has no entry.

## Runtime State Inventory

Not a rename/refactor/migration phase in the sense of Step 2.5, but it changes behaviour of deployed config:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no persisted record keys on provider/model strings introduced here; `LlmResponse` gains an optional serde-default field, so persisted JSON (waypoints, run store) deserialises unchanged | none; add a serde round-trip test for legacy JSON without `reasoning` |
| Live service config | Operator `config.yml` files with `llm.<provider>` blocks that are **parsed but currently inert** (`base_url`, `default_model`, `default_temperature`, `timeout_seconds`, `max_retries`) become live | MIGRATION §9.5 row + §9.1 behavioural note (Pitfall 7) |
| OS-registered state | None found | none |
| Secrets/env vars | Legacy env vars (`OPENAI_BASE_URL`, `ANTHROPIC_MAX_TOKENS`, `*_API_KEY`, …) keep working as gap-fillers (D-13); YAML `api_key: "${VAR}"` placeholders must be resolved (Pitfall 8) | code edit; document precedence |
| Build artifacts | `.project/current-exports.txt` (8,575 lines) regenerated by `make api-surface-update`; consumer muster updated | regenerate in the same commit as each surface change |

## Open Questions (RESOLVED)

Every question below is answered by a named plan task; each carries its own `RESOLVED:` line (revision 1, 2026-10-15).

1. **`LlmResponse.reasoning` is a registered break, not additive (D-11 premise).**
   - Known: `LlmResponse` is constructible, no `Default`, ~83 literal sites; existing §9.2 row and allowlist entry cover `constructible_struct_adds_field`.
   - Unclear: extend the Phase 38 row vs add a row; whether to also add a constructor to make the next field free.
   - Recommendation: add `#[serde(default, skip_serializing_if = "Option::is_none")] pub reasoning: Option<String>`, extend the existing `LlmResponse` §9.2 row to name both fields, migrate all literals in one dedicated commit, add `LlmResponse::new(..)` + `with_reasoning(..)` so Phase 51 and later are free, update the muster. The streaming delta is additive (non-exhaustive); add `StreamingResponse::with_reasoning`.
   - RESOLVED: as recommended -- plan 50-11 Task 1 (`StreamingResponse.reasoning`, `LlmResponse::new`, `with_reasoning`), Task 2 (the field plus the scripted one-commit literal migration and muster update) and Task 3 (the Phase 38 9.2 row and allowlist entry extended from a measured D-27 run).

2. **How to add config fields to publicly constructible `*Config` structs.**
   - Unclear: break once and mark `#[non_exhaustive]` (X-10.3 option (a); makes Phase 51's fields free) versus plain field adds.
   - Recommendation: add fields, keep all `new()` signatures, mark each `*Config` `#[non_exhaustive]` with `with_max_retries/with_max_tokens/with_context_window/with_model_overrides` builders, add the `paladin-llm` lints table, run the D-27 diagnostic to measure the real lints, register one §9.2 row per crate-type. Confirm with the owner because it is a consumer-visible break the todo says must be carried to v0.11.0.
   - RESOLVED: decided at plan 50-08 Task 1 blocking checkpoint (`checkpoint:decision`, option-a `#[non_exhaustive]` + `with_*` builders or option-b plain fields; the owner's choice governs 50-08 Tasks 2-3 and 50-09's `LlmProviderConfig`); every literal site, the root crate's included, is migrated in 50-08 Task 3 and the register is measured in plan 50-10 Task 3.

3. **Where explicit-vs-auto provenance lives (D-01/D-02).**
   - Recommendation: Pattern 2 (refuse at build, filter at wire, no `PaladinData` field). Record the "direct `LlmRequest` callers get filter+warn" limit in ADR-0061. Needs owner awareness because D-01 reads as if refusal is universal.
   - RESOLVED: Pattern 2 adopted -- plan 50-02 Task 1 records the mechanism and its stated limit in ADR-0061 (e); plan 50-03 Tasks 1-3 implement refuse-at-build and the shared wire filter; plan 50-05 Tasks 1-2 put the four bespoke adapters on the same filter. Where the omission record is visible (the D-01 "run-metadata entry") is owner reading R-1, decided at plan 50-18 Task 1.

4. **Precedence between refusal, `ContentFilter` and `EmptyCompletion` (D-09 vs D-11a).**
   - Recommendation: refusal -> `Err(Refusal)`; `content_filter`/`SAFETY`/`RECITATION` with empty text stays `Ok(ContentFilter)` (Gemini test pins this); `FunctionCall` exempt; any other empty text -> `EmptyCompletion`. Amend the neutral `EmptyCompletion` Display suffix. Confirm the `Ok(ContentFilter)`-vs-`Err` choice with the owner; an `Ok` with empty content is what the empty guard exists to prevent elsewhere.
   - RESOLVED: CONTEXT D-09 settles `Ok(ContentFilter)`; plan 50-12 Task 2 writes the one precedence (`response_guards::classify_completion`: refusal > content filter > function call > empty on any finish), applied to the buffered and streamed paths of the compat engine, OpenAI and DeepSeek (50-12 Tasks 2-3) and of Anthropic and Gemini (50-13 Tasks 1-2).

5. **OpenAI's fabricated defaults.** `temperature` 0.7, `top_p` 1.0, `max_tokens` 4096 are always sent.
   - Recommendation: send nothing the caller did not set; add `default_max_tokens` as the config knob; document the 4096 removal in §9.1 (output caps change for every OpenAI user, and 4096 total *including reasoning* is what yields empty completions on reasoning models). Decide whether to preserve 4096 as the block default for non-reasoning models.
   - RESOLVED: plan 50-05 Task 1 stops sending the fabricated `temperature`/`top_p` and keeps 4096 as the documented compiled `max_tokens` default (A3-50-05); plan 50-08 Task 3 makes it the `OpenAIConfig.max_tokens` knob (`Some(4096)` from `new`/`from_env`), and plan 50-09 exposes it as `llm.openai.max_tokens`.

6. **API key resolution in `create_from_config`.** Recommendation: key = YAML `api_key` unless empty or an exact `${NAME}` placeholder, in which case read `NAME`, else the provider's env var; never log or `Debug`-print it; give `LlmProviderConfig` a manual redacting `Debug` (it derives `Debug`+`Serialize` with the key today) because the struct now has a production caller.
   - RESOLVED: as recommended -- plan 50-09 Task 1 (redacting `Debug`, outward-serialization check), Task 2 (`resolve_secret_placeholder`, exact `${NAME}` only) and Task 3 (`create_from_config`, errors naming the variable, never the value).

7. **Fallback chain model plumbing beyond the adapter.**
   - Known: `agent_runtime.model_fallback.providers` is `Vec<String>` (names only) resolved through env-only `factory.create` (`agent_runtime.rs:~1247`); `ModelFallbackMiddleware` wraps `FallbackLlmAdapter::new`, so the config path would keep sending hop 1's model to hop 2 — SC-1's "per-step model in a fallback chain" would pass on the library API and fail on the config path.
   - Recommendation: add an optional additive `steps: [{provider, model}]` to `ModelFallbackConfig` (keep `providers`), resolve through `create_from_config` + `from_steps`. Chain `model_profile` = strictest intersection across steps. Choose and name the D-10 flag (e.g. `hop_on_refusal(bool)`).
   - RESOLVED: plan 50-14 Task 1 (`FallbackStep`, `from_steps`, per-hop model rewrite feeding Cadence, `hop_on_refusal`) and Task 2 (`ModelFallbackConfig.steps` resolved through `create_from_config`); the chain profile is the intersection of the hops' profiles (plan 50-02 Task 3, re-pointed per step in 50-14 Task 1).

8. **`config.example.yml` blocks vs D-02.** Recommendation: remove `default_temperature` from blocks whose seeded default is `NotSent`/`Fixed` and add the §9.5 row (Pitfall 7).
   - RESOLVED: as recommended -- plan 50-10 Task 2 (`config.example.yml` documents the new keys, uses the dated defaults and drops `default_temperature` where the default model's profile does not admit it; MIGRATION §9.5 row).

9. **Seeding needs the operator's keys.** None of the seven vendor key variables is set here. Plan 50-04 needs a human-action checkpoint that runs a scripted probe (`GET /models` or equivalent, then one `generate()` per seeded model with the framework default params, then the quota/refusal/`json_object`/cache-field probes), writes a dated transcript into the plan directory, and fails closed (omit unverifiable models). Reuse and extend `live_vendor_smoke.rs` rather than a new binary.
   - RESOLVED: plan 50-04 Task 1 rebuilds `live_vendor_smoke` as the nine-provider harness with `--probe`/`--dry-run` (autonomous, wave 1); plan 50-17 Task 1 is the operator's `checkpoint:human-action` run on real keys (wave 3, beside 50-05, so it blocks only 50-06) and 50-17 Task 2 scrubs and commits `50-SEED-PROBE.md`, failing closed.

10. **Where the "Switching providers" page lives.** CONTEXT says "beside" the provider-expansion and contributing-providers pages. Recommendation: `docs/src/user-guides/switching-providers.md` (operator-facing, sits with the other user guides in `SUMMARY.md`), cross-linked from both of those pages; generate the per-provider/per-model table from the registry (a test that fails when a seeded model is missing from the page keeps it from drifting).
   - RESOLVED: as recommended -- plan 50-16 Task 1 (`docs/src/user-guides/switching-providers.md`, cross-links, drift test `switching_providers_page_lists_every_seeded_model`).

11. **Warn de-duplication.** Recommendation: one `warn!` per adapter instance per parameter name (`Mutex<HashSet<&'static str>>` or per-parameter `AtomicBool`), metadata on every response, so the visibility rule holds without log flooding.
   - RESOLVED: plan 50-03 Task 1 (`sampling::OmissionLog`, one `warn!` per adapter instance per (model, parameter); `ModelRegistry::note_unprofiled`, one per instance per model), reused by every request builder in 50-03 Task 2 and 50-05 Tasks 1-2.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Moonshot quota error is `exceeded_current_quota_error` at 429 or 403 (sources disagree) | State of the Art | Wrong override either retries a hard cap or treats a throttle as permanent; verify live (D-11a) |
| A2 | K3 effort field is top-level `reasoning_effort` per docs vs `thinking.effort` per handoff live probe | State of the Art | Phase 51 mis-seeds effort; Phase 50 must not encode either |
| A3 | DeepSeek `deepseek-chat`/`-reasoner` retired 2026-07-24; v4 models 1M ctx / 384K output | State of the Art | Current default is broken in production; verify live and replace |
| A4 | OpenAI gpt-5.x reject `stop`/`temperature`; want `max_completion_tokens` | State of the Art | Profile for OpenAI models wrong; verify live |
| A5 | Grok-4-class rejects `stop`/penalties/`reasoning_effort`; `grok-4*` strings retired | State of the Art | Grok default model may be dead; verify live |
| A6 | Gemini/Qwen/Ollama facts (stopSequences, token-limit listing fields, `enable_thinking`, `/api/show`, `num_ctx`) | State of the Art | Registry data wrong; verify live |
| A7 | Anthropic Haiku 4.5/Sonnet 4.6/older still accept `temperature`; 5.x accepts `stop_sequences` | State of the Art | Over- or under-refusal on Anthropic models; probe each seeded model |
| A8 | Design claim: refuse-at-build + filter-at-wire satisfies D-01/D-02 without provenance on `PaladinData` | Pattern 2 | If the owner wants refusal for direct `LlmRequest` callers, a provenance carrier (request metadata key) is needed |
| A9 | `ContentFilter` stays `Ok(..)` with empty content (Gemini test precedent) | Pitfall 5 / OQ 4 | Callers see empty `Ok` on a filtered response |
| A10 | `max_tokens` on newer OpenAI/xAI models must be sent as `max_completion_tokens` | Pitfall 11 | Requests rejected; per-model wire-name field in the profile |

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | everything | yes | 1.97.1 | — |
| `cargo clippy` | `make clean-code` | yes | 0.1.97 | — |
| `cargo-semver-checks` | D-27 measurement, `semver` CI job | yes | 0.50.0 | — |
| `cargo-audit` / `cargo-deny` | `make security` | yes | 0.22.2 / 0.20.2 | — |
| `cargo-llvm-cov` | 82 % gate | **no** | — | CI `coverage` job (`scripts/coverage.sh`, features `integration-tests,llm-all`); local approximation by adding tests for every new branch |
| `mdbook` | docs page build | **no** | — | CI docs job; add a plain markdown-link check test |
| Docker | integration/cadence tests | yes | 29.8.2 | not needed by this phase |
| Vendor API keys (`OPENAI/ANTHROPIC/DEEPSEEK/MOONSHOT/XAI/GEMINI/DASHSCOPE_API_KEY`) | D-08 seeding, D-11a usage-cap verification, live smoke | **no** (none set) | — | **Blocking for 50-04 and the live parts of 50-05..07**: human-action checkpoint with the operator's keys; entries not probed are omitted |
| Vendor documentation hosts | citing vendor facts | partially | only `platform.claude.com` and `raw.githubusercontent.com` reachable; `platform.kimi.ai`, `api-docs.deepseek.com`, `platform.openai.com`, `ai.google.dev`, `docs.x.ai`, `alibabacloud.com`, `docs.ollama.com` returned DNS failure or proxy 403 (policy denial; not retried) | WebSearch summaries tagged `[ASSUMED]`; the live probe is authoritative |

**Missing dependencies with no fallback:** vendor API keys for the seed/usage-cap probes (checkpoint required).
**Missing dependencies with fallback:** `cargo-llvm-cov` (CI), `mdbook` (CI).

## Validation Architecture

`workflow.nyquist_validation` is absent from `.planning/config.json`, so this section is included.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` (unit in `#[cfg(test)]`, integration in `tests/`), `mockito` 1.7 / `wiremock` 0.6 mock transports, `tokio::test` |
| Config file | `Cargo.toml` `[[test]]` entries; features `llm-all` (all nine adapters), `integration-tests` |
| Quick run command | `cargo test -p paladin-llm --all-features --lib <module>` (e.g. `conformance`, `sampling`, `fallback`) |
| Full suite command | `cargo test --workspace --features integration-tests,llm-all` then `make clean-code check-gates api-surface security check-consumer-surface check-migration-allowlist` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PARITY-01 | Same agent, nine providers, config-only switch; wire body + outcome; manual/auto temp on `(1,1)`; prompt on wire; per-step model; empty `stop` | integration (mock transports) | `cargo test --test provider_swap_matrix --features llm-all` | Wave 0 |
| PARITY-02 | Profile lookup; explicit refused naming provider/model/parameter; auto omitted + warn + metadata; `Fixed` semantics; decorators delegate | unit + builder tests | `cargo test -p paladin-llm --all-features --lib sampling` / `cargo test --lib paladin_builder` | Wave 0 (`sampling.rs`) |
| PARITY-02 | ADR-0061 present, PROMOTION.md at 0062, ADR-0004 note | doc guard | `make check-gates` (existing ADR/changelog guards) + grep assertion in plan verify | partial |
| PARITY-03 | Layered precedence agent > YAML > env > default; `${VAR}` placeholder; 300 s reasoning default; YAML-only provider available; existing YAML parses | unit | `cargo test -p paladin-llm --all-features --lib provider_factory config` | Wave 0 |
| PARITY-03 | Facade exports six more pairs; surface baseline | compile + gate | `cargo check --features llm-all` ; `make api-surface` | gate exists |
| PARITY-04 | `kimi-k3` window 1,048,576 via Commissary and HistoryTrimmer; override beats seeded; unknown model falls back + warn + metadata; table dates present | unit | `cargo test -p paladin-llm --all-features --lib registry commissary window` | Wave 0 |
| PARITY-05 | Per adapter: empty on `stop` -> `EmptyCompletion`; refusal -> `Refusal`; `content_filter` stays `Ok`; cache write/read; reasoning tokens + text (normal + stream); stop on wire; usage-cap override | conformance + adapter unit | `cargo test -p paladin-llm --all-features --lib conformance` | extend |
| PARITY-05 | `Refusal` is `Permanent`; labelled in `typed_origin`/`llm_error_class` | unit | `cargo test -p paladin-ports --lib llm_error_transience_table` ; `cargo test -p paladin-battalion --lib llm_failure` | extend |
| PARITY-06 | Hop N gets step N model; Cadence keyed on it; served model recorded; refusal stops by default, hops with the flag | unit | `cargo test -p paladin-llm --all-features --lib fallback` | extend |
| PARITY-07 | All nine in conformance (Anthropic new); smoke compiles; docs page lists every seeded model; MIGRATION/allowlist set-equal | conformance + guards | `cargo test -p paladin-llm --all-features --lib conformance` ; `cargo build -p paladin-llm --example live_vendor_smoke --all-features` ; `make check-migration-allowlist` | extend |
| PARITY-08 | OpenAI request body `messages[].content` carries `query` (+`context`) on the execution-service path | adapter test (mockito) | `cargo test -p paladin-llm --lib openai` | Wave 0 (write red first) |
| all | Consumer surface still compiles | compile-only | `make check-consumer-surface` | exists |
| all | Legacy persisted `LlmResponse` JSON deserialises without `reasoning` | unit | `cargo test -p paladin-ports --lib llm_response_without_cost_key_deserializes_to_none` pattern | extend |

### Sampling Rate

- **Per task commit:** the quick command for the touched module, plus `cargo fmt --check` and `cargo clippy -p <crate> --all-features -- -D warnings`.
- **Per wave merge:** `cargo test --workspace --features integration-tests,llm-all`.
- **Phase gate:** `make clean-code check-gates api-surface security check-consumer-surface check-migration-allowlist check-api-examples` green, CI coverage >= 82 %, D-27 `cargo semver-checks` diagnostic transcript for every crate with a registered break, before `/gsd-verify-work`.

### Wave 0 Gaps

- [ ] `tests/provider_swap_matrix.rs` + `[[test]] required-features = ["llm-all"]` — PARITY-01 (nine mock servers, one agent definition, config-only switch)
- [ ] `crates/paladin-llm/src/sampling.rs` with table-driven tests — PARITY-02
- [ ] `crates/paladin-llm/src/registry/` tables with a "every entry has a `verified_on` date" test — PARITY-04
- [ ] Anthropic `ConformanceFixture` + `Wire::Anthropic` variant; new cases; `CASE_COUNT` update — PARITY-07
- [ ] Shared capturing-logger helper (single `OnceLock`) if any `warn!` assertion is made
- [ ] Red mock-transport OpenAI regression test (written and run red before the fix) — PARITY-08

## Security Domain

`security_enforcement` is not disabled in config; included.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (outbound credentials only) | — |
| V3 Session Management | no | — |
| V4 Access Control | yes (narrow) | runtime-provisioned agents (`FacadeProvisioner`) can now select `provider` + `model` against operator-defined blocks; the caller must not be able to choose `base_url` or a key. Keep `base_url`/`api_key` operator-only (YAML), per-agent input limited to `provider` and `model` |
| V5 Input Validation | yes | per-agent `model` flows into the Gemini request **path** (existing `validate_model_identifier` guard, `gemini/adapter.rs` module docs) — keep it, and validate `model` and `provider` names before building anything; YAML `models.<id>` override keys and numeric overrides range-checked (no zero/overflow windows); refuse `Commissary` reservations >= window already exists |
| V6 Cryptography | no | no new crypto |
| V7 Error handling & logging | yes | redact-then-truncate for `Refusal.detail` and every vendor body in an error; no API key in any log or error from `create_from_config`; the `warn!` text names parameters, never values that could be secrets |
| V8 Data protection | yes | `LlmProviderConfig` derives `Debug` + `Serialize` with `api_key` in clear (`config/llm.rs:7`); with a production caller it must get a redacting `Debug` and `skip_serializing` on the key (or be verified never serialised outward); same review for each `*Config` that holds a key |
| V14 Configuration | yes | env-vs-YAML precedence documented; a YAML secret that is a literal `${VAR}` placeholder is never sent as a credential |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Credential header forwarded via redirect to an attacker host | Information Disclosure | every adapter already builds its client with `Policy::none()`; `base_url` from YAML now live — keep the policy, add a test per constructed adapter that the redirect policy survives `create_from_config` |
| Secret sliced across a truncation boundary in `Refusal.detail` / error text | Information Disclosure | `redact_and_bound` (redact first); regression test with a key straddling the budget |
| Literal `${OPENAI_API_KEY}` sent to the vendor as the key (and echoed in a vendor 401 body) | Information Disclosure | placeholder resolution (Pitfall 8); 401 bodies already redacted |
| Hostile model identifier injected into a URL path | Tampering | Gemini guard exists; add the same shape check at `create_from_config` for every provider |
| Silent parameter substitution hiding a policy decision | Repudiation | D-01 visibility rule: `warn!` + response metadata, asserted in tests |
| Provider chosen by an API caller outside the operator's allowlist | Elevation of Privilege | resolve only providers with a configured block / key; unknown -> `HostBuildError` naming provider and model |
| Manual-review gap (no Rust SAST gates a merge) | — | the manual credential-handling checklist in `security.instructions.md` runs on every plan touching config/adapters; CodeQL stays advisory |

## Project Constraints (from CLAUDE.md)

- TDD red-green-refactor; **82 % workspace line coverage** (ADR-0006, `cargo llvm-cov --fail-under-lines` in CI); doc tests on all public APIs; `# Examples` heading gate (`make check-api-examples`) for pub `*Builder/*Port/*Service`.
- Dependencies flow inward only (core -> ports -> adapters); never import infrastructure from core/application. `ModelProfile` therefore lives in `paladin-ports`; seeded data in `paladin-llm`; `PaladinBuilder` (application) only calls the port.
- Ubiquitous language: Medieval Military terms for role-bearing services; plain names for descriptors (D-00d). `Entity<T>` stays generic; "node" is only a WarGraph/Campaign vertex.
- No `unwrap()`/`expect()`/`panic!` in library code; prefer borrowing; lazy iterators.
- Before committing a parent task: `cargo test` -> `cargo fmt --check` -> `cargo clippy` -> `make api-surface` (refresh with `make api-surface-update` + CHANGELOG), conventional commit; **stop after each major task and wait for go-ahead**.
- Security: `make security`, `cargo clippy -- -D warnings`, manual credential-handling review; CodeQL advisory only; **do not reintroduce Snyk or record a phase as blocked on it**.
- Redact before truncate; no API key interpolated in logs; HTTP clients carrying a credential header must not follow redirects.
- Reusable commands: `make clean-code`, `make check-gates`, `make check-consumer-surface`, `make check-migration-allowlist`, `make api-surface`.

## Sources

### Primary (HIGH confidence — the tree)
- `/home/user/paladin-dev-env` at `5a6454d3`: `crates/paladin-ports/src/output/llm_port.rs`, `crates/paladin-llm/src/{compat/engine.rs,openai/adapter.rs,anthropic/adapter.rs,deepseek/adapter.rs,gemini/adapter.rs,kimi/adapter.rs,fallback.rs,cadence.rs,provider_factory.rs,config/{llm.rs,bridge.rs},window.rs,services/commissary.rs,conformance.rs}`, `crates/paladin-llm/examples/live_vendor_smoke.rs`, `src/application/services/paladin/{paladin_builder.rs,paladin_execution_service.rs}`, `src/infrastructure/web/{agent_host.rs,facade_provisioner.rs}`, `src/config/{agents.rs,agent_runtime.rs,settings.rs}`, `src/lib.rs`, `Cargo.toml`, `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `.planning/decisions/{0004,PROMOTION}.md`, `tests/consumer_surface_muster.rs`, `Makefile`

### Secondary (MEDIUM confidence — official docs fetched this session)
- [Claude models overview](https://platform.claude.com/docs/en/models/overview) — current IDs, 1M context, 128K output, Models API fields (2026-10-15)
- [Claude Opus 5.5 migration guide](https://platform.claude.com/docs/en/models/opus-5-5/migration-guide) — sampling parameters rejected, thinking cannot be disabled
- [Handling stop reasons](https://platform.claude.com/docs/en/build-with-claude/handling-stop-reasons) — full `stop_reason` list
- [Refusals and fallback](https://platform.claude.com/docs/en/build-with-claude/refusals-and-fallback) — refusal response shape, categories, mid-stream behaviour, billing
- [openai/openai-openapi](https://github.com/openai/openai-openapi) `openapi.yaml` v2.3.0 — `stop` maxItems 4, `refusal`, `finish_reason` enum
- [Ollama OpenAI compatibility](https://raw.githubusercontent.com/ollama/ollama/main/docs/api/openai-compatibility.mdx) — supported parameters, `num_ctx`
- [MoonshotAI/Kimi-K3](https://github.com/MoonshotAI/Kimi-K3) — 1,048,576 context, thinking always on, `reasoning_effort`

### Tertiary (LOW confidence — web-search summaries, vendor hosts blocked; all `[ASSUMED]`)
- Kimi API quickstart / thinking pages (platform.kimi.ai), Kimi error-code pages (kimi.ai, kimi.com help) via search summary
- DeepSeek V4 model/deprecation notes via third-party blogs; xAI Grok parameter restrictions via third-party wrappers; Alibaba Model Studio `stop`/`enable_thinking` via search summary; OpenAI forum/NVIDIA NeMo summaries for gpt-5.x parameter rejection; Gemini `generationConfig` field list via search summary

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependency; all tooling verified locally
- Architecture: HIGH for codebase mechanics and the premise corrections (grep-verified); MEDIUM for the provenance design (A8), which is a recommendation to confirm
- Pitfalls: HIGH for 1-11 and 13 (read from code and fetched official Anthropic/OpenAI schema); MEDIUM for 12 (depends on unverifiable vendor facts)
- Vendor facts: MEDIUM for Anthropic, LOW for the other eight providers until the D-08 probe

**Research date:** 2026-10-15
**Valid until:** 2026-10-22 for vendor facts (fast-moving; several sources already contradict the handoff); 2026-11-15 for codebase facts, or until the next commit touching `crates/paladin-llm`, `crates/paladin-ports/src/output/llm_port.rs` or `paladin_builder.rs`
