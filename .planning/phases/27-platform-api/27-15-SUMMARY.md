---
phase: 27-platform-api
plan: 15
subsystem: api
tags: [pagination, cancellation, fork, thread-lifecycle, rate-limiting, ssrf, openapi, hexagonal-ports]

# Dependency graph
requires:
  - phase: 27-platform-api (plan 07)
    provides: "CancellationProbe seam, RunSubmissionPort::cancel + CancelOutcome, LocalRunTokens"
  - phase: 27-platform-api (plan 08)
    provides: "ThreadApiState builder pattern, run_id surfaced on resume"
  - phase: 27-platform-api (plan 12)
    provides: "RunApiState route-merge pattern, AssistantResolver::resolve, allowed_roles on ResolvedAssistant"
  - phase: 27-platform-api (plan 13)
    provides: "webhook::SsrfGuard::check_url (shared, async), WebhookDeliveryRepositoryPort::list_for_run, RunSubmissionError::WebhookRejected"
  - phase: 27-platform-api (plan 14)
    provides: "schedule/admin.rs's own (now-collapsed) SsrfGuard, RunApiState.schedules, five /v1/schedules* routes"
provides:
  - "crates/paladin-web/src/pagination.rs: PageQuery, resolve_limit (1..=100, 400 on 0/>100), encode_cursor/decode_cursor (base64url no-padding over JSON, 400 invalid_cursor never 500) -- the ONE shared pagination shape run_controller.rs/thread_controller.rs/assistant_controller.rs/schedule_controller.rs all call"
  - "GET /v1/runs (filterable by thread_id/assistant_id/status, paginated), POST /v1/runs/{id}/cancel (D-16, D-46 authorize via allowed_roles resolved inside RunSubmissionService), GET /v1/runs/{id}/webhook-deliveries (paginated, newest-first)"
  - "GET /v1/threads, GET /v1/threads/{id}, POST /v1/threads/{id}/fork, DELETE /v1/threads/{id} on ThreadApiState (now #[non_exhaustive] with runs/run_submission collaborators, D-45)"
  - "RunSubmissionPort::fork(ForkRun) + RunSubmissionPort::cancel gains a requested_by parameter (D-46 invocation-shaped authorization, resolved via AssistantResolver inside RunSubmissionService -- paladin-web never resolves allowed_roles itself, ADR-0031)"
  - "WorkerDispatch::Fork { from, edit } -- decide() prioritizes an unstarted fork_from over Start/Resume/ResumeWith; run_once drives WarEngine::fork; a corrupt persisted from_waypoint_id is recorded as an engine failure, never a panic"
  - "http_surface_tests.rs: ten_concurrent_submits_one_accepted (real run_router + SqliteRunRepository, PRD acceptance 3) and fork_run_completes_from_waypoint (full submit -> complete -> fork -> complete round trip, fork_of == Some(wp2) proven)"
  - "The 429 rate limiter (pre-existing tower-governor layer) proven on /v1/runs* for the first time"
  - "schedule/admin.rs's 27-14-era duplicate SsrfGuard collapsed onto 27-13's shared webhook::SsrfGuard (one guard, two call sites: write-time here, send-time in the webhook client)"
affects: [27-16, 27-17, 27-18]

# Tech tracking
tech-stack:
  added:
    - "base64 (workspace dependency already used elsewhere) promoted to a direct paladin-web dependency for pagination.rs's base64url cursor encoding -- no new package, Cargo.lock gains a dependency-list line only"
  patterns:
    - "One pagination module (pagination.rs), four call sites: every list handler across run/thread/assistant/schedule controllers deserializes limit/cursor into its own query struct but resolves/encodes/decodes through the SAME three shared functions, so PLAT-FR-16's pagination claim is a single sentence, not four independently-drifting ones"
    - "D-46 authorization lives entirely inside RunSubmissionService, never in paladin-web: submit/cancel/fork all resolve the target assistant's allowed_roles via AssistantResolver and reject with Forbidden BEFORE any repository write, because paladin-web has no visibility into allowed_roles at all (ADR-0031) -- the HTTP layer's only job is extracting (principal.id, principal.role) into requested_by and mapping Forbidden to 403"
    - "cancel's requested_by is Option, not required: None skips the authorization check entirely (an internal/same-process caller with no principal to authorize against), so every pre-existing test/caller passing no principal keeps working unchanged"
    - "WorkerDispatch::Fork takes priority over Start/Resume/ResumeWith in decide(): a run's fork_from is checked FIRST, and only once the latest Waypoint's fork_of already matches it does dispatch fall through to normal resume -- this is what makes redelivery after the fork Waypoint exists resume normally with no special-casing"

