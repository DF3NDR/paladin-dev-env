# Phase 51: Adjustable Reasoning Effort - Research

**Researched:** 2026-10-21
**Domain:** Rust multi-provider LLM adapter layer (hexagonal), per-model parameter policy, vendor reasoning/thinking wire forms
**Confidence:** MEDIUM-HIGH. Codebase facts HIGH (read at branch head `7b77ffef`, after Phases 50 and 50.1 sealed). Vendor facts MEDIUM (vendor docs fetched today, desk only, no live call made by this research). Two vendor facts are LOW and flagged as blocking questions (Kimi wire form, Anthropic thinking-default pairing).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
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
- **D-00k:** The handoff's §8 five-plan shape (red tests → types and plumbing → compat presets →
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

### Deferred Ideas (OUT OF SCOPE)
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
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REASON-01 | `ReasoningEffort` settable per request / agent / provider; precedence request → agent → provider → vendor default; unset is byte-identical | Types must live in `paladin-core` (layering, see Architecture). Three `LlmRequest` sites, `PaladinData`, `AgentDefinition`, `config_knob_builders!`, `LlmProviderConfig`. Node-cache fingerprint gap (Pitfall 7). `XHigh` serde rename trap (Pitfall 9). |
| REASON-02 | Wire form declared per preset (compile-enforced) and per native adapter; exact map or `UnsupportedParameter` before HTTP | `ReasoningWire` on `CompatRequestParameters` (reshape `all()` to take the wire), `map_reasoning` x4, shared `EffortSupport::resolve` admission rule. Vendor mapping tables below. |
| REASON-03 | No preset emits a field its vendor ignores; Kimi regression; wire-body case per adapter; live smoke | **Kimi wire conflict (Open Question 1)**: vendor docs and the handoff disagree; the probe must use valid values and compare `reasoning_tokens`. Conformance suite hook (`send_capturing`). |
| REASON-04 | Levels and budget bounds per model from the Phase 50 registry; per-hop fallback resolution | `ModelProfile` is `#[non_exhaustive]`, fail-closed `new()`; add an `EffortSupport` field; `intersect` rule; D-08 pre-flight in `FallbackLlmAdapter`. Seeded `reasoning` flag is NOT "can reason" (Open Question 2). |
| REASON-05 | Requested and effective effort on metadata and trace; `reasoning_tokens` filled; reasoning text typed | Text and tokens already done by Phase 50 on all adapters (verified). Metadata via `RequestNotes` + `PaladinResult.run_metadata` (free). Trace: `NodeFinished` field (about 70 occurrences) vs new variant (Open Question 5). |
| REASON-06 | `Budget(n)` validated vs vendor bounds and `max_tokens`; effort/budget/timeout guidance | Anthropic >=1024 and < `max_tokens` (cited); Qwen 1..=32768 (cited); Gemini 3.x has no budget form in seeded range. Kimi k3 timing table (handoff 4.1) shows `high` (359 s) exceeds the 300 s reasoning default. |
| REASON-07 | ADR-0062, mdBook page, MIGRATION 9.2, CHANGELOG, api-surface, 82 % floor | Drift-test pattern (`switching_page_tests`), `make api-surface-update`, semver-allowlist rows for non-`#[non_exhaustive]` structs (Pitfall 8). |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

Treated with the same authority as locked decisions. [VERIFIED: /workspace/CLAUDE.md and the three imported `.github` instruction files]

- **TDD (Red-Green-Refactor)**; coverage floor **82 % workspace line coverage** (ADR-0006, CI `coverage` job); every public API needs a doc test and rustdoc.
- **Dependencies flow inward only**: core -> nothing; ports -> core; adapters -> core + ports. Never import infrastructure from core or application. (Forces `ReasoningEffort`/`ReasoningFallback` into `paladin-core`.)
- **Ubiquitous language**: Medieval Military terms in code and docs; `Entity<T>` is deliberately generic and "node" means only a WarGraph/Campaign vertex (ADR-0060). `ReasoningEffort` is a plain unit name (Milestone 13 vocabulary).
- **Before committing a parent task**: `cargo test`, `cargo fmt --check`, `cargo clippy`, `make api-surface` (an intended surface change: `make api-surface-update` plus a CHANGELOG entry); conventional commits; stop after each major task and wait for the go-ahead.
- **Security**: `make security` (cargo-audit + cargo-deny) and `cargo clippy -- -D warnings` on new code, plus the manual credential-handling review (redact before truncation; no key in logs/`Debug`; clients never follow redirects). CodeQL is advisory only; **do not reintroduce Snyk**.
- **No `unwrap()`/`expect()`/`panic!` in library code**; return `Result`; layer-specific `thiserror` errors converted at boundaries; builders for complex construction; private fields and `#[non_exhaustive]` where the API may grow; prefer borrowing and lazy iterators.
- Follow the existing `Entity<T>`/port/service/builder patterns; `make clean-code` (fmt + clippy + check) before committing.

## Summary

