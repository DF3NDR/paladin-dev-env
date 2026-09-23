# Phase 25: Node-Level Fault Tolerance - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-05
**Phase:** 25-node-level-fault-tolerance
**Mode:** `--auto` — every gray area auto-selected; every question resolved on the recommended
option without `AskUserQuestion`. Each selection is logged as
`[auto] [Area] — Q → Selected (recommended default)`.
**Areas discussed:** Error taxonomy & the port boundary, Aegis attachment & graph semantics,
Retry & timeout mechanics, Error handlers & compensation, Model fallback & serving-provider
metadata, Node cache placement & keying, Observability records & trace fields, Program
bookkeeping, tests & docs

---

## Error taxonomy & the port boundary (FT-01)

`[auto] Taxonomy — Q: "How does LlmError classification survive the PaladinError boundary
(core cannot see ports)?" → Selected: "Structured PaladinError::LlmFailure carrying transience/
status by value, converted at the application boundary" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Structured `LlmFailure` variant on `PaladinError` carrying `transience`/`status`/`provider`/`message`, converted where both types are visible | Preserves classification by value, Display byte-identical, `is_retryable` parity, FT-FR-02a-sanctioned replacement | ✓ |
| Move `LlmError` into `paladin-core` and re-export from ports | Lets `PaladinError::Llm(LlmError)` exist; large cross-crate move touching all nine adapters | |
| Parse the stringified message in `transience()` | Forbidden by FT-FR-01 ("do not parse strings to classify") | |

`[auto] Taxonomy — Q: "Which adapters gain the status-carrying variant now?" → Selected: "All
nine via one shared map_http_status helper; existing dedicated mappings byte-identical"
(recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| All nine adapters through one helper; 401/429/400/402/404 mappings unchanged | RT-06 then verifies rather than rebuilds; one register row | ✓ |
| Only the three default providers now, the rest in RT-06 | Leaves FT-FR-01's "provider adapters" partially unmet until Phase 26 | |
| A new variant per adapter | Nine shapes to classify; contradicts the shared conformance suite RT-06 wants | |

`[auto] Taxonomy — Q: "#[non_exhaustive] on PaladinError/LlmError/BattalionError?" → Selected:
"Yes, all three, same change, Y rows + allowlist entries" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Mark all three `#[non_exhaustive]` now (X-10.2 preference) | One-time break in the sanctioned v0.10.0 window; future variants free | ✓ |
| Deliberate-breaking variant additions without the attribute | Every future variant is another break; PRD explicitly asks for the attribute | |

`[auto] Taxonomy — Q: "How to resolve the three-way NodeError name collision?" → Selected: "New
structured core NodeError per PRD; engine newtype renamed StateNodeError; legacy summary untouched"
(recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| New `core::node_error::NodeError`; rename the Phase 22 engine newtype to `StateNodeError` (new-in-0.10, free); keep legacy `battalion::NodeError` | One name, one concept; no X-10 cost | ✓ |
| Name the new type `EngineNodeError` / `AegisError` | Departs from the PRD and the ubiquitous language | |
| Reuse the engine's `NodeError(String)` and grow it | Stringly (X-06), cannot carry transience/attempt | |

`[auto] Taxonomy — Q: "Is NodeError a live-error wrapper or a serde value type?" → Selected:
"Serde value type with serializable source summaries" (recommended default — forced by FT-FR-11/14
persistence and BattalionError: Clone)`

`[auto] Taxonomy — Q: "How does the structured error reach the Failed Waypoint?" → Selected:
"Additive #[serde(default)] node_error: Option<NodeError> on WaypointStatus::Failed, no reshape"
(recommended default)`

**Notes:** `is_retryable()` keeps its legacy answers (only caller: the facade circuit breaker);
`transience()` is the new authority. `StopReason` is left for Phase 26.

---

## Aegis attachment & graph semantics (FT-02 §2.1)

`[auto] Attachment — Q: "Where does the Aegis attach?" → Selected: "WarGraph sidecar map +
default_aegis, per-node wins wholesale" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `WarGraph::set_aegis` / `with_default_aegis` sidecar (the `defer_flags`/`dynamic_targets`/`worker_templates` pattern) | No `NodeSpec` reshape; validation resolves per-node-wins-wholesale | ✓ |
| An `aegis` field on every `NodeSpec` variant | `Function(Arc<dyn StateNode>)` is a tuple variant; every constructor and match reshapes | |
| A `NodeSpec::Guarded { inner, aegis }` wrapper variant | Nesting complicates every `match` on `NodeSpec` | |

`[auto] Attachment — Q: "Which Aegis parts enter the graph fingerprint?" → Selected: "on_error and
cache hash; retry and timeout excluded (like EngineLimits)" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Hash routing/merge-affecting parts only; `v4` → `v5` | Tuning a retry never trips `GraphMismatch` on resume | ✓ |
| Hash the whole Aegis | Every timeout tweak invalidates resumability | |
| Hash nothing | A changed `Route` target silently alters routing under a matching fingerprint | |

`[auto] Attachment — Q: "Which node kinds support which Aegis parts?" → Selected: "Paladin/Function
full; Battalion timeout+on_error only; Gate none" (recommended default; ⚠ narrows PRD §2.1's
blanket sentence, flagged in CONTEXT D-12)`

| Option | Description | Selected |
|--------|-------------|----------|
| Matrix: full on Paladin/Function; `timeout`+`on_error` on Battalion; none on Gate; typed validation errors otherwise | Every §3 acceptance criterion stays provable; child-Waypoint isolation deferred | ✓ |
| Full Aegis on every kind including Battalion retry/cache | Needs per-attempt child-thread namespacing + Failed-child resume rule; no FR forces it | |
| Paladin/Function only | Loses a cheap, coherent Battalion timeout | |

`[auto] Attachment — Q: "How are custom predicates and handlers registered?" → Selected: "Same as
CF-01 evaluators, bundled into one EngineRegistries passed to validate" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `with_retry_predicate` / `with_error_handler` on `WarEngine`; one `EngineRegistries` bundle to `validate` | Fail-closed at validation; stops `validate`'s signature growing per phase | ✓ |
| Three separate `validate` parameters | Third signature change in three phases | |
| Registry on `WarGraph` itself | Graph would own trait objects; breaks fingerprint/serde symmetry | |

