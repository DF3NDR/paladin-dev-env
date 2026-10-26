# Phase 51: Adjustable Reasoning Effort - Context

**Gathered:** 2026-10-21
**Status:** Ready for planning

<domain>
## Phase Boundary

One provider-neutral `ReasoningEffort` setting, settable per request, per agent and per provider,
that every adapter translates **exactly** to its vendor's native wire form or refuses **before the
HTTP call** with a new additive `LlmError::UnsupportedParameter`. Supported levels and budget bounds
are per model, read from Phase 50's model-profile registry. Requested and effective effort reach the
response metadata and the Phase 28 trace stream. A silent-no-op regression test (Kimi), offline
wire-body cases per adapter/preset, a live effort probe and conformance check, ADR-0062 amending
ADR-0004, an mdBook "Reasoning effort" page, MIGRATION §9.2 and CHANGELOG close it out.
Requirements REASON-01..07.

Out of scope (handoff §10, unchanged): native tool calling / interleaved thinking with tools
(ADR-0042), Anthropic's per-message effort beta, live discovery of levels from vendor `/models`
endpoints, the OpenAI Responses API wire, and every web3sec-side change.

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by the roadmap, the handoff and earlier phases — not re-asked)

- **D-00a:** Gates as Phase 50 D-00a / 50.1 D-00a: 82 % workspace line-coverage floor (ADR-0006);
  `make clean-code`, `make check-gates`, `make api-surface` (+ `make api-surface-update` and a
  CHANGELOG entry; baseline `.project/current-exports.txt`), `make security`,
  `make check-api-examples`, `make check-migration-allowlist`, `make check-consumer-surface` and
  the manual credential-handling review gate every commit. No `unwrap`/`expect`/`panic!` added to
  library code. Response bodies redacted **before** truncation in any error text.
- **D-00b:** X-03 / ADR-0060 clean-break policy: any break gets a MIGRATION §9.2 row and a
  measured `.cargo/semver-checks-allowlist.toml` lint; no deprecated aliases. This phase is meant
  to be **additive**: `LlmRequest`, `LlmError`, `ModelProfile`, `StreamingResponse` and the nine
  `*Config` types are all `#[non_exhaustive]` (Phase 50 D-12 / 50.1 D-00d). Any break the planner
  finds anyway (e.g. a field on a `TraceEvent` struct variant, or on the constructible
  `LlmResponse`) is registered in the same commit as the code, and the consumer muster
  (`tests/consumer_surface_muster.rs`) is updated, never silenced.
- **D-00c:** ADR number **0062** (`.planning/decisions/PROMOTION.md` "Next free ADR number: 0062";
  reserved for this phase by Phase 50 D-04). It is recorded as *amending ADR-0004*, ADR-0004 gains
  a dated `**Amended by ADR-0062**` note, and PROMOTION.md advances to 0063 in the same commit.
- **D-00d:** Handoff **D-1 (enum shape):** `#[non_exhaustive]` `ReasoningEffort { Off, Auto,
  Minimal, Low, Medium, High, XHigh, Max, Budget(u32) }`, plain name (Milestone 13 §5 vocabulary:
  a unit, not a role). `Auto` stays distinct from "unset" (`None`); `Budget` ships now.
- **D-00e:** Handoff **D-2 (placement):** `LlmRequest.reasoning: Option<ReasoningEffort>`
  (`#[serde(default)]`) + `with_reasoning`, the `response_format` precedent; a `#[serde(default)]`
  `PaladinData` field, a `PaladinBuilder` setter and an optional YAML `AgentDefinition` field; a
  `reasoning_effort` knob on the nine `*Config` types through `config_knob_builders!`
  (`crates/paladin-llm/src/registry/mod.rs:182`) and `llm.<provider>.reasoning_effort` in YAML.
  Precedence request → agent → provider config → vendor default. Never `PromptParameters`, never
  `ProviderCapabilities` (its `provider_capabilities_gained_no_field` guard stays).
- **D-00f:** Handoff **D-3 (policy):** fail loudly by default — `LlmError::UnsupportedParameter
  { param, value, provider, model }`, classified `Permanent` in the `Transience` table and given a
  row in `crates/paladin-battalion/src/llm_failure.rs` (Phase 50 D-00g) — for a level the vendor
  lacks, a level gated off for that model, `Off` on an always-thinking model, `Budget` where only
  levels exist, and a level the vendor is known to coerce silently. Never the compat engine's
  omit-and-log path. Nearest-level substitution is an explicit opt-in, shaped by D-05..D-07 below.