key-files:
  created:
    - crates/paladin-web/src/pagination.rs
    - src/application/services/run/http_surface_tests.rs
  modified:
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/src/thread_controller.rs
    - crates/paladin-web/src/assistant_controller.rs
    - crates/paladin-web/src/schedule_controller.rs
    - crates/paladin-web/src/lib.rs
    - crates/paladin-web/Cargo.toml
    - crates/paladin-web/openapi.json
    - crates/paladin-ports/src/input/run_submission_port.rs
    - src/application/services/run/submission.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/mod.rs
    - src/application/services/run/cancel_tests.rs
    - src/application/services/run/schedule/admin.rs
    - src/application/services/run/schedule/mod.rs
    - src/application/services/run/schedule/service.rs
    - src/application/services/run/schedule/tests.rs
    - MIGRATION.md

key-decisions:
  - "cancel's authorization check does one extra repository read + resolve when requested_by is Some, not on the cheap default path. Rather than adding a new required WaypointPort/AssistantResolver dependency the HTTP layer would need to pre-resolve, RunSubmissionService::cancel calls self.repository.get(run_id) then self.resolver.resolve(...) itself, exactly mirroring submit's own resolve-then-authorize order -- the ONE place that can answer 'what are this run's allowed_roles' is the facade that already holds the resolver."
  - "fork() requires a WaypointPort (with_waypoints builder, optional, defaulting to None -> NotWired) rather than skipping the from_waypoint_id existence check when unwired. A silently-skipped validation would let a caller fork from a waypoint that never existed, and this repo's convention (D-24 precedent) is a genuine 501 over an unproven success."
  - "ThreadSummaryDto (GET /threads, list) omits latest_waypoint_id; ThreadResponse (GET /threads/{id}, one thread) includes it. WaypointPort::list_threads returns ThreadSummary, which carries no waypoint_id field -- adding one would touch every backend adapter in paladin-storage, outside this plan's declared file scope. GET /threads/{id} calls WaypointPort::latest instead, which returns a full Waypoint and DOES carry waypoint_id, so the asymmetry is a real capability difference in the underlying port, not an oversight. Documented in ThreadSummaryDto's own rustdoc."
  - "The duplicate SsrfGuard in schedule/admin.rs is collapsed onto webhook::SsrfGuard per the wave_context's explicit instruction -- schedule/admin.rs, schedule/mod.rs and schedule/service.rs are outside this plan's declared files_modified, treated as the plan's own cross-cutting 'one guard everywhere' intent per the orchestrator's own carve-out text, not a spontaneous scope expansion."

requirements-completed: [PLAT-06, PLAT-02]

