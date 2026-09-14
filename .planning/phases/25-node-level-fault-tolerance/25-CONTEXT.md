# Phase 25: Node-Level Fault Tolerance - Context

**Gathered:** 2026-09-05
**Status:** Ready for planning
**Mode:** `--auto` (all gray areas auto-selected on recommended defaults; audit trail in 25-DISCUSSION-LOG.md)

<domain>
## Phase Boundary

Phase 25 delivers epic `FT` (PRD 04) on top of the Phase 22/22.1/23/24 engine, and nothing from
later epics beyond the seams PRD 04 itself declares:

1. **Error taxonomy (FT-01).** `Transience { Transient, Permanent, Unknown }` in `paladin-core`;
   `transience()` on `PaladinError` and `LlmError`, table-driven per variant; provider adapters
   gain a status-carrying `LlmError` variant so 408/429/5xx versus other 4xx classify **by value**,
   never by parsing a message; a structured, serializable `NodeError { node_id, attempt,
   transience, source }` carried through engine execution; `BattalionError::Node(NodeError)`; the
   three touched pre-existing public enums handled per X-10 and registered in `MIGRATION.md` §9.2
   (FT-FR-01, FT-FR-02, FT-FR-02a).
2. **Aegis retry (FT-02).** `Aegis { retry, timeout, on_error, cache }` attachable per node with a
   graph-level `default_aegis`; the exact backoff sequence under `tokio::time::pause` with asserted
   jitter bounds; predicate gating (Permanent → one attempt); attempt isolation with
   `AttemptRecord` history on the `NodeExecutionRecord`; per-task retry inside a Muster; retries
   within a superstep with no intermediate Waypoint (FT-FR-03…07). The Phase 23 E2E-3 seam in
   `tests/integration/e2e_muster_defer_order_test.rs` is replaced by a real per-task retry.
