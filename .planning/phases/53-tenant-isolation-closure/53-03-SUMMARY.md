---
phase: 53-tenant-isolation-closure
plan: 03
subsystem: tenant-isolation
tags: [rust, tenant-isolation, thread-routes, visibility-gate, redaction, axum, openapi, tdd]

requires:
  - phase: 53-tenant-isolation-closure
    provides: "plan 53-01 RunReadScope::permits_thread (the one thread-owner rule)"
  - phase: 53-tenant-isolation-closure
    provides: "plan 53-02 redact_store_error_for_scope and run_repo_error_for_scope (the one role-aware redaction rule)"
provides:
  - "load_visible_thread: the one gate every /v1/threads/{id}* route enters (latest run decides the owner)"
  - "waypoint_error_for_scope, thread_store_error, THREAD_STORE_ERROR_MESSAGE, latest_run_for_thread (private)"
  - "OpenAPI-derived route matrix proving hidden equals missing on all six {id} operations"
  - "GET /v1/threads/{id}/history of an unknown thread is 404 for every role"
  - "MIGRATION.md 11.1 row M-B-24 and 11.6 thread paragraph; paladin-web CHANGELOG entries; regenerated openapi.json"
affects: [53-04, 53-07, 53-08]

tech-stack:
  added: []
  patterns:
    - "One private gate per resource family; every handler binds the Principal and calls it right after the 501 check and id parse, before any body-dependent work"
    - "Gate before require_admin on mutating admin routes so the 403 never confirms a hidden object exists"
    - "Route matrix enumerated from the router's own OpenAPI document and asserted equal to a table, so an ungated new route fails the test"

key-files:
  created: []
  modified:
    - crates/paladin-web/src/thread_controller.rs
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/openapi.json
    - crates/paladin-web/CHANGELOG.md
    - MIGRATION.md

key-decisions:
  - "An Admin scope returns from load_visible_thread before any run lookup, so a run-store outage cannot fail an Admin read; the only Admin thread route that loads runs is DELETE (busy check), which answers the fixed 500 run store error (PD-03-5)"
  - "Waypoint Serialization and SchemaVersionUnsupported are hideable (a tenant gets the missing-thread 404); Backend and NotFound are not (PD-03-3)"
  - "fork_thread enters the gate too, so the controller answers hidden and missing alike before the submission service runs; ensure_thread_visible still re-checks for library callers (PD-03-1)"
  - "limit and cursor validation on history stays before the gate because it reveals nothing about the thread"

patterns-established:
  - "Watched-red by local mutation: bypass the gate in one handler, run the matrix, copy the failing assertion, restore from a backup copy"
  - "hidden-equals-missing comparison covers status, sorted headers and swapped body bytes; content-length is the one derived header excluded, because the two ids differ in length and the swapped-body equality already proves the lengths agree"

requirements-completed: [TENANT-03, TENANT-06]

