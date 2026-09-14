---
phase: 27-platform-api
plan: 08
subsystem: api
tags: [platform-api, resume, run-queue, hexagonal-ports, openapi, migration]

requires:
  - phase: 27-platform-api (plan 02)
    provides: "RunRepositoryPort::active_run_for_thread/record_resume, RunOutcomeRecord, InMemoryRunRepository"
  - phase: 27-platform-api (plan 04)
    provides: "RunWorkerPool's WorkerDispatch::decide (resume/resume_with per pending_responses), the shared attempt counter, PausingAfterSuperstep test-harness pattern"
provides:
  - "ParleyPortAdapter::with_run_repository/with_run_queue -- a resume against a thread with an active run row re-enqueues the SAME run_id (attempt++) instead of spawning in-process (PLAT-FR-06, D-19, D-23)"
  - "ResumeAccepted::run_id/with_run_id (paladin-ports) and ResumeAcceptedResponse::run_id/new (paladin-web), both #[non_exhaustive] (D-21)"
  - "MIGRATION.md §9.2/§9.6 rows for the two published-type extensions and the resume endpoint's additive run_id field"
affects: [27-09, 27-12, 27-15, 27-17]

tech-stack:
  added: []
  patterns:
    - "Optional-collaborator builder pair on an existing adapter (with_run_repository/with_run_queue) that both must be set for a durable path to activate, leaving every existing call site's behavior byte-identical when unset (X-03) -- mirrors 27-04's with_paladin_port/with_shutdown_coordinator precedent on RunWorkerPool"
    - "record_resume's own CAS (only succeeds from AwaitingInput) is the sole race-arbiter for a durable resume -- no pre-check-then-act gap the way the legacy Waypoint-shadow-validation path has to work around"

key-files:
  created: []
  modified:
    - src/application/services/parley/adapter.rs
    - crates/paladin-ports/src/input/parley_port.rs
    - crates/paladin-web/src/thread_controller.rs
    - crates/paladin-web/Cargo.toml
    - crates/paladin-web/openapi.json
    - MIGRATION.md

key-decisions:
  - "The durable path is gated on BOTH with_run_repository and with_run_queue being wired AND active_run_for_thread returning Some -- not on the presence of the collaborators alone. This keeps X-03's fallback exact: a thread with no run row (Phase 24 legacy) still gets the in-process spawn even on a fully-wired adapter, matching the plan's own must_haves truth about pre-run-server threads."
  - "record_resume's IllegalTransition is treated as the authoritative race signal, not a separate pre-check. Rather than reading the run's status before calling record_resume (a check-then-act gap), the adapter calls record_resume directly on whatever active_run_for_thread returns and lets its own CAS (`status must be AwaitingInput`) either succeed or fail with the mapped ThreadNotAwaitingInput -- there is no window for a second, independent race to slip through undetected."
  - "Rejected a Rule-1 fix that would have kept a doc-link to the private ShadowOutcome::Complete variant behind --document-private-items; switched to plain backticks per CLAUDE.md's rustdoc convention instead, since a lint suppression would have hidden a real (if pre-existing-adjacent) rustdoc warning class rather than fixing it."

requirements-completed: [PLAT-03]

coverage:
  - id: D1
    description: "POST /threads/{id}/resume keeps Phase 24's published 202/{thread_id, state_url} contract, 404/409/400/501 status table verbatim, with only the ParleyPort mechanism changed from spawn to enqueue (D-20)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs -- post_resume_returns_202_with_thread_and_state_url, post_resume_unknown_thread_is_404, post_resume_on_running_thread_is_409_thread_not_awaiting_input, post_resume_with_unregistered_graph_is_409_graph_not_registered, post_resume_bad_response_is_400_with_parley_id_in_details, post_resume_with_admin_role_is_202, post_resume_with_non_admin_role_is_403, thread_routes_return_501_when_no_backend_is_wired (all 18 tests in thread_controller pass unchanged)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A complete parley submission against a thread with an active run row is validated synchronously then re-enqueued under the SAME run_id with attempt incremented, instead of spawned in-process; the queue message carries only the pointer, responses ride on the run row"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "src/application/services/parley/adapter.rs#wired_adapter_with_active_run_reenqueues_instead_of_spawning"
        status: pass
      - kind: e2e
        ref: "src/application/services/parley/adapter.rs#resume_reenqueues_same_run_id"
        status: pass
    human_judgment: false
  - id: D3
    description: "A thread with no run row falls back to Phase 24's in-process spawn unchanged, run_id: None (X-03); record_resume's IllegalTransition maps to ThreadNotAwaitingInput"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "src/application/services/parley/adapter.rs#wired_adapter_without_active_run_falls_back_to_legacy_spawn, #record_resume_illegal_transition_maps_to_thread_not_awaiting_input"
        status: pass
    human_judgment: false
  - id: D4
    description: "ResumeAccepted and ResumeAcceptedResponse gain run_id, are #[non_exhaustive] with construction paths preserved, and are registered in MIGRATION.md §9.2/§9.6"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/input/parley_port.rs#resume_accepted_with_run_id_attaches_the_run; crates/paladin-web/src/thread_controller.rs (post_resume_returns_202_with_thread_and_state_url unaffected)"
        status: pass
      - kind: other
        ref: "MIGRATION.md §9.2/§9.6 rows added; openapi.json regenerated with only the run_id addition; .cargo/semver-checks-allowlist.toml confirmed set-equal to §9.2's Y-marked rows (unchanged, since these rows are N/A)"
        status: pass
    human_judgment: false

