# Phase 27: Platform API - Context

**Gathered:** 2026-09-07
**Amended:** 2026-09-08 — six factual corrections applied after `27-RESEARCH.md` disproved
claims made during discussion. D-34 is **revised** (its rationale was void); D-08, D-25, D-33 and
D-53 keep their decisions but carry corrected facts and, for D-33, a narrowed scope. Each is marked
inline with `Correction (research, 2026-09-08)`. See `27-RESEARCH.md` for the evidence.
**Status:** Ready for planning

<domain>
## Phase Boundary

Turn `paladin-web` from a synchronous execute surface into a **durable run server**. Today
`POST /agents/{id}/execute` holds the HTTP connection for the whole run, so a long workflow lives
or dies with a TCP connection; the agent registry is code-wired and unversioned; there are no
API-managed schedules and no completion callbacks.

This phase delivers, over a production-shaped HTTP API:

1. **Runs decoupled from execution** — `POST /runs` enqueues and returns `202` (p99 ≤ 250 ms, no
   engine work on the request path); a `RunRepositoryPort` (SQLite + Postgres) persists every
   status transition through a monotonic status machine with typed illegal-transition errors
   (PLAT-01, PLAT-FR-01).
2. **A durable worker pool** — `RunQueuePort` (InMemory + Redis) with lease heartbeats,
   at-least-once redelivery that **resumes** a killed thread rather than restarting it,
   cross-instance cancellation observed at superstep boundaries, and a one-active-run-per-thread
   `409 ThreadBusy` invariant that holds under 10 concurrent submits (PLAT-02, PLAT-FR-02…05).
3. **Parley + streaming integration** — `AwaitingInput` releases the worker; `POST
   /threads/{id}/resume` re-enqueues under the same `run_id`; `GET /runs/{id}/stream` bridges live
   trace events to SSE with a documented polling-backed degraded mode and 15 s heartbeats
   (PLAT-03, PLAT-FR-06, PLAT-FR-07).
4. **Versioned assistants** — append-only immutable versions (no `PUT`, ever), `latest` frozen at
   submit time, full publish-time validation with machine-readable violations, an audit trail, the
   code-registered registry exposed read-only, and `WarGraphDoc` with a documented JSON Schema, a
   registry-resolving `compile()` and a restart-stable fingerprint round-trip (PLAT-04,
   PLAT-FR-08…12).
5. **Schedules and webhooks** — cron schedules surviving restart without duplicate or
   missed-then-double firing; HMAC-signed webhook delivery with bounded retry, persisted queryable
   attempts, and an SSRF guard at write **and** send time (PLAT-05, PLAT-FR-13…15).
6. **Production shape** — existing auth + rate limiting on every new endpoint, scopes on mutating
   routes, pagination everywhere (`limit ≤ 100`, opaque cursor), `openapi.json` regenerated, and a
   CI job generating Python + TypeScript clients and smoke-testing them (PLAT-06, PLAT-FR-16,
   PLAT-FR-17).

**Not this phase.** The authoritative `TraceEvent` enum, the OTel exporter, Mermaid/DOT graph
export and the `paladin-eval` crate (OBS-01…04, Phase 28); `MIGRATION.md` §9 finalisation, the
v0.9-config boot test, the program acceptance audit and the version bump (SHIP-01…04, Phase 29);
multi-tenant orgs / RBAC beyond the existing scopes; usage metering and billing; horizontal
autoscaling logic (the queue makes scale-out possible — orchestrating replicas is a deployment
concern, with a k8s worker-replica example the only artifact owed); hand-written SDKs.

**Scale note for the planner.** Six requirements, seventeen FRs and nine acceptance criteria make
this the largest phase of the milestone — comparable to Phase 26's twenty-one plans. It decomposes
cleanly along PRD 06 §5's TDD ordering (status machine → queue contract suite → run repository
contract suite → worker pool → cancellation → assistants/WarGraphDoc → scheduler → webhooks → HTTP
controllers → E2E + SDK job), and that ordering should drive plan boundaries.

</domain>

<decisions>
## Implementation Decisions

Every decision below was auto-selected in `--auto` mode: the recommended option was taken for each
question, with the rationale and the rejected alternative recorded so a human can audit or reverse
it. Nothing here re-litigates a decision already locked in Phases 22-26 — those are carried forward
under "Prior-phase decisions that constrain this phase" in the canonical refs.

Reversibility ratings follow `gsd-core/references/planner-reversibility.md`; unrated decisions are
plainly reversible.

### Run identity, the status machine & persistence (PLAT-01; PLAT-FR-01)

- **D-01: `RunId`, `Run`, `RunStatus` and `AssistantRef` are core types, in a new
  `crates/paladin-core/src/platform/container/run.rs`.** `ThreadId` already lives in core
  (`crates/paladin-core/src/platform/container/waypoint.rs:43`) and every port that carries a run
  must take core types (ADR-0016, ADR-0038). `Run` is `#[non_exhaustive]` with a `Default`/builder
  construction path, carries `schema_version: String` (X-04), and every field added later gets
  `#[serde(default)]` so previously written rows still deserialize (X-10.3). — **Reversibility:**
  one-way — a persisted row shape and a published wire shape; changing it after v0.10 needs a data
  migration.
- **D-02: The status machine is a pure function written before anything else.**
  `RunStatus::try_transition(from, to) -> Result<RunStatus, IllegalTransition>` with a structured
  `IllegalTransition { from, to }` error (X-06) — not a `bool`, so the illegal case carries its own
  explanation to the caller and into the 409 envelope. Legal edges, exhaustively: `Queued →
  {Running, Cancelled}`; `Running → {Completed, Failed, Halted, Cancelled, AwaitingInput}`;
  `AwaitingInput → {Running, Cancelled, Failed}` (the third is parley expiry, HITL-FR-06). The four
  terminals (`Completed`, `Failed`, `Halted`, `Cancelled`) are absorbing — "monotonic" means no
  edge leaves a terminal. PRD 06 §5's test-plan item 1 is therefore the first plan's first test.
- **D-03: `RunRepositoryPort` in `paladin-ports/src/output/run_repository_port.rs`; SQLite +
  Postgres + InMemory adapters in `paladin-storage`, behind the *existing* `sqlite` / `postgres`
  features.** `sqlx 0.8` already carries both drivers
  (`crates/paladin-storage/Cargo.toml:19-26`), so no new crate name enters the tree (X-07, X-11.4).
  The three adapters share one contract suite at `crates/paladin-storage/src/run/contract_tests.rs`,
  mirroring `crates/paladin-storage/src/waypoint/contract_tests.rs` — the house pattern Phase 22/24
  established for exactly this two-backend shape.
- **D-04: Transitions are applied by compare-and-set, never read-modify-write.**
  `update_status(&self, run_id, from: RunStatus, to: RunStatus, at)` compiles to `UPDATE runs SET
  status = ?to WHERE run_id = ?id AND status = ?from`; zero rows affected → `IllegalTransition`.
  Monotonicity then holds under concurrent workers with no application lock and no advisory lock,
  and it is the same primitive D-17's busy-thread index and D-37's tick claim rely on. Rejected:
  read-then-write inside a transaction — correct on Postgres, quietly racy on SQLite under WAL with
  multiple processes. — **Reversibility:** costly — the port signature is published and every
  adapter and call site encodes the CAS shape.