coverage:
  - id: D1
    description: "GET /v1/threads/{id} is visible to the owning tenant, a peer and an Admin and hidden from another tenant behind the byte-equal missing-thread 404 (tracer)"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "cargo test -p paladin-web --lib thread_controller::tests::get_thread_is_visible_to_its_tenant_and_admin_and_hidden_from_others"
        status: pass
    human_judgment: false
  - id: D2
    description: "A zero-run thread is Admin-only; a mixed-tenant thread belongs to its latest run's tenant and moves when a newer run arrives"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::zero_run_thread_is_visible_to_admin_only"
        status: pass
      - kind: integration
        ref: "thread_controller::tests::mixed_tenant_thread_belongs_to_the_latest_runs_tenant"
        status: pass
    human_judgment: false
  - id: D3
    description: "All six {id} operations, read from the router's own OpenAPI document, hide a foreign thread behind the byte-equal missing-thread 404 (no 403) and give the owner and Admin the documented status"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404"
        status: pass
    human_judgment: false
  - id: D4
    description: "resume and delete run the gate before require_admin: foreign non-admin 404, same-tenant non-admin 403, Admin passes"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::post_resume_by_a_foreign_user_is_404_not_403"
        status: pass
    human_judgment: false
  - id: D5
    description: "A second DELETE of a deleted thread is 404 for Admin, owner and foreign tenant"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::second_delete_of_a_thread_is_404_for_every_role"
        status: pass
    human_judgment: false
  - id: D6
    description: "No run store wired: a non-admin gets the missing-thread 404 on every route, an Admin is unaffected"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::thread_routes_fail_closed_for_non_admin_without_a_run_store"
        status: pass
    human_judgment: false
  - id: D7
    description: "Unreadable rows on every route: tenant 404 equal to missing, Admin fixed 500 (run store error on DELETE only, thread store error for the waypoint seam), no injected text in any body"
    requirement: "TENANT-06"
    verification:
      - kind: integration
        ref: "thread_controller::tests::serialization_on_thread_routes_is_hidden_for_tenant_and_redacted_for_admin"
        status: pass
      - kind: integration
        ref: "thread_controller::tests::unreadable_rows_on_get_thread_follow_the_role_aware_redaction"
        status: pass
    human_judgment: false
  - id: D8
    description: "Tenant hints in headers or query change nothing on the thread routes"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::tenant_hints_in_headers_or_query_change_nothing_on_thread_routes"
        status: pass
    human_judgment: false
  - id: D9
    description: "openapi.json regenerated (history 404, rewritten thread descriptions only); frozen v0.9 golden still passes; MIGRATION allowlist gate green"
    requirement: "TENANT-03"
    verification:
      - kind: other
        ref: "cargo test -p paladin-web --lib openapi_matches_committed_baseline; cargo test -p paladin-web --test openapi_golden_v0_9; make check-migration-allowlist"
        status: pass
    human_judgment: false

duration: 70min
completed: 2026-10-31
status: complete
---

# Phase 53 Plan 03: Thread visibility gate on every /v1/threads/{id} route Summary

**One private gate, `load_visible_thread`, now fronts all six `/v1/threads/{id}*` operations: a thread belongs to the tenant of its latest run, a foreign tenant gets the byte-identical missing-thread `404` (on `resume` and `delete` too, because the gate runs before `require_admin`), a run-less thread is Admin-only, a missing run store fails closed for non-admins, and no thread route echoes backend text.**

## Performance

- **Duration:** about 70 min wall clock (about 20 min of it test and doc authoring before the first commit; builds were warm)
- **Started:** 2026-10-31T11:15Z (approximate; no start stamp was recorded)
- **Completed:** 2026-10-31T12:30Z
- **Tasks:** 3 (Task 1 tracer, Tasks 2 and 3 auto)
- **Files modified:** 5
- **Plan start SHA:** `7fce8fab6a5c7f646e6f22fc32b9a2a012aec855` (the 53-02 docs commit)

## Accomplishments

