# Phase 50: Provider Parity & Configuration-Driven Model Swap - Context

**Gathered:** 2026-10-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Any Paladin agent can be moved between the nine LLM providers (Anthropic, OpenAI, DeepSeek, Kimi,
Qwen, Grok, Gemini, Ollama, OpenAI-compatible) by changing configuration only. Either the behaviour
is equivalent, or the build fails early with an error naming the provider, the model and the
parameter. The phase delivers, in the handoff's order: the red provider-swap matrix test and the
OpenAI empty-user-message fix (PARITY-01, PARITY-08); per-provider *and* per-model parameter-support
modelling under one policy recorded as an ADR amending ADR-0004 (PARITY-02); config-driven provider
construction through a public library API with all nine adapter/config pairs exported from the
facade (PARITY-03); a dated per-model profile registry with config overrides that Commissary and
`resolve_context_window` read (PARITY-04); response/error parity across every adapter — empty
output, typed refusal, usage cap, cache read/write, reasoning tokens and text, stop sequences
(PARITY-05); per-hop models in fallback chains with Cadence keyed on them (PARITY-06); and all nine
adapters in the shared conformance suite and the credential-gated live smoke plus an mdBook
"Switching providers" page and MIGRATION/CHANGELOG entries (PARITY-07).

Library-level APIs matter as much as server config: the only consumer (web3sec) builds adapters
directly from their `*Config` types and never touches `agent_host` or `LlmProviderFactory`.

**Out of scope** (handoff §9, ADR-0042): native tool/function calling, compat vision, prompt
caching (`cache_control`), request-id/trace propagation, the reasoning-effort setting itself
(Phase 51), and every web3sec-side change.

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by earlier phases — not re-asked)

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
  additive on the Rust surface wherever the type allows it (D-03, D-09, D-15, the streaming
  delta in D-11); the **known registered breaks** are the new field on the constructible
  `LlmResponse` (D-11) and new fields on the publicly constructible `*Config` structs (D-12), each
  needing a §9.2 row, a measured allowlist lint and a muster update — see `50-RESEARCH.md` Open
  Questions 1 and 2. Any further break the planner finds is recorded in the same commit as the
  code.
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

### Parameter-drop policy and support model (PARITY-02, handoff D-1)

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

### Per-model profile registry (PARITY-04, handoff D-3)

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

### Typed refusal and reasoning text (PARITY-05, handoff D-2)

- **D-09:** **Typed refusal:** a new additive `LlmError::Refusal { provider, model, detail }`
  variant on the `#[non_exhaustive]` enum, classified **`Permanent`** in the `Transience` table and
  given a row in the `llm_failure` label table. It is **distinct from `ContentFilter`**, which on
  the tree is a `FinishReason` variant, not an `LlmError`: Anthropic `stop_reason: "refusal"` and
  OpenAI's `refusal` message field map to `Err(Refusal)`; vendor `content_filter` / `SAFETY`
  finishes stay `Ok(LlmResponse { finish_reason: ContentFilter })` as today (the Gemini adapter
  pins this). `detail` carries the vendor's refusal text **redacted then truncated** (D-00a).
  Rejected: mapping refusals onto the `ContentFilter` finish reason.
  — **Reversibility:** costly — a public error variant consumers will match on; the additive
  variant needs no §9.2 row, but removing it later would.
