# Phase 43: Rate Pacing - Pattern Map

**Mapped:** 2026-10-07
**Files analyzed:** 22 new/modified
**Analogs found:** 21 / 22

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/paladin-ports/src/output/cadence_port.rs` (new) | port | request-response + state | `crates/paladin-ports/src/output/node_cache_port.rs` | role-match |
| `crates/paladin-ports/src/output/llm_port.rs` (mod: `RateLimitExceeded` struct variant, constructors, accessors, `transience` arm) | model/error | transform | itself (lines 341-342, 505-511) | exact |
| `crates/paladin-llm/src/cadence.rs` (new) | service (LlmPort decorator) | request-response | `crates/paladin-llm/src/pricing.rs` | exact |
| `crates/paladin-llm/src/rate_limit_headers.rs` (new) | utility | transform | `crates/paladin-llm/src/http_status.rs` | role-match |
| `crates/paladin-llm/src/http_status.rs` (mod: `map_http_status_with_hints`) | utility | transform | itself (lines 60-110) | exact |
| `crates/paladin-llm/src/fallback.rs` (mod: pace-first branch, `with_cadence`) | service | request-response | itself (`after_failure` 222-246, `record_hop` 202-216) | exact |
| `crates/paladin-llm/src/{openai/adapter,compat/engine,anthropic/adapter,deepseek,gemini}` (mod: stop 429 retry, snapshot headers) | adapter | request-response | their own `map_error` / retry loops | exact |
| `crates/paladin-llm/src/conformance.rs` (mod: new case, `CASE_COUNT` 9 to 10) | test | request-response | itself (macro lines 487-520) | exact |
| `crates/paladin-storage/src/cadence/mod.rs` (new, `ResilientCadence` composite) | adapter | request-response | `crates/paladin-storage/src/node_cache/mod.rs` | role-match (composite has no analog) |
| `crates/paladin-storage/src/cadence/in_memory.rs` (new) | adapter | CRUD (state) | `crates/paladin-storage/src/node_cache/in_memory.rs` | role-match |
| `crates/paladin-storage/src/cadence/redis.rs` (new) | adapter | request-response (Lua EVAL) | `crates/paladin-storage/src/run_queue/redis.rs` | exact |
| `crates/paladin-storage/src/cadence/contract_tests.rs` (new) | test | request-response | `crates/paladin-storage/src/node_cache/contract_tests.rs` | exact |
| `crates/paladin-storage/Cargo.toml` (mod: `redis-cadence`) | config | n/a | `redis-cache` feature, lines 29-33 | exact |
| `crates/paladin-battalion/src/engine/superstep.rs` (mod: `NodeCacheBinding` + lock loop) | service | event-driven | itself (173-300) | exact |
| `crates/paladin-battalion/src/engine/mod.rs` (mod: `WarEngine::with_cadence`) | builder | n/a | `WarEngine::with_node_cache` | exact |
| `crates/paladin-storage/src/node_cache/redis.rs` (mod: fenced put, if Open Question 2 resolves that way) | adapter | CRUD | itself | exact |
| `src/config/treasurer.rs` (mod: `CadenceConfig`, `TreasurerConfig.cadence`) | config | n/a | itself (`AllowanceConfig`, lines 488-648) + `src/config/run_queue.rs` | exact |
| `src/infrastructure/web/agent_host.rs` (mod, line 209) | composition | n/a | `with_pricing(llm, price_table)` | exact |
| `src/infrastructure/web/facade_provisioner.rs` (mod) | composition | n/a | its `treasurer: TreasurerConfig` field | exact |
| `src/infrastructure/web/run_api_wiring.rs` (mod, near line 789) | composition | n/a | `Treasurer::with_pricing` wiring | role-match |
| `.github/copilot-instructions.md`, `MIGRATION.md`, `CHANGELOG`, docs | docs | n/a | existing rows | n/a |
| Lua scripts `record_429`, `gate`, `try_lock`, `unlock` (inside redis.rs) | script | atomic RMW | `RUN_QUEUE_*_LUA` consts | exact |

## Pattern Assignments

### `crates/paladin-llm/src/cadence.rs` (decorator, request-response)

**Analog:** `crates/paladin-llm/src/pricing.rs`

**Imports and log target** (lines 28-47):
```rust
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};
pub const PRICING_LOG_TARGET: &str = "paladin::pricing";   // use CADENCE_LOG_TARGET = "paladin::cadence"
```

**Bounded warn-once set, poison-recovered** (lines 58-86), reuse for the "one warning per outage" latch:
```rust
let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
```

**Struct, manual Debug, `new`** (lines 100-124):
```rust
#[derive(Clone)]
pub struct PricingLlmAdapter { inner: Arc<dyn LlmPort>, table: Arc<PriceTable> }
impl fmt::Debug for PricingLlmAdapter { /* debug_struct with inner_provider, no secrets */ }
```

**`with_*` constructor returning `inner` unchanged when inert** (lines 144-149). Cadence version returns `inner` when `enabled: false` or when `inner.get_provider_name() == "fallback"` (research Pattern 1):
```rust
pub fn with_pricing(inner: Arc<dyn LlmPort>, table: &Arc<PriceTable>) -> Arc<dyn LlmPort> {
    if table.is_empty() { return inner; }
    Arc::new(PricingLlmAdapter::new(inner, Arc::clone(table)))
}
```

**Trait impl: generate/stream wrap, identity delegation** (lines 152-228):
```rust
async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
    let mut response = self.inner.generate(request).await?;
    ...
}
async fn generate_stream(&self, request: LlmRequest)
    -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> { ... }
