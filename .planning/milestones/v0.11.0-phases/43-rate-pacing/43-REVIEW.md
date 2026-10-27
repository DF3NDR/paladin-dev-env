---
phase: 43-rate-pacing
reviewed: 2026-10-09T00:00:00Z
depth: standard
files_reviewed: 61
files_reviewed_list:
  - crates/paladin-battalion/src/commander.rs
  - crates/paladin-battalion/src/conclave_execution_service.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/src/llm_decision.rs
  - crates/paladin-battalion/src/llm_failure.rs
  - crates/paladin-eval/src/scenario.rs
  - crates/paladin-llm/src/anthropic/adapter.rs
  - crates/paladin-llm/src/cadence.rs
  - crates/paladin-llm/src/compat/engine.rs
  - crates/paladin-llm/src/conformance.rs
  - crates/paladin-llm/src/deepseek/adapter.rs
  - crates/paladin-llm/src/error.rs
  - crates/paladin-llm/src/fallback.rs
  - crates/paladin-llm/src/gemini/adapter.rs
  - crates/paladin-llm/src/grok/adapter.rs
  - crates/paladin-llm/src/http_status.rs
  - crates/paladin-llm/src/kimi/adapter.rs
  - crates/paladin-llm/src/lib.rs
  - crates/paladin-llm/src/mock.rs
  - crates/paladin-llm/src/openai/adapter.rs
  - crates/paladin-llm/src/openai_compatible/adapter.rs
  - crates/paladin-llm/src/pricing.rs
  - crates/paladin-llm/src/qwen/adapter.rs
  - crates/paladin-llm/src/rate_limit_headers.rs
  - crates/paladin-ports/src/output/cadence_port.rs
  - crates/paladin-ports/src/output/llm_port.rs
  - crates/paladin-ports/src/output/mod.rs
  - crates/paladin-ports/src/output/node_cache_port.rs
  - crates/paladin-ports/src/output/rate_limit_hints.rs
  - crates/paladin-storage/src/cadence/contract_tests.rs
  - crates/paladin-storage/src/cadence/in_memory.rs
  - crates/paladin-storage/src/cadence/mod.rs
  - crates/paladin-storage/src/cadence/redis.rs
  - crates/paladin-storage/src/cadence/resilient.rs
  - crates/paladin-storage/src/cadence/test_logger.rs
  - crates/paladin-storage/src/lib.rs
  - crates/paladin-storage/src/node_cache/in_memory.rs
  - crates/paladin-storage/src/node_cache/redis.rs
  - crates/paladin-storage/src/redis_url.rs
  - crates/paladin-storage/src/run_queue/redis.rs
  - src/application/services/paladin/middleware/resilience.rs
  - src/bin/paladin-server.rs
  - src/config/agent_runtime.rs
  - src/config/mod.rs
  - src/config/treasurer.rs
  - src/infrastructure/cadence.rs
  - src/infrastructure/mod.rs
  - src/infrastructure/web/agent_host.rs
  - src/infrastructure/web/facade_provisioner.rs
  - src/infrastructure/web/run_api_wiring.rs
  - tests/cli/error_handling_test.rs
  - tests/cli/formation_execution_test.rs
  - tests/cli/paladin_execution_test.rs
  - tests/integration/openai_content_analysis_integration_test.rs
  - tests/integration/paladin_integration_test.rs
  - tests/unit/llm/anthropic_adapter_test.rs
  - tests/unit/llm/deepseek_adapter_test.rs
  - tests/unit/mock_llm_adapter_test.rs
findings:
  critical: 0
  warning: 7
  info: 7
  total: 14
status: issues_found
---

# Phase 43: Code Review Report

**Reviewed:** 2026-10-09
**Depth:** standard
**Files Reviewed:** 61 Rust sources (CHANGELOG/MIGRATION/docs/CI entries read as context only)
**Status:** issues_found

## Summary

