# Phase 43: Rate Pacing - Context

**Gathered:** 2026-10-07
**Status:** Ready for planning
**Mode:** interactive — every decision below was selected by the operator (four gray areas,
thirteen questions, logged in `43-DISCUSSION-LOG.md`). The recommended option was chosen in
eleven; the fallback-chain question was re-opened by the operator and settled on a new option,
and the persona/feature names are the operator's own.

<domain>
## Phase Boundary

This phase delivers the **pacing half of the Treasurer** (FUT-09, PRD R4): outbound LLM calls
back off on provider 429s instead of thrashing, in-process and across the whole worker fleet,
and no run is ever left unpaced. Concretely:

1. **Typed retry delay** (PACE-01). `LlmError::RateLimitExceeded` carries a retry delay parsed
   from `Retry-After` (delta-seconds or HTTP-date) and from the provider rate-limit headers,
   header names verified against official OpenAI and Anthropic documentation.
2. **In-process pacing decorator** (PACE-02). A `CadenceLlmAdapter` `LlmPort` decorator in
   `paladin-llm` paces each `(provider, model)` key, honouring the retry delay as a minimum and
   backing off with jitter when the provider gives none, proven against a mocked 429 on a
   `FallbackLlmAdapter` hop.
3. **Fleet-wide pacing through Redis** (PACE-03). With Redis configured, a 429 on one worker
   slows every worker sharing that key, through an atomic Lua script that reads the Redis server
   clock.
4. **Distributed stampede lock** (PACE-04). Set-if-absent with expiry, a fencing token and
   delete-only-if-owner stop concurrent workers from issuing the same cached request twice.
5. **Degradation** (PACE-05). When Redis is unavailable, pacing degrades to conservative
   per-process pacing with a trace warning; a run is never unpaced, covered by a test.

Not in this phase: proactive pacing from remaining/reset headers on successful responses
(deferred, see below), any change to the allowance/admission contract (Phase 41), the halt
contract (Phase 42), tenant-aware or per-API-key pacing (no requirement), the Treasurer mdBook
page (Phase 46 CURR-23), and crate release mechanics (Phase 47).

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by ADR-0052 and Phases 38-42 — cited, not re-asked)

- **D-00a:** Phase 38 D-09 placed cost metering in a stateless `LlmPort` decorator in
  `paladin-llm` (`crates/paladin-llm/src/pricing.rs`, sibling shape to
  `crates/paladin-llm/src/fallback.rs`) and named it "the natural home for the Phase 43 pacing
  decorator". The pacing decorator is that sibling: same crate, same `Arc<dyn LlmPort>`
  decorator shape, same identity-method delegation, same `with_*` constructor idiom
  (`with_pricing` in `pricing.rs`).
- **D-00b:** Phase 38 D-07 reserved `treasurer.pacing`-style keys under the operator's
  `treasurer:` config section (`src/config/treasurer.rs`); this phase adds its subtree there, as
  Phase 41 added `allowance`. Every struct under `treasurer:` rejects unknown keys.
- **D-00c:** Phase 26's "middleware never retries itself" rule and the `RetryPolicy` ownership of
  retries (engine nodes via `crates/paladin-battalion/src/engine/retry.rs`; the agent loop via
  `src/application/services/paladin/middleware/resilience.rs`) stand. The Phase 25 `Transience`
  classification (`LlmError::transience`, `crates/paladin-ports/src/output/llm_port.rs`) stays:
  a 429 is `Transient`.
- **D-00d:** `NodeCachePort`'s best-effort contract (D-29: a cache failure never fails a run,
  `crates/paladin-ports/src/output/node_cache_port.rs`) is untouched by this phase.
- **D-00e:** Phase 41's `429 allowance_exhausted` HTTP contract with `Retry-After` (41 D-12/D-13)
  is a different 429 — the API's own admission refusal, not a provider rate limit — and this phase
  never touches it or reuses its error type.