- **D-00g:** Handoff **D-4 (reasoning text):** already delivered by Phase 50 D-11 —
  `LlmResponse.reasoning` and `StreamingResponse.reasoning` are typed `Option<String>`. This phase
  makes sure every adapter that receives reasoning text fills them and does **not** add a second
  surface.
- **D-00h:** Handoff **D-5 (Anthropic):** `output_config.effort` where the model's profile says
  it is supported, `thinking.budget_tokens` only on models that still accept it, chosen per model
  profile; the deprecated form is never sent to a model that rejects it.
- **D-00i:** **Unset is byte-identical.** `reasoning: None` at every level produces today's request
  body on every adapter, pinned by a mockito wire-body test per adapter/preset written red first.
  Consequently **no compiled-in effort default for any provider, Kimi included**: REASON-06's
  "kimi-k3 at the documented defaults answers" is met by the mdBook guidance and a documented
  `llm.kimi.reasoning_effort` / `max_tokens` / timeout example, not a baked-in value.
- **D-00j:** Phase 50 D-13 / 50.1 D-00h: reasoning-capable models already default to a 300 s
  timeout; Cadence (ADR-0058) is keyed on `(provider, model)`. Phase 50 D-15: `FallbackStep
  { port, model }` / `FallbackLlmAdapter::from_steps` rewrite `request.model` per hop; this phase
  resolves effort against that per-hop model (REASON-04).
- **D-00k [informational]:** The handoff's §8 five-plan shape (red tests → types and plumbing → compat presets →
  native adapters → live conformance, docs and ADR) is the starting point for planning, not a
  constraint. The handoff's §4 vendor table and §6.5 mapping table are **inputs to re-verify**, not
  data to encode (see D-01).

### Effort-level data and unprofiled models

- **D-01:** **Source of truth: a new live effort probe.** `crates/paladin-llm/examples/live_vendor_smoke.rs`
  gains an effort-probe mode (beside `--probe` and `--usage`) that, per candidate model on the
  operator's keys, sends each `ReasoningEffort` level through the vendor's declared wire form, sends
  one **invalid** value (must be rejected: proves the field is parsed), and records
  `reasoning_tokens` per accepted level. It runs as a **human-action checkpoint** in the 50-17 /
  50.1 pattern; its scrubbed transcript is a new dated evidence file in this phase directory with
  the `50.1-SEED-PROBE.md` provenance header. Supported levels and budget bounds are written to
  `ModelProfile` (a new additive field or fields on the `#[non_exhaustive]` struct; naming is
  Claude's discretion) **only where measured**, each entry keeping its `verified_on` date.
  ADR-0061's seed rule stays word for word: a value with no evidence in a transcript is not
  written. Rejected: desk-research seeding with a "documented" tier (breaks ADR-0061); overrides
  only (every model refuses out of the box).
  — **Reversibility:** reversible — data rows with their own verified-on dates.
- **D-02:** **Probe scope: every seeded entry with `reasoning: true`.** Non-reasoning seeded
  entries get an explicit "does not reason" effort state rather than a measurement: `Off` on such a
  model is an **exact no-op** (nothing sent, accepted), and every other level refuses. Ollama and
  OpenAI-compatible stay out of the probe run (no daemon / endpoint configured, as in 50-17 and
  50.1). The planner drafts the candidate list from the seeded tables and shows it in the
  checkpoint; the owner strikes or adds ids before the run (50.1 D-16 pattern). Rejected: every
  seeded model; web3sec's models plus one per vendor.
- **D-03:** **No measured levels → refuse.** An explicit effort (anything but unset) on a model
  with no measured effort data — unknown to the registry, or profiled but unprobed for effort, and
  with no config override — returns `UnsupportedParameter` before the HTTP call. The error text
  names the `llm.<provider>.models.<model-id>` override (Phase 50 D-06 shape) that unlocks it.
  Deliberately **stricter than** Phase 50 D-07's warn-and-fall-back for sampling parameters,
  because a wrong effort silently changes cost, latency and answers. Unset effort is unaffected.
  Rejected: adapter-wide fallback with `warn!` + `paladin.unprofiled_model`.
  — **Reversibility:** costly — a consumer-visible refusal rule recorded in ADR-0062; loosening
  it later is a behaviour change consumers may already depend on.
- **D-04:** **A wire form that cannot be confirmed live ships `Unsupported`.** Where the probe
  cannot confirm a vendor's request field (xAI Grok's chat-completions effort field is unconfirmed
  in the handoff; any other vendor whose invalid-value probe is not rejected), that preset's or
  adapter's declared wire form is `ReasoningWire::Unsupported` and every explicit effort refuses.
  The mdBook mapping-table row says "unverified" with the reason. Flipping it later is a probe plus
  a one-line wire change. Rejected: shipping the desk-research form.