- **D-05: No separate status-history table.** The `runs` row carries `status` plus
  `submitted_at` / `started_at` / `finished_at`; the *execution* history a caller actually wants is
  already the Waypoint chain (`GET /threads/{id}/history`, Phase 24 D-27). PLAT-FR-01 asks that
  every transition be **persisted**, which CAS on the row satisfies durably. Rejected: an append-only
  `run_status_history` table — real audit value, but it duplicates the Chronicle for the one entity
  that already has one, and nothing in PLAT-01…06 reads it. Promotable later without a breaking
  change (a new table, no shape change).

### Queue port, leases and redelivery (PLAT-02; PLAT-FR-02, FR-03)

- **D-06: `RunQueuePort` is a NEW port; the pre-existing `QueuePort` is not extended.**
  `crates/paladin-ports/src/output/queue_port.rs` is a general queue-lifecycle trait (create/delete
  queue, enqueue/dequeue, start/complete/fail, stats, health) with no lease token and no visibility
  extension. Adding required methods to it would break every implementor — forbidden outright by
  X-10.4. New file `crates/paladin-ports/src/output/run_queue_port.rs` carrying PRD 06 §2.2's
  signature verbatim (`enqueue`, `dequeue(lease)`, `extend_lease`, `ack`, `nack(requeue_delay)`,
  `depth`). — **Reversibility:** costly — a published port trait with two shipped adapters.
- **D-07: The queue carries a pointer, not a payload.** `QueuedRun { run_id, thread_id, attempt,
  enqueued_at }`; the worker re-reads the full `Run` through `RunRepositoryPort` on dequeue. A
  message that embedded the input could disagree with the database after a resume or a cancel, and
  the repository is already the source of truth for status. `LeaseToken` is an opaque `String`
  newtype so Redis and InMemory can both express it. `QueueError` is `thiserror`,
  `#[non_exhaustive]`, structured (X-06): `LeaseExpired { token }`, `UnknownLease { token }`,
  `Backend { message }`, `Serialization { message }`.
- **D-08: The Redis adapter uses a sorted-set visibility lease driven by Lua, not Redis Streams.**
  PRD 06 §2.2 leaves this to the implementer and says the contract suite decides. The ZSET pattern
  makes claim-and-expire a single atomic `EVAL`, gives `extend_lease` and `nack(delay)` as trivial
  score updates, and the `script` feature is already enabled on the pinned `redis 0.32.2`
  (`crates/paladin-storage/Cargo.toml:61-64`). **Correction (research, 2026-09-08): there is no Lua
  precedent in this tree** — no `EVAL`/`Script` call exists anywhere, including
  `crates/paladin-storage/src/node_cache/redis.rs`, whose reusable house style is the connection
  manager and `safe_iterators`/`scan_match`, not scripting. The ZSET+Lua lease adapter is therefore
  **greenfield**, and the plan must budget for establishing the idiom (script loading, `NOSCRIPT`
  reload, testing) rather than copying one. The decision stands — the enabled `script` feature and
  the contract suite still make it the right shape — but the rationale no longer leans on a
  precedent that does not exist. Rejected:
  Streams + consumer groups — redelivery is native, but `XCLAIM`-based lease extension and delayed
  requeue are awkward and the pending-entries list becomes a second source of truth. —
  **Reversibility:** reversible — the contract suite is the contract; the backend is swappable
  behind it.
- **D-09: Redelivery resumes; it never restarts.** The worker has exactly one entry point, which
  reads the `Run`, then branches on the thread's latest Waypoint: absent → `WarEngine::start`;
  present with pending responses → `WarEngine::resume_with`; present otherwise →
  `WarEngine::resume` (`crates/paladin-battalion/src/engine/mod.rs:1787, 1839, 2086`). PLAT-FR-03's
  kill-mid-run assertion — no node re-executes beyond the interrupted superstep — is the Waypoint's
  own guarantee from Phase 22/24, so this phase adds dispatch, not new engine machinery.
- **D-10: Heartbeat at `lease / 4`, derived rather than separately configurable.** PLAT-FR-02 says
  "≤ lease/3"; a quarter leaves one whole missed heartbeat of slack before expiry. Config exposes
  `run_worker.lease_seconds` (default 60) only — two knobs that must be kept in a ratio is a
  configuration footgun, and an operator who widens the lease should not have to remember to widen
  the heartbeat.

### Worker pool placement and shutdown (PLAT-FR-02)

- **D-11: The worker pool lives in the facade (`src/application/services/run/…`), never in
  `paladin-web`.** ADR-0031 forbids a `paladin-web → paladin-battalion` edge in the default build,
  and driving `WarEngine` needs battalion. The root crate is the only assembly point that sees the
  engine, the queue adapter and the repository — precisely the Phase 24 D-24/D-25 arrangement.
- **D-12: `paladin-web` writes through a new input port and reads through the repository port
  directly.** `RunSubmissionPort` (`crates/paladin-ports/src/input/run_submission_port.rs`) carries
  `submit`, `cancel`, `resume`-adjacent operations in core types; `GET /runs/{id}` and the list
  endpoints read `RunRepositoryPort` directly. This is exactly Phase 24's split — `GET …/state` and
  `GET …/history` read `WaypointPort` directly while resume goes through `ParleyPort` (24-CONTEXT
  D-24/D-25) — so there is one precedent to follow rather than a second convention to learn.
  `paladin-web` never names `WarEngine`, `WarGraph` or a queue. — **Reversibility:** costly — a
  published port and the crate-boundary shape of the whole feature.
- **D-13: Workers are tokio tasks registered with the existing `ShutdownCoordinator`**
  (`crates/paladin-battalion/src/engine/shutdown.rs`, Phase 24 D-21). On shutdown a worker stops
  dequeuing, lets its in-flight run reach a superstep boundary and persist, then exits — draining
  rather than dropping. A run in flight at grace expiry ends as `Halted` (the engine's existing
  cancellation-to-`Halted` path), which is a valid redelivery point, so shutdown never loses work.

### Cross-instance cancellation (PLAT-FR-04)

- **D-14: The engine gains a `CancellationProbe` as an optional trait object consulted at superstep
  boundaries, beside — not instead of — the existing token.** New builder method
  `WarEngine::with_cancellation_probe(Arc<dyn CancellationProbe>)` alongside
  `with_cancellation_token` (`crates/paladin-battalion/src/engine/mod.rs:1585`): an added builder
  method changes no existing signature and adds no required trait method (X-10.4). The trait lives
  in `paladin-ports` (`output/cancellation_probe.rs`) so the DB-backed implementation is an adapter,
  and it is **infallible by design** — `async fn is_cancelled(&self, thread: &ThreadId) -> bool` —
  because a probe failure must never fail a run. That mirrors `TraceSink`'s "errors are diagnostics
  only" contract (`crates/paladin-ports/src/output/trace_sink_port.rs:46-57`); the adapter logs and
  returns `false`. — **Reversibility:** costly — a new public port and an engine seam.
- **D-15: Debouncing is the adapter's job, not the engine's.** The engine calls the probe once per
  superstep boundary, unconditionally and simply. The facade adapter caches the answer for
  `run_worker.min_probe_interval_ms` (default 1000) so a fast graph cannot hammer the database.
  Policy in the adapter, mechanism in the engine.