- **D-00f:** The redis adapter idiom is fixed by `crates/paladin-storage/src/run_queue/redis.rs`
  and `crates/paladin-storage/src/node_cache/redis.rs`: one `redis::Script` `EVAL` per atomic
  operation, "now" from the server's own `TIME` command, never the client clock; a shared
  optional `redis` dependency behind a per-adapter Cargo feature (`redis-queue`, `redis-cache`);
  config as `backend: in_memory | redis { url_env }` with the URL read from the named env var at
  boot (`src/config/run_queue.rs`, `src/config/node_cache.rs`).

### Back-off ownership

- **D-01:** **The decorator gates; the existing layers retry.** On a 429, `CadenceLlmAdapter`
  records a per-`(provider, model)` not-before instant on the `CadencePort` (from the typed retry
  delay when the provider gave one, else exponential back-off with jitter), then surfaces the
  `RateLimitExceeded` **unchanged**. Every later call to that key, from any task in the process
  (or any worker, with Redis), waits until the instant passes before it is sent. The decorator
  never re-issues a request and never sleeps on the failing call itself; `RetryPolicy` and
  `FallbackLlmAdapter` keep owning retries, so attempt counts are never multiplied by a third
  loop. — **Reversibility:** costly — the decorator's no-retry contract is what every retry
  predicate and attempt-count test above it assumes.
- **D-02:** **Provider adapters stop retrying 429 inside their own loops.** The OpenAI adapter
  (`crates/paladin-llm/src/openai/adapter.rs`, the `0..=max_retries` loop), the compat engine
  (`crates/paladin-llm/src/compat/engine.rs`, `call_api_with_retry`) and the Anthropic adapter
  surface `RateLimitExceeded` on the **first** 429. Their network and 5xx retries are untouched.
  Verified by one per-adapter conformance case in `crates/paladin-llm/src/conformance.rs` (a
  mocked 429 is observed exactly once); the existing 429-mapping tests keep passing. Rationale:
  one layer sees every 429 the instant it happens, so `Retry-After` is honoured on the first hit
  and no thrash hides below the decorator. — **Reversibility:** reversible.
- **D-03:** **Fallback chains pace first, hop last.** A 429 inside a `FallbackLlmAdapter` hop does
  not hop immediately. The chain retries the same provider after its gate clears, up to a
  configurable cumulative wait ceiling (`treasurer.cadence.fallback_pace_budget_secs`, default on
  the order of 60s); only once that budget is exhausted does the 429 reach the hop rule and the
  chain move to the next provider. 5xx, timeouts and network errors hop immediately as today.
  Rationale (operator): a run that silently changes provider mid-run breaks benchmark
  consistency, shifts tool-call formats and token accounting, and forfeits the provider's prompt
  cache, while a 429 loses no provider-side state at all (every request carries the full
  history); pacing must actually learn the provider's rate rather than sidestep it.
  — **Reversibility:** costly — `FallbackLlmAdapter`'s "stateless, never retries a hop" docs and
  tests (`crates/paladin-llm/src/fallback.rs`, module docs and `hops_eventually`) must be
  rewritten for the one 429 exception, and the hop trace contract (28-06 D-03) is published.

### Pacing semantics & degradation

- **D-04:** **Reactive only.** A key is gated only after a 429. The typed retry delay sets the
  gate when present; otherwise exponential back-off with full jitter from
  `base_backoff_ms`, doubling per consecutive 429 on that key up to `max_backoff_ms`, reset to
  zero consecutive on the first success. Provider remaining/reset headers are parsed and carried
  on the error (PACE-01) but never gate on their own. — **Reversibility:** reversible — proactive
  gating is additive (deferred below).
- **D-05:** **Degraded mode is the in-process gate with a stricter multiplier.** When
  `backend: redis` is configured and Redis is unreachable, the decorator falls back to the
  in-process `CadencePort` for the same keys and multiplies every computed delay by
  `degraded_multiplier` (default 2.0), since this worker can no longer see the fleet's 429s.
  Exactly one trace warning per outage (not per call), and automatic recovery on the next
  successful Redis round-trip. A run is never unpaced. — **Reversibility:** reversible.
