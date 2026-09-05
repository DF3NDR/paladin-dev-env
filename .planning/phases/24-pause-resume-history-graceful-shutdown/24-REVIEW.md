---
phase: 24-pause-resume-history-graceful-shutdown
reviewed: 2026-09-05T08:25:13Z
depth: standard
files_reviewed: 36
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - .project/v0.10.0/08-traceability-matrix.md
  - crates/paladin-battalion/src/engine/directive_parser.rs
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/input_mapping.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/node.rs
  - crates/paladin-battalion/src/engine/shutdown.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/src/llm_decision.rs
  - crates/paladin-core/src/platform/container/directive.rs
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/parley.rs
  - crates/paladin-core/src/platform/container/waypoint.rs
  - crates/paladin-ports/src/input/mod.rs
  - crates/paladin-ports/src/input/parley_port.rs
  - crates/paladin-ports/src/output/waypoint_port.rs
  - crates/paladin-storage/src/waypoint/contract_tests.rs
  - crates/paladin-storage/src/waypoint/in_memory.rs
  - crates/paladin-storage/src/waypoint/postgres.rs
  - crates/paladin-storage/src/waypoint/retention.rs
  - crates/paladin-storage/src/waypoint/sqlite.rs
  - crates/paladin-web/Cargo.toml
  - crates/paladin-web/openapi.json
  - crates/paladin-web/src/agent_auth.rs
  - crates/paladin-web/src/agent_controller.rs
  - crates/paladin-web/src/lib.rs
  - crates/paladin-web/src/openapi.rs
  - crates/paladin-web/src/thread_controller.rs
  - docs/src/SUMMARY.md
  - docs/src/deployment/kubernetes.md
  - docs/src/deployment/production.md
  - docs/src/user-guides/parley-and-chronicle.md
  - k8s/README.md
  - k8s/deployment.yaml
  - k8s/server/deployment.yaml
  - src/application/services/chronicle.rs
  - src/application/services/mod.rs
  - src/application/services/parley/adapter.rs
  - src/application/services/parley/mod.rs
  - src/application/services/parley/registry.rs
  - src/application/services/waypoint_retention.rs
  - src/bin/paladin-server.rs
  - src/config/engine.rs
  - src/config/mod.rs
  - src/config/setup/service_runner.rs
  - src/config/waypoint_store.rs
  - tests/integration/e2e_approval_gate_test.rs
  - tests/integration/multi_parley_suspension_test.rs
  - tests/integration/parley_resume_stress_test.rs
  - tests/integration/subgraph_formation_in_campaign_test.rs
  - tests/integration/waypoint_retention_fault_injection_test.rs
findings:
  critical: 2
  warning: 3
  info: 0
  total: 5
status: issues_found
---

# Phase 24: Code Review Report

**Reviewed:** 2026-09-05T08:25:13Z
**Depth:** standard
**Files Reviewed:** 36 (of the listed 51; the remainder are docs/yaml/json/traceability files given a light consistency pass only, plus integration test files reviewed for reliability only per house rule)
**Status:** issues_found

## Summary