`[auto] Attachment — Q: "Name of the Aegis retry type given the legacy battalion::RetryPolicy?" →
Selected: "PRD names in a dedicated aegis module; prelude exports Aegis and Transience only"
(recommended default)`

---

## Retry & timeout mechanics (FT-FR-03…10)

`[auto] Retry — Q: "Where does the retry loop wrap?" → Selected: "Around the whole per-node dispatch
closure; interceptors run per attempt" (recommended default — the 22-09 'Aegis wraps OUTSIDE this
chain' contract)`

`[auto] Retry — Q: "Backoff under shutdown?" → Selected: "Cancellation-aware sleep (select! on the
token); node recorded Skipped { reason: shutdown }" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `select!` the backoff sleep against the run's `CancellationToken` | No shutdown grace burned by a sleeping retry; HITL-04 semantics preserved | ✓ |
| Let the sleep finish, then observe cancellation | Up to `max_interval` (60 s) of grace wasted | |

`[auto] Timeouts — Q: "How does the engine observe progress for Paladin nodes?" → Selected:
"Defaulted PaladinPort::execute_observed(heartbeat) (X-10.4); service beats on LLM calls, stream
chunks, tool calls; degrades to wall clock when a port never beats" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Defaulted `execute_observed` method whose default delegates to `execute`; real implementation in `PaladinExecutionService` | No required trait method (X-10.4); honest degradation documented | ✓ |
| Switch the engine to `PaladinPort::execute_stream` when `idle_timeout` is set | Forks engine behavior; the streaming path may not match the tool loop; `MockPaladinPort` rejects streaming | |
| Heartbeat only via `ctx.heartbeat()` (Function nodes) | Idle timeout meaningless for Paladin nodes, which are the FR's target | |

`[auto] Timeouts — Q: "What happens when EngineLimits.run_timeout fires?" → Selected: "Typed
EngineError::RunTimeoutExceeded on the existing limit-error path; in-flight attempts record
Timeout(EngineRun)" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Follow the `RecursionLimitExceeded` / `NodeVisitLimitExceeded` path | Consistent typed-limit semantics by construction | ✓ |
| Treat it as cancellation → `Halted`, resumable | Invents a resumable timeout no FR asks for; conflates budget with shutdown | |

`[auto] Timeouts — Q: "The PRD's 'legacy Battalion timeout' nesting clause?" → Selected: "Vacuous —
bridges carry no Battalion timeout into EngineLimits; document it" (recommended default)`

`[auto] Retry — Q: "How to keep NodeContext PartialEq with a heartbeat handle?" → Selected:
"HeartbeatHandle newtype with a manual always-equal PartialEq" (recommended default)`

---

## Error handlers & compensation (FT-FR-11…15)

`[auto] Handlers — Q: "How is a Route target made reachable/eligible?" → Selected: "Goto machinery +
the 22-15 worklist insertion point; no mark_dynamic_target needed" (recommended default)`