- **D-10:** **Chain behaviour on refusal:** **stop by default** — `Permanent` short-circuits
  exactly as today, so the typed error reaches the consumer, who decides deliberately whether to
  try another provider (the handoff's stated use). A **plainly named chain setting, default off**,
  on `FallbackLlmAdapter` opts in to hopping on `Refusal`, mirroring Phase 51's
  fail-loud-default / explicit-opt-in pattern. Rejected: always hop (hides the refusal, spends
  budget on every hop); always stop with no opt-in.
- **D-11:** **Reasoning text** surfaces as a **typed `Option<String>` on `LlmResponse`** and a
  **typed field on the streaming delta**, on the normal and streaming paths of every adapter that
  receives it (compat presets' `reasoning_content`, DeepSeek, Anthropic thinking blocks). This is
  the shape Phase 51's roadmap entry already assumes. Rejected: `metadata["reasoning_content"]`.
  **Premise correction (research):** `StreamingResponse` is `#[non_exhaustive]`, so the delta
  field is additive; `LlmResponse` is **constructible and deliberately not `#[non_exhaustive]`**
  (`MIGRATION.md` §9.2, Phase 38's X-10.3 option (b) choice), so the new field is a **registered
  break**: extend the existing `LlmResponse` §9.2 row and allowlist entry, migrate every literal
  site in one commit, and update the consumer muster. Whether to also add
  `LlmResponse::new(..)`/`with_reasoning(..)` so later fields are free is the planner's call
  (`50-RESEARCH.md` Open Question 1). The decision to type the field stands.
  — **Reversibility:** costly — a public field on a constructible struct in `paladin-ai-ports`;
  Phase 51 reads it.
- **D-11a:** The rest of PARITY-05 is settled by the requirement text and not re-opened: empty
  text is `EmptyCompletion` on **any** stop reason in every adapter (OpenAI included); compat
  presets map `prompt_tokens_details.cache_write_tokens` via `with_cache_write`; usage-cap
  overrides per compat vendor are **live-verified** under the credential gate before being
  encoded; stop sequences go on the wire wherever the profile says the vendor accepts them.

### Config-driven construction and fallback hops (PARITY-03, PARITY-06, handoff D-4)

- **D-12:** **Public library API:** `LlmProviderFactory::create_from_config(&LlmConfig, provider,
  model)` beside the env-only `create(name)`, returning `Arc<dyn LlmPort>` and **routed through
  the existing `From<&LlmProviderConfig>` bridges in `crates/paladin-llm/src/config/bridge.rs`**,
  which gain the new fields (default `max_tokens`, context-window override, the D-06 model
  overrides). `agent_host` resolves per-agent `provider` + `model` (already in
  `src/config/agents.rs`) through the same call. The `*Config` types remain the direct-construction
  path web3sec uses and gain the same fields. Rejected: a separate builder type.
  **Premise correction (research):** the `*Config` structs are publicly constructible and
  `paladin-llm` has no semver-lint table yet, so adding fields is a consumer-visible break however
  it is done. Whether to break once and mark each `*Config` `#[non_exhaustive]` with `with_*`
  builders (so Phase 51's fields are free) or to add plain fields is **open for the owner at plan
  time** (`50-RESEARCH.md` Open Question 2); either way the break gets §9.2 rows, measured lints
  and a muster update (D-00b, D-00i).
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

### Folded Todos

- **web3sec moves once, straight to v0.11.0, after Phases 50 and 51**
  (`.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md`). Problem: the consumer
  must not adopt `v0.10.1` and break again at `v0.11.0`. Fit: every intentional break or additive
  surface change in this phase (facade re-exports, `Refusal`, `ModelProfile`, `FallbackStep`,
  `create_from_config`, new config fields) is carried into `MIGRATION.md` §9.2/§9.8 and the
  CHANGELOG before the tag, and `tests/consumer_surface_muster.rs` is kept green or updated as
  part of the registered break (D-00i).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The handoff and the phase record
- `.planning/phases/50-provider-parity-config-driven-model-swap/50-HANDOFF-web3sec-parity.md` —
  the source audit: §3 live vendor facts (dated, re-verify), §4 parity matrix, §5 gaps B-1..B-5,
  G-1..G-11, H-1..H-3, §6 decisions D-1..D-4 (settled above), §7 six-plan shape, §9 out of scope.
  Its `path:line` references are at `df24689c`; the ROADMAP entry re-verified them on `83a16bfa`.
- `.planning/ROADMAP.md` § "Phase 50" — goal, ground truth, six success criteria.
- `.planning/REQUIREMENTS.md` § "Provider parity (PARITY)" — PARITY-01..08.
- `.planning/phases/51-adjustable-reasoning-effort/51-HANDOFF-web3sec-reasoning-effort.md` and
  `.planning/ROADMAP.md` § "Phase 51" — the dependent phase; its D-1..D-5 constrain the
  `ModelProfile` fields (effort levels, budget bounds) and the typed reasoning surfaces (D-11).

### ADRs this phase extends or obeys
- `.planning/decisions/0004-temperature-validation.md` — provider-aware validation at the ports
  boundary, no undiscoverable substitution; amended by the new ADR-0061 (D-04).
- `.planning/decisions/0046-facade-llm-feature-flag-wiring.md` — the nine facade features and the
  three defaults (D-14).
- `.planning/decisions/0042-llm-native-tool-calling-deferred.md` — tool calling and compat vision
  stay out of scope.
- `.planning/decisions/0049-commissary-design-and-rename.md` — Commissary's role and the
  `resolve_context_window` contract the registry feeds (D-05).
- `.planning/decisions/0058-rate-pacing-cadence.md` — the `(provider, model)` Cadence key (D-15).
- `.planning/decisions/0006-coverage-gate.md` — the 82 % floor.
- `.planning/decisions/0051-token-economy-versioning-x03-supersession.md`,
  `.planning/decisions/0059-legacy-clean-break-removal.md`,
  `.planning/decisions/0060-base-entity-rename.md` — the X-03 supersession pattern to follow only
  if a break becomes unavoidable (D-00b).
- `.planning/decisions/PROMOTION.md` — next free ADR number (0061) and the numbering protocol
  (D-00c, D-04).
- `.planning/milestones/v0.10.0-phases/26-agent-runtime-enhancements/26-CONTEXT.md` § D-28 — the
  `ProviderCapabilities` no-new-field guard and the `#[non_exhaustive]` precedent (D-00e, D-03).
- `.planning/milestones/v0.10.0-phases/30-token-economy-vocabulary-commissary-anchoring/30-CONTEXT.md`
  § D-01 — plain names for units/descriptors, medieval names for roles (D-00d).

### Migration, surface and security gates
- `MIGRATION.md` §9.2 (Rust API changes), §9.5 (configuration), §9.8 (library consumers
  v0.10.1 → v0.11.0) — rows for every intentional break and the consumer note this phase extends.
- `CHANGELOG.md` `[Unreleased]` — entry for every surface change.
- `.cargo/semver-checks-allowlist.toml` — paired with §9.2 `Y` rows (D-00b).
- `tests/consumer_surface_muster.rs` — the Phase 49 consumer-surface guard (D-00i).
- `.github/instructions/security.instructions.md` — redact-then-truncate and the manual
  credential-handling review (D-00a, D-09).
- `.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md` — folded todo.

### Docs
- `docs/src/SUMMARY.md`, `docs/src/appendix/provider-expansion.md`,
  `docs/src/contributing/contributing-providers.md` — where the "Switching providers" page joins
  the mdBook.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/paladin-llm/src/config/bridge.rs`: `From<&LlmProviderConfig>` for eight adapter configs
  already exists with no production caller — the D-12 constructor routes through it rather than
  duplicating the mapping.
- `crates/paladin-llm/src/config/llm.rs`: `LlmConfig` already has one `Option<LlmProviderConfig>`
  per provider plus `default_provider`; `LlmProviderConfig` has `api_key`, `base_url`,
  `default_model`, `default_temperature`, `timeout_seconds`, `max_retries` and needs `max_tokens`,
  a context-window override and the D-06 `models` map.
- `crates/paladin-llm/src/conformance.rs` (`ConformanceFixture` trait) and
  `crates/paladin-llm/examples/live_vendor_smoke.rs`: the shared suite and credential-gated smoke
  to extend; Anthropic and the originals are missing from both.
- `crates/paladin-llm/src/compat/engine.rs` (`CompatEngineConfig`, `CompatCapabilities`,
  `CompatRequestParameters`, `error_override`): the one place the six compat presets share
  request building, empty-completion detection and usage mapping — most PARITY-05 fixes land once
  here.
- `crates/paladin-llm/src/window.rs` (`resolve_context_window`, `WindowFallbackPolicy`) and
  `crates/paladin-llm/src/services/commissary.rs`: `resolve_context_window` already takes a
  caller-supplied table; Commissary just supplies none today.
- `crates/paladin-llm/src/fallback.rs`: `SERVED_BY_METADATA_KEY = "paladin.served_by"`,
  `with_cadence`, `with_trace_emitter`; hop-on-Transient / stop-on-Permanent logic.
- `crates/paladin-core/src/platform/container/transience.rs` and
  `crates/paladin-battalion/src/llm_failure.rs`: the tables a new `LlmError` variant must join.
- Mock transports (`mockito`, `wiremock`) already used by adapter tests; `MockLlmAdapter` and
  `MultiStepMockLlmPort` exported from the facade.

### Established Patterns
- Additive public surface via `#[non_exhaustive]` on `LlmError`, `LlmRequest`, `LlmResponse` and
  `ProviderCapabilities`; new trait methods get default bodies (the Phase 26 `response_format`
  precedent).
- Fail loud by default, substitute only on explicit opt-in (ADR-0004; Phase 51's D-3).
- Every live vendor fact is dated and re-verified under the existing credential gate (Phases
  17-19 temperature probes).
- Redact before truncate for any vendor text embedded in an error.
- Every public-surface change: `make api-surface-update` + CHANGELOG; every break: §9.2 row +
  allowlist entry, measured lint.

### Integration Points
- `src/application/services/paladin/paladin_builder.rs` (`validate`, `build`,
  `auto_temperature_enabled`, `manual_temperature_override`): where D-01/D-02 enforcement and the
  D-03 `model_profile` lookup go.
- `crates/paladin-llm/src/openai/adapter.rs` user-turn construction (B-4): reads `context`, must
  read `query` with `context` appended.
- `crates/paladin-llm/src/provider_factory.rs` (`LlmProviderFactory::create`, `provider_names`)
  and `src/infrastructure/web/agent_host.rs` (`default_provider` resolution): the D-12/D-13 path.
- `src/config/agents.rs` `AgentDefinition { model, provider: Option<String>, temperature,
  timeout_seconds, … }`: the per-agent inputs to D-13.
- `src/lib.rs` facade re-export block (`#[cfg(feature = "llm-…")] pub use …`): D-14.
- `tests/consumer_surface_muster.rs`: goes red on surface changes and is updated with them.

</code_context>

<specifics>
## Specific Ideas

- The provider-swap matrix test must be **red before the fixes** (SC-1): write it against the
  current tree first, prove the five blocking shapes fail, then land fixes plan by plan.
- The OpenAI B-4 fix is "the single highest-value fix in this handoff" and ships in Plan 1 with its
  own red mock-transport test (PARITY-08).
- Every error this phase introduces names **provider, model and parameter** — the phase's one
  user-visible promise.
- Visibility rule everywhere a value is not sent (D-01 auto drop, D-07 unknown-model fallback):
  `warn!` plus a run-metadata entry, in the `paladin.served_by` key style.
- Reasoning-capable models default to **300 s** timeouts; the handoff measured 56 s / 359 s /
  473-492 s on `kimi-k3` at low / high / max effort.
- The consumer builds adapters from `*Config` types directly: every new knob must be reachable
  there, not only through YAML or `agent_host`.

</specifics>

<deferred>
## Deferred Ideas

- **Live `/models` discovery** of per-model limits (handoff D-3 option c) — optional later work once
  the dated table plus overrides is in; adds a network dependency to construction.
- **Native tool/function calling, compat vision, prompt caching, request-id propagation** — out of
  scope here (handoff §9, ADR-0042).
- **Reasoning effort** — Phase 51.

### Reviewed Todos (not folded)
- *Reject removed v0.10 Battalion and Conclave config keys with an error naming the key*
  (`2026-10-11-reject-removed-v0-10-battalion-config-keys.md`) — shares the "error names the key"
  principle but its files are Battalion/Conclave; stays its own item before the Phase 47 publish.
- *Verify local `make coverage` reproduces CI's 82.39 %* (`2026-08-13-verify-local-coverage-reproduction.md`)
  — tooling; relevant only as a reminder that this phase adds much test code against the floor.
- *Phase 44 code review follow-ups* (`2026-10-11-phase-44-review-follow-ups.md`) — unrelated to
  provider work.
- *Review the unfinished content / general data orchestration system before removing any
  base-layer item* (`2026-10-13-review-content-data-orchestration-base-items.md`) — unrelated.

</deferred>

---

*Phase: 50-provider-parity-config-driven-model-swap*
*Context gathered: 2026-10-15*
