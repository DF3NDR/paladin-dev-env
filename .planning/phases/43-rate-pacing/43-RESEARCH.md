# Phase 43: Rate Pacing - Research

**Researched:** 2026-10-07
**Domain:** Rust async LLM-call pacing (Cadence): provider 429 header parsing, an `LlmPort` gating decorator, a Redis-backed fleet-wide pacing/lock port with in-process fallback, engine-side cache-stampede lock
**Confidence:** MEDIUM-HIGH (codebase grounding and Anthropic headers HIGH; OpenAI header names MEDIUM because the official page could not be fetched directly from this sandbox, see Assumptions Log A1/A2)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried forward (locked by ADR-0052 and Phases 38-42 — cited, not re-asked)

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

#### Back-off ownership

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

#### Pacing semantics & degradation

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

#### Naming, config & defaults

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

#### Stampede lock placement

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

### Deferred Ideas (OUT OF SCOPE)

- **Proactive pacing from remaining/reset headers** — read `x-ratelimit-remaining-*` /
  `anthropic-ratelimit-*-remaining` on successful responses and start spacing calls before the
  first 429. The header parse ships in this phase (PACE-01) so the data is available; gating on
  it is a future phase.
- **Resolve-then-connect pinning for the Redis URL** is not needed (operator-configured, not
  caller-chosen), noted only so the SSRF guard precedent is not misapplied here.
- **Per-tenant or per-API-key pacing** — no requirement; pacing keys are `(provider, model)`.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PACE-01 | `LlmError::RateLimitExceeded` carries a retry delay parsed from `Retry-After` (delta-seconds or HTTP-date) and from provider rate-limit headers, header names verified against official OpenAI and Anthropic docs | Verified header tables (Standard Stack / Header Reference); `httpdate` for all three HTTP-date forms; closure-based header lookup so `http_status.rs` stays un-gated; variant-shape recommendation (`#[non_exhaustive]` struct variant + constructor); 86 occurrences / 26 files enumerated; Pitfalls 1, 2, 3 |
| PACE-02 | `LlmPort` pacing decorator paces each provider/model in-process, backs off with jitter on a 429, retry delay is a minimum, wraps every `FallbackLlmAdapter` hop, mocked-429 test proves back-off not thrash | `CadenceLlmAdapter` design, gate semantics, in-flight de-escalation rule, composition (per-hop wrapping via `FallbackLlmAdapter::with_cadence` and/or `LlmProviderFactory`), D-02 scope gaps (DeepSeek, Gemini own retry loops), paused-clock test idiom, Pitfalls 4-8 |
| PACE-03 | With Redis configured, pacing state is shared across workers through an atomic Lua script using the Redis server clock | Lua scripts prototyped and run against local Redis 7.0.15 (`record_429`, `gate`); `ConnectionManager` is `Clone`; relative-duration return avoids cross-host clock math; Pitfalls 9-11 |
| PACE-04 | Distributed cache-stampede lock: set-if-absent with expiry, fencing token, delete-only-if-owner | `try_lock`/`unlock` Lua prototyped and run; engine integration points in `superstep.rs`; fencing-needs-resource-side-check finding and D-00d/D-14 tension (Open Question 2); loser loop re-tries the lock; Pitfalls 12-15 |
| PACE-05 | Redis unavailable: degrade to conservative per-process pacing with a trace warning, never unpaced, covered by a test | Composite adapter in `paladin-storage` (primary + in-process fallback + outage latch), redis crate defaults have NO timeouts (verified) so explicit timeouts are mandatory, injected-failing-port test needs no Redis, Pitfalls 9, 10 |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

Extracted actionable directives (treated as locked, same authority as CONTEXT.md):

- **TDD red-green-refactor**; workspace line-coverage floor **82%** (`cargo llvm-cov --fail-under-lines`, ADR-0006); all public APIs need doc tests (note: `paladin-llm` and `paladin-storage` set `[lib] doctest = false`, so doc tests apply to `paladin-ports` and the root crate; follow the existing per-crate convention).
- **Dependencies flow inward only**: core -> nothing; ports -> core; adapters -> core + ports. `paladin-llm` and `paladin-storage` must not depend on `paladin-battalion`; `paladin-battalion` must not import infrastructure.
- **Ubiquitous language**: use Medieval Military terms; add **Cadence** to the table in `.github/copilot-instructions.md`.
- **Before committing a parent task**: `cargo test` -> `cargo fmt --check` -> `cargo clippy` (`-- -D warnings` for security) -> `make api-surface` (intentional surface change: `make api-surface-update` + CHANGELOG entry) -> conventional commit. Stop after each major task.
- **Security**: `make security` (cargo-audit + cargo-deny); manual credential-handling review per `security.instructions.md` (redact before truncating response bodies; no log of keys; no `Debug` of credential-bearing config; HTTP clients that send credential headers must not follow redirects). CodeQL is advisory only; **do not reintroduce Snyk**.
- No `unwrap()`/`expect()`/`panic!` in library code; prefer borrowing; keep iterators lazy; `thiserror` error enums; `Send + Sync` ports with `#[async_trait]`; rustdoc on all public items; all public types `Debug`; structs have private fields where feasible; lines under 100 chars.
- Lock poisoning must be recovered (`PoisonError::into_inner`) — the `pricing.rs` precedent — never propagated or unwrapped.

## Summary

Phase 43 adds a reactive, key-scoped (`(provider, model)`) pacing layer. The architecture is already fully decided; the research task was to ground it in the actual code and find where the decisions collide with reality. The shape that falls out is: a pure-data `RateLimitHints`/delay parse step at each adapter's non-2xx path (headers must be snapshotted before `response.text()` consumes the response); a new `CadencePort` (gate / record_rate_limited / record_success / try_lock / unlock) in `paladin-ports`; an in-process adapter and a Redis adapter (plus a composite that owns the outage latch) in `paladin-storage`; a `CadenceLlmAdapter` in `paladin-llm` that waits on the gate before sending and records 429s after; a pace-first branch in `FallbackLlmAdapter`; and an engine-side lock wrapped around the node-cache miss path.

The Lua semantics were prototyped and executed against a local Redis 7.0.15 in this session: `TIME` followed by writes works (same precedent as `run_queue`), a late in-flight 429 inside an active gate does not escalate the streak, an explicit `Retry-After` raises (never lowers) the gate, lock acquisition returns monotonically increasing integer tokens via `INCR`, a held lock refuses a second acquirer, and the owner-checked unlock refuses a wrong token. All gate arithmetic returns a *relative* wait (microseconds) computed against the server clock, so no cross-host clock comparison ever happens on the client.

Five findings the CONTEXT does not anticipate and the planner must resolve (details in Open Questions): (1) D-02 names three adapters, but **DeepSeek and Gemini have their own 429-retrying loops and both sit in the conformance suite**, so the new "observed exactly once" case would fail for them unless they are included, and **Anthropic is not in the conformance macro at all** (needs a bespoke test); (2) **D-14 requires the fencing token to reach `NodeCachePort::put`, which contradicts D-00d "NodeCachePort unchanged"** — a default-method addition resolves it additively; (3) **fallback chains built by config (`resolve_chain` -> `ModelFallbackMiddleware`) replace the service's port via `llm_override`**, so decorators applied only in `build_agent` do not wrap fallback hops (the same pre-existing gap already exists for pricing) — per-hop wrapping must happen at the factory or the chain builder; (4) `backoff_delay` cannot be called from `paladin-llm`/`paladin-storage` (battalion is not a dependency of either, and the dependency arrow points the other way), and its jitter is additive "equal-ish" not full jitter; (5) the `redis` `ConnectionManager` defaults to **no response or connect timeout**, so a black-holed Redis would hang every LLM call unless timeouts are set explicitly — this is the single biggest PACE-05 risk.

**Primary recommendation:** Build bottom-up in four layers with a pure `CadencePolicy` math function in `paladin-ports` (no `rand`; caller injects a unit-interval jitter fraction), keep `CadenceLlmAdapter` a pure gate-then-delegate decorator, put the degraded-mode latch in a composite adapter in `paladin-storage` so PACE-05 is testable with an injected failing port and no Redis, and resolve the four contract-level questions (D-02 scope, D-14 fencing seam, per-hop composition site, `TraceEvent` vs `log`) before writing plans.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Parse `Retry-After` / provider rate-limit headers | `paladin-llm` (adapter edge, `http_status.rs` sibling) | `paladin-ports` (carries the typed value on `LlmError`) | Headers exist only at the HTTP boundary; the typed result is domain data every layer above consumes |
| Typed retry delay on `LlmError` | `paladin-ports` | — | `LlmError` lives in `llm_port.rs`; ports layer owns it |
| Pacing state machine (gate, streak, de-escalation) | `paladin-storage` adapters (in-memory + Redis) behind `CadencePort` | `paladin-ports` (pure `CadencePolicy` arithmetic) | Cross-call state lives behind a port with in-memory and Redis adapters (established pattern); decorators stay stateless |
| Waiting on the gate / recording 429s | `paladin-llm` (`CadenceLlmAdapter`) | — | Decorator at the `LlmPort` boundary, sibling to `PricingLlmAdapter` (D-00a) |
| Pace-first-hop-last | `paladin-llm` (`FallbackLlmAdapter`) | `paladin-ports` (`CadencePort` read of remaining gate) | The only place a hop decision is made |
| Fleet-wide shared state | Redis (via `paladin-storage` Redis adapter) | — | Atomic Lua + server `TIME` |
| Degraded-mode latch + one-warning-per-outage | `paladin-storage` composite adapter | `paladin-llm` (logs only) | Keeps the decorator oblivious; testable with an injected failing port |
| Stampede lock around cache miss | `paladin-battalion` engine (`superstep.rs`) | `paladin-ports` (`CadencePort::try_lock`) | The engine owns the miss-to-put window (D-11) |
| Fenced cache write | `paladin-storage` Redis `NodeCache` adapter | `paladin-ports` (`NodeCachePort` seam) | Fencing must be checked by the resource being written (see Pitfall 13) |
| Config (`treasurer.cadence`) + composition | root crate `src/config/treasurer.rs`, `src/infrastructure/web/*` | — | Composition roots; Settings is where the Redis URL env name is resolved |
| Redis URL handling | root crate config (env-var name only) | `paladin-storage` (receives resolved URL) | URL may carry a password; never stored on a Debug-able config type |