- **D-16: Cancel is persisted-flag-first, local-signal-second.** `POST /runs/{id}/cancel` writes the
  durable flag through the repository, then best-effort cancels the in-process `CancellationToken`
  if the run is local — durability first means a cancel is never lost to a crash between the two
  steps. Idempotent on a non-terminal run; `409 conflict` on a terminal one (PRD 06 §2.1). The
  engine returns `RunOutcome::Halted` and the run is recorded `Cancelled`: the two vocabularies
  differ deliberately (the *waypoint* halted; the *run* was cancelled) and the mapping is rustdoc'd
  where it happens.

### Thread serialization — the `409 ThreadBusy` invariant (PLAT-FR-05)

- **D-17: The one-active-run-per-thread invariant is a database uniqueness constraint, not an
  application check.** A partial unique index — `CREATE UNIQUE INDEX … ON runs(thread_id) WHERE
  status IN ('queued','running','awaiting_input')` — supported by both SQLite and Postgres. The
  insert's constraint violation maps to `RunRepositoryError::ThreadBusy` → `409`. A check-then-insert
  in the handler cannot hold under PRD acceptance 3's ten concurrent submits, and holds even less
  across instances. — **Reversibility:** costly — a schema constraint with a migration.
- **D-18: `AwaitingInput` counts as busy — a deliberate tightening of the PRD's literal text.**
  PRD 06 §2.1 names `Queued|Running`; a thread suspended awaiting input also has an active run, and
  admitting a second run onto it would start a concurrent execution over the same Waypoint chain.
  The correct client action there is `POST /threads/{id}/resume`, not a new run. This only ever
  *adds* 409s relative to the PRD, so no acceptance test asserting 409-for-Queued-or-Running can
  break; the `409` body names `resume` as the remedy. Flagged here because it is a visible API
  behavior a reviewer should confirm rather than discover.
- **D-19: Resume is exempt by construction, not by exception.** A resume re-enqueues the *same*
  `run_id` (PLAT-FR-06) — an `UPDATE`, not an `INSERT` — so the unique index is never consulted and
  no special case is written into the constraint.

### Parley integration and resume (PLAT-03; PLAT-FR-06)

- **D-20: Phase 24's published resume contract is kept verbatim; only the mechanism behind
  `ParleyPort` changes.** `POST /v1/threads/{id}/resume` keeps its `202`, its `{ thread_id,
  state_url }` body and its whole status-code table (404 `ThreadNotFound`; 409 for
  `ThreadNotAwaitingInput` / `GraphNotRegistered`; 400 for the four validation variants; 501
  unwired) exactly as 24-CONTEXT D-25 published them. The facade adapter still validates
  synchronously through `WarEngine::resume_with`'s validation path, then **enqueues** where it
  previously spawned a background task. This is the deferral Phase 24 recorded explicitly
  ("PLAT-01…03 replace D-25's in-process background task under the same 202 contract").
- **D-21: The resume response gains `run_id`, and the X-10.3 cost is paid, not dodged.** A client
  that resumes needs the run id to stream or poll it. `ResumeAccepted` (`paladin-ports`) and
  `ResumeAcceptedResponse` (`crates/paladin-web/src/thread_controller.rs:360`) each gain the field
  and are marked `#[non_exhaustive]` in the same change, with a construction path preserved;
  both are registered in `MIGRATION.md` §9.2 and the endpoint change in §9.6. Rejected: leaving the
  client to guess or to list runs by thread — a second round trip for a value the server already
  holds.
- **D-22: `AwaitingInput` releases the worker by ACKing, not NACKing.** A suspended run is not
  unfinished queue work: it is durably parked, with the Waypoint as its resume point, and a nack
  would spin — redelivery would re-resume a thread that has no new responses. The test asserts queue
  depth returns to zero while the run sits in `AwaitingInput`.
- **D-23: One `attempt` counter, shared by redelivery and resume.** PRD 06 §2.1 gives `Run` a single
  `attempt` field; both lease redelivery and resume re-enqueue increment it. Webhook and trace
  payloads carry it, and the preceding status distinguishes the two causes (a redelivery follows
  `Running`, a resume follows `AwaitingInput`) — so one field stays honest without a second one.

### Run streaming (PLAT-FR-07)

- **D-24: The live path is a `TraceSink` adapter feeding a per-run broadcast bus.**
  `TraceSink`/`TraceEvent` already exist and are `#[non_exhaustive]`
  (`crates/paladin-ports/src/output/trace_sink_port.rs`), and `WarEngine::with_trace_sink` is
  already the wiring point (`engine/mod.rs:1569`). This phase adds a facade `RunEventBus`: a
  `tokio::sync::broadcast` channel per active run, written by a `TraceSink` implementation and read
  by the SSE handler. Fire-and-forget is preserved — the bus never blocks the engine, and a full
  channel drops oldest and counts.
- **D-25: The seven wire event names are frozen here; the enum they map from is Phase 28's.**
  `superstep`, `node_started`, `node_finished`, `state_delta`, `parley`, `done`, `error` (PRD 06
  PLAT-FR-07) are the published contract, specified in the OpenAPI schema. Today's `TraceEvent`
  variants map onto them in **one function**, and unmapped variants are dropped; when OBS-01 lands
  the authoritative enum, that single function is the only thing that changes.
  **Correction (research, 2026-09-08): a `TraceEvent` mapping alone cannot produce all seven.**
  `TraceEvent` has exactly eight variants today — `RunStarted`, `SuperstepStarted`, `NodeStarted`,
  `NodeFinished`, `DeltaMerged`, `WaypointSaved`, `RunFinished`, `FallbackHop`
  (`crates/paladin-ports/src/output/trace_sink_port.rs:67-152`) — with **no** parley variant, no
  error variant, and a `RunFinished` that does not distinguish success from failure. So the bus has
  two producers, not one: the `TraceSink` adapter emits `superstep` / `node_started` /
  `node_finished` / `state_delta`, and the **worker** publishes `parley`, `done` and `error`
  directly from the `RunOutcome` it already matches on
  (`crates/paladin-battalion/src/engine/mod.rs:272-312`, whose `AwaitingInput` / `Completed` /
  `Failed` / `Halted` arms carry exactly the needed information). Phase 28 can later collapse this
  into the single mapping function once the authoritative enum carries those cases. Rejected: defining
  the authoritative enum here — it is explicitly Phase 28's deliverable and PRD 06 lists PRD 07 as a
  soft dependency for exactly this reason. — **Reversibility:** costly — the wire names are a
  published contract; the internal enum is not.
- **D-26: Degraded mode is a first-class documented path, not an error case.** If the run is
  executing on another instance, or is already terminal, the handler synthesizes events from
  persisted Waypoints by polling `WaypointPort::history` at `run_stream.poll_interval` (default
  1 s), and always terminates with `done` or `error`. The OpenAPI description and the mdBook page
  state plainly that the degraded path gives no ordering guarantee relative to the live path and may
  coalesce supersteps — PLAT-FR-07 asks for a *documented* degraded mode, and naming the limitation
  is what satisfies it. 15 s heartbeat comment lines on both paths (idle-proxy defence).
- **D-27: `paladin-web` consumes a `RunEventStreamPort`, it does not own the bus.** ADR-0031 again:
  the port (input side) returns a `Stream` of **core** `RunStreamEvent` values, and the facade
  decides live-versus-degraded behind it. The controller's only job is SSE framing and heartbeats.

### Assistants — storage, immutability, validation (PLAT-04; PLAT-FR-08…11)

