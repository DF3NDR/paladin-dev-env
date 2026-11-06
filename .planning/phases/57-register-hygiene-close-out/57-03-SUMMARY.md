---
phase: 57-register-hygiene-close-out
plan: 03
subsystem: web
tags: [thread-ownership, tenant-isolation, adr-0063, thread-controller, submission-guard, openapi, sqlite]

requires:
  - phase: 57-register-hygiene-close-out
    provides: "RunRepositoryPort::owner_run_for_thread (57-02, option-a) and RunReadScope::permits_thread(owner)"
provides:
  - "thread gate, thread list walk and submit/fork guard all read ownership only through owner_run_for_thread"
  - "newer-schema (UnknownSchemaVersion) owner rows skipped in the tenant list and hidden on the single-thread gate"
  - "HTTP tracer over on-disk SQLite proving a later Admin or schedule run never moves a thread"
  - "openapi.json and rustdoc describing the owner-run (earliest attributed run) rule"
affects: [57-04, 57-11]

tech-stack:
  added: []
  patterns:
    - "thread-only owner-lookup error classifier (thread_owner_error_for_scope) that hides both Serialization and UnknownSchemaVersion, leaving the run routes' classifier untouched"

key-files:
  created: []
  modified:
    - crates/paladin-web/src/thread_controller.rs
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/openapi.json
    - src/application/services/run/submission.rs
    - src/application/services/run/http_surface_tests.rs

key-decisions:
  - "PD-03-1 followed: a new private thread_owner_error_for_scope hides Serialization and UnknownSchemaVersion; run_repo_error_for_scope is unchanged so GET /v1/runs/{id} keeps the recorded T-53-10 residual"
  - "RUN_STORE_ERROR_MESSAGE became pub(crate) in run_controller.rs so the thread classifier renders the same fixed 'run store error' 500; crate-private, no public surface change"

patterns-established:
  - "Authorization reads on a thread go through the single-row owner_run_for_thread, never a list page"

requirements-completed: []  # REG-02 spans 57-03, 57-04 and 57-11; not ticked here

duration: ~75min
completed: 2026-11-04
status: complete
---

# Phase 57 Plan 03: Web and service thread-owner wiring Summary

**The thread gate, the tenant list walk and the submit/fork guard now authorize on a thread's owner run (its earliest attributed run) through `owner_run_for_thread`, a newer-schema owner row is skipped in lists and hidden as a `404` on single reads, and `openapi.json` says so.**

Plan-start SHA: `6d32dbc7d4f013e9e8f0259a55ecfcaf2309ef6a`

## Performance

- **Duration:** ~75 min
- **Completed:** 2026-11-04
- **Tasks:** 3 of 3
- **Files modified:** 5

## Accomplishments

- `load_visible_thread` and `visible_thread_page` call `runs.owner_run_for_thread(..)`; the private `latest_run_for_thread` is deleted, so the controller holds no recency read (P-03-2).
- `RunSubmissionService::ensure_thread_visible` calls `self.repository.owner_run_for_thread(thread_id)` and matches `Some(owner) if !scope.permits_thread(Some(&owner))` as `UnknownThread`; thread birth (`None` -> `Ok`), the `requested_by: None` early return and the `All` early return are unchanged. `RunQuery` is no longer used by that function.
- Row 85: the list walk's skip-and-log arm matches `Serialization | UnknownSchemaVersion` for a Tenant scope; `load_visible_thread` uses the new private `thread_owner_error_for_scope`, which hides both variants. The log line interpolates only the error's own `Display` (P-03-3).
- `MockRunRepository` overrides `owner_run_for_thread` (attributed first, then `submitted_at` ascending, then `run_id` ascending), counts calls in `owner_calls`, and gains `ThreadRunFailure::UnknownSchemaVersion`. The two bounded-walk cost assertions now read `owner_call_count()`.
- `openapi.json` regenerated with `make openapi`: five `description` lines changed, nothing else.

## Task Commits