The pacing core is carefully built. The Lua scripts take `TIME` on the server, return relative waits, and receive keys and durations only through `KEYS`/`ARGV`. I traced the Lua against `CadencePolicy::delay_for` and the in-memory adapter and found the three in agreement on the closed boundary, the in-flight rule, the jitter floor, the streak cap and the 24 h clamps. The lock and pacing key spaces cannot collide, because `%` is escaped in the provider half. Tokens are issued only to the lock winner, since `INCR` and `SET` share one script and `EXISTS` is checked first. `unlock` is a compare-and-delete. The stampede lock is released on every exit of the attempt loop, and I found no early `return` inside it. The header parser is bounded and panic-free on hostile input.

Credential handling meets the checklist. The Redis URL is redacted through the shared helper. `RedisCadenceConfig` and `RedisCadence` implement `Debug` by hand. The URL appears in no error, because the `Client::open` error is discarded. The boot log names only the backend kind. The header snapshot reads only rate-limit and `date` headers and stores parsed integers and `Duration`s. No new HTTP client was added, and the existing no-redirect clients are unchanged.

I found no BLOCKER. The defects are behavioural: one TTL mismatch between the fencing counter and the node-cache fence marker, two behaviour changes that the docs and config describe as "previous behaviour restored", one config bypass in the default fallback wiring, one cancellation window that loses the very 429 the gate exists for, a zero-delay edge, and a key-space collision in the node cache.

## Warnings

### WR-01: Fencing counter can expire before the node-cache fence marker it numbers, so valid writes are refused

**File:** `crates/paladin-storage/src/cadence/redis.rs:556` (counter TTL) and `crates/paladin-storage/src/node_cache/redis.rs:306` (marker TTL)
**Issue:** The cadence fencing counter lives for `max(10 x lock_ttl, 1 h)`, refreshed on each acquisition (default lock TTL 120 s, so 1 h). The node-cache fence marker lives for `max(2 x entry_ttl, 1 h)`. The two are not related.

For a `CachePolicy` TTL above 30 minutes, the counter can expire while the marker is still alive. Take an entry with a 2 h TTL. The marker lives about 4 h. The counter expires after 1 h, because an acquisition happens only on a miss.

Suppose the previous epoch left token N > 1 in the marker, for example because a holder crashed or was overtaken. When the entry expires at 2 h, the next `try_lock` restarts the counter at 1. `put_fenced` sees `1 < N` and silently returns `Ok(())`, treating the write as stale. Each later miss does the same until the counter climbs past N or the marker expires. The node runs and pays for the LLM call every time, and its result is never cached. This defeats PACE-04.

The reverse also holds. An equal token from a new epoch is accepted, so a holder from a previous epoch is not distinguished from a current one.

The cadence module comment (research Pitfall 13) says the counter must outlive any lock it numbers. It does not say it must outlive the markers that store its tokens.
**Fix:** Make the two lifetimes compatible, or make tokens epoch-safe.
```rust
// Option A (smallest): derive the counter TTL from the longest consumer, not the lock TTL.
// e.g. a 7-day floor and a configurable ceiling, and document that it must be >= 2 * the
// largest CachePolicy ttl in use.
const MIN_FENCE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
// Option B: cap the node-cache marker at the counter's guaranteed lifetime, so a marker never
// outlives the counter that produced its token:
let fence_ttl_ms = ttl_ms.saturating_mul(2).clamp(MIN_FENCE_TTL_MS, MAX_FENCE_TTL_MS);
```
Add a Redis-backed test: fill with token 2, expire the counter, then assert that a token 1 write is accepted or that the cache is not starved.

### WR-02: Surfacing the first 429 removes adapter-level back-off, but `enabled: false` is documented as restoring previous behaviour