- **D-28: `AssistantDefinition` is a tagged envelope over opaque JSON in core; only the facade knows
  how to interpret it.** `AssistantDefinition { kind: AssistantKind /* Agent | Workflow */, body:
  serde_json::Value }`. `paladin-ports`, `paladin-storage` and `paladin-web` must carry, persist and
  echo a definition without depending on `paladin-battalion` (X-01, ADR-0031); the facade is the only
  crate that can compile one. Rejected: a typed `Workflow(WarGraphDoc)` arm — it would drag
  engine-shaped types into `paladin-core`, which has no engine. Also rejected: an asymmetric
  `Agent(PaladinConfigDoc)` + `Workflow(Value)` pair — core does own `PaladinConfig`, so it would
  typecheck, but two arms with two different levels of typing means two validation seams and two
  persistence shapes for one concept. — **Reversibility:** one-way — both a persisted column and a
  published request/response body.
- **D-29: Immutability is enforced by the schema and the router, not by handler discipline.**
  `assistant_versions` has primary key `(assistant_id, version)`; the adapter exposes
  `append_version`, `get_version`, `list_versions` and `soft_delete_assistant` — there is **no**
  `update_version` method to call — and no `PUT` route is registered, so the router cannot express
  the operation. "No PUT, ever" then holds because the code to violate it does not exist. —
  **Reversibility:** one-way — the PRD makes immutability an explicit promise clients will build on.
- **D-30: `latest` is resolved and frozen inside the same transaction that inserts the run.**
  `POST /runs` without `version` reads `assistants.latest` and writes the resolved
  `(assistant_id, version)` onto the run row in one transaction. A concurrent version publish then
  lands strictly before (the new run sees it) or strictly after (the run keeps the old) —
  PLAT-FR-08's freeze-at-submit becomes a database property provable by a concurrency test, rather
  than a timing hope in application code.
- **D-31: Validation is a facade service returning a machine-readable violation list; nothing is
  persisted on failure.** `Vec<ValidationViolation { path, code, message }>` renders into the
  existing error envelope's `details` slot (`ApiError::with_details`,
  `crates/paladin-web/src/error.rs:65`) as `400`. `Agent` bodies validate through the existing
  Paladin config validation; `Workflow` bodies deserialize to `WarGraphDoc` and then `compile()`
  against the server's registries — **compile is the validation**, so a version that exists is
  always a version that runs.
- **D-32: Code-registered agents are exposed as read-only synthetic assistants, config flag default
  ON.** PLAT-FR-11's "one discovery surface" is only true if it is on by default:
  `assistants.expose_code_registry: bool = true`, entries rendered as `{ source: "code", version: 1 }`,
  and every mutating route rejects them with `409 conflict` / code `code_registered_immutable`. The
  existing `AgentRegistry` (`crates/paladin-web/src/agent_registry.rs`) is untouched (X-03).

### `WarGraphDoc` (PLAT-FR-12)

- **D-33: `WarGraphDoc` lives in `paladin-battalion` beside `WarGraph`, with the PRD's literal
  inherent `compile()`.** New `crates/paladin-battalion/src/engine/graph_doc.rs`;
  `WarGraphDoc::compile(&EngineRegistries) -> Result<WarGraph, CompileError>` resolving named edge
  evaluators, aegis handlers, dispatch rules and tools through the existing registries
  (`engine/registries.rs`, `engine/dispatch_registry.rs`). It cannot live in core (it names node
  kinds, edge conditions and aegis policies — engine vocabulary) and it cannot live in ports (ports
  see only core). The facade is the sole deserializer. This also replaces Phase 24's deliberately
  minimal fingerprint-keyed `GraphRegistry`
  (`src/application/services/parley/registry.rs`) behind the unchanged `ParleyPort` — the
  replacement 24-CONTEXT D-26 anticipated by name.
  **Scope correction (research, 2026-09-08): `WarGraphDoc` node kinds are `{Paladin, Gate,
  Workflow}` for v0.10 — `Function` nodes are NOT expressible.** `EngineRegistries`
  (`crates/paladin-battalion/src/engine/registries.rs:32-50`) carries `edge_evaluators`,
  `retry_predicates`, `error_handlers` and `output_schemas` — there is **no** name→`Arc<dyn
  StateNode>` registry, so a data-driven document has no way to name an arbitrary `Function` node's
  behavior. The options were to add a node registry (new public surface, an X-10 event, and a
  capability no PLAT FR asks for) or to scope the document to the kinds that *are* nameable. Scoping
  wins: it is the smallest change preserving D-33's intent, every PRD 06 acceptance path (a workflow
  assistant with a `Gate`) is still expressible, and a node registry stays available later as a
  purely additive change. The unsupported kind must produce a **typed `CompileError` naming the
  limitation**, never a silent drop, and the limitation is documented on the JSON Schema page.
- **D-34 (REVISED after research, 2026-09-08): The JSON Schema is DERIVED with `schemars` and
  checked in as a golden file.** The original decision hand-authored the schema to avoid adding a
  runtime dependency — **that rationale was factually wrong and is withdrawn**: `schemars = "1.2"`
  is already a **direct workspace dependency** (`Cargo.toml:143`), added in Phase 26 plan 26-17
  precisely for `schemars::schema_for!`, while `jsonschema` — the crate the hand-authored guard
  needed — is **not in `Cargo.lock` at all** (0 occurrences) and would have been the genuinely new
  dependency. With the facts corrected, deriving is both cheaper and better guarded. So:
  `WarGraphDoc` derives `JsonSchema`; a test runs `schema_for!(WarGraphDoc)` and asserts byte
  equality against the checked-in `docs/schemas/wargraph-doc.schema.json` (regenerated by an
  `--bless`-style path, mirroring how this repo blesses other golden files), plus an mdBook page.
  Example docs under `crates/paladin-battalion/tests/fixtures/graph_docs/` still round-trip
  deserialize → compile → re-serialize byte-identically. A struct field added without regenerating
  the golden file now fails on the *derive* comparison rather than on a hand-maintained corpus —
  a strictly stronger drift guard with no new dependency. Rejected (now): hand-authoring, which buys
  nothing once `schemars` is already present and leaves the schema free to drift silently.
- **D-35: Fingerprint stability is proven across a real process boundary, not a round trip.**
  `GraphFingerprint::from_canonical_bytes`
  (`crates/paladin-core/src/platform/container/waypoint.rs:391`) is content-addressed, so
  "restart-stable" really means the canonical encoding contains no `HashMap` iteration order, no
  pointer and no timestamp — none of which a same-process round trip can catch. The test writes doc
  + fingerprint in one process and recomputes in a second, the same two-process shape Phase 24 used
  for cross-process resume (24-CONTEXT D-28).

### Schedules (PLAT-05; PLAT-FR-13)

- **D-36: Run schedules do NOT use `tokio-cron-scheduler`.** The existing
  `TokioCronSchedulerAdapter` (`crates/paladin-storage/src/scheduler.rs`) is in-memory, offers no
  status query and no persistence — its own module docs say so — which is exactly what PLAT-FR-13
  forbids. It stays unchanged for existing `SchedulerPort` consumers (X-03). Run schedules get a
  `ScheduleRepositoryPort` (SQLite + Postgres, same feature gates as D-03) over
  `{ schedule_id, assistant_id, version?, cron, timezone, input, enabled, thread_strategy,
  on_missed, last_tick, next_tick, skipped_ticks }`, driven by one facade `ScheduleService`.
  Rejected: persisting alongside `tokio-cron-scheduler` and re-registering jobs at boot — the
  duplicate-or-missed-double-fire guarantee would then live in two places.