- **D-06:** **A huge retry delay is capped at the gate and surfaced beyond it.** The gate waits
  at most `max_wait_secs` (default a few minutes). A delay beyond that is not slept on: the call
  surfaces `RateLimitExceeded` carrying the full delay immediately, so `RetryPolicy` exhausts or
  the run halts with an actionable, typed error rather than tying up a worker lease for an hour
  in silence. — **Reversibility:** reversible.

### Naming, config & defaults

- **D-07:** **Persona: Cadence.** The one who sets the marching pace. The port is `CadencePort`
  (`crates/paladin-ports/src/output/cadence_port.rs`), the decorator `CadenceLlmAdapter`
  (`crates/paladin-llm/src/cadence.rs`), the config subtree `treasurer.cadence`, the Cargo
  feature `redis-cadence`, the log/trace target `paladin::cadence`. "Cadence" joins the
  ubiquitous-language table beside Treasurer and Commissary (update
  `.github/copilot-instructions.md`'s term table and the Phase 42 vocabulary guard test if it
  enumerates framework words). — **Reversibility:** one-way — the port, decorator and feature
  names are published Rust API and the config key is operator-facing; renaming later needs an
  X-10 register row and a config migration.
- **D-08:** **On by default, in-process.** Omitting `treasurer.cadence` yields in-process pacing
  with the default back-off on every `LlmPort` the server composes (agent host, facade
  provisioner, worker run wiring), so no deployment is unpaced. Redis sharing is opt-in via
  `backend: redis { url_env }`. An explicit `enabled: false` turns the decorator off (nothing
  installed). — **Reversibility:** costly — an on-by-default behaviour change is a §9.1 MIGRATION
  row, and turning it back to opt-in would again be one.
- **D-09:** **Redis adapter in `paladin-storage`, feature `redis-cadence`.** The adapter lives
  beside `node_cache/redis.rs` and `run_queue/redis.rs` under
  `crates/paladin-storage/src/cadence/{mod.rs,in_memory.rs,redis.rs,contract_tests.rs}`, shares
  the already-declared optional `redis` dependency, and is gated by a new `redis-cadence` feature
  of the same shape as `redis-cache`/`redis-queue`. The stampede lock ships under the same
  feature. `paladin-llm` gains no Redis dependency. — **Reversibility:** costly — the feature name
  is a published Cargo surface.
- **D-10:** **Config shape.** `treasurer.cadence { enabled: bool = true, backend: in_process |
  redis { url_env }, base_backoff_ms = 500, max_backoff_ms = 30000, max_wait_secs = 300,
  degraded_multiplier = 2.0, fallback_pace_budget_secs = 60, lock_ttl_secs = 60 }`. Numeric
  defaults are the planner's to tune within the stated orders of magnitude; `validate()` rejects
  zero/negative durations, a multiplier below 1.0, and a `redis` backend whose `url_env` is empty
  or unset at boot, exactly like `RunQueueConfig::validate`. Env overrides follow
  `EnvOverridable` (`APP_TREASURER_CADENCE_*`) for scalar fields only.

### Stampede lock placement

- **D-11:** **The lock lives on `CadencePort`; the engine calls it.** `CadencePort` gains
  `try_lock(key, ttl) -> Result<Option<FencingToken>, _>` and `unlock(key, token)`. The engine's
  `NodeCacheBinding` (`crates/paladin-battalion/src/engine/superstep.rs`, `lookup_node_cache` /
  `store_node_cache`) takes an optional cadence handle; on a cache miss it tries the lock before
  executing the node and releases it after the `put`. `NodeCachePort` is unchanged (D-00d).
  — **Reversibility:** costly — the engine's cache-miss path and the port's method set are
  published.
