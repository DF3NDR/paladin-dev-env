# Phase 25: Node-Level Fault Tolerance - Research

**Researched:** 2026-09-05
**Domain:** Rust internal engine feature work — error taxonomy, per-node retry/timeout/backoff,
typed compensation handlers, LLM provider fallback, and deterministic node-result caching, layered
onto the existing `paladin-battalion` `WarEngine` superstep engine.
**Confidence:** HIGH — this phase has no external-library unknowns; every claim below was checked
directly against the tree at `HEAD` (Phase 24 close) rather than derived from training knowledge.
`25-CONTEXT.md` (34 locked decisions, `--auto` mode) already did the deep design work; this
document's job is to (a) confirm every file:line anchor and code-shape claim it makes still holds,
(b) resolve the "Claude's Discretion" list with concrete recommendations, and (c) add the sections
the planner needs that CONTEXT.md doesn't carry (Validation Architecture, Security Domain, Package
Legitimacy Audit, Environment Availability).

## Summary

Phase 25 is pure `paladin-core` / `paladin-battalion` / `paladin-llm` / `paladin-storage` internal
engineering — no new external dependency is required (`rand = "0.8"` is **already** a
`paladin-battalion` dependency at the workspace level; `redis = "0.32.2"` is **already** a
`paladin-storage` dependency behind the existing `redis-queue` feature; `paladin-storage` is where
the new `redis-cache` feature and `node_cache/` module land, mirroring the existing `redis.rs`
`ConnectionManager` pattern verbatim). Every type this phase must add or extend was independently
located in the tree and matches CONTEXT.md's anchors exactly: `PaladinError` (9 variants, none
`#[non_exhaustive]` yet), `LlmError` (10 variants, `NetworkError`/`AuthenticationError`/
`InvalidPrompt`/`RateLimitExceeded`/`UsageLimitExceeded`/`ModelNotAvailable`/`TokenLimitExceeded`/
`EmptyCompletion`/`ProcessingError`/`Timeout`), `BattalionError`, the `engine/` module's
`superstep.rs` dispatch closure, `graph.rs`'s sidecar-map pattern and reachability worklist, and
`hooks.rs`'s explicit "Aegis wraps OUTSIDE the interceptor chain" rustdoc contract. The
`MIGRATION.md` §9.2 rows for `BattalionError`, `PaladinError`, `LlmError`, `PaladinResult` are
still `TBD — owner FT-*, Phase 25` exactly as CONTEXT.md states, and `.cargo/semver-checks-
allowlist.toml`'s schema/wiring (set-equality check against §9.2 `Y` rows, both directions, no
blanket wildcards) is live in `.github/workflows/ci.yml`'s `semver` job today.

The one genuinely new *pattern* this phase introduces to the codebase is a **paused-clock async
test** (`tokio::time::pause` / `start_paused`): a repo-wide grep found **zero** existing uses in
`paladin-battalion` src or tests. The backoff-under-paused-clock tests (FT-FR-04, D-15) will be the
first of their kind here — flag this to the planner as a place to budget extra iteration, not
because the technique is exotic (it is the standard `tokio::test` idiom) but because there is no
in-repo precedent to copy verbatim the way there is for the registry pattern (`edge_evaluator.rs`)
or the sidecar-map pattern (`graph.rs:448-472`).

**Primary recommendation:** Follow CONTEXT.md's 34 decisions and 8-wave plan shape (D-33) as
locked; this research adds no counter-recommendation to any of them. Where CONTEXT.md left a name
or internal detail to "Claude's discretion," this document proposes concrete answers (see
`## Don't Hand-Roll` and `## Code Examples`) so the planner is not left to invent them mid-plan.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FT-01 | `transience()` on `PaladinError`/`LlmError`, table-driven; status-carrying `LlmError` variant, no string parsing; structured `NodeError`; `BattalionError::Node(NodeError)`; X-10 register rows | Confirmed exact current variant lists for both enums (§ Architecture Patterns, § Code Examples); confirmed `.cargo/semver-checks-allowlist.toml` schema and `ci.yml` `semver` job wiring live and ready to receive the four new rows; confirmed zero `#[non_exhaustive]` markers exist yet on any of the three enums |
| FT-02 | Per-node Aegis retry: exact backoff under paused clock, predicate gating, attempt isolation, per-task Muster retry | Confirmed `superstep.rs:1515-1595` dispatch closure and `hooks.rs:208-212` "Aegis wraps OUTSIDE" contract; confirmed `rand 0.8` already available (no new dep); confirmed zero existing `tokio::time::pause` usage (new-pattern flag, see Summary) |
| FT-03 | Wall-clock `run_timeout` + progress-aware `idle_timeout`, nested with engine bound | Confirmed `EngineLimits.run_timeout` declared-but-unenforced at `graph.rs:404-429`; confirmed `RecursionLimitExceeded`/`NodeVisitLimitExceeded` construction sites (`superstep.rs:1265`/`1328`, wrapped by `EngineError` at `mod.rs:157`/`166`) as the precedent path `RunTimeoutExceeded` should follow; confirmed `PaladinPort::execute_observed` does not exist yet (clean slate for the defaulted method) |
| FT-04 | `Route`/`Absorb`/`Custom` handlers, fail-closed registration, `max_node_visits` bound, E2E-3 | Confirmed the exact reachability-worklist insertion point (`graph.rs:1102-1154`, `validate_eligible_set`) that `Route { to }` targets must seed into; confirmed the registry pattern to copy (`edge_evaluator.rs:63-135`, `EdgeConditionEvaluator` + `EdgeEvaluatorRegistry`); confirmed the exact E2E-3 seam block and its replacement contract (`tests/integration/e2e_muster_defer_order_test.rs:350-388`, `tests/helpers/mock_paladin_port.rs`'s `FaultyPaladinPort`) |
| FT-05 | `FallbackLlmAdapter`, Transient/Unknown-only hop, streaming first-chunk rule, `served_by` on `PaladinResult` | Confirmed `PaladinResult`'s existing `#[serde(default, skip_serializing_if = ...)]` precedent fields (`plan`, `handoff_history`) as the exact pattern `served_by` should follow; confirmed the struct-literal migration surface (91 `PaladinResult { .. }` sites total, 70 already use `..Default::default()`, 21 full literals concentrated in rustdoc examples / `execution_result.rs` / two test files / one real call site in `paladin_execution_service.rs`) |
| FT-06 | `CachePolicy` + `NodeCachePort`, InMemory + Redis, `cache_hit` merge, Append-dispatch `Deny` marker | Confirmed `redis 0.32.2` + `ConnectionManager` pattern in `crates/paladin-storage/src/redis.rs`; confirmed `paladin-storage`'s Docker-gated CI job pattern (`postgres-integration`, `docker-integration` jobs in `ci.yml`) that a `redis-cache` Tier-2 suite should reuse; confirmed `FieldSpec` (`battlefield.rs:100-111`) has no `cache` field yet (clean slate) |

</phase_requirements>

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

PRD 04 is the FR-level source of truth and already locks the type sketches (§2.1), the FR
semantics (FT-FR-01…20), the acceptance criteria (§3), the TDD ordering (§4) and the out-of-scope
list (§5). The decisions below settle only what PRD 04 left open, or what the shipped Phase
22/22.1/23/24 tree makes concrete — including two places where the tree contradicts a PRD premise
(D-12, D-26), which are flagged with ⚠ for the developer to overturn at plan review if wanted. Do
not re-litigate anything PRD 04, PRD 01-03, overview §3 (X-01…X-11) or the Phase 22/22.1/23/24
CONTEXT decisions state.

**D-01: `Transience` is a core value type.** `paladin_core::platform::container::transience::
Transience { Transient, Permanent, Unknown }` — `Copy + Eq + Hash + Serialize + Deserialize`,
deliberately **not** `#[non_exhaustive]` (three-valued by design; predicates match it exhaustively,
and a fourth value would be a taxonomy change, not an addition). `paladin-ports` imports it for
`LlmError::transience()` (ADR-0016: core owns port value types; no new core dependency, ADR-0015).

**D-02: LlmError classification crosses the PaladinError boundary by value, not by parsing.**
`paladin-core` cannot see `LlmError` (X-01), and today every LLM failure is erased into
`PaladinError::LlmError(e.to_string())` (`src/application/services/paladin/
paladin_execution_service.rs:1607/1731/1901/1926`, `crates/paladin-battalion/src/
conclave_execution_service.rs` ×3, `temperature_service.rs` ×1). Add a structured
`PaladinError::LlmFailure { transience: Transience, status: Option<u16>, provider: Option<String>,
message: String }` (X-06) and switch every site that holds a real `LlmError` to it through one
conversion helper living where both types are visible (application layer / `paladin-battalion`,
never core). Its `Display` is byte-identical to today's `"LLM error: {message}"` (with `message` =
the source `LlmError`'s Display), and `is_retryable()` returns `true` for it — the legacy blanket
answer for `LlmError(_)`, so `src/infrastructure/resilience/circuit_breaker.rs:224/281` behaves
identically (X-03). `PaladinError::LlmError(String)` is retained, unconstructed by first-party
code from now on (X-06), and classifies `Unknown`. This is the FT-FR-02a-sanctioned "stringly
variant replaced" case: one §9.2 row, deliberate-breaking, with the Display-parity and
`is_retryable`-parity arguments in the justification column. — **Reversibility:** costly — eight
conversion sites and the register row.

**D-03: One status-carrying `LlmError` variant, one shared mapping helper, all nine adapters.**
Add `LlmError::ProviderError { provider: String, status: u16, message: String }` (name at Claude's
discretion) for every non-2xx status that has no dedicated variant today — 5xx, 408, and the
"unknown 4xx" arms that currently collapse into `ProcessingError(format!("HTTP {}: …"))`
(`crates/paladin-llm/src/openai/adapter.rs:379-389`, `deepseek/adapter.rs:501-520`, kimi's
`http_500_maps_to_processing_error_carrying_status`). The existing dedicated mappings —
401 → `AuthenticationError`, 429 → `RateLimitExceeded`, 400 → `InvalidPrompt`/`TokenLimitExceeded`,
402 → `UsageLimitExceeded`, 404 → `ModelNotAvailable` — stay byte-identical (X-03). One
`map_http_status(provider, status, redacted_excerpt) -> LlmError` helper in `paladin-llm` is
applied to the non-2xx branch of **all nine** adapters (openai, anthropic, deepseek, kimi, qwen,
grok, gemini, ollama, openai_compatible) in this phase; RT-06 (Phase 26) then only re-verifies the
three v0.8.0 paths, it does not rebuild them. The excerpt passes through
`crates/paladin-llm/src/redaction.rs` **before** bounding (security.instructions.md). Tests that
assert `msg.contains("500")` become variant/status assertions. `LlmProviderError` (paladin-llm's
crate-local enum) is untouched unless an adapter needs it. §9.2 row per FT-FR-02a.

**D-04: Three pre-existing enums become non-exhaustive in one change.** X-10.2 applies to
`PaladinError`, `LlmError` and `BattalionError`. In-tree exhaustive matches gain `_` arms; each
enum gets a §9.2 row marked `Y` (`enum_marked_non_exhaustive` is itself a major change to `cargo
semver-checks`) mirrored by a `.cargo/semver-checks-allowlist.toml` entry and the per-crate
`[package.metadata.cargo_semver_checks.lints]` suppression in the same commit. `StopReason` is
**not** touched — Phase 26 (RT-02) decides its X-10.2 exception. — **Reversibility:** costly — the
attribute is a one-release decision; removing it later is non-breaking but the `_` arms stay.

**D-05: The transience table's non-obvious rows.** `PaladinError`: `Timeout`, `CircuitBreakerOpen`
→ Transient; `ConfigurationError`, `StopWordDetected`, `GarrisonRequired`, `MaxRetriesExceeded` →
Permanent; `ExecutionError(String)`, `LlmError(String)` → Unknown; `GarrisonError`/`ArsenalError` →
Unknown unless the inner variant is obviously permanent (planner decides per inner variant; the
table test pins every row). `LlmError`: `NetworkError`, `Timeout`, `RateLimitExceeded` → Transient;
`AuthenticationError`, `InvalidPrompt`, `UsageLimitExceeded`, `ModelNotAvailable`,
`TokenLimitExceeded`, `EmptyCompletion` → Permanent; `ProcessingError(String)` → Unknown;
`ProviderError` → Transient for 408/429/5xx, Permanent for any other 4xx; `AllProvidersFailed` →
the transience of its last error. `is_retryable()` / `is_terminal()` keep every existing answer
(X-03) and are documented as legacy predicates superseded by `transience()`.

**D-06: The NodeError name collision is resolved in favour of the PRD.** The tree already holds
`paladin_core::platform::container::battalion::NodeError { node_name, error }` (a pre-existing v0.9
public plain-data summary on `BattalionResult` — untouched, X-03) and
`paladin_battalion::engine::node::NodeError(pub String)` (new in 0.10, Phase 22 — the `StateNode`
author's failure type). The new structured type is
`paladin_core::platform::container::node_error::NodeError` (the PRD's and the ubiquitous language's
name); the engine's newtype is **renamed `StateNodeError`** (free — the `engine` module is absent
at `v0.9.0`, deliberate-zero note) and becomes `NodeErrorSource::Function`'s payload; the legacy
summary's rustdoc cross-links the new type. `BattalionError::Node(NodeError)` path-qualifies the
new type. — **Reversibility:** reversible now, one-way once v0.10.0 ships.

**D-07: NodeError is a serde value type with no live error objects inside it.** It is persisted on
the Waypoint (FT-FR-14), written into a Battlefield field (FT-FR-11) and must sit inside
`BattalionError: Clone`, while `PaladinError` is neither `Clone` nor serde. So `NodeErrorSource`
carries serializable summaries — `Paladin { kind, status, provider, message }`, `Llm { .. }`,
`Function { message }`, `Timeout(TimeoutKind)`, `Cancelled` — built by `From` conversions at the
engine boundary, with the source error's `Display` in `message`. `Clone + PartialEq + Serialize +
Deserialize`, `thiserror` Display, no own `schema_version` (nested inside the versioned Waypoint,
the `ParleyRequest` precedent). Exact field set is Claude's discretion.

**D-08: Where the structured error surfaces.** `WaypointStatus::Failed` gains an additive
`#[serde(default)] node_error: Option<NodeError>` beside the existing `error: String` (kept as the
display line) and `failed_node` — no reshape (the Phase 22.1/24 additive-field precedent, plus a
three-backend contract round-trip case); `None` for pre-Aegis and engine-limit failures.
`EngineError` gains a structured `NodeFailed(NodeError)` variant used by the exhausted-failure
path (any stringly node-failure arm on that path is retired, retained unconstructed);
`RunOutcome::Failed` exposes the same `NodeError`; `From<EngineError> for BattalionError` (added if
absent) maps it to `BattalionError::Node(NodeError)`.

