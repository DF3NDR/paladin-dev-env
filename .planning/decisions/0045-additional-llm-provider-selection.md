# ADR-0045: Additional LLM provider selection study

## Status

Accepted

**Date:** 2026-08-17

Every verdict recorded below was human-selected in an interactive `/gsd-discuss-phase 17` session
on 2026-08-16 (D-00i). None was `--auto`-derived — the candidate field, the qualifying bar, and
every individual disposition trace to that session's transcript (`17-DISCUSSION-LOG.md`) and are
locked in `17-CONTEXT.md` as D-01, D-02 and D-03. This ADR turns those locked verdicts into a
durable, citable artifact; it does not re-litigate any of them.

## Context

**What ships today.** `paladin-llm` ships three real providers plus a mock, each behind its own
Cargo feature: `default = ["openai", "mock"]`, `openai = ["dep:reqwest", "dep:rand"]`,
`anthropic = ["dep:reqwest", "dep:rand"]`, `deepseek = ["dep:reqwest", "dep:rand"]`, `mock = []`
(`crates/paladin-llm/Cargo.toml:17-24`). All four are resolved by provider name through
`LlmProviderFactory::create()`, a hand-written `match` over feature-gated arms
(`crates/paladin-llm/src/provider_factory.rs:62-117`).

**The measured fact that drives the whole verdict set.** `deepseek/adapter.rs` (1,368 lines) and
`openai/adapter.rs` (741 lines) implement the *same* `POST {base_url}/chat/completions` protocol
twice, each with its own private wire structs:

```
crates/paladin-llm/src/openai/adapter.rs:350:        let url = format!("{}/chat/completions", self.config.base_url);
crates/paladin-llm/src/openai/adapter.rs:407:        let url = format!("{}/chat/completions", self.config.base_url);
crates/paladin-llm/src/deepseek/adapter.rs:602:        let url = format!("{}/chat/completions", self.config.base_url);
crates/paladin-llm/src/deepseek/adapter.rs:705:        let url = format!("{}/chat/completions", self.config.base_url);
```

Once a shared compatible core exists (PROV-02, D-05), a fifth OpenAI-compatible provider costs a
preset — a `base_url`, an env-var name, a default model, a model list, a capabilities block — not
a second full adapter. That marginal-cost fact, not brand recognition, is the qualifying bar this
study scores every candidate against.

**The correction PROV-01 requires the study to carry.** "Llama" names a model family, not a
provider — there is no Meta inference endpoint to write an adapter against. A Llama row must name
the *host* it targets or be rejected for lacking one (`REQUIREMENTS.md` PROV-01, ROADMAP Phase 17
criterion 1).

### Criteria (recorded before scoring)

PROV-01's own nine scoring criteria, restated verbatim before any candidate is scored, per D-00e
and per this study's own "criteria before verdicts" requirement:

1. Wire compatibility with the existing adapter shape (OpenAI-compatible `/chat/completions` vs.
   bespoke, as Anthropic already is).
2. Streaming support and chunk format.
3. Tool/function-calling support.
4. Token-usage reporting feeding `TokenUsage` (`paladin-core`, ADR-0016).
5. Auth model.
6. Whether `reqwest` suffices or a new heavyweight dependency is implied — a new dependency is a
   cost, and `make deny` / `make audit` must still pass.
7. Hosted-only vs. self-hostable.
8. Licence and ToS constraints on programmatic access.
9. Obtainability for testing without a paid account.

**The qualifying bar is marginal build cost against the shared core, not brand recognition
(D-01).** Every OpenAI-compatible candidate scores well on criteria 1-2 and 6 for free, because the
core (once built) already satisfies them; what differentiates candidates is criteria 5, 7, 8 and 9
— auth shape, hosting model, licence posture and testability without a paid account.

## Decision

The scored candidate field, one row per candidate, exactly one verdict each:

