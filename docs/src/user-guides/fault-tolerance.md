# Aegis: Retry, Timeout, Error Handlers, Model Fallback and Node Caching

Per-node fault tolerance for the `WarEngine` — a typed error taxonomy, exact-backoff retry,
wall-clock and progress-aware timeouts, saga-style compensation handlers, an ordered model
fallback chain, and result caching — built as one opt-in policy attached beside a node, over the
superstep engine the [Control Flow](control-flow.md) and [Parley & Chronicle](parley-and-chronicle.md)
guides introduce.

> Every Rust sample on this page is compiled code pulled from the `paladin-doc-examples` crate via
> mdBook `{{#include}}`, so a sample cannot drift from the landed API.

---

## Table of Contents

1. [What an Aegis Is](#what-an-aegis-is)
2. [The Error Taxonomy: Why `transience()` Exists](#the-error-taxonomy-why-transience-exists)
3. [Retry](#retry)
4. [Timeouts: Wall Clock versus Idle](#timeouts-wall-clock-versus-idle)
5. [Error Handlers and Compensation](#error-handlers-and-compensation)
6. [Model Fallback](#model-fallback)
7. [Node Caching](#node-caching)
8. [The Node-Kind Support Matrix](#the-node-kind-support-matrix)
9. [Limitations You Should Know About](#limitations-you-should-know-about)
10. [Security Notes](#security-notes)

---

## What an Aegis Is

An **Aegis** is the whole fault-tolerance policy for one node:

```rust,ignore
pub struct Aegis {
    pub retry: Option<RetryPolicy>,
    pub timeout: Option<TimeoutPolicy>,
    pub on_error: Option<ErrorHandlerSpec>,
    pub cache: Option<CachePolicy>,
}
```

Every field is independently optional, so a node can carry only a timeout, only a cache, or all
four. An Aegis is **not** a field on `NodeSpec` — it attaches as a sidecar on the `WarGraph`,
keyed by `NodeId`, through two builder methods:

- `WarGraph::set_aegis(node_id, aegis)` — this node's own policy.
- `WarGraph::with_default_aegis(aegis)` — the graph-wide fallback for every node with no entry
  of its own.

```rust,ignore
{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:attach}}
```

**A node's own entry wins wholesale.** `aegis_for(node)` returns the node's own `set_aegis` entry
if there is one, else the default, else nothing — it never merges the two field by field. In the
sample above, `writer` sets `retry` but not `timeout`, so `writer` has *no* per-attempt timeout
even though the default declares one. If you want the default's timeout plus a different retry
policy, spell the timeout out on the node's own Aegis.

A node with no Aegis at all — the state every v0.9 graph is in — behaves byte-identically to
before this feature existed: one attempt, no bound, no handler, no cache. Nothing here is on by
default.

Validation is fail-closed and happens in `WarGraph::validate`, before any node runs: an Aegis on
an undeclared node (`EngineError::AegisOnUndeclaredNode`), a `RetryPolicy` with
`max_attempts: 0` (`RetryPolicyInvalid` — never "unlimited", never a silent single attempt), a
`TimeoutPolicy` with a zero duration (`TimeoutPolicyInvalid`), a `Custom` name nobody registered
(`UnregisteredRetryPredicate` / `UnregisteredErrorHandler`), or a policy on a node kind that does
not support it (`AegisUnsupportedForNodeKind`, see [the matrix](#the-node-kind-support-matrix))
each list **every** offender in one error.

## The Error Taxonomy: Why `transience()` Exists

A configuration error and a `503 Service Unavailable` must not be retried identically: resending a
request the provider rejected as malformed burns every attempt for nothing, while giving up on an
overloaded upstream after one try throws away a recovery that would have cost half a second. The
engine therefore never decides "retry or not" from an error's *message*. Every error is classified
into one of three values by reading its **typed** fields:

| `Transience` | Meaning | Examples |
|---|---|---|
| `Transient` | Retrying the same operation has a reasonable chance of succeeding | a network blip, a request timeout, a rate limit (`429`), `408`, any `5xx`, an open circuit breaker |
| `Permanent` | Retrying would fail identically | a rejected credential, an invalid prompt, a model that does not exist, a configuration error, any other `4xx` |
| `Unknown` | The error carries no typed field that distinguishes the two | a bare `ExecutionError(String)` or `ProcessingError(String)` |

`PaladinError::transience()` and `LlmError::transience()` are table-driven, one arm per variant.
Provider adapters no longer collapse an HTTP status into a string: every non-2xx response without a
dedicated variant becomes `LlmError::ProviderError { provider, status, message }`, and `status`
is what the table reads.

```rust,ignore
{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:transience}}
```

When a node fails under an Aegis, the failure travels as one structured value rather than a
string:

```rust,ignore
pub struct NodeError {
    pub node_id: NodeId,
    pub attempt: u32,          // 1-indexed; the attempt that produced it
    pub transience: Transience,
    pub source: NodeErrorSource, // Paladin { kind, message } | Llm { status, provider, .. }
                                 // | Function { message } | Timeout(TimeoutKind) | Cancelled
}
```

The same `NodeError` appears on the failed Waypoint (`WaypointStatus::Failed.node_error`), on
`RunOutcome::node_error()`, inside `EngineError::NodeFailed`, and as `BattalionError::Node` at the
crate boundary — so a caller, an operator reading Chronicle output and a compensation handler all
see the identical value. The human-readable display line on the Waypoint is unchanged from v0.9.

## Retry

A `RetryPolicy` retries a failed attempt **inside the same superstep**: no Waypoint is written
between attempts, sibling nodes in the superstep are unaffected, and every attempt reads the same
immutable Battlefield snapshot — a failed attempt's delta never reaches the merged state.

```rust,ignore
{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:retry}}
```

**The backoff formula.** The delay before attempt `n` (for `n >= 2`) is

```text
min(initial_interval × backoff_factor ^ (n − 2), max_interval)  +  jitter
```

where `jitter`, when enabled, adds a uniformly random duration in `[0, delay)`. With the defaults
and jitter off, the waits before attempts 2, 3, 4 and 5 are exactly 500 ms, 1 s, 2 s and 4 s.
`max_attempts` counts attempts *including the first*: `3` means the node runs at most three times.

**The predicate.** `retry_on` gates every retry on the error's classification:

| `RetryPredicate` | Retries |
|---|---|
| `TransientOnly` (default) | `Transient` only — a `Permanent` **or** `Unknown` error takes exactly one attempt |
| `TransientAndUnknown` | `Transient` and `Unknown` |
| `Custom(name)` | Whatever the `RetryPredicateEvaluator` registered under `name` answers; an unregistered name fails validation |

A `Custom` predicate sees the structured `NodeError` and the attempt about to run:

```rust,ignore
{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:custom_predicate}}
```

**What the record shows.** `NodeExecutionRecord.attempt` is the attempt that finally succeeded (or
the one that exhausted the budget), and `NodeExecutionRecord.attempts` lists every *failed*
attempt in order as an `AttemptRecord { attempt, started_at, duration_ms, error }`. The trace
sink receives one `NodeStarted`/`NodeFinished` pair **per attempt**, each carrying `attempt`, so
an observer can tell three attempts from one.

**Interceptors run once per attempt.** A `NodeInterceptor`'s `before` hook runs before every
attempt (and its `after` hook only after the attempt that succeeded), because the retry loop wraps
*outside* the interceptor chain. An interceptor's own `Skip`/`Fail` decision is never retried as if
it were the node's fault.

**Inside a Muster.** Retry is per *task*: a failing mustered task retries in its own future with
its own attempt counter while its siblings finish and are recorded normally. A progress Waypoint
inside a Muster superstep lists only completed tasks.

**A Parley is not a failure.** A node that returns `NextStep::Parley` leaves the retry loop as a
success; the post-resume re-run starts again at attempt 1 with a fresh budget.

**Shutdown during backoff.** The backoff wait races the engine's cancellation token, so a run
cancelled while a node sleeps between attempts aborts at once (never burning the shutdown grace
window). The node is recorded `Skipped { reason: "shutdown" }` and re-listed in the Halted
Waypoint's vanguard, and `resume` re-executes it from attempt 1.

**Tuning is not a graph change.** `retry` and `timeout` are deliberately excluded from
`WarGraph::fingerprint()`, so tightening either never makes `resume` fail with `GraphMismatch`.
`on_error` and `cache`, which change what a run *does*, are hashed (fingerprint version `v5`).

## Timeouts: Wall Clock versus Idle

A `TimeoutPolicy` bounds **each attempt** with up to two independent limits, both named by a typed
`TimeoutKind` on the resulting `NodeError` — never inferred from message text:

| Field | What it bounds | Fires as |
|---|---|---|
| `run_timeout` | A hard wall clock on the attempt; progress cannot extend it | `Timeout(Run)` |
| `idle_timeout` | The longest gap between two observed **progress events**; each event restarts the window | `Timeout(Idle)` |

The difference matters for streaming: a model that emits a chunk every 100 ms for two minutes is
healthy, while one that goes silent for 30 s is stalled. A wall clock alone cannot tell them apart.

```rust,ignore
{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:timeout}}
```

A timed-out attempt is `Transient`, so under a `RetryPolicy` it is retried like any other transient
failure, and its partial work is discarded exactly as any failed attempt's is — the attempt future
is dropped on expiry, so a half-finished delta can never become a `Directive`.

**What counts as progress.** For a Paladin node, `PaladinExecutionService` beats the attempt's
`HeartbeatHandle` after every LLM completion, every streamed chunk and every Armament invocation,
through the new `PaladinPort::execute_observed` method. A Function node beats explicitly via
`ctx.heartbeat()`. A `Battalion` node beats once per child superstep.

```rust,ignore
{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:heartbeat}}
```

`ctx.heartbeat()` on a node with no `idle_timeout` is a free no-op, so a node can beat
unconditionally.

**The run-level budget.** `EngineLimits.run_timeout` (from `EngineConfig.run_timeout_secs` /
`APP_ENGINE_RUN_TIMEOUT_SECS`) bounds the *whole* run and nests outside every per-attempt bound:
the effective per-attempt deadline is `min(run_timeout, remaining engine budget)`, and the fired
kind names whichever was tightest. An attempt the engine budget cuts records
`Timeout(EngineRun)`; that kind is **never retried** (the budget is gone), and the run ends with
`EngineError::RunTimeoutExceeded { elapsed, limit }` through the same `Failed`-Waypoint path
`RecursionLimitExceeded` and `NodeVisitLimitExceeded` take. The budget is measured per
`start`/`resume` call — a resume restarts it — and a Battalion child measures its own budget
against its own limits.

## Error Handlers and Compensation

`on_error` runs **only** on a node's *final* failure — after retries are exhausted, or immediately
when the predicate refuses the error — and never while retries remain. Three handler shapes exist:

```rust,ignore
{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:handlers}}
```

| `ErrorHandlerSpec` | Effect | Record shows |
|---|---|---|
| `Route { to, error_field }` | Serialises the `NodeError` as JSON into `error_field` and places `to` in the next Vanguard **instead of** the failed node's static successors | `Failed` |
| `Absorb { fallback_delta }` | Merges `fallback_delta` (schema-validated; an empty delta is legal) and fires the static edges as on success | `Failed` |
| `Custom(name)` | Awaits the registered `ErrorHandler::handle(&NodeError, &Battlefield)` over the **pre-superstep** snapshot and honours the `Directive` it returns exactly like a node's own | `Failed` |
| *(none)* | The run fails: a `Failed` Waypoint with `node_error: Some(..)` and `RunOutcome::Failed(EngineError::NodeFailed(..))` | `Failed` |

The node's record reads `Failed` whichever handler ran — the node *did* fail; the handler decided
what happens next.

**A worked compensation chain.** `book` calls a provider that rejects its credential (a
`Permanent` failure). Under the default predicate the retry policy is consulted once and declines,
so `book` executes exactly once; `Route` writes the structured error into `booking_error` and
`cancel` runs in the next superstep, reading it:

```rust,ignore
{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:compensation}}
```

Validation checks the wiring before the run: the Route target must be a declared node and not a
worker template (`RouteTargetUnknown` / `RouteTargetIsWorkerTemplate`), `error_field` must be a
declared field whose dispatch is not `Sum` (`RouteErrorFieldUndeclared` /
`RouteErrorFieldDispatchInvalid` — a serialised error object cannot be summed), and an `Absorb`
delta may write only declared fields (`AbsorbDeltaSchemaInvalid`). A Route target reachable only
by routing needs **no** `mark_dynamic_target` — it is eligible by declaration.

**Loops are bounded.** A routed visit counts against `max_node_visits` through the one existing
counter, so an `a → b → a` compensation cycle ends with `NodeVisitLimitExceeded` rather than
spinning.

**Custom handlers** are registered on the engine, like edge evaluators, and may return any
`NextStep` — including `Parley`, which suspends the run through the same human-in-the-loop path a
node's own parley takes (one `AwaitingInput` Waypoint; the post-resume re-run is a fresh attempt
1 with `ctx.parley_response()` set, and no retry budget is consumed):

```rust,ignore
{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:custom_handler}}
```

**On a worker template** (a node mustered as a task) the rule is: `Absorb`, or a `Custom` handler
that returns `NextStep::Edges` — a delta that becomes that task's aggregation contribution. A
`Route` on a worker template is rejected at validation (`HandlerNotAllowedOnWorkerTemplate`), and a
`Custom` handler returning `Goto`/`End`/`Parley`/`Muster` from inside a mustered task fails the run
with `MusterHandlerMustBeDeltaOnly { node, task_key, returned }` before any routing side effect.
Handle it at the aggregator instead.

## Model Fallback

`FallbackLlmAdapter` composes an ordered chain of `LlmPort`s into one `LlmPort`, so it drops in
wherever a single provider adapter would — no new trait, no policy type:

```rust,ignore
{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:fallback}}
```

**The hop rule.** Every call starts at element 0. The chain moves to the next element only when
the current one fails with an error whose `transience()` is `Transient` or `Unknown`; a `Permanent`
error short-circuits, so a malformed prompt or a rejected credential is never re-sent to every
provider. When the whole chain fails, the caller gets
`LlmError::AllProvidersFailed { attempts, last }` with one summary per provider in chain order,
and its own transience is exactly the last attempt's. No state survives a call: one call's hop
never changes where the next — or a concurrent one — starts.

**Streaming: the first-chunk rule.** `generate_stream` falls through only if the call itself fails
or the stream's **first** item is an error (the adapter peeks it and still delivers it). Once any
chunk has been delivered, a later error propagates unchanged — a partial answer is never silently
completed by a different model.

**Observability.** Each hop emits `TraceEvent::FallbackHop { node_id: None, from_provider,
to_provider }` through `FallbackLlmAdapter::with_trace_sink` and a `warn!` log line naming both
providers. A served response is stamped with the serving provider's name under
`LlmResponse.metadata["paladin.served_by"]`, which `PaladinExecutionService` copies into
`PaladinResult.served_by` (`None` for a non-fallback port). Treat `served_by` as observability,
not attestation.

**Circuit breakers sit above the chain.** The facade `CircuitBreaker` yields
`PaladinError::CircuitBreakerOpen` above the port boundary and is invisible to the adapter. A
breaker wrapping an *individual* `LlmPort` inside the chain must surface an `LlmError` the chain
classifies `Transient` (a `NetworkError`, or a `503` `ProviderError`) for the chain to hop past it.

## Node Caching

A `CachePolicy` lets a node's result be **served from a cache instead of executed**. Two things
are needed — a backend on the engine and a policy on the node:

```rust,ignore
{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:cache}}
```

**Key composition.** The key is a versioned, collision-resistant digest over: the graph fingerprint,
the node id, the node's *input* (a Paladin node's rendered `InputMapping` string; a Function
node's full Battlefield snapshot, or the `Fields(..)` subset), the mustered `task_key` and payload
when present, and the Paladin's own configuration fingerprint (model, system prompt, temperature,
`max_loops`, stop words). Change the prompt, the model, the graph shape or the input and the key
changes — a stale hit is impossible by construction, not by convention. `CacheKeySpec::Fields`
naming an undeclared field is a validation error (`CacheKeyFieldUndeclared`), because a typo would
otherwise silently narrow the key.

**TTL.** `ttl` is a hard expiry with a closed boundary: an entry read at exactly its `expires_at`
instant is a miss. The engine re-checks expiry on every hit regardless of what the backend served.

**What a hit and a miss look like.** The lookup runs before attempt 1 and outside the interceptor
chain. A hit merges the stored delta with **zero** executions — no port call, no interceptor —
and is recorded as `Succeeded` at attempt 1 with `cache_hit: true` on both the
`NodeExecutionRecord` and the `NodeFinished` trace event. A miss executes the node normally and
stores the delta only after a *successful*, `Edges`-routed attempt: a failure, a handler outcome,
or a `Goto`/`End`/`Parley`/`Muster` directive is never cached (replaying only the delta would drop
the routing).

**Best effort by construction.** A backend `get` error is a miss; a `put` error is logged and the
run continues. Without a backend, any `cache` policy anywhere in the graph — including inside a
`Battalion` child — fails `start`/`resume` with `CachePolicyWithoutCacheBackend` before a node
runs.

**Backends.** `InMemoryNodeCache` (always available; process-local) and `RedisNodeCache` behind
the `redis-cache` cargo feature on `paladin-storage` (facade passthrough `redis-cache`), which
stores each entry as JSON under `{key_prefix}:{key}` with a server-side expiry and invalidates by
cursor-based scan. Both run the same contract suite. Operators configure the backend through
`NodeCacheConfig` (`src/config/node_cache.rs`), off by default:

| Field | Default | Env var |
|---|---|---|
| `enabled` | `false` | `APP_NODE_CACHE_ENABLED` |
| `backend` (`in_memory` \| `redis`) | `in_memory` | `APP_NODE_CACHE_BACKEND` |
| `redis_host` | `localhost` | `APP_NODE_CACHE_REDIS_HOST` |
| `redis_port` | `6379` | `APP_NODE_CACHE_REDIS_PORT` |
| `redis_password` | *(none)* | `APP_NODE_CACHE_REDIS_PASSWORD` |
| `redis_db` | `0` | `APP_NODE_CACHE_REDIS_DB` |
| `key_prefix` | `paladin:node_cache` | `APP_NODE_CACHE_KEY_PREFIX` |

The per-node *policies* (retry, timeout, handlers, cache TTL and key) are code, never
configuration — there is no `APP_AEGIS_*` variable, on purpose.

## The Node-Kind Support Matrix

| Node kind | `retry` | `timeout` | `on_error` | `cache` |
|---|---|---|---|---|
| `Paladin` | yes | yes | yes | yes |
| `Function` | yes | yes | yes | yes |
| `Battalion` (subgraph) | **rejected** | yes | yes | **rejected** |
| `Gate` | **rejected** | **rejected** | **rejected** | **rejected** |

A `Battalion` node's child run is the attempt unit: `timeout` cancels the child through the
inherited token and the parent attempt fails with `Timeout`; `on_error` compensates the child's
failure. `retry` and `cache` are rejected because child Waypoints are durable state — attempt
isolation and cache replay would need per-attempt child-thread namespacing and a resume rule for a
failed child, which do not exist yet. A `Gate` node has no attempt to retry, time or cache; its
expiry is the Parley `on_expire` policy. Any Aegis on a `Gate`, or `retry`/`cache` on a
`Battalion`, is `EngineError::AegisUnsupportedForNodeKind`, naming every offender. (A cache
policy *inside* a Battalion child's own nodes works, through the inherited backend.)

## Limitations You Should Know About

1. **`idle_timeout` degrades to a wall clock on a port that never beats.** The default
   `PaladinPort::execute_observed` delegates to `execute` and correctly claims no progress. An
   `idle_timeout` on a node whose port does not override it therefore fires after the window
   elapses from the start of the attempt, whether or not work is happening — still a bound, never
   *no* bound. `PaladinExecutionService` reports progress; a custom port must call
   `heartbeat.beat()` from `execute_observed` to get the progress-aware behaviour.
2. **Caching an `Append` field replays the append.** A cached delta is merged exactly as the
   original was, so a hit on a node writing an `Append`-dispatch field appends *again* — and a
   Chronicle fork that hits the cache appends once more on the fork. This is allowed; the opt-out
   is the schema-level `cache: Deny` marker (`FieldSpec::with_cache(CacheMarker::Deny)`), which
   rejects a Paladin `output_field` marked `Deny` at validation and refuses to store any Function
   delta that touches a `Deny` field.
3. **Author-visible state carries raw content.** An `error_field` written by a `Route` handler
   holds a provider's (redacted, bounded) error text; a Waypoint payload holds whatever the
   Battlefield holds. Both inherit the M-B-04 warning in `MIGRATION.md` §9.1: a downstream node
   or an LLM call that reads them is reading provider-influenced text. Do not template either into
   a prompt without the same care you would give any untrusted input.
4. **A hanging `EdgeConditionEvaluator` is still unbounded — R-23-01 remains an accepted
   risk.** Per-attempt timeouts wrap *node execution*, not edge evaluation. A hostile or hanging
   evaluator is author-supplied in-process code with the same trust as a `StateNode`;
   `EngineLimits::max_supersteps` remains the run-level bound, and the new timeout machinery does
   not cover it. This page does not claim otherwise.

## Security Notes

- **Redact, then bound.** Every provider response body that enters an error — `ProviderError`,
  `LlmFailure`, a `NodeError` — passes through `map_http_status`, which redacts credentials
  *before* truncating to the 512-character excerpt budget. The order is load-bearing: bounding first
  can slice a secret across the truncation boundary and leak the surviving tail. Adapter-specific
  pre-checks (an Anthropic `403`, a Gemini error envelope) run ahead of the shared helper and never
  duplicate its table.
- **Classification never reads a message.** `transience()` on every variant reads typed fields;
  a status is a `u16`, never a substring.
- **`NodeCacheConfig.redis_password` is never `Debug`-printed** — the struct renders it as a
  fixed placeholder.
- **A cached delta is trusted state.** The key includes the graph fingerprint, the Redis key
  prefix is configuration, and no cross-graph collision is possible by construction; a backend
  shared with untrusted writers is a backend concern, not something the engine can defend against.
- **`served_by` is observability, not provenance.**