## Standard Stack

### Core

No new top-level crates are required except an optional promotion of one already-locked transitive crate.

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `redis` | 0.32.2 declared in `paladin-storage` (0.32.7 resolved in `Cargo.lock`) — features `aio, tokio-comp, connection-manager, script, safe_iterators` already enabled | `Script::invoke_async`, `ConnectionManager`, `cmd("TIME")` | Already the house dependency; `Script::invoke_async` auto-retries `NOSCRIPT` once (verified in `run_queue/redis.rs` module docs) [VERIFIED: codebase + registry source `redis-0.32.7`] |
| `httpdate` | 1.0.3 (already in `Cargo.lock` via hyper) | `parse_http_date` for all three RFC 7231 HTTP-date forms (IMF-fixdate, RFC 850, asctime) | What hyper itself uses; avoids chrono's RFC 2822 parser handling only IMF-fixdate [VERIFIED: `package-legitimacy check` -> OK, 11.1M weekly downloads, repo `pyfisch/httpdate`; registry source inspected] |
| `chrono` | 0.4.38 (workspace; already a dependency of `paladin-llm`) | `DateTime::parse_from_rfc3339` for Anthropic `anthropic-ratelimit-*-reset`; local wall clock for HTTP-date deltas | Already in tree [VERIFIED: codebase] |
| `rand` | 0.8 (workspace; **optional in `paladin-llm`**, behind provider features) | Jitter fraction draw | Already in tree; see Pitfall 8 for the feature-gating problem [VERIFIED: `package-legitimacy check` -> OK] |
| `tokio` | workspace `1` with `full` | `time::sleep`/`Instant` (paused clock in tests), `sync::Mutex`, `select!` | Existing [VERIFIED: codebase] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `mockito` | 1.7.0 (dev-dep of `paladin-llm`) | Mock 429 with `Retry-After` headers against real adapters for conformance cases | Conformance suite and per-adapter header-parse tests |
| `MockLlmAdapter` (`MockScriptEntry::Error`) | in-tree | Script `[429(retry_after), Text]` for decorator/fallback tests | Decorator and `FallbackLlmAdapter` pace-first tests |
| `serial_test` | in-tree (config tests) | Env-var override tests | `APP_TREASURER_CADENCE_*` tests |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `httpdate` | `chrono::DateTime::parse_from_rfc2822` (no new dependency edge) | Handles only IMF-fixdate; RFC 850 and asctime (which RFC 7231 says recipients MUST accept) would yield `None` — acceptable under "unparseable -> None", but weaker. Recommended: `httpdate`. |
| Hand-written Go-duration parser for OpenAI `x-ratelimit-reset-*` (`"6m0s"`, `"1s"`, `"6ms"`) | `humantime` crate | A ~25-line bounded parser with table tests is cheaper than vetting a new crate; its grammar is tiny. [ASSUMED that `humantime` would accept `6m0s` — not checked, so do not depend on it.] |
| Jitter from `rand` inside `paladin-ports` | Caller injects a unit-interval `f64` | Keeps `paladin-ports` dependency-free and makes the policy a pure, table-testable function. Recommended. |

**Installation:**
```bash
# paladin-llm/Cargo.toml
#   httpdate = "1.0"        # new direct dep (already in Cargo.lock)
#   rand: change `optional = true` -> required (see Pitfall 8), or draw jitter in the caller
# paladin-storage/Cargo.toml
#   redis-cadence = ["dep:redis"]   # new feature, same shape as redis-cache
#   rand = { workspace = true }     # for the in-process adapter's jitter, if drawn there
```

**Version verification:** `httpdate 1.0.3`, `redis 0.32.7`, `rand 0.8`, `chrono 0.4.x` read from `/home/user/paladin-dev-env/Cargo.lock` and the local registry source on 2026-10-07; none is a new package name to the workspace except promoting `httpdate` from transitive to direct.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| httpdate | crates.io | ~10 yrs (2016-10) | 11.1M/wk | github.com/pyfisch/httpdate | OK | Approved (already transitive via hyper) |
| rand | crates.io | ~11 yrs | 38.2M/wk | github.com/rust-random/rand | OK | Approved (already a workspace dep) |
| redis | crates.io | ~12 yrs | 2.2M/wk | github.com/redis-rs/redis-rs | OK | Approved (already a workspace dep) |
| chrono | crates.io | ~12 yrs | 15.0M/wk | github.com/chronotope/chrono | OK | Approved (already a workspace dep) |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

Verdicts from `gsd-tools query package-legitimacy check --ecosystem crates httpdate rand redis chrono` (all `OK`, no postinstall, no deprecation). Crates have no install-script vector comparable to npm `postinstall`; `cargo-deny`/`cargo-audit` (`make security`) remain the gate for the new direct edge.

## Header Reference (PACE-01 — verify before quoting in rustdoc)

### Anthropic [VERIFIED: `platform.claude.com/docs/en/api/rate-limits`, fetched in this session, "Response headers" table]

| Header | Meaning (verbatim gist) |
|--------|-------------------------|
| `retry-after` | "The number of seconds to wait until you can retry the request. Earlier retries will fail. Not sent with the spend-cap 429." |
| `anthropic-ratelimit-requests-limit` / `-remaining` / `-reset` | request budget; `-reset` = "time when the request rate limit will be fully replenished, provided in RFC 3339 format" |
| `anthropic-ratelimit-tokens-limit` / `-remaining` / `-reset` | most-restrictive token limit in effect (remaining rounded to nearest thousand) |
| `anthropic-ratelimit-input-tokens-limit` / `-remaining` / `-reset` | input tokens |
| `anthropic-ratelimit-output-tokens-limit` / `-remaining` / `-reset` | output tokens |
| `anthropic-priority-input-tokens-*`, `anthropic-priority-output-tokens-*` | Priority Tier only |
| `anthropic-fast-*` | fast-mode limits (names not enumerated on that page) |

Facts that change the design: the API "uses the token bucket algorithm" (capacity continuously replenished), so `-reset` is *full-replenishment* time, an **over-estimate** of when one more request is admitted — `retry-after` is the precise value and must win. The **spend-cap 429** (`error.type = rate_limit_error`, `error.details.error_code = enforced_spend_limit_reached`) carries **no `retry-after`** and "retrying, including the SDK's automatic retries, fails until access resumes" — pacing on it is futile (Pitfall 6). Acceleration-limit 429s also exist ("sharp increase in usage").

### OpenAI [CITED: OpenAI rate-limits guide, `developers.openai.com/api/docs/guides/rate-limits` — direct fetch is egress-blocked in this sandbox; names/semantics below are from search-result excerpts of that official page. MEDIUM confidence; re-verify before merge, see A1]

| Header | Meaning |
|--------|---------|
| `x-ratelimit-limit-requests` | max requests before exhausting the limit (example `60`) |
| `x-ratelimit-limit-tokens` | max tokens (example `150000`) |
| `x-ratelimit-remaining-requests` | remaining requests (example `59`) |
| `x-ratelimit-remaining-tokens` | remaining tokens (example `149984`) |
| `x-ratelimit-reset-requests` | time until the request limit resets; **duration string** (`1s`, `6m0s`, `6ms`) |
| `x-ratelimit-reset-tokens` | time until the token limit resets; duration string (example `6m0s`) |
| `Retry-After` | "minimum number of seconds to wait before retrying a temporary rate-limit error, when present"; can appear on 429 (temporary limit) and 503 (model overload); NOT a fix for quota/billing errors that need user action |
| project-scoped token variants (e.g. `x-ratelimit-limit-project-tokens`) | present "when a project-scoped token limit applies" — **exact names LOW confidence (A1)** |

Not in OpenAI's docs but observed: `openai-python`'s `_base_client.py` reads `retry-after-ms` (divided by 1000) *before* `retry-after`, and falls back to parsing `retry-after` as an HTTP-date via `email.utils.parsedate_tz`; it honours a server delay only when finite and `0 < delay <= MAX_RETRY_AFTER_DELAY` [CITED: `raw.githubusercontent.com/openai/openai-python/main/src/openai/_base_client.py`, fetched]. Azure OpenAI emits `retry-after-ms`; Azure also reportedly returns `-1`/`0` placeholder values in `x-ratelimit-*` headers (community thread) — parse defensively (negative -> `None`). Treat `retry-after-ms` as an optional extra for the OpenAI-compatible family; do not describe it as OpenAI-documented (A3).

