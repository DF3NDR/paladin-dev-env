# PRD 03 — Pause/Resume (Parley), Execution History & Forking (Chronicle), Graceful Shutdown (Epic `HITL`)

**Depends on:** PRD 01 (Waypoints, engine, cancellation seam), PRD 02 (Directive::Parley variant).
**Primary crates:** `paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-web` (endpoints), facade.

---

## 1. Problem Statement

Paladin workflows currently run to completion or fail; the only client interaction is an SSE stream that can time out. There is no way to (a) pause a run at a decision point and wait — for minutes or days — for a human to approve, edit, or choose, without holding a connection or a thread; (b) inspect the historical states of a run; (c) rewind to an earlier state and try an alternate path; or (d) stop an in-flight run during a deploy without losing work.

Human intervention points are now a baseline expectation for production agent systems (approval of destructive actions, review of tool calls, correction of state). This epic delivers **Parley** (pause/resume), **Chronicle** (history, replay, fork), and **graceful shutdown**, all as thin, well-specified layers over the Waypoint substrate from PRD 01.

## 2. Domain Design

### 2.1 Parley types (in `paladin-core`)

```rust
pub struct ParleyRequest {
    pub parley_id: Uuid,
    pub node_id: NodeId,
    pub kind: ParleyKind,
    pub prompt: String,                       // human-readable question/instruction
    pub payload: serde_json::Value,           // structured context (e.g. the proposed action)
    pub choices: Option<Vec<String>>,         // for Choice kind
    pub expires_at: Option<DateTime<Utc>>,    // optional deadline
    pub created_at: DateTime<Utc>,
}

pub enum ParleyKind {
    Approval,          // expect boolean-ish response
    Choice,            // expect one of `choices`
    FreeText,          // expect arbitrary string
    StateEdit,         // expect a StateDelta to apply before resuming
}

pub struct ParleyResponse {
    pub parley_id: Uuid,
    pub value: serde_json::Value,             // shape validated per kind (HITL-FR-05)
    pub responded_by: Option<String>,         // audit identity, free-form
    pub responded_at: DateTime<Utc>,
}
```

### 2.2 Fork lineage (extends `Waypoint` from PRD 01)

`Waypoint` gains `pub fork_of: Option<WaypointId>` (None for the mainline). `parent_waypoint_id` + `fork_of` together make the Chronicle a tree. `WaypointSummary` exposes both plus `status`, `superstep`, `created_at`.

## 3. Functional Requirements

### 3.1 Parley — pause

- **HITL-FR-01 (Raising a parley).** A node returns `NextStep::Parley(ParleyRequest)` (Function nodes directly; Paladin nodes via a `DirectiveParser` that recognizes a documented JSON envelope, and via a first-class `NodeSpec::Gate { request_template }` node type that always parleys — the Gate is the primary approval-gate building block and MUST render `prompt`/`payload` from the Battlefield with the same templating as `InputMapping`).
- **HITL-FR-02 (Suspension semantics).** On Parley, the engine: merges the emitting superstep's deltas (peer nodes complete normally); persists a Waypoint with `status: AwaitingInput { parley }`; releases ALL resources (no task, timer, or connection remains for that run); returns `RunOutcome::AwaitingInput { parley, waypoint }`. A suspended thread MUST survive full process termination and be resumable from a different process instance sharing the WaypointPort backend (integration test required).
- **HITL-FR-03 (Multiple parleys in one superstep).** If several nodes parley in the same superstep, ALL requests are recorded on the Waypoint (`AwaitingInput` carries `Vec<ParleyRequest>` — adjust the PRD-01 stub accordingly) and ALL must be answered before the run continues. Responses are matched by `parley_id`; answering a subset keeps the thread suspended and MUST be queryable as partially-answered.
- **HITL-FR-04 (Resume API).** `WarEngine::resume_with(&self, graph, thread: ThreadId, responses: Vec<ParleyResponse>) -> Result<RunOutcome, EngineError>`. Errors (all typed): `ThreadNotAwaitingInput`, `UnknownParleyId`, `ParleyExpired`, `ResponseShapeInvalid { parley_id, reason }`, `GraphMismatch` (per ENG-FR-14). The response value is delivered to the paused node's continuation: for `Gate` nodes the value is written to a configured `output_field` and routing proceeds via static edges/conditions (so an approval gate is just Gate + two conditional edges); for Function/Paladin parleys the node re-runs with `NodeContext::parley_response()` populated (documented contract: parley-raising nodes MUST be written idempotent up to the parley point).
- **HITL-FR-05 (Response validation).** Before resuming: `Approval` accepts JSON true/false or "yes"/"no"/"approve"/"deny" (case-insensitive, normalized to bool); `Choice` must exactly match one of `choices`; `StateEdit` must deserialize to a `StateDelta` valid against the schema (ENG-FR-10 rules apply — unknown field rejects the response, not the run). Invalid responses reject with `ResponseShapeInvalid` and leave the thread suspended.
- **HITL-FR-06 (Expiry).** If `expires_at` passed: `resume_with` fails with `ParleyExpired`; a per-request `on_expire: FailRun | ResumeWithDefault(Value)` policy, evaluated lazily at resume-time or by the Doc 06 scheduler sweep — no background timer is required by this epic.

### 3.2 Chronicle — history, replay, fork