1. **Task 1 (tracer): thread gate, list walk and submit guard read the owner run (rows 86, 87)** - `a9de68468` (fix)
2. **Task 2: newer-schema owner row skipped in the list and hidden on the gate; owner-rule tests; re-pointed test double (row 85)** - `a5c81bc06` (fix)
3. **Task 3: owner-run wording in rustdoc and utoipa descriptions; regenerated openapi.json** - `15594e064` (docs)

**Plan metadata:** recorded by the docs commit that follows this file.

## Tracer gate

tracer gate: verify re-run passed; orchestrator-approved (unattended session), not owner-approved. The re-run executed Task 1's full `<verify>` on the committed state (`a9de68468`): the tracer printed `... ok` (1 passed), `cross_tenant_` 3 passed, `application::services::run::submission::` 35 passed, and both greps held.

## Red-green

| Test | RED (watched, pre-change code) | GREEN |
|------|-------------------------------|-------|
| `http_surface_tests::thread_owner_is_the_earliest_attributed_run_over_sqlite` | Failed at `http_surface_tests.rs:1832`: `the owner tenant must read its thread: {"error":{"code":"not_found",...,"message":"unknown thread 'owner-thread-e3fa0180-...'"}}`, left `404`, right `200`. The Admin run (the newest) owned the thread, so acme was locked out of its own thread. | `ok` after the controller and service rewiring; re-run `ok` on the commit |
| `thread_controller::tests::list_threads_skips_a_newer_schema_thread_for_a_tenant` | FAILED at `thread_controller.rs:3804`: `{"error":{"code":"internal",...,"message":"run store error"}}`, left `500`, right `200` | `ok` after widening the skip arm |
| `thread_controller::tests::newer_schema_owner_row_hides_the_thread_from_a_tenant` | FAILED at `thread_controller.rs:3858`: `owner-key: {"error":{"code":"internal",...,"message":"run store error"}}`, left `500`, right `404` | `ok` after `thread_owner_error_for_scope` |
| `admin_run_on_a_tenant_thread_does_not_move_its_owner`, `unattributed_run_on_a_tenant_thread_does_not_move_its_owner`, `submit_guard_reads_the_owner_run_not_the_newest_run` | Not failing-first: Task 1's rewiring already made the owner rule hold, so these passed on first run. They pin the rule at the controller and service as a regression net (the plan's "rows 86/87 regression net"). | `ok` |

## Changed expectations