- **D-12:** **A lock loser waits and re-reads, then executes.** It polls `cache.get` at a short
  interval until the entry appears or the lock TTL elapses, then serves the hit. If the TTL passes
  with no entry (holder crashed or its node failed), it executes uncached rather than stall or
  error. Bounded wait, never a duplicate provider call in the common case.
- **D-13:** **The lock fails open onto an in-process lock.** With no Redis configured, or Redis
  unreachable, the same `try_lock`/`unlock` run against a per-process lock keyed identically, so
  tasks in one worker still coalesce while separate workers may duplicate. One trace warning per
  outage. The node always executes; caching stays best-effort (D-29). Fail-closed was rejected
  because the node cache exists purely as an optimisation and must never fail a run.
- **D-14:** **TTL from config, fencing token from a Redis counter.** `lock_ttl_secs` (default 60s,
  longer than a typical node call) sets the `SET NX PX` expiry. The fencing token is a
  monotonically increasing integer from `INCR` on a per-key counter; `unlock` is a Lua
  delete-only-if-token-matches. The in-process adapter uses an `AtomicU64`. The token is also
  handed to the Redis `NodeCachePort` adapter's `put` path so a stale holder's late write (token
  lower than the last seen for that key) is ignored.

### Claude's Discretion

- **PACE-01 shape of the typed delay.** `RateLimitExceeded` is today a unit variant matched in 26
  files across the workspace; whether it becomes a struct variant (`RateLimitExceeded { retry_after:
  Option<Duration>, provider_hint: .. }`) or carries the delay on a sibling value is the planner's
  call, provided: the header names are verified against the official OpenAI and Anthropic docs
  and quoted in rustdoc; delta-seconds and HTTP-date forms both parse; an unparseable header
  yields `None`, never a guess; and the change is registered as a §9.2 X-10 row in
  `MIGRATION.md` with `make api-surface-update` and a CHANGELOG entry.
- **Gate wait mechanics.** Whether the gate is a `tokio::time::sleep_until` per call or a
  per-key `Notify`, how the wait is observed (one `trace`-level event per gated call naming the
  key and delay), and the jitter distribution (full jitter preferred).
- **Lua script layout.** One script per operation (`record_429`, `next_allowed`, `try_lock`,
  `unlock`) mirroring `run_queue/redis.rs`; key namespace `paladin:cadence:<provider>:<model>`;
  TTLs on every key so an idle provider leaves no residue.
- **Where the decorator composes.** `Cadence(Pricing(Fallback(..)))` versus
  `Pricing(Cadence(Fallback(..)))`: pricing must still see the served response, and cadence must
  wrap each hop for D-03 to work; the planner picks the exact nesting and documents it in the
  module docs the way `pricing.rs` documents its own placement.
- **Contract-test shape.** A `contract_tests.rs` run against in-memory and Redis adapters, as
  `run_queue` and `node_cache` do, with the Redis half behind the existing `redis-integration`
  CI job.
- **Default numeric values** within the orders of magnitude in D-10.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Locked design (cite, do not re-open)
- `.planning/decisions/0052-mid-run-treasurer-enforcement.md` — cost and enforcement metered at
  the `LlmPort` boundary on both run paths; the decorator placement this phase reuses.
- `.planning/decisions/0050-treasurer-reservation.md` — the Treasurer's role and vocabulary.
- `.planning/decisions/0049-commissary-design-and-rename.md` — how a persona was named and
  introduced into the ubiquitous language (precedent for "Cadence").