coverage:
  - id: D1
    description: "pagination.rs's resolve_limit/encode_cursor/decode_cursor is the one shared implementation every list handler (runs, threads, assistants, schedules) calls; limit=0/101 -> 400, limit=100 succeeds, malformed cursor -> 400 invalid_cursor never 500"
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/pagination.rs -- 10 tests: resolve_limit_{none,zero,101,100,one}, cursor_round_trips, cursor_is_base64url_no_padding, decode_cursor_{bad_base64,bad_shape,never_leaks_input}"
        status: pass
    human_judgment: false
  - id: D2
    description: "GET /v1/runs is filterable/paginated with an empty-set 200 shape; POST /v1/runs/{id}/cancel is idempotent 202 with D-46 authorization and 409 on a terminal run; GET /v1/runs/{id}/webhook-deliveries is paginated and never leaks a secret"
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs -- list_runs_{returns_501_when_unwired,empty_is_200_with_empty_items_and_null_cursor,rejects_limit_zero_and_101,rejects_malformed_cursor_as_400_never_500,rejects_unknown_status}, cancel_run_{accepted_returns_202,already_terminal_returns_409,returns_501_when_unwired}, list_webhook_deliveries_{returns_501_when_unwired,empty_is_200_with_empty_items,no_secret_in_body}"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --lib run_controller -- 30 passed"
        status: pass
    human_judgment: false
  - id: D3
    description: "D-46 two-tier scopes hold at the router level: submit/cancel are invocation-shaped (Forbidden -> 403), the admin-gated assistant route merged onto the SAME run_router is still 403 for a non-admin, an unauthenticated request is 401, and the pre-existing rate limiter answers 429 on /v1/runs for the first time"
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs -- run_controller_auth::{unauthenticated_request_is_401, submit_forbidden_role_is_403, cancel_forbidden_role_is_403, admin_only_assistant_route_is_403_for_non_admin, rate_limited_request_is_429}"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --lib run_controller_auth -- 5 passed"
        status: pass
    human_judgment: false
  - id: D4
    description: "GET /threads, GET /threads/{id}, POST /threads/{id}/fork and DELETE /threads/{id} exist on ThreadApiState (now #[non_exhaustive] with runs/run_submission), covering the 501/200/400/403/404/409 table"
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs -- list_threads_*, get_thread_*, fork_thread_*, delete_thread_* (15 new tests)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --lib thread_controller -- 33 passed (Phase 24's 18 plus 15 new)"
        status: pass
    human_judgment: false
  - id: D5
    description: "WorkerDispatch::Fork drives WarEngine::fork exactly once when a run's fork_from Waypoint has not yet been produced, and falls through to normal resume once it has; a run forked from its second superstep's Waypoint completes end to end and its history records fork_of == Some(wp2)"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker.rs -- decide_returns_fork_when_no_waypoint_exists_yet_and_fork_from_is_set, decide_returns_fork_when_latest_waypoint_fork_of_does_not_match, decide_falls_through_to_resume_once_fork_of_already_matches, parse_fork_waypoint_id_{round_trips_a_valid_id,rejects_garbage}, fork_edit_to_state_delta_{merges_object_fields,is_empty_for_none}"
        status: pass
      - kind: e2e
        ref: "src/application/services/run/http_surface_tests.rs#fork_run_completes_from_waypoint"
        status: pass
    human_judgment: false
  - id: D6
    description: "Ten concurrent POST /v1/runs for one thread through the real run_router over SqliteRunRepository produce exactly one 202 and nine 409 thread_busy (PRD acceptance 3, D-52), proven under real concurrent writers, not a single-threaded check-then-insert"
    requirement: "PLAT-02"
    verification:
      - kind: e2e
        ref: "src/application/services/run/http_surface_tests.rs#ten_concurrent_submits_one_accepted (multi_thread flavor, 30s timeout guard, run 4 times consecutively -- stable every time)"
        status: pass
    human_judgment: false
  - id: D7
    description: "schedule/admin.rs's independent, 27-14-era SsrfGuard duplicate is deleted and both write-time (schedule create/patch) and send-time (webhook client) checks route through the ONE shared webhook::SsrfGuard, with no behavior change to either call site"
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --lib services::run::schedule -- 21 passed (create_rejects_webhook_url_via_ssrf_guard, patch_webhook_rejected_by_guard unaffected)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib services::run::webhook -- 21 passed (webhook::ssrf::tests::webhook_ssrf_guard remains the single exhaustive classification-table proof)"
        status: pass
    human_judgment: false

duration: ~55min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 15: Remaining Run Routes, Thread Lifecycle Routes, and Cross-Cutting Pagination/Scopes/429 Summary

**One shared `pagination.rs` (limit/cursor, base64url opaque cursors) used by every `/v1` list endpoint; `GET /v1/runs`, `POST /v1/runs/{id}/cancel` and `GET /v1/runs/{id}/webhook-deliveries` complete the run surface; `GET/DELETE /v1/threads[/{id}]` and `POST /v1/threads/{id}/fork` (backed by a new `WorkerDispatch::Fork` driving `WarEngine::fork`) complete the thread surface; D-46's two-tier authorization and the existing rate limiter are proven at the router level; and the ten-concurrent-submits race is proven end to end over real SQLite.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-09-08T08:29:06Z (worktree base `9f288751`)
- **Completed:** 2026-09-08T09:23:58Z
- **Tasks:** 2 (both `type="auto" tdd="true"`)
- **Files modified:** 20 (2 created, 18 modified)

## Accomplishments

- `crates/paladin-web/src/pagination.rs` — `PageQuery`, `resolve_limit` (`None` → 20, `Some(0)`/`>100` → `400 bad_request` "limit must be between 1 and 100", `Some(100)` → 100), `encode_cursor`/`decode_cursor` (base64url no-padding over compact JSON; malformed input → `400 invalid_cursor`, never the raw input or internal encoding in the message, never a `500`) — the ONE implementation `run_controller.rs`/`thread_controller.rs`/`assistant_controller.rs`/`schedule_controller.rs` all call; the latter two switched their locally duplicated `parse_limit` onto it (the only behavior change is the newly typed `limit == 0` rejection, previously silently defaulted).
- `GET /v1/runs` — filterable by `thread_id`/`assistant_id`/`status`, ordered `(submitted_at DESC, run_id DESC)`, `{ items: [], next_cursor: null }` on empty, the cursor-walk-is-not-a-snapshot caveat stated in the OpenAPI description. `RunResponse` gained a redacted `webhook` field and a `pending_responses` count; `SubmitRunRequest` gained the `webhook` field it never actually exposed before this plan (the SSRF guard existed since 27-13 but no HTTP field ever reached it).
- `POST /v1/runs/{id}/cancel` — `202` idempotent on a non-terminal run, `409` on a terminal one, `403` when the run's own assistant `allowed_roles` exclude the principal (D-46: `RunSubmissionPort::cancel` gained a `requested_by: Option<(String, UserRole)>` parameter; `RunSubmissionService::cancel` resolves the run then its assistant's `allowed_roles` and authorizes BEFORE the durable cancel flag is ever written, only when a principal is supplied).
- `GET /v1/runs/{id}/webhook-deliveries` — paginated, newest-first, no secret or payload in the wire shape.
- `RunSubmissionPort::fork(ForkRun)` — validates the fork point exists (`WaypointPort::get`, `with_waypoints` builder, `NotWired` when unwired), copies the assistant reference from the thread's most recent run (`RunQuery{thread_id, limit: 1}`, `UnknownThread` if none), authorizes via the same `allowed_roles` mechanism, enforces the SAME busy-thread invariant `submit` does, then inserts+enqueues a run carrying `fork_from`.
- `GET /v1/threads`, `GET /v1/threads/{id}`, `POST /v1/threads/{id}/fork`, `DELETE /v1/threads/{id}` — `ThreadApiState` gained `runs`/`run_submission` fields (`with_runs`/`with_run_submission`) and is now `#[non_exhaustive]` (X-10.3, construction stays builder-only); `DELETE` is `require_admin` and `409 thread_busy` while a run is active; `fork` is invocation-shaped.
- `WorkerDispatch::Fork { from, edit }` — `decide()` checks `run.fork_from` FIRST: if the latest Waypoint's `fork_of` does not already match it, dispatch is `Fork` (drives `WarEngine::fork`); once it matches, dispatch falls through to normal `Start`/`Resume`/`ResumeWith` — redelivery after the fork Waypoint exists just resumes. A corrupt persisted `from_waypoint_id` (this service's own prior write) is recorded as an engine failure via the existing `record_engine_failure` path, never a panic.
- `http_surface_tests.rs` — `ten_concurrent_submits_one_accepted` (ten concurrent `POST /v1/runs` for one thread through the real `run_router` over `SqliteRunRepository` on a temp file: exactly one `202`, nine `409`, run 4× consecutively with no flakiness) and `fork_run_completes_from_waypoint` (submit a 3-node chain to `Completed`, fork from its superstep-2 Waypoint, drive the fork to `Completed`, confirm a Waypoint records `fork_of == Some(wp2)`).
- The pre-existing `tower-governor` rate limiter is proven on `/v1/runs*` for the first time (`run_controller_auth::rate_limited_request_is_429`), and D-46's two-tier scopes are proven at the ROUTER level (not just per-file): a non-admin hitting the admin-gated `POST /v1/assistants` route merged onto the same `run_router` still gets `403`.
- `schedule/admin.rs`'s independent, 27-14-era `SsrfGuard` duplicate is deleted; `ScheduleService`'s write-time webhook check now routes through 27-13's shared `webhook::SsrfGuard` — one guard, two call sites (write-time here, send-time in the webhook client), no behavior change to either.