- **D-37: A tick is *claimed* by a conditional update, which makes multi-replica safe without leader
  election.** `UPDATE schedules SET last_tick = ?next, next_tick = ?after WHERE schedule_id = ?
  AND next_tick = ?next` — exactly one replica's update affects a row, and that replica submits the
  run. Restart safety falls out of the same mechanism: `next_tick` is persisted, so a restart
  neither double-fires (the claim already advanced it) nor missed-then-double-fires (`on_missed:
  Skip` recomputes from now; `RunOnce` fires once, then recomputes). PRD acceptance 5 — scheduler
  restarted across a tick boundary, exactly-once per policy — tests this directly. —
  **Reversibility:** costly — a schema plus a concurrency contract two adapters implement.
- **D-38: Cron is parsed with `croner`, and both 5-field and 6-field forms are accepted.** `croner`
  is already in `Cargo.lock` (pulled by `tokio-cron-scheduler`), so it is MSRV-proven in this
  workspace before it is ever promoted to a direct dependency. PLAT-FR-13 wants standard 5-field
  with optional seconds; the existing adapter requires 6 and rejects 5
  (`crates/paladin-storage/src/scheduler.rs:59-95`). The new path normalizes both and **reuses that
  extracted field-count logic** rather than re-deriving it; the existing adapter's stricter contract
  is left exactly as published (X-03). Timezone is UTC by default with an optional IANA name.
- **D-39: `thread_strategy` and `on_missed` are core enums carrying the PRD's defaults.**
  `NewThreadPerTick` (default) / `FixedThread(ThreadId)`; `Skip` (default) / `RunOnce`.
  `FixedThread` onto a busy thread skips **and increments `skipped_ticks`** on the row, so
  `GET /schedules/{id}` can answer "why did nothing run" without a log dive — PLAT-FR-13 asks for a
  counted metric and this is where it lives.

### Webhooks (PLAT-05; PLAT-FR-14, FR-15)

- **D-40: Delivery is a persisted queue drained by a service, not a spawned task.** "Async off the
  run-completion path" is satisfied either way, but a spawned task loses deliveries on restart —
  and PLAT-FR-14 already requires attempts to be persisted and queryable
  (`GET /runs/{id}/webhook-deliveries`). Given that table exists, the retry schedule becomes a
  `next_attempt_at` column drained by `WebhookDeliveryService`, not a sleeping task. One mechanism,
  restart-durable, queryable for free. — **Reversibility:** costly — a schema and a published
  read endpoint.
- **D-41: HMAC over the exact bytes sent, signed once from one buffer.** `hmac 0.12` (already in
  `Cargo.lock`) with the workspace's existing `sha2`; `X-Paladin-Signature: sha256=<hex>` computed
  over the serialized body **buffer that is then handed to the client** — never re-serialized for
  sending. Re-serialization between signing and sending is the classic signature-mismatch bug, and
  the test asserts a receiver-side verification against the raw captured body (mockito).
- **D-42: The SSRF guard is a standalone table-tested function applied at BOTH write time and send
  time, and the webhook client follows no redirects.** Rejects non-`http(s)` schemes and any host
  resolving to loopback, link-local (`169.254.0.0/16`, `fe80::/10`), RFC1918, unique-local,
  unspecified, or the metadata address `169.254.169.254`. Overridable only by
  `webhooks.allow_private: bool` (default `false`, X-09). Redirect-following is **disabled** on the
  webhook `reqwest::Client`: `security.instructions.md` already makes that the house rule for
  clients sending a credential header, and the signature header is a credential — a followed
  redirect would forward it to an attacker-chosen host and would also bypass the write-time check.
  DNS rebinding: resolve-then-connect pinning is **documented as a known limitation** in rustdoc and
  the security docs rather than implemented — PRD 06 PLAT-FR-15 explicitly permits that, and
  claiming coverage we do not have is worse than naming the gap.
- **D-43: `4xx` dead-letters immediately; `5xx` / timeout / connect-error retries 5 times with 1 s…
  60 s exponential backoff; the clock is injectable.** The retry-schedule test runs under
  `tokio::time::pause` — no test sleeps for a minute. `2xx` is delivered. A dead-lettered delivery
  stays queryable with its final status and response code.

### HTTP surface, scopes and pagination (PLAT-06; PLAT-FR-16, FR-17)

- **D-44: One new `RunApiState` + `run_router`, mirroring Phase 24's `ThreadApiState`; the
  pre-existing `AgentApiState` stays untouched.** Nested under `API_V1_PREFIX` (ADR-0037), merged by
  `paladin-server` behind the same auth middleware as `/v1/agents/*` and `/v1/threads/*`. Runs,
  assistants, schedules and webhook-delivery routes share the one state rather than spawning four
  states with the same three fields. An unwired deployment answers **501 `not_implemented`** naming
  the config to set, while the spec still lists the paths — the D-24 precedent exactly.
- **D-45: `ThreadApiState` gains fields for the new thread routes and is marked
  `#[non_exhaustive]` in the same change.** `POST /threads/{id}/fork` and `DELETE /threads/{id}`
  belong on the existing thread router, which needs the run-submission seam. `ThreadApiState`
  already has a builder (`with_waypoints` / `with_parley` / `with_auth`,
  `crates/paladin-web/src/thread_controller.rs:107-119`), so construction stays builder-only and the
  X-10.3 mitigation is free; registered in `MIGRATION.md` §9.2. Rejected: a parallel fork/delete
  router — two routers over one resource is worse than one registered struct change.
- **D-46: "Admin/writer scope" maps onto the existing two roles by *shape of operation*, with no
  new role variant.** `UserRole` is `{Admin, User}`
  (`crates/paladin-core/src/platform/container/user.rs:72`); adding a `Writer` variant is an X-10.2
  break on a core public enum for no FR. PLAT-FR-16 asks for consistency with `agent_controller`'s
  conventions, and those conventions are two-tier: *invocation* is gated by `authorize_invoke`
  against the target's allowed roles, *registry mutation* is gated by `require_admin`
  (`crates/paladin-web/src/agent_auth.rs:201, 214`). So: **run submission, cancel, resume and fork
  are invocation-shaped** → any authenticated principal, subject to the assistant's own
  `allowed_roles` where present; **assistant and schedule create/update/delete are registry-shaped**
  → `require_admin`. Reads require authentication only. A finer-grained scope model is a deferred
  idea, and §9.6 says so.
- **D-47: Pagination is Phase 24's `limit` + opaque `cursor` shape, applied verbatim everywhere.**
  `?limit=20&cursor=…`, `limit ≤ 100`, `{ items, next_cursor }`, cursor content documented as opaque
  (the last returned id) — 24-CONTEXT D-27 chose this shape specifically so PLAT-06 would need no
  breaking change. It now applies to `/runs`, `/threads`, `/assistants`, `/assistants/{id}/versions`,
  `/schedules` and `/runs/{id}/webhook-deliveries`. One shape is what lets PLAT-FR-16 be a
  one-sentence claim instead of six separate ones.
