# ADR-0058: Rate pacing (the Cadence): a gate-not-retry decorator, fleet pacing through Redis, a fail-open stampede lock

## Status

Accepted

**Date:** 2026-10-09

**Phase:** 43 (Rate Pacing), plan 43-13. This ADR is written at the close of the phase, after plans 43-01 through
43-12 built the design it records, so every statement below describes code that exists in the tree (the Code
Conformance section names the tests). The decisions are the Phase 43 planning decisions D-00a..D-00f and D-01..D-14
(`43-CONTEXT.md`), the operator's own selections in `43-DISCUSSION-LOG.md`, plus the planner's resolutions of the
seven open questions in `43-RESEARCH.md`. Where the planner read a locked decision rather than copying it, the
Decision section says so and says what the alternative reading would have cost. No CONTEXT decision was re-opened.

## Context

Phase 43 delivers the pacing half of the Treasurer (FUT-09, PRD R4; requirements PACE-01..PACE-05): outbound LLM
calls back off on provider `429`s instead of thrashing, in one process and across a whole worker fleet, and no run is
ever left unpaced.

Before the phase, a provider `429` was handled by three uncoordinated layers. The OpenAI adapter, the compat engine,
the DeepSeek adapter and the Gemini adapter each retried a `429` inside a private loop; `RetryPolicy` (engine nodes
and the agent-loop resilience middleware) retried it again above them; and `FallbackLlmAdapter` hopped to the next
provider on the first one. The attempt count multiplied across the layers, no layer saw the provider's `Retry-After`
(the adapters mapped from `(status, body)` only), and a worker fleet sharing one provider key had no way to learn that
a sibling had just been refused. The result is the thrash the PRD names: bursts of refused calls that make the rate
limit worse and, in a fallback chain, a run that silently changes provider mid-run.

Six earlier decisions are binding and none is re-opened (D-00a..D-00f):

- **D-00a.** Phase 38 D-09 put cost metering in a stateless `LlmPort` decorator (`pricing.rs`) and named it the natural
  home for the pacing decorator. `CadenceLlmAdapter` is that sibling, in the same crate, with the same
  `Arc<dyn LlmPort>` decorator shape, identity-method delegation and `with_*` constructor idiom.
- **D-00b.** Phase 38 D-07 reserved `treasurer.pacing`-style keys under the operator's `treasurer:` section. The
  `treasurer.cadence` subtree is added there, as Phase 41 added `allowance`, and rejects unknown keys.
- **D-00c.** Phase 26's rule that middleware never retries itself, and `RetryPolicy`'s ownership of retries, stand.
  The Phase 25 `Transience` classification stays: a `429` is `Transient`.
- **D-00d.** `NodeCachePort`'s best-effort contract (D-29: a cache failure never fails a run) is untouched. See the
  one deliberate reading of D-00d/D-11 in group (g).
- **D-00e.** Phase 41's `429 allowance_exhausted` HTTP contract (ADR-0056, 41 D-12/D-13) is the API's own admission
  refusal, a different `429` from a provider's rate limit. Nothing here touches it or reuses its error type.
- **D-00f.** The Redis adapter idiom is the one `run_queue/redis.rs` and `node_cache/redis.rs` fixed: one
  `redis::Script` `EVAL` per atomic operation, "now" from the server's `TIME`, a shared optional `redis` dependency
  behind a per-adapter Cargo feature, and config as `backend: in_memory | redis { url_env }`.

ADR-0050 reserved `Treasurer` as the single officer word for cross-run spend governance; Cadence is its pacing
persona, introduced into the ubiquitous language the way ADR-0049 introduced the Commissary (group (a)). ADR-0052
fixed the `LlmPort` boundary as the place spend is metered; the pacing decorator composes at the same boundary and adds
no ceiling. ADR-0057's halt contract is the place a capped delay (D-06) ends up when it exhausts `RetryPolicy`.

## Decision

### (a) The persona and the names (D-07)

**D-07. Cadence is the one who sets the marching pace.** The port is `CadencePort`
(`crates/paladin-ports/src/output/cadence_port.rs`), the decorator `CadenceLlmAdapter`
(`crates/paladin-llm/src/cadence.rs`), the config subtree `treasurer.cadence`, the Cargo feature `redis-cadence`, the
log target `paladin::cadence`. `Cadence` is a row in the ubiquitous-language table of
`.github/copilot-instructions.md`, beside `Commissary`. `tests/treasurer_vocabulary_guard.rs` enumerates only the
Treasurer officer word and the downstream fixture term (`officer_word`, `fixture_term`), so it needed no change; the
Phase 43 closeout recorded the grep that shows it. **Reversibility: one-way.** The port, decorator and feature names are
published Rust API and the config key is operator-facing; renaming later needs an X-10 register row and a config
migration.

### (b) Gate, not retry; adapters surface the first 429 (D-01, D-02; Open Questions 1 and 7)

**D-01. The decorator gates; the existing layers retry.** On a `429`, `CadenceLlmAdapter` records a per-`(provider,
model)` not-before instant on the `CadencePort` and surfaces the `RateLimitExceeded` unchanged. Every later call to
that key, from any task in the process (or any worker, with Redis), waits until the instant passes before it is sent.
The decorator never re-issues a request and never sleeps on the failing call itself, so `RetryPolicy` and
`FallbackLlmAdapter` keep owning retries and an attempt count is never multiplied by a third loop. Streaming calls are
paced exactly like buffered ones: a `429` as the call's error or as the stream's first item opens the gate, and later
items pass through unrecorded. **Reversibility: costly** (every retry predicate and attempt-count test above assumes
the no-retry contract).