| Candidate | Endpoint / host | Wire shape | Auth | New dependency? | Testable without a paid key? | Verdict | Reason |
|---|---|---|---|---|---|---|---|
| **Kimi** (Moonshot AI) | `https://api.moonshot.ai/v1` | OpenAI-compatible `/chat/completions` | `MOONSHOT_API_KEY` bearer | No — `reqwest` suffices | No (hosted, paid) | **build** | The vendor's own docs frame the API as a migration target from OpenAI. Marginal cost is one preset. |
| **Qwen** (Alibaba, DashScope compatible-mode) | `https://dashscope-intl.aliyuncs.com/compatible-mode/v1` | OpenAI-compatible `/chat/completions` | `DASHSCOPE_API_KEY` bearer | No | No (hosted, paid) | **build** | Compatible-mode is an explicit OpenAI-SDK drop-in. Answers PROV-01's "which route" test by naming DashScope compatible-mode over a local shim. |
| **Grok** (xAI) | `https://api.x.ai/v1` | OpenAI-compatible `/chat/completions` | `XAI_API_KEY` bearer | No | No (hosted, paid) | **build** | OpenAI-compatible; added to the candidate field by the user during the 2026-08-16 session and qualifies on the same marginal-cost basis. |
| **Ollama** (self-hosted) | `http://localhost:11434/v1` | OpenAI-compatible `/chat/completions` | none required | No | **Yes — no API key** | **build** | Settles the Meta/Llama row (D-02). The only candidate testable with no API key, the only self-hosted coverage, and it fits the shipped Docker-gated Tier-2 shelf alongside Redis/MinIO/Qdrant. |
| **Gemini** (Google) | `generativelanguage.googleapis.com/v1beta` | Bespoke — resource-oriented URL, `contents[]`/`parts[]` body, `?alt=sse` streaming | `x-goog-api-key` header | No | No (hosted, paid) | **build** | The one bespoke adapter worth paying for — closer to the Anthropic adapter than to the compatible four. |
| **Meta / Llama** (as named in the request) | *no Meta inference endpoint exists* | — model family, not a provider — | — | — | — | **reject as a row; dispositioned via Ollama** | A model family, not a provider. The row is answered by naming a host: Ollama (D-02). Recorded as one row, not one row per possible host. |
| **Groq** | `https://api.groq.com/openai/v1` | OpenAI-compatible | API key bearer | No | No (hosted, paid) | **reject — already covered** | Reachable via the generic operator-configured provider (D-03) with configuration and no new code. |
| **Together** | `https://api.together.xyz/v1` | OpenAI-compatible | API key bearer | No | No (hosted, paid) | **reject — already covered** | As Groq. |
| **Mistral** | `https://api.mistral.ai/v1` | OpenAI-compatible | API key bearer | No | No (hosted, paid) | **reject — already covered** | As Groq. |
| **Fireworks** | `https://api.fireworks.ai/inference/v1` | OpenAI-compatible | API key bearer | No | No (hosted, paid) | **reject — already covered** | As Groq. |
| **Bedrock** (AWS) | AWS-hosted, OpenAI-compatible mode available | OpenAI-compatible (via compat mode) | AWS credentials / SigV4 for the native API; API key for compat mode | No, via compat mode | No (hosted, paid) | **reject — already covered** | Reached through its OpenAI-compatible mode by the generic provider. A bespoke SigV4 adapter is a different, unrequested capability. |

**D-03's consequence, stated in its own paragraph because it is the contested position this ADR
exists to hold:** a generic operator-configured OpenAI-compatible provider ships as public surface
(PROV-01/PROV-02), so a future request for any of the five rejected names — Groq, Together,
Mistral, Fireworks, Bedrock — is answered by configuration rather than by a new phase. The five
named presets built above (Kimi, Qwen, Grok, Ollama, Gemini) remain as curated presets carrying
correct capabilities and model lists — the generic provider covers *access*, a preset covers
*curation*. **Reintroduction trigger, stated explicitly:** a specific request for one of the
rejected names' curated capabilities and model list, not merely for access to the endpoint —
access already exists via the generic provider the day it ships.

## Considered Options

- Build every OpenAI-compatible candidate as its own named preset — rejected: curation cost with no access benefit once the generic provider exists.
- Defer Groq/Together/Mistral/Fireworks/Bedrock rather than reject them — rejected: a deferral implies future work that D-03 has already made unnecessary, and each deferral becomes a phase.
- Ship no generic provider and treat every new endpoint as a new adapter — rejected: unbounded phase generation for a capability that is four config fields.
- Reject the Llama row outright for lacking a host — rejected: Ollama is a concrete host that also buys keyless testability, so rejecting the row would discard the phase's only credential-free end-to-end exercise of the shared core.

## Code Locations

