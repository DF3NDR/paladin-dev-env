# PRD 04 — Node-Level Fault Tolerance: Aegis (Retry, Timeout, Error Handlers), Model Fallback, Node Caching (Epic `FT`)

**Depends on:** PRD 01 for engine integration (§2.1–2.4). §2.5 (model fallback) and the error-taxonomy work (§2.0) are standalone and may ship first.
**Primary crates:** `paladin-core` (policies, error taxonomy), `paladin-battalion` (engine integration), `paladin-llm` (fallback adapter), `paladin-memory` (cache backend).

---

## 1. Problem Statement

Fault tolerance in Paladin today exists at two granularities that don't match where failures happen: a global circuit breaker around LLM calls, and Battalion-level strategies (`FailFast`, `ContinueOnError`, `RetryThenContinue`) plus a Battalion-level timeout. Missing:

- **Per-node retry** with backoff/jitter and, critically, a **retryable-vs-permanent error taxonomy** — today a config error and a 503 are retried (or not) identically.
- **Per-node timeouts**, including an *idle* timeout that distinguishes a stalled stream from a slow-but-progressing one.
- **Typed error handlers**: node failures collapse into `BattalionError::PaladinError(String)`, destroying the information a compensation step needs. There is no way to say "if the booking node fails permanently, run the cancel-reservation node."
- **Model fallback**: when a provider fails or its breaker opens, the request dies instead of failing over to a second model.
- **Node result caching** for expensive deterministic steps.

The policy bundle attached to a node is the **Aegis**.

## 2. Functional Requirements

### 2.0 Error taxonomy (prerequisite, standalone)

- **FT-FR-01 (Transience classification).** `PaladinError` and `LlmError` gain a method `fn transience(&self) -> Transience` with `pub enum Transience { Transient, Permanent, Unknown }`. Classification rules (MUST, with unit tests per variant): connection/IO failures, timeouts, HTTP 429/5xx from providers → `Transient`; configuration/validation errors, HTTP 4xx except 408/429, schema violations, `DispatchConflict` → `Permanent`; anything unclassifiable → `Unknown`. Provider adapters (`paladin-llm`) MUST map status codes into variants that classify correctly (add variants if the current ones erase status codes — do not parse strings to classify).
- **FT-FR-02 (Structured NodeError).** New `NodeError` (core, thiserror) carried through engine execution: `{ node_id, attempt, source: NodeErrorSource, transience }` where `NodeErrorSource ∈ { Paladin(PaladinError), Llm(LlmError), Function(anyhow-like boxed error with message), Timeout(TimeoutKind), Cancelled }`. `BattalionError` gains a `Node(NodeError)` variant; new engine paths MUST use it — never stringify (X-06).
- **FT-FR-02a (Public-type compatibility, X-10).** FT-FR-01 and FT-FR-02 add variants to three *pre-existing* public enums (`PaladinError`, `LlmError`, `BattalionError`). Before adding any variant: (1) mark each enum `#[non_exhaustive]` unless a written justification for an exception is recorded; (2) add a `transience()` method rather than a new required trait method; (3) record every touched enum in `MIGRATION.md` §9.2 with the mitigation chosen; (4) confirm `cargo semver-checks` passes or that the entry is on the explicit allowlist. Adapters in `paladin-llm` that previously erased HTTP status codes into string messages must gain status-carrying variants — this is the one place a "stringly" variant may be *replaced*, and the replacement is itself a register entry.

### 2.1 Aegis — policy bundle

```rust
pub struct Aegis {
    pub retry: Option<RetryPolicy>,
    pub timeout: Option<TimeoutPolicy>,
    pub on_error: Option<ErrorHandlerSpec>,
    pub cache: Option<CachePolicy>,
}

pub struct RetryPolicy {
    pub max_attempts: u32,                 // total attempts incl. first; MUST be ≥ 1
    pub initial_interval: Duration,        // default 500 ms
    pub backoff_factor: f64,               // default 2.0
    pub max_interval: Duration,            // default 60 s
    pub jitter: bool,                      // default true; uniform in [0, interval)
    pub retry_on: RetryPredicate,          // default: Transience::Transient only
}
pub enum RetryPredicate {
    TransientOnly,
    TransientAndUnknown,
    Custom(String),                        // engine-registered predicate, same registry pattern as CF-FR-01
}

pub struct TimeoutPolicy {
    pub run_timeout: Option<Duration>,     // hard wall-clock cap per ATTEMPT
    pub idle_timeout: Option<Duration>,    // max time without observable progress
}

pub enum ErrorHandlerSpec {
    /// Route to a recovery node with the NodeError injected into a state field.
    Route { to: NodeId, error_field: FieldName },
    /// Absorb: record the failure, write a fallback delta, continue via edges.
    Absorb { fallback_delta: StateDelta },
    /// Registered handler: full power — receives NodeError + Battlefield, returns a Directive.
    Custom(String),
}

pub struct CachePolicy {
    pub ttl: Duration,
    pub key: CacheKeySpec,                 // Default = hash(node_id, resolved input/muster payload, paladin config fingerprint)
}
```