**D-02. Provider adapters stop retrying a 429 inside their own loops.** Network, timeout and 5xx retries are untouched.
CONTEXT names the OpenAI adapter, the compat engine and the Anthropic adapter. **Open Question 1, resolved: D-02 is
extended to DeepSeek and Gemini.** Both ran private loops that retried a `429`, both are in the shared conformance
suite, and leaving them out would have made the new "observed exactly once" case fail for them and kept a thrash below
the decorator, the exact outcome D-02 exists to remove. The extension is one non-retryable arm per adapter, so it is
reversible line by line. The proof is conformance case 10, `rate_limit_is_surfaced_once_with_its_retry_delay`
(`CASE_COUNT` pin 9 to 10), instantiated for every adapter in the macro; Anthropic is not in the macro, so its proof is
the hand-written `anthropic_429_is_surfaced_once_with_its_retry_delay_and_four_dimensions`. The OpenAI conformance
fixture runs with `max_retries: 0`, so its "exactly once" half is vacuous there and the OpenAI proof is
`openai_429_is_surfaced_on_the_first_attempt` (43-01) with a non-zero `max_retries`. **Reversibility: reversible.**

**Quota-class 429s (Open Question 7, resolved).** A `429` that means "your spend is exhausted" is not a rate limit and
pacing it would only wait out `max_backoff_ms` for nothing. An OpenAI `429` whose `error.code` or `error.type` is
`insufficient_quota`, and an Anthropic `429` whose `error.details.error_code` is `enforced_spend_limit_reached`, map to
the existing permanent `LlmError::UsageLimitExceeded` before the generic arm, the pattern Phase 41 used for Anthropic's
`400` usage cap. The probe compares a short identifier and renders nothing from the body. The OpenAI code string is
verified in the evidence record (group (j)).

### (c) Pace first, hop last (D-03; Open Question 4)

**D-03. A 429 inside a fallback hop does not hop immediately.** The chain retries the same provider after its gate
clears, up to `treasurer.cadence.fallback_pace_budget_secs` (default 60) of cumulative wait per hop, and only once the
budget is spent does the `429` reach the hop rule. A `5xx`, timeout or network error still hops immediately. The
operator's rationale, recorded verbatim in spirit: a run that silently changes provider breaks benchmark consistency,
shifts tool-call formats and token accounting and forfeits the provider's prompt cache, while a `429` loses no
provider-side state at all because every request re-sends the full history. Pacing must learn the provider's rate, not
sidestep it. **Reversibility: costly** (the "stateless, never retries a hop" documentation and tests were rewritten for
the one `429` exception, and the hop trace contract of 28-06 D-03 is published).

Two guards the plan did not specify were added by 43-06 and are part of the decision. A hop is paced only when its gate
reads a non-zero wait of at most `max_wait`; an unreadable gate is **not** a zero wait (treating it as one would retry
the same hop immediately until the wall-clock budget ran out, which is the thrash this phase removes), and a gate
beyond `max_wait` is refused by the decorator without sleeping, so a budget above `max_wait` would otherwise spin
without time passing. In both cases the `429` reaches the hop rule. `TraceEvent::FallbackHop` fires only at a real
hop; `AllProvidersFailed.attempts` lists every attempt, paced ones included.

**Where per-hop wrapping happens (Open Question 4, resolved at the chain builder).** `ModelFallbackMiddleware::new(chain)`
sets `llm_override`, which replaces the service's port, so a decorator applied only in `build_agent` never sees a
config-built fallback hop (a pre-existing gap for pricing too). Wrapping at a new `LlmProviderFactory` wrapper was
rejected because `LlmProviderFactory` is a unit struct and a field is a public-surface change for the register;
instead `FallbackLlmAdapter::with_cadence(&CadenceWiring)` wraps each hop under its own provider name (never under
`"fallback"`), `ModelFallbackMiddleware::paced` builds a paced chain, and `AgentRuntimeDeps.cadence` (an in-process
wiring by default, `None` opts out) is what the private `fallback_middleware` helper that `build_chain` calls reads.
A second `with_cadence` call on a paced chain is a no-op so a hop is never double-gated. A chain a library user builds
with `FallbackLlmAdapter::new` and never calls `with_cadence` on keeps hopping immediately on a `429`, by design
(Known limitation, group (i)).

### (d) Pacing semantics (D-04, D-06; Open Question 6)

**D-04. Reactive only.** A key is gated only after a `429`. The provider's explicit delay sets the gate when present
and is a **minimum**: jitter only ever adds to it, and it is never reduced by `max_backoff_ms`. When the provider gives
none, the gate is exponential back-off with **full jitter** from `base_backoff_ms`, doubling per consecutive `429` on
that key up to `max_backoff_ms`, reset on the first success; a delay derived from a reset header (not an explicit
`Retry-After`) is clamped to `max_backoff_ms`. A `429` that arrives while a gate is already active (in flight when it
opened) does not escalate the streak. Waiters released by one gate are spread by up to `min(wait / 10, 1 s)` so they do
not all send in the same instant. Provider remaining/reset headers are parsed and carried on the error but never gate
on their own. The Cadence does not call `backoff_delay` from `paladin-battalion`: neither `paladin-llm` nor
`paladin-storage` depends on it and its jitter is additive, not full, so `CadencePolicy` in `paladin-ports` carries the
arithmetic and the Redis Lua script reproduces it. **Reversibility: reversible** (proactive gating is additive and is
the deferred idea).