`[auto] Handlers — Q: "Which handlers are allowed on a mustered worker template?" → Selected:
"Absorb and delta-only Custom; Route rejected at validation" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `Absorb` + delta-only `Custom` per task; `Route` rejected with a typed error naming the aggregator alternative | Aggregation semantics stay well-defined | ✓ |
| Allow `Route` from a task, recovery node runs after the muster superstep | Aggregator would see N−1 results; undefined by PRD | |
| No handlers inside a Muster | Loses the cheap per-task `Absorb` (fallback result) | |

`[auto] Handlers — Q: "Custom handler signature sync or async?" → Selected: "async_trait, like
EdgeConditionEvaluator::evaluate; may return Parley" (recommended default)`

`[auto] Handlers — Q: "Handler → Parley composition tested?" → Selected: "Yes — one integration test;
post-resume re-run is attempt 1 with parley_response set" (recommended default)`

---

## Model fallback & serving-provider metadata (FT-FR-16, 17)

`[auto] Fallback — Q: "Where do provider names for AllProvidersFailed come from?" → Selected:
"Existing LlmPort::get_provider_name(); chain is Vec<Arc<dyn LlmPort>> per PRD" (recommended
default)`

`[auto] Fallback — Q: "Streaming fall-through rule?" → Selected: "Err from generate_stream or an Err
FIRST item falls through; after any Ok chunk, propagate" (recommended default)`

`[auto] Fallback — Q: "How does a hop reach the TraceSink from paladin-llm?" → Selected: "Optional
with_trace_sink on the adapter emitting TraceEvent::FallbackHop { node_id: None, .. } + tracing::warn"
(recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Adapter-held optional `Arc<dyn TraceSink>`; PRD 07's `FallbackHop` shape now | FT-FR-17 is this phase's FR; Phase 28 renames nothing | ✓ |
| Log/metric only, defer the event to Phase 28 | Fails FT-FR-17's "emits a TraceSink event" | |

`[auto] Fallback — Q: "How does PaladinResult record the serving provider under X-10.3?" →
Selected: "Option (b): add served_by, keep the struct constructible, register deliberate-breaking
(constructible_struct_adds_field)" (recommended default; ⚠ corrects FT-FR-17's premise)`

