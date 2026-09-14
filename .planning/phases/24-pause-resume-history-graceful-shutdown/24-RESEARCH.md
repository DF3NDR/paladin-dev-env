# Phase 24: Pause/Resume, History & Graceful Shutdown - Research

**Researched:** 2026-09-04
**Domain:** Rust async orchestration engine — checkpointed suspension/resume, lineage-tracked
replay/fork, cooperative-cancellation graceful shutdown, Axum HTTP surface over existing ports
**Confidence:** HIGH (every mechanism below is grounded in the shipped Phase 22/22.1/23 tree, not
external library docs — this phase extends `paladin-battalion`/`paladin-core`/`paladin-web`
internals with no new external dependency)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

PRD 03 (`.project/v0.10.0/03-pause-resume-history-shutdown.md`) is the FR-level source of truth
and already locks the type sketches (§2.1/§2.2), the FR semantics (HITL-FR-01…16), the acceptance
criteria (§4) and the TDD ordering (§5). The following decisions (D-01…D-30) settle only what PRD
03 left open or what the shipped Phase 22/22.1/23 tree makes concrete. **Do not re-litigate
anything PRD 03, PRD 01, PRD 02, overview §3 (X-01…X-11), or the Phase 22/22.1/23 CONTEXT
decisions state.**

- **D-01:** `ParleyRequest` is finalised in `paladin-core` beside the Phase 22 stub
  (`waypoint.rs:452`) — `parley_id`, `node_id`, `kind: ParleyKind`, `prompt`, `payload:
  serde_json::Value`, `choices: Option<Vec<String>>`, `expires_at: Option<DateTime<Utc>>`,
  `created_at`, plus `on_expire: OnExpire` (`FailRun` default | `ResumeWithDefault(Value)`).
  `ParleyKind`, `ParleyResponse { parley_id, value, responded_by, responded_at }` and `OnExpire`
  land in the same module. `parley_id` is a serde-transparent `ParleyId(Uuid)` newtype using
  UUIDv7, mirroring `WaypointId`. All new persisted types derive serde and are `#[non_exhaustive]`
  where they are enums. Reversibility: one-way after v0.10.0 ships; free now.
- **D-02:** `WaypointStatus::AwaitingInput { parleys: Vec<ParleyRequest>, responses:
  Vec<ParleyResponse> }` replaces the Phase 22 single-`parley` field. `parleys` is every request
  raised in the suspending superstep; `responses` is the accepted subset so far. The
  `AwaitingInput` Waypoint's `vanguard` is exactly the parleying nodes (D-08). `RunOutcome::
  AwaitingInput` becomes `{ parleys: Vec<ParleyRequest>, waypoint: WaypointId }` carrying only the
  still-unanswered requests. The three-backend contract suite gains round-trip cases; no SQL
  migration. Reversibility: one-way after release.
- **D-03:** The parleying node is observable on the Waypoint via a new `NodeOutcomeKind::Parleyed`
  variant recording the raise in the suspending superstep's `completed` records. The emitting
  node's own `Directive.delta` merges at raise time. Function/Paladin parleys are documented
  "idempotent up to the parley point" (HITL-FR-04) because the node re-runs on resume (D-08).
- **D-04:** A parley inside a nested Battalion child is a typed failure this phase. Replace the
  current stringly `NodeError` (`superstep.rs:475`) with structured `EngineError::
  ParleyInChildUnsupported { node, child_thread }` (X-06); document it on `NodeSpec::Battalion` and
  the Gate rustdoc; cover it with a test; record the propagation design under Deferred Ideas.
  Rejected: designing parent-suspension-through-child now.
- **D-05:** `NodeSpec::Gate { request: GateRequestTemplate, output_field: Option<FieldName> }` on
  the already-`#[non_exhaustive]` enum. `GateRequestTemplate { kind, prompt_template:
  InputMapping, payload_template: Option<InputMapping>, choices, expires_in: Option<Duration>,
  on_expire }` — `expires_at = now + expires_in` stamped at raise time. `output_field` is required
  for Approval/Choice/FreeText and must be `None` for StateEdit; `WarGraph::validate` rejects other
  combinations and checks field-type compatibility via typed `EngineError` variants. A Gate has no
  `run` body: first visit raises, post-resume visit writes the normalised value (D-06) to
  `output_field` (or returns the StateEdit delta) and routes via static edges.
- **D-06:** Edge evaluation treats a Gate like a Paladin node — the engine-path `EdgeContext.output`
  for a Gate source is its `output_field` value (23 D-02's Paladin rule), so `Contains`/`Regex`/
  registered evaluators work on the delivered value. Approval values normalise to JSON `true`/
  `false` before delivery (HITL-FR-05: `true`/`false`, `"yes"/"no"/"approve"/"deny"`
  case-insensitive); a String `output_field` gets `"true"`/`"false"`.
- **D-07:** Paladin nodes raise a parley through the `StructuredDirective` envelope (23 D-11 gains
  `"next": {"parley": {"kind": ..., "prompt": ..., "payload": {...}, "choices": [...],
  "expires_in_secs": N}}`), and receive the answer through a `parley.` InputMapping namespace
  (`{parley.value}`, `{parley.prompt}`, `{parley.kind}`, `{parley.responded_by}`), resolved from
  `NodeContext`, never the Battlefield — graph validation rejects schema fields starting with
  `parley.` (mirrors 23 D-15's `muster.` rule). `NodeContext` gains `parley_response:
  Option<ParleyResponse>` with a `parley_response()` accessor.
- **D-08:** Resume is an ordinary superstep whose Vanguard is exactly the parleying nodes. Once
  every parley is answered, the engine seeds superstep N+1 from the `AwaitingInput` Waypoint with
  `vanguard = parleying nodes`, each node's `NodeContext.parley_response` populated, and runs the
  normal superstep loop. Responses are durably consumed only when the first post-resume Waypoint
  persists — re-submitting the same responses after a mid-write crash is safe. Rejected: a
  dedicated "merge responses" pseudo-superstep.
- **D-09:** `GRAPH_FINGERPRINT_VERSION` bumps `v3` → `v4`. New sorted, length-prefixed section per
  Gate node: `kind`, `output_field`, `choices`, `on_expire` kind; `prompt_template`/
  `payload_template`/`expires_in` excluded like `InputMapping` templates (ENG-FR-14). Golden hex
  re-pinned, one difference test per hashed property. Reversibility: one-way after v0.10.0 ships;
  free now.
- **D-10:** Validation is total before any state changes. `resume_with` loads `latest(thread)`,
  checks `GraphMismatch`, requires `AwaitingInput` (else `EngineError::ThreadNotAwaitingInput`),
  then validates every submitted response before persisting anything: `UnknownParleyId`,
  `ParleyAlreadyAnswered` (new), `ResponseShapeInvalid` (Approval/Choice/FreeText/StateEdit
  matrix; StateEdit deserialises to a StateDelta validated against the schema, unknown field
  rejects the response not the run), `ParleyExpired`. Any error leaves the thread suspended with
  no Waypoint written.
- **D-11:** Partial answers persist as a new `AwaitingInput` Waypoint. A valid subset writes a
  child Waypoint at the same superstep with `responses` extended and returns `RunOutcome::
  AwaitingInput` listing the remaining parleys. Plain `WarEngine::resume` on an `AwaitingInput`
  thread fails with `EngineError::ThreadAwaitingInput { thread, parleys }` — it never guesses.
- **D-12:** Expiry policy evaluated lazily at resume time against `Utc::now()` (no timer; the Doc
  06 sweep is Phase 27). `on_expire: FailRun` → a `Failed` Waypoint is persisted and `resume_with`
  returns `Err(ParleyExpired)`; thereafter resumable only by replay/fork. `ResumeWithDefault(v)` →
  `v` validated per kind at graph-validate time (a Gate) or raise time (a Directive), substituted
  as the response with `responded_by: None`, recorded (`defaulted: true`, exact field Claude's
  discretion).
- **D-13:** `expires_at` in tests is set in the past explicitly — no clock abstraction is added
  this phase.
- **D-14:** `fork_of` marks the branch root and propagates along the branch. `Waypoint` gains
  `#[serde(default)] fork_of: Option<WaypointId>` (additive, no migration, contract-suite round
  trip). The fork's first Waypoint has `parent_waypoint_id = Some(from)` and `fork_of = Some(from)`;
  every subsequent Waypoint on that branch inherits `fork_of = Some(from)`; mainline Waypoints
  carry `None`; a fork of a fork carries the newer `from`. `WaypointSummary` gains `fork_of`.
  Reversibility: one-way after release.
- **D-15:** `latest(thread)` stays "most recently created across branches" — now load-bearing. All
  three backends already order by `created_at DESC, superstep DESC`; add a contract-suite case
  asserting a fork's newest Waypoint wins over a later-superstep mainline Waypoint, on all three.
  Document on `WarEngine::resume` that resume-without-branch-qualifier resumes the newest branch.
  Retention treats every branch's Waypoints as ordinary members of the thread.
- **D-16:** `replay`/`fork` live on `WarEngine`; `ChronicleService` is a facade application
  service. `WarEngine::replay(graph, thread, from) -> Result<RunOutcome, EngineError>` and
  `WarEngine::fork(graph, thread, from, edit: StateDelta)` re-enter `superstep::run` from
  `get(thread, from)` with `parent = from`, `fork_of = Some(from)`, superstep numbering continuing
  from `from.superstep + 1`, fingerprint checked, edit merged through the schema's dispatch rules
  before the first forked superstep. Typed errors: `WaypointNotFound { thread, waypoint }`,
  `ForkFromFailed`-style guards at Claude's discretion. `ChronicleService`
  (`src/application/services/chronicle.rs`) exposes `history`, `inspect`, `latest_on_branch` over
  `Arc<dyn WaypointPort>` — no engine dependency.
- **D-17:** Immutability is asserted byte-for-byte. The replay test serialises every mainline
  Waypoint before and after `replay` and asserts equality; the fork-with-edit test flips a
  conditional edge.
- **D-18:** Subgraph forks never share child Waypoints. A branch runs its `NodeSpec::Battalion`
  children under a child thread id derived from the parent thread and the branch root
  (`ThreadId::child_on_branch(parent, branch_root, node)` — same length-prefixed, injective
  encoding as `ThreadId::child`, 22.1 CR-01 rule). Mainline runs keep `ThreadId::child` unchanged.
- **D-19:** Grace is enforced inside the superstep, at the point cancellation is first seen
  mid-flight. Add a second observation: while awaiting the superstep's spawned node tasks, if the
  token fires, keep awaiting them until `cancel_observed_at + shutdown_grace`; tasks still running
  at the deadline are aborted (`JoinHandle::abort`), recorded `NodeOutcomeKind::Skipped { reason:
  "shutdown" }`, their deltas discarded, and their ids re-listed in the Halted Waypoint's vanguard
  alongside the normally computed next Vanguard. Skipped nodes' outgoing edges stay `Pending` in
  the `FrontierSnapshot` so resume re-executes them exactly once. The existing boundary check is
  unchanged. `shutdown_grace = Duration::ZERO` aborts immediately. Mechanism inside `superstep.rs`
  is Claude's discretion; the contract above is locked.