Attachment points: `NodeSpec` gains `aegis: Option<Aegis>`; `WarGraph` gains `default_aegis: Option<Aegis>` applied to every node lacking one (per-node wins wholesale, no field-level merging — document this).

### 2.2 Retry semantics

- **FT-FR-03 (Attempt isolation).** Each attempt starts from the same pre-superstep Battlefield snapshot; deltas from failed attempts are discarded entirely. Only the successful attempt's delta merges. `NodeExecutionRecord.attempt` records the succeeding attempt number; failed attempts append to a `Vec<AttemptRecord>` on the record (timing + error summary per attempt) for observability.
- **FT-FR-04 (Backoff correctness).** Delay before attempt *n* (n ≥ 2) = `min(initial_interval × backoff_factor^(n−2), max_interval)`, plus jitter if enabled. Deterministic-clock unit tests (tokio `time::pause`) MUST assert the exact sequence for a 5-attempt policy with and without jitter (jitter test asserts bounds, not exact values).
- **FT-FR-05 (Predicate gating).** An error whose transience fails the predicate MUST NOT be retried — it flows to `on_error` (or fails the run) immediately, even with attempts remaining. Explicit tests: Permanent error under `TransientOnly` → 1 attempt total; Transient → retried.
- **FT-FR-06 (Retry × Muster).** In a Muster, retries are per-task; one task's retries never delay or re-run sibling tasks. E2E-3 (overview §6) is the acceptance test.
- **FT-FR-07 (Retry × Waypoints).** Retries happen *within* a superstep; no Waypoint is written between attempts. If the process dies mid-retry, resume re-executes the node from attempt 1 (documented; test via kill-during-backoff simulation).

### 2.3 Timeout semantics

- **FT-FR-08 (run_timeout).** Cancels the attempt at the wall-clock cap → `NodeError { source: Timeout(Run), transience: Transient }` (feeds the retry policy). The attempt's partial work is discarded (FT-FR-03).
- **FT-FR-09 (idle_timeout & progress).** "Observable progress" = any stream chunk received from `execute_stream`, any TraceSink node event emitted by the node, or an explicit `ctx.heartbeat()` call (add `heartbeat()` to `NodeContext`). The idle timer resets on each progress event. Firing → `Timeout(Idle)`, transient. Test: a mock stream emitting a chunk every 100 ms under `idle_timeout=250 ms` survives; a stream that stalls 300 ms is killed; both under `run_timeout` large enough not to interfere.
- **FT-FR-10 (Interaction with Battalion/global timeouts).** Per-attempt timeouts nest inside the run-level `EngineLimits.run_timeout` and any legacy Battalion timeout; the tightest applicable bound fires; the error names which bound fired.

### 2.4 Error handlers / compensation

- **FT-FR-11 (Route handler).** After retries exhaust (or a non-retryable error), `Route { to, error_field }` serializes the `NodeError` (structured JSON: node, attempts, transience, message chain) into `error_field` (schema MUST declare it; validation error at graph build if missing) and places `to` in the next Vanguard, replacing the failed node's static successors. The run is NOT failed. This enables Saga-style compensation chains (recovery nodes are ordinary nodes and may themselves have Aegis policies).
- **FT-FR-12 (Absorb handler).** Records the failure in the NodeExecutionRecord (`outcome: Failed` but run continues), merges `fallback_delta`, and proceeds via the node's normal static edges as if it succeeded.
- **FT-FR-13 (Custom handler).** Registered as `Arc<dyn ErrorHandler>`: `fn handle(&self, err: &NodeError, state: &Battlefield) -> Result<Directive, NodeError>` — may Goto, End, Parley (compose with Doc 03: "on payment failure, parley a human"), or re-fail. Unregistered name → graph validation error (same fail-closed rule as CF-FR-02).
- **FT-FR-14 (No handler).** Absent `on_error`, exhausted failure fails the run: `Failed` Waypoint with the structured `NodeError` (never a bare string), `RunOutcome::Failed`.
- **FT-FR-15 (Handler loops).** Handler-routed nodes count against `max_node_visits`; a compensation cycle (A fails → route B → B routes A) terminates via ENG-FR-03 rather than spinning forever. Test required.