async fn validate_model(&self, model: &str) -> Result<bool, LlmError> { self.inner.validate_model(model).await }
async fn get_available_models(&self) -> Result<Vec<String>, LlmError> { self.inner.get_available_models().await }
fn get_provider_name(&self) -> &'static str { self.inner.get_provider_name() }
fn get_capabilities(&self) -> ProviderCapabilities { self.inner.get_capabilities() }
```
Cadence differences: gate-wait before delegating (key = `(inner.get_provider_name(), request.model)`), record 429 or success after, return the error UNCHANGED, never retry (D-01). For streams, record on the call `Err` and on a first-item `Err`; record success on stream completion or first Ok chunk (planner's choice).

**Test module gate** (line 232): `#[cfg(all(test, feature = "mock"))]`, with a `request(model)` helper and `MockLlmAdapter::new().with_response(..)`. Use `#[tokio::test(start_paused = true)]` for time (idiom from `crates/paladin-battalion/src/engine/retry.rs:265`).

---

### `crates/paladin-llm/src/fallback.rs` (D-03 pace-first branch)

**Analog:** itself. The single edit point is `after_failure` (lines 222-246) and `record_hop` (202-216).

```rust
async fn after_failure(&self, index: usize, from: &'static str, err: LlmError,
    attempts: &mut Vec<(String, String)>) -> Result<(), LlmError> {
    attempts.push((from.to_string(), err.to_string()));
    if err.transience() == Transience::Permanent { return Err(err); }
    match self.chain.get(index + 1) {
        Some(next) => { self.record_hop(from, next.get_provider_name(), &err).await; Ok(()) }
        None => Err(LlmError::AllProvidersFailed { attempts: std::mem::take(attempts), last: Box::new(err) }),
    }
}
```
Add a branch before the hop: `RateLimitExceeded { .. }` AND cumulative paced wait for this hop below `fallback_pace_budget` leads to retrying the same index (caller loop must be able to re-enter the same hop). Emit a debug/trace line per pace-retry. Rewrite module docs lines 1-60 ("never retries a hop") and test `hops_eventually` (line 391). Trace emitter fallback chain: `self.trace_emitter.clone().or_else(current_trace_emitter)` (line 205), reuse for gated-call and degraded-mode events. Test helpers to reuse: `provider`, `failing`, `chain`, `transient` (lines 440-470).

---

### `crates/paladin-storage/src/cadence/redis.rs` (Redis adapter, Lua EVAL)

**Analog:** `crates/paladin-storage/src/run_queue/redis.rs`

**Imports** (lines 96-98): `use redis::{AsyncCommands, Client, Script, aio::ConnectionManager};` and `tokio::sync::RwLock`. Do NOT copy the `Arc<RwLock<ConnectionManager>>` + `.write()` per call (lines 366, 396). Research anti-pattern: clone `ConnectionManager` per call, and set explicit connect/response timeouts through `ConnectionManagerConfig` (redis defaults to none).