### Parse rules (recommended)

1. Precedence for the effective retry delay: `Retry-After` (delta-seconds -> HTTP-date) > `retry-after-ms` (optional) > reset-derived delay of the *exhausted* dimension (remaining == 0; if no dimension reports remaining, the smallest reset) clamped to `max_backoff_ms` (because reset = full-replenishment/over-estimate) > `None`.
2. Delta-seconds: accept an unsigned integer; also tolerate a non-negative decimal (some gateways send `1.5`); use `Duration::try_from_secs_f64`, **never** `Duration::from_secs_f64` (panics on NaN/negative/overflow).
3. HTTP-date: `httpdate::parse_http_date`; delta = date - response `Date` header when that header parses, else the local `SystemTime::now()`; a date in the past -> `Some(Duration::ZERO)`; unparseable -> `None`.
4. Anthropic `-reset`: `DateTime::parse_from_rfc3339`, same delta rule.
5. OpenAI `-reset`: bounded Go-duration parser: `<number><unit>` repeated, units `ms|s|m|h` (and `us`/`µs` if cheap); reject empty, negative, trailing garbage, > some sane cap (e.g. 1 day) -> `None`.
6. Header values that are not valid UTF-8 or are absent -> `None`. Never log or embed a raw header value in an error (Pitfall 16).

## Architecture Patterns

### System Architecture Diagram

```
Run path A (agent loop)                        Run path B (engine node, WarEngine)
  PaladinExecutionService                        superstep.rs per-node task
        |  (RetryPolicy owns attempts)                 |  (Aegis RetryPolicy owns attempts)
        |                                              |-- cache binding? --yes--> [cache.get] -hit-> merge delta, done
        |                                              |                          miss
        |                                              |                           v
        |                                              |            CadencePort.try_lock(cache_key, ttl)
        |                                              |             |held by other           |acquired(token)
        |                                              |             v                         v
        |                                              |   poll: cache.get / try_lock     execute node (below)
        |                                              |   until hit | acquired | ttl        then cache.put(fenced) ; unlock
        |                                              |   ttl elapsed -> execute uncached
        v                                              v
  Arc<dyn LlmPort>  ===  Pricing( Fallback( Cadence(hop1), Cadence(hop2) ) )   [or Pricing(Cadence(provider))]
                                           |
        FallbackLlmAdapter: 429 from hop i?  ----yes----> same hop while (elapsed < pace_budget AND gate_wait fits budget)
                                           |                       else hop to i+1 ; 5xx/net/timeout hop immediately
                                           v
                 CadenceLlmAdapter.generate / generate_stream(request)
                   1. key = (inner.get_provider_name(), request.model)
                   2. loop { w = port.gate(key)  // read-only, server clock
                             if w > max_wait  -> return RateLimitExceeded{retry_after: w}   (D-06, no send)
                             if w == 0 -> break ; sleep(w + spread jitter) ; re-check }
                   3. result = inner.<call>(request)
                   4. Ok  -> if streak>0 { port.record_success(key) }
                      429 -> port.record_rate_limited(key, err.retry_after) ; return err UNCHANGED
                      other -> pass through
                                           |
                                           v
                       Provider adapter (openai / compat / anthropic / deepseek / gemini)
                         first 429 surfaces immediately (D-02); headers snapshotted -> RateLimitExceeded{retry_after, hints}
                                           |
                                           v
                                   HTTPS to provider

CadencePort implementations (paladin-storage):
   ResilientCadence { primary: RedisCadence (redis-cadence), fallback: InMemoryCadence(multiplier), outage latch }
        |- healthy: Redis Lua EVAL (server TIME) -> {wait_us, streak}
        |- primary Err/timeout: latch warn ONCE, serve from fallback (delays x degraded_multiplier), probe primary every N s
        |- first successful primary round-trip: clear latch, log recovery
   InMemoryCadence (tokio::time::Instant; paused-clock testable) when backend = in_process
```

### Recommended Project Structure

```
crates/paladin-ports/src/output/
├── cadence_port.rs        # CadencePort, CadenceKey, FencingToken, CadenceError, CadencePolicy (pure math)
├── llm_port.rs            # RateLimitExceeded gains retry_after + hints (constructor + accessors)
└── node_cache_port.rs     # + put_fenced default method (if Open Question 2 resolves that way)

crates/paladin-llm/src/
├── cadence.rs             # CadenceLlmAdapter, with_cadence(), CADENCE_LOG_TARGET = "paladin::cadence"
├── rate_limit_headers.rs  # closure-based header parsing (un-gated; no reqwest in signatures)
├── http_status.rs         # + map_http_status_with_hints (keep map_http_status signature)
├── fallback.rs            # + with_cadence(port, pace_budget); pace-first branch
└── conformance.rs         # + rate_limit_is_surfaced_once case (CASE_COUNT 9 -> 10)

crates/paladin-storage/src/cadence/
├── mod.rs                 # ResilientCadence composite + re-exports
├── in_memory.rs           # InMemoryCadence (always available, no feature gate, like node_cache::in_memory)
├── redis.rs               # RedisCadence (#[cfg(feature = "redis-cadence")]), Lua consts, connection w/ timeouts
└── contract_tests.rs      # shared contract suite run by both adapters

crates/paladin-battalion/src/engine/
├── superstep.rs           # NodeCacheBinding gains Option<Arc<dyn CadencePort>> ; lock around miss->put
└── mod.rs                 # WarEngine::with_cadence

src/config/treasurer.rs            # CadenceConfig + TreasurerConfig.cadence ; validate ; EnvOverridable
src/infrastructure/web/{agent_host,facade_provisioner,run_api_wiring}.rs   # compose beside with_pricing
```

### Pattern 1: Stateless gating decorator (copy `pricing.rs`)
**What:** `CadenceLlmAdapter { inner: Arc<dyn LlmPort>, port: Arc<dyn CadencePort>, settings: Arc<CadenceSettings> }`, all identity methods delegate, `with_cadence(inner, port, settings)` returns `inner` unchanged (`Arc::ptr_eq`) when disabled or when `inner.get_provider_name() == "fallback"` (a chain is paced per hop; gating the chain under key `("fallback", model)` would double-gate). Mirrors `with_pricing`'s "empty table installs no layer".
**When to use:** every `LlmPort` the server composes, and every `FallbackLlmAdapter` hop.
**Example:**
```rust
// Source: shape of crates/paladin-llm/src/pricing.rs (PricingLlmAdapter) — gate-then-delegate
async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
    let key = CadenceKey::new(self.inner.get_provider_name(), &request.model);
    self.wait_for_gate(&key).await?;          // may return Err(rate_limited(Some(w))) beyond max_wait (D-06)
    match self.inner.generate(request).await {
        Ok(resp) => { self.note_success(&key).await; Ok(resp) }
        Err(err @ LlmError::RateLimitExceeded { .. }) => {
            self.note_rate_limited(&key, err.retry_after()).await; // best-effort, never fails the call
            Err(err)                                               // surfaced UNCHANGED (D-01)
        }
        Err(other) => Err(other),
    }
}
```

### Pattern 2: Read-only gate, state-changing record (cancellation-safe)
**What:** `gate(key) -> {wait, streak}` is read-only; only `record_rate_limited` and `record_success` mutate. A task cancelled while sleeping on the gate leaves no state behind. The gate loop re-checks after waking because a concurrent 429 may have extended it.
**When to use:** always. Avoid a "reserve a slot then sleep" design — it leaks reservations on cancellation.

### Pattern 3: In-flight 429s must not escalate (the core correctness rule)
**What:** if `now < not_before` when a 429 is recorded, the 429 belongs to a request sent *before* the gate: do not increment `streak`; set `not_before = max(not_before, now + retry_after)`. Only a 429 observed with the gate already clear (`now >= not_before`) escalates. Prototyped and verified in Redis (late 429 inside gate returned `streak` unchanged at 1, delay raised to the larger `Retry-After`). The in-memory adapter implements the identical rule on `tokio::time::Instant`.
**Why:** with N concurrent in-flight calls on one key, naive escalation applies N doublings from a single provider hiccup.