**D-06. A huge retry delay is capped at the gate and surfaced beyond it.** The gate waits at most `max_wait_secs`
(default 300). A reading above that is refused with `LlmError::rate_limited(Some(wait))` before any provider call or
port write, so `RetryPolicy` exhausts or the run halts (ADR-0057) with a typed error rather than holding a worker
lease in silence. The refusal is not recorded as a new `429`, so it never escalates the shared streak. **Reversibility:
reversible.**

**The typed delay (PACE-01, CONTEXT "Claude's Discretion").** `LlmError::RateLimitExceeded` became the struct variant
`{ retry_after: Option<Duration>, hints: Option<Box<RateLimitHints>> }`, marked `#[non_exhaustive]`, constructed with
`LlmError::rate_limited(..)` / `rate_limited_with_hints(..)` and read with `retry_after()` / `rate_limit_hints()`.
`Display` stays `Rate limit exceeded` and `transience()` stays `Transient`. `RateLimitHints` holds parsed integers and
durations only, with no raw header string, so no header value can reach a rendered error. The shared parser
(`rate_limit_headers.rs`) reads `Retry-After` in delta-seconds or any of the three HTTP-date forms, `retry-after-ms`,
OpenAI's `x-ratelimit-*` with Go-style resets and Anthropic's `anthropic-ratelimit-*` with RFC 3339 resets measured from
the response `Date`; an unparseable header yields `None`, never a guess; every delay is clamped to 24 hours.
**Provider dimensions are parsed for OpenAI and Anthropic only (Open Question 6, resolved):** every adapter gets
`Retry-After` through the generic family, but the reset-derived fallback exists only for the two providers PACE-01
names. **Gemini's body-level `google.rpc.RetryInfo.retryDelay` is left unparsed** (research assumption A8, low
impact); it is a documented follow-up, not an oversight.

### (e) Degraded mode and the log target (D-05; Open Question 5)

**D-05. Degraded mode is the in-process gate with a stricter multiplier.** `ResilientCadence` serves from the shared
backend while it answers and, on the first error or timeout, latches and serves from an in-process fallback built with
`InMemoryCadence::with_multiplier(degraded_multiplier)` (default 2.0; a non-finite value or one below 1.0 is treated as
1.0 so the fallback can never pace less than the policy). It never returns `Err`, so a run is never unpaced. Exactly
one `warn` per outage and one `info` on recovery; at most one probe per `DEFAULT_PROBE_INTERVAL` (5 s) is in flight and
concurrent callers are served by the fallback without waiting; a cancelled probe releases its claim through a drop
guard. The Redis adapter's connection is lazy with 500 ms response and connection timeouts and one reconnect retry,
because the `redis` crate's `ConnectionManager` defaults to none and a black-holed server would otherwise hang every
LLM call (research finding 5, the largest PACE-05 risk). **Reversibility: reversible.**

**The "trace warning" is a `log::warn!` under `paladin::cadence`, with no `TraceEvent` variant (Open Question 5,
resolved).** PACE-05 says "trace warning". A `TraceEvent` variant would be a published-schema change: the enum is
`#[non_exhaustive]` but matched in about a dozen files, feeds the persisted run trace and the SSE wire, and the
decorator has no node id to attach one to. `log::warn!(target: "paladin::cadence", ..)` satisfies the requirement,
is testable with a capturing logger (`one_warning_per_outage_and_one_recovery_line`), and costs no schema. A
`TraceEvent::CadenceGated` variant, if the operator ever wants gated waits visible in run traces, is its own plan
(schema, sinks, goldens). Gated calls and paced fallback attempts log at `debug`/`trace` naming provider, model and
durations only.

### (f) On by default, one shared wiring (D-08, D-10)