**D-09: Aegis types live in a new core aegis module under the PRD's names.** The module is
`paladin_core::platform::container::aegis` (`Aegis`, `RetryPolicy`, `RetryPredicate`,
`TimeoutPolicy`, `ErrorHandlerSpec`, `CachePolicy`, `CacheKeySpec`), serde-derived, engine-matched
enums `#[non_exhaustive]`. The legacy `battalion::RetryPolicy { max_attempts, base_delay, .. }`
(v0.9, used by `ErrorStrategy::RetryThenContinue`) is untouched and documented as the legacy
Battalion-strategy type; the core prelude re-exports `Aegis` and `Transience` only — never both
`RetryPolicy`s. `RetryPolicy::default()` = PRD defaults (`initial_interval` 500 ms, factor 2.0,
`max_interval` 60 s, jitter on, `TransientOnly`) with `max_attempts: 3` (the legacy default; PRD
names none); `max_attempts == 0` is a typed validation error.

**D-10: Aegis attaches as a `WarGraph` sidecar, not as a field on every `NodeSpec` variant.**
`WarGraph::set_aegis(node_id, Aegis)` and `WarGraph::with_default_aegis(Aegis)`, stored as
`HashMap<NodeId, Aegis>` + `Option<Aegis>` beside `defer_flags` / `dynamic_targets` /
`worker_templates` (`crates/paladin-battalion/src/engine/graph.rs:448-472`, the house pattern for
per-node annotations); resolved at validation with per-node-wins-wholesale over `default_aegis` (no
field-level merging — documented on both methods, PRD). `set_aegis` on an undeclared node is a
typed validation error listing all offenders. Rejected: a field on each of the four `NodeSpec`
variants (`Function(Arc<dyn StateNode>)` is a tuple variant — every constructor and match in the
tree would reshape). — **Reversibility:** costly — the builder surface is what user graphs call.

**D-11: Routing-affecting and merge-affecting Aegis parts hash, tuning parts do not.** `on_error`
and `cache` enter `WarGraph::fingerprint()` (sorted by node id, length-prefixed, the 22.1 D-15…D-19
discipline; version `v4` → `v5`, golden re-pinned); `retry` and `timeout` are excluded exactly as
every `EngineLimits` field is (Phase 23 D-18), so tightening a retry policy or a timeout never
makes `resume` fail `GraphMismatch`. — **Reversibility:** one-way after v0.10.0 — fingerprints are
stored on Waypoints.

**D-12: Node-kind support matrix.** `Paladin` and `Function` nodes: the full Aegis. `Battalion`
nodes: `timeout` and `on_error` supported (the child run is the attempt unit; a timed-out child is
cancelled through the inherited token and the parent attempt fails with `Timeout`); `retry` and
`cache` are **rejected at validation** with a typed error naming the reason — child Waypoints are
durable state, so attempt isolation and cache replay would need per-attempt child-thread
namespacing and a resume rule for a Failed child (recorded under Deferred Ideas). `Gate` nodes: any
Aegis is rejected (there is no attempt to retry, time or cache; expiry is HITL-FR-06's
`on_expire`). ⚠ **This narrows PRD 04 §2.1's blanket "`NodeSpec` gains `aegis`"** — every §3
acceptance criterion is a Paladin or Function node, so nothing provable is lost; the developer may
overturn this at plan review, in which case the child-thread-per-attempt design under Deferred
Ideas is the starting point.

**D-13: Registries follow CF-01, bundled once.** `RetryPredicate::Custom(name)` resolves to a
registered `Arc<dyn RetryPredicate>`-style trait object and `ErrorHandlerSpec::Custom(name)` to an
`Arc<dyn ErrorHandler>`, both registered on `WarEngine` (`with_retry_predicate`,
`with_error_handler`) exactly like `with_edge_evaluator`. Rather than growing `WarGraph::validate`'s
parameter list a third time, the three registries are passed as one `EngineRegistries {
edge_evaluators, retry_predicates, error_handlers }` reference (new-in-0.10 signature change,
deliberate-zero note). Unregistered names fail validation with typed `EngineError` variants listing
**every** offender before any node executes. A child Battalion inherits the parent's registries
wholesale (Phase 23 D-21). `ErrorHandler::handle` is `#[async_trait]` + `Send + Sync`: `async fn
handle(&self, err: &NodeError, state: &Battlefield) -> Result<Directive, NodeError>` — async like
`EdgeConditionEvaluator::evaluate` (`crates/paladin-battalion/src/edge_evaluator.rs:63-75`).

**D-14: The Aegis wraps the whole per-node dispatch closure.** The retry loop lives around the
`tokio::spawn` body in `crates/paladin-battalion/src/engine/superstep.rs:1515-1595` (trace →
interceptor `before` chain → `execute_vanguard_node` → interceptor `after` chain → trace), so
interceptors run **once per attempt** — the "Aegis wraps OUTSIDE this chain" contract documented at
`engine/hooks.rs:208-212` (22-09). `InterceptDecision::Fail(err)` is an attempt failure subject to
the predicate; `Skip` is not an error and is never retried. Attempt isolation is by construction:
every attempt reads the same immutable superstep snapshot (Phase 22 D-12) and only the succeeding
attempt's `Directive` leaves the closure — a failed attempt's delta never reaches the merge.

**D-15: Backoff is pausable and cancellation-aware.** Delays use `tokio::time::sleep` /
`tokio::time::Instant` only (never `std::thread::sleep`), delay(n) = min(initial × factor^(n−2),
max) plus jitter uniform in [0, delay) from `rand`; the wait is `select!`ed against the run's
`CancellationToken`, so a node sleeping in backoff at SIGTERM is aborted immediately, recorded
`Skipped { reason: "shutdown" }` and re-listed in the Halted vanguard (HITL-04 D-19/D-20) — no
shutdown grace is burned by a sleep. Tests: the exact 5-attempt sequence with jitter off; bounds
with jitter on; Permanent → 1 attempt; Transient → retried; Unknown → retried only under
`TransientAndUnknown`.

**D-16: Records and trace fields pre-conform to PRD 07's shape.** `NodeExecutionRecord` gains
`#[serde(default)] attempts: Vec<AttemptRecord>` (failed attempts only: `{ attempt, started_at,
duration_ms, error: NodeError }`) and `#[serde(default)] cache_hit: bool`; `attempt` records the
succeeding attempt number (PRD). `TraceEvent::NodeStarted` / `NodeFinished` gain `attempt: u32` and
`NodeFinished` gains `cache_hit: bool`, emitted **once per attempt** (PRD 07's span-per-attempt
shape, so Phase 28 renames nothing); every other PRD 07 payload field stays Phase 28's. This
departs from Phase 24's "no new `TraceEvent` fields" resolution because FT-FR-03/17 are this
phase's own FRs; recorded in the §9.2 deliberate-zero note (new-in-0.10 types).

**D-17: Retry × Muster, × Waypoints, × Parley.** Retries are per task, keyed `(node_id, task_key)`,
inside the task's own spawned future — siblings never wait or re-run. No Waypoint is written
between attempts; Phase 23's intra-superstep muster-progress Waypoints (D-14) record only
*completed* tasks; a resume re-executes the interrupted node from attempt 1 (documented; the
kill-during-backoff test aborts the run task while paused inside a backoff, resumes from the
`RecordingWaypointStore`, and asserts the attempt counter restarts and the port call count). A
`NextStep::Parley` Directive is a success — never retried — and the post-resume re-run of a
parleying node starts at attempt 1.

**D-18: NodeContext gains an attempt counter and a heartbeat.** `NodeContext` gains `attempt: u32`
and `heartbeat()`. The heartbeat is backed by a `HeartbeatHandle` newtype (an `Arc`-shared
`tokio::sync` primitive) with a manual `PartialEq` (handles compare equal) so `NodeContext: Clone +
PartialEq + Debug` is preserved (`engine/node.rs:24`); `heartbeat()` on a node without
`idle_timeout` is a no-op.

**D-19: Progress for Paladin nodes comes through a defaulted `PaladinPort` method (X-10.4).**
`PaladinPort::execute_observed(&self, paladin, input, heartbeat: &HeartbeatHandle)` whose default
body delegates to `execute` — a *correct* default: it claims no progress. The engine always calls
it; `PaladinExecutionService` implements it to beat on every LLM-call completion, every stream
chunk when it streams, and every Armament invocation; Function nodes beat via `ctx.heartbeat()`; a
child Battalion beats once per child superstep. An `idle_timeout` on a node whose port never beats
degrades to a per-attempt wall clock — stated on `TimeoutPolicy`'s rustdoc and in the guide.
FT-FR-09's chunk-every-100 ms / stalls-300 ms test drives the handle from a mock port under a
paused clock. §9.2 row: `PaladinPort`, default method, `N`.