**Lua const idiom, server clock** (lines 120-145):
```rust
pub const RUN_QUEUE_CLAIM_LUA: &str = r#"
local time = redis.call('TIME')
local now_us = tonumber(time[1]) * 1000000 + tonumber(time[2])
...
"#;
```
Document `KEYS[]`/`ARGV[]` in the const rustdoc as the analog does. One script per op (`record_429`, `gate`, `try_lock`, `unlock`). Return relative waits in microseconds.

**Construction** (lines 376-404):
```rust
let client = Client::open(config.connection_url.as_str()).map_err(|_| QueueError::Backend {
    message: format!("failed to parse ... url: {}", redact_connection_url(&config.connection_url)),
})?;
let conn = ConnectionManager::new(client.clone()).await.map_err(|e| ...)?;
... claim_script: Script::new(RUN_QUEUE_CLAIM_LUA),
```
Always route the URL through `redact_connection_url` (security.instructions.md).

**Invocation and error mapping** (lines 440-455):
```rust
self.claim_script.key(ready_key(&self.prefix)).arg(duration_to_micros(lease))
    .invoke_async(&mut *conn).await
    .map_err(|e| QueueError::Backend { message: format!("redis run queue claim failed: {e}") })?;
```
`Script::invoke_async` auto-reloads on NOSCRIPT (module docs lines 66-71). Server `TIME` for non-script reads: lines 410-420. Key namespace: `paladin:cadence:<provider>:<model>`, with TTLs on every key.

Fencing counter: `INCR` on a per-key counter inside the `try_lock` script (D-14); `unlock` is a compare-and-delete script.

---

### `crates/paladin-storage/src/cadence/{mod.rs,in_memory.rs,contract_tests.rs}`

**Analog:** `crates/paladin-storage/src/node_cache/{mod.rs,in_memory.rs,contract_tests.rs}`

**mod.rs layout** (node_cache/mod.rs lines 1-33): always-available `in_memory`, plain (non-`#[cfg(test)]`) `contract_tests`, feature-gated redis:
```rust
pub mod in_memory;
pub mod contract_tests;
#[cfg(feature = "redis-cache")]   // becomes redis-cadence
pub mod redis;
pub use in_memory::InMemoryNodeCache;
#[cfg(feature = "redis-cache")]
pub use redis::{RedisNodeCache, RedisNodeCacheConfig};
```
**contract_tests.rs** (lines 1-40): one named `pub async fn` per contract clause taking `&dyn Port`, invoked unchanged by each backend's `#[tokio::test]`s. Use `tokio::time::Instant` and `start_paused` rather than wall-clock sleeps (the node-cache file uses injected zero TTL; for cadence, use paused clock).

`ResilientCadence` (composite, outage latch) has no analog; build from research Pattern 5, test with an injected always-failing `Arc<dyn CadencePort>` (no Redis needed).

**Cargo feature** (`crates/paladin-storage/Cargo.toml` lines 29-33, 63-67): add `redis-cadence = ["dep:redis"]` beside `redis-queue`/`redis-cache`; do not change the `redis` dep line.

---

### `crates/paladin-ports/src/output/cadence_port.rs` (port)

**Analog:** `crates/paladin-ports/src/output/node_cache_port.rs` (lines 195-224)

```rust
#[async_trait]
pub trait NodeCachePort: Send + Sync {
    /// Look up ...
    async fn get(&self, key: &NodeCacheKey) -> Result<Option<CachedDelta>, NodeCacheError>;
    async fn put(&self, key: &NodeCacheKey, delta: &StateDelta, ttl: Duration) -> Result<(), NodeCacheError>;
    async fn invalidate(&self, prefix: &str) -> Result<u64, NodeCacheError>;
}
```
Copy: `Send + Sync`, `#[async_trait]`, `Result<_, <Domain>Error>` with `thiserror` enum, rustdoc stating the best-effort contract. Methods per research: `gate`, `record_rate_limited`, `record_success`, `try_lock(key, ttl) -> Result<Option<FencingToken>, _>`, `unlock(key, token)`. Keep the pure `CadencePolicy` math (jitter fraction injected, no `rand`) in this file. Per CONTEXT D-00d, `NodeCachePort` stays unchanged; Open Question 2 (fencing token reaching `put`) must be resolved with an additive default method.