- **D-48: `openapi.json` is regenerated and diff-reviewed on every PR that touches routes; the
  golden-diff *gate* remains SHIP-02's.** `crates/paladin-web/openapi.json` (1,453 lines today) is
  the drift baseline (ADR-0037). DTOs stay `utoipa::ToSchema` types in `paladin-web` (ADR-0038).
- **D-49: Client generation uses one pinned `openapi-generator-cli` for both Python and TypeScript,
  in a `sdk-clients` CI job that runs on every PR.** One pinned tool covering both languages keeps
  the job's real purpose in view — proving the *spec* is complete enough to generate against, not
  shipping an idiomatic SDK (hand-polished SDKs are explicitly out of scope). Smoke test: boot the
  test server on the all-InMemory profile, then list assistants → submit a run → poll status from
  each generated client. Runs on every PR rather than path-filtered: a required check that does not
  run on a given PR blocks its merge on GitHub, and that trap is not worth the saved minutes.
  Researcher item: confirm whether CI has Docker for the generator image, else the npm/pip
  distributions.

### Config, tests and program bookkeeping

- **D-50: Every new subsystem is config-gated and defaults to OFF (X-09).** New structs in
  `src/config/`, each with `Default` + `validate()` + `EnvOverridable`, mirroring
  `src/config/waypoint_store.rs` (Phase 24's X-09 precedent): `run_store.rs`
  (`backend: disabled | sqlite { path } | postgres { url env }`, default `disabled` → 501),
  `run_queue.rs` (`in_memory | redis { url env }`), `run_worker.rs` (`concurrency`,
  `lease_seconds`, `min_probe_interval_ms`), `assistants.rs` (`expose_code_registry: bool = true`),
  `schedules.rs` (`enabled: bool = false`), `webhooks.rs` (`allow_private: bool = false`,
  `max_attempts: 5`, `timeout`). Defaulting off is not caution for its own sake — it is what makes
  SHIP-02's "v0.9 config boots v0.10 with all new subsystems disabled" test pass by construction
  rather than by patch.
- **D-51: Test tiers — Docker is unavailable in this devcontainer, so Redis and Postgres suites are
  Tier 2 and are never marked passed locally.** Carried forward verbatim from 24-CONTEXT D-28 and
  reaffirmed in Phase 25/26. Tier 1 (local, always run): InMemory queue, SQLite run store on a temp
  file, `MockLlmAdapter`, `CountingFunctionNode`, mockito for webhooks, `tokio::time::pause` for
  backoff. Tier 2 (CI / UAT): the Redis queue contract suite, the Postgres run-repository and
  schedule-repository contract suites, and PRD acceptance 2's worker-death redelivery test — which
  is specified against Redis, so it gets an InMemory twin running Tier 1 so the *logic* is covered
  locally even when the backend is not.
- **D-52: Concurrency stress tests per X-05, with exact counts and timeout guards.**
  `#[tokio::test(flavor = "multi_thread")]` for: 10 concurrent submits to one thread → exactly 1
  accepted (PLAT-FR-05 / acceptance 3); lease expiry with two workers → exactly one completion and
  no duplicated node execution (PLAT-FR-03 / acceptance 2); two `ScheduleService` instances over one
  tick → exactly one fire (PLAT-FR-13 / acceptance 5).
- **D-53: `MIGRATION.md` registrations are filled as the work lands, not swept up at the end.**
  §9.2: `ThreadApiState` (fields + `#[non_exhaustive]`), `ResumeAccepted` /
  `ResumeAcceptedResponse` (`run_id`), and any pre-existing public type this epic touches — the
  verification pass diffs the public API and every unregistered change is a finding (X-10.1).
  §9.3: `hmac`, `croner`, `jsonschema` (dev-only), plus `ipnet` if the SSRF guard needs more than
  `std::net::IpAddr` — each `cargo msrv verify`'d at **1.88** (X-11.1). **Correction (research,
  2026-09-08): the workspace MSRV is 1.88** (`Cargo.toml:18`; CI's `msrv` job pins
  `RUSTUP_TOOLCHAIN: "1.88"`), not the 1.85 the program overview still states — a later phase raised
  it. Every "verify at 1.85" instruction in this phase reads 1.88. §9.4: the `runs`, `assistants`,
  `assistant_versions`, `schedules` and `webhook_deliveries` migrations. §9.5: every struct in
  D-50. §9.6: every new endpoint plus the resume-response field.
- **D-54: Coverage stays at the 82% workspace floor (ADR-0006), and tests land inside each plan
  rather than in a trailing test plan.** This phase adds a large surface across five crates; a
  trailing catch-up plan is how a floor gets missed. Note for planners: **doc tests do not count
  toward `cargo llvm-cov` and are skipped by `--tests`** — public-API doc tests are still required
  (X-02) but must not be relied on for the coverage number.

### Claude's Discretion

- Exact module and file names within the crates fixed above, and how the SQL migrations are split.
- Whether the run row's `webhook` spec is an inline JSON column or a joined table.
- Plan decomposition and ordering beyond PRD 06 §5's TDD sequence, and how many plans the phase
  takes.
- The precise `RunStreamEvent` payload fields, within the seven frozen event names (D-25).
- Whether `ipnet` is needed at all, or `std::net::IpAddr` classification suffices (D-42).
- Whether `run_status_history` is promoted to a real table if research surfaces an FR that reads it
  (D-05 leaves this additive).

### Folded Todos

None. `todo.match-phase 27` returned one match at score 0.2 — below the `--auto` fold threshold of
0.4 — and it is unrelated to this phase (see Reviewed Todos).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase source of truth (behavior)

- `.project/v0.10.0/06-platform-api.md` — **the** specification for this phase. §1 problem
  statement; §2.1 `Run` shape and the full HTTP surface; §2.2 the `RunQueuePort` signature; §2.3
  the `Assistant`/`AssistantVersion`/`AssistantDefinition` shapes and assistant routes; §2.4
  schedules and `WebhookSpec`; §3 PLAT-FR-01…17 (the binding functional requirements); §4 the nine
  acceptance criteria; §5 the TDD ordering that should drive plan boundaries; §6 out of scope.
- `.planning/REQUIREMENTS.md` lines 215-256 — PLAT-01…06 as written for this milestone, including
  the wording the roadmap's success criteria were derived from.
- `.planning/ROADMAP.md` lines 576-589 — Phase 27's goal, dependencies (Phase 22, Phase 24) and the
  five success criteria that must be TRUE at verification.
- `.project/v0.10.0/00-program-overview.md` §3 — **X-01…X-11, non-negotiable and unrestated per
  requirement.** X-01 hexagonal dependency rule; X-02 TDD + 82% floor; X-03 backward compatibility
  and the stop-and-flag rule; X-04 `schema_version` on every persisted type; X-05 `Send + Sync` and
  the multi-thread stress-test requirement; X-06 no new stringly-typed errors; X-07 feature gates;
  X-08 docs; X-09 config structs; X-10 semver hygiene and the §9.2 register; X-11 MSRV (stated as
  1.85 there, **actually 1.88** today — see D-53) and
  dependency discipline.

### Program deliverable this phase appends to

- `MIGRATION.md` §9.2 (Rust API register), §9.3 (toolchain & dependencies), §9.4 (persistence &
  schema migrations), §9.5 (configuration & environment), §9.6 (HTTP API) — D-53 lists exactly what
  this phase owes each section. SHIP-01 (Phase 29) fails if any is left "TBD".

### Prior-phase decisions that constrain this phase