- `load_visible_thread(state, principal, thread) -> Result<Waypoint, ApiError>`: 501 without a waypoint backend; loads the latest Waypoint (missing is the hidden `404`); an `All` scope returns there with no run lookup; no run store is the hidden `404` (D-07); otherwise the newest run is read (`RunQuery { thread_id, limit: 1, scope: All }`) and `RunReadScope::permits_thread` decides. Rustdoc covers hidden-equals-missing, latest by `submitted_at`, zero-run Admin-only, fail-closed, and the D-14 limitation.
- `waypoint_error_for_scope` (hideable: `Serialization`, `SchemaVersionUnsupported`), `thread_store_error` (fixed `500 "thread store error"` for every role), `latest_run_for_thread`, `THREAD_STORE_ERROR_MESSAGE`. Both reuse 53-02's `redact_store_error_for_scope` and `run_repo_error_for_scope`; nothing was reimplemented.
- All six handlers enter the gate in the PD-03-2 order. `resume_thread` and `delete_thread` call it before `require_admin`. `fork_thread` calls it before parsing `from_waypoint_id` or the webhook. `get_thread_state` and `get_thread` reuse the Waypoint the gate returns (no second `latest()`).
- Every backend-text site is gone: the verify regex `ApiError::internal\((e|other)\.to_string\(\)\)` matches nothing in `thread_controller.rs`. `list_threads`, `get_thread_history`, `delete_thread` (store and busy check) and the `map_parley_error` fallthrough now render the fixed messages.
- `GET /v1/threads/{id}/history` of an unknown thread is `404` for every role, with the `404` response added to its `#[utoipa::path]`.
- Module docs rewritten: seven-row route table and `## Authorization (tenant visibility, ADR-0063)`; `run_controller.rs` closing paragraph now points at the gate instead of calling the thread routes deployment-wide.
- `openapi.json` regenerated with `make openapi`; the diff touches only `/v1/threads...` operations (descriptions, and the history `404`). `openapi_golden_v0_9` passes unchanged.
- `MIGRATION.md` 11.1 row `M-B-24` and an 11.6 paragraph; `crates/paladin-web/CHANGELOG.md` `### Security` and `### Changed` under `[Unreleased]`. 11.2 stays empty; sections 9 and 10 are untouched.

## Task Commits

1. **Task 1 (tracer): `load_visible_thread` wired from the core rule to `GET /v1/threads/{id}` over real HTTP** - `e466c2654` (feat)
2. **Task 2: state, history, resume, fork and delete through the gate; redaction; re-seeded tests; docs, OpenAPI, MIGRATION, CHANGELOG** - `0502efa2f` (feat)
3. **Task 3: OpenAPI-derived route matrix, delete idempotency, corrupt-row and tenant-hint proofs** - `260f4c5ee` (test)

Tracer feedback gate (auto mode active): the tracer's full `<verify>` (four named tests plus the whole `thread_controller` module, 39 passed) was re-run after its commit and was green before Task 2 began.

## Prove-then-lock and watched-red

- **Tracer red:** the four tracer tests were written first against the ungated `get_thread` and failed (4 failed, 34 baseline tests passed); the gate turned them green.
- **Task 2 red:** after gating the handlers, 13 pre-gate tests failed until re-seeded (the resume, fork and history tests, `delete_thread_non_admin_is_403`, `post_resume_with_non_admin_role_is_403`, `post_resume_with_admin_role_is_202`). All were re-seeded under their original names.
- **AC-06 watched-red (gate-bypass mutation, uncommitted, restored from a backup copy):** with `load_visible_thread` replaced by a no-op in `get_thread_history`, four tests failed, including the matrix. The matrix assertion that fired (the foreign-tenant status check in `every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404`, `thread_controller.rs:2917` in the mutated file):

  ```
  assertion `left == right` failed: GET /v1/threads/{id}/history as a foreign tenant must be the missing-thread 404, never 403
    left: 200
   right: 404
  ```

  The same mutation also failed `tenant_hints_in_headers_or_query_change_nothing_on_thread_routes` (`GET /history with hints`: 200 vs 404), `serialization_on_thread_routes_is_hidden_for_tenant_and_redacted_for_admin` (`GET .../history as owner-key, run store failing`: 200 vs 404) and `thread_routes_fail_closed_for_non_admin_without_a_run_store` (`GET .../history as owner-key without a run store`: 200 vs 404). `git diff` after restoring showed no mutation text.

## Verification