**File:** `crates/paladin-llm/src/openai/adapter.rs:560-572` (the same change is in `anthropic`, `deepseek`, `gemini` and `compat/engine`); docs in `docs/src/getting-started/configuration.md:545` and `src/config/treasurer.rs:44`
**Issue:** Every adapter's retry loop now returns `RateLimitExceeded` on the first attempt, with no sleep. The DeepSeek test even changed from "retries 4 times" to "1 call". The pacing layer is the only thing that now waits after a 429. With `treasurer.cadence.enabled: false`, `compose_llm` installs no decorator, so a 429 is returned immediately with no wait at all. The adapters previously retried with jittered exponential back-off.

The docs say `enabled: false` "restores the previous behaviour". It does not. The same holds for any library user who builds an adapter directly. Operators who disable pacing, perhaps because it misbehaved, silently lose all 429 back-off unless a `RetryPolicy` is configured above the adapter.
**Fix:** Correct the docs and the `CadenceConfig` rustdoc to state that disabling pacing also disables 429 back-off. Alternatively, keep an adapter-level retry gated on a "paced" flag. Also add a CHANGELOG/MIGRATION note under `paladin-llm` saying that adapters no longer retry a 429 by themselves.

### WR-03: `AgentRuntimeDeps::default()` installs a private pacing wiring that ignores `treasurer.cadence`

**File:** `src/config/agent_runtime.rs:306-317, 337`
**Issue:** `default_cadence()` builds `InMemoryCadence` from `CadenceConfig::default()`. A `model_fallback` chain built via `AgentRuntimeConfig::build_chain(&AgentRuntimeDeps::default())` is therefore:
- paced even when the operator set `treasurer.cadence.enabled: false`;
- paced with default timings rather than the configured `max_wait`, `max_backoff` and `fallback_pace_budget`;
- paced with a private in-memory gate that is not the process-wide wiring, and never the Redis backend.

A 429 seen through that chain does not gate the agent host or the run engine, which contradicts the "one wiring per process" rule that `infrastructure/cadence.rs` states. `examples/agent_runtime_middleware.rs` uses exactly this default, and `paladin-server` supplies no `AgentRuntimeDeps` at all.
**Fix:** Default `cadence` to `None`, or make it a required constructor argument, and let composition roots pass the shared wiring explicitly. If the default is kept, document that it bypasses `treasurer.cadence`.

### WR-04: The 429 is recorded inline after the response, so cancelling the caller loses the signal

**File:** `crates/paladin-llm/src/cadence.rs:402-404` (`generate`) and `:436-445` (`generate_stream`), via `note_failure` at `:274`
**Issue:** After `inner.generate` returns a 429, `note_outcome` awaits `record_rate_limited` on the port, which for Redis may take up to the 1 s operation budget. If the caller's future is dropped in that window, for example by an outer `tokio::time::timeout`, a run cancel, or a grace-deadline abort (the engine aborts tasks mid-await), the 429 is never recorded. `ResilientCadence` is dropped mid-await too, so its fallback never records it. The provider has told us to slow down, and the whole fleet then sends into the limit.

The window is small but is reachable in exactly the situations (timeouts, shutdown) where 429s cluster.
**Fix:** Make the record cancellation-safe. For example, `tokio::spawn` the `record_rate_limited` future (cloning the `Arc<dyn CadencePort>`), or record into the in-process fallback synchronously first. Add a test that drops the future while the port's `record_rate_limited` is pending.

### WR-05: A zero delay (`Retry-After: 0`, past HTTP-date, or exhausted-dimension reset of 0) produces a zero-length gate

**File:** `crates/paladin-ports/src/output/cadence_port.rs:339-341`, `crates/paladin-llm/src/cadence.rs:290-300`, `crates/paladin-ports/src/output/rate_limit_hints.rs:effective_retry_after`
**Issue:** `delay_for(.., Some(Duration::ZERO), ..)` returns `ZERO`, and the unit test pins this ("even below base it is not raised"). `parse_retry_after` returns `Some(ZERO)` for `0` and for any HTTP-date at or before the reference. `effective_retry_after` returns `(0, ResetHeader)` for an exhausted dimension that reports a zero reset.