## Task Commits

Each task was committed atomically:

1. **Task 1: `pagination.rs`, remaining run routes, scopes, and the 429 proof** — `227ffecc` (feat)
2. **Task 2: Thread routes (list, get, fork, delete), `ThreadApiState` registration, `RunSubmissionPort::fork`, worker fork dispatch, `http_surface_tests.rs`, MIGRATION registration, SsrfGuard collapse** — `02faf98f` (feat)

**Plan metadata:** this file's own commit (docs: complete plan) — committed alongside this SUMMARY per worktree execution mode.

_TDD note: both tasks carry `tdd="true"`. Tests were written and passing before each commit; no separate RED-then-GREEN commit pair was produced (test + implementation landed together per task, consistent with every prior `27-platform-api` plan's documented convention for this worktree)._

## Files Created/Modified

- `crates/paladin-web/src/pagination.rs` — `PageQuery`, `resolve_limit`, `encode_cursor`/`decode_cursor`, `DEFAULT_PAGE_LIMIT`/`MAX_PAGE_LIMIT`, 10 tests.
- `crates/paladin-web/src/run_controller.rs` — `list_runs`, `cancel_run`, `list_webhook_deliveries` handlers; `RunListQuery`/`RunListResponse`/`CancelRunResponse`/`WebhookDeliveryDto`/`WebhookDeliveryListResponse`/`RunWebhookRequestDto`/`RunWebhookDto`; `RunApiState.webhook_deliveries` + `with_webhook_deliveries`; `map_submission_error` made `pub(crate)` with `WebhookRejected`/`UnknownThread`/`UnknownWaypoint` arms; `to_run_webhook_spec` made `pub(crate)`; the `run_controller_auth` test module.
- `crates/paladin-web/src/thread_controller.rs` — `list_threads`, `get_thread`, `fork_thread`, `delete_thread` handlers; `ThreadSummaryDto`/`ThreadListQuery`/`ThreadListResponse`/`ThreadResponse`/`ForkThreadRequest`/`ForkThreadResponse`; `ThreadApiState` gains `runs`/`run_submission` + builders, `#[non_exhaustive]`; `MockWaypointStore::delete_thread` (test double) fixed to actually delete rather than stub `Ok(0)`.
- `crates/paladin-web/src/assistant_controller.rs` — `parse_limit`/`MAX_ASSISTANT_LIMIT`/`DEFAULT_ASSISTANT_LIMIT` removed; both call sites switched to `crate::pagination::resolve_limit`.
- `crates/paladin-web/src/schedule_controller.rs` — same swap (`parse_limit`/`MAX_SCHEDULE_LIMIT`/`DEFAULT_SCHEDULE_LIMIT` removed).
- `crates/paladin-web/src/lib.rs` — declares `pub mod pagination;`.
- `crates/paladin-web/Cargo.toml` — `base64 = { workspace = true }` (already a workspace dependency, new direct edge for `paladin-web`).
- `crates/paladin-web/openapi.json` — regenerated; diff is exactly the new `/v1/runs`/`/v1/threads*` paths and schemas (pure additions across both commits).
- `crates/paladin-ports/src/input/run_submission_port.rs` — `ForkRun`; `cancel` gains `requested_by`; `UnknownThread`/`UnknownWaypoint` error variants; `fork` trait method; `AlwaysUnwired` test double updated.
- `src/application/services/run/submission.rs` — `waypoints: Option<Arc<dyn WaypointPort>>` + `with_waypoints`; `authorize_invocation` shared helper; `submit`/`cancel`/`fork` all call it; `fork` impl.
- `src/application/services/run/worker.rs` — `WorkerDispatch::Fork`; `decide` gains a `fork_from` parameter; `parse_fork_waypoint_id`/`fork_edit_to_state_delta`; the `Fork` dispatch arm in `run_once`; 9 new tests.
- `src/application/services/run/mod.rs` — declares `mod http_surface_tests;`.
- `src/application/services/run/http_surface_tests.rs` — `ten_concurrent_submits_one_accepted`, `fork_run_completes_from_waypoint`.
- `src/application/services/run/cancel_tests.rs` — 4 `.cancel(&run_id)` call sites updated to `.cancel(&run_id, None)` (Rule 3, outside declared scope).
- `src/application/services/run/schedule/admin.rs` — duplicate `SsrfGuard`/`classify_ipv4`/`classify_ipv6`/`ssrf_guard_tests` removed; both `check_url` call sites now `.await` the shared guard.
- `src/application/services/run/schedule/mod.rs` — `pub use admin::SsrfGuard;` removed (no longer exists there).
- `src/application/services/run/schedule/service.rs` — `use super::admin::SsrfGuard` → `use super::super::webhook::SsrfGuard`; `SsrfGuard::default()` → `SsrfGuard::new(false)`.
- `src/application/services/run/schedule/tests.rs` — `RecordingSubmission` gains `fork`; `cancel` signature updated (Rule 3, outside declared scope).
- `MIGRATION.md` — two new §9.2 rows (`ThreadApiState` extension, `RunSubmissionPort` extension) and a new §9.6 subsection (the 7-route status-code table, the pagination/scope/rate-limit paragraphs, the SsrfGuard-collapse note).
- `Cargo.lock` — one dependency-list line change (`base64` gains a `paladin-web` consuming edge; no new package).

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **`cancel`'s authorization check is opt-in via `Option<(String, UserRole)>`, not a new required parameter.** `None` (every pre-existing caller, including `cancel_tests.rs`'s four call sites this task updated) skips the check entirely and keeps the exact pre-27-15 behavior; `Some` (the HTTP handler, always) does one extra `repository.get` + `resolver.resolve` before writing the durable cancel flag. This mirrors `submit`'s own resolve-then-authorize order rather than inventing a second convention.
2. **`fork` fails closed (`NotWired`) rather than skipping the waypoint-existence check when `WaypointPort` is unwired.** The alternative — silently accepting any `from_waypoint_id` when no validator is configured — would let a caller fork from a waypoint that never existed; a genuine `501` is the honest answer this repo's own D-24 precedent already establishes for "the collaborator this route needs is not configured."
3. **`ThreadSummaryDto` (list) and `ThreadResponse` (get-one) are asymmetric on purpose.** `WaypointPort::list_threads` returns `ThreadSummary`, which has no `waypoint_id` field at all; `WaypointPort::latest` (used by `get_thread`) returns a full `Waypoint`, which does. Adding a `waypoint_id` to `ThreadSummary` would require changing `WaypointPort`'s return contract and every `paladin-storage` backend adapter — outside this plan's declared file scope — so the list endpoint omits it and says so in its own rustdoc, rather than doing an N+1 `latest()` call per list item or silently claiming a field that isn't there.
4. **The `SsrfGuard` collapse (`schedule/admin.rs`/`mod.rs`/`service.rs`) is a deviation from the plan's own declared `files_modified`, authorized explicitly by the wave_context text** ("If `schedule/admin.rs` is outside your declared `files_modified`, treat this as the plan's own cross-cutting 'one guard everywhere' intent"). Not a spontaneous scope expansion — the orchestrator's own merged-wave context named this exact collapse as owed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `RunSubmissionPort::cancel`'s new `requested_by` parameter broke 4 call sites in `cancel_tests.rs`, outside this plan's declared file scope**
- **Found during:** Task 1, first `cargo check --workspace --all-targets --all-features`
- **Issue:** `src/application/services/run/cancel_tests.rs` (27-07's own test file, not in this plan's `files_modified`) calls `service.cancel(&run_id)`/`service_b.cancel(&run_id)` four times with the old one-argument signature.
- **Fix:** Updated all four call sites to `.cancel(&run_id, None)` — `None` preserves the exact pre-27-15 behavior (no authorization check), since none of these tests exercise the new D-46 authorization path.
- **Files modified:** `src/application/services/run/cancel_tests.rs`
- **Verification:** `cargo test -p paladin-ai --lib cancel_tests` — 5 passed, unchanged assertions.
- **Committed in:** `227ffecc` (Task 1 commit)