- `.planning/phases/24-pause-resume-history-graceful-shutdown/24-CONTEXT.md` — D-24 (thread routes
  get their own state and router, `AgentApiState` untouched, 501 when unwired), **D-25 (the
  published `202` resume contract and its complete status-code table — kept verbatim by D-20)**,
  D-26 (the fingerprint-keyed `GraphRegistry` this phase's `WarGraphDoc` registry replaces behind
  the same `ParleyPort`), D-27 (the `limit` + opaque-cursor pagination shape D-47 generalizes),
  D-28 (test tiers; Docker unavailable locally → Redis/Postgres are Tier 2). Its Deferred Ideas
  section names four items as explicitly Phase 27's.
- `.planning/phases/26-agent-runtime-enhancements/26-CONTEXT.md` — D-21 (`RunScope { vault_namespace }`
  in core, `#[non_exhaustive]`, "so Phase 27 can add `user_id`/`run_id` without a break", reached
  through the defaulted `PaladinPort::execute_scoped`). Deriving a `RunScope` from an HTTP run
  request is recorded there as this phase's work.
- `.planning/phases/25-node-level-fault-tolerance/25-CONTEXT.md` — Aegis/retry/timeout semantics the
  worker inherits; its deferred note that a `Failed` thread stays terminal (PLAT-02 resumes
  `Running` threads, not `Failed` ones).
- `.planning/phases/22-battlefield-state-superstep-engine/22-CONTEXT.md` — Waypoint durability and
  superstep boundaries, which are what make D-09's resume-not-restart guarantee true.

### Standing decisions and governance

- `.planning/decisions/0006-coverage-gate.md` (ADR-0006) — the single 82% workspace line-coverage
  floor (D-54).
- `.planning/decisions/0015-core-ports-dependency-allowlist.md` (ADR-0015) — what a new port trait
  may import; `utoipa` stays in `paladin-web`.
- `.planning/decisions/0016-port-value-type-ownership.md` (ADR-0016) — core owns port value types,
  ports re-export (D-01, D-28).
- `.planning/decisions/0031-extracted-crate-dependency-rule.md` (ADR-0031) — why `paladin-web` may
  not depend on `paladin-battalion` in its default build; the single most load-bearing constraint on
  this phase's crate layout (D-11, D-12, D-27, D-28).
- `.planning/decisions/0037-agent-route-surface-v1.md` (ADR-0037) — routes are `/v1`-prefixed;
  `crates/paladin-web/openapi.json` is the drift-guard baseline (D-44, D-48).
- `.planning/decisions/0038-agent-provisioner-placement.md` (ADR-0038) — HTTP DTOs stay in
  `paladin-web`; a port's parameter types must be core types (D-12, D-28).
- `.planning/decisions/0039-http-topology-no-garrison-no-arsenal.md` (ADR-0039) — HTTP-served agents
  are LLM + prompt only; `paladin-server` registers no `WarGraph`s, which is why end-to-end graph
  runs are proven in `paladin-web`'s `oneshot` tests with an in-test registry.
- `.planning/decisions/0040-opaque-bearer-token-mechanism.md` (ADR-0040) and
  `.planning/decisions/0041-in-process-token-store-single-replica-scope.md` (ADR-0041) — **read
  together with this phase's multi-instance ambitions.** The token store is in-process and
  explicitly single-replica scoped; cross-instance cancellation and a multi-replica worker pool make
  that limitation newly visible. This phase does not fix it (no FR), but the k8s worker-replica
  example and the deployment docs must not imply otherwise.
- `.github/instructions/security.instructions.md` — the manual credential-handling review is the
  primary control (no merge-gating Rust SAST). Directly binding here: the webhook client carries a
  signature credential and therefore must not follow redirects (D-42); no API key or secret may
  reach a webhook payload, a trace event, a run row or an error body; response bodies are redacted
  **before** truncation.

### Existing implementation this phase extends

- `crates/paladin-web/src/thread_controller.rs` — the state/router/DTO/pagination pattern D-44…D-47
  follow, and the file D-21/D-45 modify.
- `crates/paladin-web/src/error.rs` — the `{ error: { code, message, details } }` envelope every new
  endpoint uses; `with_details` is the slot D-31's violation list renders into.
- `crates/paladin-web/src/agent_auth.rs` — `authorize_invoke` and `require_admin`, the two-tier
  convention D-46 maps onto.
- `crates/paladin-storage/src/waypoint/` — the `mod.rs` / `in_memory.rs` / `sqlite.rs` /
  `postgres.rs` / `contract_tests.rs` layout D-03 mirrors for runs, schedules and assistants.
- `crates/paladin-battalion/src/engine/mod.rs` — `start` (1787), `resume` (1839),
  `resume_with_options` (1897), `resume_with` (2086), `with_cancellation_token` (1585),
  `with_trace_sink` (1569), `RunOutcome` (272); the seams D-09, D-14 and D-24 attach to.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`crates/paladin-storage/src/waypoint/{mod,in_memory,sqlite,postgres,contract_tests}.rs`** — a
  complete, shipped example of the exact thing this phase needs three more of: one port, three
  adapters, one shared contract suite, two cargo features, no new crate names. `RunRepositoryPort`,
  `ScheduleRepositoryPort` and the assistant repository should be near-mechanical copies of its
  shape.
- **`crates/paladin-storage/src/node_cache/redis.rs`** — the Redis house style (connection manager,
  `safe_iterators`/`scan_match`, error mapping), the closest analog for D-08's sorted-set lease
  adapter. Note the limit research found: it contains **no Lua** — D-08's scripted lease is
  greenfield and only the connection/error scaffolding transfers.
- **`crates/paladin-storage/src/scheduler.rs:59-95`** — a working 5-field↔6-field cron normalization
  with a good error message, written to be unit-testable as a free function. D-38 extracts and
  reuses it rather than writing a second one. The adapter *around* it is deliberately not reused
  (D-36).
- **`crates/paladin-web/src/thread_controller.rs`** — `ThreadApiState` + builder (79-119),
  `HistoryQuery`/`HistoryResponse` cursor pagination (404-413), the `utoipa_axum::OpenApiRouter`
  assembly (681-711), and `tower::util::oneshot` controller tests (732+). Every new controller
  should be a copy of this file's structure.
- **`crates/paladin-web/src/agent_controller.rs:441-579`** — a working SSE implementation
  (`SseEventStream`, `timed_event_stream`, `Sse::new(...)`) including the `text/event-stream`
  `utoipa` annotation. D-24/D-26's stream endpoint starts here.
- **`crates/paladin-web/src/job_store.rs`** — the existing fire-and-poll job pattern, and a useful
  contrast: its own module docs call jobs "ephemeral (lost on restart)" and point at the
  queue/worker topology as the durable answer. That topology is what this phase builds; the
  `JobStore` itself stays untouched under X-03.
- **`crates/paladin-battalion/src/engine/shutdown.rs`** — the `ShutdownCoordinator` D-13 registers
  workers with, already wired for Phase 24's background continuations.
- **`crates/paladin-ports/src/output/trace_sink_port.rs`** — `TraceSink` + `#[non_exhaustive]
  TraceEvent`, already engine-wired; D-24's bus is a sink implementation, not new plumbing. Its
  "errors are diagnostics only" doc section is the contract D-14's probe copies.