- `crates/paladin-llm/Cargo.toml:17-24` — the `[features]` block: `default = ["openai", "mock"]` plus the three per-provider flags this study's build list extends.
- `crates/paladin-llm/src/provider_factory.rs:62-117` — `LlmProviderFactory::create()`'s hardcoded `match`, the site PROV-03/D-10 replaces with a table-driven registry.
- `crates/paladin-llm/src/provider_factory.rs:16` — the `UnknownProvider` error text naming the three supported providers today.
- `crates/paladin-llm/src/deepseek/adapter.rs:602` and `:705` — the two `{base_url}/chat/completions` call sites measured above.
- `crates/paladin-llm/src/openai/adapter.rs:350` and `:407` — the second copy of the same protocol.
- `crates/paladin-llm/src/anthropic/adapter.rs` (1,180 lines) — the bespoke-protocol template Gemini follows under D-08.
- `crates/paladin-ports/src/output/llm_port.rs:1291` — `get_provider_name(&self) -> &'static str`, the `LlmPort` method D-07 works around.
- `crates/paladin-ports/src/output/llm_port.rs:1363` — `get_capabilities(&self) -> ProviderCapabilities`.

## Code Conformance

**No code change is made or required by this ADR.** At authoring time none of the five build-marked
adapters exists yet:

```
$ ls crates/paladin-llm/src/
anthropic
config
deepseek
error.rs
lib.rs
llm_analysis_service.rs
mock.rs
openai
provider_factory.rs
```

`kimi/`, `qwen/`, `grok/`, `ollama/` and `gemini/` are all absent, as is any generic
OpenAI-compatible provider module. This study records the verdicts that plans 17-01, 17-03, 17-04
and 17-05 build against — it does not claim conformance that does not exist (D-00e).

## Downstream Consumers

- **PROV-02** (`REQUIREMENTS.md`) — this study fixes its size at five named adapters (Kimi, Qwen,
  Grok, Ollama, Gemini) plus one generic operator-configured provider, settling the "one adapter or
  four" open question its own note carried.
- **PROV-03** (`REQUIREMENTS.md`) — the factory and feature-flag wiring resolves exactly the build
  list this study names.
- **PROV-04** (`REQUIREMENTS.md`) — model currency and testing apply to exactly this build list.
- **ADR-0046** (plan 17-06) — the phase's second contested position, D-11's breaking default-build
  flag change, is deliberately recorded as its own ADR rather than folded into this one (D-00g, the
  coarse-supersession-unit cost ADR-0034 already paid once).
- **`.planning/REQUIREMENTS.md`'s PROV-01 row** — amended in place (plan 17-02, Task 2) with a dated
  verdict summary pointing at this ADR.

## Amendment (2026-11-17): Z.ai

The original table above is not edited. This amendment records one more candidate, scored on the
same marginal-cost bar (D-01) and the same nine criteria, in the original table's column format:

| Candidate | Endpoint / host | Wire shape | Auth | New dependency? | Testable without a paid key? | Verdict | Reason |
|---|---|---|---|---|---|---|---|
| **Z.ai** (Zhipu AI international platform, GLM models) | `https://api.z.ai/api/paas/v4` (docs.z.ai, accessed 2026-11-17) | OpenAI-compatible `/chat/completions` | `ZAI_API_KEY` bearer | No — `reqwest` suffices | No (hosted, paid) | **build** | The 2026-11-17 request for a named, curated z.ai preset is this ADR's stated reintroduction trigger ("a specific request for one of the rejected names' curated capabilities and model list, not merely for access to the endpoint"); access was already available through `openai-compatible`. Marginal cost is one preset on the shared `CompatEngine`. |

**What the amendment does and does not change.**

- Z.ai was not one of the rejected names in the original table; it reaches the same conclusion by
  the same route. A bare request for access to an OpenAI-compatible endpoint is still answered by
  the generic provider (D-03). This request asked for the curated preset, so it builds one.
- The preset ships with **no live evidence**: no seed probe, no effort probe, no usage probe and no
  quota-body capture exist for Z.ai. Its seeded model table is empty (every model is unprofiled),
  its reasoning wire is `Unsupported` (every explicit effort is refused), it declares no
  `error_override`, and every compiled fact cites its docs.z.ai page and access date in
  `crates/paladin-llm/src/zai/adapter.rs`.
- Z.ai is configured through `ZAI_API_KEY`, `ZAI_BASE_URL`, `ZAI_MODEL` and `ZAI_TIMEOUT_SECONDS`
  only. It has **no `llm.zai` config block** in this release: adding a field to the published,
  constructible `LlmConfig` would be a semver break, so the owner deferred it (quick task
  261117-t5c, decision option-b, env-only-now) to a later release. Consequently Z.ai is not in
  the provider swap matrix, which builds every provider from its config block.
- Z.ai documents a separate endpoint and key family for its GLM Coding Plan. The preset defaults to
  the general endpoint and reaches the other only through `ZAI_BASE_URL`.
