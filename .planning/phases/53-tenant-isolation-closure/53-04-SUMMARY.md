---
phase: 53-tenant-isolation-closure
plan: 04
subsystem: tenant-isolation
tags: [rust, tenant-isolation, thread-list, pagination, axum, openapi, tdd]

requires:
  - phase: 53-tenant-isolation-closure
    provides: "plan 53-01 RunReadScope::permits_thread (the one thread-owner rule)"
  - phase: 53-tenant-isolation-closure
    provides: "plan 53-02 run_repo_error_for_scope (the role-aware run-store error rule)"
  - phase: 53-tenant-isolation-closure
    provides: "plan 53-03 latest_run_for_thread, thread_store_error, THREAD_STORE_ERROR_MESSAGE"
provides:
  - "visible_thread_page: bounded, newest-first, tenant-filtered walk behind GET /v1/threads"
  - "MAX_EXAMINED_THREADS = 500 and THREAD_SCAN_BATCH = 100 (private consts)"
  - "MIGRATION.md 11.1 row M-B-25 and 11.6 list paragraph; paladin-web CHANGELOG entry; regenerated openapi.json description"
affects: [53-07, 53-08]

tech-stack:
  added: []
  patterns:
    - "A list route applies the single-object gate's rule per item (same latest-run lookup, same permits_thread) instead of a second ownership rule"
    - "Per-request examination cap with a cursor from the last EXAMINED item, so a filtered page may be short but the walk always advances"

key-files:
  created: []
  modified:
    - crates/paladin-web/src/thread_controller.rs
    - crates/paladin-web/openapi.json
    - crates/paladin-web/CHANGELOG.md
    - MIGRATION.md

key-decisions:
  - "Admin (All scope) keeps the single list_threads(limit + 1) fetch and performs no owner lookup; only a Tenant scope walks (D-04)"
  - "A Serialization failure on a foreign thread's owner lookup is skipped and logged at error for a Tenant scope; every other run-repository error is the fixed 500 run store error; a waypoint list error is the fixed 500 thread store error (PD-04-3)"
  - "The walk returns the last examined thread's last_updated_at as next_cursor, or null only when the store is exhausted; when exactly limit items are kept on the final batch the next request returns an empty page with a null cursor (PD-04-2)"
  - "Non-admin with no run store wired lists nothing, without touching the waypoint store (D-07)"

patterns-established:
  - "Watched-red by local mutation for proofs written after the implementation: remove the cap check, or disable the Serialization skip, and copy the failing assertion; restore from a backup copy"

requirements-completed: [TENANT-03]

coverage:
  - id: D1
    description: "GET /v1/threads as a tenant omits other tenants' threads and zero-run threads, keeps newest-activity order, and an Admin sees all (tracer)"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "cargo test -p paladin-web --lib thread_controller::tests::list_threads_omits_foreign_and_zero_run_threads"
        status: pass
    human_judgment: false
  - id: D2
    description: "Nothing visible is exactly {\"items\":[],\"next_cursor\":null}"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::list_threads_returns_the_empty_page_shape_when_nothing_is_visible"
        status: pass
    human_judgment: false
  - id: D3
    description: "Following next_cursor to null yields every visible thread exactly once, newest first, with no repeat; a page can be short (even empty) with a non-null cursor"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::list_threads_short_pages_advance_the_cursor_without_repeats"
        status: pass
    human_judgment: false
  - id: D4
    description: "A request examines exactly 500 threads (five batched list calls of 100) and returns a short page with a cursor equal to the 500th examined thread's timestamp; the walk then continues to a null cursor"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::list_threads_examination_cap_returns_a_short_page_with_a_cursor"
        status: pass
    human_judgment: false
  - id: D5
    description: "A foreign thread with an unreadable run row is skipped and logged for a tenant; a Backend failure is the fixed 500 run store error with no backend text; the Admin list is unaffected"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::list_threads_skips_a_corrupt_thread_for_a_tenant"
        status: pass
    human_judgment: false
  - id: D6
    description: "No run store wired: a non-admin lists nothing, an Admin lists everything"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::list_threads_fails_closed_for_non_admin_without_a_run_store"
        status: pass
    human_judgment: false
  - id: D7
    description: "The Admin path performs zero run-repository list calls and exactly one list_threads(limit + 1) call"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::list_threads_admin_path_performs_no_owner_lookup"
        status: pass
    human_judgment: false
  - id: D8
    description: "Tenant hints in headers or query change nothing on the list"
    requirement: "TENANT-03"
    verification:
      - kind: integration
        ref: "thread_controller::tests::list_threads_ignores_tenant_hints"
        status: pass
    human_judgment: false
  - id: D9
    description: "openapi.json regenerated (list_threads summary/description and the 200 description only); frozen v0.9 golden passes; MIGRATION allowlist gate, changelog gate and API-surface check green"
    requirement: "TENANT-03"
    verification:
      - kind: other
        ref: "cargo test -p paladin-web --lib openapi_matches_committed_baseline; cargo test -p paladin-web --test openapi_golden_v0_9; make check-migration-allowlist; make check-changelogs; make api-surface"
        status: pass
    human_judgment: false