- `cargo test -p paladin-web`: lib 311 passed (the `thread_controller` module has **44** tests: the 34 measured on the base, plus 10 added), `auth_rbac` 5, `openapi_golden_v0_9` 9, doctests 2.
- All 34 baseline `thread_controller` test names still exist and pass (compared by name against the base commit; none missing).
- `cargo fmt --check`, `cargo clippy -p paladin-web --all-targets -- -D warnings`, `make check-migration-allowlist`, `make check-changelogs`, `make api-surface` ("API surface unchanged", 4323 items): exit 0.
- Root crate users of `thread_router` re-checked: `cargo test --lib http_surface_tests` (15 passed, including 53-01's `cross_tenant_fork_is_the_unknown_thread_404`), `cargo test --features web-server --test e2e_platform_api --test v0_9_config_boot` (1 and 9 passed).
- Plan verify greps: no backend-text regex match, `narrows D-24 pending` and `**not** tenant-scoped` gone, six `load_visible_thread(&state, &principal` call sites, gate line before `require_admin(` in `resume_thread` and `delete_thread`.
- Manual credential-handling check (security.instructions.md): the new `log::error!` lines (in `redact_store_error_for_scope`, called from the thread helpers) format only the route context string and the store error; no API key, config value or request body is formatted, and no response body carries the injected text (asserted).
- Pre-existing failures not touched: `run_api_wiring::tests::build_run_api_persists_*` (network, TESTS-01); they were not run here.

## Decisions Made

See `key-decisions` above; all follow the plan's PD-03-1, PD-03-2, PD-03-3 and PD-03-5.

## Deviations from Plan

None that change behavior or the public surface. Minor ordering and scope differences:

**1. [Rule 3 - Blocking] `list_threads` leak site replaced in Task 1, not Task 2**
- **Found during:** Task 1 (clippy `-D warnings` flagged `thread_store_error` as dead code)
- **Fix:** routed the `list_threads` store error through `thread_store_error("thread API list", e)` in the tracer commit, which is the same change Task 2 specified.
- **Commit:** `e466c2654`

**2. [Rule 3 - Blocking] One extra tracer test**
- **Found during:** Task 1 (the `fail_latest` and `fail_reads_with` seams and two `WaypointFailure` variants were dead code until Task 3)
- **Fix:** added `unreadable_rows_on_get_thread_follow_the_role_aware_redaction` (GET only, all three waypoint variants plus the run-store seam) so the Task 1 commit was clippy-clean. It stays as a distinct proof.
- **Commit:** `e466c2654`

**3. [Rule 1 - Test design] `content-length` excluded from the header comparison**
- **Found during:** Task 1 (the plan's two ids differ in length, so `content-length` differs by exactly that amount)
- **Fix:** the helper drops `content-length` from the sorted-header comparison and compares the swapped body bytes, which proves the lengths agree. Documented in the helper's rustdoc.

**4. Matrix uses a fresh state per row**
- The plan described one state with DELETE ordered last; each row builds its own seeded fixture instead, so the DELETE row cannot disturb the others. The per-row order (owner and peer, foreign hidden against missing, Admin last) is as specified, and `published == table` is asserted on every row.

**5. `post_resume_unknown_thread_is_404` and `get_thread_state_with_non_admin_role_is_200_not_403` re-seeded although they still passed**
- Left unseeded they would have passed for the wrong reason (the gate's `404`, not the parley mapping; and a `404` under a test named "200"). They are seeded so they test what their names say.

Handler-order consequence of PD-03-2 (not a deviation): `resume_thread` now checks the parley port before `require_admin`, so a non-admin on a deployment with no parley port gets `501` rather than `403`.

## Notes for plan 53-04

The module docs of `thread_controller.rs` state that `GET /threads` is outside this gate and that its tenant-filtered walk is plan 53-04's. When 53-04 lands, update that last sentence of the `## Authorization (tenant visibility, ADR-0063)` section.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path, file access or schema change; the work narrows existing routes.

## Self-Check: PASSED

- Files: `crates/paladin-web/src/thread_controller.rs`, `run_controller.rs`, `openapi.json`, `CHANGELOG.md` and `MIGRATION.md` all exist and are committed.
- Commits: `e466c2654`, `0502efa2f`, `260f4c5ee` all present in `git log`.
