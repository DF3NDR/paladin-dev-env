# PRD 06 — Platform API: Background Runs, Threads, Assistants, Schedules, Webhooks (Epic `PLAT`)

**Depends on:** PRD 01 (engine, Waypoints), PRD 03 (parley/resume, halt). PRD 07's TraceSink feeds run streaming but is not a hard dependency (buffered fallback specified below).
**Primary crates:** `paladin-web`, `paladin-ports`, facade (services, config), `paladin-storage` (queue/registry backends behind ports).

---

## 1. Problem Statement

`paladin-web` today is synchronous: `POST /agents/{id}/execute` holds the HTTP connection for the whole run (or streams SSE until done/timeout). Long workflows therefore live or die with a connection. There is no way to submit a run and come back; no API to list/inspect threads; no managed, *versioned* agent/workflow configurations (the agent registry is code-wired, unversioned — no audit trail, no rollback, no config change without redeploy); no API-managed schedules; no completion webhooks.

This epic turns paladin-web into a durable run server: **runs** execute on a worker pool consuming a persistent queue; **threads** are inspectable and resumable over HTTP; **assistants** are named, versioned configurations; **schedules** trigger runs on cron expressions; **webhooks** notify on terminal states. Every feature is specified as ports + application services + axum controllers so X-01 holds.

## 2. Domain & API Design

### 2.1 Runs

```rust
pub struct RunId(pub Uuid);
pub struct Run {
    pub run_id: RunId,
    pub thread_id: ThreadId,
    pub assistant: AssistantRef,            // (assistant_id, version) resolved at submit time and FROZEN for the run
    pub input: serde_json::Value,           // initial StateDelta (engine) or string input (legacy agents)
    pub status: RunStatus,                  // Queued | Running | AwaitingInput | Completed | Failed | Halted | Cancelled
    pub submitted_at / started_at / finished_at,
    pub webhook: Option<WebhookSpec>,
    pub attempt: u32,                       // queue-level delivery attempt
}
```

HTTP surface (all under the existing auth/error-envelope/utoipa conventions):

- `POST /runs` → `202 { run_id, thread_id }`. Body: `{ assistant_id, version?, thread_id?, input, webhook? }`. Omitted `thread_id` → server-generated. Submitting to a thread whose latest run is `Queued|Running` → `409 ThreadBusy` (one active run per thread — the serialization invariant).
- `GET /runs/{run_id}` → full Run.
- `GET /runs/{run_id}/stream` → SSE of run events (see PLAT-FR-07).
- `POST /runs/{run_id}/cancel` → cooperative cancel (PRD 03 halt semantics). Idempotent; cancelling a terminal run → `409`.
- `GET /threads`, `GET /threads/{id}`, `GET /threads/{id}/history`, `GET /threads/{id}/state`, `POST /threads/{id}/resume` (absorbs HITL-FR-16), `POST /threads/{id}/fork` (body: `{ from_waypoint_id, edit? }` → submits a new Run on the fork), `DELETE /threads/{id}`.

### 2.2 Queue port

```rust
#[async_trait]
pub trait RunQueuePort: Send + Sync {
    async fn enqueue(&self, run: QueuedRun) -> Result<(), QueueError>;
    /// At-least-once lease; invisible to other workers for `lease` duration.
    async fn dequeue(&self, lease: Duration) -> Result<Option<LeasedRun>, QueueError>;
    async fn extend_lease(&self, token: &LeaseToken, by: Duration) -> Result<(), QueueError>;
    async fn ack(&self, token: LeaseToken) -> Result<(), QueueError>;
    async fn nack(&self, token: LeaseToken, requeue_delay: Duration) -> Result<(), QueueError>;
    async fn depth(&self) -> Result<u64, QueueError>;
}
```