This phase lands Parley pause/resume (Gate nodes, `resume_with`'s total-validation matrix),
Chronicle history/branch inspection, a `ShutdownCoordinator` for graceful-shutdown draining, and
the `paladin-web` HTTP surface (`thread_controller.rs`) plus the `ParleyPortAdapter` facade that
lets `paladin-web` trigger a resume without depending on `paladin-battalion`. The engineering in
`shutdown.rs`, `parley.rs`, `parley_port.rs`, `waypoint.rs`'s `child_on_branch`/`fork_of`
additions, and the storage layer's parameterized-SQL retention/pruning is careful and
well-documented — no SQL injection, no credential leakage, no unwrap/expect/panic newly
introduced in the storage or ports layers, and the project's fail-closed conventions
(`#[non_exhaustive]` catch-alls) are followed consistently.

Two BLOCKER-class problems were found, both load-bearing for this phase's own stated goals:

1. **`GET/POST /v1/threads/{id}/*` has no per-thread authorization** — any principal who clears
   `require_authentication` (including the lowest-privileged configured role) can read, resume,
   or page through the history of *any* thread by id, with zero ownership/role scoping. This
   contradicts the stricter posture `agent_controller.rs`'s sibling routes already enforce
   (`authorize_invoke`, `require_admin`) and directly exposes Battlefield-derived content
   (`parleys[].payload`, response values, full history) that may carry sensitive
   author/business data.
2. **A shutdown-grace abort mid-Muster discards the round's `MusterProgress`**, so resuming a
   thread halted while Muster worker tasks were in flight re-dispatches the aborted worker as an
   *ordinary* vanguard node (losing its `task_key`/`payload` context) and forgets which sibling
   tasks in that same round had already completed — undermining the "Halted is always a
   consistent restart point" contract HITL-04 exists to provide, for exactly the Muster case.

Three WARNING-class findings round out the review: a `RwLock::unwrap()` pair newly added to
`GraphRegistry` (library-code panic-on-poison, against the project's own house rule demonstrated
correctly elsewhere in this same phase's `superstep.rs`), an unguarded validation-drift risk
between `ParleyPortAdapter::shadow_validate`'s hand-maintained re-implementation and the real
`WarEngine::resume_with`, and a fire-and-forget race in the same adapter that can hand a losing
concurrent `resume_with` caller a `202 Accepted` for a submission that silently has no effect.

## Critical Issues

### CR-01: `/v1/threads/*` routes have no per-thread authorization — any authenticated principal can read/resume/list any thread

**File:** `crates/paladin-web/src/thread_controller.rs:475-633` (handlers `get_thread_state`,
`resume_thread`, `get_thread_history`); wiring at `crates/paladin-web/src/thread_controller.rs:646-655`
(`thread_openapi_router`) and `src/bin/paladin-server.rs` (`build_thread_state`/
`thread_state_over_store`, which layers the SAME `AgentAuthConfig` the agent routes use via
`.with_auth(auth)`).

**Issue:** `thread_openapi_router` layers only `crate::agent_auth::require_authentication`
(authenticates — proves *a* valid credential was presented) via `route_layer`. Unlike
`agent_controller.rs`, which extracts `Extension<Principal>` in every sensitive handler and
calls `authorize_invoke(&principal, &entry.allowed_roles)` or `require_admin(&principal)`
(`crates/paladin-web/src/agent_controller.rs:251,259,358,361,423,426,527,534,611,619`), none of
`get_thread_state`, `resume_thread`, or `get_thread_history` ever extract a `Principal` or
perform any authorization check. `ThreadApiState` carries no notion of thread ownership, and
`ThreadId`/`Waypoint` (`crates/paladin-core/src/platform/container/waypoint.rs`) carry no
owner/principal field either — there is no mechanism, anywhere in this call path, that could
restrict *which* threads a given credential may touch.

Concretely: any caller holding a single low-privilege API key (`role: UserRole::User`, per
`agent_auth.rs`'s own `Principal`) can call `GET /v1/threads/{any-id}/state` for a thread they
have no business relationship to, receiving the full `ThreadStateResponse` — including every
outstanding `ParleyRequestDto.payload` (`thread_controller.rs:209`, author-supplied context
explicitly *not* guaranteed secret-free beyond "no credential", per `parley.rs:143-146`'s own
comment) and every `ParleyResponseDto.value`/`responded_by`. The same caller can also submit
`POST /v1/threads/{any-id}/resume` and drive that thread's execution forward, and can page
through its entire `GET /v1/threads/{any-id}/history`. `src/application/services/chronicle.rs`'s
own module doc states "authorisation is enforced at the HTTP adapter's `route_layer`" — but the
route layer that exists only *authenticates*, it never *authorizes* per-thread, so that
documented contract is not actually met by the code that ships in this phase.

This is a broken object-level authorization (IDOR-class) gap: authentication alone is not
authorization, and the sibling `agent_controller.rs` routes in this exact same crate demonstrate
the project already has the pattern (`authorize_invoke`/`require_admin`) to close it.

**Fix:** At minimum, gate `resume_thread` behind `require_admin` (or a new, thread-scoped role
check) the way `agent_controller.rs`'s admin-only routes do, and/or introduce a thread-ownership
concept threaded through from creation so `authorize_invoke`-style scoping can apply to
`get_thread_state`/`get_thread_history` too. Example minimal fix (admin-gated, matching the
existing `require_admin` idiom used elsewhere in this same crate):

```rust
pub async fn resume_thread(
    State(state): State<ThreadApiState>,
    Extension(principal): Extension<crate::agent_auth::Principal>,
    Path(id): Path<String>,
    Json(body): Json<ResumeRequest>,
) -> Result<(StatusCode, JsonValue), ApiError> {
    crate::agent_auth::require_admin(&principal)?;
    // ...unchanged...
}
```
and equivalently for `get_thread_state`/`get_thread_history` (or a coarser `authorize_invoke`
check once a real per-thread role/ownership model exists). At minimum this must be resolved
before shipping to any multi-tenant or multi-role deployment; until it is, document loudly that
every credential configured for `/v1/threads/*` must be treated as admin-equivalent.

### CR-02: A shutdown-grace abort mid-Muster discards `MusterProgress`, breaking resumability of the interrupted round

**File:** `crates/paladin-battalion/src/engine/superstep.rs:2094-2121` (the Halted-on-abort
branch), in conjunction with the dispatch-entry abort bookkeeping at
`crates/paladin-battalion/src/engine/superstep.rs:1698-1701` and the resume-side reconstruction
at `crates/paladin-battalion/src/engine/mod.rs:1245-1287` / `crates/paladin-battalion/src/engine/superstep.rs:1417-1433` (dispatch-entries assembly from `vanguard` + `pending_muster.unfinished_tasks()`).

**Issue:** When the mid-superstep grace deadline fires while a Muster round is dispatching
worker tasks (`dispatch_entries` mixes ordinary vanguard nodes with `muster_dispatch` entries —
`superstep.rs:1417-1433`), every still-outstanding handle is aborted and its dispatch-order
`NodeId` (which for a Muster task is the **worker template's** `NodeId`, not a per-task
identifier) is pushed into `aborted_node_ids` with no distinction between "this was an ordinary
vanguard node" and "this was one Muster task among several sharing this worker" (`superstep.rs:1698-1701`).

The Halted Waypoint built afterward (`superstep.rs:2102-2115`) does two things that together lose
the round's state:

1. It unions every aborted node id — including a Muster worker's `NodeId` — into
   `halted_vanguard` (`superstep.rs:2095-2101`), which becomes the persisted `vanguard`.
2. It passes `muster_progress: None` explicitly (`superstep.rs:2113`), even though
   `muster_node`/`muster_tasks`/`muster_completed_so_far` (populated earlier in this same
   function, e.g. `superstep.rs:1409,1439,1836-1847`) are still in scope and may be non-empty —
   i.e. this round had already-completed sibling tasks and a real, resumable `MusterProgress`
   the individual per-task progress Waypoints (`superstep.rs:1842-1865`) captured moments
   earlier, but which the *newer*, `latest()`-returned Halted Waypoint now shadows.

On resume (`WarEngine::resume`/`resume_with_options`, `mod.rs:1275-1296`), `latest.muster_progress`
is `None`, so `pending_muster` is `None` and `unfinished_tasks()` never runs
(`superstep.rs:1386-1416`); the aborted worker's bare `NodeId` instead re-enters as an *ordinary*
`dispatch_entries` tuple with `muster_ctx: None` (`superstep.rs:1429-1433`). This means:

- The worker re-executes with `NodeContext.muster` = `None` — any `InputMapping` template
  referencing `{muster.payload}`/`{muster.task_key}` (the whole point of a worker template,
  per `node.rs:31-37`) either fails to render or silently resolves to nothing, producing
  incorrect input for a re-run that is supposed to be resuming a specific task.
- Every sibling task's completion state accumulated in `muster_completed_so_far` before the
  abort (already durably written to a progress Waypoint) becomes unreachable through the
  documented resume path, since resume reads `latest.muster_progress` directly rather than
  walking `parent_waypoint_id` back to the last non-`None` `MusterProgress`.
- No test in this phase exercises the abort+Muster intersection (`grep` across
  `superstep.rs`/`mod.rs` for a muster+halt/cancel/abort test finds none), so this regression
  would not be caught by the existing suite.

This directly contradicts `ShutdownCoordinator`'s own module doc and `RunOutcome::Halted`'s
rustdoc promise ("a `Halted` `Waypoint` was persisted, so it is always a consistent restart
point") for the specific, reachable case of a shutdown landing mid-Muster.

**Fix:** When building the Halted Waypoint, preserve the in-flight `MusterProgress` instead of
hard-coding `None`, and only add a *non-muster* aborted node's id to `halted_vanguard` (an
aborted muster task must be recoverable via `unfinished_tasks()`, not via the ordinary vanguard):

```rust
if !aborted_node_ids.is_empty() {
    let mut halted_vanguard = next_vanguard.clone();
    let mut seen: HashSet<NodeId> = halted_vanguard.iter().cloned().collect();
    let aborted_muster_progress = muster_node.as_ref().map(|node| MusterProgress {
        node: node.clone(),
        tasks: muster_tasks.clone(),
        completed: muster_completed_so_far.clone(),
    });
    for node in aborted_node_ids {
        // A muster worker's own progress is recovered via `unfinished_tasks()`
        // below, not by re-adding it to the ordinary vanguard.
        if aborted_muster_progress.is_some() && node == /* the muster node id */ {
            continue;
        }
        if seen.insert(node.clone()) {
            halted_vanguard.push(node);
        }
    }
    let waypoint = build_waypoint(
        // ...
        aborted_muster_progress, // was: None
        // ...
    );
    // ...
}
```
Add a regression test that starts a Muster round, aborts mid-dispatch via a zero/near-zero
`shutdown_grace` and a cancelled token, and asserts the resumed run dispatches exactly the
unfinished `task_key`s with their original payloads (mirroring the existing
`resume_mid_muster_runs_exactly_the_unfinished_tasks` test's assertions, but triggered by
cancellation instead of a fresh `start`/`resume` boundary).

## Warnings

### WR-01: `GraphRegistry` panics on lock poisoning via `.unwrap()` on `RwLock`, against this project's own house rule

**File:** `src/application/services/parley/registry.rs:65,74`

**Issue:** `register` and `resolve` call `self.graphs.write().unwrap()` /
`self.graphs.read().unwrap()`. CLAUDE.md and `.github/instructions/rust.instructions.md` both
state "Avoid `unwrap()`/`expect()` and `panic!` in library code — return `Result`", and this
same phase's `superstep.rs:1567-1582` explicitly documents the correct alternative pattern for an
analogous "this should be unreachable, but must not panic" situation ("library code must not
`.expect()` an invariant it cannot enforce"). If any writer holding the lock panics elsewhere in
the process (e.g. a bug in a future caller building a `WarGraph`), every subsequent
`register`/`resolve` call across the whole process panics on poison, taking down the parley
resume path with it — a poisoned-lock panic in a shared, `Arc`-held registry is far more likely
to cascade than a normal single-owner panic.

**Fix:** Use `.unwrap_or_else(|e| e.into_inner())` (recovering the guard after poisoning, which
is safe here since the map itself cannot be left in a torn state by a panic mid-`insert`/`get`),
or thread a `Result`/log-and-recover path through `register`/`resolve` instead of panicking:

```rust
pub fn register(&self, graph: WarGraph) -> GraphFingerprint {
    let fingerprint = graph.fingerprint();
    self.graphs
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(fingerprint.clone(), Arc::new(graph));
    fingerprint
}
```

### WR-02: `ParleyPortAdapter::shadow_validate` is a hand-maintained re-implementation of `WarEngine::resume_with`'s validation with no automated drift guard

**File:** `src/application/services/parley/adapter.rs:186-374` (`shadow_validate`,
`shadow_validate_response_shape`, `shadow_validate_parley_value_for_kind`,
`shadow_normalize_approval_value`), mirroring `crates/paladin-battalion/src/engine/mod.rs:1350-1509`
and crate-private helpers in `graph.rs`.

**Issue:** The module's own docs are candid that this duplication is a deliberate, accepted
tradeoff (`paladin-battalion`'s validators are `pub(crate)` and unreachable from this crate), and
that a prediction/reality mismatch always resolves in the real engine's favor. That mitigates
*correctness* of the persisted result, but not *observability*: if a future change to
`WarEngine::resume_with`'s validation ordering, a new `OnExpire`/`ParleyKind` variant, or a new
`EngineError` variant is made in `paladin-battalion` without a matching update here, the failure
mode is not a compile error or an obviously-failing test — it is `shadow_validate` predicting
`DelegateSynchronously` when the real call would actually reach the continuation (or vice versa),
silently flipping whether a caller gets an immediate `202` versus a background-spawned one, or
`map_engine_error`'s catch-all (`adapter.rs:411-413`) silently downgrading a new, more specific
`EngineError` variant to the generic `ParleyError::Rejected`, erasing an HTTP status distinction
`thread_controller.rs`'s `map_parley_error` depends on. There is no property/fuzz test in this
module (or anywhere in the reviewed scope) that exercises both `shadow_validate` and a real
`WarEngine::resume_with` call against the *same* randomized parley/response fixture and asserts
their outcomes agree.

**Fix:** Add a property-style test (or at minimum a fixture table) that runs both
`shadow_validate` and a real `WarEngine::resume_with` call over the same set of
parley/response/expiry combinations and asserts the `ShadowOutcome` variant matches whether the
real call reached the continuation — so a future validation change in `paladin-battalion` that
silently desyncs this adapter fails a test in *this* crate, not just in production.

### WR-03: A losing concurrent `resume_with` race can hand the caller a `202 Accepted` for a submission that silently has no effect

**File:** `src/application/services/parley/adapter.rs:141-164` (`ShadowOutcome::Complete` arm)

**Issue:** When `shadow_validate` predicts `Complete`, the adapter registers with the
`ShutdownCoordinator`, spawns the real `engine.resume_with(...)` call in the background, and
immediately returns `Ok(ResumeAccepted::new(...))` to the caller — before the spawned task has
even started running, let alone validated anything against the real, current Waypoint state. The
spawned task's own result is discarded (`let _ = engine.resume_with(...).await;`,
`adapter.rs:152-154`), with the comment acknowledging "no synchronous caller remains to report a
failure to." If two concurrent `resume_with` calls for the same thread both observe (via their
own `latest()` snapshot) a state where the submission looks complete, both requests receive an
immediate `202`, but only one of the two spawned background calls can actually succeed against
`WarEngine::resume_with`'s own total-validation (the loser fails, e.g. with
`ParleyAlreadyAnswered`, entirely silently). The losing caller has no way to distinguish "still
processing" from "silently dropped" — polling `GET /threads/{id}/state` shows the thread exactly
as it was before their (accepted, but ineffective) call, indefinitely.

**Fix:** At minimum, log the discarded error at `warn`/`error` level (currently nothing is
logged) so an operator can detect the race from server logs, and consider surfacing it via
tracing (`TraceEvent`) so `thread_controller.rs`'s poll path could eventually expose "last resume
attempt failed" on the Waypoint rather than leaving a silently-dropped submission
indistinguishable from one still in flight.

---

_Reviewed: 2026-09-05T08:25:13Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