- **Dependency head start (corrected by research, 2026-09-08):** `hmac 0.12`, `croner` and `ipnet`
  are already resolved in `Cargo.lock` transitively, so promoting the ones we need (D-38, D-41,
  possibly D-42) adds no new tree and carries low MSRV risk. `schemars 1.2` is stronger still — a
  **direct** workspace dependency since Phase 26 (`Cargo.toml:143`), which is what flipped D-34.
  `jsonschema` is **not** in the lockfile and is no longer needed. X-11.1 still requires
  `cargo msrv verify` at **1.88** for each promotion.

### Established Patterns

- **Ports own the contract; contract suites own the proof.** Every multi-backend port in this tree
  ships one shared test suite the adapters are run through. D-08 leans on this explicitly: the
  contract suite, not the PRD, decides whether the Redis lease implementation is correct.
- **Typed structured errors, no new stringly variants** (X-06). Every error enum in the recent
  phases is `thiserror` + `#[non_exhaustive]` + named fields; D-02, D-07 and D-31 follow it.
- **Config structs default to disabled, and an unwired subsystem answers 501 rather than 500**
  (`src/config/waypoint_store.rs` + 24-CONTEXT D-24/D-26). D-50 and D-44 apply it to six new
  subsystems.
- **Builder-only construction on public state structs**, which is what makes adding a field to
  `ThreadApiState` a registered-but-cheap change rather than a breakage (D-45).
- **Two-tier HTTP authorization** — invocation gated per-target, registry mutation gated by
  `require_admin` — already established in `agent_controller`; D-46 extends it rather than inventing
  a scope system.
- **Tier 1 / Tier 2 test split, with Docker-dependent suites routed to CI/UAT and never marked
  passed locally** (24-CONTEXT D-28, reaffirmed in 25 and 26). D-51 carries it forward unchanged.

### Integration Points

- `src/bin/paladin-server.rs` + `crates/paladin-web/src/app.rs` — where the new router is merged,
  behind the same auth middleware, and where the worker pool and scheduler are started from config.
- `src/application/services/parley/{adapter,registry}.rs` — the `ParleyPort` adapter whose
  background-spawn becomes an enqueue (D-20), and the `GraphRegistry` the assistant registry replaces
  (D-33).
- `src/config/mod.rs` + `src/config/settings.rs` — where D-50's six new config structs register.
- `migrations/` — where D-03/D-36/D-40's tables land.
- `crates/paladin-web/openapi.json` — regenerated by every route change (D-48); the SHIP-02 golden
  diff later restricts itself to pre-existing paths, so new paths must be *added* without altering
  old ones.
- `.github/workflows/` — the `sdk-clients` job (D-49) and the MSRV job that must stay green as
  dependencies are promoted (D-53).
- `k8s/` + `docs/src/deployment-topologies/queue-worker.md` — PRD 06 §6 owes a worker-replica
  example; the queue/worker topology page already exists and is referenced by `job_store.rs`.

</code_context>

<specifics>
## Specific Ideas

- **PRD 06 §5's TDD ordering is not advisory here — it is the plan spine.** Status machine (pure) →
  queue contract suite (InMemory then Redis) → run repository contract suite → worker pool with
  InMemory everything and a mock engine → cancellation probe and cross-instance cancel → assistants
  and `WarGraphDoc` → scheduler under a paused clock → webhook delivery and SSRF → HTTP controllers
  via `oneshot` → full-lifecycle E2E and the SDK job. Each stage's tests are writable before the
  stage below it exists, which is what makes a phase this large tractable.
- **PRD acceptance 1 is the phase's real integration test** and should be written early as a
  failing skeleton: create a workflow assistant with a Gate → submit a run → SSE shows progress →
  the run suspends `AwaitingInput` → a mockito webhook receives the parley → resume via the API →
  the run completes → history and fork endpoints verified. Everything else exists to make that pass.
- **Two decisions deliberately depart from the PRD's literal text, both tightening rather than
  loosening, and both are flagged for a reviewer:** D-18 (`AwaitingInput` counts as busy for
  `409 ThreadBusy`) and D-42 (redirects disabled on the webhook client). Neither can break a stated
  acceptance test; both are visible API behavior.
- **The 250 ms p99 budget in PLAT-FR-01 is an architectural claim, not a benchmark to tune.** It
  holds because `POST /runs` does exactly two things — one insert and one enqueue — and touches no
  engine code. The test should assert the *absence* of engine work on that path as much as the
  latency number, since a latency assertion alone is flaky in CI.

</specifics>

<deferred>
## Deferred Ideas

- **The authoritative `TraceEvent` enum, `seq` ordering, the structured-log and OTel sinks, and the
  SSE bridge as a formal TraceSink adapter** — OBS-01/OBS-02 (Phase 28). D-25 freezes only the seven
  wire names and confines the mapping to one function so Phase 28 has a single edit point.
- **`WarGraphDoc → Mermaid/DOT` export and the execution overlay** — OBS-03 (Phase 28) visualizes
  the artifact D-33 defines.
- **Finer-grained scopes (a real `Writer` role, per-assistant ACLs, org-scoped RBAC)** — D-46 maps
  onto the existing two roles; PRD 06 §6 puts multi-tenant RBAC out of scope outright.
- **A multi-replica-safe auth token store** — ADR-0041 scopes the current in-process store to a
  single replica; this phase makes multi-replica deployment realistic and therefore makes the
  limitation newly visible, but no PLAT FR covers it.
- **Resolve-then-connect DNS-rebinding pinning for webhooks** — D-42 documents the gap as PRD 06
  PLAT-FR-15 permits; implementing it is a later security decision.
- **An append-only `run_status_history` table** — D-05 rejects it for now; additive later.
- **Generating the `WarGraphDoc` JSON Schema with `schemars` instead of hand-authoring it** — D-34's
  rejected alternative; revisit if the document's shape starts churning.
- **Horizontal autoscaling logic and replica orchestration** — PRD 06 §6; only a k8s worker-replica
  documentation example is owed.
- **Hand-polished Python/TypeScript SDKs** — PRD 06 §6; D-49 ships the generated-client CI gate
  only.
- **Resuming a `Failed` thread from its last good Waypoint after a fix** — carried from 25-CONTEXT;
  PLAT-02 resumes `Running`/`AwaitingInput` threads, and a `Failed` thread stays terminal.
- **Parley propagation through nested Battalions** — 24-CONTEXT D-04's deferral, unchanged and
  still unclaimed by any FR.
- **`MIGRATION.md` §9 finalisation, the v0.9-config boot test, the openapi golden diff, the program
  acceptance audit and the v0.10.0 version bump** — SHIP-01…04 (Phase 29). This phase *fills* its
  §9 rows (D-53); Phase 29 proves they are complete.
- **22-REVIEW.md WR-01/WR-02, 22-deferred-items.md item 1 (`qdrant` `--all-features` rustdoc break),
  24-REVIEW.md's advisory warnings, 25-REVIEW.md follow-ups** — unchanged, not this phase's.

### Reviewed Todos (not folded)

- "Verify local make coverage reproduces CI's 82.39% figure"
  (`.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md`, score 0.2, matched only on
  the keyword "local") — a local-tooling check owned by the maintainer, unrelated to the Platform
  API. Below the `--auto` fold threshold of 0.4; left in the todo list, as it was in Phases 24-26.

</deferred>

---

*Phase: 27-Platform API*
*Context gathered: 2026-09-07*