Adapters: InMemory (tests/dev) and Redis (existing `redis-queue` stack; visibility via sorted-set lease pattern or streams — implementer's choice, contract suite decides). Shared contract suite including lease-expiry redelivery.

### 2.3 Assistants

```rust
pub struct Assistant {
    pub assistant_id: String,               // slug, unique
    pub versions: Vec<AssistantVersion>,    // append-only
    pub latest: u32,
}
pub struct AssistantVersion {
    pub version: u32,                       // 1-based, monotonically increasing
    pub definition: AssistantDefinition,    // see below
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub note: Option<String>,               // changelog line
}
pub enum AssistantDefinition {
    /// Single agent: full Paladin config as data (system prompt, model, limits, tools by name, middleware config).
    Agent(PaladinConfigDoc),
    /// Workflow: a serialized WarGraph definition (see PLAT-FR-12).
    Workflow(WarGraphDoc),
}
```

HTTP: `POST /assistants` (creates v1), `GET /assistants`, `GET /assistants/{id}`, `GET /assistants/{id}/versions/{v}`, `POST /assistants/{id}/versions` (creates v n+1 — versions are IMMUTABLE once created; no PUT on a version, ever), `DELETE /assistants/{id}` (soft-delete flag; existing runs referencing it remain readable).

### 2.4 Schedules & webhooks

- `POST /schedules` `{ assistant_id, version?, cron: String, input, enabled, webhook? }`, `GET/PATCH/DELETE /schedules/{id}`.
- `WebhookSpec { url, secret: Option<String>, events: Vec<RunEventKind> }` — delivery on terminal statuses (+ `AwaitingInput`, which is the killer notification for parley flows).

## 3. Functional Requirements

Runs & workers:

- **PLAT-FR-01 (Submit-execute decoupling).** `POST /runs` MUST return within 250 ms p99 under nominal load (enqueue only; no engine work on the request path). Run status transitions MUST be persisted via a `RunRepositoryPort` (SQLite + Postgres adapters, contract suite) and every transition MUST be monotonic per the status machine (Queued→Running→{terminal|AwaitingInput}; AwaitingInput→Running on resume; illegal transitions are typed errors).
- **PLAT-FR-02 (Worker pool).** A `RunWorkerPool { concurrency: usize }` (config) consumes the queue: dequeue → mark Running → drive `WarEngine::start/resume` (or the legacy executor for `Agent` assistants) → map `RunOutcome` to status → ack. Lease heartbeat: the worker MUST `extend_lease` at intervals ≤ lease/3 while executing.
- **PLAT-FR-03 (At-least-once + resumable idempotency).** If a worker dies mid-run, lease expiry redelivers the run to another worker, which MUST `resume` the thread (not restart) — Waypoints make redelivery safe. Test: kill worker (drop future) after superstep 2, assert second worker completes with no node re-execution beyond the interrupted superstep.
- **PLAT-FR-04 (Cancellation).** `POST /runs/{id}/cancel` sets a persisted cancel flag AND signals the in-process token if the run is local; a worker on another instance MUST observe the flag at superstep boundaries (engine consults a `CancellationProbe` callback between supersteps — add this narrow seam to the engine if not present). Result: `Halted` waypoint, `Cancelled` run status.
- **PLAT-FR-05 (Thread serialization).** The `409 ThreadBusy` invariant MUST hold under concurrent submits (multi-thread test: 10 concurrent submits to one thread → exactly 1 accepted).
- **PLAT-FR-06 (Parley integration).** A run reaching `AwaitingInput` releases its worker (ack + status AwaitingInput). `POST /threads/{id}/resume` validates responses (HITL-FR-05) then enqueues a resume run reusing the same `run_id` (attempt++). Webhook `AwaitingInput` event carries the ParleyRequests.
- **PLAT-FR-07 (Run streaming).** `GET /runs/{id}/stream`: if the run executes on THIS instance, bridge live TraceSink events to SSE (event types: `superstep`, `node_started`, `node_finished`, `state_delta`, `parley`, `done`, `error` — payloads specified in the OpenAPI schema). If remote or already terminal, fall back to polling-backed synthetic events from persisted Waypoints (documented degraded mode; `done`/`error` always eventually delivered). Heartbeat comment lines every 15 s to defeat idle proxies.

Assistants:

- **PLAT-FR-08 (Immutability & resolution).** Versions immutable (append-only); `POST /runs` without `version` resolves `latest` AT SUBMIT TIME and freezes it on the Run (a later version publish never changes an in-flight or queued run). Both properties tested.
- **PLAT-FR-09 (Validation at publish).** Creating a version MUST validate the definition fully: Paladin config validation for `Agent`; graph validation (schema, edges, custom-condition registration names present in the server's registry, aegis handler names, fingerprint computable) for `Workflow`. Invalid → 400 with a machine-readable list of violations; nothing persisted.
- **PLAT-FR-10 (Audit).** Every version records creator + timestamp + note; `GET .../versions` returns the changelog. Run records reference `(assistant_id, version)` so any historical run's exact configuration is reconstructable.
- **PLAT-FR-11 (Registry compatibility).** The existing code-registered `AgentRegistry` remains (X-03); assistants are an additional, data-driven registry. A config flag exposes code-registered agents read-only via `GET /assistants` as synthetic single-version entries (`source: "code"`), so clients have one discovery surface.
- **PLAT-FR-12 (WarGraphDoc).** A serde-serializable graph definition document (nodes with kind/config/aegis, edges with conditions by name, schema, limits, entry) with a documented JSON Schema, `WarGraphDoc::compile(&Registry) -> Result<WarGraph, CompileError>` resolving named evaluators/handlers/tools, and round-trip tests (doc → compile → fingerprint stable across process restarts). This is the artifact assistants store and the artifact PRD 07 visualizes.

Schedules & webhooks:

- **PLAT-FR-13 (Cron semantics).** Standard 5-field cron + optional seconds, UTC (timezone field optional, IANA name). The scheduler (built on the existing job-scheduling infrastructure) MUST: skip a tick if the schedule's previous run for that tick is still active on its thread strategy (`thread_strategy: NewThreadPerTick (default) | FixedThread(ThreadId)` — FixedThread + busy thread = skip + counted metric); persist `last_tick`, `next_tick`; survive restart without duplicate or missed-then-double firing (catch-up policy: `on_missed: Skip (default) | RunOnce`).
- **PLAT-FR-14 (Webhook delivery).** POST JSON `{ run_id, thread_id, assistant, status, event, timestamp, parleys? }`. If `secret` set: `X-Paladin-Signature: sha256=<hmac-hex>` over the raw body. Retries: 5 attempts, exponential backoff 1s..60s, on 5xx/timeout/connect-error only; 2xx = delivered; 4xx = dead (no retry). Delivery attempts persisted and queryable (`GET /runs/{id}/webhook-deliveries`). Delivery MUST be async off the run-completion path (failure to deliver never affects run status).
- **PLAT-FR-15 (SSRF guard).** Webhook and any outbound URL configuration MUST reject, at write time and at send time: non-http(s) schemes, loopback, link-local, RFC1918/private and metadata (169.254.169.254) targets — overridable ONLY by an explicit server config allowlist (`webhook_allow_private: bool`, default false). Tested with a table of malicious URLs incl. DNS-rebinding note in docs (resolve-then-connect pinning documented as a limitation if not implemented).

Cross-cutting API:

- **PLAT-FR-16 (Auth & limits).** All new endpoints sit behind the existing auth layer and tower-http rate limiting; mutating endpoints require the admin/writer scope consistent with current `agent_controller` conventions. Pagination on every list endpoint (limit ≤ 100, opaque cursor). OpenAPI regenerated; `openapi.json` diff reviewed in the PR.
- **PLAT-FR-17 (Client SDK groundwork, optional-scope).** The OpenAPI spec MUST be complete enough to generate working Python and TypeScript clients with off-the-shelf generators; a CI job generates both and compiles/smoke-tests them (list assistants, submit run, poll status against a test server). Hand-polished SDKs are explicitly out of scope; the generated-client CI gate is in scope.

## 4. Acceptance Criteria

1. Full lifecycle integration test: create assistant (workflow with a Gate) → submit run → SSE shows progress → run suspends AwaitingInput → webhook received (mockito) with parley → resume via API → run completes → history/fork endpoints verified.
2. Worker-death redelivery test (PLAT-FR-03) green on the Redis queue via docker-compose target.
3. ThreadBusy race test exact (PLAT-FR-05).
4. Version immutability + freeze-at-submit tests (PLAT-FR-08).
5. Cron restart test: scheduler restarted across a tick boundary; assert exactly-once per `on_missed` policy.
6. Webhook: signature verification, retry schedule (paused clock), 4xx dead-letter, SSRF table.
7. Generated Python + TS clients smoke-pass in CI.
8. Coverage per X-02.
9. **Versioning gate (X-10/X-11):** any pre-existing public type touched by this epic is recorded in `MIGRATION.md` §9.2 with its mitigation; `cargo semver-checks` and the MSRV job pass; new dependencies listed in §9.3; new migrations in §9.4; new config/env in §9.5.

## 5. Test Plan (TDD ordering)

1. Status-machine unit tests (pure).
2. Queue contract suite (InMemory → Redis).
3. RunRepository contract suite.
4. Worker pool integration with InMemory everything + mock engine.
5. Cancellation probe + cross-instance cancel.
6. Assistants: validation, immutability, resolution, WarGraphDoc round-trip.
7. Scheduler tests under paused clock.
8. Webhook delivery + SSRF.
9. HTTP controller tests (oneshot pattern) per endpoint.
10. Full-lifecycle E2E; SDK generation job.

## 6. Out of Scope

Multi-tenant orgs/RBAC beyond existing scopes; usage metering/billing; horizontal autoscaling logic (the queue makes scale-out possible; orchestration of replicas is deployment concern — k8s docs updated with a worker-replica example); hand-written SDKs.