**D-08. On by default, in process.** Omitting `treasurer.cadence` yields in-process pacing with the default back-off on
every `LlmPort` the server composes: the resident agents (`build_agent`), the runtime provisioner
(`FacadeProvisioner`) and the run engine's port (`paladin_port_from_settings_with_ledger`). An explicit `enabled:
false` installs nothing. `paladin-server` builds **one** `CadenceWiring` and passes clones to all three through the
new `*_with_cadence` entry points, so a `429` seen through any of them delays the next call through the others; the
pre-existing entry points keep their signatures, build a wiring from settings and delegate. The run engine's port goes
through `compose_llm`, so no `with_pricing(` call remains in `facade_provisioner.rs`. **Reversibility: costly** (an
on-by-default behaviour change is a MIGRATION 9.1 row, M-B-05, and turning it back to opt-in would be another).

**Composition order.** The decorators nest as `Pricing(Cadence(provider))` for a single provider and as
`Pricing(Fallback(Cadence(hop1), Cadence(hop2), ..))` for a chain. Pricing is outermost so it still sees the served
response and prices only calls that were actually sent; Cadence is innermost so it gates each provider call, including
each hop, under that hop's own `(provider, model)` key. The rejected order, `Cadence(Pricing(Fallback(..)))`, would
gate a whole chain as one key named `"fallback"` and make D-03 impossible.

**D-10. Config shape and defaults.** `treasurer.cadence { enabled = true, backend: in_process | redis { url_env },
base_backoff_ms = 500, max_backoff_ms = 30000, max_wait_secs = 300, degraded_multiplier = 2.0,
fallback_pace_budget_secs = 60, lock_ttl_secs = 120 }`. `lock_ttl_secs` is **120, not CONTEXT's 60** (Open Question 3,
resolved: the planner may tune numerics within an order of magnitude, and the research finding is that a lock shorter
than the node's p99 duration, retries included, is overtaken). The struct is `deny_unknown_fields`; `validate()` names
the offending key and rejects zero durations, a multiplier below 1.0 (or non-finite), and a `redis` backend whose
`url_env` is empty or names an unset variable. Seven scalar keys have `APP_TREASURER_CADENCE_*` overrides through
`EnvOverridable`; `backend` has no env form. The config carries the NAME of the environment variable, never the URL.
**Reversibility:** the numeric defaults are reversible; the key shape is operator-facing and follows D-07's rating.

### (g) The Redis adapter, the stampede lock and the fence (D-09, D-11..D-14, D-00f; Open Questions 2 and 3)

**D-09. Redis adapter in `paladin-storage`, feature `redis-cadence`.** `RedisCadence` lives beside `node_cache/redis.rs`
and `run_queue/redis.rs`, shares the already-declared optional `redis` dependency, and the feature is `redis-cadence =
["dep:redis"]` on `paladin-storage` with a facade passthrough, in neither `default`, `storage` nor `full`.
`paladin-llm` gains no Redis dependency (`cargo tree -p paladin-llm -e normal | grep -c ' redis v'` is 0), and
`paladin-battalion` depends on the `CadencePort` trait only. The three pacing scripts (`CADENCE_RECORD_429_LUA`,
`CADENCE_GATE_LUA`, `CADENCE_RECORD_SUCCESS_LUA`) each read `redis.call('TIME')`, return **relative** waits so skew
between workers cannot shorten or lengthen one, set `PEXPIRE` on every write, and create no key on a read. The shared
contract suite (`contract_tests::run_all`) passes unchanged against the in-memory adapter, the Redis adapter at both
jitter ends, and the resilient composite healthy and degraded. **Reversibility: costly** (the feature name is a
published Cargo surface).

**Key layout.** Pacing keys are `<prefix>:<provider>:<model>` with `%` and `:` percent-escaped in the provider half only
(ordinary keys are written verbatim), so `("a:b", "c")` and `("a", "b:c")` cannot collide. Lock and fence keys are
`<prefix>:%lock:<key>` and `<prefix>:%fence:<key>`; a provider half never contains `%l` or `%f`, so no pacing key equals
a lock or fence key (the plan's literal `<prefix>:lock:<key>` layout collided with a provider named `lock` and would
have raised `WRONGTYPE`; `a_lock_key_can_never_be_a_pacing_key` fails under it). A forged or corrupted wait read back
from Redis is clamped to the 24 h ceiling, so the decorator is never handed an unbounded wait.

**The namespace is fixed for the server.** The prefix is the constant `paladin:cadence` (`DEFAULT_CADENCE_KEY_PREFIX`).
`RedisCadenceConfig::with_key_prefix` exists for a library user who builds the adapter by hand, but `treasurer.cadence`
has no `key_prefix` key by design and `build_cadence` never sets one. Two independent fleets pointed at one Redis server
through the shipped server therefore share pacing state for the same provider and model. That is a conscious trade for a shared provider key, and a limitation when the fleets use
different provider keys (group (i)).

**D-11. The lock lives on `CadencePort`; the engine calls it.** `CadencePort` gained required methods `try_lock(key,
ttl) -> Result<Option<FencingToken>, CadenceError>` and `unlock(key, token) -> Result<bool, CadenceError>`, with
`LockKey` and `FencingToken`. `WarEngine::with_cadence(port, lock_ttl)` puts the lock around the node cache's
miss-to-store window through a private `NodeCacheBinding.lock`; a nested Battalion run inherits it with the cache. A
cache-policy node therefore executes once for N concurrent identical misses. **Reversibility: costly** (the engine's
cache-miss path and the port's method set are published).

**D-12. A lock loser waits and re-reads, then executes.** A loser polls `cache.get` every `STAMPEDE_POLL_INTERVAL`
(100 ms plus up to 50 percent jitter, capped at TTL/10 and never longer than the time left) until the entry appears or
the lock TTL elapses, then serves the hit; if the TTL passes with no entry it executes uncached rather than stall or
error. **Each poll also re-tries the lock**, because a holder that failed or released without writing would otherwise
leave every waiter idle until the TTL (research Pitfall 14); `failed_holder_hands_the_lock_over_within_one_poll`
proves the handover. **The winner re-reads the cache once after acquiring** (43-11): a dispatch that missed, waited
while the holder stored and released, then won `try_lock` would otherwise run a duplicate. That costs one extra `get`
per locked miss and nothing when no lock is configured. The wait goes through `retry::wait_backoff`, so the run's
cancellation token wins and a waiting dispatch is interrupted promptly.

**D-13. The lock fails open onto an in-process lock.** `ResilientCadence::try_lock` follows the same route/latch/probe
rules as pacing: healthy goes to the primary; an error latches (one warning shared with the pacing outage) and the same
call is served by the fallback with a `Local` token; `unlock` routes by token source (`Local` to the fallback,
`Distributed` to the primary, `Ok(false)` immediately while degraded). Any `Err` or `None` from the lock means "proceed
without it" at the engine, so the node always executes and no lock error can fail a run. Fail-closed was rejected
because the node cache exists purely as an optimisation (D-29).

**D-14. TTL from config, fencing token from a Redis counter.** `lock_ttl_secs` sets the `SET NX PX` expiry (a zero TTL
is raised to 1 ms and an excessive one clamped to the ceiling, because Redis rejects `PX 0`). The token is an `INCR`
on a per-key counter whose TTL is `max(10 * lock TTL, 1 h)`; `unlock` is a Lua delete-only-if-token-matches. The
in-process adapter uses an `AtomicU64`. `FencingToken` is `#[non_exhaustive]` and tagged by source (`Distributed(u64)`,
`Local(u64)`), deliberately has no `Ord`, and `Local(1) != Distributed(1)`, so a process-local token can never be
compared with a Redis one.

