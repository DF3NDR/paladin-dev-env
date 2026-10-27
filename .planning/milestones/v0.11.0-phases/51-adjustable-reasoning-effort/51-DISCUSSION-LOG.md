# Phase 51: Adjustable Reasoning Effort - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-21
**Phase:** 51-adjustable-reasoning-effort
**Areas discussed:** Effort data & unprofiled models, Nearest-level opt-in, Fallback-chain hops, Self-hosted endpoints

Handoff decisions D-1..D-5 were carried forward as already settled by the roadmap entry and
REQUIREMENTS (D-4 already delivered by Phase 50 D-11); they were presented, not re-asked.

---

## Todo cross-reference

| Option | Description | Selected |
|--------|-------------|----------|
| web3sec single bump | Already folded in Phase 50; fold here too | |
| OpenAI Responses API | New `/v1/responses` wire for the pro tier | |

**User's choice:** neither. Both are recorded as reviewed, not folded.

---

## Effort data & unprofiled models

**Where do supported levels come from?**

| Option | Description | Selected |
|--------|-------------|----------|
| New live effort probe | Human-action checkpoint; levels written only where measured | ✓ |
| Desk research, marked as such | "documented" provenance tier; breaks ADR-0061 | |
| Overrides only this phase | No seeded levels; everything refuses out of the box | |

**Explicit effort on a model with no measured levels?**

| Option | Description | Selected |
|--------|-------------|----------|
| Refuse | `UnsupportedParameter`, error points at the override | ✓ |
| Adapter-wide fallback + warn! | Phase 50 D-07 path for sampling | |

**Wire field that cannot be confirmed live (Grok, Ollama)?**

| Option | Description | Selected |
|--------|-------------|----------|
| Unsupported until probed | Any explicit effort refuses; table says "unverified" | ✓ |
| Desk-research wire form | Ship documented field; silent-no-op risk | |

**Probe scope?**

| Option | Description | Selected |
|--------|-------------|----------|
| Reasoning-capable seeded | Every `reasoning: true` entry; non-reasoning get explicit "does not reason" | ✓ |
| Every seeded model | All ~45 ids | |
| web3sec's models + 1/vendor | Quick; most models refuse until later | |

---

## Nearest-level opt-in

**Where is the opt-in set?**

| Option | Description | Selected |
|--------|-------------|----------|
| Beside effort, same precedence | Request / agent / provider, same resolution | ✓ |
| Provider config only | Operator-wide policy | |
| Per request only | Smallest surface | |

**Direction of "nearest"?**

| Option | Description | Selected |
|--------|-------------|----------|
| Caller picks: lower or higher | `NearestLower` / `NearestHigher` | ✓ |
| Always round down | Cost-safe | |
| Always round up | Quality-safe | |

**Which refusals can it substitute?**

| Option | Description | Selected |
|--------|-------------|----------|
| Level→level only | Missing or gated level only | ✓ |
| Also Off→lowest level | Off on always-thinking models substitutes | |
| Everything incl. Budget | Heuristic Budget↔level mapping | |

---

## Fallback-chain hops

**A hop's model cannot honour the effort?**

| Option | Description | Selected |
|--------|-------------|----------|
| Pre-flight every hop | Refuse before the first HTTP call, naming the hop | ✓ |
| Per hop, stop the chain | Today's Permanent semantics; hidden until failover | |
| Per hop, skip that hop | Failover silently loses hops | |

**Build-time check of agent/provider effort?**

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, plus send-time | `PaladinBuilder::build` / `agent_host` refuse, like Phase 50 D-01 | ✓ |
| Send-time only | One check site | |

---

## Self-hosted endpoints

**How does an operator unlock effort on OpenAI-compatible / Ollama?**

| Option | Description | Selected |
|--------|-------------|----------|
| Declare wire + levels | `with_reasoning_wire` + per-model overrides, both required | ✓ |
| Per-model levels only | Fixed wire forms | |
| Nothing this phase | Both stay Unsupported | |

**How does the operator verify a declared wire form?**

| Option | Description | Selected |
|--------|-------------|----------|
| Documented probe command | Effort-probe mode accepts self-hosted targets | ✓ |
| Docs warning only | No tool support | |

**Can the wire override apply to vendor presets?**

| Option | Description | Selected |
|--------|-------------|----------|
| Self-hosted only | Vendor presets change via probe + release | ✓ |
| Every compat preset | Flexible escape hatch | |

---

## Claude's Discretion

- Plan count/order; `ReasoningWire` and `ReasoningFallback` variant names; reshaping
  `CompatRequestParameters::all()` so every preset still states a reasoning position;
  `ModelProfile` field names; `map_reasoning` signatures; Gemini level-vs-budget selection;
  metadata/trace key names; wire-body test layout; mdBook page placement.

## Deferred Ideas

- Live `/models` discovery of supported levels.
- Heuristic Budget↔level and Off→lowest-level substitution.
- Wire-form overrides on named vendor presets.
- OpenAI Responses API (todo 2026-10-20, not folded).