- **D-20:** `WarEngine::with_shutdown_grace(Duration)` (default 30s) is a runtime setting, not a
  graph setting — not part of `EngineLimits` and never hashed. `EngineConfig` gains
  `shutdown_grace_secs: u64` (default `30`, env `APP_ENGINE_SHUTDOWN_GRACE_SECS`) and
  `graceful_shutdown: bool` (default `true`, env `APP_ENGINE_GRACEFUL_SHUTDOWN` — the M-B-02
  disable switch). `Settings` is not touched. Reversibility: costly — operator-facing once
  documented in §9.5.
- **D-21:** `ShutdownCoordinator` + `RunGuard` live in `paladin-battalion::engine::shutdown`. A
  root `tokio_util::sync::CancellationToken`, an in-flight counter and a `Notify`; `register()`
  returns a child token + RAII guard; `cancel_and_wait(grace)` cancels the root and waits until
  idle or the deadline. Placed in battalion so embedded-library users and Phase 27's worker pool
  reuse it. X-05 stress test with exact counts and a timeout guard.
- **D-22:** Both process entry points wire it. `src/bin/paladin-server.rs::shutdown_signal` and
  `ServiceRunner::wait_for_shutdown` cancel the coordinator on SIGTERM/SIGINT, then wait ≤
  `shutdown_grace` (skipped when `graceful_shutdown = false`) before axum's
  `with_graceful_shutdown` completes / the runner exits. Every in-flight engine run the facade
  starts — today only the background continuation spawned by the resume port (D-25) — registers
  with it. `resume` on a `Halted` thread is tested explicitly.
- **D-23:** Operator surface. `terminationGracePeriodSeconds: 60` (= 2× 30s) added to
  `k8s/server/deployment.yaml` and `k8s/deployment.yaml`; `k8s/README.md`,
  `docs/src/deployment/kubernetes.md` and `docs/src/deployment/production.md` updated with the 2×
  rule and the env switch; `MIGRATION.md` §9.1 M-B-02 worked example (before: no wait,
  `terminationGracePeriodSeconds` unset; after: 60s, `APP_ENGINE_SHUTDOWN_GRACE_SECS=30`,
  `APP_ENGINE_GRACEFUL_SHUTDOWN=false` to opt out), §9.5 new fields, §9.8's "adjust termination
  grace" bullet made concrete.
- **D-24:** Thread routes get their own state and router; `AgentApiState` is untouched. New
  `ThreadApiState { waypoints: Option<Arc<dyn WaypointPort>>, parley: Option<Arc<dyn ParleyPort>>,
  auth }` and `thread_router(state)` nested under `API_V1_PREFIX` and merged by `paladin-server`
  behind the same auth middleware as `/v1/agents/*`. `GET …/state` and `GET …/history` read
  `WaypointPort` directly; DTOs are `utoipa::ToSchema` types in `paladin-web`. When the deployment
  has no waypoint backend wired, every thread route answers 501 `not_implemented` naming the
  config to set. Rejected: `paladin-web` depending on `paladin-battalion`.
- **D-25:** Resume goes through a port; the facade runs the continuation in the background. New
  `ParleyPort` in `paladin-ports`: `async fn resume_with(&self, thread: &ThreadId, responses:
  Vec<ParleyResponse>) -> Result<ResumeAccepted, ParleyError>` where `ParleyError` mirrors the
  D-10 variants plus `ThreadNotFound` and `GraphNotRegistered`. The facade adapter validates
  synchronously via `WarEngine::resume_with`'s validation path, then spawns the continuation as a
  background task registered with the `ShutdownCoordinator` and returns immediately. `POST
  /v1/threads/{id}/resume` returns 202 Accepted `{ thread_id, state_url }`; clients poll `GET
  …/state`. Status mapping: 404 `ThreadNotFound`; 409 `conflict` for `ThreadNotAwaitingInput` and
  `GraphNotRegistered` (distinct codes); 400 `bad_request` for
  `UnknownParleyId`/`ParleyAlreadyAnswered`/`ResponseShapeInvalid`/`ParleyExpired`; 501 when
  unwired. Rejected: running the resume inline and holding the connection. Reversibility: costly —
  Phase 27 re-enqueues under the same shape.
- **D-26:** The graph for a thread is resolved by fingerprint from a code-registered registry. The
  facade adapter holds a `GraphRegistry` keyed by `GraphFingerprint`; unregistered →
  `GraphNotRegistered`. Phase 27's `WarGraphDoc`/assistant registry replaces the lookup behind the
  same port. `paladin-server` gets a minimal `WaypointStoreConfig` (`backend: disabled | sqlite {
  path } | postgres { url env }`, default disabled → 501); it registers no graphs itself, so
  end-to-end HTTP resume against a real graph is proven in `paladin-web`'s `tower::util::oneshot`
  tests with an in-test registry, not in the binary.
- **D-27:** History pagination is `limit` + opaque `cursor`. `GET …/history?limit=20&cursor=…`
  (limit ≤ 100), response `{ items: [WaypointSummary incl. fork_of], next_cursor }`; the cursor's
  content is the last returned `waypoint_id` (documented as opaque to clients). `openapi.json`
  regenerated and diff-reviewed; `MIGRATION.md` §9.6's endpoint list filled.
- **D-28:** Tiers. Everything HITL adds is Tier 1 (`CountingFunctionNode`, `RecordingPaladinPort`,
  `InMemoryWaypointStore`, `SqliteWaypointStore` on a temp file, `MockLlmAdapter`). Cross-process =
  two `WarEngine` instances over one SQLite file. The Postgres contract-suite additions (D-02,
  D-14, D-15) are Tier 2 via CI's `postgres-integration` job — Docker is unavailable in the
  devcontainer, so route them to UAT, never mark them passed locally. E2E-2 is
  `tests/integration/e2e_approval_gate_test.rs` with a `[[test]]` entry mirroring
  `e2e_crash_resume`; the X-05 stress test is acceptance 7 (10 suspended threads resumed
  concurrently, exact outcomes, `flavor = "multi_thread"`, timeout guard).
- **D-29:** `.project/` and `MIGRATION.md` registrations. §9.2: resolve the `Waypoint` row and add
  a deliberate-zero note for every other type touched (`RunOutcome`, `EngineError`, `NodeSpec`,
  `NodeContext`, `NodeOutcomeKind`, `WaypointSummary`, `TraceEvent`-untouched, `ThreadApiState`
  new) — the new `ParleyPort` trait is new. §9.1 M-B-02 example, §9.5, §9.6.
  `08-traceability-matrix.md` G-05/G-06/G-09/G-15/G-26 rows gain their test anchors. `cargo
  semver-checks` (vs 0.9.0), `msrv` (1.88), `make security`, `cargo clippy -- -D warnings` and
  coverage ≥ 82% (ADR-0006) green on the phase's final commit.