The result is `nb == now`, so no gate, and the streak is incremented. Combined with WR-02, nothing paces the next call. Some gateways emit `Retry-After: 0` on every 429, and an HTTP-date in the past is what clock skew or a stale `Date` header produces.

A literal `Retry-After: 0` could arguably be honoured. A delay derived from a reset header, which the decorator already treats as an over-estimate, should not be allowed to disable pacing.
**Fix:** Treat a zero delay as "no usable delay" before it reaches the port.
```rust
// cadence.rs minimum_delay
let delay = err.retry_after()?;
if delay.is_zero() { return None; } // let the policy's base back-off floor apply
```
Alternatively, floor `Some(d)` at the policy `base` for `RetryDelaySource::ResetHeader` only. Keep the unit test, but add one for the decorator.

### WR-06: `insufficient_quota` and spend-cap 429s become `Permanent`, which stops fallback chains from failing over

**File:** `crates/paladin-llm/src/openai/adapter.rs:389` and `crates/paladin-llm/src/anthropic/adapter.rs:395`; interaction with `crates/paladin-llm/src/fallback.rs` `after_failure` (`err.transience() == Permanent` returns the error)
**Issue:** The new mapping is correct for pacing, because pacing an account wall is futile. `UsageLimitExceeded` is `Transience::Permanent`, though, and `FallbackLlmAdapter::after_failure` short-circuits on `Permanent`. An exhausted OpenAI quota used to be a transient 429 and the chain hopped to the backup provider. It now aborts the whole chain, even though a different provider could serve the request. The fallback chain exists for this failure.

Anthropic's existing 400 usage-cap arm already behaves this way, so the semantics are consistent. Phase 43 widens the set of situations where it applies. The CHANGELOG/MIGRATION notes do not call this out.
**Fix:** Decide deliberately. Either let the chain hop on `UsageLimitExceeded` (account-level walls are provider-specific, so the next provider is unaffected), or document the change in MIGRATION. A hop-on-usage-limit rule belongs in `after_failure`, with a test for it.

### WR-07: Node-cache fence marker key collides with a legitimate entry key `<key>:fence`

**File:** `crates/paladin-storage/src/node_cache/redis.rs:160-162` (`fence_key`), Lua at `:76`
**Issue:** The marker for key `X` is `{prefix}:X:fence`, which is the entry key of `NodeCacheKey("X:fence")`. `NodeCacheKey` is documented as an opaque string. Entry keys are currently `graph:node:hash`, so a collision needs a user-composed key. `put_fenced` for `X` would overwrite the JSON entry of `X:fence` with a bare token string, so that entry's `get` fails to deserialize. Writing the entry `X:fence` overwrites the marker with JSON, which `tonumber` reads as `nil`, and the fence is disabled for `X`. The sibling cadence module avoided the same class of bug by escaping.
**Fix:** Put the marker in a namespace an entry key cannot reach, as the cadence module does with `%lock`/`%fence`.
```rust
fn fence_key(config: &RedisNodeCacheConfig, key: &NodeCacheKey) -> String {
    format!("{}:%fence:{}", config.key_prefix, key.as_str())
}
```
`invalidate(prefix)` then needs to scan both patterns, or accept that it removes the fence marker via a second `SCAN`.

## Info

### IN-01: `invalidate` now over-counts

**File:** `crates/paladin-storage/src/node_cache/redis.rs` (module docs "Fenced writes")
**Issue:** The port contract says `invalidate` returns the number of entries removed. With `put_fenced` it also counts the marker keys, so one entry reports 2. The module docs acknowledge this, but callers that use the count as an entry count are misled.
**Fix:** Count only keys that do not end in the marker suffix (or use the `%fence` namespace from WR-07 so the marker pattern is scanned separately and not counted).

### IN-02: Stale doc comment about the Redis adapter