duration: 45min
completed: 2026-10-31
status: complete
---

# Phase 53 Plan 04: Tenant-filtered thread list Summary

**`GET /v1/threads` now lists a user-role principal only the threads whose latest run is its tenant's: `visible_thread_page` walks the waypoint store in batches of 100, reuses the single-thread gate's latest-run lookup and `permits_thread`, examines at most 500 threads per request, and returns a cursor from the last examined thread so a filtered page may be short but the walk always advances and never repeats an item.**

## Performance

- **Duration:** about 45 min wall clock (builds were warm)
- **Completed:** 2026-10-31
- **Tasks:** 2 (Task 1 tracer, Task 2 auto)
- **Files modified:** 4
- **Plan start SHA:** `9c60a584243e596c9099642342dd3af99a288392` (the 53-03 docs commit)

## Accomplishments

- `visible_thread_page(waypoints, runs, scope, limit, before) -> Result<(Vec<ThreadSummary>, Option<DateTime<Utc>>), ApiError>`: loops over `list_threads(Some(THREAD_SCAN_BATCH), cursor)`; per examined thread it calls `latest_run_for_thread` (scope `All`) and keeps the thread when `RunReadScope::permits_thread` admits it; stops with `Some(last examined last_updated_at)` when `limit` threads are kept or `MAX_EXAMINED_THREADS` were examined; an empty batch or a batch shorter than `THREAD_SCAN_BATCH` ends the walk with `None`. A waypoint list error is the fixed `thread store error` 500.
- Owner-lookup errors: `RunRepositoryError::Serialization` under a `Tenant` scope is logged at `error` (`thread API list owner lookup failed ... thread skipped as not visible`) and the thread is skipped, so one tenant's corrupt row cannot fail another tenant's list; any other error goes through `run_repo_error_for_scope(&RunReadScope::All, ...)` to the fixed `run store error` 500.
- `list_threads` binds `Extension(principal)` and branches on `principal.read_scope()`: `All` keeps the existing `limit + 1` single fetch with no owner lookup; a `Tenant` scope with no run store returns `{"items":[],"next_cursor":null}` (D-07); otherwise it calls `visible_thread_page`. A small private helper `thread_summary_dto` now builds the DTO for both paths.
- Docs: `list_threads` rustdoc and `#[utoipa::path]` descriptions, and the module `## Authorization` closing paragraph (the sentence 53-03 flagged as outside the gate) now describe the tenant filter, short pages and the keyset-not-snapshot property (PD-04-4).
- `MIGRATION.md` 11.1 row `M-B-25` and an 11.6 paragraph; `crates/paladin-web/CHANGELOG.md` `[Unreleased]` `### Security` entry. 11.2 stays empty; sections 9 and 10 are untouched.
- `openapi.json` regenerated with `make openapi`; only the `list_threads` summary, description and 200 description changed.

## Task Commits

1. **Task 1 (tracer): `visible_thread_page` and the tenant branch of `list_threads`, paging waypoint double, omission and empty-shape tests** - `1caa1f543` (feat)
2. **Task 2: cursor, cap, corrupt-row, fail-closed, Admin-path and tenant-hint proofs; docs, OpenAPI, MIGRATION, CHANGELOG** - `d76eef279` (test)

Tracer feedback gate (auto mode active): the tracer's `<verify>` (both named tests and the whole `thread_controller` module, 46 passed) was re-run after its commit and was green before Task 2 began; "Tracer verified end-to-end -- expanding".

## Prove-then-lock and watched-red