- **D-30:** Docs (X-08). New mdBook page `docs/src/user-guides/parley-and-chronicle.md` (Gate
  approval gate, resume, partial answers, expiry, history/replay/fork, the
  `paladin-notifications` composition example PRD 03 §6 requires, graceful shutdown from the
  embedder's side), wired into `SUMMARY.md` after the control-flow page; deployment pages per
  D-23; doc-tests on every new public API; `cargo doc` with no new broken intra-doc links;
  `CHANGELOG.md` `[Unreleased]`.

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
- Whether `TraceEvent` gains `ParleyRaised`/`RunHalted` — recommended no (Phase 28 owns the
  authoritative enum; `RunFinished` suffices) — and whether `BATTLEFIELD_SCHEMA_VERSION` bumps for
  the additive fields (follow the `visit_counts` precedent).
- Plan/wave decomposition, respecting PRD 03 §5's TDD order. Suggested: (1) Parley types,
  `AwaitingInput` payload, validation matrix units, contract-suite cases; (2) engine suspension +
  `resume_with` + partial answers + expiry on InMemory then SQLite, Gate node, `parley.`
  namespace, envelope, fingerprint `v4`, `ParleyInChildUnsupported`; (3) E2E-2 + multi-parley +
  cross-process + stress; (4) Chronicle: `fork_of`, `replay`/`fork`, `ChronicleService`,
  `child_on_branch`, immutability/subgraph-fork tests; (5) shutdown grace in `superstep.rs`,
  `ShutdownCoordinator`, `EngineConfig` fields, `paladin-server`/`ServiceRunner` wiring, k8s/docs,
  M-B-02 example; (6) `ParleyPort`, facade adapter + `GraphRegistry` + `WaypointStoreConfig`,
  `ThreadApiState` + routes + DTOs + oneshot tests, `openapi.json`, §9.6; (7) mdBook page,
  MIGRATION/CHANGELOG/traceability sweep, CI evidence. (2) precedes (3)–(6); (4), (5) and (6) are
  independent of each other except (6)'s dependency on (5)'s coordinator.

### Deferred Ideas (OUT OF SCOPE)

- **Parley propagation through nested Battalions** (D-04) — a child subgraph raising a parley
  suspends the parent with the child's requests (tagged with the child thread) and `resume_with`
  flows down. No FR in PRD 03; the developer may promote it into this phase at plan review or into
  a later phase.
- **Background run submission and worker-pool re-enqueue of resumes** — PLAT-01…03 (Phase 27)
  replace D-25's in-process background task under the same 202 contract.
- **`WarGraphDoc`/assistant registry** — PLAT-04 (Phase 27) replaces D-26's fingerprint-keyed code
  registry behind the same `ParleyPort`.
- **Opaque-cursor and `limit ≤ 100` everywhere, admin/writer scopes on mutating routes** —
  PLAT-06 (Phase 27); D-27/D-24 pre-conform.
- **Scheduler sweep for expired parleys** (HITL-FR-06's "or by the Doc 06 scheduler sweep") —
  Phase 27.
- **`TraceEvent` variants for parley/halt** — OBS-01 (Phase 28) owns the authoritative enum.
- **No mdBook page for the WarEngine itself** — Phase 22 residual still open; this phase adds the
  Parley/Chronicle page only (Phase 29 / a docs pass).
- **22-REVIEW.md WR-01/WR-02** and **22-deferred-items.md item 1** (qdrant rustdoc) — unchanged.

Also out of this phase's boundary (later phases own them): retry/timeout/error handlers and the
E2E-3 Aegis half (Phase 25); middleware and `NodeInterceptor` visibility of routing (Phase 26);
background run submission, worker-pool re-enqueue of resumes, assistants/`WarGraphDoc` registry,
opaque-cursor pagination policy, admin/writer scopes, the Doc 06 expiry sweep (Phase 27); the
authoritative `TraceEvent` enum and trace consumers (Phase 28); `MIGRATION.md` §9.6 golden diff,
§9.8 checklist finalisation (Phase 29). Notification of a waiting parley is composed in
application code with the existing `paladin-notifications` — a doc example is required, no new
port (PRD 03 §6). Any other behavioral change discovered mid-implementation is an X-03
stop-and-flag event.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|---------------------|
| HITL-01 | Parley suspension: node/`Gate` raises `ParleyRequest`, superstep merges peers, persists multi-parley `AwaitingInput` Waypoint, releases every resource, resumable from a different process over the same backend, partial answers queryable | Pattern 1 (additive Waypoint field), Pattern 2 (fingerprint `v4` for Gate), Code Example ("Extending `NodeOutcomeKind`/`WaypointStatus`"), Pitfall 2 (resume-path fallthrough), Don't-Hand-Roll row 1 (durability IS the cross-process mechanism) |
| HITL-02 | `resume_with(graph, thread, responses)` typed per-`ParleyKind` validation, `expires_at`/`on_expire` policy, E2E-2 (approval gate, both branches, process drop/recreate) | Pattern 4 (`parley.` InputMapping namespace), Pitfall 2, Pitfall 3 (`InputMapping::render` signature break), Validation Architecture row "E2E-2", Security Domain rows 2 & 4 |
| HITL-03 | Chronicle `history`/`inspect`/`replay`/`fork`-with-edit, `fork_of` lineage, byte-identical mainline, branch-aware `latest`, subgraph-fork isolation | Pattern 1 (`fork_of` additive field), Architectural Responsibility Map row "Chronicle", Anti-Pattern "reconstructing child history by filtering `checkpoint_ns`" |
| HITL-04 | Graceful shutdown within `shutdown_grace`, over-grace nodes `Skipped` + re-listed in vanguard, `resume` continues `Halted`, SIGTERM/SIGINT wired, `k8s/` + docs updated, disable switch | Pitfall 1 (dispatch-loop restructuring — primary engineering risk), Pattern 5 (`EngineConfig` extension), Pitfall 4 (`EngineLimits` conflation trap), Code Examples (batch-race shape), Don't-Hand-Roll row 2 |
| HITL-05 | `GET/POST/GET /v1/threads/{id}/{state,resume,history}` over utoipa + error envelope, `openapi.json` regenerated | Pattern 6 (injection-only `ThreadApiState`), Pattern 7 (`202` background-job precedent), Security Domain row 1 & row on background-task DoS, Validation Architecture row "HITL-05" |
</phase_requirements>

## Summary

Phase 24 is the least "reach for a library" phase in this program: HITL-01…05 are almost entirely
extensions of code that already exists and already has the exact shape PRD 03 needs. `ParleyRequest`
is a stub at `waypoint.rs:452`, `WaypointStatus::AwaitingInput` and `Halted` already exist,
`NextStep::Parley` is already a `Directive` variant that currently fails closed with
`EngineError::ParleyNotSupported`, cancellation is already observed at superstep boundaries and
produces a resumable `Halted` Waypoint, `WaypointPort::history` already returns lineage-capable
`WaypointSummary`s, and `paladin-web`'s Axum + utoipa + `ApiError` envelope conventions are fully
established with a `202 Accepted`-background-job precedent already shipped
(`enqueue_job`/`get_job`, `agent_controller.rs:609-676`). The work is disciplined extension, not
invention: replace three typed-failure arms with real suspension/resume/replay logic, add one new
`NodeSpec` variant (`Gate`) that reuses the existing Paladin-node edge-evaluation path, bump the
graph fingerprint to `v4` following the exact `v2`→`v3` precedent, and add one new port
(`ParleyPort`) plus one new Axum sub-router (`thread_router`) that never touches
`AgentApiState`.

No new external crate is needed anywhere in this phase — `chrono`, `uuid`, `serde_json`,
`tokio-util` (for `CancellationToken`), `blake3`, `thiserror`, `async-trait`, `utoipa`, `axum` are
already dependencies of the crates that need them (verified against each crate's `Cargo.toml`
below). The single largest correctness risk is in `superstep.rs`'s node-dispatch loop: today,
`dispatch_entries.iter().zip(handles)` awaits each spawned `tokio::spawn` `JoinHandle` **strictly
sequentially, one at a time, in dispatch order** — not via `join_all`/`FuturesUnordered`. D-19's
grace-deadline mechanism (finish in-flight nodes within `shutdown_grace`, then `JoinHandle::abort`
the rest) cannot be bolted onto this sequential-await shape without first racing the *whole batch*
of handles against a single deadline; naively adding a per-handle `tokio::time::timeout` in the
existing loop would abort node 2 while node 1 is still within its grace window, misordering the
abort semantics D-19 requires. This is flagged as Pitfall 1 below because it is the one place in
this phase where "extend the existing code" is not simply mechanical.

**Primary recommendation:** Follow the shipped Phase 22/22.1/23 patterns byte-for-byte — additive
`#[serde(default)]` Waypoint fields, `#[non_exhaustive]` typed `EngineError`/`ParleyError` variants,
length-prefixed fingerprint sections, config structs standalone under `src/config/` never touching
`Settings`, injection-only ports for `paladin-web` — and budget real engineering time only for the
superstep dispatch-loop restructuring (Pitfall 1) and the cross-process/multi-thread stress tests
(D-28's X-05 pattern), which are the only places genuinely new concurrency logic is required.

## Architectural Responsibility Map

This project is a Rust hexagonal-architecture library/service (Ports & Adapters, DDD), not a
browser/SSR/CDN web application — the standard web tiers below are re-mapped to this project's own
layering: **Core** (`paladin-core`, pure domain types, no I/O), **Engine** (`paladin-battalion`,
the `WarEngine`/superstep orchestration — this project's "backend business logic" tier), **Ports**
(`paladin-ports`, interface/trait boundary — analogous to an API contract layer), **Storage
Adapters** (`paladin-storage`, the "database" tier), **Application/Facade**
(`src/application/services/`, `src/config/` — composition root, analogous to a backend server
process), **HTTP Adapter** (`paladin-web` + `src/bin/paladin-server.rs` — the actual "API/Backend"
tier in the standard sense, since this is a headless Rust service with no browser/SSR/CDN
component).

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `ParleyRequest`/`ParleyResponse`/`ParleyKind`/`OnExpire` value types | Core | — | ADR-0016: core owns port value types; no I/O, pure data (D-01) |
| Parley suspension (superstep merges, releases resources, persists `AwaitingInput`) | Engine | Storage Adapters | `superstep.rs`'s dispatch loop and `build_waypoint`/`persist_waypoint` own this; storage just durably writes the payload (D-02, D-19) |
| `Gate` node (raise on first visit, deliver on resume, route via edges) | Engine | Core (`NodeSpec` variant declared alongside `Paladin`/`Function`/`Battalion`) | Dispatch logic lives in `superstep.rs`; the variant shape is declared in `graph.rs` (D-05) |
| `resume_with` typed validation (per-`ParleyKind`, expiry, partial answers) | Engine | — | `WarEngine::resume_with` on `mod.rs`, the same layer `resume`/`resume_with_options` already live at (D-10…D-13) |
| Chronicle (`history`/`inspect`/`latest_on_branch`, `replay`/`fork`) | Engine (`replay`/`fork` on `WarEngine`) | Application/Facade (`ChronicleService` read-facade) | `replay`/`fork` re-enter `superstep::run`; `ChronicleService` is a thin `WaypointPort`-only facade with no engine dependency, reused by `paladin-web` (D-16) |
| `fork_of` lineage, `latest_on_branch` ordering | Core (type) | Storage Adapters (query/ordering) | `Waypoint.fork_of` is a core field; `latest`'s `created_at DESC, superstep DESC` ordering is a storage-adapter SQL/BTree concern already shared across all three backends (D-14, D-15) |
| Graceful shutdown grace window, `ShutdownCoordinator`/`RunGuard` | Engine (`engine::shutdown`, in-superstep enforcement) | Application/Facade (process wiring) | The deadline/abort mechanism must live where the spawned `JoinHandle`s are (`superstep.rs`); the coordinator's `register()`/`cancel_and_wait()` API is consumed by both `paladin-server.rs` and `ServiceRunner` (D-19, D-21, D-22) |
| `EngineConfig.shutdown_grace_secs` / `graceful_shutdown` | Application/Facade | — | Config structs are standalone under `src/config/`, never fields on `Settings` (X-10 avoidance, D-20) |
| `k8s/` manifests, deployment docs | Application/Facade (ops surface) | — | `terminationGracePeriodSeconds`/docs are operator-facing config, not code (D-23) |
| `ParleyPort` (resume trigger) | Ports | Application/Facade (adapter) | Port trait in `paladin-ports/src/input/`; the facade adapter in `src/application/services/parley/` implements it over `WarEngine` (D-25) |
| `ThreadApiState`/`thread_router`/DTOs, `/v1/threads/*` routes | HTTP Adapter | Ports (reads `WaypointPort`/`ParleyPort` directly, never `paladin-battalion`) | ADR-0031 forbids `paladin-web` → `paladin-battalion`; D-24 keeps `AgentApiState` untouched |
| `GraphRegistry` (fingerprint → graph lookup for HTTP resume) | Application/Facade | — | Code-registered, not `paladin-web`-owned (D-26, mirrors 23 D-26's "code-configured, off by default" precedent) |

## Standard Stack

### Core

No new external library is added by this phase. Every mechanism below is built on dependencies
already present in the crate that needs it.

| Dependency | Crate already carrying it | Verified version | Purpose in this phase |
|------------|---------------------------|-------------------|------------------------|
| `tokio-util` (`CancellationToken`) | `paladin-battalion` (`Cargo.toml:28`, `tokio-util = "0.7"`) [VERIFIED: crates/paladin-battalion/Cargo.toml] | `0.7` | `ShutdownCoordinator`'s root token + `RunGuard`'s child tokens (D-21) — `hooks.rs` already imports `tokio_util::sync::CancellationToken` |
| `chrono` | `paladin-core`, `paladin-battalion`, `paladin-ports`, `paladin-web` (all `Cargo.toml`s) [VERIFIED: grep across crate manifests] | workspace-pinned | `ParleyRequest.expires_at`/`created_at`, `ParleyResponse.responded_at` (D-01) |
| `uuid` | same four crates [VERIFIED: grep across crate manifests] | workspace-pinned | `ParleyId(Uuid)` newtype, UUIDv7 like `WaypointId` (D-01) |
| `serde_json` | same four crates [VERIFIED: grep across crate manifests] | workspace-pinned | `ParleyRequest.payload: serde_json::Value`, envelope `{"parley": {...}}` extension (D-01, D-07) |
| `blake3` | `paladin-core` (`Cargo.toml:27`, `blake3 = "1.8.2"`) [VERIFIED: crates/paladin-core/Cargo.toml] | `1.8.2` | `GraphFingerprint::from_canonical_bytes` — unchanged mechanism, `v4` is a new hashed section not a new hash function (D-09) |
| `thiserror` | every crate in this phase's scope | workspace-pinned | Every new `EngineError`/`ParleyError`/`InputMappingError`-style variant (X-06) |
| `async-trait` | `paladin-battalion`, `paladin-ports` | workspace-pinned | `ParleyPort`, any new trait method |
| `utoipa` / `utoipa-axum` | `paladin-web` only | already pinned (see `openapi.rs`) | Thread DTOs' `ToSchema`, `thread_router`'s `#[utoipa::path]` annotations (D-24, ADR-0038) |
| `axum` | `paladin-web`, `src/bin/paladin-server.rs` | already pinned (0.8.4 per `docs/src/deployment/production.md:342`) [CITED: docs/src/deployment/production.md] | `thread_router` nesting, `oneshot` tests |

### Supporting

| Library | Purpose | When to Use |
|---------|---------|-------------|
| `tokio::sync::Notify` | `ShutdownCoordinator`'s idle-wait primitive (D-21) | Already a `tokio` feature the workspace enables; no new dependency line needed — confirm `tokio`'s `sync` feature is on in `paladin-battalion/Cargo.toml` before assuming it (grep did not show a `features = [...]` restriction, so it is likely already enabled via `full`/`rt-multi-thread`; verify at plan time) |
| `tokio::time::timeout` / `tokio::select!` | Racing the batch of spawned node `JoinHandle`s against `shutdown_grace` (Pitfall 1) | Only inside `superstep.rs`'s dispatch/join loop — do not introduce a per-handle timeout inside the existing sequential-await `for` loop; restructure to a batch-level race (see Pitfall 1 and Code Examples) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled `ShutdownCoordinator` (counter + `CancellationToken` + `Notify`) | A crate like `tokio-graceful-shutdown` | D-21 explicitly locks the hand-rolled shape ("in-flight counter and a `Notify`") — introducing a third-party shutdown-orchestration crate this late, for a ~40-line primitive with an exact locked contract, adds a dependency-audit burden (`make security`/`cargo-deny`) with no corresponding benefit; **do not introduce this dependency** |
| `futures::future::join_all` for the grace-deadline race | `tokio::select!` biased loop draining handles one at a time | `join_all` doesn't let you distinguish "still running past the deadline" per-handle for the abort step; a `select!`-based drain (or `FuturesUnordered` + `tokio::time::sleep_until` deadline) is the standard shape for "wait for N tasks with a shared deadline, abort stragglers" — see Code Examples |

**Installation:** No `Cargo.toml` changes required in `paladin-core`, `paladin-battalion`,
`paladin-ports`, or `paladin-storage`. `paladin-web`'s existing `utoipa`/`axum`/`utoipa-axum` cover
the new DTOs and router.

**Version verification:** All dependency versions above were confirmed present via direct
`Cargo.toml` reads on 2026-09-04 (see per-row citations); no `cargo add`/registry lookup was
needed since nothing new is added.

## Package Legitimacy Audit

**Not applicable — no new external package is installed by this phase.** Every mechanism (parley
types, `Gate` node, `resume_with`, Chronicle, shutdown coordinator, `ParleyPort`, thread routes)
is built from dependencies already present in the relevant crate's `Cargo.toml`, verified above by
direct file read. There is nothing to run `gsd-tools query package-legitimacy check` against.

**Packages removed due to [SLOP] verdict:** none (none proposed).
**Packages flagged as suspicious [SUS]:** none (none proposed).

## Architecture Patterns

### System Architecture Diagram

```text
                          ┌─────────────────────────────────────────┐
                          │  paladin-web: POST /v1/threads/{id}/resume│
                          │  (axum handler, ThreadApiState)           │
                          └───────────────┬───────────────────────────┘
                                          │ ParleyPort::resume_with(thread, responses)
                                          ▼
   ┌───────────────────────────────────────────────────────────────────┐
   │ src/application/services/parley/ (facade ParleyPort adapter)       │
   │  1. GraphRegistry.lookup(fingerprint on latest Waypoint)            │
   │  2. WarEngine::resume_with(graph, thread, responses) -- SYNC        │
   │     validation only (D-10): typed errors surface here, thread      │
   │     stays suspended, NOTHING persisted on error                    │
   │  3. spawn background continuation, registered with                 │
   │     ShutdownCoordinator (D-21) -- returns 202 immediately (D-25)    │
   └───────────────┬───────────────────────────────────────────────────┘
                   │ background task
                   ▼
   ┌───────────────────────────────────────────────────────────────────┐
   │ paladin-battalion::engine::superstep::run  (resume superstep,       │
   │ vanguard = exactly the parleying nodes, D-08)                       │
   │                                                                     │
   │  loop {                                                            │
   │    check CancellationToken at boundary -> Halted (existing, D-19    │
   │      extends the MID-superstep observation point)                   │
   │    dispatch vanguard via tokio::spawn (existing)                    │
   │    await handles -- batch-raced against shutdown_grace deadline,    │
   │      abort stragglers -> Skipped { reason: "shutdown" },             │
   │      re-list in next Vanguard (D-19, Pitfall 1)                     │
   │    Gate nodes: write output_field from parley_response (D-05)       │
   │    Function/Paladin nodes: re-run, parley_response resolves via     │
   │      NodeContext / `parley.` InputMapping namespace (D-07)          │
   │    merge deltas -> Battlefield                                      │
   │    resolve edges (Gate treated like Paladin: output_field is        │
   │      EdgeContext.output for Custom evaluators, D-06)                │
   │    NextStep::Parley(req) on ANY node -> suspend: merge peers,        │
   │      persist AwaitingInput{parleys, responses}, RunOutcome::         │
   │      AwaitingInput (D-02, D-03)                                     │
   │    persist ONE Waypoint (ENG-FR-11, unchanged)                      │
   │  }                                                                   │
   └───────────────┬───────────────────────────────────────────────────┘
                   │ save()
                   ▼
   ┌───────────────────────────────────────────────────────────────────┐
   │ WaypointPort (InMemory | SQLite | Postgres) -- unchanged trait,     │
   │ new AwaitingInput{parleys,responses} payload, new fork_of field,    │
   │ same created_at DESC, superstep DESC latest() ordering (D-15)       │
   └───────────────────────────────────────────────────────────────────┘

   Read side (no engine dependency):
   GET /v1/threads/{id}/state | /history  ─▶ ThreadApiState.waypoints
       (Arc<dyn WaypointPort>) ─▶ same backend, read-only  ─▶
       ChronicleService::history/inspect/latest_on_branch (D-16, D-24)

   Process shutdown:
   SIGTERM/SIGINT ─▶ shutdown_signal() (paladin-server.rs) /
       ServiceRunner::wait_for_shutdown() ─▶ ShutdownCoordinator::
       cancel_and_wait(shutdown_grace) ─▶ root CancellationToken.cancel()
       ─▶ every registered in-flight superstep::run observes it at its
       mid-superstep grace point (D-19) ─▶ coordinator waits idle or
       deadline ─▶ axum::serve(...).with_graceful_shutdown() completes /
       ServiceRunner exits (D-22)
```

### Recommended Project Structure

No new crates or top-level directories. New files, following existing sibling-module conventions:

```
crates/paladin-core/src/platform/container/
├── waypoint.rs          # extend: ParleyRequest (full shape), WaypointStatus::AwaitingInput
│                         #   { parleys, responses }, NodeOutcomeKind::Parleyed, fork_of field
├── directive.rs          # extend: NextStep::Parley doc comment (suspension now real)
└── parley.rs              # NEW (Claude's discretion vs extending waypoint.rs, D-01) --
                            #   ParleyKind, ParleyResponse, OnExpire, ParleyId(Uuid)

crates/paladin-battalion/src/engine/
├── superstep.rs           # extend: Parley suspension arm, grace-deadline batch race + abort,
│                           #   Gate dispatch, resume-superstep vanguard seeding
├── mod.rs                  # extend: resume_with, replay, fork, with_shutdown_grace,
│                           #   RunOutcome::AwaitingInput reshape, new EngineError variants
├── graph.rs                 # extend: NodeSpec::Gate, GateRequestTemplate, validate() Gate
│                            #   checks, fingerprint() v3->v4 `;gates:` section
├── node.rs                   # extend: NodeContext.parley_response, parley_response() accessor
├── input_mapping.rs           # extend: `parley.` namespace (mirrors `muster.`, D-07)
├── directive_parser.rs         # extend: envelope `next: {"parley": {...}}` (mirrors D-11's
│                              #   existing goto/muster/end handling)
└── shutdown.rs                 # NEW -- ShutdownCoordinator, RunGuard (D-21)

crates/paladin-ports/src/
├── input/parley_port.rs        # NEW -- ParleyPort trait (D-25)
└── output/waypoint_port.rs      # extend: WaypointSummary.fork_of

crates/paladin-storage/src/waypoint/
└── contract_tests.rs             # extend: AwaitingInput{parleys,responses} round trip,
                                   #   fork_of round trip, latest-across-branches case

src/application/services/
├── chronicle.rs                  # NEW -- ChronicleService (D-16), beside waypoint_retention.rs
└── parley/                        # NEW -- facade ParleyPort adapter + GraphRegistry (D-25/D-26)

src/config/
├── engine.rs                      # extend: shutdown_grace_secs, graceful_shutdown fields
└── waypoint_store.rs               # NEW -- WaypointStoreConfig (D-26)

src/bin/paladin-server.rs           # extend: shutdown_signal cancels ShutdownCoordinator
src/config/setup/service_runner.rs  # extend: wait_for_shutdown cancels ShutdownCoordinator

crates/paladin-web/src/
├── thread_controller.rs            # NEW -- ThreadApiState, DTOs, thread_router (D-24)
├── openapi.rs                       # extend: compose thread paths into build_openapi
└── openapi.json                     # regenerated (UPDATE_OPENAPI=1 cargo test, per D-27)

tests/integration/
├── e2e_approval_gate_test.rs         # NEW -- E2E-2 (mirrors e2e_crash_resume_test.rs)
└── (multi-parley, cross-process, chronicle immutability/subgraph-fork, shutdown-grace,
     10-thread stress tests -- new files, exact names Claude's discretion)

docs/src/user-guides/parley-and-chronicle.md   # NEW, wired into SUMMARY.md after control-flow.md
```

### Pattern 1: Additive `#[serde(default)]` Waypoint field, no migration

**What:** Every new field on `Waypoint`/`WaypointSummary` (`fork_of`) follows the exact precedent
`visit_counts`/`frontier`/`muster_progress`/`checkpoint_ns` already set: `#[serde(default)]`, a
dedicated "payload without the new field deserializes with the old default" unit test that strips
the JSON key before deserializing (not just round-tripping a value that was already present), and
no SQL migration because the SQLite/Postgres backends store the whole Waypoint as a JSON payload
column.
**When to use:** Any new field on `Waypoint`, `WaypointSummary`, or `WaypointStatus::AwaitingInput`.
**Example (the exact pattern to replicate, from the shipped `checkpoint_ns` field):**
```rust
// Source: crates/paladin-core/src/platform/container/waypoint.rs:1191-1235 (verified in tree)
#[test]
fn waypoint_payload_without_checkpoint_ns_deserializes_as_none() {
    let waypoint = /* construct with checkpoint_ns: Some(...) */;
    let mut value = serde_json::to_value(&waypoint).unwrap();
    value.as_object_mut().unwrap().remove("checkpoint_ns"); // simulate a pre-field payload
    let restored: Waypoint = serde_json::from_value(value).unwrap();
    assert_eq!(restored.checkpoint_ns, None);
}
```
Apply identically for `fork_of: Option<WaypointId>` on both `Waypoint` and `WaypointSummary`
(D-14).

### Pattern 2: Length-prefixed, sorted, version-bumped fingerprint sections

**What:** `WarGraph::fingerprint()` hashes deterministically-sorted node/edge/schema sections into
one `blake3` digest, each field written through `push_field` (an 8-byte little-endian length
prefix, never a delimiter) to prevent the exact collision class Phase 22.1's CR-01 found and
fixed. Every layout change bumps `GRAPH_FINGERPRINT_VERSION`.
**When to use:** D-09's Gate section (`kind`, `output_field`, `choices`, `on_expire` kind — sorted
by node id, following the `;battalion:`/`;directive_parsers:` sections' exact shape at
`graph.rs:1169-1211`).
**Example:**
```rust
// Source: crates/paladin-battalion/src/engine/graph.rs:1172-1211 (verified in tree) --
// the EXACT precedent to extend for a new `;gates:` section:
buf.extend_from_slice(b";directive_parsers:");
for id in &self.node_order {
    let Some(NodeSpec::Paladin { directive_parser, .. }) = self.nodes.get(id) else { continue };
    push_field(&mut buf, id.as_str().as_bytes());
    let parser_json = serde_json::to_string(directive_parser).unwrap_or_default();
    push_field(&mut buf, parser_json.as_bytes());
}
// D-09: append a ";gates:" section here, same shape, then bump
// GRAPH_FINGERPRINT_VERSION from "v3" to "v4" and re-pin the golden hex test.
```
Also update `pub const GRAPH_FINGERPRINT_VERSION: &str = "v3";` → `"v4"` at
`waypoint.rs:263`, and its doc comment block (which already documents the `v1`→`v2`→`v3` history
verbatim — extend it with a `v4` paragraph in the same style, D-09).

### Pattern 3: Edge evaluation reads a node's `output_field`, not the whole Battlefield, for `Custom` evaluators

**What:** `evaluate_edge_condition` (superstep.rs:2299-2353) special-cases `NodeSpec::Paladin`
sources for the `Custom(name)` evaluator path only: it reads `battlefield.get::<String>(output_field)`
rather than the whole-Battlefield JSON string every other arm (`Contains`/`Regex`/the
`Custom`-evaluator fallback) uses.
**When to use:** D-06 requires a `Gate` source to be treated identically. `Contains`/`Regex`
already work with no code change (they read the whole rendered Battlefield JSON, and a Gate's
`output_field` is an ordinary schema field merged into that Battlefield on the post-resume
superstep) — **only the `Custom` evaluator match arm needs a new `NodeSpec::Gate` case.**
**Example:**
```rust
// Source: crates/paladin-battalion/src/engine/superstep.rs:2328-2334 (verified in tree)
let output = match graph.node(source) {
    Some(NodeSpec::Paladin { output_field, .. }) => battlefield
        .get::<String>(output_field)
        .ok()
        .flatten()
        .unwrap_or_default(),
    // D-06: add `| Some(NodeSpec::Gate { output_field, .. })` to this arm --
    // do NOT add a separate match arm; a Gate's output_field behaves identically
    // to a Paladin's for this one purpose.
    _ => serde_json::to_string(battlefield).unwrap_or_default(),
};
```

### Pattern 4: Namespaced `InputMapping` placeholder, resolved from `NodeContext` never the Battlefield

**What:** `{muster.payload}`/`{muster.task_key}` resolve from `NodeContext.muster: Option<MusterContext>`
passed as a *separate* parameter to `InputMapping::render`, never falling through to a Battlefield
read even if a schema field happens to share the name — and graph validation independently rejects
declaring a schema field with that prefix.
**When to use:** D-07's `parley.` namespace (`{parley.value}`, `{parley.prompt}`, `{parley.kind}`,
`{parley.responded_by}`), resolved from `NodeContext.parley_response: Option<ParleyResponse>`.
**Example:**
```rust
// Source: crates/paladin-battalion/src/engine/input_mapping.rs:143-202 (verified in tree)
fn resolve(placeholder: &str, state: &Battlefield, muster: Option<&MusterContext>)
    -> Result<String, InputMappingError> {
    if let Some(name) = placeholder.strip_prefix("muster.") {
        return Self::resolve_muster(name, placeholder, muster);
    }
    // ... falls through to Battlefield lookup only for non-namespaced placeholders
}
// D-07: InputMapping::render's signature must grow a THIRD parameter
// (`parley: Option<&ParleyResponse>`) alongside `muster`, with the identical
// strip_prefix("parley.") + resolve_parley dispatch, mirroring resolve_muster's
// shape exactly (UndeclaredField on no-context, never a silent Battlefield read).
```
This is a **breaking change to `InputMapping::render`'s signature** (adds a parameter) — every
existing call site in `superstep.rs` must be updated. Grep `InputMapping::render(` before starting
this task to enumerate every call site; do not miss one (a missed call site is a silent
compile-time-caught break here, not a runtime hazard, since Rust requires the new argument — but
still worth a task-level checklist item).

### Pattern 5: Config structs standalone under `src/config/`, never fields on `Settings`

**What:** `EngineConfig`, `WaypointRetentionConfig`, and every X-09 config struct in this codebase
is `Default` + `validate()` + `EnvOverridable` (`APP_*` env vars), constructed independently and
composed by the binary — `Settings` (all-pub, not `#[non_exhaustive]`) never grows a field for
them.
**When to use:** D-20's `shutdown_grace_secs`/`graceful_shutdown` on `EngineConfig`; D-26's new
`WaypointStoreConfig`.
**Example:**
```rust
// Source: src/config/engine.rs:40-141 (verified in tree) -- the EXACT shape to extend:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub max_supersteps: u64,
    pub max_node_visits: u32,
    pub run_timeout_secs: Option<u64>,
    #[serde(with = "waypoint_durability_serde")]
    pub waypoint_durability: WaypointDurability,
    pub max_muster_tasks: u32,
    // D-20 additions:
    // pub shutdown_grace_secs: u64,   // default 30, env APP_ENGINE_SHUTDOWN_GRACE_SECS
    // pub graceful_shutdown: bool,    // default true,  env APP_ENGINE_GRACEFUL_SHUTDOWN
}
// impl EnvOverridable::apply_env_overrides gains two more `if let Some(v) = read_env::<T>(...)`
// blocks, following the four existing ones verbatim (engine.rs:117-140).
// impl Default gains the two new fields at their documented defaults.
// A new test `default_engine_config_matches_todays_engine_defaults` (already exists,
// engine.rs:228) must keep passing UNCHANGED -- it asserts EngineLimits::default() equality,
// and shutdown_grace_secs/graceful_shutdown deliberately do NOT feed EngineLimits (D-20: "not
// part of EngineLimits and never hashed") -- do not add them to `impl From<EngineConfig> for
// EngineLimits` (engine.rs:143-169).
```

### Pattern 6: Injection-only port for `paladin-web` (never names an adapter or the facade)

**What:** `paladin-web` depends only on `paladin-ports` + `paladin-core`
[VERIFIED: crates/paladin-web/Cargo.toml lines 16-17] — no `paladin-battalion` edge in its default
build (ADR-0031). `AgentApiState` already carries this pattern via `provisioner:
Option<Arc<dyn AgentProvisioner>>`.
**When to use:** D-24/D-25's `ThreadApiState { waypoints: Option<Arc<dyn WaypointPort>>, parley:
Option<Arc<dyn ParleyPort>>, auth }` — both are `paladin-ports` trait objects, injected by the
binary (`src/bin/paladin-server.rs`), never constructed inside `paladin-web`.
**Example:**
```rust
// Source: crates/paladin-web/src/agent_controller.rs:58-69 (verified in tree) --
// the EXACT precedent ThreadApiState should follow:
#[derive(Clone)]
pub struct AgentApiState {
    pub registry: Arc<AgentRegistry>,
    pub provisioner: Option<Arc<dyn AgentProvisioner>>,  // <- injection-only trait object
    pub timeouts: TimeoutPolicy,
    pub jobs: Arc<JobStore>,
    pub auth: crate::agent_auth::AgentAuthConfig,
}
```

### Pattern 7: `202 Accepted` background-job pattern

**What:** `POST /agents/{id}/jobs` validates synchronously, `tokio::spawn`s the real work into a
`JobStore`, and returns `202 Accepted` with a poll handle immediately — never holding the HTTP
connection for the underlying work's duration.
**When to use:** D-25's `POST /v1/threads/{id}/resume`: validate via `WarEngine::resume_with`'s
synchronous validation path, spawn the continuation (registered with `ShutdownCoordinator`), return
`202 { thread_id, state_url }` for the client to poll `GET .../state`.
**Example:**
```rust
// Source: crates/paladin-web/src/agent_controller.rs:609-647 (verified in tree)
pub async fn enqueue_job(/* ... */) -> Result<(StatusCode, JsonValue), ApiError> {
    // ... synchronous validation (404/400 here) ...
    let job_id = state.jobs.create();
    tokio::spawn(async move { /* ... run, record outcome in job store ... */ });
    Ok((StatusCode::ACCEPTED, ok_body(&json!({ "job_id": job_id }))))
}
```

### Anti-Patterns to Avoid

- **Adding a per-handle `tokio::time::timeout` inside the existing `for (entry, handle) in
  dispatch_entries.iter().zip(handles) { handle.await... }` loop (superstep.rs:1258).** This awaits
  handles strictly in dispatch order; wrapping each individual `.await` in a timeout would abort
  handle N because handle N-1 took too long, not because handle N itself exceeded the shared
  deadline. The grace window is a single deadline shared by the whole batch — race the *collection*
  of handles against one `tokio::time::sleep_until(deadline)`, not each handle independently. See
  Pitfall 1 and the Code Examples section for the corrected shape.
- **Coercing `NextStep::Parley` to `Edges` on any code path.** D-01 through D-13 are built entirely
  on the premise that a Parley is never silently treated as an ordinary edge resolution — the
  existing `ParleyNotSupported` test (`superstep.rs:3380-3405`) already asserts "no AwaitingInput
  waypoint may be written for an unsupported Parley"; the replacement code must assert the inverse
  (an AwaitingInput waypoint IS written, containing every parley from the suspending superstep).
- **Reconstructing a child's Waypoint history by filtering a parent thread's history by
  `checkpoint_ns`.** The `checkpoint_ns` field's own rustdoc (waypoint.rs:551-560) explicitly
  states isolation comes ENTIRELY from `ThreadId::child`'s distinct derived id, and
  `checkpoint_ns` is a debugging record only — this same rule applies to D-18's
  `ThreadId::child_on_branch`: do not add a lookup path that filters by branch marker instead of
  using the derived thread id itself.
- **Treating `Contains`/`Regex` edge evaluation as needing a Gate-specific code change.** They
  already work against the whole rendered Battlefield JSON regardless of node kind (Pattern 3
  above) — only the `Custom` evaluator's `output_field`-extraction match arm needs a new case.
- **Adding `shutdown_grace_secs`/`graceful_shutdown` to `EngineLimits` or the fingerprint hash.**
  D-20 explicitly locks these as runtime settings, never graph settings — they must never be hashed
  (this would make `shutdown_grace` changes look like graph changes to `resume`'s `GraphMismatch`
  check).
- **Giving `paladin-web` a direct `paladin-battalion` dependency to reach `WarEngine::resume_with`
  directly.** ADR-0031 forbids this in the default build; go through `ParleyPort` (Pattern 6).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cross-process cancellation signal propagation to a suspended run | A custom database poll-loop or a new pub/sub channel | The existing `WaypointPort` durability contract itself — a suspended run's state lives entirely in the persisted `AwaitingInput` Waypoint; a different process's `resume_with` call is just a normal port read/write, no signaling needed (HITL-01's "resumable from a different process" requirement is satisfied by durability alone, not by any new coordination primitive) |
| Waiting for N spawned tasks with a shared deadline, then aborting stragglers | A hand-rolled poll loop over `JoinHandle::is_finished()` | `tokio::select!` draining a `FuturesUnordered<JoinHandle<T>>` against one `tokio::time::sleep_until(deadline)` branch, calling `.abort()` on every handle still in the set when the sleep branch fires | The standard, panic-safe, cancellation-safe shape for exactly this problem; `tokio::select!`'s cancellation safety guarantees no handle is silently dropped without either completing or being explicitly aborted |
| UUID-time-ordering for `ParleyId` | A custom counter or timestamp-prefixed string id | `Uuid::now_v7()`, exactly as `WaypointId::new()` already does (`waypoint.rs:171`) | `WaypointId` already solved "need a fresh, sortable-by-creation-order id" in this codebase; `ParleyId` is the identical problem, not a new one |
| Opaque pagination cursor for `GET /v1/threads/{id}/history` | A base64-encoded composite cursor object | The `waypoint_id` of the last returned item, documented as opaque to clients (D-27) | `WaypointPort::history`'s own `before: Option<WaypointId>` parameter already IS an exclusive cursor by id — no new encoding is needed, just don't document its internal structure to callers |

**Key insight:** This phase's "don't hand-roll" list is short because the phase itself is almost
entirely "don't hand-roll a NEW mechanism — extend the one that already exists." The one genuine
new primitive is the grace-deadline task race (`ShutdownCoordinator`), and even that composes
`tokio`'s own primitives rather than inventing anything.

## Common Pitfalls

### Pitfall 1: The superstep dispatch loop awaits `JoinHandle`s sequentially, not as a batch

**What goes wrong:** `superstep.rs:1258-1263` is `for (entry, handle) in
dispatch_entries.iter().zip(handles) { let (...) = handle.await...; }` — a plain sequential
`for` loop over `.await`, not `join_all`/`try_join_all`/`FuturesUnordered`. A naive
implementation of D-19's grace deadline (wrap each `.await` in `tokio::time::timeout(remaining,
handle)`) computes "remaining" independently per iteration, so a slow node earlier in
`dispatch_entries` order consumes grace time that a later, fast node never gets a fair share of —
and worse, if node 1 hangs past its own per-iteration timeout, the loop moves on to abort node 1
but has not even started *awaiting* node 2 yet, so node 2's own grace clock hasn't started. The
whole batch must be raced against ONE shared deadline computed once, before the loop.
**Why it happens:** The existing loop was written for the "wait for everyone, no deadline" case
(ENG-FR-01/22/23's pre-Phase-24 contract never needed a mid-batch bailout) — the sequential shape
is simplest for a full-wait, and became load-bearing before this phase needed to interrupt it.
**How to avoid:** Restructure the join phase to something like:
```rust
// Not verified in tree (this is new code this phase must write) -- shape, not exact source:
let deadline = cancel_observed_at.map(|t| t + shutdown_grace);
let mut remaining: FuturesUnordered<_> = handles.into_iter().enumerate().collect();
let mut results = vec![None; dispatch_entries.len()];
loop {
    match deadline {
        Some(dl) => tokio::select! {
            biased;
            Some((i, res)) = remaining.next() => { results[i] = Some(res); }
            _ = tokio::time::sleep_until(dl.into()), if !remaining.is_empty() => {
                // abort every still-outstanding handle; record Skipped { reason: "shutdown" }
                break;
            }
        },
        None => match remaining.next().await {
            Some((i, res)) => results[i] = Some(res),
            None => break,
        },
    }
}
```
Note `handles` must be `Vec<(usize, JoinHandle<T>)>` (or similarly indexed) so an abort can be
matched back to its `dispatch_entries` position for the `NodeExecutionRecord`/vanguard bookkeeping;
`FuturesUnordered` does not preserve input order. This is genuinely new logic, not a mechanical
extension — plan real time for it and a dedicated unit test asserting exact ordering/abort behavior
under a controlled clock (D-13's precedent: no clock abstraction, use explicit past-relative
deadlines in tests).
**Warning signs:** A shutdown-grace test that passes with 1 slow node but fails or flakes with 2+
slow nodes at different completion times is the signature of this bug.

### Pitfall 2: `resume_with_options`'s `Completed`-only short-circuit silently mishandles `AwaitingInput`/`Halted`

**What goes wrong:** `mod.rs:890-900` special-cases only `WaypointStatus::Completed` before
falling through to the generic vanguard-restore-and-run path. Today (pre-Phase-24) that fallthrough
is harmless for `Halted` (the generic path already produces the D-22-required "resume continues a
Halted thread" behavior with zero extra code — confirmed by reading the fallthrough: it restores
`latest.vanguard`, `latest.visit_counts`, `latest.frontier`, and calls `superstep::run` at
`latest.superstep + 1`, exactly what a resumed run needs). But for `AwaitingInput`, the SAME
fallthrough would incorrectly treat the parleying nodes' `vanguard` as ordinary, unfired vanguard
entries and re-run them as if nothing happened — silently discarding the pending Parley semantics.
**Why it happens:** `AwaitingInput` did not exist as a real, reachable status before this phase
(`ParleyNotSupported` prevented it from ever being written) — the fallthrough was never wrong until
this phase makes `AwaitingInput` a real, persisted status a generic `resume` can be called against.
**How to avoid:** Add an explicit `WaypointStatus::AwaitingInput { .. }` arm in
`resume_with_options` BEFORE the generic vanguard-restore path, returning
`EngineError::ThreadAwaitingInput { thread, parleys }` (D-11) — plain `resume`/`resume_with_options`
must never silently proceed past a pending Parley. Only `resume_with` (new method) is allowed to
advance an `AwaitingInput` thread.
**Warning signs:** A test that calls plain `resume()` (not `resume_with`) on a thread whose latest
Waypoint is `AwaitingInput` and asserts it returns an error, not `RunOutcome::Completed`/`Running`.

### Pitfall 3: `InputMapping::render`'s new third parameter is a breaking signature change with multiple call sites

**What goes wrong:** Adding `parley: Option<&ParleyResponse>` to `InputMapping::render` (Pattern 4)
changes every existing call site's arity. Missing one is a compile error (safe), but a lazier fix
of passing `None` everywhere except the one call site that actually needs it, without auditing
whether OTHER call sites should also thread a real `parley_response` through (e.g., a Gate node's
own post-resume dispatch path, if it also renders through `InputMapping` rather than writing
`output_field` directly), silently breaks `{parley.value}` resolution in a code path that compiles
fine and passes tests that don't specifically exercise it.
**Why it happens:** Rust's type system catches "forgot to pass an argument," not "passed the wrong
(always-`None`) value for a working but untested code path."
**How to avoid:** Grep `InputMapping::render(` across the whole `paladin-battalion` crate (not just
`superstep.rs`) before starting the parley-namespace task, and enumerate every call site by name in
the plan/task description, not just "update the call sites."

### Pitfall 4: `EngineConfig`'s existing `default_engine_config_matches_todays_engine_defaults` test is a tripwire, not decoration

**What goes wrong:** `engine.rs:227-234`'s test asserts `EngineLimits::from(EngineConfig::default())
== EngineLimits::default()`. If D-20's two new fields are added to `impl From<EngineConfig> for
EngineLimits` (even by accident, e.g. via a careless `..Default::default()` struct-update pattern
that pulls in unrelated fields), this test starts failing for the wrong reason, OR — worse — if it's
made to pass by ALSO changing `EngineLimits::default()`, that silently makes `shutdown_grace`
graph-fingerprint-relevant, directly contradicting D-20's "never hashed" requirement.
**Why it happens:** `EngineLimits`'s fields are exactly what the graph fingerprint hashes (per
23 D-18's note "every `EngineLimits` field... stays excluded from the hash" — wait, actually
re-verify: `EngineLimits` fields are NOT hashed into the fingerprint per that same note, but they
ARE part of `WarGraph` construction) — the risk is architectural conflation, not fingerprint
hashing directly, but the KEEP-SEPARATE discipline is the same one D-20 names explicitly.
**How to avoid:** `shutdown_grace_secs`/`graceful_shutdown` must NOT appear in `impl From<EngineConfig>
for EngineLimits` at all — they are consumed only by `WarEngine::with_shutdown_grace(Duration)`
(a new builder method, D-20), a completely separate code path from the `EngineLimits` conversion.
**Warning signs:** A diff touching `impl From<EngineConfig> for EngineLimits` in the same commit
that adds the two shutdown fields is a signal to re-check this boundary.

### Pitfall 5: `WaypointDurability::BestEffort` and a same-superstep partial-answer Waypoint

**What goes wrong:** D-11's partial-answer Waypoint ("a valid subset writes a child Waypoint at the
SAME superstep") is a second same-superstep-multiple-Waypoints precedent (the first being D-14's
mid-muster progress Waypoints). Under `WaypointDurability::BestEffort`, a failed `save()` is logged
and the run continues — for an ordinary superstep-complete Waypoint that's an accepted (if
discouraged) risk, but for a partial-answer Waypoint specifically, a lost write means a second
`resume_with` call against the SAME stale `AwaitingInput` Waypoint would look like re-submitting an
already-answered parley, which D-08 already anticipates ("if the process dies between validation
and that write, the AwaitingInput Waypoint is still the latest and re-submitting the same responses
is safe") — but only if the test suite actually proves idempotent re-submission under
`BestEffort`, not just under `Strict`.
**Why it happens:** `BestEffort` is rarely tested end-to-end (its own doc comment says "do not
select this in any example" — `mod.rs:93-98`) so its interaction with a NEW multi-Waypoint-per-
superstep code path is easy to leave unverified.
**How to avoid:** At minimum, confirm D-08's re-submission-is-safe claim in a test — it does not
need to force `BestEffort` specifically, but the "resubmit the same responses after a simulated
write failure" scenario (the `RecordingWaypointStore::fail_next_save` fixture already exists,
`test_support.rs:33-104`) is directly reusable here.
**Warning signs:** None of D-28's named test fixtures explicitly mention `fail_next_save` for the
partial-answer path — confirm the plan includes it, since the fixture already exists and D-08's
prose explicitly invokes this exact scenario ("D-08's durable-consumption test").

## Code Examples

### Extending `NodeOutcomeKind` and `WaypointStatus` (additive, `#[non_exhaustive]`)

```rust
// Source: crates/paladin-core/src/platform/container/waypoint.rs:401-479 (verified in tree)
// -- current shape to extend, NOT replace:
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NodeOutcomeKind {
    Succeeded,
    Failed,
    Skipped { reason: String },   // D-19 reuses this EXACT variant with reason: "shutdown"
    Ended,
    // D-03 adds: Parleyed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WaypointStatus {
    Running,
    Completed,
    Failed { error: String, failed_node: NodeId },
    AwaitingInput { parley: ParleyRequest },   // D-02 REPLACES this field:
    // -> AwaitingInput { parleys: Vec<ParleyRequest>, responses: Vec<ParleyResponse> }
    Halted,
}
```
Both enums are already `#[non_exhaustive]`, so adding `Parleyed` needs no X-10 register row beyond
the standard `MIGRATION.md` §9.2 entry (D-29). Changing `AwaitingInput`'s field SHAPE (not adding a
sibling variant) is a breaking change to that variant's payload — this is explicitly sanctioned by
D-02 ("one-way after release... the Waypoint payload is a stored contract") since v0.10.0 has not
shipped yet.

### The existing `ParleyNotSupported` test this phase's Parley-suspension test directly inverts

```rust
// Source: crates/paladin-battalion/src/engine/superstep.rs:3376-3406 (verified in tree)
// -- this EXACT test's assertions must flip once suspension is real:
next: NextStep::Parley(ParleyRequest { /* ... */ }),
// ...
error: EngineError::ParleyNotSupported { node },
// ...
assert!(
    waypoints.iter().all(|w| !matches!(w.status, WaypointStatus::AwaitingInput { .. })),
    "no AwaitingInput waypoint may be written for an unsupported Parley"
);
// D-02/D-03's replacement test asserts the OPPOSITE: exactly one AwaitingInput waypoint IS
// written, carrying every parley raised in that superstep, and RunOutcome::AwaitingInput is
// returned rather than RunOutcome::Failed.
```

### `RunOutcome` reshape target

```rust
// Source: crates/paladin-battalion/src/engine/mod.rs:102-141 (verified in tree)
pub enum RunOutcome {
    Completed { final_state: Battlefield, waypoint: WaypointId },
    AwaitingInput { parley: ParleyRequest, waypoint: WaypointId },
    // D-02 reshapes to: AwaitingInput { parleys: Vec<ParleyRequest>, waypoint: WaypointId }
    // (only the STILL-UNANSWERED requests, per D-02's RunOutcome note)
    Halted { waypoint: WaypointId },
    Failed { error: EngineError, waypoint: Option<WaypointId> },
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `NextStep::Parley` fails the run (`EngineError::ParleyNotSupported`) | Real suspension: merge peers, persist `AwaitingInput`, release resources | This phase (HITL-01) | The `ParleyNotSupported` variant stays on `EngineError` (removing a public enum variant is a breaking change this program forbids, X-03/Out of Scope table) but becomes unreachable dead code once every `NextStep::Parley` path routes to suspension — document it as superseded rather than deleting it |
| `WaypointStatus::AwaitingInput { parley: ParleyRequest }` (single) | `{ parleys: Vec<ParleyRequest>, responses: Vec<ParleyResponse> }` (multi, partial-answer-aware) | This phase (D-02) | Every consumer pattern-matching the old single-field shape breaks at compile time — this is intentional and total within this workspace (no external consumers yet, pre-1.0 stored-contract-breaking is sanctioned by D-02) |
| `GRAPH_FINGERPRINT_VERSION = "v3"` | `"v4"` (adds `;gates:` section) | This phase (D-09) | Every stored Waypoint's fingerprint from `v3` and earlier is recognized as stale on `resume` (typed `GraphMismatch`, never silently reinterpreted) — same non-breaking-at-runtime, breaking-for-old-threads pattern as `v1`→`v2`→`v3` |
| Cancellation observed ONLY at the superstep boundary (top of `loop`) | ALSO observed mid-superstep while awaiting spawned node tasks, with a grace deadline | This phase (D-19 extends ENG-FR-23) | The existing top-of-loop check (`superstep.rs:893-915`) is UNCHANGED — this is a second, independent observation point, not a replacement |

**Deprecated/outdated:** Nothing in this phase's scope is removed — every change is additive
(`#[non_exhaustive]` variant growth, `#[serde(default)]` field growth, new methods alongside
existing ones) per X-03's "deprecations allowed, removals are not (before v0.11.0)" rule.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|----------------|
| A1 | `tokio`'s `sync` feature (for `tokio::sync::Notify`) is already enabled in `paladin-battalion`'s `Cargo.toml` via a broad feature set (e.g. `full`/`rt-multi-thread`) rather than a minimal explicit feature list | Standard Stack / Supporting | Low — if not enabled, adding it is a one-line `Cargo.toml` change with no external-package legitimacy concern (it's the `tokio` crate already in the dependency tree); verify with `grep tokio crates/paladin-battalion/Cargo.toml` at plan time before assuming |
| A2 | `Uuid::now_v7()` (used for `ParleyId`, mirroring `WaypointId`) does not require a `uuid` crate feature flag beyond what `WaypointId` already uses | Don't Hand-Roll | Low — `WaypointId::new()` already calls `Uuid::now_v7()` in the shipped tree (`waypoint.rs:171`), so whatever feature enables it for `WaypointId` already covers `ParleyId` in the same crate |
| A3 | `axum` 0.8.4 (cited from `docs/src/deployment/production.md:342`, not re-verified against `Cargo.lock` directly in this research pass) is still the pinned version at Phase 24's start | Standard Stack | Low — this affects only whether `axum`-specific API shapes (e.g. `with_graceful_shutdown`, `nest`) referenced in Code Examples/Patterns are current; a version drift would surface as a compile error immediately, not a silent behavioral gap |

**If this table is empty:** N/A — three low-risk assumptions logged above, none blocking planning;
none require a `checkpoint:human-verify` gate since none involve installing new external packages
or making an irreversible architectural commitment.

## Open Questions

1. **Exact restructuring shape of the superstep dispatch/join loop for the grace deadline (Pitfall 1).**
   - What we know: the current loop is strictly sequential (`.zip(handles)` + per-iteration
     `.await`); a batch-level race against one shared deadline is required; `FuturesUnordered`
     loses input order and needs an explicit index to restore it for `NodeExecutionRecord`
     ordering (which the tree's existing tests may or may not depend on being dispatch-order —
     verify whether `completed_records` order is asserted anywhere before assuming it can shift).
   - What's unclear: whether `completed_records`'s current append order (dispatch order, via
     `for (entry, handle) in dispatch_entries.iter().zip(handles)`) is load-bearing for any
     existing determinism test (ENG-FR-04/08's "byte-identical Battlefields" claims are about the
     MERGED Battlefield, which is independently sorted before merge — but `completed` on the
     Waypoint itself may not be re-sorted).
   - Recommendation: grep existing tests asserting `waypoint.completed`'s order before deciding
     whether the grace-deadline restructuring needs to preserve or may relax dispatch-order
     appending; if any test depends on it, sort `completed_records` by original dispatch index
     after the race resolves, not by completion order.

2. **Whether `GateRequestTemplate`'s `expires_in: Option<Duration>` needs `serde` support for
   `Duration` directly, or should be stored as a numeric seconds field.**
   - What we know: `EngineConfig` already has a "no serde on `Duration` directly, store as u64
     seconds" pattern precedent is NOT quite this — `EngineConfig.run_timeout_secs: Option<u64>`
     stores seconds, then `.map(Duration::from_secs)` converts at the `EngineLimits` boundary. A
     `NodeSpec`/`GateRequestTemplate` is Rust-constructed (graph-building code), not necessarily
     `serde`-round-tripped the way config is.
   - What's unclear: whether `NodeSpec` (and therefore `GateRequestTemplate`) needs to be
     `Serialize`/`Deserialize` at all — a scan of `graph.rs`'s `NodeSpec` enum shows NO `#[derive(Serialize,
     Deserialize)]` on it (it holds `Arc<dyn StateNode>` and `Box<Paladin>`, both
     non-serializable) — so `GateRequestTemplate` likely does NOT need to serialize `Duration`
     at all, and can use `std::time::Duration` directly like `EngineLimits.run_timeout` does.
   - Recommendation: confirm `NodeSpec` has no `Serialize`/`Deserialize` derive (grep already shows
     none) before assuming a `Duration`-serde shim is needed; it almost certainly is not.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|--------------|------------|---------|----------|
| Rust toolchain | All Tier 1 tests, compilation | ✓ | `cargo 1.97.1`, `rustc 1.97.1` (workspace `rust-version = "1.88"`, MSRV floor) [VERIFIED: `cargo --version`, `Cargo.toml:18`] | — |
| SQLite (via `rusqlite`/`sqlx`, bundled) | `SqliteWaypointStore` Tier 1 tests, cross-process E2E-2 test (two engine instances over one file) | ✓ | System `sqlite3` CLI 3.40.1 present [VERIFIED: `sqlite3 --version`]; the Rust crates bundle their own SQLite via feature flags regardless | — |
| Docker | Postgres contract-suite Tier 2 cases (D-02/D-14/D-15 additions) | ✗ | — | Route to UAT via CI's `postgres-integration` job (already the established D-28/STATE.md pattern for this project) — **never mark these cases passed locally**, matching the carried concern from Phase 23's close |
| Postgres | Same as Docker (Postgres runs via Docker Compose in CI) | ✗ (no Docker) | — | Same fallback — CI-only |

**Missing dependencies with no fallback:** none — Docker/Postgres has an established, already-used
fallback (CI's `postgres-integration` job).

**Missing dependencies with fallback:** Docker/Postgres, routed to CI per the existing D-28
pattern.

## Validation Architecture

`workflow.nyquist_validation` is absent from `.planning/config.json` (only `_auto_chain_active`
and `worktree_skip_hooks` are set) — treated as enabled per the default rule; this section is
required.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]`/`#[tokio::test]` via `cargo test`, workspace-standard (no `nextest` reference found in `Makefile`) |
| Config file | none — plain `cargo test` / `[[test]]` entries in root `Cargo.toml` (verified pattern at `Cargo.toml:201-271`) |
| Quick run command | `cargo test -p paladin-battalion` / `cargo test -p paladin-core` / `cargo test -p paladin-web` (per-crate, fast, Tier 1 only) |
| Full suite command | `cargo test` (workspace) for Tier 1; `make test-integration-docker` for Tier 2 (Postgres) — unavailable locally in this devcontainer, CI-only |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|---------------------|--------------|
| HITL-01 | Parley suspension merges peers, persists multi-parley `AwaitingInput`, releases resources, resumable cross-process | unit + integration | `cargo test -p paladin-battalion parley` (unit); new cross-process test over `SqliteWaypointStore` | ❌ Wave 0/1 — new file |
| HITL-01 | `Gate` node raises on first visit, delivers on resume | unit | `cargo test -p paladin-battalion gate` | ❌ Wave 0/1 — new tests in `superstep.rs`/`graph.rs` |
| HITL-02 | `resume_with` validates typed responses per `ParleyKind`, leaves thread suspended on error | unit | `cargo test -p paladin-battalion resume_with` | ❌ Wave 0/2 |
| HITL-02 | E2E-2 (approval gate, both branches, process drop/recreate) | integration | `cargo test --test e2e_approval_gate` | ❌ Wave 0/3 — new `tests/integration/e2e_approval_gate_test.rs` + `[[test]]` entry (mirrors `e2e_crash_resume` at `Cargo.toml:259-262`) |
| HITL-03 | `replay`/`fork` byte-identical mainline, `fork_of` lineage, `latest_on_branch` | unit + integration | `cargo test -p paladin-battalion chronicle` / `fork` / `replay` | ❌ Wave 0/4 |
| HITL-03 | Subgraph fork never shares child Waypoints | integration | new test extending `tests/integration/subgraph_formation_in_campaign_test.rs`'s pattern | ❌ Wave 0/4 |
| HITL-04 | Grace window finishes in-flight superstep, aborts stragglers as `Skipped`, re-lists in vanguard | unit + stress | `cargo test -p paladin-battalion shutdown` (`flavor = "multi_thread"`, X-05 pattern) | ❌ Wave 0/5 |
| HITL-04 | `resume` continues a `Halted` thread | unit | already-passable via the existing generic fallthrough (Pitfall 2) — add an explicit assertion test | ❌ Wave 0/5 (small addition — mechanism already correct) |
| HITL-04 | 10 suspended threads resumed concurrently (X-05 stress, acceptance 7) | integration/stress | new `tests/integration/*_stress_test.rs`, `flavor = "multi_thread"`, timeout guard | ❌ Wave 0/5 |
| HITL-05 | `GET/POST /v1/threads/*` 409/400/404/501 semantics, `openapi.json` regenerated | integration (`tower::util::oneshot`) | `cargo test -p paladin-web thread` | ❌ Wave 0/6 — new `thread_controller.rs` tests |
| HITL-04 (Postgres contract cases) | D-02/D-14/D-15 Waypoint payload additions round-trip on Postgres | Tier 2 (CI-only) | `make test-integration-docker` (CI's `postgres-integration` job) | ❌ Wave 0/1 — extend `contract_tests.rs`; **not runnable locally, no Docker** |

### Sampling Rate

- **Per task commit:** `cargo test -p <touched-crate>` (fast, Tier 1)
- **Per wave merge:** `cargo test` (full workspace Tier 1) + `cargo fmt --check` + `cargo clippy -- -D warnings`
- **Phase gate:** Full Tier 1 suite green locally; Tier 2 (Postgres) proven only via CI's
  `postgres-integration` job — route to UAT per the Phase 23 carried-concern precedent, never
  mark it passed from a local run in this devcontainer.

### Wave 0 Gaps

- [ ] `tests/integration/e2e_approval_gate_test.rs` — covers HITL-02 (E2E-2), needs a `[[test]]`
      entry in root `Cargo.toml` mirroring `e2e_crash_resume` (`Cargo.toml:259-262`)
- [ ] New cross-process test (two `WarEngine` instances, one `SqliteWaypointStore` file) — covers
      HITL-01's "resumable from a different process" clause; likely lives beside
      `e2e_crash_resume_test.rs` given that file's own documented rationale for simulating a crash
      via re-seeding a fresh SQLite file (directly reusable technique, see that file's own header
      comment, `tests/integration/e2e_crash_resume_test.rs:8-22`)
  - [ ] A stress test file for acceptance 7 (X-05 pattern: 10 suspended threads resumed
      concurrently, exact counts, `flavor = "multi_thread"`, timeout guard)
- [ ] `contract_tests.rs` extensions for D-02 (`AwaitingInput{parleys,responses}`), D-14
      (`fork_of`), D-15 (latest-across-branches) — run on InMemory + SQLite locally; Postgres via
      CI only
- [ ] `crates/paladin-web/src/thread_controller.rs`'s own `#[cfg(test)]` module (oneshot tests
      mirroring `agent_controller.rs:747+`)
- No new test framework/tooling install needed — `cargo test` and the existing `[[test]]`
  registration mechanism cover every new test file this phase adds.

## Security Domain

`security_enforcement` is not set to `false` anywhere found in `.planning/config.json` — treated
as enabled; this section is required. `.github/instructions/security.instructions.md` is the
project's authoritative security posture document and is already referenced in CONTEXT.md's
canonical refs for this exact reason (the `state` endpoint's raw-prompt/output exposure warning,
M-B-04).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|-----------------|---------|--------------------|
| V2 Authentication | yes (indirectly) | New `/v1/threads/*` routes reuse the SAME `require_authentication` middleware already applied to `/v1/agents/*` (`agent_controller.rs:713-718`) — no new auth mechanism, this phase's D-24 explicitly composes behind "the same auth middleware" |
| V3 Session Management | no | This is a stateless-token API (opaque bearer / API key), unchanged by this phase — no session state introduced |
| V4 Access Control | yes | D-24: "authenticated callers, any role; scopes are PLAT-06" — this phase deliberately defers admin/writer scoping to Phase 27 (PLAT-06); document this as an accepted interim posture, not an oversight, in the mdBook page (D-30) |
| V5 Input Validation | yes | `resume_with`'s per-`ParleyKind` typed validation matrix (D-10) IS the input-validation control for this phase's highest-risk surface — a `StateEdit` response deserializes to a `StateDelta` validated against the schema, with an unknown field rejecting the RESPONSE not the run (never partial-apply an invalid edit) |
| V6 Cryptography | no | No new cryptographic primitive is introduced — `blake3` fingerprinting is unchanged (integrity, not confidentiality; already covered by Phase 22's original threat model) |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| A parley `payload`/`prompt` containing an embedded credential or secret reaching a log/error/HTTP response | Information Disclosure | `security.instructions.md`'s existing rule ("response bodies redacted BEFORE truncation... no log statement interpolates an API key") already governs this class generally; this phase's SPECIFIC new risk is the `GET /v1/threads/{id}/state` endpoint returning `parleys`/`responses` verbatim — since `payload` is author-controlled (M-B-04's warning, cited in CONTEXT.md canonical refs), the mdBook doc page (D-30) must explicitly warn graph authors never to put secrets in a Gate's `payload_template` |
| A malicious/buggy `on_expire: ResumeWithDefault(value)` substituting an attacker-influenced default that bypasses intended approval-gate branching | Tampering / Elevation of Privilege | D-12 requires the default value be "validated per kind at graph-validate time (a Gate) or at raise time (a Directive)" — the SAME typed validation matrix as a real response (D-10), never a raw unchecked substitution; confirm the plan's `on_expire` tasks reuse the exact same validator function as `resume_with`, not a parallel weaker check |
| A resumed thread whose `graph_fingerprint` no longer matches (graph redeployed between suspend and resume) silently continuing against semantically different routing | Tampering | Already covered structurally — `resume_with` inherits `resume`'s existing `GraphMismatch` check (D-10: "checks GraphMismatch (ENG-FR-14)"); no new gap, just confirm the new method doesn't accidentally bypass it |
| Cross-thread parley id collision letting one thread's `resume_with` call answer a DIFFERENT thread's outstanding parley | Spoofing / Tampering | `resume_with(graph, thread, responses)` is scoped by `thread: &ThreadId` from the start — the loaded `AwaitingInput` Waypoint's own `parleys` list is the only valid target set for `UnknownParleyId` checking (D-10); `ParleyId` (UUIDv7, D-01) is not attacker-guessable in practice, but the validation must still check membership in the LOADED thread's own outstanding set, never a global parley-id lookup across threads |
| `POST /v1/threads/{id}/resume`'s background-spawned continuation running unbounded after the requesting connection is gone | Denial of Service (resource exhaustion) | D-25's "spawns the continuation as a background task registered with the ShutdownCoordinator" is itself the mitigation — the task is bounded by the SAME engine limits (`max_supersteps`, `max_node_visits`) every other run is, and is now also bounded by graceful-shutdown's grace window rather than running forever past a shutdown signal |

## Sources

### Primary (HIGH confidence — direct tree reads, 2026-09-04)

- `crates/paladin-core/src/platform/container/waypoint.rs` (1267 lines, read in full) — `ThreadId`,
  `WaypointId`, `GraphFingerprint`/`GRAPH_FINGERPRINT_VERSION`, `NodeOutcomeKind`, `ParleyRequest`
  stub, `WaypointStatus`, `Waypoint`, `FrontierSnapshot`, `MusterProgress`
- `crates/paladin-core/src/platform/container/directive.rs` (151 lines, read in full) —
  `Directive`, `NextStep` (incl. `Parley`), `MusterTask`, `MusterContext`
- `crates/paladin-battalion/src/engine/node.rs` (67 lines, read in full) — `StateNode`,
  `NodeContext`, `NodeError`
- `crates/paladin-battalion/src/engine/input_mapping.rs` (433 lines, read in full) —
  `InputMapping::render`, `muster.` namespace precedent
- `crates/paladin-battalion/src/engine/directive_parser.rs` (385 lines, read in full) — envelope
  extraction, `DirectiveParser`, `OnParseError`
- `crates/paladin-ports/src/output/waypoint_port.rs` (426 lines, read in full) — `WaypointPort`,
  `WaypointSummary`, `WaypointError`, `prune_thread`
- `crates/paladin-battalion/src/engine/superstep.rs` (targeted reads: lines 400-520, 780-930,
  1050-1410, 2280-2360) — the child-Battalion `AwaitingInput` arm, boundary cancellation check,
  the tokio::spawn dispatch/join loop, `evaluate_edge_condition`
- `crates/paladin-battalion/src/engine/mod.rs` (targeted reads: lines 90-350, 600-970) —
  `RunOutcome`, `EngineError`, `WarEngine` builders, `start`/`resume`/`resume_with_options`
- `crates/paladin-battalion/src/engine/graph.rs` (targeted reads: lines 1-220, 1100-1225) —
  `NodeSpec`, `StateMap`, `fingerprint()`, `push_field`
- `src/application/services/waypoint_retention.rs` (targeted read) — `protected_waypoints`,
  the phase's own forward-looking seam comments naming Phase 24 by name
- `src/config/engine.rs` (494 lines, read in full) — `EngineConfig` pattern to extend for D-20
- `crates/paladin-web/src/agent_controller.rs` (targeted reads: lines 1-100, 580-760) —
  `AgentApiState`, the `202 Accepted` job pattern, `API_V1_PREFIX`/router composition
- `crates/paladin-web/src/error.rs` (242 lines, read in full) — `ApiError`/`ApiErrorBody` envelope
- `crates/paladin-web/src/openapi.rs` (210 lines, read in full) — `build_openapi`, drift-guard test
- `src/bin/paladin-server.rs` (363 lines, read in full) — `shutdown_signal`,
  `axum::serve(...).with_graceful_shutdown`, state composition
- `src/config/setup/service_runner.rs` (targeted reads: lines 1-100, 200-320) — `wait_for_shutdown`
- Grep verification: `crates/paladin-*/Cargo.toml` (dependency confirmation), `k8s/*.yaml`
  (`terminationGracePeriodSeconds` absence), `docs/src/deployment/production.md` (axum 0.8.4,
  the "30" default cited by D-23), `docs/src/SUMMARY.md` (control-flow page anchor),
  `crates/paladin-storage/src/waypoint/{in_memory,sqlite,postgres}.rs` (latest ordering),
  `tests/integration/e2e_crash_resume_test.rs` + root `Cargo.toml` `[[test]]` block (E2E test
  registration pattern), `Makefile` (test/coverage commands)
- `.planning/phases/24-pause-resume-history-graceful-shutdown/24-CONTEXT.md` — D-01…D-30, the
  phase's locked decisions, read in full
- `.planning/REQUIREMENTS.md`, `.planning/STATE.md` — project-level requirement text and
  cross-phase carried concerns (Postgres/Docker unavailability, prior fingerprint-version history)

### Secondary (MEDIUM confidence)

- None used — this phase required no external documentation lookup (no new library), so no
  Context7/web-search source was consulted.

### Tertiary (LOW confidence)

- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new dependencies, every crate/version claim verified by direct
  `Cargo.toml`/binary-version read in this session
- Architecture: HIGH — every pattern cited is a direct quote/paraphrase of shipped, currently
  compiling code at an exact line range, not inferred from documentation
- Pitfalls: HIGH for Pitfalls 2-5 (each traced to an exact current code shape and its precise
  interaction with a locked decision); MEDIUM for Pitfall 1 (the dispatch-loop restructuring is
  genuinely new code this phase must design, not merely extend — flagged as the phase's primary
  engineering risk rather than a known trap with a known fix)

**Research date:** 2026-09-04
**Valid until:** Effectively pinned to the current commit on `feature/phase-22` — this research is
tree-state-dependent (exact line numbers, exact enum shapes) rather than library-version-dependent,
so it should be re-verified (not necessarily re-researched) if Phase 24 planning/execution is
delayed past a point where `superstep.rs`/`mod.rs`/`graph.rs` receive unrelated changes from a
concurrently landed phase or hotfix.