### Pattern 4: Pace-first, hop-last (D-03)
**What:** `FallbackLlmAdapter::after_failure` gains a branch: error is `RateLimitExceeded` AND this hop's cumulative paced wait `< fallback_pace_budget` AND the hop's *current* gate wait (read from the port) fits the remaining budget -> retry the same hop (the hop's `CadenceLlmAdapter` will sleep the gate). Otherwise fall to the existing hop rule. Track `Instant` of the first 429 per hop per call. Consequence: `FallbackLlmAdapter` needs the port (`with_cadence(port, budget)`), not only the decorator.
**Must also:** keep `AllProvidersFailed.attempts` honest (record each paced retry or document that paced retries are not listed), rewrite the module docs/tests that claim "never retries a hop" (`fallback.rs`), and emit a debug/trace line for each pace-retry so a benchmark operator can see it.

### Pattern 5: Composite adapter owns degradation (D-05/PACE-05)
**What:** `ResilientCadence { primary: Arc<dyn CadencePort>, fallback: Arc<InMemoryCadence>, state: Mutex<{outage_since, last_probe}> }` implements `CadencePort`. Any primary `Err` (including timeout) latches the outage (one `log::warn!(target: "paladin::cadence")`), serves the call from the in-process fallback whose computed delays are multiplied by `degraded_multiplier`, and probes the primary no more often than every N seconds (so a dead Redis does not add a timeout to every LLM call); the first successful primary round-trip clears the latch and logs one recovery line. Because the primary is `Arc<dyn CadencePort>`, PACE-05's test injects an always-failing stub — no Redis needed — and a second `#[ignore]`/skip-gated test points `RedisCadence` at `127.0.0.1:1`.

### Pattern 6: Engine lock loop that retries the lock, not just the read (D-11/D-12)
**What:** on cache miss with a cadence handle: `loop { if let Some(hit) = lookup { serve }; match port.try_lock(key, ttl) { Some(token) => { execute; put; unlock; break } None => { sleep(poll + jitter) (select! with the run's cancellation token); if deadline elapsed { execute uncached; break } } } }`. Re-trying `try_lock` each poll makes a crashed/failed holder's release (`unlock` on every exit path) hand the lock to a waiter within one poll interval instead of every waiter idling for the full TTL.

### Anti-Patterns to Avoid
- **Decorator that sleeps-and-retries** the failing call: contradicts D-01 and multiplies attempt counts (Phase 26 rule).
- **Calling `backoff_delay` from `paladin-llm`/`paladin-storage`:** no dependency edge exists (and battalion -> llm is a dev-dep the other way); lift the arithmetic instead (see Don't Hand-Roll).
- **Holding `Arc<RwLock<ConnectionManager>>` and taking `.write()` per call** (what `run_queue/redis.rs` and `node_cache/redis.rs` do): serialises every Redis call in the process behind one lock. `ConnectionManager` is `Clone` (`Arc<Internals>`, verified) and multiplexed; clone it per call. Wrong place to copy the idiom verbatim.
- **Gate state in a decorator field** (e.g., a `Mutex<HashMap>` inside `CadenceLlmAdapter`): the same provider wrapped in two decorators (agent host + a fallback hop) would pace independently. State lives behind the shared `Arc<dyn CadencePort>`.
- **Panicking arithmetic on provider-controlled numbers** (`Duration::from_secs_f64`, `Instant + huge`): use `try_from_secs_f64`, `checked_add`, saturating clamps.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP-date parsing | Regex/strptime for three date formats | `httpdate::parse_http_date` | RFC 7231 requires accepting IMF-fixdate, RFC 850 and asctime; hyper's own parser |
| RFC 3339 timestamp (Anthropic reset) | Manual split/parse | `chrono::DateTime::parse_from_rfc3339` | Already a dependency |
| Atomic read-modify-write across workers | Client-side GET/compute/SET | One `redis::Script` per op with server `TIME` | Two workers interleave otherwise (T-27-03-01 precedent) |
| NOSCRIPT/EVALSHA handling | Manual `SCRIPT LOAD`/`EVALSHA` | `Script::invoke_async` (auto-reloads once) | Documented in `run_queue/redis.rs` |
| Lock-owner check | `GET` then `DEL` from the client | Lua compare-and-delete | Classic race: expired lock re-acquired by another worker between the two calls |
| Exponential back-off arithmetic | A third copy | One pure `CadencePolicy` function in `paladin-ports` (and ideally have battalion's `backoff_delay` delegate later) | Two copies already exist (`backoff_delay`, three adapter-private loops); `backoff_delay` is battalion-bound and uses additive jitter, so it cannot serve here |
| Redis connection pooling/reconnect | Custom reconnect loop | `ConnectionManager` cloned per call + explicit `ConnectionManagerConfig` timeouts | Built-in reconnect; only the timeouts are missing by default |
| Time in tests | Fake-clock crate | `#[tokio::test(start_paused = true)]` + `tokio::time::Instant` | Established idiom (`engine/retry.rs`, `resilience.rs` tests) |
| Redaction of error excerpts | New truncation | `crate::redaction::{redact_credentials, bounded_excerpt}` via `map_http_status` | Redact-then-bound ordering is a security control |

**Key insight:** every hard part here (atomicity, ownership checks, date grammars, reconnects, clocks) already has a house or ecosystem answer; the new work is the *state machine rules* (de-escalation, pace-first budget, outage latch, lock-retry loop), which are small, pure, and table-testable.

## Common Pitfalls

### Pitfall 1: Variant-shape change ripples through 86 occurrences / 26+ files
**What goes wrong:** `LlmError::RateLimitExceeded` used as a value (`MockLlmAdapter::with_error(LlmError::RateLimitExceeded)`, `Err(LlmError::RateLimitExceeded)`) stops compiling when it becomes a struct variant; patterns `matches!(x, LlmError::RateLimitExceeded)` stop compiling too.
**Why it happens:** a unit variant is both a value and a pattern.
**How to avoid:** make it a struct variant with variant-level `#[non_exhaustive]` and add constructors (`LlmError::rate_limit_exceeded()`, `LlmError::rate_limited(Option<Duration>)`) plus accessors (`retry_after()`, `rate_limit_hints()`); convert patterns to `RateLimitExceeded { .. }`. Do it as its own first plan/commit (mechanical, compiler-driven), keeping `#[error("Rate limit exceeded")]` byte-identical so no `Display` assertion churns. Excluded from the sweep: `NotificationPortError`, `ContentDeliveryError`, `VisionError::RateLimitExceeded(String)`, `LlmProviderError::RateLimitExceeded` (separate unit enum in `crates/paladin-llm/src/error.rs` that maps into `LlmError` — keep, update its `From`). Occurrences to convert (files): `qwen/grok/kimi/openai_compatible/gemini/deepseek/openai` adapters + tests, `compat/engine.rs`, `http_status.rs`, `mock.rs`, `pricing.rs` tests, `anthropic/adapter.rs`, `llm_decision.rs`, `llm_failure.rs`, `commander.rs`, `conclave_execution_service.rs`, `paladin-eval/src/scenario.rs`, and root `tests/` (cli, integration, unit).
**Warning signs:** `cargo check --workspace --all-targets --all-features` errors in crates the phase "didn't touch".

### Pitfall 2: Response headers are gone by the time `map_error` runs
**What goes wrong:** every adapter does `response.text().await` (consumes `Response`) and then `self.map_error(status, &body)`; headers are unreachable.
**How to avoid:** snapshot headers (only when `status == 429`) *before* `.text()`: `let hints = RateLimitHints::from_lookup(|n| response.headers().get(n).and_then(|v| v.to_str().ok()))`. Keep `map_error(status, body)` as a wrapper over a new `map_error_with_hints(status, body, &hints)` so the ~40 existing `map_error` unit tests compile unchanged. Keep `map_http_status`'s public signature (it has a doctest and is public); add `map_http_status_with_hints`. The closure-based lookup keeps `http_status.rs`/`rate_limit_headers.rs` free of `reqwest` types (the module is un-gated; `reqwest` is optional).
**Warning signs:** a conformance case with a `Retry-After: 7` mock that reads `retry_after == None`.

### Pitfall 3: Panics and overflow on provider-controlled numbers
**What goes wrong:** `Duration::from_secs_f64(f64::INFINITY)` panics; `tokio::time::Instant::now() + Duration::MAX` panics; Lua doubles lose integer precision above 2^53.
**How to avoid:** `Duration::try_from_secs_f64` (stable since 1.66; MSRV is 1.88), `checked_add` with saturation, clamp every delay to a hard ceiling (e.g. 24h) before it reaches the port, pass microseconds to Lua as integers and `string.format('%d', ...)` when writing computed values (the prototype's integral doubles stored fine, but be explicit).
**Warning signs:** a fuzz-ish test with `Retry-After: 99999999999999999999` and `NaN`/`inf`/negative values.

### Pitfall 4: Thundering herd when the gate clears
**What goes wrong:** N tasks asleep on the same key wake at the same instant and fire together, producing the next 429.
**How to avoid:** after waking, each waiter re-checks the gate (a sibling's 429 may have extended it) and adds a small per-waiter spread (`0..=min(gate*0.1, 1s)`) ; optionally cap concurrent "probe" releases. The *gate value itself* stays exactly the provider's minimum (the retry delay is never reduced).
**Warning signs:** test with 20 concurrent calls on a paused clock shows 20 sends at the identical `Instant`.

### Pitfall 5: "Retry delay honoured as a minimum" vs "full jitter" (D-04 vs PACE-02 wording)
**What goes wrong:** applying full jitter (uniform in `[0, delay)`) to a provider-supplied `Retry-After` can send *earlier* than the provider said — violating "minimum".
**How to avoid:** when a retry delay is present, `gate = retry_after` exactly (jitter lives in waiter spread, Pitfall 4); when absent, `gate = max(base_backoff, floor(ceiling * u))` where `ceiling = min(max_backoff, base * 2^(streak-1))` and `u` is a client-supplied uniform `[0,1)` (floor of `base` prevents near-zero gates that read as thrash). The success-criterion-2 test asserts `observed_gap >= retry_after` and, for the no-header path, that successive gaps lie in `[base, ceiling]` and grow.

### Pitfall 6: Quota/spend-cap 429s are not rate limits
**What goes wrong:** Anthropic's spend-cap 429 (`enforced_spend_limit_reached`, no `retry-after`) and OpenAI's quota/billing 429s are classified `Transient` by status alone; with D-03, a fallback chain would spend its whole 60s pace budget re-hitting an account-level wall before hopping, and every worker gates the key.
**How to avoid:** detect these in the adapter's 429 mapping from the typed body fields (`error.details.error_code == "enforced_spend_limit_reached"`; OpenAI `error.code == "insufficient_quota"` [ASSUMED — verify the exact code string against OpenAI's error docs]) and map to the existing `LlmError::UsageLimitExceeded` (permanent, already non-retryable everywhere, already exempt from adapter retry loops) *before* the generic 429 arm — the same pattern Phase 41 used for Anthropic's 400 usage-cap. This is an Open Question (scope), not a locked decision; at minimum, document the limitation and let `max_backoff_ms` bound the damage.
**Warning signs:** a 429 with no `Retry-After` whose body names billing/quota/spend.

### Pitfall 7: D-02 scope is incomplete and the conformance macro does not cover Anthropic
**What goes wrong:** (a) `deepseek/adapter.rs::call_api_with_retry` and `gemini/adapter.rs::execute_with_retry` (both in the conformance suite) retry `RateLimitExceeded`; DeepSeek even has a test `call_api_with_retry_still_retries_rate_limit_exceeded` asserting 4 calls ("must not regress") that D-02 flips; (b) a conformance case asserting "mocked 429 observed exactly once" fails for those two unless fixed; (c) `anthropic/adapter.rs` is not instantiated with `llm_conformance_suite!` (macro users: qwen, grok, ollama, openai_compatible, gemini, kimi, deepseek, openai), so Anthropic needs a bespoke test; (d) `kimi/qwen/grok/ollama/openai_compatible` ride `CompatEngine` so the compat fix covers them; (e) `openai/vision.rs` and `anthropic/vision.rs` have their own `VisionError` retry loops on a different port — out of scope, say so.
**How to avoid:** extend D-02 to DeepSeek and Gemini (needs operator confirmation, Open Question 1), add `rate_limit_is_surfaced_once` as case 10 to `cases` + the macro list, update `CASE_COUNT` pin (`suite_generates_the_full_case_list_for_a_fixture` asserts the documented 9), and write the Anthropic mockito test by hand. Existing `transience_by_value` uses `expect_at_least(1)` for 429, so it keeps passing.

### Pitfall 8: `rand` is optional in `paladin-llm`; cadence must not be feature-gated
**What goes wrong:** `rand` is `optional = true`, enabled by provider features (`openai`, ...); `fallback.rs`/`pricing.rs` are un-gated and compile with `--no-default-features`. A `cadence.rs` using `rand` would break `--no-default-features` builds (and `crate-isolation` CI).
**How to avoid:** pick one: (a) make `rand` a required dependency of `paladin-llm` (already workspace-vetted); (b) have the *adapters* in `paladin-storage` draw jitter (give `paladin-storage` a `rand` dep) and let the decorator draw only the tiny waiter-spread via `uuid`-derived or `rand`. Recommended: (a) — one Cargo line, no new package, and the in-process adapter's jitter can live in `paladin-ports` as a pure function taking an injected fraction.
**Warning signs:** `cargo build -p paladin-llm --no-default-features` fails.

### Pitfall 9: Redis defaults to no timeout; a black-holed server hangs every LLM call
**What goes wrong:** `ConnectionManagerConfig` defaults are `response_timeout: None`, `connection_timeout: None`, 6 reconnect retries with exponential delay [VERIFIED: `redis-0.32.7/src/aio/connection_manager.rs` constants]. With Redis unreachable-but-not-refusing (firewall drop, failover), `gate()` blocks indefinitely — the opposite of "degrade to conservative pacing".
**How to avoid:** build the cadence connection with `ConnectionManagerConfig::new().set_response_timeout(~250-500ms).set_connection_timeout(~500ms).set_number_of_retries(1-2)` and additionally wrap each call in `tokio::time::timeout`; treat timeout as `Err` -> degraded. Make construction lazy/non-fatal: if `ConnectionManager::new` fails at boot with `backend: redis` configured, start in degraded mode with one warning and retry the connect on the probe interval (a worker must be able to boot while Redis is briefly down; `validate()` only checks the env var is set).
**Warning signs:** a PACE-05 test using `127.0.0.1:1` that takes seconds instead of milliseconds.

### Pitfall 10: Outage warning per call, and a timeout tax on every call
**What goes wrong:** warning once per call spams logs; probing Redis on every call during an outage adds the timeout to every LLM request.
**How to avoid:** the latch in the composite adapter: `warn!` once on the healthy->degraded edge, `info!` once on recovery, and a probe back-off (e.g. at most one primary attempt per few seconds while degraded; concurrent callers during a probe use the fallback). The pricing `UnpricedModelWarnings` bounded-set pattern is the model for bounded state, not for the latch itself.

### Pitfall 11: Key cardinality and residue
**What goes wrong:** keys include `request.model`, which can contain `:`/`/`; an unbounded set of model strings would grow memory/keys.
**How to avoid:** reactive-only gating means state is created only on a 429 and `gate()` on an unknown key must not insert; evict in-memory entries whose gate elapsed and `streak == 0` after an idle period; every Redis key gets `PEXPIRE` (prototype: `max(ttl, delay)+ttl`) so an idle provider leaves no residue; cap the in-memory map (warn once at the cap, like `UNPRICED_MODEL_WARNING_CAPACITY = 256`). Do not parse the key anywhere (colons are harmless as an opaque string).

### Pitfall 12: Lock TTL shorter than the node (stampede returns silently)
**What goes wrong:** the engine holds the lock across the node's whole `Aegis` attempt loop (including retries and back-offs), which for LLM nodes can exceed `lock_ttl_secs = 60`. The lock expires, a second worker acquires and issues the duplicate call — the exact stampede the lock exists to prevent.
**How to avoid:** default `lock_ttl_secs` toward 120-300s, document that it must exceed the p99 node duration, and (Open Question 3) consider an owner-checked `extend_lock` renewed at TTL/3 by a small guard task. Fencing (Pitfall 13) makes a late stale write harmless but does not prevent the duplicate spend.

### Pitfall 13: Fencing is only meaningful at the resource being written
**What goes wrong:** D-14 says the token is "handed to the Redis `NodeCachePort` adapter's `put` path", but D-00d says `NodeCachePort` is unchanged and `put(&key, &delta, ttl)` has no token parameter. A token that nobody checks is not fencing (Kleppmann's argument against lock-only designs).
**How to avoid:** add a defaulted trait method `put_fenced(&self, key, delta, ttl, fence: &FencingToken) -> Result<..>` that defaults to `put` (additive, non-breaking for implementors, best-effort contract D-29 untouched); `RedisNodeCache` overrides it with a Lua script: `if tonumber(GET fence_key or 0) > token then return 0 end; SET fence_key token; PSETEX entry`. Keep the fence key on the *cache's* Redis (cadence and cache may be different servers; only the token's monotonic source must be the cadence counter). Give `FencingToken` a source discriminant (`Distributed(u64)` vs `Local(u64)`): in-process tokens (an `AtomicU64` restarting at 1 per process) must **never** be compared against Redis-issued tokens, so `Local` tokens bypass the fence check. Give the counter key a TTL much longer than the lock TTL (`>= 10x`), else a counter reset lets an old holder's token outrank a new one.

### Pitfall 14: Lock leaks and abort paths
**What goes wrong:** a node task aborted by the grace-deadline race (`IndexedHandle::abort`) never reaches `unlock`; waiters stall until TTL.
**How to avoid:** release on every normal exit path (success, failed attempt, handler-compensated, non-`Edges` directive — note `store_node_cache` stores only for `NextStep::Edges` and non-`Deny` deltas, so for other outcomes **no entry will ever appear**), accept TTL as the abort backstop, and make waiters retry `try_lock` (Pattern 6) so a released-without-entry lock hands over immediately rather than costing every waiter the full TTL. A synchronous `Drop` guard cannot `await`; if desired, `tokio::spawn` the unlock from `Drop` when a runtime handle exists — optional.

### Pitfall 15: Waiter loop must honour run cancellation
The poll sleep must be `select!`-ed with the node's cancellation token (`node_cancellation` is already cloned into the task) so a halted run does not leave tasks sleeping up to the lock TTL (Phase 42 halt contract).

### Pitfall 16: Do not leak headers or URLs
Rate-limit header values are provider-controlled strings: never interpolate raw values into errors/logs (parse to numbers/durations or drop). The Redis URL (may embed a password) is read from the env var named in config and never logged or `Debug`-formatted — reuse `redact_connection_url` from `run_queue/redis.rs`; hand-written `Debug` on any config type that holds it (precedent: `RedisRunQueueConfig`).

### Pitfall 17: Nested waiting multiplies, not adds
The agent loop's `RetryPolicy` sleeps its own backoff *after* a failure and then calls again, hitting the gate. Effective wait is `max(retry_backoff, gate)`, not the sum — correct and desirable, but an attempt-count test must not assume a wall-clock sum. Document in `cadence.rs` module docs next to `pricing.rs`-style placement notes.

### Pitfall 18: Tests that sleep for real
All timing tests use `start_paused = true` and `tokio::time::Instant` in the in-process adapter; the Redis adapter returns relative waits and the *decorator* sleeps with the tokio timer, so the same paused-clock tests work against a fake `CadencePort` while Redis-specific tests assert on returned integers, not on real elapsed time.

## Code Examples

### Port surface (recommended; dyn-compatible via `async_trait`)
```rust
// Source: pattern of crates/paladin-ports/src/output/{node_cache_port,run_queue_port}.rs
#[async_trait]
pub trait CadencePort: Send + Sync {
    /// Read-only. How long to wait before sending to `key` (zero = clear) and the live streak.
    async fn gate(&self, key: &CadenceKey) -> Result<GateReading, CadenceError>;
    /// Record a provider 429. `retry_after` raises the gate to at least that value.
    /// A 429 that arrives while a gate is already active does NOT escalate the streak.
    async fn record_rate_limited(&self, key: &CadenceKey, retry_after: Option<Duration>)
        -> Result<GateReading, CadenceError>;
    /// Reset the streak (called only when `GateReading.streak > 0`).
    async fn record_success(&self, key: &CadenceKey) -> Result<(), CadenceError>;
    /// Set-if-absent with expiry; `Some(token)` only for the single owner.
    async fn try_lock(&self, key: &LockKey, ttl: Duration)
        -> Result<Option<FencingToken>, CadenceError>;
    /// Delete only if `token` still owns the lock; `Ok(false)` when it does not.
    async fn unlock(&self, key: &LockKey, token: &FencingToken) -> Result<bool, CadenceError>;
}
```

### Redis scripts (executed and verified against Redis 7.0.15 in this session)
```lua
-- record_429.lua  KEYS[1]=state hash  ARGV: 1=retry_after_us(-1 none) 2=base_us 3=max_us
--                 4=jitter_fraction[0,1) (client-supplied; Redis never draws randomness)  5=key_ttl_ms
local t = redis.call('TIME'); local now = tonumber(t[1]) * 1000000 + tonumber(t[2])
local nb = tonumber(redis.call('HGET', KEYS[1], 'nb') or '0')
local streak = tonumber(redis.call('HGET', KEYS[1], 'streak') or '0')
local ra, base, maxb, frac = tonumber(ARGV[1]), tonumber(ARGV[2]), tonumber(ARGV[3]), tonumber(ARGV[4])
local delay
if now < nb then                       -- late 429 from a pre-gate request: do not escalate
  delay = nb - now
  if ra >= 0 and ra > delay then delay = ra end
else
  streak = streak + 1
  if ra >= 0 then delay = ra
  else
    local ceiling = math.min(maxb, base * (2 ^ (streak - 1)))
    delay = math.max(base, math.floor(ceiling * frac))
  end
end
redis.call('HSET', KEYS[1], 'nb', now + delay, 'streak', streak)
redis.call('PEXPIRE', KEYS[1], math.max(tonumber(ARGV[5]), math.floor(delay / 1000) + tonumber(ARGV[5])))
return {delay, streak}
-- observed: #1 no header -> {500000,1}; in-gate 429 with ra=2s -> {2000000,1} (streak NOT escalated);
--           after gate clears, no header -> {900000,2}
```
```lua
-- gate.lua (read-only)       KEYS[1]=state hash
local t = redis.call('TIME'); local now = tonumber(t[1]) * 1000000 + tonumber(t[2])
local v = redis.call('HMGET', KEYS[1], 'nb', 'streak')
local wait = tonumber(v[1] or '0') - now; if wait < 0 then wait = 0 end
return {wait, tonumber(v[2] or '0')}
```
```lua
-- try_lock.lua  KEYS[1]=lock key  KEYS[2]=per-key fencing counter  ARGV[1]=ttl_ms  ARGV[2]=counter_ttl_ms (>> lock ttl)
if redis.call('EXISTS', KEYS[1]) == 1 then return false end
local token = redis.call('INCR', KEYS[2])
redis.call('SET', KEYS[1], token, 'PX', ARGV[1])
redis.call('PEXPIRE', KEYS[2], ARGV[2])
return token
-- unlock.lua  KEYS[1]=lock key  ARGV[1]=token
if redis.call('GET', KEYS[1]) == ARGV[1] then return redis.call('DEL', KEYS[1]) else return 0 end
-- observed: A=1, B(held)=nil, unlock(wrong)=0, unlock(owner)=1, next acquire=2 (monotonic)
```
Rust side follows `run_queue/redis.rs`: `Script::new(CONST)`, `.key(..).arg(..).invoke_async(&mut conn)`; **clone** the `ConnectionManager` per call instead of `.write().await` on a shared `RwLock`.

### Closure-based header lookup (keeps the module un-gated)
```rust
// Source: recommended; mirrors the "string-typed seam" of http_status.rs
pub struct RateLimitHints { /* retry_after, reset_after, per-dimension limit/remaining/reset */ }
impl RateLimitHints {
    pub fn from_lookup<'a>(provider: &str, now: SystemTime,
                           get: impl Fn(&str) -> Option<&'a str>) -> Self { /* parse; None on any doubt */ }
}
// adapter 429 path, BEFORE response.text():
let hints = (status == 429).then(|| RateLimitHints::from_lookup(
    PROVIDER, SystemTime::now(),
    |n| response.headers().get(n).and_then(|v| v.to_str().ok())));
let body = response.text().await.unwrap_or_default();
return Err(self.map_error_with_hints(status, &body, hints.as_ref()));
```

### Paused-clock acceptance test for success criterion 2 (shape)
```rust
// Source: idiom of crates/paladin-battalion/src/engine/retry.rs + resilience.rs tests
#[tokio::test(start_paused = true)]
async fn mocked_429_on_a_fallback_hop_backs_off_and_stays_on_the_provider() {
    let primary = Arc::new(MockLlmAdapter::new().with_provider_name("openai").with_script(vec![
        MockScriptEntry::Error(LlmError::rate_limited(Some(Duration::from_secs(7)))),
        MockScriptEntry::Text("ok".into()),
    ]));
    let backup = Arc::new(MockLlmAdapter::new().with_provider_name("anthropic"));
    let port: Arc<dyn CadencePort> = Arc::new(InMemoryCadence::new(CadencePolicy::default()));
    let chain = FallbackLlmAdapter::new(vec![primary.clone(), backup.clone()])?
        .with_cadence(port, CadenceSettings::default()); // wraps each hop; pace budget 60s
    let start = tokio::time::Instant::now();
    let resp = chain.generate(request("gpt-x")).await?;
    assert_eq!(primary.call_count(), 2);              // retried the SAME provider
    assert_eq!(backup.call_count(), 0);               // did not hop
    assert!(start.elapsed() >= Duration::from_secs(7)); // Retry-After honoured as a minimum
    assert_eq!(resp.metadata[SERVED_BY_METADATA_KEY], "openai");
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Per-adapter private retry loops that sleep on any retryable error (OpenAI/compat/Anthropic/DeepSeek/Gemini) | Single gating layer; adapters surface the first 429 | This phase (D-02) | One layer sees every 429 and the provider's `Retry-After` on the first hit |
| 429 = unit variant `RateLimitExceeded` | Carries `retry_after` + header snapshot | This phase | Retry layers and operators can act on the provider's own number |
| Anthropic limits as fixed windows | Token bucket (continuous replenishment) | Current Anthropic docs | `-reset` headers are full-replenishment times; prefer `retry-after` |
| Fallback hops immediately on 429 | Pace first, hop last (budgeted) | This phase (D-03) | Benchmark consistency, prompt-cache retention |
| Redis fixed-window counters / client clocks | Atomic Lua on server `TIME`, relative waits returned | house idiom (Phase 27) | No cross-host clock skew |

**Deprecated/outdated:**
- `LlmError::RateLimitExceeded` as a bare unit variant — replaced (X-10 row).
- The rustdoc in `llm_port.rs` that says "check `Retry-After` header" with no way to do so — becomes real.
- `FallbackLlmAdapter` module docs "No cross-call state" / "never retries a hop" — must be rewritten for the 429 exception (D-03 reversibility note).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | OpenAI project-scoped token header exact names (e.g. `x-ratelimit-limit-project-tokens`) | Header Reference | Low: they are carried-only (D-04), never gate; a wrong name just yields `None` |
| A2 | OpenAI header names/semantics (`x-ratelimit-{limit,remaining,reset}-{requests,tokens}`, duration-string resets, `Retry-After` seconds) taken from search-result excerpts of the official guide; direct fetch of `developers.openai.com` / `platform.openai.com` / `help.openai.com` was egress-blocked | Header Reference, PACE-01 | Medium: the success criterion demands verification against the official page; re-fetch from an unrestricted network (or have the operator confirm) before the rustdoc quotes it |
| A3 | `retry-after-ms` is emitted by Azure OpenAI / read first by `openai-python`, not documented by OpenAI | Header Reference | Low: optional extra; wrong -> ignored |
| A4 | OpenAI quota 429 carries `error.code == "insufficient_quota"` | Pitfall 6 | Medium if the mapping is implemented: wrong string means no detection (falls back to generic pacing, still bounded) |
| A5 | `cargo semver-checks` lint ID/behaviour for a unit -> struct variant change on an already `#[non_exhaustive]` enum (`enum_variant_changed_kind`-style lint) and the exact `[package.metadata]` suppression key | Pitfall 1, MIGRATION | Low-Medium: CI `semver` job fails until the right allowlist entry is added; planner should run `cargo semver-checks` locally |
| A6 | Typical LLM node duration can exceed 60s (so `lock_ttl_secs = 60` is tight) | Pitfall 12 | Medium: if nodes are always faster, renewal is unnecessary scope |
| A7 | `humantime` accepting Go-style `6m0s` was not checked (hence hand-parser recommended) | Standard Stack | None (not depended on) |
| A8 | Gemini 429 bodies carry `google.rpc.RetryInfo.retryDelay` ("33s") | Open Question 6 | Low: optional enhancement only |
| A9 | Redis `TIME`-then-write inside a script is safe on every Redis the fleet runs (verified on 7.0.15; CI uses `redis:7-alpine`; Redis < 5 would need `replicate_commands`) | Code Examples | Low: matches the existing `run_queue` precedent |
| A10 | `ConnectionManager` multiplexed clone-per-call has no hidden ordering hazard for independent scripts | Anti-Patterns | Low: independent commands; verify in the contract test with concurrent callers |

## Open Questions

1. **D-02 scope: DeepSeek and Gemini (and Anthropic's conformance coverage).**
   - What we know: `deepseek/adapter.rs::call_api_with_retry` and `gemini/adapter.rs::execute_with_retry` retry `RateLimitExceeded`; both use `llm_conformance_suite!`; DeepSeek has a test asserting 4 calls on 429; Anthropic is not in the macro.
   - What's unclear: whether the operator wants them in D-02 (the goal "no thrash hides below the decorator" says yes).
   - Recommendation: include DeepSeek and Gemini in D-02; flip the DeepSeek test; add case 10 to the suite; hand-write the Anthropic test. Confirm with the operator in plan review (one-line scope extension, same rationale as D-02).

2. **D-14 fencing seam vs D-00d "NodeCachePort unchanged".**
   - What we know: `put` has no token parameter; fencing only works if the written resource checks it.
   - What's unclear: whether adding a *default* trait method counts as "unchanged".
   - Recommendation: add `put_fenced` as a default method delegating to `put` (D-29 best-effort contract untouched, no implementor breaks), override in `RedisNodeCache`; `Local` tokens bypass the check. Flag in the plan as the one deliberate reading of D-00d.

3. **Lock renewal (`extend_lock`).**
   - What we know: D-14 fixes TTL from config; nodes may outlive it (Pitfall 12).
   - Recommendation: ship without renewal in this phase but raise the default `lock_ttl_secs` (e.g. 120) and document; add renewal only if the operator asks (one more owner-checked `PEXPIRE` script + guard task).

4. **Where per-hop wrapping happens.**
   - What we know: `ModelFallbackMiddleware::new(chain)` sets `cx.llm_override`, which **replaces** the service's port (`effective_llm`); `agent_runtime.rs:419` builds its chain from `resolve_chain(&deps.llm_provider_factory)` with plain `factory.create(name)`. Decorators applied in `build_agent`/`facade_provisioner` therefore never see config-built fallback hops (pre-existing for pricing too). "A run is never left unpaced" is violated for fallback agents unless hops are wrapped where they are created.
   - Recommendation: give `LlmProviderFactory` an optional cadence wrapper (`with_cadence(port, settings)` returning a factory whose `create` returns `with_cadence(inner)`), compose it once in the server's composition roots and thread that factory to `resolve_chain`; additionally provide `FallbackLlmAdapter::with_cadence` for hand-built chains. Note `LlmProviderFactory` is currently a unit struct (`pub struct LlmProviderFactory;`) — adding a field is a public-surface change for the §9.2 register. Decide between factory-wrapping and chain-builder-wrapping in plan review; do not leave it to implementation.

5. **`TraceEvent` variant vs `log` for gated-call / degraded-mode events.**
   - What we know: CONTEXT's canonical refs point at `TraceEvent`; `TraceEvent` is `#[non_exhaustive]` but matched in ~12 files (`events.rs`, `otel_sink.rs`, `run_trace/mod.rs`, `hooks.rs`, `overlay.rs`, ...), feeds the persisted run-trace and SSE wire, and a new variant is a published-schema change. The decorator has no node id and, unlike `FallbackLlmAdapter`, no ambient emitter requirement.
   - Recommendation: PACE-05 says "trace warning" — satisfy it with `log::warn!(target: "paladin::cadence", ...)` (testable with a capturing logger) plus `log::trace!` per gated call; defer a `TraceEvent::CadenceGated` variant unless the operator wants gated waits visible in run traces. If a variant is wanted, treat it as its own plan (schema + sinks + goldens).

6. **Reset-derived delay and Gemini/other providers.**
   - Recommendation: implement `Retry-After` for every adapter via the shared mapping; implement reset-derived fallback only for OpenAI and Anthropic (the two PACE-01 names); leave Gemini `RetryInfo` and others as a documented follow-up (A8).

7. **Quota-class 429 mapping (Pitfall 6).** Recommend mapping spend-cap/quota bodies to `UsageLimitExceeded` in the same plan that touches each adapter's 429 path, or explicitly deferring with a documented limitation.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `redis-server` / `redis-cli` | PACE-03/04 live contract tests, local Lua verification | yes | 7.0.15 (system); CI uses `redis:7-alpine` | Tests self-skip with a named `SKIP:` line when unreachable (house convention) |
| `docker` | `docker compose -f docker/docker-compose.test.yml up -d redis-test` (port 6380) | yes (binary present; daemon not exercised) | — | Run `redis-server --port 6391 --save "" --appendonly no` locally (done in this session) |
| Rust toolchain / `cargo` | build and test | not probed (not needed for research) | MSRV 1.88 per `Cargo.toml` | — |
| `cargo-llvm-cov`, `cargo-audit`, `cargo-deny`, `cargo semver-checks` | gates in CLAUDE.md | not probed | — | CI jobs `coverage`, `security-audit`, `cargo-deny`, `semver` |
| Outbound docs access (`developers.openai.com`, `platform.openai.com`) | PACE-01 header verification | **no — egress-blocked** | — | `platform.claude.com` was reachable (Anthropic verified); OpenAI verified only via search excerpts + `raw.githubusercontent.com` SDK source. Re-verify from an unrestricted network |

**Missing dependencies with no fallback:** none for implementation.
**Missing dependencies with fallback:** direct OpenAI docs access (A2) — planner should add a `checkpoint:human-verify` or a final-task re-verification of the six OpenAI header names against the live page.

CI note: the existing Redis jobs are `redis-cache-integration` (port 6380, `NODE_CACHE_REDIS_TEST_URL`, SKIP-grep guard, expected-test-count guard) and `redis-queue`; CONTEXT's "redis-integration job" does not exist by that name. Add a `redis-cadence-integration` job (or extend `redis-cache-integration`) copying its SKIP-detection and test-count guards, with `CADENCE_REDIS_TEST_URL`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (tokio `#[tokio::test]`, `start_paused = true`), `mockito` 1.7.0 for HTTP, `MockLlmAdapter` for ports |
| Config file | none (Cargo features); `redis-cadence` feature on `paladin-storage` for the Redis half |
| Quick run command | `cargo test -p paladin-llm --lib cadence rate_limit fallback conformance` ; `cargo test -p paladin-storage --lib cadence` |
| Full suite command | `cargo test --workspace` then `cargo test -p paladin-storage --features redis-cadence --lib cadence` (needs Redis) |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PACE-01 | `Retry-After` delta-seconds, HTTP-date (3 forms), garbage/NaN/negative/huge -> `None` or clamp, no panic | unit (table) | `cargo test -p paladin-llm --lib rate_limit_headers` | ❌ Wave 0 |
| PACE-01 | OpenAI `x-ratelimit-*` incl. `6m0s`/`1s`/`6ms`; Anthropic RFC 3339 resets; exhausted-dimension selection | unit | `cargo test -p paladin-llm --lib rate_limit_headers` | ❌ Wave 0 |
| PACE-01 | Each adapter's real 429 path carries `retry_after` from a mocked `Retry-After` | mockito per adapter | `cargo test -p paladin-llm --all-features --lib conformance rate_limit` | ❌ Wave 0 |
| PACE-01 | `LlmError` shape: constructor/accessor, `transience()` still `Transient`, `Display` unchanged | unit | `cargo test -p paladin-ports --lib llm_port` | ✅ extend |
| PACE-02 | D-02: a mocked 429 is observed exactly once per adapter (cases added to the macro; Anthropic hand-written) | conformance | `cargo test -p paladin-llm --all-features --lib` | ❌ Wave 0 (case 10; `CASE_COUNT` pin 9->10) |
| PACE-02 | Decorator gates later calls, surfaces 429 unchanged, never re-issues, no sleep on failing call | unit, paused clock | `cargo test -p paladin-llm --lib cadence` | ❌ Wave 0 |
| PACE-02 | Retry delay is a minimum; no-header path grows exponentially within `[base, ceiling]` and resets on success | unit, paused clock | same | ❌ Wave 0 |
| PACE-02 | In-flight 429s do not escalate; 20 concurrent callers do not all send at the same instant | unit, paused clock | same | ❌ Wave 0 |
| PACE-02 | Success criterion 2: mocked 429 on a `FallbackLlmAdapter` hop -> same provider retried after gate, hop only after budget, 5xx hops immediately | unit, paused clock | `cargo test -p paladin-llm --lib fallback` | ❌ Wave 0 (existing hop tests rewritten for the exception) |
| PACE-02 | D-06: delay > `max_wait_secs` returns `RateLimitExceeded{retry_after}` without sleeping or sending | unit | `cargo test -p paladin-llm --lib cadence` | ❌ Wave 0 |
| PACE-03 | Contract suite (gate/record/success/streak/TTL residue) green on in-memory AND Redis | contract | `cargo test -p paladin-storage --lib cadence` ; `--features redis-cadence` | ❌ Wave 0 |
| PACE-03 | Two `RedisCadence` instances on one server: 429 on A slows B; waits derived from server `TIME` | integration (Redis) | `CADENCE_REDIS_TEST_URL=redis://127.0.0.1:6380/0 cargo test -p paladin-storage --features redis-cadence --lib cadence::redis` | ❌ Wave 0 |
| PACE-04 | try_lock exclusivity, monotonic tokens, owner-only unlock, TTL expiry, fenced put rejects lower token | contract (+Redis) | same | ❌ Wave 0 |
| PACE-04 | Engine: N concurrent identical cache-miss nodes execute the node once; losers serve the hit; failed holder hands lock over before TTL; cancellation aborts waiting | engine unit | `cargo test -p paladin-battalion --lib superstep node_cache` | ❌ Wave 0 |
| PACE-05 | Injected always-failing primary -> composite paces from fallback (delays x multiplier), exactly one warning, recovery clears latch | unit (no Redis) | `cargo test -p paladin-storage --lib cadence::resilient` | ❌ Wave 0 |
| PACE-05 | `RedisCadence` pointed at `127.0.0.1:1` degrades within the timeout budget (milliseconds) | integration (no server needed) | `cargo test -p paladin-storage --features redis-cadence --lib cadence::redis::unreachable` | ❌ Wave 0 |
| PACE-05 | End-to-end: with Redis down a decorated call is still gated after a 429 (never unpaced) | unit | `cargo test -p paladin-llm --lib cadence` | ❌ Wave 0 |
| Config | `treasurer.cadence` defaults, `deny_unknown_fields`, validation (zero durations, multiplier < 1.0, empty/unset `url_env`), `APP_TREASURER_CADENCE_*` | unit | `cargo test --lib config::treasurer` | ✅ extend |
| Wiring | cadence composed at agent host / facade provisioner / run wiring; `enabled:false` installs nothing (`Arc::ptr_eq`); config-built fallback hops are paced | root-crate unit | `cargo test --lib infrastructure::web` | ✅ extend |
| Governance | Vocabulary term table updated; `make api-surface`, `semver`, MIGRATION §9.1/9.2/9.3/9.5 rows, CHANGELOG | CI gates | `make api-surface`, `make security`, `cargo test --test treasurer_vocabulary_guard` | ✅ |

### Sampling Rate
- **Per task commit:** `cargo test -p <touched crate> --lib <module>` (< 30 s each).
- **Per wave merge:** `cargo test --workspace` plus `cargo test -p paladin-storage --features redis-cadence --lib cadence` against `redis-server --port 6391` (or the compose `redis-test`).
- **Phase gate:** full suite green, `cargo clippy -- -D warnings`, `cargo fmt --check`, `make api-surface`, `make security`, coverage floor 82%, before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `crates/paladin-llm/src/rate_limit_headers.rs` tests — covers PACE-01 parse matrix
- [ ] `crates/paladin-llm/src/cadence.rs` tests — covers PACE-02/05 decorator rules
- [ ] `crates/paladin-storage/src/cadence/contract_tests.rs` — shared contract for PACE-03/04 (run by in-memory, Redis, composite)
- [ ] `conformance.rs` case 10 + `CASE_COUNT` pin update; hand-written Anthropic 429-once test
- [ ] `FallbackLlmAdapter` tests rewritten for the pace-first exception
- [ ] engine stampede tests in `superstep.rs`
- [ ] CI job `redis-cadence-integration` (copy `redis-cache-integration` guards)
- [ ] Framework install: none (all dependencies present)

## Security Domain

`security_enforcement` is absent in `.planning/config.json` -> enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (no new auth surface) | — |
| V3 Session Management | no | — |
| V4 Access Control | no (pacing is per `(provider, model)`, deliberately not tenant-aware) | — |
| V5 Input Validation | yes | Header values are untrusted: parse to numbers with bounds, `try_from_secs_f64`, clamp to a ceiling, reject non-UTF-8; model/provider strings used as Redis key material are opaque and namespaced |
| V6 Cryptography | no (no new crypto) | — |
| V7 Error handling & logging | yes | Redact-then-bound response bodies (existing `map_http_status`); no raw header values in errors/logs; no API keys or Redis URL in logs or `Debug` |
| V8 Data protection | yes | Redis URL from an env var only (never in config file or `Debug`); hand-written `Debug` with `redact_connection_url` |
| V12 Files/resources (DoS) | yes | Bounded state: reactive-only key creation, capped in-memory map, TTL on every Redis key, clamped delays, bounded lock wait |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Hostile/compromised provider or gateway sends `Retry-After: 99999999999` to stall the fleet | DoS | Clamp to a hard ceiling; D-06 `max_wait_secs` refuses to sleep beyond it and surfaces the typed error; reactive gate expires via TTL |
| Header parsing panics (NaN, inf, negative, overflow) | DoS | `try_from_secs_f64`, `checked_add`, table-driven fuzz cases |
| Attacker-chosen model string explodes key cardinality | DoS | Keys created only on a 429, bounded in-memory map, `PEXPIRE` on every Redis key |
| Credential leakage via redirect (Retry-After path touches the same clients) | Information disclosure | No change to clients; they keep `redirect::Policy::none()`; a 3xx stays a `ProviderError`; do not add any new HTTP client in this phase |
| Redis URL / password in logs or Debug | Information disclosure | Env-var-name config, `redact_connection_url`, manual `Debug` |
| Lock-holder impersonation / unlocking another worker's lock | Tampering | Lua compare-and-delete on a per-acquisition token; tokens are server-issued `INCR` integers (not guessable secrets, but unlock requires the exact current value) |
| Stale lock holder overwrites newer cache entry | Tampering | Fencing token checked on the resource (`put_fenced`), `Local` tokens never compared to `Distributed` |
| Lua/script injection | Tampering | Scripts are `const` strings; all variable data via `KEYS`/`ARGV`; never string-formatted into Lua |
| Redis outage turning into an LLM-call outage (availability) | DoS | Explicit response/connect timeouts, fail-open onto in-process pacing, probe back-off |

Manual credential-handling review (per `security.instructions.md`) is required for: the new header-snapshot code in each adapter (does not touch the key), any new log line (no URL/key/raw header), and the Redis config type. `make security` must pass; the new direct dependency `httpdate` is already in `Cargo.lock`, so advisory/license posture is unchanged.

## Sources

### Primary (HIGH confidence)
- Anthropic rate limits and "Response headers" table — `https://platform.claude.com/docs/en/api/rate-limits` (fetched this session; header names, RFC 3339 resets, token-bucket statement, spend-cap 429 without `retry-after`)
- Local source: `redis-0.32.7/src/aio/connection_manager.rs` (default timeouts `None`, `Clone` derive), `httpdate-1.0.3/src/lib.rs` (`parse_http_date`)
- Repository files read: `crates/paladin-ports/src/output/{llm_port,node_cache_port}.rs`, `crates/paladin-llm/src/{pricing,fallback,http_status,conformance,mock,provider_factory}.rs`, `crates/paladin-llm/src/{openai,anthropic,deepseek,gemini,ollama}/adapter.rs`, `crates/paladin-llm/src/compat/engine.rs`, `crates/paladin-battalion/src/engine/{retry,superstep}.rs`, `crates/paladin-storage/src/{run_queue,node_cache}/redis.rs` + `contract_tests.rs`, `src/config/{treasurer,run_queue}.rs`, `src/config/agent_runtime.rs`, `src/application/services/paladin/middleware/{resilience,context}.rs`, `src/infrastructure/web/{agent_host,facade_provisioner}.rs`, `.github/workflows/ci.yml`, `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `tests/treasurer_vocabulary_guard.rs`
- Executed locally: Lua `record_429` / `gate` / `try_lock` / `unlock` against Redis 7.0.15 (outputs quoted in Code Examples)
- `gsd-tools query package-legitimacy check --ecosystem crates httpdate rand redis chrono` -> all OK

### Secondary (MEDIUM confidence)
- OpenAI rate-limits guide (`https://developers.openai.com/api/docs/guides/rate-limits`) — header names/semantics via search excerpts only (direct fetch blocked)
- `openai-python` `_base_client.py` (`https://raw.githubusercontent.com/openai/openai-python/main/src/openai/_base_client.py`) — `retry-after-ms` then `retry-after`, HTTP-date fallback, bounds check, jitter

### Tertiary (LOW confidence)
- Community threads on Azure OpenAI returning `-1`/`0` placeholder `x-ratelimit-*` values and `retry-after-ms`; third-party Anthropic header write-ups (cross-checked against the official table above)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — everything is already in the lockfile/workspace; legitimacy verified.
- Architecture: HIGH for the layering (it follows two established house patterns and was exercised against a real Redis); MEDIUM for the three contract-level open questions (D-02 scope, fencing seam, per-hop composition site).
- Pitfalls: HIGH for the code-derived ones (1, 2, 7, 8, 9, 13, per-hop composition gap); MEDIUM for provider-behaviour ones (quota 429 strings, OpenAI header exactness).

**Research date:** 2026-10-07
**Valid until:** ~2026-11-06 for codebase findings (30 days, but invalidated by any intervening change to `llm_port.rs`, `fallback.rs`, or `superstep.rs`); provider header tables ~7-14 days (provider docs change; re-verify OpenAI before merge).
