# Phase 24: Pause/Resume, History & Graceful Shutdown - Context

**Gathered:** 2026-09-04
**Status:** Ready for planning
**Mode:** `--auto` (all gray areas auto-selected on recommended defaults; audit trail in 24-DISCUSSION-LOG.md)

<domain>
## Phase Boundary

Phase 24 delivers epic `HITL` (PRD 03) on top of the Phase 22/22.1/23 engine, and nothing from
later epics beyond the seams PRD 03 itself declares:

1. **Parley suspension (HITL-01).** `NextStep::Parley(ParleyRequest)` stops being the Phase 23
   typed failure (`EngineError::ParleyNotSupported`, `engine/superstep.rs:1321`) and becomes real
   suspension: the emitting superstep's deltas merge (peers complete normally), a Waypoint with
   `status: AwaitingInput` carrying **all** of the superstep's parleys is persisted, every task,
   timer and connection for the run is released, and the run returns `RunOutcome::AwaitingInput`.
   A first-class `NodeSpec::Gate` node always parleys and renders `prompt`/`payload` from the
   Battlefield with `InputMapping` templating. A suspended thread survives process termination and
   resumes from a different engine instance over the same `WaypointPort` backend; a partially
   answered suspension is queryable.
2. **Typed resume (HITL-02).** `WarEngine::resume_with(graph, thread, responses)` validates every
   response per `ParleyKind` (Approval/Choice/FreeText/StateEdit) with typed errors that leave the
   thread suspended, honours `expires_at` with an `on_expire` policy, and delivers values to the
   paused node's continuation. Program scenario **E2E-2** (approval gate, both branches, across a
   process drop/recreate) lands as an integration test in `tests/`.
3. **Chronicle (HITL-03).** `ChronicleService::{history, inspect, latest_on_branch}` over
   `WaypointPort`; `WarEngine::replay` and `WarEngine::fork`-with-edit create a new chain with
   `fork_of` lineage while the original chain stays byte-identical (hard invariant, tested);
   branch-aware latest resolution; defined subgraph-fork semantics (HITL-FR-12).
4. **Graceful shutdown (HITL-04).** Cancellation finishes the in-flight superstep within
   `shutdown_grace` (default 30 s); over-grace nodes are recorded `Skipped` and re-listed in the
   Halted Waypoint's vanguard; `resume` continues a `Halted` thread; SIGTERM/SIGINT are wired to
   every in-flight engine run in the facade with a documented disable switch; `k8s/` manifests and
   docs updated; `MIGRATION.md` M-B-02's worked example lands.
5. **Threads over HTTP (HITL-05).** `GET /v1/threads/{id}/state`, `POST /v1/threads/{id}/resume`
   (409/400/404), `GET /v1/threads/{id}/history` (paginated) in `paladin-web`, following the
   utoipa + error-envelope conventions of `agent_controller.rs`, `openapi.json` regenerated.

**Out of this phase** (later phases own them): retry/timeout/error handlers and the E2E-3 Aegis
half (Phase 25); middleware and `NodeInterceptor` visibility of routing (Phase 26); background run
submission, worker-pool re-enqueue of resumes, assistants/`WarGraphDoc` registry, opaque-cursor
pagination policy, admin/writer scopes, the Doc 06 expiry sweep (Phase 27); the authoritative
`TraceEvent` enum and trace consumers (Phase 28); `MIGRATION.md` §9.6 golden diff, §9.8 checklist
finalisation (Phase 29). Notification of a waiting parley is composed in application code with the
existing `paladin-notifications` — a **doc example is required**, no new port (PRD 03 §6). Any
other behavioral change discovered mid-implementation is an X-03 stop-and-flag event.

</domain>

<decisions>
## Implementation Decisions

PRD 03 is the FR-level source of truth and already locks the type sketches (§2.1/§2.2), the FR
semantics (HITL-FR-01…16), the acceptance criteria (§4) and the TDD ordering (§5). The decisions
below settle only what PRD 03 left open or what the shipped Phase 22/22.1/23 tree makes concrete.
Do not re-litigate anything PRD 03, PRD 01, PRD 02, overview §3 (X-01…X-11), or the Phase 22/22.1/23
CONTEXT decisions state.

### Parley payload & the multi-parley Waypoint shape (HITL-01, HITL-FR-03)
- **D-01: `ParleyRequest` is finalised in `paladin-core` beside the Phase 22 stub.** The stub at
  `crates/paladin-core/src/platform/container/waypoint.rs:452` (`{ prompt }`) becomes the PRD §2.1
  struct — `parley_id`, `node_id`, `kind: ParleyKind`, `prompt`, `payload: serde_json::Value`,
  `choices: Option<Vec<String>>`, `expires_at: Option<DateTime<Utc>>`, `created_at` — **plus**
  `on_expire: OnExpire` (`FailRun` default | `ResumeWithDefault(Value)`; HITL-FR-06 makes it
  per-request). `ParleyKind`, `ParleyResponse { parley_id, value, responded_by, responded_at }`
  and `OnExpire` land in the same module (core owns value types, ADR-0016); no new core dependency
  (`chrono`, `uuid`, `serde_json` are already core deps). `parley_id` is a serde-transparent
  **`ParleyId(Uuid)` newtype using UUIDv7**, mirroring `WaypointId` — the PRD's bare `Uuid` is
  satisfied on the wire. All new persisted types derive serde and are `#[non_exhaustive]` where
  they are enums. — **Reversibility:** one-way after v0.10.0 ships (the Waypoint payload is a
  stored contract); free now — the `MIGRATION.md` §9.2 `Waypoint` row already says "settle before
  first release, no compat needed".
