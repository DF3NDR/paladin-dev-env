---
phase: 53-tenant-isolation-closure
reviewed: 2026-10-31
depth: standard
files_reviewed: 18
files_reviewed_list:
  - .project/current-exports.txt
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-core/src/platform/container/principal.rs
  - crates/paladin-web/CHANGELOG.md
  - crates/paladin-web/openapi.json
  - crates/paladin-web/src/agent_controller.rs
  - crates/paladin-web/src/job_store.rs
  - crates/paladin-web/src/run_controller.rs
  - crates/paladin-web/src/thread_controller.rs
  - docs/src/api-reference/platform-api.md
  - docs/src/deployment-topologies/http-service-host.md
  - docs/src/getting-started/configuration.md
  - docs/src/user-guides/parley-and-chronicle.md
  - src/application/services/run/http_surface_tests.rs
  - src/application/services/run/submission.rs
  - src/application/services/run/worker.rs
  - src/config/agents.rs
  - src/infrastructure/web/run_api_wiring.rs
findings:
  critical: 0
  warning: 5
  info: 5
  total: 10
status: issues_found
diff_base: 2b16763d0147f39122cbab5c52ffd275111cfa4b^
---

# Phase 53: Code Review

**Reviewed:** 2026-10-31 · **Depth:** standard · **Diff base:** `2b16763d0^..HEAD`

> Written by the orchestrator from the gsd-code-reviewer's returned report (the reviewer run could not write report files itself). Content is the reviewer's, unchanged in substance.

## Structural Findings (fallow)

None provided (fallow disabled).

## Narrative Findings (AI reviewer)

### Summary

The Phase 53 diffs (`2b16763d0^..HEAD`) were reviewed in all 18 files.

**What held up:**
- The gate-before-`require_admin` ordering holds on `resume` and `delete`.
- Hidden thread, hidden job and corrupt-row responses are byte-identical to the missing-object response in status, headers and body.
- There is no `unwrap`, `expect` or `panic!` in the non-test library code checked, and no `#[deprecated]` was added.
- The only new `pub` signatures are additive (`permits_thread`, `has_engine_factory`, `create_owned`, `get_visible`, `get_job_scoped`, the two consts).
- No log line or error message interpolates an API key value. The reserved-literal messages print the key name, not its value.
- The bounded list walk's cursor is the last examined thread, advances monotonically, and is capped at 500 examined threads.

**What was found:** a few ways an owner can be locked out, a cross-tenant availability problem, and some residual distinguishability. These are documented residuals or edge cases rather than regressions.

The known `build_run_api_persists_*` failures (TESTS-01, Phase 55) were not reported.

## Warnings

### WR-01: One corrupt or newer-schema run row fails the thread-owner lookup, and for the list walk fails every tenant's `GET /threads`
**Files:** `crates/paladin-web/src/thread_controller.rs:739-756, 777-840`; `crates/paladin-web/src/run_controller.rs:741-760`

`run_repo_error_for_scope` hides only `RunRepositoryError::Serialization`. `UnknownSchemaVersion` is deliberately not hideable (PD-02-2). The effects are:

- **Existence oracle on the thread routes.** `load_visible_thread` returns `500 "run store error"` to a tenant for a thread whose latest run row has a newer schema version. A missing thread returns `404`. This is the same existence signal D-09 hides for `Serialization`, and `waypoint_error_for_scope` already treats `SchemaVersionUnsupported` as hideable. The run side is inconsistent with the waypoint side.
- **Cross-tenant availability.** In `visible_thread_page`, `UnknownSchemaVersion` falls into the `Err(other)` arm (line 815) and aborts the whole walk with `500`. One such row in any tenant's thread makes every non-admin `GET /v1/threads` fail until that row is repaired or the thread ages out of the walk. After a downgrade this is likely, and `Serialization` is explicitly skipped for the same reason.

**Fix:** In the list walk, skip and log `UnknownSchemaVersion` exactly like `Serialization` for a `Tenant` scope. Consider widening `hideable` in `run_repo_error_for_scope` to `Serialization | UnknownSchemaVersion` for the single-object routes. If the 500 is kept deliberately, add a MIGRATION note that the list route can fail wholesale.

### WR-02: "Latest run" is read with `list(limit: 1)`, so a corrupt older run row locks the owner out of their thread
**Files:** `crates/paladin-web/src/thread_controller.rs:739-756`; `src/application/services/run/submission.rs:289-303`

The SQLite and Postgres `list` fetches `limit + 1` rows and deserializes all of them (`fetch_limit` at sqlite.rs ~583, `collect::<Result<_,_>>()?`). So the second-newest run row on the thread is decoded as the next-page sentinel.

A corrupt older row on tenant A's thread makes the lookup return `Serialization`. The effects are:
- Every gated route returns `404` to tenant A for its own thread.
- Admin gets `500`.
- `ensure_thread_visible` returns `500` to the owner on `submit` or `fork`.

This turns a single bad historical row into a thread-wide denial. It also means "latest run" is judged on rows that are not the latest.

**Fix:** Add a port method that reads exactly the newest run, for example `latest_run_for_thread` on `RunRepositoryPort` using `LIMIT 1`, and use it in both places. At minimum, document the limitation next to `latest_run_for_thread`.

