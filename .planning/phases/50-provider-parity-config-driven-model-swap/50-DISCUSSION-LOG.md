# Phase 50: Provider Parity & Configuration-Driven Model Swap - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-15 (session began 2026-10-14)
**Phase:** 50-provider-parity-config-driven-model-swap
**Areas discussed:** Parameter-drop policy & support model, Per-model profile registry, Typed
refusal & reasoning text, Config-driven construction & fallback hops

**Todo cross-reference:** five pending todos matched at score 0.6. Folded: *web3sec moves once,
straight to v0.11.0, after Phases 50 and 51*. Reviewed and not folded: *Reject removed v0.10
Battalion and Conclave config keys*, *Verify local coverage reproduction*, *Phase 44 review
follow-ups*, *Review content / data orchestration base items*.

---

## Parameter-drop policy & support model

**D-1: configured sampling parameter the target cannot honour**

| Option | Description | Selected |
|--------|-------------|----------|
| Refuse explicit, skip auto with warn + metadata | Handoff option (b). Operator-written value the target rejects fails the build naming provider, model, parameter; auto-selected value omitted with `warn!` and run-metadata entry | ✓ |
| Refuse always | Handoff option (a). Any configured value the target cannot send fails the build, auto included | |
| Always omit with warning | Handoff option (c). Never fail; drop and warn | |

**What counts as "explicit"** (asked twice; the first round returned no answer for this item)

| Option | Description | Selected |
|--------|-------------|----------|
| Any operator-written value | Builder setter, agent YAML `temperature:`, `llm.<provider>.default_temperature` are all explicit; only `auto_temperature` output is auto | ✓ |
| Agent-level only | Provider-block default treated like auto | |
| You decide | | |

**Per-provider/per-model support shape (PARITY-02)**

| Option | Description | Selected |
|--------|-------------|----------|
| Additive `ModelProfile` via defaulted `LlmPort` method | `fn model_profile(&self, model) -> Option<ModelProfile>` default `None`; honours Phase 26 D-28 guard | ✓ |
| Clean break of `ProviderCapabilities` | X-03 supersession ADR, 92 struct-literal sites | |
| You decide | | |

**ADR form**

| Option | Description | Selected |
|--------|-------------|----------|
| New ADR-0061 plus in-place note on 0004 | ADR-0006/0046 in-place-note pattern; Phase 51's amendment becomes ADR-0062 | ✓ |
| Amend ADR-0004 in place only | | |
| You decide | | |

**User's choice:** all four recommended options. **Notes:** none; "Next area" chosen at the check.

---

## Per-model profile registry

**D-3: source of truth**

| Option | Description | Selected |
|--------|-------------|----------|
| Dated static table + config overrides now, live later | Handoff option (b); live `/models` deferred | ✓ |
| Dated static table only | Handoff option (a) | |
| Live discovery with static fallback now | Handoff option (c) | |

**Override YAML shape**

| Option | Description | Selected |
|--------|-------------|----------|
| `llm.<provider>.models.<model-id>.*` | Nested under the provider block; optional key | ✓ |
| Top-level `llm.model_profiles` list | Flat list beside `default_provider` | |
| You decide | | |

**Unknown model (no entry, no override)**

| Option | Description | Selected |
|--------|-------------|----------|
| Fall back to adapter-wide capabilities with warn + metadata | Same visibility rule as D-1; Commissary Strict refuses only when no window declared | ✓ |
| Refuse at build time | | |
| You decide | | |

**Choosing the new dated defaults**

| Option | Description | Selected |
|--------|-------------|----------|
| Live-probe each vendor at seed time, newest general model as default | Per-entry verified-on date; unverifiable entries left out | ✓ |
| Keep resolving defaults, only add entries | | |
| You decide | | |

**User's choice:** all four recommended options. **Notes:** none; "Next area" chosen.

---

## Typed refusal & reasoning text

**D-2: surfacing a policy refusal**

| Option | Description | Selected |
|--------|-------------|----------|
| New `LlmError::Refusal {provider, model, detail}`, Permanent | Distinct from `ContentFilter`; detail redacted-then-truncated | ✓ |
| Map onto existing `ContentFilter` | | |
| You decide | | |

**Chain behaviour on refusal** (context given: `FallbackLlmAdapter` short-circuits on `Permanent`)

| Option | Description | Selected |
|--------|-------------|----------|
| Stop by default, explicit opt-in to hop | Plain-named chain setting, default off | ✓ |
| Always stop | No opt-in | |
| Always hop | Treat Refusal as Transient for chain purposes | |

**Where reasoning text surfaces**

| Option | Description | Selected |
|--------|-------------|----------|
| Typed `Option<String>` on `LlmResponse` + typed streaming-delta field | Additive on `#[non_exhaustive]` structs; matches Phase 51's assumed D-4 | ✓ |
| `metadata["reasoning_content"]` | | |
| You decide | | |

**User's choice:** all three recommended options. **Notes:** none; "Next area" chosen.

---

## Config-driven construction & fallback hops

**Public library API (PARITY-03)**

| Option | Description | Selected |
|--------|-------------|----------|
| `LlmProviderFactory::create_from_config(&LlmConfig, provider, model)` | Beside env-only `create`; routed through `config/bridge.rs` | ✓ |
| New plainly named builder type | | |
| You decide | | |

**Precedence**

| Option | Description | Selected |
|--------|-------------|----------|
| Agent > YAML block > legacy env var > compiled default | Most specific operator statement wins; env fills gaps | ✓ |
| Agent > legacy env var > YAML block > compiled default | Twelve-factor style | |
| You decide | | |

**D-4: default facade features**

| Option | Description | Selected |
|--------|-------------|----------|
| Keep `openai + anthropic + deepseek` default | ADR-0046 stands; six re-exports behind opt-in features | ✓ |
| Enable all nine by default | | |
| You decide | | |

**Fallback hop model (PARITY-06)**

| Option | Description | Selected |
|--------|-------------|----------|
| Plain `FallbackStep { port, model }` + `from_steps` | Rewrites `request.model` per hop; feeds Cadence key; recorded beside `SERVED_BY_METADATA_KEY` | ✓ |
| Hop rewrites to its adapter's configured default model | | |
| You decide | | |

**User's choice:** all four recommended options. **Notes:** "Done with this area" and "I'm ready
for context" chosen at the closing checks.

---

## Claude's Discretion

Plan count/order from the handoff's six-plan shape; matrix-test layout; run-metadata key names,
`warn!` targets, `ModelProfile` field/enum names; the opt-in chain setting's name; the
`LlmResponse`/delta field names; the per-agent resolution helper and missing-block error shape;
conformance-suite extension order and Anthropic's entry into it; live-smoke extension and which
live checks run where; mdBook page location and table shape; whether DeepSeek's current defaults
still resolve.

## Deferred Ideas

Live `/models` discovery (handoff D-3 option c); native tool calling, compat vision, prompt
caching, request-id propagation (ADR-0042, handoff §9); reasoning effort (Phase 51). Four reviewed
todos not folded (listed above).