- **D-02: AwaitingInput carries every parley and every accepted response.** The status becomes
  `WaypointStatus::AwaitingInput { parleys: Vec<ParleyRequest>, responses: Vec<ParleyResponse> }`;
  the Phase 22 single-`parley` field is replaced (the row above
  sanctions it). `parleys` is every request raised in the suspending superstep; `responses` is the
  accepted subset so far, so "partially answered" is a property of the persisted Waypoint, not of
  process memory (HITL-FR-02's survive-termination rule applies to partial answers too). The
  `AwaitingInput` Waypoint's `vanguard` is **exactly the parleying nodes** (see D-08), so the
  existing retention protection (`waypoint_retention.rs:57`, wildcard match) and resume plumbing
  work unchanged. `RunOutcome::AwaitingInput` becomes `{ parleys: Vec<ParleyRequest>, waypoint:
  WaypointId }` carrying only the still-unanswered requests. The three-backend contract suite
  gains round-trip cases for the new payload (22.1 D-23 pattern); the SQL `status` column already
  stores the serialized status, so no migration. — **Reversibility:** one-way after release.
- **D-03: The parleying node is observable on the Waypoint.** A new `NodeOutcomeKind::Parleyed`
  variant (`NodeOutcomeKind` is `#[non_exhaustive]` and new in 0.10) records the raise in the
  suspending superstep's `completed` records, mirroring how `Ended` (23 D-09) made the run-ending
  node observable. The emitting node's own `Directive.delta` **merges** at raise time (it emitted
  it); the documented contract for Function/Paladin parleys is "idempotent up to the parley point"
  (PRD HITL-FR-04) because the node re-runs on resume (D-08).
- **D-04: A parley inside a nested Battalion child is a typed failure this phase.** PRD 03 is
  silent on suspension propagating through `NodeSpec::Battalion`; the current arm
  (`superstep.rs:475`) already refuses it with a stringly `NodeError`. Replace that with a
  structured `EngineError::ParleyInChildUnsupported { node, child_thread }` (X-06), document it on
  `NodeSpec::Battalion` and the Gate rustdoc, cover it with a test, and record the propagation
  design under Deferred Ideas for the developer to promote if wanted. Rejected: designing
  parent-suspension-through-child now — a second lineage/resume protract with no FR behind it.

### Gate node & response delivery (HITL-FR-01, HITL-FR-04)
- **D-05: A first-class Gate node variant.** `NodeSpec::Gate { request: GateRequestTemplate,
  output_field: Option<FieldName> }` on the already-`#[non_exhaustive]` enum.
  `GateRequestTemplate { kind, prompt_template: InputMapping,
  payload_template: Option<InputMapping>, choices, expires_in: Option<Duration>, on_expire }` —
  `expires_at = now + expires_in` is stamped at raise time. `output_field` is **required** for
  `Approval`/`Choice`/`FreeText` and **must be `None`** for `StateEdit`; `WarGraph::validate`
  rejects the other combinations and checks the field exists in the schema with a compatible type
  (Approval → Bool or String; Choice/FreeText → String) via typed `EngineError` variants. A Gate
  has no `run` body of its own: on first visit it raises, on the post-resume visit it writes the
  normalised value (D-06) to `output_field` (or returns the `StateEdit` delta) and routes via
  static edges — so an approval gate is Gate + two conditional edges, exactly as the PRD says.