**2. [Rule 3 - Blocking] `RunSubmissionPort::fork`/`cancel`'s new shape broke `schedule/tests.rs`'s `RecordingSubmission` test double, outside this plan's declared file scope**
- **Found during:** Task 1, the same `cargo check` pass
- **Issue:** `src/application/services/run/schedule/tests.rs`'s `RecordingSubmission` implements `RunSubmissionPort` and needed both the new `cancel` parameter and a `fork` method to keep compiling.
- **Fix:** Updated `cancel`'s signature (ignoring the new parameter, unchanged `NotWired` behavior) and added a minimal `fork` returning `Ok(RunAccepted { run_id: RunId::new_v7(), thread_id: request.thread_id })`, mirroring the existing `submit` implementation's shape.
- **Files modified:** `src/application/services/run/schedule/tests.rs`
- **Verification:** `cargo test -p paladin-ai --lib services::run::schedule` — 21 passed.
- **Committed in:** `227ffecc` (Task 1 commit)

**3. [Rule 1 - Bug] `MockWaypointStore::delete_thread` test double was a stub always returning `Ok(0)`, causing `delete_thread_found_returns_204` to fail**
- **Found during:** Task 2, first `cargo test -p paladin-web --lib thread_controller` after adding the new `delete_thread` tests
- **Issue:** The pre-existing `MockWaypointStore` (used across every `thread_controller.rs` test) had a stub `delete_thread` that never actually removed anything, so a test seeding a thread and then deleting it always saw `deleted == 0` → `404`, not the expected `204`.
- **Fix:** `delete_thread` now actually removes the thread from both the `latest` and `history` maps, returning `1` if either held something for that thread, `0` otherwise — a genuine (if minimal) implementation instead of a stub.
- **Files modified:** `crates/paladin-web/src/thread_controller.rs` (test module only)
- **Verification:** `cargo test -p paladin-web --lib thread_controller` — 33 passed.
- **Committed in:** `02faf98f` (Task 2 commit)

