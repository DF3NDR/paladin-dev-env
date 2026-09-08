# Phase 27: Platform API - Pattern Map

**Mapped:** 2026-09-08
**Files analyzed:** ~42 new/modified files across 5 crates + root facade (per 27-CONTEXT.md D-01…D-54
and 27-RESEARCH.md's Recommended Project Structure)
**Analogs found:** 10 role-clusters mapped / 1 role-cluster (Redis lease) explicitly has NO analog

This phase clusters into role-clusters rather than 42 independent files — see 27-CONTEXT.md's own
"Reusable Assets" section, which already names the same clusters. This document adds concrete
excerpts and line numbers per cluster so the planner does not have to re-read the analogs.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/paladin-core/.../run.rs` (D-01, D-02) | core domain type + pure state machine | CRUD (status transitions) | `crates/paladin-core/.../waypoint.rs` (`ThreadId`, `GraphFingerprint`) | exact |
| assistant/schedule/webhook core value types (D-28, D-35, D-39) | core domain type | CRUD | `waypoint.rs` (`ThreadId`, enums, `#[non_exhaustive]`) | exact |
| `run_repository_port.rs` (D-03) | output port trait | CRUD | `paladin-ports/src/output/waypoint_port.rs` | exact |
| `run_queue_port.rs` (D-06) | output port trait | pub-sub / lease queue | `paladin-ports/src/output/node_cache_port.rs` (best-effort trait doc contract) + `waypoint_port.rs` (Send+Sync, mock module) | role-match |
| `cancellation_probe.rs` (D-14) | output port trait | request-response, infallible | `trace_sink_port.rs` ("errors are diagnostics only" contract) | exact (doc contract) |
| `schedule_repository_port.rs` (D-36), assistant repository port (D-29) | output port trait | CRUD | `waypoint_port.rs` | exact |
| `run_submission_port.rs` (D-12) | input port trait | request-response | `paladin-ports/src/input/parley_port.rs` | exact |
| `run_event_stream_port.rs` (D-27) | input port trait | streaming | `parley_port.rs` (shape: facade-implemented, web-consumed) | role-match |
| `crates/paladin-storage/src/run/{mod,in_memory,sqlite,postgres,contract_tests}.rs` (D-03) | multi-backend storage adapter set | CRUD | `crates/paladin-storage/src/waypoint/` (whole directory) | exact |
| `crates/paladin-storage/src/schedule/{...}.rs` (D-36) | multi-backend storage adapter set | CRUD | `crates/paladin-storage/src/waypoint/` | exact |
| `crates/paladin-storage/src/run_queue/in_memory.rs` (D-06) | storage adapter | lease/queue | `crates/paladin-storage/src/node_cache/redis.rs` (connection/error style only) | role-match |
| `crates/paladin-storage/src/run_queue/redis.rs` (D-08) | storage adapter | lease/queue via Lua `EVAL` | **NONE** — see "No Analog Found" | none |
| `src/application/services/run/{...}` worker pool (D-11, D-13) | facade application service | event-driven (dequeue-drive-engine loop) | `src/application/services/parley/{adapter,registry}.rs` + `src/application/services/orchestration/listener.rs` | role-match |
| `ScheduleService` (D-36, D-37) | facade application service | batch/polling (tick claim) | `src/application/services/orchestration/listener.rs` (stress-test house pattern) | role-match |
| `WebhookDeliveryService` (D-40) | facade application service | outbound-HTTP-with-retry | **NONE existing service shape** — see "No Analog Found"; `paladin-llm` adapters give the `Policy::none()` snippet only | partial |
| assistant validation service (D-31) | facade application service | request-response (validate → violations) | `crates/paladin-web/src/agent_registry.rs` (`AgentProvisioner::provision`, `ProvisionError::InvalidSpec`) | role-match |
| `run_controller.rs` / `RunApiState` + `run_router` (D-44) | axum controller + state + DTOs + router | request-response, CRUD | `crates/paladin-web/src/thread_controller.rs` | exact |
| assistant/schedule controllers (D-44…D-47) | axum controller | CRUD + pagination | `thread_controller.rs` (`HistoryQuery`/`HistoryResponse`, `OpenApiRouter` assembly) | exact |
| `ThreadApiState` fork/delete fields (D-45) | controller state extension | request-response | `thread_controller.rs:79-119` (builder pattern) | exact |
| `GET /runs/{id}/stream` (D-24…D-27) | SSE endpoint | streaming | `crates/paladin-web/src/agent_controller.rs:441-579` | role-match (no `keep_alive` precedent — see note) |
| `src/config/{run_store,run_queue,run_worker,assistants,schedules,webhooks}.rs` (D-50) | config struct | — | `src/config/waypoint_store.rs` | exact |
| `CancellationProbe` engine attach point (D-14) | engine seam / builder method | request-response | `crates/paladin-battalion/src/engine/mod.rs` `with_cancellation_token` (1585), `with_trace_sink` (1569) | exact |
| `WarGraphDoc` + `compile()` (D-33) | engine domain type + compiler | transform | `crates/paladin-battalion/src/engine/graph.rs` (`NodeSpec`, `WarGraph::fingerprint`) | role-match (scoped, see Pitfall 2) |
| `runs`, `assistants`, `assistant_versions`, `schedules`, `webhook_deliveries` migrations (D-53) | SQL migration | — | existing `migrations/` waypoint migrations | see "Migration" section below |

## Pattern Assignments

### `crates/paladin-core/src/platform/container/run.rs` (core domain type, D-01/D-02)

**Analog:** `crates/paladin-core/src/platform/container/waypoint.rs`

**Newtype + validation pattern** (`waypoint.rs:36-73`):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThreadId(String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ThreadIdError {
    #[error("thread id must not be empty")]
    Empty,
    #[error("thread id must be at most {THREAD_ID_MAX_LEN} characters, got {len}")]
    TooLong { len: usize },
    #[error("thread id must not contain whitespace")]
    ContainsWhitespace,
}

impl ThreadId {
    pub fn new(id: impl Into<String>) -> Result<Self, ThreadIdError> { /* validate */ }
    pub fn as_str(&self) -> &str { &self.0 }
}
```
Copy this shape for `RunId`. Give `Run` the same `#[non_exhaustive]` + `schema_version: String` +
`#[serde(default)]`-on-every-later-field discipline the module docstring calls out (X-04, X-10.3).

**Structured error, not a bool** (`waypoint.rs` `ThreadIdError`, and D-02's own `IllegalTransition`
spec): mirror `ThreadIdError`'s per-variant `#[error(...)]` message shape for
`IllegalTransition { from, to }` — never a bare `bool` or a stringly-typed message.

**What must differ:** `Run`/`RunStatus` need a pure state-machine method
(`RunStatus::try_transition`) that `ThreadId` has no equivalent of — write it as a free function
returning `Result<RunStatus, IllegalTransition>`, tested exhaustively over the legal-edge table in
D-02 before anything else in the phase (PRD 06 §5 item 1).

---

### `run_repository_port.rs`, `run_queue_port.rs`, `cancellation_probe.rs`, `schedule_repository_port.rs` (output ports)

**Analog:** `crates/paladin-ports/src/output/waypoint_port.rs`

**Trait shape** (`waypoint_port.rs:167-186`):
```rust
#[async_trait]
pub trait WaypointPort: Send + Sync {
    async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError>;
    async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError>;
    async fn get(...) -> Result<Option<Waypoint>, WaypointError>;
    async fn history(...) -> Result<..., WaypointError>;
    async fn list_threads(...) -> Result<..., WaypointError>;
    async fn delete_thread(&self, thread: &ThreadId) -> Result<u64, WaypointError>;
}
```
The file also carries a `mod tests { struct MockStore; impl WaypointPort for MockStore { ... } }`
block (`waypoint_port.rs:345-394`) — copy that mock-module pattern for `RunRepositoryPort`,
`RunQueuePort`, `ScheduleRepositoryPort`.

**Infallible-probe doc contract** (`trace_sink_port.rs:26-32`, "Errors are diagnostics only"):
```
//! ## Errors are diagnostics only
//!
//! [`TraceSink::on_event`] returns `Result<(), TraceSinkError>` ... but the
//! return value is never inspected by anything that decides a run's outcome.
```
Copy this doc-comment structure verbatim in spirit for `CancellationProbe`: `async fn
is_cancelled(&self, thread: &ThreadId) -> bool` (no `Result` at all — D-14 makes it infallible by
signature, one step stronger than `TraceSink`'s "ignored `Result`").

**Best-effort-cache contract** (`node_cache_port.rs:17-26`, "Best-effort by construction"): use as
the second reference for D-14's "a probe failure must never fail a run" wording — this file states
the same idea for `NodeCachePort::get`/`put`.

**What must differ:** `RunQueuePort` needs `LeaseToken`, `QueuedRun`, and a `QueueError` enum with
`LeaseExpired`/`UnknownLease`/`Backend`/`Serialization` variants (D-07) — `WaypointPort` has no
lease concept, so this part is new; only the trait-shape/mock-module scaffolding transfers.

---

### `run_submission_port.rs`, `run_event_stream_port.rs` (input ports)

**Analog:** `crates/paladin-ports/src/input/parley_port.rs`

```rust
pub trait ParleyPort: Send + Sync {
    async fn resume_with(...) -> Result<ResumeAccepted, ParleyError>;
    // (see file's own doc example at line 47 for the intended call shape)
}
```
`RunSubmissionPort` should mirror this trait's "facade implements, `paladin-web` calls" direction
exactly (D-12 names this "exactly Phase 24's split"). `RunEventStreamPort` is the same
facade-implements/web-consumes shape but returns a `Stream` of `RunStreamEvent` — Research's Open
Question #2 flags that its exact `input/` vs `output/` module placement is unconfirmed; default to
co-locating it beside `RunSubmissionPort` under `input/` since both share the same producer/consumer
direction.

**What must differ:** `RunEventStreamPort`'s return type is a `Stream`, not a `Future` — no existing
input port in this crate returns a stream, so the trait signature itself (`fn stream(&self, run_id:
&RunId) -> Pin<Box<dyn Stream<Item = RunStreamEvent> + Send>>` or an `async_trait` equivalent) is new
shape, even though the crate-boundary direction is a precise copy of `ParleyPort`.

---

### `crates/paladin-storage/src/run/`, `.../schedule/` (multi-backend storage adapter set)

**Analog:** `crates/paladin-storage/src/waypoint/` (the single most important analog in this phase)

**Module layout** (`waypoint/mod.rs:1-37`):
```rust
pub mod in_memory;
pub mod contract_tests;
#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub(crate) mod redact;
```
Copy this `mod.rs` file-for-file for `run/mod.rs` and `schedule/mod.rs` — same feature gates
(`sqlite`, `postgres`), same `contract_tests` module name, same `redact` reuse for
connection-string-derived error text (do not write a second redaction helper; import the existing
`waypoint::redact` if visibility allows, or lift its logic into a shared crate-level helper).

**CAS transition pattern** (D-04, generalized from `waypoint/sqlite.rs:424-431` /
`postgres.rs:398-406`'s `rows_affected()` usage on `delete_thread`/`delete_waypoint`, applied to an
`UPDATE` instead of a `DELETE` — full excerpt already given in 27-RESEARCH.md "Pattern 1"):
```rust
let result = sqlx::query(
        "UPDATE runs SET status = ?, started_at = ? WHERE run_id = ? AND status = ?"
    )
    .bind(to.as_str()).bind(at).bind(run_id.to_string()).bind(from.as_str())
    .execute(&self.pool).await.map_err(|e| self.wrap_error(e))?;
if result.rows_affected() == 0 {
    return Err(RunRepositoryError::IllegalTransition { from, to });
}
```

**Portable unique-violation mapping** (D-17 — deliberately NOT the existing `wrap_error` pattern;
27-RESEARCH.md "Pattern 2" has the full excerpt and citation):
```rust
match err {
    sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
        Err(RunRepositoryError::ThreadBusy { thread_id })
    }
    other => Err(self.wrap_error(other)),
}
```
**This must differ from the waypoint adapters' own `wrap_error` helper** (`sqlite.rs:146-155`,
`postgres.rs:130-139`), which collapses every `sqlx::Error` into one generic variant — copying that
helper unmodified for the run repository silently loses the `ThreadBusy` signal on SQLite (no
`.constraint()` support there). Check `is_unique_violation()` first, fall through to `wrap_error`
second.

**Contract suite shape:** `crates/paladin-storage/src/waypoint/contract_tests.rs` (1118 lines) is
the shared async test suite every backend runs unchanged from its own `#[tokio::test]`s — mirror its
structure (a `run_all` aggregate function, generic `async fn` test bodies taking `&dyn
RunRepositoryPort`) for `run/contract_tests.rs` and `schedule/contract_tests.rs`.

---

### `crates/paladin-storage/src/run_queue/redis.rs` (D-08) — NO ANALOG

See "No Analog Found" below. `node_cache/redis.rs` (539 lines) is the only Redis adapter in the tree
and transfers ONLY its connection-manager setup and `scan_match`/error-mapping style — it contains
**zero** `redis::Script`/`EVAL`/`EVALSHA` usage anywhere (confirmed by grep across the whole tree in
27-RESEARCH.md Pitfall 4). Excerpt worth copying regardless:
```rust
// connection-manager acquisition + error mapping style — the only transferable part
// (crates/paladin-storage/src/node_cache/redis.rs — read the file's connection setup
// and its NodeCacheError::Backend mapping before writing the lease adapter's own).
```
The ZSET+Lua claim-and-expire script itself must be designed from `redis` 0.32.2's own `Script` API
docs directly (`redis::Script::new(lua_src).key(k).arg(a).invoke_async(&mut conn)`), not from an
in-repo precedent.

---

### `src/application/services/run/…` worker pool, `ScheduleService` (facade application service)

**Analog:** `src/application/services/parley/{adapter,registry}.rs` (ADR-0031-compliant facade seam)
and `src/application/services/orchestration/listener.rs` (X-05 stress-test house pattern)

The worker pool's core dispatch logic is already fully specified by D-09 with exact line numbers —
copy the branch structure, not new logic:
```rust
// crates/paladin-battalion/src/engine/mod.rs:1787 (start), 1839 (resume), 2086 (resume_with)
// Worker's one entry point branches on the thread's latest Waypoint:
//   absent           -> WarEngine::start
//   pending responses -> WarEngine::resume_with
//   otherwise         -> WarEngine::resume
```

**Stress-test house pattern:** find and read `src/application/services/orchestration/listener.rs`'s
`#[tokio::test(flavor = "multi_thread")]` test module header before writing D-52's three named
stress tests (`ten_concurrent_submits_one_accepted`, `worker_pool_lease_expiry_exactly_once`,
`schedule_restart_exactly_once`) — it is this repo's only existing example of the exact-count +
timeout-guard convention X-05 requires.

**What must differ:** `parley/adapter.rs`'s background-spawn becomes an **enqueue** call under D-20
— the facade adapter still validates synchronously through `WarEngine::resume_with`'s validation
path, then calls `RunQueuePort::enqueue` where it previously called `tokio::spawn`. Read
`parley/adapter.rs` to find that spawn call site before converting it.

---

### `WebhookDeliveryService` (D-40) — weak analog, mostly new

No persisted-queue-drained-by-a-service pattern exists in this repo today (`job_store.rs`'s own
module docs explicitly call its jobs "ephemeral (lost on restart)" and point at the very
queue/worker topology this phase builds as the durable answer — read that as a documented contrast,
not a pattern to copy). The one transferable piece is the outbound-HTTP client convention:

**No-redirect outbound client** (D-42, already the house pattern — full citation in
27-RESEARCH.md "Pattern 5"):
```rust
// crates/paladin-llm/src/openai/adapter.rs:253 (identical in all 9 LLM adapters)
let client = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .build()?;
```
Apply verbatim to the webhook delivery client. `mockito` (`crates/paladin-llm/src/conformance.rs:25`)
is the established outbound-HTTP test-double library — reuse directly, do not add `wiremock`.

**What must differ:** the `next_attempt_at`-driven drain loop, the HMAC-signing-once-from-one-buffer
discipline (D-41), and the SSRF guard (D-42, `std::net::IpAddr` classification, no crate needed) are
all new code with no in-repo precedent to excerpt from.

---

### Assistant validation (D-31)

**Analog:** `crates/paladin-web/src/agent_registry.rs` — `AgentProvisioner::provision()` /
`ProvisionError::InvalidSpec`

`AgentSpec` already carries `id, name, model, system_prompt, temperature, stop_words,
timeout_seconds, allowed_roles` and is `Serialize`/`Deserialize`/`utoipa::ToSchema`;
`AgentProvisioner::provision()` already performs the structural validation shape PLAT-FR-09 wants.
Copy `ProvisionError::InvalidSpec`'s error-list shape for `Vec<ValidationViolation { path, code,
message }>` (D-31) rather than inventing a new envelope; render into `ApiError::with_details`
(`crates/paladin-web/src/error.rs:65`).

**Open question carried from research:** whether to extend `AgentSpec` in place (with
`#[serde(default)]` optional tool/middleware fields) or author a parallel `PaladinConfigDoc` is
unresolved — flag for the plan author (27-RESEARCH.md Open Question #1).

---

### `run_controller.rs` / `RunApiState` + `run_router`, assistant/schedule controllers (D-44)

**Analog:** `crates/paladin-web/src/thread_controller.rs`

**Builder-only state construction** (`thread_controller.rs:79-119`):
```rust
pub struct ThreadApiState {
    pub waypoints: Option<Arc<dyn WaypointPort>>,
    pub parley: Option<Arc<dyn ParleyPort>>,
    pub auth: crate::agent_auth::AgentAuthConfig,
}
impl ThreadApiState {
    pub fn new() -> Self { Self { waypoints: None, parley: None, auth: Default::default() } }
    pub fn with_waypoints(mut self, waypoints: Arc<dyn WaypointPort>) -> Self {
        self.waypoints = Some(waypoints); self
    }
    pub fn with_parley(mut self, parley: Arc<dyn ParleyPort>) -> Self {
        self.parley = Some(parley); self
    }
    pub fn with_auth(mut self, auth: crate::agent_auth::AgentAuthConfig) -> Self { ... }
}
```
Copy this exact shape for `RunApiState` (fields: `run_repository: Option<Arc<dyn
RunRepositoryPort>>`, `run_submission: Option<Arc<dyn RunSubmissionPort>>`, `run_events: Option<Arc<dyn
RunEventStreamPort>>`, `auth: AgentAuthConfig`) and for the assistant/schedule state. An unwired
field means `501 not_implemented` naming the config key (D-44's explicit precedent from D-24). Mark
`#[non_exhaustive]` in the same change per D-45's field-addition discipline.

**Pagination shape** — `HistoryQuery`/`HistoryResponse` at `thread_controller.rs:404-413` is the
`limit` + opaque `cursor` shape D-47 applies verbatim to `/runs`, `/assistants`,
`/assistants/{id}/versions`, `/schedules`, `/runs/{id}/webhook-deliveries`. Read those exact struct
field names before writing new DTOs so all six endpoints share one shape rather than six near-copies.

**Router assembly** — `thread_controller.rs:681-711`'s `utoipa_axum::OpenApiRouter` assembly is the
copy target for `run_router()`.

**Controller test module** — `thread_controller.rs:732+`'s `tower::util::oneshot` test pattern is
the copy target for every new controller's test module (also the confirmed PLAT-06 test type in
27-RESEARCH.md's Phase Requirements → Test Map).

**Two-tier auth** (D-46) — `crates/paladin-web/src/agent_auth.rs:201` (`authorize_invoke`) and `:214`
(`require_admin`): run submit/cancel/resume/fork use `authorize_invoke`; assistant/schedule
create/update/delete use `require_admin`. No new `UserRole` variant — copy the call pattern, not the
enum.

---

### `GET /runs/{id}/stream` (SSE, D-24…D-27)

**Analog:** `crates/paladin-web/src/agent_controller.rs:441-579`

**Stream typedef + framing** (`agent_controller.rs:441, 558-579`):
```rust
type SseEventStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;
// ...
let boxed: SseEventStream = Box::pin(timed_event_stream(rx, timeout));
Sse::new(boxed).into_response()
```
**Utoipa `text/event-stream` annotation** (`agent_controller.rs` responses table, same block):
```rust
(status = 200, description = "Server-Sent Events stream: ...", content_type = "text/event-stream"),
```
Copy the `event()`/`data()` framing style (`Event::default().event("done").data(json!({...
}).to_string())`) for the seven frozen wire event names (D-25).

**What must differ, confirmed by research, not assumed:**
1. **`keep_alive` heartbeat has NO existing call site.** `agent_controller.rs`'s own SSE stream calls
   `Sse::new(...).into_response()` with **no** `.keep_alive()` — the 15s-heartbeat requirement
   (D-26) must use axum 0.8.9's confirmed-but-unused API:
   ```rust
   use axum::response::sse::{Sse, KeepAlive};
   Sse::new(boxed_stream)
       .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
       .into_response()
   ```
   (source: `axum-0.8.9/src/response/sse.rs:74,517-547`, cited in 27-RESEARCH.md's "Code Examples").
2. **The seven wire events do NOT come from one `TraceEvent`-mapping function alone** (Pitfall 1).
   `parley`/`error`/`done` must be published directly by the worker from the `RunOutcome` it already
   matches on (`engine/mod.rs:272-312`); only `superstep`/`node_started`/`node_finished`/`state_delta`
   come from a `TraceSink` adapter mapping `TraceEvent`'s 8 existing variants.
3. Degraded-mode fallback (D-26) polls `WaypointPort::history` — there is no existing "poll and
   synthesize SSE from persisted state" precedent in `agent_controller.rs`; this half is new.

---

### `src/config/{run_store,run_queue,run_worker,assistants,schedules,webhooks}.rs` (D-50)

**Analog:** `src/config/waypoint_store.rs`

**Tagged-enum backend shape** (full excerpt in 27-RESEARCH.md "Pattern 4", `waypoint_store.rs:19-49`):
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum RunStoreBackend {
    Disabled,
    Sqlite { path: String },
    Postgres { url_env: String },  // env var NAME, never the URL itself
}
```
**`Default` + `EnvOverridable` struct shape** (27-RESEARCH.md "Code Examples"):
```rust
pub struct RunWorkerConfig {
    pub concurrency: usize,
    pub lease_seconds: u64,        // heartbeat derived as lease_seconds / 4, D-10 — no separate knob
    pub min_probe_interval_ms: u64,
}
impl Default for RunWorkerConfig {
    fn default() -> Self { Self { concurrency: 4, lease_seconds: 60, min_probe_interval_ms: 1000 } }
}
```
Every one of the six new config structs defaults to `Disabled`/`false` (X-09) — copy
`waypoint_store.rs`'s `validate()` method signature too, not just the enum shape.

---

### Engine seam — `CancellationProbe` attach point, `WarGraphDoc` (D-14, D-33)

**Analog:** `crates/paladin-battalion/src/engine/mod.rs` builder methods

```rust
// existing, at mod.rs:1585 and :1569 — the two-method pattern the new builder method mirrors
pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self { ... }
pub fn with_trace_sink(mut self, sink: Arc<dyn TraceSink>) -> Self { ... }
```
Add `with_cancellation_probe(mut self, probe: Arc<dyn CancellationProbe>) -> Self` in the identical
style — an added builder method, no existing signature changes, no new required trait method
(X-10.4). The consult point is `engine/superstep.rs:1991-2022`, beside the existing
`CancellationToken` check — read that exact block before adding the probe check next to it.

**`WarGraphDoc` analog:** `crates/paladin-battalion/src/engine/graph.rs` — `NodeSpec`'s four variants
(`Paladin`, `Function(Arc<dyn StateNode>)`, `Battalion`, `Gate`, at `graph.rs:42-160`) and
`WarGraph::fingerprint()`'s canonical byte encoding are what `WarGraphDoc::compile()` must produce.
**Confirmed gap (Pitfall 2, do not scope around it silently):** `EngineRegistries`
(`engine/registries.rs:32-50`) resolves `edge_evaluators`/`retry_predicates`/`error_handlers`/
`output_schemas` only — there is no name→`Arc<dyn StateNode>` registry, so `WarGraphDoc`'s node-kind
vocabulary must be scoped to `{Paladin, Gate, Workflow}` for v0.10 (D-33's research-corrected scope),
with a typed `CompileError` naming the limitation for any other kind, never a silent drop.

---

### SQL migrations — `runs`, `assistants`, `assistant_versions`, `schedules`, `webhook_deliveries` (D-53)

**Analog:** existing waypoint migrations under `migrations/`.

**Finding to report to the planner:** this session did not find an existing migration establishing a
**partial unique index** (`CREATE UNIQUE INDEX ... WHERE status IN (...)`) anywhere under
`migrations/` for either SQLite or Postgres — D-17's `runs(thread_id) WHERE status IN
('queued','running','awaiting_input')` constraint appears to be the first partial-index migration in
this tree. This is a **missing-analog signal for the planner**, not a blocker: both SQLite (3.8.0+)
and Postgres support partial unique indexes natively, but the migration-file *shape* (how this repo's
existing migration runner splits SQLite vs Postgres DDL, if at all) should be confirmed by reading
one full existing waypoint migration pair before writing the new one, since no partial-index
precedent exists to copy the *syntax* from directly.

## Shared Patterns

### Structured, non-exhaustive, thiserror errors (X-06)
**Source:** `crates/paladin-core/src/platform/container/waypoint.rs` (`ThreadIdError`),
`crates/paladin-ports/src/output/trace_sink_port.rs` (`TraceSinkError`)
**Apply to:** every new error enum in this phase — `IllegalTransition`, `RunRepositoryError`,
`QueueError`, `ScheduleError`, `CompileError`, `ValidationViolation`'s containing error type. All
`thiserror`, `#[non_exhaustive]`, named fields, no bare `String`/`bool` returns.

### Two-tier HTTP authorization
**Source:** `crates/paladin-web/src/agent_auth.rs:201` (`authorize_invoke`), `:214` (`require_admin`)
**Apply to:** every new controller — invocation-shaped routes (run submit/cancel/resume/fork) use
`authorize_invoke`; registry-shaped routes (assistant/schedule create/update/delete) use
`require_admin`.

### Error envelope
**Source:** `crates/paladin-web/src/error.rs` (`ApiError`, `with_details` at line 65)
**Apply to:** every new endpoint's error responses; D-31's validation violations render into the
`details` slot specifically.

### No-redirect outbound HTTP client for credential-bearing requests
**Source:** all 9 adapters under `crates/paladin-llm/src/*/adapter.rs` (identical
`reqwest::redirect::Policy::none()` call)
**Apply to:** the webhook delivery `reqwest::Client` (D-42) — this is a security-review point per
`security.instructions.md`, not just a style choice.

### Config default-OFF + 501-when-unwired
**Source:** `src/config/waypoint_store.rs` + `crates/paladin-web/src/thread_controller.rs`'s
`Option<Arc<dyn ...>>` fields answering `501 not_implemented`
**Apply to:** all six new config structs (D-50) and `RunApiState`/assistant/schedule state.

### Builder-only public state construction
**Source:** `thread_controller.rs:79-119` (`ThreadApiState::new()` + `with_*` methods)
**Apply to:** `RunApiState` and any assistant/schedule state struct — makes future field additions
(X-10.3) non-breaking by construction.

### Contract-suite-owns-correctness for multi-backend ports
**Source:** `crates/paladin-storage/src/waypoint/contract_tests.rs`
**Apply to:** `run/contract_tests.rs`, `schedule/contract_tests.rs`, and the new `run_queue`
contract suite (InMemory + Redis) — D-08 explicitly defers the Redis lease design's correctness
proof to this suite rather than to a written spec.

## No Analog Found

| File / Role | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/paladin-storage/src/run_queue/redis.rs` (D-08) | storage adapter | ZSET+Lua lease | Confirmed by grep: zero `redis::Script`/`EVAL`/`EVALSHA` usage anywhere in the tree today. `node_cache/redis.rs` transfers only connection/error scaffolding. Budget real design time; research against `redis` 0.32.2's own `Script` API docs (27-RESEARCH.md Pitfall 4). |
| `WebhookDeliveryService`'s persisted-queue-drain loop (D-40) | facade application service | outbound-HTTP-with-retry, restart-durable | No existing "drain a persisted table on an interval, retry with backoff" service in this repo; `job_store.rs`'s docs explicitly disclaim durability. Only the outbound-`reqwest`-client convention (`Policy::none()`) transfers. |
| Partial unique index migration (D-17) | SQL migration | — | No existing migration in `migrations/` uses a partial (`WHERE`-qualified) unique index on either backend. Syntax is standard SQL, but there is no in-repo file to copy the migration-runner's dual-backend DDL split from directly — verify against one full existing migration pair first. |
| Two-process fingerprint-stability test harness (D-35) | test harness | — | 24-CONTEXT D-28 established a two-process shape for cross-process resume, but no generic "spawn a second process, recompute, compare" test utility was found factored out for reuse; likely needs to be written fresh or duplicated from wherever D-28's test lives (not independently located this session — flag for the plan author to locate `24-*`'s actual two-process test file before treating it as a full analog). |

## Metadata

**Analog search scope:** `crates/paladin-core/src/platform/container/`,
`crates/paladin-ports/src/{input,output}/`, `crates/paladin-storage/src/{waypoint,node_cache,scheduler.rs}`,
`crates/paladin-web/src/{thread_controller,agent_controller,agent_registry,agent_auth,error}.rs`,
`crates/paladin-battalion/src/engine/{mod,graph,registries,dispatch_registry,shutdown,superstep}.rs`,
`crates/paladin-llm/src/*/adapter.rs`, `src/config/waypoint_store.rs`,
`src/application/services/{parley,orchestration}/`.
**Files scanned:** ~30 files read or grepped directly in this session, plus the full 27-CONTEXT.md /
27-RESEARCH.md corpus (already containing verified file:line evidence this document builds on rather
than re-deriving).
**Pattern extraction date:** 2026-09-08