- **`mixed_tenant_thread_belongs_to_the_latest_runs_tenant` renamed to `mixed_tenant_thread_belongs_to_the_earliest_attributed_runs_tenant`** (PD-03-4). It encoded the old rule (globex's newer run took the thread: globex 200, acme 404, then a newer acme run moved it back). It now asserts the owner rule: acme@t0, globex@t0+1 gives acme 200, admin 200, globex 404, and a further globex run at t0+2 changes nothing. It failed on the Task 1 rewiring exactly as the plan predicted, so the rewrite landed in the Task 1 commit (not Task 2) to keep every commit green. ADR-0063's Evidence line that names the old test is for 57-04.
- **Owner-lookup cost assertions re-pointed from `list_call_count()` to `owner_call_count()`** in `list_threads_examination_cap_returns_a_short_page_with_a_cursor` (still `MAX_EXAMINED_THREADS`) and `list_threads_admin_path_performs_no_owner_lookup` (still `0`). Reason: owner lookups no longer go through `list` (Pitfall 5). The mock's `list` no longer counts or injects per-thread failures; both moved to the `owner_run_for_thread` override.
- No other pre-existing expectation changed. `cross_tenant_submit_on_a_thread_is_the_unknown_thread_404`, `cross_tenant_fork_is_the_unknown_thread_404` and `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag` pass unmodified.

## Gates

- **fmt:** `cargo fmt --all -- --check` clean (FMT_OK).
- **clippy:** `cargo clippy -p paladin-web --all-targets -- -D warnings` exit 0; `cargo clippy --lib --tests --features web-server -- -D warnings` exit 0.
- **doc:** `cargo doc -p paladin-web --no-deps` 0 warnings; also clean under `RUSTDOCFLAGS="-D warnings"`.
- **api-surface:** `make api-surface`: `API surface unchanged` (4331 items). `make api-surface-update` not run.
- **semver (patch):** `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh --package paladin-web`: `223 checks: 223 pass, 31 skip`; `semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`. `--package paladin-ai`: `223 checks: 223 pass, 31 skip`; `0 failure(s)`.
- **openapi:** `make openapi` then `openapi_matches_committed_baseline`: `1 passed`. `git diff` on `openapi.json` is 5 insertions and 5 deletions, all `"description"` lines.
- **wording:** case-insensitive `latest run` count is 0 in `thread_controller.rs`, `run_controller.rs`, `submission.rs` and `openapi.json`; `earliest attributed run` appears in `openapi.json`.
- **Tests run:**
  - `cargo test --lib --features web-server` (root): 1509 passed, 0 failed
  - `cargo test -p paladin-web`: 333 lib + 5 + 9 + 3 passed, 0 failed
  - Postgres live-DB tests: not touched by this plan and not run.
  - Not run: the full workspace `cargo test` and `make test-all`; only the root lib and `paladin-web`.

## Evidence for WINDOWS rows 85, 86 and 87

Row 85 (a newer-schema owner row must not fail a tenant's list or reveal a thread):
- `thread_controller::tests::list_threads_skips_a_newer_schema_thread_for_a_tenant` (list walk, with the server-side log assertion)
- `thread_controller::tests::newer_schema_owner_row_hides_the_thread_from_a_tenant` (single object; status, headers and id-normalised body byte-compared with an unknown id)
- `thread_controller::tests::list_threads_skips_a_corrupt_thread_for_a_tenant` (pre-existing `Serialization` twin; unchanged and passing)
- Residual kept on purpose: `GET /v1/runs/{id}` still answers a tenant `500` for a newer-schema run row (T-53-10); `run_repo_error_for_scope` is not widened.

Row 86 (a corrupt non-owner row must not lock the owner out): wired end to end by this plan; the lookup itself is proven by 57-02's `owner_run_for_thread_decodes_only_the_owner_row`.
- `http_surface_tests::thread_owner_is_the_earliest_attributed_run_over_sqlite` (real on-disk SQLite)

Row 87 (a later Admin or schedule run must not move the owner):
- `http_surface_tests::thread_owner_is_the_earliest_attributed_run_over_sqlite` (acme, then an unattributed run, then an `ops-tenant` Admin run; acme `200` read, listed, `202` submit; globex byte-identical `404` read and `unknown thread` `404` submit with no run inserted; Admin `200`)
- `thread_controller::tests::admin_run_on_a_tenant_thread_does_not_move_its_owner`
- `thread_controller::tests::unattributed_run_on_a_tenant_thread_does_not_move_its_owner`
- `thread_controller::tests::mixed_tenant_thread_belongs_to_the_earliest_attributed_runs_tenant`
- `application::services::run::submission::tests::submit_guard_reads_the_owner_run_not_the_newest_run`

Caveat for 57-11: the rows close as `fixed` only after 57-04 records the migration note and the ADR-0063 amendment. This plan did not touch `.planning/WINDOWS.md` or tick REG-02.

## Files Created/Modified

- `crates/paladin-web/src/thread_controller.rs` - owner-run gate and list walk, `thread_owner_error_for_scope`, deleted `latest_run_for_thread`, test double and seven owner-rule tests, rewritten docs and utoipa descriptions
- `crates/paladin-web/src/run_controller.rs` - `RUN_STORE_ERROR_MESSAGE` made `pub(crate)`; module-doc line 94 reworded; `run_repo_error_for_scope` unchanged
- `crates/paladin-web/openapi.json` - five regenerated `description` strings
- `src/application/services/run/submission.rs` - `ensure_thread_visible` on the owner run, reworded docs, `submit_guard_reads_the_owner_run_not_the_newest_run`
- `src/application/services/run/http_surface_tests.rs` - the SQLite tracer

## Decisions Made

- Followed PD-03-1 to PD-03-4 as written. No new decision beyond the `pub(crate)` visibility change below.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `RUN_STORE_ERROR_MESSAGE` was private to `run_controller.rs`**
- **Found during:** Task 2, writing `thread_owner_error_for_scope`
- **Issue:** the classifier needs the fixed `run store error` message the run routes use, and the constant was a private `const`.
- **Fix:** made it `pub(crate)` (a one-token change on one line, no public item changed). The alternative, hard-coding a second copy of the string in `thread_controller.rs`, would let the two messages drift apart.
- **Files modified:** `crates/paladin-web/src/run_controller.rs`
- **Commit:** `a5c81bc06`

**2. [Rule 1 - Bug] Dangling intra-doc link to the deleted helper**
- **Found during:** Task 1 (deleting `latest_run_for_thread`)
- **Fix:** repointed the `visible_thread_page` doc link at `RunRepositoryPort::owner_run_for_thread` at once, rather than waiting for the Task 3 wording pass; also removed the two imports (`Run`, `RunQuery`) the deleted helper left unused.
- **Commit:** `a9de68468`

### Plan assumptions that did not hold (recorded, not defects)

3. **Rewrite of the mixed-tenant test happened in Task 1, not Task 2.** The Task 1 rewiring failed it immediately and the repository's commit bar requires green tests, so the rename and rewrite went into the Task 1 commit. Content is as PD-03-4 specifies.
4. **The plan's `run_controller.rs` "byte-unchanged" acceptance** is satisfied for `run_repo_error_for_scope` itself. The file's diff against the plan-start SHA has exactly two changes: the `pub(crate)` token (deviation 1) and the module-doc line (Task 3).
5. **The tracer's thread-list leg is not the walk model the plan cited.** The plan pointed at a thread-list walk test "around :1395-1500"; those lines hold the run-list e2e test, and no existing SQLite thread-router test exists. The tracer builds `SqliteRunRepository` plus `SqliteWaypointStore` itself and merges `run_router` with `thread_router`, as `build_resume_rig` does.
6. **Commit trailer.** Commits carry `Co-Authored-By: Claude Sonnet 5.5` (the attribution the session's system reminder requires, which names the model that did this work) rather than the `Claude Fable 5.1` line the orchestrator prompt and the 57-02 commits used. The `Claude-Session:` line is identical. Flagging so the orchestrator can amend if it wants uniformity.

### Left as is, on purpose

7. `RunSubmissionService::fork` still reads the newest run on the thread (`list` with `limit: 1`) to copy that run's assistant reference. That read picks an assistant to copy, not a tenant; authorization happens earlier in `ensure_thread_visible`. P-03-2 forbids recency for authorization only.

**Total deviations:** 2 auto-fixed (Rule 3, Rule 1), 3 plan-assumption corrections and 1 attribution note, 1 deliberate non-change. **Impact:** none on delivered behavior or on any acceptance criterion.

## Issues Encountered

None beyond the items above. Disk stayed above 5.9 GB throughout; no ENOSPC, no incremental-cache deletion.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path, file access or schema change. The one new log line interpolates only the error's own `Display`, and the response-body assertion in `list_threads_skips_a_newer_schema_thread_for_a_tenant` confirms the injected text never reaches a client. Threats T-57-09 (submit guard, `submit_guard_reads_the_owner_run_not_the_newest_run` and the tracer's submit legs), T-57-10 (`newer_schema_owner_row_hides_the_thread_from_a_tenant`), T-57-11 (list skip) and T-57-12 (log content) are mitigated as planned; T-57-13 stays accepted (run routes keep the T-53-10 residual).

## User Setup Required

None.

## Next Phase Readiness

- 57-04 can record the legacy mixed-tenant-thread change as a MIGRATION section 11.1 row, amend ADR-0063 (Evidence line names the old test; the residual line narrows to the run routes), and cite the renamed test.
- 57-11 can close WINDOWS rows 85, 86 and 87 citing the test names above.

## Self-Check: PASSED

All five modified source files, the SUMMARY and commits `a9de68468`, `a5c81bc06` and `15594e064` exist; no file was deleted by any commit; all three commits carry the two-line trailer; the only added `unwrap()` calls are in `#[cfg(test)]` code.