### Nearest-level opt-in (the D-3 escape hatch)

- **D-05:** **A plain `ReasoningFallback` setting beside the effort, with the same three levels and
  precedence:** per request (`LlmRequest` field + `with_reasoning_fallback`), per agent
  (`PaladinData` / `PaladinBuilder` / YAML `AgentDefinition`) and per provider (`*Config` knob +
  `llm.<provider>.reasoning_fallback`), resolved request → agent → provider → default. Default is
  refuse. Request-level reach is required because web3sec builds adapters directly. Rejected:
  provider config only; request only.
  — **Reversibility:** costly — a public type and fields on `LlmRequest`, `PaladinData` and nine
  `*Config` types.
- **D-06:** **Direction is the caller's choice:** two substituting variants, `NearestLower` (cost-
  and latency-safe) and `NearestHigher` (quality-safe), beside the default refuse variant (exact
  variant names are Claude's discretion; the enum is `#[non_exhaustive]`). "Nearest" is measured
  along the ordered `Minimal < Low < Medium < High < XHigh < Max` scale against the model's
  **measured** level set; when no level exists in the chosen direction, the request refuses (no
  silent flip of direction). A substitution logs `warn!` and records both requested and effective
  effort (D-11). Rejected: always round down; always round up.
- **D-07:** **Coverage: level → level only.** The opt-in substitutes only for "a level the model
  lacks" and "a level gated off for that model". `Off` on an always-thinking model, any `Budget` ↔
  level conversion, a level the vendor is known to coerce silently, and any model with no measured
  levels (D-03) **still refuse** with the opt-in set. Rejected: also `Off` → lowest level;
  heuristic `Budget` ↔ level mapping.

### Fallback chains and build-time checks

- **D-08:** **Pre-flight every hop.** `FallbackLlmAdapter` resolves the request's effort (and
  fallback setting) against **every** step's own model (`FallbackStep.model`, via each port's
  `model_profile`) **before the first HTTP call**. If any hop can neither honour it nor substitute
  per D-05..D-07, the whole request refuses with `UnsupportedParameter` naming that hop's provider
  and model. A misconfigured chain therefore fails on its first call, not during an outage of
  hop 1. Each hop that is actually reached sends its own resolved effort (substitution may differ
  per hop), recorded per D-11 for the served hop. Rejected: validate lazily per hop and stop the
  chain (failure hidden until failover); skip incompatible hops (failover silently loses hops).
  — **Reversibility:** reversible — behaviour inside `FallbackLlmAdapter`, no public shape.
- **D-09:** **Build-time check as well as send-time.** `PaladinBuilder::build` and `agent_host`
  refuse an agent- or provider-level effort that the agent's model profile cannot honour (for a
  chain, the per-hop check of D-08 or the intersected profile — planner's call), mirroring Phase 50
  D-01's build-time refusal of explicit sampling values. The adapter's pre-HTTP check stays the
  authority for per-request values. `ModelProfile::intersect` gains an effort rule consistent with
  its existing AND semantics for `json_object` / `json_schema` (a level is usable only if every
  model supports it). Rejected: send-time only.

### Self-hosted endpoints (OpenAI-compatible, Ollama)