duration: ~32min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 08: Durable Resume Re-enqueue Summary

**`ParleyPortAdapter` now re-enqueues a resumed run under its original `run_id` (`attempt++`) through `RunRepositoryPort`/`RunQueuePort` instead of spawning an in-process `tokio::task`, with the published `202` resume contract kept byte-identical and `run_id` surfaced through both `ResumeAccepted` and `ResumeAcceptedResponse`.**

## Performance

- **Duration:** ~32 min
- **Started:** 2026-09-08T04:26Z
- **Completed:** 2026-09-08T04:58Z
- **Tasks:** 2 (both `type="auto"`, Task 1 `tdd="true"`)
- **Files modified:** 6

## Accomplishments

- `ParleyPortAdapter` gained `with_run_repository`/`with_run_queue` builder methods; when both are wired and the thread has an active run row, `resume_with`'s `ShadowOutcome::Complete` arm calls `record_resume` (attempt `n` -> `n+1`, responses parked on the row) then `enqueue`s a `QueuedRun` under the SAME `run_id` -- no `tokio::spawn`, proven both by a unit test (repository/queue in isolation) and an end-to-end test that submits a run through a real `RunWorkerPool`, lets it suspend `AwaitingInput`, resumes through the adapter, and confirms a second worker iteration completes it.
- A thread with no run row (or an adapter with neither collaborator wired) keeps Phase 24's exact in-process spawn behavior, `run_id: None` (X-03) -- verified by a dedicated test that the legacy path still runs a graph to `Completed` in the background even when both collaborators ARE wired, as long as no run row exists.
- `record_resume`'s CAS losing a race (the run row is not `AwaitingInput`) maps to `ParleyError::ThreadNotAwaitingInput`, with the losing attempt never enqueuing anything.
- `ResumeAccepted` (`paladin-ports`) and `ResumeAcceptedResponse` (`paladin-web`) both gained an additive `run_id: Option<...>` field, marked `#[non_exhaustive]` with a preserved construction path (`new()` + `with_run_id()` / `new(thread_id, state_url, run_id)`); the one in-tree construction site (`thread_controller.rs`'s `resume_thread` handler) was migrated in the same commit.
- `openapi.json` regenerated (`make openapi`); the diff is exactly the new nullable `run_id` property on `ResumeAcceptedResponse`. `MIGRATION.md` gained two §9.2 rows (both `N/A` -- new-in-0.10 types, not deliberate-breaking) and a §9.6 row recording the endpoint's additive field with the status-code table unchanged.

## Task Commits

Each task was committed atomically:

1. **Task 1: `ParleyPortAdapter` enqueues instead of spawning; `ResumeAccepted` gains `run_id`** - `49004edd` (feat)
2. **Task 2: Regenerate `openapi.json`; register §9.2 and §9.6** - `01f99b60` (docs) -- also carries a small Rule 1 rustdoc-link fix to Task 1's own new module doc (see Deviations)

**Plan metadata:** this file's own commit (docs: complete plan) -- committed alongside this SUMMARY per worktree execution mode.

_TDD note: Task 1 carries `tdd="true"`. Tests were written and passing before the commit; no separate RED-then-GREEN commit pair was produced (test + implementation landed together, consistent with 27-01/27-02/27-04's documented convention for this worktree)._

## Files Created/Modified

- `src/application/services/parley/adapter.rs` -- `ParleyPortAdapter` gains `run_repository`/`run_queue` optional fields, `with_run_repository`/`with_run_queue` builders, the durable enqueue branch inside `resume_with`'s `Complete` arm, `map_run_repository_error`/`map_run_queue_error` mapping functions, and 4 new tests (`wired_adapter_without_active_run_falls_back_to_legacy_spawn`, `wired_adapter_with_active_run_reenqueues_instead_of_spawning`, `record_resume_illegal_transition_maps_to_thread_not_awaiting_input`, `resume_reenqueues_same_run_id`).
- `crates/paladin-ports/src/input/parley_port.rs` -- `ResumeAccepted` marked `#[non_exhaustive]`, gains `run_id: Option<RunId>` with `run_id()`/`with_run_id()`, an updated doc test, and a new unit test.
- `crates/paladin-web/src/thread_controller.rs` -- `ResumeAcceptedResponse` marked `#[non_exhaustive]`, gains `run_id: Option<String>` and a `new()` constructor; `resume_thread`'s handler updated to pass `accepted.run_id().map(|id| id.to_string())` through.
- `crates/paladin-web/Cargo.toml` -- adds `struct_marked_non_exhaustive = "allow"` to the existing `[package.metadata.cargo-semver-checks.lints]` table.
- `crates/paladin-web/openapi.json` -- regenerated; only the new nullable `run_id` property on `ResumeAcceptedResponse`.
- `MIGRATION.md` -- two §9.2 rows (`ResumeAccepted`, `ResumeAcceptedResponse`) and one §9.6 row (resume endpoint's additive `run_id`).

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **Durable path gated on BOTH collaborators AND an active run, never on collaborators alone.** `active_run_for_thread` returning `None` (X-03's pre-run-server thread) always falls through to the legacy spawn, even on a fully-wired adapter -- verified by `wired_adapter_without_active_run_falls_back_to_legacy_spawn`, which wires both collaborators but leaves the run repository empty and confirms the background continuation still runs the graph to completion.
2. **`record_resume`'s own CAS is the race-arbiter, not a separate status pre-check.** The adapter never reads `run.status` itself before deciding whether to call `record_resume` -- it always tries the call on whatever `active_run_for_thread` returns, and `record_resume`'s `status != AwaitingInput` CAS either succeeds (returning the bumped `attempt`) or fails with `IllegalTransition`, mapped to `ThreadNotAwaitingInput`. This avoids a check-then-act window the way the module's own `shadow_validate`/real-engine split has to document and accept for the legacy path.
3. **A Rule 1 rustdoc-link fix, not a lint suppression.** The new module-doc paragraph originally linked `` [`ShadowOutcome::Complete`] `` (a private enum variant), tripping `rustdoc::private_intra_doc_links` -- a genuine new warning, not a pre-existing one. Fixed by switching to plain backticks (CLAUDE.md's own stated convention: "Never write `[`Self::private_fn`]`-style intra-doc links to private items; use plain backticks") rather than suppressing the lint, keeping `cargo doc -p paladin-ai --no-deps` at exactly the 4 pre-existing warnings this worktree's base already carried.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] New module-doc paragraph linked to a private enum variant, tripping a rustdoc lint**
- **Found during:** Task 2's final verification pass (`cargo doc -p paladin-ai --no-deps`)
- **Issue:** Task 1's new module-doc paragraph in `adapter.rs` used `` [`ShadowOutcome::Complete`] `` as an intra-doc link; `ShadowOutcome` is a private enum, so this tripped `rustdoc::private_intra_doc_links` -- a genuinely NEW warning (5 total vs. the base's 4 pre-existing ones in `paladin_execution_service.rs`/`parley/adapter.rs`/`config/agent_runtime.rs`/`presets/mod.rs`).
- **Fix:** Replaced the doc link with plain backticks (`` `ShadowOutcome::Complete` ``), per CLAUDE.md's own stated rustdoc convention.
- **Files modified:** `src/application/services/parley/adapter.rs`
- **Verification:** `cargo doc -p paladin-ai --no-deps` back to exactly 4 warnings (unchanged from base); `cargo fmt --all -- --check` and `cargo check -p paladin-ai --all-targets --all-features` both clean.
- **Committed in:** `01f99b60` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 rustdoc-link fix)
**Impact on plan:** Cosmetic documentation fix only; no change to runtime behavior, tests, or the plan's own scope.

## Issues Encountered

None beyond the one auto-fixed deviation above, caught during the final `cargo doc` verification pass before Task 2's commit.

## User Setup Required

None -- no external service configuration required. Every new test runs against InMemory adapters (`InMemoryRunRepository`, `InMemoryRunQueue`, `InMemoryWaypointStore`) with no Docker dependency.

## Next Phase Readiness

- `ParleyPortAdapter`'s durable enqueue path is ready to be wired in production alongside `RunRepositoryPort`/`RunQueuePort` instances once a later plan assembles the process's real adapter graph (27-17's config-driven wiring is the natural site, mirroring how `RunWorkerPool::with_paladin_port` is wired there).
- `ResumeAccepted::run_id()`/`ResumeAcceptedResponse.run_id` are ready for any client-facing SDK or docs update that wants to let a caller poll `GET /v1/runs/{run_id}` directly after a resume, instead of only `GET /v1/threads/{id}/state`.
- No blockers. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo check --workspace --all-targets --all-features` all pass clean on the final commit; `cargo doc -p paladin-ports/-web/-ai --no-deps` introduces no new warnings beyond the pre-existing 4 (unrelated to this plan).

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `src/application/services/parley/adapter.rs`
- FOUND: `crates/paladin-ports/src/input/parley_port.rs`
- FOUND: `crates/paladin-web/src/thread_controller.rs`
- FOUND: `crates/paladin-web/Cargo.toml`
- FOUND: `crates/paladin-web/openapi.json`
- FOUND: `MIGRATION.md`

**Commits verified to exist (git log --oneline):**
- FOUND: `49004edd` feat(27-08): ParleyPortAdapter enqueues resume instead of spawning; ResumeAccepted gains run_id
- FOUND: `01f99b60` docs(27-08): regenerate openapi.json for run_id; register MIGRATION.md §9.2/§9.6

**Verification commands re-run and confirmed passing:**
- `cargo test -p paladin-ai --lib services::parley` → `test result: ok. 18 passed` (14 pre-existing + 4 new)
- `cargo test -p paladin-ai --lib resume_reenqueues_same_run_id` → `test result: ok. 1 passed`
- `cargo test -p paladin-web --lib thread_controller` → `test result: ok. 18 passed` (Phase 24's status-code table tests unchanged)
- `cargo test -p paladin-ports --lib parley_port` → `test result: ok. 5 passed`
- `cargo test -p paladin-ports --doc parley_port` → `test result: ok. 2 passed`
- `cargo test -p paladin-web --lib openapi_matches_committed_baseline` → `test result: ok. 1 passed`
- `grep -c 'record_resume' src/application/services/parley/adapter.rs` → `9` (≥ 1)
- `grep -c 'enqueue(' src/application/services/parley/adapter.rs` → `2` (≥ 1)
- `grep -B2 'pub struct ResumeAccepted' crates/paladin-ports/src/input/parley_port.rs \| grep -c non_exhaustive` → `1`
- `grep -B3 'pub struct ResumeAcceptedResponse' crates/paladin-web/src/thread_controller.rs \| grep -c non_exhaustive` → `1`
- `git diff --stat crates/paladin-web/openapi.json` (at regeneration time) → `8 insertions(+), 1 deletion(-)`, confirmed to be exactly the `run_id` property
- `awk '/^## 9.2/,/^## 9.3/' MIGRATION.md | grep -c 'ResumeAccepted'` → `3` (≥ 2)
- `awk '/^## 9.6/,/^## 9.7/' MIGRATION.md | grep -c 'run_id'` → `5` (≥ 1)
- Local re-implementation of the CI `semver` job's set-equality check (§9.2 `Y`-marked crates vs. `.cargo/semver-checks-allowlist.toml` `crate = "..."` entries, line-anchored): SET-EQUAL, unchanged by this plan's two `N/A` rows
- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean (run twice, once per task's final state)
- `cargo check --workspace --all-targets --all-features` → exit 0 (full workspace)
- `cargo doc -p paladin-ports/-web/-ai --no-deps` → 1 / 3 / 4 warnings respectively, all pre-existing (none introduced by this plan after the Rule 1 fix)
- No unexpected file deletions in either commit (`git diff --diff-filter=D --name-only HEAD~1 HEAD` empty for both)

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