---

### `crates/paladin-ports/src/output/llm_port.rs` + `http_status.rs` + `rate_limit_headers.rs` (PACE-01)

**Analog:** `llm_port.rs` lines 341-342 and 505-511; `http_status.rs` lines 60-110.

Current:
```rust
#[error("Rate limit exceeded")]
RateLimitExceeded,
...
LlmError::RateLimitExceeded => Transience::Transient,
```
and `map_http_status`: `429 => LlmError::RateLimitExceeded,` after `redact_credentials` then `bounded_excerpt(&redacted, RESPONSE_EXCERPT_CHAR_BUDGET)`. Keep `#[error("Rate limit exceeded")]` byte-identical. Keep `map_http_status`'s public signature (it has a doctest at lines 78-95 that matches `LlmError::RateLimitExceeded`; update to `{ .. }`); add `map_http_status_with_hints`. Keep rustdoc examples in `llm_port.rs` (lines 265, 335) compiling: convert to `RateLimitExceeded { .. }`. Closure-based header lookup so the un-gated module has no `reqwest` types. Snapshot headers before `response.text()`.

`RateLimitExceeded` is matched in about 26 files (86 occurrences); do the variant change as its own first, compiler-driven plan. Exclude `VisionError::RateLimitExceeded(String)`, `NotificationPortError`, `ContentDeliveryError`, and `LlmProviderError::RateLimitExceeded` (`crates/paladin-llm/src/error.rs`, keep, update its `From`).

---

### `crates/paladin-llm/src/conformance.rs` (new case)

**Analog:** the `llm_conformance_suite!` macro (lines 485-520): add `rate_limit_is_surfaced_once` to the `@cases` list and a `cases::` function; update the guard test asserting `CASE_COUNT, 9` (line ~743) to 10. Research Pitfall 7: Anthropic is not in the macro (needs a bespoke test) and DeepSeek/Gemini have their own 429-retry loops that D-02 must also cover.

---

### `crates/paladin-battalion/src/engine/superstep.rs` (stampede lock)

**Analog:** itself, `NodeCacheBinding` (173-178), `lookup_node_cache` (235-274), `store_node_cache` (277+), construction at ~2689.

```rust
#[derive(Clone)]
struct NodeCacheBinding {
    policy: CachePolicy,
    cache: Arc<dyn NodeCachePort>,
    graph_fingerprint: GraphFingerprint,
}
```
Add `cadence: Option<Arc<dyn CadencePort>>`. Error-handling idiom to copy: a backend `Err` is logged with `warn!` and treated as a miss (`warn!("node cache: get failed for {node_id}: {err} -- treated as a miss (D-29)")`). The lock path must follow the same rule: `try_lock` error means execute uncached, never fail the node. Lock loop (research Pattern 6): re-read `cache.get` and re-try `try_lock` each poll, `select!` with the run's cancellation token (see `wait_backoff` in `engine/retry.rs` lines 68-76), unlock on every exit path. Jitter idiom is `rand::thread_rng().gen_range(..)` as in `backoff_delay` (lines 51-66). `WarEngine::with_cadence` mirrors `with_node_cache`.

---

### `src/config/treasurer.rs` (`CadenceConfig`)

**Analogs:** `TreasurerConfig` / `AllowanceConfig` in the same file; `src/config/run_queue.rs`.