### 2.5 Model fallback (standalone)

- **FT-FR-16 (FallbackLlmAdapter).** In `paladin-llm`: `FallbackLlmAdapter { chain: Vec<Arc<dyn LlmPort>> }` implementing `LlmPort`. On a `Transient`-or-`Unknown` error (or an open circuit breaker) from element *i*, try *i+1*; `Permanent` errors do NOT fall through (a bad prompt shouldn't burn every provider). Exhausted chain returns the LAST error wrapped in `LlmError::AllProvidersFailed { attempts: Vec<(provider_name, error_summary)> }`. Streaming: if a provider fails BEFORE the first chunk, fall through; after first chunk, propagate the mid-stream error (no silent provider switch mid-response) — both cases tested.
- **FT-FR-17 (Observability).** Each fallback hop emits a TraceSink event and increments a metric/log with provider names. `PaladinResult` metadata records which provider ultimately served. `PaladinResult` is a pre-existing public struct constructed via `..Default::default()` throughout the codebase; the new field MUST be added such that `Default` still works and the type is (or becomes) `#[non_exhaustive]` — register it in `MIGRATION.md` §9.2 (X-10.3).

### 2.6 Node caching

- **FT-FR-18 (Cache lookup/store).** With `CachePolicy`, the engine computes the cache key BEFORE attempt 1; on hit (unexpired), the stored `StateDelta` is merged, the node records `outcome: Succeeded` with `cache_hit: true` (new bool on the record), and no execution occurs. On miss, execute normally and store the successful delta with the TTL. Failures are never cached.
- **FT-FR-19 (CachePort).** New port `output::node_cache_port::NodeCachePort { get(key) -> Option<CachedDelta>, put(key, delta, ttl), invalidate(prefix) }` with InMemory (dev/test, with TTL eviction) and Redis (`redis-queue`-style feature) adapters, sharing a contract test suite.
- **FT-FR-20 (Correctness guardrails).** Default keying MUST include the Paladin config fingerprint (model, system prompt hash) so a prompt change invalidates naturally. Caching a node whose `output_field` uses `Append` dispatch is allowed but the doc MUST warn about replay-duplication on forks; a `cache: Deny` marker on Append-dispatch fields is available at schema level.

## 3. Acceptance Criteria

1. E2E-3 passes end-to-end (with Doc 02).
2. Backoff sequence tests exact under paused clock; jitter bounded.
3. Permanent-vs-transient gating verified for every `LlmError`/`PaladinError` variant (table-driven test).
4. Idle-vs-run timeout distinguishing test (FT-FR-09) green.
5. Compensation chain: book → (permanent fail) → route to cancel → cancel succeeds → run Completes with error recorded in state; and the loop-bound test (FT-FR-15).
6. Fallback adapter: 3-provider chain, first two transient-fail, third serves; permanent short-circuit; mid-stream propagation; `AllProvidersFailed` shape.
7. Cache: hit skips execution (call-count mock), TTL expiry re-executes, config-change invalidates, contract suite green on both backends.
8. Coverage per X-02; a multi-thread stress combining Muster + per-task retry with exact counts.
9. **Versioning gate (X-10/X-11):** any pre-existing public type touched by this epic is recorded in `MIGRATION.md` §9.2 with its mitigation; `cargo semver-checks` and the MSRV job pass; new dependencies listed in §9.3; new migrations in §9.4; new config/env in §9.5.

## 4. Test Plan (TDD ordering)

1. Transience classification table tests (FT-FR-01) — pure, fast, first.
2. RetryPolicy math under paused clock; predicate gating.
3. Timeout units (run, idle, heartbeat, nesting).
4. Engine integration: attempt isolation, records, retry×waypoint, retry×muster.
5. Error handler tests (Route/Absorb/Custom/None/loop-bound), incl. Parley-from-handler composition.
6. FallbackLlmAdapter units + streaming cases.
7. CachePort contract + engine cache integration.

## 5. Out of Scope

Rate limiting (exists at web layer; provider-level rate-limit *pacing* is future work); distributed locks for cache stampede (document as backend concern); changing legacy Battalion `ErrorStrategy` semantics (untouched, X-03).