| Option | Description | Selected |
|--------|-------------|----------|
| (b) Add `served_by: Option<String>` with `#[serde(default, skip_serializing_if)]`; struct stays constructible; `Y` row + allowlist entry | Keeps all 66 `..Default::default()` sites and every downstream FRU caller compiling; migrates ~20 full literals in-tree | ✓ |
| (a) Mark `PaladinResult` `#[non_exhaustive]` (PRD's literal wording) | Cross-crate FRU is disallowed on `#[non_exhaustive]` structs — breaks all 66 FRU sites, examples, doc-examples and every downstream constructor; contradicts "Default still works" | |
| Carry the provider only in `LlmResponse.metadata`, no `PaladinResult` field | Fails FT-FR-17 ("`PaladinResult` metadata records which provider served") | |

`[auto] Fallback — Q: "The PRD's 'open circuit breaker' trigger?" → Selected: "Documented boundary —
the facade breaker sits above the port; a breaker wrapping an LlmPort must surface a Transient
LlmError" (recommended default)`

---

## Node cache placement & keying (FT-FR-18…20)

`[auto] Cache — Q: "Which crate hosts the cache adapters?" → Selected: "paladin-storage (Phase 22
D-01 precedent, redis dep already there), new redis-cache feature" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `paladin-storage/src/node_cache/` mirroring `waypoint/`; `redis-cache = ["dep:redis"]` + facade passthrough | Shipped-tree precedence; reuses `ConnectionManager` pattern | ✓ |
| `paladin-memory` as PRD 04's header suggests | No `redis` dependency there; splits engine persistence across two crates | |
| Reuse the `redis-queue` feature name for the cache | Misnamed gate; couples two unrelated subsystems | |

`[auto] Cache — Q: "Default key for a Function node whose read set is unknown?" → Selected: "Full
Battlefield snapshot hash, narrowed by CacheKeySpec::Fields; graph fingerprint prefixed"
(recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `H(graph_fingerprint, node_id, snapshot-or-rendered-input, paladin_config_fingerprint)`; `Fields(..)` narrows | Correct by construction; `invalidate(prefix)` per graph/node | ✓ |
| Require explicit `Fields` for every Function node | Rejects the PRD's "Default" keying | |
| Omit the graph fingerprint | Cross-graph `node_id` collisions return foreign deltas | |

`[auto] Cache — Q: "A CachePolicy on a node when the engine has no cache?" → Selected: "Typed
validation error before execution (fail-closed)" (recommended default)`

`[auto] Cache — Q: "Does a put failure fail the run?" → Selected: "No — logged, best-effort by
construction; a get failure is a miss" (recommended default)`

`[auto] Cache — Q: "Config for the Redis cache?" → Selected: "NodeCacheConfig in src/config/
node_cache.rs, enabled: false, mirroring WaypointStoreConfig; Aegis policies stay code-only"
(recommended default)`

---

## Observability records & trace fields

`[auto] Observability — Q: "Add attempt / cache_hit / FallbackHop to TraceEvent now, or defer to
Phase 28 as Phase 24 did?" → Selected: "Add the minimal PRD 07-shaped fields now; everything else
stays Phase 28's" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `NodeStarted`/`NodeFinished { attempt }`, `NodeFinished { cache_hit }`, `FallbackHop` — per-attempt emission | FT-FR-03/17 need them; PRD 07 shape means no rename later | ✓ |
| Defer every `TraceEvent` change to OBS-01 | Retry attempts invisible to `RecordingTraceSink` tests; FT-FR-17 unmet | |

`[auto] Observability — Q: "AttemptRecord placement?" → Selected: "#[serde(default)] attempts:
Vec<AttemptRecord> + cache_hit: bool on NodeExecutionRecord; attempt = succeeding attempt"
(recommended default)`

---

## Program bookkeeping, tests & docs

`[auto] Bookkeeping — Q: "How is the E2E-3 seam replaced?" → Selected: "Additive per-Paladin
fail_paladin_until_attempt on FaultyPaladinPort returning a 503-classified LlmFailure; default
TransientOnly predicate; exact counts" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Per-Paladin counter (additive builder) + Transient-classified error under the default predicate | Proves FT-FR-01/05/06 together; global counter semantics untouched | ✓ |
| Keep the global counter and use `TransientAndUnknown` | The scripted `ExecutionError` classifies Unknown — widening the predicate hides a classification gap | |
| Keep the warm-up scripting | Leaves the seam in place; fails success criterion 4 | |

`[auto] Bookkeeping — Q: "MIGRATION §9.2 handling?" → Selected: "Resolve the four FT rows + a
PaladinPort default-method row; Y rows mirrored in the allowlist; one deliberate-zero note for
new-in-0.10 types" (recommended default)`

`[auto] Bookkeeping — Q: "Docs placement?" → Selected: "New user-guides/fault-tolerance.md after the
Parley page" (recommended default)`

`[auto] Bookkeeping — Q: "Wave shape?" → Selected: "PRD 04 §4 order; taxonomy and fallback as
parallel standalone waves" (recommended default)`

---

## Claude's Discretion

- Exact field sets of `NodeError` / `NodeErrorSource` / `AttemptRecord` / `CachedDelta`; the
  status-carrying variant's name; `EngineRegistries` / `HeartbeatHandle` naming and internals; the
  idle-timer implementation; jitter RNG; `TimeoutKind` Display strings; predicate/handler trait
  names.
- `BATTLEFIELD_SCHEMA_VERSION` bump for additive fields (follow the 22.1/24 precedent).
- Transience rows for `GarrisonError` / `ArsenalError` inner variants.
- In-crate cache test double vs `InMemoryNodeCache` dev-dependency.
- Hop log level and metric shape; plan count and wave assignment; which plan migrates the ~20
  full-literal `PaladinResult` sites.

## Deferred Ideas

- Aegis `retry`/`cache` on `NodeSpec::Battalion` (per-attempt child-thread namespacing + Failed-
  child resume rule).
- Aegis on `Gate` nodes for expiry compensation.
- `Route` out of a single mustered task.
- `CacheKeySpec::Custom(name)`.
- Honouring `Retry-After` on 429 (needs a reshaped `RateLimitExceeded`).
- Config-driven fallback chain through `provider_factory`.
- Global default Aegis from config/env.
- Resuming a `Failed` thread after a fix.
- Full PRD 07 `TraceEvent` payload, `EdgeEvaluated`, OTel span-per-attempt (Phase 28).
- Retry/fallback as `ExecutionMiddleware` (Phase 26).
- Provider rate-limit pacing, cache-stampede locks (FUT-09).
- Timeouts around `EdgeConditionEvaluator::evaluate` (R-23-01 stays accepted).
- `LlmProviderError` status-carrying parity; the `WarEngine` mdBook page (Phase 22 residual).
