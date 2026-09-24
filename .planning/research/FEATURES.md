# Feature Research

**Domain:** LLM spend governance / cost tracking (output-side, for a Rust orchestration
**framework library**, not a hosted SaaS gateway or proxy)
**Milestone:** v0.11.0 "Treasurer Spend Governance"
**Researched:** 2026-09-24
**Confidence:** MEDIUM (web sources only — see `<source_hierarchy>` policy; cross-checked
across ≥3 independent products per claim raises individual findings to MEDIUM; no HIGH-tier
curated/official-docs source was consulted for this pass)

## How Comparable Systems Work

Surveyed: **LiteLLM proxy** (OSS gateway, closest architectural analog — Python but same
budget/pricing-table shape), **OpenRouter** (SaaS credit marketplace), **Portkey** (SaaS AI
gateway), **Helicone** (SaaS observability + gateway), **LangSmith** (SaaS tracing/cost),
**Langfuse** (OSS-core tracing/cost), plus **OpenAI** and **Anthropic** rate-limit header
contracts and general Redis distributed-rate-limiting/cache-stampede patterns.

**Pricing tables.** Every system keyed by model name, with the same four-ish price axes
Paladin's PRD already names: input/prompt, output/completion, cache-read, cache-write (write
priced ~1.25x-2x input, read ~0.1x input in Anthropic's own scheme — LiteLLM and Langfuse both
mirror this exact discount ratio in their tables, so it's a de facto industry unit, not a
LiteLLM-specific choice). Reasoning tokens are billed as output-rate tokens but *some*
providers itemize them separately and some bundle them into completion — named explicitly by
one source as a leading cause of bill mismatches across providers. Long-context tiered pricing
(different, higher rate above ~200K input tokens) is real and shipping in 2026 (Gemini,
Claude Fable-class, Anthropic 1M-context models) — LiteLLM has dedicated
`*_above_200k_tokens` price-table fields and Langfuse ships "pricing tiers" as a first-class
feature specifically to model this. Every pricing table defaults ungoverned/unknown models to
either a flat input-rate fallback (LiteLLM, for omitted cache fields) or a fail-open $0 (Portkey,
for models the pricing DB doesn't recognize) — both are documented, deliberate choices, not bugs.

**Budget/allowance windows.** The near-universal shape is a *cap value + a reset cadence*
attached to an *entity* (key, user, team/tenant, sometimes "customer"), evaluated at admission.
LiteLLM checks up to three entities simultaneously per request (key, key's owner user, key's
team) with team membership overriding personal budget — i.e. governance composes hierarchically
rather than being a single flat cap. Cadences seen: rolling monthly, rolling daily (via
"tpm/rpm"-style per-minute quotas for rate, separate from $ budgets), and an optional
lifetime/no-reset cap alongside the rolling one (matches the PRD's "rolling periods... with an
optional lifetime cap" almost exactly). No system surveyed does true reservation/hold-then-settle
accounting on the money axis — every one is **post-hoc**: the call is made, actual usage comes
back on the response, cost is computed from the real token counts, and the ledger/budget counter
is decremented *after the fact*. What every system deliberately front-loads at admission is a
**pre-flight allowance check** (do I have budget left at all, checked before the call) which is
categorically different from *estimating* the specific call's cost in advance — none of the
surveyed systems does token-cost estimation pre-flight because you don't know completion/cache/
reasoning tokens until the response returns. This directly validates the PRD's own split: R3
"refuses a draw at admission" (a threshold check against known-spent-so-far) is achievable and
is exactly what these systems do; true pre-flight per-call cost estimation is not something any
comparable system attempts, and Paladin shouldn't invent it as a distinguishing feature.

**Soft vs hard limits.** LiteLLM's `soft_budget` fires an alert (Slack) without blocking;
exceeding the hard budget blocks. Helicone graduates alert thresholds at 50/80/95% before any
hard stop, with separate thresholds per environment (dev vs prod). This two-tier
(warn-then-block) pattern is universal enough to be table stakes for any system claiming "budget
enforcement" — a hard-only cutoff with no earlier warning is the exception, not the rule, in
every product surveyed.

**In-flight requests at the limit.** None of the surveyed *proxy/gateway* products need to solve
"halt a run cleanly and resumably mid-execution," because they are stateless single-call
gateways — a blocked request just gets a 429/403 back to the caller, no in-flight
multi-step state to preserve. This is where Paladin's problem is harder and genuinely different
from every comparable system: Paladin already has durable multi-step runs with Waypoint
checkpointing, Parley pause/resume, and Aegis fault tolerance — none of which any surveyed
competitor has an analog for. The PRD's requirement ("a draw that would overspend halts the run
cleanly, checkpoint kept, resumable") is *not* something to copy from LiteLLM/Portkey/Helicone —
it has to be built on Paladin's own Waypoint/Aegis primitives, using the same typed-error +
checkpoint-then-surface pattern the codebase already uses for `ToolCallLimit`/`ModelCallLimit`
(`crates/paladin-battalion`) rather than an HTTP-gateway 429 response.

**Ledger and reporting granularity.** Universal three-tier pattern across LangSmith
(per-trace / per-project-aggregate / dashboard), Langfuse (per-observation, summed over
sliding time windows for alerts), Helicone (real-time dashboard + periodic digest) and LiteLLM
(per-request logged, rolled up by model/user/key): **per-call, per-run(-aggregate), and a
windowed/aggregate view**. This maps cleanly onto the PRD's "surfaced through heralds, CLI, and
traces" — heralds/CLI want per-run totals, traces want per-call line items, and a durable ledger
(the new port) is what makes windowed aggregate queries (this key's spend this month) possible
without re-scanning every run.

**Rate pacing on 429.** Both OpenAI and Anthropic expose the same two-part contract: (1) a
`retry-after` header present *only* on the actual 429 response, to be treated as a **floor**,
with jitter added on top because the value can read slightly short at a rolling-window boundary;
and (2) always-present `x-ratelimit-*`/`anthropic-ratelimit-*` headers (limit/remaining/reset per
dimension — requests, input tokens, output tokens, and for Anthropic also concurrency) that let a
well-behaved client back off *before* it gets a 429 at all, not just react after. Anthropic
splits reset-per-dimension so a client can tell whether it tripped RPM vs TPM and computes
`retry-after` from whichever dimension actually tripped. The universal advice beyond "honor
retry-after" is to *also reduce concurrency* on a 429, not just delay the single retried call —
directly relevant to Paladin's Muster fan-out, where a 429 from one worker should pace the whole
shared pool, not just that one task's retry.

**Distributed pacing across workers.** Outside the LLM-specific products, the standard
mechanism for "pace shared across multiple worker processes" is a token-bucket or sliding-window
counter held in Redis, mutated atomically via a Lua script (read-check-decrement in one
round-trip, avoiding the check-then-act race an ordinary `GET`+`SET` has under concurrent
workers). Paladin already has exactly this shape in production for `RunQueuePort`'s Redis
adapter (Lua-scripted lease claims, per Phase 27) — the Treasurer's cross-worker pacing state
should reuse that same Lua-atomic-operation pattern rather than inventing a new one.
**Cache-stampede locking** (the PRD's explicit R4 ask) is the standard complement: when many
workers simultaneously discover "we're rate-limited, refresh our shared backoff state," a
distributed mutex (single-flight lock in Redis, `SET NX PX` + TTL is the idiomatic primitive)
ensures only one worker computes/writes the new backoff window while the rest either wait
briefly or read the value the lock-holder just wrote, instead of every worker hammering the
provider or Redis simultaneously to recompute the same thing.

## Feature Landscape

### Table Stakes (Any Spend-Governance System Needs These)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Per-model price table (prompt/completion/cache-read/cache-write/reasoning) | Every surveyed system (LiteLLM, Langfuse, Helicone) keys cost off a model-name → per-axis-price table; PRD R1 already specifies this shape | LOW | Pure data + a pricing function over the existing `TokenUsage` split (Milestone 13 already lands the split — hard dependency, satisfied) |
| Cost computed and attached per call and per run | Universal (`ExecutionMetadata.cost_estimate`, PRD R2); every competitor exposes call-level and run/trace-level cost | LOW-MEDIUM | Producer already reserved (dead field since Milestone 13 Epic 1); this is "give it a producer," not new surface |
| Unknown-model / unpriced-model handling that fails safe (no phantom cost) | LiteLLM defaults cache-omitted fields to input rate; Portkey shows $0 and excludes from budget for unrecognized models — both document the behavior rather than silently guessing | LOW | PRD already specifies `cost_estimate` stays `None` with no configured price — matches Portkey's fail-open-to-null pattern, not LiteLLM's fallback-guess pattern; keep it None, don't guess |
| Budget/allowance cap + reset cadence per entity (key, tenant) | LiteLLM (key/user/team), Helicone (per-user, per-key), Portkey (per-team/per-endpoint/per-user) all key budgets off an identity axis with a rolling reset | MEDIUM | PRD R3 matches almost exactly: per-tenant, per-API-key, rolling daily/monthly + optional lifetime cap |
| Refuse-at-admission check | Every surveyed system blocks a new call once the relevant budget is exhausted, checked before the call is made | LOW-MEDIUM | This is the "pre-flight" half — check accumulated spend against cap, not predict the new call's cost |
| Post-hoc settle (compute actual cost after the response, update ledger) | Universal — no surveyed system attempts pre-flight per-call cost prediction because completion/cache/reasoning token counts aren't known until the response returns | LOW | Paladin already has the full `TokenUsage` on response (Milestone 13); Treasurer settles after each call the same way |
| Durable ledger with query-by-window (spend this key this month) | LiteLLM's spend tracking, Langfuse's alert windows, Helicone's dashboards all persist raw spend events and aggregate on read | MEDIUM-HIGH | New port + 3 adapters (in-memory/SQLite/Postgres) mirroring `RunRepositoryPort` — this is the single largest net-new surface in the milestone |
| Warn threshold before hard block (soft vs hard limit) | LiteLLM `soft_budget` (alert, no block) vs hard budget (block); Helicone's graduated 50/80/95% thresholds | LOW-MEDIUM | Not explicitly required by the PRD's R3 wording but is close to universal in the comparable-system survey; worth flagging to the roadmap as a likely near-scope addition even if not literally named |
| Retry-After / 429 back-off honoring provider headers | Universal OpenAI/Anthropic contract; failing to honor it gets a client rate-limited harder, not less | LOW-MEDIUM | `LlmError::RateLimitExceeded` already exists (grep-confirmed in `paladin-llm`); PRD R4 wires pacing off this signal, doesn't invent it |
| Jitter on retry timing | Named by both OpenAI's and Anthropic's own guidance as necessary because `retry-after` is a floor, not an exact value, and because a fleet of workers must not all wake simultaneously | LOW | Cheap to add once back-off exists; skipping it reproduces exactly the thundering-herd failure mode the sources warn about |

### Differentiators (Where Paladin's Framework Shape Beats the SaaS-Gateway Norm)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Clean, resumable in-flight halt on overspend (not just refuse-at-admission) | No surveyed SaaS gateway/proxy needs this because none of them have durable multi-step runs — a blocked call there is just a 429 to a stateless caller. Paladin already has Waypoint checkpointing, Aegis fault handling, and Parley suspend/resume; a mid-run allowance breach can checkpoint and surface a typed error instead of losing work | MEDIUM-HIGH | This is the PRD's own most distinctive requirement (R3, "halt cleanly, checkpoint kept, resumable") — genuinely not something to copy from a competitor, must be built on existing `WarEngine`/Waypoint primitives |
| Spend surfaced through the existing trace stream and heralds (not a separate dashboard product) | LangSmith/Langfuse/Helicone are dashboards bolted onto traces after the fact, as a separate product surface; Paladin already has a first-class `TraceRecord`/`TraceEvent` stream (Phase 28) and Herald output formatters — spend becomes just another field on primitives that already exist, not a new UI | LOW-MEDIUM | Directly reuses OBS-01..04 infrastructure; no new observability surface needed, only new event/field types |
| Treasurer composes with (doesn't replace) `TokenBudget`/`ModelCallLimit`/`ToolCallLimit` middleware | Every competitor's budget system *is* the gateway — there's no separate "input rationing" layer underneath it to compose with. Paladin's two-officer model (Commissary input-side, Treasurer output-side) is architecturally distinct from every surveyed product, which conflates the two | MEDIUM | Already locked by ADR-0049/0050 and PRD §4 "out of scope: reworking the limit middleware" — this is a genuine architectural differentiator worth naming, not a risk |
| Cross-worker pacing reusing the existing Redis Lua-atomic pattern | Generic Redis rate-limiting guidance (industry-standard) converges on exactly the Lua-atomic-operation pattern Paladin's `RunQueuePort` Redis adapter already uses for lease claims (Phase 27) | MEDIUM | Lower actual complexity than it looks because the atomic-Redis-op infrastructure and `redis-queue` feature flag already exist in the workspace — new Lua script + new Redis key namespace, not a new subsystem |

### Anti-Features (Seen in Comparable Systems, Wrong Fit for a Framework Library)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|------------------|-------------|
| Pre-flight exact cost estimation before the call is made | Sounds like better UX ("tell me the cost before I spend it") and some users will ask for it having seen SaaS gateway dashboards | No surveyed system does this because completion/cache/reasoning token counts are unknowable until the response returns — any pre-call number is a guess dressed as a fact, and a wrong guess that blocks a legitimate call is worse than no estimate | Admission check against *already-spent* balance (which every competitor does), settle actual cost post-hoc; optionally expose a *rough* upper-bound estimator (e.g. from `max_tokens`) as a clearly-labeled advisory number, never as an admission gate |
| Built-in/bundled default price table shipped with the crate | Convenient for a demo, and several observability SaaS products (Helicone's open-source cost repo, LiteLLM's `model_prices_and_context_window.json`) ship one | Prices change monthly across providers; a bundled table goes stale immediately and creates a maintenance burden + false-precision trap for a *library* (vs. a SaaS product that can push a live update). PRD R1 already states "nothing bundled" — correctly matches this anti-pattern avoidance | Operator-configured price table only, loaded from the same config source as everything else in Paladin; document how to source current prices, don't embed them |
| Credit/wallet purchasing and billing integration (Stripe-style top-ups) | OpenRouter's whole product is built around this and it's the most visible "spend governance" pattern on the market | Paladin is an embedded framework, not a billing platform; adding payment-provider integration would be a massive scope and licensing/compliance expansion utterly outside "enterprise multi-agent orchestration framework" | Allowances are operator-set numeric caps in config/DB, refilled by whatever process the embedding application already uses for billing — Treasurer enforces, never bills |
| A hosted dashboard/UI for spend visualization | Portkey, Helicone, LangSmith, Langfuse are all fundamentally SaaS dashboards; "give us a nice cost UI" is a natural ask once ledger data exists | Building and maintaining a web dashboard is a different product category and competes with `paladin-web`'s actual scope (an HTTP API, not an admin UI); it would also dwarf this milestone's remaining budget | Ledger data exposed via existing surfaces only — heralds (JSON/Markdown/Table), CLI, traces, and the HTTP API's existing run/thread endpoints; let downstream consumers (or a future `paladin-web` admin surface, explicitly out of scope here) build the UI |
| Retroactive budget application to already-in-flight or already-completed runs | Portkey explicitly notes budgets don't apply retroactively, and it's tempting to want "recompute what would have happened" for audit purposes | Requires re-deriving historical run state against a budget policy that didn't exist at execution time — an entirely different (and much harder) feature than admission-time enforcement, with unclear semantics for what "refuse" even means after the fact | Budgets/allowances apply from the moment they're configured forward only, exactly like every surveyed competitor; historical spend is still fully queryable via the ledger for reporting, just never re-enforced |

## Feature Dependencies

```
Milestone 13 Epic 2 (lossless TokenUsage split)
    └──requires (hard prerequisite, already shipped v0.10.0)──> R1 Pricing table + cost function
                                                                      └──requires──> R2 ExecutionMetadata.cost_estimate producer
                                                                                          └──requires──> R5 Spend ledger (port + adapters)
                                                                                                              └──requires──> R3 Treasurer allowance enforcement (admission refuse + in-flight halt)
                                                                                                              └──enables───> Spend surfaced in heralds / CLI / traces (R6 doc + observability wiring)

Existing TokenBudget / ModelCallLimit / ToolCallLimit middleware
    └──composed-by (not replaced by)──> R3 Treasurer (installs per-run TokenBudget)

Existing Waypoint checkpoint + Aegis fault handling (Phase 22, 25)
    └──enables──> R3's "halt cleanly, checkpoint kept, resumable" in-flight overspend behavior

Existing LlmError::RateLimitExceeded (429 signal)
    └──requires──> R4 in-process back-off/pacing
                        └──requires (for cross-worker sharing)──> Redis (existing redis-queue
                            feature, Lua-atomic pattern from RunQueuePort) ──> shared pacing state
                                                                          ──> cache-stampede lock

Existing TraceRecord/TraceEvent stream + Herald formatters (Phase 28)
    └──enhances──> R5/R6 spend surfaced in traces and heralds (no new observability subsystem needed)
```

### Dependency Notes

- **R1/R2 require Milestone 13 Epic 2 (already shipped v0.10.0, Phases 30-33):** the hard
  prerequisite named in the PRD itself is satisfied — `TokenUsage` already carries the
  prompt/completion/cache-read/cache-write/reasoning split (Phase 31, ACCT-01..05). This
  milestone can start immediately; there is no remaining blocker on the accounting shape.
- **R5 (ledger) requires R1/R2 (pricing/cost) to exist first:** the ledger stores *computed*
  cost events, so the pricing function must be callable before there's anything to persist. The
  `RunRepositoryPort` pattern (in-memory/SQLite/Postgres, already proven for runs in Phase 27)
  is the template to reuse for the new ledger port — same three-adapter shape, same migration
  approach.
- **R3 (allowances) requires R5 (ledger) to evaluate "already spent this window":** admission
  refusal needs a queryable running total per tenant/key over the rolling window, which is what
  the ledger provides. Sequencing R5 before R3's enforcement logic (even if planned in the same
  phase) avoids building allowance checks against data that doesn't exist yet.
  - **R3's in-flight-halt half is a *build*, not a *copy*:** none of the six comparable systems
    surveyed offer a template for "resumable mid-run halt," because none of them have durable
    multi-step runs. This half of R3 depends on Paladin's own Waypoint/Aegis/Parley primitives,
    not on external prior art — flag this to the roadmap as the phase most likely to need
    additional design time relative to its apparent size.
- **R3 explicitly composes with, not replaces, existing limit middleware** (PRD §4 out-of-scope,
  ADR-0049/0050 two-officer model) — this is a locked architectural decision, not a research
  finding, but it constrains phase design: the Treasurer phase should not touch
  `TokenBudget`/`ModelCallLimit`/`ToolCallLimit` internals.
- **R4 (pacing) requires the existing `LlmError::RateLimitExceeded` signal** (already in
  `paladin-llm`, confirmed in-tree) and, for the cross-worker-shared half specifically, the
  existing `redis-queue` feature and its Lua-atomic-script pattern from `RunQueuePort`
  (Phase 27) — reuse, don't reinvent, the atomicity mechanism.
- **R6 (spend in heralds/CLI/traces) enhances existing Phase 28 observability infrastructure**
  rather than requiring new plumbing — it's additive fields/events on `TraceEvent`, the Herald
  trait implementations, and the CLI, not a new subsystem.

## MVP Definition — Scoped to This Milestone (Not a Greenfield Product)

This is a subsequent-milestone feature set with the "full governance" decision already made
(operator-confirmed 2026-09-24, per PROJECT.md) — R3/R4 are **full**, not minimal. There is no
separate "launch vs defer" MVP question the way a greenfield product would have one; the PRD's
R1-R6 *are* the scoped v0.11.0 deliverable. What the comparable-system survey adds is sequencing
and scope-guarding advice within that already-fixed scope:

### In Scope (R1-R6, as PRD-specified)
- [ ] R1 — per-model price table (config-only, nothing bundled)
- [ ] R2 — `ExecutionMetadata.cost_estimate` producer, end-to-end
- [ ] R3 — Treasurer service: admission refusal + in-flight resumable halt, full per-tenant/
      per-key rolling + lifetime-cap allowances
- [ ] R4 — rate pacing on 429/Retry-After, Redis-shared across workers, cache-stampede lock
- [ ] R5 — durable spend ledger (port + in-memory/SQLite/Postgres adapters)
- [ ] R6 — docs (mdBook Treasurer page, config docs, MIGRATION entries)

### Worth Flagging to the Roadmap (Present in Every Comparable System, Not Literally in the PRD)
- [ ] Soft/warn threshold before hard block — every survey system has this two-tier pattern
      (LiteLLM `soft_budget`, Helicone's graduated 50/80/95%); the PRD's R3 wording only says
      "refused at admission," which reads as hard-only. Worth a roadmap-phase clarifying
      question rather than silently adding or silently omitting it.
- [ ] Explicit "unpriced model → cost_estimate stays None, never a guessed number" as a tested
      behavior, not just a rustdoc note — this matches Portkey's fail-open-to-null pattern and
      avoids LiteLLM's riskier fallback-to-input-rate guess for cache tokens.

### Explicitly Out of Scope (Anti-Features, Do Not Let Scope Creep In)
- [ ] Pre-flight exact per-call cost prediction (impossible before the response returns; not
      done by any comparable system)
- [ ] Bundled/default price table (PRD R1 already says "nothing bundled" — reinforced by survey)
- [ ] Billing/payment integration (wallet top-ups, Stripe-style purchasing)
- [ ] A hosted spend-visualization dashboard/UI
- [ ] Retroactive allowance enforcement against historical runs

## Competitor Feature Analysis

| Feature | LiteLLM (closest analog) | Helicone / Portkey (SaaS gateways) | Paladin's Approach |
|---------|---------------------------|-------------------------------------|---------------------|
| Pricing table shape | JSON file, per-model, per-token-type fields incl. tiered cache-above-200k | Model Registry (gateway) or open cost repo (direct) | Operator-configured config key, no bundled defaults (deliberate divergence) |
| Budget scope | key / user / team / org, hierarchical | team / endpoint / user (Portkey); user / key (Helicone) | tenant + API key (PRD-scoped); hierarchical composition not required by PRD but note LiteLLM's precedent if roadmap wants it later |
| Soft vs hard | `soft_budget` alert-only vs hard block | graduated 50/80/95% thresholds | not explicit in PRD R3 — flagged above as a roadmap question |
| In-flight handling | N/A (stateless gateway, just 403s the call) | N/A (same) | Genuinely novel: checkpoint-then-halt-then-resume, built on Waypoint/Aegis — no prior art to copy |
| Rate pacing | Not a first-class feature (relies on provider errors bubbling up) | Rate limiting is request/cost-count based, not 429-reactive pacing | R4: explicit 429/Retry-After-driven back-off, Redis-shared — closer to raw OpenAI/Anthropic SDK guidance than to any gateway's approach |
| Ledger persistence | Proxy's own DB (Postgres typically) | Hosted (SaaS) | New port, in-memory/SQLite/Postgres adapters mirroring `RunRepositoryPort` — framework-appropriate (caller chooses backend), not SaaS-appropriate (fixed hosted DB) |

## Sources

- [Budgets, Rate Limits — LiteLLM Docs](https://docs.litellm.ai/docs/proxy/users)
- [Spend Tracking — LiteLLM Docs](https://docs.litellm.ai/docs/proxy/cost_tracking)
- [Budget / Rate Limit Tiers — LiteLLM Docs](https://docs.litellm.ai/docs/proxy/rate_limit_tiers)
- [Custom LLM Pricing — LiteLLM Docs](https://docs.litellm.ai/docs/proxy/custom_pricing)
- [Add Model Pricing & Context Window — LiteLLM Docs](https://docs.litellm.ai/docs/provider_registration/add_model_pricing)
- [LLM Context Cost Modeling & Tiered Pricing Over 200k Tokens](https://www.sitepoint.com/modeling-llm-context-costs-tiered-pricing-beyond-200k-tokens/)
- [OpenRouter FAQ](https://openrouter.ai/docs/faq)
- [OpenRouter Pricing: Fees, Credits & BYOK Explained — Amnic](https://amnic.com/blogs/openrouter-pricing)
- [Budget Limits — Portkey Docs](https://portkey.ai/docs/product/ai-gateway/virtual-keys/budget-limits)
- [Rate limiting for LLM applications — Portkey Blog](https://portkey.ai/blog/rate-limiting-for-llm-applications/)
- [Take control of your AI costs — Portkey](https://portkey.ai/for/manage-and-attribute-costs)
- [Cost Tracking & Optimization — Helicone Docs](https://docs.helicone.ai/guides/cookbooks/cost-tracking)
- [How to Monitor Your LLM API Costs and Cut Spending by 90% — Helicone Blog](https://www.helicone.ai/blog/monitor-and-optimize-llm-costs)
- [Budget alerts — Helicone Advanced Course — The Neural Base](https://theneuralbase.com/helicone/learn/advanced/budget-alerts/)
- [Cost tracking — Docs by LangChain (LangSmith)](https://docs.langchain.com/langsmith/cost-tracking)
- [Token & Cost Tracking — Langfuse Docs](https://langfuse.com/docs/observability/features/token-and-cost-tracking)
- [Pricing Tiers for Accurate Model Cost Tracking — Langfuse Changelog](https://langfuse.com/changelog/2025-12-02-model-pricing-tiers)
- [LLM Cost Management: Track and Control Spend — Langfuse](https://langfuse.com/resources/engineering/llm-cost-management)
- [Rate limits — OpenAI API Docs](https://developers.openai.com/api/docs/guides/rate-limits)
- [OpenAI API Rate Limits + 429 Handling: Engineer's Guide (2026) — Respan](https://www.respan.ai/articles/openai-api-rate-limits)
- [Anthropic 429 / 529 in Production: How to Handle Rate Limits (2026) — Respan](https://www.respan.ai/articles/anthropic-api-rate-limits)
- [Claude API 429 Error Handling — SitePoint](https://www.sitepoint.com/claude-api-429-error-handling-python/)
- [API Throttling: Algorithms, Patterns & Mistakes — Redis Blog](https://redis.io/blog/api-throttling-algorithms-patterns/)
- [Build 5 Rate Limiters with Redis — Redis Tutorials](https://redis.io/tutorials/howtos/ratelimiting/)
- [How to Build Cache Stampede Prevention](https://oneuptime.com/blog/post/2026-01-30-cache-stampede-prevention/view)
- [LLM API Pricing 2026: Every Model's Cost Per Token, Compared — Morph](https://www.morphllm.com/llm-api-pricing)
- Confidence note: all findings above are **MEDIUM** (web search, cross-checked across ≥3
  independent products per claim, per `gsd-tools query classify-confidence --provider websearch
  --verified`). No curated/official-vendor-docs (HIGH-tier) source was consulted for OpenAI's or
  Anthropic's own header-naming beyond what surfaced in web search results; treat exact header
  names (`x-ratelimit-remaining-requests`, `anthropic-ratelimit-*`) as directionally correct but
  worth a direct docs.anthropic.com / platform.openai.com check before implementation.
- Paladin in-tree grounding (not web research, direct grep): `LlmError::RateLimitExceeded`,
  `ExecutionMetadata::cost_estimate` (reserved dead field), and the `RunQueuePort` Redis
  Lua-atomic lease-claim pattern all confirmed present in `crates/paladin-llm`,
  `crates/paladin-core`, and the Phase 27 platform-api work respectively.

---
*Feature research for: LLM spend governance / cost tracking, Paladin v0.11.0 Treasurer milestone*
*Researched: 2026-09-24*