The codebase is exactly as CONTEXT describes, with three corrections the planner must absorb. First, the `ModelProfile` registry exists and is the right home, but its `reasoning: bool` is **not** "this model can reason": it is "the default call returned reasoning text". Only 29 of the seeded rows are `reasoning: true`; `claude-opus-4-8` (web3sec's production model), `claude-opus-5-5`, every Sonnet/Haiku, **every OpenAI row including `o3`**, and `gpt-5.x` are all `false`. D-02 applied literally ("probe only `reasoning: true` entries; the rest get an explicit 'does not reason' state under which every level refuses") would make effort unusable on the models web3sec runs. D-02 lets the owner add ids at the checkpoint, so the fix is inside the decision, but the planner must not derive the "does not reason" state from that flag.

Second, the Kimi premise of REASON-03 is not settled. The handoff measured a top-level `reasoning_effort: "bogus"` returning HTTP 200 and called it ignored; it never sent a **valid** top-level level. Moonshot's own K3 docs, fetched today, say K3 takes the top-level `reasoning_effort` (`low|high|max`, default `max`) and that `thinking` is a K2.x-only parameter that "should not" be passed to K3, while the handoff's live run shows `thinking.effort` rejected-when-invalid and `thinking:{type:"disabled"}` working. Either the vendor changed, or it tolerates invalid top-level values while honouring valid ones. The effort probe must send valid `low` and `max` through both forms and compare `reasoning_tokens` before the Kimi wire form (and the regression test's direction) is fixed.

Third, the cheap-looking observability and cache paths have hidden edges: the node-cache fingerprint (`cache_key.rs`) does not hash effort, so A/B benchmarking (web3sec's stated use) would read cache hits from another effort; `TraceEvent::NodeFinished` has about 70 occurrences in about 23 files; and `PaladinData`, `AgentDefinition`, `CompatRequestParameters` are not `#[non_exhaustive]`, so their new fields are registered breaks (the consumer muster literal and ~20 in-tree literals change). Everything else (reasoning text, `reasoning_tokens`, per-hop models, `run_metadata` folding, streaming terminal-chunk metadata) is already built and should be reused, not rebuilt.

**Primary recommendation:** Build one shared, pure admission function (`EffortSupport::resolve(requested, fallback) -> Result<Resolved, EffortRefusal>`) in `paladin-ports` beside `ParamSupport::admits`, put `ReasoningEffort`/`ReasoningFallback` in `paladin-core` (so `PaladinData` can hold them), and have the compat engine, the four native adapters, `FallbackLlmAdapter` pre-flight, and `PaladinBuilder` all call that one function. Gate every wire form behind the live effort probe (valid-value effect, not only invalid-value rejection) and ship `ReasoningWire::Unsupported` wherever the probe does not confirm.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `ReasoningEffort`, `ReasoningFallback` value types | Domain core (`paladin-core`) | Ports (re-export on `llm_port`) | `PaladinData` (core) must hold them; core cannot import ports. Same pattern as `TokenUsage`, `Cost`. |
| `LlmRequest.reasoning`, `reasoning_fallback`, `LlmError::UnsupportedParameter` | Ports (`paladin-ports`) | — | `response_format` and `Refusal` precedents live in `llm_port.rs`. |
| Per-model level/budget support + the single admission/substitution rule | Ports (`model_profile.rs`) | Adapters (seeded data in `paladin-llm::registry`) | Pure data + pure function, shared by builder, adapters and fallback pre-flight; mirrors `ParamSupport::admits`. |
| Wire form declaration and field emission | Adapters (`paladin-llm`: compat engine presets, Anthropic, OpenAI, DeepSeek, Gemini) | — | Vendor-specific; vendor-wide wire on the preset, per-model levels on the profile. |
| Provider-level default effort/fallback | Adapter config (`*Config`, `config_knob_builders!`) | Config layer (`LlmProviderConfig`, `Layered`) | Adapter applies `request.or(config)`, like `default_max_tokens`. |
| Per-agent default + request population | Application (`PaladinData`, `PaladinBuilder`, `AgentDefinition`, `PaladinExecutionService`) | — | Resolves request → agent at the three `LlmRequest::new` sites. |
| Build-time refusal | Application (`PaladinBuilder::validate`, `agent_host`) | Ports (`model_profile`) | Phase 50 D-01 pattern; agent_host names the YAML key. |
| Fallback-chain pre-flight | Adapter decorator (`FallbackLlmAdapter`) | Ports (`model_profile` per hop) | D-08; each hop's own model via `FallbackStep.model`. |
| Requested/effective effort observability | Adapter (`RequestNotes` -> `LlmResponse.metadata`, terminal-chunk metadata) | Application (`PaladinResult.run_metadata`), Core trace (`TraceEvent`) | Metadata folding already exists; trace is the only new sink. |
| Cache correctness | Battalion engine (`cache_key.rs`) | — | Effort changes outputs; must be in the Paladin config fingerprint when set. |
| Evidence gathering | Example harness (`live_vendor_smoke --effort`) | Planning artefact (`51-EFFORT-PROBE.md`) | Human-action checkpoint, 50-17 / 50.1-07 pattern. |

## Standard Stack

No new crates. Every need is covered by what the workspace already pins. [VERIFIED: /workspace/Cargo.toml lines 21-29]

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde / serde_json | 1 / 1.0 | `ReasoningEffort` serde, request/response bodies | Already the wire layer of every adapter |
| thiserror | 2 | `LlmError::UnsupportedParameter` | `LlmError` derives it |
| async-trait | 0.1.88 | no change | `LlmPort` |
| reqwest | 0.12.4 | no change | adapters |
| log | 0.4.21 | `warn!` on substitution (target `paladin::reasoning`) | house logging |

### Supporting (tests)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| mockito | workspace dev-dep | wire-body capture (`send_capturing`) | every adapter wire case |
| tokio test-util | workspace | async tests | adapters |
| cargo-nextest 0.9.146 | installed | optional runner | — |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| New `LlmPort::reasoning_check` trait method | Fold the wire into the returned `ModelProfile` | A new trait method needs every decorator to forward it (the exact `model_profile` "must forward" hazard, ADR-0061). Folding keeps the trait unchanged. **Use folding.** |
| Per-adapter hand-written level tables | One `EffortSupport` on `ModelProfile` | Adapter-wide constants are the explicitly declined "sooner" option (roadmap). |

**Installation:** none. **Version verification:** not applicable (no packages added). `rustc 1.97.1`, `cargo 1.97.1`, `mdbook`, `cargo-deny 0.19.8`, `cargo-audit 0.22.2`, `cargo-nextest 0.9.146` present; `cargo-llvm-cov` and `cargo-semver-checks` absent locally (see Environment Availability).

## Package Legitimacy Audit

No external package is installed by this phase, so the `package-legitimacy` seam was not run.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none added) | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
 caller ──► LlmRequest { reasoning?, reasoning_fallback? }          (web3sec builds this directly)
 PaladinBuilder.reasoning_effort() / YAML agents[].reasoning_effort
        │            └─ build-time check: model_profile(model).effort.resolve()  [D-09]
        ▼
 PaladinExecutionService  (3 LlmRequest::new sites: loop, retry/second path, stream)
        │  request ← agent default (PaladinData)            [request wins; unset stays None]
        ▼
 [Cadence / Pricing decorators]  (forward model_profile, unchanged)
        ▼
 FallbackLlmAdapter ── pre-flight EVERY hop: hop.model_profile(hop_model).effort.resolve()  [D-08]
        │                 any hop refuses ──► UnsupportedParameter{hop provider, hop model} (no HTTP)
        ▼  per hop: request_for_hop() rewrites model
 Adapter (compat engine preset | Anthropic | OpenAI | DeepSeek | Gemini)
        │  effective = request.reasoning.or(config.reasoning_effort)      [provider default]
        │  profile   = registry.profile(model)  (override > seeded > adapter fallback)
        │  resolve(profile.effort, effective, fallback)
        ├─ Err(refusal) ─► LlmError::UnsupportedParameter (Permanent) ── before any HTTP
        └─ Ok(Resolved{effective, substituted})
              ├─ warn!(target "paladin::reasoning") when substituted
              ├─ wire fields:  map per ReasoningWire / map_reasoning()
              └─ RequestNotes{ reasoning_requested, reasoning_effective } ─► response.metadata
                                                     └─► terminal stream chunk metadata
 response ─► PaladinExecutionService.merge_run_metadata ─► PaladinResult.run_metadata (paladin.* last-value)
          └► trace: NodeFinished{ reasoning_requested?, reasoning_effective? }   [Open Question 5]
```

### Recommended Project Structure (changes only)
```
crates/paladin-core/src/platform/container/
├── reasoning.rs            # NEW: ReasoningEffort, ReasoningFallback (+Display/FromStr-ish helpers)
├── paladin.rs              # PaladinData: +reasoning_effort, +reasoning_fallback (serde default, skip if None)
└── trace.rs                # NodeFinished: +2 optional string fields (or new variant; OQ 5)
crates/paladin-ports/src/output/
├── model_profile.rs        # +EffortSupport, EffortLevels, Resolved, EffortRefusal, resolve(); ModelProfile.effort; intersect
└── llm_port.rs             # LlmRequest.reasoning/.reasoning_fallback + with_*; LlmError::UnsupportedParameter; re-exports
crates/paladin-llm/src/
├── reasoning.rs            # NEW (crate-private+pub keys): metadata keys, warn-once log, map helpers, RequestNotes extension
├── compat/engine.rs        # ReasoningWire; CompatRequestParameters.reasoning; build_request; with_default_reasoning
├── compat/types.rs         # CompatRequest: reasoning_effort / thinking / enable_thinking / thinking_budget (all skip-if-None)
├── {anthropic,openai,deepseek,gemini}/adapter.rs   # private map_reasoning + request struct fields
├── {kimi,qwen,grok,ollama,openai_compatible}/adapter.rs  # state the wire; self-hosted get with_reasoning_wire
├── registry/{mod,seeded}.rs # ModelOverride effort fields; seeded effort data AFTER the probe
├── fallback.rs             # D-08 pre-flight
└── conformance.rs          # new suite cases (all nine adapters)
crates/paladin-battalion/src/{llm_failure.rs,llm_decision.rs,engine/cache_key.rs}
src/{config/agents.rs, infrastructure/web/agent_host.rs, application/services/paladin/{paladin_builder,paladin_execution_service}.rs}
crates/paladin-llm/examples/live_vendor_smoke.rs  # --effort mode
docs/src/user-guides/reasoning-effort.md          # NEW page + drift test
.planning/decisions/0062-reasoning-effort-fail-loud-policy.md
```

### Pattern 1: One admission rule (the `ParamSupport::admits` pattern)
**What:** `EffortSupport` is data; `resolve` is the only code that decides accept / substitute / refuse. Builder, adapters, fallback pre-flight and the docs drift test all call it.
**When to use:** every place that asks "can this model take this effort".
**Example (recommended shape; names are discretionary):**
```rust
// paladin-ports/src/output/model_profile.rs  [recommended design, not existing code]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EffortSupport {
    /// Fail closed (D-03): nothing measured. Every explicit effort refuses.
    #[default]
    Unmeasured,
    /// Measured: the model does not reason (D-02). Off is an exact no-op; everything else refuses.
    NoReasoning,
    /// Measured native levels. Only NATIVE levels are listed, so "silently coerced by the
    /// vendor" (DeepSeek medium->high, Grok-4.5 xhigh->high) is simply "not in the set".
    Levels(EffortLevels),
}
// EffortLevels { levels: Vec<ReasoningEffort> /*Minimal..Max ascending, validated*/,
//                off: bool, auto: bool, budget: Option<(u32,u32)>,
//                thinking_default_on: Option<bool> /* Anthropic 4.6-4.8 are OFF until adaptive */ }
impl EffortSupport {
    pub fn resolve(&self, requested: ReasoningEffort, fallback: ReasoningFallback)
        -> Result<Resolved, EffortRefusal>;
}
```
`resolve` rules (D-03, D-06, D-07): `Unmeasured` -> refuse (reason names the override key); `NoReasoning` -> `Off` => `Resolved::noop`, else refuse; `Levels`: exact level in set -> ok; `Off`/`Auto` -> ok only if the flag is set; `Budget(n)` -> ok only if `budget` is `Some((lo,hi))` and `lo<=n<=hi`, never converted to a level; a level not in the set -> with `NearestLower`/`NearestHigher` pick the nearest by rank in that direction (refuse when none; never flip), otherwise refuse; substitution never applies to `Off`, `Auto`, `Budget`, `Unmeasured` (D-07).

### Pattern 2: Provider default via adapter, per-request wins (the `default_max_tokens` pattern)
`CompatEngine::with_default_max_tokens(config.max_tokens)` is the precedent: `params.max_tokens.or(self.default_max_tokens)`. Add `with_default_reasoning(effort, fallback)`; effective = `request.reasoning.or(config.reasoning_effort)`. Unset at every level => no field, no metadata, no profile lookup that can fail (D-00i).

### Pattern 3: Fold the wire into the returned profile
Adapters' `model_profile()` return `registry.profile(model)` with the effort support forced to a refusing state when the adapter's wire is `Unsupported` (generic compat, Ollama, Grok-until-probed). That lets `PaladinBuilder` and `FallbackLlmAdapter` (which only see `LlmPort::model_profile`) refuse correctly with no trait change.

### Pattern 4: Metadata through `RequestNotes`
`RequestNotes` (`sampling.rs:371`) is the one struct all nine adapters already fill and flush into `LlmResponse.metadata` and the terminal stream chunk. Add `reasoning_requested`/`reasoning_effective: Option<String>`. Keys (`pub const`, `paladin.` prefix, beside `SERVED_BY_METADATA_KEY`): `paladin.reasoning_requested`, `paladin.reasoning_effective`. Stamp **only when a request or provider effort was set** so unset responses stay byte-identical.

### Anti-Patterns to Avoid
- **Deriving "does not reason" from `ModelProfile.reasoning`** (Pitfall 1).
- **Reusing `sampling::apply` / `OmissionLog` for effort**: omit-and-log is exactly the silent no-op D-00f forbids.
- **Per-adapter level constants** (declined by the roadmap).
- **Naming the YAML/JSON level `x_high`**: derive default produces it (Pitfall 9).
- **Resolving the provider default only at construction**: it must be re-resolved per request model (Pitfall 6).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Accept/substitute/refuse logic in 6 places | Six `match`es | One `EffortSupport::resolve` | The `admits` precedent exists precisely to stop drift between build-time check and wire filter. |
| Warn-once per (provider, model, effort) | New `HashSet`+`Mutex` | Copy `OmissionLog`/`FormatLog` (`sampling.rs`) | Bounded, already reviewed (T-38-07 capacity rule). |
| Config layering for `llm.<provider>.reasoning_effort` | New env/YAML merge | `config::layers::Layered` + `LlmProviderConfig` field + `config_knob_builders!` | PARITY-03 machinery; `max_tokens` is the template. |
| Error-body redaction | New truncation | `redaction::diagnostic_excerpt` (redact THEN bound) | Security rule in `security.instructions.md`. `UnsupportedParameter` is built before any HTTP so carries no body, but a vendor 400 that rejects a field still goes through the existing mapper. |
| Drift-checked docs table | Hand-maintained markdown | Mirror `registry::switching_page_tests` | Page is parsed against the seeded data in a unit test. |
| Streaming metadata | New chunk field | `StreamingResponse.metadata` (terminal chunk) | Already exists (`with_metadata`). |
| Reasoning text / `reasoning_tokens` | Anything | Already done (Phase 50 D-11, Phase 51 verified below) | Do not add a second surface (D-00g). |
| Mock transport | New HTTP stub | `conformance::send_capturing`, `test_support` capturing mock | Captures the real serialized body. |

**Key insight:** every cross-cutting need (metadata, streaming, config layering, docs drift, redaction, warn-once) has a Phase 50 mechanism; this phase's genuinely new code is the types, the `resolve` rule, the four wire mappers, the pre-flight and the data.

## Runtime State Inventory

Not a rename/refactor/migration phase. Omitted per template. (Stored-state note: `PaladinData` is persisted in Citadel snapshots; the two new fields must be `#[serde(default, skip_serializing_if = "Option::is_none")]` so unset snapshots stay byte-identical and old snapshots load; see Pitfall 8.)

## Common Pitfalls

### Pitfall 1: `ModelProfile.reasoning` is "returned reasoning text", not "can reason" (BLOCKING for D-02)
**What goes wrong:** D-02 probes only `reasoning: true` entries and gives the rest a "does not reason" state (Off no-op, all else refuses).
**Why it happens:** the seeded rule (`seeded.rs` module docs) sets `reasoning` when "the default call returned reasoning text". Verified count: `reasoning: true` only for `claude-fable-5`, `claude-opus-5`, `deepseek-flash`, `deepseek-v4-pro`, `kimi-k2.6`, `kimi-k3`, 9 Qwen ids, 5 Grok ids, 9 Gemini ids. Every OpenAI row (including `o3`, `gpt-5.x`, `gpt-6.x`) is `false` ("Chat Completions never returns reasoning text"); Anthropic `claude-opus-4-8`, `claude-opus-4-7`, `claude-opus-4-6`, `claude-sonnet-4-6`, `claude-opus-5-5`, `claude-sonnet-5`, `claude-sonnet-5-5`, `claude-haiku-5-5`, `claude-fable-5-1` are `false` (Anthropic's `display` defaults to `omitted`, so thinking text is empty even on always-thinking models). [VERIFIED: grep of `registry/seeded.rs`; CITED: platform.claude.com/docs/en/build-with-claude/thinking]
**How to avoid:** use the D-02 owner checkpoint to **add** every seeded Anthropic and OpenAI id plus web3sec's three (`claude-opus-4-8`, `deepseek-v4-pro`, `kimi-k3`); remove only ids that are clearly non-reasoning (`gpt-4o-mini`, `grok-4.20-non-reasoning`, `qwen-max`/`qwen-plus` may be reasoning hybrids, leave to the probe). Set `EffortSupport::NoReasoning` **only** where the probe measured it (no reasoning at any level and/or vendor rejects the field for that model); every other unprobed entry stays `Unmeasured` (refuse, D-03). Raise this to the owner explicitly at the checkpoint.
**Warning signs:** candidate list drafted mechanically from `.reasoning` contains no OpenAI id.

### Pitfall 2: Kimi wire form is contradicted by the vendor docs
**What goes wrong:** REASON-03's regression ("Kimi never emits top-level `reasoning_effort`, sends `thinking:{type,effort}`") may pin the wrong behaviour.
**Why it happens:** handoff App. A only proves top-level `"bogus"` returns 200 (same as an unknown key, `reasoning_tokens` 42 vs 40). It never sent a valid top-level level. Moonshot's K3 pages (fetched 2026-10-21) say: "K3 configures reasoning effort with the top-level `reasoning_effort` request field", values `low|high|max`, default `max`; `thinking` is "a K2.x-only request parameter" that should not be passed to K3; "K3 always reasons". [CITED: platform.kimi.ai/docs/guide/use-kimi-k3-model, /use-thinking-models, /api/models-overview]. The handoff's live data (2026-10-13) says `thinking.effort:"bogus"` -> 400 and `thinking:{type:"disabled"}` -> 2 s with no reasoning.
**How to avoid:** the probe sends valid `low` and `max` through **both** forms on `kimi-k3` with a small output cap (e.g. 1024) and records `reasoning_tokens`; the wire form that makes `low` << `max` is the form (D-04 plus effect-demonstrated). If both work, prefer the vendor-documented form unless `Off` is wanted. The "never emits X" regression test is written **after** the probe decides which form is dead. Until then do not merge a Kimi test asserting a direction.

### Pitfall 3: Anthropic effort without thinking enabled yields no reasoning on default-off models
**What goes wrong:** `High` on `claude-opus-4-8` sends `output_config.effort:"high"` (the default, so identical behaviour) and the model does not think at all.
**Why it happens:** per the vendor table, thinking is **Off by default** on Opus 4.8/4.7/4.6 and Sonnet 4.6, always on for Fable/Opus 5.5/Haiku 5.5/Opus 5/Sonnet 5/5.5. Effort "works whether or not thinking is enabled" and shapes all output tokens. [CITED: platform.claude.com/docs/en/build-with-claude/thinking-troubleshooting, /effort]
**How to avoid:** the profile carries `thinking_default_on`; for a default-off model a level maps to `thinking:{type:"adaptive"}` **plus** `output_config.effort`, `Auto` maps to `thinking:{type:"adaptive"}`, `Off` maps to omit. This pairing is a design inference ([ASSUMED], Open Question 3): the probe must record `thinking_tokens` with effort alone vs paired. REASON-03's "higher level reports more `reasoning_tokens`" cannot pass on a default-off model without it.

### Pitfall 4: Anthropic `Off` and `Budget` are model- and effort-dependent
Verified rules [CITED: thinking-troubleshooting, effort, extended-thinking]: `thinking:{type:"disabled"}` is a 400 on Fable 5.1/5, Opus 5.5 (always on); accepted on Opus 5 and Haiku 5.5 only at effort `high` or below (400 with `xhigh`/`max`); a 400 on Sonnet 5.5 (which takes `between_tools` instead, `high` or below); accepted on Opus 4.6/4.7/4.8, Sonnet 4.6 and Sonnet 5. `thinking:{type:"enabled",budget_tokens}`: 400 on 4.7 and later; deprecated-but-working only on Opus 4.6 and Sonnet 4.6. `budget_tokens` >= 1024 and < `max_tokens`. So `Off` on an always-on model **refuses** (D-00f); `Off` on Opus 5 / Haiku 5.5 is valid only while the effective effort is `high` or below (a request carries one effort, so this only bites when the vendor default or a hop changes it; the probe records it); and `Budget` is only ever valid on 4.6-generation ids (D-00h: never send the deprecated form to a model that rejects it). Do not send `between_tools` (not in the `ReasoningEffort` vocabulary; `Off` on Sonnet 5.5 refuses unless the probe shows `disabled` works).

### Pitfall 5: Reasoning text visibility is not an effort control
Anthropic's `thinking.display` defaults to `"omitted"` on current models (empty `thinking` text, signature only). Adding `thinking:{type:"adaptive"}` for `Auto` must not silently add `display:"summarized"`; that is a separate feature (D-00g says fill the typed surface when text is received, not to request it). `display` is invalid with `type:"disabled"`/`between_tools`. Gemini returns thought parts only with `includeThoughts`, which the adapter does not send; also out of scope. Flag both in the mdBook page.

### Pitfall 6: A provider-level default applies to every model on that adapter
`llm.openai.reasoning_effort: high` reaches `gpt-4o-mini` through the same adapter and, under D-03, **refuses** every request for it. That is consistent with D-00f but surprising. Resolve per request against the request's own model (never once at construction), name the YAML key in the refusal, document that a provider-wide default should be set only where every served model supports it, and note the per-model default (`models.<id>.reasoning_effort`) is a natural but **undecided** follow-up (Open Question 6). Build-time (`agent_host`) can check the agent's model against the block's value and name `llm.<provider>.reasoning_effort`, the way `name_temperature_key` names the temperature key.

### Pitfall 7: The node cache ignores effort
`engine/cache_key.rs::push_paladin_config` hashes model, system prompt, temperature, max loops, stop words (FT-FR-20). Effort changes outputs (handoff 4.1: 581/582/261-token answers). Without a fix, an A/B run at `low` then `high` serves the `low` answer from the cache. **How to avoid:** append effort and fallback bytes **only when `Some`** (zero bytes when unset, so existing keys and `NODE_CACHE_KEY_VERSION` stay valid), with a test that a changed effort changes the key and an unset one does not. Provider-level defaults live outside `PaladinData` and are not in the key; document that a changed provider default needs a new cache namespace or version bump.

### Pitfall 8: Several touched types are not `#[non_exhaustive]`
`PaladinData` (20 full literals in-tree plus `tests/consumer_surface_muster.rs:206`, "thirteen fields, all named here"), `AgentDefinition` (11 literals), `CompatRequestParameters` (pub fields, `all()` signature), `TraceEvent::NodeFinished` (about 70 occurrences in about 23 files: battalion engine, storage, eval, CLI, telemetry sinks, tests). Each is a registered break (MIGRATION 9.2 row, `.cargo/semver-checks-allowlist.toml` entry measured with cargo-semver-checks, muster updated), not a silent one (D-00b). `LlmRequest`, `LlmError`, `ModelProfile`, `StreamingResponse`, the nine `*Config` are `#[non_exhaustive]` and additive. `LlmResponse` needs no change (use `metadata`). Make the two new `PaladinData` fields `#[serde(default, skip_serializing_if = "Option::is_none")]`. Update each `*Config`'s hand-written `Debug` (they list fields explicitly; a forgotten field is invisible, not a compile error).

### Pitfall 9: serde `snake_case` renders `XHigh` as `x_high`
`#[serde(rename_all = "snake_case")]` (the handoff's snippet) serialises `XHigh` as `"x_high"`; vendors and YAML use `xhigh`. Add `#[serde(rename = "xhigh")]` on that variant and pin all nine wire/YAML spellings in a round-trip test. `Budget(u32)` serialises externally tagged as `{"budget": 2048}` (YAML `reasoning_effort: {budget: 2048}`); decide and document, and give `ReasoningEffort` a `Display` (`low`, `budget:2048`) for metadata and log text. [VERIFIED by serde's documented rename algorithm; confirm with a unit test in plan 1.]

### Pitfall 10: Retry loops and the fallback hop rule must never see the refusal as retryable
`UnsupportedParameter` is `Permanent` (the `transience()` match has no wildcard, so the new variant is a compile error until given an arm; add arms in `typed_origin` and the `llm_decision` label too). Raise it from `build_request` **before** `call_api_with_retry`/`execute_with_retry`. A chain's `after_failure` short-circuits on `Permanent`, which is why D-08's pre-flight is needed: otherwise hop 2's refusal surfaces only during an outage of hop 1. `classify_fetch_failure` (compat engine) has explicit arms plus a wildcard: add an explicit arm.

### Pitfall 11: Streaming and buffered paths must share one resolution
Each bespoke adapter has a separate `generate_stream`; they all call the same `build_request` today. Keep it that way: resolve inside `build_request` and return the notes, so stream and buffered cannot diverge (Anthropic currently *replaces* `response.metadata` with `notes.into_metadata()`; make sure effort keys survive the fallback `served_by` stamping).

### Pitfall 12: Effort, output budget and timeout must be configured together
Handoff 4.1 (Kimi k3, 16,000 cap): `low` 56 s, `high` 359 s, `max` 473 s with an empty answer. The reasoning default timeout is 300 s (`REASONING_DEFAULT_TIMEOUT_SECONDS`), so **`high` times out by default**; Kimi's non-reasoning compiled default is 60 s. Anthropic recommends `max_tokens` around 64k for `xhigh|max`, and `budget_tokens < max_tokens`; Gemini `maxOutputTokens` and OpenAI `max_completion_tokens` include thinking. The mdBook guidance must show one working `llm.kimi` block (effort, `max_tokens`, `timeout_seconds`) and say `EmptyCompletion` (finish `length`) is the backstop, not a validation.

### Pitfall 13: Sampling parameters can change meaning with effort
OpenAI: "When reasoning effort is not `none`, remove `temperature`, `top_p`, and `top_logprobs`" (GPT-6 models). [CITED: developers.openai.com/api/docs/guides/deployment-checklist]. Anthropic with thinking requires sampling the profile may already refuse. The seeded sampling rows were probed at the vendor-default effort. The effort probe should include one default-sampling call per level to detect a 400; if one appears, record it and let the profile's sampling fields stay authoritative (do not hard-code).

### Pitfall 14: "Red first" cannot precede the types in Rust
The handoff's "plan 1 red tests, plan 2 types" cannot compile. Use the Phase 50 plan-01 pattern: land inert types/field/builders first, then tests marked `#[ignore = "fixed by 51-NN"]` with a `51-RED-EVIDENCE.md` of the `--ignored` run, un-ignored by the plan that fixes each; the unset-golden-body guards are committed **before** any adapter edit and pass before and after.

### Pitfall 15: Probe cost and duration
`max` on Kimi ran 8 minutes; the harness has `PROBE_TIMEOUT_SECS = 120`. Use a small output cap (<= 2048) so `length` ends the call quickly, accept `finish=length` as data (it is the `reasoning_tokens` signal), run one model at a time per vendor, print numbers only (no content), and keep `Authorization`/key values out of the transcript (security.instructions.md, 50.1 scrub procedure).

### Pitfall 16: Naming collision
`LlmRequest.reasoning` (effort, mandated by REASON-01) vs `LlmResponse.reasoning` (text), `TokenUsage::with_reasoning(tokens)` and `LlmResponse::with_reasoning(text)` vs `LlmRequest::with_reasoning(effort)`. Keep the mandated names; document the distinction in rustdoc on all three.

## Code Examples

### Effort level wire spellings (from vendor docs) — table-driven test seed
```rust
// Source: vendor docs cited in "Vendor wire forms"; plan 1 round-trip test
let cases = [("off","Off"),("auto","Auto"),("minimal","Minimal"),("low","Low"),
             ("medium","Medium"),("high","High"),("xhigh","XHigh"),("max","Max")];
// And: serde_json::to_value(ReasoningEffort::Budget(2048)) == json!({"budget": 2048})
```

### Registering a config knob (pattern to copy)
```rust
// Source: crates/paladin-llm/src/registry/mod.rs:182 (config_knob_builders!)
/// Set the provider-level default reasoning effort (resolved per request model).
#[must_use]
pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
    self.reasoning_effort = Some(effort);
    self
}
// with_reasoning_fallback likewise; with_reasoning_wire only on OllamaConfig / OpenAiCompatibleConfig (D-10).
```

### The compat declaration stays compile-enforced
```rust
// Source: compat/engine.rs:133 + kimi/adapter.rs:321 (every field stated)
// Reshape the convenience constructor so it cannot bypass REASON-02:
impl CompatRequestParameters {
    pub const fn all(reasoning: ReasoningWire) -> Self { /* five bools true, reasoning stated */ }
}
// Kimi/Grok keep literals and add `reasoning: ReasoningWire::<...>`.
```
`ReasoningWire` must stay `Copy + Eq` (the struct derives both) and `Serialize/Deserialize` for the two self-hosted YAML keys; carry **no** allowed-sets (levels come from the profile). Recommended variants: `Unsupported`, `TopLevelEffort` (`reasoning_effort`), `ThinkingObject` (`thinking:{type,effort}`), `EnableThinking` (`enable_thinking` + `thinking_budget`). DeepSeek is a bespoke adapter, so the handoff's `ThinkingToggle` variant is not needed.

### Mapping rules per wire (what `map` must emit)
| Wire | `Off` | `Auto` | level `L` | `Budget(n)` |
|------|-------|--------|-----------|-------------|
| `TopLevelEffort` | `reasoning_effort:"none"` iff profile `off` | omit (vendor default) | `reasoning_effort:"L"` | refuse |
| `ThinkingObject` | `thinking:{type:"disabled"}` iff profile `off` | `thinking:{type:"enabled"}` | `thinking:{type:"enabled",effort:"L"}` | refuse |
| `EnableThinking` | `enable_thinking:false` iff `off` | `enable_thinking:true` | refuse (Qwen has no levels) | `enable_thinking:true, thinking_budget:n` (1..=32768, cited) |
| `Unsupported` | refuse | refuse | refuse | refuse |

Note `Auto` on `TopLevelEffort` emits nothing: record effective `auto` in metadata, send no field (so it is byte-identical to unset on the wire but still audited).

## Vendor wire forms (re-verification of the handoff table, 2026-10-21)

All rows below are desk research from vendor docs fetched today; **none has been confirmed by a live probe**, and per D-01/D-04 none is encoded into a profile or a non-`Unsupported` wire until the effort probe confirms it. Seeded model ids are from `registry/seeded.rs`.

| Vendor | Wire (documented) | Documented levels / gating | Verification verdict vs handoff |
|--------|-------------------|----------------------------|---------------------------------|
| Anthropic | `output_config.effort` (GA, no beta header) and `thinking:{type:"adaptive"\|"disabled"\|"between_tools"(Sonnet 5.5 only)}`; `thinking:{type:"enabled",budget_tokens}` only on 4.6-generation | `max` on every effort-capable id; `xhigh` on Fable 5.1/5, Opus 5.5/5/4.8/4.7, Sonnet 5.5/5, Haiku 5.5 but **not** Opus 4.6 or Sonnet 4.6; default `high`, `medium` on Opus 5.5 and Haiku 5.5. Thinking default Off on Opus 4.8/4.7/4.6, Sonnet 4.6. [CITED: platform.claude.com/docs/en/build-with-claude/effort; /thinking-troubleshooting] | Handoff row confirmed and sharpened (per-model `Off` rules, default-off models, Haiku 5.5 added). Per-message effort beta stays out of scope. |
| OpenAI (Chat Completions) | top-level `reasoning_effort` | "model-dependent; can include none, minimal, low, medium, high, xhigh, max"; GPT-6 Astra and GPT-6.1 Sol do **not** accept `none`, GPT-6.1 Sol not `minimal`, default `medium`; `none` on Astra is HTTP 400; with effort != none remove `temperature`/`top_p`. [CITED: developers.openai.com/api/docs/guides/reasoning; /deployment-checklist] | Confirmed. Per-model sets for the 18 seeded ids are **not** documented: probe-only. |
| DeepSeek | top-level `reasoning_effort` plus `thinking:{type:"enabled"\|"disabled"}` (OpenAI SDK needs `thinking` in `extra_body`) | `low\|high\|max`; coercion table minimal->low, medium->high, xhigh->high, ultra->max; thinking on by default at `high`; sampling params "no effect". Seeded ids `deepseek-flash`, `deepseek-v4-pro`. [CITED: api-docs.deepseek.com/guides/thinking_mode] | Confirmed. Native levels are exactly `{low, high, max}`; every other level refuses (this is the whole "known to coerce" list). |
| xAI Grok | `reasoning.effort` on `/v1/responses`; `/v1/chat/completions` field **not documented** | grok-4.7/4.6: low\|medium\|high\|xhigh; grok-4.5: low\|medium\|high and `xhigh` "treated as high"; default `high`; "reasoning cannot be disabled"; `grok-4.3`, `grok-4.20-reasoning` not listed. [CITED: docs.x.ai/docs/guides/reasoning, /api-reference] | Handoff's "chat field unconfirmed" stands. Per D-04: `ReasoningWire::Unsupported` unless the probe's invalid-value call is rejected **and** a valid value shows an effect. |
| Gemini (native `generateContent`) | `generationConfig.thinkingConfig.thinkingLevel` (3.x+) / `thinkingBudget` (2.5); `includeThoughts` for summaries | The current docs page describes the **Interactions API** (`thinking_level`): argon/3.8-flash/3.1-pro-preview: low\|medium\|high; 3.6-flash/3-flash-preview/3.5-flash-lite: minimal\|low\|medium\|high; gemini-3-pro-preview low\|high. The `generateContent` REST shape could not be fetched; `thinkingBudget` under `generationConfig.thinkingConfig` is corroborated by a Google REST example surfaced in search. [CITED: ai.google.dev/gemini-api/docs/thinking; ai.google.dev/gemini-api/docs/openai] | Seeded Gemini ids are all 3.x+, so no `thinkingBudget` is expected (`Budget` refuses unless the probe shows it works). REST casing of `thinkingLevel` values is [ASSUMED]; the probe decides, else `Unsupported` (D-04). "Cannot disable on 3.x/2.5 Pro" [CITED: openai-compat page]. |
| Qwen / DashScope | `enable_thinking` (bool) + `thinking_budget` (int) | `thinking_budget` 1..=32768; hybrid models default per series (3.5-3.8 thinking on, `qwen-plus` off); thinking-only models cannot be disabled; some models are stream-only with thinking (`parameter.enable_thinking only support stream call`); no levels. [CITED: alibabacloud.com/help/en/model-studio/deep-thinking] | Confirmed; `Budget` is the only quantitative control. Seeded commercial ids are non-stream-capable per docs. |
| Ollama (`/v1`) | `reasoning_effort` or `reasoning:{effort}` on `/v1/chat/completions` | "model-defined names and compatibility aliases"; discover via `/api/show` [CITED: docs.ollama.com/api/openai-compatibility] | Handoff said unconfirmed; docs now list it. Still `Unsupported` by default (D-10, operator-declared), no daemon to probe. |
| Moonshot Kimi | **CONFLICT**: docs say K3 top-level `reasoning_effort`; handoff measured `thinking.effort` | `low\|high\|max`, default `max`; docs: K3 "always reasons", `thinking` K2.x-only. [CITED: platform.kimi.ai/docs/guide/use-kimi-k3-model, /use-thinking-models, /api/models-overview] | Open Question 1. |

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Anthropic `thinking:{type:"enabled",budget_tokens}` | `thinking:{type:"adaptive"}` + `output_config.effort` | deprecated 4.6, 400 on 4.7+ | `Budget` is a legacy-only control on Anthropic. |
| OpenAI `minimal`/`none` as universal | model-dependent sets incl. `xhigh`/`max` | GPT-6 generation | per-model data is mandatory. |
| Gemini `thinkingBudget` | `thinkingLevel`; docs now Interactions-API-first | 3.x | Native adapter must confirm REST form by probe. |
| Fixed-budget everywhere | Effort levels as the common control | 2026 | `Budget` remains the only Qwen control. |

**Deprecated/outdated:** the handoff's Anthropic row "type:disabled rejected on some current models" is now precise per model; its Kimi row is contradicted by the vendor docs.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Anthropic default-off models need `thinking:{type:"adaptive"}` alongside `output_config.effort` for a level to produce reasoning tokens | Pitfall 3 | Levels on Opus 4.8 would be silent no-ops for reasoning; REASON-03 fails. Probe resolves. |
| A2 | Gemini native REST accepts lowercase `thinkingLevel` strings and `thinkingBudget` only on 2.5 | Vendor table | Wrong spelling => 400 or silent ignore; probe resolves; else `Unsupported` per D-04. |
| A3 | Kimi k3 honours a valid top-level `reasoning_effort` (vendor docs) AND/OR `thinking.effort` (handoff) | Pitfall 2 | Wrong form = the exact silent no-op the phase exists to prevent. |
| A4 | An invalid string (`"bogus"`) is a valid "is the field parsed" probe for every vendor | Probe design | DeepSeek may coerce unknown strings (docs map unknowns like `ultra`); use an out-of-vocabulary string **and** a valid-value effect check. |
| A5 | `ModelProfile` field/type names (`EffortSupport`, `EffortLevels`, `thinking_default_on`) | Pattern 1 | Cosmetic; discretionary per CONTEXT. |
| A6 | Adding a fifth `reason` field to `UnsupportedParameter` is acceptable to the owner | Open Question 4 | If the roadmap's four-field shape is binding, the "why" must be packed into `value`/`param`. |
| A7 | Provider-level default should refuse (not skip) on a model that cannot honour it | Pitfall 6 | Operators with mixed-model providers get loud failures instead of silent omission. |
| A8 | serde `snake_case` yields `x_high` for `XHigh` | Pitfall 9 | None if tested in plan 1. |
| A9 | `cargo-semver-checks` can be installed/run by the executor (absent locally) | Environment | Allowlist rows cannot be "measured" (D-00b) without it. |

## Open Questions (RESOLVED — see 51-PLAN resolutions: OQ1→51-11 checkpoint, OQ2→51-10 checkpoint, OQ3→A1-51-05, OQ4→A1-51-01, OQ5→A1-51-09, OQ6→COVERAGE.md, OQ7→A1-51-08)

1. **Which Kimi k3 wire form is real?** (blocks REASON-03's regression test and the Kimi preset.)
   - Known: vendor docs = top-level `reasoning_effort`; handoff live = `thinking.effort` validated (400 on bogus), `thinking:{type:"disabled"}` -> no reasoning.
   - Unclear: whether top-level honours a valid value; whether the vendor changed since 2026-10-13.
   - Recommendation: effort probe sends `low` and `max` through both forms (cap <= 2048) and records `reasoning_tokens`; write the Kimi test after. Keys are present in the devcontainer (all seven vendor variables set; never print values); per the owner's note a live run spends money and needs the owner's go-ahead, which a subagent cannot give.

2. **How are the probe candidates and the "does not reason" state chosen (D-02)?**
   - Recommendation: candidate list = all seeded ids for vendors whose wire form is a candidate, owner strikes obvious non-reasoners; `NoReasoning` only when measured; see Pitfall 1. web3sec's `claude-opus-4-8`, `deepseek-v4-pro`, `kimi-k3` mandatory.

3. **Should an Anthropic level on a default-off model also enable adaptive thinking?** Recommendation yes (profile bit, probe-confirmed); alternative "levels send only `output_config.effort`" makes REASON-03's token-increase check meaningless on Opus 4.8.

4. **`UnsupportedParameter` carries `{param, value, provider, model}` per the roadmap; D-03 needs the error text to name the `llm.<provider>.models.<id>` override and explain why.** Recommendation: add a `reason: String` field (the variant is `#[non_exhaustive]`, `Refusal.detail` is the precedent) and a constructor `LlmError::unsupported_parameter(...)`; owner to confirm the extra field.

5. **Trace carrier (D-11).** `NodeFinished` field (locked by CONTEXT, `cost` precedent) touches about 70 occurrences and needs the engine to read the effort out of the Paladin result's `run_metadata`; a new `TraceEvent` variant emitted via `current_trace_emitter()` (as `FallbackHop` is) is smaller but a fourteenth variant changes the "thirteen-variant" tests and exhaustive matches (otel_sink, herald_sink, run_trace, events). Recommendation: follow CONTEXT (`NodeFinished`, two `Option<String>` fields, `skip_serializing_if`), do it in its own plan, size the site count in the plan, and fall back to the variant only if the engine threading proves disproportionate.

6. **Provider default vs per-model default.** Not decided; Pitfall 6. Recommendation: ship the decided provider-wide default only; record per-model default as a follow-up.

7. **CLI agent YAML (`src/application/cli/config/paladin_config.rs`)** has its own agent options; CONTEXT names only `AgentDefinition`. Recommendation: out of scope; mention in docs.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test | yes | rustc/cargo 1.97.1 | — |
| nightly-2026-09-20 | `make api-surface` | yes | pinned | — |
| mdBook | docs build | yes | /usr/local/cargo/bin/mdbook | — |
| cargo-deny / cargo-audit | `make security` | yes | 0.19.8 / 0.22.2 | — |
| cargo-nextest | optional | yes | 0.9.146 | `cargo test` |
| cargo-llvm-cov | 82 % coverage gate | **no** (CI job) | — | rely on the CI `coverage` job; locally measure with `cargo test` only |
| cargo-semver-checks | measured allowlist rows (D-00b) | **no** | — | install (`cargo install cargo-semver-checks --locked`, version 0.50.0 used in earlier rows) or run in CI; do not hand-write "measured" claims |
| Docker | integration profile | no | — | not needed for this phase |
| Vendor keys (OpenAI, Anthropic, DeepSeek, Moonshot, DashScope, xAI, Gemini) | live effort probe | yes (7 env vars set in this container; values never printed) | — | the checkpoint can run in-container with the owner's go-ahead; Ollama and OpenAI-compatible not configured |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** `cargo-llvm-cov`, `cargo-semver-checks` (above).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (unit, doc tests, `#[tokio::test]`, mockito), shell guards, compile-time muster |
| Config file | `Cargo.toml` `[[test]]`/`[[example]]` entries (`live_vendor_smoke` needs the nine adapter features; muster `required-features`) |
| Quick run command | `cargo test --offline -p paladin-llm --all-features --lib <module>` (touched adapter), `cargo test --offline -p paladin-ports --lib model_profile` (verified working, 10 tests, ~0 s warm) |
| Full suite command | `cargo test --offline --workspace --all-features` plus `make clean-code`, `make check-gates`, `make api-surface`, `make check-api-examples`, `make check-migration-allowlist`, `make check-consumer-surface`, `make security` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REASON-01 | serde spellings incl. `xhigh`, `{budget:n}`; defaults None; `with_reasoning` | unit/doc | `cargo test --offline -p paladin-core --lib reasoning` | ❌ Wave 0 |
| REASON-01 | precedence request > agent > provider; execution service puts agent effort on all 3 request sites | unit (recording mock) | `cargo test --offline -p paladin-ai --lib paladin_execution_service` | partial (recording mock exists) |
| REASON-01 | unset wire body byte-identical per adapter | mockito golden (committed BEFORE edits) | `cargo test --offline -p paladin-llm --all-features --lib conformance` | ❌ Wave 0 |
| REASON-02 | `resolve` table: exact, gated, Off, Budget, substitution lower/higher, none-in-direction, D-07 non-substitutable | unit (table) | `cargo test --offline -p paladin-ports --lib model_profile` | ❌ Wave 0 |
| REASON-02 | refusal before HTTP (mock asserts 0 hits), `Permanent`, no retry | adapter unit x9 | `cargo test --offline -p paladin-llm --all-features --lib` | ❌ |
| REASON-02 | `CompatRequestParameters::all(wire)` cannot be called without a wire (compile) | compile | `cargo check -p paladin-llm --all-features` | ❌ |
| REASON-03 | exact wire body per level per adapter/preset (conformance case) | conformance macro | `cargo test --offline -p paladin-llm --all-features --lib conformance` | ❌ |
| REASON-03 | Kimi regression (direction set by probe) | mockito | `cargo test --offline -p paladin-llm --features kimi --lib kimi` | ❌ (after probe) |
| REASON-03 | live: invalid rejected + higher level => more `reasoning_tokens` | live (human checkpoint) | `cargo run -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --effort` | ❌ harness mode |
| REASON-04 | profile intersect AND semantics; overrides apply field-wise; chain pre-flight names hop | unit | `cargo test --offline -p paladin-llm --all-features --lib fallback` | ❌ |
| REASON-04 | build-time refusal (builder + agent_host name YAML key) | unit | `cargo test --offline -p paladin-ai --lib paladin_builder agent_host` | ❌ |
| REASON-05 | metadata keys on buffered + terminal chunk; run_metadata fold; trace field serde | unit | `cargo test --offline -p paladin-llm --all-features --lib sampling` and `-p paladin-core --lib trace` | ❌ |
| REASON-05 | `reasoning_tokens` + text already mapped (regression only) | existing conformance `usage_fields_are_mapped` | same | ✅ |
| REASON-06 | Budget bounds (Anthropic >=1024 and < max_tokens; Qwen 1..=32768) | unit | `cargo test --offline -p paladin-llm --all-features --lib anthropic qwen` | ❌ |
| REASON-07 | docs page drift test; ADR number; MIGRATION/allowlist set-equality; api-surface | unit + scripts | `make check-migration-allowlist && make api-surface` | ❌ page |
| (cache) | changed effort changes cache key; unset unchanged | unit | `cargo test --offline -p paladin-battalion --lib cache_key` | ❌ |

### Sampling Rate
- **Per task commit:** the touched crate's quick command, `cargo fmt --check`, `cargo clippy --offline -p <crate> --all-targets --all-features -- -D warnings`.
- **Per wave merge:** `cargo test --offline --workspace --all-features --no-run`, `make check-gates`, `make api-surface`, `make check-api-examples`, `make check-migration-allowlist`.
- **Phase gate:** full suite above plus `make check-consumer-surface`, `make security`, CI `coverage` job >= 82 %.

### Wave 0 Gaps
- [ ] `crates/paladin-core/src/platform/container/reasoning.rs` tests (serde, ordering helpers)
- [ ] `EffortSupport::resolve` table tests in `model_profile.rs`
- [ ] golden unset-body cases for all nine adapters (before any adapter change)
- [ ] `ConformanceFixture` hook for reasoning cases + new macro cases
- [ ] `51-RED-EVIDENCE.md` pattern and `#[ignore = "fixed by 51-NN"]` convention
- [ ] `live_vendor_smoke --effort` mode (+ `--dry-run` plan listing, no network)
- [ ] docs page drift test module

## Security Domain

`security_enforcement` not set in `.planning/config.json`, so enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (no auth surface) | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | Closed enum for levels (no free-form string reaches a wire); `Budget(u32)` range-checked against profile bounds and `max_tokens`; `ModelOverride` keeps `deny_unknown_fields` and gets `validate()` rules (non-empty strictly ascending levels, `lo<=hi`, `lo>=1`); YAML `reasoning_wire` is a closed enum |
| V6 Cryptography | no | — |
| V7 Error handling / logging | yes | `UnsupportedParameter` carries provider, model, level name only (never request content or keys); vendor 400 bodies still redact-then-bound via `diagnostic_excerpt`; probe transcript prints numbers only |
| V10/V14 Config | yes | self-hosted `base_url` is operator-trusted (existing T-17-18, redirects disabled); `reasoning_wire` declaration is operator-owned |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Cost/latency amplification via unconfigured vendor default (`kimi-k3` default `max`) or an over-large `Budget`/`Max` | Denial of Service / Repudiation | Refuse out-of-profile values; document effort+`max_tokens`+timeout together; `Budget` bounded by profile and `max_tokens` |
| Silent substitution (vendor coerces, or Paladin rounds) hiding what ran | Tampering / Repudiation | Exact-or-refuse; substitution opt-in, `warn!` plus requested/effective metadata and trace |
| Credential leakage via probe transcript or error text | Information Disclosure | Probe prints names/numbers only; scrub procedure from 50.1-07; no key in any `Debug`; manual credential review gate |
| Stale cache served across effort changes | Tampering (integrity of A/B results) | Fingerprint includes effort when set (Pitfall 7) |
| Forward of operator string to the wire (`reasoning_wire` misuse) | Tampering | Closed `ReasoningWire` enum, no free-form field names |
| Retry storm on a deterministic refusal | DoS | `Permanent` classification, raised before retry loops |

## Sources

### Primary (HIGH confidence)
- Codebase at `7b77ffef`: `crates/paladin-ports/src/output/{model_profile,llm_port}.rs`, `crates/paladin-llm/src/{registry/{mod,seeded},compat/{engine,types},sampling,fallback,conformance}.rs`, adapters, `config/{llm,layers,bridge}.rs`, `examples/live_vendor_smoke.rs`, `crates/paladin-core/.../{paladin,trace,execution_result}.rs`, `crates/paladin-battalion/src/{llm_failure,llm_decision,engine/cache_key}.rs`, `src/config/agents.rs`, `src/infrastructure/web/agent_host.rs`, `src/application/services/paladin/{paladin_builder,paladin_execution_service}.rs`, `MIGRATION.md`, `Makefile`.
- Phase artefacts: `51-CONTEXT.md`, `51-HANDOFF-...md`, `51-DISCUSSION-LOG.md`, `.planning/REQUIREMENTS.md` (REASON-01..07), Phase 50 / 50.1 plan summaries and seed-probe headers.

### Secondary (MEDIUM confidence; vendor docs fetched 2026-10-21, no live call)
- https://platform.claude.com/docs/en/build-with-claude/effort ; /thinking ; /thinking-troubleshooting ; /extended-thinking
- https://developers.openai.com/api/docs/guides/reasoning ; /deployment-checklist
- https://api-docs.deepseek.com/guides/thinking_mode
- https://docs.x.ai/docs/guides/reasoning ; https://docs.x.ai/docs/api-reference
- https://ai.google.dev/gemini-api/docs/thinking ; /docs/openai ; /api/generate-content (ThinkingConfig section not retrievable)
- https://www.alibabacloud.com/help/en/model-studio/deep-thinking
- https://docs.ollama.com/capabilities/thinking ; /api/openai-compatibility
- https://platform.kimi.ai/docs/guide/use-kimi-k3-model ; /use-thinking-models ; /api/models-overview

### Tertiary (LOW confidence)
- Search summaries of third-party Kimi integration guides (conflicting on K3 effort wiring); Firebase SDK references for Gemini `thinkingConfig` shape.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, no new packages.
- Architecture: HIGH, every integration point read in source; layering (types in core) is forced by `PaladinData`.
- Pitfalls: HIGH for codebase pitfalls (1, 7, 8, 9, 10, 14, 16), MEDIUM for vendor pitfalls (3, 4, 13), LOW for Kimi (2).
- Vendor data: MEDIUM desk research; nothing encodable before the probe.

**Research date:** 2026-10-21
**Valid until:** 2026-10-28 for vendor facts (vendors change these fields often; Kimi and Gemini docs moved within the past two weeks); codebase facts valid until the next commit on the touched files.

## Planner Notes

Suggested plan decomposition (starting point, per D-00k): **51-01** inert types + `LlmError` variant + `ModelProfile`/`resolve` (TDD) + ignored red wire tests + golden unset bodies; **51-02** probe harness `--effort` mode + candidate list (owner checkpoint file); **51-03** plumbing (PaladinData, builder, AgentDefinition, agent_host, execution service x3, `*Config` knobs, `Layered`, cache fingerprint, metadata keys); **51-04** compat engine + presets + self-hosted knobs; **51-05** native adapters + Budget validation; **51-06** fallback pre-flight + `intersect`; **51-07** trace field; **51-08 (human-action)** run the probe and commit `51-EFFORT-PROBE.md`; **51-09** seed data + Kimi/Grok decisions + conformance un-ignores; **51-10** docs page + drift test, ADR-0062 + PROMOTION, MIGRATION/CHANGELOG/allowlist/api-surface/muster, verification. Plans 03-07 can run before the probe using synthetic profiles in tests; only 51-09 is blocked by it.