- **D-10:** **Operator-declared wire form plus per-model levels.** `OpenAiCompatibleConfig` and
  `OllamaConfig` gain a `with_reasoning_wire(..)` knob (and YAML `llm.<provider>.reasoning_wire`);
  the levels come from `llm.<provider>.models.<model-id>` overrides. **Both** are required before
  any explicit effort is sent; the compiled default stays `Unsupported` for both adapters
  (generic compat per REASON-02; Ollama per D-04). The operator owns verification, as with the
  capabilities preset `OpenAiCompatibleConfig::new` already requires (50.1 D-08). Verification is
  supported by the D-01 effort-probe mode accepting an `openai_compatible` / `ollama` target (base
  URL from env), and the mdBook page tells operators to run it: the invalid-value-rejected result
  is their proof the field is parsed. No network call at construction (Phase 50 D-05). **Self-hosted
  only:** the named vendor presets (Kimi, Qwen, Grok) take no wire override; they change only
  through Paladin's own probe and a release, so the shipped mapping table stays the truth for named
  vendors. Rejected: per-model levels only with fixed wire forms; nothing this phase; overridable
  wire forms on every compat preset.
  — **Reversibility:** costly — a public `ReasoningWire` type reachable from two `*Config` types
  and YAML.

### Observability (settled by REASON-05, not re-asked)

- **D-11:** Requested and effective effort go into `LlmResponse.metadata` beside
  `SERVED_BY_METADATA_KEY` / `SERVED_MODEL_METADATA_KEY` (`crates/paladin-llm/src/fallback.rs:176`,
  `:184`) under `paladin.*` keys in the existing style, and onto the Phase 28 trace stream
  (`TraceEvent::NodeFinished`, `crates/paladin-core/src/platform/container/trace.rs:248`, with
  `#[serde(default, skip_serializing_if = "Option::is_none")]` as the `cost` field precedent).
  Every adapter fills `TokenUsage.reasoning_tokens` when the vendor reports it.

### Claude's Discretion

- Plan count and order, starting from the handoff §8 shape; the effort-probe checkpoint (D-01/D-02)
  is likely its own human-action plan blocking only the seed-data plan, as in 50-17 and 50.1.
- `ReasoningWire` variant names and payloads (the handoff §6.3 candidates are a starting point),
  and how `CompatRequestParameters::all()` — used today by Qwen, Ollama and OpenAI-compatible —
  is reshaped so that **every** preset still states a reasoning position explicitly (REASON-02's
  compile-enforcement must not be bypassed by a convenience constructor).
- `ModelProfile` field names for levels and budget bounds, the "does not reason" state (D-02),
  and the override keys under `llm.<provider>.models.<model-id>`.
- The private `map_reasoning` signatures in Anthropic, OpenAI, DeepSeek and Gemini, and how Gemini
  chooses `thinkingLevel` (3.x) vs `thinkingBudget` (2.5) per profile.
- `ReasoningFallback` variant names, metadata key names, `warn!` targets, and the exact trace field
  names.
- How `Budget(n)` validation reads bounds (profile vs vendor constant) — the bounds themselves
  come from the probe or vendor docs cited in the evidence file; REASON-06 lists the known ones.
- Wire-body test layout in `crates/paladin-llm/src/conformance.rs` and where the Kimi silent-no-op
  regression test lives.
- mdBook page placement (beside `docs/src/appendix/provider-expansion.md` and Phase 50's
  "Switching providers" page) and the mapping-table layout.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The handoff and the phase record
- `.planning/phases/51-adjustable-reasoning-effort/51-HANDOFF-web3sec-reasoning-effort.md` — the
  source: vendor survey (§4), measured Kimi costs (§4.1), silent-no-op hazard (§5), recommended
  design (§6), D-1..D-5 (§7), plan shape (§8), out of scope (§10), Kimi probe transcript (App. A)
- `.planning/ROADMAP.md` § "Phase 51: Adjustable Reasoning Effort" — goal, ground truth
  (verified 2026-10-14 on `885db3b8`), six success criteria
- `.planning/REQUIREMENTS.md` — REASON-01..07

### ADRs this phase extends or obeys
- `.planning/decisions/0004-temperature-validation.md` — "no undiscoverable substitution"; amended
  by ADR-0062 (D-00c)
- `.planning/decisions/0061-per-model-parameter-support-policy.md` — the D-01 parameter policy,
  `ModelProfile` shape and the seed rule D-01 keeps word for word