- **HITL-FR-07 (History).** `ChronicleService::history(thread, page) -> Vec<WaypointSummary>` (application service over `WaypointPort::history`), newest-first, including fork branches with lineage fields so a client can reconstruct the tree.
- **HITL-FR-08 (Inspect).** `ChronicleService::inspect(thread, waypoint_id) -> Waypoint` returns the full snapshot (Battlefield, vanguard, records, status).
- **HITL-FR-09 (Replay).** `WarEngine::replay(graph, thread, from: WaypointId) -> RunOutcome` re-executes forward from a historical Waypoint **as a fork**: a new Waypoint chain is created with `fork_of = from`; the original chain is never mutated or truncated (immutability is a hard invariant — test asserts the original chain is byte-identical after replay).
- **HITL-FR-10 (Fork with edit).** `WarEngine::fork(graph, thread, from: WaypointId, edit: StateDelta) -> RunOutcome` = apply `edit` to the historical Battlefield (schema-validated), then replay. This is the "what-if" primitive.
- **HITL-FR-11 (Fork identity).** Forked runs continue under the same `ThreadId` (branches distinguished by lineage). `latest(thread)` returns the most recently created Waypoint across branches; `ChronicleService` additionally exposes `latest_on_branch(thread, branch_root)`. Document that resume-without-branch-qualifier resumes the newest branch.
- **HITL-FR-12 (Subgraph interaction).** Replaying from a Waypoint whose vanguard contains a subgraph node re-executes that child from scratch on the fork (the fork does not share the child's namespaced waypoints); test required.

### 3.3 Graceful shutdown

- **HITL-FR-13 (Cooperative halt).** Built on ENG-FR-23: triggering the engine's `CancellationToken` lets the in-flight superstep finish (all its node executions and merge), persists a `Halted` Waypoint, and returns `RunOutcome::Halted`. In-flight LLM calls are awaited up to `shutdown_grace: Duration` (config, default 30 s); past the grace period their node records `Skipped("shutdown")`, their deltas are discarded, and the Waypoint's vanguard re-includes them so resume re-executes them.
- **HITL-FR-14 (Resume after halt).** `resume` on a `Halted` thread continues normally (this is just ENG-FR-12; test explicitly).
- **HITL-FR-15 (Process integration).** The facade's `ServiceRunner` wires SIGTERM/SIGINT to cancellation tokens of all in-flight engine runs and delays process exit until all have halted or `shutdown_grace` elapses. Kubernetes docs updated: `terminationGracePeriodSeconds` guidance ≥ 2 × `shutdown_grace`. This is an operator-visible behavioral change pre-registered as `MIGRATION.md` M-B-02; the `k8s/` manifests in the repo are updated in this requirement, and a config switch to disable the wait entirely (legacy-only deployments with no engine runs) MUST exist and be documented there.

### 3.4 Web exposure (minimal — the full API surface is Doc 06)

- **HITL-FR-16.** `paladin-web` gains, for engine-backed runs: `GET /threads/{thread_id}/state` (latest Waypoint summary + parley requests if suspended), `POST /threads/{thread_id}/resume` (body: `Vec<ParleyResponse>`; 409 `ThreadNotAwaitingInput`, 400 shape errors, 404 unknown thread), `GET /threads/{thread_id}/history` (paginated). OpenAPI spec regenerated; endpoints follow the existing utoipa + error-envelope conventions in `agent_controller.rs`.

## 4. Acceptance Criteria

1. E2E-2 (overview §6): approval gate, both branches, across a full process drop/recreate.
2. Multi-parley superstep: two gates in one superstep; answering one keeps suspension; answering both resumes; asserted via state + status.
3. Replay immutability: replay from superstep 2 of a 5-superstep run; original chain unchanged; fork chain correct; `fork_of` lineage verifiable.
4. Fork-with-edit changes routing: edited field flips a conditional edge; fork takes the other branch than mainline.
5. Shutdown test: cancel mid-superstep with one slow mock node exceeding grace; assert `Halted` Waypoint re-lists the slow node in vanguard; resume completes it exactly once.
6. HTTP tests for HITL-FR-16 status codes and bodies.
7. Coverage per X-02; multi-thread stress: 10 concurrent suspended threads resumed concurrently, exact outcomes.
8. **Versioning gate (X-10/X-11):** any pre-existing public type touched by this epic is recorded in `MIGRATION.md` §9.2 with its mitigation; `cargo semver-checks` and the MSRV job pass; new dependencies listed in §9.3; new migrations in §9.4; new config/env in §9.5.

## 5. Test Plan (TDD ordering)

1. Parley type validation units (HITL-FR-05 matrix — every kind × valid/invalid).
2. Gate node unit tests (template rendering, output_field write, edge routing on response).
3. Suspension/resume integration with InMemory backend, then SQLite (cross-process simulation = two engine instances, one store).
4. Multi-parley tests.
5. Chronicle history/inspect/replay/fork tests incl. immutability and subgraph-fork.
6. Shutdown tests (fast node, slow node, grace expiry).
7. Web endpoint tests (axum tower::util oneshot pattern already used in `paladin-web`).

## 6. Out of Scope

Notification of humans that a parley is waiting (compose with existing `paladin-notifications` in application code; a doc example is required but no new port); authz on who may respond (Doc 06 platform concerns); UI.