**D-20: Nesting and the run-level bound.** `TimeoutKind::{Run, Idle, EngineRun}`; the per-attempt
deadline is `min(attempt run_timeout, remaining engine budget)` and the fired kind names whichever
was tightest. `EngineLimits.run_timeout` exceeded → typed `EngineError::RunTimeoutExceeded`, taking
whatever Waypoint/outcome path the existing limit errors (`RecursionLimitExceeded`,
`NodeVisitLimitExceeded`) take today — consistent by construction, not a new path; attempts cut by
the engine bound record `Timeout(EngineRun)`. The bridges (`engine/bridges.rs:206-275`) carry no
legacy Battalion timeout into `EngineLimits`, so PRD's "any legacy Battalion timeout" clause is
satisfied vacuously and documented as such; the legacy services are untouched (X-03).

**D-21: `Route`, `Absorb`, no-handler.** `Route { to, error_field }` writes the serialized
`NodeError` JSON to `error_field` — declared in the schema with a dispatch other than `Sum`, else a
typed validation error — and places `to` in the next Vanguard replacing the failed node's static
successors via the Goto machinery; `to` is auto-eligible in the reachability worklist (the
documented insertion point from 22-15 — no `mark_dynamic_target` needed) and must be a declared,
non-worker-template node. `Absorb { fallback_delta }` is validated against the schema at graph
validation; the record reads `outcome: Failed`, the delta merges, static edges fire as on success.
No handler → Failed Waypoint + `RunOutcome::Failed` carrying the `NodeError` (D-08). Handler-routed
visits count against `max_node_visits` (the A → B → A compensation-loop test).

**D-22: Handlers inside a Muster are delta-only.** On a worker template only `Absorb` and `Custom`
are allowed, and a `Custom` handler's Directive must be delta-only (`NextStep::Edges`) — its delta
becomes that task's contribution to the aggregation; `Route`, or a `Custom` returning
`Goto`/`End`/`Parley`/`Muster`, on a worker template is rejected at validation with a typed error
naming the alternative (handle it at the aggregator). Routing out of a single mustered task is a
deferred idea.

**D-23: Handler → Parley composition is real and tested.** A `Custom` handler may return
`NextStep::Parley`: the failed node suspends with the handler's `ParleyRequest` (HITL-01 path), and
the post-resume re-run is a fresh attempt 1 with `ctx.parley_response()` set (Phase 24 D-07/D-08).
One integration test ("on payment failure, parley a human").