**4. [wave_context-authorized] Collapsed `schedule/admin.rs`'s duplicate `SsrfGuard` onto 27-13's shared `webhook::SsrfGuard`**
- **Found during:** Task 2, per the orchestrator's own wave_context instructions
- **Issue:** 27-14 (a parallel wave that could not see 27-13's landed work) built an independent, standalone `SsrfGuard` copy in `schedule/admin.rs` with the same name/signature, explicitly documented as "for 27-15 to later collapse."
- **Fix:** Deleted the duplicate struct, its `classify_ipv4`/`classify_ipv6` helpers, and its 9 `ssrf_guard_tests` (redundant with `webhook::ssrf::tests::webhook_ssrf_guard`'s own exhaustive table); `schedule/admin.rs`'s two `check_url` call sites now `.await` the shared, async guard and render `SsrfRejection::to_string()`; `schedule/service.rs`/`schedule/mod.rs` updated their imports/re-exports accordingly.
- **Files modified:** `src/application/services/run/schedule/admin.rs`, `src/application/services/run/schedule/mod.rs`, `src/application/services/run/schedule/service.rs`
- **Verification:** `cargo test -p paladin-ai --lib services::run::schedule` — 21 passed (create/patch webhook-rejection tests unaffected by the swap); `cargo test -p paladin-ai --lib services::run::webhook` — 21 passed.
- **Committed in:** `02faf98f` (Task 2 commit)

---

**Total deviations:** 4 auto-fixed (2 Rule 3 blocking fixes for out-of-scope test doubles, 1 Rule 1 test-double bug fix, 1 wave_context-authorized cross-cutting collapse) plus 1 documented plan-acceptance-criterion discrepancy (see Issues Encountered)
**Impact on plan:** All four fixes were necessary to reach a compiling, fully-passing workspace state; none changed this plan's own architecture or scope. The SsrfGuard collapse was explicitly pre-authorized by the orchestrator's wave_context text.

## Issues Encountered

- **Plan acceptance-criterion discrepancy (not a code defect):** Task 2's acceptance criteria state `grep -c 'async fn fork' crates/paladin-ports/src/input/run_submission_port.rs` **is 1**. The actual, necessarily-correct count is **2** — the trait method declaration plus the `AlwaysUnwired` test double's required implementation (a `RunSubmissionPort` implementor MUST implement every trait method to compile). This exactly mirrors 27-07's own precedent for `cancel` (`grep -c 'async fn cancel' ... -> 2`, documented in that plan's own SUMMARY as "trait method + `AlwaysUnwired` impl"). The literal "is 1" criterion appears to have been written without accounting for the test double, and is not achievable without either breaking compilation or omitting `AlwaysUnwired`'s implementation. Verified: `grep -c 'async fn fork' crates/paladin-ports/src/input/run_submission_port.rs` → `2`.
- Two rustdoc `broken_intra_doc_links` warnings appeared in `pagination.rs`'s own module-level (`//!`) doc comment referencing same-module `pub` items by bracket-link syntax (`[`PageQuery`]`, `[`resolve_limit`]`, `[`encode_cursor`]`, `[`DEFAULT_PAGE_LIMIT`]`, `[`MAX_PAGE_LIMIT`]`) — rustdoc could not resolve any of them from a `//!` module doc, even though `///` item-level docs elsewhere in this same plan's changes (`worker.rs`) resolved identical-shaped links to in-scope items without issue. Switched to plain backticks throughout the module doc, per CLAUDE.md's own stated rustdoc convention; `cargo doc -p paladin-web --no-deps` returned to the pre-existing baseline of exactly 3 warnings (all in `thread_controller.rs`, unrelated to this plan).

## User Setup Required

None — no external service configuration required. Every new test in this plan runs against `InMemory*` adapters or a real on-disk `SqliteRunRepository` temp file (Tier 1, D-51, no Docker).

## Next Phase Readiness

- The full PRD 06 §2.1 run/thread HTTP surface is now complete: `/v1/runs*` (submit, list, get, stream, cancel, webhook-deliveries) and `/v1/threads*` (state, resume, history, list, get, fork, delete) all exist, paginate identically, and share the same two-tier D-46 authorization convention.
- `RunSubmissionPort::fork`/`cancel`'s `requested_by` parameter and `RunSubmissionService::with_waypoints` are ready for `src/bin/paladin-server.rs`'s production wiring (not this plan's scope) — the same `RunRepositoryPort`/`RunQueuePort`/`WaypointPort`/`AssistantResolver` instances a later wiring plan (27-17, per every prior plan's own SUMMARY) already assembles for `RunSubmissionService::new` need only gain one more `.with_waypoints(...)` call.
- `WorkerDispatch::Fork` is ready for the production `RunWorkerPool` the same wiring plan constructs — no additional wiring beyond what `RunWorkerPool::new` already requires (`fork_from` dispatch falls out of the existing `Run`/`Waypoint` data this pool already reads).
- No blockers. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo check --workspace --all-targets --all-features` all pass clean on the final commit; `cargo build --bin paladin-server --features web-server` succeeds (`ThreadApiState`'s `#[non_exhaustive]`/builder-only change does not break the binary's own construction sites); `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` introduces no new warnings beyond the pre-existing baseline (4/1/3 respectively).

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `crates/paladin-web/src/pagination.rs`
- FOUND: `src/application/services/run/http_surface_tests.rs`
- FOUND: `crates/paladin-web/src/run_controller.rs`
- FOUND: `crates/paladin-web/src/thread_controller.rs`
- FOUND: `crates/paladin-web/src/assistant_controller.rs`
- FOUND: `crates/paladin-web/src/schedule_controller.rs`
- FOUND: `crates/paladin-web/src/lib.rs`
- FOUND: `crates/paladin-web/Cargo.toml`
- FOUND: `crates/paladin-web/openapi.json`
- FOUND: `crates/paladin-ports/src/input/run_submission_port.rs`
- FOUND: `src/application/services/run/submission.rs`
- FOUND: `src/application/services/run/worker.rs`
- FOUND: `src/application/services/run/mod.rs`
- FOUND: `src/application/services/run/cancel_tests.rs`
- FOUND: `src/application/services/run/schedule/admin.rs`
- FOUND: `src/application/services/run/schedule/mod.rs`
- FOUND: `src/application/services/run/schedule/service.rs`
- FOUND: `src/application/services/run/schedule/tests.rs`
- FOUND: `MIGRATION.md`

**Commits verified to exist (git log --oneline):**
- FOUND: `227ffecc` feat(27-15): pagination.rs, remaining run routes, and cross-cutting scopes/429 proof
- FOUND: `02faf98f` feat(27-15): thread routes (list/get/fork/delete), fork-shaped worker dispatch, and MIGRATION registration

**Verification commands re-run and confirmed passing:**
- `cargo test -p paladin-web --lib pagination` → `test result: ok. 10 passed`
- `cargo test -p paladin-web --lib run_controller` → `test result: ok. 30 passed`
- `cargo test -p paladin-web --lib run_controller_auth` → `test result: ok. 5 passed`
- `cargo test -p paladin-web --lib thread_controller` → `test result: ok. 33 passed`
- `cargo test -p paladin-web --lib openapi_matches_committed_baseline` → `test result: ok. 1 passed`
- `cargo test -p paladin-web --lib` (whole crate) → `test result: ok. 222 passed`
- `cargo test -p paladin-ai --lib ten_concurrent_submits_one_accepted` → `test result: ok. 1 passed` (re-run 4× consecutively, stable every time)
- `cargo test -p paladin-ai --lib fork_run_completes_from_waypoint` → `test result: ok. 1 passed`
- `cargo test -p paladin-ai --lib services::run` → `test result: ok. 108 passed`
- `cargo test -p paladin-ai --lib services::run::schedule` → `test result: ok. 21 passed`
- `cargo test -p paladin-ai --lib services::run::webhook` → `test result: ok. 21 passed`
- `cargo test -p paladin-ai --lib` (whole crate) → `test result: ok. 894 passed`
- `cargo test -p paladin-ports --lib` → `test result: ok. 179 passed`
- `grep -c 'resolve_limit' crates/paladin-web/src/run_controller.rs crates/paladin-web/src/assistant_controller.rs crates/paladin-web/src/schedule_controller.rs` → `3`, `2`, `1` (all ≥ 1)
- `python3` path-existence + delete-key probe on `crates/paladin-web/openapi.json` (`/v1/runs`, `/v1/runs/{run_id}/cancel`, `/v1/runs/{run_id}/webhook-deliveries`, `/v1/threads`, `/v1/threads/{id}`, `/v1/threads/{id}/fork`, DELETE on `/v1/threads/{id}`) → `True`
- `grep -c 'omitted' crates/paladin-web/src/run_controller.rs` → `6` (≥ 1, the cursor-walk caveat text)
- `grep -B3 'pub struct ThreadApiState' crates/paladin-web/src/thread_controller.rs | grep -c non_exhaustive` → `1`
- `grep -c 'async fn fork' crates/paladin-ports/src/input/run_submission_port.rs` → `2` (see Issues Encountered — plan's literal "is 1" criterion did not account for the required test-double impl)
- `grep -c '\.fork(' src/application/services/run/worker.rs` → `1` (≥ 1)
- `awk '/^## 9.2/,/^## 9.3/' MIGRATION.md | grep -c 'ThreadApiState'` → `3` (≥ 2)
- `git diff --stat 9f288751 HEAD -- crates/paladin-web/openapi.json` → 1141 insertions, 120 deletions (line-level diff of reordered/modified JSON); a structured path/schema-set comparison (Python, not raw diff) confirms ZERO removed paths and ZERO removed schemas -- the only "changed" pre-existing entries are `/v1/runs` (gained the `GET` method) and the `SubmitRunRequest`/`RunResponse` schemas (gained this plan's own new `webhook`/`pending_responses` fields), both genuinely additive changes this plan itself made, not accidental removals
- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean
- `cargo check --workspace --all-targets --all-features` → exit 0
- `cargo build --bin paladin-server --features web-server` → exit 0
- `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` → 4/1/3 warnings respectively, all pre-existing, none introduced by this plan
- No unexpected file deletions in either commit (`git diff --diff-filter=D --name-only HEAD~1 HEAD` empty for both)

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