- `.planning/decisions/PROMOTION.md` — next free ADR number (0062)
- `.planning/decisions/0006-coverage-gate.md` — 82 % floor
- `.planning/decisions/0042-llm-native-tool-calling-deferred.md` — why interleaved thinking with
  tools is out of scope
- `.planning/decisions/0046-facade-llm-feature-flag-wiring.md` — facade default features
- `.planning/decisions/0058-rate-pacing-cadence.md` — `(provider, model)` pacing key

### Prior phase decisions this phase builds on
- `.planning/phases/50-provider-parity-config-driven-model-swap/50-CONTEXT.md` — D-01..D-15
  (parameter policy, `ModelProfile`, registry, typed reasoning text D-11, `FallbackStep` D-15,
  precedence and 300 s timeout D-13), D-00j (Phase 51 assumptions)
- `.planning/phases/50.1-consumer-surface-readiness/50.1-CONTEXT.md` — `*Config::new` shape and
  `config_knob_builders!` (D-05..D-08), muster policy (D-00g), probe checkpoint pattern (D-14..D-18)
- `.planning/phases/50.1-consumer-surface-readiness/50.1-SEED-PROBE.md` and
  `50.1-PROBE-CANDIDATES.md` — the evidence-file and candidate-list format the effort probe follows

### Migration, surface and security gates
- `MIGRATION.md` §9.2 (Rust API changes, line ~188) and §9.8 (upgrade checklist, line ~928)
- `.cargo/semver-checks-allowlist.toml`, `.project/current-exports.txt`
- `tests/consumer_surface_muster.rs` — compile-time consumer surface (`make check-consumer-surface`)
- `.github/instructions/security.instructions.md` — manual credential-handling review (the probe
  and smoke handle API keys)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ModelProfile` (`crates/paladin-ports/src/output/model_profile.rs:289`): `#[non_exhaustive]`,
  `with_*` setters, fail-closed `new()`, `intersect()` (`:466`) — effort levels and budget bounds
  are added here.
- Seeded registry (`crates/paladin-llm/src/registry/seeded.rs`): per-vendor dated tables, one
  `SeededProfile { model, verified_on, build }` per measured model; module docs state the seed rule.
- `config_knob_builders!` (`crates/paladin-llm/src/registry/mod.rs:182`): generates the `with_*`
  knobs on all nine `*Config` types — the home for `with_reasoning_effort` /
  `with_reasoning_fallback` (and `with_reasoning_wire` on the two self-hosted configs).
- `UNPROFILED_MODEL_METADATA_KEY` (`registry/mod.rs:54`), `PARAMS_OMITTED_METADATA_KEY`
  (`crates/paladin-llm/src/sampling.rs:69`), `SERVED_BY_METADATA_KEY` / `SERVED_MODEL_METADATA_KEY`
  (`fallback.rs:176`, `:184`) — the metadata-key style for requested/effective effort.
- `LlmError::Refusal` (`crates/paladin-ports/src/output/llm_port.rs:551`, classified at `:946`) —
  the most recent additive struct variant; `UnsupportedParameter` follows its pattern, including
  the `llm_failure` label row.
- `live_vendor_smoke.rs` `--probe` / `--usage` modes (`crates/paladin-llm/examples/live_vendor_smoke.rs:569`,
  `:588`) — the effort-probe mode is a third mode; Kimi is already special-cased there with
  `thinking` disabled.
- `conformance.rs` (`crates/paladin-llm/src/conformance.rs`) — offline wire-body cases.
- `ReasoningAgentOptions` (`src/presets/mod.rs`) — private fields + `Default` + builder setters,
  the pattern for an option type.

### Established Patterns
- `CompatRequestParameters` (`crates/paladin-llm/src/compat/engine.rs:133`): no `Default`, every
  preset states every field; Kimi (`kimi/adapter.rs:321`) and Grok (`grok/adapter.rs:298`) write
  literals, while Qwen (`qwen/adapter.rs:407`), Ollama (`ollama/adapter.rs:240`) and
  OpenAI-compatible (`openai_compatible/adapter.rs:654`) use `CompatRequestParameters::all()`. The
  per-model profile, not the flag, now decides what is sent (ADR-0061); the reasoning **wire form**
  is vendor-wide and fits on the preset, the **levels** are per model and fit on the profile.