**The one deliberate reading of D-00d/D-11: `NodeCachePort::put_fenced` (Open Question 2, resolved).** D-14 says the
token is handed to the Redis `NodeCachePort` adapter's `put` path so a stale holder's late write is ignored; D-00d and
D-11 say `NodeCachePort` is unchanged. Both cannot hold literally, because `put` has no token parameter and a fence that
nobody checks is not fencing. The planner read "unchanged" as "its contract and every implementor are unchanged" and
added **one defaulted method**, `put_fenced(key, delta, ttl, &FencingToken)`, whose default body calls `put`. No
implementor breaks (`paladin-battalion`'s test doubles compile unchanged), the D-29 best-effort contract is untouched,
and only `RedisNodeCache` overrides it: a `Local` token and a zero TTL behave exactly as `put`; a `Distributed(t)` runs
`NODE_CACHE_PUT_FENCED_LUA`, which ignores a write whose token is strictly lower than the last seen for the key (an
ignored write is `Ok(())` with one `debug` line naming only the token). Equal and higher tokens are accepted. **If the
operator rejects this reading,** the fallback is an inherent `RedisNodeCache::put_fenced` the engine cannot reach
through `Arc<dyn NodeCachePort>`, which leaves lock-only protection. A known consequence: `RedisNodeCache::invalidate`
counts a fence marker, because the marker sits at `<entry key>:fence` inside the prefix it scans.

**No lock renewal (Open Question 3, resolved).** The lock is not extended while a node runs; the mitigation is the
longer default `lock_ttl_secs = 120` and the documentation that it must exceed the p99 node duration including retries.
A holder that outlives the TTL can be overtaken, and its late write is refused only by a fence-checking cache. With the
in-process Cadence the tokens are `Local`, which no cache compares, so there the fence is a no-op and a late write is
last-write-wins (the entry is a deterministic result either way). An owner-checked `extend_lock` plus a guard task is
one more script and is left unbuilt unless the operator asks. The server attaches no node cache today, so
`paladin-server` attaches no lock and `lock_ttl_secs` has no effect on it.

### (h) Assumptions flagged by plans 43-07 and 43-08

These are recorded in the module docs of `cadence/redis.rs` and `cadence/resilient.rs` and in MIGRATION 9.3; the
ADR restates them so an operator reading only the decision record sees them.

- **A1 (43-07): one Redis primary.** The scripts assume a single writable primary; Redis Cluster key slotting across the
  hash, counter and lock keys is not handled.
- **A2 (43-07): Redis 5 or later.** Verified on 7.0.15. Redis 7 stringifies a numeric `nb` with `%.17g`, which prints a
  16-digit microsecond timestamp as plain digits; the scripts still write it with `string.format('%d', ..)` as a
  defence against builds that use `%.14g`, though that guard cannot be proven on this server version.
- **A3 (43-07): the fixed `paladin:cadence` namespace** shared by independent fleets (group (g)).
- **A1 (43-08): the outage blind spot.** During an outage a worker cannot see other workers' `429`s; the multiplier is a
  heuristic, not a guarantee.
- **A2 (43-08): pre-outage gates are invisible.** Gates written to Redis before the outage are invisible to the
  fallback, so the first post-latch call may send without waiting out a pre-outage fleet gate (it is still paced,
  reactively).
- **A3 (43-08): recovery is not instant and does not copy back.** Recovery is detected within one probe interval plus a
  round trip, and locally recorded gates are honoured after recovery (the larger of the two readings wins) but are not
  copied back into Redis.

### (i) Known limitations

Stated plainly, because "a run is never unpaced" is a bounded guarantee:

- **The outage blind spot** (A1-A3 of 43-08 above): during a Redis outage pacing is per process, with a stricter
  multiplier, not fleet-wide.
- **The fixed `paladin:cadence` namespace** (A3 of 43-07): independent fleets on one Redis share state.
- **Gemini's body-level `RetryInfo`** is not parsed (Open Question 6); Gemini gets `Retry-After` and otherwise the
  back-off estimate.
- **CLI one-shot commands** are outside D-08's composition roots (agent host, facade provisioner, run engine port), so
  they are not paced.
- **Hand-built `FallbackLlmAdapter` chains** that never call `with_cadence` keep hopping immediately on a `429`.
- **No lock renewal** and the stale-write window it leaves (group (g)).
- **The compat engine's `error_override` hook** runs first and receives no hints; no shipped preset sets one for `429`.

### (j) Provider header evidence (plan 43-12)

Every header name and error code the parser and the quota mapping depend on is recorded in
`.planning/phases/43-rate-pacing/43-PROVIDER-HEADER-EVIDENCE.md`, 22 names, each with a source URL and date, and each
cited in the constant's rustdoc (eight "Verified against" citations in `rate_limit_headers.rs`). The Anthropic rows are
`VERIFIED (quote)`: the executor fetched `platform.claude.com/docs/en/api/rate-limits` and `/errors` on 2026-10-09 and
quoted `retry-after`, the twelve `anthropic-ratelimit-{requests,tokens,input-tokens,output-tokens}-{limit,remaining,
reset}` headers (RFC 3339 resets) and `enforced_spend_limit_reached` (the spend-cap `429` has no `retry-after`). The
OpenAI rows are `VERIFIED (operator, 2026-10-09)`: the sandbox could not reach `platform.openai.com`,
`developers.openai.com`, `help.openai.com` or `cookbook.openai.com`, so the operator confirmed the six
`x-ratelimit-{limit,remaining,reset}-{requests,tokens}` names, the Go-style reset format (`1s`, `6m0s`, `6ms`), the
optional seconds-valued `Retry-After`, that `retry-after-ms` is **not** documented by OpenAI (it stays an optional extra
for the OpenAI-compatible family), and `insufficient_quota`. An operator attestation is not a fetched quote and the
evidence file says so; no corrections were given, so no constant or branch changed.

## Considered Options

Alternatives from the discussion log (`43-DISCUSSION-LOG.md`), one line each, then the alternatives the research and the
planner weighed.

Back-off ownership:

- **The decorator retries itself, or gates and retries once** (rejected, D-01): triples the attempt count with
  `RetryPolicy` and the adapters, and hides the first `429` from the layers that own retries.
- **Keep the adapter loops and honour `Retry-After` inside them, or leave the adapters alone** (rejected, D-02): two
  layers would sleep on one signal, or the thrash would hide below the decorator and the provider's delay be lost on
  the first hit.
- **D-02 limited to the three adapters CONTEXT names** (rejected, Open Question 1): DeepSeek and Gemini are in the
  conformance suite and would fail the new case or keep a hidden loop.
- **Hop now and gate A for later calls, hop unless B is gated too, or never hop on a `429`** (rejected, D-03): the first
  two break benchmark consistency, tool-call format and the provider's prompt cache for a condition that loses no state;
  the last strands a run on a provider that is down for a reason other than pacing.

Pacing semantics and degradation:

- **Proactive gating from remaining/reset headers, now or later** (deferred, D-04): the parse ships, the gate does not.
  Additive later.
- **A local gate with the same delays, or with a fixed floor, when Redis is down** (rejected, D-05): the first ignores
  that this worker can no longer see the fleet's `429`s; the second adds a tuning knob nobody asked for.
- **Honour any retry delay in full, or reclassify a long delay as provider-down** (rejected, D-06): a worker lease held
  for an hour in silence, or a transient condition recast as a permanent one.
- **A new `TraceEvent` variant for gating and degraded mode** (rejected, Open Question 5): a published-schema change in
  about a dozen files for no requirement; `log` under `paladin::cadence` meets PACE-05.

Naming, config and placement:

- **Quartermaster, or the plain name Pacing** (rejected, D-07): the operator chose Cadence over the recommended name.
- **Opt-in pacing** (rejected, D-08): leaves deployments unpaced by default.
- **Folding the Redis adapter under `redis-cache`, or giving `paladin-llm` its own Redis dependency** (rejected, D-09): a
  published feature should say what it does, and the decorator holds `Arc<dyn CadencePort>` only.
- **Wrapping hops at an `LlmProviderFactory` field** (rejected, Open Question 4): a public-surface change to a unit
  struct; the chain builder wraps each hop where it is made.
- **`Cadence(Pricing(Fallback(..)))`** (rejected): gates a chain as one key and breaks D-03.
- **Calling `backoff_delay` from the battalion crate** (rejected, research finding 4): the dependency arrow points the
  other way and its jitter is additive; `CadencePolicy` carries full jitter.
- **A `key_prefix` config key** (deferred): the fixed `paladin:cadence` namespace is shared by design in the server; a
  config key can be added later without breaking the default (the adapter already has `with_key_prefix`).
- **Parsing Gemini's `RetryInfo` body** (deferred, Open Question 6, assumption A8): low impact, optional.

Stampede lock:

- **Lock methods on `NodeCachePort`, or a separate `StampedeLockPort`** (rejected, D-11): `NodeCachePort` stays a cache
  and the lock stays with the other Redis-coordinated state.
- **A loser that executes uncached immediately, or waits for the holder's result only** (rejected, D-12): the first
  duplicates the provider call in the common case, the second can stall behind a crashed holder.
- **No lock at all when Redis is down, or fail closed** (rejected, D-13): the first loses same-worker coalescing, the
  second lets an optimisation fail a run.
- **A TTL derived from the node's timeout, or a fixed TTL with a random token** (rejected, D-14): the node timeout is
  not the lock's concern, and a random token cannot be compared for staleness.
- **Lock-only protection with no fence** (rejected, Open Question 2): a stale holder's late write could overwrite a newer
  entry; the defaulted `put_fenced` adds the check without breaking an implementor.
- **An inherent `RedisNodeCache::put_fenced`** (the stated fallback if the operator rejects the reading): unreachable
  through the trait object.
- **Lock renewal (`extend_lock`)** (deferred, Open Question 3): a longer default TTL and documentation instead.
- **Treating an unreadable gate as a zero wait in the fallback chain** (rejected, 43-06): an immediate retry loop.

## Code Locations

Owning plans are named.

- `crates/paladin-ports/src/output/cadence_port.rs` - `CadencePort`, `CadenceKey`, `GateReading`, `CadenceError`,
  `CadencePolicy`, `LockKey`, `FencingToken`, `try_lock`/`unlock`, `CADENCE_LOG_TARGET`, `CADENCE_DELAY_CEILING`;
  plans 43-01, 43-10
- `crates/paladin-ports/src/output/rate_limit_hints.rs` - `RateLimitHints`, `RateLimitDimension`,
  `RateLimitDimensionKind`, `RetryDelaySource`; plan 43-02
- `crates/paladin-ports/src/output/llm_port.rs` - the struct-shaped `RateLimitExceeded`, `rate_limited`,
  `rate_limited_with_hints`, `retry_after`, `rate_limit_hints`; plan 43-02
- `crates/paladin-ports/src/output/node_cache_port.rs` - the defaulted `put_fenced`; plan 43-10
- `crates/paladin-llm/src/cadence.rs` - `CadenceLlmAdapter`, `CadenceSettings`, `CadenceWiring`, `with_cadence`, the
  gate wait, the D-06 refusal, the waiter spread, streaming; plans 43-01, 43-02, 43-05
- `crates/paladin-llm/src/rate_limit_headers.rs`, `http_status.rs` - the shared parser and
  `map_http_status_with_hints`; plans 43-03, 43-12 (the verified-source rustdoc)
- `crates/paladin-llm/src/{openai/adapter,anthropic/adapter,compat/engine,deepseek/adapter,gemini/adapter}.rs` - the
  header snapshot, first-`429` surfacing, quota-class mapping; plans 43-01, 43-03, 43-04
- `crates/paladin-llm/src/conformance.rs` - case 10; plan 43-04
- `crates/paladin-llm/src/fallback.rs` - `with_cadence`, the per-hop pace loop, the rewritten module docs; plan 43-06
- `crates/paladin-storage/src/cadence/{mod,in_memory,redis,resilient,contract_tests,test_logger}.rs` -
  `InMemoryCadence`, `RedisCadence`, `ResilientCadence` and the shared contract; plans 43-01, 43-05, 43-07, 43-08, 43-10
- `crates/paladin-storage/src/redis_url.rs` - `redact_connection_url`, lifted from the run queue; plan 43-07
- `crates/paladin-storage/src/node_cache/{in_memory,redis}.rs` - the fenced write and `NODE_CACHE_PUT_FENCED_LUA`; plan
  43-10
- `crates/paladin-battalion/src/engine/{mod,superstep}.rs` - `WarEngine::with_cadence`, `NodeCacheBinding.lock`, the
  lock loop; plan 43-11
- `src/config/treasurer.rs` - `CadenceConfig`, `CadenceBackend`, `validate()`, the `APP_TREASURER_CADENCE_*` overrides;
  plans 43-01, 43-09
- `src/infrastructure/cadence.rs` - `build_cadence`, `compose_llm`; plans 43-01, 43-09
- `src/infrastructure/web/{agent_host,facade_provisioner,run_api_wiring}.rs`, `src/bin/paladin-server.rs` - the
  `*_with_cadence` composition roots and the one shared wiring; plan 43-09
- `src/application/services/paladin/middleware/resilience.rs`, `src/config/agent_runtime.rs` -
  `ModelFallbackMiddleware::paced`, `AgentRuntimeDeps.cadence`; plan 43-06
- `.github/workflows/ci.yml` - the `redis-cadence-integration` job; plan 43-07
- `docs/src/getting-started/configuration.md`, `docs/src/deployment-topologies/http-service-host.md` - the operator
  pages; plan 43-09. `docs/src/appendix/provider-expansion.md`, `docs/src/contributing/contributing-providers.md` -
  the `RateLimitExceeded` examples corrected; plan 43-13
- `MIGRATION.md` (9.1 M-B-05, 9.2 rows, 9.3, 9.5), `CHANGELOG.md` and the four crate changelogs; every plan, closed by
  43-13
- `.planning/phases/43-rate-pacing/43-PROVIDER-HEADER-EVIDENCE.md` - plan 43-12
- `.github/copilot-instructions.md` - the `Cadence` row of the term table; plan 43-13

## Code Conformance

conforms

Re-read against the tree at the close of Phase 43 (plan 43-13): every decision above is built, and each test named
below was found in the tree by `grep` before it was written down.

- **D-01, D-02:** `cadence_tracer_paces_a_real_openai_429_end_to_end` (`src/infrastructure/cadence.rs`),
  `rate_limit_is_surfaced_once_with_its_retry_delay` (`crates/paladin-llm/src/conformance.rs`, nine instantiations),
  `openai_429_is_surfaced_on_the_first_attempt`, `anthropic_429_is_surfaced_once_with_its_retry_delay_and_four_dimensions`,
  `call_api_with_retry_surfaces_rate_limit_exceeded_on_the_first_attempt` (compat engine and DeepSeek).
- **Quota-class mapping:** `openai_insufficient_quota_429_maps_to_usage_limit_exceeded_once` and
  `openai_stream_insufficient_quota_429_maps_to_usage_limit_exceeded` (`crates/paladin-llm/src/openai/adapter.rs`).
- **D-03:** `mocked_429_on_a_fallback_hop_backs_off_and_stays_on_the_provider`,
  `delayless_429s_on_a_hop_back_off_with_jitter`, `pace_budget_boundary_paces_at_the_budget_and_hops_beyond_it`,
  `streaming_429_on_a_hop_paces_before_hopping`, `a_broken_gate_hops_instead_of_retrying_hot`,
  `chain_without_cadence_still_hops_on_a_429` (`crates/paladin-llm/src/fallback.rs`).
- **D-04, D-06:** `explicit_retry_after_is_honoured_as_a_minimum`, `gate_one_ms_beyond_max_wait_surfaces_immediately`
  (`crates/paladin-llm/src/cadence.rs`) and the shared clauses in `crates/paladin-storage/src/cadence/contract_tests.rs`
  (`first_delayless_429_gates_for_base`, `escalation_doubles_the_ceiling_after_the_gate_clears`,
  `in_flight_429_does_not_escalate`, `success_resets_the_streak_and_is_idempotent`).
- **D-05:** `one_warning_per_outage_and_one_recovery_line` (`crates/paladin-storage/src/cadence/resilient.rs`) and
  `cadence_with_redis_down_still_paces` (`src/infrastructure/cadence.rs`), the never-unpaced proof.
- **D-08:** `one_wiring_paces_the_agent_host_and_the_run_engine_together`, `disabled_cadence_composes_pricing_only_at_every_site`
  (`src/infrastructure/web/facade_provisioner.rs`),
  `provisioner_without_the_feature_rejects_a_redis_backend_instead_of_pacing_in_process`.
- **D-09, D-00f:** `redis_cadence_passes_the_shared_contract`, `state_key_is_injective_across_the_provider_model_boundary`,
  `a_lock_key_can_never_be_a_pacing_key` (`crates/paladin-storage/src/cadence/redis.rs`) and
  `cadence_fleet_429_on_one_worker_slows_the_other` (`src/infrastructure/cadence.rs`).
- **D-11, D-12:** `concurrent_identical_cache_misses_execute_the_node_once`, `failed_holder_hands_the_lock_over_within_one_poll`,
  `ttl_elapsed_without_an_entry_executes_uncached`, `cancellation_interrupts_a_waiting_dispatch`,
  `a_winner_that_finds_the_entry_after_acquiring_serves_it` (`crates/paladin-battalion/src/engine/mod.rs`).
- **D-13, D-14:** `unlock_only_by_the_owner` (`contract_tests.rs`), the resilient fail-open test, and
  `redis_put_fenced_accepts_an_equal_or_higher_token` (`crates/paladin-storage/src/node_cache/redis.rs`).

This section records mutation checks where the owning plans ran them (for example the in-flight rule in 43-05 and 43-07,
the budget comparison in 43-06, the lock release in 43-11); plans 43-08, 43-10 and 43-11 wrote implementation and tests
together, so for those the mutation checks, not a red run, stand as the evidence the tests can fail.

## Downstream Consumers

- **Phase 46 (docs currency, CURR-23)** - the Treasurer mdBook page reads this ADR, with ADR-0056 and ADR-0057, for the
  pacing model, the degraded-mode wording and the known limitations; the `v0.10` to `v0.11` migration guide carries the
  MIGRATION.md rows this phase added (9.1 M-B-05, the 9.2 Phase 43 rows, 9.3 `redis-cadence`, `httpdate` and `rand`, and
  the 9.5 `treasurer.cadence` subtree).
- **Phase 44 (the `LlmError` change it builds on)** - `RateLimitExceeded` is now a `#[non_exhaustive]` struct variant
  matched as `RateLimitExceeded { .. }`; any later `LlmError` work builds on that shape and on `retry_after()`.
- **Any future proactive-pacing phase (the deferred idea)** - the remaining/reset parse already ships on the error, so
  gating on it is additive over the reactive gate and the `CadencePort` contract; it would add a reading source, not
  change a method.
- **A `TraceEvent::CadenceGated` variant, if wanted** - its own plan (schema, sinks, goldens); the log target is the
  seam.
- **A lock-renewal follow-up and a `key_prefix` config key** - both additive over the shapes recorded here, and both
  conditional on an operator need.
- **Phase 47 (release mechanics)** - the new `redis-cadence` feature, the direct `httpdate` edge and the required `rand`
  dependency are published Cargo surface for the release.