TreasurerConfig field + Default (lines 488-509):
```rust
pub struct TreasurerConfig {
    pub currency: String,
    pub pricing: BTreeMap<String, PriceRowConfig>,
    pub allowance: AllowanceConfig,      // add: pub cadence: CadenceConfig,
}
impl Default for TreasurerConfig { ... allowance: AllowanceConfig::default(), }
```
`EnvOverridable` (lines 631-648): scalars only via `read_env::<T>("APP_TREASURER_..._...")`; add `APP_TREASURER_CADENCE_*` for scalar fields (not `backend`, unless following run_queue's string form).

Backend enum + validate from `run_queue.rs`:
```rust
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum RunQueueBackend { InMemory, Redis { url_env: String, key_prefix: String } }
...
RunQueueBackend::Redis { url_env, .. } => {
    if url_env.trim().is_empty() { return Err("... requires a non-empty url_env name".to_string()); }
    if std::env::var(url_env).is_err() { return Err(format!("... names env var '{url_env}', which is not set")); }
```
Cadence: `in_process | redis { url_env }`, with `#[serde(deny_unknown_fields)]` on every struct (module docs lines 33-40). Config types carry only the env-var name, never the URL; `validate()` returns `Result<(), String>` naming the offending key. Tests: `#[serial]` plus `unsafe { env::set_var/remove_var }` as in run_queue tests. Add the `enabled: false` inert case and the "omitted section yields enabled in-process defaults" case (D-08).

### Composition points

`agent_host.rs:209`: `let llm = with_pricing(llm, price_table);` Cadence composes at the same sites; nesting must keep pricing seeing the served response while cadence wraps each hop (Claude's Discretion; document in `cadence.rs` module docs the way `pricing.rs` lines 1-25 do). `facade_provisioner.rs` already holds `treasurer: TreasurerConfig` (line 43/60) and `run_api_wiring.rs:789` is the worker path. Research finding: chains built via `resolve_chain` / `ModelFallbackMiddleware` replace the port through `llm_override`, so per-hop wrapping must happen at the factory or chain builder.

## Shared Patterns

### Redaction before truncation
**Source:** `crates/paladin-llm/src/http_status.rs` lines 95-100 and `crate::redaction`. **Apply to:** every adapter 429 mapping, header-hint carrying. Never embed raw header values in errors or logs.

### Lock poisoning recovery
**Source:** `pricing.rs` line 74 (`unwrap_or_else(PoisonError::into_inner)`). **Apply to:** all `std::sync::Mutex` use in `InMemoryCadence`, `ResilientCadence`, the decorator.

### Best-effort cache and degradation
**Source:** `superstep.rs` `lookup_node_cache` error arm. **Apply to:** lock acquisition, `record_*`: failures are logged (one warning per outage) and never fail the call.

### Log target and trace events
**Source:** `PRICING_LOG_TARGET` (`pricing.rs:47`), `TraceEvent::FallbackHop` emission (`fallback.rs:202-216`, `node_id: None`, emitter from explicit field or `current_trace_emitter`). **Apply to:** `paladin::cadence` warnings, gated-call events.

### Paused-clock tests
**Source:** `engine/retry.rs:265-280`. **Apply to:** all cadence decorator, in-memory adapter, fallback pace-first tests (`tokio::time::Instant`, `start_paused = true`).

### Mock-based LlmPort tests
**Source:** `MockLlmAdapter` (`mock` feature) as used in `pricing.rs` tests and `fallback.rs` helpers. **Apply to:** decorator and fallback tests, scripting `[429(retry_after), Text]`.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `ResilientCadence` composite (storage `cadence/mod.rs`) | adapter | request-response | No primary+fallback+outage-latch adapter exists; use research Pattern 5 |
| `CadencePolicy` pure back-off math | utility | transform | `backoff_delay` (`engine/retry.rs:51-66`) is battalion-bound with additive jitter and unreachable from `paladin-llm`/`paladin-storage`; lift arithmetic, do not call it |
| OpenAI Go-duration parser (`6m0s`) | utility | transform | No parser exists; hand-write with table tests (research) |

## Metadata

**Analog search scope:** `crates/paladin-llm/src`, `crates/paladin-ports/src/output`, `crates/paladin-storage/src/{run_queue,node_cache}`, `crates/paladin-battalion/src/engine`, `src/config`, `src/infrastructure/web`
**Files scanned:** about 14 read (targeted ranges); research file read to line 490 plus pitfalls 5-7 preview
**Pattern extraction date:** 2026-10-07
**Caveat:** research lines 491-834 (Pitfalls 8-16, Open Questions) were only skimmed; planner should read Open Questions 1-4 directly (D-02 scope, D-14 fencing seam vs D-00d, per-hop composition site, TraceEvent vs log).