- `response_format` threading (Phase 26 D-27/D-28): additive `#[serde(default)]` field +
  `with_*` builder on `LlmRequest` (`llm_port.rs:1092`, `:1218`), set from the run context at
  `src/application/services/paladin/paladin_execution_service.rs:3142`.
- `#[serde(default)]` per-agent knobs on `PaladinData`
  (`crates/paladin-core/src/platform/container/paladin.rs:175-198`).
- Build-time refusal of explicit sampling values in `PaladinBuilder` (Phase 50 D-01) — the model
  for D-09.

### Integration Points
- `LlmRequest` construction in `PaladinExecutionService`:
  `src/application/services/paladin/paladin_execution_service.rs:3140`, `:3351`, `:3978` — all
  three resolve effort and fallback by D-00e precedence.
- `AgentDefinition` (`src/config/agents.rs:321`; `model` `:326`, `provider` `:335`) — new
  optional effort and fallback fields; `agent_host` resolves them.
- `FallbackLlmAdapter::from_steps` (`crates/paladin-llm/src/fallback.rs:451`) and `FallbackStep`
  (`:304`) — D-08 pre-flight.
- Request structs per adapter: Anthropic `ClaudeRequest`, DeepSeek `DeepSeekRequest`, OpenAI
  `OpenAIRequest`, compat `CompatRequest` (`compat/types.rs`), Gemini `GeminiGenerationConfig` —
  line numbers in the handoff §3.1 have drifted; re-locate them.
- Compat `build_request` (`compat/engine.rs:595`) — effort validation must happen here **before**
  sending, and must not reuse the omit-and-log path.
- `TraceEvent::NodeFinished` (`crates/paladin-core/src/platform/container/trace.rs:248`).

</code_context>

<specifics>
## Specific Ideas

- The motivating measurement (handoff §4.1): unconfigured `kimi-k3` is a `max`-effort request that
  burned its whole 16,000-token budget in ~8 minutes with no answer; `low` answered in 56 s,
  `thinking:{type:"disabled"}` in 2 s. The mdBook guidance must show the working Kimi configuration
  (effort + `max_tokens` + timeout together).
- Kimi's measured transcript (App. A): top-level `reasoning_effort: "bogus"` → HTTP 200 ignored;
  `thinking.effort: "bogus"` → HTTP 400. That pair is the template for every vendor's
  invalid-value probe (D-01) and the Kimi silent-no-op regression test.
- "Trust probes over metadata": `kimi-k3` lists `supports_thinking_type: "only"` yet accepts
  `thinking:{type:"disabled"}`. Model listings are hints; the probe decides.
- web3sec's consumers: `claude-opus-4-8`, `deepseek-v4-pro`, `kimi-k3`, set at request level on
  directly built adapters — these must be in the probe set and in the request-level tests.

</specifics>

<deferred>
## Deferred Ideas

- Live discovery of supported levels from vendor `/models` endpoints (handoff §10; a follow-up to
  Phase 50 D-05).
- Heuristic `Budget` ↔ level mapping and `Off` → lowest-level substitution (considered and rejected
  for this phase, D-07).
- Operator wire-form overrides on named vendor presets (rejected, D-10); revisit only if a vendor
  changes wire faster than releases can follow.

### Reviewed Todos (not folded)
- `2026-10-20-openai-responses-api-for-pro-tier.md` — a new `/v1/responses` wire is its own
  capability; this phase maps Chat Completions `reasoning_effort` only. When that wire lands, its
  `reasoning: {effort}` form plugs into the same `map_reasoning` and profile data.
- `2026-10-14-web3sec-single-bump-to-v0-11-0.md` — not folded here; it stays folded in Phase 50 and
  its rule still binds through 50.1 D-00i (every Phase 51 surface change is carried into MIGRATION
  §9.2/§9.8 and the CHANGELOG before the tag).
- `2026-08-13-verify-local-coverage-reproduction.md`, `2026-10-11-phase-44-review-follow-ups.md`,
  `2026-10-11-reject-removed-v0-10-battalion-config-keys.md`,
  `2026-10-13-review-content-data-orchestration-base-items.md` — keyword matches only, unrelated.

</deferred>

---

*Phase: 51-adjustable-reasoning-effort*
*Context gathered: 2026-10-21*