3. **Timeouts (FT-03).** A per-attempt wall-clock `run_timeout` and a progress-aware
   `idle_timeout` (stream chunks, node trace events, `ctx.heartbeat()`), nested inside the run-level
   `EngineLimits.run_timeout` — whose semantics this phase finally implements, as
   `src/config/engine.rs` and `MIGRATION.md` §9.5 promised ("Doc 04/FT-03 owns timeout
   semantics") — so the tightest bound fires and the error names which (FT-FR-08…10).
4. **Typed error handlers (FT-04).** `Route` / `Absorb` / registered-`Custom` handlers after
   exhaustion or a non-retryable error; unregistered names fail closed at validation (the CF-01
   pattern); no handler → a Failed Waypoint carrying the structured `NodeError`, never a bare
   string; handler-routed nodes count against `max_node_visits`; program scenario E2E-3 passes
   end-to-end together with CF-03 (FT-FR-11…15).
5. **Model fallback (FT-05).** `FallbackLlmAdapter` in `paladin-llm` failing over on
   Transient/Unknown only, a first-chunk streaming rule, `LlmError::AllProvidersFailed`, a trace
   event plus log per hop, and the serving provider recorded on `PaladinResult` (FT-FR-16, 17).
6. **Node caching (FT-06).** `CachePolicy` + `NodeCachePort` with InMemory and Redis adapters under
   one contract suite; the key covers node identity, resolved input and the Paladin config
   fingerprint; a hit merges the stored delta with `cache_hit: true` and no execution; failures are
   never cached; the Append-dispatch replay hazard is documented with a schema-level `cache: Deny`
   marker (FT-FR-18…20).

**Out of this phase:** legacy `ErrorStrategy` / Battalion-level timeout semantics (X-03, PRD 04
§5); provider rate-limit pacing and cache-stampede locks (FUT-09); middleware exposure of retry and
fallback (RT-02, Phase 26); provider conformance close-out (RT-06, Phase 26); the authoritative
`TraceEvent` enum and OTel span-per-attempt (OBS-01/02, Phase 28) beyond the minimal fields this
phase's own FRs need; `MIGRATION.md` §9.8 finalisation (SHIP-01, Phase 29). Any other behavioral
change discovered mid-implementation is an X-03 stop-and-flag event, not a judgment call.

</domain>

<decisions>
## Implementation Decisions

PRD 04 is the FR-level source of truth and already locks the type sketches (§2.1), the FR semantics
(FT-FR-01…20), the acceptance criteria (§3), the TDD ordering (§4) and the out-of-scope list (§5).
The decisions below settle only what PRD 04 left open, or what the shipped Phase 22/22.1/23/24 tree
makes concrete — including two places where the tree contradicts a PRD premise (D-12, D-26), which
are flagged with ⚠ for the developer to overturn at plan review if wanted. Do not re-litigate
anything PRD 04, PRD 01-03, overview §3 (X-01…X-11) or the Phase 22/22.1/23/24 CONTEXT decisions
state.

### Error taxonomy & the port boundary (FT-01; FT-FR-01, 02, 02a)

- **D-01: `Transience` is a core value type.** `paladin_core::platform::container::transience::
  Transience { Transient, Permanent, Unknown }` — `Copy + Eq + Hash + Serialize + Deserialize`,
  deliberately **not** `#[non_exhaustive]` (three-valued by design; predicates match it
  exhaustively, and a fourth value would be a taxonomy change, not an addition). `paladin-ports`
  imports it for `LlmError::transience()` (ADR-0016: core owns port value types; no new core
  dependency, ADR-0015).
- **D-02: LlmError classification crosses the PaladinError boundary by value, not by parsing.**
  `paladin-core` cannot see `LlmError` (X-01), and today every LLM failure is erased into
  `PaladinError::LlmError(e.to_string())` (`src/application/services/paladin/
  paladin_execution_service.rs:1607/1731/1901/1926`, `crates/paladin-battalion/src/
  conclave_execution_service.rs` ×3, `temperature_service.rs` ×1). Add a structured
  `PaladinError::LlmFailure { transience: Transience, status: Option<u16>, provider:
  Option<String>, message: String }` (X-06) and switch every site that holds a real `LlmError` to
  it through one conversion helper living where both types are visible (application layer /
  `paladin-battalion`, never core). Its `Display` is byte-identical to today's `"LLM error:
  {message}"` (with `message` = the source `LlmError`'s Display), and `is_retryable()` returns
  `true` for it — the legacy blanket answer for `LlmError(_)`, so
  `src/infrastructure/resilience/circuit_breaker.rs:224/281` behaves identically (X-03).
  `PaladinError::LlmError(String)` is retained, unconstructed by first-party code from now on
  (X-06), and classifies `Unknown`. This is the FT-FR-02a-sanctioned "stringly variant replaced"
  case: one §9.2 row, deliberate-breaking, with the Display-parity and `is_retryable`-parity
  arguments in the justification column. — **Reversibility:** costly — eight conversion sites and
  the register row.
- **D-03: One status-carrying `LlmError` variant, one shared mapping helper, all nine adapters.**
  Add `LlmError::ProviderError { provider: String, status: u16, message: String }` (name at
  Claude's discretion) for every non-2xx status that has no dedicated variant today — 5xx, 408,
  and the "unknown 4xx" arms that currently collapse into `ProcessingError(format!("HTTP {}: …"))`
  (`crates/paladin-llm/src/openai/adapter.rs:379-389`, `deepseek/adapter.rs:501-520`, kimi's
  `http_500_maps_to_processing_error_carrying_status`). The existing dedicated mappings —
  401 → `AuthenticationError`, 429 → `RateLimitExceeded`, 400 → `InvalidPrompt`/`TokenLimitExceeded`,
  402 → `UsageLimitExceeded`, 404 → `ModelNotAvailable` — stay byte-identical (X-03). One
  `map_http_status(provider, status, redacted_excerpt) -> LlmError` helper in `paladin-llm` is
  applied to the non-2xx branch of **all nine** adapters (openai, anthropic, deepseek, kimi, qwen,
  grok, gemini, ollama, openai_compatible) in this phase; RT-06 (Phase 26) then only re-verifies
  the three v0.8.0 paths, it does not rebuild them. The excerpt passes through
  `crates/paladin-llm/src/redaction.rs` **before** bounding (security.instructions.md). Tests that
  assert `msg.contains("500")` become variant/status assertions. `LlmProviderError` (paladin-llm's
  crate-local enum) is untouched unless an adapter needs it. §9.2 row per FT-FR-02a.
- **D-04: Three pre-existing enums become non-exhaustive in one change.** X-10.2 applies to
  `PaladinError`, `LlmError` and `BattalionError`. In-tree exhaustive matches gain `_` arms; each enum gets a §9.2 row marked `Y`
  (`enum_marked_non_exhaustive` is itself a major change to `cargo semver-checks`) mirrored by a
  `.cargo/semver-checks-allowlist.toml` entry and the per-crate
  `[package.metadata.cargo_semver_checks.lints]` suppression in the same commit. `StopReason` is
  **not** touched — Phase 26 (RT-02) decides its X-10.2 exception. — **Reversibility:** costly —
  the attribute is a one-release decision; removing it later is non-breaking but the `_` arms stay.
- **D-05: The transience table's non-obvious rows.** `PaladinError`: `Timeout`,
  `CircuitBreakerOpen` → Transient; `ConfigurationError`, `StopWordDetected`, `GarrisonRequired`,
  `MaxRetriesExceeded` → Permanent; `ExecutionError(String)`, `LlmError(String)` → Unknown;
  `GarrisonError`/`ArsenalError` → Unknown unless the inner variant is obviously permanent (planner
  decides per inner variant; the table test pins every row). `LlmError`: `NetworkError`, `Timeout`,
  `RateLimitExceeded` → Transient; `AuthenticationError`, `InvalidPrompt`, `UsageLimitExceeded`,
  `ModelNotAvailable`, `TokenLimitExceeded`, `EmptyCompletion` → Permanent; `ProcessingError(String)`
  → Unknown; `ProviderError` → Transient for 408/429/5xx, Permanent for any other 4xx;
  `AllProvidersFailed` → the transience of its last error. `is_retryable()` / `is_terminal()` keep
  every existing answer (X-03) and are documented as legacy predicates superseded by `transience()`.
- **D-06: The NodeError name collision is resolved in favour of the PRD.** The tree
  already holds `paladin_core::platform::container::battalion::NodeError { node_name, error }` (a
  pre-existing v0.9 public plain-data summary on `BattalionResult` — untouched, X-03) and
  `paladin_battalion::engine::node::NodeError(pub String)` (new in 0.10, Phase 22 — the
  `StateNode` author's failure type). The new structured type is
  `paladin_core::platform::container::node_error::NodeError` (the PRD's and the ubiquitous
  language's name); the engine's newtype is **renamed `StateNodeError`** (free — the `engine`
  module is absent at `v0.9.0`, deliberate-zero note) and becomes `NodeErrorSource::Function`'s
  payload; the legacy summary's rustdoc cross-links the new type. `BattalionError::Node(NodeError)`
  path-qualifies the new type. — **Reversibility:** reversible now, one-way once v0.10.0 ships.
- **D-07: NodeError is a serde value type with no live error objects inside it.** It is persisted on
  the Waypoint (FT-FR-14), written into a Battlefield field (FT-FR-11) and must sit inside
  `BattalionError: Clone`, while `PaladinError` is neither `Clone` nor serde. So
  `NodeErrorSource` carries serializable summaries — `Paladin { kind, status, provider, message }`,
  `Llm { .. }`, `Function { message }`, `Timeout(TimeoutKind)`, `Cancelled` — built by `From`
  conversions at the engine boundary, with the source error's `Display` in `message`. `Clone +
  PartialEq + Serialize + Deserialize`, `thiserror` Display, no own `schema_version` (nested inside
  the versioned Waypoint, the `ParleyRequest` precedent). Exact field set is Claude's discretion.
- **D-08: Where the structured error surfaces.** `WaypointStatus::Failed` gains an additive
  `#[serde(default)] node_error: Option<NodeError>` beside the existing `error: String` (kept as the
  display line) and `failed_node` — no reshape (the Phase 22.1/24 additive-field precedent, plus a
  three-backend contract round-trip case); `None` for pre-Aegis and engine-limit failures.
  `EngineError` gains a structured `NodeFailed(NodeError)` variant used by the exhausted-failure
  path (any stringly node-failure arm on that path is retired, retained unconstructed);
  `RunOutcome::Failed` exposes the same `NodeError`; `From<EngineError> for BattalionError` (added
  if absent) maps it to `BattalionError::Node(NodeError)`.

### Aegis attachment & graph semantics (FT-02; PRD 04 §2.1)

- **D-09: Aegis types live in a new core aegis module under the PRD's names.** The module is
  `paladin_core::platform::container::aegis` (`Aegis`, `RetryPolicy`, `RetryPredicate`, `TimeoutPolicy`, `ErrorHandlerSpec`, `CachePolicy`,
  `CacheKeySpec`), serde-derived, engine-matched enums `#[non_exhaustive]`. The legacy
  `battalion::RetryPolicy { max_attempts, base_delay, .. }` (v0.9, used by
  `ErrorStrategy::RetryThenContinue`) is untouched and documented as the legacy Battalion-strategy
  type; the core prelude re-exports `Aegis` and `Transience` only — never both `RetryPolicy`s.
  `RetryPolicy::default()` = PRD defaults (`initial_interval` 500 ms, factor 2.0, `max_interval`
  60 s, jitter on, `TransientOnly`) with `max_attempts: 3` (the legacy default; PRD names none);
  `max_attempts == 0` is a typed validation error.
- **D-10: Aegis attaches as a `WarGraph` sidecar, not as a field on every `NodeSpec` variant.**
  `WarGraph::set_aegis(node_id, Aegis)` and `WarGraph::with_default_aegis(Aegis)`, stored as
  `HashMap<NodeId, Aegis>` + `Option<Aegis>` beside `defer_flags` / `dynamic_targets` /
  `worker_templates` (`crates/paladin-battalion/src/engine/graph.rs:448-472`, the house pattern for
  per-node annotations); resolved at validation with per-node-wins-wholesale over `default_aegis`
  (no field-level merging — documented on both methods, PRD). `set_aegis` on an undeclared node is
  a typed validation error listing all offenders. Rejected: a field on each of the four `NodeSpec`
  variants (`Function(Arc<dyn StateNode>)` is a tuple variant — every constructor and match in the
  tree would reshape). — **Reversibility:** costly — the builder surface is what user graphs call.
- **D-11: Routing-affecting and merge-affecting Aegis parts hash, tuning parts do not.**
  `on_error` and `cache` enter `WarGraph::fingerprint()` (sorted by node id, length-prefixed, the
  22.1 D-15…D-19 discipline; version `v4` → `v5`, golden re-pinned); `retry` and `timeout` are
  excluded exactly as every `EngineLimits` field is (Phase 23 D-18), so tightening a retry policy or
  a timeout never makes `resume` fail `GraphMismatch`. — **Reversibility:** one-way after v0.10.0 —
  fingerprints are stored on Waypoints.
- **D-12: Node-kind support matrix.** `Paladin` and `Function` nodes: the full Aegis. `Battalion`
  nodes: `timeout` and `on_error` supported (the child run is the attempt unit; a timed-out child is
  cancelled through the inherited token and the parent attempt fails with `Timeout`); `retry` and
  `cache` are **rejected at validation** with a typed error naming the reason — child Waypoints are
  durable state, so attempt isolation and cache replay would need per-attempt child-thread
  namespacing and a resume rule for a Failed child (recorded under Deferred Ideas). `Gate` nodes:
  any Aegis is rejected (there is no attempt to retry, time or cache; expiry is HITL-FR-06's
  `on_expire`). ⚠ **This narrows PRD 04 §2.1's blanket "`NodeSpec` gains `aegis`"** — every §3
  acceptance criterion is a Paladin or Function node, so nothing provable is lost; the developer may
  overturn this at plan review, in which case the child-thread-per-attempt design under Deferred
  Ideas is the starting point.
- **D-13: Registries follow CF-01, bundled once.** `RetryPredicate::Custom(name)` resolves to a
  registered `Arc<dyn RetryPredicate>`-style trait object and `ErrorHandlerSpec::Custom(name)` to an
  `Arc<dyn ErrorHandler>`, both registered on `WarEngine` (`with_retry_predicate`,
  `with_error_handler`) exactly like `with_edge_evaluator`. Rather than growing
  `WarGraph::validate`'s parameter list a third time, the three registries are passed as one
  `EngineRegistries { edge_evaluators, retry_predicates, error_handlers }` reference (new-in-0.10
  signature change, deliberate-zero note). Unregistered names fail validation with typed
  `EngineError` variants listing **every** offender before any node executes. A child Battalion
  inherits the parent's registries wholesale (Phase 23 D-21). `ErrorHandler::handle` is
  `#[async_trait]` + `Send + Sync`: `async fn handle(&self, err: &NodeError, state: &Battlefield)
  -> Result<Directive, NodeError>` — async like `EdgeConditionEvaluator::evaluate`
  (`crates/paladin-battalion/src/edge_evaluator.rs:63-75`).

### Retry & timeout mechanics (FT-FR-03…10)

- **D-14: The Aegis wraps the whole per-node dispatch closure.** The retry loop lives around the
  `tokio::spawn` body in `crates/paladin-battalion/src/engine/superstep.rs:1515-1595` (trace →
  interceptor `before` chain → `execute_vanguard_node` → interceptor `after` chain → trace), so
  interceptors run **once per attempt** — the "Aegis wraps OUTSIDE this chain" contract documented
  at `engine/hooks.rs:208-212` (22-09). `InterceptDecision::Fail(err)` is an attempt failure subject
  to the predicate; `Skip` is not an error and is never retried. Attempt isolation is by
  construction: every attempt reads the same immutable superstep snapshot (Phase 22 D-12) and only
  the succeeding attempt's `Directive` leaves the closure — a failed attempt's delta never reaches
  the merge.
- **D-15: Backoff is pausable and cancellation-aware.** Delays use `tokio::time::sleep` /
  `tokio::time::Instant` only (never `std::thread::sleep`), delay(n) = min(initial × factor^(n−2),
  max) plus jitter uniform in [0, delay) from `rand`; the wait is `select!`ed against the run's
  `CancellationToken`, so a node sleeping in backoff at SIGTERM is aborted immediately, recorded
  `Skipped { reason: "shutdown" }` and re-listed in the Halted vanguard (HITL-04 D-19/D-20) — no
  shutdown grace is burned by a sleep. Tests: the exact 5-attempt sequence with jitter off; bounds
  with jitter on; Permanent → 1 attempt; Transient → retried; Unknown → retried only under
  `TransientAndUnknown`.
- **D-16: Records and trace fields pre-conform to PRD 07's shape.** `NodeExecutionRecord` gains
  `#[serde(default)] attempts: Vec<AttemptRecord>` (failed attempts only: `{ attempt, started_at,
  duration_ms, error: NodeError }`) and `#[serde(default)] cache_hit: bool`; `attempt` records the
  succeeding attempt number (PRD). `TraceEvent::NodeStarted` / `NodeFinished` gain `attempt: u32`
  and `NodeFinished` gains `cache_hit: bool`, emitted **once per attempt** (PRD 07's span-per-attempt
  shape, so Phase 28 renames nothing); every other PRD 07 payload field stays Phase 28's. This
  departs from Phase 24's "no new `TraceEvent` fields" resolution because FT-FR-03/17 are this
  phase's own FRs; recorded in the §9.2 deliberate-zero note (new-in-0.10 types).
- **D-17: Retry × Muster, × Waypoints, × Parley.** Retries are per task, keyed `(node_id,
  task_key)`, inside the task's own spawned future — siblings never wait or re-run. No Waypoint is
  written between attempts; Phase 23's intra-superstep muster-progress Waypoints (D-14) record only
  *completed* tasks; a resume re-executes the interrupted node from attempt 1 (documented; the
  kill-during-backoff test aborts the run task while paused inside a backoff, resumes from the
  `RecordingWaypointStore`, and asserts the attempt counter restarts and the port call count). A
  `NextStep::Parley` Directive is a success — never retried — and the post-resume re-run of a
  parleying node starts at attempt 1.
- **D-18: NodeContext gains an attempt counter and a heartbeat.** `NodeContext` gains
  `attempt: u32` and `heartbeat()`. The heartbeat is backed by a
  `HeartbeatHandle` newtype (an `Arc`-shared `tokio::sync` primitive) with a manual `PartialEq`
  (handles compare equal) so `NodeContext: Clone + PartialEq + Debug` is preserved
  (`engine/node.rs:24`); `heartbeat()` on a node without `idle_timeout` is a no-op.
- **D-19: Progress for Paladin nodes comes through a defaulted `PaladinPort` method (X-10.4).**
  `PaladinPort::execute_observed(&self, paladin, input, heartbeat: &HeartbeatHandle)` whose default
  body delegates to `execute` — a *correct* default: it claims no progress. The engine always calls
  it; `PaladinExecutionService` implements it to beat on every LLM-call completion, every stream
  chunk when it streams, and every Armament invocation; Function nodes beat via `ctx.heartbeat()`;
  a child Battalion beats once per child superstep. An `idle_timeout` on a node whose port never
  beats degrades to a per-attempt wall clock — stated on `TimeoutPolicy`'s rustdoc and in the guide.
  FT-FR-09's chunk-every-100 ms / stalls-300 ms test drives the handle from a mock port under a
  paused clock. §9.2 row: `PaladinPort`, default method, `N`.
- **D-20: Nesting and the run-level bound.** `TimeoutKind::{Run, Idle, EngineRun}`; the
  per-attempt deadline is `min(attempt run_timeout, remaining engine budget)` and the fired kind
  names whichever was tightest. `EngineLimits.run_timeout` exceeded → typed
  `EngineError::RunTimeoutExceeded`, taking whatever Waypoint/outcome path the existing limit
  errors (`RecursionLimitExceeded`, `NodeVisitLimitExceeded`) take today — consistent by
  construction, not a new path; attempts cut by the engine bound record `Timeout(EngineRun)`. The
  bridges (`engine/bridges.rs:206-275`) carry no legacy Battalion timeout into `EngineLimits`, so
  PRD's "any legacy Battalion timeout" clause is satisfied vacuously and documented as such; the
  legacy services are untouched (X-03).

### Error handlers & compensation (FT-FR-11…15)

- **D-21: `Route`, `Absorb`, no-handler.** `Route { to, error_field }` writes the serialized
  `NodeError` JSON to `error_field` — declared in the schema with a dispatch other than `Sum`, else
  a typed validation error — and places `to` in the next Vanguard replacing the failed node's
  static successors via the Goto machinery; `to` is auto-eligible in the reachability worklist
  (the documented insertion point from 22-15 — no `mark_dynamic_target` needed) and must be a
  declared, non-worker-template node. `Absorb { fallback_delta }` is validated against the schema
  at graph validation; the record reads `outcome: Failed`, the delta merges, static edges fire as
  on success. No handler → Failed Waypoint + `RunOutcome::Failed` carrying the `NodeError` (D-08).
  Handler-routed visits count against `max_node_visits` (the A → B → A compensation-loop test).
- **D-22: Handlers inside a Muster are delta-only.** On a worker template only `Absorb` and
  `Custom` are allowed, and a `Custom` handler's Directive must be delta-only (`NextStep::Edges`) —
  its delta becomes that task's contribution to the aggregation; `Route`, or a `Custom` returning
  `Goto`/`End`/`Parley`/`Muster`, on a worker template is rejected at validation with a typed error
  naming the alternative (handle it at the aggregator). Routing out of a single mustered task is a
  deferred idea.
- **D-23: Handler → Parley composition is real and tested.** A `Custom` handler may return
  `NextStep::Parley`: the failed node suspends with the handler's `ParleyRequest` (HITL-01 path),
  and the post-resume re-run is a fresh attempt 1 with `ctx.parley_response()` set (Phase 24
  D-07/D-08). One integration test ("on payment failure, parley a human").

### Model fallback (FT-FR-16, 17)

- **D-24: The fallback adapter takes a plain chain of ports with no trait change.**
  `FallbackLlmAdapter::new(chain: Vec<Arc<dyn LlmPort>>)`. Provider
  names come from the existing `LlmPort::get_provider_name()`
  (`crates/paladin-ports/src/output/llm_port.rs:1291`); the adapter reports `"fallback"`,
  `get_capabilities()` = the first element's, `validate_model` / `get_available_models` = the first
  element that answers `Ok`. Hop on `Transient | Unknown`; `Permanent` short-circuits; exhaustion →
  `LlmError::AllProvidersFailed { attempts: Vec<(String, String)>, last: Box<LlmError> }`
  (transience = `last`'s). PRD's "or an open circuit breaker": the facade `CircuitBreaker`
  (`src/infrastructure/resilience/circuit_breaker.rs`) yields `PaladinError::CircuitBreakerOpen`
  *above* the port and is invisible to the adapter, so a breaker wrapping an individual `LlmPort`
  must surface an `LlmError` the chain classifies Transient — documented, no new variant.
- **D-25: Streaming first-chunk rule and per-hop observability.** Fall through only when
  `generate_stream` itself returns `Err` or the stream's **first** item is `Err` (peeked); after any
  `Ok` chunk, errors propagate unchanged — both cases tested. Each hop emits
  `TraceEvent::FallbackHop { node_id: None, from_provider, to_provider }` (PRD 07's shape) through
  an optional `with_trace_sink(Arc<dyn TraceSink>)` on the adapter (`paladin-llm` already depends
  on `paladin-ports`) plus a `tracing::warn!` carrying both provider names; `node_id` is `None`
  from the adapter, which cannot know the node (Phase 28 may enrich).
- **D-26: Serving provider on PaladinResult uses X-10.3 option (b), correcting a PRD premise.**
  The adapter stamps `LlmResponse.metadata["paladin.served_by"] = provider_name` (the existing map
  at `llm_port.rs:618` — no `LlmResponse` change); `PaladinExecutionService` copies it into a new
  `PaladinResult.served_by: Option<String>` with `#[serde(default, skip_serializing_if =
  "Option::is_none")]` (legacy JSON byte-identical when absent; only a fallback-served result sets
  it). `PaladinResult` stays **constructible** and is **not** marked `#[non_exhaustive]`; the field
  addition is registered deliberate-breaking (`constructible_struct_adds_field`, allowlist entry)
  with this justification: cross-crate functional-update syntax is disallowed on
  `#[non_exhaustive]` structs, so option (a) would break all 66 `..Default::default()`
  construction sites across six crates, the examples and every downstream caller — contradicting
  FT-FR-17's own "`Default` still works" intent; option (b) keeps every FRU site compiling and
  breaks only full struct literals, whose in-tree instances this phase migrates (add
  `..Default::default()` or the field). ⚠ **FT-FR-17 assumed FRU survives `#[non_exhaustive]`; it
  does not** — recorded in the §9.2 justification column. — **Reversibility:** one-way once v0.10.0
  ships (a public struct field).

### Node cache (FT-FR-18…20)

- **D-27: Port in `paladin-ports`, adapters in `paladin-storage`, one new feature.**
  `paladin_ports::output::node_cache_port::NodeCachePort { get(key) -> Option<CachedDelta>,
  put(key, delta, ttl), invalidate(prefix) }` (`Send + Sync`, `#[async_trait]`); `CachedDelta {
  schema_version, delta: StateDelta, stored_at, expires_at }` in core (X-04 — a top-level persisted
  record). Adapters go to **`paladin-storage`** as `node_cache/{in_memory,redis,contract_tests}.rs`
  mirroring `waypoint/` — not `paladin-memory` as PRD 04's header suggests: Phase 22 D-01 put engine
  persistence in `paladin-storage`, and the `redis` dependency and `ConnectionManager` pattern
  already live there (`crates/paladin-storage/src/redis.rs`). New feature `redis-cache =
  ["dep:redis"]` on `paladin-storage` with a facade passthrough `redis-cache` (X-07; sharing the
  optional `redis` dep with `redis-queue`, never in `default`, X-11.4); `InMemoryNodeCache` is
  ungated with TTL eviction. Contract suite: hit / miss / TTL expiry / invalidate-prefix / overwrite;
  the Redis tier runs in the existing Docker-gated integration job (Phase 22 D-10 tiering).
  — **Reversibility:** costly — adapters in a published crate.
- **D-28: Key composition.** `key = H(graph_fingerprint, node_id, input_component,
  paladin_config_fingerprint)` where `input_component` is the rendered input string for a Paladin
  node, or a canonical hash of the fields the node reads for a Function node — defaulting to the
  **full Battlefield snapshot** (conservative and correct, since a Function node's read set is not
  statically known) and narrowed by `CacheKeySpec::Fields(Vec<FieldName>)`; the muster payload is
  included whenever present; `paladin_config_fingerprint` hashes model, system prompt, temperature,
  max loops and stop words (Paladin nodes only), so a prompt change invalidates naturally
  (FT-FR-20). Including the graph fingerprint makes a graph edit invalidate naturally and lets
  `invalidate(prefix)` target one graph or one node. `CacheKeySpec::{Default, Fields(..)}` only;
  a registered `Custom` key function is deferred.
- **D-29: Engine integration, schema marker, config.** `WarEngine::with_node_cache(Arc<dyn
  NodeCachePort>)`; a `CachePolicy` on any node with no engine cache is a typed validation error
  before execution (fail-closed). Lookup before attempt 1; a hit merges the stored delta, records
  `outcome: Succeeded`, `attempt: 1`, `cache_hit: true`, and emits `NodeStarted`/`NodeFinished
  { cache_hit: true }` with no execution; `put` only after a successful attempt; a `put` failure is
  logged and never fails the run (the cache is an optimisation — best-effort by construction); a
  `get` failure is a miss. `FieldSpec` gains `#[serde(default)] cache: CacheMarker { Allow
  (default), Deny }`; a `CachePolicy` on a node whose `output_field` (Paladin) or declared write set
  is `Deny` is a typed validation error; the Append-dispatch replay hazard on forks is documented in
  the guide. Config: `src/config/node_cache.rs` `NodeCacheConfig { enabled: false, backend,
  redis host/port/password/db, key_prefix }` mirroring `WaypointStoreConfig` (Phase 24 D-26; X-09).
  Aegis policies themselves are per-node code, like `DirectiveParser` and `on_parse_error` — no
  config struct or env var (the Phase 23 D-26 / §9.5 rule for per-node enums).

### Program bookkeeping, tests & docs

- **D-30: `MIGRATION.md` and the semver gate.** Resolve the four FT-owned `TBD` rows in §9.2
  (`BattalionError`, `PaladinError`, `LlmError`, `PaladinResult`) and add a `PaladinPort`
  default-method row (`N`); every `Y` row gets its `.cargo/semver-checks-allowlist.toml` entry and
  per-crate lint suppression in the same commit; one deliberate-zero note (the Phase 23/24 form)
  covering every new-in-0.10 type touched — `WarGraph` sidecar + `validate` signature,
  `NodeContext`, `NodeExecutionRecord`, `WaypointStatus::Failed`, `EngineError`, `TraceEvent`,
  `FieldSpec`, the `StateNodeError` rename. §9.3: any new dependency (`rand` if not already a
  battalion dependency); §9.4: none — no SQL migration (Redis keys, JSON payload columns); §9.5:
  `NodeCacheConfig` and the `EngineConfig.run_timeout_secs` bullet updated from "plumbing-only" to
  landed; §9.1: no behavioral change — every Aegis capability is opt-in per node and a v0.9 graph
  declares none.
- **D-31: The E2E-3 seam is replaced, not worked around.** `tests/helpers/mock_paladin_port.rs`'s
  `FaultyPaladinPort` gains an additive, chainable per-Paladin counter
  `fail_paladin_until_attempt(name, n)` (its global `fail_until_attempt` semantics stay exactly as
  the recorded STATE.md decision describes) that returns the Transient-classified
  `PaladinError::LlmFailure { status: Some(503), .. }`. The scripted warm-up block in
  `one_worker_recovers_by_manual_attempt_scripting` is deleted; under a per-task
  `RetryPolicy { max_attempts: 3, .. }` and the **default** `TransientOnly` predicate the test
  asserts exactly 7 port calls (5 workers + 2 retries), two `AttemptRecord`s on the recovering
  task's record, 5 results in `task_key` order and one aggregator run — proving FT-FR-01, 05 and 06
  together rather than by widening the predicate. Plus the X-05 multi-thread stress (Muster +
  per-task retry, exact counts, timeout guard, the `listener.rs` house pattern) and the
  `RunTimeoutExceeded` / handler-loop / kill-during-backoff tests.
- **D-32: Docs.** New `docs/src/user-guides/fault-tolerance.md` ("Aegis: Retry, Timeout, Error
  Handlers, Model Fallback and Node Caching") registered after the Parley page in
  `docs/src/SUMMARY.md`, in the `parley-and-chronicle.md` shape; rustdoc + doc tests on every new
  public item (X-08); `CHANGELOG.md` `[Unreleased]`; `08-traceability-matrix.md` G-08 / G-10…G-14
  gain test anchors.
- **D-33: Plan shape follows PRD 04 §4.** Suggested waves: (1) taxonomy, `NodeError`,
  `#[non_exhaustive]`, register rows — standalone; (2) `FallbackLlmAdapter` + `served_by` —
  standalone, parallel with (1); (3) Aegis types, sidecar, validation, `EngineRegistries`;
  (4) retry loop, records, trace fields, retry × Muster/Waypoint tests; (5) timeouts, heartbeat,
  `PaladinPort::execute_observed`, `RunTimeoutExceeded`; (6) handlers, Parley composition, E2E-3
  seam replacement, stress test; (7) cache port/adapters/config/marker + engine integration;
  (8) guide, MIGRATION, traceability, gate evidence — `cargo semver-checks` (vs 0.9.0), `msrv`
  (1.88), `make security`, `cargo clippy -- -D warnings`, coverage ≥ 82% (ADR-0006) — green on
  the phase's final commit.
- **D-34: Security posture for the planner's threat model.** Every provider message that enters
  `ProviderError`, `LlmFailure` or a `NodeError` is redacted **before** it is bounded
  (`crates/paladin-llm/src/redaction.rs`; security.instructions.md) and never carries an API key;
  `error_field` and Waypoint payloads are author-visible state and inherit M-B-04's raw-content
  warning; cached deltas are trusted state — the Redis key prefix is configuration, the key includes
  the graph fingerprint, and no cross-graph key collision is possible by construction (documented as
  a backend concern, PRD §5). R-23-01 (a hanging `EdgeConditionEvaluator`) **stays accepted**: the
  per-attempt timeouts wrap node execution, not edge evaluation — re-listed in this phase's
  security section rather than silently claimed closed.

### Claude's Discretion

- Exact field sets of `NodeError` / `NodeErrorSource` / `AttemptRecord` / `CachedDelta`; the
  status-carrying variant's name (`ProviderError` vs `HttpStatus`); `EngineRegistries` and
  `HeartbeatHandle` naming and internals; the idle-timer implementation; the jitter RNG source;
  `TimeoutKind` Display strings; the `Custom` handler / predicate trait names.
- Whether `BATTLEFIELD_SCHEMA_VERSION` bumps for the additive record/status fields — follow the
  Phase 22.1 / 24 additive-field precedent (`visit_counts`, `frontier`, `fork_of`).
- The transience row for each `GarrisonError` / `ArsenalError` inner variant (D-05 rule).
- Whether `paladin-battalion`'s engine tests use `InMemoryNodeCache` via a dev-dependency on
  `paladin-storage` or a smaller in-crate `RecordingNodeCache` in `test_support.rs`.
- Whether the fallback adapter's hop log is `tracing::warn!` or `tracing::info!`, and the metric
  shape (a counter keyed by provider pair is enough).
- Plan count and wave assignment within D-33; which plan owns the ~20 full-literal `PaladinResult`
  migrations (D-26).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase source of truth (behavior)
- `.project/v0.10.0/04-fault-tolerance.md` — **The FR-level source of truth for this phase.**
  §2.0 taxonomy (FT-FR-01/02/02a), §2.1 Aegis type sketch and attachment rule, §2.2 retry
  (FT-FR-03…07), §2.3 timeouts (FT-FR-08…10), §2.4 handlers (FT-FR-11…15), §2.5 fallback
  (FT-FR-16/17), §2.6 cache (FT-FR-18…20), §3 acceptance criteria 1-9, §4 TDD ordering, §5 out of
  scope. Every plan task traces to an FR here.
- `.project/v0.10.0/00-program-overview.md` — §3 X-01…X-11 (X-04 `schema_version` on
  `CachedDelta`; X-05 stress pattern; X-06 structured errors; X-07 features; X-09 config; X-10
  register rules — D-04/D-26/D-30; X-11 — the MSRV is **1.88** since Phase 22.1 D-06…D-11, which
  supersedes §3's "1.85" text), §4 ubiquitous language, §6 **E2E-3** (the scenario this phase must
  pass with CF-03), §7 defect register (no new BUG expected), §9.2 the four FT-owned rows.
- `.project/v0.10.0/01-battlefield-state-and-execution-engine.md` — ENG-FR-03 (typed limit errors
  — D-20 follows their path), ENG-FR-11 (one Waypoint per superstep — D-17), ENG-FR-14
  (fingerprint contents/exclusions — D-11), ENG-FR-21…23 (TraceSink, interceptor chain,
  cancellation — D-14/D-15/D-16), §8 seam table ("Aegis wrapper around node execution").
- `.project/v0.10.0/02-control-flow-routing-fanout-subgraphs.md` — CF-FR-01/02 (registry +
  fail-closed validation — D-13 copies it), CF-FR-05…08 (`Directive` / `NextStep` — the handler
  return type), CF-FR-09…13 (Muster — D-17/D-22), CF-FR-14…17 (subgraphs — D-12).
- `.project/v0.10.0/03-pause-resume-history-shutdown.md` — HITL-FR-01/04 (Parley from a handler —
  D-23), HITL-FR-13…15 (shutdown grace and `Skipped` re-listing — D-15's cancellation-aware
  backoff).
- `.project/v0.10.0/05-agent-runtime-enhancements.md` — RT-FR-09 (Phase 26 middleware MUST delegate
  to this phase's `FallbackLlmAdapter` and retry — keep them plain `LlmPort` / policy types),
  RT-FR-20/22 (429/5xx mapping per FT-FR-01 — D-03's helper is what RT-06 verifies).
- `.project/v0.10.0/07-observability-tooling.md` — §2 `TraceEvent` sketch (`NodeStarted` /
  `NodeFinished { attempt, cache_hit }`, `FallbackHop`) that D-16/D-25 pre-conform to; OBS-FR-05
  span-per-attempt.
- `.project/v0.10.0/08-traceability-matrix.md` — G-08, G-10, G-11, G-12, G-13, G-14 rows.
- `.planning/REQUIREMENTS.md` — FT-01…FT-06 capability clusters with FR ranges; the X-10/X-11
  versioning gate as part of every requirement's definition of done.
- `.planning/ROADMAP.md` — Phase 25 goal, dependencies (22, 23), the five success criteria;
  Phase 26 (RT) consumes this phase's fallback and retry.

### Program deliverable this phase appends to
- `MIGRATION.md` — §9.2 rows at lines 127, 128, 129, 133 (`BattalionError`, `PaladinError`,
  `LlmError`, `PaladinResult` — "TBD — owner FT-01/FT-05, Phase 25"), the Phase 23 and Phase 24
  deliberate-zero notes (form D-30 follows), §9.5 `EngineConfig.run_timeout_secs` bullet
  ("plumbing-only this phase; Doc 04/FT-03 owns timeout semantics" — D-20 lands it), §9.3, §9.4.
- `.cargo/semver-checks-allowlist.toml` — entry schema and the set-equality rule with §9.2's `Y`
  rows (D-04, D-26).

### Prior-phase decisions that constrain this phase
- `.planning/phases/24-pause-resume-history-graceful-shutdown/24-CONTEXT.md` — D-04 (typed
  `EngineError` over a stringly `NodeError` — D-06/D-08 continue it), D-07/D-08
  (`parley_response` on the post-resume re-run — D-23), D-19/D-20 (grace deadline, abort,
  `Skipped` re-listing — D-15), D-26 (`WaypointStoreConfig` — `NodeCacheConfig` mirrors it),
  D-29 (deliberate-zero note form), and the Claude's-discretion `TraceEvent` resolution D-16
  departs from, with reasons.
- `.planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-CONTEXT.md` — D-05/D-06
  (fail-closed registry, CF-01 — D-13), D-07 (`StateNode::run` → `Directive`), D-14
  (`MusterProgress` stored contract — D-17), D-15 (muster context — D-28 key), D-16
  (`EngineConfig`), D-17 (the E2E-3 seam D-31 replaces), D-18 (fingerprint `v3`, `EngineLimits`
  excluded — D-11), D-20/D-21 (child inherits the engine wholesale — D-12/D-13), D-26
  (code-configured, off by default — D-29).
- `.planning/phases/22.1-engine-readiness-defect-and-msrv-follow-up/22.1-CONTEXT.md` — D-06…D-11
  (MSRV 1.88), D-15…D-19 (fingerprint discipline — D-11), D-21…D-25 (additive `#[serde(default)]`
  Waypoint fields + contract round-trips — D-08/D-16).
- `.planning/phases/22-battlefield-state-superstep-engine/22-CONTEXT.md` — D-01 (persistence
  adapters live in `paladin-storage` — D-27), D-09/D-10 (contract-suite style, Tier 2 provable only
  in CI), D-11 (seeded-shuffle determinism harness), D-12 (snapshot isolation — D-14).
- `.planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-11-SUMMARY.md` — the
  seam's exact contract, and why engine-level resume-after-failure was not used (the `Failed`
  Waypoint carries `muster_progress: None`).
- `.planning/phases/22-battlefield-state-superstep-engine/22-09-SUMMARY.md` — "Doc 04's Aegis
  wraps OUTSIDE the interceptor chain" (D-14).
- `.planning/phases/22-battlefield-state-superstep-engine/22-15-SUMMARY.md` — the reachability
  worklist's documented insertion point for Aegis `Route { to }` targets (D-21).
- `.planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-SECURITY.md` — R-23-01
  (hanging evaluator, accepted; per-node timeouts do not cover evaluators — D-34 re-lists it).
- `.planning/STATE.md` — the recorded `FaultyPaladinPort` global-counter decision D-31 extends
  additively.

### Standing decisions and governance
- `.planning/decisions/0006-coverage-gate.md` (ADR-0006) — 82% workspace floor.
- `.planning/decisions/0015-core-ports-dependency-allowlist.md` (ADR-0015) — `Transience`,
  `NodeError`, the Aegis types and `CachedDelta` add no core dependency; `rand` lives in
  `paladin-battalion`, `redis` stays in `paladin-storage`.
- `.planning/decisions/0016-port-value-type-ownership.md` (ADR-0016) — core owns `Transience`,
  `NodeError`, `CachedDelta`; ports re-export.
- `.planning/decisions/0031-extracted-crate-dependency-rule.md` (ADR-0031) — `paladin-web` is
  untouched by this phase.
- `.planning/decisions/0046-facade-llm-feature-flag-wiring.md` (ADR-0046) — `FallbackLlmAdapter`
  is ungated and composes the gated provider adapters.
- `.github/instructions/security.instructions.md` — redaction before truncation for every provider
  body that enters `ProviderError` / `LlmFailure` / `NodeError`; no credential in any error, log or
  cached payload.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/paladin-core/src/platform/container/paladin_error.rs` — `PaladinError` (not
  `#[non_exhaustive]`, not `Clone`; `is_retryable()` / `is_terminal()` legacy predicates D-05 keeps;
  `Timeout(u64)`, `CircuitBreakerOpen`, `MaxRetriesExceeded`, stringly `LlmError(String)` D-02
  retires from first-party construction).
- `crates/paladin-ports/src/output/llm_port.rs` — `LlmError` (line 291; `NetworkError`,
  `AuthenticationError`, `InvalidPrompt`, `RateLimitExceeded`, `UsageLimitExceeded { provider,
  regain_hint }`, `ModelNotAvailable`, `TokenLimitExceeded`, `EmptyCompletion`,
  `ProcessingError(String)`, `Timeout(String)`), `LlmPort` (`generate` 1073, `generate_stream` 1173,
  `get_provider_name` 1291 — D-24 uses it, `get_capabilities` 1363), `LlmResponse.metadata`
  (line 618 — D-26's carrier), `LlmRequest`.
- `crates/paladin-llm/src/{openai,deepseek,anthropic,kimi,qwen,grok,gemini,ollama,
  openai_compatible}/adapter.rs` — the per-adapter non-2xx arms D-03 routes through one helper
  (`openai/adapter.rs:376-389` and `:425-432`, `deepseek/adapter.rs:501-520` are the reference
  shapes); `crates/paladin-llm/src/redaction.rs` (`redact_credentials`, `bounded_excerpt` —
  redact-then-bound); `crates/paladin-llm/src/error.rs` (`LlmProviderError` → `LlmError`);
  `crates/paladin-llm/src/mock.rs` (`MockLlmAdapter::with_error` — the fallback chain's test double).
- `crates/paladin-core/src/platform/container/battalion/mod.rs` — legacy `RetryPolicy` (line 189),
  `ErrorStrategy` (240), legacy `NodeError { node_name, error }` (510), `BattalionError` (732,
  `Clone`, stringly variants — gains `Node(NodeError)`).
- `crates/paladin-battalion/src/engine/node.rs` — `NodeError(pub String)` (line 18 → D-06
  `StateNodeError`), `NodeContext { node_id, thread_id, superstep, muster, parley_response }`
  (24; D-18 adds `attempt` + `heartbeat()`), `StateNode::run`.
- `crates/paladin-battalion/src/engine/superstep.rs` — the spawned per-node dispatch closure
  (1515-1595: `NodeStarted` → interceptor `before` → `execute_vanguard_node` (391) → `after` →
  `NodeFinished`) D-14 wraps; `NodeRunOutcome` / `NodeFailure` internal enums; `node_failure`
  first-wins bookkeeping (1605-1925) D-08/D-21 extend; the muster task dispatch D-17 keys;
  `build_waypoint` / `persist_waypoint` (2989/3042); `cancelled_or_pending` (154) D-15 selects on.
- `crates/paladin-battalion/src/engine/graph.rs` — `NodeSpec` (37, `#[non_exhaustive]`),
  `EngineLimits { run_timeout: Option<Duration>, .. }` (404-429 — D-20 implements it), `WarGraph`
  sidecar sets (448-472) and `add_node` / `mark_dynamic_target` / `add_worker_template` (492-563 —
  D-10's pattern), `validate(registry, &edge_evaluators)` (651 — D-13 bundles), `fingerprint()`
  (1398, `v4` → `v5`).
- `crates/paladin-battalion/src/engine/mod.rs` — `WarEngine` (868) builders `with_edge_evaluator`
  (971), `with_trace_sink` (983), `with_interceptors` (991), `with_cancellation_token` (999),
  `with_shutdown_grace` (1049) — `with_retry_predicate` / `with_error_handler` /
  `with_node_cache` sit beside them; `EngineError` (`#[non_exhaustive]`) gains `NodeFailed`,
  `RunTimeoutExceeded` and the validation variants; `RunOutcome::Failed`.
- `crates/paladin-battalion/src/edge_evaluator.rs` — `EdgeConditionEvaluator` (63, async) and
  `EdgeEvaluatorRegistry` (113): the registry + fail-closed pattern D-13 copies for predicates and
  handlers.
- `crates/paladin-battalion/src/engine/hooks.rs` — `NodeInterceptor` (218) with the "Aegis wraps
  OUTSIDE this chain" rustdoc (208-212), `TraceDispatcher`, `InterceptDecision`.
- `crates/paladin-battalion/src/engine/test_support.rs` — `CountingFunctionNode`,
  `RecordingPaladinPort`, `RecordingWaypointStore`, `RecordingTraceSink`, `shuffle_seeded`.
- `crates/paladin-core/src/platform/container/waypoint.rs` — `NodeOutcomeKind` (503,
  `#[non_exhaustive]`), `NodeExecutionRecord { .., attempt }` (535 — D-16 adds `attempts`,
  `cache_hit`), `WaypointStatus::Failed { error, failed_node }` (562 — D-08 adds `node_error`),
  `Waypoint.schema_version` (618), `GRAPH_FINGERPRINT_VERSION`.
- `crates/paladin-core/src/platform/container/battlefield.rs` — `FieldSpec { name, dispatch,
  default, required }` (100 — D-29 adds `cache`), `DispatchRule` (71), `BattlefieldSchema`.
- `crates/paladin-core/src/platform/container/directive.rs` — `Directive`, `NextStep`,
  `MusterTask { worker, payload, .. }` (96), `MusterContext { payload, task_key }` (117).
- `crates/paladin-core/src/platform/container/execution_result.rs` — `PaladinResult` (38,
  constructible, `Default`, `new()`; D-26 adds `served_by`), `StopReason` (76 — untouched).
- `crates/paladin-ports/src/output/trace_sink_port.rs` — `TraceEvent` (66, `#[non_exhaustive]`;
  D-16/D-25 add fields and `FallbackHop`), `TraceSink` (144).
- `crates/paladin-ports/src/output/paladin_port.rs` — `PaladinPort` (631; `execute` 683,
  `execute_stream` 752 returning an `mpsc::Receiver<Result<PaladinStreamChunk, _>>`, `validate`
  810) — D-19 adds the defaulted `execute_observed`.
- `crates/paladin-storage/src/waypoint/{in_memory,sqlite,postgres,contract_tests}.rs` — the
  adapter + contract-suite shape `node_cache/` mirrors; `crates/paladin-storage/src/redis.rs` —
  `redis::aio::ConnectionManager`, `RedisQueueConfig` (host/port/password/db/key_prefix) and the
  Docker-free unit-test convention the Redis cache adapter follows.
- `src/config/engine.rs` (`run_timeout_secs` plumbing), `src/config/waypoint_store.rs` and
  `src/config/waypoint_retention.rs` — the X-09 templates for `NodeCacheConfig`.
- `src/infrastructure/resilience/circuit_breaker.rs` — `CircuitBreaker::call_async` yielding
  `PaladinError::CircuitBreakerOpen` above the port (D-24's boundary).
- `tests/helpers/mock_paladin_port.rs` — `FaultyPaladinPort` (`fail_until_attempt` global counter,
  `fail_paladin(name)` chainable builder — D-31 adds `fail_paladin_until_attempt`);
  `tests/integration/e2e_muster_defer_order_test.rs` — the fenced "PHASE 25 SEAM (FT-FR-06)" block
  (lines 350-388) and its contract; `tests/integration/e2e_crash_resume_test.rs` +
  `Cargo.toml` `[[test]]` registration — the E2E template.
- `src/application/services/orchestration/listener.rs` — the X-05 multi-thread stress house
  pattern (exact counts + timeout guard).
- `docs/src/user-guides/parley-and-chronicle.md`, `docs/src/SUMMARY.md:24` — the guide shape and
  the `SUMMARY.md` slot the fault-tolerance page follows.

### Established Patterns
- Fail-closed registries resolved at validation, listing every offender before any node executes
  (CF-01) — D-13, D-21, D-29.
- Per-node annotations as `WarGraph` sidecar sets/maps with chainable `&mut self` builders — D-10.
- Additive `#[serde(default)]` fields on persisted engine types, no SQL migration, a three-backend
  contract round-trip case — D-08, D-16.
- Fingerprint discipline: hash what changes scheduling or merge, exclude tuning, bump the version
  tag, re-pin the golden — D-11.
- Typed `EngineError` / `thiserror` variants, `#[non_exhaustive]`, no new stringly variants (X-06);
  a retired arm is retained unconstructed — D-02, D-08.
- Config structs standalone under `src/config/` (`Default` + `validate()` + `EnvOverridable`,
  `APP_*`), off by default; per-node policy enums are code, never config — D-29.
- Persistence adapters in `paladin-storage` under cargo features, InMemory ungated, one contract
  suite, Redis/Postgres tier in the Docker-gated CI job — D-27.
- X-10: `#[non_exhaustive]` + `_` arms + `Y` register row + allowlist entry in one commit; default
  trait methods only; deliberate-zero notes for new-in-0.10 types — D-04, D-19, D-26, D-30.
- Redact-then-bound for every provider body that enters an error — D-03, D-34.
- Three-tier tests; seeded-shuffle determinism; paused-clock timing tests; X-05 multi-thread stress
  with exact counts and a timeout guard — D-15, D-31.
- Ubiquitous language: Aegis, Transience, Vanguard, Waypoint, Battlefield, WarGraph, WarEngine,
  Directive, Muster, Parley, Chronicle — in code, docs and comments.

### Integration Points
- `crates/paladin-core/src/platform/container/` — new `transience.rs`, `node_error.rs`,
  `aegis.rs`, `node_cache.rs` (`CachedDelta`); `paladin_error.rs` (`LlmFailure`,
  `#[non_exhaustive]`, `transience()`); `battalion/mod.rs` (`BattalionError::Node`,
  `#[non_exhaustive]`); `waypoint.rs` (record + status fields); `battlefield.rs` (`FieldSpec.cache`);
  `execution_result.rs` (`served_by`); `lib.rs` / prelude exports.
- `crates/paladin-ports/src/output/` — `llm_port.rs` (`ProviderError`, `AllProvidersFailed`,
  `#[non_exhaustive]`, `transience()`), new `node_cache_port.rs`, `trace_sink_port.rs`
  (`attempt`, `cache_hit`, `FallbackHop`), `paladin_port.rs` (`execute_observed` default method).
- `crates/paladin-llm/src/` — new `fallback.rs` (`FallbackLlmAdapter`), new shared
  `map_http_status` helper, the nine adapters' non-2xx arms, `lib.rs` exports.
- `crates/paladin-battalion/src/engine/` — `superstep.rs` (Aegis wrapper: retry loop, deadlines,
  heartbeat, cache lookup/store, handler dispatch, `Route` vanguard placement, muster per-task
  retry), `graph.rs` (sidecar, `validate` matrix, fingerprint `v5`), `mod.rs` (builders, errors,
  `RunTimeoutExceeded`), `node.rs` (`StateNodeError`, `attempt`, `heartbeat`), new
  `aegis.rs`/`registries.rs` (`EngineRegistries`, retry math, predicate + handler traits),
  `test_support.rs`; `conclave_execution_service.rs` (D-02 sites).
- `crates/paladin-storage/src/` — new `node_cache/{mod,in_memory,redis,contract_tests}.rs`,
  `Cargo.toml` `redis-cache` feature; root `Cargo.toml` passthrough.
- `src/application/services/paladin/paladin_execution_service.rs` (D-02 conversion sites,
  `execute_observed`, `served_by` copy), `temperature_service.rs`; new `src/config/node_cache.rs`;
  `src/config/engine.rs` rustdoc.
- `tests/helpers/mock_paladin_port.rs`, `tests/integration/e2e_muster_defer_order_test.rs`
  (seam), new integration tests (compensation chain, handler loop, Parley-from-handler, fallback
  chain, cache, kill-during-backoff, run-timeout, stress) + `Cargo.toml` `[[test]]` rows.
- `docs/src/user-guides/fault-tolerance.md`, `docs/src/SUMMARY.md`, `MIGRATION.md`
  §9.2/§9.3/§9.5, `.cargo/semver-checks-allowlist.toml`, per-crate `Cargo.toml` lint metadata,
  `CHANGELOG.md`, `.project/v0.10.0/08-traceability-matrix.md`.
- **Constraints confirmed in tree:** `PaladinResult` is constructed by struct literal at ~88 sites
  across six crates, tests and examples, 66 of them with `..Default::default()` (D-26);
  `paladin-memory` has no `redis` dependency while `paladin-storage` does (D-27); the `engine`
  module, `Waypoint`, `TraceEvent`, `NodeContext` and `FieldSpec` are all absent at `v0.9.0`
  (deliberate-zero, D-30); `EdgeConditionEvaluator::evaluate` is async (D-13);
  `EngineLimits.run_timeout` is declared but never enforced (D-20).

</code_context>

<specifics>
## Specific Ideas

- **Backoff table to assert exactly** (defaults, jitter off, 5 attempts): waits of 500 ms, 1 000 ms,
  2 000 ms, 4 000 ms between attempts 1→2→3→4→5, under `tokio::time::pause`; with jitter on, each
  wait lies in `[base, 2·base)` and the test asserts bounds only.
- **E2E-3 after the seam replacement:** planner musters 5; worker `w3` (per-Paladin counter) fails
  attempts 1 and 2 with a 503-classified `LlmFailure`, succeeds on 3; port call count 7; `w3`'s
  record has `attempt: 3` and two `AttemptRecord`s; aggregator runs once; `aggregated` holds 5
  results in `task_key` order; one Waypoint for the muster superstep (plus Phase 23's progress
  Waypoints) — no Waypoint between attempts.
- **Compensation chain shape:** `book` (Paladin, permanent `AuthenticationError`-classified failure)
  → `Route { to: cancel, error_field: booking_error }` → `cancel` succeeds → `RunOutcome::Completed`
  with `booking_error` holding the `NodeError` JSON (`node_id: "book"`, `attempt: 1`, `transience:
  "Permanent"`, message chain) and `book`'s record `outcome: Failed`.
- **Loop bound:** `a` routes to `b` on failure, `b` routes to `a`, both always fail,
  `max_node_visits = 3` → `EngineError::NodeVisitLimitExceeded`, never a spin.
- **Fallback chain test:** three `MockLlmAdapter`s — first two return `ProviderError { status:
  503 }`, third serves; `served_by == "mock-3"`, two `FallbackHop` events; a first-hop
  `AuthenticationError` → exactly one call (call-count mock) and no hop; exhaustion →
  `AllProvidersFailed.attempts.len() == 3`; streaming: error-before-first-chunk falls through,
  error-after-first-chunk propagates with the partial prefix intact.
- **Cache tests:** call-count mock proves zero executions on a hit; TTL expiry re-executes under
  the paused clock; changing the system prompt changes the key; a `put` failure (failing mock
  cache) leaves the run `Completed`.
- **Idle-vs-run test (FT-FR-09):** a mock port beating every 100 ms under `idle_timeout = 250 ms`
  survives; one that stalls 300 ms fails with `Timeout(Idle)`; `run_timeout = 10 s` never fires;
  the mirror case names `Timeout(Run)`; an engine `run_timeout` tighter than both names
  `Timeout(EngineRun)` and the run ends `RunTimeoutExceeded`.
- The `StateNodeError` rename and the `EngineRegistries` bundle are the only signature changes to
  new-in-0.10 types outside the FR list; both get a deliberate-zero sentence.
- Ubiquitous language holds: Aegis, Transience, Vanguard, Waypoint, Battlefield, WarGraph,
  WarEngine, Directive, Muster, Parley.

</specifics>

<deferred>
## Deferred Ideas

- **Aegis `retry` / `cache` on `NodeSpec::Battalion` (D-12)** — needs per-attempt child-thread
  derivation (attempt 1 keeps today's `ThreadId::child` id; attempt n ≥ 2 an injective suffix) plus
  a resume rule that never continues a Failed child; no FR forces it. The developer may promote it
  into this phase at plan review.
- **Aegis on `Gate` nodes** for expiry compensation (route on `on_expire`) — interacts with
  HITL-FR-06; own decision later.
- **`Route` out of a single mustered task (D-22)** — aggregation semantics for a routed task are
  undefined today; handle at the aggregator.
- **`CacheKeySpec::Custom(name)`** registered key function — D-28 ships `Default` + `Fields`.
- **Honouring `Retry-After`** on 429 in the backoff — would need `LlmError::RateLimitExceeded` to
  carry a value (a breaking reshape of a unit variant); FUT-09 territory.
- **Config-driven fallback chain** (`llm.fallback: [..]` through `provider_factory`) — the adapter
  is code-composed this phase, like `LlmDecision` (Phase 23 D-26); a Phase 26 (RT-02) or SHIP
  decision.
- **Global default Aegis from config/env** — per-node policies are code (D-29); an operator-level
  default is a new tunable for a later phase.
- **Resuming a `Failed` thread from its last good Waypoint after a fix** — PLAT-02 (Phase 27)
  resumes Running threads; a Failed thread stays terminal this phase.
- **Full PRD 07 `NodeFinished` payload, `EdgeEvaluated`, the authoritative `TraceEvent` enum, OTel
  span-per-attempt** — OBS-01/02 (Phase 28); D-16 adds only `attempt`, `cache_hit`, `FallbackHop`.
- **Retry/fallback as `ExecutionMiddleware`** — RT-02 (Phase 26) delegates to this phase's types.
- **Provider rate-limit pacing, distributed cache-stampede locks** — FUT-09 (PRD 04 §5).
- **Timeouts around `EdgeConditionEvaluator::evaluate`** — R-23-01 stays accepted (D-34).
- **`LlmProviderError` status-carrying parity** — only if an adapter's mapping needs it.
- **No mdBook page for the `WarEngine` itself** — Phase 22 residual still open; this phase adds the
  fault-tolerance page only.
- **22-REVIEW.md WR-01/WR-02, 22-deferred-items.md item 1, 24-REVIEW.md's three advisory
  warnings** — unchanged, not this phase's.

### Reviewed Todos (not folded)
- None matched this phase (`todo.match-phase 25` returned no matches; the single pending todo,
  "Verify local make coverage reproduces CI's 82.39% figure", is a local-tooling check owned by
  the maintainer).

</deferred>

---

*Phase: 25-node-level-fault-tolerance*
*Context gathered: 2026-09-05*