- `.planning/decisions/0056-allowance-admission-model.md` — the API's own `429
  allowance_exhausted` contract, which this phase must not conflate with a provider 429.
- `.planning/decisions/0057-mid-run-halt-contract.md` — the halt contract a capped delay (D-06)
  surfaces into.
- `.planning/phases/38-design-seams-pricing-cost-producer/38-CONTEXT.md` — D-07 (config
  section), D-09 (decorator home).
- `.planning/phases/42-mid-run-halt-sse-terminal-status/42-CONTEXT.md` — carried-forward
  pattern, vocabulary guard test, SSE/halt behaviour a surfaced 429 interacts with.

### Milestone scope and requirements
- `.planning/ROADMAP.md` — Phase 43 goal, success criteria 1-5, research flag.
- `.planning/REQUIREMENTS.md` — PACE-01..PACE-05.
- `.project/Milestone_14-Treasurer/Epic_1/prd-treasurer-spend-governance.md` — R4 (FUT-09),
  "pacing under a mocked 429 asserts back-off, not thrash".
- `.planning/PROJECT.md` §Current Milestone — "Rate pacing (FUT-09, R4)" bullet.

### Breaking-change register and conventions
- `MIGRATION.md` §9.1 (behavioural: pacing on by default), §9.2 (X-10 register: the
  `RateLimitExceeded` change and the new port/decorator), §9.5 (configuration:
  `treasurer.cadence`), §9.3 (new `redis-cadence` feature).
- `.github/copilot-instructions.md` — naming-convention table (add Cadence).
- `.github/instructions/security.instructions.md` — manual credential review: the Redis URL is
  read from an env var and never logged; error bodies are redacted before truncation.

### Code the phase extends (read, do not re-derive)
- `crates/paladin-ports/src/output/llm_port.rs` — `LlmError::RateLimitExceeded` (unit variant),
  `transience()` classification, the rustdoc that already promises "retry after delay (check
  Retry-After header)".
- `crates/paladin-llm/src/pricing.rs` — the decorator shape to copy (`with_pricing`, identity
  delegation, warn-once set bounded at 256 entries, `PRICING_LOG_TARGET`).
- `crates/paladin-llm/src/fallback.rs` — hop rule (`Transient`/`Unknown` hops, `Permanent`
  stops), `record_hop`, `TraceEvent::FallbackHop`; the one place D-03 changes.
- `crates/paladin-llm/src/openai/adapter.rs`, `crates/paladin-llm/src/compat/engine.rs`,
  `crates/paladin-llm/src/anthropic/adapter.rs` — `map_error` (status + body only today; headers
  are not passed in) and the internal retry loops D-02 narrows.
- `crates/paladin-llm/src/conformance.rs` — the shared adapter conformance suite D-02 extends.
- `crates/paladin-battalion/src/engine/retry.rs` — `RetryPolicy`, `backoff_delay` (reuse the
  jitter idiom, do not duplicate it).
- `crates/paladin-battalion/src/engine/superstep.rs` — `NodeCacheBinding`,
  `lookup_node_cache`, `store_node_cache`: the miss-to-put window D-11 locks.
- `crates/paladin-ports/src/output/node_cache_port.rs` — D-29 best-effort contract (unchanged).
- `crates/paladin-storage/src/run_queue/redis.rs`, `crates/paladin-storage/src/node_cache/redis.rs`
  — Lua `EVAL` + server `TIME` idiom, feature gating, contract-test layout.
- `src/config/treasurer.rs`, `src/config/run_queue.rs`, `src/config/node_cache.rs` — config
  idiom (`Default` + `validate()` + `EnvOverridable`, `backend { url_env }`).
- `src/infrastructure/web/agent_host.rs` (`build_agent`), `src/infrastructure/web/facade_provisioner.rs`,
  `src/infrastructure/web/run_api_wiring.rs` — the three places `with_pricing` composes today;
  cadence composes at the same points.
- `src/application/services/paladin/middleware/resilience.rs` — agent-loop `RetryPolicy`
  middleware; its predicate keeps treating a 429 as retryable.
- `crates/paladin-core/src/platform/container/trace.rs` — `TraceEvent` enum for the
  degraded-mode warning and gated-call events.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `PricingLlmAdapter` / `with_pricing` (`crates/paladin-llm/src/pricing.rs`): the exact decorator
  skeleton, including the bounded warn-once set that D-05's one-warning-per-outage can reuse.
- `backoff_delay` (`crates/paladin-battalion/src/engine/retry.rs`): exponential back-off with
  optional jitter already implemented and tested; the cadence gate should call it or lift its
  arithmetic rather than write a second one.
- Redis Lua idiom (`run_queue/redis.rs`): `redis::Script`, `TIME`-sourced now, atomic single
  `EVAL`, contract tests parameterised over in-memory and Redis adapters.
- `RunQueueConfig` / `NodeCacheConfig`: the `backend` enum with `url_env` and the boot-time
  validation that the env var is set.
- `TraceEvent::FallbackHop` plumbing (`fallback.rs`, 28-06 D-03): the pattern for emitting a
  port-level trace event without a node id, reusable for gated-call and degraded-mode events.

### Established Patterns
- Decorators are stateless apart from immutable config and bounded process-wide sets;
  cross-call state lives behind a port with in-memory and Redis adapters — cadence follows this
  (decorator holds `Arc<dyn CadencePort>`).
- Middleware and decorators never retry themselves; `RetryPolicy` owns attempts (Phase 26).
- Adapters map errors from `(status, body)` only. PACE-01 needs the response headers at the
  mapping site, so each adapter's error path must start passing them; redact before truncating
  (`crates/paladin-llm/src/redaction.rs`).
- Every `treasurer:` sub-struct is `deny_unknown_fields`, `Default` is inert, `validate()` names
  the offending key.
- A new Cargo feature on `paladin-storage` shares `dep:redis`; CI's `redis-integration` job runs
  the Redis halves of the contract suites.

### Integration Points
- `LlmError::RateLimitExceeded` is matched in 26 files (adapters, eval `LlmErrorKind`, resilience
  tests, web error mapping); the variant change is a §9.2 X-10 row and an `api-surface` refresh.
- `FallbackLlmAdapter::generate` / `generate_stream`: the hop rule gains the D-03 pace budget.
- `NodeCacheBinding` in `superstep.rs`: optional cadence handle for the stampede lock; the
  `WarEngine::with_node_cache` builder grows a `with_cadence` sibling.
- `build_agent`, `facade_provisioner`, `run_api_wiring`: compose `with_cadence` beside
  `with_pricing`, from `Settings.treasurer.cadence`.
- `src/config/treasurer.rs`: new `cadence: CadenceConfig` field on `TreasurerConfig`.
- Docs: `docs/src/getting-started/configuration.md` (new subtree),
  `docs/src/deployment-topologies/http-service-host.md` (Redis for fleet pacing), ADR index.

</code_context>

<specifics>
## Specific Ideas

- The operator's framing for D-03: "we're trying to handle 429s so that they effectively retry
  the same provider after a delay"; switching providers on the first 429 would "cause all kinds of
  confusion when trying to get consistent results with benchmarks". A 429 loses no context
  because every request re-sends the full history, so the cost of hopping is consistency and the
  prompt cache, not state.
- The operator named the persona "Cadence" and the feature `redis-cadence` explicitly.
- Success criterion 2 is the acceptance test for D-01..D-03 together: a mocked 429 on a
  `FallbackLlmAdapter` hop proves the retry delay is honoured as a minimum, with jitter, and that
  the chain stays on the same provider until the pace budget is spent.

</specifics>

<deferred>
## Deferred Ideas

- **Proactive pacing from remaining/reset headers** — read `x-ratelimit-remaining-*` /
  `anthropic-ratelimit-*-remaining` on successful responses and start spacing calls before the
  first 429. The header parse ships in this phase (PACE-01) so the data is available; gating on
  it is a future phase.
- **Resolve-then-connect pinning for the Redis URL** is not needed (operator-configured, not
  caller-chosen), noted only so the SSRF guard precedent is not misapplied here.
- **Per-tenant or per-API-key pacing** — no requirement; pacing keys are `(provider, model)`.

</deferred>

---

*Phase: 43-rate-pacing*
*Context gathered: 2026-10-07*