- **Tracer red:** the two tracer tests failed against the unfiltered list (the tenant saw `["globex-2","acme-2","zero-1","acme-1"]`, and the empty-shape test received three items).
- **Task 2:** the Serialization skip and the cap were already part of the Task 1 implementation (the plan places their wiring in Task 1 step 2 and their proofs in Task 2), so the Task 2 tests passed on first run. To show they have teeth they were checked by local mutation (uncommitted, restored from a backup copy; `git diff` afterwards showed only the intended additions):
  - Mutation 1, remove `|| examined >= MAX_EXAMINED_THREADS`: `list_threads_examination_cap_returns_a_short_page_with_a_cursor` failed (`exactly the cap's worth of owner lookups on the first request`, left 603) and `list_threads_short_pages_advance_the_cursor_without_repeats` failed (`a filtered page may be shorter than limit while the cursor still advances`).
  - Mutation 2, make the `Serialization` arm unreachable: `list_threads_skips_a_corrupt_thread_for_a_tenant` failed (status 500 instead of 200).

## Verification

- `cargo test -p paladin-web`: lib 319 passed (the `thread_controller` module now has 52 tests: 44 after plan 53-03, plus 2 in Task 1 and 6 in Task 2), `auth_rbac` 5, `openapi_golden_v0_9` 9, doctests 2. All five pre-existing `list_threads_*` tests still pass.
- `cargo fmt --check`, `cargo clippy -p paladin-web --all-targets -- -D warnings`, `make check-migration-allowlist`, `make check-changelogs`, `make api-surface` ("API surface unchanged", 4323 items): exit 0.
- Root-crate users of `thread_router` re-checked: `cargo test --lib http_surface_tests` (15 passed), `cargo test --features web-server --test e2e_platform_api --test v0_9_config_boot` (1 and 9 passed).
- Public surface: no `pub` signature changed; `visible_thread_page`, the two constants and all test seams are private; no `WaypointPort` or `RunRepositoryPort` change.
- Manual credential-handling check (security.instructions.md): the one new `log::error!` formats the repository's `Serialization` message and a fixed context string only; no API key, config value or request body, and the corrupt-row test asserts the injected text never reaches a response body.
- Pre-existing failures not touched: `run_api_wiring::tests::build_run_api_persists_*` (network, TESTS-01); not run here.

## Decisions Made

See `key-decisions` above; they follow the plan's PD-04-1 to PD-04-4 and orchestrator notes 2 and 6.

## Deviations from Plan

**1. [Rule 3 - Blocking] Run-double failure seam and call counter introduced in Task 2, not Task 1**
- **Found during:** Task 1 (clippy `-D warnings` flagged `ThreadRunFailure`, `fail_on_thread` and `list_call_count` as dead code, and flagged `sort_by` and a complex tuple type in the new waypoint double)
- **Fix:** kept the Task 1 commit clippy-clean by landing the failure seam and `list_call_count` with their first users in Task 2; the waypoint call log and the `list` counter field stayed in Task 1; replaced `sort_by` with `sort_by_key(Reverse(..))` and added a `ListCall` type alias.
- **Commits:** `1caa1f543`, `d76eef279`

**2. [Rule 1 - Test design] Short-page test reaches the "short page with a non-null cursor" shape through the cap**
- **Found during:** Task 2 (with a batch size of 100 a short page with a non-null cursor can only arise when the 500-thread cap stops the walk first)
- **Fix:** `list_threads_short_pages_advance_the_cursor_without_repeats` seeds 520 foreign threads above a 25-thread alternating tail, so the first response is an empty page with a cursor and later pages are full; it asserts the collected ids equal the visible set with no repeat. A separate, smaller scenario was not needed.

**3. Log capture in a test**
- The corrupt-row test installs a tiny capturing `log::Log` once per test binary (the crate had none) to assert the `error`-level skip line; if another logger is ever installed first the HTTP-visible assertions still hold and only the log assertion would fail.

## Known Stubs

None.

## Threat Flags

None. The change narrows an existing route; no new endpoint, auth path, file access or schema change. T-53-23 (cursor timestamps as a side channel) is accepted as planned: the cursor reveals only that some thread was updated at that instant.

## Notes for plans 53-07 and 53-08

- ADR-0063 should record the list walk: per-thread latest-run lookup with the `All` scope, 500-thread examination cap in batches of 100, skip-and-log for a tenant's `Serialization` lookup, Admin unchanged, and the accepted exclusive-timestamp cursor property.
- The `GET /v1/threads` change is a behavioral change, not an API break: row `M-B-25` is in 11.1, 11.2 is untouched.

## Self-Check: PASSED

- Files: `crates/paladin-web/src/thread_controller.rs`, `crates/paladin-web/openapi.json`, `crates/paladin-web/CHANGELOG.md` and `MIGRATION.md` all exist and are committed.
- Commits: `1caa1f543` and `d76eef279` are present in `git log`.