### WR-03: Thread ownership flips to whoever submitted the latest run, including an Admin fork or an unattributed run
**Files:** `crates/paladin-core/src/platform/container/principal.rs:325-331`; `crates/paladin-web/src/thread_controller.rs:860-900`

Ownership is the latest run's `submitted_by`. The code and tests accept that an Admin of another tenant can `fork` (or `submit` onto) tenant A's thread. The new run is attributed to the Admin's tenant. Likewise, an internal or schedule-fired run with `submitted_by = None` becomes the latest run.

Either way, tenant A immediately loses read, fork and list access to its own thread: it gets `404` for a thread it created, while the Admin's tenant now "owns" it. The docs only warn about the D-14 same-id collision, not this ownership-transfer-by-admin-action path.

**Fix:** Document the transfer explicitly in the ADR-0063 notes and `parley-and-chronicle.md`. Better, derive ownership from the thread's first run, or from the earliest attributed run, and not the latest. If "latest wins" is intentional, have admin forks keep the original tenant's attribution.

### WR-04: Submit on a caller-chosen thread id is an existence oracle, and the probe has a side effect
**File:** `src/application/services/run/submission.rs:289-305`

`ensure_thread_visible` returns `UnknownThread` (`404`) for a foreign thread, but `Ok(())` and a created thread (`202`) for a non-existent id. Hidden and missing are therefore distinguishable on `POST /v1/runs {thread_id}`. The docstring accepts this on the grounds that ids are UUIDv7 and unguessable, but `ThreadId` accepts arbitrary caller-chosen strings such as `support-chat-1`, which are guessable.

Each probe of a missing id also creates a run and thread and consumes allowance. A related effect is that a thread with Waypoints but no run row is invisible to reads (Admin-only) yet open to any tenant on submit.

**Fix:** At minimum, state this as a recorded residual in the ADR and the `Thread birth` doc. A stronger fix is to tenant-namespace caller-supplied thread ids (the deferred D-14 work), or to make a foreign-thread submit indistinguishable from birth.

### WR-05: Timing difference between a hidden thread and a missing thread
**File:** `crates/paladin-web/src/thread_controller.rs:860-900`

A missing thread returns `404` after one `waypoints.latest` call. A hidden thread (foreign, or run-less for a non-admin) additionally performs `latest_run_for_thread` first. A tenant can measure the extra round trip, and the extra database latency is nonzero. Status, headers and body are identical, but the contract text says "timing" is covered.

**Fix:** Either record timing as an accepted residual in ADR-0063, or run the owner lookup unconditionally for non-admin callers (including the missing-thread branch) so both paths do the same I/O.

## Info

### IN-01: Stale test comment contradicts the shipped gate
**File:** `src/application/services/run/http_surface_tests.rs:1669`

The doc says fork bodies for a foreign versus a missing thread "are not byte-equal on this tree (the missing-thread case reaches the waypoint check)". The gate in `fork_thread` returns the identical `unknown thread '<id>'` `404` for both cases. The comment reads as a known hidden-vs-missing divergence that no longer exists.

**Fix:** Delete or correct the sentence.

### IN-02: The Treasurer boot assertion cannot fire on the shipped path
**File:** `src/infrastructure/web/run_api_wiring.rs:1035`

`with_engine_factory(engine_factory)` is applied unconditionally at line 1002. With that wiring, `pool.has_engine_factory()` is always `true`, so the check at 1035 is vacuous in production. It is also blind to a no-op factory, which the `has_engine_factory` doc admits. The code and docs say this honestly; the TENANT-08 control is a tripwire for future edits, not enforcement.

**Fix:** None required. Optionally assert in a test that the shipped `build_run_api` path sets the factory, which `shipped_wiring_with_a_treasurer_boots` already implies.

### IN-03: Boot schedule scan has no progress guard, and the warning line is unbounded
**File:** `src/infrastructure/web/run_api_wiring.rs:690-720, 742-760`

- **No progress guard.** `scan_unattributed_schedules_paged` loops until `next_cursor` is `None`. An adapter that returned a non-advancing cursor would hang boot forever, which contradicts "the boot is never blocked".
- **Unbounded log line.** `scan_report` joins every unattributed id into one `warn!` line. A table with thousands of legacy rows produces a very large single line.

**Fix:** Break if `next == cursor` and cap the page count. Print the first N ids plus "and M more".

### IN-04: A scan page failure drops the error class entirely
**File:** `src/infrastructure/web/run_api_wiring.rs:700-706`

`Err(_)` discards the error variant. That is correct for redaction, but a transient outage and a corrupt row are indistinguishable in the log (`stopped at an unreadable page`).

**Fix:** Log a fixed variant label such as `backend` or `serialization`, with no payload.

### IN-05: Error-level logging of repository error text on every hidden probe
**Files:** `crates/paladin-web/src/run_controller.rs:719-728`; `crates/paladin-web/src/thread_controller.rs:808-814`

`redact_store_error_for_scope` and the list-walk skip log the full repository error at `error` on each request. Repository `Serialization` messages are limited to run_id, thread_id, status and attribution text, with no secrets, so no leak was found. But a tenant who knows a corrupt id can generate unbounded `error` log volume at no cost. This is the same pre-existing pattern as `internal_repo_error`.

**Fix:** Rate-limit or dedupe by id, or downgrade the tenant-hidden branch to `warn`.

---

_Reviewer: Claude (gsd-code-reviewer), depth: standard_