**File:** `crates/paladin-storage/src/lib.rs:35`
**Issue:** "the Redis backend is added by a later Phase 43 plan behind its own feature". The adapter exists in this change set (`cadence/redis.rs`).
**Fix:** Reword to point at `cadence::redis` and the `redis-cadence` feature.

### IN-03: Documentation misdescribes the first back-off

**File:** `docs/src/getting-started/configuration.md:587`
**Issue:** It says a delay-less 429 gates "for a random time up to `base_backoff_ms`". The code gates exactly `base_backoff_ms` for the first 429, and `[base, min(max, base * 2^(n-1))]` for later ones (the jitter floor is `base`).
**Fix:** "the first delay-less 429 gates for `base_backoff_ms`; later consecutive 429s gate for a random time between `base_backoff_ms` and the doubled ceiling, up to `max_backoff_ms`."

### IN-04: Oversized `Retry-After` integer is dropped while a smaller one is clamped

**File:** `crates/paladin-llm/src/rate_limit_headers.rs:215-225` (`plain_decimal_seconds`)
**Issue:** An all-digit value that overflows `u64` (`99999999999999999999`) gives `None`, so the call falls back to base back-off, whereas `9999999999` is clamped to the 24 h ceiling. A hostile or buggy "very large" delay is therefore treated as "no delay", which paces less, not more. The test pins this.
**Fix:** Saturate to `CADENCE_DELAY_CEILING` on parse overflow of an all-digit value, or document the asymmetry.

### IN-05: A single provider header can close a key fleet-wide for 24 h

**File:** `crates/paladin-ports/src/output/cadence_port.rs:48` (`CADENCE_DELAY_CEILING`) and `crates/paladin-llm/src/cadence.rs` (`minimum_delay`)
**Issue:** An explicit `Retry-After` is passed through unreduced up to 24 h. Only the reset-header-derived delay is capped at `max_backoff`. With the Redis backend, a misbehaving gateway that returns `Retry-After: 86400` gates that provider and model for the whole fleet for a day. The `max_wait` check keeps workers from sleeping, but every call is refused.
**Fix:** Consider an operator-tunable cap on explicit delays (for example `max_explicit_delay_secs`) lower than 24 h.

### IN-06: `rediss://` and cluster are not supported, and failures are quiet

**File:** `crates/paladin-storage/src/cadence/redis.rs` (`manager()` and the two-key `try_lock` script)
**Issue:**
- The `redis` dependency enables `aio` and `tokio-comp` but no TLS feature. A `rediss://` URL in `url_env` fails to connect, `ResilientCadence` degrades with one warning, and the fleet silently never shares state. The docs recommend putting a password in the URL.
- `CADENCE_TRY_LOCK_LUA` touches two keys that hash to different slots, so it fails with `CROSSSLOT` on Redis Cluster. The same applies to the node-cache fenced script.

Both are consistent with the stated assumption A1 (a single primary), but neither is stated in the operator docs.
**Fix:** Add both to the "Fleet-wide pacing with Redis" section, or use `{...}` hash tags around a shared segment of the lock, fence and marker keys.

### IN-07: Env override for `enabled` silently ignores unparseable values

**File:** `src/config/treasurer.rs` (`apply_env_overrides`)
**Issue:** `APP_TREASURER_CADENCE_ENABLED=0` (or `off`, `False`) fails `parse::<bool>` and leaves pacing enabled. An operator trying to turn pacing off sees no error and no log. This is documented for `BASE_BACKOFF_MS`, but pacing is on by default, so this one has the larger surprise.
**Fix:** Log a warning (`treasurer.cadence: ignoring unparseable APP_TREASURER_CADENCE_ENABLED`) naming the variable and never its value. Also `build_cadence` runs `validate()` before the `enabled` check, so `enabled: false` with `backend: redis` and an unset `url_env` still stops boot. If that is unintended, validate only when enabled.

---

_Reviewed: 2026-10-09_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