- **D-06: Edge evaluation treats a Gate like a Paladin node.** The engine-path `EdgeContext.output`
  for a Gate source is its `output_field` value (23 D-02's Paladin rule), so `Contains("true")` /
  `Regex` / registered evaluators work on the delivered value. Approval values are normalised to
  JSON `true`/`false` before delivery (HITL-FR-05: `true`/`false`, `"yes"/"no"/"approve"/"deny"`
  case-insensitive); when `output_field` is a `String` field the bool is written as `"true"`/
  `"false"`.
- **D-07: Paladin parleys use the Directive envelope and a parley namespace.** Paladin nodes raise
  a parley through the `StructuredDirective` envelope and receive the answer through a `parley.`
  InputMapping namespace. The 23 D-11 envelope gains `"next":
  {"parley": {"kind": "...", "prompt": "...", "payload": {...}, "choices": [...],
  "expires_in_secs": N}}` (the parser fills `parley_id`, `node_id`, `created_at`). On the
  post-resume re-run, `InputMapping::render` resolves `{parley.value}`, `{parley.prompt}`,
  `{parley.kind}` and `{parley.responded_by}` from `NodeContext`, **never** from the Battlefield,
  and graph validation rejects schema fields starting with `parley.` — the exact 23 D-15 `muster.`
  rule, second namespace. `NodeContext` gains `parley_response: Option<ParleyResponse>` with a
  `parley_response()` accessor (PRD HITL-FR-04).
- **D-08: Resume is an ordinary superstep whose Vanguard is exactly the parleying nodes.** Once
  every parley is answered, the engine seeds superstep N+1 from the `AwaitingInput` Waypoint with
  `vanguard = parleying nodes`, each node's `NodeContext.parley_response` populated, and runs the
  normal superstep loop: Gates write their value (D-05), Function/Paladin nodes re-run (D-03),
  deltas merge, edges resolve, one Waypoint per superstep (ENG-FR-11 holds with no clarification).
  Responses are **durably consumed only when the first post-resume Waypoint persists** — if the
  process dies between validation and that write, the `AwaitingInput` Waypoint is still the latest
  and re-submitting the same responses is safe. Rejected: a dedicated "merge responses" pseudo-
  superstep (a second Waypoint kind with no node records).
- **D-09: `GRAPH_FINGERPRINT_VERSION` bumps `v3` → `v4`.** New sorted, length-prefixed section per
  Gate node: `kind`, `output_field`, `choices`, `on_expire` kind — everything that changes routing
  or merge; `prompt_template`/`payload_template`/`expires_in` are excluded like `InputMapping`
  templates (ENG-FR-14). Golden hex re-pinned, one difference test per hashed property (23 D-18
  pattern). — **Reversibility:** one-way after v0.10.0 ships; free now.

### Resume validation, partial answers & expiry (HITL-02, HITL-FR-05, HITL-FR-06)
- **D-10: Validation is total before any state changes.** `resume_with` loads `latest(thread)`,
  checks `GraphMismatch` (ENG-FR-14), requires `AwaitingInput` (else
  `EngineError::ThreadNotAwaitingInput { thread, status }`), then validates **every** submitted
  response against its request before persisting anything: `UnknownParleyId { parley_id }`,
  `ParleyAlreadyAnswered { parley_id }` (new — X-06), `ResponseShapeInvalid { parley_id, reason }`
  (Approval/Choice/FreeText/StateEdit matrix; `StateEdit` deserialises to a `StateDelta` validated
  against the schema, unknown field rejects the response not the run), `ParleyExpired {
  parley_id, expires_at }`. Any error leaves the thread suspended with no Waypoint written.
- **D-11: Partial answers persist as a new `AwaitingInput` Waypoint.** A valid subset writes a child
  Waypoint at the **same superstep** with `responses` extended and returns
  `RunOutcome::AwaitingInput` listing the remaining parleys (acceptance 2: answering one of two
  keeps suspension, state + status assert it). Plain `WarEngine::resume` on an `AwaitingInput`
  thread fails with `EngineError::ThreadAwaitingInput { thread, parleys }` — it never guesses.
- **D-12: Expiry policy semantics.** Evaluated lazily at resume time against `Utc::now()` (no
  timer; the Doc 06 sweep is Phase 27). `on_expire: FailRun` → a `Failed` Waypoint is persisted
  (structured reason naming the parley) and `resume_with` returns `Err(ParleyExpired)`; the thread
  is thereafter resumable only by `replay`/`fork` from an earlier Waypoint. `ResumeWithDefault(v)`
  → `v` is validated per kind at **graph-validate time** (a Gate) or at raise time (a Directive),
  substituted as the response with `responded_by: None`, and the substitution is recorded on the
  response (`defaulted: true`, Claude's discretion on the exact field) so an audit can see it.
- **D-13: `expires_at` in tests is set in the past explicitly** — no clock abstraction is added
  this phase (Phase 25's paused-clock harness may generalise later).

### Chronicle lineage & fork identity (HITL-03, HITL-FR-07…12)
- **D-14: `fork_of` marks the branch root and propagates along the branch.** `Waypoint` gains
  `#[serde(default)] fork_of: Option<WaypointId>` (additive, no migration, contract-suite round
  trip). The fork's first Waypoint has `parent_waypoint_id = Some(from)` and `fork_of = Some(from)`;
  **every subsequent Waypoint on that branch inherits `fork_of = Some(from)`**; mainline Waypoints
  carry `None`; a fork of a fork carries the newer `from`. So a branch is a queryable attribute,
  `latest_on_branch(thread, branch_root)` is a filter over `history`, and the tree is
  reconstructible from `WaypointSummary` alone — `WaypointSummary` gains `fork_of` (new type, no
  X-10 row). — **Reversibility:** one-way after release (stored payload).
- **D-15: Latest stays newest-across-branches, and that is now load-bearing.** `latest(thread)`
  keeps returning the most recently created Waypoint across branches. All three backends already
  order by `created_at DESC, superstep DESC`
  (`in_memory.rs:71`, `postgres.rs:257`, SQLite likewise); add a contract-suite case asserting a
  fork's newest Waypoint wins over a later-superstep mainline Waypoint, on all three. Document on
  `WarEngine::resume` that resume-without-branch-qualifier resumes the newest branch (HITL-FR-11).
  Retention treats every branch's Waypoints as ordinary members of the thread (latest and
  `AwaitingInput` protected as today).
- **D-16: Replay and fork on the engine, Chronicle reads in the facade.** `replay`/`fork` live on
  `WarEngine`; `ChronicleService` is a facade application service.
  `WarEngine::replay(graph, thread, from) -> Result<RunOutcome, EngineError>` and
  `WarEngine::fork(graph, thread, from, edit: StateDelta)` re-enter `superstep::run` from
  `get(thread, from)` with `parent = from`, `fork_of = Some(from)`, superstep numbering continuing
  from `from.superstep + 1`, fingerprint checked (ENG-FR-14) and the edit merged through the
  schema's dispatch rules **before** the first forked superstep. Typed errors: `WaypointNotFound
  { thread, waypoint }`, `ForkFromFailed`-style guards at Claude's discretion. `ChronicleService`
  (`src/application/services/chronicle.rs`, beside `waypoint_retention.rs`) exposes `history`
  (newest-first summaries with lineage), `inspect` (full Waypoint) and `latest_on_branch` over
  `Arc<dyn WaypointPort>` — no engine dependency, so `paladin-web` can reuse the same reads
  through the port (D-24).
- **D-17: Immutability is asserted byte-for-byte.** The replay test serialises every mainline
  Waypoint before and after `replay` and asserts equality (acceptance 3); the fork-with-edit test
  flips a conditional edge (acceptance 4).
- **D-18: Subgraph forks never share child Waypoints.** A branch runs its `NodeSpec::Battalion`
  children under a child thread id derived from the parent thread **and the branch root**
  (`ThreadId::child_on_branch(parent, branch_root, node)` — same length-prefixed, injective
  encoding as `ThreadId::child`, 22.1 CR-01 rule), so `latest(child_thread)` on a fork never
  resolves the mainline child's history and the mainline child is untouched (HITL-FR-12, tested).
  Mainline runs keep `ThreadId::child` unchanged.

### Shutdown grace mechanics & process wiring (HITL-04, HITL-FR-13…15)
- **D-19: Grace is enforced inside the in-flight superstep.** Cancellation first seen mid-flight
  starts the grace window at that moment. Today cancellation is observed only at the superstep
  boundary (`superstep.rs:893`).
  Add a second observation: while awaiting the superstep's spawned node tasks, if the token fires,
  keep awaiting them until `cancel_observed_at + shutdown_grace`; tasks still running at the
  deadline are **aborted** (`JoinHandle::abort`), recorded `NodeOutcomeKind::Skipped { reason:
  "shutdown" }`, their deltas discarded, and their ids **re-listed in the Halted Waypoint's
  vanguard** alongside the normally computed next Vanguard; nodes that finished in time merge
  normally and their edges resolve. Skipped nodes' outgoing edges stay `Pending` in the
  `FrontierSnapshot` so resume re-executes them **exactly once** (acceptance 5). The existing
  boundary check is unchanged. `shutdown_grace = Duration::ZERO` aborts immediately. Mechanism
  inside `superstep.rs` is Claude's discretion; the contract above is locked.
- **D-20: Shutdown grace is a runtime setting, not a graph setting.**
  `WarEngine::with_shutdown_grace(Duration)` (default 30 s) is the engine knob. It is **not** part
  of `EngineLimits` and never hashed. `EngineConfig`
  (`src/config/engine.rs`, X-09) gains `shutdown_grace_secs: u64` (default `30`, env
  `APP_ENGINE_SHUTDOWN_GRACE_SECS`) and `graceful_shutdown: bool` (default `true`, env
  `APP_ENGINE_GRACEFUL_SHUTDOWN`) — the latter is the **M-B-02 disable switch** ("legacy-only
  deployments": the process exits without waiting). One config struct feeds both the engine and
  the process wait; `Settings` (pre-existing, all-pub, not `#[non_exhaustive]`) is **not** touched,
  following the Phase 22/23 precedent (`EngineConfig`/`WaypointRetentionConfig` are standalone,
  `src/config/mod.rs:46,52`). — **Reversibility:** costly — env names and defaults are
  operator-facing once documented in §9.5.
- **D-21: The shutdown coordinator lives in the battalion engine module.** `ShutdownCoordinator`
  and `RunGuard` go in `paladin-battalion::engine::shutdown`. A root
  `tokio_util::sync::CancellationToken`, an in-flight counter and a `Notify`; `register()`
  returns a child token + RAII guard; `cancel_and_wait(grace)` cancels the root and waits until
  idle or the deadline. Placed in battalion (not the facade) so embedded-library users and Phase
  27's worker pool reuse it. X-05 stress test with exact counts and a timeout guard.
- **D-22: Both process entry points wire it.** `src/bin/paladin-server.rs::shutdown_signal` and
  `ServiceRunner::wait_for_shutdown` (`service_runner.rs:253`) cancel the coordinator on
  SIGTERM/SIGINT, then wait ≤ `shutdown_grace` (skipped when `graceful_shutdown = false`) before
  axum's `with_graceful_shutdown` completes / the runner exits. Every in-flight engine run the
  facade starts — today only the background continuation spawned by the resume port (D-25) —
  registers with it. `resume` on a `Halted` thread is tested explicitly (HITL-FR-14).
- **D-23: Operator surface.** `terminationGracePeriodSeconds: 60` (= 2 × 30 s) added to
  `k8s/server/deployment.yaml` and `k8s/deployment.yaml`; `k8s/README.md`,
  `docs/src/deployment/kubernetes.md` and `docs/src/deployment/production.md` (currently shows
  `30`) updated with the 2× rule and the env switch; `MIGRATION.md` §9.1 M-B-02 worked example
  (before: no wait, `terminationGracePeriodSeconds` unset; after: 60 s, `APP_ENGINE_SHUTDOWN_
  GRACE_SECS=30`, `APP_ENGINE_GRACEFUL_SHUTDOWN=false` to opt out), §9.5 new fields, §9.8's
  "adjust termination grace" bullet made concrete.

### HTTP surface placement & resume semantics (HITL-05, HITL-FR-16)
- **D-24: Thread routes get their own state and router; `AgentApiState` is untouched.** New
  `ThreadApiState { waypoints: Option<Arc<dyn WaypointPort>>, parley: Option<Arc<dyn
  ParleyPort>>, auth }` and `thread_router(state)` nested under `API_V1_PREFIX` (ADR-0037) and
  merged by `paladin-server` behind the **same** auth middleware as `/v1/agents/*` (authenticated
  callers, any role; scopes are PLAT-06). This avoids adding public fields to the pre-existing
  `AgentApiState` (an X-10.3 event) — a **deliberate zero** for `paladin-web` in §9.2.
  `GET …/state` and `GET …/history` read `WaypointPort` directly (state = latest summary + `parleys`
  + `responses` when suspended); DTOs are `utoipa::ToSchema` types in `paladin-web` (ADR-0038).
  When the deployment has no waypoint backend wired, every thread route answers **501
  `not_implemented`** naming the config to set; the spec still lists the paths. Rejected:
  `paladin-web` depending on `paladin-battalion` (a default-build leaf-to-leaf edge, ADR-0031).
- **D-25: Resume goes through a port; the facade runs the continuation in the background.** New
  `ParleyPort` in `paladin-ports` (input side; depends only on core types): `async fn resume_with(
  &self, thread: &ThreadId, responses: Vec<ParleyResponse>) -> Result<ResumeAccepted,
  ParleyError>` where `ParleyError` mirrors the D-10 variants plus `ThreadNotFound` and
  `GraphNotRegistered`. The facade adapter (`src/application/services/parley/…`) validates
  synchronously via `WarEngine::resume_with`'s validation path, then **spawns the continuation as
  a background task registered with the `ShutdownCoordinator` (D-21)** and returns immediately.
  `POST /v1/threads/{id}/resume` therefore returns **202 Accepted** `{ thread_id, state_url }`;
  clients poll `GET …/state`. Status mapping: 404 `ThreadNotFound`; 409 `conflict` for
  `ThreadNotAwaitingInput` **and** `GraphNotRegistered` (distinct `code`s in the envelope); 400
  `bad_request` for `UnknownParleyId`/`ParleyAlreadyAnswered`/`ResponseShapeInvalid`/
  `ParleyExpired` (details carry `parley_id`); 501 when unwired. Rejected: running the resume
  inline and holding the connection (defeats "release every resource" and gives HITL-FR-15 nothing
  to protect). — **Reversibility:** costly — a published HTTP contract Phase 27 re-enqueues under
  the same shape (PLAT-03).
- **D-26: The graph for a thread is resolved by fingerprint from a code-registered registry.** The
  facade adapter holds a `GraphRegistry` keyed by `GraphFingerprint` (the latest Waypoint carries
  it); unregistered → `GraphNotRegistered`. Phase 27's `WarGraphDoc`/assistant registry replaces the
  lookup behind the same port. `paladin-server` gets a minimal X-09 `WaypointStoreConfig`
  (`backend: disabled | sqlite { path } | postgres { url env }`, default disabled → 501) so a
  durable backend can be wired; it registers no graphs itself (HTTP agents are LLM + prompt only,
  ADR-0039), so end-to-end HTTP resume against a real graph is proven in `paladin-web`'s
  `tower::util::oneshot` tests with an in-test registry, not in the binary.
- **D-27: History pagination is `limit` + opaque `cursor`.** `GET …/history?limit=20&cursor=…`
  (limit ≤ 100), response `{ items: [WaypointSummary incl. fork_of], next_cursor }`; the cursor's
  content is the last returned `waypoint_id` (documented as opaque to clients) so PLAT-06's
  opaque-cursor rule holds without a later breaking change. `openapi.json` regenerated and
  diff-reviewed; `MIGRATION.md` §9.6's endpoint list filled (the golden diff stays SHIP-02's).

### Test placement, tiers & program-gate obligations
- **D-28: Tiers.** Everything HITL adds is Tier 1 (`CountingFunctionNode`, `RecordingPaladinPort`,
  `InMemoryWaypointStore`, `SqliteWaypointStore` on a temp file, `MockLlmAdapter`). Cross-process
  = two `WarEngine` instances over one SQLite file (PRD §5 item 3). The Postgres contract-suite
  additions (D-02, D-14, D-15) are Tier 2 via CI's `postgres-integration` job — Docker is
  unavailable in the devcontainer, so route them to UAT, never mark them passed locally (STATE.md
  carried concern). E2E-2 is `tests/integration/e2e_approval_gate_test.rs` with a `[[test]]` entry
  mirroring `e2e_crash_resume`; the X-05 stress test is acceptance 7 (10 suspended threads resumed
  concurrently, exact outcomes, `flavor = "multi_thread"`, timeout guard).
- **D-29: `.project/` and `MIGRATION.md` registrations.** §9.2: resolve the `Waypoint` row
  ("`AwaitingInput { parleys, responses }` — landed HITL-01, Phase 24") and add a deliberate-zero
  note for every other type touched (`RunOutcome`, `EngineError`, `NodeSpec`, `NodeContext`,
  `NodeOutcomeKind`, `WaypointSummary`, `TraceEvent`-untouched, `ThreadApiState` new) — all absent
  at `v0.9.0`; the new `ParleyPort` trait is new. §9.1 M-B-02 example (D-23), §9.5 (D-20, D-26),
  §9.6 (D-27). `08-traceability-matrix.md` G-05/G-06/G-09/G-15/G-26 rows gain their test anchors.
  `cargo semver-checks` (vs 0.9.0), `msrv` (1.88), `make security`, `cargo clippy -- -D warnings`
  and coverage ≥ 82% (ADR-0006) green on the phase's final commit.
- **D-30: Docs (X-08).** New mdBook page `docs/src/user-guides/parley-and-chronicle.md` (Gate
  approval gate, resume, partial answers, expiry, history/replay/fork, the `paladin-notifications`
  composition example PRD 03 §6 requires, graceful shutdown from the embedder's side), wired into
  `SUMMARY.md` after the control-flow page; deployment pages per D-23; doc-tests on every new public
  API; `cargo doc` with no new broken intra-doc links; `CHANGELOG.md` `[Unreleased]`.

### Claude's Discretion
- Module layout: whether Parley types get `platform::container::parley.rs` or extend
  `waypoint.rs`; the `engine::shutdown` file name; the facade parley/chronicle module split.
- Exact `EngineError`/`ParleyError` variant names and messages beyond those named above; the
  `GateRequestTemplate` builder; how `Gate` is dispatched inside `superstep.rs` (a built-in
  `StateNode` impl vs a dedicated dispatch arm); the `defaulted` marker shape (D-12).
- `ThreadId::child_on_branch` naming and whether `restart_on_resume` semantics change on a fork
  (recommended: a fork always starts children fresh, by construction of D-18).
- `ThreadApiState`/`ParleyPort`/`ResumeAccepted`/`GraphRegistry` names; the `state` DTO field set
  beyond `thread_id`, `status`, `superstep`, `waypoint_id`, `parleys`, `responses`, `fork_of`.
- Whether `TraceEvent` gains `ParleyRaised`/`RunHalted` — recommended **no** (Phase 28 owns the
  authoritative enum; `RunFinished` suffices) — and whether `BATTLEFIELD_SCHEMA_VERSION` bumps for
  the additive fields (follow the `visit_counts` precedent).
- Plan/wave decomposition, respecting PRD 03 §5's TDD order. Suggested: (1) Parley types,
  `AwaitingInput` payload, validation matrix units, contract-suite cases; (2) engine suspension +
  `resume_with` + partial answers + expiry on InMemory then SQLite, Gate node, `parley.` namespace,
  envelope, fingerprint `v4`, `ParleyInChildUnsupported`; (3) E2E-2 + multi-parley + cross-process
  + stress; (4) Chronicle: `fork_of`, `replay`/`fork`, `ChronicleService`, `child_on_branch`,
  immutability/subgraph-fork tests; (5) shutdown grace in `superstep.rs`, `ShutdownCoordinator`,
  `EngineConfig` fields, `paladin-server`/`ServiceRunner` wiring, k8s/docs, M-B-02 example; (6)
  `ParleyPort`, facade adapter + `GraphRegistry` + `WaypointStoreConfig`, `ThreadApiState` +
  routes + DTOs + oneshot tests, `openapi.json`, §9.6; (7) mdBook page, MIGRATION/CHANGELOG/
  traceability sweep, CI evidence. (2) precedes (3)–(6); (4), (5) and (6) are independent of each
  other except (6)'s dependency on (5)'s coordinator.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase source of truth (behavior)
- `.project/v0.10.0/03-pause-resume-history-shutdown.md` — **The FR-level source of truth for this
  phase.** §2.1 Parley types, §2.2 fork lineage, HITL-FR-01…16, §4 acceptance criteria 1-8, §5 TDD
  ordering, §6 out of scope (notification doc example required, no port). Every plan task traces
  to an FR here.
- `.project/v0.10.0/00-program-overview.md` — §3 X-01…X-11 (X-03 stop-and-flag; X-05 stress
  pattern; X-06 structured errors; X-09 config structs; X-10 public-type discipline — `Settings`
  and `AgentApiState` are the pre-existing structs D-20/D-24 avoid touching), §4 ubiquitous language
  (Parley, Chronicle, Vanguard, Waypoint, Thread), §6 **E2E-2** (the scenario this phase must pass
  as an integration test), §9.1 **M-B-02** (worked example owed here), §9.2 the `Waypoint` row,
  §9.5/§9.6/§9.8 content this phase appends.
- `.project/v0.10.0/01-battlefield-state-and-execution-engine.md` — ENG-FR-11 (one Waypoint per
  superstep — D-08 keeps it unclarified), ENG-FR-12/12a (resume, `FrontierSnapshot` — D-19's
  Pending-edge rule), ENG-FR-14 (fingerprint contents/exclusions — D-09), ENG-FR-18 (retention
  protects `AwaitingInput`), ENG-FR-23 (cancellation seam D-19 extends), §8 seams.
- `.project/v0.10.0/02-control-flow-routing-fanout-subgraphs.md` — CF-FR-05…08 (`Directive`/
  `NextStep::Parley`, `DirectiveParser` envelope D-07 extends), CF-FR-14…17 (subgraphs D-04/D-18
  interact with).
- `.project/v0.10.0/06-platform-api.md` — §2.1 thread endpoints and PLAT-FR-06 (Phase 27
  re-enqueues `POST /threads/{id}/resume` under the same run — D-25's 202 shape must not preclude
  it), PLAT-FR-16 pagination rule (D-27 pre-conforms).
- `.project/v0.10.0/08-traceability-matrix.md` — G-05, G-06, G-09, G-15, G-26 rows; protocol
  steps this phase's tests must be findable by.
- `.planning/REQUIREMENTS.md` — HITL-01…05 capability clusters with FR ranges; the X-10/X-11
  versioning gate as part of every requirement's definition of done.
- `.planning/ROADMAP.md` — Phase 24 goal, dependencies (22, 23), the five success criteria;
  Phase 25 depends on this phase.

### Program deliverable this phase appends to
- `MIGRATION.md` — §9.1 M-B-02 row (line 16) and its "TBD — owner HITL-04, Phase 24" example
  (line 70); §9.2 `Waypoint` row (line 90) and the Phase 23 deliberate-zero note (line 94) whose
  form D-29 follows; §9.5 `EngineConfig` bullet list (lines 121-126) D-20 extends; §9.6 (line 139)
  D-27 fills; §9.8 (line 147) "termination grace lands with HITL-04".

### Prior-phase decisions that constrain this phase
- `.planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-CONTEXT.md` — D-02
  (engine `EdgeContext.output` rule D-06 reuses), D-07 (`StateNode::run` → `Directive`), D-08/D-09
  (Goto/End vs truthful-outcome machinery — a resume superstep must not trip
  `StarvedNodeAtCompletion`), D-10 (`ParleyNotSupported` arm this phase replaces), D-11 (envelope
  D-07 extends), D-14 (intra-superstep progress Waypoints — D-11's same-superstep partial-answer
  Waypoint is the second such precedent), D-15 (`muster.` namespace rule D-07 mirrors), D-16
  (`EngineConfig` shape D-20 extends), D-18 (fingerprint `v3` D-09 bumps), D-20/D-21
  (`ThreadId::child`, child inherits engine wholesale, child observes cancellation — D-18/D-21),
  D-26 (code-configured, off by default — D-24/D-26's 501/disabled defaults follow it).
- `.planning/phases/22.1-engine-readiness-defect-and-msrv-follow-up/22.1-CONTEXT.md` — D-04
  (truthful-outcome check), D-15…D-19 (fingerprint discipline), D-21…D-25 (`FrontierSnapshot`
  on the Waypoint, additive `#[serde(default)]`, contract-suite round trips — D-02/D-14 pattern),
  CR-01 delimiter lesson (D-18's encoding rule).
- `.planning/phases/22-battlefield-state-superstep-engine/22-CONTEXT.md` — D-01/D-02 (backends
  and JSON payload columns — no migration for D-02/D-14), D-09/D-10 (contract suite style, Postgres
  Tier 2), D-11 (seeded-shuffle harness), D-12 (snapshot isolation D-08/D-19 preserve).

### Standing decisions and governance
- `.planning/decisions/0006-coverage-gate.md` (ADR-0006) — 82% workspace floor.
- `.planning/decisions/0015-core-ports-dependency-allowlist.md` (ADR-0015) — `ParleyPort` and
  the Parley value types add no forbidden dependency; `utoipa` stays in `paladin-web`.
- `.planning/decisions/0016-port-value-type-ownership.md` (ADR-0016) — core owns `ParleyRequest`/
  `ParleyResponse`/`ParleyKind`/`OnExpire`; ports re-export.
- `.planning/decisions/0031-extracted-crate-dependency-rule.md` (ADR-0031) — why `paladin-web`
  may not depend on `paladin-battalion` in its default build (D-24/D-25).
- `.planning/decisions/0037-agent-route-surface-v1.md` (ADR-0037) — thread routes are `/v1`-
  prefixed; `crates/paladin-web/openapi.json` is the drift-guard baseline D-27 regenerates.
- `.planning/decisions/0038-agent-provisioner-placement.md` (ADR-0038) — HTTP DTOs stay in
  `paladin-web`; a port's parameter types must be core types (the shape `ParleyPort` follows).
- `.planning/decisions/0039-http-topology-no-garrison-no-arsenal.md` (ADR-0039) — HTTP-served
  agents are LLM + prompt only; `paladin-server` registers no `WarGraph`s (D-26).
- `.github/instructions/security.instructions.md` — the `state` endpoint returns Waypoint-derived
  content that may contain raw prompts/outputs (M-B-04's warning); `payload` is author-controlled;
  no API key may reach a parley payload, log or error.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/paladin-core/src/platform/container/waypoint.rs` — `ParleyRequest` stub (line 452),
  `WaypointStatus::AwaitingInput { parley }` (line 473) and `Halted` (478), `NodeOutcomeKind`
  (`#[non_exhaustive]`, `Skipped { reason }` at 412, `Ended`), `Waypoint` with the additive
  `#[serde(default)]` precedents (`visit_counts`, `frontier`, `muster_progress`, `checkpoint_ns`),
  `ThreadId::child` (line 144, length-prefixed encoding D-18 copies), `WaypointId` (UUIDv7 —
  `ParleyId` copies it).
- `crates/paladin-core/src/platform/container/directive.rs` — `NextStep::Parley(ParleyRequest)`
  (line 77) with the rustdoc promising Phase 24 replaces the failure arm.
- `crates/paladin-battalion/src/engine/superstep.rs` — the `NextStep::Parley` arm (line 1321,
  `ParleyNotSupported`), the child `AwaitingInput` arm (line 475, stringly `NodeError` D-04
  replaces), the boundary cancellation check (line 893) and `build_waypoint`/`persist_waypoint`
  D-19 extends, `tokio::spawn` + `Semaphore` node dispatch (lines 1092-1206 — the `JoinHandle`s
  D-19 aborts), `Frontier::for_run`/`snapshot`, `starved_at_completion` (D-08 must not trip it).
- `crates/paladin-battalion/src/engine/mod.rs` — `RunOutcome::AwaitingInput { parley, waypoint }`
  (line 113, reshaped by D-02), `EngineError` (`#[non_exhaustive]`, line 146; `ThreadNotFound`,
  `GraphMismatch`, `ParleyNotSupported` at 269), `ResumeOptions`, `WarEngine` builders
  (`with_cancellation_token` at 750 — `with_shutdown_grace` sits beside it), `start`/`resume`/
  `resume_with_options` (765-900 — `resume_with`/`replay`/`fork` follow their shape; the
  `Completed` short-circuit at ~890 must gain `AwaitingInput`/`Halted` handling per D-11/D-22).
- `crates/paladin-battalion/src/engine/graph.rs` — `NodeSpec` (`#[non_exhaustive]`; `Paladin` with
  `output_field`/`directive_parser`, `Battalion`) D-05 extends; `validate` (dispatch resolver +
  evaluator registry) gains Gate checks; `fingerprint()` `v3` → `v4` (D-09).
- `crates/paladin-battalion/src/engine/node.rs` — `NodeContext { node_id, thread_id, superstep,
  muster }` (D-07 adds `parley_response`), `StateNode`.
- `crates/paladin-battalion/src/engine/input_mapping.rs` — `render` with the `muster.` namespace
  resolution (line 148) D-07 mirrors for `parley.`.
- `crates/paladin-battalion/src/engine/directive_parser.rs` — the envelope (line 15) and its
  `next` deserialisation (102-113) D-07 extends with `{"parley": {...}}`.
- `crates/paladin-battalion/src/engine/hooks.rs` — `TraceDispatcher`, `NodeInterceptor`
  (untouched); `tokio_util::sync::CancellationToken` already a battalion dependency (D-21).
- `crates/paladin-battalion/src/engine/test_support.rs` — `CountingFunctionNode`,
  `RecordingPaladinPort`, `RecordingWaypointStore` (`fail_next_save` — D-08's durable-consumption
  test), `RecordingTraceSink`, `shuffle_seeded`.
- `crates/paladin-ports/src/output/waypoint_port.rs` — `WaypointPort` (`save`/`latest`/`get`/
  `history(thread, limit, before)`/`list_threads`/`prune_thread`), `WaypointSummary` (D-14 adds
  `fork_of`), `WaypointError`; `crates/paladin-ports/src/input/` — where `ParleyPort` goes;
  `output/paladin_executor_port.rs` — the injection-only port shape `paladin-web` already consumes.
- `crates/paladin-storage/src/waypoint/{in_memory,sqlite,postgres,contract_tests}.rs` — `latest`
  ordering (`created_at DESC, superstep DESC`, D-15), the 30-case contract suite D-02/D-14/D-15
  extend, per-backend `#[tokio::test]` wrappers.
- `src/application/services/waypoint_retention.rs` — `AwaitingInput { .. }` protection (line 57,
  unchanged by D-02); the application-service shape `ChronicleService` mirrors.
- `src/config/engine.rs` — `EngineConfig` (`Default`, `validate`, `EnvOverridable`, `APP_ENGINE_*`,
  `From<EngineConfig> for EngineLimits`) D-20 extends; `src/config/waypoint_retention.rs` — the
  second X-09 template for `WaypointStoreConfig` (D-26).
- `crates/paladin-web/src/agent_controller.rs` — `AgentApiState` (line 58, all-pub, not
  `#[non_exhaustive]` — untouched by D-24), `#[utoipa::path]` conventions (232+), `202 Accepted`
  job pattern (line 592-646, the precedent for D-25), `API_V1_PREFIX`/`versioned_agent_parts`/
  `agent_router` (723-741 — `thread_router` nests the same way), `tower::util::oneshot` tests
  (1224+). `crates/paladin-web/src/error.rs` — `ApiError::{not_found, conflict, bad_request,
  not_implemented}` and the `ApiErrorBody` envelope D-25 maps onto. `crates/paladin-web/src/
  openapi.rs` — `build_openapi`/`openapi_spec` (thread paths must be registered there for the
  committed `openapi.json`).
- `src/bin/paladin-server.rs` — `shutdown_signal()` + `axum::serve(...).with_graceful_shutdown`
  (line ~132) and the `AgentApiState` composition (lines 60-75) D-22/D-24/D-26 extend;
  `src/config/setup/service_runner.rs` — `wait_for_shutdown` (line 253) D-22 extends.
- `k8s/server/deployment.yaml` (the real `paladin-server` Deployment, no
  `terminationGracePeriodSeconds` today) and `k8s/deployment.yaml`; `k8s/README.md`;
  `docs/src/deployment/{kubernetes,production}.md` (`production.md:351` shows 30 s — D-23 updates).
- `tests/integration/e2e_crash_resume_test.rs` + `Cargo.toml:259-262` — the E2E template and
  `[[test]]` registration E2E-2 copies; `tests/integration/subgraph_formation_in_campaign_test.rs`
  — resume-mid-child fixture D-18's subgraph-fork test can extend.
- `docs/src/SUMMARY.md:23` — the control-flow page entry the new Parley/Chronicle page follows.

### Established Patterns
- Additive `#[serde(default)]` Waypoint fields with no SQL migration + a three-backend contract
  round-trip case (D-02/D-14); JSON payload columns so status shape changes need no DDL.
- Fingerprint discipline: hash everything that changes scheduling or merge, sorted,
  length-prefixed, golden-pinned, version tag bumped on layout change (D-09).
- Typed `EngineError` variants, `thiserror`, `#[non_exhaustive]`, no new stringly variants (X-06)
  — D-04 removes one.
- Config structs standalone under `src/config/` with `Default`/`validate`/`EnvOverridable`
  (`APP_*`), never fields on `Settings` (X-10 avoidance, Phase 22/23 precedent) — D-20/D-26.
- Injection-only ports for `paladin-web` (`PaladinExecutorPort`, `AgentProvisioner`): the web
  crate never names an adapter or the facade (ADR-0031/0038) — D-24/D-25.
- Namespaced `InputMapping` placeholders resolved from `NodeContext`, never the Battlefield, with
  the prefix reserved at validation (`muster.` → `parley.`) — D-07.
- Pre-release classification for engine/Waypoint types absent at `v0.9.0` (deliberate-zero note in
  §9.2) — D-29.
- Three-tier tests; seeded-shuffle determinism harness; X-05 multi-thread stress with exact counts
  and a timeout guard; Postgres Tier 2 only provable in CI (D-28).
- Ubiquitous language: Parley, Gate, Chronicle, Waypoint, Thread, Vanguard, Battlefield, WarGraph,
  WarEngine — in code, docs and comments.

### Integration Points
- `crates/paladin-core/src/platform/container/` — Parley types (D-01), `WaypointStatus`/`Waypoint`
  (`fork_of`)/`NodeOutcomeKind::Parleyed`/`ThreadId::child_on_branch`.
- `crates/paladin-ports/src/input/` — `ParleyPort`; `output/waypoint_port.rs` — `WaypointSummary.fork_of`.
- `crates/paladin-battalion/src/engine/` — `superstep.rs` (Parley arm → suspension, resume
  superstep seeding, grace deadline + abort + Skipped re-listing, child-parley error), `mod.rs`
  (`resume_with`/`replay`/`fork`/`with_shutdown_grace`, new errors, `RunOutcome` reshape),
  `graph.rs` (`Gate`, validation, fingerprint `v4`), `node.rs` (`parley_response`),
  `input_mapping.rs` (`parley.`), `directive_parser.rs` (envelope), new `shutdown.rs`.
- `crates/paladin-storage/src/waypoint/contract_tests.rs` — new cases; no migrations.
- `src/application/services/` — `chronicle.rs`, `parley/` (facade `ParleyPort` adapter +
  `GraphRegistry`); `src/config/engine.rs` (two fields), new `src/config/waypoint_store.rs`;
  `src/bin/paladin-server.rs` and `src/config/setup/service_runner.rs` (coordinator + wait).
- `crates/paladin-web/src/` — new `thread_controller.rs` (+ DTOs, `ThreadApiState`,
  `thread_router`), `openapi.rs`, `lib.rs` exports, `openapi.json`.
- `tests/integration/` — `e2e_approval_gate_test.rs` (+ `Cargo.toml` `[[test]]`), multi-parley,
  cross-process, chronicle immutability/subgraph-fork, shutdown grace, 10-thread stress.
- `k8s/server/deployment.yaml`, `k8s/deployment.yaml`, `k8s/README.md`,
  `docs/src/deployment/{kubernetes,production}.md`, `docs/src/user-guides/parley-and-chronicle.md`,
  `docs/src/SUMMARY.md`, `MIGRATION.md` §9.1/§9.2/§9.5/§9.6/§9.8, `CHANGELOG.md`,
  `.project/v0.10.0/08-traceability-matrix.md`.
- **Constraint confirmed in tree:** `paladin-web` depends only on `paladin-ports` + `paladin-core`
  (`crates/paladin-web/Cargo.toml`); `Settings` is all-pub and not `#[non_exhaustive]`
  (`src/config/settings.rs:27`); `AgentApiState` likewise (`agent_controller.rs:58`) — neither is
  modified by this phase.

</code_context>

<specifics>
## Specific Ideas

- An approval gate should be **three lines of graph**: `add_node("approve", NodeSpec::Gate {
  … Approval, output_field: "approved" })`, `Contains("true")` edge to the action node,
  `Contains("false")` edge to the cancellation node — E2E-2 is written in exactly that shape.
- The multi-parley test asserts three states from the Waypoint alone: two `parleys`, zero
  `responses` → one response → run continues; each transition persisted (D-11).
- The immutability test serialises the whole mainline chain to bytes before `replay` and compares
  after — "byte-identical" means bytes, not field-by-field.
- The shutdown test's slow mock node must observe abort (not merely finish late): assert its
  delta never reaches the Battlefield, its record reads `Skipped { reason: "shutdown" }`, it
  appears in the Halted vanguard, and resume runs it **exactly once** (`run_count == 2` across the
  whole scenario, one aborted + one completed).
- `POST /v1/threads/{id}/resume` on a thread that is `Running` returns 409 with
  `code: "thread_not_awaiting_input"` and `details.status`; on an unregistered graph 409 with
  `code: "graph_not_registered"` — same status, distinct codes, both documented in the spec.
- The `ParleyInChildUnsupported` error text should tell the author what to do today (raise the
  parley in the parent graph) and name the deferred idea, not just say "unsupported".
- Ubiquitous language holds: Parley, Gate, Chronicle, Waypoint, Thread, Vanguard, Battlefield,
  WarGraph, WarEngine, Directive.

</specifics>

<deferred>
## Deferred Ideas

- **Parley propagation through nested Battalions** (D-04) — a child subgraph raising a parley
  suspends the parent with the child's requests (tagged with the child thread) and `resume_with`
  flows down. No FR in PRD 03; the developer may promote it into this phase at plan review or into
  a later phase.
- **Background run submission and worker-pool re-enqueue of resumes** — PLAT-01…03 (Phase 27)
  replace D-25's in-process background task under the same 202 contract.
- **`WarGraphDoc`/assistant registry** — PLAT-04 (Phase 27) replaces D-26's fingerprint-keyed
  code registry behind the same `ParleyPort`.
- **Opaque-cursor and `limit ≤ 100` everywhere, admin/writer scopes on mutating routes** —
  PLAT-06 (Phase 27); D-27/D-24 pre-conform.
- **Scheduler sweep for expired parleys** (HITL-FR-06's "or by the Doc 06 scheduler sweep") —
  Phase 27.
- **`TraceEvent` variants for parley/halt** — OBS-01 (Phase 28) owns the authoritative enum.
- **No mdBook page for the WarEngine itself** — Phase 22 residual still open; this phase adds the
  Parley/Chronicle page only (Phase 29 / a docs pass).
- **22-REVIEW.md WR-01/WR-02** and **22-deferred-items.md item 1** (qdrant rustdoc) — unchanged.

### Reviewed Todos (not folded)
- "Verify local make coverage reproduces CI's 82.39% figure" (todo score 0.2) — a local-tooling
  check unrelated to this phase; left in the todo list.

</deferred>

---

*Phase: 24-pause-resume-history-graceful-shutdown*
*Context gathered: 2026-09-04*