**D-24: The fallback adapter takes a plain chain of ports with no trait change.**
`FallbackLlmAdapter::new(chain: Vec<Arc<dyn LlmPort>>)`. Provider names come from the existing
`LlmPort::get_provider_name()` (`crates/paladin-ports/src/output/llm_port.rs:1291`); the adapter
reports `"fallback"`, `get_capabilities()` = the first element's, `validate_model` /
`get_available_models` = the first element that answers `Ok`. Hop on `Transient | Unknown`;
`Permanent` short-circuits; exhaustion → `LlmError::AllProvidersFailed { attempts:
Vec<(String, String)>, last: Box<LlmError> }` (transience = `last`'s). PRD's "or an open circuit
breaker": the facade `CircuitBreaker` (`src/infrastructure/resilience/circuit_breaker.rs`) yields
`PaladinError::CircuitBreakerOpen` *above* the port and is invisible to the adapter, so a breaker
wrapping an individual `LlmPort` must surface an `LlmError` the chain classifies Transient —
documented, no new variant.

**D-25: Streaming first-chunk rule and per-hop observability.** Fall through only when
`generate_stream` itself returns `Err` or the stream's **first** item is `Err` (peeked); after any
`Ok` chunk, errors propagate unchanged — both cases tested. Each hop emits
`TraceEvent::FallbackHop { node_id: None, from_provider, to_provider }` (PRD 07's shape) through an
optional `with_trace_sink(Arc<dyn TraceSink>)` on the adapter (`paladin-llm` already depends on
`paladin-ports`) plus a `tracing::warn!` carrying both provider names; `node_id` is `None` from the
adapter, which cannot know the node (Phase 28 may enrich).

**D-26: Serving provider on PaladinResult uses X-10.3 option (b), correcting a PRD premise.** The
adapter stamps `LlmResponse.metadata["paladin.served_by"] = provider_name` (the existing map at
`llm_port.rs:618` — no `LlmResponse` change); `PaladinExecutionService` copies it into a new
`PaladinResult.served_by: Option<String>` with `#[serde(default, skip_serializing_if =
"Option::is_none")]` (legacy JSON byte-identical when absent; only a fallback-served result sets
it). `PaladinResult` stays **constructible** and is **not** marked `#[non_exhaustive]`; the field
addition is registered deliberate-breaking (`constructible_struct_adds_field`, allowlist entry)
with this justification: cross-crate functional-update syntax is disallowed on `#[non_exhaustive]`
structs, so option (a) would break all 66 `..Default::default()` construction sites across six
crates, the examples and every downstream caller — contradicting FT-FR-17's own "`Default` still
works" intent; option (b) keeps every FRU site compiling and breaks only full struct literals,
whose in-tree instances this phase migrates (add `..Default::default()` or the field). ⚠
**FT-FR-17 assumed FRU survives `#[non_exhaustive]`; it does not** — recorded in the §9.2
justification column. — **Reversibility:** one-way once v0.10.0 ships (a public struct field).

**D-27: Port in `paladin-ports`, adapters in `paladin-storage`, one new feature.**
`paladin_ports::output::node_cache_port::NodeCachePort { get(key) -> Option<CachedDelta>, put(key,
delta, ttl), invalidate(prefix) }` (`Send + Sync`, `#[async_trait]`); `CachedDelta { schema_version,
delta: StateDelta, stored_at, expires_at }` in core (X-04 — a top-level persisted record). Adapters
go to **`paladin-storage`** as `node_cache/{in_memory,redis,contract_tests}.rs` mirroring
`waypoint/` — not `paladin-memory` as PRD 04's header suggests: Phase 22 D-01 put engine persistence
in `paladin-storage`, and the `redis` dependency and `ConnectionManager` pattern already live there
(`crates/paladin-storage/src/redis.rs`). New feature `redis-cache = ["dep:redis"]` on
`paladin-storage` with a facade passthrough `redis-cache` (X-07; sharing the optional `redis` dep
with `redis-queue`, never in `default`, X-11.4); `InMemoryNodeCache` is ungated with TTL eviction.
Contract suite: hit / miss / TTL expiry / invalidate-prefix / overwrite; the Redis tier runs in the
existing Docker-gated integration job (Phase 22 D-10 tiering). — **Reversibility:** costly —
adapters in a published crate.

**D-28: Key composition.** `key = H(graph_fingerprint, node_id, input_component,
paladin_config_fingerprint)` where `input_component` is the rendered input string for a Paladin
node, or a canonical hash of the fields the node reads for a Function node — defaulting to the
**full Battlefield snapshot** (conservative and correct, since a Function node's read set is not
statically known) and narrowed by `CacheKeySpec::Fields(Vec<FieldName>)`; the muster payload is
included whenever present; `paladin_config_fingerprint` hashes model, system prompt, temperature,
max loops and stop words (Paladin nodes only), so a prompt change invalidates naturally (FT-FR-20).
Including the graph fingerprint makes a graph edit invalidate naturally and lets
`invalidate(prefix)` target one graph or one node. `CacheKeySpec::{Default, Fields(..)}` only; a
registered `Custom` key function is deferred.

**D-29: Engine integration, schema marker, config.** `WarEngine::with_node_cache(Arc<dyn
NodeCachePort>)`; a `CachePolicy` on any node with no engine cache is a typed validation error
before execution (fail-closed). Lookup before attempt 1; a hit merges the stored delta, records
`outcome: Succeeded`, `attempt: 1`, `cache_hit: true`, and emits `NodeStarted`/`NodeFinished {
cache_hit: true }` with no execution; `put` only after a successful attempt; a `put` failure is
logged and never fails the run (the cache is an optimisation — best-effort by construction); a
`get` failure is a miss. `FieldSpec` gains `#[serde(default)] cache: CacheMarker { Allow (default),
Deny }`; a `CachePolicy` on a node whose `output_field` (Paladin) or declared write set is `Deny` is
a typed validation error; the Append-dispatch replay hazard on forks is documented in the guide.
Config: `src/config/node_cache.rs` `NodeCacheConfig { enabled: false, backend, redis
host/port/password/db, key_prefix }` mirroring `WaypointStoreConfig` (Phase 24 D-26; X-09). Aegis
policies themselves are per-node code, like `DirectiveParser` and `on_parse_error` — no config
struct or env var (the Phase 23 D-26 / §9.5 rule for per-node enums).

**D-30: `MIGRATION.md` and the semver gate.** Resolve the four FT-owned `TBD` rows in §9.2
(`BattalionError`, `PaladinError`, `LlmError`, `PaladinResult`) and add a `PaladinPort`
default-method row (`N`); every `Y` row gets its `.cargo/semver-checks-allowlist.toml` entry and
per-crate lint suppression in the same commit; one deliberate-zero note (the Phase 23/24 form)
covering every new-in-0.10 type touched — `WarGraph` sidecar + `validate` signature, `NodeContext`,
`NodeExecutionRecord`, `WaypointStatus::Failed`, `EngineError`, `TraceEvent`, `FieldSpec`, the
`StateNodeError` rename. §9.3: any new dependency (`rand` if not already a battalion dependency —
**research confirms `rand = "0.8"` IS already a `paladin-battalion` dependency, so this §9.3 row is
empty/none for this phase**); §9.4: none — no SQL migration (Redis keys, JSON payload columns);
§9.5: `NodeCacheConfig` and the `EngineConfig.run_timeout_secs` bullet updated from
"plumbing-only" to landed; §9.1: no behavioral change — every Aegis capability is opt-in per node
and a v0.9 graph declares none.

**D-31: The E2E-3 seam is replaced, not worked around.** `tests/helpers/mock_paladin_port.rs`'s
`FaultyPaladinPort` gains an additive, chainable per-Paladin counter
`fail_paladin_until_attempt(name, n)` (its global `fail_until_attempt` semantics stay exactly as the
recorded STATE.md decision describes) that returns the Transient-classified
`PaladinError::LlmFailure { status: Some(503), .. }`. The scripted warm-up block in
`one_worker_recovers_by_manual_attempt_scripting` is deleted; under a per-task `RetryPolicy {
max_attempts: 3, .. }` and the **default** `TransientOnly` predicate the test asserts exactly 7
port calls (5 workers + 2 retries), two `AttemptRecord`s on the recovering task's record, 5 results
in `task_key` order and one aggregator run — proving FT-FR-01, 05 and 06 together rather than by
widening the predicate. Plus the X-05 multi-thread stress (Muster + per-task retry, exact counts,
timeout guard, the `listener.rs` house pattern) and the `RunTimeoutExceeded` / handler-loop /
kill-during-backoff tests.

**D-32: Docs.** New `docs/src/user-guides/fault-tolerance.md` ("Aegis: Retry, Timeout, Error
Handlers, Model Fallback and Node Caching") registered after the Parley page in
`docs/src/SUMMARY.md`, in the `parley-and-chronicle.md` shape; rustdoc + doc tests on every new
public item (X-08); `CHANGELOG.md` `[Unreleased]`; `08-traceability-matrix.md` G-08 / G-10…G-14 gain
test anchors.

**D-33: Plan shape follows PRD 04 §4.** Suggested waves: (1) taxonomy, `NodeError`,
`#[non_exhaustive]`, register rows — standalone; (2) `FallbackLlmAdapter` + `served_by` —
standalone, parallel with (1); (3) Aegis types, sidecar, validation, `EngineRegistries`; (4) retry
loop, records, trace fields, retry × Muster/Waypoint tests; (5) timeouts, heartbeat,
`PaladinPort::execute_observed`, `RunTimeoutExceeded`; (6) handlers, Parley composition, E2E-3 seam
replacement, stress test; (7) cache port/adapters/config/marker + engine integration; (8) guide,
MIGRATION, traceability, gate evidence — `cargo semver-checks` (vs 0.9.0), `msrv` (1.88), `make
security`, `cargo clippy -- -D warnings`, coverage ≥ 82% (ADR-0006) — green on the phase's final
commit.

**D-34: Security posture for the planner's threat model.** Every provider message that enters
`ProviderError`, `LlmFailure` or a `NodeError` is redacted **before** it is bounded
(`crates/paladin-llm/src/redaction.rs`; security.instructions.md) and never carries an API key;
`error_field` and Waypoint payloads are author-visible state and inherit M-B-04's raw-content
warning; cached deltas are trusted state — the Redis key prefix is configuration, the key includes
the graph fingerprint, and no cross-graph key collision is possible by construction (documented as
a backend concern, PRD §5). R-23-01 (a hanging `EdgeConditionEvaluator`) **stays accepted**: the
per-attempt timeouts wrap node execution, not edge evaluation — re-listed in this phase's security
section rather than silently claimed closed.

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

**Research-informed recommendations for the discretion list** (see `## Don't Hand-Roll` and
`## Code Examples` below for the reasoning):

| Discretion item | Recommendation | Why |
|---|---|---|
| Status-carrying variant name | `ProviderError { provider, status, message }` | Matches D-03's own working name; avoids a second name (`HttpStatus`) that duplicates what the field already says |
| `EngineRegistries` naming | Keep exactly `EngineRegistries { edge_evaluators, retry_predicates, error_handlers }` as CONTEXT.md names it | Already the name used consistently across D-13/D-30's cross-references; renaming would create a mismatch between this doc and the plan |
| Jitter RNG source | `rand::rng()` (thread-local) inside the retry loop, not a struct-held `SmallRng` | No existing struct in `engine/` holds an RNG field today; a function-local `rand::rng()` call matches the zero-shared-mutable-state discipline X-05 asks for and needs no `Send`/`Sync` bookkeeping |
| `HeartbeatHandle` internals | `Arc<AtomicU64>` storing the last-heartbeat `Instant`'s `as_nanos` (or `Arc<tokio::sync::watch::Sender<Instant>>` if the idle-timer needs to `await` a change rather than poll) | `watch` gives the idle-timer a natural `changed()` await instead of a poll loop; recommend `watch` unless the planner finds a poll-based idle-timer simpler to reason about under `tokio::time::pause` |
| `BATTLEFIELD_SCHEMA_VERSION` bump | **Do not bump.** All Phase 25 additions are `#[serde(default)]` (`NodeExecutionRecord.attempts`/`cache_hit`, `WaypointStatus::Failed.node_error`, `FieldSpec.cache`) — the exact shape of the Phase 22.1/24 precedent that did NOT bump the version for `visit_counts`/`frontier`/`fork_of` | Confirmed `BATTLEFIELD_SCHEMA_VERSION = "1.0.0"` unchanged since at least Phase 22.1; bumping it would force every `BattlefieldSchema.schema_version` compatibility check to be revisited for a purely additive change, which X-04 does not require |
| InMemoryNodeCache in battalion tests | In-crate `RecordingNodeCache` in `test_support.rs` | `paladin-battalion` has no existing dev-dependency on `paladin-storage` (checked: not present in `crates/paladin-battalion/Cargo.toml`); adding one only for tests is a heavier dependency-graph change than a ~20-line in-crate `HashMap`-backed recording mock following the existing `RecordingWaypointStore`/`RecordingTraceSink` pattern in the same file |
| Fallback hop log level | `tracing::warn!` | A fallback hop is an operationally notable event (a provider degraded); `info!` would bury it under normal request-volume logging in production, and every other cross-provider degradation path in the codebase (`circuit_breaker.rs`) logs at `warn!` |
| `PaladinResult` full-literal migration owner | The wave-2 plan (`FallbackLlmAdapter` + `served_by`, D-33) | It is the plan that adds the `served_by` field and therefore the one that makes every full-literal site stop compiling; deferring the ~20-site mechanical fix to a later wave would leave wave 2 red until it lands |

</user_constraints>

<deferred>
## Deferred Ideas (OUT OF SCOPE)

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

</deferred>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Error transience classification | `paladin-core` (value type + `PaladinError`) | `paladin-ports`/`paladin-llm` (`LlmError`, adapters) | `Transience` is a core value type shared across the port boundary (X-01); classification logic on each error enum lives with that enum |
| Structured `NodeError` propagation | `paladin-core` (type) | `paladin-battalion` (construction, propagation through engine) | The type is a persisted domain value (X-04); the engine is the only place that observes attempt failures and builds it |
| Aegis retry/timeout/handler/cache policy | `paladin-battalion` (`engine/`) | `paladin-core` (`aegis` module — policy value types) | Policy *values* are core data (serde, no engine dependency); policy *execution* (the retry loop, deadline math, handler dispatch) is engine-only — it needs `tokio`, the superstep snapshot, and the Vanguard |
| LLM provider fallback | `paladin-llm` | `paladin-ports` (`LlmPort` trait, `AllProvidersFailed` variant) | The adapter composes existing `LlmPort` implementors; it is infrastructure, not domain, so it cannot live in `paladin-core` |
| Node result cache | `paladin-storage` (adapters) | `paladin-ports` (`NodeCachePort` trait), `paladin-core` (`CachedDelta` value type) | Adapters touch Redis/memory (infrastructure); the port is the abstraction the engine depends on; the cached value shape is a core persisted type (X-04) |
| Cache key computation | `paladin-battalion` (engine, at lookup time) | `paladin-core` (`CacheKeySpec` policy value) | The key needs the resolved input/Battlefield snapshot and the Paladin config, both engine-visible at execution time; the *policy* choosing what to hash is a plain value |
| Config surface (`NodeCacheConfig`, `run_timeout_secs`) | `src/config/` (application layer) | — | Mirrors every other `X-09` config struct (`WaypointStoreConfig`, `CitadelConfig`); no engine-level config exists today |

## Standard Stack

This phase adds **zero new external dependencies**. Every crate it needs is already present in the
workspace at the version this phase needs.

### Core (already-present, reused)

| Library | Version (verified in tree) | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `thiserror` | workspace-pinned (already used by every error enum touched) | `NodeError`, `Transience`-adjacent error types, `LlmError::ProviderError` | House convention for every domain error type in this repo; no alternative considered |
| `rand` | `0.8` `[VERIFIED: workspace Cargo.toml + crates/paladin-battalion/Cargo.toml, npm-equivalent check via cargo metadata]` — **already a direct `paladin-battalion` dependency**, contradicting D-30's "if not already" framing | Jitter for backoff (D-15) | Already the workspace-standard RNG crate (also used by `paladin-llm`, optionally) |
| `tokio` (`time`, `sync`, `select!`) | workspace-pinned, already a `paladin-battalion` dependency | Pausable backoff sleeps, `CancellationToken` select, `HeartbeatHandle` | House async runtime; `tokio::time::pause`/`start_paused` is the correct built-in mechanism for FT-FR-04's deterministic-clock tests — no third-party fake-clock crate needed |
| `async-trait` | workspace-pinned, already used by `EdgeConditionEvaluator`, `NodeInterceptor`, `WaypointPort` | `ErrorHandler::handle`, `RetryPredicate`-as-trait-object (if trait-object shaped), `NodeCachePort` | House convention for every `Send + Sync` async port/registry trait in this repo |
| `redis` | `0.32.2`, feature set `["aio", "tokio-comp", "connection-manager", "script"]` `[VERIFIED: crates/paladin-storage/Cargo.toml:52]` — **already a `paladin-storage` optional dependency behind `redis-queue`** | `RedisNodeCache` adapter (D-27) | Reuse the exact `ConnectionManager` pattern already proven in `crates/paladin-storage/src/redis.rs`; no reason to introduce a second Redis client |
| `serde` / `serde_json` | workspace-pinned | Every new persisted type (`Transience`, `NodeError`, `Aegis` family, `CachedDelta`) | House convention (X-04) |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `chrono` | workspace-pinned (already used throughout `waypoint.rs`) | `CachedDelta.stored_at`/`expires_at`, `AttemptRecord.started_at` | Matches every other `DateTime<Utc>` field in the persisted-type family |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `tokio::time::pause` for deterministic backoff tests | A dedicated fake-clock crate (`mock_instant`, `fake-time`) | Not needed: `tokio::time::pause`/`start_paused` is already the correct, idiomatic tool for a `tokio`-native codebase and needs no new dependency; introducing a third-party fake clock would be an unjustified new dependency for a problem `tokio` already solves |
| `rand::rng()` per-call jitter | A struct-held `SmallRng` behind a `Mutex` | The per-call approach avoids a new shared-mutable-state field in an already-concurrent engine path (X-05 prefers `tokio::sync` primitives over ad hoc mutexes when avoidable) |
| `HeartbeatHandle` as `Arc<tokio::sync::watch::Sender<Instant>>` | A hand-rolled `Arc<Mutex<Instant>>` + `Notify` pair | `watch` already gives "last value + await-a-change" semantics for free; hand-rolling the same thing with `Mutex` + `Notify` duplicates a solved problem |
| `paladin-storage`'s `redis-cache` feature reusing the existing `redis` dependency | A separate cache-specific Redis client crate | The existing `ConnectionManager` pattern in `redis.rs` already handles reconnection/pooling correctly; a second client would double the dependency surface for no benefit |

**Installation:** None required — no `Cargo.toml` dependency-block change beyond the two new
feature-flag lines (`redis-cache = ["dep:redis"]` in `paladin-storage/Cargo.toml`, and its facade
passthrough in the root `Cargo.toml`, both reusing the dependency `paladin-storage` already
declares).

**Version verification performed:**
```
$ grep -n "^rand" Cargo.toml crates/paladin-battalion/Cargo.toml
Cargo.toml:30:rand = "0.8"
crates/paladin-battalion/Cargo.toml:27:rand          = "0.8"

$ grep -n "^redis" crates/paladin-storage/Cargo.toml
crates/paladin-storage/Cargo.toml:52:redis = { version = "0.32.2", features = [...], optional = true }
```
Both are already resolved in `Cargo.lock`; no version bump is needed for this phase's purposes.

## Package Legitimacy Audit

**Not applicable — no new external packages are introduced by this phase.** Every crate this
phase's design calls for (`rand`, `redis`, `tokio`, `thiserror`, `async-trait`, `serde`, `chrono`)
is already a resolved dependency of the workspace, most of them already dependencies of the exact
crate (`paladin-battalion`, `paladin-storage`) this phase touches. The Package Legitimacy Gate
protocol (registry/postinstall-script verification) was checked against this fact directly in the
tree (`grep` against `Cargo.toml` files, not against training-data package names) rather than
skipped — there is simply nothing new to audit.

| Package | Registry | Disposition |
|---------|----------|-------------|
| *(none — no new packages)* | — | N/A |

**Packages removed due to `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** none.

## Architecture Patterns

### System Architecture Diagram

```
                         ┌─────────────────────────────────────────┐
                         │        WarEngine::run / resume            │
                         └───────────────────┬───────────────────────┘
                                              │ per superstep
                                              ▼
                         ┌─────────────────────────────────────────┐
                         │   compute_next_vanguard (graph.rs)        │
                         │   -- Route{to} targets pre-seeded into    │
                         │      the eligible-set worklist (D-21)     │
                         └───────────────────┬───────────────────────┘
                                              │ ready nodes
                                              ▼
                 ┌────────────────────────────────────────────────────┐
                 │        tokio::spawn per node (superstep.rs)          │
                 │  ┌──────────────────────────────────────────────┐   │
                 │  │   AEGIS RETRY LOOP  (NEW, wraps everything    │   │
                 │  │   below -- "outside the interceptor chain")   │   │
                 │  │                                                │   │
                 │  │  attempt = 1..max_attempts:                    │   │
                 │  │   ┌────────────────────────────────────────┐  │   │
                 │  │   │ 0. CACHE LOOKUP (NEW, attempt 1 only)    │  │   │
                 │  │   │    NodeCachePort::get(key) -- hit skips  │  │   │
                 │  │   │    everything below, records cache_hit   │  │   │
                 │  │   └───────────────┬────────────────────────┘  │   │
                 │  │                   │ miss                       │   │
                 │  │                   ▼                            │   │
                 │  │   ┌────────────────────────────────────────┐  │   │
                 │  │   │ 1. TraceEvent::NodeStarted{attempt}       │  │   │
                 │  │   │ 2. interceptor `before` chain (existing)  │  │   │
                 │  │   │ 3. execute_vanguard_node (existing)       │  │   │
                 │  │   │      -- Paladin: PaladinPort::            │  │   │
                 │  │   │         execute_observed(&heartbeat) (NEW)│  │   │
                 │  │   │      -- races run_timeout/idle_timeout    │  │   │
                 │  │   │         (NEW, min() against engine budget)│  │   │
                 │  │   │ 4. interceptor `after` chain (existing)   │  │   │
                 │  │   │ 5. TraceEvent::NodeFinished{attempt,      │  │   │
                 │  │   │    cache_hit} (NEW fields)                │  │   │
                 │  │   └───────────────┬────────────────────────┘  │   │
                 │  │                   │ Err(NodeError)             │   │
                 │  │                   ▼                            │   │
                 │  │   ┌────────────────────────────────────────┐  │   │
                 │  │   │ RetryPredicate::allows(transience)? (NEW) │  │   │
                 │  │   │  yes -> discard delta, append AttemptRecord│ │   │
                 │  │   │         backoff sleep (select! vs cancel) │  │   │
                 │  │   │         -> loop                            │  │   │
                 │  │   │  no  -> exit loop with NodeError           │  │   │
                 │  │   └───────────────┬────────────────────────┘  │   │
                 │  └───────────────────┼────────────────────────────┘   │
                 │                      │ exhausted / non-retryable       │
                 │                      ▼                                 │
                 │   ┌──────────────────────────────────────────────┐    │
                 │   │  ERROR HANDLER DISPATCH (NEW)                  │    │
                 │   │   Route{to,error_field}  -> vanguard[to] +      │    │
                 │   │                             error_field delta   │    │
                 │   │   Absorb{fallback_delta} -> merge, Failed rec,  │    │
                 │   │                             static edges fire   │    │
                 │   │   Custom(name)  -> ErrorHandler::handle ->      │    │
                 │   │                    Directive (may Parley)       │    │
                 │   │   none          -> Failed Waypoint,             │    │
                 │   │                    RunOutcome::Failed           │    │
                 │   └──────────────────────────────────────────────┘    │
                 │           success path: successful delta only          │
                 │           merges into the superstep's merge set        │
                 │           (existing merge machinery, unchanged)         │
                 └────────────────────────────────────────────────────────┘
                                              │
                                              ▼
                         ┌─────────────────────────────────────────┐
                         │   persist_waypoint (existing, unchanged   │
                         │   call site; NEW additive fields on the   │
                         │   record/status types it serializes)      │
                         └─────────────────────────────────────────┘

  Separate, standalone (no engine dependency):

  ┌────────────────────────────┐        ┌──────────────────────────────┐
  │  FallbackLlmAdapter (NEW)   │        │  NodeCachePort adapters (NEW) │
  │  chain: Vec<Arc<dyn LlmPort>>│       │  InMemoryNodeCache            │
  │  hop on Transient|Unknown   │        │  RedisNodeCache               │
  │  short-circuit on Permanent │        │  (both share one contract     │
  │  streaming first-chunk rule │        │   test suite)                 │
  └────────────────────────────┘        └──────────────────────────────┘
```

### Recommended Project Structure

```
crates/paladin-core/src/platform/container/
├── transience.rs         # NEW: Transience enum + shared predicate helpers
├── node_error.rs         # NEW: NodeError, NodeErrorSource, TimeoutKind
├── aegis.rs              # NEW: Aegis, RetryPolicy, RetryPredicate, TimeoutPolicy,
│                         #      ErrorHandlerSpec, CachePolicy, CacheKeySpec
├── node_cache.rs         # NEW: CachedDelta value type (X-04, schema_version)
├── paladin_error.rs      # EXTEND: LlmFailure variant, transience(), #[non_exhaustive]
├── battalion/mod.rs      # EXTEND: BattalionError::Node(NodeError), #[non_exhaustive]
├── waypoint.rs           # EXTEND: NodeExecutionRecord.attempts/cache_hit,
│                         #         WaypointStatus::Failed.node_error
├── battlefield.rs        # EXTEND: FieldSpec.cache: CacheMarker
└── execution_result.rs   # EXTEND: PaladinResult.served_by

crates/paladin-ports/src/output/
├── llm_port.rs           # EXTEND: LlmError::ProviderError/AllProvidersFailed,
│                         #         transience(), #[non_exhaustive]
├── node_cache_port.rs    # NEW: NodeCachePort trait
├── paladin_port.rs       # EXTEND: execute_observed default method
└── trace_sink_port.rs    # EXTEND: NodeStarted/NodeFinished.attempt, .cache_hit,
                          #         FallbackHop variant

crates/paladin-llm/src/
├── fallback.rs           # NEW: FallbackLlmAdapter
├── http_status.rs        # NEW (name at Claude's discretion): map_http_status() helper
└── {nine adapters}/adapter.rs  # EXTEND: non-2xx arms route through the shared helper

crates/paladin-battalion/src/engine/
├── aegis.rs               # NEW: retry math (backoff/jitter), predicate trait + registry
├── registries.rs          # NEW: EngineRegistries bundle
├── superstep.rs           # EXTEND: retry loop around the dispatch closure,
│                          #         cache lookup/store, handler dispatch, Route placement
├── graph.rs               # EXTEND: aegis sidecar map, validate() matrix, fingerprint v5
├── mod.rs                 # EXTEND: with_retry_predicate/with_error_handler/with_node_cache,
│                          #         RunTimeoutExceeded, NodeFailed
├── node.rs                # EXTEND: StateNodeError rename, NodeContext.attempt/heartbeat
└── test_support.rs        # EXTEND: RecordingNodeCache (if in-crate mock chosen)

crates/paladin-storage/src/
└── node_cache/
    ├── mod.rs
    ├── in_memory.rs        # NEW: TTL-evicting HashMap cache, ungated
    ├── redis.rs            # NEW: mirrors redis.rs's ConnectionManager pattern
    └── contract_tests.rs   # NEW: shared hit/miss/TTL/invalidate/overwrite suite

src/config/
└── node_cache.rs           # NEW: NodeCacheConfig mirroring WaypointStoreConfig
```

### Pattern 1: Fail-closed registry (copy verbatim from `edge_evaluator.rs`)

**What:** A `HashMap<String, Arc<dyn Trait>>` registry with `register`/`get`, validated at graph
build time — every `Custom(name)` reference must resolve or the whole graph fails validation
before any node executes, listing every offender at once.
**When to use:** `RetryPredicate::Custom`, `ErrorHandlerSpec::Custom` (D-13) — this is the exact
mechanism CF-01 (BUG-01) already established and D-13 explicitly reuses.
**Example (existing code, the pattern to replicate):**
```rust
// Source: crates/paladin-battalion/src/edge_evaluator.rs:63-135 (already in tree)
#[async_trait]
pub trait EdgeConditionEvaluator: Send + Sync {
    async fn evaluate(
        &self,
        output: &str,
        ctx: &EdgeContext<'_>,
    ) -> Result<bool, EdgeEvaluatorError>;
}

#[derive(Default, Clone)]
pub struct EdgeEvaluatorRegistry {
    inner: HashMap<String, Arc<dyn EdgeConditionEvaluator>>,
}

impl EdgeEvaluatorRegistry {
    pub fn register(&mut self, name: impl Into<String>, evaluator: Arc<dyn EdgeConditionEvaluator>) {
        self.inner.insert(name.into(), evaluator);
    }
    pub fn get(&self, name: &str) -> Option<&Arc<dyn EdgeConditionEvaluator>> {
        self.inner.get(name)
    }
}
```
`RetryPredicateRegistry` and `ErrorHandlerRegistry` should be byte-for-byte this shape (rename
`EdgeConditionEvaluator`→`RetryPredicate`/`ErrorHandler`, `EdgeEvaluatorError`→ the handler's own
error type, `evaluate`→`allows`/`handle`).

### Pattern 2: Sidecar map on `WarGraph`, not a `NodeSpec` field

**What:** Per-node optional annotations stored as `HashMap<NodeId, T>` beside `WarGraph`'s existing
`defer_flags` / `dynamic_targets` / `worker_templates` sets, with a chainable `&mut self` builder
method, rather than reshaping the `NodeSpec` enum's variants.
**When to use:** `Aegis` attachment (D-10) — `NodeSpec::Function(Arc<dyn StateNode>)` is a tuple
variant; adding a field to every variant would force every constructor and match arm in the tree to
change, whereas a sidecar map is purely additive.
**Example (existing code, the pattern to replicate):**
```rust
// Source: crates/paladin-battalion/src/engine/graph.rs:448-472 (already in tree)
pub struct WarGraph {
    // ...
    dynamic_targets: HashSet<NodeId>,
    worker_templates: HashSet<NodeId>,
    // NEW, following the identical shape:
    // aegis: HashMap<NodeId, Aegis>,
    // default_aegis: Option<Aegis>,
}

impl WarGraph {
    pub fn mark_dynamic_target(&mut self, id: NodeId) -> &mut Self {
        self.dynamic_targets.insert(id);
        self
    }
    // NEW, following the identical shape:
    // pub fn set_aegis(&mut self, id: NodeId, aegis: Aegis) -> &mut Self { ... }
    // pub fn with_default_aegis(&mut self, aegis: Aegis) -> &mut Self { ... }
}
```

### Pattern 3: Aegis wraps OUTSIDE the interceptor chain (explicit, already documented in-tree)

**What:** The retry/timeout loop is a wrapper *around* the existing "trace → interceptor before →
execute → interceptor after → trace" sequence, never a participant inside it.
**When to use:** Always, for FT-02/FT-03 — this is not a discretionary choice, it is a documented
contract the codebase already states.
**Example:**
```rust
// Source: crates/paladin-battalion/src/engine/hooks.rs:208-212 (already in tree, rustdoc)
/// # Doc 04's Aegis wraps OUTSIDE this chain
///
/// Per-node fault tolerance (retry, timeout, typed error handlers, model
/// fallback) is a WRAPPER around a node's whole interceptor-wrapped
/// execution, not a participant inside this chain. Nesting Aegis's policy
/// INSIDE the interceptor chain would mean an interceptor's `Skip`/`Fail`
/// decision could itself be retried as if it were a node's own transient
/// failure, which is never the intended semantics...
```

### Pattern 4: Additive `#[serde(default)]` fields on persisted types, no schema-version bump

**What:** New optional fields on already-persisted, `schema_version`-carrying types are added with
`#[serde(default)]` and the container's schema version is left unchanged, because the change is
purely additive and old data still deserializes.
**When to use:** `NodeExecutionRecord.attempts`/`cache_hit`, `WaypointStatus::Failed.node_error`,
`FieldSpec.cache` (D-08, D-16, D-29) — the exact precedent Phase 22.1/24 already set for
`visit_counts`, `frontier`, `fork_of`.
**Example:**
```rust
// Source: crates/paladin-core/src/platform/container/execution_result.rs:59-60,67-68
// (already in tree -- the precedent this phase's `served_by` field follows)
#[serde(default, skip_serializing_if = "Option::is_none")]
pub plan: Option<TaskPlan>,

#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub handoff_history: Vec<HandoffRecord>,
```

### Anti-Patterns to Avoid

- **Parsing an HTTP status or provider name out of an error's `Display` string to classify
  transience.** FT-FR-01 explicitly forbids this ("do not parse strings to classify"); every status
  must arrive as a typed field on a variant (D-03's `ProviderError { status: u16, .. }`).
- **Reshaping `NodeSpec`'s tuple variants to carry an `aegis` field.** D-10 rejected this
  explicitly; use the sidecar map instead.
- **Retrying inside the `NodeInterceptor` `before`/`after` chain.** `hooks.rs` explicitly documents
  why this is wrong (Pattern 3 above) — an interceptor's `Skip`/`Fail` is deliberate policy, not a
  transient fault.
- **Adding a required method to `PaladinPort`, `LlmPort`, or any other pre-existing public trait.**
  X-10.4 forbids it outright; every new capability is a *defaulted* method (`execute_observed`) or a
  new trait (`NodeCachePort`, `RetryPredicate`, `ErrorHandler`).
- **Marking `PaladinResult` `#[non_exhaustive]` to add `served_by`.** D-26 explicitly rejects this
  (breaks all 66 `..Default::default()` sites); use the additive-field-with-`Default`-preserved
  path instead.
- **Bumping `BATTLEFIELD_SCHEMA_VERSION` for this phase's additive fields.** See the discretion
  table above — none of this phase's persisted-field additions require it.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Deterministic backoff timing tests | A custom fake-clock/time-mocking shim | `tokio::time::pause` + `tokio::time::advance`/`sleep` under `#[tokio::test(start_paused = true)]` | `tokio` already ships exactly this; the codebase has zero existing fake-clock crates and none should be introduced |
| Exponential backoff + jitter math | A backoff crate (`backoff`, `exponential-backoff`) | Hand-write the formula per FT-FR-04's exact spec (`min(initial × factor^(n−2), max) + jitter`) | The PRD mandates an *exact*, test-asserted sequence; a generic backoff crate's internal jitter/rounding behavior is not guaranteed to match the required exact values, and pulling in a dependency to implement four lines of arithmetic that must be asserted byte-for-byte anyway buys nothing |
| HTTP status → error-variant mapping | Nine independent per-adapter mapping blocks (the status quo) | One shared `map_http_status()` helper (D-03) | Avoids nine copies of classification logic drifting out of sync; this is the phase's own explicit FR, not a "don't-hand-roll" library substitution — the point is *don't hand-roll it nine times*, hand-roll it once |
| Cache TTL eviction (InMemory backend) | A generic TTL-cache crate (`moka`, `ttl_cache`) | A small hand-rolled `HashMap<Key, (CachedDelta, Instant)>` with lazy expiry-on-read (mirroring the existing `redis.rs`/`waypoint/in_memory.rs` house style for InMemory adapters) | The existing `InMemory*` adapters in this codebase (`in_memory` Waypoint store, in-memory Garrison) are all hand-rolled `HashMap`s with no eviction library; introducing one library just for this one adapter breaks house consistency for a data structure simple enough not to need one |
| Redis connection management | A hand-rolled reconnect/retry wrapper around `redis::Client` | The existing `ConnectionManager` pattern in `crates/paladin-storage/src/redis.rs` | Already solved, already tested, already the house pattern — copy it, don't reinvent it |
| Registered custom-name dispatch (predicates, handlers) | Two new bespoke registry types with their own validation error shapes | Copy `EdgeEvaluatorRegistry`'s exact shape (Pattern 1) | D-13 explicitly calls for reusing CF-01's mechanism; a divergent shape would be an unforced inconsistency the plan-checker would likely flag |

**Key insight:** almost nothing in this phase should be "don't hand-roll, use a library" — this is
*internal engine* work where the PRD itself specifies exact algorithms (backoff formula, key
composition, nesting semantics) that a generic library cannot be trusted to reproduce byte-for-byte
under test. The real "don't hand-roll" risk here is *hand-rolling the same pattern nine different
ways across nine LLM adapters or three new registries* — the mitigation is copying one
already-proven in-tree pattern consistently, not reaching for an external crate.

## Common Pitfalls

### Pitfall 1: Retrying an interceptor decision as if it were a node fault
**What goes wrong:** If the retry loop is placed inside the interceptor chain (wrapping only
`execute_vanguard_node`), an interceptor's `Skip`/`Fail` `InterceptDecision` could be
misinterpreted as a transient node failure and retried, silently changing interceptor semantics.
**Why it happens:** The natural first instinct when adding a retry loop is to wrap the innermost
call, not the whole per-attempt sequence.
**How to avoid:** Wrap the *entire* per-node dispatch closure (`superstep.rs:1515-1595`: trace →
before → execute → after → trace) in the retry loop, per `hooks.rs:208-212`'s explicit contract.
**Warning signs:** A test where a `Skip` decision gets retried, or where `before`/`after` run more
than once per attempt for reasons other than a genuine node-execution failure.

### Pitfall 2: Treating `#[non_exhaustive]` as free
**What goes wrong:** Marking `PaladinError`/`LlmError`/`BattalionError` `#[non_exhaustive]` without
adding `_` arms to every in-tree exhaustive `match` breaks the build immediately; forgetting the
`.cargo/semver-checks-allowlist.toml` entry + per-crate `[package.metadata.cargo_semver_checks.lints]`
suppression in the *same commit* makes the `semver` CI job fail on a change that is legitimately
deliberate-breaking.
**Why it happens:** `#[non_exhaustive]` and its downstream match-arm fallout are easy to apply to
the enum definition and easy to forget everywhere the enum is matched exhaustively across six-plus
crates.
**How to avoid:** Grep for every exhaustive `match` on the three enums *before* adding the
attribute, add `_` arms in the same commit, and add the allowlist entry + lint suppression in that
same commit (D-04, D-30). The allowlist file's own header documents the set-equality check the
`semver` CI job runs in both directions — use it as a checklist.
**Warning signs:** `cargo build` failures citing "non-exhaustive patterns" after adding the
attribute; a `semver` CI job failure citing an allowlist/§9.2 mismatch.

### Pitfall 3: Forgetting that `PaladinResult`'s field addition breaks full struct literals, not FRU sites
**What goes wrong:** Assuming "additive field + `#[serde(default)]`" alone is enough and being
surprised when ~21 full-literal `PaladinResult { .. }` sites (concentrated in rustdoc examples,
`execution_result.rs`, two test files, and one real call site in `paladin_execution_service.rs`,
per this research's direct count) fail to compile.
**Why it happens:** Rust struct-literal construction breaks on any new field regardless of
`#[serde(default)]`, which only affects deserialization — a purely Rust-compiler concern orthogonal
to the serde concern.
**How to avoid:** Budget a dedicated pass (D-33 recommends the wave-2 plan, the one adding the
field) that adds `..Default::default()` or the new field to every full-literal site found by
`grep -rn "PaladinResult *{"`. Re-run the same count after the change to confirm zero full literals
remain that aren't intentionally exhaustive (e.g., the `Default`/`new()` impls in
`execution_result.rs` itself, which construct the type differently).
**Warning signs:** A wave-2 `cargo build`/`cargo test` failure listing "missing field `served_by`"
across multiple crates/examples/tests simultaneously.

### Pitfall 4: Assuming the fingerprint version bump is optional
**What goes wrong:** Adding `on_error`/`cache` to `WarGraph::fingerprint()`'s hash input without
bumping `GRAPH_FINGERPRINT_VERSION` (`v4`→`v5`) and re-pinning the golden hex tests
(`fingerprint_golden_hex_v4`, `fingerprint_version_is_v4` at `graph.rs:2384`/`2564`) leaves a
silent hash-input change that the existing golden tests will catch as a *failure*, not silently
pass — but only if the version constant and its own test are updated together.
**Why it happens:** The fingerprint discipline (D-11, following Phase 22.1's D-15…D-19) requires
every hash-input change to be paired with a version bump; skipping the bump makes a v0.9-vintage
Waypoint's stored fingerprint silently stop matching a v0.10 graph with retry/timeout tuning changes
— exactly the `GraphMismatch` failure D-11 is designed to *prevent* for non-hashed fields, so
getting the hashed/non-hashed split wrong in either direction breaks a real invariant.
**How to avoid:** Follow D-11 precisely: `on_error` and `cache` ARE hashed (routing/merge-affecting);
`retry` and `timeout` are NOT hashed (tuning-only, like every `EngineLimits` field). Bump the
version constant and update the golden tests in the same commit as the fingerprint function change.
**Warning signs:** `fingerprint_version_is_v4`-style tests failing after an unrelated-looking
change; a `resume` integration test failing `GraphMismatch` after only a retry-policy tuning edit
(this specific failure mode is exactly what D-11 exists to prevent).

### Pitfall 5: Introducing a second `RetryPolicy` name collision
**What goes wrong:** The legacy `paladin_core::platform::container::battalion::RetryPolicy {
max_attempts, base_delay, .. }` (v0.9, used by `ErrorStrategy::RetryThenContinue`) already exists.
Re-exporting both `RetryPolicy` types from the core prelude, or accidentally importing the wrong one
in a new module, produces a confusing compile error or (worse) silently wrong behavior if type
inference picks the legacy one.
**Why it happens:** Both types plausibly live under `paladin_core::platform::container`, and both
are named `RetryPolicy`.
**How to avoid:** D-09 is explicit: the core prelude re-exports `Aegis` and `Transience` only, never
either `RetryPolicy`. Always fully qualify or alias when both are in scope in the same file (e.g.,
integration tests exercising both the legacy Battalion path and the new engine path).
**Warning signs:** A `use` statement pulling in the wrong `RetryPolicy` via a glob import; a test
asserting legacy `RetryThenContinue` semantics that silently starts using the new type's defaults
after a prelude change.

### Pitfall 6: Cache key omitting the graph fingerprint
**What goes wrong:** If `CacheKeySpec`'s hash omits `graph_fingerprint`, a cached delta from one
graph version (or even an entirely different graph reusing the same node id) can be served to a
structurally different graph, producing state-shape mismatches or silently wrong cached data across
otherwise-unrelated workflows.
**Why it happens:** It's tempting to key only on `(node_id, input, paladin_config)` since those feel
like "the inputs that matter," but node identity is only unique *within* a graph.
**How to avoid:** D-28 mandates including `graph_fingerprint` in the key precisely so
`invalidate(prefix)` can target one graph or one node, and so a graph edit invalidates naturally —
implement the key exactly as specified.
**Warning signs:** A cache-hit test using two different `WarGraph` instances sharing a node id that
unexpectedly returns the same cached delta.

### Pitfall 7: Retry-loop delay burns shutdown grace
**What goes wrong:** If the backoff `sleep` is not raced against the run's `CancellationToken` via
`select!`, a node paused mid-backoff at SIGTERM will not be interrupted promptly, consuming
`shutdown_grace` time that should be reserved for finishing genuinely in-flight work.
**Why it happens:** A plain `tokio::time::sleep(delay).await` is the naive first implementation and
compiles fine; the cancellation-awareness only becomes visible under a graceful-shutdown test.
**How to avoid:** D-15 mandates `select!` between the sleep and the cancellation token, recording
`Skipped { reason: "shutdown" }` and re-listing in the Halted vanguard on cancellation — exactly the
HITL-04 D-19/D-20 mechanism already in place for in-flight execution.
**Warning signs:** A graceful-shutdown integration test that hangs or exceeds `shutdown_grace` when
a node happens to be in backoff at the moment SIGTERM arrives.

## Code Examples

### Existing `PaladinPort` trait signature the defaulted `execute_observed` method must extend

```rust
// Source: crates/paladin-ports/src/output/paladin_port.rs:631-810 (already in tree)
#[async_trait]
pub trait PaladinPort: Send + Sync {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError>;

    async fn execute_stream(
        &self,
        paladin: &Paladin,
        input: &str,
    ) -> Result<mpsc::Receiver<Result<PaladinStreamChunk, PaladinError>>, PaladinError>;

    fn validate(&self, paladin: &Paladin) -> Result<(), PaladinError>;

    // NEW (D-19), X-10.4-compliant default method:
    // async fn execute_observed(
    //     &self,
    //     paladin: &Paladin,
    //     input: &str,
    //     heartbeat: &HeartbeatHandle,
    // ) -> Result<PaladinResult, PaladinError> {
    //     // correct default: claims no progress
    //     self.execute(paladin, input).await
    // }
}
```

### Existing `PaladinError`/`LlmError` variant inventories (confirmed exhaustively via `awk`/`grep`, not training-data recall)

```rust
// Source: crates/paladin-core/src/platform/container/paladin_error.rs:19-59 (already in tree)
pub enum PaladinError {
    ConfigurationError(String),
    ExecutionError(String),
    LlmError(String),
    Timeout(u64),
    StopWordDetected(String),
    CircuitBreakerOpen,
    MaxRetriesExceeded(u32),
    GarrisonError(#[from] GarrisonError),
    GarrisonRequired,
    ArsenalError(#[from] ArsenalError),
    // NEW (D-02): LlmFailure { transience, status, provider, message }
}

// Source: crates/paladin-ports/src/output/llm_port.rs:291-457 (already in tree)
pub enum LlmError {
    NetworkError(String),
    AuthenticationError(String),
    InvalidPrompt(String),
    RateLimitExceeded,
    UsageLimitExceeded { provider: String, regain_hint: String }, // approximate shape
    ModelNotAvailable(String),
    TokenLimitExceeded,
    EmptyCompletion(String),
    ProcessingError(String),
    Timeout(String),
    // NEW (D-03): ProviderError { provider, status, message }
    // NEW (D-24): AllProvidersFailed { attempts, last }
}
```

### Existing reachability-worklist insertion point `Route { to }` targets must seed into

```rust
// Source: crates/paladin-battalion/src/engine/graph.rs:1102-1154 (already in tree, `validate_eligible_set`)
fn validate_eligible_set(&self) -> Result<(), EngineError> {
    // ...
    let mut worklist: Vec<NodeId> = Vec::new();
    for id in /* entry nodes UNION dynamic_targets */ {
        worklist.push(id.clone());
    }
    // NEW (D-21): also seed every `Route { to }` target declared by any
    // node's `on_error: Some(ErrorHandlerSpec::Route { to, .. })` here --
    // no `mark_dynamic_target` call needed, this worklist is the one
    // documented insertion point (22-15).
    while let Some(current) = worklist.pop() {
        for edge in /* outgoing edges of current */ {
            worklist.push(edge.to.clone());
        }
    }
    // ...
    Ok(())
}
```

### Existing Redis adapter pattern the `RedisNodeCache` adapter must mirror

```rust
// Source: crates/paladin-storage/src/redis.rs:1-150 (already in tree)
use redis::{AsyncCommands, Client, aio::ConnectionManager};

pub struct RedisQueueAdapter {
    conn: Arc<RwLock<ConnectionManager>>,
    // ...
}

impl RedisQueueAdapter {
    pub async fn new(/* config */) -> Result<Self, QueueError> {
        let client = Client::open(/* url */)?;
        let conn = ConnectionManager::new(client.clone()).await.map_err(|e| {
            // typed error mapping, no unwrap
        })?;
        Ok(Self { conn: Arc::new(RwLock::new(conn)), /* ... */ })
    }
}
// RedisNodeCache follows this exact shape: Client::open -> ConnectionManager::new
// -> Arc<RwLock<ConnectionManager>>, using SET with EX for put(key, delta, ttl),
// GET for get(key), and SCAN + DEL (or a Lua script via the "script" feature
// already enabled) for invalidate(prefix).
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `PaladinError::LlmError(String)` — every LLM failure erased to a display string | `PaladinError::LlmFailure { transience, status, provider, message }` — structured, classifiable by value | This phase (FT-01) | Downstream code (circuit breaker, retry logic) can branch on `transience()` instead of guessing from a string; legacy variant retained unconstructed for compatibility |
| Battalion-level `ErrorStrategy` (`FailFast`/`ContinueOnError`/`RetryThenContinue`) as the only fault-tolerance granularity | Per-node `Aegis` policy bundle, orthogonal to and coexisting with the legacy strategy (untouched, X-03) | This phase (FT-02…FT-06) | Fault tolerance moves from "whole Battalion" granularity to "this one node" granularity, without removing the legacy mechanism |
| `EngineLimits.run_timeout` declared but never enforced | Enforced, feeding `EngineError::RunTimeoutExceeded` down the same path as `RecursionLimitExceeded`/`NodeVisitLimitExceeded` | This phase (FT-03) | Closes a documented gap the config type and `MIGRATION.md` §9.5 have been promising since Phase 23 ("Doc 04/FT-03 owns timeout semantics") |
| A node failure collapsing into `BattalionError::PaladinError(String)`, destroying compensation-relevant information | `BattalionError::Node(NodeError)` — structured, carries transience + source + attempt | This phase (FT-01/FT-04) | Enables Saga-style compensation (`Route`/`Absorb`/`Custom`) that was previously impossible because the information needed to decide "should I compensate" didn't survive to the handler |
| Nine independent per-adapter HTTP-status-to-`LlmError` mapping blocks, several collapsing 5xx/408/unknown-4xx into a stringly `ProcessingError` | One shared `map_http_status()` helper applied to all nine adapters | This phase (FT-01, D-03) | RT-06 (Phase 26) becomes a verification pass over 3 already-correct paths instead of a rebuild of all nine |

**Deprecated/outdated:** nothing is marked `#[deprecated]` by this phase (per D-30's §9.7 note —
`MIGRATION.md` §9.7 stays empty this phase). The legacy `battalion::RetryPolicy` and
`ErrorStrategy` remain fully live, untouched, and undeprecated (X-03) — they are a parallel
mechanism, not a superseded one, this release.

## Assumptions Log

Every claim in this document was either (a) verified directly against the tree via `Read`/`Bash`
tool calls in this research session, or (b) copied verbatim from `25-CONTEXT.md`'s already-locked
decisions (which themselves cite file:line anchors this research independently re-verified). No
claim in this document rests on unverified training-data recall of an external library's behavior,
because **this phase introduces no new external dependency**. The table below lists the small
number of genuinely open/discretionary items where this research's own *recommendation* (not the
underlying fact) is a judgment call rather than a tree-verified fact.

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `watch`-based `HeartbeatHandle` is preferable to a poll-based `AtomicU64` timestamp | Claude's Discretion recommendations | Low — both are internal-only implementation choices with no public API difference; if the planner picks the poll-based approach instead, no FR is at risk, only implementation ergonomics |
| A2 | `tracing::warn!` (not `info!`) is the right log level for a fallback hop | Claude's Discretion recommendations | Very low — purely an operational-log-verbosity choice with no test-observable consequence beyond what the planner's own test asserts |
| A3 | The wave-2 plan (not a later wave) should own the ~21-site `PaladinResult` full-literal migration | Pitfall 3, Claude's Discretion recommendations | Low-medium — if deferred to a later wave, wave 2 (and every wave after it, transitively) will not compile until the migration lands; this is a build-ordering risk, not a correctness risk, and is easily caught by `cargo build` on the first affected wave |

**All FR-level behavioral claims (backoff formula, classification table, nesting semantics, cache
key composition, etc.) are `[CITED: .project/v0.10.0/04-fault-tolerance.md]` or
`[VERIFIED: tree grep/read]`, not `[ASSUMED]`** — PRD 04 is the FR-level source of truth per
CONTEXT.md's own framing, and this research independently re-verified every file:line anchor
CONTEXT.md cites against it.

## Open Questions

1. **Should D-12's Battalion-node `retry`/`cache` restriction be overturned at plan review?**
   - What we know: PRD 04 §2.1 states a blanket "`NodeSpec` gains `aegis`" with no node-kind
     carve-out; every §3 acceptance criterion (E2E-3, the backoff table, the compensation chain,
     the cache tests) exercises only Paladin/Function nodes, so D-12's narrowing loses no provable
     acceptance criterion.
   - What's unclear: whether the developer wants the child-thread-per-attempt design (listed under
     Deferred Ideas) pulled into this phase's scope rather than deferred.
   - Recommendation: plan as D-12 states (Paladin/Function get full Aegis; Battalion gets
     `timeout`/`on_error` only; Gate gets none), flagged ⚠ for a go/no-go check at plan review, per
     CONTEXT.md's own framing. Do not silently resolve this in the plan — surface it explicitly.

2. **Should D-26's `#[non_exhaustive]`-vs-FRU tradeoff be revisited if a future phase needs
   `PaladinResult` exhaustive-match safety?**
   - What we know: D-26 chose option (b) — keep `PaladinResult` constructible, not
     `#[non_exhaustive]`, and pay the one-time cost of migrating ~21 full-literal sites — because
     FRU (`..Default::default()`) is disallowed cross-crate on `#[non_exhaustive]` structs, and 70
     of 91 in-tree construction sites already rely on FRU.
   - What's unclear: whether a later phase (RT-05's `execute_structured<T>`, or OBS's trace
     enrichment) will want to add more fields to `PaladinResult` and hit the same tradeoff again,
     compounding the deliberate-breaking register entries.
   - Recommendation: no action needed this phase — flagged here only so the planner's `MIGRATION.md`
     §9.2 justification column (D-26 already drafts the exact text) is understood as a *pattern*
     future phases touching `PaladinResult` should expect to repeat, not a one-off exception.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All of Phase 25 | ✓ | `rustc 1.97.1` (dev container; MSRV gate pins 1.88 in CI) | — |
| `cargo-semver-checks` | D-04/D-30's semver gate | ✓ (locally installed) | `0.50.0`, matching `ci.yml`'s pinned `cargo-semver-checks@0.50.0` | — |
| `cargo-llvm-cov` | ADR-0006's 82% coverage floor | ✓ (locally installed) | `0.8.7` | — |
| Docker | Redis/Postgres Tier-2 contract-suite tiers (D-27's `redis-cache` adapter) | ✗ (confirmed absent in this devcontainer, matching STATE.md's Phase 23/24 close notes) | — | The InMemory tier and the contract-suite structure are fully provable locally; the Redis tier is provable only through CI's Docker-gated job (`docker-integration`/`postgres-integration` pattern in `.github/workflows/ci.yml`) — route Redis-tier evidence to CI, never claim it "passed locally" |
| `redis-cli` | Manual smoke-testing only, not required by any test | ✓ (binary present; no server running) | — | Not needed for automated tests — the contract suite talks to a real `redis-test` container only inside the Docker-gated CI job |

**Missing dependencies with no fallback:** none — Docker's absence has a documented, already-used
fallback (route to CI).

**Missing dependencies with fallback:** Docker (see above).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in), no external test framework; `#[tokio::test]` / `#[tokio::test(flavor = "multi_thread")]` for async, `#[tokio::test(start_paused = true)]` (or `tokio::time::pause()` inside a plain `#[tokio::test]`) for deterministic-clock tests |
| Config file | none — Rust's built-in test harness; no `pytest.ini`/`jest.config`-equivalent exists or is needed |
| Quick run command | `cargo test -p paladin-core -p paladin-ports -p paladin-battalion -p paladin-llm -p paladin-storage` (scoped to the crates this phase touches) |
| Full suite command | `cargo test` (workspace) for unit + existing integration tests; `make test-integration-docker` for the Redis-cache Tier-2 suite (Docker unavailable locally — CI-only, see Environment Availability) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FT-FR-01 | `transience()` table-driven per variant, both enums | unit | `cargo test -p paladin-core transience` / `cargo test -p paladin-ports transience` | ❌ new — `crates/paladin-core/src/platform/container/transience.rs` (Wave 0/1) |
| FT-FR-02 | `NodeError`/`NodeErrorSource` construction, `BattalionError::Node` | unit | `cargo test -p paladin-core node_error` | ❌ new — `node_error.rs` (Wave 1) |
| FT-FR-02a | X-10 register rows, `#[non_exhaustive]`, allowlist set-equality | integration (CI job) | `cargo semver-checks check-release --package paladin-core --baseline-version 0.9.0` (per `ci.yml`'s `semver` job) | ✅ job exists, rows are the new content (Wave 1/8) |
| FT-FR-03…07 | Attempt isolation, exact backoff sequence, predicate gating, retry × Muster, retry × Waypoint | unit + integration | `cargo test -p paladin-battalion retry` (paused-clock unit tests); `cargo test --test e2e_muster_defer_order_test` (integration) | ❌ new unit tests (Wave 4); existing integration file gets the seam replacement (D-31, Wave 6) |
| FT-FR-08…10 | `run_timeout`/`idle_timeout` distinguishing, nesting, `RunTimeoutExceeded` | unit + integration | `cargo test -p paladin-battalion timeout` | ❌ new — timeout unit tests (Wave 5); a new `RunTimeoutExceeded` integration test (Wave 5) |
| FT-FR-11…15 | `Route`/`Absorb`/`Custom` handlers, fail-closed, loop bound, E2E-3 | integration | `cargo test --test e2e_muster_defer_order_test`; new compensation-chain, loop-bound, and Parley-from-handler integration tests | ❌ new (Wave 6); registers under `Cargo.toml [[test]]` per the `e2e_crash_resume_test.rs` template |
| FT-FR-16, 17 | `FallbackLlmAdapter` chain, streaming first-chunk rule, `served_by` | unit | `cargo test -p paladin-llm fallback` | ❌ new — `fallback.rs` + tests (Wave 2) |
| FT-FR-18…20 | Cache hit/miss/TTL/invalidate/config-change contract suite, engine integration | unit (InMemory) + integration (Redis, CI-only) | `cargo test -p paladin-storage node_cache` (InMemory tier, local); Redis tier via `make test-integration-docker` (CI-only, Docker absent locally) | ❌ new — `node_cache/` module + contract suite (Wave 7) |
| §3.8 | X-05 multi-thread stress (Muster + per-task retry, exact counts, timeout guard) | integration | `cargo test --test e2e_muster_defer_order_test -- --test-threads=1` (or a dedicated new stress test file) following `src/application/services/orchestration/listener.rs`'s house pattern | ❌ new (Wave 6) |
| §3.9 | Versioning gate — semver, MSRV, coverage ≥ 82% | CI gate | `cargo semver-checks check-release ...`; `cargo +1.88 check --workspace --all-features` (MSRV job); `cargo llvm-cov --fail-under-lines 82` | ✅ all three jobs already exist in `ci.yml`; this phase's job is to keep them green, not create them (Wave 8) |

### Sampling Rate

- **Per task commit:** `cargo test -p <crate touched by the task>` (fast, scoped)
- **Per wave merge:** `cargo test` (full workspace) + `cargo clippy -- -D warnings` + `cargo fmt --check`
- **Phase gate:** Full suite green, `make security` (audit + deny), `cargo semver-checks` vs 0.9.0,
  MSRV job at 1.88, coverage ≥ 82% (ADR-0006) — all before `/gsd-verify-work`, matching D-33's wave
  8 exactly.

### Wave 0 Gaps

- [ ] `crates/paladin-core/src/platform/container/transience.rs` — new module, no existing test
      scaffolding; needs its own `#[cfg(test)] mod tests` block (house convention, per
      `rust.instructions.md`)
- [ ] A paused-clock test helper/idiom is **not yet established anywhere in this repo** — the first
      plan that writes a backoff test (Wave 4) should establish the idiom once
      (`#[tokio::test(start_paused = true)]` + `tokio::time::advance`) and every subsequent
      paused-clock test (idle timeout, kill-during-backoff) should copy it, rather than each
      re-deriving its own pattern
- [ ] `tests/helpers/mock_paladin_port.rs`'s `FaultyPaladinPort` needs the additive
      `fail_paladin_until_attempt(name, n)` method (D-31) before the E2E-3 seam replacement (Wave 6)
      can be written — sequence this before, not concurrent with, the seam-replacement plan
- [ ] Framework install: none — `cargo test`, `cargo-semver-checks`, and `cargo-llvm-cov` are all
      already installed/pinned; no new tool installation is required

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Not touched by this phase (no auth surface changes) |
| V3 Session Management | no | Not touched by this phase |
| V4 Access Control | no | Not touched by this phase |
| V5 Input Validation | yes | Typed `EngineError` validation errors for every new fail-closed check (unregistered `Custom` names, undeclared `set_aegis`/`Route{to}` targets, Battalion/Gate-node Aegis rejection, `error_field` schema-dispatch validation, `CachePolicy`-vs-`Deny`-marker validation) — all validated at graph-build time, before any node executes, following the existing CF-01 fail-closed pattern |
| V6 Cryptography | no | No new cryptographic primitive; Redis cache keys use a non-cryptographic hash (content-addressing, not a security boundary) — no change from the existing `WaypointStoreConfig`/fingerprint precedent |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Provider error message / API credential leaking into a `NodeError`, `ProviderError`, or `LlmFailure` message field | Information Disclosure | Redact-then-bound via `crates/paladin-llm/src/redaction.rs` **before** any excerpt is truncated (D-03, D-34; `security.instructions.md`'s explicit "response bodies are redacted before truncation" rule) — apply this at the single shared `map_http_status()` helper so all nine adapters get it uniformly, rather than per-adapter |
| A compensation handler's `Route`/`Absorb` writing attacker- or provider-controlled content into a Battlefield `error_field`, later rendered to a human or another LLM call | Tampering / Information Disclosure | `error_field`/Waypoint payloads are author-visible state and inherit the existing M-B-04 raw-content warning (D-34) — no new mitigation invented here, just carried forward and re-documented in this phase's guide |
| Handler-routed compensation loop (A fails → routes B → B routes A) spinning without bound | Denial of Service | `max_node_visits` bounds every handler-routed visit exactly as it bounds ordinary visits (D-21, FT-FR-15) — reuses `ENG-FR-03`'s existing typed-limit-error mechanism, no new bespoke loop guard |
| Cross-graph or cross-node cache-key collision serving stale/wrong `StateDelta` to an unrelated node | Tampering | Cache key includes `graph_fingerprint` by construction (D-28) — makes cross-graph collision impossible by design, not by runtime check; document this as the mitigation in the fault-tolerance guide (D-32) |
| A hanging `EdgeConditionEvaluator::evaluate` implementation blocking a superstep indefinitely | Denial of Service | **R-23-01 stays accepted, not newly mitigated by this phase** (D-34) — per-attempt timeouts wrap node execution, not edge evaluation; re-list this in this phase's own security write-up rather than silently implying it's closed by the new timeout machinery |
| A `NodeCachePort::put` failure (e.g., Redis unavailable) failing the run | Denial of Service (self-inflicted) | Cache is best-effort by construction (D-29): a `put` failure is logged and never fails the run; a `get` failure is treated as a miss, not an error — mitigates an operational dependency (Redis) from becoming a correctness/availability dependency |

**Carried-forward, not newly closed:** R-23-01 (hanging evaluator) remains an accepted risk this
phase does not address — state this plainly in the phase's own security documentation rather than
letting the new per-attempt timeout machinery read as though it covers edge evaluation too, which it
does not.

## Sources

### Primary (HIGH confidence — direct tree verification this session)
- `.planning/phases/25-node-level-fault-tolerance/25-CONTEXT.md` — 34 locked decisions, canonical
  refs, code-context anchors (re-verified, not merely trusted)
- `.project/v0.10.0/04-fault-tolerance.md` — FR-level source of truth (FT-FR-01…20, §3 acceptance
  criteria, §4 TDD ordering, §5 out of scope) — read in full
- `.project/v0.10.0/00-program-overview.md` — X-01…X-11 cross-cutting rules, §6 E2E-3, §9
  `MIGRATION.md` structure and pre-populated §9.2 rows — read in full
- `.planning/REQUIREMENTS.md` — FT-01…FT-06 capability clusters, traceability table
- `.planning/STATE.md` — Phase 23/24 close blockers/concerns (E2E-3 seam note, Docker
  unavailability, `FaultyPaladinPort` global-counter decision) — targeted sections read
- Direct tree reads/greps performed this session (not training-data recall): `paladin_error.rs`,
  `llm_port.rs`, `battalion/mod.rs`, `engine/{graph,mod,node,superstep,hooks,edge_evaluator}.rs`,
  `waypoint.rs`, `battlefield.rs`, `execution_result.rs`, `paladin_port.rs`,
  `crates/paladin-storage/src/redis.rs`, `crates/paladin-storage/Cargo.toml`,
  root `Cargo.toml`, `crates/paladin-battalion/Cargo.toml`,
  `.cargo/semver-checks-allowlist.toml`, `.github/workflows/ci.yml`, `MIGRATION.md`,
  `tests/helpers/mock_paladin_port.rs`, `tests/integration/e2e_muster_defer_order_test.rs`

### Secondary (MEDIUM confidence)
- None required — every claim traced to a primary source above.

### Tertiary (LOW confidence)
- None — this phase introduced no unverified web-search-only claims; no external library research
  was needed since no new dependency is added.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all versions confirmed via direct `Cargo.toml` reads
  and `cargo` tool version checks, not training-data recall
- Architecture: HIGH — every pattern cited exists verbatim in the tree today and was read directly
  in this session (not merely trusted from CONTEXT.md's citation)
- Pitfalls: HIGH — each pitfall is either an explicit in-tree rustdoc warning (Pattern 3), a
  measured count (Pitfall 3's 21-site literal audit), or a documented precedent this phase must
  follow consistently (Pitfalls 4-6, tied to D-11/D-09/D-28)
- Validation architecture: HIGH — test framework and CI job names/commands confirmed against
  `.github/workflows/ci.yml` and local tool versions, not assumed

**Research date:** 2026-09-05
**Valid until:** Effectively the length of Phase 25's own execution window — this is an
internal-codebase snapshot, not a claim about a third-party ecosystem that drifts over time. Re-run
the tree greps in this document (not a fresh literature search) if Phase 25 planning is resumed
more than a few weeks after this date, since the tree itself may have moved.
